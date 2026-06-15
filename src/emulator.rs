use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};

use crate::asm::Program;
use crate::isa::*;

#[derive(Debug, Clone, Copy, Default)]
struct Flags {
    zero: bool,
    negative: bool,
    greater: bool,
}

enum FdHandle {
    Stdin,
    Stdout,
    Stderr,
    File(File),
    Closed,
}

pub struct Machine {
    program: Program,
    regs: [u64; GPR_COUNT],
    fds: Vec<FdHandle>,
    memory: Vec<u8>,
    ip: usize,
    flags: Flags,
    heap_next: u64,
    allocations: HashMap<u64, usize>,
    pid: u64,
    tid: u64,
    uid: u64,
    gid: u64,
    sigmask: u64,
    inbox: Option<(u64, u64)>,
}

impl Machine {
    pub fn new(program: Program) -> Self {
        let mut fds = Vec::with_capacity(FDR_COUNT);
        fds.push(FdHandle::Stdin);
        fds.push(FdHandle::Stdout);
        fds.push(FdHandle::Stderr);
        for _ in 3..FDR_COUNT {
            fds.push(FdHandle::Closed);
        }

        let mut memory = vec![0; MEMORY_SIZE];
        let data_start = DATA_BASE as usize;
        let data_end = data_start + program.data.len();
        if data_end <= memory.len() {
            memory[data_start..data_end].copy_from_slice(&program.data);
        }

        let mut regs = [0; GPR_COUNT];
        regs[31] = STACK_TOP;

        Self {
            program,
            regs,
            fds,
            memory,
            ip: 0,
            flags: Flags::default(),
            heap_next: HEAP_BASE,
            allocations: HashMap::new(),
            pid: 1,
            tid: 1,
            uid: 1000,
            gid: 1000,
            sigmask: 0,
            inbox: None,
        }
    }

    pub fn run(&mut self) -> Result<i32, String> {
        let mut steps = 0usize;
        loop {
            if steps > 10_000_000 {
                return Err("execution step limit exceeded".to_string());
            }
            steps += 1;
            let Some(instr) = self.program.instructions.get(self.ip).cloned() else {
                return Ok(0);
            };
            self.ip += 1;
            if let Some(code) = self.exec(instr)? {
                return Ok(code);
            }
            self.regs[0] = 0;
        }
    }

    fn exec(&mut self, instr: Instr) -> Result<Option<i32>, String> {
        match instr {
            Instr::Nop | Instr::Fence | Instr::Yield | Instr::WaitOnFd(_, _) => {}
            Instr::Li(dst, value) => {
                let v = self.resolve_value(value)?;
                self.write_reg(dst, v);
            }
            Instr::Mov(dst, src) => self.write_reg(dst, self.read_reg(src)),
            Instr::Add(dst, a, b) => {
                self.write_reg(dst, self.read_reg(a).wrapping_add(self.read_reg(b)))
            }
            Instr::Sub(dst, a, b) => {
                self.write_reg(dst, self.read_reg(a).wrapping_sub(self.read_reg(b)))
            }
            Instr::Mul(dst, a, b) => {
                self.write_reg(dst, self.read_reg(a).wrapping_mul(self.read_reg(b)))
            }
            Instr::Div(dst, a, b) => {
                let divisor = self.read_reg(b);
                if divisor == 0 {
                    return Err("hardware SIGFPE: division by zero".to_string());
                }
                self.write_reg(dst, self.read_reg(a) / divisor);
            }
            Instr::And(dst, a, b) => self.write_reg(dst, self.read_reg(a) & self.read_reg(b)),
            Instr::Or(dst, a, b) => self.write_reg(dst, self.read_reg(a) | self.read_reg(b)),
            Instr::Xor(dst, a, b) => self.write_reg(dst, self.read_reg(a) ^ self.read_reg(b)),
            Instr::Not(dst, src) => self.write_reg(dst, !self.read_reg(src)),
            Instr::Lsl(dst, a, b) => {
                self.write_reg(dst, self.read_reg(a) << (self.read_reg(b) & 63))
            }
            Instr::Lsr(dst, a, b) => {
                self.write_reg(dst, self.read_reg(a) >> (self.read_reg(b) & 63))
            }
            Instr::Asr(dst, a, b) => self.write_reg(
                dst,
                ((self.read_reg(a) as i64) >> (self.read_reg(b) & 63)) as u64,
            ),
            Instr::Cmp(a, b) => {
                let lhs = self.read_reg(a) as i64;
                let rhs = self.read_reg(b) as i64;
                self.flags = Flags {
                    zero: lhs == rhs,
                    negative: lhs < rhs,
                    greater: lhs > rhs,
                };
            }
            Instr::Jmp(target) => self.ip = self.resolve_target(target)?,
            Instr::Branch(condition, target) => {
                if self.condition(condition) {
                    self.ip = self.resolve_target(target)?;
                }
            }
            Instr::Call(target) => {
                self.regs[31] = self.regs[31].wrapping_sub(8);
                self.store_u64(self.regs[31], self.ip as u64)?;
                self.ip = self.resolve_target(target)?;
            }
            Instr::Ret => {
                let next = self.load_u64(self.regs[31])?;
                self.regs[31] = self.regs[31].wrapping_add(8);
                self.ip = next as usize;
            }
            Instr::Ld(dst, mem, width) => {
                let addr = self.resolve_mem(mem)?;
                let value = self.load_width(addr, width)?;
                self.write_reg(dst, value);
            }
            Instr::St(mem, src, width) => {
                let addr = self.resolve_mem(mem)?;
                self.store_width(addr, self.read_reg(src), width)?;
            }
            Instr::Alloc(dst, bytes_reg) => {
                let len = self.read_reg(bytes_reg) as usize;
                let len = len.max(1);
                let addr = align_up(self.heap_next, 64);
                let end = addr
                    .checked_add(len as u64)
                    .ok_or_else(|| "allocation overflow".to_string())?;
                if end as usize >= self.memory.len() {
                    return Err(format!("out of silicon heap memory allocating {len} bytes"));
                }
                self.heap_next = end;
                self.allocations.insert(addr, len);
                self.write_reg(dst, addr);
            }
            Instr::Free(ptr) => {
                self.allocations.remove(&self.read_reg(ptr));
            }
            Instr::OpenFd(dst, path_reg, flags_reg) => {
                let path = self.read_c_string(self.read_reg(path_reg))?;
                let flags = self.read_reg(flags_reg);
                let file = if flags & 1 == 1 {
                    OpenOptions::new()
                        .create(true)
                        .truncate(false)
                        .append(true)
                        .read(true)
                        .open(&path)
                } else {
                    File::open(&path)
                }
                .map_err(|err| format!("OPEN_FD {path:?}: {err}"))?;
                self.fds[dst.0] = FdHandle::File(file);
            }
            Instr::ReadFd(fd, buf, len) => {
                let addr = self.read_reg(buf);
                let len = self.read_reg(len) as usize;
                let mut tmp = vec![0; len];
                let count = match &mut self.fds[fd.0] {
                    FdHandle::Stdin => io::stdin()
                        .read(&mut tmp)
                        .map_err(|err| format!("READ_FD fd0: {err}"))?,
                    FdHandle::File(file) => file
                        .read(&mut tmp)
                        .map_err(|err| format!("READ_FD fd{}: {err}", fd.0))?,
                    FdHandle::Stdout | FdHandle::Stderr | FdHandle::Closed => 0,
                };
                self.write_bytes(addr, &tmp[..count])?;
            }
            Instr::WriteFd(fd, buf, len) => {
                let data = self.read_bytes(self.read_reg(buf), self.read_reg(len) as usize)?;
                match &mut self.fds[fd.0] {
                    FdHandle::Stdout => {
                        io::stdout()
                            .write_all(&data)
                            .map_err(|err| format!("WRITE_FD fd1: {err}"))?;
                        io::stdout().flush().map_err(|err| err.to_string())?;
                    }
                    FdHandle::Stderr => {
                        io::stderr()
                            .write_all(&data)
                            .map_err(|err| format!("WRITE_FD fd2: {err}"))?;
                        io::stderr().flush().map_err(|err| err.to_string())?;
                    }
                    FdHandle::File(file) => file
                        .write_all(&data)
                        .map_err(|err| format!("WRITE_FD fd{}: {err}", fd.0))?,
                    FdHandle::Stdin | FdHandle::Closed => {
                        return Err(format!("WRITE_FD on non-writable fd{}", fd.0));
                    }
                }
            }
            Instr::FdDup(dst, src) => {
                self.fds[dst.0] = match &self.fds[src.0] {
                    FdHandle::Stdin => FdHandle::Stdin,
                    FdHandle::Stdout => FdHandle::Stdout,
                    FdHandle::Stderr => FdHandle::Stderr,
                    FdHandle::File(file) => FdHandle::File(
                        file.try_clone()
                            .map_err(|err| format!("FD_DUP fd{}: {err}", src.0))?,
                    ),
                    FdHandle::Closed => FdHandle::Closed,
                };
            }
            Instr::GetPcr(dst, pcr) => self.write_reg(dst, self.read_pcr(pcr)),
            Instr::SetPcr(pcr, src) => self.write_pcr(pcr, self.read_reg(src))?,
            Instr::Fork(dst) => self.write_reg(dst, 2),
            Instr::Exec(_, _) => {
                return Err("EXEC is not host-virtualized by this emulator".to_string());
            }
            Instr::Spawn(dst, _) => {
                self.tid += 1;
                self.write_reg(dst, self.tid);
            }
            Instr::Sleep(_) => {}
            Instr::Exit(code) => return Ok(Some(self.read_reg(code) as i32)),
            Instr::Mmap(dst, _hint, len, _prot, _fd, _offset) => {
                let requested = self.read_reg(len);
                let saved = self.regs[30];
                self.regs[30] = requested;
                let alloc = Instr::Alloc(dst, Reg(30));
                self.exec(alloc)?;
                self.regs[30] = saved;
            }
            Instr::Munmap(addr, _len) => {
                self.allocations.remove(&self.read_reg(addr));
            }
            Instr::Sigaction(_, _) | Instr::Sigret => {}
            Instr::SigmaskSet(mask) => self.sigmask = self.read_reg(mask),
            Instr::Kill(_, _) => {}
            Instr::LockCmpxchg(dst, addr_reg, expected, new_value) => {
                let addr = self.read_reg(addr_reg);
                let current = self.load_u64(addr)?;
                let expected = self.read_reg(expected);
                if current == expected {
                    self.store_u64(addr, self.read_reg(new_value))?;
                }
                self.write_reg(dst, current);
            }
            Instr::FutexWait(_, _) | Instr::FutexWake(_, _) => {}
            Instr::Inb(dst, _) => self.write_reg(dst, 0),
            Instr::Outb(_, _) => {}
            Instr::LoadUcode(_, _) => {
                if self.uid != 0 {
                    return Err("LOAD_UCODE denied: UID must be 0".to_string());
                }
            }
            Instr::MsgSend(_pid, v1, v2) => {
                self.inbox = Some((self.read_reg(v1), self.read_reg(v2)))
            }
            Instr::MsgRecv(dst1, dst2) => {
                let (v1, v2) = self.inbox.take().unwrap_or((0, 0));
                self.write_reg(dst1, v1);
                self.write_reg(dst2, v2);
            }
            Instr::FAdd(dst, a, b) => self.write_reg(
                dst,
                f64_bin(self.read_reg(a), self.read_reg(b), |x, y| x + y),
            ),
            Instr::FSub(dst, a, b) => self.write_reg(
                dst,
                f64_bin(self.read_reg(a), self.read_reg(b), |x, y| x - y),
            ),
            Instr::FMul(dst, a, b) => self.write_reg(
                dst,
                f64_bin(self.read_reg(a), self.read_reg(b), |x, y| x * y),
            ),
            Instr::FDiv(dst, a, b) => self.write_reg(
                dst,
                f64_bin(self.read_reg(a), self.read_reg(b), |x, y| x / y),
            ),
            Instr::VAdd32(dst, a, b) => {
                let lhs = self.read_reg(a);
                let rhs = self.read_reg(b);
                let lo = ((lhs as u32).wrapping_add(rhs as u32)) as u64;
                let hi = (((lhs >> 32) as u32).wrapping_add((rhs >> 32) as u32) as u64) << 32;
                self.write_reg(dst, hi | lo);
            }
        }
        Ok(None)
    }

    fn read_reg(&self, reg: Reg) -> u64 {
        if reg.0 == 0 { 0 } else { self.regs[reg.0] }
    }

    fn write_reg(&mut self, reg: Reg, value: u64) {
        if reg.0 != 0 && reg.0 != 31 {
            self.regs[reg.0] = value;
        }
    }

    fn condition(&self, condition: Condition) -> bool {
        match condition {
            Condition::Eq => self.flags.zero,
            Condition::Ne => !self.flags.zero,
            Condition::Lt => self.flags.negative,
            Condition::Gt => self.flags.greater,
            Condition::Le => self.flags.zero || self.flags.negative,
            Condition::Ge => self.flags.zero || self.flags.greater,
        }
    }

    fn resolve_value(&self, value: Value) -> Result<u64, String> {
        match value {
            Value::Imm(v) => Ok(v as u64),
            Value::Label(label) => {
                if let Some(addr) = self.program.data_labels.get(&label) {
                    Ok(*addr)
                } else if let Some(ip) = self.program.labels.get(&label) {
                    Ok(*ip as u64)
                } else {
                    Err(format!("unknown label {label:?}"))
                }
            }
        }
    }

    fn resolve_target(&self, target: Target) -> Result<usize, String> {
        match target {
            Target::Address(ip) => Ok(ip),
            Target::Label(label) => self
                .program
                .labels
                .get(&label)
                .copied()
                .ok_or_else(|| format!("unknown code label {label:?}")),
        }
    }

    fn resolve_mem(&self, mem: MemRef) -> Result<u64, String> {
        match mem {
            MemRef::BaseOffset(base, offset) => Ok(self.read_reg(base).wrapping_add(offset as u64)),
            MemRef::Label(label) => self
                .program
                .data_labels
                .get(&label)
                .copied()
                .ok_or_else(|| format!("unknown data label {label:?}")),
        }
    }

    fn load_width(&self, addr: u64, width: Width) -> Result<u64, String> {
        let bytes = self.read_bytes(addr, width.bytes())?;
        Ok(match width {
            Width::Byte => bytes[0] as u64,
            Width::Word => u32::from_le_bytes(bytes.try_into().unwrap()) as u64,
            Width::Double => u64::from_le_bytes(bytes.try_into().unwrap()),
        })
    }

    fn store_width(&mut self, addr: u64, value: u64, width: Width) -> Result<(), String> {
        match width {
            Width::Byte => self.write_bytes(addr, &[value as u8]),
            Width::Word => self.write_bytes(addr, &(value as u32).to_le_bytes()),
            Width::Double => self.write_bytes(addr, &value.to_le_bytes()),
        }
    }

    fn load_u64(&self, addr: u64) -> Result<u64, String> {
        self.load_width(addr, Width::Double)
    }

    fn store_u64(&mut self, addr: u64, value: u64) -> Result<(), String> {
        self.store_width(addr, value, Width::Double)
    }

    fn read_bytes(&self, addr: u64, len: usize) -> Result<Vec<u8>, String> {
        let start = addr as usize;
        let end = start
            .checked_add(len)
            .ok_or_else(|| format!("memory range overflow at 0x{addr:x}"))?;
        if end > self.memory.len() {
            return Err(format!("memory read out of range: 0x{addr:x} + {len}"));
        }
        Ok(self.memory[start..end].to_vec())
    }

    fn write_bytes(&mut self, addr: u64, data: &[u8]) -> Result<(), String> {
        let start = addr as usize;
        let end = start
            .checked_add(data.len())
            .ok_or_else(|| format!("memory range overflow at 0x{addr:x}"))?;
        if end > self.memory.len() {
            return Err(format!(
                "memory write out of range: 0x{addr:x} + {}",
                data.len()
            ));
        }
        self.memory[start..end].copy_from_slice(data);
        Ok(())
    }

    fn read_c_string(&self, addr: u64) -> Result<String, String> {
        let mut pos = addr as usize;
        if pos >= self.memory.len() {
            return Err(format!("string address out of range: 0x{addr:x}"));
        }
        let start = pos;
        while pos < self.memory.len() && self.memory[pos] != 0 {
            pos += 1;
        }
        String::from_utf8(self.memory[start..pos].to_vec())
            .map_err(|err| format!("invalid utf-8 string at 0x{addr:x}: {err}"))
    }

    fn read_pcr(&self, pcr: Pcr) -> u64 {
        match pcr {
            Pcr::Pid => self.pid,
            Pcr::Tid => self.tid,
            Pcr::Uid => self.uid,
            Pcr::Gid => self.gid,
            Pcr::Sigmask => self.sigmask,
        }
    }

    fn write_pcr(&mut self, pcr: Pcr, value: u64) -> Result<(), String> {
        match pcr {
            Pcr::Pid | Pcr::Tid => Err("PID and TID PCRs are read-only".to_string()),
            Pcr::Uid if self.uid != 0 => {
                Err("SET_PCR UID denied: current UID is not 0".to_string())
            }
            Pcr::Uid => {
                self.uid = value;
                Ok(())
            }
            Pcr::Gid => {
                self.gid = value;
                Ok(())
            }
            Pcr::Sigmask => {
                self.sigmask = value;
                Ok(())
            }
        }
    }
}

fn align_up(value: u64, align: u64) -> u64 {
    (value + align - 1) & !(align - 1)
}

fn f64_bin(a: u64, b: u64, op: fn(f64, f64) -> f64) -> u64 {
    op(f64::from_bits(a), f64::from_bits(b)).to_bits()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_integer_loop() {
        let program = Program::parse(
            r#"
            .text
              LI r1, 5
              LI r2, 1
            loop:
              LI r3, 1
              CMP r1, r3
              BLE done
              MUL r2, r2, r1
              SUB r1, r1, r3
              JMP loop
            done:
              LI r3, 120
              CMP r2, r3
              BNE bad
              EXIT r0
            bad:
              LI r1, 1
              EXIT r1
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        assert_eq!(machine.run().unwrap(), 0);
    }
}

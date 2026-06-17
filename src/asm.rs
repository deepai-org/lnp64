use std::collections::HashMap;

use crate::isa::*;

#[derive(Debug, Clone)]
pub struct Program {
    pub instructions: Vec<Instr>,
    pub labels: HashMap<String, usize>,
    pub data_labels: HashMap<String, u64>,
    pub data: Vec<u8>,
}

impl Program {
    pub fn parse(source: &str) -> Result<Self, String> {
        let mut parser = Parser::default();
        for (idx, raw) in source.lines().enumerate() {
            parser.line_no = idx + 1;
            parser.parse_line(raw)?;
        }
        parser.finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Section {
    #[default]
    Text,
    Data,
}

#[derive(Default)]
struct Parser {
    section: Section,
    line_no: usize,
    instructions: Vec<Instr>,
    labels: HashMap<String, usize>,
    data_labels: HashMap<String, u64>,
    data: Vec<u8>,
    data_relocs: Vec<(usize, String)>,
}

impl Parser {
    fn finish(mut self) -> Result<Program, String> {
        for (offset, label) in &self.data_relocs {
            let Some(value) = self
                .data_labels
                .get(label)
                .copied()
                .or_else(|| self.labels.get(label).map(|addr| *addr as u64))
            else {
                return Err(format!("unknown data label {label:?}"));
            };
            self.data[*offset..*offset + 8].copy_from_slice(&value.to_le_bytes());
        }
        Ok(Program {
            instructions: self.instructions,
            labels: self.labels,
            data_labels: self.data_labels,
            data: self.data,
        })
    }

    fn parse_line(&mut self, raw: &str) -> Result<(), String> {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            return Ok(());
        }
        if line == ".text" {
            self.section = Section::Text;
            return Ok(());
        }
        if line == ".data" {
            self.section = Section::Data;
            return Ok(());
        }

        let mut rest = line;
        if let Some((label, after)) = split_label(rest) {
            self.define_label(label)?;
            rest = after.trim();
            if rest.is_empty() {
                return Ok(());
            }
        }

        match self.section {
            Section::Text => {
                let instr = self.parse_instr(rest)?;
                self.instructions.push(instr);
            }
            Section::Data => self.parse_data(rest)?,
        }
        Ok(())
    }

    fn define_label(&mut self, label: &str) -> Result<(), String> {
        validate_label(label).map_err(|err| self.err(err))?;
        if self.labels.contains_key(label) || self.data_labels.contains_key(label) {
            return Err(self.err(format!("duplicate label {label:?}")));
        }
        match self.section {
            Section::Text => {
                self.labels
                    .insert(label.to_string(), self.instructions.len());
            }
            Section::Data => {
                self.data_labels
                    .insert(label.to_string(), DATA_BASE + self.data.len() as u64);
            }
        }
        Ok(())
    }

    fn parse_data(&mut self, line: &str) -> Result<(), String> {
        let (directive, rest) = split_once_ws(line);
        match directive {
            ".string" => {
                let value = parse_string(rest.trim()).map_err(|err| self.err(err))?;
                self.data.extend_from_slice(value.as_bytes());
                self.data.push(0);
            }
            ".bytes" => {
                for part in split_operands(rest) {
                    let byte = parse_i64(&part).map_err(|err| self.err(err))?;
                    if !(0..=255).contains(&byte) {
                        return Err(self.err(format!("byte out of range: {byte}")));
                    }
                    self.data.push(byte as u8);
                }
            }
            ".quad" => {
                let text = rest.trim();
                let value = if let Ok(value) = parse_i64(text) {
                    value as u64
                } else {
                    let offset = self.data.len();
                    self.data_relocs.push((offset, text.to_string()));
                    0
                };
                self.data.extend_from_slice(&value.to_le_bytes());
            }
            ".zero" => {
                let count = parse_i64(rest.trim()).map_err(|err| self.err(err))?;
                if count < 0 {
                    return Err(self.err(".zero requires a non-negative count".to_string()));
                }
                self.data.resize(self.data.len() + count as usize, 0);
            }
            other => return Err(self.err(format!("unknown data directive {other:?}"))),
        }
        Ok(())
    }

    fn parse_instr(&self, line: &str) -> Result<Instr, String> {
        let (op_raw, rest) = split_once_ws(line);
        let op = op_raw.to_ascii_uppercase();
        let args = split_operands(rest);
        let arity = |n: usize| -> Result<(), String> {
            if args.len() == n {
                Ok(())
            } else {
                Err(self.err(format!("{op} expects {n} operands, got {}", args.len())))
            }
        };

        let instr = match op.as_str() {
            "NOP" => {
                arity(0)?;
                Instr::Nop
            }
            "LI" => {
                arity(2)?;
                Instr::Li(reg(&args[0])?, value(&args[1]))
            }
            "MOV" => {
                arity(2)?;
                Instr::Mov(reg(&args[0])?, reg(&args[1])?)
            }
            "ADD" => alu3(&args, Instr::Add)?,
            "SUB" => alu3(&args, Instr::Sub)?,
            "MUL" => alu3(&args, Instr::Mul)?,
            "DIV" => alu3(&args, Instr::Div)?,
            "AND" => alu3(&args, Instr::And)?,
            "OR" => alu3(&args, Instr::Or)?,
            "XOR" => alu3(&args, Instr::Xor)?,
            "NOT" => {
                arity(2)?;
                Instr::Not(reg(&args[0])?, reg(&args[1])?)
            }
            "LSL" => alu3(&args, Instr::Lsl)?,
            "LSR" => alu3(&args, Instr::Lsr)?,
            "ASR" => alu3(&args, Instr::Asr)?,
            "CMP" => {
                arity(2)?;
                Instr::Cmp(reg(&args[0])?, reg(&args[1])?)
            }
            "JMP" => {
                arity(1)?;
                Instr::Jmp(target(&args[0]))
            }
            "BEQ" => branch(&args, Condition::Eq)?,
            "BNE" => branch(&args, Condition::Ne)?,
            "BLT" => branch(&args, Condition::Lt)?,
            "BGT" => branch(&args, Condition::Gt)?,
            "BLE" => branch(&args, Condition::Le)?,
            "BGE" => branch(&args, Condition::Ge)?,
            "CALL" => {
                arity(1)?;
                Instr::Call(target(&args[0]))
            }
            "CALL_REG" => {
                arity(1)?;
                Instr::CallReg(reg(&args[0])?)
            }
            "RET" => {
                arity(0)?;
                Instr::Ret
            }
            "LD" | "LD.D" => load(&args, Width::Double)?,
            "LD.W" => load(&args, Width::Word)?,
            "LD.B" => load(&args, Width::Byte)?,
            "ST" | "ST.D" => store(&args, Width::Double)?,
            "ST.W" => store(&args, Width::Word)?,
            "ST.B" => store(&args, Width::Byte)?,
            "FENCE" => {
                arity(0)?;
                Instr::Fence
            }
            "PULL" => {
                arity(4)?;
                Instr::Pull(
                    reg(&args[0])?,
                    fd(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                )
            }
            "PUSH" => {
                arity(4)?;
                Instr::Push(
                    reg(&args[0])?,
                    fd(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                )
            }
            "AWAIT" => {
                arity(3)?;
                Instr::Await(reg(&args[0])?, fd(&args[1])?, reg(&args[2])?)
            }
            "AWAIT_DYN" => {
                arity(3)?;
                Instr::AwaitDyn(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "POLL_FD" => {
                arity(3)?;
                Instr::PollFd(reg(&args[0])?, fd(&args[1])?, reg(&args[2])?)
            }
            "POLL_FD_DYN" => {
                arity(3)?;
                Instr::PollFdDyn(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "ALLOC" => {
                arity(2)?;
                Instr::Alloc(reg(&args[0])?, reg(&args[1])?)
            }
            "ALLOC_EX" => {
                arity(3)?;
                Instr::AllocEx(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "ALLOC_SIZE" => {
                arity(2)?;
                Instr::AllocSize(reg(&args[0])?, reg(&args[1])?)
            }
            "RANDOM" => {
                arity(3)?;
                Instr::Random(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "FREE" => {
                arity(1)?;
                Instr::Free(reg(&args[0])?)
            }
            "OPEN_FD" => {
                arity(3)?;
                Instr::OpenFd(fd(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "OPEN_FD_DYN" => {
                arity(3)?;
                Instr::OpenFdDyn(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "OPEN_DIR" => {
                arity(3)?;
                Instr::OpenDir(fd(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "OPEN_DIR_DYN" => {
                arity(3)?;
                Instr::OpenDirDyn(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "READ_FD" => {
                arity(3)?;
                Instr::ReadFd(fd(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "READ_FD_DYN" => {
                arity(3)?;
                Instr::ReadFdDyn(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "PREAD_FD" => {
                arity(4)?;
                Instr::PreadFd(
                    fd(&args[0])?,
                    reg(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                )
            }
            "PREAD_FD_DYN" => {
                arity(4)?;
                Instr::PreadFdDyn(
                    reg(&args[0])?,
                    reg(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                )
            }
            "READDIR_FD" => {
                arity(2)?;
                Instr::ReaddirFd(fd(&args[0])?, reg(&args[1])?)
            }
            "READDIR_FD_DYN" => {
                arity(2)?;
                Instr::ReaddirFdDyn(reg(&args[0])?, reg(&args[1])?)
            }
            "REWINDDIR_FD" => {
                arity(1)?;
                Instr::RewinddirFd(fd(&args[0])?)
            }
            "REWINDDIR_FD_DYN" => {
                arity(1)?;
                Instr::RewinddirFdDyn(reg(&args[0])?)
            }
            "WRITE_FD" => {
                arity(3)?;
                Instr::WriteFd(fd(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "WRITE_FD_DYN" => {
                arity(3)?;
                Instr::WriteFdDyn(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "PWRITE_FD" => {
                arity(4)?;
                Instr::PwriteFd(
                    fd(&args[0])?,
                    reg(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                )
            }
            "PWRITE_FD_DYN" => {
                arity(4)?;
                Instr::PwriteFdDyn(
                    reg(&args[0])?,
                    reg(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                )
            }
            "MKDIR_PATH" => {
                arity(2)?;
                Instr::MkdirPath(reg(&args[0])?, reg(&args[1])?)
            }
            "UNLINK_PATH" => {
                arity(1)?;
                Instr::UnlinkPath(reg(&args[0])?)
            }
            "RENAME_PATH" => {
                arity(2)?;
                Instr::RenamePath(reg(&args[0])?, reg(&args[1])?)
            }
            "LINK_PATH" => {
                arity(3)?;
                Instr::LinkPath(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "SYMLINK_PATH" => {
                arity(2)?;
                Instr::SymlinkPath(reg(&args[0])?, reg(&args[1])?)
            }
            "READLINK_PATH" => {
                arity(3)?;
                Instr::ReadlinkPath(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "CHDIR_PATH" => {
                arity(1)?;
                Instr::ChdirPath(reg(&args[0])?)
            }
            "GETCWD_PATH" => {
                arity(2)?;
                Instr::GetcwdPath(reg(&args[0])?, reg(&args[1])?)
            }
            "CHMOD_PATH" => {
                arity(3)?;
                Instr::ChmodPath(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "CHOWN_PATH" => {
                arity(4)?;
                Instr::ChownPath(
                    reg(&args[0])?,
                    reg(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                )
            }
            "UTIME_PATH" => {
                arity(3)?;
                Instr::UtimePath(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "UTIME_FD" => {
                arity(2)?;
                Instr::UtimeFd(fd(&args[0])?, reg(&args[1])?)
            }
            "UTIME_FD_DYN" => {
                arity(2)?;
                Instr::UtimeFdDyn(reg(&args[0])?, reg(&args[1])?)
            }
            "STAT_PATH" => {
                arity(3)?;
                Instr::StatPath(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "STAT_FD" => {
                arity(2)?;
                Instr::StatFd(reg(&args[0])?, fd(&args[1])?)
            }
            "STAT_FD_DYN" => {
                arity(2)?;
                Instr::StatFdDyn(reg(&args[0])?, reg(&args[1])?)
            }
            "FD_CLOSE" => {
                arity(1)?;
                Instr::FdClose(fd(&args[0])?)
            }
            "FD_CLOSE_DYN" => {
                arity(1)?;
                Instr::FdCloseDyn(reg(&args[0])?)
            }
            "FD_SEEK" => {
                arity(3)?;
                Instr::FdSeek(fd(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "FD_SEEK_DYN" => {
                arity(3)?;
                Instr::FdSeekDyn(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "WAIT_ON_FD" => {
                arity(2)?;
                Instr::WaitOnFd(fd(&args[0])?, reg(&args[1])?)
            }
            "FD_DUP" => {
                arity(2)?;
                Instr::FdDup(fd(&args[0])?, fd(&args[1])?)
            }
            "FD_DUP2" => {
                arity(2)?;
                Instr::FdDup2(fd(&args[0])?, fd(&args[1])?)
            }
            "ERRNO_GET" => {
                arity(1)?;
                Instr::ErrnoGet(reg(&args[0])?)
            }
            "ERRNO_SET" => {
                arity(1)?;
                Instr::ErrnoSet(reg(&args[0])?)
            }
            "WAIT_PID" => {
                arity(2)?;
                Instr::WaitPid(reg(&args[0])?, reg(&args[1])?)
            }
            "GET_PCR" => {
                arity(2)?;
                Instr::GetPcr(
                    reg(&args[0])?,
                    parse_pcr(&args[1]).map_err(|err| self.err(err))?,
                )
            }
            "SET_PCR" => {
                arity(2)?;
                Instr::SetPcr(
                    parse_pcr(&args[0]).map_err(|err| self.err(err))?,
                    reg(&args[1])?,
                )
            }
            "FORK" => {
                arity(1)?;
                Instr::Fork(reg(&args[0])?)
            }
            "EXEC" => {
                arity(2)?;
                Instr::Exec(reg(&args[0])?, reg(&args[1])?)
            }
            "SPAWN" => {
                arity(2)?;
                Instr::Spawn(reg(&args[0])?, reg(&args[1])?)
            }
            "THREAD_JOIN" => {
                arity(3)?;
                Instr::ThreadJoin(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "YIELD" => {
                arity(0)?;
                Instr::Yield
            }
            "SLEEP" => {
                arity(1)?;
                Instr::Sleep(reg(&args[0])?)
            }
            "EXIT" => {
                arity(1)?;
                Instr::Exit(reg(&args[0])?)
            }
            "MMAP" => {
                arity(6)?;
                Instr::Mmap(
                    reg(&args[0])?,
                    reg(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                    fd(&args[4])?,
                    reg(&args[5])?,
                )
            }
            "MUNMAP" => {
                arity(2)?;
                Instr::Munmap(reg(&args[0])?, reg(&args[1])?)
            }
            "MPROTECT" => {
                arity(3)?;
                Instr::Mprotect(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "SIGACTION" => {
                arity(2)?;
                Instr::Sigaction(reg(&args[0])?, reg(&args[1])?)
            }
            "SIGMASK_SET" => {
                arity(1)?;
                Instr::SigmaskSet(reg(&args[0])?)
            }
            "ALARM" => {
                arity(2)?;
                Instr::Alarm(reg(&args[0])?, reg(&args[1])?)
            }
            "KILL" => {
                arity(2)?;
                Instr::Kill(reg(&args[0])?, reg(&args[1])?)
            }
            "SIGRET" => {
                arity(0)?;
                Instr::Sigret
            }
            "LOCK.CMPXCHG" => {
                arity(4)?;
                Instr::LockCmpxchg(
                    reg(&args[0])?,
                    reg(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                )
            }
            "FUTEX_WAIT" => {
                arity(2)?;
                Instr::FutexWait(reg(&args[0])?, reg(&args[1])?)
            }
            "FUTEX_WAKE" => {
                arity(2)?;
                Instr::FutexWake(reg(&args[0])?, reg(&args[1])?)
            }
            "INB" => {
                arity(2)?;
                Instr::Inb(reg(&args[0])?, reg(&args[1])?)
            }
            "OUTB" => {
                arity(2)?;
                Instr::Outb(reg(&args[0])?, reg(&args[1])?)
            }
            "LOAD_UCODE" => {
                arity(2)?;
                Instr::LoadUcode(reg(&args[0])?, reg(&args[1])?)
            }
            "MSG_SEND" => {
                arity(3)?;
                Instr::MsgSend(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "OBJECT_CTL" | "EVENT_CTL" | "TIMER_CTL" => {
                arity(2)?;
                Instr::ObjectCtl(reg(&args[0])?, reg(&args[1])?)
            }
            "DMA_CTL" => {
                arity(2)?;
                Instr::DmaCtl(reg(&args[0])?, reg(&args[1])?)
            }
            "CAP_DUP" => {
                arity(2)?;
                Instr::CapDup(reg(&args[0])?, reg(&args[1])?)
            }
            "CAP_REVOKE" => {
                arity(2)?;
                Instr::CapRevoke(reg(&args[0])?, reg(&args[1])?)
            }
            "DOMAIN_CTL" => {
                arity(2)?;
                Instr::DomainCtl(reg(&args[0])?, reg(&args[1])?)
            }
            "CALL_CAP" => {
                arity(4)?;
                Instr::CallCap(
                    reg(&args[0])?,
                    fd(&args[1])?,
                    reg(&args[2])?,
                    reg(&args[3])?,
                )
            }
            "RET_CAP" => {
                arity(3)?;
                Instr::RetCap(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?)
            }
            "FADD" => fpu3(&args, Instr::FAdd)?,
            "FSUB" => fpu3(&args, Instr::FSub)?,
            "FMUL" => fpu3(&args, Instr::FMul)?,
            "FDIV" => fpu3(&args, Instr::FDiv)?,
            "VADD.32" => vec3(&args, Instr::VAdd32)?,
            _ => return Err(self.err(format!("unknown instruction {op_raw:?}"))),
        };
        Ok(instr)
    }

    fn err(&self, message: String) -> String {
        format!("line {}: {message}", self.line_no)
    }
}

fn alu3(args: &[String], ctor: fn(Reg, Reg, Reg) -> Instr) -> Result<Instr, String> {
    if args.len() != 3 {
        return Err(format!(
            "ALU instruction expects 3 operands, got {}",
            args.len()
        ));
    }
    Ok(ctor(reg(&args[0])?, reg(&args[1])?, reg(&args[2])?))
}

fn fpu3(args: &[String], ctor: fn(FReg, FReg, FReg) -> Instr) -> Result<Instr, String> {
    if args.len() != 3 {
        return Err(format!(
            "FPU instruction expects 3 operands, got {}",
            args.len()
        ));
    }
    Ok(ctor(freg(&args[0])?, freg(&args[1])?, freg(&args[2])?))
}

fn vec3(args: &[String], ctor: fn(VReg, VReg, VReg) -> Instr) -> Result<Instr, String> {
    if args.len() != 3 {
        return Err(format!(
            "vector instruction expects 3 operands, got {}",
            args.len()
        ));
    }
    Ok(ctor(vreg(&args[0])?, vreg(&args[1])?, vreg(&args[2])?))
}

fn branch(args: &[String], condition: Condition) -> Result<Instr, String> {
    if args.len() != 1 {
        return Err(format!("branch expects 1 operand, got {}", args.len()));
    }
    Ok(Instr::Branch(condition, target(&args[0])))
}

fn load(args: &[String], width: Width) -> Result<Instr, String> {
    if args.len() != 2 {
        return Err(format!("load expects 2 operands, got {}", args.len()));
    }
    Ok(Instr::Ld(reg(&args[0])?, mem(&args[1])?, width))
}

fn store(args: &[String], width: Width) -> Result<Instr, String> {
    if args.len() != 2 {
        return Err(format!("store expects 2 operands, got {}", args.len()));
    }
    Ok(Instr::St(mem(&args[0])?, reg(&args[1])?, width))
}

fn reg(text: &str) -> Result<Reg, String> {
    parse_reg(text)
}

fn fd(text: &str) -> Result<FdReg, String> {
    parse_fd(text)
}

fn freg(text: &str) -> Result<FReg, String> {
    parse_freg(text)
}

fn vreg(text: &str) -> Result<VReg, String> {
    parse_vreg(text)
}

fn value(text: &str) -> Value {
    parse_i64(text)
        .map(Value::Imm)
        .unwrap_or_else(|_| Value::Label(text.to_string()))
}

fn target(text: &str) -> Target {
    if let Ok(addr) = text.parse::<usize>() {
        Target::Address(addr)
    } else {
        Target::Label(text.to_string())
    }
}

fn mem(text: &str) -> Result<MemRef, String> {
    let trimmed = text.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return Ok(MemRef::Label(trimmed.to_string()));
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    let parts = split_operands(inner);
    match parts.as_slice() {
        [base] => Ok(MemRef::BaseOffset(reg(base)?, 0)),
        [base, offset] => Ok(MemRef::BaseOffset(reg(base)?, parse_i64(offset)?)),
        _ => Err(format!("invalid memory reference {text:?}")),
    }
}

fn strip_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut escaped = false;
    for (idx, ch) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' if in_string => escaped = true,
            '"' => in_string = !in_string,
            ';' | '#' if !in_string => return &line[..idx],
            _ => {}
        }
    }
    line
}

fn split_label(line: &str) -> Option<(&str, &str)> {
    let idx = line.find(':')?;
    let label = line[..idx].trim();
    if label.is_empty() || label.contains(char::is_whitespace) {
        return None;
    }
    Some((label, &line[idx + 1..]))
}

fn split_once_ws(line: &str) -> (&str, &str) {
    let trimmed = line.trim();
    if let Some(idx) = trimmed.find(char::is_whitespace) {
        (&trimmed[..idx], trimmed[idx..].trim())
    } else {
        (trimmed, "")
    }
}

pub fn split_operands(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut bracket_depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for ch in input.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' if in_string => {
                current.push(ch);
                escaped = true;
            }
            '"' => {
                in_string = !in_string;
                current.push(ch);
            }
            '[' if !in_string => {
                bracket_depth += 1;
                current.push(ch);
            }
            ']' if !in_string => {
                bracket_depth = bracket_depth.saturating_sub(1);
                current.push(ch);
            }
            ',' if !in_string && bracket_depth == 0 => {
                let item = current.trim();
                if !item.is_empty() {
                    out.push(item.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let item = current.trim();
    if !item.is_empty() {
        out.push(item.to_string());
    }
    out
}

fn validate_label(label: &str) -> Result<(), String> {
    let mut chars = label.chars();
    let Some(first) = chars.next() else {
        return Err("empty label".to_string());
    };
    if !(first == '_' || first == '.' || first.is_ascii_alphabetic()) {
        return Err(format!("invalid label {label:?}"));
    }
    if chars.any(|ch| !(ch == '_' || ch == '.' || ch.is_ascii_alphanumeric())) {
        return Err(format!("invalid label {label:?}"));
    }
    Ok(())
}

pub fn parse_i64(text: &str) -> Result<i64, String> {
    let text = text.trim();
    if let Some(hex) = text.strip_prefix("0x") {
        i64::from_str_radix(hex, 16).map_err(|_| format!("invalid integer {text:?}"))
    } else if let Some(hex) = text.strip_prefix("-0x") {
        i64::from_str_radix(hex, 16)
            .map(|v| -v)
            .map_err(|_| format!("invalid integer {text:?}"))
    } else {
        text.parse::<i64>()
            .map_err(|_| format!("invalid integer {text:?}"))
    }
}

pub fn parse_string(text: &str) -> Result<String, String> {
    let text = text.trim();
    if !text.starts_with('"') || !text.ends_with('"') {
        return Err(format!("expected string literal, got {text:?}"));
    }
    let inner = &text[1..text.len() - 1];
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        let Some(esc) = chars.next() else {
            return Err("unterminated escape sequence".to_string());
        };
        match esc {
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            '0' => out.push('\0'),
            '\\' => out.push('\\'),
            '"' => out.push('"'),
            other => return Err(format!("unsupported escape \\{other}")),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_data_and_text_labels() {
        let program = Program::parse(
            r#"
            .data
            msg: .string "ok\n"
            num: .quad 7
            .text
            start:
              LI r1, msg
              LD r2, num
              EXIT r0
            "#,
        )
        .unwrap();
        assert_eq!(program.labels["start"], 0);
        assert_eq!(program.data_labels["msg"], DATA_BASE);
        assert_eq!(program.instructions.len(), 3);
    }

    #[test]
    fn parses_object_ctl_profile_aliases() {
        let program = Program::parse(
            r#"
            .text
              OBJECT_CTL r1, r2
              EVENT_CTL r3, r4
              TIMER_CTL r5, r6
            "#,
        )
        .unwrap();
        assert!(matches!(
            program.instructions[0],
            Instr::ObjectCtl(Reg(1), Reg(2))
        ));
        assert!(matches!(
            program.instructions[1],
            Instr::ObjectCtl(Reg(3), Reg(4))
        ));
        assert!(matches!(
            program.instructions[2],
            Instr::ObjectCtl(Reg(5), Reg(6))
        ));
    }

    #[test]
    fn parses_dma_ctl_instruction() {
        let program = Program::parse(
            r#"
            .text
              DMA_CTL r1, r2
            "#,
        )
        .unwrap();
        assert!(matches!(
            program.instructions[0],
            Instr::DmaCtl(Reg(1), Reg(2))
        ));
    }

    #[test]
    fn parses_capability_control_instructions() {
        let program = Program::parse(
            r#"
            .text
              CAP_DUP r1, r2
              CAP_REVOKE r3, r4
            "#,
        )
        .unwrap();
        assert!(matches!(
            program.instructions[0],
            Instr::CapDup(Reg(1), Reg(2))
        ));
        assert!(matches!(
            program.instructions[1],
            Instr::CapRevoke(Reg(3), Reg(4))
        ));
    }

    #[test]
    fn parses_allocator_metadata_instructions() {
        let program = Program::parse(
            r#"
            .text
              ALLOC_EX r1, r2, r3
              ALLOC_SIZE r4, r5
            "#,
        )
        .unwrap();
        assert!(matches!(
            program.instructions[0],
            Instr::AllocEx(Reg(1), Reg(2), Reg(3))
        ));
        assert!(matches!(
            program.instructions[1],
            Instr::AllocSize(Reg(4), Reg(5))
        ));
    }

    #[test]
    fn parses_random_instruction() {
        let program = Program::parse(
            r#"
            .text
              RANDOM r1, r2, r3
            "#,
        )
        .unwrap();
        assert!(matches!(
            program.instructions[0],
            Instr::Random(Reg(1), Reg(2), Reg(3))
        ));
    }

    #[test]
    fn parses_thread_join_instruction() {
        let program = Program::parse(
            r#"
            .text
              THREAD_JOIN r1, r2, r3
            "#,
        )
        .unwrap();
        assert!(matches!(
            program.instructions[0],
            Instr::ThreadJoin(Reg(1), Reg(2), Reg(3))
        ));
    }

    #[test]
    fn parses_alarm_instruction() {
        let program = Program::parse(
            r#"
            .text
              ALARM r1, r2
            "#,
        )
        .unwrap();
        assert!(matches!(
            program.instructions[0],
            Instr::Alarm(Reg(1), Reg(2))
        ));
    }
}

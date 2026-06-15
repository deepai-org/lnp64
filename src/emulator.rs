use std::collections::{HashMap, VecDeque};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use crate::asm::Program;
use crate::isa::*;

const STACK_SIZE: u64 = 64 * 1024;
const CALL_FRAME_SIZE: u64 = 256;
const MMAP_BASE: u64 = 0x200_000;
const SIGCHLD: u64 = 17;
const SIGSEGV: u64 = 11;

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
    TcpListener {
        listener: TcpListener,
        pending: Option<TcpStream>,
    },
    Closed,
}

impl FdHandle {
    fn clone_handle(&self) -> Result<Self, String> {
        match self {
            FdHandle::Stdin => Ok(FdHandle::Stdin),
            FdHandle::Stdout => Ok(FdHandle::Stdout),
            FdHandle::Stderr => Ok(FdHandle::Stderr),
            FdHandle::File(file) => file
                .try_clone()
                .map(FdHandle::File)
                .map_err(|err| format!("failed to clone fd: {err}")),
            FdHandle::TcpListener { listener, pending } => Ok(FdHandle::TcpListener {
                listener: listener
                    .try_clone()
                    .map_err(|err| format!("failed to clone listener fd: {err}"))?,
                pending: match pending {
                    Some(stream) => Some(
                        stream
                            .try_clone()
                            .map_err(|err| format!("failed to clone pending stream: {err}"))?,
                    ),
                    None => None,
                },
            }),
            FdHandle::Closed => Ok(FdHandle::Closed),
        }
    }

    fn file_clone(&self) -> Result<Option<File>, String> {
        match self {
            FdHandle::File(file) => file
                .try_clone()
                .map(Some)
                .map_err(|err| format!("failed to clone file-backed fd: {err}")),
            _ => Ok(None),
        }
    }
}

struct Vma {
    start: u64,
    len: u64,
    prot: u64,
    file: Option<File>,
    file_offset: u64,
    resident: bool,
}

impl Vma {
    fn anonymous(start: u64, len: u64, prot: u64) -> Self {
        Self {
            start,
            len,
            prot,
            file: None,
            file_offset: 0,
            resident: true,
        }
    }

    fn contains(&self, addr: u64, len: usize) -> bool {
        let Some(end) = addr.checked_add(len as u64) else {
            return false;
        };
        addr >= self.start && end <= self.start + self.len
    }

    fn clone_vma(&self) -> Result<Self, String> {
        Ok(Self {
            start: self.start,
            len: self.len,
            prot: self.prot,
            file: match &self.file {
                Some(file) => Some(
                    file.try_clone()
                        .map_err(|err| format!("failed to clone VMA file: {err}"))?,
                ),
                None => None,
            },
            file_offset: self.file_offset,
            resident: self.resident,
        })
    }
}

struct Process {
    pid: u64,
    parent_pid: Option<u64>,
    program: Program,
    fds: Vec<FdHandle>,
    memory: Vec<u8>,
    vmas: Vec<Vma>,
    heap_next: u64,
    mmap_next: u64,
    allocations: HashMap<u64, usize>,
    uid: u64,
    gid: u64,
    sigmask: u64,
    signal_handlers: HashMap<u64, usize>,
    pending_signals: VecDeque<u64>,
    inbox: VecDeque<(u64, u64)>,
    ucode_ports: HashMap<u64, u8>,
}

impl Process {
    fn new(pid: u64, parent_pid: Option<u64>, program: Program) -> Self {
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

        let mut vmas = vec![
            Vma::anonymous(DATA_BASE, program.data.len().max(1) as u64, 0b11),
            Vma::anonymous(STACK_TOP - STACK_SIZE, STACK_SIZE, 0b11),
        ];
        vmas.sort_by_key(|vma| vma.start);

        Self {
            pid,
            parent_pid,
            program,
            fds,
            memory,
            vmas,
            heap_next: HEAP_BASE,
            mmap_next: MMAP_BASE,
            allocations: HashMap::new(),
            uid: if pid == 1 { 0 } else { 1000 },
            gid: if pid == 1 { 0 } else { 1000 },
            sigmask: 0,
            signal_handlers: HashMap::new(),
            pending_signals: VecDeque::new(),
            inbox: VecDeque::new(),
            ucode_ports: HashMap::new(),
        }
    }

    fn fork_clone(&self, pid: u64) -> Result<Self, String> {
        let mut fds = Vec::with_capacity(FDR_COUNT);
        for fd in &self.fds {
            fds.push(fd.clone_handle()?);
        }
        let mut vmas = Vec::with_capacity(self.vmas.len());
        for vma in &self.vmas {
            vmas.push(vma.clone_vma()?);
        }
        Ok(Self {
            pid,
            parent_pid: Some(self.pid),
            program: self.program.clone(),
            fds,
            memory: self.memory.clone(),
            vmas,
            heap_next: self.heap_next,
            mmap_next: self.mmap_next,
            allocations: self.allocations.clone(),
            uid: self.uid,
            gid: self.gid,
            sigmask: self.sigmask,
            signal_handlers: self.signal_handlers.clone(),
            pending_signals: VecDeque::new(),
            inbox: VecDeque::new(),
            ucode_ports: self.ucode_ports.clone(),
        })
    }

    fn exec(&mut self, program: Program) {
        let pid = self.pid;
        let parent_pid = self.parent_pid;
        let mut replacement = Process::new(pid, parent_pid, program);
        replacement.fds = std::mem::take(&mut self.fds);
        replacement.uid = self.uid;
        replacement.gid = self.gid;
        replacement.sigmask = self.sigmask;
        replacement.ucode_ports = std::mem::take(&mut self.ucode_ports);
        *self = replacement;
    }
}

#[derive(Clone)]
struct SavedSignalContext {
    ip: usize,
    regs: [u64; GPR_COUNT],
    flags: Flags,
}

#[derive(Clone)]
struct Thread {
    tid: u64,
    pid: u64,
    regs: [u64; GPR_COUNT],
    fregs: [u64; FPR_COUNT],
    vregs: [u128; VR_COUNT],
    ip: usize,
    flags: Flags,
    signal_stack: Vec<SavedSignalContext>,
}

impl Thread {
    fn new(tid: u64, pid: u64) -> Self {
        let mut regs = [0; GPR_COUNT];
        regs[31] = STACK_TOP - CALL_FRAME_SIZE;
        Self {
            tid,
            pid,
            regs,
            fregs: [0; FPR_COUNT],
            vregs: [0; VR_COUNT],
            ip: 0,
            flags: Flags::default(),
            signal_stack: Vec::new(),
        }
    }
}

pub struct Machine {
    processes: HashMap<u64, Process>,
    threads: HashMap<u64, Thread>,
    ready: VecDeque<u64>,
    sleepers: Vec<(u64, u64)>,
    futex_waiters: HashMap<u64, VecDeque<u64>>,
    fd_waiters: Vec<(u64, usize)>,
    current_tid: u64,
    next_pid: u64,
    next_tid: u64,
    last_exit: i32,
}

impl Machine {
    pub fn new(program: Program) -> Self {
        let root_pid = 1;
        let root_tid = 1;
        let process = Process::new(root_pid, None, program);
        let thread = Thread::new(root_tid, root_pid);

        let mut processes = HashMap::new();
        processes.insert(root_pid, process);
        let mut threads = HashMap::new();
        threads.insert(root_tid, thread);

        let mut ready = VecDeque::new();
        ready.push_back(root_tid);

        Self {
            processes,
            threads,
            ready,
            sleepers: Vec::new(),
            futex_waiters: HashMap::new(),
            fd_waiters: Vec::new(),
            current_tid: root_tid,
            next_pid: 2,
            next_tid: 2,
            last_exit: 0,
        }
    }

    pub fn run(&mut self) -> Result<i32, String> {
        let mut steps = 0usize;
        while !self.threads.is_empty() {
            if steps > 10_000_000 {
                return Err("execution step limit exceeded".to_string());
            }
            steps += 1;
            self.tick_sleepers();
            self.poll_fd_waiters();

            let Some(tid) = self.ready.pop_front() else {
                if self.sleepers.is_empty() && self.fd_waiters.is_empty() {
                    return Err("hardware runqueue deadlock: no ready threads".to_string());
                }
                if !self.fd_waiters.is_empty() {
                    thread::sleep(Duration::from_millis(10));
                }
                continue;
            };
            if !self.threads.contains_key(&tid) {
                continue;
            }
            self.current_tid = tid;
            self.deliver_signal_if_needed()?;
            if !self.threads.contains_key(&tid) {
                continue;
            }

            let instr = {
                let thread = self.thread()?;
                let process = self
                    .processes
                    .get(&thread.pid)
                    .ok_or_else(|| format!("missing process {}", thread.pid))?;
                let Some(instr) = process.program.instructions.get(thread.ip).cloned() else {
                    self.exit_current(0)?;
                    continue;
                };
                instr
            };
            self.thread_mut()?.ip += 1;
            let keep_ready = self.exec(instr)?;
            if keep_ready && self.threads.contains_key(&tid) {
                self.wake_thread(tid);
            }
        }
        Ok(self.last_exit)
    }

    fn exec(&mut self, instr: Instr) -> Result<bool, String> {
        match instr {
            Instr::Nop | Instr::Fence => {}
            Instr::Li(dst, value) => {
                let v = self.resolve_value(value)?;
                self.write_reg(dst, v)?;
            }
            Instr::Mov(dst, src) => self.write_reg(dst, self.read_reg(src)?)?,
            Instr::Add(dst, a, b) => {
                self.write_reg(dst, self.read_reg(a)?.wrapping_add(self.read_reg(b)?))?
            }
            Instr::Sub(dst, a, b) => {
                self.write_reg(dst, self.read_reg(a)?.wrapping_sub(self.read_reg(b)?))?
            }
            Instr::Mul(dst, a, b) => {
                self.write_reg(dst, self.read_reg(a)?.wrapping_mul(self.read_reg(b)?))?
            }
            Instr::Div(dst, a, b) => {
                let divisor = self.read_reg(b)?;
                if divisor == 0 {
                    self.raise_current_signal(8)?;
                    return Ok(true);
                }
                self.write_reg(dst, self.read_reg(a)? / divisor)?;
            }
            Instr::And(dst, a, b) => self.write_reg(dst, self.read_reg(a)? & self.read_reg(b)?)?,
            Instr::Or(dst, a, b) => self.write_reg(dst, self.read_reg(a)? | self.read_reg(b)?)?,
            Instr::Xor(dst, a, b) => self.write_reg(dst, self.read_reg(a)? ^ self.read_reg(b)?)?,
            Instr::Not(dst, src) => self.write_reg(dst, !self.read_reg(src)?)?,
            Instr::Lsl(dst, a, b) => {
                self.write_reg(dst, self.read_reg(a)? << (self.read_reg(b)? & 63))?
            }
            Instr::Lsr(dst, a, b) => {
                self.write_reg(dst, self.read_reg(a)? >> (self.read_reg(b)? & 63))?
            }
            Instr::Asr(dst, a, b) => self.write_reg(
                dst,
                ((self.read_reg(a)? as i64) >> (self.read_reg(b)? & 63)) as u64,
            )?,
            Instr::Cmp(a, b) => {
                let lhs = self.read_reg(a)? as i64;
                let rhs = self.read_reg(b)? as i64;
                self.thread_mut()?.flags = Flags {
                    zero: lhs == rhs,
                    negative: lhs < rhs,
                    greater: lhs > rhs,
                };
            }
            Instr::Jmp(target) => {
                let ip = self.resolve_target(target)?;
                self.thread_mut()?.ip = ip;
            }
            Instr::Branch(condition, target) => {
                if self.condition(condition)? {
                    let ip = self.resolve_target(target)?;
                    self.thread_mut()?.ip = ip;
                }
            }
            Instr::Call(target) => {
                let ip = self.resolve_target(target)?;
                let ret = self.thread()?.ip as u64;
                let sp = self.thread()?.regs[31].wrapping_sub(CALL_FRAME_SIZE);
                self.thread_mut()?.regs[31] = sp;
                self.store_u64(sp, ret)?;
                self.thread_mut()?.ip = ip;
            }
            Instr::Ret => {
                let sp = self.thread()?.regs[31];
                let next = self.load_u64(sp)?;
                self.thread_mut()?.regs[31] = sp.wrapping_add(CALL_FRAME_SIZE);
                self.thread_mut()?.ip = next as usize;
            }
            Instr::Ld(dst, mem, width) => {
                let addr = self.resolve_mem(mem)?;
                let value = self.load_width(addr, width)?;
                self.write_reg(dst, value)?;
            }
            Instr::St(mem, src, width) => {
                let addr = self.resolve_mem(mem)?;
                self.store_width(addr, self.read_reg(src)?, width)?;
            }
            Instr::Alloc(dst, bytes_reg) => {
                let len = (self.read_reg(bytes_reg)? as usize).max(1);
                let addr = {
                    let process = self.process_mut()?;
                    let addr = align_up(process.heap_next, 64);
                    let end = addr
                        .checked_add(len as u64)
                        .ok_or_else(|| "allocation overflow".to_string())?;
                    if end as usize >= process.memory.len() {
                        return Err(format!("out of silicon heap memory allocating {len} bytes"));
                    }
                    process.heap_next = end;
                    process.allocations.insert(addr, len);
                    process.vmas.push(Vma::anonymous(addr, len as u64, 0b11));
                    addr
                };
                self.write_reg(dst, addr)?;
            }
            Instr::Free(ptr) => {
                let ptr = self.read_reg(ptr)?;
                let process = self.process_mut()?;
                process.allocations.remove(&ptr);
                process.vmas.retain(|vma| vma.start != ptr);
            }
            Instr::OpenFd(dst, path_reg, flags_reg) => {
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let flags = self.read_reg(flags_reg)?;
                if let Some(addr) = path.strip_prefix("tcp-listen:") {
                    let listener = TcpListener::bind(addr)
                        .map_err(|err| format!("OPEN_FD TCP listener {addr:?}: {err}"))?;
                    listener
                        .set_nonblocking(true)
                        .map_err(|err| format!("OPEN_FD TCP nonblocking {addr:?}: {err}"))?;
                    self.process_mut()?.fds[dst.0] = FdHandle::TcpListener {
                        listener,
                        pending: None,
                    };
                } else {
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
                    self.process_mut()?.fds[dst.0] = FdHandle::File(file);
                }
            }
            Instr::ReadFd(fd, buf, len) => {
                let addr = self.read_reg(buf)?;
                let len = self.read_reg(len)? as usize;
                let mut tmp = vec![0; len];
                let count = match &mut self.process_mut()?.fds[fd.0] {
                    FdHandle::Stdin => io::stdin()
                        .read(&mut tmp)
                        .map_err(|err| format!("READ_FD fd0: {err}"))?,
                    FdHandle::File(file) => file
                        .read(&mut tmp)
                        .map_err(|err| format!("READ_FD fd{}: {err}", fd.0))?,
                    FdHandle::TcpListener { listener, pending } => {
                        if pending.is_none() {
                            match listener.accept() {
                                Ok((stream, _)) => {
                                    stream.set_nonblocking(false).map_err(|err| {
                                        format!("READ_FD fd{} stream blocking: {err}", fd.0)
                                    })?;
                                    *pending = Some(stream);
                                }
                                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {}
                                Err(err) => {
                                    return Err(format!("READ_FD fd{} accept: {err}", fd.0));
                                }
                            };
                        }
                        if let Some(stream) = pending {
                            stream
                                .read(&mut tmp)
                                .map_err(|err| format!("READ_FD fd{} stream: {err}", fd.0))?
                        } else {
                            0
                        }
                    }
                    FdHandle::Stdout | FdHandle::Stderr | FdHandle::Closed => 0,
                };
                self.write_bytes(addr, &tmp[..count])?;
                self.write_reg(Reg(1), count as u64)?;
            }
            Instr::WriteFd(fd, buf, len) => {
                let data = self.read_bytes(self.read_reg(buf)?, self.read_reg(len)? as usize)?;
                match &mut self.process_mut()?.fds[fd.0] {
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
                    FdHandle::TcpListener { pending, .. } => {
                        let Some(stream) = pending else {
                            return Err(format!("WRITE_FD fd{} has no accepted stream", fd.0));
                        };
                        stream
                            .write_all(&data)
                            .map_err(|err| format!("WRITE_FD fd{} stream: {err}", fd.0))?;
                    }
                    FdHandle::Stdin | FdHandle::Closed => {
                        return Err(format!("WRITE_FD on non-writable fd{}", fd.0));
                    }
                }
            }
            Instr::WaitOnFd(fd, _) => {
                if !self.fd_ready(fd.0)? {
                    self.fd_waiters.push((self.current_tid, fd.0));
                    self.ready.retain(|tid| *tid != self.current_tid);
                    return Ok(false);
                }
            }
            Instr::FdDup(dst, src) => {
                let cloned = self.process()?.fds[src.0].clone_handle()?;
                self.process_mut()?.fds[dst.0] = cloned;
            }
            Instr::GetPcr(dst, pcr) => {
                let value = self.read_pcr(pcr)?;
                self.write_reg(dst, value)?;
            }
            Instr::SetPcr(pcr, src) => self.write_pcr(pcr, self.read_reg(src)?)?,
            Instr::Fork(dst) => {
                let child_pid = self.next_pid;
                self.next_pid += 1;
                let child_tid = self.next_tid;
                self.next_tid += 1;

                let parent_pid = self.thread()?.pid;
                let child_process = self.process()?.fork_clone(child_pid)?;
                let mut child_thread = self.thread()?.clone();
                child_thread.pid = child_pid;
                child_thread.tid = child_tid;
                if dst.0 != 0 && dst.0 != 31 {
                    child_thread.regs[dst.0] = 0;
                }
                self.processes.insert(child_pid, child_process);
                self.threads.insert(child_tid, child_thread);
                self.ready.push_back(child_tid);
                self.write_reg(dst, child_pid)?;
                let _ = parent_pid;
            }
            Instr::Exec(path_reg, _argv_reg) => {
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let source = fs::read_to_string(&path)
                    .map_err(|err| format!("EXEC failed to read {path:?}: {err}"))?;
                let program = Program::parse(&source)
                    .map_err(|err| format!("EXEC failed to assemble {path:?}: {err}"))?;
                self.process_mut()?.exec(program);
                let pid = self.thread()?.pid;
                let tid = self.thread()?.tid;
                *self.thread_mut()? = Thread::new(tid, pid);
            }
            Instr::Spawn(dst, entry) => {
                let tid = self.next_tid;
                self.next_tid += 1;
                let mut child = self.thread()?.clone();
                child.tid = tid;
                child.ip = self.read_reg(entry)? as usize;
                child.regs[31] = STACK_TOP - CALL_FRAME_SIZE - ((tid - 1) * 4096);
                self.threads.insert(tid, child);
                self.ready.push_back(tid);
                self.write_reg(dst, tid)?;
            }
            Instr::Yield => return Ok(true),
            Instr::Sleep(ticks_reg) => {
                let ticks = self.read_reg(ticks_reg)?.max(1);
                self.sleepers.push((self.current_tid, ticks));
                self.ready.retain(|tid| *tid != self.current_tid);
                return Ok(false);
            }
            Instr::Exit(code) => {
                let code = self.read_reg(code)? as i32;
                self.exit_current(code)?;
                return Ok(false);
            }
            Instr::Mmap(dst, hint, len, prot, fd, offset) => {
                let len = self.read_reg(len)?.max(1);
                let prot = self.read_reg(prot)?;
                let hint = self.read_reg(hint)?;
                let offset = self.read_reg(offset)?;
                let file = self.process()?.fds[fd.0].file_clone()?;
                let addr = {
                    let process = self.process_mut()?;
                    let addr = if hint != 0 {
                        hint
                    } else {
                        align_up(process.mmap_next, 4096)
                    };
                    let end = addr
                        .checked_add(len)
                        .ok_or_else(|| "MMAP range overflow".to_string())?;
                    if end as usize >= process.memory.len() {
                        return Err(format!("MMAP out of range: 0x{addr:x} + {len}"));
                    }
                    process.mmap_next = end;
                    process.vmas.push(Vma {
                        start: addr,
                        len,
                        prot,
                        file,
                        file_offset: offset,
                        resident: false,
                    });
                    addr
                };
                self.write_reg(dst, addr)?;
            }
            Instr::Munmap(addr, _len) => {
                let addr = self.read_reg(addr)?;
                self.process_mut()?.vmas.retain(|vma| vma.start != addr);
            }
            Instr::Sigaction(signum, handler) => {
                let signum = self.read_reg(signum)?;
                let handler = self.read_reg(handler)? as usize;
                self.process_mut()?.signal_handlers.insert(signum, handler);
            }
            Instr::SigmaskSet(mask) => {
                let mask = self.read_reg(mask)?;
                self.process_mut()?.sigmask = mask;
            }
            Instr::Kill(pid, signum) => {
                let pid = self.read_reg(pid)?;
                let signum = self.read_reg(signum)?;
                if let Some(process) = self.processes.get_mut(&pid) {
                    process.pending_signals.push_back(signum);
                    if let Some(tid) = self
                        .threads
                        .values()
                        .find(|thread| thread.pid == pid)
                        .map(|thread| thread.tid)
                    {
                        self.wake_thread(tid);
                    }
                }
            }
            Instr::Sigret => {
                let saved = self
                    .thread_mut()?
                    .signal_stack
                    .pop()
                    .ok_or_else(|| "SIGRET with empty signal stack".to_string())?;
                let thread = self.thread_mut()?;
                thread.ip = saved.ip;
                thread.regs = saved.regs;
                thread.flags = saved.flags;
            }
            Instr::LockCmpxchg(dst, addr_reg, expected, new_value) => {
                let addr = self.read_reg(addr_reg)?;
                let current = self.load_u64(addr)?;
                if current == self.read_reg(expected)? {
                    self.store_u64(addr, self.read_reg(new_value)?)?;
                }
                self.write_reg(dst, current)?;
            }
            Instr::FutexWait(addr_reg, expected_reg) => {
                let addr = self.read_reg(addr_reg)?;
                let expected = self.read_reg(expected_reg)?;
                if self.load_u64(addr)? == expected {
                    self.futex_waiters
                        .entry(addr)
                        .or_default()
                        .push_back(self.current_tid);
                    self.ready.retain(|tid| *tid != self.current_tid);
                    return Ok(false);
                }
            }
            Instr::FutexWake(addr_reg, count_reg) => {
                let addr = self.read_reg(addr_reg)?;
                let count = self.read_reg(count_reg)?;
                let mut to_wake = Vec::new();
                if let Some(waiters) = self.futex_waiters.get_mut(&addr) {
                    for _ in 0..count {
                        let Some(tid) = waiters.pop_front() else {
                            break;
                        };
                        to_wake.push(tid);
                    }
                }
                for tid in to_wake {
                    self.wake_thread(tid);
                }
            }
            Instr::Inb(dst, port) => {
                let value = self
                    .process()?
                    .ucode_ports
                    .get(&self.read_reg(port)?)
                    .copied()
                    .unwrap_or(0);
                self.write_reg(dst, value as u64)?;
            }
            Instr::Outb(port, src) => {
                let port = self.read_reg(port)?;
                let value = self.read_reg(src)? as u8;
                self.process_mut()?.ucode_ports.insert(port, value);
            }
            Instr::LoadUcode(buf, len) => {
                if self.process()?.uid != 0 {
                    self.raise_current_signal(SIGSEGV)?;
                    return Ok(true);
                }
                let blob = self.read_bytes(self.read_reg(buf)?, self.read_reg(len)? as usize)?;
                self.load_microcode(&blob)?;
            }
            Instr::MsgSend(pid, v1, v2) => {
                let pid = self.read_reg(pid)?;
                let msg = (self.read_reg(v1)?, self.read_reg(v2)?);
                if let Some(process) = self.processes.get_mut(&pid) {
                    process.inbox.push_back(msg);
                    if let Some(tid) = self
                        .threads
                        .values()
                        .find(|thread| thread.pid == pid)
                        .map(|thread| thread.tid)
                    {
                        self.wake_thread(tid);
                    }
                }
            }
            Instr::MsgRecv(dst1, dst2) => {
                let Some((v1, v2)) = self.process_mut()?.inbox.pop_front() else {
                    self.thread_mut()?.ip = self.thread()?.ip.saturating_sub(1);
                    self.ready.retain(|tid| *tid != self.current_tid);
                    return Ok(false);
                };
                self.write_reg(dst1, v1)?;
                self.write_reg(dst2, v2)?;
            }
            Instr::FAdd(dst, a, b) => {
                self.write_freg(dst, self.read_f64(a)? + self.read_f64(b)?)?
            }
            Instr::FSub(dst, a, b) => {
                self.write_freg(dst, self.read_f64(a)? - self.read_f64(b)?)?
            }
            Instr::FMul(dst, a, b) => {
                self.write_freg(dst, self.read_f64(a)? * self.read_f64(b)?)?
            }
            Instr::FDiv(dst, a, b) => {
                self.write_freg(dst, self.read_f64(a)? / self.read_f64(b)?)?
            }
            Instr::VAdd32(dst, a, b) => {
                let lhs = self.thread()?.vregs[a.0];
                let rhs = self.thread()?.vregs[b.0];
                let mut lanes = [0u32; 4];
                for (idx, lane) in lanes.iter_mut().enumerate() {
                    let shift = idx * 32;
                    let l = ((lhs >> shift) & 0xffff_ffff) as u32;
                    let r = ((rhs >> shift) & 0xffff_ffff) as u32;
                    *lane = l.wrapping_add(r);
                }
                let packed = lanes.iter().enumerate().fold(0u128, |acc, (idx, lane)| {
                    acc | ((*lane as u128) << (idx * 32))
                });
                self.thread_mut()?.vregs[dst.0] = packed;
            }
        }
        Ok(true)
    }

    fn thread(&self) -> Result<&Thread, String> {
        self.threads
            .get(&self.current_tid)
            .ok_or_else(|| format!("missing current thread {}", self.current_tid))
    }

    fn thread_mut(&mut self) -> Result<&mut Thread, String> {
        self.threads
            .get_mut(&self.current_tid)
            .ok_or_else(|| format!("missing current thread {}", self.current_tid))
    }

    fn process(&self) -> Result<&Process, String> {
        let pid = self.thread()?.pid;
        self.processes
            .get(&pid)
            .ok_or_else(|| format!("missing process {pid}"))
    }

    fn process_mut(&mut self) -> Result<&mut Process, String> {
        let pid = self.thread()?.pid;
        self.processes
            .get_mut(&pid)
            .ok_or_else(|| format!("missing process {pid}"))
    }

    fn read_reg(&self, reg: Reg) -> Result<u64, String> {
        Ok(if reg.0 == 0 {
            0
        } else {
            self.thread()?.regs[reg.0]
        })
    }

    fn write_reg(&mut self, reg: Reg, value: u64) -> Result<(), String> {
        if reg.0 != 0 && reg.0 != 31 {
            self.thread_mut()?.regs[reg.0] = value;
        }
        Ok(())
    }

    fn read_f64(&self, reg: FReg) -> Result<f64, String> {
        Ok(f64::from_bits(self.thread()?.fregs[reg.0]))
    }

    fn write_freg(&mut self, reg: FReg, value: f64) -> Result<(), String> {
        self.thread_mut()?.fregs[reg.0] = value.to_bits();
        Ok(())
    }

    fn condition(&self, condition: Condition) -> Result<bool, String> {
        let flags = self.thread()?.flags;
        Ok(match condition {
            Condition::Eq => flags.zero,
            Condition::Ne => !flags.zero,
            Condition::Lt => flags.negative,
            Condition::Gt => flags.greater,
            Condition::Le => flags.zero || flags.negative,
            Condition::Ge => flags.zero || flags.greater,
        })
    }

    fn resolve_value(&self, value: Value) -> Result<u64, String> {
        match value {
            Value::Imm(v) => Ok(v as u64),
            Value::Label(label) => {
                if let Some(addr) = self.process()?.program.data_labels.get(&label) {
                    Ok(*addr)
                } else if let Some(ip) = self.process()?.program.labels.get(&label) {
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
                .process()?
                .program
                .labels
                .get(&label)
                .copied()
                .ok_or_else(|| format!("unknown code label {label:?}")),
        }
    }

    fn resolve_mem(&self, mem: MemRef) -> Result<u64, String> {
        match mem {
            MemRef::BaseOffset(base, offset) => {
                Ok(self.read_reg(base)?.wrapping_add(offset as u64))
            }
            MemRef::Label(label) => self
                .process()?
                .program
                .data_labels
                .get(&label)
                .copied()
                .ok_or_else(|| format!("unknown data label {label:?}")),
        }
    }

    fn load_width(&mut self, addr: u64, width: Width) -> Result<u64, String> {
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

    fn load_u64(&mut self, addr: u64) -> Result<u64, String> {
        self.load_width(addr, Width::Double)
    }

    fn store_u64(&mut self, addr: u64, value: u64) -> Result<(), String> {
        self.store_width(addr, value, Width::Double)
    }

    fn read_bytes(&mut self, addr: u64, len: usize) -> Result<Vec<u8>, String> {
        self.ensure_mapped(addr, len, false)?;
        let process = self.process()?;
        let start = addr as usize;
        let end = start
            .checked_add(len)
            .ok_or_else(|| format!("memory range overflow at 0x{addr:x}"))?;
        Ok(process.memory[start..end].to_vec())
    }

    fn write_bytes(&mut self, addr: u64, data: &[u8]) -> Result<(), String> {
        self.ensure_mapped(addr, data.len(), true)?;
        let process = self.process_mut()?;
        let start = addr as usize;
        let end = start
            .checked_add(data.len())
            .ok_or_else(|| format!("memory range overflow at 0x{addr:x}"))?;
        process.memory[start..end].copy_from_slice(data);
        Ok(())
    }

    fn ensure_mapped(&mut self, addr: u64, len: usize, write: bool) -> Result<(), String> {
        let process = self.process_mut()?;
        let idx = process
            .vmas
            .iter()
            .position(|vma| vma.contains(addr, len))
            .ok_or_else(|| format!("hardware SIGSEGV: unmapped address 0x{addr:x} + {len}"))?;
        if write && process.vmas[idx].prot & 0b10 == 0 {
            return Err(format!("hardware SIGSEGV: write denied at 0x{addr:x}"));
        }
        if !write && process.vmas[idx].prot & 0b01 == 0 {
            return Err(format!("hardware SIGSEGV: read denied at 0x{addr:x}"));
        }
        if !process.vmas[idx].resident {
            let (start, vma_len, file_offset) = {
                let vma = &process.vmas[idx];
                (vma.start, vma.len, vma.file_offset)
            };
            if let Some(file) = &mut process.vmas[idx].file {
                file.seek(SeekFrom::Start(file_offset))
                    .map_err(|err| format!("file-backed VMA seek failed: {err}"))?;
                let mut tmp = vec![0; vma_len as usize];
                let count = file
                    .read(&mut tmp)
                    .map_err(|err| format!("file-backed VMA page-in failed: {err}"))?;
                let start = start as usize;
                process.memory[start..start + count].copy_from_slice(&tmp[..count]);
            }
            process.vmas[idx].resident = true;
        }
        Ok(())
    }

    fn read_c_string(&mut self, addr: u64) -> Result<String, String> {
        let mut bytes = Vec::new();
        let mut pos = addr;
        loop {
            let byte = self.load_width(pos, Width::Byte)? as u8;
            if byte == 0 {
                break;
            }
            bytes.push(byte);
            pos += 1;
        }
        String::from_utf8(bytes).map_err(|err| format!("invalid utf-8 string at 0x{addr:x}: {err}"))
    }

    fn read_pcr(&self, pcr: Pcr) -> Result<u64, String> {
        let process = self.process()?;
        Ok(match pcr {
            Pcr::Pid => process.pid,
            Pcr::Tid => self.thread()?.tid,
            Pcr::Uid => process.uid,
            Pcr::Gid => process.gid,
            Pcr::Sigmask => process.sigmask,
        })
    }

    fn write_pcr(&mut self, pcr: Pcr, value: u64) -> Result<(), String> {
        let process = self.process_mut()?;
        match pcr {
            Pcr::Pid | Pcr::Tid => Err("PID and TID PCRs are read-only".to_string()),
            Pcr::Uid if process.uid != 0 => {
                Err("SET_PCR UID denied: current UID is not 0".to_string())
            }
            Pcr::Uid => {
                process.uid = value;
                Ok(())
            }
            Pcr::Gid => {
                process.gid = value;
                Ok(())
            }
            Pcr::Sigmask => {
                process.sigmask = value;
                Ok(())
            }
        }
    }

    fn exit_current(&mut self, code: i32) -> Result<(), String> {
        let tid = self.current_tid;
        let pid = self.thread()?.pid;
        let parent_pid = self.process()?.parent_pid;
        self.threads.remove(&tid);
        self.last_exit = code;
        if !self.threads.values().any(|thread| thread.pid == pid) {
            self.processes.remove(&pid);
            if let Some(parent_pid) = parent_pid {
                if let Some(parent) = self.processes.get_mut(&parent_pid) {
                    parent.pending_signals.push_back(SIGCHLD);
                }
            }
        }
        Ok(())
    }

    fn wake_thread(&mut self, tid: u64) {
        if self.threads.contains_key(&tid) && !self.ready.contains(&tid) {
            self.sleepers.retain(|(sleep_tid, _)| *sleep_tid != tid);
            self.ready.push_back(tid);
        }
    }

    fn tick_sleepers(&mut self) {
        let mut woke = Vec::new();
        for (tid, ticks) in &mut self.sleepers {
            *ticks = ticks.saturating_sub(1);
            if *ticks == 0 {
                woke.push(*tid);
            }
        }
        self.sleepers.retain(|(_, ticks)| *ticks != 0);
        for tid in woke {
            self.wake_thread(tid);
        }
    }

    fn poll_fd_waiters(&mut self) {
        let waiters = std::mem::take(&mut self.fd_waiters);
        for (tid, fd) in waiters {
            let ready = self
                .with_thread_process(tid, |machine| machine.fd_ready(fd))
                .unwrap_or(false);
            if ready {
                self.wake_thread(tid);
            } else if self.threads.contains_key(&tid) {
                self.fd_waiters.push((tid, fd));
            }
        }
    }

    fn with_thread_process<T>(
        &mut self,
        tid: u64,
        f: impl FnOnce(&mut Self) -> Result<T, String>,
    ) -> Result<T, String> {
        let saved = self.current_tid;
        self.current_tid = tid;
        let result = f(self);
        self.current_tid = saved;
        result
    }

    fn fd_ready(&mut self, fd: usize) -> Result<bool, String> {
        let handle = &mut self.process_mut()?.fds[fd];
        match handle {
            FdHandle::Stdin | FdHandle::Stdout | FdHandle::Stderr | FdHandle::File(_) => Ok(true),
            FdHandle::Closed => Ok(false),
            FdHandle::TcpListener { listener, pending } => {
                if pending.is_some() {
                    return Ok(true);
                }
                match listener.accept() {
                    Ok((stream, _)) => {
                        stream
                            .set_nonblocking(false)
                            .map_err(|err| format!("TCP accepted stream blocking failed: {err}"))?;
                        *pending = Some(stream);
                        Ok(true)
                    }
                    Err(err) if err.kind() == io::ErrorKind::WouldBlock => Ok(false),
                    Err(err) => Err(format!("TCP accept failed: {err}")),
                }
            }
        }
    }

    fn raise_current_signal(&mut self, signum: u64) -> Result<(), String> {
        self.process_mut()?.pending_signals.push_back(signum);
        Ok(())
    }

    fn deliver_signal_if_needed(&mut self) -> Result<(), String> {
        let pid = self.thread()?.pid;
        let signum = {
            let Some(process) = self.processes.get_mut(&pid) else {
                return Ok(());
            };
            let Some(pos) = process
                .pending_signals
                .iter()
                .position(|sig| process.sigmask & (1u64 << sig.min(&63)) == 0)
            else {
                return Ok(());
            };
            process.pending_signals.remove(pos)
        };
        let Some(signum) = signum else {
            return Ok(());
        };
        let handler = self.process()?.signal_handlers.get(&signum).copied();
        if let Some(handler) = handler {
            let saved = {
                let thread = self.thread()?;
                SavedSignalContext {
                    ip: thread.ip,
                    regs: thread.regs,
                    flags: thread.flags,
                }
            };
            let thread = self.thread_mut()?;
            thread.signal_stack.push(saved);
            thread.ip = handler;
        } else {
            if signum != SIGCHLD {
                self.exit_current(128 + signum as i32)?;
            }
        }
        Ok(())
    }

    fn load_microcode(&mut self, blob: &[u8]) -> Result<(), String> {
        let text = String::from_utf8_lossy(blob);
        for line in text.lines() {
            let parts = line.split_whitespace().collect::<Vec<_>>();
            if parts.is_empty() {
                continue;
            }
            if parts.len() != 3 || !parts[0].eq_ignore_ascii_case("PORT") {
                return Err(format!("invalid microcode directive {line:?}"));
            }
            let port = parse_num(parts[1])?;
            let value = parse_num(parts[2])?;
            if value > 255 {
                return Err(format!("microcode port value out of range: {value}"));
            }
            self.process_mut()?.ucode_ports.insert(port, value as u8);
        }
        Ok(())
    }
}

fn align_up(value: u64, align: u64) -> u64 {
    (value + align - 1) & !(align - 1)
}

fn parse_num(text: &str) -> Result<u64, String> {
    if let Some(hex) = text.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).map_err(|_| format!("invalid number {text:?}"))
    } else {
        text.parse::<u64>()
            .map_err(|_| format!("invalid number {text:?}"))
    }
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

    #[test]
    fn runs_system_primitive_subset() {
        let program = Program::parse(
            r#"
            .text
              GET_PCR r1, PID
              LI r2, 1
              CMP r1, r2
              BNE bad

              LI r3, 16
              ALLOC r4, r3
              CMP r4, r0
              BEQ bad

              LI r5, 41
              ST [r4, 0], r5
              LI r6, 41
              LI r7, 42
              LOCK.CMPXCHG r8, r4, r6, r7
              LD r9, [r4, 0]
              CMP r9, r7
              BNE bad

              MSG_SEND r1, r6, r7
              MSG_RECV r10, r11
              CMP r10, r6
              BNE bad
              CMP r11, r7
              BNE bad
              FREE r4
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

    #[test]
    fn fork_exec_spawn_signal_futex_mmap_and_microcode_execute() {
        let exec_path = "/tmp/lnp64_exec_test.s";
        fs::write(
            exec_path,
            r#"
            .text
              LI r1, 0
              EXIT r1
            "#,
        )
        .unwrap();

        let program = Program::parse(&format!(
            r#"
            .data
            exec_path: .string "{exec_path}"
            ucode: .string "PORT 9 123\n"
            .text
              LI r1, handler
              LI r2, 10
              SIGACTION r2, r1
              GET_PCR r3, PID
              KILL r3, r2
              YIELD
              LD r20, sig_flag
              LI r4, 1
              CMP r20, r4
              LI r1, 2
              BNE bad

              LI r5, 16
              LI r25, 3
              MMAP r6, r0, r5, r25, fd0, r0
              LI r7, 99
              ST [r6, 0], r7
              LD r8, [r6, 0]
              CMP r8, r7
              LI r1, 3
              BNE bad

              LI r9, ucode
              LI r10, 11
              LOAD_UCODE r9, r10
              LI r11, 9
              INB r12, r11
              LI r13, 123
              CMP r12, r13
              LI r1, 4
              BNE bad

              LI r14, futex_word
              LI r15, 0
              LI r16, waiter
              SPAWN r17, r16
              YIELD
              LI r18, 1
              ST [r14, 0], r18
              FUTEX_WAKE r14, r18
              LI r26, 3
              SLEEP r26
              LD r19, [r14, 0]
              LI r21, 2
              CMP r19, r21
              MOV r1, r19
              BNE bad

              FORK r22
              CMP r22, r0
              BEQ child
              YIELD
              LI r23, exec_path
              EXEC r23, r0
            child:
              LI r24, 0
              EXIT r24

            waiter:
              FUTEX_WAIT r14, r15
              LI r18, 2
              ST [r14, 0], r18
              EXIT r0

            handler:
              LI r20, 1
              ST sig_flag, r20
              SIGRET

            bad:
              EXIT r1

            .data
            futex_word: .quad 0
            sig_flag: .quad 0
            "#
        ))
        .unwrap();
        let mut machine = Machine::new(program);
        assert_eq!(machine.run().unwrap(), 0);
        let _ = fs::remove_file(exec_path);
    }

    #[test]
    fn uses_dedicated_fpu_and_vector_register_files() {
        let program = Program::parse(
            r#"
            .text
              FADD f3, f1, f2
              VADD.32 v3, v1, v2
              EXIT r0
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        {
            let thread = machine.threads.get_mut(&1).unwrap();
            thread.fregs[1] = 1.5f64.to_bits();
            thread.fregs[2] = 2.25f64.to_bits();
            thread.vregs[1] = 1 | (2 << 32) | (3 << 64) | (4 << 96);
            thread.vregs[2] = 10 | (20 << 32) | (30 << 64) | (40 << 96);
        }
        assert_eq!(machine.run().unwrap(), 0);
        let thread = machine.threads.get(&1);
        assert!(thread.is_none(), "thread exits after verification run");

        let program = Program::parse(
            r#"
            .text
              FADD f3, f1, f2
              VADD.32 v3, v1, v2
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        {
            let thread = machine.threads.get_mut(&1).unwrap();
            thread.fregs[1] = 1.5f64.to_bits();
            thread.fregs[2] = 2.25f64.to_bits();
            thread.vregs[1] = 1 | (2 << 32) | (3 << 64) | (4 << 96);
            thread.vregs[2] = 10 | (20 << 32) | (30 << 64) | (40 << 96);
        }
        machine.current_tid = 1;
        let fadd = machine.processes[&1].program.instructions[0].clone();
        machine.exec(fadd).unwrap();
        let vadd = machine.processes[&1].program.instructions[1].clone();
        machine.exec(vadd).unwrap();
        let thread = machine.threads.get(&1).unwrap();
        assert_eq!(f64::from_bits(thread.fregs[3]), 3.75);
        assert_eq!(thread.vregs[3], 11 | (22 << 32) | (33 << 64) | (44 << 96));
    }
}

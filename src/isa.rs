use std::fmt;

pub const GPR_COUNT: usize = 32;
pub const FDR_COUNT: usize = 256;
pub const FPR_COUNT: usize = 32;
pub const VR_COUNT: usize = 16;
pub const DATA_BASE: u64 = 0x10_000;
pub const STACK_TOP: u64 = 0x680_000;
pub const HEAP_BASE: u64 = 0x100_000;
pub const ARG_BASE: u64 = 0x700_000;
pub const ARG_SIZE: u64 = 0x20_000;
pub const MEMORY_SIZE: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Reg(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FdReg(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FReg(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VReg(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pcr {
    Pid,
    Ppid,
    Tid,
    Uid,
    Gid,
    Tp,
    Sigmask,
    RealtimeSec,
    RealtimeNsec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Label(String),
    Address(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Imm(i64),
    Label(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemRef {
    BaseOffset(Reg, i64),
    Label(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instr {
    Nop,
    Li(Reg, Value),
    Mov(Reg, Reg),
    Add(Reg, Reg, Reg),
    Sub(Reg, Reg, Reg),
    Mul(Reg, Reg, Reg),
    Div(Reg, Reg, Reg),
    And(Reg, Reg, Reg),
    Or(Reg, Reg, Reg),
    Xor(Reg, Reg, Reg),
    Not(Reg, Reg),
    Lsl(Reg, Reg, Reg),
    Lsr(Reg, Reg, Reg),
    Asr(Reg, Reg, Reg),
    Cmp(Reg, Reg),
    Jmp(Target),
    Branch(Condition, Target),
    Call(Target),
    CallReg(Reg),
    Ret,
    Ld(Reg, MemRef, Width),
    St(MemRef, Reg, Width),
    Fence,
    Pull(Reg, FdReg, Reg, Reg),
    Push(Reg, FdReg, Reg, Reg),
    Await(Reg, FdReg, Reg),
    AwaitDyn(Reg, Reg, Reg),
    PollFd(Reg, FdReg, Reg),
    PollFdDyn(Reg, Reg, Reg),
    Alloc(Reg, Reg),
    AllocEx(Reg, Reg, Reg),
    AllocSize(Reg, Reg),
    Free(Reg),
    OpenFd(FdReg, Reg, Reg),
    OpenFdDyn(Reg, Reg, Reg),
    OpenDir(FdReg, Reg, Reg),
    OpenDirDyn(Reg, Reg, Reg),
    ReadFd(FdReg, Reg, Reg),
    ReadFdDyn(Reg, Reg, Reg),
    PreadFd(FdReg, Reg, Reg, Reg),
    PreadFdDyn(Reg, Reg, Reg, Reg),
    ReaddirFd(FdReg, Reg),
    ReaddirFdDyn(Reg, Reg),
    RewinddirFd(FdReg),
    RewinddirFdDyn(Reg),
    WriteFd(FdReg, Reg, Reg),
    WriteFdDyn(Reg, Reg, Reg),
    PwriteFd(FdReg, Reg, Reg, Reg),
    PwriteFdDyn(Reg, Reg, Reg, Reg),
    MkdirPath(Reg, Reg),
    UnlinkPath(Reg),
    RenamePath(Reg, Reg),
    LinkPath(Reg, Reg, Reg),
    SymlinkPath(Reg, Reg),
    ReadlinkPath(Reg, Reg, Reg),
    ChdirPath(Reg),
    GetcwdPath(Reg, Reg),
    ChmodPath(Reg, Reg, Reg),
    ChownPath(Reg, Reg, Reg, Reg),
    UtimePath(Reg, Reg, Reg),
    UtimeFd(FdReg, Reg),
    UtimeFdDyn(Reg, Reg),
    StatPath(Reg, Reg, Reg),
    StatFd(Reg, FdReg),
    StatFdDyn(Reg, Reg),
    FdClose(FdReg),
    FdCloseDyn(Reg),
    FdSeek(FdReg, Reg, Reg),
    FdSeekDyn(Reg, Reg, Reg),
    WaitOnFd(FdReg, Reg),
    FdDup(FdReg, FdReg),
    FdDup2(FdReg, FdReg),
    ErrnoGet(Reg),
    ErrnoSet(Reg),
    WaitPid(Reg, Reg),
    GetPcr(Reg, Pcr),
    SetPcr(Pcr, Reg),
    EnvGet(Reg, Reg, Reg, Reg),
    Random(Reg, Reg, Reg),
    Fork(Reg),
    Exec(Reg, Reg),
    Spawn(Reg, Reg),
    ThreadJoin(Reg, Reg, Reg),
    Yield,
    Sleep(Reg),
    Exit(Reg),
    Mmap(Reg, Reg, Reg, Reg, FdReg, Reg),
    Munmap(Reg, Reg),
    Mprotect(Reg, Reg, Reg),
    Sigaction(Reg, Reg),
    SigmaskSet(Reg),
    Alarm(Reg, Reg),
    Kill(Reg, Reg),
    Sigret,
    LockCmpxchg(Reg, Reg, Reg, Reg),
    FutexWait(Reg, Reg),
    FutexWake(Reg, Reg),
    Inb(Reg, Reg),
    Outb(Reg, Reg),
    LoadUcode(Reg, Reg),
    MsgSend(Reg, Reg, Reg),
    ObjectCtl(Reg, Reg),
    DmaCtl(Reg, Reg),
    CapSend(Reg, Reg),
    CapRecv(Reg, Reg),
    CapDup(Reg, Reg),
    CapRevoke(Reg, Reg),
    DomainCtl(Reg, Reg),
    CallCap(Reg, FdReg, Reg, Reg),
    RetCap(Reg, Reg, Reg),
    FAdd(FReg, FReg, FReg),
    FSub(FReg, FReg, FReg),
    FMul(FReg, FReg, FReg),
    FDiv(FReg, FReg, FReg),
    VAdd32(VReg, VReg, VReg),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Width {
    Byte,
    Word,
    Double,
}

impl Width {
    pub fn bytes(self) -> usize {
        match self {
            Width::Byte => 1,
            Width::Word => 4,
            Width::Double => 8,
        }
    }
}

impl fmt::Display for Reg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "r{}", self.0)
    }
}

pub fn parse_reg(text: &str) -> Result<Reg, String> {
    let Some(rest) = text.strip_prefix('r') else {
        return Err(format!("expected register, got {text:?}"));
    };
    let idx = rest
        .parse::<usize>()
        .map_err(|_| format!("invalid register {text:?}"))?;
    if idx >= GPR_COUNT {
        return Err(format!("register out of range: {text}"));
    }
    Ok(Reg(idx))
}

pub fn parse_fd(text: &str) -> Result<FdReg, String> {
    let Some(rest) = text.strip_prefix("fd") else {
        return Err(format!("expected fd register, got {text:?}"));
    };
    let idx = rest
        .parse::<usize>()
        .map_err(|_| format!("invalid fd register {text:?}"))?;
    if idx >= FDR_COUNT {
        return Err(format!("fd register out of range: {text}"));
    }
    Ok(FdReg(idx))
}

pub fn parse_freg(text: &str) -> Result<FReg, String> {
    let Some(rest) = text.strip_prefix('f') else {
        return Err(format!("expected FPU register, got {text:?}"));
    };
    let idx = rest
        .parse::<usize>()
        .map_err(|_| format!("invalid FPU register {text:?}"))?;
    if idx >= FPR_COUNT {
        return Err(format!("FPU register out of range: {text}"));
    }
    Ok(FReg(idx))
}

pub fn parse_vreg(text: &str) -> Result<VReg, String> {
    let Some(rest) = text.strip_prefix('v') else {
        return Err(format!("expected vector register, got {text:?}"));
    };
    let idx = rest
        .parse::<usize>()
        .map_err(|_| format!("invalid vector register {text:?}"))?;
    if idx >= VR_COUNT {
        return Err(format!("vector register out of range: {text}"));
    }
    Ok(VReg(idx))
}

pub fn parse_pcr(text: &str) -> Result<Pcr, String> {
    match text.to_ascii_uppercase().as_str() {
        "PID" => Ok(Pcr::Pid),
        "PPID" => Ok(Pcr::Ppid),
        "TID" => Ok(Pcr::Tid),
        "UID" => Ok(Pcr::Uid),
        "GID" => Ok(Pcr::Gid),
        "TP" => Ok(Pcr::Tp),
        "SIGMASK" => Ok(Pcr::Sigmask),
        "REALTIME_SEC" => Ok(Pcr::RealtimeSec),
        "REALTIME_NSEC" => Ok(Pcr::RealtimeNsec),
        _ => Err(format!("unknown PCR {text:?}")),
    }
}

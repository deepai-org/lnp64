use std::fmt;

pub const GPR_COUNT: usize = 32;
pub const FDR_COUNT: usize = 256;
pub const DATA_BASE: u64 = 0x10_000;
pub const STACK_TOP: u64 = 0x80_000;
pub const HEAP_BASE: u64 = 0x100_000;
pub const MEMORY_SIZE: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Reg(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FdReg(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pcr {
    Pid,
    Tid,
    Uid,
    Gid,
    Sigmask,
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
    Ret,
    Ld(Reg, MemRef, Width),
    St(MemRef, Reg, Width),
    Fence,
    Alloc(Reg, Reg),
    Free(Reg),
    OpenFd(FdReg, Reg, Reg),
    ReadFd(FdReg, Reg, Reg),
    WriteFd(FdReg, Reg, Reg),
    WaitOnFd(FdReg, Reg),
    FdDup(FdReg, FdReg),
    GetPcr(Reg, Pcr),
    SetPcr(Pcr, Reg),
    Fork(Reg),
    Exec(Reg, Reg),
    Spawn(Reg, Reg),
    Yield,
    Sleep(Reg),
    Exit(Reg),
    Mmap(Reg, Reg, Reg, Reg, FdReg, Reg),
    Munmap(Reg, Reg),
    Sigaction(Reg, Reg),
    SigmaskSet(Reg),
    Kill(Reg, Reg),
    Sigret,
    LockCmpxchg(Reg, Reg, Reg, Reg),
    FutexWait(Reg, Reg),
    FutexWake(Reg, Reg),
    Inb(Reg, Reg),
    Outb(Reg, Reg),
    LoadUcode(Reg, Reg),
    MsgSend(Reg, Reg, Reg),
    MsgRecv(Reg, Reg),
    FAdd(Reg, Reg, Reg),
    FSub(Reg, Reg, Reg),
    FMul(Reg, Reg, Reg),
    FDiv(Reg, Reg, Reg),
    VAdd32(Reg, Reg, Reg),
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

pub fn parse_pcr(text: &str) -> Result<Pcr, String> {
    match text.to_ascii_uppercase().as_str() {
        "PID" => Ok(Pcr::Pid),
        "TID" => Ok(Pcr::Tid),
        "UID" => Ok(Pcr::Uid),
        "GID" => Ok(Pcr::Gid),
        "SIGMASK" => Ok(Pcr::Sigmask),
        _ => Err(format!("unknown PCR {text:?}")),
    }
}

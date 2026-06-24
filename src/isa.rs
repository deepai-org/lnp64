use std::fmt;

pub const GPR_COUNT: usize = 32;
pub const FDR_COUNT: usize = 256;
pub const FPR_COUNT: usize = 32;
pub const VR_COUNT: usize = 16;
pub const DATA_BASE: u64 = 0x10_000;
pub const STACK_TOP: u64 = 0x1800_000;
pub const HEAP_BASE: u64 = 0x100_000;
pub const ARG_BASE: u64 = 0x1900_000;
pub const ARG_SIZE: u64 = 0x20_000;
pub const MEMORY_SIZE: usize = 32 * 1024 * 1024;

// Flat-exec / top-level RTL program fixture allocation windows. These MIRROR the
// fixed SRAM windows in rtl/core/lnp64_core_tile.sv (HEAP_ARCH_BASE /
// MMAP_ARCH_BASE) and rtl/engines/lnp64_engine_shells.sv so the flat-exec
// emulator (the cosim oracle) hands out the same heap/mmap addresses as the RTL
// top-program fixture and the per-program manifest cosim is byte-exact. The
// real-ELF runtime keeps HEAP_BASE/MMAP_BASE; these apply only to the flat-exec
// fixture, whose layout is otherwise an arbitrary shared convention.
pub const FLAT_EXEC_HEAP_BASE: u64 = 0x10_f000;
pub const FLAT_EXEC_MMAP_BASE: u64 = 0x20_e000;

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
    Sigpending,
    RealtimeSec,
    RealtimeNsec,
    CredProfile,
    CredHandle,
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

/// Condition for the fused compare-and-select (`sel.<cc>`), mirroring the
/// branch conditions (beq/bne/blt/bge/bltu/bgeu).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelCond {
    Eq,
    Ne,
    Lt,
    Ge,
    Ltu,
    Geu,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instr {
    Nop,
    Li(Reg, Value),
    Auipc(Reg, Value),
    Mov(Reg, Reg),
    Add(Reg, Reg, Reg),
    Addi(Reg, Reg, i64),
    Sub(Reg, Reg, Reg),
    Mul(Reg, Reg, Reg),
    Mulh(Reg, Reg, Reg),
    Mulhu(Reg, Reg, Reg),
    Mulhsu(Reg, Reg, Reg),
    Div(Reg, Reg, Reg),
    Udiv(Reg, Reg, Reg),
    Srem(Reg, Reg, Reg),
    Urem(Reg, Reg, Reg),
    And(Reg, Reg, Reg),
    Andi(Reg, Reg, i64),
    Or(Reg, Reg, Reg),
    Ori(Reg, Reg, i64),
    Xor(Reg, Reg, Reg),
    Xori(Reg, Reg, i64),
    Not(Reg, Reg),
    Lsl(Reg, Reg, Reg),
    Lsli(Reg, Reg, i64),
    Lsr(Reg, Reg, Reg),
    Lsri(Reg, Reg, i64),
    Asr(Reg, Reg, Reg),
    Asri(Reg, Reg, i64),
    SextB(Reg, Reg),
    SextH(Reg, Reg),
    SextW(Reg, Reg),
    ZextB(Reg, Reg),
    ZextH(Reg, Reg),
    ZextW(Reg, Reg),
    Clz(Reg, Reg),
    Ctz(Reg, Reg),
    Popcnt(Reg, Reg),
    Rol(Reg, Reg, Reg),
    Ror(Reg, Reg, Reg),
    Bswap16(Reg, Reg),
    Bswap32(Reg, Reg),
    Bswap64(Reg, Reg),
    Slt(Reg, Reg, Reg),
    Sltu(Reg, Reg, Reg),
    Slti(Reg, Reg, i64),
    Sltiu(Reg, Reg, i64),
    Liu(Reg, Reg, i64),
    Jmp(Target),
    Jal(Reg, Target),
    Jalr(Reg, Reg, i64),
    Beq(Reg, Reg, Target),
    Bne(Reg, Reg, Target),
    Blt(Reg, Reg, Target),
    Bge(Reg, Reg, Target),
    Bltu(Reg, Reg, Target),
    Bgeu(Reg, Reg, Target),
    /// Fused compare-and-select: rd = (ra <cc> rb) ? rt : rf.
    /// Operands: (cc, rd, ra, rb, rt, rf).
    Sel(SelCond, Reg, Reg, Reg, Reg, Reg),
    Ld(Reg, MemRef, Width),
    LdS(Reg, MemRef, Width),
    St(MemRef, Reg, Width),
    LrD(Reg, Reg),
    ScD(Reg, Reg, Reg),
    Fence,
    Isync(Reg, Reg, Reg),
    Pull(Reg, FdReg, Reg, Reg),
    Push(Reg, FdReg, Reg, Reg),
    Await(Reg, FdReg, Reg),
    AwaitDyn(Reg, Reg, Reg, Reg),
    Alloc(Reg, Reg),
    AllocEx(Reg, Reg, Reg),
    AllocSize(Reg, Reg),
    Free(Reg),
    OpenFd(FdReg, Reg, Reg),
    OpenFdDyn(Reg, Reg, Reg),
    OpenAtDyn(Reg, Reg, Reg, Reg),
    OpenDir(FdReg, Reg, Reg),
    OpenDirDyn(Reg, Reg, Reg),
    ReadFd(FdReg, Reg, Reg),
    PreadFd(FdReg, Reg, Reg, Reg),
    PreadFdDyn(Reg, Reg, Reg, Reg),
    ReaddirFd(FdReg, Reg),
    ReaddirFdDyn(Reg, Reg),
    RewinddirFd(FdReg),
    RewinddirFdDyn(Reg),
    WriteFd(FdReg, Reg, Reg),
    PwriteFd(FdReg, Reg, Reg, Reg),
    PwriteFdDyn(Reg, Reg, Reg, Reg),
    MkdirPath(Reg, Reg),
    MkdirPathAt(Reg, Reg, Reg),
    UnlinkPath(Reg),
    UnlinkPathAt(Reg, Reg, Reg),
    RenamePath(Reg, Reg),
    RenamePathAt(Reg, Reg, Reg, Reg),
    LinkPath(Reg, Reg, Reg),
    LinkPathAt(Reg, Reg, Reg, Reg, Reg),
    SymlinkPath(Reg, Reg),
    SymlinkPathAt(Reg, Reg, Reg),
    ReadlinkPath(Reg, Reg, Reg),
    ReadlinkPathAt(Reg, Reg, Reg, Reg),
    ChdirPath(Reg),
    GetcwdPath(Reg, Reg),
    ChmodPath(Reg, Reg, Reg),
    ChmodPathAt(Reg, Reg, Reg, Reg),
    ChownPath(Reg, Reg, Reg, Reg),
    ChownPathAt(Reg, Reg, Reg, Reg, Reg),
    UtimePath(Reg, Reg, Reg),
    UtimePathAt(Reg, Reg, Reg, Reg),
    UtimeFd(FdReg, Reg),
    UtimeFdDyn(Reg, Reg),
    StatPath(Reg, Reg, Reg),
    StatPathAt(Reg, Reg, Reg, Reg),
    StatFd(Reg, FdReg),
    StatFdDyn(Reg, Reg),
    FcntlFdDyn(Reg, Reg, Reg),
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
    SetPcr(Reg, Pcr, Reg),
    EnvGet(Reg, Reg, Reg, Reg),
    Random(Reg, Reg, Reg),
    Fork(Reg),
    Exec(Reg, Reg, Reg),
    Spawn(Reg, Reg),
    CloneSpawn(Reg, Reg, Reg),
    ThreadJoin(Reg, Reg, Reg),
    ThreadDetach(Reg, Reg),
    Yield,
    Sleep(Reg),
    Exit(Reg),
    Mmap(Reg, Reg, Reg, Reg, FdReg, Reg),
    Munmap(Reg, Reg),
    Mprotect(Reg, Reg, Reg),
    MmapBootstrap(Reg, Reg, Reg, Reg),
    MunmapBootstrap(Reg, Reg),
    MprotectBootstrap(Reg, Reg, Reg, Reg),
    Sigaction(Reg, Reg),
    SigmaskSet(Reg),
    Alarm(Reg, Reg),
    Kill(Reg, Reg),
    Sigret,
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
    NsCtl(Reg, Reg),
    CallCap(Reg, FdReg, Reg, Reg),
    RetCap(Reg, Reg, Reg),
    FAdd(FReg, FReg, FReg),
    FSub(FReg, FReg, FReg),
    FMul(FReg, FReg, FReg),
    FDiv(FReg, FReg, FReg),
    VAdd32(VReg, VReg, VReg),
    // Unified endpoint IPC (Phase 3, unified_object_model.md). Operands are GPRs
    // holding values (handles/pointers), per the FDR->GPR migration.
    // EndpointCreate rd, rs1(mode/capacity hint) -> rd = endpoint handle token.
    EndpointCreate(Reg, Reg),
    // Send rd, rs1(ep handle), rs2(msg-descriptor ptr) -> rd = bytes sent or -errno.
    Send(Reg, Reg, Reg),
    // Recv rd, rs1(ep handle), rs2(msg-descriptor ptr) -> rd = bytes received or -errno.
    Recv(Reg, Reg, Reg),
    // Wait rd, rs1(waitset-descriptor ptr), rs2(timeout) -> rd = #ready or -errno.
    // Collapses await/await_ex/waitable_probe/futex_wait/thread_join/wait_pid/sleep/alarm.
    Wait(Reg, Reg, Reg),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Width {
    Byte,
    Half,
    Word,
    Double,
}

impl Width {
    pub fn bytes(self) -> usize {
        match self {
            Width::Byte => 1,
            Width::Half => 2,
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
        "UID" | "POSIX_UID" => Ok(Pcr::Uid),
        "GID" | "POSIX_GID" => Ok(Pcr::Gid),
        "TP" => Ok(Pcr::Tp),
        "TLS_BASE" => Ok(Pcr::Tp),
        "SIGMASK" => Ok(Pcr::Sigmask),
        "SIGPENDING" => Ok(Pcr::Sigpending),
        "REALTIME_SEC" => Ok(Pcr::RealtimeSec),
        "REALTIME_NSEC" => Ok(Pcr::RealtimeNsec),
        "CRED_PROFILE" => Ok(Pcr::CredProfile),
        "CRED_HANDLE" => Ok(Pcr::CredHandle),
        _ => Err(format!("unknown PCR {text:?}")),
    }
}

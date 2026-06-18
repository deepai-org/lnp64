#![allow(dead_code)]

use crate::native::{CloneProfile, MetadataOp, ObjectKind, ObjectProfile, Waitable};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatSurface {
    Open,
    CwdRoot,
    Read,
    Write,
    Close,
    Pipe,
    PollSelectEpoll,
    Fork,
    Exec,
    PthreadCreate,
    Mmap,
    FdPassing,
    SocketLoopback,
    Timer,
    CallGate,
    Signal,
    Errno,
    ResourceDomain,
    Stat,
    Chmod,
    Fcntl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityLayer {
    Native,
    Personality,
    RuntimeLibc,
    Unsupported,
    IntentionallyExcluded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativePrimitive {
    OpenAt,
    Pull,
    Push,
    Close,
    ObjectCtl {
        kind: ObjectKind,
        profile: ObjectProfile,
    },
    EventQueue,
    Await,
    Exec,
    Mmap,
    Mprotect,
    Munmap,
    CapabilityDuplicate,
    CapabilitySend,
    CapabilityRecv,
    DomainCtl,
    GateCall,
    GateReturn,
    Sleep,
    Clone {
        profile: CloneProfile,
    },
    EventDelivery,
    AbiSignalFrame,
    ExplicitResult,
    TlsErrnoView,
    Metadata(MetadataOp),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompatibilityLowering {
    pub surface: CompatSurface,
    pub native: &'static [NativePrimitive],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompatibilitySurfacePolicy {
    pub surface: CompatSurface,
    pub layer: CompatibilityLayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetBsdSyscallEntry {
    pub number: u16,
    pub name: &'static str,
    pub surface: CompatSurface,
    pub layer: CompatibilityLayer,
}

const fn surface_policy(
    surface: CompatSurface,
    layer: CompatibilityLayer,
) -> CompatibilitySurfacePolicy {
    CompatibilitySurfacePolicy { surface, layer }
}

const fn netbsd_entry(
    number: u16,
    name: &'static str,
    surface: CompatSurface,
    layer: CompatibilityLayer,
) -> NetBsdSyscallEntry {
    NetBsdSyscallEntry {
        number,
        name,
        surface,
        layer,
    }
}

pub const LOWER_OPEN: &[NativePrimitive] = &[NativePrimitive::OpenAt];
pub const LOWER_CWD_ROOT: &[NativePrimitive] = &[NativePrimitive::OpenAt];
pub const LOWER_READ: &[NativePrimitive] = &[NativePrimitive::Pull];
pub const LOWER_WRITE: &[NativePrimitive] = &[NativePrimitive::Push];
pub const LOWER_CLOSE: &[NativePrimitive] = &[NativePrimitive::Close];
pub const LOWER_PIPE: &[NativePrimitive] = &[NativePrimitive::ObjectCtl {
    kind: ObjectKind::Queue,
    profile: ObjectProfile::Pipe,
}];
pub const LOWER_WAIT: &[NativePrimitive] = &[
    NativePrimitive::EventQueue,
    NativePrimitive::Await,
    NativePrimitive::Pull,
];
pub const LOWER_FORK: &[NativePrimitive] = &[NativePrimitive::Clone {
    profile: CloneProfile::NewProcessCow,
}];
pub const LOWER_EXEC: &[NativePrimitive] = &[NativePrimitive::OpenAt, NativePrimitive::Exec];
pub const LOWER_PTHREAD_CREATE: &[NativePrimitive] = &[NativePrimitive::Clone {
    profile: CloneProfile::NewThreadSharedVm,
}];
pub const LOWER_MMAP: &[NativePrimitive] = &[
    NativePrimitive::Mmap,
    NativePrimitive::Mprotect,
    NativePrimitive::Munmap,
];
pub const LOWER_FD_PASSING: &[NativePrimitive] = &[
    NativePrimitive::CapabilityDuplicate,
    NativePrimitive::CapabilitySend,
    NativePrimitive::CapabilityRecv,
];
pub const LOWER_SOCKET_LOOPBACK: &[NativePrimitive] = &[
    NativePrimitive::ObjectCtl {
        kind: ObjectKind::Endpoint,
        profile: ObjectProfile::TcpStream,
    },
    NativePrimitive::Push,
    NativePrimitive::Pull,
    NativePrimitive::Await,
];
pub const LOWER_TIMER: &[NativePrimitive] = &[
    NativePrimitive::ObjectCtl {
        kind: ObjectKind::Timer,
        profile: ObjectProfile::Default,
    },
    NativePrimitive::Await,
    NativePrimitive::Pull,
    NativePrimitive::EventDelivery,
];
pub const LOWER_CALL_GATE: &[NativePrimitive] = &[
    NativePrimitive::ObjectCtl {
        kind: ObjectKind::Queue,
        profile: ObjectProfile::CallGate,
    },
    NativePrimitive::GateCall,
    NativePrimitive::GateReturn,
];
pub const LOWER_SIGNAL: &[NativePrimitive] = &[
    NativePrimitive::EventDelivery,
    NativePrimitive::AbiSignalFrame,
];
pub const LOWER_ERRNO: &[NativePrimitive] = &[
    NativePrimitive::ExplicitResult,
    NativePrimitive::TlsErrnoView,
];
pub const LOWER_STAT: &[NativePrimitive] = &[NativePrimitive::Metadata(MetadataOp::GetMeta)];
pub const LOWER_CHMOD: &[NativePrimitive] = &[NativePrimitive::Metadata(MetadataOp::SetMeta)];
pub const LOWER_FCNTL: &[NativePrimitive] = &[
    NativePrimitive::Metadata(MetadataOp::GetMeta),
    NativePrimitive::Metadata(MetadataOp::SetMeta),
    NativePrimitive::Metadata(MetadataOp::ObjectCtl),
];
pub const LOWER_RESOURCE_DOMAIN: &[NativePrimitive] = &[NativePrimitive::DomainCtl];

pub const COMPATIBILITY_LOWERINGS: &[CompatibilityLowering] = &[
    CompatibilityLowering {
        surface: CompatSurface::Open,
        native: LOWER_OPEN,
    },
    CompatibilityLowering {
        surface: CompatSurface::CwdRoot,
        native: LOWER_CWD_ROOT,
    },
    CompatibilityLowering {
        surface: CompatSurface::Read,
        native: LOWER_READ,
    },
    CompatibilityLowering {
        surface: CompatSurface::Write,
        native: LOWER_WRITE,
    },
    CompatibilityLowering {
        surface: CompatSurface::Close,
        native: LOWER_CLOSE,
    },
    CompatibilityLowering {
        surface: CompatSurface::Pipe,
        native: LOWER_PIPE,
    },
    CompatibilityLowering {
        surface: CompatSurface::PollSelectEpoll,
        native: LOWER_WAIT,
    },
    CompatibilityLowering {
        surface: CompatSurface::Fork,
        native: LOWER_FORK,
    },
    CompatibilityLowering {
        surface: CompatSurface::Exec,
        native: LOWER_EXEC,
    },
    CompatibilityLowering {
        surface: CompatSurface::PthreadCreate,
        native: LOWER_PTHREAD_CREATE,
    },
    CompatibilityLowering {
        surface: CompatSurface::Mmap,
        native: LOWER_MMAP,
    },
    CompatibilityLowering {
        surface: CompatSurface::FdPassing,
        native: LOWER_FD_PASSING,
    },
    CompatibilityLowering {
        surface: CompatSurface::SocketLoopback,
        native: LOWER_SOCKET_LOOPBACK,
    },
    CompatibilityLowering {
        surface: CompatSurface::Timer,
        native: LOWER_TIMER,
    },
    CompatibilityLowering {
        surface: CompatSurface::CallGate,
        native: LOWER_CALL_GATE,
    },
    CompatibilityLowering {
        surface: CompatSurface::Signal,
        native: LOWER_SIGNAL,
    },
    CompatibilityLowering {
        surface: CompatSurface::Errno,
        native: LOWER_ERRNO,
    },
    CompatibilityLowering {
        surface: CompatSurface::ResourceDomain,
        native: LOWER_RESOURCE_DOMAIN,
    },
    CompatibilityLowering {
        surface: CompatSurface::Stat,
        native: LOWER_STAT,
    },
    CompatibilityLowering {
        surface: CompatSurface::Chmod,
        native: LOWER_CHMOD,
    },
    CompatibilityLowering {
        surface: CompatSurface::Fcntl,
        native: LOWER_FCNTL,
    },
];

pub const COMPATIBILITY_SURFACE_POLICIES: &[CompatibilitySurfacePolicy] = &[
    surface_policy(CompatSurface::Open, CompatibilityLayer::Personality),
    surface_policy(CompatSurface::CwdRoot, CompatibilityLayer::Personality),
    surface_policy(CompatSurface::Read, CompatibilityLayer::Native),
    surface_policy(CompatSurface::Write, CompatibilityLayer::Native),
    surface_policy(CompatSurface::Close, CompatibilityLayer::Native),
    surface_policy(CompatSurface::Pipe, CompatibilityLayer::Personality),
    surface_policy(
        CompatSurface::PollSelectEpoll,
        CompatibilityLayer::Personality,
    ),
    surface_policy(CompatSurface::Fork, CompatibilityLayer::Personality),
    surface_policy(CompatSurface::Exec, CompatibilityLayer::Personality),
    surface_policy(
        CompatSurface::PthreadCreate,
        CompatibilityLayer::RuntimeLibc,
    ),
    surface_policy(CompatSurface::Mmap, CompatibilityLayer::Native),
    surface_policy(CompatSurface::FdPassing, CompatibilityLayer::Native),
    surface_policy(
        CompatSurface::SocketLoopback,
        CompatibilityLayer::Personality,
    ),
    surface_policy(CompatSurface::Timer, CompatibilityLayer::Personality),
    surface_policy(CompatSurface::CallGate, CompatibilityLayer::Native),
    surface_policy(CompatSurface::Signal, CompatibilityLayer::Personality),
    surface_policy(CompatSurface::Errno, CompatibilityLayer::RuntimeLibc),
    surface_policy(CompatSurface::ResourceDomain, CompatibilityLayer::Native),
    surface_policy(CompatSurface::Stat, CompatibilityLayer::Personality),
    surface_policy(CompatSurface::Chmod, CompatibilityLayer::Personality),
    surface_policy(CompatSurface::Fcntl, CompatibilityLayer::Personality),
];

// NetBSD-current sys/syscall.h revision 1.330 subset used by the personality gate.
pub const NETBSD_SYSCALLS: &[NetBsdSyscallEntry] = &[
    netbsd_entry(
        2,
        "fork",
        CompatSurface::Fork,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(3, "read", CompatSurface::Read, CompatibilityLayer::Native),
    netbsd_entry(4, "write", CompatSurface::Write, CompatibilityLayer::Native),
    netbsd_entry(
        5,
        "open",
        CompatSurface::Open,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(6, "close", CompatSurface::Close, CompatibilityLayer::Native),
    netbsd_entry(
        7,
        "compat_50_wait4",
        CompatSurface::PollSelectEpoll,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        12,
        "chdir",
        CompatSurface::CwdRoot,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        13,
        "fchdir",
        CompatSurface::CwdRoot,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        15,
        "chmod",
        CompatSurface::Chmod,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        27,
        "recvmsg",
        CompatSurface::SocketLoopback,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        28,
        "sendmsg",
        CompatSurface::SocketLoopback,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        29,
        "recvfrom",
        CompatSurface::SocketLoopback,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        30,
        "accept",
        CompatSurface::SocketLoopback,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        31,
        "getpeername",
        CompatSurface::SocketLoopback,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        32,
        "getsockname",
        CompatSurface::SocketLoopback,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        37,
        "kill",
        CompatSurface::Signal,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        41,
        "dup",
        CompatSurface::FdPassing,
        CompatibilityLayer::Native,
    ),
    netbsd_entry(
        42,
        "pipe",
        CompatSurface::Pipe,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        59,
        "execve",
        CompatSurface::Exec,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        73,
        "munmap",
        CompatSurface::Mmap,
        CompatibilityLayer::Native,
    ),
    netbsd_entry(
        74,
        "mprotect",
        CompatSurface::Mmap,
        CompatibilityLayer::Native,
    ),
    netbsd_entry(
        90,
        "dup2",
        CompatSurface::FdPassing,
        CompatibilityLayer::Native,
    ),
    netbsd_entry(
        92,
        "fcntl",
        CompatSurface::Fcntl,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        97,
        "compat_30_socket",
        CompatSurface::SocketLoopback,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        98,
        "connect",
        CompatSurface::SocketLoopback,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        104,
        "bind",
        CompatSurface::SocketLoopback,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        105,
        "setsockopt",
        CompatSurface::SocketLoopback,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        106,
        "listen",
        CompatSurface::SocketLoopback,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        118,
        "getsockopt",
        CompatSurface::SocketLoopback,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        120,
        "readv",
        CompatSurface::Read,
        CompatibilityLayer::Native,
    ),
    netbsd_entry(
        121,
        "writev",
        CompatSurface::Write,
        CompatibilityLayer::Native,
    ),
    netbsd_entry(
        128,
        "rename",
        CompatSurface::Open,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        133,
        "sendto",
        CompatSurface::SocketLoopback,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        134,
        "shutdown",
        CompatSurface::SocketLoopback,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        135,
        "socketpair",
        CompatSurface::SocketLoopback,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        136,
        "mkdir",
        CompatSurface::Open,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        173,
        "pread",
        CompatSurface::Read,
        CompatibilityLayer::Native,
    ),
    netbsd_entry(
        174,
        "pwrite",
        CompatSurface::Write,
        CompatibilityLayer::Native,
    ),
    netbsd_entry(
        177,
        "timerfd_create",
        CompatSurface::Timer,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        178,
        "timerfd_settime",
        CompatSurface::Timer,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        179,
        "timerfd_gettime",
        CompatSurface::Timer,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(197, "mmap", CompatSurface::Mmap, CompatibilityLayer::Native),
    netbsd_entry(
        199,
        "lseek",
        CompatSurface::Read,
        CompatibilityLayer::Native,
    ),
    netbsd_entry(
        209,
        "poll",
        CompatSurface::PollSelectEpoll,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        267,
        "eventfd",
        CompatSurface::PollSelectEpoll,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        291,
        "compat_16___sigaction14",
        CompatSurface::Signal,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        293,
        "__sigprocmask14",
        CompatSurface::Signal,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        295,
        "compat_16___sigreturn14",
        CompatSurface::Signal,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        296,
        "__getcwd",
        CompatSurface::CwdRoot,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        309,
        "_lwp_create",
        CompatSurface::PthreadCreate,
        CompatibilityLayer::RuntimeLibc,
    ),
    netbsd_entry(
        340,
        "__sigaction_sigtramp",
        CompatSurface::Signal,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        394,
        "__socket30",
        CompatSurface::SocketLoopback,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        417,
        "__select50",
        CompatSurface::PollSelectEpoll,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        430,
        "__nanosleep50",
        CompatSurface::Timer,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        449,
        "__wait450",
        CompatSurface::PollSelectEpoll,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        453,
        "pipe2",
        CompatSurface::Pipe,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        465,
        "fexecve",
        CompatSurface::Exec,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        468,
        "openat",
        CompatSurface::Open,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        502,
        "epoll_create1",
        CompatSurface::PollSelectEpoll,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        503,
        "epoll_ctl",
        CompatSurface::PollSelectEpoll,
        CompatibilityLayer::Personality,
    ),
    netbsd_entry(
        504,
        "epoll_pwait2",
        CompatSurface::PollSelectEpoll,
        CompatibilityLayer::Personality,
    ),
];

pub const OBJECT_CTL_CREATE_RECORD_SIZE: u64 = 72;
pub const DOMAIN_CTL_RECORD_SIZE: u64 = 208;

pub fn lowering_for(surface: CompatSurface) -> &'static [NativePrimitive] {
    COMPATIBILITY_LOWERINGS
        .iter()
        .find(|entry| entry.surface == surface)
        .map(|entry| entry.native)
        .unwrap_or(&[])
}

pub fn layer_for(surface: CompatSurface) -> Option<CompatibilityLayer> {
    COMPATIBILITY_SURFACE_POLICIES
        .iter()
        .find(|entry| entry.surface == surface)
        .map(|entry| entry.layer)
}

pub fn netbsd_syscall(number: u16) -> Option<&'static NetBsdSyscallEntry> {
    NETBSD_SYSCALLS.iter().find(|entry| entry.number == number)
}

pub fn netbsd_syscall_by_name(name: &str) -> Option<&'static NetBsdSyscallEntry> {
    NETBSD_SYSCALLS.iter().find(|entry| entry.name == name)
}

pub fn netbsd_syscall_lowering(number: u16) -> &'static [NativePrimitive] {
    netbsd_syscall(number)
        .map(|entry| lowering_for(entry.surface))
        .unwrap_or(&[])
}

pub const fn pipe_object_profile() -> (ObjectKind, ObjectProfile) {
    (ObjectKind::Queue, ObjectProfile::Pipe)
}

pub const fn fork_clone_profile() -> CloneProfile {
    CloneProfile::NewProcessCow
}

pub const fn pthread_clone_profile() -> CloneProfile {
    CloneProfile::NewThreadSharedVm
}

pub const fn signal_waitable(signum: u64) -> Waitable {
    Waitable::Signal(signum)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_field<'a>(manifest: &'a str, key: &str) -> &'a str {
        let prefix = format!("{key}=");
        manifest
            .lines()
            .find_map(|line| line.strip_prefix(&prefix))
            .unwrap_or_else(|| panic!("missing manifest field {key}"))
    }

    fn manifest_csv_contains(manifest: &str, key: &str, value: &str) -> bool {
        manifest_field(manifest, key)
            .split(',')
            .any(|entry| entry == value)
    }

    fn relocation_rows(manifest: &str) -> Vec<(u16, &str, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(3, ',');
                let number = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing relocation number in {line}"))
                    .parse()
                    .unwrap_or_else(|_| panic!("invalid relocation number in {line}"));
                let name = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing relocation name in {line}"));
                let calculation = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing relocation calculation in {line}"));
                (number, name, calculation)
            })
            .collect()
    }

    fn intrinsic_rows(manifest: &str) -> Vec<(&str, &str, &str, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(4, '|');
                let name = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing intrinsic name in {line}"));
                let primitive = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing intrinsic primitive in {line}"));
                let result = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing intrinsic result in {line}"));
                let operands = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing intrinsic operands in {line}"));
                (name, primitive, result, operands)
            })
            .collect()
    }

    fn isel_rows(manifest: &str) -> Vec<(&str, &str, Vec<&str>)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(3, '|');
                let group = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing isel group in {line}"));
                let status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing isel status in {line}"));
                let opcodes = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing isel opcodes in {line}"))
                    .split(',')
                    .collect();
                (group, status, opcodes)
            })
            .collect()
    }

    fn exec_plan_rows(manifest: &str) -> Vec<(&str, &str, Vec<&str>)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(3, '|');
                let record = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing exec-plan record in {line}"));
                let requirement = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing exec-plan requirement in {line}"));
                let record_fields = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing exec-plan fields in {line}"))
                    .split(',')
                    .collect();
                (record, requirement, record_fields)
            })
            .collect()
    }

    fn contract_rows(manifest: &str) -> Vec<(&str, &str, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(3, '|');
                let name = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing contract name in {line}"));
                let path = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing contract path in {line}"));
                let test = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing contract test in {line}"));
                (name, path, test)
            })
            .collect()
    }

    fn inline_asm_rows(manifest: &str) -> Vec<(&str, &str, &str, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(4, '|');
                let constraint = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing inline-asm constraint in {line}"));
                let class = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing inline-asm class in {line}"));
                let values = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing inline-asm values in {line}"));
                let usage = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing inline-asm use in {line}"));
                (constraint, class, values, usage)
            })
            .collect()
    }

    fn crt_startup_rows(manifest: &str) -> Vec<(&str, &str, Vec<&str>)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(3, '|');
                let item = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing crt startup item in {line}"));
                let requirement = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing crt startup requirement in {line}"));
                let contract = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing crt startup contract in {line}"))
                    .split(',')
                    .collect();
                (item, requirement, contract)
            })
            .collect()
    }

    fn transition_rows(manifest: &str) -> Vec<(&str, &str, Vec<&str>, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(4, '|');
                let phase = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing transition phase in {line}"));
                let status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing transition status in {line}"));
                let artifacts = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing transition artifacts in {line}"))
                    .split(',')
                    .collect();
                let gate = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing transition gate in {line}"));
                (phase, status, artifacts, gate)
            })
            .collect()
    }

    fn llvm_bootstrap_rows(manifest: &str) -> Vec<(&str, &str, Vec<&str>, Vec<&str>, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(5, '|');
                let case = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm bootstrap case in {line}"));
                let source = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm bootstrap source in {line}"));
                let backend = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm bootstrap backend contracts in {line}"))
                    .split(',')
                    .collect();
                let runtime = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm bootstrap runtime contracts in {line}"))
                    .split(',')
                    .collect();
                let status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm bootstrap status in {line}"));
                (case, source, backend, runtime, status)
            })
            .collect()
    }

    #[test]
    fn toolchain_contract_index_is_complete() {
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let rows = contract_rows(contract_index);
        let mut names = std::collections::BTreeSet::new();
        let mut paths = std::collections::BTreeSet::new();
        let mut tests = std::collections::BTreeSet::new();
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

        for (name, path, test) in rows {
            assert!(names.insert(name), "duplicate contract name {name}");
            assert!(paths.insert(path), "duplicate contract path {path}");
            assert!(tests.insert(test), "duplicate contract test {test}");
            assert!(
                manifest_root.join(path).is_file(),
                "contract {name} path {path} does not exist"
            );
            assert!(!test.is_empty(), "empty test for contract {name}");
        }
        for name in [
            "contract_index",
            "target",
            "relocations",
            "psabi",
            "intrinsics",
            "isel",
            "llvm_bootstrap",
            "exec_plan",
            "loader",
            "debug_unwind",
            "inline_asm",
            "crt_startup",
            "transition",
        ] {
            assert!(names.contains(name), "missing contract index row {name}");
        }
    }

    #[test]
    fn llvm_bootstrap_manifest_names_first_clang_gate() {
        let bootstrap_manifest = include_str!("../toolchain/lnp64_llvm_bootstrap.manifest");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let rows = llvm_bootstrap_rows(bootstrap_manifest);
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut cases = std::collections::BTreeSet::new();

        assert!(contract_index.contains(
            "llvm_bootstrap|toolchain/lnp64_llvm_bootstrap.manifest|llvm_bootstrap_manifest_names_first_clang_gate"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_llvm_bootstrap.manifest"));
        assert!(roadmap.contains("toolchain/lnp64_llvm_bootstrap.manifest"));
        for case in ["hello", "arithmetic", "memory", "calls", "simple libc"] {
            assert!(
                roadmap.contains(case),
                "roadmap must describe llvm bootstrap case {case}"
            );
        }

        for (case, source, backend_contracts, runtime_contracts, status) in rows {
            assert!(cases.insert(case), "duplicate llvm bootstrap case {case}");
            assert!(
                manifest_root.join(source).exists(),
                "llvm bootstrap case {case} names missing source/gate {source}"
            );
            assert_eq!(
                status, "planned",
                "case {case} must stay planned until real Clang/lld/loader execution exists"
            );
            assert!(
                backend_contracts.contains(&"static_link"),
                "case {case} must require static linking"
            );
            assert!(
                !runtime_contracts.is_empty(),
                "case {case} must name runtime expectations"
            );
        }

        for case in ["hello", "arithmetic", "memory", "calls", "simple_libc"] {
            assert!(cases.contains(case), "missing llvm bootstrap case {case}");
        }
    }

    #[test]
    fn toolchain_transition_manifest_records_layered_deliverables() {
        let manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let conformance = include_str!("../conformance_matrix.md");
        let libc = include_str!("../libc_roadmap.md");
        let object_format = include_str!("../object_format.md");
        let psabi = include_str!("../psABI.md");
        let rows = transition_rows(manifest);
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut phases = std::collections::BTreeSet::new();

        for (phase, status, artifacts, gate) in rows {
            assert!(phases.insert(phase), "duplicate transition phase {phase}");
            assert!(
                ["required", "planned"].contains(&status),
                "unknown transition status {status} for {phase}"
            );
            assert!(!artifacts.is_empty(), "empty artifacts for {phase}");
            assert!(!gate.is_empty(), "empty gate for {phase}");
            for artifact in artifacts {
                assert!(
                    manifest_root.join(artifact).exists(),
                    "transition phase {phase} names missing artifact {artifact}"
                );
            }
        }

        for phase in [
            "toy_compiler_retirement",
            "real_toolchain_target",
            "minimal_llvm_clang_path",
            "libc_runtime_shim",
            "software_loader_exec_plan",
            "netbsd_personality_layers",
            "conformance_gates",
        ] {
            assert!(phases.contains(phase), "missing transition phase {phase}");
        }

        assert!(roadmap.contains("## Toy Compiler Freeze Policy"));
        assert!(roadmap.contains("## First Acceptance Gates"));
        assert!(roadmap.contains("## Checked Transition Deliverables"));
        assert!(roadmap.contains("`minimal_llvm_clang_path` row is still marked planned"));
        assert!(roadmap.contains("without the toy C compiler"));
        assert!(psabi.contains("## Register Model"));
        assert!(psabi.contains("## Calling Convention"));
        assert!(psabi.contains("## Debug and Unwind Minimum"));
        assert!(object_format.contains("## Relocation Model"));
        assert!(object_format.contains("## Exec-Plan Descriptor Boundary"));
        assert!(libc.contains("startup"));
        assert!(libc.contains("errno"));
        assert!(libc.contains("pthread"));
        assert!(conformance.contains("scripts/run_software_gates.sh"));
        assert!(conformance.contains("scripts/run_netbsd_personality_system.sh"));
    }

    #[test]
    fn llvm_target_manifest_records_required_backend_contract() {
        let manifest = include_str!("../toolchain/lnp64_target.manifest");
        let object_format = include_str!("../object_format.md");
        let psabi_doc = include_str!("../psABI.md");
        let roadmap = include_str!("../toolchain_roadmap.md");
        assert_eq!(manifest_field(manifest, "triple"), "lnp64-unknown-none");
        assert_eq!(manifest_field(manifest, "object_format"), "ELF64");
        assert_eq!(manifest_field(manifest, "endianness"), "little");
        assert_eq!(manifest_field(manifest, "data_model"), "LP64");
        assert_eq!(manifest_field(manifest, "pointer_width"), "64");
        assert_eq!(manifest_field(manifest, "e_machine"), "0x6c64");
        assert_eq!(manifest_field(manifest, "psabi"), "psABI.md");
        assert_eq!(
            manifest_field(manifest, "psabi_contract"),
            "toolchain/lnp64_psabi.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "object_contract"),
            "object_format.md"
        );
        assert_eq!(
            manifest_field(manifest, "relocation_contract"),
            "toolchain/lnp64_relocations.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "intrinsic_contract"),
            "toolchain/lnp64_intrinsics.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "isel_contract"),
            "toolchain/lnp64_isel.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "exec_plan_contract"),
            "toolchain/lnp64_exec_plan.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "debug_unwind_contract"),
            "toolchain/lnp64_debug_unwind.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "inline_asm_contract"),
            "toolchain/lnp64_inline_asm.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "crt_startup_contract"),
            "toolchain/lnp64_crt_startup.manifest"
        );
        assert_eq!(manifest_field(manifest, "gpr"), "r0-r31");
        assert_eq!(manifest_field(manifest, "fdr"), "fd0-fd255");
        for pcr in ["PID", "PPID", "TID", "TP", "SIGMASK", "SIGPENDING"] {
            assert!(manifest_csv_contains(manifest, "pcr", pcr), "missing {pcr}");
        }
        assert!(manifest_csv_contains(
            manifest,
            "native_primitives",
            "CLONE"
        ));
        for profile in [
            "new_process_cow",
            "new_thread_shared_vm",
            "spawn_entry",
            "domain_task",
        ] {
            assert!(
                manifest_csv_contains(manifest, "clone_profiles", profile),
                "missing clone profile {profile}"
            );
            assert!(
                psabi_doc.contains(profile),
                "psABI.md is missing clone profile {profile}"
            );
        }
        for relocation in [
            "R_LNP64_NONE",
            "R_LNP64_ABS64",
            "R_LNP64_ABS32",
            "R_LNP64_PC32",
            "R_LNP64_BRANCH26",
            "R_LNP64_GOT64",
            "R_LNP64_GLOB_DAT",
            "R_LNP64_RELATIVE",
            "R_LNP64_TLS_TPREL64",
            "R_LNP64_TLS_DTPREL64",
            "R_LNP64_FDR_DESC64",
            "R_LNP64_CAP_DESC64",
            "R_LNP64_CALLGATE64",
        ] {
            assert!(
                manifest_csv_contains(manifest, "relocations", relocation),
                "missing {relocation}"
            );
        }
        for relocation in manifest_field(manifest, "relocations").split(',') {
            assert!(
                object_format.contains(&format!("`{relocation}`")),
                "manifest relocation {relocation} is missing from object_format.md"
            );
        }
        for intrinsic in [
            "__lnp_openat",
            "__lnp_pull",
            "__lnp_push",
            "__lnp_mmap",
            "__lnp_await",
            "__lnp_gate_call",
            "__lnp_call",
            "__lnp_gate_return",
            "__lnp_domain_ctl",
            "__lnp_domain_create",
            "__lnp_object_ctl",
            "__lnp_object_create",
            "__lnp_cap_dup",
            "__lnp_cap_send",
            "__lnp_cap_recv",
            "__lnp_cap_revoke",
        ] {
            assert!(
                manifest_csv_contains(manifest, "intrinsics", intrinsic),
                "missing {intrinsic}"
            );
        }
        assert_eq!(
            manifest_field(manifest, "toy_compiler_policy"),
            "bootstrap_smoke_only_after_llvm_gate"
        );
        assert!(roadmap.contains("`CLONE` is a backend-visible native primitive"));
        assert!(roadmap.contains("new_thread_shared_vm"));
        assert!(psabi_doc.contains("## Native Clone Profiles"));
        assert!(roadmap.contains("## Toy Compiler Freeze Policy"));
        assert!(roadmap.contains("They are not the long-term application"));
        assert!(roadmap.contains("only small fixes needed to keep existing smoke"));
    }

    #[test]
    fn intrinsic_manifest_matches_target_manifest() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let intrinsic_manifest = include_str!("../toolchain/lnp64_intrinsics.manifest");
        let rows = intrinsic_rows(intrinsic_manifest);
        let mut names = std::collections::BTreeSet::new();
        let target_intrinsics: std::collections::BTreeSet<_> =
            manifest_field(target_manifest, "intrinsics")
                .split(',')
                .collect();
        let target_primitives: std::collections::BTreeSet<_> =
            manifest_field(target_manifest, "native_primitives")
                .split(',')
                .collect();

        assert_eq!(
            manifest_field(target_manifest, "intrinsic_contract"),
            "toolchain/lnp64_intrinsics.manifest"
        );
        assert_eq!(rows.len(), target_intrinsics.len());
        for (name, primitive, result, operands) in rows {
            assert!(
                name.starts_with("__lnp_"),
                "intrinsic {name} must stay in the private LNP namespace"
            );
            assert!(names.insert(name), "duplicate intrinsic {name}");
            assert!(
                target_intrinsics.contains(name),
                "intrinsic manifest names {name}, but target manifest does not"
            );
            assert!(
                target_primitives.contains(primitive),
                "intrinsic {name} maps to unknown primitive {primitive}"
            );
            assert!(!result.is_empty(), "intrinsic {name} has empty result");
            assert!(!operands.is_empty(), "intrinsic {name} has empty operands");
        }
        for name in target_intrinsics {
            assert!(
                names.contains(name),
                "target manifest intrinsic {name} is missing from intrinsic manifest"
            );
        }
    }

    #[test]
    fn private_intrinsics_do_not_expose_posix_compatibility_names() {
        let intrinsic_manifest = include_str!("../toolchain/lnp64_intrinsics.manifest");
        let forbidden = [
            "fork", "pipe", "pthread", "signal", "sig", "errno", "poll", "select", "epoll",
            "socket",
        ];

        for (name, _, _, _) in intrinsic_rows(intrinsic_manifest) {
            for word in forbidden {
                assert!(
                    !name.contains(word),
                    "private native intrinsic {name} leaks compatibility spelling {word}"
                );
            }
        }
    }

    #[test]
    fn isel_manifest_covers_backend_starting_opcode_groups() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let isel_manifest = include_str!("../toolchain/lnp64_isel.manifest");
        let asm_source = include_str!("asm.rs");
        let rows = isel_rows(isel_manifest);
        let mut groups = std::collections::BTreeSet::new();
        let mut opcodes = std::collections::BTreeSet::new();

        assert_eq!(
            manifest_field(target_manifest, "isel_contract"),
            "toolchain/lnp64_isel.manifest"
        );
        for (group, status, group_opcodes) in rows {
            assert!(groups.insert(group), "duplicate isel group {group}");
            assert!(
                ["required", "profile", "intrinsic", "bootstrap"].contains(&status),
                "unknown isel status {status}"
            );
            assert!(!group_opcodes.is_empty(), "empty isel group {group}");
            for opcode in group_opcodes {
                assert!(!opcode.is_empty(), "empty opcode in {group}");
                assert!(opcodes.insert(opcode), "duplicate isel opcode {opcode}");
                assert!(
                    asm_source.contains(&format!("\"{opcode}\"")),
                    "isel opcode {opcode} is missing from the assembler parser"
                );
            }
        }
        for group in [
            "constants",
            "integer_alu",
            "control_flow",
            "memory",
            "atomics",
            "native_primitives",
        ] {
            assert!(groups.contains(group), "missing isel group {group}");
        }
    }

    #[test]
    fn exec_plan_manifest_matches_loader_boundary_contract() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let exec_plan_manifest = include_str!("../toolchain/lnp64_exec_plan.manifest");
        let object_format = include_str!("../object_format.md");
        let rows = exec_plan_rows(exec_plan_manifest);
        let mut records = std::collections::BTreeSet::new();

        assert_eq!(
            manifest_field(target_manifest, "exec_plan_contract"),
            "toolchain/lnp64_exec_plan.manifest"
        );
        for (record, requirement, fields) in rows {
            assert!(
                records.insert(record),
                "duplicate exec-plan record {record}"
            );
            assert!(
                ["required", "optional"].contains(&requirement),
                "unknown exec-plan requirement {requirement}"
            );
            assert!(!fields.is_empty(), "empty exec-plan record {record}");
            let mut record_fields = std::collections::BTreeSet::new();
            for field in fields {
                assert!(
                    !field.is_empty(),
                    "empty field in exec-plan record {record}"
                );
                assert!(
                    record_fields.insert(field),
                    "duplicate field {field} in exec-plan record {record}"
                );
            }
        }
        for record in ["header", "entry", "vma", "fdr_grant"] {
            assert!(
                records.contains(record),
                "missing exec-plan record {record}"
            );
        }

        assert!(object_format.contains("## Exec-Plan Descriptor Boundary"));
        assert!(
            object_format.contains("exec-plan descriptor is the only object consumed by hardware")
        );
        assert!(object_format.contains("entry PC, initial SP"));
        assert!(object_format.contains("VMA records: target virtual address"));
        assert!(object_format.contains("startup FDR grants"));
        assert!(object_format.contains("old image remains"));
    }

    #[test]
    fn psabi_manifest_records_current_calling_convention_contract() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let psabi_manifest = include_str!("../toolchain/lnp64_psabi.manifest");
        let psabi_doc = include_str!("../psABI.md");

        assert_eq!(
            manifest_field(target_manifest, "psabi_contract"),
            "toolchain/lnp64_psabi.manifest"
        );
        assert_eq!(
            manifest_field(psabi_manifest, "name"),
            manifest_field(target_manifest, "call_conv")
        );
        assert_eq!(
            manifest_field(psabi_manifest, "doc"),
            manifest_field(target_manifest, "psabi")
        );
        assert_eq!(
            manifest_field(psabi_manifest, "stack_alignment"),
            manifest_field(target_manifest, "stack_alignment")
        );
        assert_eq!(manifest_field(psabi_manifest, "gpr_count"), "32");
        assert_eq!(manifest_field(psabi_manifest, "fdr_count"), "256");
        assert_eq!(manifest_field(psabi_manifest, "zero_register"), "r0");
        assert_eq!(manifest_field(psabi_manifest, "stack_pointer"), "r31");
        assert_eq!(manifest_field(psabi_manifest, "link_register"), "LR");
        assert_eq!(manifest_field(psabi_manifest, "argument_gprs"), "r1-r6");
        assert_eq!(manifest_field(psabi_manifest, "return_gprs"), "r1");
        assert_eq!(
            manifest_field(psabi_manifest, "caller_clobbered_gprs"),
            "r1-r30"
        );
        assert_eq!(manifest_field(psabi_manifest, "callee_saved_gprs"), "none");
        assert_eq!(
            manifest_field(psabi_manifest, "entry_page_base"),
            "0x700000"
        );
        assert_eq!(manifest_field(psabi_manifest, "entry_page_size"), "0x20000");
        assert_eq!(
            manifest_field(psabi_manifest, "entry_strings_base"),
            "0x701000"
        );
        assert_eq!(manifest_field(psabi_manifest, "thread_pointer_pcr"), "TP");
        assert!(manifest_csv_contains(
            psabi_manifest,
            "errno_ops",
            "ERRNO_GET"
        ));
        assert!(manifest_csv_contains(
            psabi_manifest,
            "errno_ops",
            "ERRNO_SET"
        ));
        assert!(manifest_csv_contains(
            psabi_manifest,
            "signal_return",
            "SIGRET"
        ));
        assert!(manifest_csv_contains(
            psabi_manifest,
            "signal_return",
            "GATE_RETURN"
        ));

        assert!(
            psabi_doc.contains("Integer and pointer arguments are passed in `r1` through `r6`.")
        );
        assert!(psabi_doc.contains("Return values are placed in `r1`."));
        assert!(psabi_doc.contains("There is no callee-saved GPR set"));
        assert!(psabi_doc.contains("`r31` points at the current thread's stack/local region."));
        assert!(psabi_doc.contains("The thread pointer is read and written through the `TP` PCR."));
        assert!(psabi_doc.contains("`SIGRET` is the POSIX spelling"));
        assert!(psabi_doc.contains("`GATE_RETURN`"));
    }

    #[test]
    fn debug_unwind_manifest_records_minimum_backend_contract() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let debug_unwind_manifest = include_str!("../toolchain/lnp64_debug_unwind.manifest");
        let psabi_doc = include_str!("../psABI.md");
        let roadmap = include_str!("../toolchain_roadmap.md");

        assert_eq!(
            manifest_field(target_manifest, "debug_unwind_contract"),
            "toolchain/lnp64_debug_unwind.manifest"
        );
        assert_eq!(
            manifest_field(debug_unwind_manifest, "debug_format"),
            "DWARFv5"
        );
        assert_eq!(
            manifest_field(debug_unwind_manifest, "line_tables"),
            "required"
        );
        for register in ["r0-r31", "LR", "TP"] {
            assert!(manifest_csv_contains(
                debug_unwind_manifest,
                "register_numbers",
                register
            ));
        }
        assert_eq!(
            manifest_field(debug_unwind_manifest, "stack_pointer"),
            "r31"
        );
        assert_eq!(
            manifest_field(debug_unwind_manifest, "return_address"),
            "LR"
        );
        assert_eq!(
            manifest_field(debug_unwind_manifest, "cfi"),
            "required_for_non_leaf"
        );
        assert_eq!(
            manifest_field(debug_unwind_manifest, "exception_model"),
            "none_v0"
        );
        assert_eq!(
            manifest_field(debug_unwind_manifest, "signal_unwind"),
            "psabi_signal_frame"
        );

        assert!(psabi_doc.contains("## Debug and Unwind Minimum"));
        assert!(psabi_doc.contains("There is no v0 language exception runtime"));
        assert!(roadmap.contains("toolchain/lnp64_debug_unwind.manifest"));
    }

    #[test]
    fn inline_asm_manifest_records_backend_constraints() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let inline_asm_manifest = include_str!("../toolchain/lnp64_inline_asm.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let rows = inline_asm_rows(inline_asm_manifest);
        let mut constraints = std::collections::BTreeMap::new();

        assert_eq!(
            manifest_field(target_manifest, "inline_asm_contract"),
            "toolchain/lnp64_inline_asm.manifest"
        );
        for (constraint, class, values, usage) in rows {
            assert!(!class.is_empty(), "empty inline-asm class for {constraint}");
            assert!(
                !values.is_empty(),
                "empty inline-asm values for {constraint}"
            );
            assert!(!usage.is_empty(), "empty inline-asm use for {constraint}");
            assert!(
                constraints.insert(constraint, (class, values)).is_none(),
                "duplicate inline-asm constraint {constraint}"
            );
        }

        assert_eq!(constraints["r"], ("gpr", "r0-r31"));
        assert_eq!(constraints["f"], ("fdr", "fd0-fd255"));
        assert_eq!(
            constraints["p"],
            (
                "pcr",
                "PID,PPID,TID,TP,UID,GID,SIGMASK,SIGPENDING,REALTIME_SEC,REALTIME_NSEC"
            )
        );
        assert_eq!(constraints["m"], ("memory", "base_gpr_plus_signed_offset"));
        assert_eq!(constraints["i"], ("immediate", "signed_16_or_symbolic"));
        assert!(roadmap.contains("toolchain/lnp64_inline_asm.manifest"));
    }

    #[test]
    fn crt_startup_manifest_records_process_entry_contract() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let crt_manifest = include_str!("../toolchain/lnp64_crt_startup.manifest");
        let psabi_manifest = include_str!("../toolchain/lnp64_psabi.manifest");
        let psabi_doc = include_str!("../psABI.md");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let rows = crt_startup_rows(crt_manifest);
        let mut contracts = std::collections::BTreeMap::new();

        assert_eq!(
            manifest_field(target_manifest, "crt_startup_contract"),
            "toolchain/lnp64_crt_startup.manifest"
        );
        for (item, requirement, contract) in rows {
            assert_eq!(requirement, "required", "crt startup item {item}");
            assert!(!contract.is_empty(), "empty crt startup contract {item}");
            assert!(
                contracts.insert(item, contract).is_none(),
                "duplicate crt startup item {item}"
            );
        }

        assert!(contracts["entry_symbol"].contains(&"_start"));
        assert!(contracts["main_signature"].contains(&"main(argc"));
        assert!(contracts["main_signature"].contains(&"argv"));
        assert!(contracts["main_signature"].contains(&"envp)"));
        assert!(contracts["startup_page"].contains(&"base=0x700000"));
        assert!(contracts["startup_page"].contains(&"size=0x20000"));
        assert_eq!(
            manifest_field(psabi_manifest, "entry_page_base"),
            "0x700000"
        );
        assert_eq!(manifest_field(psabi_manifest, "entry_page_size"), "0x20000");
        assert!(contracts["entry_strings"].contains(&"base=0x701000"));
        assert_eq!(
            manifest_field(psabi_manifest, "entry_strings_base"),
            "0x701000"
        );
        assert!(contracts["tls"].contains(&"thread_pointer_pcr=TP"));
        assert!(contracts["errno"].contains(&"ERRNO_GET"));
        assert!(contracts["errno"].contains(&"ERRNO_SET"));
        assert!(contracts["auxv"].contains(&"ENV_GET"));
        assert!(contracts["process_exit"].contains(&"EXIT"));

        assert!(psabi_doc.contains("If a source file defines `_start`"));
        assert!(psabi_doc.contains("For C `main`, the compiler initializes parameters specially"));
        assert!(roadmap.contains("toolchain/lnp64_crt_startup.manifest"));
    }

    #[test]
    fn relocation_manifest_matches_object_format_and_target_manifest() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let relocation_manifest = include_str!("../toolchain/lnp64_relocations.manifest");
        let object_format = include_str!("../object_format.md");
        let rows = relocation_rows(relocation_manifest);
        let target_relocations: std::collections::BTreeSet<_> =
            manifest_field(target_manifest, "relocations")
                .split(',')
                .collect();
        let mut numbers = std::collections::BTreeSet::new();
        let mut names = std::collections::BTreeSet::new();

        assert_eq!(rows.len(), 13);
        assert_eq!(
            target_relocations.len(),
            rows.len(),
            "target manifest must enumerate the complete relocation contract"
        );
        for (idx, (number, name, calculation)) in rows.iter().enumerate() {
            assert_eq!(*number as usize, idx, "relocation numbers must be dense");
            assert!(
                numbers.insert(*number),
                "duplicate relocation number {number}"
            );
            assert!(names.insert(*name), "duplicate relocation name {name}");
            assert!(!calculation.is_empty(), "empty calculation for {name}");
            assert!(
                object_format.contains(&format!("| {number} | `{name}` |")),
                "relocation {number},{name} is missing from object_format.md"
            );
            assert!(
                target_relocations.contains(name),
                "relocation manifest {name} is missing from target manifest"
            );
        }
        for name in target_relocations {
            assert!(
                names.contains(name),
                "target manifest relocation {name} is missing from relocation manifest"
            );
        }
    }

    #[test]
    fn compatibility_table_names_native_primitives() {
        assert_eq!(lowering_for(CompatSurface::Open), LOWER_OPEN);
        assert_eq!(lowering_for(CompatSurface::Read), LOWER_READ);
        assert_eq!(lowering_for(CompatSurface::Write), LOWER_WRITE);
        assert_eq!(lowering_for(CompatSurface::Close), LOWER_CLOSE);
        assert_eq!(
            lowering_for(CompatSurface::Pipe),
            &[NativePrimitive::ObjectCtl {
                kind: ObjectKind::Queue,
                profile: ObjectProfile::Pipe,
            }]
        );
        assert_eq!(
            lowering_for(CompatSurface::Fork),
            &[NativePrimitive::Clone {
                profile: CloneProfile::NewProcessCow,
            }]
        );
        assert_eq!(
            lowering_for(CompatSurface::PthreadCreate),
            &[NativePrimitive::Clone {
                profile: CloneProfile::NewThreadSharedVm,
            }]
        );
        assert!(lowering_for(CompatSurface::Exec).contains(&NativePrimitive::Exec));
        assert!(lowering_for(CompatSurface::Mmap).contains(&NativePrimitive::Mmap));
        assert!(lowering_for(CompatSurface::FdPassing).contains(&NativePrimitive::CapabilitySend));
        assert!(lowering_for(CompatSurface::SocketLoopback).contains(&NativePrimitive::Await));
        assert!(lowering_for(CompatSurface::Timer).contains(&NativePrimitive::EventDelivery));
        assert!(lowering_for(CompatSurface::CallGate).contains(&NativePrimitive::GateReturn));
        assert!(lowering_for(CompatSurface::Signal).contains(&NativePrimitive::EventDelivery));
        assert!(lowering_for(CompatSurface::Errno).contains(&NativePrimitive::TlsErrnoView));
        assert!(lowering_for(CompatSurface::ResourceDomain).contains(&NativePrimitive::DomainCtl));
        assert_eq!(
            lowering_for(CompatSurface::Stat),
            &[NativePrimitive::Metadata(MetadataOp::GetMeta)]
        );
        assert_eq!(
            lowering_for(CompatSurface::Chmod),
            &[NativePrimitive::Metadata(MetadataOp::SetMeta)]
        );
        assert_eq!(
            lowering_for(CompatSurface::Fcntl),
            &[
                NativePrimitive::Metadata(MetadataOp::GetMeta),
                NativePrimitive::Metadata(MetadataOp::SetMeta),
                NativePrimitive::Metadata(MetadataOp::ObjectCtl),
            ]
        );
    }

    #[test]
    fn compatibility_lowering_pins_native_architecture_boundaries() {
        assert_eq!(
            lowering_for(CompatSurface::PollSelectEpoll),
            &[
                NativePrimitive::EventQueue,
                NativePrimitive::Await,
                NativePrimitive::Pull,
            ]
        );
        assert_eq!(
            lowering_for(CompatSurface::Fork),
            &[NativePrimitive::Clone {
                profile: CloneProfile::NewProcessCow,
            }]
        );
        assert_eq!(
            lowering_for(CompatSurface::PthreadCreate),
            &[NativePrimitive::Clone {
                profile: CloneProfile::NewThreadSharedVm,
            }]
        );
        assert_eq!(
            lowering_for(CompatSurface::Signal),
            &[
                NativePrimitive::EventDelivery,
                NativePrimitive::AbiSignalFrame,
            ]
        );
        assert_eq!(
            lowering_for(CompatSurface::Errno),
            &[
                NativePrimitive::ExplicitResult,
                NativePrimitive::TlsErrnoView,
            ]
        );
    }

    #[test]
    fn netbsd_system_gate_surfaces_are_registered() {
        let surfaces = [
            CompatSurface::CwdRoot,
            CompatSurface::Open,
            CompatSurface::Read,
            CompatSurface::Write,
            CompatSurface::Close,
            CompatSurface::Pipe,
            CompatSurface::PollSelectEpoll,
            CompatSurface::Fork,
            CompatSurface::Exec,
            CompatSurface::PthreadCreate,
            CompatSurface::Mmap,
            CompatSurface::FdPassing,
            CompatSurface::SocketLoopback,
            CompatSurface::Timer,
            CompatSurface::CallGate,
            CompatSurface::Signal,
            CompatSurface::ResourceDomain,
            CompatSurface::Errno,
        ];
        for surface in surfaces {
            assert!(
                !lowering_for(surface).is_empty(),
                "missing lowering for {surface:?}"
            );
        }
    }

    #[test]
    fn netbsd_system_gate_canonical_native_primitives_cover_runner_requirements() {
        fn gate_has(
            surfaces: &[CompatSurface],
            mut required: impl FnMut(&NativePrimitive) -> bool,
        ) -> bool {
            surfaces
                .iter()
                .flat_map(|surface| lowering_for(*surface))
                .any(|primitive| required(primitive))
        }

        let surfaces = [
            CompatSurface::CwdRoot,
            CompatSurface::Open,
            CompatSurface::Read,
            CompatSurface::Write,
            CompatSurface::Close,
            CompatSurface::Pipe,
            CompatSurface::PollSelectEpoll,
            CompatSurface::Fork,
            CompatSurface::Exec,
            CompatSurface::PthreadCreate,
            CompatSurface::Mmap,
            CompatSurface::FdPassing,
            CompatSurface::SocketLoopback,
            CompatSurface::Timer,
            CompatSurface::CallGate,
            CompatSurface::Signal,
            CompatSurface::ResourceDomain,
        ];

        assert!(gate_has(&surfaces, |primitive| *primitive == NativePrimitive::OpenAt));
        assert!(gate_has(&surfaces, |primitive| *primitive == NativePrimitive::Pull));
        assert!(gate_has(&surfaces, |primitive| *primitive == NativePrimitive::Push));
        assert!(gate_has(&surfaces, |primitive| *primitive == NativePrimitive::Close));
        assert!(gate_has(&surfaces, |primitive| matches!(
            primitive,
            NativePrimitive::ObjectCtl { .. }
        )));
        assert!(gate_has(&surfaces, |primitive| *primitive == NativePrimitive::Await));
        assert!(gate_has(&surfaces, |primitive| *primitive == NativePrimitive::Exec));
        assert!(gate_has(&surfaces, |primitive| *primitive == NativePrimitive::Mmap));
        assert!(gate_has(&surfaces, |primitive| *primitive
            == NativePrimitive::Mprotect));
        assert!(gate_has(&surfaces, |primitive| *primitive == NativePrimitive::Munmap));
        assert!(gate_has(&surfaces, |primitive| *primitive
            == NativePrimitive::CapabilityDuplicate));
        assert!(gate_has(&surfaces, |primitive| *primitive
            == NativePrimitive::CapabilitySend));
        assert!(gate_has(&surfaces, |primitive| *primitive
            == NativePrimitive::CapabilityRecv));
        assert!(gate_has(&surfaces, |primitive| *primitive
            == NativePrimitive::DomainCtl));
        assert!(gate_has(&surfaces, |primitive| *primitive
            == NativePrimitive::GateCall));
        assert!(gate_has(&surfaces, |primitive| *primitive
            == NativePrimitive::GateReturn));
        assert!(gate_has(&surfaces, |primitive| {
            *primitive
                == NativePrimitive::Clone {
                    profile: CloneProfile::NewProcessCow,
                }
        }));
        assert!(gate_has(&surfaces, |primitive| {
            *primitive
                == NativePrimitive::Clone {
                    profile: CloneProfile::NewThreadSharedVm,
                }
        }));
        assert!(gate_has(&surfaces, |primitive| *primitive
            == NativePrimitive::EventDelivery));
        assert!(gate_has(&surfaces, |primitive| *primitive
            == NativePrimitive::AbiSignalFrame));
    }

    #[test]
    fn compatibility_surfaces_have_layer_policy() {
        for entry in COMPATIBILITY_LOWERINGS {
            assert!(
                layer_for(entry.surface).is_some(),
                "missing layer policy for {:?}",
                entry.surface
            );
        }
        for (idx, entry) in COMPATIBILITY_LOWERINGS.iter().enumerate() {
            assert!(
                !COMPATIBILITY_LOWERINGS[..idx]
                    .iter()
                    .any(|seen| seen.surface == entry.surface),
                "duplicate lowering for {:?}",
                entry.surface
            );
        }
        for policy in COMPATIBILITY_SURFACE_POLICIES {
            assert!(
                !lowering_for(policy.surface).is_empty(),
                "missing lowering for policy surface {:?}",
                policy.surface
            );
        }
        for (idx, policy) in COMPATIBILITY_SURFACE_POLICIES.iter().enumerate() {
            assert!(
                !COMPATIBILITY_SURFACE_POLICIES[..idx]
                    .iter()
                    .any(|seen| seen.surface == policy.surface),
                "duplicate layer policy for {:?}",
                policy.surface
            );
        }
        assert_eq!(
            layer_for(CompatSurface::Errno),
            Some(CompatibilityLayer::RuntimeLibc)
        );
        assert_eq!(
            layer_for(CompatSurface::Signal),
            Some(CompatibilityLayer::Personality)
        );
        assert_eq!(
            layer_for(CompatSurface::ResourceDomain),
            Some(CompatibilityLayer::Native)
        );
    }

    #[test]
    fn netbsd_syscall_numbers_route_to_compat_surfaces() {
        assert_eq!(
            netbsd_syscall(2).map(|entry| entry.surface),
            Some(CompatSurface::Fork)
        );
        assert_eq!(
            netbsd_syscall(3).map(|entry| entry.surface),
            Some(CompatSurface::Read)
        );
        assert_eq!(
            netbsd_syscall(4).map(|entry| entry.surface),
            Some(CompatSurface::Write)
        );
        assert_eq!(
            netbsd_syscall(5).map(|entry| entry.surface),
            Some(CompatSurface::Open)
        );
        assert_eq!(
            netbsd_syscall(42).map(|entry| entry.surface),
            Some(CompatSurface::Pipe)
        );
        assert_eq!(
            netbsd_syscall(197).map(|entry| entry.surface),
            Some(CompatSurface::Mmap)
        );
        assert_eq!(
            netbsd_syscall(340).map(|entry| entry.surface),
            Some(CompatSurface::Signal)
        );
        assert_eq!(
            netbsd_syscall(468).map(|entry| entry.surface),
            Some(CompatSurface::Open)
        );
        assert!(netbsd_syscall_lowering(54).is_empty());
    }

    #[test]
    fn netbsd_syscall_dispatch_is_layered_over_native_lowerings() {
        for entry in NETBSD_SYSCALLS {
            assert_eq!(
                Some(entry.layer),
                layer_for(entry.surface),
                "layer mismatch for {}",
                entry.name
            );
            assert!(
                !netbsd_syscall_lowering(entry.number).is_empty(),
                "missing native lowering for {}",
                entry.name
            );
        }
    }

    #[test]
    fn netbsd_system_gate_syscalls_are_registered() {
        let names = [
            "fork",
            "read",
            "write",
            "open",
            "openat",
            "close",
            "compat_50_wait4",
            "__wait450",
            "chdir",
            "fchdir",
            "__getcwd",
            "chmod",
            "dup",
            "dup2",
            "fcntl",
            "pipe",
            "pipe2",
            "execve",
            "fexecve",
            "mmap",
            "mprotect",
            "munmap",
            "poll",
            "__select50",
            "epoll_create1",
            "epoll_ctl",
            "epoll_pwait2",
            "timerfd_create",
            "timerfd_settime",
            "timerfd_gettime",
            "__nanosleep50",
            "_lwp_create",
            "__socket30",
            "bind",
            "listen",
            "connect",
            "accept",
            "recvfrom",
            "sendto",
            "sendmsg",
            "recvmsg",
            "getsockname",
            "getsockopt",
            "setsockopt",
            "__sigaction_sigtramp",
            "__sigprocmask14",
            "kill",
            "compat_16___sigreturn14",
        ];
        for name in names {
            assert!(
                netbsd_syscall_by_name(name).is_some(),
                "missing NetBSD syscall dispatch entry for {name}"
            );
        }
    }
}

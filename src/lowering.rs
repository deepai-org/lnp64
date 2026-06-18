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

    fn relocation_rows(manifest: &str) -> Vec<(u16, &str, &str, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(4, ',');
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
                let loader_status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing relocation loader status in {line}"));
                (number, name, calculation, loader_status)
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

    fn mc_encoding_rows(
        manifest: &str,
    ) -> Vec<(&str, &str, Vec<&str>, &str, Vec<&str>, Vec<&str>)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(6, '|');
                let group = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing MC encoding group in {line}"));
                let format = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing MC encoding format in {line}"));
                let opcodes = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing MC encoding opcodes in {line}"))
                    .split(',')
                    .collect();
                let operands = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing MC encoding operands in {line}"));
                let relocations = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing MC encoding relocations in {line}"))
                    .split(',')
                    .collect();
                let surfaces = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing MC encoding surfaces in {line}"))
                    .split(',')
                    .collect();
                (group, format, opcodes, operands, relocations, surfaces)
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

    fn loader_security_rows(manifest: &str) -> Vec<(&str, &str, Vec<&str>, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(4, '|');
                let requirement = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing loader security requirement in {line}"));
                let boundary = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing loader security boundary in {line}"));
                let evidence = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing loader security evidence in {line}"))
                    .split(',')
                    .collect();
                let status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing loader security status in {line}"));
                (requirement, boundary, evidence, status)
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

    fn register_class_rows(manifest: &str) -> Vec<(&str, &str, &str, &str, Vec<&str>, &str, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(7, '|');
                let class = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing register class in {line}"));
                let values = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing register values in {line}"));
                let width = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing register width in {line}"));
                let allocatable = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing allocatable register set in {line}"));
                let reserved = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing reserved register set in {line}"))
                    .split(',')
                    .collect();
                let role = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing register role in {line}"));
                let debug = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing debug register role in {line}"));
                (class, values, width, allocatable, reserved, role, debug)
            })
            .collect()
    }

    fn netbsd_layer_rows(manifest: &str) -> Vec<(&str, &str, Vec<&str>, &str, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(5, '|');
                let layer = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing NetBSD layer in {line}"));
                let status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing NetBSD layer status in {line}"));
                let artifacts = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing NetBSD layer artifacts in {line}"))
                    .split(',')
                    .collect();
                let gate = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing NetBSD layer gate in {line}"));
                let next_blocker = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing NetBSD layer blocker in {line}"));
                (layer, status, artifacts, gate, next_blocker)
            })
            .collect()
    }

    fn conformance_gate_rows(manifest: &str) -> Vec<(&str, &str, Vec<&str>, &str, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(5, '|');
                let category = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing conformance category in {line}"));
                let status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing conformance status in {line}"));
                let artifacts = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing conformance artifacts in {line}"))
                    .split(',')
                    .collect();
                let gate = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing conformance gate in {line}"));
                let coverage = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing conformance coverage in {line}"));
                (category, status, artifacts, gate, coverage)
            })
            .collect()
    }

    fn toy_compiler_policy_rows(manifest: &str) -> Vec<(&str, &str, Vec<&str>, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(4, '|');
                let rule = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing toy compiler policy rule in {line}"));
                let status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing toy compiler policy status in {line}"));
                let artifacts = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing toy compiler policy artifacts in {line}"))
                    .split(',')
                    .collect();
                let evidence = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing toy compiler policy evidence in {line}"));
                (rule, status, artifacts, evidence)
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

    fn llvm_gate_rows(manifest: &str) -> Vec<(&str, &str, Vec<&str>, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(4, '|');
                let gate = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm gate name in {line}"));
                let command = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm gate command in {line}"));
                let requires = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm gate requirements in {line}"))
                    .split(',')
                    .collect();
                let status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm gate status in {line}"));
                (gate, command, requires, status)
            })
            .collect()
    }

    fn run_elf_rows(manifest: &str) -> Vec<(&str, &str, Vec<&str>, &str, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(5, '|');
                let stage = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing run-elf stage in {line}"));
                let status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing run-elf status in {line}"));
                let artifacts = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing run-elf artifacts in {line}"))
                    .split(',')
                    .collect();
                let evidence = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing run-elf evidence in {line}"));
                let blocker = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing run-elf blocker in {line}"));
                (stage, status, artifacts, evidence, blocker)
            })
            .collect()
    }

    fn llvm_filemap_rows(manifest: &str) -> Vec<(&str, &str, &str, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(4, '|');
                let layer = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm filemap layer in {line}"));
                let path = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm filemap path in {line}"));
                let status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm filemap status in {line}"));
                let purpose = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing llvm filemap purpose in {line}"));
                (layer, path, status, purpose)
            })
            .collect()
    }

    fn libc_shim_rows(manifest: &str) -> Vec<(&str, Vec<&str>, Vec<&str>, Vec<&str>, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(5, '|');
                let group = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing libc shim group in {line}"));
                let public_surface = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing libc shim public surface in {line}"))
                    .split(',')
                    .collect();
                let native_lowering = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing libc shim native lowering in {line}"))
                    .split(',')
                    .collect();
                let evidence = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing libc shim evidence in {line}"))
                    .split(',')
                    .collect();
                let status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing libc shim status in {line}"));
                (group, public_surface, native_lowering, evidence, status)
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
            "registers",
            "relocations",
            "mc_encoding",
            "psabi",
            "intrinsics",
            "intrinsic_header",
            "clang_driver",
            "llvm_filemap",
            "libc_shim",
            "netbsd_layers",
            "conformance_gates",
            "toy_compiler_policy",
            "isel",
            "llvm_bootstrap",
            "llvm_gates",
            "run_elf",
            "linker_script",
            "exec_plan",
            "loader_security",
            "loader",
            "exec_descriptor_validator",
            "debug_unwind",
            "inline_asm",
            "crt_startup",
            "crt0",
            "transition",
        ] {
            assert!(names.contains(name), "missing contract index row {name}");
        }
    }

    #[test]
    fn llvm_gate_manifest_pins_non_toy_clang_commands() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let gate_manifest = include_str!("../toolchain/lnp64_llvm_gates.manifest");
        let gate_driver = include_str!("../scripts/run_llvm_bootstrap_gates.sh");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let rows = llvm_gate_rows(gate_manifest);
        let mut gates = std::collections::BTreeSet::new();
        let mut commands = std::collections::BTreeMap::new();
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

        assert_eq!(
            manifest_field(target_manifest, "llvm_gate_contract"),
            "toolchain/lnp64_llvm_gates.manifest"
        );
        assert!(
            manifest_root
                .join("scripts/run_llvm_bootstrap_gates.sh")
                .is_file()
        );
        assert!(contract_index.contains(
            "llvm_gates|toolchain/lnp64_llvm_gates.manifest|llvm_gate_manifest_pins_non_toy_clang_commands"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_llvm_gates.manifest"));
        assert!(transition_manifest.contains("scripts/run_llvm_bootstrap_gates.sh"));
        assert!(roadmap.contains("toolchain/lnp64_llvm_gates.manifest"));
        assert!(roadmap.contains("scripts/run_llvm_bootstrap_gates.sh --dry-run"));

        for (gate, command, requirements, status) in rows {
            assert!(gates.insert(gate), "duplicate llvm gate {gate}");
            commands.insert(gate, command);
            assert_eq!(
                status, "planned",
                "gate {gate} must stay planned until real Clang/lld/loader execution exists"
            );
            assert!(!command.is_empty(), "empty llvm gate command for {gate}");
            assert!(
                !requirements.is_empty(),
                "empty llvm gate requirements for {gate}"
            );
            assert!(
                !command.contains("lnp64 cc") && !command.contains("cargo run -- cc"),
                "llvm gate {gate} must not use the toy compiler command"
            );
            assert!(
                !command.contains("src/c_compiler"),
                "llvm gate {gate} must not route through the in-repo C compiler"
            );
        }

        for gate in [
            "gate_driver",
            "compile_hello",
            "compile_arithmetic",
            "compile_memory",
            "compile_calls",
            "assemble_crt0",
            "link_static",
            "inspect_exec_plan",
            "run_without_toy_compiler",
            "simple_libc_gate",
        ] {
            assert!(gates.contains(gate), "missing llvm gate {gate}");
        }
        assert!(
            commands["gate_driver"].contains("scripts/run_llvm_bootstrap_gates.sh --dry-run"),
            "llvm gate driver must expose the dry-run script"
        );
        assert!(gate_driver.contains("toolchain/lnp64_llvm_gates.manifest"));
        assert!(gate_driver.contains("--dry-run"));
        assert!(gate_driver.contains("--run"));
        assert!(gate_driver.contains("LNP64_RUN_PLANNED_LLVM_GATES"));
        assert!(gate_driver.contains(r"command//\{build\}/"));
        assert!(!gate_driver.contains("lnp64 cc"));
        assert!(!gate_driver.contains("cargo run -- cc"));
        assert!(
            commands["link_static"].contains("-T toolchain/lnp64_static.ld"),
            "static link gate must use checked LNP64 linker script"
        );
        assert!(
            commands["run_without_toy_compiler"].contains("lnp64 run-elf"),
            "no-toy execution gate must route through the checked run-elf boundary"
        );
        assert!(
            commands["assemble_crt0"].contains("toolchain/crt0_lnp64.s"),
            "crt0 gate must assemble checked startup stub"
        );
        for gate in [
            "compile_hello",
            "compile_arithmetic",
            "compile_memory",
            "compile_calls",
        ] {
            assert!(
                commands[gate].contains("-I toolchain"),
                "{gate} must include checked private intrinsic header path"
            );
        }
    }

    #[test]
    fn static_linker_script_records_loader_mapping_contract() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let linker_script = include_str!("../toolchain/lnp64_static.ld");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let object_format = include_str!("../object_format.md");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let linker_path = manifest_field(target_manifest, "linker_script_contract");

        assert_eq!(linker_path, "toolchain/lnp64_static.ld");
        assert!(manifest_root.join(linker_path).is_file());
        assert!(contract_index.contains(
            "linker_script|toolchain/lnp64_static.ld|static_linker_script_records_loader_mapping_contract"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_static.ld"));
        assert!(roadmap.contains("toolchain/lnp64_static.ld"));

        for required in [
            "OUTPUT_ARCH(lnp64)",
            "ENTRY(_start)",
            "PHDRS",
            "PT_LOAD",
            "PT_TLS",
            "PT_NOTE",
            ".text",
            ".rodata",
            ".data",
            ".bss",
            ".tdata",
            ".tbss",
            ".note.lnp64.startup",
            ".note.lnp64.capreq",
        ] {
            assert!(
                linker_script.contains(required),
                "linker script missing {required}"
            );
        }
        for section in [
            ".text",
            ".rodata",
            ".data",
            ".bss",
            ".tdata",
            ".tbss",
            ".note.lnp64.startup",
            ".note.lnp64.capreq",
        ] {
            assert!(
                object_format.contains(section),
                "object format missing linked section {section}"
            );
        }
        assert!(
            !linker_script.contains("PT_DYNAMIC"),
            "static v0 linker script must not emit PT_DYNAMIC"
        );
    }

    #[test]
    fn clang_driver_manifest_matches_llvm_gates() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let driver_manifest = include_str!("../toolchain/lnp64_clang_driver.manifest");
        let gate_manifest = include_str!("../toolchain/lnp64_llvm_gates.manifest");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let driver_path = manifest_field(target_manifest, "clang_driver_contract");

        assert_eq!(driver_path, "toolchain/lnp64_clang_driver.manifest");
        assert!(manifest_root.join(driver_path).is_file());
        assert!(contract_index.contains(
            "clang_driver|toolchain/lnp64_clang_driver.manifest|clang_driver_manifest_matches_llvm_gates"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_clang_driver.manifest"));
        assert!(roadmap.contains("toolchain/lnp64_clang_driver.manifest"));
        assert_eq!(
            manifest_field(driver_manifest, "triple"),
            manifest_field(target_manifest, "triple")
        );
        for flag in ["-ffreestanding", "-fno-pic", "-Itoolchain"] {
            assert!(
                manifest_csv_contains(driver_manifest, "cflags", flag),
                "driver cflags missing {flag}"
            );
        }
        assert_eq!(manifest_field(driver_manifest, "assembler"), "llvm-mc");
        assert!(manifest_csv_contains(
            driver_manifest,
            "assembler_flags",
            "-triple=lnp64-unknown-none"
        ));
        assert!(manifest_csv_contains(
            driver_manifest,
            "assembler_flags",
            "-filetype=obj"
        ));
        assert_eq!(manifest_field(driver_manifest, "linker"), "ld.lld");
        for flag in [
            "-static",
            "-m",
            "elf64lnp64",
            "-T",
            "toolchain/lnp64_static.ld",
        ] {
            assert!(
                manifest_csv_contains(driver_manifest, "linker_flags", flag),
                "driver linker flags missing {flag}"
            );
        }
        assert_eq!(
            manifest_field(driver_manifest, "crt0"),
            "toolchain/crt0_lnp64.s"
        );
        assert_eq!(
            manifest_field(driver_manifest, "intrinsic_header"),
            "toolchain/lnp64_intrinsics.h"
        );
        assert_eq!(
            manifest_field(driver_manifest, "loader_probe"),
            "lnp64 elf-plan"
        );
        assert_eq!(
            manifest_field(driver_manifest, "status"),
            "planned_until_backend"
        );

        assert!(gate_manifest.contains("clang --target=lnp64-unknown-none"));
        assert!(gate_manifest.contains("-ffreestanding -fno-pic -I toolchain"));
        assert!(gate_manifest.contains("llvm-mc -triple=lnp64-unknown-none"));
        assert!(gate_manifest.contains("toolchain/crt0_lnp64.s"));
        assert!(gate_manifest.contains("ld.lld -static -m elf64lnp64"));
        assert!(gate_manifest.contains("-T toolchain/lnp64_static.ld"));
        assert!(gate_manifest.contains("lnp64 elf-plan"));
    }

    #[test]
    fn run_elf_manifest_records_execution_boundary() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let run_elf_manifest = include_str!("../toolchain/lnp64_run_elf.manifest");
        let gate_manifest = include_str!("../toolchain/lnp64_llvm_gates.manifest");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let conformance = include_str!("../conformance_matrix.md");
        let loader_security = include_str!("../toolchain/lnp64_loader_security.manifest");
        let main_source = include_str!("main.rs");
        let loader_source = include_str!("loader.rs");
        let emulator_source = include_str!("emulator.rs");
        let lowering_source = include_str!("lowering.rs");
        let evidence_corpus =
            format!("{main_source}\n{loader_source}\n{emulator_source}\n{lowering_source}");
        let rows = run_elf_rows(run_elf_manifest);
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut stages = std::collections::BTreeMap::new();

        assert_eq!(
            manifest_field(target_manifest, "run_elf_contract"),
            "toolchain/lnp64_run_elf.manifest"
        );
        assert!(contract_index.contains(
            "run_elf|toolchain/lnp64_run_elf.manifest|run_elf_manifest_records_execution_boundary"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_run_elf.manifest"));
        assert!(roadmap.contains("toolchain/lnp64_run_elf.manifest"));
        assert!(conformance.contains("toolchain/lnp64_run_elf.manifest"));
        assert!(gate_manifest.contains("lnp64 run-elf"));
        assert!(main_source.contains("\"run-elf\""));
        assert!(main_source.contains("ELF text fetch/decode is not implemented yet"));
        assert!(loader_security.contains("submit_exec_plan"));
        assert!(
            loader_security.contains("emulator_commits_exec_descriptor_memory_image_atomically")
        );

        for (stage, status, artifacts, evidence, blocker) in rows {
            assert!(
                stages
                    .insert(stage, (status, artifacts.clone(), evidence, blocker))
                    .is_none(),
                "duplicate run-elf stage {stage}"
            );
            assert!(
                ["tested", "partial", "planned"].contains(&status),
                "unknown run-elf status {status} for {stage}"
            );
            assert!(
                !artifacts.is_empty(),
                "empty artifacts for run-elf stage {stage}"
            );
            for artifact in artifacts {
                assert!(
                    manifest_root.join(artifact).exists(),
                    "run-elf stage {stage} names missing artifact {artifact}"
                );
            }
            assert!(!evidence.is_empty(), "empty run-elf evidence for {stage}");
            if status == "tested" {
                assert_eq!(blocker, "none", "tested run-elf stage {stage} has blocker");
                assert!(
                    evidence_corpus.contains(evidence),
                    "tested run-elf evidence {evidence} for {stage} is not present"
                );
            } else {
                assert_ne!(
                    blocker, "none",
                    "unfinished run-elf stage {stage} lacks blocker"
                );
            }
        }

        for stage in [
            "load_static_elf",
            "materialize_vmas",
            "descriptor_validate",
            "descriptor_commit",
            "cli_probe",
            "cli_surface",
            "entry_state",
            "text_fetch_decode",
            "stdout_exit",
            "no_toy_compiler",
        ] {
            assert!(stages.contains_key(stage), "missing run-elf stage {stage}");
        }
        for stage in [
            "load_static_elf",
            "materialize_vmas",
            "descriptor_validate",
            "descriptor_commit",
            "cli_probe",
        ] {
            assert_eq!(stages[stage].0, "tested", "{stage} should be tested");
        }
        assert_eq!(stages["entry_state"].0, "partial");
        assert_eq!(stages["cli_surface"].0, "partial");
        for stage in ["text_fetch_decode", "stdout_exit", "no_toy_compiler"] {
            assert_eq!(
                stages[stage].0, "planned",
                "{stage} must stay planned until ELF execution exists"
            );
        }
        assert!(roadmap.contains("run_without_toy_compiler` gate remains planned"));
    }

    #[test]
    fn llvm_filemap_manifest_names_backend_source_surface() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let filemap_manifest = include_str!("../toolchain/lnp64_llvm_filemap.manifest");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let rows = llvm_filemap_rows(filemap_manifest);
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let filemap_path = manifest_field(target_manifest, "llvm_filemap_contract");
        let mut layers = std::collections::BTreeSet::new();
        let mut paths = std::collections::BTreeSet::new();
        let mut purposes = Vec::new();
        let mut statuses = std::collections::BTreeMap::new();

        assert_eq!(filemap_path, "toolchain/lnp64_llvm_filemap.manifest");
        assert!(manifest_root.join(filemap_path).is_file());
        assert!(contract_index.contains(
            "llvm_filemap|toolchain/lnp64_llvm_filemap.manifest|llvm_filemap_manifest_names_backend_source_surface"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_llvm_filemap.manifest"));
        assert!(roadmap.contains("toolchain/lnp64_llvm_filemap.manifest"));

        for (layer, path, status, purpose) in rows {
            layers.insert(layer);
            assert!(paths.insert(path), "duplicate llvm-project path {path}");
            statuses.insert(path, status);
            assert!(
                ["planned", "scaffolded"].contains(&status),
                "unknown llvm-project status {status} for {path}"
            );
            if status == "scaffolded" {
                assert!(
                    manifest_root.join(path).is_file(),
                    "scaffolded llvm-project file {path} is missing"
                );
            }
            assert!(
                path.starts_with("llvm/") || path.starts_with("clang/") || path.starts_with("lld/"),
                "llvm filemap path {path} must name an llvm-project source tree path"
            );
            assert!(
                !purpose.is_empty(),
                "llvm filemap path {path} must describe its purpose"
            );
            purposes.push(purpose);
        }

        for layer in [
            "llvm_target",
            "llvm_mc",
            "llvm_asmparser",
            "llvm_disassembler",
            "llvm_targetinfo",
            "lld",
            "clang_basic",
            "clang_driver",
            "llvm_tests",
            "clang_tests",
        ] {
            assert!(layers.contains(layer), "missing llvm filemap layer {layer}");
        }
        for path in [
            "llvm/lib/Target/LNP64/CMakeLists.txt",
            "llvm/lib/Target/LNP64/LNP64.td",
            "llvm/lib/Target/LNP64/LNP64RegisterInfo.td",
            "llvm/lib/Target/LNP64/LNP64InstrInfo.td",
            "llvm/lib/Target/LNP64/LNP64CallingConv.td",
            "llvm/lib/Target/LNP64/LNP64TargetMachine.cpp",
            "llvm/lib/Target/LNP64/LNP64Subtarget.cpp",
            "llvm/lib/Target/LNP64/LNP64InstrInfo.cpp",
            "llvm/lib/Target/LNP64/LNP64RegisterInfo.cpp",
            "llvm/lib/Target/LNP64/LNP64ISelLowering.cpp",
            "llvm/lib/Target/LNP64/LNP64FrameLowering.cpp",
            "llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCTargetDesc.cpp",
            "llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCCodeEmitter.cpp",
            "llvm/lib/Target/LNP64/AsmParser/LNP64AsmParser.cpp",
            "llvm/lib/Target/LNP64/Disassembler/LNP64Disassembler.cpp",
            "llvm/lib/Target/LNP64/TargetInfo/LNP64TargetInfo.cpp",
            "lld/ELF/Arch/LNP64.cpp",
            "clang/lib/Basic/Targets/LNP64.h",
            "clang/lib/Basic/Targets/LNP64.cpp",
            "clang/lib/Driver/ToolChains/Arch/LNP64.cpp",
            "llvm/test/CodeGen/LNP64/hello.ll",
            "llvm/test/MC/LNP64/basic.s",
            "clang/test/Driver/lnp64.c",
        ] {
            assert!(paths.contains(path), "missing llvm filemap path {path}");
        }
        for path in [
            "llvm/lib/Target/LNP64/CMakeLists.txt",
            "llvm/lib/Target/LNP64/LNP64.td",
            "llvm/lib/Target/LNP64/LNP64RegisterInfo.td",
            "llvm/lib/Target/LNP64/LNP64InstrInfo.td",
            "llvm/lib/Target/LNP64/LNP64CallingConv.td",
            "llvm/lib/Target/LNP64/LNP64TargetMachine.cpp",
            "llvm/lib/Target/LNP64/LNP64Subtarget.cpp",
            "llvm/lib/Target/LNP64/LNP64InstrInfo.cpp",
            "llvm/lib/Target/LNP64/LNP64RegisterInfo.cpp",
            "llvm/lib/Target/LNP64/LNP64ISelLowering.cpp",
            "llvm/lib/Target/LNP64/LNP64FrameLowering.cpp",
            "llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCTargetDesc.cpp",
            "llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCCodeEmitter.cpp",
            "llvm/lib/Target/LNP64/AsmParser/LNP64AsmParser.cpp",
            "llvm/lib/Target/LNP64/Disassembler/LNP64Disassembler.cpp",
            "llvm/lib/Target/LNP64/TargetInfo/LNP64TargetInfo.cpp",
            "lld/ELF/Arch/LNP64.cpp",
            "clang/lib/Basic/Targets/LNP64.h",
            "clang/lib/Basic/Targets/LNP64.cpp",
            "clang/lib/Driver/ToolChains/Arch/LNP64.cpp",
            "llvm/test/CodeGen/LNP64/hello.ll",
            "llvm/test/MC/LNP64/basic.s",
            "clang/test/Driver/lnp64.c",
        ] {
            assert_eq!(statuses[path], "scaffolded", "{path} should be scaffolded");
        }
        for concept in [
            "register",
            "calling",
            "relocation",
            "inline asm",
            "driver",
            "static",
            "no toy compiler",
        ] {
            assert!(
                purposes.iter().any(|purpose| purpose.contains(concept)),
                "llvm filemap must cover {concept}"
            );
        }
        let target_td = include_str!("../llvm/lib/Target/LNP64/LNP64.td");
        let registers_td = include_str!("../llvm/lib/Target/LNP64/LNP64RegisterInfo.td");
        let calling_td = include_str!("../llvm/lib/Target/LNP64/LNP64CallingConv.td");
        let instr_td = include_str!("../llvm/lib/Target/LNP64/LNP64InstrInfo.td");
        let instr_info = include_str!("../llvm/lib/Target/LNP64/LNP64InstrInfo.cpp");
        let cmake = include_str!("../llvm/lib/Target/LNP64/CMakeLists.txt");
        let target_info = include_str!("../llvm/lib/Target/LNP64/TargetInfo/LNP64TargetInfo.cpp");
        let mc_desc_header =
            include_str!("../llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCTargetDesc.h");
        let mc_desc_cmake = include_str!("../llvm/lib/Target/LNP64/MCTargetDesc/CMakeLists.txt");
        let mc_desc = include_str!("../llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCTargetDesc.cpp");
        let mc_emitter =
            include_str!("../llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCCodeEmitter.cpp");
        let mc_asm_backend =
            include_str!("../llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCAsmBackend.cpp");
        let inst_printer =
            include_str!("../llvm/lib/Target/LNP64/InstPrinter/LNP64InstPrinter.cpp");
        let asm_parser = include_str!("../llvm/lib/Target/LNP64/AsmParser/LNP64AsmParser.cpp");
        let disassembler =
            include_str!("../llvm/lib/Target/LNP64/Disassembler/LNP64Disassembler.cpp");
        let target_machine = include_str!("../llvm/lib/Target/LNP64/LNP64TargetMachine.cpp");
        let subtarget = include_str!("../llvm/lib/Target/LNP64/LNP64Subtarget.cpp");
        let isel = include_str!("../llvm/lib/Target/LNP64/LNP64ISelLowering.cpp");
        let isel_header = include_str!("../llvm/lib/Target/LNP64/LNP64ISelLowering.h");
        let frame = include_str!("../llvm/lib/Target/LNP64/LNP64FrameLowering.cpp");
        let reginfo = include_str!("../llvm/lib/Target/LNP64/LNP64RegisterInfo.cpp");
        let clang_target = include_str!("../clang/lib/Basic/Targets/LNP64.cpp");
        let clang_driver = include_str!("../clang/lib/Driver/ToolChains/Arch/LNP64.cpp");
        let lld_arch = include_str!("../lld/ELF/Arch/LNP64.cpp");
        let codegen_test = include_str!("../llvm/test/CodeGen/LNP64/hello.ll");
        let mc_test = include_str!("../llvm/test/MC/LNP64/basic.s");
        let clang_driver_test = include_str!("../clang/test/Driver/lnp64.c");

        assert!(target_td.contains("def LNP64 : Target"));
        for required in ["GPR", "FDR", "FPR", "VR", "PCR", "LR", "FLAGS", "R31"] {
            assert!(
                registers_td.contains(required),
                "register TableGen missing {required}"
            );
        }
        assert!(registers_td.contains(r#"sequence "FD%u", 0, 255"#));
        assert!(calling_td.contains("CC_LNP64"));
        assert!(calling_td.contains("R1, R2, R3, R4, R5, R6"));
        for opcode in [
            "ADD",
            "LD",
            "CALL",
            "RET",
            "PULL",
            "OBJECT_CTL",
            "CAP_REVOKE",
        ] {
            assert!(instr_td.contains(opcode), "instr TableGen missing {opcode}");
        }
        for shape in [
            "class LNP64RRR",
            "(outs GPR:$rd)",
            "(ins GPR:$rs1, GPR:$rs2)",
            "class LNP64MemLoad",
            "$offset($base)",
            "class LNP64Native4",
            "(ins GPR:$cap, GPR:$arg0, GPR:$arg1)",
        ] {
            assert!(instr_td.contains(shape), "instr TableGen missing {shape}");
        }
        assert!(cmake.contains("LNP64GenRegisterInfo.inc"));
        for source in [
            "LNP64TargetMachine.cpp",
            "LNP64Subtarget.cpp",
            "LNP64ISelLowering.cpp",
            "LNP64FrameLowering.cpp",
            "add_subdirectory(InstPrinter)",
            "add_subdirectory(AsmParser)",
            "add_subdirectory(Disassembler)",
        ] {
            assert!(cmake.contains(source), "CMake missing {source}");
        }
        assert!(cmake.contains("add_llvm_target(LNP64CodeGen"));
        assert!(mc_desc_cmake.contains("LNP64MCAsmBackend.cpp"));
        assert!(target_info.contains("LLVMInitializeLNP64TargetInfo"));
        assert!(target_info.contains("RegisterTarget<Triple::lnp64>"));
        assert!(mc_desc.contains("LLVMInitializeLNP64TargetMC"));
        assert!(mc_desc.contains("RegisterMCCodeEmitter"));
        assert!(mc_desc.contains("RegisterMCAsmBackend"));
        assert!(mc_desc.contains("RegisterMCInstPrinter"));
        assert!(mc_desc_header.contains("fixup_lnp64_branch26"));
        assert!(mc_emitter.contains("createLNP64MCCodeEmitter"));
        assert!(mc_asm_backend.contains("createLNP64AsmBackend"));
        assert!(mc_asm_backend.contains("LNP64ELFObjectWriter"));
        assert!(mc_asm_backend.contains("R_LNP64_BRANCH26"));
        assert!(inst_printer.contains("createLNP64MCInstPrinter"));
        assert!(inst_printer.contains("printMemOperand"));
        assert!(inst_printer.contains("call_reg"));
        assert!(mc_emitter.contains("case LNP64::AND"));
        assert!(mc_emitter.contains("case LNP64::CMP"));
        assert!(mc_emitter.contains("case LNP64::LD_W"));
        assert!(mc_emitter.contains("case LNP64::LD_H"));
        assert!(mc_emitter.contains("case LNP64::ST_B"));
        assert!(mc_emitter.contains("case LNP64::ST_H"));
        assert!(mc_emitter.contains("not implemented yet"));
        assert!(mc_emitter.contains("isInt<14>(Offset)"));
        assert!(asm_parser.contains("LLVMInitializeLNP64AsmParser"));
        assert!(asm_parser.contains("RegisterMCAsmParser"));
        assert!(asm_parser.contains("parseImmediateOrMemory"));
        assert!(asm_parser.contains("buildInstruction"));
        assert!(asm_parser.contains(r#".Case("call", LNP64::CALL)"#));
        assert!(asm_parser.contains(r#".Case("ld.w", LNP64::LD_W)"#));
        assert!(asm_parser.contains(r#".Case("ld.h", LNP64::LD_H)"#));
        assert!(disassembler.contains("LLVMInitializeLNP64Disassembler"));
        assert!(disassembler.contains("RegisterMCDisassembler"));
        assert!(disassembler.contains("readLE32"));
        assert!(disassembler.contains("case 0x10"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::ADD)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::AND)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::CMP)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::CALL)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::CALL_REG)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::LD_W)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::LD_H)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::ST_B)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::ST_H)"));
        assert!(disassembler.contains("SignExtend64<14>"));
        assert!(disassembler.contains("decodeBranchTarget"));
        assert!(disassembler.contains("MCDisassembler::Fail"));
        assert!(target_machine.contains("LLVMInitializeLNP64Target"));
        assert!(target_machine.contains("e-m:e-p:64:64-i64:64-n64-S128"));
        assert!(subtarget.contains("TLInfo(TM, *this)"));
        assert!(isel.contains("addRegisterClass(MVT::i64"));
        assert!(isel.contains("ISD::ADD"));
        assert!(isel.contains("ISD::SDIV"));
        assert!(isel.contains("setOperationAction(ISD::BR_CC, MVT::i64, Custom)"));
        assert!(isel.contains("LNP64GenCallingConv.inc"));
        assert!(isel.contains("LowerOperation"));
        assert!(isel.contains("ISD::BR_CC"));
        assert!(
            isel.contains(
                "LNP64 conditional branch lowering only supports signed comparisons today"
            )
        );
        assert!(isel.contains("EmitInstrWithCustomInserter"));
        assert!(isel.contains("LNP64::PseudoBEQ"));
        assert!(isel.contains("BuildMI(*BB, MI, DL, TII.get(LNP64::CMP))"));
        assert!(isel.contains("BuildMI(*BB, MI, DL, TII.get(BranchOpcode))"));
        assert!(isel.contains("LowerFormalArguments"));
        assert!(isel.contains("CCInfo.AnalyzeFormalArguments(Ins, CC_LNP64)"));
        assert!(isel.contains("MF.addLiveIn(VA.getLocReg(), &LNP64::GPRRegClass)"));
        assert!(isel.contains("LowerReturn"));
        assert!(isel.contains("CCInfo.AnalyzeReturn(Outs, RetCC_LNP64)"));
        assert!(isel.contains("DAG.getCopyToReg"));
        assert!(isel.contains("LowerCall"));
        assert!(isel.contains("ArgCCInfo.AnalyzeCallOperands(CLI.Outs, CC_LNP64)"));
        assert!(isel.contains("DAG.getTargetGlobalAddress"));
        assert!(isel.contains("DAG.getTargetExternalSymbol"));
        assert!(isel.contains("indirect call callee must lower to an i64 register"));
        assert!(isel.contains("LNP64ISD::CALL"));
        assert!(isel.contains("CalleeName == \"__lnp_call\" || CalleeName == \"__lnp_pull\""));
        assert!(
            isel.contains(
                "CalleeName == \"__lnp_domain_ctl\" || CalleeName == \"__lnp_object_ctl\""
            )
        );
        assert!(isel.contains("LNP64ISD::DOMAIN_CTL"));
        assert!(isel.contains("LNP64ISD::GATE_CALL"));
        assert!(isel.contains("LNP64ISD::OBJECT_CTL"));
        assert!(isel.contains("LNP64ISD::PULL"));
        assert!(isel.contains("LNP64ISD::PUSH"));
        assert!(isel.contains("RetCCInfo.AnalyzeCallResult(CLI.Ins, RetCC_LNP64)"));
        assert!(isel.contains("native shim lowering expects three arguments and a result"));
        assert!(isel.contains("native control lowering expects one argument and a result"));
        assert!(isel.contains("LNP64ISD::RET_FLAG"));
        assert!(isel.contains("setLoadExtAction(ISD::ZEXTLOAD, MVT::i64, MemVT, Legal)"));
        assert!(isel.contains("setTruncStoreAction(MVT::i64, MemVT, Legal)"));
        assert!(isel.contains("computeRegisterProperties"));
        assert!(isel.contains("varargs lowering is not implemented yet"));
        assert!(isel_header.contains("getTargetNodeName"));
        assert!(isel_header.contains("LowerOperation"));
        assert!(isel_header.contains("EmitInstrWithCustomInserter"));
        assert!(isel_header.contains("BR_EQ"));
        assert!(isel_header.contains("LowerFormalArguments"));
        assert!(isel_header.contains("LowerReturn"));
        assert!(isel_header.contains("LowerCall"));
        assert!(isel_header.contains("CALL"));
        assert!(isel_header.contains("DOMAIN_CTL"));
        assert!(isel_header.contains("GATE_CALL"));
        assert!(isel_header.contains("OBJECT_CTL"));
        assert!(isel_header.contains("PULL"));
        assert!(isel_header.contains("PUSH"));
        assert!(isel_header.contains("RET_FLAG"));
        assert!(instr_td.contains("def simm16_imm"));
        assert!(instr_td.contains("def simm14_imm"));
        assert!(instr_td.contains("def brtarget : Operand<OtherVT>"));
        assert!(instr_td.contains("(ins brtarget:$target)"));
        assert!(instr_td.contains("def SDT_LNP64BrCC"));
        assert!(instr_td.contains("def LNP64breq"));
        assert!(instr_td.contains("def LNP64brne"));
        assert!(instr_td.contains("class LNP64CondBranchPseudo"));
        assert!(instr_td.contains("usesCustomInserter = 1"));
        assert!(instr_td.contains("def PseudoBEQ"));
        assert!(instr_td.contains("(PseudoBEQ GPR:$lhs, GPR:$rhs, bb:$target)"));
        assert!(instr_td.contains("def LNP64retflag"));
        assert!(instr_td.contains("def LNP64call"));
        assert!(instr_td.contains("def LNP64domainctl"));
        assert!(instr_td.contains("def LNP64gatecall"));
        assert!(instr_td.contains("def LNP64objectctl"));
        assert!(instr_td.contains("def LNP64pull"));
        assert!(instr_td.contains("def LNP64push"));
        assert!(instr_td.contains("(set GPR:$rd, simm16_imm:$imm)"));
        assert!(instr_td.contains("(set GPR:$rd, (add GPR:$rs1, GPR:$rs2))"));
        assert!(instr_td.contains("(set GPR:$rd, (shl GPR:$rs1, GPR:$rs2))"));
        assert!(instr_td.contains("let Pattern = [(br bb:$target)]"));
        assert!(instr_td.contains("(LNP64call tglobaladdr:$target)"));
        assert!(instr_td.contains("(LNP64call texternalsym:$target)"));
        assert!(instr_td.contains("(LNP64call GPR:$target)"));
        assert!(instr_td.contains("(i64 (load (add GPR:$base, simm14_imm:$offset)))"));
        assert!(instr_td.contains("(i64 (zextloadi32 (add GPR:$base, simm14_imm:$offset)))"));
        assert!(instr_td.contains("(i64 (zextloadi16 (add GPR:$base, simm14_imm:$offset)))"));
        assert!(instr_td.contains("(i64 (zextloadi8 (add GPR:$base, simm14_imm:$offset)))"));
        assert!(instr_td.contains("(ST GPR:$rs, GPR:$base, simm14_imm:$offset)"));
        assert!(instr_td.contains("(ST_W GPR:$rs, GPR:$base, simm14_imm:$offset)"));
        assert!(instr_td.contains("(ST_H GPR:$rs, GPR:$base, simm14_imm:$offset)"));
        assert!(instr_td.contains("(ST_B GPR:$rs, GPR:$base, simm14_imm:$offset)"));
        assert!(instr_td.contains("(LNP64domainctl GPR:$arg)"));
        assert!(instr_td.contains("(LNP64gatecall GPR:$cap, GPR:$arg0, GPR:$arg1)"));
        assert!(instr_td.contains("(LNP64objectctl GPR:$arg)"));
        assert!(instr_td.contains("(LNP64pull GPR:$cap, GPR:$arg0, GPR:$arg1)"));
        assert!(instr_td.contains("(LNP64push GPR:$cap, GPR:$arg0, GPR:$arg1)"));
        assert!(instr_td.contains("isReturn = 1"));
        assert!(instr_td.contains("Defs = [LR]"));
        assert!(instr_td.contains("Defs = [FLAGS]"));
        assert!(instr_td.contains("Uses = [LR]"));
        assert!(instr_td.contains("Uses = [FLAGS]"));
        assert!(instr_td.contains("let Pattern = [(LNP64retflag)]"));
        assert!(instr_td.contains("isBranch = 1"));
        assert!(instr_info.contains("copyPhysReg"));
        assert!(instr_info.contains("BuildMI(MBB, I, DL, get(LNP64::MOV), DestReg)"));
        assert!(instr_info.contains("storeRegToStackSlot"));
        assert!(instr_info.contains("loadRegFromStackSlot"));
        assert!(instr_info.contains("addFrameIndex(FrameIndex)"));
        assert!(isel.contains("setStackPointerRegisterToSaveRestore(LNP64::R31)"));
        assert!(frame.contains("StackGrowsDown"));
        assert!(frame.contains("Align(16)"));
        assert!(frame.contains("emitSPAdjust"));
        assert!(frame.contains("LNP64::R30"));
        assert!(frame.contains("TII.get(Amount < 0 ? LNP64::SUB : LNP64::ADD)"));
        assert!(reginfo.contains("Reserved.set(LNP64::R0)"));
        assert!(reginfo.contains("Reserved.set(LNP64::R30)"));
        assert!(reginfo.contains("eliminateFrameIndex"));
        assert!(reginfo.contains("ChangeToRegister(LNP64::R31"));
        assert!(reginfo.contains("MFI.getObjectOffset"));
        assert!(reginfo.contains("isInt<14>(Offset)"));
        assert!(reginfo.contains("NoCalleeSaved"));
        assert!(clang_target.contains("resetDataLayout(\"e-m:e-p:64:64-i64:64-n64-S128\")"));
        assert!(clang_target.contains("__LNP64__"));
        for constraint in ["case 'r'", "case 'f'", "case 'p'", "case 'm'", "case 'i'"] {
            assert!(
                clang_target.contains(constraint),
                "clang target missing asm constraint {constraint}"
            );
        }
        assert!(clang_driver.contains("getLNP64TargetCPU"));
        assert!(clang_driver.contains("toolchain/crt0_lnp64.s"));
        assert!(clang_driver.contains("elf64lnp64"));
        assert!(clang_driver.contains("toolchain/lnp64_static.ld"));
        assert!(lld_arch.contains("getLNP64TargetInfo"));
        for reloc in [
            "R_LNP64_ABS64",
            "R_LNP64_RELATIVE",
            "R_LNP64_TLS_TPREL64",
            "R_LNP64_FDR_DESC64",
            "R_LNP64_BRANCH26",
        ] {
            assert!(lld_arch.contains(reloc), "lld arch missing {reloc}");
        }
        assert!(codegen_test.contains("llc -mtriple=lnp64-unknown-none"));
        assert!(codegen_test.contains("XFAIL: *"));
        assert!(codegen_test.contains("define i64 @arith"));
        assert!(codegen_test.contains("define i64 @control"));
        assert!(codegen_test.contains("define i64 @gate"));
        assert!(codegen_test.contains("define i64 @read_stream"));
        assert!(codegen_test.contains("define i64 @jump"));
        assert!(codegen_test.contains("define i64 @branch_if"));
        assert!(codegen_test.contains("define i64 @call_direct"));
        assert!(codegen_test.contains("define i64 @call_indirect"));
        assert!(codegen_test.contains("define i64 @memory"));
        assert!(codegen_test.contains("%biased = add i64 %sum, 7"));
        assert!(codegen_test.contains("br label %exit"));
        assert!(codegen_test.contains("call i64 @callee"));
        assert!(codegen_test.contains("; CHECK: call callee"));
        assert!(codegen_test.contains("; CHECK: cmp"));
        assert!(codegen_test.contains("; CHECK: beq"));
        assert!(codegen_test.contains("; CHECK: call_reg"));
        assert!(codegen_test.contains("; CHECK: jmp"));
        for mnemonic in ["ld.b", "ld.h", "ld.w", "st.b", "st.h", "st.w"] {
            assert!(
                codegen_test.contains(&format!("; CHECK: {mnemonic}")),
                "codegen fixture missing narrow memory check for {mnemonic}"
            );
        }
        assert!(codegen_test.contains("; CHECK: lsl"));
        assert!(codegen_test.contains("; CHECK: ret"));
        assert!(codegen_test.contains("__lnp_call"));
        assert!(codegen_test.contains("__lnp_domain_ctl"));
        assert!(codegen_test.contains("__lnp_object_ctl"));
        assert!(codegen_test.contains("__lnp_pull"));
        assert!(codegen_test.contains("__lnp_push"));
        assert!(codegen_test.contains("; CHECK: domain_ctl"));
        assert!(codegen_test.contains("; CHECK: gate_call"));
        assert!(codegen_test.contains("; CHECK: object_ctl"));
        assert!(codegen_test.contains("; CHECK: pull"));
        assert!(codegen_test.contains("; CHECK: push"));
        assert!(mc_test.contains("llvm-mc -triple=lnp64-unknown-none"));
        assert!(mc_test.contains("li r1, 42"));
        assert!(mc_test.contains("ld.h r5, 18(r31)"));
        assert!(mc_test.contains("st.h r5, 26(r31)"));
        assert!(mc_test.contains("XFAIL: *"));
        assert!(clang_driver_test.contains("--target=lnp64-unknown-none"));
        assert!(clang_driver_test.contains("elf64lnp64"));
        assert!(clang_driver_test.contains("toolchain/crt0_lnp64.s"));
        assert!(clang_driver_test.contains("XFAIL: *"));
    }

    #[test]
    fn libc_shim_manifest_covers_runtime_surfaces() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let shim_manifest = include_str!("../toolchain/lnp64_libc_shim.manifest");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let libc_roadmap = include_str!("../libc_roadmap.md");
        let conformance = include_str!("../conformance_matrix.md");
        let c_compiler = include_str!("c_compiler.rs");
        let emulator = include_str!("emulator.rs");
        let evidence_corpus = format!("{conformance}\n{c_compiler}\n{emulator}");
        let rows = libc_shim_rows(shim_manifest);
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let shim_path = manifest_field(target_manifest, "libc_shim_contract");
        let mut groups = std::collections::BTreeMap::new();

        assert_eq!(shim_path, "toolchain/lnp64_libc_shim.manifest");
        assert!(manifest_root.join(shim_path).is_file());
        assert!(contract_index.contains(
            "libc_shim|toolchain/lnp64_libc_shim.manifest|libc_shim_manifest_covers_runtime_surfaces"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_libc_shim.manifest"));
        assert!(roadmap.contains("toolchain/lnp64_libc_shim.manifest"));
        assert!(libc_roadmap.contains("toolchain/lnp64_libc_shim.manifest"));

        for (group, public_surface, native_lowering, evidence, status) in rows {
            assert!(
                groups
                    .insert(
                        group,
                        (public_surface.clone(), native_lowering.clone(), status),
                    )
                    .is_none(),
                "duplicate libc shim group {group}"
            );
            assert!(
                ["tested", "partial", "planned"].contains(&status),
                "unknown libc shim status {status} for {group}"
            );
            assert!(
                !public_surface.is_empty(),
                "empty public surface for libc shim group {group}"
            );
            assert!(
                !native_lowering.is_empty(),
                "empty native lowering for libc shim group {group}"
            );
            assert!(
                !evidence.is_empty(),
                "empty evidence for libc shim group {group}"
            );
            for item in public_surface.iter().chain(native_lowering.iter()) {
                assert!(!item.is_empty(), "empty item in libc shim group {group}");
            }
            for item in evidence {
                assert!(
                    manifest_root.join(item).exists() || evidence_corpus.contains(item),
                    "libc shim evidence {item} for {group} is not present in repo evidence"
                );
            }
        }

        for group in [
            "startup_env_auxv",
            "errno_tls",
            "fd_io",
            "malloc_heap",
            "pthread_futex",
            "poll_select_epoll_kqueue",
            "mmap_mprotect",
            "signals_as_events",
            "sockets_endpoints",
        ] {
            assert!(
                groups.contains_key(group),
                "missing libc shim group {group}"
            );
        }
        assert_eq!(
            groups["poll_select_epoll_kqueue"].2, "partial",
            "kqueue/kevent must stay partial until real event-queue backend exists"
        );
        for group in [
            "startup_env_auxv",
            "errno_tls",
            "fd_io",
            "malloc_heap",
            "pthread_futex",
            "mmap_mprotect",
            "signals_as_events",
            "sockets_endpoints",
        ] {
            assert_eq!(groups[group].2, "tested", "{group} should be tested");
        }

        for (group, required_public, required_native) in [
            (
                "startup_env_auxv",
                vec!["_start", "argv", "envp", "getauxval"],
                vec!["crt0", "TLS", "ENV_GET", "EXIT"],
            ),
            (
                "errno_tls",
                vec!["errno", "__errno_location", "strerror"],
                vec!["TLS", "ERRNO_SET", "completion_helpers"],
            ),
            (
                "fd_io",
                vec!["openat", "read", "write", "fcntl", "stdio"],
                vec!["__lnp_openat", "__lnp_pull", "__lnp_push", "CAP_DUP", "FDR"],
            ),
            (
                "malloc_heap",
                vec!["malloc", "free", "posix_memalign"],
                vec!["ALLOC", "ALLOC_EX", "ALLOC_SIZE", "FREE"],
            ),
            (
                "pthread_futex",
                vec!["pthread_create", "pthread_join", "futex"],
                vec!["CLONE", "FUTEX_WAIT", "FUTEX_WAKE", "AWAIT"],
            ),
            (
                "poll_select_epoll_kqueue",
                vec!["poll", "select", "epoll_wait", "kqueue"],
                vec!["event_queue", "AWAIT", "OBJECT_CTL", "waitable_generation"],
            ),
            (
                "mmap_mprotect",
                vec!["mmap", "munmap", "mprotect"],
                vec!["MMAP", "MUNMAP", "MPROTECT", "VMA"],
            ),
            (
                "signals_as_events",
                vec!["sigaction", "signal", "SIGRET"],
                vec!["event_delivery", "signal_frame", "SIGRET"],
            ),
            (
                "sockets_endpoints",
                vec!["socket", "accept", "getsockopt", "recv"],
                vec!["OBJECT_CTL", "endpoint_profile", "GET_META", "PULL", "PUSH"],
            ),
        ] {
            let (public_surface, native_lowering, _) = &groups[group];
            for item in required_public {
                assert!(
                    public_surface.contains(&item),
                    "libc shim group {group} missing public surface {item}"
                );
            }
            for item in required_native {
                assert!(
                    native_lowering.contains(&item),
                    "libc shim group {group} missing native lowering {item}"
                );
            }
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
    fn crt0_startup_stub_matches_crt_contract() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let crt_manifest = include_str!("../toolchain/lnp64_crt_startup.manifest");
        let crt0 = include_str!("../toolchain/crt0_lnp64.s");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let crt0_path = manifest_field(target_manifest, "crt0_contract");

        assert_eq!(crt0_path, "toolchain/crt0_lnp64.s");
        assert!(manifest_root.join(crt0_path).is_file());
        assert!(
            contract_index
                .contains("crt0|toolchain/crt0_lnp64.s|crt0_startup_stub_matches_crt_contract")
        );
        assert!(transition_manifest.contains("toolchain/crt0_lnp64.s"));
        assert!(roadmap.contains("toolchain/crt0_lnp64.s"));

        for required in [
            "_start:",
            "LI r7, 0x700000",
            "LD r1, [r7, 0]",
            "LI r2, 0x700008",
            "MUL r3, r1, r8",
            "ADD r3, r3, r2",
            "ADD r3, r3, r8",
            "ERRNO_SET r0",
            "CALL main",
            "EXIT r1",
        ] {
            assert!(crt0.contains(required), "crt0 missing {required}");
        }
        assert!(crt_manifest.contains("entry_symbol|required|_start"));
        assert!(crt_manifest.contains("main_signature|required|main(argc,argv,envp)"));
        assert!(crt_manifest.contains("process_exit|required|EXIT"));
        assert!(!crt0.contains("lnp64 cc"));
        assert!(!crt0.contains("cargo run -- cc"));
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
    fn netbsd_layers_manifest_preserves_personality_order() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let layers_manifest = include_str!("../toolchain/lnp64_netbsd_layers.manifest");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let personality_doc = include_str!("../netbsd_personality_abi.md");
        let system_gate = include_str!("../scripts/run_netbsd_personality_system.sh");
        let rows = netbsd_layer_rows(layers_manifest);
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let layers_path = manifest_field(target_manifest, "netbsd_layers_contract");
        let mut seen = std::collections::BTreeSet::new();
        let mut ordered_layers = Vec::new();
        let mut statuses = std::collections::BTreeMap::new();
        let mut blockers = std::collections::BTreeMap::new();

        assert_eq!(layers_path, "toolchain/lnp64_netbsd_layers.manifest");
        assert!(manifest_root.join(layers_path).is_file());
        assert!(contract_index.contains(
            "netbsd_layers|toolchain/lnp64_netbsd_layers.manifest|netbsd_layers_manifest_preserves_personality_order"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_netbsd_layers.manifest"));
        assert!(roadmap.contains("toolchain/lnp64_netbsd_layers.manifest"));
        assert!(personality_doc.contains("toolchain/lnp64_netbsd_layers.manifest"));
        assert!(personality_doc.contains("No full monolithic NetBSD kernel port"));
        assert!(system_gate.contains("forbidden primitive in trace"));
        for forbidden in [
            "IRQ",
            "MMIO",
            "DMA_CTL",
            "PAGE_TABLE",
            "SCHED_CTL",
            "RAW_SYSCALL",
        ] {
            assert!(
                system_gate.contains(forbidden),
                "NetBSD system gate must reject forbidden primitive {forbidden}"
            );
        }

        for (layer, status, artifacts, gate, next_blocker) in rows {
            assert!(seen.insert(layer), "duplicate NetBSD layer {layer}");
            ordered_layers.push(layer);
            statuses.insert(layer, status);
            blockers.insert(layer, next_blocker);
            assert!(
                ["bootstrap_gate", "scaffolded", "planned", "blocked"].contains(&status),
                "unknown NetBSD layer status {status} for {layer}"
            );
            assert!(
                !artifacts.is_empty(),
                "empty artifacts for NetBSD layer {layer}"
            );
            for artifact in artifacts {
                assert!(
                    manifest_root.join(artifact).exists(),
                    "NetBSD layer {layer} names missing artifact {artifact}"
                );
            }
            if gate != "none" {
                assert!(
                    manifest_root.join(gate).exists(),
                    "NetBSD layer {layer} names missing gate {gate}"
                );
            }
            assert!(
                !next_blocker.is_empty(),
                "NetBSD layer {layer} must name its next blocker"
            );
        }

        assert_eq!(
            ordered_layers,
            vec![
                "libc_userland_pieces",
                "rump_filesystem_components",
                "rump_network_socket_personality",
                "process_signal_thread_compat",
                "larger_userland_commands",
                "fuller_machine_port",
            ],
            "NetBSD personality layers must stay in the planned bring-up order"
        );
        assert_eq!(statuses["fuller_machine_port"], "blocked");
        assert!(
            blockers["fuller_machine_port"].contains("not_credible_yet"),
            "fuller machine port must remain blocked on rump services/static userland credibility"
        );
        assert_ne!(
            statuses["larger_userland_commands"], "bootstrap_gate",
            "larger NetBSD userland must not be treated as current bootstrap coverage"
        );
    }

    #[test]
    fn conformance_gate_manifest_covers_required_layers() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let gate_manifest = include_str!("../toolchain/lnp64_conformance_gates.manifest");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let conformance = include_str!("../conformance_matrix.md");
        let run_all = include_str!("../scripts/run_all_gates.sh");
        let run_software = include_str!("../scripts/run_software_gates.sh");
        let run_real_packages = include_str!("../scripts/run_real_packages.sh");
        let run_demos = include_str!("../scripts/run_demos.sh");
        let rows = conformance_gate_rows(gate_manifest);
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let gate_path = manifest_field(target_manifest, "conformance_gate_contract");
        let mut categories = std::collections::BTreeMap::new();

        assert_eq!(gate_path, "toolchain/lnp64_conformance_gates.manifest");
        assert!(manifest_root.join(gate_path).is_file());
        assert!(contract_index.contains(
            "conformance_gates|toolchain/lnp64_conformance_gates.manifest|conformance_gate_manifest_covers_required_layers"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_conformance_gates.manifest"));
        assert!(roadmap.contains("toolchain/lnp64_conformance_gates.manifest"));
        assert!(conformance.contains("toolchain/lnp64_conformance_gates.manifest"));

        for (category, status, artifacts, gate, coverage) in rows {
            assert!(
                categories
                    .insert(category, (status, artifacts.clone(), gate, coverage))
                    .is_none(),
                "duplicate conformance gate category {category}"
            );
            assert!(
                ["tested", "planned"].contains(&status),
                "unknown conformance gate status {status} for {category}"
            );
            assert!(
                !artifacts.is_empty(),
                "empty artifacts for conformance gate {category}"
            );
            for artifact in artifacts {
                assert!(
                    manifest_root.join(artifact).exists(),
                    "conformance gate {category} names missing artifact {artifact}"
                );
            }
            assert!(
                !gate.is_empty(),
                "empty gate for conformance category {category}"
            );
            assert!(
                gate.starts_with("cargo test")
                    || gate == "simple_libc_gate"
                    || gate == "scripts/run_llvm_bootstrap_gates.sh --dry-run"
                    || manifest_root.join(gate).exists(),
                "conformance gate {category} names missing gate {gate}"
            );
            assert!(
                !coverage.is_empty(),
                "empty coverage note for conformance category {category}"
            );
        }

        for category in [
            "asm_demos",
            "c_tests",
            "randomized_emulator",
            "adversarial_fault",
            "package_tests",
            "netbsd_personality",
            "llvm_built_versions",
            "aggregate_hygiene",
        ] {
            assert!(
                categories.contains_key(category),
                "missing conformance category {category}"
            );
        }
        assert_eq!(categories["llvm_built_versions"].0, "planned");
        assert!(
            categories["llvm_built_versions"]
                .1
                .contains(&"scripts/run_llvm_bootstrap_gates.sh")
        );
        assert_eq!(
            categories["llvm_built_versions"].2,
            "scripts/run_llvm_bootstrap_gates.sh --dry-run"
        );
        for category in [
            "asm_demos",
            "c_tests",
            "randomized_emulator",
            "adversarial_fault",
            "package_tests",
            "netbsd_personality",
            "aggregate_hygiene",
        ] {
            assert_eq!(
                categories[category].0, "tested",
                "{category} should be tested by current gates"
            );
        }

        assert!(run_software.contains("cargo test"));
        assert!(run_software.contains("bash scripts/run_toolchain_contracts.sh"));
        assert!(run_software.contains("bash scripts/run_llvm_bootstrap_gates.sh --dry-run"));
        assert!(run_software.contains("bash scripts/run_demos.sh"));
        assert!(run_software.contains("bash scripts/run_userland.sh"));
        assert!(run_software.contains("bash scripts/run_netbsd_personality_system.sh"));
        assert!(run_software.contains("bash scripts/run_real_packages.sh"));
        assert!(run_all.contains("bash scripts/run_software_gates.sh"));
        assert!(run_all.contains("git diff --check"));
        assert!(run_real_packages.contains("scripts/run_libc_test.sh"));
        assert!(run_real_packages.contains("scripts/run_sbase.sh"));
        assert!(run_demos.contains("demos/hello.c"));
        assert!(run_demos.contains("for src in demos/*.s"));
    }

    #[test]
    fn toy_compiler_policy_manifest_freezes_bootstrap_role() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let policy_manifest = include_str!("../toolchain/lnp64_toy_compiler_policy.manifest");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let conformance = include_str!("../conformance_matrix.md");
        let llvm_gates = include_str!("../toolchain/lnp64_llvm_gates.manifest");
        let llvm_bootstrap = include_str!("../toolchain/lnp64_llvm_bootstrap.manifest");
        let intrinsics = include_str!("../toolchain/lnp64_intrinsics.manifest");
        let intrinsic_header = include_str!("../toolchain/lnp64_intrinsics.h");
        let c_compiler = include_str!("c_compiler.rs");
        let lowering_source = include_str!("lowering.rs");
        let libc_roadmap = include_str!("../libc_roadmap.md");
        let evidence_corpus = format!(
            "{target_manifest}\n{roadmap}\n{conformance}\n{llvm_gates}\n{llvm_bootstrap}\n{intrinsics}\n{intrinsic_header}\n{c_compiler}\n{lowering_source}\n{libc_roadmap}"
        );
        let rows = toy_compiler_policy_rows(policy_manifest);
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut rules = std::collections::BTreeMap::new();

        assert_eq!(
            manifest_field(target_manifest, "toy_compiler_policy"),
            "bootstrap_smoke_only_after_llvm_gate"
        );
        assert_eq!(
            manifest_field(target_manifest, "toy_compiler_policy_contract"),
            "toolchain/lnp64_toy_compiler_policy.manifest"
        );
        assert!(contract_index.contains(
            "toy_compiler_policy|toolchain/lnp64_toy_compiler_policy.manifest|toy_compiler_policy_manifest_freezes_bootstrap_role"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_toy_compiler_policy.manifest"));
        assert!(roadmap.contains("only small fixes needed to keep existing smoke"));
        assert!(conformance.contains("toolchain/lnp64_toy_compiler_policy.manifest"));

        for (rule, status, artifacts, evidence) in rows {
            assert!(
                rules
                    .insert(rule, (status, artifacts.clone(), evidence))
                    .is_none(),
                "duplicate toy compiler policy rule {rule}"
            );
            assert!(
                ["required", "planned"].contains(&status),
                "unknown toy compiler policy status {status} for {rule}"
            );
            assert!(
                !artifacts.is_empty(),
                "empty toy compiler policy artifacts for {rule}"
            );
            for artifact in artifacts {
                assert!(
                    manifest_root.join(artifact).exists(),
                    "toy compiler policy {rule} names missing artifact {artifact}"
                );
            }
            assert!(
                !evidence.is_empty(),
                "empty toy compiler policy evidence for {rule}"
            );
            if status == "required" {
                assert!(
                    evidence_corpus.contains(evidence),
                    "toy compiler policy evidence {evidence} for {rule} is not present"
                );
            }
        }

        for rule in [
            "smoke_generator_only",
            "private_native_shims",
            "compat_lowering_boundary",
            "no_toy_in_llvm_gates",
            "replacement_program_set",
        ] {
            assert!(
                rules.contains_key(rule),
                "missing toy compiler policy rule {rule}"
            );
        }
        for rule in [
            "smoke_generator_only",
            "private_native_shims",
            "compat_lowering_boundary",
            "no_toy_in_llvm_gates",
        ] {
            assert_eq!(rules[rule].0, "required", "{rule} should be required");
        }
        assert_eq!(rules["replacement_program_set"].0, "planned");
        for intrinsic in manifest_field(target_manifest, "intrinsics").split(',') {
            assert!(intrinsic.starts_with("__lnp_"));
            assert!(intrinsics.contains(intrinsic));
            assert!(intrinsic_header.contains(intrinsic));
        }
        assert!(!llvm_gates.contains("lnp64 cc"));
        assert!(!llvm_gates.contains("cargo run -- cc"));
        for case in ["hello", "arithmetic", "memory", "calls", "simple_libc"] {
            assert!(
                llvm_bootstrap.contains(case),
                "replacement program set missing {case}"
            );
        }
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
            manifest_field(manifest, "register_contract"),
            "toolchain/lnp64_registers.manifest"
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
            manifest_field(manifest, "mc_encoding_contract"),
            "toolchain/lnp64_mc_encoding.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "intrinsic_contract"),
            "toolchain/lnp64_intrinsics.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "intrinsic_header_contract"),
            "toolchain/lnp64_intrinsics.h"
        );
        assert_eq!(
            manifest_field(manifest, "clang_driver_contract"),
            "toolchain/lnp64_clang_driver.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "llvm_filemap_contract"),
            "toolchain/lnp64_llvm_filemap.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "libc_shim_contract"),
            "toolchain/lnp64_libc_shim.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "netbsd_layers_contract"),
            "toolchain/lnp64_netbsd_layers.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "conformance_gate_contract"),
            "toolchain/lnp64_conformance_gates.manifest"
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
            manifest_field(manifest, "loader_security_contract"),
            "toolchain/lnp64_loader_security.manifest"
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
        assert_eq!(
            manifest_field(manifest, "crt0_contract"),
            "toolchain/crt0_lnp64.s"
        );
        assert_eq!(
            manifest_field(manifest, "llvm_gate_contract"),
            "toolchain/lnp64_llvm_gates.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "run_elf_contract"),
            "toolchain/lnp64_run_elf.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "linker_script_contract"),
            "toolchain/lnp64_static.ld"
        );
        assert_eq!(manifest_field(manifest, "gpr"), "r0-r31");
        assert_eq!(manifest_field(manifest, "fdr"), "fd0-fd255");
        assert_eq!(manifest_field(manifest, "fpr"), "f0-f31");
        assert_eq!(manifest_field(manifest, "vr"), "v0-v15");
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
        assert_eq!(
            manifest_field(manifest, "toy_compiler_policy_contract"),
            "toolchain/lnp64_toy_compiler_policy.manifest"
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
    fn intrinsic_header_matches_intrinsic_manifest() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let intrinsic_manifest = include_str!("../toolchain/lnp64_intrinsics.manifest");
        let intrinsic_header = include_str!("../toolchain/lnp64_intrinsics.h");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let rows = intrinsic_rows(intrinsic_manifest);
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let header_path = manifest_field(target_manifest, "intrinsic_header_contract");
        let mut declarations = std::collections::BTreeSet::new();

        assert_eq!(header_path, "toolchain/lnp64_intrinsics.h");
        assert!(manifest_root.join(header_path).is_file());
        assert!(contract_index.contains(
            "intrinsic_header|toolchain/lnp64_intrinsics.h|intrinsic_header_matches_intrinsic_manifest"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_intrinsics.h"));
        assert!(roadmap.contains("toolchain/lnp64_intrinsics.h"));
        assert!(intrinsic_header.contains("#ifndef LNP64_INTRINSICS_H"));
        assert!(intrinsic_header.contains("typedef unsigned long lnp64_word_t;"));
        assert!(intrinsic_header.contains("typedef lnp64_word_t lnp64_cap_t;"));

        for (name, primitive, _, operands) in rows {
            assert!(
                declarations.insert(name),
                "duplicate intrinsic declaration check for {name}"
            );
            assert!(
                intrinsic_header.contains(&format!(" {name}(")),
                "intrinsic header is missing declaration for {name}"
            );
            assert!(
                !primitive.is_empty() && !operands.is_empty(),
                "manifest row for {name} must keep primitive and operands"
            );
        }
        for forbidden in [
            "fork", "pipe", "pthread", "signal", "poll", "select", "epoll",
        ] {
            assert!(
                !intrinsic_header.contains(forbidden),
                "private intrinsic header leaks compatibility word {forbidden}"
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
    fn mc_encoding_manifest_covers_initial_backend_opcodes() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let mc_manifest = include_str!("../toolchain/lnp64_mc_encoding.manifest");
        let isel_manifest = include_str!("../toolchain/lnp64_isel.manifest");
        let relocation_manifest = include_str!("../toolchain/lnp64_relocations.manifest");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let conformance = include_str!("../conformance_matrix.md");
        let filemap = include_str!("../toolchain/lnp64_llvm_filemap.manifest");
        let mc_emitter =
            include_str!("../llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCCodeEmitter.cpp");
        let mc_asm_backend =
            include_str!("../llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCAsmBackend.cpp");
        let lld_arch = include_str!("../lld/ELF/Arch/LNP64.cpp");
        let rows = mc_encoding_rows(mc_manifest);
        let relocation_names: std::collections::BTreeSet<_> = relocation_rows(relocation_manifest)
            .into_iter()
            .map(|(_, name, _, _)| name)
            .collect();
        let mut groups = std::collections::BTreeMap::new();
        let mut encoded_opcodes = std::collections::BTreeSet::new();

        assert_eq!(
            manifest_field(target_manifest, "mc_encoding_contract"),
            "toolchain/lnp64_mc_encoding.manifest"
        );
        assert!(contract_index.contains(
            "mc_encoding|toolchain/lnp64_mc_encoding.manifest|mc_encoding_manifest_covers_initial_backend_opcodes"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_mc_encoding.manifest"));
        assert!(roadmap.contains("toolchain/lnp64_mc_encoding.manifest"));
        assert!(conformance.contains("toolchain/lnp64_mc_encoding.manifest"));
        assert!(filemap.contains("LNP64MCCodeEmitter.cpp"));
        assert!(mc_manifest.contains("fixed32_no_operand"));
        assert!(mc_manifest.contains("opcode[31:24]"));
        assert!(mc_manifest.contains("fixed32_rrr"));
        assert!(mc_manifest.contains("fixed32_mem_base_simm"));
        assert!(mc_manifest.contains("simm24_words[23:0]"));

        for (group, format, opcodes, operands, relocations, surfaces) in rows {
            assert!(
                groups
                    .insert(
                        group,
                        (format, opcodes.clone(), operands, relocations.clone())
                    )
                    .is_none(),
                "duplicate MC encoding group {group}"
            );
            assert!(
                format.starts_with("fixed32_"),
                "initial MC group {group} must use a fixed32 format class"
            );
            assert!(!opcodes.is_empty(), "empty MC opcode group {group}");
            assert!(!operands.is_empty(), "empty MC operands for {group}");
            assert!(!surfaces.is_empty(), "empty LLVM surfaces for {group}");
            for opcode in opcodes {
                assert!(
                    encoded_opcodes.insert(opcode),
                    "duplicate MC opcode {opcode}"
                );
            }
            for relocation in relocations {
                if relocation != "none" {
                    assert!(
                        relocation_names.contains(relocation),
                        "MC group {group} names unknown relocation {relocation}"
                    );
                }
            }
            for surface in surfaces {
                assert!(
                    surface.ends_with(".td") || surface.ends_with(".cpp"),
                    "MC group {group} names unexpected LLVM surface {surface}"
                );
            }
        }

        for group in [
            "constants",
            "integer_alu_rrr",
            "control_branch",
            "memory",
            "atomics",
            "native_primitives",
        ] {
            assert!(
                groups.contains_key(group),
                "missing MC encoding group {group}"
            );
        }
        for (_group, status, opcodes) in isel_rows(isel_manifest) {
            if status == "required" || status == "intrinsic" {
                for opcode in opcodes {
                    assert!(
                        encoded_opcodes.contains(opcode),
                        "required/intrinsic isel opcode {opcode} lacks MC encoding coverage"
                    );
                }
            }
        }
        assert!(groups["control_branch"].3.contains(&"R_LNP64_BRANCH26"));
        assert!(groups["control_branch"].3.contains(&"R_LNP64_PC32"));
        assert!(
            groups["native_primitives"]
                .3
                .contains(&"R_LNP64_CAP_DESC64")
        );
        assert!(
            groups["native_primitives"]
                .3
                .contains(&"R_LNP64_CALLGATE64")
        );
        assert!(mc_emitter.contains("encodeFixed32NoOperand"));
        assert!(mc_emitter.contains("encodeFixed32RI"));
        assert!(mc_emitter.contains("encodeFixed32RR"));
        assert!(mc_emitter.contains("encodeFixed32RRR"));
        assert!(mc_emitter.contains("encodeFixed32Mem"));
        assert!(mc_emitter.contains("encodeFixed32Branch"));
        assert!(mc_emitter.contains("encodeFixed32BranchOperand"));
        assert!(mc_emitter.contains("fixup_lnp64_branch26"));
        assert!(mc_emitter.contains("encodeFixed32Reg"));
        assert!(mc_emitter.contains("case LNP64::NOP"));
        assert!(mc_emitter.contains("case LNP64::RET"));
        assert!(mc_emitter.contains("case LNP64::LI"));
        assert!(mc_emitter.contains("case LNP64::ADD"));
        assert!(mc_emitter.contains("case LNP64::CALL"));
        assert!(mc_emitter.contains("case LNP64::CALL_REG"));
        assert!(mc_emitter.contains("case LNP64::LD"));
        assert!(mc_emitter.contains("case LNP64::ST"));
        assert!(mc_emitter.contains("encodeFixed32NoOperand(0x00)"));
        assert!(mc_emitter.contains("encodeFixed32NoOperand(0x1f)"));
        assert!(mc_emitter.contains("emitLE32"));
        assert!(mc_asm_backend.contains("getRelocType"));
        assert!(mc_asm_backend.contains("fixup_lnp64_branch26"));
        assert!(mc_asm_backend.contains("writeNopData"));
        assert!(lld_arch.contains("relocateBranch26"));
        assert!(lld_arch.contains("read32le(Loc)"));
        assert!(lld_arch.contains("R_LNP64_BRANCH26 out of range"));
        assert!(!lld_arch.contains("R_LNP64_BRANCH26 is not encoded yet"));
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
    fn loader_security_manifest_covers_exec_plan_security() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let security_manifest = include_str!("../toolchain/lnp64_loader_security.manifest");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let object_format = include_str!("../object_format.md");
        let loader_source = include_str!("loader.rs");
        let emulator_source = include_str!("emulator.rs");
        let lowering_source = include_str!("lowering.rs");
        let conformance = include_str!("../conformance_matrix.md");
        let evidence_corpus =
            format!("{loader_source}\n{emulator_source}\n{lowering_source}\n{conformance}");
        let rows = loader_security_rows(security_manifest);
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let security_path = manifest_field(target_manifest, "loader_security_contract");
        let mut requirements = std::collections::BTreeMap::new();

        assert_eq!(security_path, "toolchain/lnp64_loader_security.manifest");
        assert!(manifest_root.join(security_path).is_file());
        assert!(contract_index.contains(
            "loader_security|toolchain/lnp64_loader_security.manifest|loader_security_manifest_covers_exec_plan_security"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_loader_security.manifest"));
        assert!(roadmap.contains("toolchain/lnp64_loader_security.manifest"));
        assert!(object_format.contains("The loader must choose ASLR layout"));
        assert!(object_format.contains("W^X/NX policy"));
        assert!(object_format.contains("executable provenance"));
        assert!(object_format.contains("old image remains"));

        for (requirement, boundary, evidence, status) in rows {
            assert!(
                requirements
                    .insert(requirement, (boundary, evidence.clone(), status))
                    .is_none(),
                "duplicate loader security requirement {requirement}"
            );
            assert!(
                [
                    "software_loader",
                    "loader_to_emulator",
                    "loader_and_exec_validator",
                    "software_loader_and_layout",
                    "exec_descriptor_validator",
                    "emulator_exec",
                ]
                .contains(&boundary),
                "unknown loader security boundary {boundary}"
            );
            assert!(
                ["tested", "partial"].contains(&status),
                "unknown loader security status {status} for {requirement}"
            );
            assert!(
                !evidence.is_empty(),
                "empty evidence for loader security requirement {requirement}"
            );
            for item in evidence {
                assert!(
                    evidence_corpus.contains(item),
                    "loader security evidence {item} for {requirement} is not present"
                );
            }
        }

        for requirement in [
            "parse_elf_headers",
            "apply_relocations",
            "prepare_vmas",
            "startup_metadata",
            "submit_exec_plan",
            "wx_nx_policy",
            "aslr_load_bias",
            "provenance_authority",
            "precommit_preservation",
        ] {
            assert!(
                requirements.contains_key(requirement),
                "missing loader security requirement {requirement}"
            );
        }
        assert_eq!(
            requirements["provenance_authority"].2, "partial",
            "generation/lineage authority validation must not be overclaimed"
        );
        for requirement in [
            "parse_elf_headers",
            "apply_relocations",
            "prepare_vmas",
            "startup_metadata",
            "submit_exec_plan",
            "wx_nx_policy",
            "aslr_load_bias",
            "precommit_preservation",
        ] {
            assert_eq!(
                requirements[requirement].2, "tested",
                "{requirement} should be tested"
            );
        }
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
        assert_eq!(manifest_field(psabi_manifest, "fpr_count"), "32");
        assert_eq!(manifest_field(psabi_manifest, "vr_count"), "16");
        assert_eq!(manifest_field(psabi_manifest, "zero_register"), "r0");
        assert_eq!(manifest_field(psabi_manifest, "stack_pointer"), "r31");
        assert_eq!(manifest_field(psabi_manifest, "link_register"), "LR");
        assert_eq!(manifest_field(psabi_manifest, "argument_gprs"), "r1-r6");
        assert_eq!(manifest_field(psabi_manifest, "return_gprs"), "r1");
        assert_eq!(
            manifest_field(psabi_manifest, "caller_clobbered_gprs"),
            "r1-r29"
        );
        assert_eq!(manifest_field(psabi_manifest, "callee_saved_gprs"), "none");
        assert_eq!(manifest_field(psabi_manifest, "backend_scratch_gpr"), "r30");
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
        assert!(psabi_doc.contains("`r30` is reserved as a backend scratch register"));
        assert!(psabi_doc.contains("`r1` through `r29` as caller-clobbered"));
        assert!(psabi_doc.contains("callee-saved GPR set in the v0 compiler ABI"));
        assert!(psabi_doc.contains("`r31` points at the current thread's stack/local region."));
        assert!(psabi_doc.contains("The thread pointer is read and written through the `TP` PCR."));
        assert!(psabi_doc.contains("`SIGRET` is the POSIX spelling"));
        assert!(psabi_doc.contains("`GATE_RETURN`"));
    }

    #[test]
    fn register_manifest_records_backend_classes() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let register_manifest = include_str!("../toolchain/lnp64_registers.manifest");
        let psabi_manifest = include_str!("../toolchain/lnp64_psabi.manifest");
        let inline_asm_manifest = include_str!("../toolchain/lnp64_inline_asm.manifest");
        let debug_unwind_manifest = include_str!("../toolchain/lnp64_debug_unwind.manifest");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let conformance = include_str!("../conformance_matrix.md");
        let rows = register_class_rows(register_manifest);
        let mut classes = std::collections::BTreeMap::new();

        assert_eq!(
            manifest_field(target_manifest, "register_contract"),
            "toolchain/lnp64_registers.manifest"
        );
        assert!(contract_index.contains(
            "registers|toolchain/lnp64_registers.manifest|register_manifest_records_backend_classes"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_registers.manifest"));
        assert!(roadmap.contains("toolchain/lnp64_registers.manifest"));
        assert!(conformance.contains("toolchain/lnp64_registers.manifest"));

        for (class, values, width, allocatable, reserved, role, debug) in rows {
            assert!(
                classes
                    .insert(class, (values, width, allocatable, reserved, role, debug))
                    .is_none(),
                "duplicate register class {class}"
            );
            assert!(!values.is_empty(), "empty register values for {class}");
            assert!(!width.is_empty(), "empty register width for {class}");
            assert!(
                !allocatable.is_empty(),
                "empty allocatable register set for {class}"
            );
            assert!(!role.is_empty(), "empty register role for {class}");
            assert!(!debug.is_empty(), "empty debug register role for {class}");
        }

        for class in ["gpr", "fdr", "fpr", "vr", "pcr", "special"] {
            assert!(
                classes.contains_key(class),
                "missing register class {class}"
            );
        }
        assert_eq!(classes["gpr"].0, manifest_field(target_manifest, "gpr"));
        assert_eq!(classes["fdr"].0, manifest_field(target_manifest, "fdr"));
        assert_eq!(classes["fpr"].0, manifest_field(target_manifest, "fpr"));
        assert_eq!(classes["vr"].0, manifest_field(target_manifest, "vr"));
        assert_eq!(classes["gpr"].1, "64");
        assert_eq!(classes["gpr"].2, "r1-r29");
        assert!(classes["gpr"].3.contains(&"r0"));
        assert!(classes["gpr"].3.contains(&"r30"));
        assert!(
            classes["gpr"]
                .3
                .contains(&manifest_field(psabi_manifest, "stack_pointer"))
        );
        assert!(
            classes["special"]
                .0
                .split(',')
                .any(|value| value == manifest_field(psabi_manifest, "link_register"))
        );
        assert!(
            classes["special"]
                .0
                .split(',')
                .any(|value| value == manifest_field(psabi_manifest, "thread_pointer_pcr"))
        );
        assert!(
            classes["special"]
                .0
                .split(',')
                .any(|value| value == "FLAGS")
        );
        assert!(classes["special"].3.contains(&"FLAGS"));
        assert!(classes["special"].4.contains("hidden_compare_flags"));

        for pcr in ["PID", "PPID", "TID", "TP", "SIGMASK", "SIGPENDING"] {
            assert!(
                classes["pcr"].0.split(',').any(|value| value == pcr),
                "missing PCR {pcr}"
            );
        }
        for (constraint, class, values, _usage) in inline_asm_rows(inline_asm_manifest) {
            if ["gpr", "fdr", "fpr", "vr"].contains(&class) {
                assert_eq!(
                    classes[class].0, values,
                    "inline asm constraint {constraint} disagrees with register class {class}"
                );
            }
        }
        for register in ["r0-r31", "LR", "TP"] {
            assert!(manifest_csv_contains(
                debug_unwind_manifest,
                "register_numbers",
                register
            ));
        }
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
            constraints["d"],
            ("fpr", manifest_field(target_manifest, "fpr"))
        );
        assert_eq!(
            constraints["v"],
            ("vr", manifest_field(target_manifest, "vr"))
        );
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
        let loader_source = include_str!("loader.rs");
        let roadmap = include_str!("../toolchain_roadmap.md");
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
        for (idx, (number, name, calculation, loader_status)) in rows.iter().enumerate() {
            assert_eq!(*number as usize, idx, "relocation numbers must be dense");
            assert!(
                numbers.insert(*number),
                "duplicate relocation number {number}"
            );
            assert!(names.insert(*name), "duplicate relocation name {name}");
            assert!(!calculation.is_empty(), "empty calculation for {name}");
            assert!(
                loader_status.starts_with("supported_") || loader_status.starts_with("planned_"),
                "unknown loader status {loader_status} for {name}"
            );
            assert!(
                object_format.contains(&format!("| {number} | `{name}` |")),
                "relocation {number},{name} is missing from object_format.md"
            );
            assert!(
                target_relocations.contains(name),
                "relocation manifest {name} is missing from target manifest"
            );
            if loader_status.starts_with("supported_") {
                assert!(
                    loader_source.contains(&format!("const {name}:")),
                    "loader-supported relocation {name} is missing from loader constants"
                );
                if *name != "R_LNP64_NONE" {
                    assert!(
                        roadmap.contains(name),
                        "loader-supported relocation {name} is missing from toolchain roadmap"
                    );
                }
            }
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

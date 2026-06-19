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

    fn intrinsic_lowering_rows(manifest: &str) -> Vec<(&str, &str, &str, &str, Vec<&str>, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(6, '|');
                let name = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing intrinsic lowering name in {line}"));
                let primitive = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing intrinsic lowering primitive in {line}"));
                let abi_shape = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing intrinsic lowering ABI shape in {line}"));
                let status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing intrinsic lowering status in {line}"));
                let evidence = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing intrinsic lowering evidence in {line}"))
                    .split(',')
                    .collect();
                let blocker = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing intrinsic lowering blocker in {line}"));
                (name, primitive, abi_shape, status, evidence, blocker)
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

    fn toy_retirement_queue_rows(manifest: &str) -> Vec<(&str, &str, Vec<&str>, &str, &str)> {
        manifest
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let mut fields = line.splitn(5, '|');
                let surface = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing toy retirement surface in {line}"));
                let status = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing toy retirement status in {line}"));
                let toy_artifacts = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing toy retirement artifacts in {line}"))
                    .split(',')
                    .collect();
                let replacement_target = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing toy retirement replacement in {line}"));
                let blocker = fields
                    .next()
                    .unwrap_or_else(|| panic!("missing toy retirement blocker in {line}"));
                (surface, status, toy_artifacts, replacement_target, blocker)
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
            "intrinsic_lowering",
            "intrinsic_header",
            "clang_driver",
            "llvm_filemap",
            "libc_shim",
            "netbsd_layers",
            "conformance_gates",
            "toy_compiler_policy",
            "toy_retirement_queue",
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
            "minilibc_smoke",
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
        let libc_test_driver = include_str!("../scripts/run_libc_test.sh");
        let real_tblgen = include_str!("../scripts/run_real_llvm_tblgen.sh");
        let real_tblgen_docker = include_str!("../scripts/run_real_llvm_tblgen_docker.sh");
        let real_llc = include_str!("../scripts/run_real_llvm_lnp64.sh");
        let real_llc_docker = include_str!("../scripts/run_real_llvm_lnp64_docker.sh");
        let real_objects_docker = include_str!("../scripts/run_real_llvm_lnp64_objects_docker.sh");
        let real_mc_docker = include_str!("../scripts/run_real_llvm_lnp64_mc_docker.sh");
        let real_clang_target = include_str!("../clang/lib/Basic/Targets/LNP64.cpp");
        let llvm_dockerfile = include_str!("../Dockerfile.llvm");
        let errno_header = include_str!("../toolchain/include/errno.h");
        let netinet_in_header = include_str!("../toolchain/include/netinet/in.h");
        let poll_header = include_str!("../toolchain/include/poll.h");
        let search_header = include_str!("../toolchain/include/search.h");
        let pthread_header = include_str!("../toolchain/include/pthread.h");
        let semaphore_header = include_str!("../toolchain/include/semaphore.h");
        let signal_header = include_str!("../toolchain/include/signal.h");
        let stdarg_header = include_str!("../toolchain/include/stdarg.h");
        let stddef_header = include_str!("../toolchain/include/stddef.h");
        let stdint_header = include_str!("../toolchain/include/stdint.h");
        let stdio_header = include_str!("../toolchain/include/stdio.h");
        let stdlib_header = include_str!("../toolchain/include/stdlib.h");
        let string_header = include_str!("../toolchain/include/string.h");
        let sys_mman_header = include_str!("../toolchain/include/sys/mman.h");
        let sys_epoll_header = include_str!("../toolchain/include/sys/epoll.h");
        let sys_event_header = include_str!("../toolchain/include/sys/event.h");
        let sys_select_header = include_str!("../toolchain/include/sys/select.h");
        let sys_auxv_header = include_str!("../toolchain/include/sys/auxv.h");
        let sys_socket_header = include_str!("../toolchain/include/sys/socket.h");
        let sys_timerfd_header = include_str!("../toolchain/include/sys/timerfd.h");
        let time_header = include_str!("../toolchain/include/time.h");
        let unistd_header = include_str!("../toolchain/include/unistd.h");
        let libc_string_min = include_str!("../toolchain/liblnp64_string_min.c");
        let libc_convert_min = include_str!("../toolchain/liblnp64_convert_min.c");
        let libc_path_min = include_str!("../toolchain/liblnp64_path_min.c");
        let libc_search_min = include_str!("../toolchain/liblnp64_search_min.c");
        let libc_sort_min = include_str!("../toolchain/liblnp64_sort_min.c");
        let libc_alloc_min = include_str!("../toolchain/liblnp64_alloc_min.c");
        let libc_fd_min = include_str!("../toolchain/liblnp64_fd_min.c");
        let libc_meta_min = include_str!("../toolchain/liblnp64_meta_min.c");
        let libc_process_min = include_str!("../toolchain/liblnp64_process_min.c");
        let libc_errno_min = include_str!("../toolchain/liblnp64_errno_min.c");
        let libc_startup_min = include_str!("../toolchain/liblnp64_startup_min.c");
        let libc_random_min = include_str!("../toolchain/liblnp64_random_min.c");
        let libc_stdio_min = include_str!("../toolchain/liblnp64_stdio_min.c");
        let libc_time_min = include_str!("../toolchain/liblnp64_time_min.c");
        let libc_vma_min = include_str!("../toolchain/liblnp64_vma_min.c");
        let libc_futex_min = include_str!("../toolchain/liblnp64_futex_min.c");
        let lnp64_futex_header = include_str!("../toolchain/include/lnp64/futex.h");
        let lnp64_intrinsics_target_header =
            include_str!("../toolchain/include/lnp64/intrinsics.h");
        let libc_pthread_min = include_str!("../toolchain/liblnp64_pthread_min.c");
        let libc_sem_min = include_str!("../toolchain/liblnp64_sem_min.c");
        let libc_poll_min = include_str!("../toolchain/liblnp64_poll_min.c");
        let libc_signal_min = include_str!("../toolchain/liblnp64_signal_min.c");
        let libc_socket_min = include_str!("../toolchain/liblnp64_socket_min.c");
        let libc_sbase_min = include_str!("../toolchain/liblnp64_sbase_min.c");
        let elf_exec_test_clang = include_str!("../userland/elf_exec_test_clang.c");
        let spawn_task_clang = include_str!("../userland/spawn_task_clang.c");
        let gate_trace_test_clang = include_str!("../userland/gate_trace_test_clang.c");
        let fd_passing_test_clang = include_str!("../userland/fd_passing_test_clang.c");
        let classifier_test_clang = include_str!("../userland/classifier_test_clang.c");
        let domain_ctl_clang = include_str!("../userland/domain_ctl_clang.h");
        let netbsd_init_clang = include_str!("../userland/netbsd_init_clang.c");
        let netbsd_personality_clang = include_str!("../userland/netbsd_personality_clang_smoke.c");
        let netbsd_sh_clang = include_str!("../userland/netbsd_sh_clang.c");
        let poll_test_clang = include_str!("../userland/poll_test_clang.c");
        let signal_gate_test_clang = include_str!("../userland/signal_gate_test_clang.c");
        let signal_fault_test_clang = include_str!("../userland/signal_fault_test_clang.c");
        let socket_loopback_test_clang = include_str!("../userland/socket_loopback_test_clang.c");
        let timer_test_clang = include_str!("../userland/timer_test_clang.c");
        let httpd_demo = include_str!("../demos/httpd.c");
        let netcat_demo = include_str!("../demos/netcat.c");
        let parallel_hash_demo = include_str!("../demos/parallel_hash.c");
        let ping_pong_demo = include_str!("../demos/ping_pong.c");
        let producer_consumer_demo = include_str!("../demos/producer_consumer.c");
        let sqlite_lite_demo = include_str!("../demos/sqlite_lite.c");
        let lnp64_isel_lowering = include_str!("../llvm/lib/Target/LNP64/LNP64ISelLowering.cpp");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");

        for minilibc_intrinsic_source in [
            libc_alloc_min,
            libc_fd_min,
            libc_futex_min,
            libc_poll_min,
            libc_process_min,
            libc_pthread_min,
            libc_sem_min,
            libc_signal_min,
            libc_socket_min,
            libc_startup_min,
            libc_time_min,
            libc_vma_min,
        ] {
            assert!(minilibc_intrinsic_source.contains("#include <lnp64/intrinsics.h>"));
            assert!(!minilibc_intrinsic_source.contains("#include \"lnp64_intrinsics.h\""));
        }
        for native_demo_source in [
            httpd_demo,
            netcat_demo,
            parallel_hash_demo,
            ping_pong_demo,
            producer_consumer_demo,
            sqlite_lite_demo,
        ] {
            assert!(native_demo_source.contains("#include <lnp64/intrinsics.h>"));
            assert!(!native_demo_source.contains("#include \"lnp64_intrinsics.h\""));
        }
        let real_llc_lines: Vec<_> = real_llc.lines().collect();
        for (index, line) in real_llc_lines.iter().enumerate() {
            if line.contains("-I toolchain") && line.trim_end().ends_with('\\') {
                let next = real_llc_lines
                    .get(index + 1)
                    .expect("continued include line must have a following line");
                assert!(
                    line.contains("-I toolchain/include") || next.contains("-I toolchain/include"),
                    "real LLVM compile line {} must include installed target headers",
                    index + 1
                );
            }
            assert!(
                !line.contains("-I toolchain -c")
                    || line.contains("-I toolchain -I toolchain/include -c"),
                "single-line real LLVM compile line {} must include installed target headers",
                index + 1
            );
        }
        let rows = llvm_gate_rows(gate_manifest);
        let mut gates = std::collections::BTreeSet::new();
        let mut commands = std::collections::BTreeMap::new();
        let mut statuses = std::collections::BTreeMap::new();
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
        assert!(
            manifest_root
                .join("scripts/run_real_llvm_tblgen.sh")
                .is_file()
        );
        assert!(
            manifest_root
                .join("scripts/run_real_llvm_tblgen_docker.sh")
                .is_file()
        );
        assert!(
            manifest_root
                .join("scripts/run_real_llvm_lnp64.sh")
                .is_file()
        );
        assert!(
            manifest_root
                .join("scripts/run_real_llvm_lnp64_docker.sh")
                .is_file()
        );
        assert!(
            manifest_root
                .join("scripts/run_real_llvm_lnp64_mc_docker.sh")
                .is_file()
        );
        assert!(manifest_root.join("Dockerfile.llvm").is_file());
        assert!(contract_index.contains(
            "llvm_gates|toolchain/lnp64_llvm_gates.manifest|llvm_gate_manifest_pins_non_toy_clang_commands"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_llvm_gates.manifest"));
        assert!(transition_manifest.contains("scripts/run_llvm_bootstrap_gates.sh"));
        assert!(roadmap.contains("toolchain/lnp64_llvm_gates.manifest"));
        assert!(roadmap.contains("scripts/run_llvm_bootstrap_gates.sh --dry-run"));
        assert!(roadmap.contains("scripts/run_real_llvm_tblgen_docker.sh"));
        assert!(roadmap.contains("scripts/run_real_llvm_lnp64_docker.sh"));
        assert!(roadmap.contains("Dockerfile.llvm"));

        for (gate, command, requirements, status) in rows {
            assert!(gates.insert(gate), "duplicate llvm gate {gate}");
            commands.insert(gate, command);
            statuses.insert(gate, status);
            assert!(
                ["planned", "tested"].contains(&status),
                "unknown LLVM gate status {status} for {gate}"
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
        assert_eq!(statuses["real_llc_build"], "tested");
        assert_eq!(statuses["real_mc_build"], "tested");
        assert_eq!(statuses["simple_libc_gate"], "tested");
        for requirement in [
            "clang_zlib_adler32_object",
            "clang_zlib_crc32_object",
            "clang_zlib_package_object",
            "clang_natsort_package_object",
            "clang_jsmn_package_object",
            "clang_inih_package_object",
            "clang_cwalk_package_object",
            "clang_varargs_call_object",
            "clang_sbase_command_objects",
            "clang_sbase_libutil_objects",
            "clang_sbase_support_object",
            "clang_userland_ucat_object",
            "clang_userland_init_object",
            "clang_userland_lnpsh_object",
            "clang_userland_spawn_task_object",
            "clang_netbsd_init_object",
            "clang_netbsd_shell_object",
            "clang_netbsd_loader_target_child_object",
            "clang_netbsd_elf_exec_parent_object",
            "clang_netbsd_fork_wait_child_object",
            "clang_netbsd_thread_child_object",
            "clang_netbsd_poll_child_object",
            "clang_netbsd_signal_gate_child_object",
            "clang_netbsd_signal_fault_child_object",
            "clang_netbsd_timer_child_object",
            "clang_netbsd_mmap_child_object",
            "clang_netbsd_fd_passing_child_object",
            "clang_netbsd_fs_service_child_object",
            "clang_netbsd_classifier_child_object",
            "clang_netbsd_socket_loopback_child_object",
            "clang_netbsd_gate_trace_child_object",
            "clang_netbsd_domain_nested_child_object",
            "clang_netbsd_domain_budget_child_object",
            "clang_minilibc_meta_impl_object",
            "clang_meta_libc_object",
            "clang_minilibc_random_impl_object",
            "clang_minilibc_stdio_impl_object",
            "clang_libc_test_argv_object",
            "clang_libc_test_env_object",
            "clang_libc_test_random_object",
            "clang_libc_test_string_memcpy_bounded_object",
            "clang_libc_test_string_memmove_bounded_object",
            "clang_libc_test_search_insque_object",
            "clang_libc_test_malloc_0_object",
            "clang_libc_test_fgets_eof_object",
            "clang_libc_test_access_bounded_object",
            "clang_libc_test_stat_object",
            "clang_libc_test_utime_object",
            "clang_libc_test_fdopen_object",
            "clang_libc_test_fcntl_basic_bounded_object",
            "clang_libc_test_pthread_tsd_object",
            "clang_libc_test_sem_init_object",
            "clang_minilibc_pthread_impl_object",
            "clang_minilibc_sem_impl_object",
            "zlib_package_static_link",
            "natsort_package_static_link",
            "jsmn_package_static_link",
            "inih_package_static_link",
            "cwalk_package_static_link",
            "libc_test_argv_static_link",
            "libc_test_env_static_link",
            "libc_test_random_static_link",
            "libc_test_ctype_static_link",
            "libc_test_string_static_link",
            "libc_test_string_memcpy_bounded_static_link",
            "libc_test_string_memmove_bounded_static_link",
            "libc_test_string_memmem_static_link",
            "libc_test_string_strchr_static_link",
            "libc_test_string_strcspn_static_link",
            "libc_test_string_strstr_static_link",
            "libc_test_udiv_static_link",
            "libc_test_basename_static_link",
            "libc_test_dirname_static_link",
            "libc_test_strtol_static_link",
            "libc_test_clock_gettime_static_link",
            "libc_test_access_bounded_static_link",
            "libc_test_stat_static_link",
            "libc_test_utime_static_link",
            "libc_test_fdopen_static_link",
            "libc_test_fcntl_basic_bounded_static_link",
            "libc_test_pthread_tsd_static_link",
            "libc_test_sem_init_static_link",
            "libc_test_qsort_bounded_static_link",
            "libc_test_search_insque_static_link",
            "libc_test_malloc_0_static_link",
            "libc_test_fgets_eof_static_link",
            "zlib_package_run_elf",
            "natsort_package_run_elf",
            "jsmn_package_run_elf",
            "inih_package_run_elf",
            "cwalk_package_run_elf",
            "libc_test_argv_run_elf",
            "libc_test_env_run_elf",
            "libc_test_random_run_elf",
            "libc_test_ctype_run_elf",
            "libc_test_string_run_elf",
            "libc_test_string_memcpy_bounded_run_elf",
            "libc_test_string_memmove_bounded_run_elf",
            "libc_test_string_memmem_run_elf",
            "libc_test_string_strchr_run_elf",
            "libc_test_string_strcspn_run_elf",
            "libc_test_string_strstr_run_elf",
            "libc_test_udiv_run_elf",
            "libc_test_basename_run_elf",
            "libc_test_dirname_run_elf",
            "libc_test_strtol_run_elf",
            "libc_test_clock_gettime_run_elf",
            "libc_test_access_bounded_run_elf",
            "libc_test_stat_run_elf",
            "libc_test_utime_run_elf",
            "libc_test_fdopen_run_elf",
            "libc_test_fcntl_basic_bounded_run_elf",
            "libc_test_pthread_tsd_run_elf",
            "libc_test_sem_init_run_elf",
            "libc_test_qsort_bounded_run_elf",
            "libc_test_search_insque_run_elf",
            "libc_test_malloc_0_run_elf",
            "libc_test_fgets_eof_run_elf",
            "sbase_echo_static_link",
            "sbase_echo_run_elf",
            "sbase_path_static_link",
            "sbase_path_run_elf",
            "sbase_cat_static_link",
            "sbase_cat_run_elf",
            "userland_ucat_static_link",
            "userland_ucat_run_elf",
            "userland_init_static_link",
            "userland_init_run_elf",
            "userland_lnpsh_static_link",
            "userland_lnpsh_run_elf",
            "userland_spawn_task_static_link",
            "userland_spawn_task_run_elf",
            "netbsd_init_static_link",
            "netbsd_shell_static_link",
            "netbsd_init_shell_system_run_elf",
            "netbsd_loader_target_child_static_link",
            "netbsd_loader_target_child_run_elf",
            "netbsd_elf_exec_parent_static_link",
            "netbsd_elf_exec_parent_run_elf",
            "netbsd_fork_wait_child_static_link",
            "netbsd_fork_wait_child_run_elf",
            "netbsd_thread_child_static_link",
            "netbsd_thread_child_run_elf",
            "netbsd_poll_child_static_link",
            "netbsd_poll_child_run_elf",
            "netbsd_signal_gate_child_static_link",
            "netbsd_signal_gate_child_run_elf",
            "netbsd_signal_fault_child_static_link",
            "netbsd_signal_fault_child_run_elf",
            "netbsd_timer_child_static_link",
            "netbsd_timer_child_run_elf",
            "netbsd_mmap_child_static_link",
            "netbsd_mmap_child_run_elf",
            "netbsd_fd_passing_child_static_link",
            "netbsd_fd_passing_child_run_elf",
            "netbsd_namespace_child_static_link",
            "netbsd_namespace_child_run_elf",
            "netbsd_fs_service_child_static_link",
            "netbsd_fs_service_child_run_elf",
            "netbsd_classifier_child_static_link",
            "netbsd_classifier_child_run_elf",
            "netbsd_socket_loopback_child_static_link",
            "netbsd_socket_loopback_child_run_elf",
            "netbsd_gate_trace_child_static_link",
            "netbsd_gate_trace_child_run_elf",
            "netbsd_domain_nested_child_static_link",
            "netbsd_domain_nested_child_run_elf",
            "netbsd_domain_budget_child_static_link",
            "netbsd_domain_budget_child_run_elf",
            "metadata_libc_static_link",
            "metadata_libc_run_elf",
        ] {
            assert!(
                gate_manifest.contains(requirement),
                "real LLVM gate manifest missing package requirement {requirement}"
            );
        }
        assert!(
            !lnp64_isel_lowering.contains("varargs call lowering is not implemented yet"),
            "real LLVM backend must lower ordinary calls to variadic prototypes"
        );

        for gate in [
            "gate_driver",
            "real_tblgen",
            "real_mc_build",
            "real_objects_build",
            "real_llc_build",
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
        assert!(
            commands["real_tblgen"].contains("scripts/run_real_llvm_tblgen_docker.sh"),
            "real LLVM TableGen gate must run through the Docker-backed script"
        );
        assert!(
            commands["real_llc_build"].contains("scripts/run_real_llvm_lnp64_docker.sh"),
            "real LLVM llc gate must run through the Docker-backed script"
        );
        assert!(
            commands["real_objects_build"]
                .contains("bash scripts/run_real_llvm_lnp64_objects_docker.sh"),
            "real LLVM object gate must run through the Docker-backed script"
        );
        assert!(
            commands["real_mc_build"].contains("scripts/run_real_llvm_lnp64_mc_docker.sh"),
            "real LLVM MC gate must run through the Docker-backed script"
        );
        assert!(
            commands["simple_libc_gate"]
                .contains("scripts/run_libc_test.sh --backend llvm --loader exec-plan"),
            "simple libc replacement gate must request the LLVM/exec-plan backend"
        );
        assert!(gate_driver.contains("toolchain/lnp64_llvm_gates.manifest"));
        assert!(gate_driver.contains("--dry-run"));
        assert!(gate_driver.contains("--run"));
        assert!(gate_driver.contains("LNP64_LLVM_GATE_FILTER"));
        assert!(gate_driver.contains("filter_allows_gate"));
        assert!(gate_driver.contains("no LLVM gate rows matched"));
        assert!(gate_driver.contains("LNP64_RUN_PLANNED_LLVM_GATES"));
        assert!(gate_driver.contains("skipping planned gate"));
        assert!(gate_driver.contains(r"command//\{build\}/"));
        assert!(!gate_driver.contains("lnp64 cc"));
        assert!(!gate_driver.contains("cargo run -- cc"));
        assert!(libc_test_driver.contains("--backend toy|llvm"));
        assert!(libc_test_driver.contains("backend=\"llvm\""));
        assert!(libc_test_driver.contains("loader=\"exec-plan\""));
        assert!(libc_test_driver.contains("--loader asm|exec-plan"));
        assert!(libc_test_driver.contains("exec bash scripts/run_real_llvm_lnp64_docker.sh"));
        assert!(libc_test_driver.contains("llvm backend requires --loader exec-plan"));
        assert!(libc_test_driver.contains("Use\n--backend toy --loader asm"));
        assert!(libc_test_driver.contains("lnp64 cc --toy-bootstrap"));
        assert!(real_tblgen.contains("llvm-tblgen"));
        assert!(real_tblgen.contains("llvm-config"));
        assert!(real_tblgen.contains("-gen-register-info"));
        assert!(real_tblgen.contains("-gen-instr-info"));
        assert!(real_tblgen.contains("-gen-callingconv"));
        assert!(real_tblgen.contains("-gen-subtarget"));
        assert!(real_tblgen_docker.contains("Dockerfile.llvm"));
        assert!(real_tblgen_docker.contains("scripts/run_real_llvm_tblgen.sh"));
        assert!(real_tblgen_docker.contains("LNP64_LLVM_DOCKER_SKIP_BUILD"));
        assert!(real_tblgen_docker.contains(r#"--user "$uid:$gid""#));
        assert!(real_llc_docker.contains("LNP64_LLVM_DOCKER_SKIP_BUILD"));
        assert!(real_llc_docker.contains("LNP64_LLVM_DOCKER_SKIP_RUN_ELF"));
        assert!(real_llc_docker.contains(r#"LNP64_LLVM_GATE="${LNP64_LLVM_GATE:-full}""#));
        assert!(real_llc_docker.contains("run-elf execution skipped by LNP64_LLVM_GATE"));
        assert!(real_objects_docker.contains("LNP64_LLVM_GATE=objects"));
        assert!(real_objects_docker.contains("scripts/run_real_llvm_lnp64_docker.sh"));
        assert!(real_mc_docker.contains("LNP64_LLVM_DOCKER_SKIP_BUILD"));
        assert!(real_llc.contains("llvmorg-14.0.6"));
        assert!(real_llc.contains("LNP64_LLVM_GATE"));
        assert!(real_llc.contains("full|mc|objects"));
        assert!(real_llc.contains("ninja -C \"$build_dir\" -j \"$jobs\" llvm-mc llvm-objdump"));
        assert!(
            real_llc
                .contains("ninja -C \"$build_dir\" -j \"$jobs\" llc llvm-mc llvm-objdump clang")
        );
        assert!(real_llc.contains("real LLVM LNP64 object-only gate passed"));
        assert!(real_llc.contains("real LLVM LNP64 llvm-objdump crt0 decode smoke passed"));
        assert!(real_llc.contains("git clone"));
        assert!(
            real_llc.contains("git -C \"$project_dir\" sparse-checkout set llvm cmake clang lld")
        );
        assert!(real_llc.contains("LLVM_ENABLE_PROJECTS=\"clang;lld\""));
        assert!(real_llc.contains("LLVM_TARGETS_TO_BUILD=LNP64"));
        assert!(real_llc.contains(r#"ninja -C "$build_dir""#));
        assert!(real_llc.contains("llc llvm-mc llvm-objdump clang lld"));
        assert!(real_llc.contains(r#"llc="$build_dir/bin/llc""#));
        assert!(real_llc.contains(r#"clang="$build_dir/bin/clang""#));
        assert!(real_llc.contains(r#"llvm_mc="$build_dir/bin/llvm-mc""#));
        assert!(real_llc.contains(r#"llvm_objdump="$build_dir/bin/llvm-objdump""#));
        assert!(real_llc.contains(r#"lld="$build_dir/bin/lld""#));
        assert!(real_llc.contains(r#""$llc" --version"#));
        assert!(real_llc.contains("clang/lib/Basic/Targets/LNP64.h"));
        assert!(real_llc.contains("clang/lib/Basic/Targets/LNP64.cpp"));
        assert!(real_clang_target.contains("MaxAtomicInlineWidth = 64"));
        assert!(real_clang_target.contains("MaxAtomicPromoteWidth = MaxAtomicInlineWidth"));
        assert!(real_llc.contains("clang/lib/Driver/ToolChains/Arch/LNP64.cpp"));
        assert!(real_llc.contains("Targets/LNP64.cpp"));
        assert!(real_llc.contains("BareMetal(Triple)"));
        assert!(real_llc.contains("lld/ELF/Arch/LNP64.cpp"));
        assert!(real_llc.contains("elf64lnp64"));
        assert!(real_llc.contains("getLNP64TargetInfo"));
        assert!(real_llc.contains("ELF-only LNP64 smoke linker"));
        assert!(real_llc.contains("-verify-machineinstrs"));
        assert!(real_llc.contains("-filetype=obj"));
        assert!(real_llc.contains("real LLVM LNP64 llc smoke passed"));
        assert!(real_llc.contains("--target=lnp64-unknown-none"));
        assert!(real_llc.contains("-fno-jump-tables"));
        assert!(real_llc.contains("int main(void)"));
        assert!(real_llc.contains("scalar-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang scalar compile smoke passed"));
        assert!(real_llc.contains("scalar-arith-clang-smoke.o"));
        assert!(real_llc.contains("grep -q 'addi r'"));
        assert!(real_llc.contains("grep -q 'udiv r'"));
        assert!(real_llc.contains("grep -q 'srem r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang scalar arithmetic object smoke passed"));
        assert!(real_llc.contains("high-mul-clang-smoke.o"));
        assert!(real_llc.contains("grep -q 'mulhu r'"));
        assert!(real_llc.contains("grep -q 'mulh r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang high-multiply object smoke passed"));
        assert!(real_llc.contains("high-mul-mc-smoke.o"));
        assert!(real_llc.contains("mulhsu r7, r8, r9"));
        assert!(real_llc.contains("real LLVM LNP64 llvm-mc high-multiply smoke passed"));
        assert!(real_llc.contains("auipc-mc-smoke.o"));
        assert!(real_llc.contains("auipc r1, 4096"));
        assert!(real_llc.contains("real LLVM LNP64 llvm-mc auipc smoke passed"));
        assert!(real_llc.contains("mmap-mc-smoke.o"));
        assert!(real_llc.contains("mmap r1, r2, r3, r4"));
        assert!(real_llc.contains("munmap r5, r6"));
        assert!(real_llc.contains("mprotect r7, r8, r9, r10"));
        assert!(real_llc.contains("real LLVM LNP64 llvm-mc mmap opcode smoke passed"));
        assert!(real_llc.contains("env-get-mc-smoke.o"));
        assert!(real_llc.contains("env_get r1, r2, r3, r4"));
        assert!(real_llc.contains("real LLVM LNP64 llvm-mc env_get opcode smoke passed"));
        assert!(real_llc.contains("get-pcr-mc-smoke.o"));
        assert!(real_llc.contains("get_pcr r1, PID"));
        assert!(real_llc.contains("set_pcr r3, SIGMASK, r2"));
        assert!(real_llc.contains("stale two-operand SET_PCR unexpectedly assembled"));
        assert!(real_llc.contains("real LLVM LNP64 llvm-mc GET_PCR opcode smoke passed"));
        assert!(real_llc.contains("open-at-mc-smoke.o"));
        assert!(real_llc.contains("open_at r1, r2, r3, r4"));
        assert!(real_llc.contains("real LLVM LNP64 llvm-mc OPEN_AT opcode smoke passed"));
        assert!(real_llc.contains("clone-control-mc-smoke.o"));
        assert!(real_llc.contains("clone.spawn r1, r2, r3"));
        assert!(real_llc.contains("thread_join r4, r5, r6"));
        assert!(real_llc.contains("real LLVM LNP64 llvm-mc clone control opcode smoke passed"));
        assert!(real_llc.contains("compat-meta-mc-smoke.o"));
        assert!(real_llc.contains("stat_path_at r1, r2, r3, r4"));
        assert!(real_llc.contains("stat_fd_dyn r5, r6"));
        assert!(real_llc.contains("utime_path_at r7, r8, r9, r10"));
        assert!(real_llc.contains("utime_fd_dyn r11, r12"));
        assert!(real_llc.contains("fcntl_fd_dyn r13, r14, r15"));
        assert!(
            real_llc.contains("real LLVM LNP64 llvm-mc compatibility metadata opcode smoke passed")
        );
        assert!(real_llc.contains("cap-control-mc-smoke.o"));
        assert!(real_llc.contains("cap_dup r1, r2"));
        assert!(real_llc.contains("cap_send r3, r4"));
        assert!(real_llc.contains("cap_recv r5, r6"));
        assert!(real_llc.contains("cap_revoke r7, r8"));
        assert!(
            real_llc.contains("real LLVM LNP64 llvm-mc capability control opcode smoke passed")
        );
        assert!(real_llc.contains("atomic-mc-smoke.o"));
        assert!(real_llc.contains("amo.swap r1, r2, r3"));
        assert!(real_llc.contains("amo.or r10, r11, r12"));
        assert!(real_llc.contains("lock.cmpxchg r13, r14, r15, r16"));
        assert!(real_llc.contains("amo.xor r17, r18, r19"));
        assert!(real_llc.contains("futex_wait r20, r21"));
        assert!(real_llc.contains("futex_wake r22, r23"));
        assert!(real_llc.contains("fence.acq_rel"));
        assert!(real_llc.contains("isync r24, r25, r26"));
        assert!(real_llc.contains("real LLVM LNP64 llvm-mc atomic opcode smoke passed"));
        assert!(real_llc.contains("signal-alias-mc-smoke.o"));
        assert!(real_llc.contains("sigaction r1, r2"));
        assert!(real_llc.contains("sigmask_set r3"));
        assert!(real_llc.contains("kill r4, r5"));
        assert!(real_llc.contains("alarm r6, r7"));
        assert!(real_llc.contains("yield"));
        assert!(real_llc.contains("sigret"));
        assert!(real_llc.contains("real LLVM LNP64 llvm-mc signal alias opcode smoke passed"));
        assert!(real_llc.contains("scalar-extend-clang-smoke.o"));
        assert!(real_llc.contains("grep -q 'zext.w r'"));
        assert!(real_llc.contains("grep -q 'sext.b r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang scalar extension object smoke passed"));
        assert!(real_llc.contains("signed-load-clang-smoke.o"));
        assert!(real_llc.contains("grep -q 'sext.h r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang signed-load object smoke passed"));
        assert!(real_llc.contains("bitmanip-clang-smoke.o"));
        assert!(real_llc.contains("grep -q 'clz r'"));
        assert!(real_llc.contains("grep -q 'bswap64 r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang bit-manip object smoke passed"));
        assert!(real_llc.contains("csel-clang-smoke.o"));
        assert!(real_llc.contains("grep -q 'csel.gt r'"));
        assert!(real_llc.contains("grep -q 'csel.ult r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang csel object smoke passed"));
        assert!(real_llc.contains("call-clobber-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang call-clobber object smoke passed"));
        assert!(real_llc.contains("debug-line-clang-smoke.o"));
        assert!(real_llc.contains("-g -gdwarf-5"));
        assert!(real_llc.contains("grep -q '.debug_info'"));
        assert!(real_llc.contains("grep -q '.debug_line'"));
        assert!(real_llc.contains("grep -q '.debug_frame'"));
        assert!(real_llc.contains("grep -q '.rela.debug_line'"));
        assert!(real_llc.contains("real LLVM LNP64 clang debug section smoke passed"));
        assert!(real_llc.contains("-c demos/hello.c"));
        assert!(real_llc.contains("hello-clang-smoke.o"));
        assert!(real_llc.contains("hello-clang-smoke.dump"));
        assert!(real_llc.contains("real LLVM LNP64 clang hello object smoke passed"));
        assert!(real_llc.contains("-c demos/factorial.c"));
        assert!(real_llc.contains("factorial-clang-smoke.o"));
        assert!(real_llc.contains("factorial-clang-smoke.dump"));
        assert!(real_llc.contains("ld.w r"));
        assert!(real_llc.contains("st.w r"));
        assert!(real_llc.contains("mul r"));
        assert!(real_llc.contains("cmp r"));
        assert!(real_llc.contains("real LLVM LNP64 clang factorial object smoke passed"));
        assert!(real_llc.contains("-c demos/allocator.c"));
        assert!(real_llc.contains("allocator-clang-smoke.o"));
        assert!(real_llc.contains("allocator-clang-smoke.dump"));
        assert!(real_llc.contains("real LLVM LNP64 clang allocator object smoke passed"));
        assert!(real_llc.contains("-c demos/fibonacci.c"));
        assert!(real_llc.contains("fibonacci-clang-smoke.o"));
        assert!(real_llc.contains("fibonacci-clang-smoke.dump"));
        assert!(real_llc.contains("<fib_recursive>:"));
        assert!(real_llc.contains("<main>:"));
        assert!(real_llc.contains("ret"));
        assert!(real_llc.contains("real LLVM LNP64 clang fibonacci object smoke passed"));
        assert!(real_llc.contains("indirect-call-clang-smoke.o"));
        assert!(real_llc.contains("call_reg"));
        assert!(real_llc.contains("real LLVM LNP64 clang indirect call object smoke passed"));
        assert!(real_llc.contains("intrinsic-await-clang-smoke.o"));
        assert!(real_llc.contains("await r"));
        assert!(real_llc.contains("real LLVM LNP64 clang intrinsic await object smoke passed"));
        assert!(real_llc.contains("intrinsic-call-clang-smoke.o"));
        assert!(real_llc.contains("gate_call r"));
        assert!(real_llc.contains("real LLVM LNP64 clang intrinsic call object smoke passed"));
        assert!(real_llc.contains("intrinsic-gate-return-clang-smoke.o"));
        assert!(real_llc.contains("gate_return r"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang intrinsic gate return object smoke passed")
        );
        assert!(real_llc.contains("intrinsic-control-clang-smoke.o"));
        assert!(real_llc.contains("object_ctl r"));
        assert!(real_llc.contains("domain_ctl r"));
        assert!(real_llc.contains("__lnp_object_create(999"));
        assert!(real_llc.contains("real LLVM LNP64 clang intrinsic control object smoke passed"));
        assert!(real_llc.contains("intrinsic-cap-control-clang-smoke.o"));
        assert!(real_llc.contains("cap_dup r"));
        assert!(real_llc.contains("cap_send r"));
        assert!(real_llc.contains("cap_recv r"));
        assert!(real_llc.contains("cap_revoke r"));
        assert!(
            real_llc
                .contains("real LLVM LNP64 clang intrinsic capability control object smoke passed")
        );
        assert!(real_llc.contains("lnp64-intrinsic-cap-control-linked.elf"));
        assert!(
            real_llc.contains("real LLVM LNP64 lld intrinsic capability control link smoke passed")
        );
        assert!(real_llc.contains("intrinsic-amo-clang-smoke.o"));
        assert!(real_llc.contains("amo.add r"));
        assert!(real_llc.contains("amo.xor r"));
        assert!(real_llc.contains("amo.swap r"));
        assert!(real_llc.contains("real LLVM LNP64 clang intrinsic AMO object smoke passed"));
        assert!(real_llc.contains("c11-atomic-clang-smoke.o"));
        assert!(real_llc.contains("__atomic_load_n"));
        assert!(real_llc.contains("__atomic_store_n"));
        assert!(real_llc.contains("__atomic_fetch_add"));
        assert!(real_llc.contains("__atomic_fetch_xor"));
        assert!(real_llc.contains("__atomic_compare_exchange_n"));
        assert!(real_llc.contains("grep -q 'lock.cmpxchg r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang C11 atomic object smoke passed"));
        assert!(real_llc.contains("exit-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang exit object smoke passed"));
        assert!(real_llc.contains("toolchain/liblnp64_process_min.c"));
        assert!(libc_process_min.contains("__lnp_exit"));
        assert!(libc_process_min.contains("int pid(void)"));
        assert!(libc_process_min.contains("int getpid(void)"));
        assert!(libc_process_min.contains("int getppid(void)"));
        assert!(libc_process_min.contains("unsigned int getuid(void)"));
        assert!(libc_process_min.contains("unsigned int getegid(void)"));
        assert!(libc_process_min.contains("__lnp_get_pid"));
        assert!(libc_process_min.contains("int fork(void)"));
        assert!(libc_process_min.contains("int waitpid(int pid, int *status, int options)"));
        assert!(libc_process_min.contains("int execve(const char *path"));
        assert!(libc_process_min.contains("int execl(const char *path"));
        assert!(libc_process_min.contains("#include <stdarg.h>"));
        assert!(libc_process_min.contains("lnp64_exec_compat"));
        assert!(libc_process_min.contains("lnp64_fork_compat"));
        assert!(libc_process_min.contains("lnp64_wait_pid_compat"));
        assert!(real_llc.contains("liblnp64-process-min.o"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$libc_process_impl_c\""));
        assert!(real_llc.contains("grep -q 'exit r'"));
        assert!(real_llc.contains("grep -q 'get_pcr r'"));
        assert!(real_llc.contains("grep -q 'fork r'"));
        assert!(real_llc.contains("grep -q 'wait_pid r'"));
        assert!(real_llc.contains("grep -q 'exec r'"));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang minilibc process implementation object smoke passed"
            )
        );
        assert!(real_llc.contains("toolchain/liblnp64_errno_min.c"));
        assert!(libc_errno_min.contains("ERRNO_GET") || libc_errno_min.contains("errno_get"));
        assert!(libc_errno_min.contains("ERRNO_SET") || libc_errno_min.contains("errno_set"));
        assert!(libc_errno_min.contains("lnp64_errno_initialized"));
        assert!(libc_errno_min.contains("__errno_location"));
        assert!(errno_header.contains("int *__errno_location(void);"));
        assert!(errno_header.contains("#define errno (*__errno_location())"));
        for signal_define in [
            "#define SIGINT 2",
            "#define SIGFPE 8",
            "#define SIGSEGV 11",
            "#define SIGALRM 14",
            "#define SIGTERM 15",
        ] {
            assert!(signal_header.contains(signal_define));
        }
        assert!(signal_header.contains("__lnp64_sighandler_t signal"));
        assert!(signal_header.contains("int raise(int signum);"));
        assert!(real_llc.contains("liblnp64-errno-min.o"));
        assert!(real_llc.contains("grep -q 'errno_get r'"));
        assert!(real_llc.contains("grep -q 'errno_set r'"));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang minilibc errno implementation object smoke passed"
            )
        );
        assert!(real_llc.contains("toolchain/liblnp64_startup_min.c"));
        assert!(libc_startup_min.contains("getauxval"));
        assert!(sys_auxv_header.contains("#define AT_PAGESZ 6"));
        assert!(sys_auxv_header.contains("#define AT_HWCAP 16"));
        assert!(sys_auxv_header.contains("#define AT_RANDOM 25"));
        assert!(sys_auxv_header.contains("unsigned long getauxval(unsigned long type);"));
        assert!(libc_startup_min.contains("char **environ"));
        assert!(libc_startup_min.contains("char *getenv("));
        assert!(libc_startup_min.contains("int setenv("));
        assert!(libc_startup_min.contains("int unsetenv("));
        assert!(libc_startup_min.contains("int clearenv("));
        assert!(libc_startup_min.contains("int putenv("));
        assert!(libc_startup_min.contains("env_get %0, %1, %2, %3"));
        assert!(unistd_header.contains("extern char **environ;"));
        assert!(real_llc.contains("liblnp64-startup-min.o"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$libc_startup_impl_c\""));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang minilibc startup implementation object smoke passed"
            )
        );
        assert!(real_llc.contains("toolchain/liblnp64_random_min.c"));
        assert!(libc_random_min.contains("#include <stdlib.h>"));
        assert!(libc_random_min.contains("long random(void)"));
        assert!(libc_random_min.contains("void srandom(unsigned int seed)"));
        assert!(libc_random_min.contains("char *initstate("));
        assert!(libc_random_min.contains("char *setstate("));
        assert!(real_llc.contains("liblnp64-random-min.o"));
        assert!(real_llc.contains("grep -q '<random>:'"));
        assert!(real_llc.contains("grep -q '<srandom>:'"));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang minilibc random implementation object smoke passed"
            )
        );
        assert!(real_llc.contains("toolchain/liblnp64_time_min.c"));
        assert!(libc_time_min.contains("clock_gettime"));
        assert!(libc_time_min.contains("get_pcr %0, REALTIME_SEC"));
        assert!(libc_time_min.contains("get_pcr %0, REALTIME_NSEC"));
        assert!(libc_time_min.contains("int usleep(unsigned int usec)"));
        assert!(libc_time_min.contains("unsigned int sleep(unsigned int seconds)"));
        assert!(libc_time_min.contains("int timerfd_create(int clockid, int flags)"));
        assert!(libc_time_min.contains("int timerfd_settime("));
        assert!(libc_time_min.contains("int timerfd_gettime("));
        assert!(libc_time_min.contains("LNP64_OBJECT_KIND_TIMER"));
        assert!(libc_time_min.contains("__lnp_object_ctl"));
        assert!(
            libc_time_min.contains("long status = (long)__lnp_object_ctl((lnp64_word_t)record);")
        );
        assert!(libc_time_min.contains("errno = (int)-status;"));
        assert!(libc_time_min.contains("__lnp_push"));
        assert!(libc_time_min.contains("__lnp_yield"));
        assert!(time_header.contains("struct itimerspec"));
        assert!(time_header.contains("#include <stddef.h>"));
        assert!(time_header.contains("size_t strftime(char *s, size_t max"));
        assert!(sys_timerfd_header.contains("int timerfd_create(int clockid, int flags);"));
        assert!(sys_timerfd_header.contains("int timerfd_settime("));
        assert!(sys_timerfd_header.contains("int timerfd_gettime("));
        assert!(unistd_header.contains("unsigned int alarm(unsigned int seconds);"));
        assert!(unistd_header.contains("int usleep(unsigned int usec);"));
        assert!(real_llc.contains("liblnp64-time-min.o"));
        assert!(real_llc.contains("grep -q 'get_pcr r'"));
        assert!(real_llc.contains("grep -q 'yield' \"$libc_time_impl_dump\""));
        assert!(real_llc.contains("grep -q 'object_ctl r' \"$libc_time_impl_dump\""));
        assert!(real_llc.contains("grep -q 'push r' \"$libc_time_impl_dump\""));
        assert!(
            real_llc
                .contains("real LLVM LNP64 clang minilibc time implementation object smoke passed")
        );
        assert!(libc_string_min.contains("int isascii(int ch)"));
        assert!(libc_string_min.contains("int isblank(int ch)"));
        assert!(libc_string_min.contains("int iscntrl(int ch)"));
        assert!(libc_string_min.contains("int isprint(int ch)"));
        assert!(libc_string_min.contains("int isgraph(int ch)"));
        assert!(libc_string_min.contains("int ispunct(int ch)"));
        assert!(real_llc.contains("libc-test-print-clang-smoke.o"));
        assert!(real_llc.contains("libc-test-argv-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/argv.c"));
        assert!(real_llc.contains("libc-test-env-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/env.c"));
        assert!(real_llc.contains("libc-test-random-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/random.c"));
        assert!(real_llc.contains("libc-test-ctype-bounded-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/ctype_bounded.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test harness object smoke passed"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test argv object smoke passed"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test env object smoke passed"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test random object smoke passed"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang libc-test ctype_bounded object smoke passed")
        );
        assert!(real_llc.contains("libc-test-string-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/string.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test string object smoke passed"));
        assert!(real_llc.contains("libc-test-string-memcpy-bounded-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/string_memcpy_bounded.c"));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang libc-test string_memcpy_bounded object smoke passed"
            )
        );
        assert!(real_llc.contains("libc-test-string-memmove-bounded-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/string_memmove_bounded.c"));
        assert!(real_llc.contains(
            "real LLVM LNP64 clang libc-test string_memmove_bounded object smoke passed"
        ));
        assert!(real_llc.contains("libc-test-string-memmem-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/string_memmem.c"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang libc-test string_memmem object smoke passed")
        );
        assert!(real_llc.contains("libc-test-string-strchr-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/string_strchr.c"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang libc-test string_strchr object smoke passed")
        );
        assert!(real_llc.contains("libc-test-string-strcspn-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/string_strcspn.c"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang libc-test string_strcspn object smoke passed")
        );
        assert!(real_llc.contains("libc-test-string-strstr-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/string_strstr.c"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang libc-test string_strstr object smoke passed")
        );
        assert!(real_llc.contains("libc-test-udiv-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/udiv.c"));
        assert!(real_llc.contains("grep -q 'udiv r'"));
        assert!(real_llc.contains("grep -q 'urem r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test udiv object smoke passed"));
        assert!(real_llc.contains("libc-test-basename-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/basename.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test basename object smoke passed"));
        assert!(real_llc.contains("libc-test-dirname-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/dirname.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test dirname object smoke passed"));
        assert!(real_llc.contains("libc-test-strtol-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/strtol.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test strtol object smoke passed"));
        assert!(real_llc.contains("libc-test-clock-gettime-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/clock_gettime.c"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang libc-test clock_gettime object smoke passed")
        );
        assert!(real_llc.contains("libc-test-access-bounded-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/access_bounded.c"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang libc-test access_bounded object smoke passed")
        );
        assert!(real_llc.contains("libc-test-stat-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/stat.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test stat object smoke passed"));
        assert!(real_llc.contains("libc-test-utime-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/utime.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test utime object smoke passed"));
        assert!(real_llc.contains("libc-test-ungetc-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/ungetc.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test ungetc object smoke passed"));
        assert!(real_llc.contains("libc-test-fdopen-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/fdopen.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test fdopen object smoke passed"));
        assert!(real_llc.contains("libc-test-fcntl-basic-bounded-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/fcntl_basic_bounded.c"));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang libc-test fcntl_basic_bounded object smoke passed"
            )
        );
        assert!(real_llc.contains("libc-test-pthread-tsd-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/pthread_tsd.c"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang libc-test pthread_tsd object smoke passed")
        );
        assert!(real_llc.contains("libc-test-sem-init-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/sem_init.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test sem_init object smoke passed"));
        assert!(real_llc.contains("libc-test-qsort-bounded-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/qsort_bounded.c"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang libc-test qsort_bounded object smoke passed")
        );
        assert!(real_llc.contains("libc-test-search-insque-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/search_insque.c"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang libc-test search_insque object smoke passed")
        );
        assert!(real_llc.contains("libc-test-search-lsearch-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/functional/search_lsearch.c"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang libc-test search_lsearch object smoke passed")
        );
        assert!(real_llc.contains("libc-test-malloc-0-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/regression/malloc-0.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test malloc-0 object smoke passed"));
        assert!(real_llc.contains("libc-test-fgets-eof-clang-smoke.o"));
        assert!(real_llc.contains("third_party/libc-test/regression/fgets-eof.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang libc-test fgets-eof object smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-ctype-bounded-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test ctype_bounded link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-string-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test string link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-string-memcpy-bounded-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_memcpy_bounded_obj" \"#));
        assert!(
            real_llc
                .contains("real LLVM LNP64 lld libc-test string_memcpy_bounded link smoke passed")
        );
        assert!(real_llc.contains("lnp64-libc-test-string-memmove-bounded-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_memmove_bounded_obj" \"#));
        assert!(
            real_llc
                .contains("real LLVM LNP64 lld libc-test string_memmove_bounded link smoke passed")
        );
        assert!(real_llc.contains("lnp64-libc-test-string-memmem-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test string_memmem link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-string-strchr-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test string_strchr link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-string-strcspn-linked.elf"));
        assert!(
            real_llc.contains("real LLVM LNP64 lld libc-test string_strcspn link smoke passed")
        );
        assert!(real_llc.contains("lnp64-libc-test-string-strstr-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test string_strstr link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-udiv-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test udiv link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-basename-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test basename link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-dirname-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test dirname link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-strtol-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test strtol link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-clock-gettime-linked.elf"));
        assert!(real_llc.contains(
            r#""$libc_test_clock_gettime_obj" \
  "$libc_test_print_obj" "$libc_time_impl_obj" "$libc_errno_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test clock_gettime link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-access-bounded-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_access_bounded_obj""#));
        assert!(real_llc.contains(r#""$libc_meta_impl_obj" "$libc_fd_impl_obj""#));
        assert!(
            real_llc.contains("real LLVM LNP64 lld libc-test access_bounded link smoke passed")
        );
        assert!(real_llc.contains("lnp64-libc-test-stat-linked.elf"));
        assert!(real_llc.contains(
            r#""$libc_test_stat_obj" \
  "$libc_test_print_obj" "$libc_stdio_impl_obj" "$libc_meta_impl_obj" \
  "$libc_fd_impl_obj" "$libc_errno_impl_obj" "$libc_time_impl_obj" \
  "$libc_process_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test stat link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-utime-linked.elf"));
        assert!(real_llc.contains(
            r#""$libc_test_utime_obj" \
  "$libc_test_print_obj" "$libc_stdio_impl_obj" "$libc_meta_impl_obj" \
  "$libc_fd_impl_obj" "$libc_errno_impl_obj" "$libc_time_impl_obj" \
  "$libc_process_impl_obj" "$libc_string_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test utime link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-ungetc-linked.elf"));
        assert!(real_llc.contains(
            r#""$libc_test_ungetc_obj" \
  "$libc_test_print_obj" "$libc_stdio_impl_obj" "$libc_string_impl_obj" \
  "$libc_fd_impl_obj" "$libc_errno_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test ungetc link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-fdopen-linked.elf"));
        assert!(real_llc.contains(
            r#""$libc_test_fdopen_obj" \
  "$libc_test_print_obj" "$libc_stdio_impl_obj" "$libc_string_impl_obj" \
  "$libc_fd_impl_obj" "$libc_errno_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test fdopen link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-fcntl-basic-bounded-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_fcntl_basic_obj""#));
        assert!(real_llc.contains(r#""$libc_stdio_impl_obj" "$libc_meta_impl_obj""#));
        assert!(
            real_llc
                .contains("real LLVM LNP64 lld libc-test fcntl_basic_bounded link smoke passed")
        );
        assert!(real_llc.contains("lnp64-libc-test-pthread-tsd-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_pthread_tsd_obj""#));
        assert!(real_llc.contains(r#""$libc_pthread_impl_obj" "$libc_alloc_impl_obj""#));
        assert!(real_llc.contains(r#""$libc_alloc_impl_obj" "$libc_string_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test pthread_tsd link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-sem-init-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_sem_init_obj""#));
        assert!(real_llc.contains(r#""$libc_pthread_impl_obj" "$libc_sem_impl_obj""#));
        assert!(real_llc.contains(r#""$libc_sem_impl_obj" "$libc_futex_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test sem_init link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-qsort-bounded-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_qsort_bounded_obj" \"#));
        assert!(real_llc.contains(r#""$libc_sort_impl_obj" "$libc_string_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test qsort_bounded link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-search-insque-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_search_insque_obj" \"#));
        assert!(real_llc.contains(r#""$libc_search_impl_obj" "$libc_alloc_impl_obj""#));
        assert!(real_llc.contains(r#""$libc_string_impl_obj" "$libc_fd_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test search_insque link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-search-lsearch-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_search_lsearch_obj""#));
        assert!(
            real_llc.contains("real LLVM LNP64 lld libc-test search_lsearch link smoke passed")
        );
        assert!(search_header.contains("void insque(void *elem, void *pred);"));
        assert!(search_header.contains("void remque(void *elem);"));
        assert!(real_llc.contains("lnp64-libc-test-malloc-0-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_malloc_0_obj" \"#));
        assert!(real_llc.contains(r#""$libc_alloc_impl_obj" "$libc_string_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test malloc-0 link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-fgets-eof-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_fgets_eof_obj" \"#));
        assert!(real_llc.contains(r#""$libc_stdio_impl_obj" "$libc_string_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test fgets-eof link smoke passed"));
        assert!(real_llc.contains("toolchain/liblnp64_futex_min.c"));
        assert!(lnp64_futex_header.contains("int futex_wait("));
        assert!(lnp64_futex_header.contains("int futex_wake("));
        assert!(libc_futex_min.contains("#include <lnp64/futex.h>"));
        assert!(libc_futex_min.contains("__lnp_futex_wait"));
        assert!(libc_futex_min.contains("__lnp_futex_wake"));
        assert!(real_llc.contains("liblnp64-futex-min.o"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$libc_futex_impl_c\""));
        assert!(real_llc.contains("grep -q 'futex_wait r'"));
        assert!(real_llc.contains("grep -q 'futex_wake r'"));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang minilibc futex implementation object smoke passed"
            )
        );
        assert!(real_llc.contains("futex-libc-clang-smoke.o"));
        assert!(real_llc.contains("#include <lnp64/futex.h>"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$futex_libc_c\""));
        assert!(real_llc.contains("real LLVM LNP64 clang futex libc object smoke passed"));
        assert!(pthread_header.contains("int pthread_create("));
        assert!(pthread_header.contains("int pthread_key_create("));
        assert!(pthread_header.contains("void *pthread_getspecific("));
        assert!(libc_pthread_min.contains("__lnp_spawn_entry"));
        assert!(libc_pthread_min.contains("__lnp_thread_join"));
        assert!(libc_pthread_min.contains("lnp64_run_tsd_destructors"));
        assert!(real_llc.contains("toolchain/liblnp64_pthread_min.c"));
        assert!(real_llc.contains("liblnp64-pthread-min.o"));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang minilibc pthread implementation object smoke passed"
            )
        );
        assert!(semaphore_header.contains("typedef struct"));
        assert!(semaphore_header.contains("int sem_init("));
        assert!(semaphore_header.contains("int sem_timedwait("));
        assert!(libc_sem_min.contains("__lnp_futex_wait"));
        assert!(libc_sem_min.contains("__lnp_futex_wake"));
        assert!(libc_sem_min.contains("__atomic_compare_exchange_n"));
        assert!(real_llc.contains("toolchain/liblnp64_sem_min.c"));
        assert!(real_llc.contains("liblnp64-sem-min.o"));
        assert!(real_llc.contains(
            "real LLVM LNP64 clang minilibc semaphore implementation object smoke passed"
        ));
        assert!(real_llc.contains("toolchain/liblnp64_poll_min.c"));
        assert!(libc_poll_min.contains("#include <poll.h>"));
        assert!(libc_poll_min.contains("#include <sys/epoll.h>"));
        assert!(libc_poll_min.contains("#include <sys/event.h>"));
        assert!(libc_poll_min.contains("#include <sys/select.h>"));
        assert!(!libc_poll_min.contains("typedef unsigned long nfds_t;"));
        assert!(libc_poll_min.contains("int poll(struct pollfd *fds"));
        assert!(libc_poll_min.contains("int select(int nfds"));
        assert!(libc_poll_min.contains("int epoll_create1(int flags)"));
        assert!(libc_poll_min.contains("int epoll_ctl(int epfd, int op, int fd"));
        assert!(libc_poll_min.contains("int epoll_wait(int epfd"));
        assert!(libc_poll_min.contains("int kqueue(void)"));
        assert!(libc_poll_min.contains("int kevent(int kq"));
        assert!(libc_poll_min.contains("__lnp_await"));
        assert!(poll_header.contains("struct pollfd"));
        assert!(poll_header.contains("#define POLLIN"));
        assert!(poll_header.contains("int poll(struct pollfd *fds"));
        assert!(sys_select_header.contains("typedef struct"));
        assert!(sys_select_header.contains("int select(int nfds"));
        assert!(sys_epoll_header.contains("struct epoll_event"));
        assert!(sys_epoll_header.contains("#define EPOLL_CTL_ADD"));
        assert!(sys_epoll_header.contains("int epoll_ctl(int epfd"));
        assert!(sys_event_header.contains("struct kevent"));
        assert!(sys_event_header.contains("#define EVFILT_READ"));
        assert!(sys_event_header.contains("int kevent(int kq"));
        assert!(real_llc.contains("#include <poll.h>"));
        assert!(real_llc.contains("#include <sys/epoll.h>"));
        assert!(real_llc.contains("#include <sys/event.h>"));
        assert!(real_llc.contains("#include <sys/select.h>"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$poll_libc_c\""));
        assert!(real_llc.contains("select(1, &readfds, &writefds, &exceptfds, &timeout)"));
        assert!(real_llc.contains("epoll_create1(0)"));
        assert!(real_llc.contains("epoll_ctl(ep, EPOLL_CTL_ADD, 0, &ev)"));
        assert!(real_llc.contains("epoll_wait(ep, &out, 1, 0)"));
        assert!(real_llc.contains("kqueue()"));
        assert!(real_llc.contains("change.filter = EVFILT_READ"));
        assert!(real_llc.contains("change.flags = EV_ADD"));
        assert!(real_llc.contains("kevent(kq, &change, 1, 0, 0, &ts)"));
        assert!(real_llc.contains("poll-libc-clang-smoke.o"));
        assert!(real_llc.contains("liblnp64-poll-min.o"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$libc_poll_impl_c\""));
        assert!(real_llc.contains("grep -q 'await r'"));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang poll/select/epoll/kqueue libc object smoke passed"
            )
        );
        assert!(real_llc.contains(
            "real LLVM LNP64 clang minilibc poll/select/epoll/kqueue implementation object smoke passed"
        ));
        assert!(real_llc.contains("toolchain/liblnp64_signal_min.c"));
        assert!(libc_signal_min.contains("#include <signal.h>"));
        assert!(libc_signal_min.contains("#include <unistd.h>"));
        assert!(!real_llc.contains("#include \"lnp64_intrinsics.h\""));
        assert!(!libc_signal_min.contains("typedef unsigned long sigset_t;"));
        assert!(!libc_signal_min.contains("struct sigaction {"));
        assert!(libc_signal_min.contains("sighandler_t signal"));
        assert!(libc_signal_min.contains("int sigaction(int signum"));
        assert!(libc_signal_min.contains("int sigprocmask(int how"));
        assert!(libc_signal_min.contains("int kill(int pid"));
        assert!(libc_signal_min.contains("lnp64_word_t status = __lnp_kill"));
        assert!(libc_signal_min.contains("int raise(int signum"));
        assert!(libc_signal_min.contains("kill((int)__lnp_get_pid(), signum)"));
        assert!(libc_signal_min.contains("unsigned int alarm(unsigned int seconds)"));
        assert!(signal_header.contains("struct sigaction"));
        assert!(signal_header.contains("#define SIG_SETMASK"));
        assert!(signal_header.contains("int sigaction(int signum"));
        assert!(signal_header.contains("int sigprocmask(int how"));
        assert!(signal_header.contains("int kill(int pid"));
        assert!(real_llc.contains("signal-libc-clang-smoke.o"));
        assert!(real_llc.contains("#include <lnp64/intrinsics.h>"));
        assert!(real_llc.contains("#include <signal.h>"));
        assert!(real_llc.contains("#include <unistd.h>"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$signal_libc_c\""));
        assert!(real_llc.contains("signal(10, SIG_IGN)"));
        assert!(real_llc.contains("sigaction(12, &act, 0)"));
        assert!(real_llc.contains("sigprocmask(SIG_SETMASK, &mask, 0)"));
        assert!(real_llc.contains("kill((int)__lnp_get_pid(), 10)"));
        assert!(real_llc.contains("raise(12)"));
        assert!(real_llc.contains("real LLVM LNP64 clang signal libc object smoke passed"));
        assert!(real_llc.contains("liblnp64-signal-min.o"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$libc_signal_impl_c\""));
        assert!(real_llc.contains("grep -q 'sigaction r'"));
        assert!(real_llc.contains("grep -q 'sigmask_set r'"));
        assert!(real_llc.contains("grep -q 'kill r'"));
        assert!(real_llc.contains("grep -q 'alarm r'"));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang minilibc signal implementation object smoke passed"
            )
        );
        assert!(real_llc.contains("toolchain/liblnp64_socket_min.c"));
        assert!(libc_socket_min.contains("#include <sys/socket.h>"));
        assert!(!libc_socket_min.contains("typedef unsigned long socklen_t;"));
        assert!(libc_socket_min.contains("int socket(int domain"));
        assert!(libc_socket_min.contains("int bind(int fd"));
        assert!(libc_socket_min.contains("int listen(int fd"));
        assert!(libc_socket_min.contains("int connect(int fd"));
        assert!(libc_socket_min.contains("int accept(int fd"));
        assert!(libc_socket_min.contains("int getsockname(int fd"));
        assert!(libc_socket_min.contains("int getsockopt(int fd"));
        assert!(libc_socket_min.contains("int setsockopt(int fd"));
        assert!(libc_socket_min.contains("long send(int fd"));
        assert!(libc_socket_min.contains("long recv(int fd"));
        assert!(libc_socket_min.contains("lnp64_complete_status"));
        assert!(libc_socket_min.contains("lnp64_errno_store(lnp64_errno_load())"));
        assert!(libc_socket_min.contains("__lnp_object_ctl"));
        assert!(libc_socket_min.contains("__lnp_push"));
        assert!(libc_socket_min.contains("__lnp_pull"));
        assert!(sys_socket_header.contains("#define AF_INET"));
        assert!(sys_socket_header.contains("#define MSG_NOSIGNAL"));
        assert!(sys_socket_header.contains("int socket(int domain, int type, int protocol);"));
        assert!(netinet_in_header.contains("#define IPPROTO_TCP"));
        assert!(real_llc.contains("socket-libc-clang-smoke.o"));
        assert!(real_llc.contains("#include <sys/socket.h>"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$libc_socket_impl_c\""));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$socket_libc_c\""));
        assert!(real_llc.contains("socket(AF_INET, SOCK_STREAM, 0)"));
        assert!(real_llc.contains("setsockopt(server, SOL_SOCKET, SO_REUSEADDR"));
        assert!(real_llc.contains("getsockopt(server, SOL_SOCKET, SO_ERROR"));
        assert!(real_llc.contains("bind(server, \"127.0.0.1:0\", 0)"));
        assert!(real_llc.contains("connect(client, addr, addrlen)"));
        assert!(real_llc.contains("accept(server, 0, 0)"));
        assert!(real_llc.contains("send(client, \"z\", 1, MSG_NOSIGNAL)"));
        assert!(real_llc.contains("recv(accepted, buf, 1, 0)"));
        assert!(real_llc.contains("real LLVM LNP64 clang socket libc object smoke passed"));
        assert!(real_llc.contains("userland/netbsd_personality_clang_smoke.c"));
        assert!(real_llc.contains("netbsd-personality-clang-smoke.o"));
        assert!(netbsd_personality_clang.contains("#include <lnp64/intrinsics.h>"));
        assert!(netbsd_personality_clang.contains("#include <poll.h>"));
        assert!(netbsd_personality_clang.contains("#include <sys/mman.h>"));
        assert!(netbsd_personality_clang.contains("#include <sys/socket.h>"));
        assert!(netbsd_personality_clang.contains("MAP_FAILED"));
        assert!(netbsd_personality_clang.contains("PROT_READ | PROT_WRITE"));
        assert!(!netbsd_personality_clang.contains("void *mmap(void *addr"));
        assert!(!netbsd_personality_clang.contains("int socket(int domain"));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD personality smoke object passed"));
        assert!(real_llc.contains("liblnp64-socket-min.o"));
        assert!(real_llc.contains("grep -q 'object_ctl r'"));
        assert!(real_llc.contains("grep -q 'push r'"));
        assert!(real_llc.contains("grep -q 'pull r'"));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang minilibc socket implementation object smoke passed"
            )
        );
        assert!(real_llc.contains("errno-clang-smoke.o"));
        assert!(real_llc.contains("lnp64_errno_store(22)"));
        assert!(real_llc.contains("real LLVM LNP64 clang errno object smoke passed"));
        assert!(real_llc.contains("startup-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang startup argv/envp object smoke passed"));
        assert!(real_llc.contains("toolchain/liblnp64_stdio_min.c"));
        assert!(libc_stdio_min.contains("#include <stdio.h>"));
        assert!(!libc_stdio_min.contains("typedef struct __lnp64_file FILE;"));
        assert!(libc_stdio_min.contains("int vsnprintf("));
        assert!(libc_stdio_min.contains("int snprintf("));
        assert!(libc_stdio_min.contains("FILE *fmemopen("));
        assert!(libc_stdio_min.contains("int vsnprintf(char *str, size_t size"));
        assert!(libc_stdio_min.contains("int snprintf(char *str, size_t size"));
        assert!(libc_stdio_min.contains("FILE *fmemopen(void *buf, size_t size"));
        assert!(libc_stdio_min.contains("size_t fread(void *ptr, size_t size, size_t count"));
        assert!(
            libc_stdio_min.contains("size_t fwrite(const void *ptr, size_t size, size_t count")
        );
        assert!(libc_stdio_min.contains("char *fgets("));
        assert!(libc_stdio_min.contains("int fseek(FILE *stream"));
        assert!(libc_stdio_min.contains("long ftell(FILE *stream)"));
        assert!(libc_stdio_min.contains("int fseeko(FILE *stream, off_t offset, int whence)"));
        assert!(libc_stdio_min.contains("off_t ftello(FILE *stream)"));
        assert!(libc_stdio_min.contains("int fscanf(FILE *stream"));
        assert!(libc_stdio_min.contains("FILE *tmpfile(void)"));
        assert!(libc_stdio_min.contains("int fileno(FILE *stream)"));
        assert!(stdio_header.contains("#include <stdarg.h>"));
        assert!(stdio_header.contains("#include <stddef.h>"));
        assert!(stdio_header.contains("#include <sys/types.h>"));
        assert!(
            stdio_header.contains("int vfprintf(FILE *stream, const char *format, va_list ap);")
        );
        assert!(stdio_header.contains("int fseeko(FILE *stream, off_t offset, int whence);"));
        assert!(stdio_header.contains("off_t ftello(FILE *stream);"));
        assert!(
            stdio_header
                .contains("int vsnprintf(char *str, size_t size, const char *format, va_list ap);")
        );
        assert!(
            stdio_header.contains("int snprintf(char *str, size_t size, const char *format, ...);")
        );
        assert!(stdio_header.contains("ssize_t getline(char **lineptr, size_t *n, FILE *stream);"));
        assert!(stdio_header.contains("size_t fread(void *ptr, size_t size, size_t count"));
        assert!(stdio_header.contains("size_t fwrite(const void *ptr, size_t size, size_t count"));
        assert!(stdio_header.contains("FILE *fmemopen(void *buf, size_t size"));
        assert!(!stdio_header.contains("__builtin_va_list"));
        assert!(real_llc.contains("liblnp64-stdio-min.o"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$libc_stdio_impl_c\""));
        assert!(real_llc.contains("grep -q '<vsnprintf>:'"));
        assert!(real_llc.contains("grep -q '<snprintf>:'"));
        assert!(real_llc.contains("grep -q '<tmpfile>:'"));
        assert!(real_llc.contains("grep -q '<fileno>:'"));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang minilibc stdio implementation object smoke passed"
            )
        );
        assert!(real_llc.contains("getauxval-clang-smoke.o"));
        assert!(real_llc.contains("#include <sys/auxv.h>"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$getauxval_c\""));
        assert!(real_llc.contains("real LLVM LNP64 clang getauxval object smoke passed"));
        assert!(real_llc.contains("libc-string-clang-smoke.o"));
        assert!(real_llc.contains("#include <ctype.h>"));
        assert!(real_llc.contains("#include <string.h>"));
        assert!(real_llc.contains("strcmp(\"abc\", \"abc\")"));
        assert!(real_llc.contains("strncmp(\"abcdef\", \"abcxyz\", 3)"));
        assert!(real_llc.contains("strcpy(dst, \"xy\")"));
        assert!(real_llc.contains("strncpy(bounded, \"abc\", 6)"));
        assert!(real_llc.contains("strncat(bounded, \"zpq\", 1)"));
        assert!(real_llc.contains("strchr(\"abcd\", 'c')"));
        assert!(real_llc.contains("strrchr(scan, 'a')"));
        assert!(real_llc.contains("strstr(\"abcde\", \"bcd\")"));
        assert!(real_llc.contains("strspn(\"abc123\", \"abc\")"));
        assert!(real_llc.contains("strcspn(\"abc123\", \"321\")"));
        assert!(real_llc.contains("strpbrk(\"abc123\", \"29\")"));
        assert!(real_llc.contains("strtok(tokens, \",\")"));
        assert!(real_llc.contains("strlcpy(small, \"abcdef\", sizeof(small))"));
        assert!(real_llc.contains("strlcat(small, \"cdef\", sizeof(small))"));
        assert!(real_llc.contains("memmem(hay, 7, needle, 2)"));
        assert!(real_llc.contains("tolower('Q')"));
        assert!(real_llc.contains("toupper('q')"));
        assert!(real_llc.contains("grep -q 'sext.w'"));
        assert!(stdarg_header.contains("typedef __builtin_va_list va_list;"));
        assert!(stdarg_header.contains("#define va_start(ap, last) __builtin_va_start(ap, last)"));
        assert!(stdarg_header.contains("#define va_arg(ap, type) __builtin_va_arg(ap, type)"));
        assert!(stddef_header.contains("typedef unsigned long size_t;"));
        assert!(stddef_header.contains("typedef long ptrdiff_t;"));
        assert!(stddef_header.contains("#define NULL ((void *)0)"));
        assert!(string_header.contains("#include <stddef.h>"));
        assert!(libc_string_min.contains("#include <string.h>"));
        assert!(!libc_string_min.contains("typedef unsigned long size_t;"));
        assert!(libc_string_min.contains("void *memmove"));
        assert!(libc_string_min.contains("int strcmp"));
        assert!(libc_string_min.contains("int strncmp"));
        assert!(libc_string_min.contains("char *strcpy"));
        assert!(libc_string_min.contains("char *strncpy"));
        assert!(libc_string_min.contains("char *strncat"));
        assert!(libc_string_min.contains("char *strchr"));
        assert!(libc_string_min.contains("char *strrchr"));
        assert!(libc_string_min.contains("char *strstr"));
        assert!(libc_string_min.contains("size_t strspn"));
        assert!(libc_string_min.contains("size_t strcspn"));
        assert!(libc_string_min.contains("char *strpbrk"));
        assert!(libc_string_min.contains("char *strtok"));
        assert!(libc_string_min.contains("size_t strlcpy"));
        assert!(libc_string_min.contains("size_t strlcat"));
        assert!(libc_string_min.contains("void *memmem"));
        assert!(libc_string_min.contains("int isalpha"));
        assert!(libc_string_min.contains("int isdigit"));
        assert!(libc_string_min.contains("int isspace"));
        assert!(libc_string_min.contains("int isxdigit"));
        assert!(libc_string_min.contains("int tolower"));
        assert!(libc_string_min.contains("int toupper"));
        assert!(real_llc.contains("real LLVM LNP64 clang minilibc string object smoke passed"));
        assert!(real_llc.contains("toolchain/liblnp64_string_min.c"));
        assert!(real_llc.contains("liblnp64-string-min.o"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$libc_string_impl_c\""));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang minilibc string implementation object smoke passed"
            )
        );
        assert!(real_llc.contains("convert-clang-smoke.o"));
        assert!(stdlib_header.contains("char *getenv(const char *name);"));
        assert!(
            stdlib_header
                .contains("int setenv(const char *name, const char *value, int overwrite);")
        );
        assert!(stdlib_header.contains("int unsetenv(const char *name);"));
        assert!(stdlib_header.contains("int clearenv(void);"));
        assert!(stdlib_header.contains("int putenv(char *string);"));
        assert!(stdlib_header.contains("long random(void);"));
        assert!(stdlib_header.contains("void srandom(unsigned int seed);"));
        assert!(
            stdlib_header.contains("char *initstate(unsigned int seed, char *state, size_t size);")
        );
        assert!(stdlib_header.contains("char *setstate(char *state);"));
        assert!(stdlib_header.contains("int atoi(const char *nptr);"));
        assert!(stdlib_header.contains("long atol(const char *nptr);"));
        assert!(stdlib_header.contains("long strtol(const char *nptr, char **endptr, int base);"));
        assert!(
            stdlib_header
                .contains("unsigned long strtoul(const char *nptr, char **endptr, int base);")
        );
        assert!(
            stdlib_header.contains("long long strtoll(const char *nptr, char **endptr, int base);")
        );
        assert!(
            stdlib_header.contains(
                "unsigned long long strtoull(const char *nptr, char **endptr, int base);"
            )
        );
        assert!(real_llc.contains("#include <errno.h>"));
        assert!(real_llc.contains("#include <stdlib.h>"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$convert_c\""));
        assert!(real_llc.contains("errno = 0;"));
        assert!(real_llc.contains("strtol(s, &end, 8)"));
        assert!(real_llc.contains("strtol(s, &end, 37)"));
        assert!(real_llc.contains("strtoull(s, &end, 0)"));
        assert!(real_llc.contains("real LLVM LNP64 clang numeric conversion object smoke passed"));
        assert!(real_llc.contains("toolchain/liblnp64_convert_min.c"));
        assert!(real_llc.contains("liblnp64-convert-min.o"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$libc_convert_impl_c\""));
        assert!(libc_convert_min.contains("#include <errno.h>"));
        assert!(libc_convert_min.contains("#include <stdlib.h>"));
        assert!(libc_convert_min.contains("strtoull"));
        assert!(libc_convert_min.contains("strtoll"));
        assert!(libc_convert_min.contains("lnp64_errno_store(EINVAL)"));
        assert!(libc_convert_min.contains("lnp64_errno_store(ERANGE)"));
        assert!(real_llc.contains(
            "real LLVM LNP64 clang minilibc numeric conversion implementation object smoke passed"
        ));
        assert!(real_llc.contains("path-clang-smoke.o"));
        assert!(real_llc.contains("#include <libgen.h>"));
        assert!(real_llc.contains("#include <string.h>"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$path_c\""));
        assert!(real_llc.contains("check_basename(\"/usr/lib\", \"lib\")"));
        assert!(real_llc.contains("check_dirname(\"/usr/lib\", \"/usr\")"));
        assert!(real_llc.contains("real LLVM LNP64 clang path helper object smoke passed"));
        assert!(real_llc.contains("toolchain/liblnp64_path_min.c"));
        assert!(real_llc.contains("liblnp64-path-min.o"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$libc_path_impl_c\""));
        assert!(libc_path_min.contains("#include <libgen.h>"));
        assert!(libc_path_min.contains("#include <string.h>"));
        assert!(libc_path_min.contains("char *basename"));
        assert!(libc_path_min.contains("char *dirname"));
        assert!(libc_path_min.contains("end = strlen(path);"));
        assert!(libc_path_min.contains("lnp64_dot"));
        assert!(
            real_llc
                .contains("real LLVM LNP64 clang minilibc path implementation object smoke passed")
        );
        assert!(real_llc.contains("search-clang-smoke.o"));
        assert!(real_llc.contains("#include <search.h>"));
        assert!(real_llc.contains("#include <string.h>"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$search_c\""));
        assert!(real_llc.contains("get(key_a)"));
        assert!(real_llc.contains("remque(p->p)"));
        assert!(real_llc.contains("real LLVM LNP64 clang search helper object smoke passed"));
        assert!(real_llc.contains("toolchain/liblnp64_search_min.c"));
        assert!(real_llc.contains("liblnp64-search-min.o"));
        assert!(libc_search_min.contains("#include <search.h>"));
        assert!(libc_search_min.contains("#include <string.h>"));
        assert!(!libc_search_min.contains("typedef unsigned long size_t;"));
        assert!(libc_search_min.contains("void *lfind"));
        assert!(libc_search_min.contains("void *lsearch"));
        assert!(libc_search_min.contains("void insque"));
        assert!(libc_search_min.contains("void remque"));
        assert!(libc_search_min.contains("lnp64_search_copy_key(found, key, width)"));
        assert!(
            real_llc.contains(
                "real LLVM LNP64 clang minilibc search implementation object smoke passed"
            )
        );
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$libc_search_impl_c\""));
        assert!(real_llc.contains("sort-clang-smoke.o"));
        assert!(stdint_header.contains("typedef unsigned long uint64_t;"));
        assert!(real_llc.contains("#include <stdint.h>"));
        assert!(real_llc.contains("#include <stdlib.h>"));
        assert!(real_llc.contains("#include <string.h>"));
        assert!(!real_llc.contains("typedef unsigned long uint64_t;"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$sort_c\""));
        assert!(real_llc.contains("qsort(names, 6"));
        assert!(real_llc.contains("qsort(nums, 8"));
        assert!(real_llc.contains("qsort(chars, sizeof chars - 1"));
        assert!(real_llc.contains("qsort(wide, 6"));
        assert!(real_llc.contains("real LLVM LNP64 clang sort helper object smoke passed"));
        assert!(real_llc.contains("toolchain/liblnp64_sort_min.c"));
        assert!(real_llc.contains("liblnp64-sort-min.o"));
        assert!(libc_sort_min.contains("#include <stdlib.h>"));
        assert!(!libc_sort_min.contains("typedef unsigned long size_t;"));
        assert!(libc_sort_min.contains("void qsort"));
        assert!(libc_sort_min.contains("lnp64_swap_bytes"));
        assert!(libc_sort_min.contains("compar(prev, cur)"));
        assert!(
            real_llc
                .contains("real LLVM LNP64 clang minilibc sort implementation object smoke passed")
        );
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$libc_sort_impl_c\""));
        assert!(real_llc.contains("toolchain/liblnp64_alloc_min.c"));
        assert!(libc_alloc_min.contains("#include <stdlib.h>"));
        assert!(libc_alloc_min.contains("#include <string.h>"));
        assert!(!libc_alloc_min.contains("typedef unsigned long size_t;"));
        assert!(libc_alloc_min.contains("void *alloc(size_t size)"));
        assert!(libc_alloc_min.contains("__lnp_alloc(size)"));
        assert!(libc_alloc_min.contains("__lnp_alloc_size(ptr)"));
        assert!(stdlib_header.contains("void *calloc(size_t count, size_t size);"));
        assert!(stdlib_header.contains("void *realloc(void *ptr, size_t size);"));
        assert!(stdlib_header.contains("void free(void *ptr);"));
        assert!(real_llc.contains("liblnp64-alloc-min.o"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$libc_alloc_impl_c\""));
        assert!(real_llc.contains("grep -q 'alloc r'"));
        assert!(real_llc.contains("grep -q 'alloc_size r'"));
        assert!(real_llc.contains("grep -q 'free r'"));
        assert!(real_llc.contains(
            "real LLVM LNP64 clang minilibc allocation implementation object smoke passed"
        ));
        assert!(real_llc.contains("calloc-clang-smoke.o"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$calloc_c\""));
        assert!(real_llc.contains("real LLVM LNP64 clang calloc object smoke passed"));
        assert!(real_llc.contains("realloc-clang-smoke.o"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$realloc_c\""));
        assert!(real_llc.contains("real LLVM LNP64 clang realloc object smoke passed"));
        assert!(real_llc.contains("read-clang-smoke.o"));
        assert!(unistd_header.contains("ssize_t read(int fd, void *buf, size_t count);"));
        assert!(unistd_header.contains("ssize_t write(int fd, const void *buf, size_t count);"));
        assert!(real_llc.contains("#include <unistd.h>"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$read_c\""));
        assert!(real_llc.contains("real LLVM LNP64 clang read object smoke passed"));
        assert!(real_llc.contains("write-clang-smoke.o"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$write_c\""));
        assert!(real_llc.contains("fd write ok"));
        assert!(real_llc.contains("real LLVM LNP64 clang write object smoke passed"));
        assert!(real_llc.contains("userland/ucat_clang.c"));
        assert!(real_llc.contains("userland-ucat-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang userland ucat object smoke passed"));
        assert!(real_llc.contains("userland/init_clang.c"));
        assert!(real_llc.contains("userland-init-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang userland init object smoke passed"));
        assert!(real_llc.contains("userland/lnpsh_clang.c"));
        assert!(real_llc.contains("userland-lnpsh-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang userland lnpsh object smoke passed"));
        assert!(real_llc.contains("userland/spawn_task_clang.c"));
        assert!(lnp64_intrinsics_target_header.contains("../../lnp64_intrinsics.h"));
        assert!(spawn_task_clang.contains("#include <lnp64/intrinsics.h>"));
        assert!(real_llc.contains("userland-spawn-task-clang-smoke.o"));
        assert!(real_llc.contains("grep -q 'clone.spawn r'"));
        assert!(real_llc.contains("grep -q 'thread_join r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang userland spawn task object smoke passed"));
        assert!(real_llc.contains("userland/netbsd_init_clang.c"));
        assert!(real_llc.contains("netbsd-init-clang-smoke.o"));
        assert!(netbsd_init_clang.contains("execl(\"/bin/netbsd_sh.elf\""));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD init object passed"));
        assert!(real_llc.contains("userland/netbsd_sh_clang.c"));
        assert!(real_llc.contains("netbsd-sh-clang-smoke.o"));
        assert!(netbsd_sh_clang.contains("\"/bin/fork_wait_test.elf\""));
        assert!(netbsd_sh_clang.contains("\"/bin/domain_budget_test.elf\""));
        assert!(!netbsd_sh_clang.contains(".s\""));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD shell object passed"));
        assert!(real_llc.contains("userland/loader_target_clang.c"));
        assert!(real_llc.contains("netbsd-loader-target-clang-smoke.o"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang NetBSD loader target child object passed")
        );
        assert!(real_llc.contains("userland/elf_exec_test_clang.c"));
        assert!(real_llc.contains("netbsd-elf-exec-test-clang-smoke.o"));
        assert!(
            elf_exec_test_clang.contains("execl(\"/bin/loader_target.elf\", \"loader_target\", 0)")
        );
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD ELF exec parent object passed"));
        assert!(real_llc.contains("userland/fork_wait_test_clang.c"));
        assert!(real_llc.contains("netbsd-fork-wait-test-clang-smoke.o"));
        assert!(real_llc.contains(r#"grep -q 'call ' "$netbsd_fork_wait_test_dump""#));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD fork/wait child object passed"));
        assert!(real_llc.contains("userland/thread_test_clang.c"));
        assert!(real_llc.contains("netbsd-thread-test-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD thread child object passed"));
        assert!(real_llc.contains("userland/poll_test_clang.c"));
        assert!(real_llc.contains("netbsd-poll-test-clang-smoke.o"));
        assert!(poll_test_clang.contains("#include <lnp64/intrinsics.h>"));
        assert!(poll_test_clang.contains("#include <poll.h>"));
        assert!(poll_test_clang.contains("#include <sys/epoll.h>"));
        assert!(poll_test_clang.contains("#include <sys/select.h>"));
        assert!(!poll_test_clang.contains("typedef unsigned long nfds_t"));
        assert!(!poll_test_clang.contains("int poll(struct pollfd"));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD poll child object passed"));
        assert!(real_llc.contains("userland/signal_gate_test_clang.c"));
        assert!(signal_gate_test_clang.contains("#include <lnp64/intrinsics.h>"));
        assert!(real_llc.contains("netbsd-signal-gate-test-clang-smoke.o"));
        assert!(real_llc.contains(r#"grep -q 'yield' "$netbsd_signal_gate_test_dump""#));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD signal gate child object passed"));
        assert!(real_llc.contains("userland/signal_fault_test_clang.c"));
        assert!(signal_fault_test_clang.contains("#include <lnp64/intrinsics.h>"));
        assert!(real_llc.contains("netbsd-signal-fault-test-clang-smoke.o"));
        assert!(real_llc.contains(r#"grep -q 'div r' "$netbsd_signal_fault_test_dump""#));
        assert!(real_llc.contains(r#"grep -q 'sigret' "$netbsd_signal_fault_test_dump""#));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD signal fault child object passed"));
        assert!(real_llc.contains("userland/timer_test_clang.c"));
        assert!(real_llc.contains("netbsd-timer-test-clang-smoke.o"));
        assert!(timer_test_clang.contains("#include <lnp64/intrinsics.h>"));
        assert!(timer_test_clang.contains("#include <poll.h>"));
        assert!(!timer_test_clang.contains("typedef unsigned long nfds_t"));
        assert!(!timer_test_clang.contains("int poll(struct pollfd"));
        assert!(real_llc.contains(r#"grep -q 'yield' "$netbsd_timer_test_dump""#));
        assert!(real_llc.contains(r#"grep -q 'sigret' "$netbsd_timer_test_dump""#));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD timer child object passed"));
        assert!(real_llc.contains("userland/mmap_test_clang.c"));
        assert!(real_llc.contains("netbsd-mmap-test-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD mmap child object passed"));
        assert!(real_llc.contains("userland/socket_loopback_test_clang.c"));
        assert!(real_llc.contains("netbsd-socket-loopback-test-clang-smoke.o"));
        assert!(socket_loopback_test_clang.contains("#include <poll.h>"));
        assert!(!socket_loopback_test_clang.contains("typedef unsigned long nfds_t"));
        assert!(!socket_loopback_test_clang.contains("int poll(struct pollfd"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang NetBSD socket loopback child object passed")
        );
        assert!(real_llc.contains("userland/gate_trace_test_clang.c"));
        assert!(gate_trace_test_clang.contains("#include <lnp64/intrinsics.h>"));
        assert!(real_llc.contains("netbsd-gate-trace-test-clang-smoke.o"));
        assert!(real_llc.contains("__lnp_domain_create"));
        assert!(real_llc.contains("__lnp_call_gate_create"));
        assert!(real_llc.contains(r#"grep -q 'domain_ctl r' "$netbsd_gate_trace_test_dump""#));
        assert!(real_llc.contains(r#"grep -q 'object_ctl r' "$netbsd_gate_trace_test_dump""#));
        assert!(real_llc.contains(r#"grep -q 'gate_call r' "$netbsd_gate_trace_test_dump""#));
        assert!(real_llc.contains(r#"grep -q 'gate_return r' "$netbsd_gate_trace_test_dump""#));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD gate trace child object passed"));
        assert!(real_llc.contains("toolchain/liblnp64_fd_min.c"));
        assert!(libc_fd_min.contains("#include <fcntl.h>"));
        assert!(libc_fd_min.contains("#include <unistd.h>"));
        assert!(!libc_fd_min.contains("typedef unsigned long size_t;"));
        assert!(libc_fd_min.contains("int openat(int dirfd, const char *path, int flags, ...)"));
        assert!(libc_fd_min.contains("int open(const char *path, int flags, ...)"));
        assert!(libc_fd_min.contains("ssize_t read(int fd, void *buf, size_t len)"));
        assert!(libc_fd_min.contains("ssize_t write(int fd, const void *buf, size_t len)"));
        assert!(libc_fd_min.contains("__lnp_pull"));
        assert!(libc_fd_min.contains("__lnp_push"));
        assert!(libc_fd_min.contains("off_t lseek(int fd, off_t offset, int whence)"));
        assert!(libc_fd_min.contains("fd_seek_dyn %1, %2, %3"));
        assert!(libc_fd_min.contains("LNP64_FDR_TOKEN_MARKER"));
        assert!(libc_fd_min.contains("LNP64_FDR_TOKEN_INDEX_MASK"));
        assert!(libc_fd_min.contains("int close(int fd)"));
        assert!(libc_fd_min.contains("__lnp_cap_revoke"));
        assert!(real_llc.contains("liblnp64-fd-min.o"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$libc_fd_impl_c\""));
        assert!(real_llc.contains("grep -q 'pull r'"));
        assert!(real_llc.contains("grep -q 'fd_seek_dyn r'"));
        assert!(real_llc.contains("grep -q 'push r'"));
        assert!(real_llc.contains("grep -q 'cap_revoke r'"));
        assert!(
            real_llc
                .contains("real LLVM LNP64 clang minilibc fd implementation object smoke passed")
        );
        assert!(real_llc.contains("toolchain/liblnp64_meta_min.c"));
        assert!(libc_meta_min.contains("stat_path_at"));
        assert!(libc_meta_min.contains("stat_fd_dyn"));
        assert!(libc_meta_min.contains("utime_path_at"));
        assert!(libc_meta_min.contains("utime_fd_dyn"));
        assert!(libc_meta_min.contains("fcntl_fd_dyn"));
        assert!(libc_meta_min.contains("int access(const char *path, int mode)"));
        assert!(libc_meta_min.contains("static struct stat lnp64_access_stat"));
        assert!(
            libc_meta_min.contains("lnp64_stat_path_at(AT_FDCWD, path, &lnp64_access_stat, 0)")
        );
        assert!(libc_meta_min.contains("DIR *opendir(const char *name)"));
        assert!(libc_meta_min.contains("int mkdirat(int dirfd, const char *path, mode_t mode)"));
        assert!(libc_meta_min.contains("int renameat(int olddirfd, const char *oldpath"));
        assert!(libc_meta_min.contains("ssize_t readlinkat("));
        assert!(libc_meta_min.contains("int fchmodat("));
        assert!(libc_meta_min.contains("int fchownat("));
        assert!(libc_meta_min.contains("int faccessat("));
        assert!(libc_meta_min.contains("va_arg(ap, long)"));
        assert!(libc_meta_min.contains("lnp64_complete_status"));
        assert!(real_llc.contains("liblnp64-meta-min.o"));
        assert!(real_llc.contains("grep -q 'stat_path_at r'"));
        assert!(real_llc.contains("grep -q 'stat_fd_dyn r'"));
        assert!(real_llc.contains("grep -q 'utime_path_at r'"));
        assert!(real_llc.contains("grep -q 'utime_fd_dyn r'"));
        assert!(real_llc.contains("grep -q 'fcntl_fd_dyn r'"));
        assert!(real_llc.contains("grep -q 'open_dir_dyn r'"));
        assert!(real_llc.contains("grep -q 'mkdir_path_at r'"));
        assert!(real_llc.contains("grep -q 'rename_path_at r'"));
        assert!(real_llc.contains("grep -q 'link_path_at r'"));
        assert!(real_llc.contains("grep -q 'symlink_path_at r'"));
        assert!(real_llc.contains("grep -q 'readlink_path_at r'"));
        assert!(real_llc.contains("grep -q 'getcwd_path r'"));
        assert!(real_llc.contains("grep -q 'chmod_path_at r'"));
        assert!(real_llc.contains("grep -q 'chown_path_at r'"));
        assert!(real_llc.contains("grep -q 'errno_get r'"));
        assert!(real_llc.contains(
            "real LLVM LNP64 clang minilibc metadata implementation object smoke passed"
        ));
        assert!(real_llc.contains("meta-libc-clang-smoke.o"));
        assert!(real_llc.contains("mkdirat(AT_FDCWD"));
        assert!(real_llc.contains("renameat(AT_FDCWD"));
        assert!(real_llc.contains("readlinkat(AT_FDCWD"));
        assert!(real_llc.contains("opendir(\"target/llvm-lnp64-build\")"));
        assert!(real_llc.contains("S_ISREG(st.st_mode)"));
        assert!(real_llc.contains("st.st_nlink <= 0"));
        assert!(real_llc.contains("futimens(-1, omit)"));
        assert!(real_llc.contains("errno != EBADF"));
        assert!(real_llc.contains("real LLVM LNP64 clang metadata libc object smoke passed"));
        assert!(real_llc.contains("stack-args-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang stack-argument object smoke passed"));
        assert!(real_llc.contains("toolchain/crt0_lnp64.s"));
        assert!(real_llc.contains("real LLVM LNP64 llvm-mc crt0 smoke passed"));
        assert!(real_llc.contains("toolchain/liblnp64_min.s"));
        assert!(real_llc.contains("real LLVM LNP64 llvm-mc minilibc smoke passed"));
        assert!(real_llc.contains("liblnp64-min-smoke.dump"));
        assert!(real_llc.contains("pull r1, r1, r2, r3"));
        assert!(real_llc.contains("alloc r1, r1"));
        assert!(real_llc.contains("alloc_size r3, r2"));
        assert!(real_llc.contains("free r1"));
        assert!(
            real_llc.contains("real LLVM LNP64 llvm-objdump minilibc native decode smoke passed")
        );
        assert!(real_llc.contains("lnp64-libc-string-linked.elf"));
        assert!(real_llc.contains(r#""$libc_string_obj" "$libc_string_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld minilibc string link smoke passed"));
        assert!(real_llc.contains("lnp64-convert-linked.elf"));
        assert!(real_llc.contains(
            r#""$convert_obj" "$libc_convert_impl_obj" \
  "$libc_errno_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld numeric conversion link smoke passed"));
        assert!(real_llc.contains("lnp64-path-linked.elf"));
        assert!(real_llc.contains(
            r#""$path_obj" "$libc_path_impl_obj" \
  "$libc_string_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld path helper link smoke passed"));
        assert!(real_llc.contains("lnp64-search-linked.elf"));
        assert!(real_llc.contains(
            r#""$search_obj" "$libc_search_impl_obj" \
  "$libc_string_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld search helper link smoke passed"));
        assert!(real_llc.contains("lnp64-sort-linked.elf"));
        assert!(real_llc.contains(
            r#""$sort_obj" "$libc_sort_impl_obj" \
  "$libc_string_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld sort helper link smoke passed"));
        assert!(real_llc.contains("lnp64-calloc-linked.elf"));
        assert!(real_llc.contains(
            r#""$calloc_obj" "$libc_alloc_impl_obj" \
  "$libc_string_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld calloc link smoke passed"));
        assert!(real_llc.contains("lnp64-realloc-linked.elf"));
        assert!(real_llc.contains(
            r#""$realloc_obj" "$libc_alloc_impl_obj" \
  "$libc_string_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld realloc link smoke passed"));
        assert!(real_llc.contains("lnp64-read-linked.elf"));
        assert!(real_llc.contains(r#""$read_obj" "$libc_fd_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld read link smoke passed"));
        assert!(real_llc.contains("lnp64-write-linked.elf"));
        assert!(real_llc.contains(r#""$write_obj" "$libc_fd_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld write link smoke passed"));
        assert!(real_llc.contains("lnp64-userland-ucat-linked.elf"));
        assert!(real_llc.contains(r#""$userland_ucat_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld userland ucat link smoke passed"));
        assert!(real_llc.contains("lnp64-userland-init-linked.elf"));
        assert!(real_llc.contains(r#""$userland_init_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld userland init link smoke passed"));
        assert!(real_llc.contains("lnp64-userland-lnpsh-linked.elf"));
        assert!(real_llc.contains(r#""$userland_lnpsh_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld userland lnpsh link smoke passed"));
        assert!(real_llc.contains("lnp64-userland-spawn-task-linked.elf"));
        assert!(real_llc.contains(r#""$userland_spawn_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld userland spawn task link smoke passed"));
        assert!(real_llc.contains("lnp64-netbsd-init-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_init_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD init link passed"));
        assert!(real_llc.contains("lnp64-netbsd-sh-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_sh_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD shell link passed"));
        assert!(real_llc.contains("lnp64-netbsd-loader-target-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_loader_target_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD loader target child link passed"));
        assert!(real_llc.contains("lnp64-netbsd-elf-exec-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_elf_exec_test_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD ELF exec parent link passed"));
        assert!(real_llc.contains("lnp64-netbsd-fork-wait-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_fork_wait_test_obj" \"#));
        assert!(
            real_llc
                .contains(r#""$libc_process_impl_obj" "$libc_errno_impl_obj" "$libc_fd_impl_obj""#)
        );
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD fork/wait child link passed"));
        assert!(real_llc.contains("lnp64-netbsd-thread-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_thread_test_obj" \"#));
        assert!(real_llc.contains(r#""$libc_string_impl_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD thread child link passed"));
        assert!(real_llc.contains("lnp64-netbsd-poll-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_poll_test_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD poll child link passed"));
        assert!(real_llc.contains("lnp64-netbsd-signal-gate-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_signal_gate_test_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD signal gate child link passed"));
        assert!(real_llc.contains("lnp64-netbsd-signal-fault-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_signal_fault_test_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD signal fault child link passed"));
        assert!(real_llc.contains("lnp64-netbsd-timer-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_timer_test_obj" \"#));
        assert!(real_llc.contains(r#""$libc_time_impl_obj" "$libc_signal_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD timer child link passed"));
        assert!(real_llc.contains("lnp64-netbsd-mmap-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_mmap_test_obj" \"#));
        assert!(real_llc.contains(r#""$libc_vma_impl_obj" "$libc_errno_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD mmap child link passed"));
        assert!(real_llc.contains("userland/fd_passing_test_clang.c"));
        assert!(fd_passing_test_clang.contains("#include <lnp64/intrinsics.h>"));
        assert!(real_llc.contains("netbsd-fd-passing-test-clang-smoke.o"));
        assert!(real_llc.contains(r#"grep -q 'cap_dup r' "$netbsd_fd_passing_test_dump""#));
        assert!(real_llc.contains(r#"grep -q 'cap_send r' "$netbsd_fd_passing_test_dump""#));
        assert!(real_llc.contains(r#"grep -q 'cap_recv r' "$netbsd_fd_passing_test_dump""#));
        assert!(real_llc.contains(r#"grep -q 'cap_revoke r' "$netbsd_fd_passing_test_dump""#));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD fd passing child object passed"));
        assert!(real_llc.contains("lnp64-netbsd-fd-passing-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_fd_passing_test_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD fd passing child link passed"));
        assert!(real_llc.contains("userland/namespace_test_clang.c"));
        assert!(real_llc.contains("netbsd-namespace-test-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD namespace child object passed"));
        assert!(real_llc.contains("lnp64-netbsd-namespace-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_namespace_test_obj" \"#));
        assert!(real_llc.contains(r#""$libc_meta_impl_obj" "$libc_errno_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD namespace child link passed"));
        assert!(real_llc.contains("userland/fs_service_test_clang.c"));
        assert!(real_llc.contains("netbsd-fs-service-test-clang-smoke.o"));
        assert!(real_llc.contains(r#"grep -q 'ld.b r' "$netbsd_fs_service_test_dump""#));
        assert!(real_llc.contains(r#"grep -q 'st.b r' "$netbsd_fs_service_test_dump""#));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD fs service child object passed"));
        assert!(real_llc.contains("lnp64-netbsd-fs-service-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_fs_service_test_obj" \"#));
        assert!(real_llc.contains(r#""$libc_alloc_impl_obj" "$libc_string_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD fs service child link passed"));
        assert!(real_llc.contains("userland/classifier_test_clang.c"));
        assert!(classifier_test_clang.contains("#include <lnp64/intrinsics.h>"));
        assert!(real_llc.contains("netbsd-classifier-test-clang-smoke.o"));
        assert!(classifier_test_clang.contains("#include <poll.h>"));
        assert!(!classifier_test_clang.contains("typedef unsigned long nfds_t"));
        assert!(!classifier_test_clang.contains("int poll(struct pollfd"));
        assert!(real_llc.contains(r#"grep -q 'object_ctl r' "$netbsd_classifier_test_dump""#));
        assert!(real_llc.contains(r#"grep -q 'cap_dup r' "$netbsd_classifier_test_dump""#));
        assert!(real_llc.contains(r#"grep -q 'pull r' "$netbsd_classifier_test_dump""#));
        assert!(real_llc.contains("real LLVM LNP64 clang NetBSD classifier child object passed"));
        assert!(real_llc.contains("lnp64-netbsd-classifier-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_classifier_test_obj" \"#));
        assert!(real_llc.contains(r#""$libc_poll_impl_obj" "$libc_fd_impl_obj""#));
        assert!(real_llc.contains("netbsd-classifier-test-linked.dump"));
        assert!(real_llc.contains(r#"grep -q 'await r' "$netbsd_classifier_test_linked_dump""#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD classifier child link passed"));
        assert!(real_llc.contains("lnp64-netbsd-socket-loopback-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_socket_loopback_test_obj" "$libc_socket_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD socket loopback child link passed"));
        assert!(real_llc.contains("lnp64-netbsd-gate-trace-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_gate_trace_test_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD gate trace child link passed"));
        assert!(real_llc.contains("userland/domain_nested_test_clang.c"));
        assert!(domain_ctl_clang.contains("#include <lnp64/intrinsics.h>"));
        assert!(real_llc.contains("netbsd-domain-nested-test-clang-smoke.o"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang NetBSD domain nested child object passed")
        );
        assert!(real_llc.contains("lnp64-netbsd-domain-nested-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_domain_nested_test_obj" "$libc_fd_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD domain nested child link passed"));
        assert!(real_llc.contains("userland/domain_budget_test_clang.c"));
        assert!(real_llc.contains("netbsd-domain-budget-test-clang-smoke.o"));
        assert!(real_llc.contains(r#"grep -q 'alloc r' "$netbsd_domain_budget_test_dump""#));
        assert!(
            real_llc.contains("real LLVM LNP64 clang NetBSD domain budget child object passed")
        );
        assert!(real_llc.contains("lnp64-netbsd-domain-budget-test-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_domain_budget_test_obj" "$libc_fd_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld NetBSD domain budget child link passed"));
        assert!(real_llc.contains("lnp64-meta-libc-linked.elf"));
        assert!(real_llc.contains(
            r#""$meta_libc_obj" "$libc_meta_impl_obj" \
  "$libc_fd_impl_obj" "$libc_errno_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld metadata libc link smoke passed"));
        assert!(real_llc.contains("toolchain/liblnp64_vma_min.c"));
        assert!(libc_vma_min.contains("#include <sys/mman.h>"));
        assert!(!libc_vma_min.contains("typedef unsigned long size_t;"));
        assert!(libc_vma_min.contains(
            "void *mmap(void *addr, size_t len, int prot, int flags, int fd, off_t offset)"
        ));
        assert!(libc_vma_min.contains("int mprotect("));
        assert!(libc_vma_min.contains("int munmap("));
        assert!(libc_vma_min.contains("lnp64_complete_status"));
        assert!(libc_vma_min.contains("lnp64_complete_ptr"));
        assert!(libc_vma_min.contains("lnp64_errno_store(lnp64_errno_load())"));
        assert!(libc_vma_min.contains("__lnp_mmap_bootstrap"));
        assert!(libc_vma_min.contains("__lnp_mprotect_bootstrap"));
        assert!(libc_vma_min.contains("__lnp_munmap_bootstrap"));
        assert!(sys_mman_header.contains("#define MAP_FAILED"));
        assert!(sys_mman_header.contains("void *mmap("));
        assert!(sys_mman_header.contains("int mprotect("));
        assert!(sys_mman_header.contains("int munmap("));
        assert!(real_llc.contains("liblnp64-vma-min.o"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$libc_vma_impl_c\""));
        assert!(
            real_llc
                .contains("real LLVM LNP64 clang minilibc VMA implementation object smoke passed")
        );
        assert!(real_llc.contains("mmap-libc-clang-smoke.o"));
        assert!(real_llc.contains("#include <sys/mman.h>"));
        assert!(real_llc.contains("-I toolchain/include \\\n  -c \"$mmap_libc_c\""));
        assert!(real_llc.contains("MAP_FAILED"));
        assert!(real_llc.contains("PROT_READ | PROT_WRITE"));
        assert!(real_llc.contains("real LLVM LNP64 clang mmap libc object smoke passed"));
        assert!(real_llc.contains("lnp64-mmap-libc-linked.elf"));
        assert!(real_llc.contains(r#""$mmap_libc_obj" "$libc_vma_impl_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld mmap libc link smoke passed"));
        assert!(real_llc.contains("lnp64-futex-libc-linked.elf"));
        assert!(real_llc.contains(r#""$futex_libc_obj" "$libc_futex_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld futex libc link smoke passed"));
        assert!(real_llc.contains("lnp64-poll-libc-linked.elf"));
        assert!(real_llc.contains(r#""$poll_libc_obj" "$libc_poll_impl_obj""#));
        assert!(
            real_llc
                .contains("real LLVM LNP64 lld poll/select/epoll/kqueue libc link smoke passed")
        );
        assert!(real_llc.contains("lnp64-signal-libc-linked.elf"));
        assert!(real_llc.contains(
            r#""$signal_libc_obj" \
  "$libc_signal_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld signal libc link smoke passed"));
        assert!(real_llc.contains("lnp64-socket-libc-linked.elf"));
        assert!(real_llc.contains(
            r#""$socket_libc_obj" \
  "$libc_socket_impl_obj" "$libc_errno_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld socket libc link smoke passed"));
        assert!(real_llc.contains("lnp64-netbsd-personality-clang-linked.elf"));
        assert!(real_llc.contains(r#""$netbsd_personality_clang_obj" "$libc_fd_impl_obj" \"#));
        assert!(
            real_llc.contains("real LLVM LNP64 lld NetBSD personality clang smoke link passed")
        );
        assert!(real_llc.contains("lnp64-exit-linked.elf"));
        assert!(real_llc.contains(r#""$exit_obj" "$libc_process_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld exit link smoke passed"));
        assert!(real_llc.contains("lnp64-errno-linked.elf"));
        assert!(real_llc.contains(r#""$errno_obj" "$libc_errno_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld errno link smoke passed"));
        assert!(real_llc.contains("lnp64-startup-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld startup argv/envp link smoke passed"));
        assert!(real_llc.contains("lnp64-getauxval-linked.elf"));
        assert!(real_llc.contains(r#""$getauxval_obj" "$libc_startup_impl_obj" \"#));
        assert!(real_llc.contains(r#""$libc_errno_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld getauxval link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-argv-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_argv_obj" \"#));
        assert!(real_llc.contains(r#""$libc_stdio_impl_obj" "$libc_fd_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test argv link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-env-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_env_obj" \"#));
        assert!(real_llc.contains(r#""$libc_startup_impl_obj" "$libc_string_impl_obj""#));
        assert!(real_llc.contains(r#""$libc_errno_impl_obj" "$libc_fd_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test env link smoke passed"));
        assert!(real_llc.contains("lnp64-libc-test-random-linked.elf"));
        assert!(real_llc.contains(r#""$libc_test_random_obj" \"#));
        assert!(real_llc.contains(r#""$libc_random_impl_obj" "$libc_fd_impl_obj""#));
        assert!(real_llc.contains("real LLVM LNP64 lld libc-test random link smoke passed"));
        assert!(real_llc.contains("lnp64-scalar-arith-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld scalar arithmetic link smoke passed"));
        assert!(real_llc.contains("lnp64-high-mul-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld high-multiply link smoke passed"));
        assert!(real_llc.contains("lnp64-scalar-extend-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld scalar extension link smoke passed"));
        assert!(real_llc.contains("lnp64-bitmanip-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld bit-manip link smoke passed"));
        assert!(real_llc.contains("lnp64-csel-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld csel link smoke passed"));
        assert!(real_llc.contains("lnp64-call-clobber-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld call-clobber link smoke passed"));
        assert!(real_llc.contains("lnp64-intrinsic-await-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld intrinsic await link smoke passed"));
        assert!(real_llc.contains("lnp64-intrinsic-call-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld intrinsic call link smoke passed"));
        assert!(real_llc.contains("lnp64-intrinsic-gate-return-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld intrinsic gate return link smoke passed"));
        assert!(real_llc.contains("--triple=lnp64-unknown-none"));
        assert!(real_llc.contains("errno_set r0"));
        assert!(real_llc.contains("exit r1"));
        assert!(real_llc.contains("real LLVM LNP64 llvm-objdump crt0 decode smoke passed"));
        assert!(real_llc.contains("native-heap-smoke.o"));
        assert!(real_llc.contains("alloc_ex r3, r1, r2"));
        assert!(real_llc.contains("real LLVM LNP64 native heap opcode smoke passed"));
        assert!(real_llc.contains("-T toolchain/lnp64_static.ld"));
        assert!(real_llc.contains("real LLVM LNP64 lld static link smoke passed"));
        assert!(real_llc.contains("lnp64-native-heap-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld native heap link smoke passed"));
        assert!(real_llc.contains("lnp64-intrinsic-control-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld intrinsic control link smoke passed"));
        assert!(real_llc.contains("lnp64-intrinsic-cap-control-linked.elf"));
        assert!(
            real_llc.contains("real LLVM LNP64 lld intrinsic capability control link smoke passed")
        );
        assert!(real_llc.contains("intrinsic-mmap-clang-smoke.o"));
        assert!(real_llc.contains("__lnp_mmap_bootstrap"));
        assert!(real_llc.contains("__lnp_mprotect_bootstrap"));
        assert!(real_llc.contains("__lnp_munmap_bootstrap"));
        assert!(real_llc.contains("grep -q 'mmap r'"));
        assert!(real_llc.contains("grep -q 'mprotect r'"));
        assert!(real_llc.contains("grep -q 'munmap r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang intrinsic mmap object smoke passed"));
        assert!(real_llc.contains("lnp64-intrinsic-mmap-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld intrinsic mmap link smoke passed"));
        assert!(real_llc_docker.contains("lnp64-intrinsic-mmap-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf intrinsic mmap execution passed")
        );
        assert!(real_llc.contains("intrinsic-get-pcr-clang-smoke.o"));
        assert!(real_llc.contains("__lnp_get_pid"));
        assert!(real_llc.contains("grep -q 'get_pcr r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang intrinsic GET_PCR object smoke passed"));
        assert!(real_llc.contains("lnp64-intrinsic-get-pcr-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld intrinsic GET_PCR link smoke passed"));
        assert!(real_llc_docker.contains("lnp64-intrinsic-get-pcr-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf intrinsic GET_PCR execution passed")
        );
        assert!(real_llc.contains("intrinsic-set-pcr-clang-smoke.o"));
        assert!(real_llc.contains("__lnp_set_thread_pointer"));
        assert!(real_llc.contains("__lnp_set_event_mask"));
        assert!(real_llc.contains("grep -q 'set_pcr r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang intrinsic SET_PCR object smoke passed"));
        assert!(real_llc.contains("lnp64-intrinsic-set-pcr-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld intrinsic SET_PCR link smoke passed"));
        assert!(real_llc_docker.contains("lnp64-intrinsic-set-pcr-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf intrinsic SET_PCR execution passed")
        );
        assert!(real_llc.contains("intrinsic-openat-clang-smoke.o"));
        assert!(real_llc.contains("__lnp_openat"));
        assert!(real_llc.contains("grep -q 'open_at r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang intrinsic OPEN_AT object smoke passed"));
        assert!(real_llc.contains("lnp64-intrinsic-openat-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld intrinsic OPEN_AT link smoke passed"));
        assert!(real_llc_docker.contains("lnp64-intrinsic-openat-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf intrinsic OPEN_AT execution passed")
        );
        assert!(real_llc.contains("intrinsic-clone-clang-smoke.o"));
        assert!(real_llc.contains("__lnp_spawn_entry"));
        assert!(real_llc.contains("__lnp_thread_join"));
        assert!(real_llc.contains("grep -q 'clone.spawn r'"));
        assert!(real_llc.contains("grep -q 'thread_join r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang intrinsic CLONE object smoke passed"));
        assert!(real_llc.contains("lnp64-intrinsic-clone-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld intrinsic CLONE link smoke passed"));
        assert!(real_llc_docker.contains("lnp64-intrinsic-clone-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf intrinsic CLONE execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-poll-libc-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf poll/select/epoll/kqueue libc execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-signal-libc-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf signal libc execution passed"));
        assert!(real_llc_docker.contains("lnp64-socket-libc-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf socket libc execution passed"));
        assert!(real_llc_docker.contains("lnp64-netbsd-personality-clang-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf NetBSD personality clang smoke passed")
        );
        assert!(real_llc_docker.contains("netbsd clang personality smoke ok"));
        assert!(real_llc.contains("lnp64-intrinsic-amo-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld intrinsic AMO link smoke passed"));
        assert!(real_llc.contains("lnp64-c11-atomic-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld C11 atomic link smoke passed"));
        assert!(real_llc.contains("lnp64-stack-args-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld stack-argument link smoke passed"));
        assert!(real_llc.contains("pcr-clang-smoke.o"));
        assert!(real_llc.contains("-c demos/pcr.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang PCR demo object smoke passed"));
        assert!(real_llc.contains("cat-clang-smoke.o"));
        assert!(real_llc.contains("-c demos/cat.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang cat demo object smoke passed"));
        assert!(real_llc.contains("json-parser-clang-smoke.o"));
        assert!(real_llc.contains("-c demos/json_parser.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang json parser demo object smoke passed"));
        assert!(real_llc.contains("rot13-clang-smoke.o"));
        assert!(real_llc.contains("-c demos/rot13.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang rot13 demo object smoke passed"));
        assert!(real_llc.contains("producer-consumer-clang-smoke.o"));
        assert!(real_llc.contains("-c demos/producer_consumer.c"));
        assert!(real_llc.contains("grep -q 'clone.spawn r'"));
        assert!(real_llc.contains("grep -q 'thread_join r'"));
        assert!(real_llc.contains("grep -q 'lock.cmpxchg r'"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang producer consumer demo object smoke passed")
        );
        assert!(real_llc.contains("parallel-hash-clang-smoke.o"));
        assert!(real_llc.contains("-c demos/parallel_hash.c"));
        assert!(real_llc.contains("grep -q 'amo.add r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang parallel hash demo object smoke passed"));
        assert!(real_llc.contains("sqlite-lite-clang-smoke.o"));
        assert!(real_llc.contains("-c demos/sqlite_lite.c"));
        assert!(real_llc.contains("grep -q 'mmap r'"));
        assert!(real_llc.contains("grep -q 'fence'"));
        assert!(real_llc.contains("real LLVM LNP64 clang sqlite lite demo object smoke passed"));
        assert!(real_llc.contains("ping-pong-clang-smoke.o"));
        assert!(real_llc.contains("-c demos/ping_pong.c"));
        assert!(real_llc.contains("grep -q 'object_ctl r'"));
        assert!(real_llc.contains("real LLVM LNP64 clang ping pong demo object smoke passed"));
        assert!(real_llc.contains("zlib-adler32-clang-smoke.o"));
        assert!(real_llc.contains("-c third_party/zlib/adler32.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang zlib adler32 object smoke passed"));
        assert!(real_llc.contains("zlib-crc32-clang-smoke.o"));
        assert!(real_llc.contains("-c third_party/zlib/crc32.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang zlib crc32 object smoke passed"));
        assert!(real_llc.contains("zlib-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang zlib package object smoke passed"));
        assert!(real_llc.contains("lnp64-zlib-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld zlib package link smoke passed"));
        assert!(real_llc.contains("natsort-strnatcmp-clang-smoke.o"));
        assert!(real_llc.contains("-c third_party/natsort/strnatcmp.c"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang natsort implementation object smoke passed")
        );
        assert!(real_llc.contains("natsort-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang natsort package object smoke passed"));
        assert!(real_llc.contains("lnp64-natsort-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld natsort package link smoke passed"));
        assert!(real_llc.contains("jsmn-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang jsmn package object smoke passed"));
        assert!(real_llc.contains("lnp64-jsmn-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld jsmn package link smoke passed"));
        assert!(real_llc.contains("inih-clang-smoke.o"));
        assert!(real_llc.contains("-O0 -ffreestanding"));
        assert!(real_llc.contains("real LLVM LNP64 clang inih package object smoke passed"));
        assert!(real_llc.contains("lnp64-inih-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld inih package link smoke passed"));
        assert!(real_llc.contains("cwalk-clang-impl.o"));
        assert!(real_llc.contains("-c third_party/cwalk/src/cwalk.c"));
        assert!(
            real_llc.contains("real LLVM LNP64 clang cwalk implementation object smoke passed")
        );
        assert!(real_llc.contains("cwalk-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 clang cwalk package object smoke passed"));
        assert!(real_llc.contains("lnp64-cwalk-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld cwalk package link smoke passed"));
        assert!(real_llc.contains("varargs-call-clang-smoke.o"));
        assert!(real_llc.contains("printf(\"lnp64 %d %s"));
        assert!(real_llc.contains("real LLVM LNP64 clang varargs call object smoke passed"));
        assert!(real_llc.contains("sbase_commands=("));
        for command in [
            "echo", "cat", "wc", "yes", "basename", "dirname", "head", "tee", "cksum", "tail",
            "cmp", "uniq", "sort", "grep", "sed", "cp", "mv", "ls", "chmod", "chown", "ln",
            "mkdir", "rm", "cut", "tr", "touch", "find",
        ] {
            assert!(real_llc.contains(command));
        }
        assert!(real_llc.contains("-Werror=implicit-function-declaration"));
        assert!(real_llc.contains("sbase-$sbase_cmd-clang-smoke.o"));
        assert!(real_llc.contains("third_party/sbase/$sbase_cmd.c"));
        assert!(transition_manifest.contains("third_party/sbase/fs.h"));
        assert!(real_llc.contains("real LLVM LNP64 clang sbase command object smokes passed"));
        assert!(real_llc.contains("sbase_libutil_sources=("));
        for source in [
            "concat", "confirm", "cp", "enmasse", "fnck", "getlines", "linecmp", "writeall",
        ] {
            assert!(real_llc.contains(source));
        }
        assert!(real_llc.contains("sbase-libutil-$sbase_libutil-clang-smoke.o"));
        assert!(real_llc.contains("third_party/sbase/libutil/$sbase_libutil.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang sbase libutil object smokes passed"));
        assert!(real_llc.contains("toolchain/liblnp64_sbase_min.c"));
        assert!(libc_sbase_min.contains("void putword(FILE *stream, const char *word)"));
        assert!(libc_sbase_min.contains("void eprintf(const char *fmt, ...)"));
        assert!(libc_sbase_min.contains("void weprintf(const char *fmt, ...)"));
        assert!(libc_sbase_min.contains("char *argv0;"));
        assert!(real_llc.contains("liblnp64-sbase-min.o"));
        assert!(
            real_llc
                .contains("real LLVM LNP64 clang sbase support implementation object smoke passed")
        );
        assert!(real_llc.contains("lnp64-sbase-echo-linked.elf"));
        assert!(real_llc.contains(r#""$build_dir/sbase-echo-clang-smoke.o" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld sbase echo link smoke passed"));
        assert!(real_llc.contains("for sbase_path_cmd in basename dirname"));
        assert!(real_llc.contains("lnp64-sbase-$sbase_path_cmd-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld sbase path command link smoke passed"));
        assert!(real_llc.contains("lnp64-sbase-cat-linked.elf"));
        assert!(real_llc.contains("sbase-libutil-concat-clang-smoke.o"));
        assert!(real_llc.contains("sbase-libutil-writeall-clang-smoke.o"));
        assert!(real_llc.contains("real LLVM LNP64 lld sbase cat link smoke passed"));
        assert!(real_llc.contains("netcat-clang-smoke.o"));
        assert!(real_llc.contains("-c demos/netcat.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang netcat demo object smoke passed"));
        assert!(real_llc.contains("lnp64-netcat-clang-linked.elf"));
        assert!(real_llc.contains(r#""$netcat_obj" "$libc_fd_impl_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld netcat demo link smoke passed"));
        assert!(real_llc.contains("httpd-clang-smoke.o"));
        assert!(real_llc.contains("-c demos/httpd.c"));
        assert!(real_llc.contains("real LLVM LNP64 clang httpd demo object smoke passed"));
        assert!(real_llc.contains("lnp64-httpd-clang-linked.elf"));
        assert!(real_llc.contains(r#""$httpd_obj" "$libc_fd_impl_obj" \"#));
        assert!(real_llc.contains("real LLVM LNP64 lld httpd demo link smoke passed"));
        assert!(real_llc.contains("lnp64-$demo-clang-linked.elf"));
        assert!(real_llc.contains(
            r#""$demo_obj" "$libc_fd_impl_obj" \
    "$libc_alloc_impl_obj" "$libc_string_impl_obj" "$libc_process_impl_obj" \
    "$libc_futex_impl_obj""#
        ));
        assert!(real_llc.contains("real LLVM LNP64 lld clang demo link smoke passed"));
        assert!(real_llc.contains("rewrite_with_perl"));
        assert!(real_llc_docker.contains("Dockerfile.llvm"));
        assert!(real_llc_docker.contains("scripts/run_real_llvm_lnp64.sh"));
        assert!(real_llc_docker.contains(r#"--user "$uid:$gid""#));
        assert!(real_mc_docker.contains("Dockerfile.llvm"));
        assert!(real_mc_docker.contains("LNP64_LLVM_GATE=mc"));
        assert!(real_mc_docker.contains("scripts/run_real_llvm_lnp64.sh"));
        assert!(real_mc_docker.contains(r#"--user "$uid:$gid""#));
        assert!(llvm_dockerfile.contains("llvm-dev"));
        assert!(llvm_dockerfile.contains("llvm-runtime"));
        assert!(llvm_dockerfile.contains("clang"));
        assert!(llvm_dockerfile.contains("lld"));
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
        for flag in [
            "-ffreestanding",
            "-fno-pic",
            "-fno-jump-tables",
            "-Itoolchain/include",
            "-Itoolchain",
        ] {
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
            "toolchain/include/lnp64/intrinsics.h"
        );
        assert_eq!(
            manifest_field(driver_manifest, "loader_probe"),
            "lnp64 elf-plan"
        );
        assert_eq!(
            manifest_field(driver_manifest, "status"),
            "active_real_backend"
        );

        assert!(gate_manifest.contains("clang --target=lnp64-unknown-none"));
        assert!(gate_manifest.contains("-ffreestanding -fno-pic -fno-jump-tables -I toolchain"));
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
        let real_llc_docker = include_str!("../scripts/run_real_llvm_lnp64_docker.sh");
        let evidence_corpus = format!(
            "{main_source}\n{loader_source}\n{emulator_source}\n{lowering_source}\n{real_llc_docker}"
        );
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
        assert!(real_llc_docker.contains("cargo build --quiet --bin lnp64"));
        assert!(real_llc_docker.contains(r#""$lnp64_bin" elf-plan"#));
        assert!(real_llc_docker.contains(r#""$lnp64_bin" run-elf"#));
        assert!(real_llc_docker.contains("LNP64_LLVM_DOCKER_SKIP_RUN_ELF"));
        assert!(!real_llc_docker.contains("cargo run --quiet -- elf-plan"));
        assert!(!real_llc_docker.contains("cargo run --quiet -- run-elf"));
        assert!(real_llc_docker.contains("lnp64-$demo-clang-linked.elf"));
        assert!(real_llc_docker.contains("hello from LNP64"));
        assert!(real_llc_docker.contains("factorial ok"));
        assert!(real_llc_docker.contains("alloc ok"));
        assert!(real_llc_docker.contains("fibonacci ok"));
        assert!(real_llc_docker.contains("pcr ok"));
        assert!(real_llc_docker.contains("cat ok"));
        assert!(real_llc_docker.contains("json parser ok"));
        assert!(real_llc_docker.contains("rot13 ok"));
        assert!(real_llc_docker.contains("producer consumer ok"));
        assert!(real_llc_docker.contains("parallel hash ok"));
        assert!(real_llc_docker.contains("sqlite lite ok"));
        assert!(real_llc_docker.contains("ping pong ok"));
        assert!(real_llc_docker.contains("exit=0"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf clang demo execution passed"));
        assert!(real_llc_docker.contains("lnp64-native-heap-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf native heap execution passed"));
        assert!(real_llc_docker.contains("lnp64-indirect-call-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf indirect call execution passed"));
        assert!(real_llc_docker.contains("lnp64-scalar-arith-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf scalar arithmetic execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-high-mul-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf high-multiply execution passed"));
        assert!(real_llc_docker.contains("lnp64-scalar-extend-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf scalar extension execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-bitmanip-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf bit-manip execution passed"));
        assert!(real_llc_docker.contains("lnp64-csel-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf csel execution passed"));
        assert!(real_llc_docker.contains("lnp64-call-clobber-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf call-clobber execution passed"));
        assert!(real_llc_docker.contains("lnp64-stack-args-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf stack-argument execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-string-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf minilibc string execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-convert-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf numeric conversion execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-path-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf path helper execution passed"));
        assert!(real_llc_docker.contains("lnp64-search-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf search helper execution passed"));
        assert!(real_llc_docker.contains("lnp64-sort-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sort helper execution passed"));
        assert!(real_llc_docker.contains("lnp64-zlib-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf zlib package execution passed"));
        assert!(real_llc_docker.contains("lnp64-natsort-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf natsort package execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-jsmn-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf jsmn package execution passed"));
        assert!(real_llc_docker.contains("lnp64-inih-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf inih package execution passed"));
        assert!(real_llc_docker.contains("lnp64-cwalk-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf cwalk package execution passed"));
        assert!(real_llc_docker.contains("lnp64-libc-test-ctype-bounded-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test ctype_bounded execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-string-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test string execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-string-memcpy-bounded-linked.elf"));
        assert!(
            real_llc_docker.contains(
                "real LLVM LNP64 run-elf libc-test string_memcpy_bounded execution passed"
            )
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-string-memmove-bounded-linked.elf"));
        assert!(
            real_llc_docker.contains(
                "real LLVM LNP64 run-elf libc-test string_memmove_bounded execution passed"
            )
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-string-memmem-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test string_memmem execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-string-strchr-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test string_strchr execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-string-strcspn-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test string_strcspn execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-string-strstr-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test string_strstr execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-udiv-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test udiv execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-basename-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test basename execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-dirname-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test dirname execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-strtol-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test strtol execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-clock-gettime-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test clock_gettime execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-access-bounded-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test access_bounded execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-stat-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test stat execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-utime-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test utime execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-ungetc-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test ungetc execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-fdopen-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test fdopen execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-fcntl-basic-bounded-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test fcntl_basic_bounded execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-pthread-tsd-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test pthread_tsd execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-sem-init-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test sem_init execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-qsort-bounded-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test qsort_bounded execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-search-insque-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test search_insque execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-search-lsearch-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test search_lsearch execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-malloc-0-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test malloc-0 execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-fgets-eof-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf libc-test fgets-eof execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-calloc-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf calloc execution passed"));
        assert!(real_llc_docker.contains("lnp64-realloc-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf realloc execution passed"));
        assert!(real_llc_docker.contains("lnp64-read-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf read execution passed"));
        assert!(real_llc_docker.contains("lnp64-write-linked.elf"));
        assert!(real_llc_docker.contains("fd write ok"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf write execution passed"));
        assert!(real_llc_docker.contains("lnp64-meta-libc-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf metadata libc execution passed"));
        assert!(real_llc_docker.contains("lnp64-mmap-libc-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf mmap libc execution passed"));
        assert!(real_llc_docker.contains("lnp64-futex-libc-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf futex libc execution passed"));
        assert!(real_llc_docker.contains("lnp64-poll-libc-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf poll/select/epoll/kqueue libc execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-signal-libc-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf signal libc execution passed"));
        assert!(real_llc_docker.contains("lnp64-socket-libc-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf socket libc execution passed"));
        assert!(real_llc_docker.contains("lnp64-netbsd-personality-clang-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf NetBSD personality clang smoke passed")
        );
        assert!(real_llc_docker.contains("netbsd clang personality smoke ok"));
        assert!(real_llc_docker.contains("netbsd-elf-exec-fixture-root"));
        assert!(real_llc_docker.contains("lnp64-netbsd-elf-exec-test-linked.elf"));
        assert!(real_llc_docker.contains("loader_target ok"));
        assert!(real_llc_docker.contains("elf_exec_test ok"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf NetBSD ELF exec parent passed"));
        assert!(real_llc_docker.contains("netbsd-namespace-fixture-root"));
        assert!(real_llc_docker.contains("lnp64-netbsd-namespace-test-linked.elf"));
        assert!(real_llc_docker.contains("namespace_test ok"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf NetBSD namespace child passed"));
        assert!(real_llc_docker.contains("lnp64-netbsd-fork-wait-test-linked.elf"));
        assert!(real_llc_docker.contains("fork_wait_test ok"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf NetBSD fork/wait child passed"));
        assert!(real_llc_docker.contains("LNP64_LLVM_PACKAGE_FILTER=netbsd"));
        assert!(real_llc_docker.contains(
            "LNP64_LLVM_PACKAGE_FILTER=netbsd bash scripts/run_real_llvm_package_gate.sh"
        ));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf NetBSD package/system gate passed")
        );
        assert!(real_llc_docker.contains("lnp64-sbase-echo-linked.elf"));
        assert!(real_llc_docker.contains("echo hello clang --expect 'hello clang'"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sbase echo execution passed"));
        assert!(real_llc_docker.contains("lnp64-sbase-basename-linked.elf"));
        assert!(real_llc_docker.contains("basename /usr/local/bin/clang --expect '^clang$'"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf sbase basename execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-sbase-dirname-linked.elf"));
        assert!(
            real_llc_docker.contains("dirname /usr/local/bin/clang --expect '^/usr/local/bin$'")
        );
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sbase dirname execution passed"));
        assert!(real_llc_docker.contains("run-elf --namespace-root \"$sbase_fixture_root\""));
        assert!(real_llc_docker.contains("lnp64-sbase-cat-linked.elf"));
        assert!(real_llc_docker.contains("cat input/cat.txt"));
        assert!(real_llc_docker.contains("cat via clang"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf sbase cat execution passed"));
        assert!(real_llc_docker.contains("lnp64-userland-ucat-linked.elf"));
        assert!(real_llc_docker.contains("userland-fixture-root"));
        assert!(real_llc_docker.contains("ucat etc/motd"));
        assert!(real_llc_docker.contains("welcome from clang ucat"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf userland ucat execution passed"));
        assert!(real_llc_docker.contains("lnp64-userland-init-linked.elf"));
        assert!(real_llc_docker.contains("init /"));
        assert!(real_llc_docker.contains("lnp64 clang init: boot"));
        assert!(real_llc_docker.contains("lnp64 clang init: root /"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf userland init execution passed"));
        assert!(real_llc_docker.contains("lnp64-userland-lnpsh-linked.elf"));
        assert!(real_llc_docker.contains("lnpsh clang: scripted console"));
        assert!(real_llc_docker.contains("console"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf userland lnpsh execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-userland-spawn-task-linked.elf"));
        assert!(real_llc_docker.contains("userland spawn: parent"));
        assert!(real_llc_docker.contains("userland spawn: child"));
        assert!(real_llc_docker.contains("userland spawn: joined"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf userland spawn task execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-errno-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf errno execution passed"));
        assert!(real_llc_docker.contains("lnp64-intrinsic-push-linked.elf"));
        assert!(real_llc_docker.contains("intrinsic push ok"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf intrinsic push execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-intrinsic-await-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf intrinsic await execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-intrinsic-call-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf intrinsic call execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-intrinsic-gate-return-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf intrinsic gate return execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-intrinsic-control-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf intrinsic control execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-intrinsic-cap-control-linked.elf"));
        assert!(
            real_llc_docker
                .contains("real LLVM LNP64 run-elf intrinsic capability control execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-intrinsic-mmap-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf intrinsic mmap execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-intrinsic-clone-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf intrinsic CLONE execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-intrinsic-amo-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf intrinsic AMO execution passed"));
        assert!(real_llc_docker.contains("lnp64-c11-atomic-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf C11 atomic execution passed"));
        assert!(real_llc_docker.contains("lnp64-exit-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf exit execution passed"));
        assert!(real_llc_docker.contains("lnp64-startup-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf startup argv/envp execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-getauxval-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf getauxval execution passed"));
        assert!(real_llc_docker.contains("lnp64-libc-test-argv-linked.elf"));
        assert!(real_llc_docker.contains("lnp64-argv --expect"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test argv execution passed")
        );
        assert!(real_llc_docker.contains("lnp64-libc-test-env-linked.elf"));
        assert!(real_llc_docker.contains("real LLVM LNP64 run-elf libc-test env execution passed"));
        assert!(real_llc_docker.contains("lnp64-libc-test-random-linked.elf"));
        assert!(
            real_llc_docker.contains("real LLVM LNP64 run-elf libc-test random execution passed")
        );
        assert!(main_source.contains("\"run-elf\""));
        assert!(main_source.contains("take_run_namespace_root(&mut args)?"));
        assert!(main_source.contains("probe.machine.set_namespace_root(root)?"));
        assert!(main_source.contains("run_committed_exec"));
        assert!(loader_security.contains("submit_exec_plan"));
        assert!(
            loader_security.contains("emulator_commits_exec_descriptor_memory_image_atomically")
        );
        assert!(
            emulator_source.contains("exec_descriptor_startup_metadata_base_is_runtime_visible")
        );
        assert!(emulator_source.contains("fn startup_metadata_base(&self)"));
        assert!(
            emulator_source
                .contains("ENV_KEY_STARTUP_METADATA_PTR => Some(self.startup_metadata_base()?")
        );
        assert!(emulator_source.contains("fn exec_static_elf_image("));
        assert!(emulator_source.contains("committed_exec_opcode_loads_static_elf_child"));
        assert!(emulator_source.contains("crate::loader::load_static_elf"));
        assert!(emulator_source.contains("0x7f => Instr::Exec(a, b, c)"));

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
            "real_clang_lld_probe",
            "real_clang_demo_execution",
            "real_native_heap_execution",
            "real_libc_test_ctype_execution",
            "real_libc_test_string_execution",
            "real_libc_test_string_memcpy_bounded_execution",
            "real_libc_test_string_memmove_bounded_execution",
            "real_libc_test_string_memmem_execution",
            "real_libc_test_string_strchr_execution",
            "real_libc_test_string_strcspn_execution",
            "real_libc_test_string_strstr_execution",
            "real_libc_test_udiv_execution",
            "real_libc_test_basename_execution",
            "real_libc_test_dirname_execution",
            "real_libc_test_strtol_execution",
            "real_libc_test_clock_gettime_execution",
            "real_libc_test_access_bounded_execution",
            "real_libc_test_stat_execution",
            "real_libc_test_utime_execution",
            "real_libc_test_ungetc_execution",
            "real_libc_test_fdopen_execution",
            "real_libc_test_fcntl_basic_bounded_execution",
            "real_libc_test_pthread_tsd_execution",
            "real_libc_test_sem_init_execution",
            "real_libc_test_qsort_bounded_execution",
            "real_libc_test_search_insque_execution",
            "real_libc_test_search_lsearch_execution",
            "real_libc_test_malloc_0_execution",
            "real_libc_test_fgets_eof_execution",
            "real_numeric_conversion_execution",
            "real_path_helper_execution",
            "real_search_helper_execution",
            "real_sort_helper_execution",
            "real_read_execution",
            "real_write_execution",
            "real_userland_ucat_execution",
            "real_userland_init_execution",
            "real_userland_lnpsh_execution",
            "real_userland_spawn_task_execution",
            "real_netbsd_loader_target_child_execution",
            "real_netbsd_elf_exec_parent_execution",
            "real_netbsd_fork_wait_child_execution",
            "real_netbsd_thread_child_execution",
            "real_netbsd_poll_child_execution",
            "real_netbsd_signal_gate_child_execution",
            "real_netbsd_signal_fault_child_execution",
            "real_netbsd_timer_child_execution",
            "real_netbsd_mmap_child_execution",
            "real_netbsd_fd_passing_child_execution",
            "real_netbsd_namespace_child_execution",
            "real_netbsd_fs_service_child_execution",
            "real_netbsd_classifier_child_execution",
            "real_netbsd_socket_loopback_child_execution",
            "real_netbsd_gate_trace_child_execution",
            "real_netbsd_domain_nested_child_execution",
            "real_netbsd_domain_budget_child_execution",
            "real_netbsd_init_shell_system_execution",
            "real_metadata_libc_execution",
            "real_mmap_libc_execution",
            "real_futex_libc_execution",
            "real_poll_select_epoll_kqueue_libc_execution",
            "real_signal_libc_execution",
            "real_socket_libc_execution",
            "real_netbsd_personality_clang_execution",
            "real_sbase_echo_execution",
            "real_sbase_basename_execution",
            "real_sbase_dirname_execution",
            "real_sbase_cat_execution",
            "real_errno_execution",
            "real_startup_execution",
            "real_getauxval_execution",
            "real_libc_test_argv_execution",
            "real_libc_test_env_execution",
            "real_libc_test_random_execution",
            "real_intrinsic_await_execution",
            "real_intrinsic_call_execution",
            "real_intrinsic_gate_return_execution",
            "real_intrinsic_push_execution",
            "real_intrinsic_control_execution",
            "real_intrinsic_capability_control_execution",
            "real_intrinsic_mmap_execution",
            "real_intrinsic_get_pcr_execution",
            "real_intrinsic_set_pcr_execution",
            "real_intrinsic_openat_execution",
            "real_intrinsic_clone_execution",
            "real_intrinsic_amo_execution",
            "real_c11_atomic_execution",
            "real_stack_argument_execution",
            "real_exit_execution",
            "entry_state",
            "exec_opcode_static_elf",
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
            "cli_surface",
            "real_clang_lld_probe",
            "real_clang_demo_execution",
            "real_native_heap_execution",
            "real_libc_test_ctype_execution",
            "real_libc_test_string_execution",
            "real_libc_test_string_memcpy_bounded_execution",
            "real_libc_test_string_memmove_bounded_execution",
            "real_libc_test_string_memmem_execution",
            "real_libc_test_string_strchr_execution",
            "real_libc_test_string_strcspn_execution",
            "real_libc_test_string_strstr_execution",
            "real_libc_test_udiv_execution",
            "real_libc_test_basename_execution",
            "real_libc_test_dirname_execution",
            "real_libc_test_strtol_execution",
            "real_libc_test_clock_gettime_execution",
            "real_libc_test_access_bounded_execution",
            "real_libc_test_stat_execution",
            "real_libc_test_utime_execution",
            "real_libc_test_ungetc_execution",
            "real_libc_test_fdopen_execution",
            "real_libc_test_fcntl_basic_bounded_execution",
            "real_libc_test_pthread_tsd_execution",
            "real_libc_test_sem_init_execution",
            "real_libc_test_qsort_bounded_execution",
            "real_libc_test_search_insque_execution",
            "real_libc_test_search_lsearch_execution",
            "real_libc_test_malloc_0_execution",
            "real_libc_test_fgets_eof_execution",
            "real_numeric_conversion_execution",
            "real_path_helper_execution",
            "real_search_helper_execution",
            "real_sort_helper_execution",
            "real_write_execution",
            "real_userland_ucat_execution",
            "real_userland_init_execution",
            "real_userland_lnpsh_execution",
            "real_userland_spawn_task_execution",
            "real_netbsd_loader_target_child_execution",
            "real_netbsd_elf_exec_parent_execution",
            "real_netbsd_fork_wait_child_execution",
            "real_netbsd_thread_child_execution",
            "real_netbsd_poll_child_execution",
            "real_netbsd_signal_gate_child_execution",
            "real_netbsd_signal_fault_child_execution",
            "real_netbsd_timer_child_execution",
            "real_netbsd_mmap_child_execution",
            "real_netbsd_fd_passing_child_execution",
            "real_netbsd_namespace_child_execution",
            "real_netbsd_fs_service_child_execution",
            "real_netbsd_classifier_child_execution",
            "real_netbsd_socket_loopback_child_execution",
            "real_netbsd_gate_trace_child_execution",
            "real_netbsd_domain_nested_child_execution",
            "real_netbsd_domain_budget_child_execution",
            "real_netbsd_init_shell_system_execution",
            "real_metadata_libc_execution",
            "real_mmap_libc_execution",
            "real_futex_libc_execution",
            "real_poll_select_epoll_kqueue_libc_execution",
            "real_signal_libc_execution",
            "real_socket_libc_execution",
            "real_netbsd_personality_clang_execution",
            "real_sbase_echo_execution",
            "real_sbase_basename_execution",
            "real_sbase_dirname_execution",
            "real_sbase_cat_execution",
            "real_intrinsic_push_execution",
            "real_intrinsic_control_execution",
            "real_libc_test_argv_execution",
            "real_intrinsic_mmap_execution",
            "real_intrinsic_amo_execution",
            "real_c11_atomic_execution",
            "real_exit_execution",
            "real_errno_execution",
            "real_startup_execution",
            "real_getauxval_execution",
            "real_libc_test_env_execution",
            "real_libc_test_random_execution",
            "entry_state",
            "exec_opcode_static_elf",
            "text_fetch_decode",
        ] {
            assert_eq!(stages[stage].0, "tested", "{stage} should be tested");
        }
        assert_eq!(stages["stdout_exit"].0, "partial");
        assert_eq!(stages["no_toy_compiler"].0, "partial");
        assert!(roadmap.contains("run_without_toy_compiler` gate is partial"));
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
            "llvm/lib/Target/LNP64/LNP64ISelDAGToDAG.cpp",
            "llvm/lib/Target/LNP64/LNP64FrameLowering.cpp",
            "llvm/lib/Target/LNP64/LNP64AsmPrinter.cpp",
            "llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCTargetDesc.cpp",
            "llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCAsmInfo.cpp",
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
            "llvm/lib/Target/LNP64/LNP64ISelDAGToDAG.cpp",
            "llvm/lib/Target/LNP64/LNP64FrameLowering.cpp",
            "llvm/lib/Target/LNP64/LNP64AsmPrinter.cpp",
            "llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCTargetDesc.cpp",
            "llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCAsmInfo.cpp",
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
        let mc_asm_info = include_str!("../llvm/lib/Target/LNP64/MCTargetDesc/LNP64MCAsmInfo.h");
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
        let asm_printer = include_str!("../llvm/lib/Target/LNP64/LNP64AsmPrinter.cpp");
        let subtarget = include_str!("../llvm/lib/Target/LNP64/LNP64Subtarget.cpp");
        let isel = include_str!("../llvm/lib/Target/LNP64/LNP64ISelLowering.cpp");
        let dag_isel = include_str!("../llvm/lib/Target/LNP64/LNP64ISelDAGToDAG.cpp");
        let isel_header = include_str!("../llvm/lib/Target/LNP64/LNP64ISelLowering.h");
        let frame = include_str!("../llvm/lib/Target/LNP64/LNP64FrameLowering.cpp");
        let reginfo = include_str!("../llvm/lib/Target/LNP64/LNP64RegisterInfo.cpp");
        let clang_target_header = include_str!("../clang/lib/Basic/Targets/LNP64.h");
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
        assert!(registers_td.contains("class LNP64GPR<bits<16> Enc"));
        assert!(calling_td.contains("CC_LNP64"));
        assert!(calling_td.contains("R1, R2, R3, R4, R5, R6"));
        assert!(calling_td.contains("iPTR"));
        for opcode in [
            "ADD",
            "LI32",
            "LD",
            "CALL",
            "YIELD",
            "LR_GET",
            "LR_SET",
            "RET",
            "CSET_EQ",
            "CSET_ULT",
            "CMPU",
            "ERRNO_SET",
            "FORK",
            "WAIT_PID",
            "GET_PCR",
            "SET_PCR",
            "OPEN_AT",
            "CLONE_SPAWN",
            "THREAD_JOIN",
            "EXIT",
            "AWAIT",
            "GATE_CALL",
            "GATE_RETURN",
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
            "class LNP64PcrGet",
            "(ins PCR:$pcr)",
            "class LNP64PcrSet",
        ] {
            assert!(instr_td.contains(shape), "instr TableGen missing {shape}");
        }
        assert!(cmake.contains("LNP64GenRegisterInfo.inc"));
        assert!(cmake.contains("LNP64GenDAGISel.inc"));
        assert!(cmake.contains("AsmPrinter"));
        assert!(cmake.contains("SelectionDAG"));
        assert!(cmake.contains("add_llvm_component_group(LNP64)"));
        assert!(cmake.contains("ADD_TO_COMPONENT"));
        for source in [
            "LNP64TargetMachine.cpp",
            "LNP64AsmPrinter.cpp",
            "LNP64Subtarget.cpp",
            "LNP64ISelLowering.cpp",
            "LNP64ISelDAGToDAG.cpp",
            "LNP64FrameLowering.cpp",
            "add_subdirectory(InstPrinter)",
            "add_subdirectory(AsmParser)",
            "add_subdirectory(Disassembler)",
        ] {
            assert!(cmake.contains(source), "CMake missing {source}");
        }
        assert!(cmake.contains("add_llvm_target(LNP64CodeGen"));
        assert!(mc_desc_cmake.contains("LNP64MCAsmBackend.cpp"));
        assert!(mc_desc_cmake.contains("LNP64MCAsmInfo.cpp"));
        assert!(mc_desc_cmake.contains("LNP64InstPrinter"));
        assert!(target_info.contains("LLVMInitializeLNP64TargetInfo"));
        assert!(target_info.contains("RegisterTarget<Triple::lnp64>"));
        assert!(mc_desc.contains("LLVMInitializeLNP64TargetMC"));
        assert!(mc_desc.contains("RegisterMCAsmInfo<LNP64MCAsmInfo>"));
        assert!(mc_desc.contains("RegisterMCCodeEmitter"));
        assert!(mc_desc.contains("RegisterMCAsmBackend"));
        assert!(mc_desc.contains("RegisterMCInstPrinter"));
        assert!(mc_desc_header.contains("fixup_lnp64_branch26"));
        assert!(mc_asm_info.contains("MCAsmInfoELF"));
        assert!(mc_emitter.contains("createLNP64MCCodeEmitter"));
        assert!(mc_asm_backend.contains("createLNP64AsmBackend"));
        assert!(mc_asm_backend.contains("LNP64ELFObjectWriter"));
        assert!(mc_asm_backend.contains("R_LNP64_BRANCH26"));
        assert!(inst_printer.contains("createLNP64MCInstPrinter"));
        assert!(inst_printer.contains("getLNP64Mnemonic"));
        assert!(inst_printer.contains("printMemOperand"));
        assert!(inst_printer.contains("errno_set"));
        assert!(inst_printer.contains("fork"));
        assert!(inst_printer.contains("wait_pid"));
        assert!(inst_printer.contains("cset.eq"));
        assert!(inst_printer.contains("cset.ult"));
        assert!(inst_printer.contains("case LNP64::EXIT"));
        assert!(inst_printer.contains("case LNP64::MMAP"));
        assert!(inst_printer.contains("case LNP64::MUNMAP"));
        assert!(inst_printer.contains("case LNP64::MPROTECT"));
        assert!(inst_printer.contains("case LNP64::ENV_GET"));
        assert!(inst_printer.contains("case LNP64::LA"));
        assert!(inst_printer.contains("case LNP64::AUIPC"));
        assert!(inst_printer.contains("case LNP64::LI32"));
        assert!(inst_printer.contains("case LNP64::YIELD"));
        assert!(inst_printer.contains("call_reg"));
        assert!(inst_printer.contains("lr_get"));
        assert!(inst_printer.contains("lr_set"));
        assert!(mc_emitter.contains("case LNP64::AND"));
        assert!(mc_emitter.contains("case LNP64::CMP"));
        assert!(mc_emitter.contains("case LNP64::CMPU"));
        assert!(mc_emitter.contains("case LNP64::CSET_EQ"));
        assert!(mc_emitter.contains("case LNP64::CSET_ULT"));
        assert!(mc_emitter.contains("case LNP64::LA"));
        assert!(mc_emitter.contains("case LNP64::AUIPC"));
        assert!(mc_emitter.contains("case LNP64::LI32"));
        assert!(mc_emitter.contains("case LNP64::MMAP"));
        assert!(mc_emitter.contains("case LNP64::MUNMAP"));
        assert!(mc_emitter.contains("case LNP64::MPROTECT"));
        assert!(mc_emitter.contains("case LNP64::ENV_GET"));
        assert!(mc_emitter.contains("case LNP64::CLONE_SPAWN"));
        assert!(mc_emitter.contains("case LNP64::THREAD_JOIN"));
        assert!(mc_emitter.contains("case LNP64::FORK"));
        assert!(mc_emitter.contains("case LNP64::WAIT_PID"));
        assert!(mc_emitter.contains("case LNP64::YIELD"));
        assert!(mc_emitter.contains("fixup_lnp64_pcrel32"));
        assert!(mc_emitter.contains("fixup_lnp64_abs32"));
        assert!(mc_emitter.contains("case LNP64::LD_W"));
        assert!(mc_emitter.contains("case LNP64::LD_H"));
        assert!(mc_emitter.contains("case LNP64::ST_B"));
        assert!(mc_emitter.contains("case LNP64::ST_H"));
        assert!(mc_emitter.contains("not implemented yet"));
        assert!(mc_emitter.contains("isInt<14>(Offset)"));
        assert!(asm_parser.contains("LLVMInitializeLNP64AsmParser"));
        assert!(asm_parser.contains("RegisterMCAsmParser"));
        assert!(asm_parser.contains("tryParseRegister"));
        assert!(asm_parser.contains("parseImmediateOrMemory"));
        assert!(asm_parser.contains("buildInstruction"));
        assert!(asm_parser.contains(r#".Case("la", LNP64::LA)"#));
        assert!(asm_parser.contains(r#".Case("auipc", LNP64::AUIPC)"#));
        assert!(asm_parser.contains(r#".Case("li32", LNP64::LI32)"#));
        assert!(asm_parser.contains(r#".Case("yield", LNP64::YIELD)"#));
        assert!(asm_parser.contains("Opcode == LNP64::YIELD"));
        assert!(asm_parser.contains(r#".Case("call", LNP64::CALL)"#));
        assert!(asm_parser.contains(r#".Case("lr_get", LNP64::LR_GET)"#));
        assert!(asm_parser.contains(r#".Case("lr_set", LNP64::LR_SET)"#));
        assert!(asm_parser.contains(r#".Case("cset.eq", LNP64::CSET_EQ)"#));
        assert!(asm_parser.contains(r#".Case("cmpu", LNP64::CMPU)"#));
        assert!(asm_parser.contains(r#".Case("cset.ult", LNP64::CSET_ULT)"#));
        assert!(asm_parser.contains(r#".Case("errno_get", LNP64::ERRNO_GET)"#));
        assert!(asm_parser.contains(r#".Case("errno_set", LNP64::ERRNO_SET)"#));
        assert!(asm_parser.contains(r#".Case("fork", LNP64::FORK)"#));
        assert!(asm_parser.contains(r#".Case("wait_pid", LNP64::WAIT_PID)"#));
        assert!(asm_parser.contains(r#".Case("exit", LNP64::EXIT)"#));
        assert!(asm_parser.contains(r#".Case("mmap", LNP64::MMAP)"#));
        assert!(asm_parser.contains(r#".Case("munmap", LNP64::MUNMAP)"#));
        assert!(asm_parser.contains(r#".Case("mprotect", LNP64::MPROTECT)"#));
        assert!(asm_parser.contains(r#".Case("get_pcr", LNP64::GET_PCR)"#));
        assert!(asm_parser.contains(r#".Case("set_pcr", LNP64::SET_PCR)"#));
        assert!(asm_parser.contains(r#".Case("PID", LNP64::PID)"#));
        assert!(asm_parser.contains(r#".Case("env_get", LNP64::ENV_GET)"#));
        assert!(asm_parser.contains(r#".Case("open_at", LNP64::OPEN_AT)"#));
        assert!(asm_parser.contains(r#".Case("clone.spawn", LNP64::CLONE_SPAWN)"#));
        assert!(asm_parser.contains(r#".Case("thread_join", LNP64::THREAD_JOIN)"#));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::YIELD)"));
        assert!(instr_td.contains("def OPEN_AT : LNP64Native4"));
        assert!(instr_td.contains("def FORK : LNP64RuntimeGet"));
        assert!(instr_td.contains("def WAIT_PID : LNP64RR"));
        assert!(instr_td.contains("def CLONE_SPAWN : LNP64RRR"));
        assert!(instr_td.contains("def THREAD_JOIN : LNP64RRR"));
        assert!(instr_td.contains("def GET_PCR : LNP64PcrGet"));
        assert!(instr_td.contains("def SET_PCR : LNP64PcrSet"));
        assert!(asm_parser.contains(r#".Case("ld.w", LNP64::LD_W)"#));
        assert!(asm_parser.contains(r#".Case("ld.h", LNP64::LD_H)"#));
        assert!(disassembler.contains("LLVMInitializeLNP64Disassembler"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::FORK)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::WAIT_PID)"));
        assert!(disassembler.contains("RegisterMCDisassembler"));
        assert!(disassembler.contains("readLE32"));
        assert!(disassembler.contains("ArrayRef<uint8_t> Bytes"));
        assert!(!disassembler.contains("MemoryObject"));
        assert!(disassembler.contains("case 0x10"));
        assert!(disassembler.contains("case 0x03"));
        assert!(disassembler.contains("case 0x04"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::LA)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::AUIPC)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::LI32)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::ADD)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::AND)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::CMP)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::CMPU)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::CSET_EQ)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::CSET_ULT)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::CALL)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::CALL_REG)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::LR_GET)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::LR_SET)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::ERRNO_GET)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::ERRNO_SET)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::EXIT)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::MMAP)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::MUNMAP)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::MPROTECT)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::ENV_GET)"));
        assert!(disassembler.contains("case 0x54"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::GET_PCR)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::SET_PCR)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::OPEN_AT)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::CLONE_SPAWN)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::THREAD_JOIN)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::LD_W)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::LD_H)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::ST_B)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::ST_H)"));
        assert!(disassembler.contains("SignExtend64<14>"));
        assert!(disassembler.contains("decodeBranchTarget"));
        assert!(disassembler.contains("MCDisassembler::Fail"));
        assert!(target_machine.contains("LLVMInitializeLNP64Target"));
        assert!(target_machine.contains("LLVMInitializeLNP64AsmPrinter"));
        assert!(target_machine.contains("createPassConfig"));
        assert!(target_machine.contains("addInstSelector"));
        assert!(target_machine.contains("createLNP64ISelDag"));
        assert!(target_machine.contains("TargetLoweringObjectFileELF"));
        assert!(target_machine.contains("e-m:e-p:64:64-i64:64-n64-S128"));
        assert!(target_machine.contains("initAsmInfo()"));
        assert!(dag_isel.contains("SelectionDAGISel"));
        assert!(dag_isel.contains("LNP64GenDAGISel.inc"));
        assert!(dag_isel.contains("SelectCode(Node)"));
        assert!(dag_isel.contains("SelectFrameIndexValue"));
        assert!(dag_isel.contains("LNP64::PseudoFRAMEADDR"));
        assert!(dag_isel.contains("SelectFrameIndexLoad"));
        assert!(dag_isel.contains("SelectFrameIndexStore"));
        assert!(dag_isel.contains("getTargetFrameIndex"));
        assert!(dag_isel.contains("ISD::SEXTLOAD"));
        assert!(dag_isel.contains("ISD::EXTLOAD"));
        assert!(dag_isel.contains("MemVT == MVT::i1"));
        assert!(dag_isel.contains("LNP64::PseudoLD_SW"));
        assert!(dag_isel.contains("LNP64::PseudoLD_SH"));
        assert!(dag_isel.contains("LNP64::PseudoLD_SB"));
        assert!(dag_isel.contains("LNP64::LD_W"));
        assert!(dag_isel.contains("LNP64::ST_W"));
        assert!(dag_isel.contains("LNP64::ST_H"));
        assert!(dag_isel.contains("LNP64::ST_B"));
        assert!(asm_printer.contains("RegisterAsmPrinter<LNP64AsmPrinter>"));
        assert!(asm_printer.contains("void LNP64AsmPrinter::emitInstruction"));
        assert!(asm_printer.contains("PrintAsmOperand"));
        assert!(asm_printer.contains("printLNP64AsmReg"));
        assert!(inst_printer.contains("case LNP64::GET_PCR"));
        assert!(inst_printer.contains("return \"get_pcr\""));
        assert!(inst_printer.contains("case LNP64::OPEN_AT"));
        assert!(inst_printer.contains("return \"open_at\""));
        assert!(inst_printer.contains("case LNP64::CLONE_SPAWN"));
        assert!(inst_printer.contains("return \"clone.spawn\""));
        assert!(inst_printer.contains("case LNP64::THREAD_JOIN"));
        assert!(inst_printer.contains("return \"thread_join\""));
        assert!(inst_printer.contains("case LNP64::OPEN_AT:\n  case LNP64::PULL:"));
        assert!(inst_printer.contains("OS << \"PID\""));
        assert!(inst_printer.contains("OS << \"SIGMASK\""));
        assert!(asm_printer.contains("PrintAsmMemoryOperand"));
        assert!(asm_printer.contains("MachineOperand::MO_MachineBasicBlock"));
        assert!(asm_printer.contains("MachineOperand::MO_GlobalAddress"));
        assert!(asm_printer.contains("MachineOperand::MO_ExternalSymbol"));
        assert!(asm_printer.contains("EmitToStreamer(*OutStreamer, Inst)"));
        assert!(subtarget.contains("TLInfo(TM, *this)"));
        assert!(isel.contains("addRegisterClass(MVT::i64"));
        assert!(isel.contains("ISD::ADD"));
        assert!(isel.contains("ISD::SDIV"));
        assert!(isel.contains("setOperationAction(ISD::BR_CC, MVT::i64, Custom)"));
        assert!(isel.contains("setOperationAction(ISD::BRCOND, MVT::Other, Custom)"));
        assert!(isel.contains("getLNP64CSetInstr"));
        assert!(isel.contains("isLNP64UnsignedSetCCPseudo"));
        assert!(isel.contains("LNP64::PseudoLINeg32"));
        assert!(isel.contains("LNP64::PseudoLI64"));
        assert!(isel.contains("TII.get(LNP64::LI32)"));
        assert!(isel.contains("TII.get(LNP64::LSLI)"));
        assert!(isel.contains("TII.get(LNP64::OR)"));
        assert!(isel.contains("LNP64GenCallingConv.inc"));
        assert!(isel.contains("LowerOperation"));
        assert!(isel.contains("setOperationAction(ISD::GlobalAddress, MVT::i64, Custom)"));
        assert!(isel.contains("ISD::GlobalAddress"));
        assert!(isel.contains("LNP64ISD::WRAPPER"));
        assert!(isel.contains("ISD::BR_CC"));
        assert!(isel.contains("ISD::BRCOND"));
        assert!(isel.contains("DAG.getNode(LNP64ISD::BR_NE"));
        assert!(
            isel.contains(
                "LNP64 conditional branch lowering only supports integer comparisons today"
            )
        );
        assert!(isel.contains("EmitInstrWithCustomInserter"));
        assert!(isel.contains("LNP64::PseudoBEQ"));
        assert!(isel.contains("LNP64::PseudoBULT"));
        assert!(isel.contains("isLNP64UnsignedBranchPseudo"));
        assert!(isel.contains("BuildMI(*BB, MI, DL, TII.get(CmpOpcode))"));
        assert!(isel.contains("isLNP64UnsignedBranchPseudo(MI.getOpcode()) ? LNP64::CMPU"));
        assert!(isel.contains("TII.get(getLNP64CSetInstr(MI.getOpcode()))"));
        assert!(isel.contains("BuildMI(*BB, MI, DL, TII.get(BranchOpcode))"));
        assert!(isel.contains("LowerFormalArguments"));
        assert!(isel.contains("CCInfo.AnalyzeFormalArguments(Ins, CC_LNP64)"));
        assert!(isel.contains("CreateFixedObject"));
        assert!(isel.contains("MachinePointerInfo::getFixedStack"));
        assert!(isel.contains("MF.addLiveIn(VA.getLocReg(), &LNP64::GPRRegClass)"));
        assert!(isel.contains("LowerReturn"));
        assert!(isel.contains("CCInfo.AnalyzeReturn(Outs, RetCC_LNP64)"));
        assert!(isel.contains("DAG.getCopyToReg"));
        assert!(isel.contains("LowerCall"));
        assert!(isel.contains("ArgCCInfo.AnalyzeCallOperands(CLI.Outs, CC_LNP64)"));
        assert!(isel.contains("DAG.getCALLSEQ_START"));
        assert!(isel.contains("DAG.getCALLSEQ_END"));
        assert!(isel.contains("DAG.getTargetGlobalAddress"));
        assert!(isel.contains("DAG.getTargetExternalSymbol"));
        assert!(isel.contains("indirect call callee must lower to an i64 register"));
        assert!(isel.contains("ISD::ATOMIC_LOAD"));
        assert!(isel.contains("ISD::ATOMIC_STORE"));
        assert!(isel.contains("ISD::ATOMIC_LOAD_ADD"));
        assert!(isel.contains("ISD::ATOMIC_LOAD_XOR"));
        assert!(isel.contains("ISD::ATOMIC_CMP_SWAP"));
        assert!(isel.contains("LNP64ISD::CALL"));
        assert!(isel.contains("CalleeName == \"__lnp_await\" || CalleeName == \"__lnp_call\""));
        assert!(
            isel.contains(
                "CalleeName == \"__lnp_domain_ctl\" || CalleeName == \"__lnp_object_ctl\""
            )
        );
        assert!(isel.contains("LNP64ISD::AWAIT"));
        assert!(isel.contains("LNP64ISD::DOMAIN_CTL"));
        assert!(isel.contains("LNP64ISD::GATE_CALL"));
        assert!(isel.contains("LNP64ISD::GATE_RETURN"));
        assert!(isel.contains("LNP64ISD::OBJECT_CTL"));
        assert!(isel.contains("LNP64ISD::PULL"));
        assert!(isel.contains("LNP64ISD::PUSH"));
        assert!(isel.contains("RetCCInfo.AnalyzeCallResult(CLI.Ins, RetCC_LNP64)"));
        assert!(isel.contains("native shim lowering expects three arguments and a result"));
        assert!(isel.contains("native control lowering expects one argument and a result"));
        assert!(isel.contains("LNP64ISD::RET_FLAG"));
        assert!(isel.contains("setLoadExtAction(ISD::ZEXTLOAD, MVT::i64, MemVT, Legal)"));
        assert!(isel.contains("MVT::i1"));
        assert!(isel.contains("setLoadExtAction(ISD::SEXTLOAD, MVT::i64, MemVT, Legal)"));
        assert!(isel.contains("setLoadExtAction(ISD::EXTLOAD, MVT::i64, MemVT, Legal)"));
        assert!(instr_td.contains("zextloadi1"));
        assert!(isel.contains("getLNP64SignedLoadInstr"));
        assert!(isel.contains("getLNP64SignExtendInstr"));
        assert!(isel.contains("setTruncStoreAction(MVT::i64, MemVT, Legal)"));
        assert!(isel.contains("LNP64TargetLowering::getConstraintType"));
        assert!(isel.contains("return C_RegisterClass"));
        assert!(isel.contains("LNP64TargetLowering::getRegForInlineAsmConstraint"));
        assert!(isel.contains("return std::make_pair(0U, &LNP64::GPRRegClass)"));
        assert!(isel.contains("computeRegisterProperties"));
        assert!(isel.contains("CCState ArgCCInfo(CLI.CallConv, CLI.IsVarArg"));
        assert!(isel.contains("unsigned VarArgStackBytes = 0"));
        assert!(isel.contains("if (CLI.IsVarArg && !CLI.Outs[I].IsFixed)"));
        assert!(isel.contains("VarArgStackOffset += alignTo"));
        assert!(isel.contains("setOperationAction(ISD::VASTART, MVT::Other, Custom)"));
        assert!(isel.contains("setOperationAction(ISD::VAEND, MVT::Other, Expand)"));
        assert!(isel.contains("case ISD::VASTART"));
        assert!(isel.contains("CreateFixedObject(8, 0, /*IsImmutable=*/true)"));
        assert!(!isel.contains("varargs lowering is not implemented yet"));
        assert!(isel_header.contains("getTargetNodeName"));
        assert!(isel_header.contains("LowerOperation"));
        assert!(isel_header.contains("getConstraintType"));
        assert!(isel_header.contains("getRegForInlineAsmConstraint"));
        assert!(isel_header.contains("EmitInstrWithCustomInserter"));
        assert!(isel_header.contains("BR_EQ"));
        assert!(isel_header.contains("BR_ULT"));
        assert!(isel_header.contains("LowerFormalArguments"));
        assert!(isel_header.contains("LowerReturn"));
        assert!(isel_header.contains("LowerCall"));
        assert!(isel_header.contains("AWAIT"));
        assert!(isel_header.contains("CALL"));
        assert!(isel_header.contains("DOMAIN_CTL"));
        assert!(isel_header.contains("GATE_CALL"));
        assert!(isel_header.contains("GATE_RETURN"));
        assert!(isel_header.contains("OBJECT_CTL"));
        assert!(isel_header.contains("PULL"));
        assert!(isel_header.contains("PUSH"));
        assert!(isel_header.contains("WRAPPER"));
        assert!(isel_header.contains("RET_FLAG"));
        assert!(instr_td.contains("def simm16_imm"));
        assert!(instr_td.contains("def simm14_imm"));
        assert!(instr_td.contains("def uimm32_imm"));
        assert!(instr_td.contains("def sneg32_imm"));
        assert!(instr_td.contains("def wide64_imm"));
        assert!(instr_td.contains("def all_ones_imm"));
        assert!(instr_td.contains("def brtarget : Operand<OtherVT>"));
        assert!(instr_td.contains("(ins brtarget:$target)"));
        assert!(instr_td.contains("def SDT_LNP64BrCC"));
        assert!(instr_td.contains("def LNP64breq"));
        assert!(instr_td.contains("def LNP64brne"));
        assert!(instr_td.contains("def LNP64brult"));
        assert!(instr_td.contains("class LNP64CondBranchPseudo"));
        assert!(instr_td.contains("class LNP64SetCCPseudo"));
        assert!(instr_td.contains("class LNP64SignedLoadPseudo"));
        assert!(instr_td.contains("class LNP64FrameAddrPseudo"));
        assert!(instr_td.contains("def CSET_EQ"));
        assert!(instr_td.contains("def CSET_ULT"));
        assert!(instr_td.contains("def PseudoLD_SB"));
        assert!(instr_td.contains("def PseudoFRAMEADDR"));
        assert!(instr_td.contains("usesCustomInserter = 1"));
        assert!(instr_td.contains("def PseudoBEQ"));
        assert!(instr_td.contains("def PseudoBULT"));
        assert!(instr_td.contains("(PseudoBEQ GPR:$lhs, GPR:$rhs, bb:$target)"));
        assert!(instr_td.contains("(PseudoBULT GPR:$lhs, GPR:$rhs, bb:$target)"));
        assert!(instr_td.contains("(PseudoCSETEQ GPR:$lhs, GPR:$rhs)"));
        assert!(instr_td.contains("(PseudoCSETULT GPR:$lhs, GPR:$rhs)"));
        assert!(instr_td.contains("(PseudoCSETNEI GPR:$lhs, simm16_imm:$rhs)"));
        assert!(instr_td.contains("(PseudoCSETUGEI GPR:$lhs, simm16_imm:$rhs)"));
        assert!(instr_td.contains("(i64 (sextloadi8 (add GPR:$base, simm14_imm:$offset)))"));
        assert!(instr_td.contains("(i64 (extloadi16 (add GPR:$base, simm14_imm:$offset)))"));
        assert!(instr_td.contains("(PseudoLD_SH GPR:$base, simm14_imm:$offset)"));
        assert!(instr_td.contains("def LNP64retflag"));
        assert!(instr_td.contains("def SDT_LNP64Call"));
        assert!(instr_td.contains("SDTypeProfile<0, -1, []>"));
        assert!(instr_td.contains("def LNP64call"));
        assert!(instr_td.contains("def LNP64domainctl"));
        assert!(instr_td.contains("def LNP64gatecall"));
        assert!(instr_td.contains("def LNP64objectctl"));
        assert!(instr_td.contains("def LNP64pull"));
        assert!(instr_td.contains("def LNP64push"));
        assert!(instr_td.contains("def LNP64wrapper"));
        assert!(instr_td.contains("(set GPR:$rd, simm16_imm:$imm)"));
        assert!(instr_td.contains("def LA"));
        assert!(instr_td.contains("def AUIPC"));
        assert!(instr_td.contains("def LI32"));
        assert!(instr_td.contains("def PseudoLINeg32"));
        assert!(instr_td.contains("let Size = 8"));
        assert!(instr_td.contains("(i64 (LNP64wrapper tglobaladdr:$target))"));
        assert!(instr_td.contains("(set GPR:$rd, uimm32_imm:$imm)"));
        assert!(instr_td.contains("(PseudoLINeg32 sneg32_imm:$imm)"));
        assert!(instr_td.contains("(PseudoLI64 wide64_imm:$imm)"));
        assert!(instr_td.contains("(set GPR:$rd, (add GPR:$rs1, GPR:$rs2))"));
        assert!(instr_td.contains("(set GPR:$rd, (xor GPR:$rs, all_ones_imm))"));
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
        assert!(instr_td.contains("Defs = [LR, R1, R2"));
        assert!(instr_td.contains("R28, R29]"));
        assert!(instr_td.contains("Defs = [FLAGS]"));
        assert!(instr_td.contains("Uses = [LR]"));
        assert!(instr_td.contains("Uses = [FLAGS]"));
        assert!(instr_td.contains("let Pattern = [(LNP64retflag)]"));
        assert!(instr_td.contains("isBranch = 1"));
        assert!(instr_td.contains("ADJCALLSTACKDOWN"));
        assert!(instr_td.contains("ADJCALLSTACKUP"));
        assert!(instr_info.contains("/*CFSetupOpcode=*/LNP64::ADJCALLSTACKDOWN"));
        assert!(instr_info.contains("/*CFDestroyOpcode=*/LNP64::ADJCALLSTACKUP"));
        assert!(instr_info.contains("/*ReturnOpcode=*/LNP64::RET"));
        assert!(instr_info.contains("copyPhysReg"));
        assert!(instr_info.contains("BuildMI(MBB, I, DL, get(LNP64::MOV), DestReg)"));
        assert!(instr_info.contains("storeRegToStackSlot"));
        assert!(instr_info.contains("loadRegFromStackSlot"));
        assert!(instr_info.contains("addFrameIndex(FrameIndex)"));
        assert!(isel.contains("setStackPointerRegisterToSaveRestore(LNP64::R31)"));
        assert!(frame.contains("StackGrowsDown"));
        assert!(frame.contains("bool LNP64FrameLowering::hasFP"));
        assert!(frame.contains("/*LocalAreaOffset=*/0"));
        assert!(frame.contains("Align(16)"));
        assert!(frame.contains("emitSPAdjust"));
        assert!(frame.contains("LNP64::R30"));
        assert!(frame.contains("TII.get(Amount < 0 ? LNP64::SUB : LNP64::ADD)"));
        assert!(reginfo.contains("Reserved.set(LNP64::R0)"));
        assert!(reginfo.contains("Reserved.set(LNP64::R30)"));
        assert!(reginfo.contains("eliminateFrameIndex"));
        assert!(reginfo.contains("void LNP64RegisterInfo::eliminateFrameIndex"));
        assert!(reginfo.contains("ChangeToRegister(LNP64::R31"));
        assert!(reginfo.contains("LNP64::PseudoFRAMEADDR"));
        assert!(reginfo.contains("LNP64 frame address offset exceeds signed-16 LI range"));
        assert!(reginfo.contains("TII.get(LNP64::ADD)"));
        assert!(reginfo.contains("MFI.getObjectOffset"));
        assert!(reginfo.contains("isInt<14>(Offset)"));
        assert!(reginfo.contains("NoCalleeSaved"));
        assert!(clang_target.contains("resetDataLayout(\"e-m:e-p:64:64-i64:64-n64-S128\")"));
        assert!(clang_target.contains("__LNP64__"));
        assert!(clang_target_header.contains("getTargetBuiltins()"));
        assert!(clang_target_header.contains("isValidCPUName(StringRef Name)"));
        assert!(clang_target.contains("Name == \"generic-lnp64\""));
        assert!(clang_target_header.contains("setCPU(const std::string &Name)"));
        assert!(clang_target_header.contains("hasFeature(StringRef Feature)"));
        assert!(clang_target.contains("const char *LNP64TargetInfo::getClobbers() const"));
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
        assert!(lld_arch.contains("copyRel = R_LNP64_NONE"));
        assert!(lld_arch.contains("relativeRel = R_LNP64_RELATIVE"));
        assert!(lld_arch.contains("switch (Rel.type)"));
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
        assert!(codegen_test.contains("define i64 @invert"));
        assert!(codegen_test.contains("define i64 @control"));
        assert!(codegen_test.contains("define i64 @gate"));
        assert!(codegen_test.contains("define i64 @read_stream"));
        assert!(codegen_test.contains("define i64 @wait_ready"));
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
        assert!(codegen_test.contains("; CHECK: not"));
        assert!(codegen_test.contains("; CHECK: ret"));
        assert!(codegen_test.contains("__lnp_call"));
        assert!(codegen_test.contains("__lnp_domain_ctl"));
        assert!(codegen_test.contains("__lnp_object_ctl"));
        assert!(codegen_test.contains("__lnp_await"));
        assert!(codegen_test.contains("__lnp_pull"));
        assert!(codegen_test.contains("__lnp_push"));
        assert!(codegen_test.contains("__lnp_gate_return"));
        assert!(codegen_test.contains("; CHECK: domain_ctl"));
        assert!(codegen_test.contains("; CHECK: gate_call"));
        assert!(codegen_test.contains("; CHECK: gate_return"));
        assert!(codegen_test.contains("; CHECK: object_ctl"));
        assert!(codegen_test.contains("; CHECK: await"));
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
        let run_elf_manifest = include_str!("../toolchain/lnp64_run_elf.manifest");
        let real_llc = include_str!("../scripts/run_real_llvm_lnp64.sh");
        let real_llc_docker = include_str!("../scripts/run_real_llvm_lnp64_docker.sh");
        let c_compiler = include_str!("c_compiler.rs");
        let emulator = include_str!("emulator.rs");
        let evidence_corpus = format!(
            "{conformance}\n{run_elf_manifest}\n{real_llc}\n{real_llc_docker}\n{c_compiler}\n{emulator}"
        );
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
            "string_ctype",
            "numeric_conversion",
            "process_identity",
            "random_state",
            "path_helpers",
            "search_helpers",
            "sort_helpers",
            "time_clock",
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
            "string_ctype",
            "numeric_conversion",
            "process_identity",
            "random_state",
            "path_helpers",
            "search_helpers",
            "sort_helpers",
            "time_clock",
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
                vec![
                    "_start",
                    "argv",
                    "envp",
                    "environ",
                    "getenv",
                    "setenv",
                    "unsetenv",
                    "clearenv",
                    "putenv",
                    "getauxval",
                ],
                vec!["crt0", "TLS", "ENV_GET", "EXIT"],
            ),
            (
                "errno_tls",
                vec!["errno", "__errno_location", "strerror"],
                vec!["TLS", "ERRNO_SET", "completion_helpers"],
            ),
            (
                "string_ctype",
                vec!["strlen", "strcmp", "memcpy", "isalpha", "tolower"],
                vec!["load_store", "integer_alu", "static_link"],
            ),
            (
                "numeric_conversion",
                vec!["atoi", "strtol", "strtoull"],
                vec!["integer_alu", "ERRNO_SET", "static_link"],
            ),
            (
                "process_identity",
                vec![
                    "pid", "getpid", "getppid", "getuid", "geteuid", "getgid", "getegid",
                ],
                vec!["GET_PCR"],
            ),
            (
                "random_state",
                vec!["random", "srandom", "initstate", "setstate"],
                vec!["integer_alu", "load_store", "static_link"],
            ),
            (
                "path_helpers",
                vec!["basename", "dirname"],
                vec!["load_store", "integer_alu", "static_link"],
            ),
            (
                "search_helpers",
                vec!["lfind", "lsearch", "insque", "remque"],
                vec!["load_store", "integer_alu", "static_link"],
            ),
            (
                "sort_helpers",
                vec!["qsort"],
                vec!["load_store", "integer_alu", "static_link"],
            ),
            (
                "time_clock",
                vec![
                    "clock_gettime",
                    "time",
                    "usleep",
                    "sleep",
                    "timerfd_create",
                    "timerfd_settime",
                    "timerfd_gettime",
                ],
                vec![
                    "GET_PCR",
                    "REALTIME_SEC",
                    "REALTIME_NSEC",
                    "YIELD",
                    "OBJECT_CTL",
                    "PUSH",
                    "AWAIT",
                    "PULL",
                    "errno_tls",
                ],
            ),
            (
                "fd_io",
                vec![
                    "openat", "read", "write", "fcntl", "stat", "fstat", "futimens", "stdio",
                ],
                vec![
                    "__lnp_openat",
                    "__lnp_pull",
                    "__lnp_push",
                    "CAP_DUP",
                    "FDR",
                    "GET_META",
                    "SET_META",
                    "FD_SEEK_DYN",
                ],
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
        let mut statuses = std::collections::BTreeMap::new();

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
            statuses.insert(case, status);
            assert!(
                manifest_root.join(source).exists(),
                "llvm bootstrap case {case} names missing source/gate {source}"
            );
            assert!(
                ["tested", "partial", "planned"].contains(&status),
                "unknown LLVM bootstrap status {status} for {case}"
            );
            if status == "tested" {
                assert!(
                    backend_contracts.contains(&"static_link"),
                    "tested case {case} must require static linking"
                );
            }
            assert!(
                !runtime_contracts.is_empty(),
                "case {case} must name runtime expectations"
            );
        }

        for case in [
            "hello",
            "arithmetic",
            "memory",
            "calls",
            "pcr",
            "cat",
            "json_parser",
            "rot13",
            "producer_consumer",
            "parallel_hash",
            "sqlite_lite",
            "ping_pong",
            "zlib_checksum",
            "natsort",
            "jsmn",
            "inih_parse_string",
            "cwalk",
            "sbase_commands",
            "userland_ucat",
            "userland_init",
            "userland_lnpsh",
            "userland_spawn_task",
            "netbsd_init_root",
            "netbsd_shell_root",
            "netbsd_loader_target_child",
            "netbsd_elf_exec_parent",
            "netbsd_fork_wait_child",
            "netbsd_thread_child",
            "netbsd_poll_child",
            "netbsd_signal_gate_child",
            "netbsd_signal_fault_child",
            "netbsd_timer_child",
            "netbsd_mmap_child",
            "netbsd_fd_passing_child",
            "netbsd_namespace_child",
            "netbsd_fs_service_child",
            "netbsd_classifier_child",
            "netbsd_socket_loopback_child",
            "netbsd_gate_trace_child",
            "netbsd_domain_nested_child",
            "netbsd_domain_budget_child",
            "netbsd_personality_clang",
            "netcat",
            "httpd",
            "simple_libc",
        ] {
            assert!(cases.contains(case), "missing llvm bootstrap case {case}");
        }
        for case in [
            "hello",
            "arithmetic",
            "memory",
            "calls",
            "pcr",
            "cat",
            "json_parser",
            "rot13",
            "producer_consumer",
            "parallel_hash",
            "sqlite_lite",
            "ping_pong",
            "zlib_checksum",
            "natsort",
            "jsmn",
            "inih_parse_string",
            "cwalk",
        ] {
            assert_eq!(statuses[case], "tested", "{case} should be tested");
        }
        assert_eq!(statuses["sbase_commands"], "partial");
        assert_eq!(statuses["userland_ucat"], "partial");
        assert_eq!(statuses["userland_init"], "partial");
        assert_eq!(statuses["userland_lnpsh"], "partial");
        assert_eq!(statuses["userland_spawn_task"], "partial");
        assert_eq!(statuses["netbsd_init_root"], "partial");
        assert_eq!(statuses["netbsd_shell_root"], "partial");
        assert_eq!(statuses["netbsd_loader_target_child"], "partial");
        assert_eq!(statuses["netbsd_elf_exec_parent"], "partial");
        assert_eq!(statuses["netbsd_fork_wait_child"], "partial");
        assert_eq!(statuses["netbsd_thread_child"], "partial");
        assert_eq!(statuses["netbsd_poll_child"], "partial");
        assert_eq!(statuses["netbsd_signal_gate_child"], "partial");
        assert_eq!(statuses["netbsd_signal_fault_child"], "partial");
        assert_eq!(statuses["netbsd_timer_child"], "partial");
        assert_eq!(statuses["netbsd_mmap_child"], "partial");
        assert_eq!(statuses["netbsd_fd_passing_child"], "partial");
        assert_eq!(statuses["netbsd_namespace_child"], "partial");
        assert_eq!(statuses["netbsd_fs_service_child"], "partial");
        assert_eq!(statuses["netbsd_classifier_child"], "partial");
        assert_eq!(statuses["netbsd_socket_loopback_child"], "partial");
        assert_eq!(statuses["netbsd_gate_trace_child"], "partial");
        assert_eq!(statuses["netbsd_domain_nested_child"], "partial");
        assert_eq!(statuses["netbsd_domain_budget_child"], "partial");
        assert_eq!(statuses["netbsd_personality_clang"], "partial");
        assert_eq!(statuses["netcat"], "partial");
        assert_eq!(statuses["httpd"], "partial");
        assert_eq!(statuses["simple_libc"], "partial");
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
            ".globl _start",
            ".type _start,@function",
            "LI r7, 0x7000",
            "LI r8, 0x100",
            "MUL r7, r7, r8",
            "LD r1, 0(r7)",
            "LI r2, 8",
            "ADD r2, r7, r2",
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
    fn minilibc_smoke_stub_matches_real_llvm_gate() {
        let minilibc = include_str!("../toolchain/liblnp64_min.s");
        let real_llc = include_str!("../scripts/run_real_llvm_lnp64.sh");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");

        assert!(contract_index.contains(
            "minilibc_smoke|toolchain/liblnp64_min.s|minilibc_smoke_stub_matches_real_llvm_gate"
        ));
        for required in [
            ".globl write",
            "write:",
            "PUSH r1, r1, r2, r3",
            ".globl read",
            "read:",
            "PULL r1, r1, r2, r3",
            ".globl alloc",
            "alloc:",
            "ALLOC r1, r1",
            ".globl malloc",
            "malloc:",
            "CALL alloc",
            ".globl calloc",
            "calloc:",
            "CALL memset",
            ".globl realloc",
            "realloc:",
            "ALLOC_SIZE r3, r2",
            "CALL memcpy",
            ".globl free",
            "free:",
            "FREE r1",
            "LI r1, 0",
            ".globl strlen",
            "strlen:",
            "LD.B r3, 0(r2)",
            ".globl memcpy",
            "memcpy:",
            "ST.B r7, 0(r1)",
            ".globl memmove",
            "memmove:",
            "memmove_backward_loop:",
            "CALL memcpy",
            ".globl memcmp",
            "memcmp:",
            "memcmp_diff:",
            ".globl memset",
            "memset:",
            "ST.B r2, 0(r1)",
            ".globl _exit",
            "_exit:",
            "EXIT r1",
            ".globl exit",
            "exit:",
            "__lnp64_min_realloc_old:",
            "__lnp64_min_realloc_size:",
            "__lnp64_min_realloc_new:",
        ] {
            assert!(minilibc.contains(required), "minilibc missing {required}");
        }
        assert!(real_llc.contains("toolchain/liblnp64_min.s"));
        assert!(real_llc.contains("liblnp64-min-smoke.o"));
        assert!(real_llc.contains("lnp64-$demo-clang-linked.elf"));
        assert!(real_llc.contains("real LLVM LNP64 lld clang demo link smoke passed"));
        assert!(roadmap.contains("toolchain/liblnp64_min.s"));
        assert!(!minilibc.contains("lnp64 cc"));
        assert!(!minilibc.contains("cargo run -- cc"));
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
        let mut phases = std::collections::BTreeMap::new();

        for (phase, status, artifacts, gate) in rows {
            assert!(
                phases
                    .insert(phase, (status, artifacts.clone(), gate))
                    .is_none(),
                "duplicate transition phase {phase}"
            );
            assert!(
                ["required", "partial", "planned"].contains(&status),
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
            assert!(
                phases.contains_key(phase),
                "missing transition phase {phase}"
            );
        }

        assert_eq!(phases["real_toolchain_target"].0, "required");
        for artifact in [
            "toolchain/lnp64_target.manifest",
            "toolchain/lnp64_registers.manifest",
            "toolchain/lnp64_psabi.manifest",
            "toolchain/lnp64_relocations.manifest",
            "toolchain/lnp64_mc_encoding.manifest",
            "toolchain/lnp64_inline_asm.manifest",
            "toolchain/lnp64_debug_unwind.manifest",
            "toolchain/lnp64_crt_startup.manifest",
            "toolchain/crt0_lnp64.s",
            "toolchain/lnp64_intrinsics.manifest",
            "toolchain/lnp64_intrinsic_lowering.manifest",
            "toolchain/lnp64_intrinsics.h",
            "toolchain/lnp64_isel.manifest",
            "toolchain/lnp64_exec_plan.manifest",
            "toolchain/lnp64_clang_driver.manifest",
            "toolchain/lnp64_static.ld",
            "toolchain/lnp64_run_elf.manifest",
            "psABI.md",
            "object_format.md",
        ] {
            assert!(
                phases["real_toolchain_target"].1.contains(&artifact),
                "real_toolchain_target is missing artifact {artifact}"
            );
        }

        assert!(roadmap.contains("## Toy Compiler Freeze Policy"));
        assert!(roadmap.contains("## First Acceptance Gates"));
        assert!(roadmap.contains("## Checked Transition Deliverables"));
        assert!(roadmap.contains("`minimal_llvm_clang_path` row is now partial"));
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
        let run_real_package_gate = include_str!("../scripts/run_real_llvm_package_gate.sh");
        let run_demos = include_str!("../scripts/run_demos.sh");
        let run_netbsd_smoke = include_str!("../scripts/run_netbsd_personality_smoke.sh");
        let run_netbsd_system = include_str!("../scripts/run_netbsd_personality_system.sh");
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
                ["tested", "partial", "planned"].contains(&status),
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
            "llvm_package_tests",
            "netbsd_personality",
            "llvm_built_versions",
            "aggregate_hygiene",
        ] {
            assert!(
                categories.contains_key(category),
                "missing conformance category {category}"
            );
        }
        assert_eq!(categories["llvm_built_versions"].0, "partial");
        assert!(
            categories["llvm_built_versions"]
                .1
                .contains(&"scripts/run_llvm_bootstrap_gates.sh")
        );
        assert!(
            categories["llvm_built_versions"]
                .1
                .contains(&"scripts/run_real_llvm_lnp64_objects_docker.sh")
        );
        assert_eq!(
            categories["llvm_built_versions"].2,
            "scripts/run_real_llvm_lnp64_docker.sh"
        );
        assert!(
            categories["llvm_built_versions"]
                .3
                .contains("real_clang_object_gate")
        );
        assert_eq!(
            categories["llvm_package_tests"].2,
            "scripts/run_real_llvm_lnp64_docker.sh"
        );
        assert!(categories["llvm_package_tests"].3.contains("zlib"));
        assert!(categories["llvm_package_tests"].3.contains("cwalk"));
        assert!(
            categories["llvm_package_tests"]
                .3
                .contains("sbase_commands")
        );
        for category in [
            "asm_demos",
            "c_tests",
            "randomized_emulator",
            "adversarial_fault",
            "package_tests",
            "llvm_package_tests",
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
        assert!(!run_software.contains("bash scripts/run_netbsd_personality_smoke.sh"));
        assert!(run_software.contains("bash scripts/run_real_packages.sh"));
        assert!(run_all.contains("bash scripts/run_software_gates.sh"));
        assert!(run_all.contains("git diff --check"));
        assert!(run_real_packages.contains("scripts/run_real_llvm_package_gate.sh"));
        assert!(run_real_packages.contains("real LLVM LNP64 package gate"));
        assert!(run_real_package_gate.contains("netbsd)"));
        assert!(run_real_package_gate.contains("lnp64-netbsd-init-linked.elf"));
        assert!(run_real_package_gate.contains("lnp64-netbsd-sh-linked.elf"));
        assert!(run_real_package_gate.contains("netbsd-system-fixture-root"));
        assert!(run_real_package_gate.contains("netbsd personality system ok"));
        assert!(
            run_real_package_gate
                .contains("real LLVM LNP64 run-elf NetBSD init/shell system passed")
        );
        assert!(
            run_real_package_gate
                .contains("for selected in zlib natsort jsmn inih cwalk sbase userland netbsd")
        );
        assert!(!run_real_packages.contains("cc --toy-bootstrap"));
        assert!(run_demos.contains("scripts/run_real_llvm_lnp64_docker.sh"));
        assert!(run_demos.contains("demos/netbsd_personality_smoke.c"));
        assert!(run_demos.contains("--legacy-toy"));
        assert!(run_demos.contains("include_legacy_toy=0"));
        assert!(run_demos.contains("if [[ \"$include_legacy_toy\" == \"1\" ]]"));
        assert!(run_demos.contains("for src in demos/*.s"));
        assert!(run_netbsd_smoke.contains("mode=\"llvm\""));
        assert!(run_netbsd_smoke.contains("LNP64_LLVM_PACKAGE_FILTER=netbsd"));
        assert!(run_netbsd_smoke.contains("scripts/run_real_llvm_package_gate.sh"));
        assert!(run_netbsd_smoke.contains("--legacy-toy"));
        assert!(run_netbsd_system.contains("LNP64_LLVM_PACKAGE_FILTER=netbsd"));
        assert!(run_netbsd_system.contains("scripts/run_real_llvm_package_gate.sh"));
        assert!(run_netbsd_system.contains("--legacy-toy"));
        assert_eq!(
            categories["netbsd_personality"].3,
            "real_clang_netbsd_child_elf_gate_with_legacy_toy_smoke_system_opt_in"
        );
        assert_eq!(
            categories["asm_demos"].3,
            "assembly_demo_smoke_path_with_legacy_toy_c_opt_in"
        );
        assert!(categories["c_tests"].3.contains("default_to_real_clang"));
        for migrated_demo in [
            "demos/allocator.c",
            "demos/cat.c",
            "demos/factorial.c",
            "demos/fibonacci.c",
            "demos/hello.c",
            "demos/json_parser.c",
            "demos/netcat.c",
            "demos/httpd.c",
            "demos/parallel_hash.c",
            "demos/pcr.c",
            "demos/ping_pong.c",
            "demos/producer_consumer.c",
            "demos/rot13.c",
            "demos/sqlite_lite.c",
        ] {
            assert!(
                !run_demos.contains(migrated_demo),
                "migrated real-Clang demo {migrated_demo} must not be routed through run_demos.sh"
            );
        }
    }

    #[test]
    fn toy_compiler_policy_manifest_freezes_bootstrap_role() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let policy_manifest = include_str!("../toolchain/lnp64_toy_compiler_policy.manifest");
        let retirement_queue = include_str!("../toolchain/lnp64_toy_retirement_queue.manifest");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let conformance = include_str!("../conformance_matrix.md");
        let llvm_gates = include_str!("../toolchain/lnp64_llvm_gates.manifest");
        let llvm_bootstrap = include_str!("../toolchain/lnp64_llvm_bootstrap.manifest");
        let run_elf = include_str!("../toolchain/lnp64_run_elf.manifest");
        let libc_test_readme = include_str!("../third_party/libc-test/README.lnp64.md");
        let intrinsics = include_str!("../toolchain/lnp64_intrinsics.manifest");
        let intrinsic_header = include_str!("../toolchain/lnp64_intrinsics.h");
        let main_source = include_str!("main.rs");
        let rtl_top_manifest_checker =
            include_str!("../scripts/check_rtl_top_level_program_manifest.py");
        let legacy_toy_scripts = [
            (
                "scripts/run_cwalk.sh",
                include_str!("../scripts/run_cwalk.sh"),
            ),
            (
                "scripts/run_demos.sh",
                include_str!("../scripts/run_demos.sh"),
            ),
            (
                "scripts/run_inih.sh",
                include_str!("../scripts/run_inih.sh"),
            ),
            (
                "scripts/run_jsmn.sh",
                include_str!("../scripts/run_jsmn.sh"),
            ),
            (
                "scripts/run_libc_test.sh",
                include_str!("../scripts/run_libc_test.sh"),
            ),
            (
                "scripts/run_natsort.sh",
                include_str!("../scripts/run_natsort.sh"),
            ),
            (
                "scripts/run_netbsd_personality_smoke.sh",
                include_str!("../scripts/run_netbsd_personality_smoke.sh"),
            ),
            (
                "scripts/run_netbsd_personality_system.sh",
                include_str!("../scripts/run_netbsd_personality_system.sh"),
            ),
            (
                "scripts/run_rtl_top_program_smoke.sh",
                include_str!("../scripts/run_rtl_top_program_smoke.sh"),
            ),
            (
                "scripts/run_rtl_top_toy_c_smoke.sh",
                include_str!("../scripts/run_rtl_top_toy_c_smoke.sh"),
            ),
            (
                "scripts/run_sbase.sh",
                include_str!("../scripts/run_sbase.sh"),
            ),
            (
                "scripts/run_userland.sh",
                include_str!("../scripts/run_userland.sh"),
            ),
            (
                "scripts/run_zlib.sh",
                include_str!("../scripts/run_zlib.sh"),
            ),
        ];
        let c_compiler = include_str!("c_compiler.rs");
        let lowering_source = include_str!("lowering.rs");
        let libc_roadmap = include_str!("../libc_roadmap.md");
        let legacy_toy_script_corpus = legacy_toy_scripts
            .iter()
            .map(|(_, script)| *script)
            .collect::<Vec<_>>()
            .join("\n");
        let evidence_corpus = format!(
            "{target_manifest}\n{roadmap}\n{conformance}\n{llvm_gates}\n{llvm_bootstrap}\n{run_elf}\n{libc_test_readme}\n{retirement_queue}\n{intrinsics}\n{intrinsic_header}\n{main_source}\n{legacy_toy_script_corpus}\n{c_compiler}\n{lowering_source}\n{libc_roadmap}"
        );
        let rows = toy_compiler_policy_rows(policy_manifest);
        let queue_rows = toy_retirement_queue_rows(retirement_queue);
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut rules = std::collections::BTreeMap::new();
        let mut queued_surfaces = std::collections::BTreeMap::new();

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
        assert!(contract_index.contains(
            "toy_retirement_queue|toolchain/lnp64_toy_retirement_queue.manifest|toy_retirement_queue_manifest_records_remaining_surfaces"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_toy_compiler_policy.manifest"));
        assert!(transition_manifest.contains("toolchain/lnp64_toy_retirement_queue.manifest"));
        assert!(roadmap.contains("only small fixes needed to keep existing smoke"));
        assert!(roadmap.contains("toolchain/lnp64_toy_retirement_queue.manifest"));
        assert!(conformance.contains("toolchain/lnp64_toy_compiler_policy.manifest"));

        for (rule, status, artifacts, evidence) in rows {
            assert!(
                rules
                    .insert(rule, (status, artifacts.clone(), evidence))
                    .is_none(),
                "duplicate toy compiler policy rule {rule}"
            );
            assert!(
                ["required", "partial", "planned", "blocked"].contains(&status),
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
            if status == "required" || status == "partial" || status == "blocked" {
                assert!(
                    evidence_corpus.contains(evidence),
                    "toy compiler policy evidence {evidence} for {rule} is not present"
                );
            }
        }

        for rule in [
            "smoke_generator_only",
            "explicit_legacy_cc_flag",
            "private_native_shims",
            "compat_lowering_boundary",
            "no_toy_in_llvm_gates",
            "replacement_program_set",
            "clang_libc_replacements",
            "remaining_toy_only_libc",
            "remaining_toy_queue",
        ] {
            assert!(
                rules.contains_key(rule),
                "missing toy compiler policy rule {rule}"
            );
        }
        for rule in [
            "smoke_generator_only",
            "explicit_legacy_cc_flag",
            "private_native_shims",
            "compat_lowering_boundary",
            "no_toy_in_llvm_gates",
        ] {
            assert_eq!(rules[rule].0, "required", "{rule} should be required");
        }
        assert_eq!(rules["replacement_program_set"].0, "partial");
        assert_eq!(rules["clang_libc_replacements"].0, "partial");
        assert_eq!(rules["remaining_toy_only_libc"].0, "blocked");
        assert_eq!(rules["remaining_toy_queue"].0, "blocked");
        let explicit_legacy_artifacts = &rules["explicit_legacy_cc_flag"].1;
        for (script_name, script) in legacy_toy_scripts {
            if script.contains("cc --toy-bootstrap") {
                assert!(
                    explicit_legacy_artifacts.contains(&script_name),
                    "{script_name} contains a toy compiler invocation but is not covered by explicit_legacy_cc_flag"
                );
            }
        }
        for (surface, status, toy_artifacts, replacement_target, blocker) in queue_rows {
            assert!(
                queued_surfaces
                    .insert(surface, (status, replacement_target, blocker))
                    .is_none(),
                "duplicate toy retirement surface {surface}"
            );
            assert!(
                ["partial", "blocked"].contains(&status),
                "unknown toy retirement status {status} for {surface}"
            );
            assert!(
                !toy_artifacts.is_empty(),
                "empty toy retirement artifact list for {surface}"
            );
            for artifact in toy_artifacts {
                assert!(
                    manifest_root.join(artifact).exists(),
                    "toy retirement surface {surface} names missing artifact {artifact}"
                );
            }
            assert!(
                replacement_target.contains("Clang")
                    || replacement_target.contains("scripts/run_real_llvm")
                    || replacement_target.contains("scripts/run_rtl_top_clang")
                    || replacement_target.contains("scripts/run_rtl_top_linked_llvm"),
                "toy retirement surface {surface} lacks a real-toolchain replacement target"
            );
            assert!(
                !blocker.is_empty(),
                "toy retirement surface {surface} lacks a blocker"
            );
        }
        for (surface, expected_status) in [
            ("legacy_demo_smoke", "blocked"),
            ("minimal_userland_image", "partial"),
            ("netbsd_personality_system", "partial"),
            ("legacy_libc_test_backend", "partial"),
            ("rtl_c_program_smoke", "partial"),
        ] {
            assert_eq!(
                queued_surfaces.get(surface).map(|row| row.0),
                Some(expected_status),
                "missing or wrong toy retirement queue status for {surface}"
            );
        }
        assert!(run_elf.contains("real_libc_test_pthread_tsd_execution"));
        assert!(run_elf.contains("real_libc_test_sem_init_execution"));
        assert!(run_elf.contains("real_libc_test_access_bounded_execution"));
        assert!(run_elf.contains("real_libc_test_fcntl_basic_bounded_execution"));
        assert!(!run_elf.contains("real_libc_test_fcntl_execution"));
        assert!(libc_test_readme.contains("Do not add new Rust toy compiler fcntl/fork"));
        assert!(
            libc_test_readme.contains("bash scripts/run_libc_test.sh --backend toy --loader asm")
        );
        for intrinsic in manifest_field(target_manifest, "intrinsics").split(',') {
            assert!(intrinsic.starts_with("__lnp_"));
            assert!(intrinsics.contains(intrinsic));
            assert!(intrinsic_header.contains(intrinsic));
        }
        assert!(main_source.contains("deprecated Rust bootstrap C compiler"));
        assert!(main_source.contains("cc --toy-bootstrap"));
        for (script_name, script) in legacy_toy_scripts {
            for (idx, line) in script.lines().enumerate() {
                if line.contains(" cc ") || line.contains(" -- cc ") {
                    assert!(
                        line.contains("--toy-bootstrap"),
                        "{script_name}:{} invokes the legacy C compiler without --toy-bootstrap: {line}",
                        idx + 1
                    );
                }
            }
        }
        assert!(!llvm_gates.contains("lnp64 cc"));
        assert!(!llvm_gates.contains("cargo run -- cc"));
        for (script_name, script) in legacy_toy_scripts {
            if matches!(
                script_name,
                "scripts/run_cwalk.sh"
                    | "scripts/run_inih.sh"
                    | "scripts/run_jsmn.sh"
                    | "scripts/run_natsort.sh"
                    | "scripts/run_sbase.sh"
                    | "scripts/run_zlib.sh"
            ) {
                assert!(
                    script.contains("scripts/run_real_llvm_package_gate.sh"),
                    "{script_name} should route package coverage through real LLVM"
                );
                let package_name = script_name
                    .strip_prefix("scripts/run_")
                    .and_then(|name| name.strip_suffix(".sh"))
                    .expect("legacy package script name shape");
                assert!(
                    script.contains(&format!("LNP64_LLVM_PACKAGE_FILTER={package_name}")),
                    "{script_name} should run only its own package subset"
                );
                assert!(
                    !script.contains("cc --toy-bootstrap"),
                    "{script_name} must not invoke the toy compiler"
                );
            }
        }
        assert!(legacy_toy_script_corpus.contains("--legacy-toy"));
        assert!(legacy_toy_script_corpus.contains("LNP64_LLVM_PACKAGE_FILTER=userland"));
        assert!(legacy_toy_script_corpus.contains("LNP64_LLVM_PACKAGE_FILTER=netbsd"));
        assert!(legacy_toy_script_corpus.contains("scripts/run_real_llvm_package_gate.sh"));
        assert!(legacy_toy_script_corpus.contains("include_legacy_toy=0"));
        assert!(rtl_top_manifest_checker.contains("toolchain/lnp64_llvm_bootstrap.manifest"));
        assert!(!rtl_top_manifest_checker.contains("RUN_DEMOS"));
        assert!(!rtl_top_manifest_checker.contains("non_network"));
        for case in [
            "hello",
            "arithmetic",
            "memory",
            "calls",
            "pcr",
            "cat",
            "json_parser",
            "rot13",
            "producer_consumer",
            "parallel_hash",
            "sqlite_lite",
            "ping_pong",
            "netcat",
            "httpd",
            "userland_ucat",
            "userland_init",
            "userland_lnpsh",
            "userland_spawn_task",
            "netbsd_init_root",
            "netbsd_shell_root",
            "netbsd_loader_target_child",
            "netbsd_fork_wait_child",
            "netbsd_thread_child",
            "netbsd_poll_child",
            "netbsd_signal_gate_child",
            "netbsd_signal_fault_child",
            "netbsd_timer_child",
            "netbsd_mmap_child",
            "netbsd_fd_passing_child",
            "netbsd_namespace_child",
            "netbsd_fs_service_child",
            "netbsd_classifier_child",
            "netbsd_socket_loopback_child",
            "netbsd_gate_trace_child",
            "netbsd_domain_nested_child",
            "netbsd_domain_budget_child",
            "netbsd_personality_clang",
            "simple_libc",
        ] {
            assert!(
                llvm_bootstrap.contains(case),
                "replacement program set missing {case}"
            );
        }
    }

    #[test]
    fn toy_retirement_queue_manifest_records_remaining_surfaces() {
        let queue_manifest = include_str!("../toolchain/lnp64_toy_retirement_queue.manifest");
        let policy_manifest = include_str!("../toolchain/lnp64_toy_compiler_policy.manifest");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut queued_surfaces = std::collections::BTreeMap::new();

        assert!(policy_manifest.contains("remaining_toy_queue"));
        assert!(roadmap.contains("toy-compiler retirement queue"));
        assert!(contract_index.contains(
            "toy_retirement_queue|toolchain/lnp64_toy_retirement_queue.manifest|toy_retirement_queue_manifest_records_remaining_surfaces"
        ));

        for (surface, status, toy_artifacts, replacement_target, blocker) in
            toy_retirement_queue_rows(queue_manifest)
        {
            assert!(
                queued_surfaces
                    .insert(surface, (status, replacement_target, blocker))
                    .is_none(),
                "duplicate toy retirement surface {surface}"
            );
            assert!(
                ["partial", "blocked"].contains(&status),
                "unknown toy retirement status {status} for {surface}"
            );
            assert!(
                !toy_artifacts.is_empty(),
                "empty toy retirement artifact list for {surface}"
            );
            for artifact in toy_artifacts {
                assert!(
                    manifest_root.join(artifact).exists(),
                    "toy retirement surface {surface} names missing artifact {artifact}"
                );
            }
            assert!(
                replacement_target.contains("Clang")
                    || replacement_target.contains("scripts/run_real_llvm")
                    || replacement_target.contains("scripts/run_rtl_top_clang")
                    || replacement_target.contains("scripts/run_rtl_top_linked_llvm"),
                "toy retirement surface {surface} lacks a real-toolchain replacement target"
            );
            assert!(
                !blocker.is_empty(),
                "toy retirement surface {surface} lacks a blocker"
            );
        }

        for (surface, expected_status) in [
            ("legacy_demo_smoke", "blocked"),
            ("minimal_userland_image", "partial"),
            ("netbsd_personality_system", "partial"),
            ("legacy_libc_test_backend", "partial"),
            ("rtl_c_program_smoke", "partial"),
        ] {
            assert_eq!(
                queued_surfaces.get(surface).map(|row| row.0),
                Some(expected_status),
                "missing or wrong toy retirement queue status for {surface}"
            );
        }
    }

    #[test]
    fn rtl_toy_c_smokes_have_linked_llvm_replacement_coverage() {
        let manifest = include_str!("../tests/rtl/top_level_program_manifest.json");
        let linked_gate = "\"rtl_gate\": \"scripts/run_rtl_top_linked_llvm_smoke.sh\"";

        let entry_for = |source: &str| {
            let source_marker = format!("\"source\": \"{source}\"");
            let source_idx = manifest
                .find(&source_marker)
                .unwrap_or_else(|| panic!("missing RTL top-level manifest source {source}"));
            let entry_start = manifest[..source_idx]
                .rfind("    {")
                .unwrap_or_else(|| panic!("missing manifest entry start for {source}"));
            let entry_end = manifest[source_idx..]
                .find("\n    }")
                .map(|offset| source_idx + offset)
                .unwrap_or_else(|| panic!("missing manifest entry end for {source}"));
            &manifest[entry_start..entry_end]
        };

        for toy_source in [
            "tests/rtl/programs/top_return_12.c",
            "tests/rtl/programs/top_branch_if.c",
            "tests/rtl/programs/top_loop_sum.c",
            "tests/rtl/programs/top_call_return.c",
            "tests/rtl/programs/top_subtract.c",
            "tests/rtl/programs/top_bitwise.c",
            "tests/rtl/programs/top_shift.c",
            "tests/rtl/programs/top_not.c",
            "tests/rtl/programs/top_factorial_mul.c",
            "tests/rtl/programs/top_udiv_urem.c",
            "tests/rtl/programs/top_signed_division.c",
            "tests/rtl/programs/top_byte_array.c",
            "tests/rtl/programs/top_heap_byte_lanes.c",
            "demos/allocator.c",
            "demos/hello.c",
            "demos/factorial.c",
            "demos/fibonacci.c",
            "demos/json_parser.c",
            "demos/ping_pong.c",
            "demos/rot13.c",
        ] {
            let toy_entry = entry_for(toy_source);
            assert!(
                toy_entry.contains("\"rtl_gate\": \"scripts/run_rtl_top_toy_c_smoke.sh\""),
                "{toy_source} should remain an explicit legacy toy-C smoke while covered by linked LLVM"
            );
        }

        for (linked_source, feature) in [
            ("tests/rtl/programs/top_linked_main.c", "startup_call_main"),
            ("tests/rtl/programs/top_linked_loop_branch.c", "branch"),
            ("tests/rtl/programs/top_linked_loop_branch.c", "call_return"),
            (
                "tests/rtl/programs/top_linked_bitwise_shift.c",
                "bitwise_alu",
            ),
            ("tests/rtl/programs/top_linked_bitwise_shift.c", "shift_alu"),
            ("tests/rtl/programs/top_linked_factorial_mul.c", "mul"),
            (
                "tests/rtl/programs/top_linked_factorial_native.c",
                "push_pull",
            ),
            (
                "tests/rtl/programs/top_linked_fibonacci_native.c",
                "call_return",
            ),
            (
                "tests/rtl/programs/top_linked_divrem.c",
                "unsigned_division",
            ),
            ("tests/rtl/programs/top_linked_divrem.c", "signed_division"),
            (
                "tests/rtl/programs/top_linked_byte_array.c",
                "byte_load_store",
            ),
            ("tests/rtl/programs/top_linked_heap_byte_lanes.c", "heap"),
            ("tests/rtl/programs/top_linked_allocator_native.c", "heap"),
            ("tests/rtl/programs/top_linked_allocator_native.c", "free"),
            ("tests/rtl/programs/top_linked_json_parser_native.c", "heap"),
            ("tests/rtl/programs/top_linked_json_parser_native.c", "free"),
            ("tests/rtl/programs/top_linked_clone_join.c", "thread_join"),
            ("tests/rtl/programs/top_linked_hello_native.c", "push_pull"),
            ("tests/rtl/programs/top_linked_rot13_native.c", "push_pull"),
            ("tests/rtl/programs/top_linked_rot13_native.c", "free"),
        ] {
            let linked_entry = entry_for(linked_source);
            assert!(
                linked_entry.contains(linked_gate),
                "{linked_source} must use the linked LLVM RTL smoke gate"
            );
            assert!(
                linked_entry.contains("\"status\": \"active\""),
                "{linked_source} should be active replacement coverage"
            );
            assert!(
                linked_entry.contains(feature),
                "{linked_source} must advertise replacement feature {feature}"
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
            manifest_field(manifest, "intrinsic_lowering_contract"),
            "toolchain/lnp64_intrinsic_lowering.manifest"
        );
        assert_eq!(
            manifest_field(manifest, "intrinsic_header_contract"),
            "toolchain/lnp64_intrinsics.h"
        );
        assert_eq!(
            manifest_field(manifest, "target_intrinsic_header_contract"),
            "toolchain/include/lnp64/intrinsics.h"
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
        for pcr in [
            "PID",
            "PPID",
            "TID",
            "TP",
            "UID",
            "GID",
            "SIGMASK",
            "SIGPENDING",
            "REALTIME_SEC",
            "REALTIME_NSEC",
            "CRED_PROFILE",
            "CRED_HANDLE",
        ] {
            assert!(manifest_csv_contains(manifest, "pcr", pcr), "missing {pcr}");
        }
        assert!(manifest_csv_contains(
            manifest,
            "native_primitives",
            "CLONE"
        ));
        assert!(manifest_csv_contains(
            manifest,
            "native_primitives",
            "THREAD_JOIN"
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
            "__lnp_call_gate_create",
            "__lnp_cap_dup",
            "__lnp_cap_send",
            "__lnp_cap_recv",
            "__lnp_cap_revoke",
            "__lnp_alloc",
            "__lnp_alloc_ex",
            "__lnp_alloc_size",
            "__lnp_free",
            "__lnp_get_pid",
            "__lnp_spawn_entry",
            "__lnp_thread_join",
            "__lnp_yield",
            "__lnp_mmap_bootstrap",
            "__lnp_munmap_bootstrap",
            "__lnp_mprotect_bootstrap",
            "__lnp_exit",
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
    fn intrinsic_lowering_manifest_matches_real_llvm_surface() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let intrinsic_manifest = include_str!("../toolchain/lnp64_intrinsics.manifest");
        let lowering_manifest = include_str!("../toolchain/lnp64_intrinsic_lowering.manifest");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let intrinsic_header = include_str!("../toolchain/lnp64_intrinsics.h");
        let isel = include_str!("../llvm/lib/Target/LNP64/LNP64ISelLowering.cpp");
        let real_llc = include_str!("../scripts/run_real_llvm_lnp64.sh");
        let fd_min = include_str!("../toolchain/liblnp64_fd_min.c");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let target_intrinsics: std::collections::BTreeSet<_> =
            manifest_field(target_manifest, "intrinsics")
                .split(',')
                .collect();
        let intrinsic_by_name: std::collections::BTreeMap<_, _> =
            intrinsic_rows(intrinsic_manifest)
                .into_iter()
                .map(|(name, primitive, result, operands)| (name, (primitive, result, operands)))
                .collect();
        let rows = intrinsic_lowering_rows(lowering_manifest);
        let mut names = std::collections::BTreeSet::new();

        assert_eq!(
            manifest_field(target_manifest, "intrinsic_lowering_contract"),
            "toolchain/lnp64_intrinsic_lowering.manifest"
        );
        assert!(contract_index.contains(
            "intrinsic_lowering|toolchain/lnp64_intrinsic_lowering.manifest|intrinsic_lowering_manifest_matches_real_llvm_surface"
        ));
        assert_eq!(rows.len(), target_intrinsics.len());
        assert!(roadmap.contains("toolchain/lnp64_intrinsic_lowering.manifest"));
        assert!(roadmap.contains("cannot silently lower"));

        for (name, primitive, abi_shape, status, evidence, blocker) in rows {
            assert!(
                names.insert(name),
                "duplicate intrinsic lowering row {name}"
            );
            assert!(
                target_intrinsics.contains(name),
                "lowering manifest names {name}, but target manifest does not"
            );
            let Some((declared_primitive, _, declared_operands)) = intrinsic_by_name.get(name)
            else {
                panic!("lowering manifest names {name}, but intrinsic manifest does not");
            };
            assert_eq!(
                primitive, *declared_primitive,
                "lowering primitive for {name} diverges from intrinsic manifest"
            );
            assert!(
                !abi_shape.is_empty() && !declared_operands.is_empty(),
                "intrinsic {name} must keep ABI operands explicit"
            );
            for path in evidence {
                assert!(
                    manifest_root.join(path).is_file(),
                    "lowering evidence path {path} for {name} does not exist"
                );
            }

            let callee_probe = format!("CalleeName == \"{name}\"");
            match status {
                "call_lowered" => {
                    assert_eq!(blocker, "none", "lowered intrinsic {name} has blocker");
                    assert!(
                        isel.contains(&callee_probe),
                        "call-lowered intrinsic {name} is missing from LLVM call lowering"
                    );
                    assert!(
                        real_llc.contains(name) || fd_min.contains(name),
                        "call-lowered intrinsic {name} is missing from real LLVM smoke coverage"
                    );
                }
                "inline_asm_lowered" => {
                    assert_eq!(blocker, "none", "inline intrinsic {name} has blocker");
                    let asm_mnemonic = primitive.to_ascii_lowercase().replace('.', ".");
                    assert!(
                        intrinsic_header.contains(&format!("static inline"))
                            && intrinsic_header.contains(name),
                        "inline intrinsic {name} is missing from the intrinsic header"
                    );
                    assert!(
                        intrinsic_header.contains(&format!("\"{asm_mnemonic} "))
                            || intrinsic_header.contains(&format!("\"{asm_mnemonic}")),
                        "inline intrinsic {name} is missing asm mnemonic {asm_mnemonic}"
                    );
                }
                "inline_record_builder_lowered" => {
                    assert_eq!(
                        blocker, "none",
                        "record-builder intrinsic {name} has blocker"
                    );
                    assert!(
                        intrinsic_header.contains("static inline")
                            && intrinsic_header.contains(name),
                        "record-builder intrinsic {name} is missing from the intrinsic header"
                    );
                    assert!(
                        real_llc.contains(name),
                        "record-builder intrinsic {name} lacks real LLVM smoke coverage"
                    );
                }
                "pending_encoding" | "pending_argblock" | "pending_libc_record_builder" => {
                    assert_ne!(blocker, "none", "pending intrinsic {name} needs a blocker");
                    assert!(
                        !isel.contains(&callee_probe),
                        "pending intrinsic {name} must not have ad-hoc LLVM call lowering"
                    );
                    assert!(
                        intrinsic_header.contains(name),
                        "pending intrinsic {name} should remain declared at the ABI boundary"
                    );
                }
                _ => panic!("unknown intrinsic lowering status {status} for {name}"),
            }
        }

        for name in target_intrinsics {
            assert!(
                names.contains(name),
                "target manifest intrinsic {name} is missing from lowering manifest"
            );
        }
        assert!(intrinsic_header.contains("#define LNP64_OBJECT_CTL_CREATE 1UL"));
        assert!(intrinsic_header.contains("static inline lnp64_word_t __lnp_object_create"));
        assert!(intrinsic_header.contains("lnp64_word_t record[9];"));
        assert!(intrinsic_header.contains("record[0] = LNP64_OBJECT_CTL_CREATE;"));
        assert!(intrinsic_header.contains("record[8] = 0;"));
        assert!(intrinsic_header.contains("return __lnp_object_ctl((lnp64_word_t)record);"));
        assert!(intrinsic_header.contains("static inline lnp64_word_t __lnp_domain_create"));
        assert!(intrinsic_header.contains("lnp64_word_t record[25];"));
        assert!(intrinsic_header.contains("return __lnp_domain_ctl((lnp64_word_t)record);"));
        assert!(intrinsic_header.contains("static inline lnp64_word_t __lnp_call_gate_create"));
        assert!(intrinsic_header.contains("record[2] = 4;"));
        for (name, mnemonic) in [
            ("__lnp_cap_dup", "cap_dup"),
            ("__lnp_cap_send", "cap_send"),
            ("__lnp_cap_recv", "cap_recv"),
            ("__lnp_cap_revoke", "cap_revoke"),
        ] {
            assert!(
                intrinsic_header.contains(name) && intrinsic_header.contains(mnemonic),
                "capability intrinsic {name} must lower through {mnemonic} in the header"
            );
            assert!(
                real_llc.contains("intrinsic-cap-control-clang-smoke.o")
                    && real_llc.contains(mnemonic),
                "capability intrinsic {name} lacks real LLVM object smoke coverage"
            );
        }
        assert!(intrinsic_header.contains("lnp64_word_t record[4];"));
        assert!(
            intrinsic_header
                .contains("record[1] = 0;\n  record[2] = rights;\n  record[3] = flags;")
        );
        assert!(intrinsic_header.contains("record[2] = 0;\n  record[3] = flags;"));
        assert!(real_llc.contains("lnp64-intrinsic-cap-control-linked.elf"));
        assert!(
            real_llc.contains("real LLVM LNP64 lld intrinsic capability control link smoke passed")
        );
    }

    #[test]
    fn intrinsic_header_matches_intrinsic_manifest() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let intrinsic_manifest = include_str!("../toolchain/lnp64_intrinsics.manifest");
        let intrinsic_header = include_str!("../toolchain/lnp64_intrinsics.h");
        let target_intrinsic_header = include_str!("../toolchain/include/lnp64/intrinsics.h");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let roadmap = include_str!("../toolchain_roadmap.md");
        let rows = intrinsic_rows(intrinsic_manifest);
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let header_path = manifest_field(target_manifest, "intrinsic_header_contract");
        let target_header_path =
            manifest_field(target_manifest, "target_intrinsic_header_contract");
        let mut declarations = std::collections::BTreeSet::new();

        assert_eq!(header_path, "toolchain/lnp64_intrinsics.h");
        assert_eq!(target_header_path, "toolchain/include/lnp64/intrinsics.h");
        assert!(manifest_root.join(header_path).is_file());
        assert!(manifest_root.join(target_header_path).is_file());
        assert!(contract_index.contains(
            "intrinsic_header|toolchain/lnp64_intrinsics.h|intrinsic_header_matches_intrinsic_manifest"
        ));
        assert!(contract_index.contains(
            "target_intrinsic_header|toolchain/include/lnp64/intrinsics.h|target_intrinsic_header_wraps_canonical_header"
        ));
        assert!(transition_manifest.contains("toolchain/lnp64_intrinsics.h"));
        assert!(transition_manifest.contains("toolchain/include/lnp64/intrinsics.h"));
        assert!(roadmap.contains("toolchain/lnp64_intrinsics.h"));
        assert!(target_intrinsic_header.contains("#include \"../../lnp64_intrinsics.h\""));
        assert!(intrinsic_header.contains("#ifndef LNP64_INTRINSICS_H"));
        assert!(intrinsic_header.contains("typedef unsigned long lnp64_word_t;"));
        assert!(intrinsic_header.contains("typedef lnp64_word_t lnp64_cap_t;"));

        for (name, primitive, _, operands) in rows {
            assert!(
                declarations.insert(name),
                "duplicate intrinsic declaration check for {name}"
            );
            assert!(
                intrinsic_header.contains(&format!(" {name}("))
                    || intrinsic_header.contains(&format!("*{name}(")),
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
    fn target_intrinsic_header_wraps_canonical_header() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let target_intrinsic_header = include_str!("../toolchain/include/lnp64/intrinsics.h");
        let contract_index = include_str!("../toolchain/lnp64_contracts.manifest");
        let transition_manifest = include_str!("../toolchain/lnp64_transition.manifest");
        let manifest_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let target_header_path =
            manifest_field(target_manifest, "target_intrinsic_header_contract");

        assert_eq!(target_header_path, "toolchain/include/lnp64/intrinsics.h");
        assert!(manifest_root.join(target_header_path).is_file());
        assert!(target_intrinsic_header.contains("#include \"../../lnp64_intrinsics.h\""));
        assert!(contract_index.contains(
            "target_intrinsic_header|toolchain/include/lnp64/intrinsics.h|target_intrinsic_header_wraps_canonical_header"
        ));
        assert!(transition_manifest.contains("toolchain/include/lnp64/intrinsics.h"));
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
        let asm_parser = include_str!("../llvm/lib/Target/LNP64/AsmParser/LNP64AsmParser.cpp");
        let inst_printer =
            include_str!("../llvm/lib/Target/LNP64/InstPrinter/LNP64InstPrinter.cpp");
        let disassembler =
            include_str!("../llvm/lib/Target/LNP64/Disassembler/LNP64Disassembler.cpp");
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
        assert!(mc_manifest.contains("fixed32_rri_simm14"));
        assert!(mc_manifest.contains("fixed32_mem_base_simm"));
        assert!(mc_manifest.contains("simm24_words[23:0]"));
        assert!(mc_manifest.contains("fixed32_mmap_bootstrap_control"));
        assert!(mc_manifest.contains("fixed32_env_get_control"));
        assert!(mc_manifest.contains("fixed32_pcr_control"));
        assert!(mc_manifest.contains("Final __lnp_mmap remains blocked"));
        assert!(mc_manifest.contains("F9 argument-block encoding"));

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
            "wide_constants",
            "integer_alu_rrr",
            "integer_alu_rri",
            "integer_compare_value",
            "control_branch",
            "runtime_control",
            "memory",
            "atomics",
            "heap_rr",
            "heap_rrr",
            "heap_reg",
            "mmap_bootstrap_control",
            "env_get_control",
            "pcr_control",
            "native_primitives",
            "clone_control",
            "compat_metadata_control",
            "native_control_rr",
            "native_capability_rr",
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
        assert!(groups["native_capability_rr"].1.contains(&"CAP_DUP"));
        assert!(groups["native_capability_rr"].1.contains(&"CAP_SEND"));
        assert!(groups["native_capability_rr"].1.contains(&"CAP_RECV"));
        assert!(groups["native_capability_rr"].1.contains(&"CAP_REVOKE"));
        assert!(groups["clone_control"].0.contains("fixed32_clone_control"));
        assert!(groups["clone_control"].1.contains(&"CLONE.SPAWN"));
        assert!(groups["clone_control"].1.contains(&"THREAD_JOIN"));
        assert!(
            groups["compat_process_control"]
                .0
                .contains("fixed32_compat_process")
        );
        assert!(groups["compat_process_control"].1.contains(&"FORK"));
        assert!(groups["compat_process_control"].1.contains(&"WAIT_PID"));
        assert!(groups["compat_process_control"].1.contains(&"EXEC"));
        assert!(mc_emitter.contains("case LNP64::EXEC:"));
        assert!(mc_emitter.contains("encodeFixed32RRR(0x7f"));
        assert!(asm_parser.contains(".Case(\"exec\", LNP64::EXEC)"));
        assert!(inst_printer.contains("case LNP64::EXEC:"));
        assert!(disassembler.contains("case 0x7f:"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::EXEC)"));
        assert!(
            groups["compat_metadata_control"]
                .0
                .contains("fixed32_compat_metadata")
        );
        assert!(
            groups["compat_metadata_control"]
                .1
                .contains(&"STAT_PATH_AT")
        );
        assert!(groups["compat_metadata_control"].1.contains(&"STAT_FD_DYN"));
        assert!(
            groups["compat_metadata_control"]
                .1
                .contains(&"UTIME_PATH_AT")
        );
        assert!(
            groups["compat_metadata_control"]
                .1
                .contains(&"UTIME_FD_DYN")
        );
        assert!(
            groups["compat_metadata_control"]
                .1
                .contains(&"FCNTL_FD_DYN")
        );
        assert!(groups["compat_metadata_control"].1.contains(&"FD_SEEK_DYN"));
        assert!(
            groups["compat_metadata_control"]
                .1
                .contains(&"UNLINK_PATH_AT")
        );
        for opcode in [
            "OPEN_DIR_DYN",
            "MKDIR_PATH_AT",
            "RENAME_PATH_AT",
            "LINK_PATH_AT",
            "SYMLINK_PATH_AT",
            "READLINK_PATH_AT",
            "CHDIR_PATH",
            "GETCWD_PATH",
            "CHMOD_PATH_AT",
            "CHOWN_PATH_AT",
        ] {
            assert!(
                groups["compat_namespace_control"].1.contains(&opcode),
                "missing namespace opcode {opcode}"
            );
        }
        assert!(!groups.contains_key("native_control_planned"));
        assert!(asm_parser.contains(r#".Case("clone.spawn", LNP64::CLONE_SPAWN)"#));
        assert!(asm_parser.contains(r#".Case("thread_join", LNP64::THREAD_JOIN)"#));
        assert!(asm_parser.contains(r#".Case("unlink_path_at", LNP64::UNLINK_PATH_AT)"#));
        assert!(asm_parser.contains(r#".Case("stat_path_at", LNP64::STAT_PATH_AT)"#));
        assert!(asm_parser.contains(r#".Case("stat_fd_dyn", LNP64::STAT_FD_DYN)"#));
        assert!(asm_parser.contains(r#".Case("utime_path_at", LNP64::UTIME_PATH_AT)"#));
        assert!(asm_parser.contains(r#".Case("utime_fd_dyn", LNP64::UTIME_FD_DYN)"#));
        assert!(asm_parser.contains(r#".Case("fcntl_fd_dyn", LNP64::FCNTL_FD_DYN)"#));
        assert!(asm_parser.contains(r#".Case("fd_seek_dyn", LNP64::FD_SEEK_DYN)"#));
        assert!(asm_parser.contains("Opcode == LNP64::UNLINK_PATH_AT"));
        assert!(inst_printer.contains(r#"return "clone.spawn";"#));
        assert!(inst_printer.contains(r#"return "thread_join";"#));
        assert!(inst_printer.contains(r#"return "unlink_path_at";"#));
        assert!(inst_printer.contains(r#"return "stat_path_at";"#));
        assert!(inst_printer.contains(r#"return "stat_fd_dyn";"#));
        assert!(inst_printer.contains(r#"return "utime_path_at";"#));
        assert!(inst_printer.contains(r#"return "utime_fd_dyn";"#));
        assert!(inst_printer.contains(r#"return "fcntl_fd_dyn";"#));
        assert!(inst_printer.contains(r#"return "fd_seek_dyn";"#));
        assert!(mc_emitter.contains("case LNP64::CLONE_SPAWN"));
        assert!(mc_emitter.contains("case LNP64::THREAD_JOIN"));
        assert!(mc_emitter.contains("case LNP64::UNLINK_PATH_AT"));
        assert!(mc_emitter.contains("case LNP64::STAT_PATH_AT"));
        assert!(mc_emitter.contains("case LNP64::STAT_FD_DYN"));
        assert!(mc_emitter.contains("case LNP64::UTIME_PATH_AT"));
        assert!(mc_emitter.contains("case LNP64::UTIME_FD_DYN"));
        assert!(mc_emitter.contains("case LNP64::FCNTL_FD_DYN"));
        assert!(mc_emitter.contains("case LNP64::FD_SEEK_DYN"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::CLONE_SPAWN)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::THREAD_JOIN)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::UNLINK_PATH_AT)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::STAT_PATH_AT)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::STAT_FD_DYN)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::UTIME_PATH_AT)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::UTIME_FD_DYN)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::FCNTL_FD_DYN)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::FD_SEEK_DYN)"));
        for (mnemonic, opcode) in [
            ("open_dir_dyn", "OPEN_DIR_DYN"),
            ("mkdir_path_at", "MKDIR_PATH_AT"),
            ("rename_path_at", "RENAME_PATH_AT"),
            ("link_path_at", "LINK_PATH_AT"),
            ("symlink_path_at", "SYMLINK_PATH_AT"),
            ("readlink_path_at", "READLINK_PATH_AT"),
            ("chdir_path", "CHDIR_PATH"),
            ("getcwd_path", "GETCWD_PATH"),
            ("chmod_path_at", "CHMOD_PATH_AT"),
            ("chown_path_at", "CHOWN_PATH_AT"),
        ] {
            assert!(asm_parser.contains(&format!(r#".Case("{mnemonic}", LNP64::{opcode})"#)));
            assert!(inst_printer.contains(&format!(r#"return "{mnemonic}";"#)));
            assert!(mc_emitter.contains(&format!("case LNP64::{opcode}")));
            assert!(disassembler.contains(&format!("Instr.setOpcode(LNP64::{opcode})")));
        }
        assert!(asm_parser.contains(r#".Case("cap_send", LNP64::CAP_SEND)"#));
        assert!(asm_parser.contains(r#".Case("cap_recv", LNP64::CAP_RECV)"#));
        assert!(asm_parser.contains(r#".Case("cap_dup", LNP64::CAP_DUP)"#));
        assert!(asm_parser.contains(r#".Case("cap_revoke", LNP64::CAP_REVOKE)"#));
        assert!(inst_printer.contains(r#"return "cap_send";"#));
        assert!(inst_printer.contains(r#"return "cap_recv";"#));
        assert!(inst_printer.contains(r#"return "cap_dup";"#));
        assert!(inst_printer.contains(r#"return "cap_revoke";"#));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::CAP_SEND)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::CAP_RECV)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::CAP_DUP)"));
        assert!(disassembler.contains("Instr.setOpcode(LNP64::CAP_REVOKE)"));
        assert!(mc_emitter.contains("encodeFixed32NoOperand"));
        assert!(mc_emitter.contains("encodeFixed32RI"));
        assert!(mc_emitter.contains("encodeFixed32RR"));
        assert!(mc_emitter.contains("encodeFixed32RRR"));
        assert!(mc_emitter.contains("encodeFixed32RRI"));
        assert!(mc_emitter.contains("encodeFixed32Mem"));
        assert!(mc_emitter.contains("encodeFixed32Branch"));
        assert!(mc_emitter.contains("encodeFixed32BranchOperand"));
        assert!(mc_emitter.contains("fixup_lnp64_branch26"));
        assert!(mc_emitter.contains("encodeFixed32Reg"));
        assert!(mc_emitter.contains("case LNP64::NOP"));
        assert!(mc_emitter.contains("case LNP64::YIELD"));
        assert!(mc_emitter.contains("case LNP64::RET"));
        assert!(mc_emitter.contains("case LNP64::LI"));
        assert!(mc_emitter.contains("case LNP64::LI32"));
        assert!(mc_emitter.contains("case LNP64::ADD"));
        assert!(mc_emitter.contains("case LNP64::ADDI"));
        assert!(mc_emitter.contains("case LNP64::UDIV"));
        assert!(mc_emitter.contains("case LNP64::SREM"));
        assert!(mc_emitter.contains("case LNP64::UREM"));
        assert!(mc_emitter.contains("case LNP64::MULH"));
        assert!(mc_emitter.contains("case LNP64::MULHSU"));
        assert!(mc_emitter.contains("case LNP64::AMO_SWAP"));
        assert!(mc_emitter.contains("case LNP64::AMO_OR"));
        assert!(mc_emitter.contains("case LNP64::AMO_XOR"));
        assert!(mc_emitter.contains("case LNP64::LOCK_CMPXCHG"));
        assert!(mc_emitter.contains("case LNP64::FUTEX_WAIT"));
        assert!(mc_emitter.contains("case LNP64::FUTEX_WAKE"));
        assert!(mc_emitter.contains("case LNP64::FENCE"));
        assert!(mc_emitter.contains("case LNP64::ISYNC"));
        assert!(mc_emitter.contains("case LNP64::ENV_GET"));
        assert!(mc_emitter.contains("encodeFixed32RRRR"));
        assert!(mc_emitter.contains("case LNP64::SEXT_B"));
        assert!(mc_emitter.contains("case LNP64::ZEXT_W"));
        assert!(mc_emitter.contains("case LNP64::CLZ"));
        assert!(mc_emitter.contains("case LNP64::BSWAP64"));
        assert!(mc_emitter.contains("case LNP64::CSEL_GT"));
        assert!(mc_emitter.contains("case LNP64::CSEL_ULT"));
        assert!(mc_emitter.contains("case LNP64::CALL"));
        assert!(mc_emitter.contains("case LNP64::CALL_REG"));
        assert!(mc_emitter.contains("case LNP64::LR_GET"));
        assert!(mc_emitter.contains("case LNP64::LR_SET"));
        assert!(mc_emitter.contains("case LNP64::CMPU"));
        assert!(mc_emitter.contains("case LNP64::CSET_ULT"));
        assert!(mc_emitter.contains("case LNP64::CSET_EQ"));
        assert!(mc_emitter.contains("case LNP64::ERRNO_GET"));
        assert!(mc_emitter.contains("case LNP64::ERRNO_SET"));
        assert!(mc_emitter.contains("case LNP64::EXIT"));
        assert!(mc_emitter.contains("case LNP64::ALLOC"));
        assert!(mc_emitter.contains("case LNP64::ALLOC_EX"));
        assert!(mc_emitter.contains("case LNP64::ALLOC_SIZE"));
        assert!(mc_emitter.contains("case LNP64::FREE"));
        assert!(mc_emitter.contains("case LNP64::AWAIT"));
        assert!(mc_emitter.contains("case LNP64::GATE_CALL"));
        assert!(mc_emitter.contains("case LNP64::GATE_RETURN"));
        assert!(mc_emitter.contains("case LNP64::OBJECT_CTL"));
        assert!(mc_emitter.contains("case LNP64::DOMAIN_CTL"));
        assert!(mc_emitter.contains("case LNP64::CAP_SEND"));
        assert!(mc_emitter.contains("case LNP64::CAP_RECV"));
        assert!(mc_emitter.contains("case LNP64::CAP_DUP"));
        assert!(mc_emitter.contains("case LNP64::CAP_REVOKE"));
        assert!(mc_emitter.contains("case LNP64::LD"));
        assert!(mc_emitter.contains("case LNP64::ST"));
        assert!(mc_emitter.contains("encodeFixed32NoOperand(0x00)"));
        assert!(mc_emitter.contains("encodeFixed32NoOperand(0x06)"));
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

        for pcr in [
            "PID",
            "PPID",
            "TID",
            "TP",
            "UID",
            "GID",
            "SIGMASK",
            "SIGPENDING",
            "REALTIME_SEC",
            "REALTIME_NSEC",
            "CRED_PROFILE",
            "CRED_HANDLE",
        ] {
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
        assert_eq!(
            manifest_field(debug_unwind_manifest, "real_llvm_debug_sections"),
            "clang_debug_sections_object"
        );
        assert_eq!(
            manifest_field(debug_unwind_manifest, "line_table_decode"),
            "blocked_until_debug_relocation_decoding"
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
                "PID,PPID,TID,TP,UID,GID,SIGMASK,SIGPENDING,REALTIME_SEC,REALTIME_NSEC,CRED_PROFILE,CRED_HANDLE"
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
    fn scheduler_heap_realtime_contracts_scope_current_architecture() {
        fn assert_contains(document: &str, needle: &str) {
            assert!(
                document.contains(needle),
                "missing architecture text: {needle}"
            );
        }

        let design = include_str!("../design.md");
        let hardware = include_str!("../hardware_design.md");
        let formal_roadmap = include_str!("../formal_rtl_codesign_roadmap.md");
        let formal_theorems = include_str!("../formal_theorems.md");
        let readme = include_str!("../README.md");

        assert_contains(design, "Fixed Weighted-Fair Virtual-Deadline Active-Window");
        assert_contains(hardware, "bounded active windows");
        assert_contains(hardware, "virtual-deadline buckets");
        assert_contains(formal_roadmap, "fixed monotonic weight table");
        assert_contains(formal_roadmap, "sticky");
        assert_contains(formal_roadmap, "bounded migration");
        assert_contains(formal_roadmap, "bounded wakeup insertion");
        assert_contains(formal_roadmap, "bounded preemption");
        assert_contains(formal_roadmap, "no scheduler bytecode");
        assert_contains(hardware, "red-black trees");
        assert_contains(hardware, "no red-black tree");

        assert_contains(design, "tightly synchronized global monotonic timebase");
        assert_contains(hardware, "Resource Domain id/generation");
        assert_contains(hardware, "submitter TID/generation");
        assert_contains(formal_theorems, "reservation/deadline metadata");
        assert_contains(hardware, "operation id");
        assert_contains(hardware, "cancellation epoch");
        assert_contains(hardware, "completion target");
        assert_contains(hardware, "No realtime-admitted Class D");
        assert_contains(hardware, "undifferentiated FIFO entry");

        assert_contains(design, "LNP64 Default Heap Algorithm");
        assert_contains(hardware, "fixed size-class dispatch");
        assert_contains(hardware, "per-thread allocation windows");
        assert_contains(hardware, "bounded transfer queues");
        assert_contains(hardware, "domain-owned slab/run pages");
        assert_contains(hardware, "generation fields");
        assert_contains(hardware, "exact-pointer free");
        assert_contains(hardware, "invalid pointers and double free");
        assert_contains(hardware, "NX heap backing");
        assert_contains(hardware, "bounded hot");
        assert_contains(hardware, "Class D owner-engine transactions with inherited");
        assert_contains(hardware, "Rust-style intra-program memory safety");
        assert_contains(hardware, "Ordinary `LD`/`ST`");

        assert_contains(formal_roadmap, "no-lost-wakeup");
        assert_contains(formal_roadmap, "bounded fairness");
        assert_contains(formal_theorems, "deadline comparison");
        assert_contains(formal_roadmap, "exact-pointer free");
        assert_contains(formal_roadmap, "invalid/double/foreign-free rejection");
        assert_contains(formal_roadmap, "domain accounting");
        assert_contains(formal_roadmap, "no hidden unbounded path in Class A/B/C");
        assert_contains(readme, "Realtime contract soundness");
    }

    #[test]
    fn resource_domain_tree_contracts_scope_current_architecture() {
        fn assert_contains(document: &str, needle: &str) {
            assert!(
                document.contains(needle),
                "missing Resource Domain contract text: {needle}"
            );
        }

        let design = include_str!("../design.md");
        let hardware = include_str!("../hardware_design.md");
        let formal_roadmap = include_str!("../formal_rtl_codesign_roadmap.md");
        let formal_theorems = include_str!("../formal_theorems.md");

        assert_contains(
            hardware,
            "V1 uses a **Fixed Monotonic Resource-Domain Tree**",
        );
        assert_contains(hardware, "do not change parentage or");
        assert_contains(hardware, "budget ownership");
        assert_contains(
            hardware,
            "Domain operations are fixed owner-engine transitions",
        );
        assert_contains(hardware, "software callbacks or policy bytecode");
        assert_contains(
            design,
            "Resource Domains are control-plane expensive and data-plane cheap",
        );
        assert_contains(design, "Hot scheduler and allocator paths must not walk");
        assert_contains(hardware, "Resident effective scheduling record");
        assert_contains(hardware, "not by walking the domain tree during dispatch");
        assert_contains(hardware, "resident effective heap-domain record");
        assert_contains(hardware, "must not walk the Resource Domain tree");
        assert_contains(hardware, "`ALLOC`/`FREE` hot path");
        assert_contains(hardware, "monotonic intersection");
        assert_contains(hardware, "must not walk an unbounded ancestor chain");
        assert_contains(hardware, "hierarchy depth is bounded");
        assert_contains(hardware, "Class D domain-engine work");
        assert_contains(hardware, "bounded cursors");
        assert_contains(hardware, "single-owner and monotonic");
        assert_contains(hardware, "stale attachments fail closed");
        assert_contains(
            formal_roadmap,
            "effective-domain records consumed by scheduler, heap",
        );
        assert_contains(
            formal_roadmap,
            "resident generation-checked effective records",
        );
        assert_contains(formal_roadmap, "resident effective scheduling records");
        assert_contains(formal_roadmap, "resident effective heap-domain records");
        assert_contains(
            formal_theorems,
            "flattened effective-domain records consumed by scheduler, heap",
        );
        assert_contains(
            formal_theorems,
            "does not require an unbounded ancestor walk",
        );
        assert_contains(
            formal_theorems,
            "Class D domain-engine refill/recompute of effective records",
        );
        assert_contains(formal_theorems, "scheduler dispatch consumes");
        assert_contains(formal_theorems, "heap hot paths consume");
        assert_contains(formal_theorems, "`ALLOC`/`FREE` hot paths do not walk");
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

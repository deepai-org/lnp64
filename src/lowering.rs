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
pub const LOWER_METADATA: &[NativePrimitive] = &[NativePrimitive::Metadata(MetadataOp::GetMeta)];
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
        native: LOWER_METADATA,
    },
    CompatibilityLowering {
        surface: CompatSurface::Chmod,
        native: LOWER_METADATA,
    },
    CompatibilityLowering {
        surface: CompatSurface::Fcntl,
        native: LOWER_METADATA,
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

    #[test]
    fn llvm_target_manifest_records_required_backend_contract() {
        let manifest = include_str!("../toolchain/lnp64_target.manifest");
        let object_format = include_str!("../object_format.md");
        assert_eq!(manifest_field(manifest, "triple"), "lnp64-unknown-none");
        assert_eq!(manifest_field(manifest, "object_format"), "ELF64");
        assert_eq!(manifest_field(manifest, "endianness"), "little");
        assert_eq!(manifest_field(manifest, "data_model"), "LP64");
        assert_eq!(manifest_field(manifest, "pointer_width"), "64");
        assert_eq!(manifest_field(manifest, "e_machine"), "0x6c64");
        assert_eq!(manifest_field(manifest, "psabi"), "psABI.md");
        assert_eq!(
            manifest_field(manifest, "object_contract"),
            "object_format.md"
        );
        assert_eq!(
            manifest_field(manifest, "relocation_contract"),
            "toolchain/lnp64_relocations.manifest"
        );
        assert_eq!(manifest_field(manifest, "gpr"), "r0-r31");
        assert_eq!(manifest_field(manifest, "fdr"), "fd0-fd31");
        for pcr in ["PID", "PPID", "TID", "TP", "SIGMASK", "SIGPENDING"] {
            assert!(manifest_csv_contains(manifest, "pcr", pcr), "missing {pcr}");
        }
        for relocation in [
            "R_LNP64_ABS64",
            "R_LNP64_PC32",
            "R_LNP64_BRANCH26",
            "R_LNP64_GOT64",
            "R_LNP64_TLS_TPREL64",
            "R_LNP64_RELATIVE",
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
            "__lnp_gate_return",
            "__lnp_domain_ctl",
            "__lnp_object_ctl",
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
    }

    #[test]
    fn relocation_manifest_matches_object_format_and_target_manifest() {
        let target_manifest = include_str!("../toolchain/lnp64_target.manifest");
        let relocation_manifest = include_str!("../toolchain/lnp64_relocations.manifest");
        let object_format = include_str!("../object_format.md");
        let rows = relocation_rows(relocation_manifest);
        let mut numbers = std::collections::BTreeSet::new();
        let mut names = std::collections::BTreeSet::new();

        assert_eq!(rows.len(), 13);
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
        }
        for name in manifest_field(target_manifest, "relocations").split(',') {
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

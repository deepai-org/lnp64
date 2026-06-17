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

pub const OBJECT_CTL_CREATE_RECORD_SIZE: u64 = 72;
pub const DOMAIN_CTL_RECORD_SIZE: u64 = 208;

pub fn lowering_for(surface: CompatSurface) -> &'static [NativePrimitive] {
    COMPATIBILITY_LOWERINGS
        .iter()
        .find(|entry| entry.surface == surface)
        .map(|entry| entry.native)
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
}

#![allow(dead_code)]

use crate::native::{CloneProfile, MetadataOp, ObjectKind, ObjectProfile, Waitable};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatSurface {
    Open,
    Read,
    Write,
    Close,
    Pipe,
    PollSelectEpoll,
    Fork,
    PthreadCreate,
    Signal,
    Errno,
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
pub const LOWER_PTHREAD_CREATE: &[NativePrimitive] = &[NativePrimitive::Clone {
    profile: CloneProfile::NewThreadSharedVm,
}];
pub const LOWER_SIGNAL: &[NativePrimitive] = &[
    NativePrimitive::EventDelivery,
    NativePrimitive::AbiSignalFrame,
];
pub const LOWER_ERRNO: &[NativePrimitive] = &[
    NativePrimitive::ExplicitResult,
    NativePrimitive::TlsErrnoView,
];
pub const LOWER_METADATA: &[NativePrimitive] = &[NativePrimitive::Metadata(MetadataOp::GetMeta)];

pub const COMPATIBILITY_LOWERINGS: &[CompatibilityLowering] = &[
    CompatibilityLowering {
        surface: CompatSurface::Open,
        native: LOWER_OPEN,
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
        surface: CompatSurface::PthreadCreate,
        native: LOWER_PTHREAD_CREATE,
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
        assert!(lowering_for(CompatSurface::Signal).contains(&NativePrimitive::EventDelivery));
        assert!(lowering_for(CompatSurface::Errno).contains(&NativePrimitive::TlsErrnoView));
    }
}

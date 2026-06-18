#![allow(dead_code)]

pub type ErrorCode = u64;
pub type NativeResult<T> = Result<T, ErrorCode>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapabilityHandle(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Domain(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    Counter,
    Queue,
    MemoryObject,
    DmaBuffer,
    Endpoint,
    Timer,
    Classifier,
    Servicelet,
}

impl ObjectKind {
    pub const fn code(self) -> u64 {
        match self {
            Self::Counter => 1,
            Self::Queue => 2,
            Self::MemoryObject => 3,
            Self::DmaBuffer => 4,
            Self::Endpoint => 5,
            Self::Timer => 6,
            Self::Classifier => 7,
            Self::Servicelet => 8,
        }
    }

    pub const fn from_code(code: u64) -> Option<Self> {
        match code {
            1 => Some(Self::Counter),
            2 => Some(Self::Queue),
            3 => Some(Self::MemoryObject),
            4 => Some(Self::DmaBuffer),
            5 => Some(Self::Endpoint),
            6 => Some(Self::Timer),
            7 => Some(Self::Classifier),
            8 => Some(Self::Servicelet),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectProfile {
    Default,
    Pipe,
    EventFd,
    TcpStream,
    CallGate,
    ClassifierTable,
    ServiceletProgram,
}

impl ObjectProfile {
    pub const fn code(self) -> u64 {
        match self {
            Self::Default => 0,
            Self::Pipe | Self::EventFd => 1,
            Self::TcpStream => 2,
            Self::CallGate => 4,
            Self::ClassifierTable => 1,
            Self::ServiceletProgram => 1,
        }
    }

    pub const fn from_code_for_kind(kind: ObjectKind, code: u64) -> Option<Self> {
        match (kind, code) {
            (_, 0) => Some(Self::Default),
            (ObjectKind::Queue, 1) => Some(Self::Pipe),
            (ObjectKind::Counter, 1) => Some(Self::EventFd),
            (ObjectKind::Classifier, 1) => Some(Self::ClassifierTable),
            (ObjectKind::Servicelet, 1) => Some(Self::ServiceletProgram),
            (ObjectKind::Endpoint, 2) => Some(Self::TcpStream),
            (ObjectKind::Queue, 4) => Some(Self::CallGate),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataOp {
    GetMeta,
    SetMeta,
    ObjectCtl,
    DomainCtl,
    NsCtl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneProfile {
    NewProcessCow,
    NewThreadSharedVm,
    SpawnEntry,
    DomainTask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waitable {
    Capability(CapabilityHandle),
    EventQueue(CapabilityHandle),
    Domain(Domain),
    Process(u64),
    Thread(u64),
    Timer(CapabilityHandle),
    Signal(u64),
    Futex(u64),
    Irq(u64),
    Call(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSource {
    CompatSignal,
    HardwareFault,
    Timer,
    Kill,
    ChildExit,
    Futex,
    Irq,
    CallCompletion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeEvent {
    Signal { signum: u64, source: EventSource },
    ChildExit { pid: u64 },
    Timer { signum: u64 },
    Futex { addr: u64 },
    Irq { line: u64 },
    CallCompletion { token: u64 },
}

impl NativeEvent {
    pub const fn compat_signal(signum: u64) -> Self {
        Self::Signal {
            signum,
            source: EventSource::CompatSignal,
        }
    }

    pub const fn kill_signal(signum: u64) -> Self {
        Self::Signal {
            signum,
            source: EventSource::Kill,
        }
    }

    pub const fn fault_signal(signum: u64) -> Self {
        Self::Signal {
            signum,
            source: EventSource::HardwareFault,
        }
    }

    pub const fn timer_signal(signum: u64) -> Self {
        Self::Signal {
            signum,
            source: EventSource::Timer,
        }
    }

    pub const fn child_signal(signum: u64) -> Self {
        Self::Signal {
            signum,
            source: EventSource::ChildExit,
        }
    }

    pub const fn signal_number(self) -> Option<u64> {
        match self {
            Self::Signal { signum, .. } | Self::Timer { signum } => Some(signum),
            _ => None,
        }
    }
}

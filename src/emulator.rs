use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::fd::AsRawFd;
use std::os::raw::{c_char, c_int};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileExt, MetadataExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::asm::Program;
use crate::isa::*;
use crate::native::{
    CloneProfile, EventSource, NativeEvent, NativeResult, ObjectKind, ObjectProfile,
};

const STACK_SIZE: u64 = 4 * 1024 * 1024;
const CALL_FRAME_SIZE: u64 = 32 * 1024;
const THREAD_STACK_STRIDE: u64 = 0x40_000;
const MMAP_BASE: u64 = 0x200_000;
const ASLR_PAGE: u64 = 4096;
const ASLR_HEAP_PAGES: u64 = 16;
const ASLR_MMAP_PAGES: u64 = 16;
const ASLR_STACK_PAGES: u64 = 16;
const SIGCHLD: u64 = 17;
const SIGALRM: u64 = 14;
const SIGSEGV: u64 = 11;
const SIGFPE: u64 = 8;
const SIGNAL_NUMBER_LIMIT: u64 = 64;
const SIG_DFL_HANDLER: usize = 0;
const SIG_IGN_HANDLER: usize = 1;
const SOCKET_AF_INET: u64 = 2;
const SOCKET_TYPE_STREAM: u64 = 1;
const SOCKET_LEVEL_SOL_SOCKET: u64 = 1;
const SOCKET_LEVEL_IPPROTO_TCP: u64 = 6;
const SOCKET_OPT_TCP_NODELAY: u64 = 1;
const SOCKET_OPT_SO_REUSEADDR: u64 = 2;
const SOCKET_OPT_SO_ERROR: u64 = 4;
const SOCKET_OPT_SO_BROADCAST: u64 = 6;
const SOCKET_OPT_SO_SNDBUF: u64 = 7;
const SOCKET_OPT_SO_RCVBUF: u64 = 8;
const SOCKET_OPT_SO_KEEPALIVE: u64 = 9;
const MESSAGE_ENDPOINT_FD: usize = FDR_COUNT - 1;
const PROCESS_INBOX_LIMIT: usize = 1024;
const UTIME_NOW_LNP64: i64 = 1_073_741_823;
const UTIME_OMIT_LNP64: i64 = 1_073_741_822;
const LNP64_STAT_RECORD_SIZE: usize = 104;
const ROOT_DOMAIN_ID: u64 = 1;
const MAX_RESOURCE_DOMAINS: usize = 4096;
const MAX_DOMAIN_DEPTH: u64 = 16;

const ENV_KEY_ISA_VERSION: u64 = 1;
const ENV_KEY_PAGE_SIZE: u64 = 2;
const ENV_KEY_CACHE_LINE_SIZE: u64 = 3;
const ENV_KEY_TIMEBASE_HZ: u64 = 4;
const ENV_KEY_HWCAP0: u64 = 5;
const ENV_KEY_HWCAP1: u64 = 6;
const ENV_KEY_ARCH_THREAD_LIMIT: u64 = 7;
const ENV_KEY_PROCESS_LIMIT: u64 = 8;
const ENV_KEY_DEFAULT_FDR_LIMIT: u64 = 9;
const ENV_KEY_EVENT_QUEUE_LIMIT: u64 = 10;
const ENV_KEY_FUTEX_BUCKET_COUNT: u64 = 11;
const ENV_KEY_ARGC: u64 = 12;
const ENV_KEY_ARGV_BASE: u64 = 13;
const ENV_KEY_ENVP_BASE: u64 = 14;
const ENV_KEY_AUXV_BASE: u64 = 15;
const ENV_KEY_AUXV_ENTRY: u64 = 16;
const ENV_KEY_PERSONALITY_ID: u64 = 17;
const ENV_KEY_BOOT_MANIFEST_FLAGS: u64 = 18;
const ENV_KEY_IMPLEMENTATION_PROFILE: u64 = 19;
const ENV_KEY_DMA_ALIGNMENT: u64 = 20;
const ENV_KEY_TIMER_GRANULARITY_NS: u64 = 21;
const ENV_KEY_MONOTONIC_COUNTER_BITS: u64 = 22;
const ENV_KEY_TIME_BEHAVIOR_FLAGS: u64 = 23;
const ENV_KEY_OPCODE_FEATURE_BITS: u64 = 24;
const ENV_KEY_OBJECT_PROFILE_BITS: u64 = 25;
const ENV_KEY_DOMAIN_FEATURE_BITS: u64 = 26;
const ENV_KEY_SECURITY_PROFILE_BITS: u64 = 27;
const ENV_KEY_SCHEDULER_FEATURE_BITS: u64 = 28;
const ENV_KEY_CLASSIFIER_FEATURE_BITS: u64 = 29;
const ENV_KEY_TOPOLOGY_RECORD_COUNT: u64 = 30;
const ENV_KEY_TOPOLOGY_RECORD_FORMAT: u64 = 31;
const ENV_KEY_RESOURCE_DOMAIN_LIMIT: u64 = 32;
const ENV_KEY_DMA_MAX_DESCRIPTORS: u64 = 33;
const ENV_KEY_CLASSIFIER_ENTRY_LIMIT: u64 = 34;
const ENV_KEY_STARTUP_METADATA_PTR: u64 = 35;
const ENV_KEY_STARTUP_METADATA_LEN: u64 = 36;
const ENV_KEY_STARTUP_METADATA_FORMAT: u64 = 37;
const ENV_KEY_STARTUP_METADATA_VERSION: u64 = 38;
const ENV_KEY_SERVICELET_VERIFY_VERSION: u64 = 39;
const ENV_KEY_SERVICELET_PROGRAM_LIMIT: u64 = 40;
const ENV_KEY_SERVICELET_INSTRUCTION_LIMIT: u64 = 41;
const ENV_KEY_SERVICELET_CYCLE_LIMIT: u64 = 42;
const ENV_KEY_SERVICELET_RECORD_LIMIT: u64 = 43;
const ENV_KEY_SERVICELET_ACTION_LIMIT: u64 = 44;
const ENV_KEY_SERVICELET_ISA_MASK: u64 = 45;
const ENV_KEY_SERVICELET_FLAG_MASK: u64 = 46;
const ENV_KEY_CLASSIFIER_ALLOWED_QUEUE_LIMIT: u64 = 47;
const ENV_KEY_CLASSIFIER_ROUTE_BYTE_LIMIT: u64 = 48;
const ENV_KEY_SIGNAL_NUMBER_LIMIT: u64 = 49;
const ENV_KEY_PROCESS_ENTRY_RECORD: u64 = 64;
const ENV_KEY_TOPOLOGY_RECORD: u64 = 65;
const ENV_ISA_VERSION: u64 = 1;
const ENV_IMPLEMENTATION_PROFILE_REFERENCE: u64 = 1;
const ENV_HWCAP0_RANDOM: u64 = 1 << 0;
const ENV_HWCAP0_CAPABILITIES: u64 = 1 << 1;
const ENV_HWCAP0_RESOURCE_DOMAINS: u64 = 1 << 2;
const ENV_HWCAP0_DMA: u64 = 1 << 3;
const ENV_HWCAP0_FUTEX: u64 = 1 << 4;
const ENV_HWCAP0_OBJECTS: u64 = 1 << 5;
const ENV_HWCAP0_CALL_CAP: u64 = 1 << 6;
const ENV_HWCAP0_CLASSIFIER: u64 = 1 << 7;
const ENV_CACHE_LINE_SIZE: u64 = 64;
const ENV_DMA_ALIGNMENT: u64 = 64;
const ENV_TIMEBASE_HZ: u64 = 1_000_000_000;
const ENV_TIMER_GRANULARITY_NS: u64 = 1_000_000;
const ENV_THREAD_LIMIT: u64 = 4096;
const ENV_PROCESS_LIMIT: u64 = 4096;
const ENV_EVENT_QUEUE_LIMIT: u64 = 4096;
const PROCESS_EVENT_QUEUE_LIMIT: usize = ENV_EVENT_QUEUE_LIMIT as usize;
const ENV_FUTEX_BUCKET_COUNT: u64 = 4096;
const ENV_TOPOLOGY_RECORD_COUNT: u64 = 5;
const ENV_TOPOLOGY_RECORD_FORMAT: u64 = 1;
const ENV_TOPOLOGY_RECORD_SIZE: usize = 64;
const ENV_STARTUP_METADATA_FORMAT: u64 = 1;
const ENV_STARTUP_METADATA_VERSION: u64 = 1;
const ENV_OPCODE_FEATURE_BASE_ISA: u64 = 1 << 0;
const ENV_OPCODE_FEATURE_FDR: u64 = 1 << 1;
const ENV_OPCODE_FEATURE_OBJECT_CTL: u64 = 1 << 2;
const ENV_OPCODE_FEATURE_DOMAIN_CTL: u64 = 1 << 3;
const ENV_OPCODE_FEATURE_DMA_CTL: u64 = 1 << 4;
const ENV_OPCODE_FEATURE_CALL_CAP: u64 = 1 << 5;
const ENV_OPCODE_FEATURE_ENV_GET: u64 = 1 << 6;
const ENV_OPCODE_FEATURE_RANDOM: u64 = 1 << 7;
const ENV_OPCODE_FEATURE_AWAIT: u64 = 1 << 8;
const ENV_OPCODE_FEATURE_NS_CTL: u64 = 1 << 9;
const NS_CTL_VERSION: u64 = 1;
const NS_OP_RESOLVE: u64 = 1;
const NS_RESOLVE_FLAG_NOFOLLOW_FINAL: u64 = 1 << 0;
const ENV_OBJECT_PROFILE_COUNTER: u64 = 1 << 0;
const ENV_OBJECT_PROFILE_QUEUE: u64 = 1 << 1;
const ENV_OBJECT_PROFILE_MEMORY_OBJECT: u64 = 1 << 2;
const ENV_OBJECT_PROFILE_DMA_BUFFER: u64 = 1 << 3;
const ENV_OBJECT_PROFILE_ENDPOINT: u64 = 1 << 4;
const ENV_OBJECT_PROFILE_TIMER: u64 = 1 << 5;
const ENV_OBJECT_PROFILE_CALL_GATE: u64 = 1 << 6;
const ENV_OBJECT_PROFILE_CLASSIFIER_TABLE: u64 = 1 << 7;
const ENV_OBJECT_PROFILE_SERVICELET_PROGRAM: u64 = 1 << 8;
const ENV_DOMAIN_FEATURE_NESTED: u64 = 1 << 0;
const ENV_DOMAIN_FEATURE_BUDGETS: u64 = 1 << 1;
const ENV_DOMAIN_FEATURE_SECURITY_POLICY: u64 = 1 << 2;
const ENV_DOMAIN_FEATURE_LIFECYCLE: u64 = 1 << 3;
const ENV_SECURITY_PROFILE_ASLR: u64 = 1 << 0;
const ENV_SECURITY_PROFILE_NX: u64 = 1 << 1;
const ENV_SECURITY_PROFILE_WX_DENY: u64 = 1 << 2;
const ENV_SECURITY_PROFILE_GUARD_PAGES: u64 = 1 << 3;
const ENV_SECURITY_PROFILE_CAP_REVOCATION: u64 = 1 << 4;
const ENV_SECURITY_PROFILE_ENTROPY_QUOTA: u64 = 1 << 5;
const ENV_SECURITY_PROFILE_NO_RAW_IRQ: u64 = 1 << 6;
const ENV_SECURITY_PROFILE_NO_RAW_MMIO: u64 = 1 << 7;
const ENV_SECURITY_PROFILE_NO_RAW_SYSCALL: u64 = 1 << 8;
const ENV_SECURITY_PROFILE_ALL: u64 = ENV_SECURITY_PROFILE_ASLR
    | ENV_SECURITY_PROFILE_NX
    | ENV_SECURITY_PROFILE_WX_DENY
    | ENV_SECURITY_PROFILE_GUARD_PAGES
    | ENV_SECURITY_PROFILE_CAP_REVOCATION
    | ENV_SECURITY_PROFILE_ENTROPY_QUOTA
    | ENV_SECURITY_PROFILE_NO_RAW_IRQ
    | ENV_SECURITY_PROFILE_NO_RAW_MMIO
    | ENV_SECURITY_PROFILE_NO_RAW_SYSCALL;
const ENV_SCHEDULER_FEATURE_RUNQUEUE: u64 = 1 << 0;
const ENV_SCHEDULER_FEATURE_AWAIT: u64 = 1 << 1;
const ENV_SCHEDULER_FEATURE_FD_WAITERS: u64 = 1 << 2;
const ENV_SCHEDULER_FEATURE_THREAD_JOIN: u64 = 1 << 3;
const ENV_SCHEDULER_FEATURE_ALL: u64 = ENV_SCHEDULER_FEATURE_RUNQUEUE
    | ENV_SCHEDULER_FEATURE_AWAIT
    | ENV_SCHEDULER_FEATURE_FD_WAITERS
    | ENV_SCHEDULER_FEATURE_THREAD_JOIN;
const ENV_CLASSIFIER_FEATURE_EXACT: u64 = 1 << 0;
const ENV_CLASSIFIER_FEATURE_MASKED: u64 = 1 << 1;
const ENV_CLASSIFIER_FEATURE_RANGE: u64 = 1 << 2;
const ENV_CLASSIFIER_FEATURE_HASH: u64 = 1 << 3;
const ENV_CLASSIFIER_FEATURE_MARK: u64 = 1 << 4;
const ENV_CLASSIFIER_FEATURE_COUNT: u64 = 1 << 5;
const ENV_CLASSIFIER_FEATURE_DROP: u64 = 1 << 6;
const ENV_CLASSIFIER_FEATURE_ROUTE: u64 = 1 << 7;
const ENV_CLASSIFIER_FEATURE_NEEDS_SOFTWARE: u64 = 1 << 8;
const ENV_CLASSIFIER_FEATURE_ALL: u64 = ENV_CLASSIFIER_FEATURE_EXACT
    | ENV_CLASSIFIER_FEATURE_MASKED
    | ENV_CLASSIFIER_FEATURE_RANGE
    | ENV_CLASSIFIER_FEATURE_HASH
    | ENV_CLASSIFIER_FEATURE_MARK
    | ENV_CLASSIFIER_FEATURE_COUNT
    | ENV_CLASSIFIER_FEATURE_DROP
    | ENV_CLASSIFIER_FEATURE_ROUTE
    | ENV_CLASSIFIER_FEATURE_NEEDS_SOFTWARE;
const ENV_TIME_FLAG_MONOTONIC: u64 = 1 << 0;
const ENV_TIME_FLAG_REALTIME: u64 = 1 << 1;
const AT_UID: u64 = 11;
const AT_EUID: u64 = 12;
const AT_GID: u64 = 13;
const AT_EGID: u64 = 14;
const AT_PAGESZ: u64 = 6;
const AT_HWCAP: u64 = 16;
const AT_CLKTCK: u64 = 17;
const AT_RANDOM: u64 = 25;

const DOMAIN_OP_CREATE: u64 = 1;
const DOMAIN_OP_CONFIGURE: u64 = 2;
const DOMAIN_OP_QUERY: u64 = 3;
const DOMAIN_OP_FREEZE: u64 = 4;
const DOMAIN_OP_RESUME: u64 = 5;
const DOMAIN_OP_DESTROY: u64 = 6;
const DOMAIN_OP_ATTACH_SELF: u64 = 7;
const DOMAIN_OP_DETACH_SELF: u64 = 8;

const DOMAIN_STATE_ACTIVE: u64 = 0;
const DOMAIN_STATE_FROZEN: u64 = 1;
const DOMAIN_STATE_DESTROYED: u64 = 2;
const DOMAIN_QUERY_SIZE: u64 = 200;
const DOMAIN_BOOL_INHERIT: u64 = 0;
const DOMAIN_BOOL_ENABLE: u64 = 1;
const DOMAIN_BOOL_DISABLE: u64 = 2;
const DOMAIN_SECURITY_ASLR_ENABLED: u64 = 144;
const DOMAIN_SECURITY_ALLOW_WX: u64 = 152;
const DOMAIN_SECURITY_ALLOW_JIT_TRANSITION: u64 = 160;
const DOMAIN_SECURITY_ENTROPY_QUOTA: u64 = 168;
const DOMAIN_SECURITY_DMA_ALLOWED: u64 = 176;
const DOMAIN_SECURITY_HARDENING_PROFILE: u64 = 184;
const DOMAIN_SECURITY_EXEC_SOURCE_POLICY: u64 = 192;
const EXEC_SOURCE_ANONYMOUS_JIT: u64 = 1 << 0;
const EXEC_SOURCE_FILE_MAPPING: u64 = 1 << 1;

const DOMAIN_CAP_PROCESS: u64 = 1 << 0;
const DOMAIN_CAP_MEMORY: u64 = 1 << 1;
const DOMAIN_CAP_FDR: u64 = 1 << 2;
const DOMAIN_CAP_IO: u64 = 1 << 3;
const DOMAIN_CAP_OBJECT: u64 = 1 << 4;
const DOMAIN_CAP_CALL: u64 = 1 << 5;

const OBJECT_OP_CREATE: u64 = 1;
const OBJECT_OP_SOCKET_BIND: u64 = 2;
const OBJECT_OP_SOCKET_LISTEN: u64 = 3;
const OBJECT_OP_SOCKET_CONNECT: u64 = 4;
const OBJECT_OP_SOCKET_ACCEPT: u64 = 5;
const OBJECT_OP_SOCKET_GETSOCKNAME: u64 = 6;
const OBJECT_OP_SOCKET_GETSOCKOPT: u64 = 7;
const OBJECT_OP_SOCKET_SETSOCKOPT: u64 = 8;
const OBJECT_OP_CLASSIFY: u64 = 9;
const OBJECT_OP_CLASSIFIER_QUERY: u64 = 10;
const EVENTFD_SEMAPHORE: u64 = 1;
const EVENTFD_NONBLOCK: u64 = 0x800;
const CLASSIFIER_MAX_RULES: usize = 64;
const CLASSIFIER_MAX_ALLOWED_QUEUES: usize = 64;
const CLASSIFIER_MAX_ROUTE_BYTES: usize = 4096;
const PIPE_BUFFER_BYTE_LIMIT: usize = ENV_EVENT_QUEUE_LIMIT as usize;
const PIPE_CAPABILITY_LIMIT: usize = ENV_EVENT_QUEUE_LIMIT as usize;
const CLASSIFIER_RULE_SIZE: u64 = 64;
const CLASSIFY_ENVELOPE_SIZE: u64 = 72;
const CLASSIFY_RESULT_SIZE: u64 = 64;
const CLASSIFIER_COUNTERS_SIZE: u64 = 40;
const SERVICELET_VERIFY_VERSION: u64 = 1;
const SERVICELET_MAX_PROGRAM_BYTES: u64 = 4096;
const SERVICELET_MAX_INSTRUCTIONS: u64 = 512;
const SERVICELET_MAX_CYCLES: u64 = 4096;
const SERVICELET_MAX_RECORD_BYTES: u64 = 4096;
const SERVICELET_MAX_ACTION_BYTES: u64 = 256;
const SERVICELET_ALLOWED_ISA_MASK: u64 = 0x0f;
const SERVICELET_FLAG_ALLOW_STATIC_LOOPS: u64 = 1 << 0;
const CLASSIFY_PROFILE_PACKET: u64 = 1;
const CLASSIFY_PROFILE_IPC: u64 = 2;
const CLASSIFY_PROFILE_EVENT: u64 = 3;
const CLASSIFY_PROFILE_DMA_COMPLETION: u64 = 4;
const CLASSIFY_PROFILE_STORAGE_COMPLETION: u64 = 5;
const CLASSIFY_PROFILE_TRACE: u64 = 6;
const CLASSIFY_PROFILE_RUNTIME_TASK: u64 = 7;
const CLASSIFY_RULE_EXACT: u64 = 1;
const CLASSIFY_RULE_MASKED: u64 = 2;
const CLASSIFY_RULE_RANGE: u64 = 3;
const CLASSIFY_RULE_HASH: u64 = 4;
const CLASSIFY_FIELD_SERVICE_ID: u64 = 1;
const CLASSIFY_FIELD_DST_PORT: u64 = 2;
const CLASSIFY_FIELD_SRC_IPV4: u64 = 3;
const CLASSIFY_FIELD_DST_IPV4: u64 = 4;
const CLASSIFY_FIELD_HASH: u64 = 5;
const CLASSIFY_FIELD_PROFILE: u64 = 6;
const CLASSIFY_FIELD_DOMAIN_ID: u64 = 7;
const CLASSIFY_FIELD_INLINE0: u64 = 8;
const CLASSIFY_FIELD_INLINE1: u64 = 9;
const CLASSIFY_FIELD_INLINE2: u64 = 10;
const CLASSIFY_ACTION_MARK: u64 = 1;
const CLASSIFY_ACTION_COUNT: u64 = 2;
const CLASSIFY_ACTION_DROP: u64 = 3;
const CLASSIFY_ACTION_ROUTE: u64 = 4;
const CLASSIFY_ACTION_NEEDS_SOFTWARE: u64 = 5;
const DMA_OP_COPY: u64 = 1;
const DMA_OP_FILL: u64 = 2;
const CALL_MODE_SYNC: u64 = 0;
const CALL_MODE_ASYNC: u64 = 1;
const CALL_MODE_HANDOFF: u64 = 2;
const CALL_GATE_FLAG_CAP_PASS: u64 = 1;
const CALL_ARG_CAP_MARKER: u64 = 1 << 63;
const MAX_CAP_CALL_DEPTH: usize = 8;
const FDR_TOKEN_MARKER: u64 = 1 << 62;
const FDR_TOKEN_SHIFT: u64 = 8;
const FDR_TOKEN_INDEX_MASK: u64 = 0xff;
const CAP_RIGHT_READ: u64 = 1 << 0;
const CAP_RIGHT_WRITE: u64 = 1 << 1;
const CAP_RIGHT_SEEK: u64 = 1 << 2;
const CAP_RIGHT_STAT: u64 = 1 << 3;
const CAP_RIGHT_POLL: u64 = 1 << 4;
const CAP_RIGHT_CALL: u64 = 1 << 5;
const CAP_RIGHT_DUP: u64 = 1 << 6;
const CAP_RIGHT_REVOKE: u64 = 1 << 7;
const CAP_RIGHT_TRANSFER: u64 = 1 << 8;
const CAP_RIGHT_ALL: u64 = (1 << 9) - 1;
const CAP_DUP_FLAG_SEAL: u64 = 1 << 0;
const CAP_SEND_FLAG_MOVE: u64 = 1 << 0;
const AT_FDCWD_VALUE: u64 = -100i64 as u64;
const POLLIN_MASK: u64 = 1;
const POLLOUT_MASK: u64 = 4;
const POLLNVAL_MASK: u64 = 32;

#[repr(C)]
#[derive(Clone, Copy)]
struct HostTimespec {
    tv_sec: i64,
    tv_nsec: i64,
}

unsafe extern "C" {
    fn utimensat(
        dirfd: c_int,
        pathname: *const c_char,
        times: *const HostTimespec,
        flags: c_int,
    ) -> c_int;
    fn futimens(fd: c_int, times: *const HostTimespec) -> c_int;
}

#[derive(Debug, Clone, Copy, Default)]
struct Flags {
    zero: bool,
    negative: bool,
    greater: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct FileLockKey {
    dev: u64,
    ino: u64,
}

#[derive(Clone, Copy, Debug)]
struct AdvisoryLock {
    owner_pid: u64,
    lock_type: u64,
}

enum FdHandle {
    Stdin,
    Stdout,
    Stderr,
    MessageEndpoint,
    File(File),
    Dir {
        path: String,
        entries: Vec<String>,
        pos: usize,
    },
    PipeReader(Rc<RefCell<PipeBuffer>>),
    PipeWriter(Rc<RefCell<PipeBuffer>>),
    Counter(Rc<RefCell<u64>>),
    EventCounter {
        value: Rc<RefCell<u64>>,
        semaphore: bool,
    },
    MemoryObject {
        data: Rc<RefCell<Vec<u8>>>,
        pos: usize,
    },
    Timer(Rc<RefCell<TimerState>>),
    ClassifierTable(Rc<RefCell<ClassifierTable>>),
    ServiceletProgram(Rc<ServiceletProgram>),
    DmaBuffer {
        addr: u64,
        len: u64,
    },
    CallGate {
        entry: usize,
        domain_id: u64,
        domain_generation: u64,
        mode: u64,
        completion_fd: Option<usize>,
        completion_generation: Option<u64>,
        flags: u64,
    },
    TcpSocket {
        domain: u64,
        sock_type: u64,
        protocol: u64,
        bound_addr: Option<String>,
    },
    TcpListener {
        listener: TcpListener,
        pending: Option<TcpStream>,
    },
    TcpStream(TcpStream),
    Closed,
}

impl FdHandle {
    fn clone_handle(&self) -> Result<Self, String> {
        match self {
            FdHandle::Stdin => Ok(FdHandle::Stdin),
            FdHandle::Stdout => Ok(FdHandle::Stdout),
            FdHandle::Stderr => Ok(FdHandle::Stderr),
            FdHandle::MessageEndpoint => Ok(FdHandle::MessageEndpoint),
            FdHandle::File(file) => file
                .try_clone()
                .map(FdHandle::File)
                .map_err(|err| format!("failed to clone fd: {err}")),
            FdHandle::Dir { path, entries, pos } => Ok(FdHandle::Dir {
                path: path.clone(),
                entries: entries.clone(),
                pos: *pos,
            }),
            FdHandle::PipeReader(buffer) => Ok(FdHandle::PipeReader(Rc::clone(buffer))),
            FdHandle::PipeWriter(buffer) => Ok(FdHandle::PipeWriter(Rc::clone(buffer))),
            FdHandle::Counter(value) => Ok(FdHandle::Counter(Rc::clone(value))),
            FdHandle::EventCounter { value, semaphore } => Ok(FdHandle::EventCounter {
                value: Rc::clone(value),
                semaphore: *semaphore,
            }),
            FdHandle::MemoryObject { data, pos } => Ok(FdHandle::MemoryObject {
                data: Rc::clone(data),
                pos: *pos,
            }),
            FdHandle::Timer(timer) => Ok(FdHandle::Timer(Rc::clone(timer))),
            FdHandle::ClassifierTable(table) => Ok(FdHandle::ClassifierTable(Rc::clone(table))),
            FdHandle::ServiceletProgram(program) => {
                Ok(FdHandle::ServiceletProgram(Rc::clone(program)))
            }
            FdHandle::DmaBuffer { addr, len } => Ok(FdHandle::DmaBuffer {
                addr: *addr,
                len: *len,
            }),
            FdHandle::CallGate {
                entry,
                domain_id,
                domain_generation,
                mode,
                completion_fd,
                completion_generation,
                flags,
            } => Ok(FdHandle::CallGate {
                entry: *entry,
                domain_id: *domain_id,
                domain_generation: *domain_generation,
                mode: *mode,
                completion_fd: *completion_fd,
                completion_generation: *completion_generation,
                flags: *flags,
            }),
            FdHandle::TcpSocket {
                domain,
                sock_type,
                protocol,
                bound_addr,
            } => Ok(FdHandle::TcpSocket {
                domain: *domain,
                sock_type: *sock_type,
                protocol: *protocol,
                bound_addr: bound_addr.clone(),
            }),
            FdHandle::TcpListener { listener, pending } => Ok(FdHandle::TcpListener {
                listener: listener
                    .try_clone()
                    .map_err(|err| format!("failed to clone listener fd: {err}"))?,
                pending: match pending {
                    Some(stream) => Some(
                        stream
                            .try_clone()
                            .map_err(|err| format!("failed to clone pending stream: {err}"))?,
                    ),
                    None => None,
                },
            }),
            FdHandle::TcpStream(stream) => stream
                .try_clone()
                .map(FdHandle::TcpStream)
                .map_err(|err| format!("failed to clone TCP stream fd: {err}")),
            FdHandle::Closed => Ok(FdHandle::Closed),
        }
    }

    fn file_clone(&self) -> Result<Option<File>, String> {
        match self {
            FdHandle::File(file) => file
                .try_clone()
                .map(Some)
                .map_err(|err| format!("failed to clone file-backed fd: {err}")),
            _ => Ok(None),
        }
    }
}

#[derive(Clone, Copy)]
struct FdCapability {
    rights: u64,
    sealed: bool,
    narrowable: bool,
    revocable: bool,
    lineage: u64,
    revoked: bool,
}

impl FdCapability {
    fn full(lineage: u64) -> Self {
        Self {
            rights: CAP_RIGHT_ALL,
            sealed: false,
            narrowable: true,
            revocable: true,
            lineage,
            revoked: false,
        }
    }

    fn closed(lineage: u64) -> Self {
        Self {
            revoked: true,
            ..Self::full(lineage)
        }
    }
}

struct CapabilityPayload {
    handle: FdHandle,
    capability: FdCapability,
}

#[derive(Default)]
struct PipeBuffer {
    bytes: VecDeque<u8>,
    capabilities: VecDeque<CapabilityPayload>,
}

impl PipeBuffer {
    fn can_push_bytes(&self, len: usize) -> bool {
        self.bytes
            .len()
            .checked_add(len)
            .is_some_and(|next| next <= PIPE_BUFFER_BYTE_LIMIT)
    }

    fn can_push_capability(&self) -> bool {
        self.capabilities.len() < PIPE_CAPABILITY_LIMIT
    }

    fn push_bytes(&mut self, data: &[u8]) -> Result<(), u64> {
        if !self.can_push_bytes(data.len()) {
            return Err(11);
        }
        self.bytes.extend(data.iter().copied());
        Ok(())
    }

    fn push_capability(&mut self, payload: CapabilityPayload) -> Result<(), u64> {
        if !self.can_push_capability() {
            return Err(11);
        }
        self.capabilities.push_back(payload);
        Ok(())
    }
}

#[derive(Clone)]
struct ClassifierRule {
    kind: u64,
    field: u64,
    value: u64,
    mask_or_end: u64,
    action: u64,
    action_arg: u64,
    hash_mod: u64,
}

#[derive(Default)]
struct ClassifierCounters {
    hits: u64,
    drops: u64,
    routes: u64,
    malformed: u64,
    fallback: u64,
}

struct ClassifierTable {
    rules: Vec<ClassifierRule>,
    allowed_queues: Vec<ClassifierAllowedQueue>,
    counters: ClassifierCounters,
}

struct ClassifierAllowedQueue {
    token: u64,
    fd: usize,
    generation: u64,
}

struct ClassifierEnvelope {
    profile: u64,
    source: u64,
    source_generation: u64,
    domain_id: u64,
    record_ptr: u64,
    record_len: usize,
    inline0: u64,
    inline1: u64,
    inline2: u64,
}

#[derive(Default)]
struct ClassifierParsedFields {
    src_ipv4: Option<u64>,
    dst_ipv4: Option<u64>,
    src_port: Option<u64>,
    dst_port: Option<u64>,
    hash: u64,
    needs_software: bool,
}

enum ClassifyParseError {
    Malformed,
    NeedsSoftware,
}

#[derive(Default)]
struct TimerState {
    remaining: u64,
    interval: u64,
    expirations: u64,
}

#[derive(Clone)]
#[allow(dead_code)]
struct ServiceletProgram {
    program_len: u64,
    isa_subset: u64,
    instruction_limit: u64,
    cycle_limit: u64,
    record_read_limit: u64,
    action_write_limit: u64,
    flags: u64,
    owner_domain_id: u64,
    owner_generation: u64,
}

struct Vma {
    start: u64,
    len: u64,
    prot: u64,
    file: Option<File>,
    file_offset: u64,
    resident: bool,
    guard: bool,
}

impl Vma {
    fn anonymous(start: u64, len: u64, prot: u64) -> Self {
        Self {
            start,
            len,
            prot,
            file: None,
            file_offset: 0,
            resident: true,
            guard: false,
        }
    }

    fn guard(start: u64, len: u64) -> Self {
        Self {
            start,
            len,
            prot: 0,
            file: None,
            file_offset: 0,
            resident: true,
            guard: true,
        }
    }

    fn contains(&self, addr: u64, len: usize) -> bool {
        let Some(end) = addr.checked_add(len as u64) else {
            return false;
        };
        let Some(vma_end) = self.start.checked_add(self.len) else {
            return false;
        };
        addr >= self.start && end <= vma_end
    }

    fn clone_vma(&self) -> Result<Self, String> {
        Ok(Self {
            start: self.start,
            len: self.len,
            prot: self.prot,
            file: match &self.file {
                Some(file) => Some(
                    file.try_clone()
                        .map_err(|err| format!("failed to clone VMA file: {err}"))?,
                ),
                None => None,
            },
            file_offset: self.file_offset,
            resident: self.resident,
            guard: self.guard,
        })
    }
}

#[derive(Clone, Copy)]
struct Allocation {
    len: usize,
    guard_before: Option<u64>,
    guard_after: Option<u64>,
}

#[derive(Clone, Copy)]
struct ProcessLayout {
    stack_top: u64,
    heap_base: u64,
    mmap_base: u64,
}

impl ProcessLayout {
    fn for_process(pid: u64, domain_id: u64, aslr_enabled: bool) -> Self {
        if !aslr_enabled {
            return Self {
                stack_top: STACK_TOP,
                heap_base: HEAP_BASE,
                mmap_base: MMAP_BASE,
            };
        }
        Self {
            stack_top: STACK_TOP
                - Self::page_offset(pid, domain_id, 0x5a17_51ac_57ac_0001, ASLR_STACK_PAGES),
            heap_base: HEAP_BASE
                + Self::page_offset(pid, domain_id, 0x5a17_51ac_481e_0002, ASLR_HEAP_PAGES),
            mmap_base: MMAP_BASE
                + Self::page_offset(pid, domain_id, 0x5a17_51ac_aa9d_0003, ASLR_MMAP_PAGES),
        }
    }

    fn page_offset(pid: u64, domain_id: u64, salt: u64, pages: u64) -> u64 {
        let mut x = pid
            .wrapping_mul(0x9e37_79b9_7f4a_7c15)
            .wrapping_add(domain_id.rotate_left(17))
            ^ salt;
        x ^= x >> 30;
        x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
        x ^= x >> 27;
        x = x.wrapping_mul(0x94d0_49bb_1331_11eb);
        x ^= x >> 31;
        ((x % pages) + 1) * ASLR_PAGE
    }
}

#[derive(Clone, Copy)]
struct DomainLimits {
    cpu: u64,
    memory: u64,
    pids: u64,
    fdrs: u64,
}

impl DomainLimits {
    fn root() -> Self {
        Self {
            cpu: u64::MAX,
            memory: u64::MAX,
            pids: u64::MAX,
            fdrs: u64::MAX,
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
struct DomainUsage {
    cpu: u64,
    memory: u64,
    pids: u64,
    fdrs: u64,
}

#[derive(Clone, Copy)]
struct DomainSecurityPolicy {
    aslr_enabled: bool,
    allow_wx: bool,
    allow_jit_transition: bool,
    entropy_quota: u64,
    dma_allowed: bool,
    hardening_profile: u64,
    executable_source_policy: u64,
}

impl DomainSecurityPolicy {
    fn root() -> Self {
        Self {
            aslr_enabled: true,
            allow_wx: false,
            allow_jit_transition: true,
            entropy_quota: u64::MAX,
            dma_allowed: true,
            hardening_profile: 0,
            executable_source_policy: u64::MAX,
        }
    }
}

struct ResourceDomain {
    id: u64,
    generation: u64,
    parent: Option<u64>,
    children: Vec<u64>,
    profile: u64,
    limits: DomainLimits,
    capability_mask: u64,
    upcall_mask: u64,
    security: DomainSecurityPolicy,
    frozen: bool,
    destroyed: bool,
    cpu_ticks: u64,
}

impl ResourceDomain {
    fn root() -> Self {
        Self {
            id: ROOT_DOMAIN_ID,
            generation: 1,
            parent: None,
            children: Vec::new(),
            profile: 16,
            limits: DomainLimits::root(),
            capability_mask: u64::MAX,
            upcall_mask: u64::MAX,
            security: DomainSecurityPolicy::root(),
            frozen: false,
            destroyed: false,
            cpu_ticks: 0,
        }
    }
}

struct Process {
    pid: u64,
    parent_pid: Option<u64>,
    domain_id: u64,
    program: Program,
    fds: Vec<FdHandle>,
    fd_generations: Vec<u64>,
    fd_capabilities: Vec<FdCapability>,
    memory: Vec<u8>,
    vmas: Vec<Vma>,
    stack_top: u64,
    heap_next: u64,
    mmap_next: u64,
    allocations: HashMap<u64, Allocation>,
    uid: u64,
    gid: u64,
    sigmask: u64,
    signal_handlers: HashMap<u64, SignalDisposition>,
    pending_events: VecDeque<NativeEvent>,
    inbox: VecDeque<(u64, u64)>,
    ucode_ports: HashMap<u64, u8>,
    errno: u64,
    namespace_root: Option<PathBuf>,
    cwd: PathBuf,
}

#[derive(Clone, Copy)]
enum SignalDisposition {
    Handler(usize),
    Ignore,
}

impl Process {
    fn new(
        pid: u64,
        parent_pid: Option<u64>,
        domain_id: u64,
        program: Program,
        layout: ProcessLayout,
    ) -> Self {
        let mut fds = Vec::with_capacity(FDR_COUNT);
        fds.push(FdHandle::Stdin);
        fds.push(FdHandle::Stdout);
        fds.push(FdHandle::Stderr);
        for _ in 3..FDR_COUNT {
            fds.push(FdHandle::Closed);
        }
        fds[MESSAGE_ENDPOINT_FD] = FdHandle::MessageEndpoint;
        let fd_generations = vec![1; FDR_COUNT];
        let mut fd_capabilities = Vec::with_capacity(FDR_COUNT);
        for idx in 0..FDR_COUNT {
            let lineage = idx as u64 + 1;
            if matches!(fds[idx], FdHandle::Closed) {
                fd_capabilities.push(FdCapability::closed(lineage));
            } else {
                fd_capabilities.push(FdCapability::full(lineage));
            }
        }

        let mut memory = vec![0; MEMORY_SIZE];
        let data_start = DATA_BASE as usize;
        let data_end = data_start + program.data.len();
        if data_end <= memory.len() {
            memory[data_start..data_end].copy_from_slice(&program.data);
        }

        let mut vmas = vec![
            Vma::anonymous(DATA_BASE, program.data.len().max(1) as u64, 0b11),
            Vma::anonymous(layout.stack_top - STACK_SIZE, STACK_SIZE, 0b11),
            Vma::anonymous(ARG_BASE, ARG_SIZE, 0b11),
        ];
        vmas.sort_by_key(|vma| vma.start);

        Self {
            pid,
            parent_pid,
            domain_id,
            program,
            fds,
            fd_generations,
            fd_capabilities,
            memory,
            vmas,
            stack_top: layout.stack_top,
            heap_next: layout.heap_base,
            mmap_next: layout.mmap_base,
            allocations: HashMap::new(),
            uid: if pid == 1 { 0 } else { 1000 },
            gid: if pid == 1 { 0 } else { 1000 },
            sigmask: 0,
            signal_handlers: HashMap::new(),
            pending_events: VecDeque::new(),
            inbox: VecDeque::new(),
            ucode_ports: HashMap::new(),
            errno: 0,
            namespace_root: Some(PathBuf::from("/")),
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    fn fork_clone(&self, pid: u64) -> Result<Self, String> {
        let mut fds = Vec::with_capacity(FDR_COUNT);
        for fd in &self.fds {
            fds.push(fd.clone_handle()?);
        }
        let mut vmas = Vec::with_capacity(self.vmas.len());
        for vma in &self.vmas {
            vmas.push(vma.clone_vma()?);
        }
        Ok(Self {
            pid,
            parent_pid: Some(self.pid),
            domain_id: self.domain_id,
            program: self.program.clone(),
            fds,
            fd_generations: self.fd_generations.clone(),
            fd_capabilities: self.fd_capabilities.clone(),
            memory: self.memory.clone(),
            vmas,
            stack_top: self.stack_top,
            heap_next: self.heap_next,
            mmap_next: self.mmap_next,
            allocations: self.allocations.clone(),
            uid: self.uid,
            gid: self.gid,
            sigmask: self.sigmask,
            signal_handlers: self.signal_handlers.clone(),
            pending_events: VecDeque::new(),
            inbox: VecDeque::new(),
            ucode_ports: self.ucode_ports.clone(),
            errno: self.errno,
            namespace_root: self.namespace_root.clone(),
            cwd: self.cwd.clone(),
        })
    }

    fn exec(&mut self, program: Program, layout: ProcessLayout) {
        let pid = self.pid;
        let parent_pid = self.parent_pid;
        let domain_id = self.domain_id;
        let mut replacement = Process::new(pid, parent_pid, domain_id, program, layout);
        replacement.fds = std::mem::take(&mut self.fds);
        replacement.fd_generations = std::mem::take(&mut self.fd_generations);
        replacement.fd_capabilities = std::mem::take(&mut self.fd_capabilities);
        replacement.uid = self.uid;
        replacement.gid = self.gid;
        replacement.sigmask = self.sigmask;
        replacement.namespace_root = self.namespace_root.clone();
        replacement.cwd = self.cwd.clone();
        replacement.errno = self.errno;
        replacement.ucode_ports = std::mem::take(&mut self.ucode_ports);
        *self = replacement;
    }
}

#[derive(Clone)]
struct SavedSignalContext {
    ip: usize,
    regs: [u64; GPR_COUNT],
    flags: Flags,
}

#[derive(Clone)]
struct CallContinuation {
    return_ip: usize,
    result_reg: Reg,
    caller_domain_id: u64,
}

#[derive(Clone)]
struct Thread {
    tid: u64,
    pid: u64,
    thread_pointer: u64,
    regs: [u64; GPR_COUNT],
    fregs: [u64; FPR_COUNT],
    vregs: [u128; VR_COUNT],
    ip: usize,
    flags: Flags,
    signal_stack: Vec<SavedSignalContext>,
    cap_call_stack: Vec<CallContinuation>,
}

impl Thread {
    fn new(tid: u64, pid: u64, stack_top: u64) -> Self {
        let mut regs = [0; GPR_COUNT];
        regs[31] = stack_top - CALL_FRAME_SIZE;
        Self {
            tid,
            pid,
            thread_pointer: 0,
            regs,
            fregs: [0; FPR_COUNT],
            vregs: [0; VR_COUNT],
            ip: 0,
            flags: Flags::default(),
            signal_stack: Vec::new(),
            cap_call_stack: Vec::new(),
        }
    }
}

#[derive(Clone, Copy)]
struct FdWaiter {
    tid: u64,
    fd: usize,
    generation: u64,
    mask: u64,
    result: Option<Reg>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FdWaiterState {
    Pending,
    Ready,
    Stale,
    Error(u64),
}

pub struct Machine {
    processes: HashMap<u64, Process>,
    threads: HashMap<u64, Thread>,
    domains: HashMap<u64, ResourceDomain>,
    ready: VecDeque<u64>,
    domain_parked: VecDeque<u64>,
    sleepers: Vec<(u64, u64)>,
    alarms: Vec<(u64, u64)>,
    futex_waiters: HashMap<u64, VecDeque<u64>>,
    thread_join_waiters: HashMap<u64, VecDeque<u64>>,
    child_waiters: HashMap<u64, VecDeque<u64>>,
    completed_threads: HashMap<u64, u64>,
    completed_children: HashMap<(u64, u64), i32>,
    fd_waiters: Vec<FdWaiter>,
    current_tid: u64,
    next_pid: u64,
    next_tid: u64,
    next_domain_id: u64,
    next_call_op_id: u64,
    next_cap_lineage: u64,
    advisory_locks: HashMap<FileLockKey, AdvisoryLock>,
    random_state: u64,
    last_exit: i32,
}

impl Machine {
    pub fn new(program: Program) -> Self {
        let root_pid = 1;
        let root_tid = 1;
        let layout = ProcessLayout::for_process(root_pid, ROOT_DOMAIN_ID, true);
        let process = Process::new(root_pid, None, ROOT_DOMAIN_ID, program, layout);
        let thread = Thread::new(root_tid, root_pid, layout.stack_top);

        let mut processes = HashMap::new();
        processes.insert(root_pid, process);
        let mut threads = HashMap::new();
        threads.insert(root_tid, thread);
        let mut domains = HashMap::new();
        domains.insert(ROOT_DOMAIN_ID, ResourceDomain::root());

        let mut ready = VecDeque::new();
        ready.push_back(root_tid);

        Self {
            processes,
            threads,
            domains,
            ready,
            domain_parked: VecDeque::new(),
            sleepers: Vec::new(),
            alarms: Vec::new(),
            futex_waiters: HashMap::new(),
            thread_join_waiters: HashMap::new(),
            child_waiters: HashMap::new(),
            completed_threads: HashMap::new(),
            completed_children: HashMap::new(),
            fd_waiters: Vec::new(),
            current_tid: root_tid,
            next_pid: 2,
            next_tid: 2,
            next_domain_id: 2,
            next_call_op_id: 1,
            next_cap_lineage: FDR_COUNT as u64 + 1,
            advisory_locks: HashMap::new(),
            random_state: 0x4d59_5df4_d0f3_3173,
            last_exit: 0,
        }
    }

    pub fn set_args(&mut self, args: &[String]) -> Result<(), String> {
        self.set_process_entry(args, &[])
    }

    pub fn set_namespace_root(&mut self, root: impl Into<PathBuf>) -> Result<(), String> {
        let root = root.into();
        let root = fs::canonicalize(&root)
            .map_err(|err| format!("failed to resolve namespace root {}: {err}", root.display()))?;
        if !root.is_dir() {
            return Err(format!(
                "namespace root {} is not a directory",
                root.display()
            ));
        }
        let process = self.process_mut()?;
        process.namespace_root = Some(root.clone());
        process.cwd = root;
        Ok(())
    }

    pub fn set_process_entry(&mut self, args: &[String], env: &[String]) -> Result<(), String> {
        let pid = self.thread()?.pid;
        let arg_page = Self::build_process_entry_page(args, env)?;
        self.install_process_entry_page(pid, &arg_page)
    }

    fn build_process_entry_page(args: &[String], env: &[String]) -> Result<Vec<u8>, String> {
        let argc_addr = ARG_BASE as usize;
        let argv_addr = (ARG_BASE + 8) as usize;
        let argv_slots = args
            .len()
            .checked_add(1)
            .ok_or_else(|| "argv table size overflow".to_string())?;
        let argv_bytes = argv_slots
            .checked_mul(8)
            .ok_or_else(|| "argv table size overflow".to_string())?;
        let envp_addr = argv_addr
            .checked_add(argv_bytes)
            .ok_or_else(|| "argv table address overflow".to_string())?;
        let env_slots = env
            .len()
            .checked_add(1)
            .ok_or_else(|| "envp table size overflow".to_string())?;
        let env_bytes = env_slots
            .checked_mul(8)
            .ok_or_else(|| "envp table size overflow".to_string())?;
        let mut str_addr = ARG_BASE + 0x1000;
        let arg_page_start = ARG_BASE as usize;
        let mut arg_page = vec![0u8; ARG_SIZE as usize];
        if envp_addr
            .checked_add(env_bytes)
            .ok_or_else(|| "envp table address overflow".to_string())?
            > str_addr as usize
        {
            return Err("process entry pointer table exceeds reserved argument area".to_string());
        }
        let arg_page_len = arg_page.len();
        let page_offset = |addr: usize| -> Result<usize, String> {
            addr.checked_sub(arg_page_start)
                .filter(|offset| *offset < arg_page_len)
                .ok_or_else(|| "process entry address outside argument page".to_string())
        };
        let argc_off = page_offset(argc_addr)?;
        arg_page[argc_off..argc_off + 8].copy_from_slice(&(args.len() as u64).to_le_bytes());
        for (idx, arg) in args.iter().enumerate() {
            let ptr_slot = argv_addr
                .checked_add(
                    idx.checked_mul(8)
                        .ok_or_else(|| "argv slot offset overflow".to_string())?,
                )
                .ok_or_else(|| "argv slot address overflow".to_string())?;
            let ptr_slot_off = page_offset(ptr_slot)?;
            arg_page[ptr_slot_off..ptr_slot_off + 8].copy_from_slice(&str_addr.to_le_bytes());
            let bytes = arg.as_bytes();
            let start = str_addr as usize;
            let end = start
                .checked_add(bytes.len())
                .ok_or_else(|| "argv data address overflow".to_string())?;
            if end
                .checked_add(1)
                .ok_or_else(|| "argv data address overflow".to_string())?
                >= (ARG_BASE + ARG_SIZE) as usize
            {
                return Err("argv data exceeds emulated argument page".to_string());
            }
            let start_off = page_offset(start)?;
            let end_off = page_offset(end)?;
            arg_page[start_off..end_off].copy_from_slice(bytes);
            arg_page[end_off] = 0;
            str_addr = str_addr
                .checked_add(bytes.len() as u64)
                .and_then(|addr| addr.checked_add(1))
                .ok_or_else(|| "argv string cursor overflow".to_string())?;
        }
        let null_slot = argv_addr
            .checked_add(
                args.len()
                    .checked_mul(8)
                    .ok_or_else(|| "argv null slot offset overflow".to_string())?,
            )
            .ok_or_else(|| "argv null slot address overflow".to_string())?;
        let null_slot_off = page_offset(null_slot)?;
        arg_page[null_slot_off..null_slot_off + 8].copy_from_slice(&0u64.to_le_bytes());
        for (idx, item) in env.iter().enumerate() {
            let ptr_slot = envp_addr
                .checked_add(
                    idx.checked_mul(8)
                        .ok_or_else(|| "envp slot offset overflow".to_string())?,
                )
                .ok_or_else(|| "envp slot address overflow".to_string())?;
            let ptr_slot_off = page_offset(ptr_slot)?;
            arg_page[ptr_slot_off..ptr_slot_off + 8].copy_from_slice(&str_addr.to_le_bytes());
            let bytes = item.as_bytes();
            let start = str_addr as usize;
            let end = start
                .checked_add(bytes.len())
                .ok_or_else(|| "envp data address overflow".to_string())?;
            if end
                .checked_add(1)
                .ok_or_else(|| "envp data address overflow".to_string())?
                >= (ARG_BASE + ARG_SIZE) as usize
            {
                return Err("envp data exceeds emulated argument page".to_string());
            }
            let start_off = page_offset(start)?;
            let end_off = page_offset(end)?;
            arg_page[start_off..end_off].copy_from_slice(bytes);
            arg_page[end_off] = 0;
            str_addr = str_addr
                .checked_add(bytes.len() as u64)
                .and_then(|addr| addr.checked_add(1))
                .ok_or_else(|| "envp string cursor overflow".to_string())?;
        }
        let null_slot = envp_addr
            .checked_add(
                env.len()
                    .checked_mul(8)
                    .ok_or_else(|| "envp null slot offset overflow".to_string())?,
            )
            .ok_or_else(|| "envp null slot address overflow".to_string())?;
        let null_slot_off = page_offset(null_slot)?;
        arg_page[null_slot_off..null_slot_off + 8].copy_from_slice(&0u64.to_le_bytes());
        Ok(arg_page)
    }

    fn install_process_entry_page(&mut self, pid: u64, arg_page: &[u8]) -> Result<(), String> {
        if arg_page.len() != ARG_SIZE as usize {
            return Err("process entry page has invalid size".to_string());
        }
        let arg_page_start = ARG_BASE as usize;
        let arg_page_end = (ARG_BASE + ARG_SIZE) as usize;
        let process = self
            .processes
            .get_mut(&pid)
            .ok_or_else(|| format!("missing process {pid}"))?;
        process.memory[arg_page_start..arg_page_end].copy_from_slice(&arg_page);
        Ok(())
    }

    pub fn run(&mut self) -> Result<i32, String> {
        let mut steps = 0usize;
        while !self.threads.is_empty() {
            if steps > 200_000_000 {
                return Err("execution step limit exceeded".to_string());
            }
            steps += 1;
            self.tick_sleepers();
            self.tick_alarms();
            self.tick_timers();
            self.poll_fd_waiters();

            let Some(tid) = self.ready.pop_front() else {
                if self.sleepers.is_empty() && self.alarms.is_empty() && self.fd_waiters.is_empty()
                {
                    return Err("hardware runqueue deadlock: no ready threads".to_string());
                }
                if !self.fd_waiters.is_empty() {
                    thread::sleep(Duration::from_millis(10));
                }
                continue;
            };
            if !self.threads.contains_key(&tid) {
                continue;
            }
            self.current_tid = tid;
            if self.thread_domain_frozen(tid)? {
                if !self.domain_parked.contains(&tid) {
                    self.domain_parked.push_back(tid);
                }
                continue;
            }
            self.deliver_signal_if_needed()?;
            if !self.threads.contains_key(&tid) {
                continue;
            }

            let (ip, instr) = {
                let thread = self.thread()?;
                let process = self
                    .processes
                    .get(&thread.pid)
                    .ok_or_else(|| format!("missing process {}", thread.pid))?;
                let Some(instr) = process.program.instructions.get(thread.ip).cloned() else {
                    if let Some(fault) = self.instruction_fetch_fault(thread.ip as u64)? {
                        return Err(fault);
                    }
                    self.exit_current(0)?;
                    continue;
                };
                (thread.ip, instr)
            };
            self.thread_mut()?.ip += 1;
            self.charge_cpu_tick()?;
            let keep_ready = self.exec(instr.clone()).map_err(|err| {
                let context = self.fault_context(tid);
                format!("{err} at tid {tid} ip {ip}: {instr:?}{context}")
            })?;
            if keep_ready && self.threads.contains_key(&tid) {
                self.wake_thread(tid);
            }
        }
        Ok(self.last_exit)
    }

    fn exec(&mut self, instr: Instr) -> Result<bool, String> {
        match instr {
            Instr::Nop | Instr::Fence => {}
            Instr::Isync(result, addr, len) => {
                let addr = self.read_reg(addr)?;
                let len = self.read_reg(len)?;
                self.isync_range(result, addr, len)?;
            }
            Instr::Li(dst, value) => {
                let v = self.resolve_value(value)?;
                self.write_reg(dst, v)?;
            }
            Instr::Mov(dst, src) => self.write_reg(dst, self.read_reg(src)?)?,
            Instr::Add(dst, a, b) => {
                self.write_reg(dst, self.read_reg(a)?.wrapping_add(self.read_reg(b)?))?
            }
            Instr::Sub(dst, a, b) => {
                self.write_reg(dst, self.read_reg(a)?.wrapping_sub(self.read_reg(b)?))?
            }
            Instr::Mul(dst, a, b) => {
                self.write_reg(dst, self.read_reg(a)?.wrapping_mul(self.read_reg(b)?))?
            }
            Instr::Div(dst, a, b) => {
                Self::ensure_result_reg_writable(dst)?;
                let divisor = self.read_reg(b)?;
                if divisor == 0 {
                    self.raise_current_signal(SIGFPE)?;
                    return Ok(true);
                }
                self.write_reg(dst, self.read_reg(a)? / divisor)?;
            }
            Instr::And(dst, a, b) => self.write_reg(dst, self.read_reg(a)? & self.read_reg(b)?)?,
            Instr::Or(dst, a, b) => self.write_reg(dst, self.read_reg(a)? | self.read_reg(b)?)?,
            Instr::Xor(dst, a, b) => self.write_reg(dst, self.read_reg(a)? ^ self.read_reg(b)?)?,
            Instr::Not(dst, src) => self.write_reg(dst, !self.read_reg(src)?)?,
            Instr::Lsl(dst, a, b) => {
                self.write_reg(dst, self.read_reg(a)? << (self.read_reg(b)? & 63))?
            }
            Instr::Lsr(dst, a, b) => {
                self.write_reg(dst, self.read_reg(a)? >> (self.read_reg(b)? & 63))?
            }
            Instr::Asr(dst, a, b) => self.write_reg(
                dst,
                ((self.read_reg(a)? as i64) >> (self.read_reg(b)? & 63)) as u64,
            )?,
            Instr::Cmp(a, b) => {
                let lhs = self.read_reg(a)? as i64;
                let rhs = self.read_reg(b)? as i64;
                self.thread_mut()?.flags = Flags {
                    zero: lhs == rhs,
                    negative: lhs < rhs,
                    greater: lhs > rhs,
                };
            }
            Instr::Jmp(target) => {
                let ip = self.resolve_target(target)?;
                self.thread_mut()?.ip = ip;
            }
            Instr::Branch(condition, target) => {
                if self.condition(condition)? {
                    let ip = self.resolve_target(target)?;
                    self.thread_mut()?.ip = ip;
                }
            }
            Instr::Call(target) => {
                let ret = self.thread()?.ip as u64;
                let sp = self.thread()?.regs[31].wrapping_sub(CALL_FRAME_SIZE);
                if std::env::var_os("LNP64_TRACE_CALLS").is_some() {
                    let thread = self.thread()?;
                    eprintln!(
                        "CALL {target:?} ret={ret} sp={sp:#x} r1={} r2={} r3={}",
                        thread.regs[1], thread.regs[2], thread.regs[3]
                    );
                }
                let ip = self.resolve_target(target)?;
                self.store_u64(sp, ret)?;
                self.thread_mut()?.regs[31] = sp;
                self.thread_mut()?.ip = ip;
            }
            Instr::CallReg(target) => {
                let ip = self.read_reg(target)? as usize;
                let ret = self.thread()?.ip as u64;
                let sp = self.thread()?.regs[31].wrapping_sub(CALL_FRAME_SIZE);
                if std::env::var_os("LNP64_TRACE_CALLS").is_some() {
                    let thread = self.thread()?;
                    eprintln!(
                        "CALL_REG {ip} ret={ret} sp={sp:#x} r1={} r2={} r3={}",
                        thread.regs[1], thread.regs[2], thread.regs[3]
                    );
                }
                self.store_u64(sp, ret)?;
                self.thread_mut()?.regs[31] = sp;
                self.thread_mut()?.ip = ip;
            }
            Instr::Ret => {
                let sp = self.thread()?.regs[31];
                let next = self.load_u64(sp)?;
                if std::env::var_os("LNP64_TRACE_CALLS").is_some() {
                    eprintln!("RET next={next} sp={sp:#x}");
                }
                self.thread_mut()?.regs[31] = sp.wrapping_add(CALL_FRAME_SIZE);
                self.thread_mut()?.ip = next as usize;
            }
            Instr::Ld(dst, mem, width) => {
                let addr = self.resolve_mem(mem)?;
                let value = self.load_width(addr, width)?;
                self.write_reg(dst, value)?;
            }
            Instr::St(mem, src, width) => {
                let addr = self.resolve_mem(mem)?;
                self.store_width(addr, self.read_reg(src)?, width)?;
            }
            Instr::Pull(result, fd, buf, len) => {
                Self::ensure_result_reg_writable(result)?;
                self.require_domain_cap(DOMAIN_CAP_IO)?;
                if fd.0 == MESSAGE_ENDPOINT_FD {
                    let Some((v1, v2)) = self.process_mut()?.inbox.pop_front() else {
                        self.thread_mut()?.ip = self.thread()?.ip.saturating_sub(1);
                        self.ready.retain(|tid| *tid != self.current_tid);
                        return Ok(false);
                    };
                    self.complete_reg_ok(result, v1)?;
                    self.write_reg(Reg(30), v2)?;
                } else {
                    let addr = self.read_reg(buf)?;
                    let len = self.read_reg(len)? as usize;
                    if let Some(count) = self.read_fd_index(fd.0, addr, len)? {
                        self.complete_reg_ok(result, count as u64)?;
                    } else {
                        let errno = self.process()?.errno;
                        self.complete_reg_err(result, errno)?;
                    }
                }
            }
            Instr::Push(result, fd, buf, len) => {
                Self::ensure_result_reg_writable(result)?;
                self.require_domain_cap(DOMAIN_CAP_IO)?;
                let addr = self.read_reg(buf)?;
                let len = self.read_reg(len)? as usize;
                self.write_fd_index(fd.0, addr, len)?;
                self.write_reg(result, self.read_reg(Reg(1))?)?;
            }
            Instr::Await(result, fd, mask) => {
                Self::ensure_result_reg_writable(result)?;
                let mask = self.read_reg(mask)?;
                let Some(ready) = self.await_fd_ready_or_error(result, fd.0, mask)? else {
                    return Ok(true);
                };
                if !ready {
                    self.push_fd_waiter(fd.0, mask, Some(result))?;
                    self.ready.retain(|tid| *tid != self.current_tid);
                    return Ok(false);
                }
                self.complete_reg_ok(result, 0)?;
            }
            Instr::AwaitDyn(result, fd_reg, mask) => {
                Self::ensure_result_reg_writable(result)?;
                let fd = self.read_reg(fd_reg)?;
                let mask = self.read_reg(mask)?;
                let Some(fd) = self.checked_fd_index(fd)? else {
                    self.complete_reg_err(result, 9)?;
                    return Ok(true);
                };
                let Some(ready) = self.await_fd_ready_or_error(result, fd, mask)? else {
                    return Ok(true);
                };
                if !ready {
                    self.push_fd_waiter(fd, mask, Some(result))?;
                    self.ready.retain(|tid| *tid != self.current_tid);
                    return Ok(false);
                }
                self.complete_reg_ok(result, 0)?;
            }
            Instr::PollFd(result, fd, events) => {
                Self::ensure_result_reg_writable(result)?;
                let events = self.read_reg(events)?;
                let revents = self.poll_fd_mask(fd.0 as u64, events)?;
                self.complete_reg_ok(result, revents)?;
            }
            Instr::PollFdDyn(result, fd_reg, events) => {
                Self::ensure_result_reg_writable(result)?;
                let fd = self.read_reg(fd_reg)?;
                let events = self.read_reg(events)?;
                let revents = match self.checked_fd_index(fd)? {
                    Some(fd) => self.poll_fd_index_mask(fd, events)?,
                    None => POLLNVAL_MASK,
                };
                self.complete_reg_ok(result, revents)?;
            }
            Instr::Alloc(dst, bytes_reg) => {
                Self::ensure_result_reg_writable(dst)?;
                let len = (self.read_reg(bytes_reg)? as usize).max(1);
                let addr = self.alloc_heap(len, 64, false)?;
                self.complete_reg_ok(dst, addr)?;
            }
            Instr::AllocEx(dst, bytes_reg, align_reg) => {
                Self::ensure_result_reg_writable(dst)?;
                let len = (self.read_reg(bytes_reg)? as usize).max(1);
                let align = self.read_reg(align_reg)?.clamp(1, 4096).next_power_of_two();
                let addr = self.alloc_heap(len, align, true)?;
                self.complete_reg_ok(dst, addr)?;
            }
            Instr::AllocSize(dst, ptr_reg) => {
                Self::ensure_result_reg_writable(dst)?;
                let ptr = self.read_reg(ptr_reg)?;
                let size = self
                    .process()?
                    .allocations
                    .get(&ptr)
                    .map(|allocation| allocation.len)
                    .unwrap_or(0);
                self.complete_reg_ok(dst, size as u64)?;
            }
            Instr::Free(ptr) => {
                let ptr = self.read_reg(ptr)?;
                let process = self.process_mut()?;
                if let Some(allocation) = process.allocations.remove(&ptr) {
                    process.vmas.retain(|vma| {
                        vma.start != ptr
                            && Some(vma.start) != allocation.guard_before
                            && Some(vma.start) != allocation.guard_after
                    });
                }
            }
            Instr::OpenFd(dst, path_reg, flags_reg) => {
                self.require_domain_cap(DOMAIN_CAP_FDR)?;
                if !self.check_domain_budget(0, 0, 0, self.fd_slot_delta(dst.0)?)? {
                    return Ok(true);
                }
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let Some(path) = self.resolve_process_path_or_errno(&path)? else {
                    return Ok(true);
                };
                let flags = self.read_reg(flags_reg)?;
                match Self::open_fd_handle(&path, flags) {
                    Ok(handle) => {
                        self.bump_fd_generation(dst.0)?;
                        self.process_mut()?.fds[dst.0] = handle;
                        let capability = self.fresh_fd_capability();
                        self.install_fd_capability(dst.0, capability)?;
                        self.set_status_ok()?;
                    }
                    Err(_) => self.set_status_errno(5)?,
                }
            }
            Instr::OpenFdDyn(dst_reg, path_reg, flags_reg) => {
                Self::ensure_result_reg_writable(dst_reg)?;
                self.require_domain_cap(DOMAIN_CAP_FDR)?;
                if !self.check_domain_budget(0, 0, 0, 1)? {
                    self.write_reg(dst_reg, -1i64 as u64)?;
                    return Ok(true);
                }
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let Some(path) = self.resolve_process_path_or_errno(&path)? else {
                    self.write_reg(dst_reg, -1i64 as u64)?;
                    return Ok(true);
                };
                let flags = self.read_reg(flags_reg)?;
                match Self::open_fd_handle(&path, flags) {
                    Ok(handle) => match self.alloc_fd_handle(handle)? {
                        Some(fd) => {
                            let token = self.fd_token(fd)?;
                            self.set_errno(0)?;
                            self.write_reg(dst_reg, token)?;
                            self.write_reg(Reg(1), token)?;
                        }
                        None => self.write_reg(dst_reg, -1i64 as u64)?,
                    },
                    Err(_) => {
                        self.write_reg(dst_reg, -1i64 as u64)?;
                        self.set_status_errno(5)?;
                    }
                }
            }
            Instr::OpenAtDyn(dst_reg, dir_reg, path_reg, flags_reg) => {
                Self::ensure_result_reg_writable(dst_reg)?;
                self.require_domain_cap(DOMAIN_CAP_FDR)?;
                if !self.check_domain_budget(0, 0, 0, 1)? {
                    self.write_reg(dst_reg, -1i64 as u64)?;
                    return Ok(true);
                }
                let dir_value = self.read_reg(dir_reg)?;
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let Some(path) = self.resolve_process_path_at_or_errno(dir_value, &path)? else {
                    self.write_reg(dst_reg, -1i64 as u64)?;
                    return Ok(true);
                };
                let flags = self.read_reg(flags_reg)?;
                match Self::open_fd_handle(&path, flags) {
                    Ok(handle) => match self.alloc_fd_handle(handle)? {
                        Some(fd) => {
                            let token = self.fd_token(fd)?;
                            self.set_errno(0)?;
                            self.write_reg(dst_reg, token)?;
                            self.write_reg(Reg(1), token)?;
                        }
                        None => self.write_reg(dst_reg, -1i64 as u64)?,
                    },
                    Err(_) => {
                        self.write_reg(dst_reg, -1i64 as u64)?;
                        self.set_status_errno(5)?;
                    }
                }
            }
            Instr::OpenDir(dst, path_reg, _flags_reg) => {
                self.require_domain_cap(DOMAIN_CAP_FDR)?;
                if !self.check_domain_budget(0, 0, 0, self.fd_slot_delta(dst.0)?)? {
                    return Ok(true);
                }
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let Some(path) = self.resolve_process_path_or_errno(&path)? else {
                    return Ok(true);
                };
                match Self::open_dir_handle(&path) {
                    Ok(handle) => {
                        self.bump_fd_generation(dst.0)?;
                        self.process_mut()?.fds[dst.0] = handle;
                        let capability = self.fresh_fd_capability();
                        self.install_fd_capability(dst.0, capability)?;
                        self.set_status_ok()?;
                    }
                    Err(err) => self.set_status_io_error(err)?,
                }
            }
            Instr::OpenDirDyn(dst_reg, path_reg, _flags_reg) => {
                Self::ensure_result_reg_writable(dst_reg)?;
                self.require_domain_cap(DOMAIN_CAP_FDR)?;
                if !self.check_domain_budget(0, 0, 0, 1)? {
                    self.write_reg(dst_reg, -1i64 as u64)?;
                    return Ok(true);
                }
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let Some(path) = self.resolve_process_path_or_errno(&path)? else {
                    self.write_reg(dst_reg, -1i64 as u64)?;
                    return Ok(true);
                };
                match Self::open_dir_handle(&path) {
                    Ok(handle) => match self.alloc_fd_handle(handle)? {
                        Some(fd) => {
                            let token = self.fd_token(fd)?;
                            self.set_errno(0)?;
                            self.write_reg(dst_reg, token)?;
                            self.write_reg(Reg(1), token)?;
                        }
                        None => self.write_reg(dst_reg, -1i64 as u64)?,
                    },
                    Err(err) => {
                        self.write_reg(dst_reg, -1i64 as u64)?;
                        self.set_status_io_error(err)?;
                    }
                }
            }
            Instr::ReadFd(fd, buf, len) => {
                self.require_domain_cap(DOMAIN_CAP_IO)?;
                let addr = self.read_reg(buf)?;
                let len = self.read_reg(len)? as usize;
                if let Some(count) = self.read_fd_index(fd.0, addr, len)? {
                    self.complete_ok(count as u64)?;
                }
            }
            Instr::ReadFdDyn(fd_reg, buf, len) => {
                self.require_domain_cap(DOMAIN_CAP_IO)?;
                let fd = self.read_reg(fd_reg)?;
                let addr = self.read_reg(buf)?;
                let len = self.read_reg(len)? as usize;
                if let Some(fd) = self.checked_fd_index(fd)? {
                    if let Some(count) = self.read_fd_index(fd, addr, len)? {
                        self.complete_ok(count as u64)?;
                    }
                }
            }
            Instr::PreadFd(fd, buf, len, offset) => {
                self.require_domain_cap(DOMAIN_CAP_IO)?;
                let addr = self.read_reg(buf)?;
                let len = self.read_reg(len)? as usize;
                let offset = self.read_reg(offset)?;
                self.pread_fd_index(fd.0, addr, len, offset)?;
            }
            Instr::PreadFdDyn(fd_reg, buf, len, offset) => {
                self.require_domain_cap(DOMAIN_CAP_IO)?;
                let fd = self.read_reg(fd_reg)?;
                let addr = self.read_reg(buf)?;
                let len = self.read_reg(len)? as usize;
                let offset = self.read_reg(offset)?;
                if let Some(fd) = self.checked_fd_index(fd)? {
                    self.pread_fd_index(fd, addr, len, offset)?;
                }
            }
            Instr::ReaddirFd(fd, dirent_buf) => {
                let addr = self.read_reg(dirent_buf)?;
                self.readdir_fd_index(fd.0, addr)?;
            }
            Instr::ReaddirFdDyn(fd_reg, dirent_buf) => {
                let fd = self.read_reg(fd_reg)?;
                let addr = self.read_reg(dirent_buf)?;
                if let Some(fd) = self.checked_fd_index(fd)? {
                    self.readdir_fd_index(fd, addr)?;
                }
            }
            Instr::RewinddirFd(fd) => match &mut self.process_mut()?.fds[fd.0] {
                FdHandle::Dir { pos, .. } => {
                    *pos = 0;
                    self.set_status_ok()?;
                }
                _ => self.set_status_errno(20)?,
            },
            Instr::RewinddirFdDyn(fd_reg) => {
                let fd = self.read_reg(fd_reg)?;
                if let Some(fd) = self.checked_fd_index(fd)? {
                    self.rewinddir_fd_index(fd)?;
                }
            }
            Instr::WriteFd(fd, buf, len) => {
                self.require_domain_cap(DOMAIN_CAP_IO)?;
                let addr = self.read_reg(buf)?;
                let len = self.read_reg(len)? as usize;
                self.write_fd_index(fd.0, addr, len)?;
            }
            Instr::WriteFdDyn(fd_reg, buf, len) => {
                self.require_domain_cap(DOMAIN_CAP_IO)?;
                let fd = self.read_reg(fd_reg)?;
                let addr = self.read_reg(buf)?;
                let len = self.read_reg(len)? as usize;
                if let Some(fd) = self.checked_fd_index(fd)? {
                    self.write_fd_index(fd, addr, len)?;
                }
            }
            Instr::PwriteFd(fd, buf, len, offset) => {
                self.require_domain_cap(DOMAIN_CAP_IO)?;
                let addr = self.read_reg(buf)?;
                let len = self.read_reg(len)? as usize;
                let offset = self.read_reg(offset)?;
                self.pwrite_fd_index(fd.0, addr, len, offset)?;
            }
            Instr::PwriteFdDyn(fd_reg, buf, len, offset) => {
                self.require_domain_cap(DOMAIN_CAP_IO)?;
                let fd = self.read_reg(fd_reg)?;
                let addr = self.read_reg(buf)?;
                let len = self.read_reg(len)? as usize;
                let offset = self.read_reg(offset)?;
                if let Some(fd) = self.checked_fd_index(fd)? {
                    self.pwrite_fd_index(fd, addr, len, offset)?;
                }
            }
            Instr::MkdirPath(path_reg, _mode_reg) => {
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let Some(path) = self.resolve_process_path_or_errno(&path)? else {
                    return Ok(true);
                };
                match fs::create_dir(&path) {
                    Ok(()) => self.set_status_ok()?,
                    Err(err) => self.set_status_io_error(err)?,
                }
            }
            Instr::MkdirPathAt(dir_reg, path_reg, _mode_reg) => {
                let dir_value = self.read_reg(dir_reg)?;
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let Some(path) = self.resolve_process_path_at_or_errno(dir_value, &path)? else {
                    return Ok(true);
                };
                match fs::create_dir(&path) {
                    Ok(()) => self.set_status_ok()?,
                    Err(err) => self.set_status_io_error(err)?,
                }
            }
            Instr::UnlinkPath(path_reg) => {
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let Some(path) = self.resolve_process_path_or_errno(&path)? else {
                    return Ok(true);
                };
                match fs::remove_file(&path) {
                    Ok(()) => self.set_status_ok()?,
                    Err(file_err) => match fs::remove_dir(&path) {
                        Ok(()) => self.set_status_ok()?,
                        Err(_) => self.set_status_io_error(file_err)?,
                    },
                }
            }
            Instr::UnlinkPathAt(dir_reg, path_reg, _flags_reg) => {
                let dir_value = self.read_reg(dir_reg)?;
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let Some(path) = self.resolve_process_path_at_or_errno(dir_value, &path)? else {
                    return Ok(true);
                };
                match fs::remove_file(&path) {
                    Ok(()) => self.set_status_ok()?,
                    Err(file_err) => match fs::remove_dir(&path) {
                        Ok(()) => self.set_status_ok()?,
                        Err(_) => self.set_status_io_error(file_err)?,
                    },
                }
            }
            Instr::RenamePath(old_reg, new_reg) => {
                let old = self.read_c_string(self.read_reg(old_reg)?)?;
                let new = self.read_c_string(self.read_reg(new_reg)?)?;
                let Some(old) = self.resolve_process_path_or_errno(&old)? else {
                    return Ok(true);
                };
                let Some(new) = self.resolve_process_path_or_errno(&new)? else {
                    return Ok(true);
                };
                match fs::rename(&old, &new) {
                    Ok(()) => self.set_status_ok()?,
                    Err(err) => self.set_status_io_error(err)?,
                }
            }
            Instr::RenamePathAt(old_dir_reg, old_reg, new_dir_reg, new_reg) => {
                let old_dir = self.read_reg(old_dir_reg)?;
                let old = self.read_c_string(self.read_reg(old_reg)?)?;
                let new_dir = self.read_reg(new_dir_reg)?;
                let new = self.read_c_string(self.read_reg(new_reg)?)?;
                let Some(old) = self.resolve_process_path_at_or_errno(old_dir, &old)? else {
                    return Ok(true);
                };
                let Some(new) = self.resolve_process_path_at_or_errno(new_dir, &new)? else {
                    return Ok(true);
                };
                match fs::rename(&old, &new) {
                    Ok(()) => self.set_status_ok()?,
                    Err(err) => self.set_status_io_error(err)?,
                }
            }
            Instr::LinkPath(old_reg, new_reg, flags_reg) => {
                let old = self.read_c_string(self.read_reg(old_reg)?)?;
                let new = self.read_c_string(self.read_reg(new_reg)?)?;
                let flags = self.read_reg(flags_reg)?;
                let Some(old_path) = self.resolve_process_path_or_errno(&old)? else {
                    return Ok(true);
                };
                let Some(new) = self.resolve_process_path_or_errno(&new)? else {
                    return Ok(true);
                };
                let result = if flags & 1 == 1 {
                    std::os::unix::fs::symlink(&old, &new)
                } else {
                    fs::hard_link(&old_path, &new)
                };
                match result {
                    Ok(()) => self.set_status_ok()?,
                    Err(err) => self.set_status_io_error(err)?,
                }
            }
            Instr::LinkPathAt(old_dir_reg, old_reg, new_dir_reg, new_reg, flags_reg) => {
                let old_dir = self.read_reg(old_dir_reg)?;
                let old = self.read_c_string(self.read_reg(old_reg)?)?;
                let new_dir = self.read_reg(new_dir_reg)?;
                let new = self.read_c_string(self.read_reg(new_reg)?)?;
                let flags = self.read_reg(flags_reg)?;
                let Some(new_path) = self.resolve_process_path_at_or_errno(new_dir, &new)? else {
                    return Ok(true);
                };
                let result = if flags & 1 == 1 {
                    std::os::unix::fs::symlink(&old, &new_path)
                } else {
                    let Some(old_path) = self.resolve_process_path_at_or_errno(old_dir, &old)?
                    else {
                        return Ok(true);
                    };
                    fs::hard_link(&old_path, &new_path)
                };
                match result {
                    Ok(()) => self.set_status_ok()?,
                    Err(err) => self.set_status_io_error(err)?,
                }
            }
            Instr::SymlinkPath(target_reg, link_reg) => {
                let target = self.read_c_string(self.read_reg(target_reg)?)?;
                let link = self.read_c_string(self.read_reg(link_reg)?)?;
                let Some(link) = self.resolve_process_path_or_errno(&link)? else {
                    return Ok(true);
                };
                match std::os::unix::fs::symlink(&target, &link) {
                    Ok(()) => self.set_status_ok()?,
                    Err(err) => self.set_status_io_error(err)?,
                }
            }
            Instr::SymlinkPathAt(target_reg, dir_reg, link_reg) => {
                let target = self.read_c_string(self.read_reg(target_reg)?)?;
                let dir_value = self.read_reg(dir_reg)?;
                let link = self.read_c_string(self.read_reg(link_reg)?)?;
                let Some(link) = self.resolve_process_path_at_or_errno(dir_value, &link)? else {
                    return Ok(true);
                };
                match std::os::unix::fs::symlink(&target, &link) {
                    Ok(()) => self.set_status_ok()?,
                    Err(err) => self.set_status_io_error(err)?,
                }
            }
            Instr::ReadlinkPath(path_reg, buf_reg, len_reg) => {
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let Some(path) = self.resolve_process_path_no_follow_final_or_errno(&path)? else {
                    return Ok(true);
                };
                let buf = self.read_reg(buf_reg)?;
                let len = self.read_reg(len_reg)? as usize;
                match fs::read_link(&path) {
                    Ok(target) => {
                        let bytes = target.to_string_lossy();
                        let data = bytes.as_bytes();
                        let count = data.len().min(len);
                        self.write_bytes(buf, &data[..count])?;
                        self.complete_ok(count as u64)?;
                    }
                    Err(err) => self.set_status_io_error(err)?,
                }
            }
            Instr::ReadlinkPathAt(dir_reg, path_reg, buf_reg, len_reg) => {
                let dir_value = self.read_reg(dir_reg)?;
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let Some(path) =
                    self.resolve_process_path_at_no_follow_final_or_errno(dir_value, &path)?
                else {
                    return Ok(true);
                };
                let buf = self.read_reg(buf_reg)?;
                let len = self.read_reg(len_reg)? as usize;
                match fs::read_link(&path) {
                    Ok(target) => {
                        let bytes = target.to_string_lossy();
                        let data = bytes.as_bytes();
                        let count = data.len().min(len);
                        self.write_bytes(buf, &data[..count])?;
                        self.complete_ok(count as u64)?;
                    }
                    Err(err) => self.set_status_io_error(err)?,
                }
            }
            Instr::ChdirPath(path_reg) => {
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let Some(path) = self.resolve_process_path_or_errno(&path)? else {
                    return Ok(true);
                };
                match fs::metadata(&path) {
                    Ok(metadata) if metadata.is_dir() => {
                        self.process_mut()?.cwd = PathBuf::from(path);
                        self.set_status_ok()?;
                    }
                    Ok(_) => self.set_status_errno(20)?,
                    Err(err) => self.set_status_io_error(err)?,
                }
            }
            Instr::GetcwdPath(buf_reg, len_reg) => {
                let buf = self.read_reg(buf_reg)?;
                let len = self.read_reg(len_reg)? as usize;
                let cwd = self.process()?.cwd.to_string_lossy().into_owned();
                let bytes = cwd.as_bytes();
                let Some(required_len) = bytes.len().checked_add(1) else {
                    self.set_status_errno(34)?;
                    return Ok(true);
                };
                if len == 0 || required_len > len {
                    self.set_status_errno(34)?;
                } else if self.ensure_mapped(buf, required_len, true).is_err() {
                    self.set_status_errno(14)?;
                } else {
                    self.write_bytes_offset(buf, 0, bytes)?;
                    self.write_bytes_offset(buf, bytes.len() as u64, &[0])?;
                    self.complete_ok(buf)?;
                }
            }
            Instr::ChmodPath(path_reg, mode_reg, _flags_reg) => {
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let Some(path) = self.resolve_process_path_or_errno(&path)? else {
                    return Ok(true);
                };
                let mode = self.read_reg(mode_reg)? as u32;
                match fs::set_permissions(&path, fs::Permissions::from_mode(mode)) {
                    Ok(()) => self.set_status_ok()?,
                    Err(err) => self.set_status_io_error(err)?,
                }
            }
            Instr::ChmodPathAt(dir_reg, path_reg, mode_reg, _flags_reg) => {
                let dir_value = self.read_reg(dir_reg)?;
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let Some(path) = self.resolve_process_path_at_or_errno(dir_value, &path)? else {
                    return Ok(true);
                };
                let mode = self.read_reg(mode_reg)? as u32;
                match fs::set_permissions(&path, fs::Permissions::from_mode(mode)) {
                    Ok(()) => self.set_status_ok()?,
                    Err(err) => self.set_status_io_error(err)?,
                }
            }
            Instr::ChownPath(path_reg, uid_reg, gid_reg, _flags_reg) => {
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let Some(path) = self.resolve_process_path_or_errno(&path)? else {
                    return Ok(true);
                };
                let uid = self.read_reg(uid_reg)?;
                let gid = self.read_reg(gid_reg)?;
                let uid = (uid != -1i64 as u64).then_some(uid as u32);
                let gid = (gid != -1i64 as u64).then_some(gid as u32);
                match std::os::unix::fs::chown(&path, uid, gid) {
                    Ok(()) => self.set_status_ok()?,
                    Err(err) => self.set_status_io_error(err)?,
                }
            }
            Instr::ChownPathAt(dir_reg, path_reg, uid_reg, gid_reg, _flags_reg) => {
                let dir_value = self.read_reg(dir_reg)?;
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let Some(path) = self.resolve_process_path_at_or_errno(dir_value, &path)? else {
                    return Ok(true);
                };
                let uid = self.read_reg(uid_reg)?;
                let gid = self.read_reg(gid_reg)?;
                let uid = (uid != -1i64 as u64).then_some(uid as u32);
                let gid = (gid != -1i64 as u64).then_some(gid as u32);
                match std::os::unix::fs::chown(&path, uid, gid) {
                    Ok(()) => self.set_status_ok()?,
                    Err(err) => self.set_status_io_error(err)?,
                }
            }
            Instr::UtimePath(path_reg, times_reg, flags_reg) => {
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let Some(path) = self.resolve_process_path_or_errno(&path)? else {
                    return Ok(true);
                };
                let times_ptr = self.read_reg(times_reg)?;
                let flags = self.read_reg(flags_reg)? as c_int;
                self.utime_path(&path, times_ptr, flags)?;
            }
            Instr::UtimePathAt(dir_reg, path_reg, times_reg, flags_reg) => {
                let dir_value = self.read_reg(dir_reg)?;
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let Some(path) = self.resolve_process_path_at_or_errno(dir_value, &path)? else {
                    return Ok(true);
                };
                let times_ptr = self.read_reg(times_reg)?;
                let flags = self.read_reg(flags_reg)? as c_int;
                self.utime_path(&path, times_ptr, flags)?;
            }
            Instr::UtimeFd(fd, times_reg) => {
                let times_ptr = self.read_reg(times_reg)?;
                self.utime_fd_index(fd.0, times_ptr)?;
            }
            Instr::UtimeFdDyn(fd_reg, times_reg) => {
                let fd = self.read_reg(fd_reg)?;
                let times_ptr = self.read_reg(times_reg)?;
                if let Some(fd) = self.checked_fd_index(fd)? {
                    self.utime_fd_index(fd, times_ptr)?;
                }
            }
            Instr::StatPath(statbuf_reg, path_reg, flags_reg) => {
                let statbuf = self.read_reg(statbuf_reg)?;
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let Some(path) = self.resolve_process_path_or_errno(&path)? else {
                    return Ok(true);
                };
                let flags = self.read_reg(flags_reg)?;
                let result = if flags & 1 == 1 {
                    fs::symlink_metadata(&path)
                } else {
                    fs::metadata(&path)
                };
                match result {
                    Ok(metadata) => {
                        self.write_lnp64_stat(statbuf, &metadata)?;
                        self.set_status_ok()?;
                    }
                    Err(err) => self.set_status_io_error(err)?,
                }
            }
            Instr::StatPathAt(statbuf_reg, dir_reg, path_reg, flags_reg) => {
                let statbuf = self.read_reg(statbuf_reg)?;
                let dir_value = self.read_reg(dir_reg)?;
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let Some(path) = self.resolve_process_path_at_or_errno(dir_value, &path)? else {
                    return Ok(true);
                };
                let flags = self.read_reg(flags_reg)?;
                let result = if flags & 1 == 1 {
                    fs::symlink_metadata(&path)
                } else {
                    fs::metadata(&path)
                };
                match result {
                    Ok(metadata) => {
                        self.write_lnp64_stat(statbuf, &metadata)?;
                        self.set_status_ok()?;
                    }
                    Err(err) => self.set_status_io_error(err)?,
                }
            }
            Instr::StatFd(statbuf_reg, fd) => {
                let statbuf = self.read_reg(statbuf_reg)?;
                self.stat_fd_index(statbuf, fd.0)?;
            }
            Instr::StatFdDyn(statbuf_reg, fd_reg) => {
                let statbuf = self.read_reg(statbuf_reg)?;
                let fd = self.read_reg(fd_reg)?;
                if let Some(fd) = self.checked_fd_index(fd)? {
                    self.stat_fd_index(statbuf, fd)?;
                }
            }
            Instr::FcntlFdDyn(fd_reg, cmd_reg, arg_reg) => {
                let fd = self.read_reg(fd_reg)?;
                let cmd = self.read_reg(cmd_reg)?;
                let arg = self.read_reg(arg_reg)?;
                if let Some(fd) = self.checked_fd_index(fd)? {
                    self.fcntl_fd_index(fd, cmd, arg)?;
                }
            }
            Instr::FdClose(fd) => {
                self.close_fd_index_checked(fd.0)?;
            }
            Instr::FdCloseDyn(fd_reg) => {
                let fd = self.read_reg(fd_reg)?;
                if let Some(fd) = self.checked_fd_index(fd)? {
                    self.close_fd_index_checked(fd)?;
                }
            }
            Instr::FdSeek(fd, offset_reg, whence_reg) => {
                let offset = self.read_reg(offset_reg)? as i64;
                let whence = self.read_reg(whence_reg)?;
                self.fd_seek_index(fd.0, offset, whence)?;
            }
            Instr::FdSeekDyn(fd_reg, offset_reg, whence_reg) => {
                let fd = self.read_reg(fd_reg)?;
                let offset = self.read_reg(offset_reg)? as i64;
                let whence = self.read_reg(whence_reg)?;
                if let Some(fd) = self.checked_fd_index(fd)? {
                    self.fd_seek_index(fd, offset, whence)?;
                }
            }
            Instr::WaitOnFd(fd, _) => {
                if fd.0 >= FDR_COUNT {
                    self.set_status_errno(9)?;
                    return Ok(true);
                }
                if self.ensure_fd_right(fd.0, CAP_RIGHT_POLL).is_err() {
                    return Ok(true);
                }
                if !self.fd_ready(fd.0)? {
                    self.push_fd_waiter(fd.0, 0, None)?;
                    self.ready.retain(|tid| *tid != self.current_tid);
                    return Ok(false);
                }
            }
            Instr::FdDup(dst, src) => {
                self.require_domain_cap(DOMAIN_CAP_FDR)?;
                if !self.check_domain_budget(0, 0, 0, self.fd_slot_delta(dst.0)?)? {
                    return Ok(true);
                }
                let cloned = self.process()?.fds[src.0].clone_handle()?;
                let rights = self.process()?.fd_capabilities[src.0].rights;
                match self.duplicate_fd_capability(src.0, dst.0, rights, false) {
                    Ok(()) => {}
                    Err(errno) => {
                        self.set_status_errno(errno)?;
                        return Ok(true);
                    }
                }
                self.bump_fd_generation(dst.0)?;
                self.process_mut()?.fds[dst.0] = cloned;
            }
            Instr::FdDup2(dst, src) => {
                self.require_domain_cap(DOMAIN_CAP_FDR)?;
                if !self.check_domain_budget(0, 0, 0, self.fd_slot_delta(dst.0)?)? {
                    return Ok(true);
                }
                let cloned = self.process()?.fds[src.0].clone_handle()?;
                let rights = self.process()?.fd_capabilities[src.0].rights;
                match self.duplicate_fd_capability(src.0, dst.0, rights, false) {
                    Ok(()) => {}
                    Err(errno) => {
                        self.set_status_errno(errno)?;
                        return Ok(true);
                    }
                }
                self.bump_fd_generation(dst.0)?;
                self.process_mut()?.fds[dst.0] = cloned;
                self.set_status_ok()?;
            }
            Instr::ErrnoGet(dst) => {
                let errno = self.process()?.errno;
                self.write_reg(dst, errno)?;
            }
            Instr::ErrnoSet(src) => {
                let errno = self.read_reg(src)?;
                self.set_errno(errno)?;
            }
            Instr::WaitPid(status_dst, pid_reg) => {
                Self::ensure_result_reg_writable(status_dst)?;
                if status_dst.0 == 1 {
                    return Err(
                        "WAIT_PID status destination aliases status register r1".to_string()
                    );
                }
                let pid = self.read_reg(pid_reg)?;
                let current_pid = self.thread()?.pid;
                let completed = if pid == 0 {
                    self.completed_children
                        .keys()
                        .find(|(parent, _)| *parent == current_pid)
                        .copied()
                } else {
                    Some((current_pid, pid)).filter(|key| self.completed_children.contains_key(key))
                };
                if let Some(key) = completed {
                    let status = self
                        .completed_children
                        .remove(&key)
                        .unwrap_or(self.last_exit);
                    self.write_reg(status_dst, status as u64)?;
                    self.set_status_ok()?;
                    return Ok(true);
                }
                let live_child = if pid == 0 {
                    self.processes
                        .values()
                        .any(|process| process.parent_pid == Some(current_pid))
                } else {
                    self.processes
                        .get(&pid)
                        .is_some_and(|process| process.parent_pid == Some(current_pid))
                };
                if live_child {
                    self.thread_mut()?.ip = self.thread()?.ip.saturating_sub(1);
                    let current_tid = self.current_tid;
                    Self::push_unique_waiter(
                        self.child_waiters.entry(current_pid).or_default(),
                        current_tid,
                    );
                    self.ready.retain(|tid| *tid != self.current_tid);
                    return Ok(false);
                }
                if pid == 0 {
                    self.write_reg(status_dst, self.last_exit as u64)?;
                    self.set_status_ok()?;
                } else {
                    self.set_status_errno(10)?;
                }
            }
            Instr::GetPcr(dst, pcr) => {
                let value = self.read_pcr(pcr)?;
                self.write_reg(dst, value)?;
            }
            Instr::SetPcr(pcr, src) => self.write_pcr(pcr, self.read_reg(src)?)?,
            Instr::EnvGet(result, key, index_or_buf, len_or_flags) => {
                self.env_get(result, key, index_or_buf, len_or_flags)?;
            }
            Instr::Random(result, buf, len_reg) => {
                Self::ensure_result_reg_writable(result)?;
                let len = self.read_reg(len_reg)?;
                let bytes = if len == 0 { 8 } else { len };
                if len == 0 {
                    if self.consume_domain_entropy(bytes).is_err() {
                        self.complete_reg_err(result, 1)?;
                        return Ok(true);
                    }
                    let value = self.next_random_u64();
                    self.complete_reg_ok(result, value)?;
                } else {
                    let addr = self.read_reg(buf)?;
                    let len = usize::try_from(len)
                        .map_err(|_| "RANDOM length does not fit host usize".to_string())?;
                    self.ensure_mapped(addr, len, true)?;
                    if self.consume_domain_entropy(bytes).is_err() {
                        self.complete_reg_err(result, 1)?;
                        return Ok(true);
                    }
                    let data = self.random_bytes(len);
                    self.write_bytes(addr, &data)?;
                    self.complete_reg_ok(result, bytes)?;
                }
            }
            Instr::Fork(dst) => {
                self.clone_with_profile(CloneProfile::NewProcessCow, dst, None)?;
            }
            Instr::Exec(path_reg, argv_reg, envp_reg) => {
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let argv = self.read_reg(argv_reg)?;
                let envp = self.read_reg(envp_reg)?;
                let args = self.collect_exec_args(&path, argv)?;
                let env = self.collect_exec_env(envp)?;
                let Some(source_path) = self.resolve_process_path_or_errno(&path)? else {
                    return Ok(true);
                };
                let source = match fs::read_to_string(&source_path) {
                    Ok(source) => source,
                    Err(err) => {
                        self.set_status_io_error(err)?;
                        return Ok(true);
                    }
                };
                let program = match Program::parse(&source) {
                    Ok(program) => program,
                    Err(_) => {
                        self.set_status_errno(8)?;
                        return Ok(true);
                    }
                };
                let pid = self.thread()?.pid;
                let domain_id = self.process()?.domain_id;
                let aslr_enabled = self
                    .domains
                    .get(&domain_id)
                    .map(|domain| domain.security.aslr_enabled)
                    .unwrap_or(true);
                let layout = ProcessLayout::for_process(pid, domain_id, aslr_enabled);
                let entry_page = Self::build_process_entry_page(&args, &env)?;
                self.process_mut()?.exec(program, layout);
                let pid = self.thread()?.pid;
                let tid = self.thread()?.tid;
                *self.thread_mut()? = Thread::new(tid, pid, layout.stack_top);
                self.install_process_entry_page(pid, &entry_page)?;
            }
            Instr::Spawn(dst, entry) => {
                let entry = self.read_reg(entry)?;
                self.clone_with_profile(CloneProfile::SpawnEntry, dst, Some(entry))?;
            }
            Instr::ThreadJoin(result, tid_reg, retval_reg) => {
                Self::ensure_result_reg_writable(result)?;
                let tid = self.read_reg(tid_reg)?;
                let retval_ptr = self.read_reg(retval_reg)?;
                if tid == self.current_tid {
                    self.write_reg(result, 35)?;
                } else if let Some(value) = self.completed_threads.get(&tid).copied() {
                    if retval_ptr != 0 {
                        self.ensure_mapped(retval_ptr, 8, true)?;
                    }
                    self.completed_threads.remove(&tid);
                    if retval_ptr != 0 {
                        self.store_u64(retval_ptr, value)?;
                    }
                    self.write_reg(result, 0)?;
                } else if self.threads.contains_key(&tid) {
                    self.thread_mut()?.ip = self.thread()?.ip.saturating_sub(1);
                    let current_tid = self.current_tid;
                    Self::push_unique_waiter(
                        self.thread_join_waiters.entry(tid).or_default(),
                        current_tid,
                    );
                    self.ready
                        .retain(|ready_tid| *ready_tid != self.current_tid);
                    return Ok(false);
                } else {
                    self.write_reg(result, 3)?;
                }
            }
            Instr::Yield => return Ok(true),
            Instr::Sleep(ticks_reg) => {
                let ticks = self.read_reg(ticks_reg)?.max(1);
                self.sleepers
                    .retain(|(sleep_tid, _)| *sleep_tid != self.current_tid);
                self.sleepers.push((self.current_tid, ticks));
                self.ready.retain(|tid| *tid != self.current_tid);
                return Ok(false);
            }
            Instr::Exit(code) => {
                let code = self.read_reg(code)? as i32;
                self.exit_current(code)?;
                return Ok(false);
            }
            Instr::Mmap(dst, hint, len, prot, fd, offset) => {
                Self::ensure_result_reg_writable(dst)?;
                let len = self.read_reg(len)?;
                if len == 0 {
                    self.set_status_errno(22)?;
                    self.write_reg(dst, -1i64 as u64)?;
                    return Ok(true);
                }
                self.require_domain_cap(DOMAIN_CAP_MEMORY)?;
                if !self.check_domain_budget(len, 1, 0, 0)? {
                    self.write_reg(dst, -1i64 as u64)?;
                    return Ok(true);
                }
                let prot = self.read_reg(prot)?;
                if prot & !0b111 != 0 {
                    self.complete_reg_err(dst, 22)?;
                    return Ok(true);
                }
                if !self.domain_allows_prot(prot)? {
                    self.set_status_errno(1)?;
                    self.write_reg(dst, -1i64 as u64)?;
                    return Ok(true);
                }
                let hint = self.read_reg(hint)?;
                let offset = self.read_reg(offset)?;
                let file = self.process()?.fds[fd.0].file_clone()?;
                if !self.domain_allows_executable_source(prot, file.is_some())? {
                    self.set_status_errno(1)?;
                    self.write_reg(dst, -1i64 as u64)?;
                    return Ok(true);
                }
                let (addr, end) = {
                    let process = self.process()?;
                    let addr = if hint != 0 {
                        hint
                    } else {
                        let Some(addr) = checked_align_up(process.mmap_next, 4096) else {
                            self.set_status_errno(22)?;
                            self.write_reg(dst, -1i64 as u64)?;
                            return Ok(true);
                        };
                        addr
                    };
                    let Some(end) = addr.checked_add(len) else {
                        self.set_status_errno(22)?;
                        self.write_reg(dst, -1i64 as u64)?;
                        return Ok(true);
                    };
                    if end as usize > process.memory.len() {
                        self.set_status_errno(12)?;
                        self.write_reg(dst, -1i64 as u64)?;
                        return Ok(true);
                    }
                    if process.vmas.iter().any(|vma| {
                        let Some(vma_end) = vma.start.checked_add(vma.len) else {
                            return true;
                        };
                        addr < vma_end && end > vma.start
                    }) {
                        self.set_status_errno(12)?;
                        self.write_reg(dst, -1i64 as u64)?;
                        return Ok(true);
                    }
                    (addr, end)
                };
                {
                    let process = self.process_mut()?;
                    process.mmap_next = process.mmap_next.max(end);
                    process.vmas.push(Vma {
                        start: addr,
                        len,
                        prot,
                        file,
                        file_offset: offset,
                        resident: false,
                        guard: false,
                    });
                }
                self.complete_reg_ok(dst, addr)?;
            }
            Instr::Munmap(addr, len) => {
                let addr = self.read_reg(addr)?;
                let len = self.read_reg(len)?;
                if len == 0 {
                    self.set_status_errno(22)?;
                    return Ok(true);
                }
                let Some(_end) = addr.checked_add(len) else {
                    self.set_status_errno(22)?;
                    return Ok(true);
                };
                let Some(idx) = self
                    .process()?
                    .vmas
                    .iter()
                    .position(|vma| vma.start == addr && vma.len == len)
                else {
                    self.set_status_errno(22)?;
                    return Ok(true);
                };
                self.process_mut()?.vmas.remove(idx);
                self.set_errno(0)?;
            }
            Instr::Mprotect(addr, len, prot) => {
                let addr = self.read_reg(addr)?;
                let len = self.read_reg(len)?;
                let prot = self.read_reg(prot)?;
                self.mprotect_range(addr, len, prot)?;
            }
            Instr::Sigaction(signum, handler) => {
                let signum = self.read_reg(signum)?;
                if !Self::valid_signal_number(signum) {
                    self.set_status_errno(22)?;
                    return Ok(true);
                }
                let handler = self.read_reg(handler)? as usize;
                match handler {
                    SIG_DFL_HANDLER => {
                        self.process_mut()?.signal_handlers.remove(&signum);
                    }
                    SIG_IGN_HANDLER => {
                        self.process_mut()?
                            .signal_handlers
                            .insert(signum, SignalDisposition::Ignore);
                    }
                    _ => {
                        self.process_mut()?
                            .signal_handlers
                            .insert(signum, SignalDisposition::Handler(handler));
                    }
                }
            }
            Instr::SigmaskSet(mask) => {
                let mask = self.read_reg(mask)?;
                self.process_mut()?.sigmask = mask;
            }
            Instr::Alarm(dst, seconds) => {
                Self::ensure_result_reg_writable(dst)?;
                let seconds = self.read_reg(seconds)?;
                let pid = self.thread()?.pid;
                let previous = self
                    .alarms
                    .iter()
                    .find(|(alarm_pid, _)| *alarm_pid == pid)
                    .map(|(_, ticks)| ticks.div_ceil(100))
                    .unwrap_or(0);
                self.alarms.retain(|(alarm_pid, _)| *alarm_pid != pid);
                if seconds != 0 {
                    self.alarms.push((pid, seconds.saturating_mul(100)));
                }
                self.write_reg(dst, previous)?;
            }
            Instr::Kill(pid, signum) => {
                let pid = self.read_reg(pid)?;
                let signum = self.read_reg(signum)?;
                if !Self::valid_signal_number(signum) {
                    self.set_status_errno(22)?;
                    return Ok(true);
                }
                if !self.processes.contains_key(&pid) {
                    self.set_status_errno(3)?;
                    return Ok(true);
                }
                if !self.queue_process_event(pid, NativeEvent::kill_signal(signum)) {
                    self.set_status_errno(11)?;
                    return Ok(true);
                }
                self.set_status_ok()?;
            }
            Instr::Sigret => {
                let saved = self
                    .thread_mut()?
                    .signal_stack
                    .pop()
                    .ok_or_else(|| "SIGRET with empty signal stack".to_string())?;
                let thread = self.thread_mut()?;
                thread.ip = saved.ip;
                thread.regs = saved.regs;
                thread.flags = saved.flags;
            }
            Instr::LockCmpxchg(dst, addr_reg, expected, new_value) => {
                Self::ensure_result_reg_writable(dst)?;
                let addr = self.read_reg(addr_reg)?;
                let current = self.load_u64(addr)?;
                if current == self.read_reg(expected)? {
                    self.store_u64(addr, self.read_reg(new_value)?)?;
                }
                self.write_reg(dst, current)?;
            }
            Instr::FutexWait(addr_reg, expected_reg) => {
                let addr = self.read_reg(addr_reg)?;
                let expected = self.read_reg(expected_reg)?;
                if self.load_u64(addr)? == expected {
                    let current_tid = self.current_tid;
                    Self::push_unique_waiter(
                        self.futex_waiters.entry(addr).or_default(),
                        current_tid,
                    );
                    self.ready.retain(|tid| *tid != self.current_tid);
                    return Ok(false);
                }
            }
            Instr::FutexWake(addr_reg, count_reg) => {
                let addr = self.read_reg(addr_reg)?;
                let count = self.read_reg(count_reg)?;
                let mut to_wake = Vec::new();
                let mut remove_waiter_entry = false;
                if let Some(waiters) = self.futex_waiters.get_mut(&addr) {
                    for _ in 0..count {
                        let Some(tid) = waiters.pop_front() else {
                            break;
                        };
                        to_wake.push(tid);
                    }
                    remove_waiter_entry = waiters.is_empty();
                }
                if remove_waiter_entry {
                    self.futex_waiters.remove(&addr);
                }
                for tid in to_wake {
                    self.wake_thread(tid);
                }
            }
            Instr::Inb(dst, port) => {
                Self::ensure_result_reg_writable(dst)?;
                if self.process()?.uid != 0 {
                    self.raise_current_signal(SIGSEGV)?;
                    return Ok(true);
                }
                self.require_domain_cap(DOMAIN_CAP_IO)?;
                let value = self
                    .process()?
                    .ucode_ports
                    .get(&self.read_reg(port)?)
                    .copied()
                    .unwrap_or(0);
                self.write_reg(dst, value as u64)?;
            }
            Instr::Outb(port, src) => {
                if self.process()?.uid != 0 {
                    self.raise_current_signal(SIGSEGV)?;
                    return Ok(true);
                }
                self.require_domain_cap(DOMAIN_CAP_IO)?;
                let port = self.read_reg(port)?;
                let value = self.read_reg(src)? as u8;
                self.process_mut()?.ucode_ports.insert(port, value);
            }
            Instr::LoadUcode(buf, len) => {
                if self.process()?.uid != 0 {
                    self.raise_current_signal(SIGSEGV)?;
                    return Ok(true);
                }
                self.require_domain_cap(DOMAIN_CAP_IO)?;
                let blob = self.read_bytes(self.read_reg(buf)?, self.read_reg(len)? as usize)?;
                self.load_microcode(&blob)?;
            }
            Instr::MsgSend(pid, v1, v2) => {
                let pid = self.read_reg(pid)?;
                let msg = (self.read_reg(v1)?, self.read_reg(v2)?);
                let Some(process) = self.processes.get_mut(&pid) else {
                    self.set_status_errno(3)?;
                    return Ok(true);
                };
                if process.inbox.len() >= PROCESS_INBOX_LIMIT {
                    self.set_status_errno(11)?;
                    return Ok(true);
                }
                process.inbox.push_back(msg);
                self.set_status_ok()?;
                if let Some(tid) = self
                    .threads
                    .values()
                    .find(|thread| thread.pid == pid)
                    .map(|thread| thread.tid)
                {
                    self.wake_thread(tid);
                }
            }
            Instr::ObjectCtl(result, argblock) => {
                self.object_ctl(result, self.read_reg(argblock)?)?;
            }
            Instr::DmaCtl(result, argblock) => {
                self.dma_ctl(result, self.read_reg(argblock)?)?;
            }
            Instr::CapSend(result, argblock) => {
                self.cap_send(result, self.read_reg(argblock)?)?;
            }
            Instr::CapRecv(result, argblock) => {
                self.cap_recv(result, self.read_reg(argblock)?)?;
            }
            Instr::CapDup(result, argblock) => {
                self.cap_dup(result, self.read_reg(argblock)?)?;
            }
            Instr::CapRevoke(result, argblock) => {
                self.cap_revoke(result, self.read_reg(argblock)?)?;
            }
            Instr::DomainCtl(result, argblock) => {
                self.domain_ctl(result, self.read_reg(argblock)?)?;
            }
            Instr::NsCtl(result, argblock) => {
                self.ns_ctl(result, self.read_reg(argblock)?)?;
            }
            Instr::CallCap(result, call_gate, arg0, arg1) => {
                self.call_cap(
                    result,
                    call_gate.0,
                    self.read_reg(arg0)?,
                    self.read_reg(arg1)?,
                )?;
            }
            Instr::RetCap(result, value0, value1) => {
                self.ret_cap(result, self.read_reg(value0)?, self.read_reg(value1)?)?;
            }
            Instr::FAdd(dst, a, b) => {
                self.write_freg(dst, self.read_f64(a)? + self.read_f64(b)?)?
            }
            Instr::FSub(dst, a, b) => {
                self.write_freg(dst, self.read_f64(a)? - self.read_f64(b)?)?
            }
            Instr::FMul(dst, a, b) => {
                self.write_freg(dst, self.read_f64(a)? * self.read_f64(b)?)?
            }
            Instr::FDiv(dst, a, b) => {
                self.write_freg(dst, self.read_f64(a)? / self.read_f64(b)?)?
            }
            Instr::VAdd32(dst, a, b) => {
                let lhs = self.thread()?.vregs[a.0];
                let rhs = self.thread()?.vregs[b.0];
                let mut lanes = [0u32; 4];
                for (idx, lane) in lanes.iter_mut().enumerate() {
                    let shift = idx * 32;
                    let l = ((lhs >> shift) & 0xffff_ffff) as u32;
                    let r = ((rhs >> shift) & 0xffff_ffff) as u32;
                    *lane = l.wrapping_add(r);
                }
                let packed = lanes.iter().enumerate().fold(0u128, |acc, (idx, lane)| {
                    acc | ((*lane as u128) << (idx * 32))
                });
                self.thread_mut()?.vregs[dst.0] = packed;
            }
        }
        Ok(true)
    }

    fn thread(&self) -> Result<&Thread, String> {
        self.threads
            .get(&self.current_tid)
            .ok_or_else(|| format!("missing current thread {}", self.current_tid))
    }

    fn thread_mut(&mut self) -> Result<&mut Thread, String> {
        self.threads
            .get_mut(&self.current_tid)
            .ok_or_else(|| format!("missing current thread {}", self.current_tid))
    }

    fn clone_with_profile(
        &mut self,
        profile: CloneProfile,
        dst: Reg,
        entry: Option<u64>,
    ) -> Result<(), String> {
        self.require_domain_cap(DOMAIN_CAP_PROCESS)?;
        Self::ensure_result_reg_writable(dst)?;
        if profile == CloneProfile::DomainTask {
            self.set_status_errno(38)?;
            self.write_reg(dst, -1i64 as u64)?;
            return Ok(());
        }
        if !self.check_domain_budget(0, 0, 1, 0)? {
            self.write_reg(dst, -1i64 as u64)?;
            return Ok(());
        }
        match profile {
            CloneProfile::NewProcessCow => {
                let child_pid = self.next_pid;
                let child_tid = self.next_tid;

                let child_process = self.process()?.fork_clone(child_pid)?;
                let mut child_thread = self.thread()?.clone();
                child_thread.pid = child_pid;
                child_thread.tid = child_tid;
                if dst.0 != 0 && dst.0 != 31 {
                    child_thread.regs[dst.0] = 0;
                }
                self.next_pid += 1;
                self.next_tid += 1;
                self.processes.insert(child_pid, child_process);
                self.threads.insert(child_tid, child_thread);
                self.ready.push_back(child_tid);
                self.complete_reg_ok(dst, child_pid)?;
            }
            CloneProfile::NewThreadSharedVm | CloneProfile::SpawnEntry => {
                let Some(entry) = entry else {
                    self.set_status_errno(22)?;
                    self.write_reg(dst, -1i64 as u64)?;
                    return Ok(());
                };
                let tid = self.next_tid;
                let stack_top = self.process()?.stack_top;
                let stack_offset = match tid
                    .checked_sub(1)
                    .and_then(|index| index.checked_mul(THREAD_STACK_STRIDE))
                {
                    Some(offset) => offset,
                    None => {
                        self.set_status_errno(12)?;
                        self.write_reg(dst, -1i64 as u64)?;
                        return Ok(());
                    }
                };
                let Some(child_stack) = stack_top
                    .checked_sub(CALL_FRAME_SIZE)
                    .and_then(|base| base.checked_sub(stack_offset))
                else {
                    self.set_status_errno(12)?;
                    self.write_reg(dst, -1i64 as u64)?;
                    return Ok(());
                };
                if self
                    .ensure_mapped(child_stack, CALL_FRAME_SIZE as usize, true)
                    .is_err()
                {
                    self.set_status_errno(12)?;
                    self.write_reg(dst, -1i64 as u64)?;
                    return Ok(());
                }
                let mut child = self.thread()?.clone();
                child.tid = tid;
                child.thread_pointer = 0;
                child.ip = entry as usize;
                child.regs[31] = child_stack;
                let thread_return = self.process()?.program.instructions.len() as u64;
                self.store_u64(child.regs[31], thread_return)?;
                self.next_tid += 1;
                self.threads.insert(tid, child);
                self.ready.push_back(tid);
                self.complete_reg_ok(dst, tid)?;
            }
            CloneProfile::DomainTask => unreachable!("domain task profile returned before budget"),
        }
        Ok(())
    }

    fn process(&self) -> Result<&Process, String> {
        let pid = self.thread()?.pid;
        self.processes
            .get(&pid)
            .ok_or_else(|| format!("missing process {pid}"))
    }

    fn fault_context(&mut self, tid: u64) -> String {
        let Some(thread) = self.threads.get(&tid) else {
            return String::new();
        };
        let sp = thread.regs[31];
        let r1 = thread.regs[1];
        let r2 = thread.regs[2];
        let r3 = thread.regs[3];
        let ret = self
            .load_u64(sp)
            .map(|value| format!(" ret={value}"))
            .unwrap_or_default();
        format!(" r1={r1} r2={r2} r3={r3} r31={sp}{ret}")
    }

    fn process_mut(&mut self) -> Result<&mut Process, String> {
        let pid = self.thread()?.pid;
        self.processes
            .get_mut(&pid)
            .ok_or_else(|| format!("missing process {pid}"))
    }

    fn open_fd_handle(path: &str, flags: u64) -> Result<FdHandle, String> {
        if let Some(addr) = path.strip_prefix("tcp-listen:") {
            let addr = addr
                .parse::<SocketAddr>()
                .map_err(|err| format!("OPEN_FD TCP listener address {addr:?}: {err}"))?;
            let listener = TcpListener::bind(addr)
                .map_err(|err| format!("OPEN_FD TCP listener {addr:?}: {err}"))?;
            listener
                .set_nonblocking(true)
                .map_err(|err| format!("OPEN_FD TCP nonblocking {addr:?}: {err}"))?;
            Ok(FdHandle::TcpListener {
                listener,
                pending: None,
            })
        } else {
            let file = if flags & 1 == 1 {
                OpenOptions::new()
                    .create(true)
                    .truncate(false)
                    .append(true)
                    .read(true)
                    .open(path)
            } else if flags & 2 == 2 || flags & 4 == 4 {
                OpenOptions::new()
                    .create(true)
                    .truncate(flags & 2 == 2)
                    .write(true)
                    .read(true)
                    .open(path)
            } else {
                File::open(path)
            }
            .map_err(|err| format!("OPEN_FD {path:?}: {err}"))?;
            Ok(FdHandle::File(file))
        }
    }

    fn open_dir_handle(path: &str) -> io::Result<FdHandle> {
        let mut entries = Vec::new();
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            entries.push(entry.file_name().to_string_lossy().into_owned());
        }
        entries.sort();
        Ok(FdHandle::Dir {
            path: path.to_string(),
            entries,
            pos: 0,
        })
    }

    fn errno_from_io(err: &io::Error) -> u64 {
        err.raw_os_error()
            .filter(|errno| *errno > 0)
            .map(|errno| errno as u64)
            .unwrap_or(5)
    }

    fn valid_signal_number(signum: u64) -> bool {
        (1..SIGNAL_NUMBER_LIMIT).contains(&signum)
    }

    fn enqueue_pending_event(process: &mut Process, event: NativeEvent) -> bool {
        if event
            .signal_number()
            .is_some_and(|signum| !Self::valid_signal_number(signum))
        {
            return false;
        }
        if process.pending_events.len() >= PROCESS_EVENT_QUEUE_LIMIT {
            return false;
        }
        process.pending_events.push_back(event);
        true
    }

    fn set_errno(&mut self, errno: u64) -> Result<(), String> {
        self.process_mut()?.errno = errno;
        Ok(())
    }

    fn complete_ok(&mut self, value: u64) -> Result<(), String> {
        self.complete_reg_ok(Reg(1), value)
    }

    fn complete_err(&mut self, errno: u64) -> Result<(), String> {
        self.complete_reg_err(Reg(1), errno)
    }

    fn complete_reg_ok(&mut self, result: Reg, value: u64) -> Result<(), String> {
        Self::ensure_result_reg_writable(result)?;
        self.set_errno(0)?;
        self.write_reg(result, value)
    }

    fn complete_reg_err(&mut self, result: Reg, errno: u64) -> Result<(), String> {
        Self::ensure_result_reg_writable(result)?;
        self.set_errno(errno)?;
        self.write_reg(result, -1i64 as u64)
    }

    fn set_status_ok(&mut self) -> Result<(), String> {
        self.complete_ok(0)
    }

    fn set_status_errno(&mut self, errno: u64) -> Result<(), String> {
        self.complete_err(errno)
    }

    fn set_status_io_error(&mut self, err: io::Error) -> Result<(), String> {
        self.set_status_errno(Self::errno_from_io(&err))
    }

    fn resolve_process_path_or_errno(&mut self, path: &str) -> Result<Option<String>, String> {
        match self.resolve_process_path(path) {
            Ok(path) => Ok(Some(path)),
            Err(err) if err.starts_with("path resolution denied:") => {
                self.set_status_errno(13)?;
                Ok(None)
            }
            Err(err) => Err(err),
        }
    }

    fn resolve_process_path_at_or_errno(
        &mut self,
        dir_value: u64,
        path: &str,
    ) -> Result<Option<String>, String> {
        if path.starts_with('/') || dir_value == AT_FDCWD_VALUE {
            return self.resolve_process_path_or_errno(path);
        }
        let dir_fd = match self.decode_fd_value(dir_value) {
            Ok(fd) => fd,
            Err(errno) => {
                self.set_status_errno(errno)?;
                return Ok(None);
            }
        };
        if let Err(errno) = self.fd_right_errno(dir_fd, CAP_RIGHT_READ) {
            self.set_status_errno(errno)?;
            return Ok(None);
        }
        let base = {
            let process = self.process()?;
            match process.fds.get(dir_fd) {
                Some(FdHandle::Dir { path, .. }) => PathBuf::from(path),
                _ => {
                    self.set_status_errno(20)?;
                    return Ok(None);
                }
            }
        };
        match self.resolve_process_path_from_base(&base, path) {
            Ok(path) => Ok(Some(path)),
            Err(err) if err.starts_with("path resolution denied:") => {
                self.set_status_errno(13)?;
                Ok(None)
            }
            Err(err) => Err(err),
        }
    }

    fn resolve_process_path_no_follow_final_or_errno(
        &mut self,
        path: &str,
    ) -> Result<Option<String>, String> {
        match self.resolve_process_path_no_follow_final(path) {
            Ok(path) => Ok(Some(path)),
            Err(err) if err.starts_with("path resolution denied:") => {
                self.set_status_errno(13)?;
                Ok(None)
            }
            Err(err) => Err(err),
        }
    }

    fn resolve_process_path_at_no_follow_final_or_errno(
        &mut self,
        dir_value: u64,
        path: &str,
    ) -> Result<Option<String>, String> {
        if path.starts_with('/') || dir_value == AT_FDCWD_VALUE {
            return self.resolve_process_path_no_follow_final_or_errno(path);
        }
        let dir_fd = match self.decode_fd_value(dir_value) {
            Ok(fd) => fd,
            Err(errno) => {
                self.set_status_errno(errno)?;
                return Ok(None);
            }
        };
        if let Err(errno) = self.fd_right_errno(dir_fd, CAP_RIGHT_READ) {
            self.set_status_errno(errno)?;
            return Ok(None);
        }
        let base = {
            let process = self.process()?;
            match process.fds.get(dir_fd) {
                Some(FdHandle::Dir { path, .. }) => PathBuf::from(path),
                _ => {
                    self.set_status_errno(20)?;
                    return Ok(None);
                }
            }
        };
        match self.resolve_process_path_no_follow_final_from_base(&base, path) {
            Ok(path) => Ok(Some(path)),
            Err(err) if err.starts_with("path resolution denied:") => {
                self.set_status_errno(13)?;
                Ok(None)
            }
            Err(err) => Err(err),
        }
    }

    fn resolve_process_path_at_raw(
        &self,
        dir_value: u64,
        path: &str,
        no_follow_final: bool,
    ) -> Result<String, u64> {
        if path.starts_with('/') || dir_value == AT_FDCWD_VALUE {
            let resolved = if no_follow_final {
                self.resolve_process_path_no_follow_final(path)
            } else {
                self.resolve_process_path(path)
            };
            return Self::path_resolution_result_to_errno(resolved);
        }
        let dir_fd = self.decode_fd_value(dir_value)?;
        self.fd_right_errno(dir_fd, CAP_RIGHT_READ)?;
        let base = {
            let process = self.process().map_err(|_| 3u64)?;
            match process.fds.get(dir_fd) {
                Some(FdHandle::Dir { path, .. }) => PathBuf::from(path),
                _ => return Err(20),
            }
        };
        let resolved = if no_follow_final {
            self.resolve_process_path_no_follow_final_from_base(&base, path)
        } else {
            self.resolve_process_path_from_base(&base, path)
        };
        Self::path_resolution_result_to_errno(resolved)
    }

    fn path_resolution_result_to_errno(result: Result<String, String>) -> Result<String, u64> {
        match result {
            Ok(path) => Ok(path),
            Err(err) if err.starts_with("path resolution denied:") => Err(13),
            Err(_) => Err(5),
        }
    }

    fn resolve_process_path(&self, path: &str) -> Result<String, String> {
        let process = self.process()?;
        self.resolve_process_path_from_base(&process.cwd, path)
    }

    fn resolve_process_path_from_base(&self, base: &Path, path: &str) -> Result<String, String> {
        if path.is_empty() {
            return Err("path resolution denied: empty path".to_string());
        }
        if path.starts_with("tcp-listen:") {
            return Ok(path.to_string());
        }
        let process = self.process()?;
        let root = process.namespace_root.as_ref().ok_or_else(|| {
            "path resolution denied: missing namespace root capability".to_string()
        })?;
        let candidate = if path.starts_with('/') {
            normalize_path_lexical(&root.join(path.trim_start_matches('/')))
        } else {
            normalize_path_lexical(&base.join(path))
        };
        let root = normalize_path_lexical(root);
        if !candidate.starts_with(&root) {
            return Err(format!(
                "path resolution denied: {:?} escapes namespace root {:?}",
                candidate, root
            ));
        }
        self.ensure_path_stays_within_namespace_root(&root, &candidate)?;
        Ok(candidate.to_string_lossy().into_owned())
    }

    fn resolve_process_path_no_follow_final(&self, path: &str) -> Result<String, String> {
        let process = self.process()?;
        self.resolve_process_path_no_follow_final_from_base(&process.cwd, path)
    }

    fn resolve_process_path_no_follow_final_from_base(
        &self,
        base: &Path,
        path: &str,
    ) -> Result<String, String> {
        if path.is_empty() {
            return Err("path resolution denied: empty path".to_string());
        }
        if path.starts_with("tcp-listen:") {
            return Ok(path.to_string());
        }
        let process = self.process()?;
        let root = process.namespace_root.as_ref().ok_or_else(|| {
            "path resolution denied: missing namespace root capability".to_string()
        })?;
        let candidate = if path.starts_with('/') {
            normalize_path_lexical(&root.join(path.trim_start_matches('/')))
        } else {
            normalize_path_lexical(&base.join(path))
        };
        let root = normalize_path_lexical(root);
        if !candidate.starts_with(&root) {
            return Err(format!(
                "path resolution denied: {:?} escapes namespace root {:?}",
                candidate, root
            ));
        }
        let parent = candidate.parent().unwrap_or(&candidate);
        self.ensure_path_stays_within_namespace_root(&root, parent)?;
        Ok(candidate.to_string_lossy().into_owned())
    }

    fn ensure_path_stays_within_namespace_root(
        &self,
        root: &Path,
        candidate: &Path,
    ) -> Result<(), String> {
        let Ok(root) = fs::canonicalize(root) else {
            return Ok(());
        };
        let mut existing = candidate.to_path_buf();
        while !existing.exists() {
            if !existing.pop() {
                return Ok(());
            }
        }
        let resolved = fs::canonicalize(&existing)
            .map_err(|err| format!("path resolution denied: {:?}: {err}", existing))?;
        if !resolved.starts_with(&root) {
            return Err(format!(
                "path resolution denied: {:?} resolves through {:?} outside namespace root {:?}",
                candidate, resolved, root
            ));
        }
        Ok(())
    }

    fn write_lnp64_stat(&mut self, addr: u64, metadata: &fs::Metadata) -> Result<(), String> {
        self.ensure_mapped(addr, LNP64_STAT_RECORD_SIZE, true)?;
        let uid = self.process()?.uid;
        let gid = self.process()?.gid;
        let fields = [
            (0, metadata.mode() as u64),
            (8, metadata.size()),
            (16, metadata.dev()),
            (24, metadata.ino()),
            (32, metadata.mtime() as u64),
            (40, metadata.mtime_nsec() as u64),
            (48, metadata.nlink()),
            (56, uid),
            (64, gid),
            (72, metadata.atime() as u64),
            (80, metadata.atime_nsec() as u64),
            (88, metadata.ctime() as u64),
            (96, metadata.ctime_nsec() as u64),
        ];
        for (offset, value) in fields {
            self.store_u64_offset(addr, offset, value)?;
        }
        Ok(())
    }

    fn write_synthetic_stat(&mut self, addr: u64, mode: u64, size: u64) -> Result<(), String> {
        self.ensure_mapped(addr, LNP64_STAT_RECORD_SIZE, true)?;
        let fields = [
            (0, mode),
            (8, size),
            (16, 0),
            (24, 0),
            (32, 0),
            (40, 0),
            (48, 1),
            (56, self.process()?.uid),
            (64, self.process()?.gid),
            (72, 0),
            (80, 0),
            (88, 0),
            (96, 0),
        ];
        for (offset, value) in fields {
            self.store_u64_offset(addr, offset, value)?;
        }
        Ok(())
    }

    fn store_u64_offset(&mut self, base: u64, offset: u64, value: u64) -> Result<(), String> {
        let addr = base
            .checked_add(offset)
            .ok_or_else(|| "address overflow".to_string())?;
        self.store_u64(addr, value)
    }

    fn load_u64_offset(&mut self, base: u64, offset: u64) -> Result<u64, String> {
        let addr = base
            .checked_add(offset)
            .ok_or_else(|| "address overflow".to_string())?;
        self.load_u64(addr)
    }

    fn checked_record_base(base: u64, index: u64, stride: u64) -> Result<u64, String> {
        let offset = index
            .checked_mul(stride)
            .ok_or_else(|| "address overflow".to_string())?;
        base.checked_add(offset)
            .ok_or_else(|| "address overflow".to_string())
    }

    fn write_bytes_offset(&mut self, base: u64, offset: u64, data: &[u8]) -> Result<(), String> {
        let addr = base
            .checked_add(offset)
            .ok_or_else(|| "address overflow".to_string())?;
        self.write_bytes(addr, data)
    }

    fn alloc_heap(&mut self, len: usize, align: u64, guarded: bool) -> Result<u64, String> {
        self.require_domain_cap(DOMAIN_CAP_MEMORY)?;
        let guard_len = if guarded { 4096 } else { 0 };
        let memory_delta = (len as u64)
            .checked_add(guard_len)
            .and_then(|value| value.checked_add(guard_len))
            .ok_or_else(|| "allocation size overflow".to_string())?;
        if !self.check_domain_budget(memory_delta, 1 + u64::from(guarded) * 2, 0, 0)? {
            return Ok(-1i64 as u64);
        }
        let align = align.max(1).next_power_of_two();
        let addr = {
            let process = self.process_mut()?;
            let addr = if guarded {
                checked_align_up(
                    process
                        .heap_next
                        .checked_add(guard_len)
                        .ok_or_else(|| "allocation overflow".to_string())?,
                    align,
                )
                .ok_or_else(|| "allocation overflow".to_string())?
            } else {
                checked_align_up(process.heap_next, align)
                    .ok_or_else(|| "allocation overflow".to_string())?
            };
            let end = addr
                .checked_add(len as u64)
                .and_then(|value| value.checked_add(guard_len))
                .ok_or_else(|| "allocation overflow".to_string())?;
            if end as usize >= process.memory.len() {
                return Err(format!("out of silicon heap memory allocating {len} bytes"));
            }
            process.heap_next = end;
            let guard_before = if guarded {
                let start = addr
                    .checked_sub(guard_len)
                    .ok_or_else(|| "allocation guard underflow".to_string())?;
                process.vmas.push(Vma::guard(start, guard_len));
                Some(start)
            } else {
                None
            };
            let guard_after = if guarded {
                let start = addr
                    .checked_add(len as u64)
                    .ok_or_else(|| "allocation guard overflow".to_string())?;
                process.vmas.push(Vma::guard(start, guard_len));
                Some(start)
            } else {
                None
            };
            process.allocations.insert(
                addr,
                Allocation {
                    len,
                    guard_before,
                    guard_after,
                },
            );
            process.vmas.push(Vma::anonymous(addr, len as u64, 0b11));
            addr
        };
        Ok(addr)
    }

    fn checked_fd_index(&mut self, fd: u64) -> Result<Option<usize>, String> {
        match self.decode_fd_value(fd) {
            Ok(fd) => Ok(Some(fd)),
            Err(errno) => {
                self.set_status_errno(errno)?;
                Ok(None)
            }
        }
    }

    fn alloc_fd_handle(&mut self, handle: FdHandle) -> Result<Option<usize>, String> {
        let fd = {
            let process = self.process_mut()?;
            process
                .fds
                .iter()
                .position(|candidate| matches!(candidate, FdHandle::Closed))
        };
        if let Some(fd) = fd {
            self.bump_fd_generation(fd)?;
            self.process_mut()?.fds[fd] = handle;
            let capability = self.fresh_fd_capability();
            self.install_fd_capability(fd, capability)?;
            Ok(Some(fd))
        } else {
            self.set_status_errno(24)?;
            Ok(None)
        }
    }

    fn fd_token(&self, fd: usize) -> Result<u64, String> {
        let generation = self
            .process()?
            .fd_generations
            .get(fd)
            .copied()
            .ok_or_else(|| format!("fd index out of range: {fd}"))?;
        Ok(FDR_TOKEN_MARKER | (generation << FDR_TOKEN_SHIFT) | fd as u64)
    }

    fn fresh_fd_capability(&mut self) -> FdCapability {
        let lineage = self.next_cap_lineage;
        self.next_cap_lineage = self
            .next_cap_lineage
            .saturating_add(1)
            .max(FDR_COUNT as u64 + 1);
        FdCapability::full(lineage)
    }

    fn install_fd_capability(&mut self, fd: usize, capability: FdCapability) -> Result<(), String> {
        let process = self.process_mut()?;
        let Some(slot) = process.fd_capabilities.get_mut(fd) else {
            return Err(format!("fd index out of range: {fd}"));
        };
        *slot = capability;
        Ok(())
    }

    fn close_fd_index(&mut self, fd: usize) -> Result<(), String> {
        self.release_process_file_locks_for_fd(fd)?;
        self.bump_fd_generation(fd)?;
        self.process_mut()?.fds[fd] = FdHandle::Closed;
        let lineage = self.fresh_fd_capability().lineage;
        self.install_fd_capability(fd, FdCapability::closed(lineage))?;
        Ok(())
    }

    fn close_fd_index_checked(&mut self, fd: usize) -> Result<(), String> {
        if matches!(self.process()?.fds.get(fd), Some(FdHandle::Closed) | None) {
            return self.set_status_errno(9);
        }
        self.close_fd_index(fd)?;
        self.set_status_ok()
    }

    fn file_lock_key_for_fd(&self, fd: usize) -> Result<FileLockKey, u64> {
        self.fd_right_errno(fd, CAP_RIGHT_STAT)?;
        self.file_lock_key_for_fd_unchecked(fd)
    }

    fn file_lock_key_for_fd_unchecked(&self, fd: usize) -> Result<FileLockKey, u64> {
        let process = self.process().map_err(|_| 3u64)?;
        let Some(FdHandle::File(file)) = process.fds.get(fd) else {
            return Err(9);
        };
        let metadata = file.metadata().map_err(|err| Self::errno_from_io(&err))?;
        Ok(FileLockKey {
            dev: metadata.dev(),
            ino: metadata.ino(),
        })
    }

    fn release_process_file_locks_for_fd(&mut self, fd: usize) -> Result<(), String> {
        let pid = self.process()?.pid;
        if let Ok(key) = self.file_lock_key_for_fd_unchecked(fd) {
            self.advisory_locks
                .retain(|lock_key, lock| *lock_key != key || lock.owner_pid != pid);
        }
        Ok(())
    }

    fn release_fd_locks_for_replacement(&mut self, fd: usize) -> Result<(), String> {
        if !matches!(self.process()?.fds.get(fd), Some(FdHandle::Closed)) {
            self.release_process_file_locks_for_fd(fd)?;
        }
        Ok(())
    }

    fn fcntl_fd_index(&mut self, fd: usize, cmd: u64, arg: u64) -> Result<(), String> {
        const F_GETLK_LNP64: u64 = 5;
        const F_SETLK_LNP64: u64 = 6;
        const F_SETLKW_LNP64: u64 = 7;
        const F_UNLCK_LNP64: u64 = 2;
        if arg == 0 {
            return self.set_status_errno(14);
        }
        self.ensure_mapped(arg, 40, true)?;
        let key = match self.file_lock_key_for_fd(fd) {
            Ok(key) => key,
            Err(errno) => return self.set_status_errno(errno),
        };
        let pid = self.process()?.pid;
        let requested_type = self.load_u64_offset(arg, 0)?;
        match cmd {
            F_GETLK_LNP64 => {
                if let Some(lock) = self.advisory_locks.get(&key).copied() {
                    if lock.owner_pid != pid {
                        self.store_u64_offset(arg, 0, lock.lock_type)?;
                        self.store_u64_offset(arg, 32, lock.owner_pid)?;
                    } else {
                        self.store_u64_offset(arg, 0, F_UNLCK_LNP64)?;
                        self.store_u64_offset(arg, 32, 0)?;
                    }
                } else {
                    self.store_u64_offset(arg, 0, F_UNLCK_LNP64)?;
                    self.store_u64_offset(arg, 32, 0)?;
                }
                self.set_status_ok()
            }
            F_SETLK_LNP64 | F_SETLKW_LNP64 => {
                if requested_type == F_UNLCK_LNP64 {
                    self.advisory_locks
                        .retain(|lock_key, lock| *lock_key != key || lock.owner_pid != pid);
                    return self.set_status_ok();
                }
                if let Some(lock) = self.advisory_locks.get(&key).copied() {
                    if lock.owner_pid != pid {
                        return self.set_status_errno(11);
                    }
                }
                self.advisory_locks.insert(
                    key,
                    AdvisoryLock {
                        owner_pid: pid,
                        lock_type: requested_type,
                    },
                );
                self.set_status_ok()
            }
            _ => self.set_status_errno(22),
        }
    }

    fn duplicate_fd_capability(
        &mut self,
        src: usize,
        dst: usize,
        rights: u64,
        sealed: bool,
    ) -> Result<(), u64> {
        let source = self
            .process()
            .map_err(|_| 3u64)?
            .fd_capabilities
            .get(src)
            .copied()
            .ok_or(9u64)?;
        if matches!(self.process().map_err(|_| 3u64)?.fds[src], FdHandle::Closed) {
            return Err(9);
        }
        if source.revoked {
            return Err(116);
        }
        if source.sealed || source.rights & CAP_RIGHT_DUP == 0 {
            return Err(1);
        }
        if rights & !source.rights != 0 {
            return Err(1);
        }
        if (rights != source.rights || sealed) && !source.narrowable {
            return Err(1);
        }
        let mut duplicate = source;
        duplicate.rights = rights;
        duplicate.sealed = sealed;
        duplicate.narrowable = source.narrowable && !sealed;
        duplicate.revoked = false;
        self.install_fd_capability(dst, duplicate).map_err(|_| 9u64)
    }

    fn fd_right_errno(&self, fd: usize, right: u64) -> Result<(), u64> {
        let process = self.process().map_err(|_| 3u64)?;
        let Some(capability) = process.fd_capabilities.get(fd).copied() else {
            return Err(9);
        };
        if matches!(process.fds.get(fd), Some(FdHandle::Closed) | None) {
            return Err(9);
        }
        if capability.revoked {
            return Err(116);
        }
        if capability.rights & right != right {
            return Err(1);
        }
        Ok(())
    }

    fn ensure_fd_right(&mut self, fd: usize, right: u64) -> Result<(), String> {
        match self.fd_right_errno(fd, right) {
            Ok(()) => Ok(()),
            Err(errno) => {
                self.set_status_errno(errno)?;
                Err(format!("fd {fd} capability right denied"))
            }
        }
    }

    fn decode_fd_value(&self, value: u64) -> Result<usize, u64> {
        if value == -1i64 as u64 {
            return Err(9);
        }
        if value < FDR_COUNT as u64 {
            return Ok(value as usize);
        }
        if value & FDR_TOKEN_MARKER == 0 {
            return Err(9);
        }
        let fd = (value & FDR_TOKEN_INDEX_MASK) as usize;
        if fd >= FDR_COUNT {
            return Err(9);
        }
        let generation = (value & !FDR_TOKEN_MARKER) >> FDR_TOKEN_SHIFT;
        if generation == 0 {
            return Err(9);
        }
        let process = self.process().map_err(|_| 3u64)?;
        if process.fd_generations.get(fd).copied() != Some(generation)
            || matches!(process.fds[fd], FdHandle::Closed)
            || process
                .fd_capabilities
                .get(fd)
                .is_none_or(|cap| cap.revoked)
        {
            return Err(116);
        }
        Ok(fd)
    }

    fn bump_fd_generation(&mut self, fd: usize) -> Result<(), String> {
        let generation = self
            .process()?
            .fd_generations
            .get(fd)
            .copied()
            .ok_or_else(|| format!("fd index out of range: {fd}"))?;
        let next = generation.saturating_add(1).max(1);
        self.process_mut()?.fd_generations[fd] = next;
        Ok(())
    }

    fn write_fd_index(&mut self, fd: usize, addr: u64, len: usize) -> Result<(), String> {
        if self.ensure_fd_right(fd, CAP_RIGHT_WRITE).is_err() {
            return Ok(());
        }
        let data = self.read_bytes(addr, len)?;
        if matches!(
            self.process()?.fds.get(fd),
            Some(FdHandle::MemoryObject { .. })
        ) {
            return self.write_memory_object_fd_index(fd, &data);
        }
        let mut wake_fd_waiters = false;
        let result = match &mut self.process_mut()?.fds[fd] {
            FdHandle::Stdout => {
                let mut out = io::stdout();
                out.write_all(&data).and_then(|()| out.flush())
            }
            FdHandle::Stderr => {
                let mut err = io::stderr();
                err.write_all(&data).and_then(|()| err.flush())
            }
            FdHandle::File(file) => file.write_all(&data),
            FdHandle::PipeWriter(buffer) => {
                let result = buffer
                    .borrow_mut()
                    .push_bytes(&data)
                    .map_err(|errno| io::Error::from_raw_os_error(errno as i32));
                if result.is_ok() && !data.is_empty() {
                    wake_fd_waiters = true;
                }
                result
            }
            FdHandle::Counter(value) => {
                let next = if data.len() >= 8 {
                    u64::from_le_bytes(data[..8].try_into().unwrap())
                } else {
                    data.iter()
                        .enumerate()
                        .fold(0u64, |acc, (idx, byte)| acc | ((*byte as u64) << (idx * 8)))
                };
                *value.borrow_mut() = next;
                Ok(())
            }
            FdHandle::EventCounter { value, .. } => {
                let addend = if data.len() >= 8 {
                    u64::from_le_bytes(data[..8].try_into().unwrap())
                } else {
                    data.iter()
                        .enumerate()
                        .fold(0u64, |acc, (idx, byte)| acc | ((*byte as u64) << (idx * 8)))
                };
                let mut value = value.borrow_mut();
                *value = value.saturating_add(addend);
                if addend != 0 {
                    wake_fd_waiters = true;
                }
                Ok(())
            }
            FdHandle::Timer(timer) => {
                let ticks = if data.len() >= 8 {
                    u64::from_le_bytes(data[..8].try_into().unwrap())
                } else {
                    data.iter()
                        .enumerate()
                        .fold(0u64, |acc, (idx, byte)| acc | ((*byte as u64) << (idx * 8)))
                };
                let mut timer = timer.borrow_mut();
                timer.remaining = ticks;
                timer.interval = 0;
                timer.expirations = 0;
                Ok(())
            }
            FdHandle::TcpListener { pending, .. } => {
                if let Some(stream) = pending {
                    stream.write_all(&data)
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::NotConnected,
                        "listener has no accepted stream",
                    ))
                }
            }
            FdHandle::TcpStream(stream) => stream.write_all(&data),
            FdHandle::Stdin
            | FdHandle::MessageEndpoint
            | FdHandle::Dir { .. }
            | FdHandle::PipeReader(_)
            | FdHandle::MemoryObject { .. }
            | FdHandle::TcpSocket { .. }
            | FdHandle::DmaBuffer { .. }
            | FdHandle::CallGate { .. }
            | FdHandle::ClassifierTable(_)
            | FdHandle::ServiceletProgram(_)
            | FdHandle::Closed => Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "fd is not writable",
            )),
        };
        match result {
            Ok(()) => {
                self.complete_ok(data.len() as u64)?;
                if wake_fd_waiters {
                    self.poll_fd_waiters();
                }
            }
            Err(err) => self.set_status_io_error(err)?,
        }
        Ok(())
    }

    fn write_memory_object_fd_index(&mut self, fd: usize, data: &[u8]) -> Result<(), String> {
        let (object, start) = {
            let process = self.process()?;
            match process.fds.get(fd) {
                Some(FdHandle::MemoryObject { data, pos }) => (Rc::clone(data), *pos),
                Some(_) => return Err("fd is not a memory object".to_string()),
                None => return Err(format!("fd index out of range: {fd}")),
            }
        };
        let Some(end) = start.checked_add(data.len()) else {
            self.set_status_errno(12)?;
            return Ok(());
        };
        if end > MEMORY_SIZE {
            self.set_status_errno(12)?;
            return Ok(());
        }
        let current_len = object.borrow().len();
        let growth = end.saturating_sub(current_len);
        if growth != 0 {
            if let Err(errno) = self.ensure_domain_budget_errno(growth as u64, 0, 0, 0) {
                self.set_status_errno(errno)?;
                return Ok(());
            }
        }

        {
            let mut object = object.borrow_mut();
            if end > object.len() {
                object.resize(end, 0);
            }
            object[start..end].copy_from_slice(data);
        }
        match &mut self.process_mut()?.fds[fd] {
            FdHandle::MemoryObject { pos, .. } => *pos = end,
            _ => return Err("fd is not a memory object".to_string()),
        }
        self.complete_ok(data.len() as u64)
    }

    fn pwrite_fd_index(
        &mut self,
        fd: usize,
        addr: u64,
        len: usize,
        offset: u64,
    ) -> Result<(), String> {
        if self.ensure_fd_right(fd, CAP_RIGHT_WRITE).is_err() {
            return Ok(());
        }
        let data = self.read_bytes(addr, len)?;
        let result = match &mut self.process_mut()?.fds[fd] {
            FdHandle::File(file) => {
                let mut written = 0usize;
                while written < data.len() {
                    let count = match file.write_at(&data[written..], offset + written as u64) {
                        Ok(count) => count,
                        Err(err) => return self.set_status_io_error(err),
                    };
                    if count == 0 {
                        return Err("PWRITE_FD wrote zero bytes".to_string());
                    }
                    written += count;
                }
                Ok(())
            }
            _ => Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "fd does not support offset writes",
            )),
        };
        match result {
            Ok(()) => self.complete_ok(data.len() as u64)?,
            Err(err) => self.set_status_io_error(err)?,
        }
        Ok(())
    }

    fn readdir_fd_index(&mut self, fd: usize, addr: u64) -> Result<(), String> {
        let entry = match &self.process()?.fds[fd] {
            FdHandle::Dir { entries, pos, .. } => {
                if *pos >= entries.len() {
                    None
                } else {
                    Some(entries[*pos].clone())
                }
            }
            _ => {
                self.set_status_errno(20)?;
                None
            }
        };
        if let Some(entry) = entry {
            let mut bytes = entry.into_bytes();
            bytes.push(0);
            self.write_bytes(addr, &bytes)?;
            if let FdHandle::Dir { pos, .. } = &mut self.process_mut()?.fds[fd] {
                *pos += 1;
            }
            self.set_errno(0)?;
            self.write_reg(Reg(1), 1)?;
        } else if self.read_reg(Reg(1))? != -1i64 as u64 {
            self.set_errno(0)?;
            self.write_reg(Reg(1), 0)?;
        }
        Ok(())
    }

    fn rewinddir_fd_index(&mut self, fd: usize) -> Result<(), String> {
        match &mut self.process_mut()?.fds[fd] {
            FdHandle::Dir { pos, .. } => {
                *pos = 0;
                self.set_status_ok()
            }
            _ => self.set_status_errno(20),
        }
    }

    fn utime_path(&mut self, path: &str, times_ptr: u64, flags: c_int) -> Result<(), String> {
        let times = self.host_timespec_pair(times_ptr)?;
        let times_ptr = times
            .as_ref()
            .map(|pair| pair.as_ptr())
            .unwrap_or(std::ptr::null());
        let c_path = CString::new(Path::new(path).as_os_str().as_bytes())
            .map_err(|_| "UTIME_PATH path contains NUL byte".to_string())?;
        let rc = unsafe { utimensat(-100, c_path.as_ptr(), times_ptr, flags) };
        if rc == 0 {
            self.set_status_ok()
        } else {
            self.set_status_io_error(io::Error::last_os_error())
        }
    }

    fn utime_fd_index(&mut self, fd: usize, times_ptr: u64) -> Result<(), String> {
        let times = self.host_timespec_pair(times_ptr)?;
        let times_ptr = times
            .as_ref()
            .map(|pair| pair.as_ptr())
            .unwrap_or(std::ptr::null());
        let raw_fd = match &self.process()?.fds[fd] {
            FdHandle::File(file) => Some(file.as_raw_fd()),
            _ => None,
        };
        let Some(raw_fd) = raw_fd else {
            return self.set_status_errno(9);
        };
        let rc = unsafe { futimens(raw_fd, times_ptr) };
        if rc == 0 {
            self.set_status_ok()
        } else {
            self.set_status_io_error(io::Error::last_os_error())
        }
    }

    fn host_timespec_pair(&mut self, times_ptr: u64) -> Result<Option<[HostTimespec; 2]>, String> {
        if times_ptr == 0 {
            return Ok(None);
        }
        let now = Self::system_time_to_host_timespec(SystemTime::now());
        let atime = self.host_timespec_at(times_ptr, now)?;
        let mtime = self.host_timespec_at(
            times_ptr
                .checked_add(16)
                .ok_or_else(|| "address overflow".to_string())?,
            now,
        )?;
        Ok(Some([atime, mtime]))
    }

    fn host_timespec_at(&mut self, addr: u64, now: HostTimespec) -> Result<HostTimespec, String> {
        let sec = self.load_u64_offset(addr, 0)? as i64;
        let nsec = self.load_u64_offset(addr, 8)? as i64;
        if nsec == UTIME_NOW_LNP64 {
            Ok(now)
        } else if nsec == UTIME_OMIT_LNP64 {
            Ok(HostTimespec {
                tv_sec: 0,
                tv_nsec: UTIME_OMIT_LNP64,
            })
        } else {
            Ok(HostTimespec {
                tv_sec: sec,
                tv_nsec: nsec,
            })
        }
    }

    fn system_time_to_host_timespec(time: SystemTime) -> HostTimespec {
        let duration = time
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0));
        HostTimespec {
            tv_sec: duration.as_secs() as i64,
            tv_nsec: duration.subsec_nanos() as i64,
        }
    }

    fn stat_fd_index(&mut self, statbuf: u64, fd: usize) -> Result<(), String> {
        if self.ensure_fd_right(fd, CAP_RIGHT_STAT).is_err() {
            return Ok(());
        }
        let metadata = match &self.process()?.fds[fd] {
            FdHandle::File(file) => Some(file.metadata().map_err(|err| Self::errno_from_io(&err))),
            FdHandle::Dir { path, .. } => {
                Some(fs::metadata(path).map_err(|err| Self::errno_from_io(&err)))
            }
            _ => None,
        };
        match metadata {
            Some(Ok(metadata)) => {
                self.write_lnp64_stat(statbuf, &metadata)?;
                self.set_status_ok()?;
            }
            Some(Err(errno)) => self.set_status_errno(errno)?,
            None => {
                self.write_synthetic_stat(statbuf, 0o020000, 0)?;
                self.set_status_ok()?;
            }
        }
        Ok(())
    }

    fn fd_seek_index(&mut self, fd: usize, offset: i64, whence: u64) -> Result<(), String> {
        if self.ensure_fd_right(fd, CAP_RIGHT_SEEK).is_err() {
            return Ok(());
        }
        let seek_from = match whence {
            0 => Some(SeekFrom::Start(offset as u64)),
            1 => Some(SeekFrom::Current(offset)),
            2 => Some(SeekFrom::End(offset)),
            _ => None,
        };
        if let Some(seek_from) = seek_from {
            let result = match &mut self.process_mut()?.fds[fd] {
                FdHandle::File(file) => file.seek(seek_from),
                _ => Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "fd is not seekable",
                )),
            };
            match result {
                Ok(pos) => self.complete_ok(pos)?,
                Err(err) => self.set_status_io_error(err)?,
            }
        } else {
            self.set_status_errno(22)?;
        }
        Ok(())
    }

    fn mprotect_range(&mut self, addr: u64, len: u64, prot: u64) -> Result<(), String> {
        if len == 0 {
            self.set_status_errno(22)?;
            return Ok(());
        }
        if prot & !0b111 != 0 {
            self.set_status_errno(22)?;
            return Ok(());
        }
        if !self.domain_allows_prot(prot)? {
            self.set_status_errno(1)?;
            return Ok(());
        }
        let Some(end) = addr.checked_add(len) else {
            self.set_status_errno(22)?;
            return Ok(());
        };
        let idx = {
            let process = self.process()?;
            if let Some(idx) = process
                .vmas
                .iter()
                .position(|vma| vma.start == addr && vma.len == len)
            {
                idx
            } else if process.vmas.iter().any(|vma| {
                vma.start
                    .checked_add(vma.len)
                    .is_some_and(|vma_end| addr >= vma.start && end <= vma_end)
            }) {
                self.set_status_errno(22)?;
                return Ok(());
            } else {
                self.set_status_errno(12)?;
                return Ok(());
            }
        };
        let (old_prot, file_backed) = {
            let vma = &self.process()?.vmas[idx];
            (vma.prot, vma.file.is_some())
        };
        let adds_execute = old_prot & 0b100 == 0 && prot & 0b100 != 0;
        if adds_execute && !self.domain_allows_executable_source(prot, file_backed)? {
            self.set_status_errno(1)?;
            return Ok(());
        }
        self.process_mut()?.vmas[idx].prot = prot;
        self.set_status_ok()?;
        Ok(())
    }

    fn isync_range(&mut self, result: Reg, addr: u64, len: u64) -> Result<(), String> {
        if len == 0 {
            return self.complete_reg_err(result, 22);
        }
        let Some(end) = addr.checked_add(len) else {
            return self.complete_reg_err(result, 22);
        };
        let process = self.process()?;
        let Some(vma) = process.vmas.iter().find(|vma| {
            let vma_end = vma.start.saturating_add(vma.len);
            addr >= vma.start && end <= vma_end
        }) else {
            return self.complete_reg_err(result, 14);
        };
        if vma.guard || vma.prot == 0 || vma.prot & 0b101 == 0 {
            return self.complete_reg_err(result, 14);
        }
        self.complete_reg_ok(result, 0)
    }

    fn read_fd_index(&mut self, fd: usize, addr: u64, len: usize) -> Result<Option<usize>, String> {
        if self.ensure_fd_right(fd, CAP_RIGHT_READ).is_err() {
            return Ok(None);
        }
        self.ensure_mapped(addr, len, true)?;
        enum ReadCommit {
            None,
            Pipe {
                buffer: Rc<RefCell<PipeBuffer>>,
                count: usize,
            },
            EventCounter {
                value: Rc<RefCell<u64>>,
                semaphore: bool,
            },
            MemoryObject {
                end: usize,
            },
            Timer {
                timer: Rc<RefCell<TimerState>>,
            },
        }
        let mut tmp = vec![0; len];
        let mut commit = ReadCommit::None;
        let count = match &mut self.process_mut()?.fds[fd] {
            FdHandle::Stdin => io::stdin()
                .read(&mut tmp)
                .map_err(|err| format!("READ_FD fd0: {err}"))?,
            FdHandle::File(file) => file
                .read(&mut tmp)
                .map_err(|err| format!("READ_FD fd{fd}: {err}"))?,
            FdHandle::PipeReader(buffer) => {
                let buffer_ref = buffer.borrow();
                let count = len.min(buffer_ref.bytes.len());
                for (dst, byte) in tmp.iter_mut().zip(buffer_ref.bytes.iter()).take(count) {
                    *dst = *byte;
                }
                drop(buffer_ref);
                if count != 0 {
                    commit = ReadCommit::Pipe {
                        buffer: Rc::clone(buffer),
                        count,
                    };
                }
                count
            }
            FdHandle::Counter(value) => {
                let bytes = value.borrow().to_le_bytes();
                let count = len.min(bytes.len());
                tmp[..count].copy_from_slice(&bytes[..count]);
                count
            }
            FdHandle::EventCounter { value, semaphore } => {
                let observed = *value.borrow();
                if observed == 0 {
                    0
                } else {
                    let observed = if *semaphore { 1 } else { observed };
                    let bytes = observed.to_le_bytes();
                    let count = len.min(bytes.len());
                    tmp[..count].copy_from_slice(&bytes[..count]);
                    if count != 0 {
                        commit = ReadCommit::EventCounter {
                            value: Rc::clone(value),
                            semaphore: *semaphore,
                        };
                    }
                    count
                }
            }
            FdHandle::MemoryObject { data, pos } => {
                let data = data.borrow();
                let available = data.len().saturating_sub(*pos);
                let count = len.min(available);
                tmp[..count].copy_from_slice(&data[*pos..*pos + count]);
                if count != 0 {
                    commit = ReadCommit::MemoryObject { end: *pos + count };
                }
                count
            }
            FdHandle::Timer(timer) => {
                let expirations = timer.borrow().expirations;
                if expirations == 0 {
                    0
                } else {
                    let bytes = expirations.to_le_bytes();
                    let count = len.min(bytes.len());
                    tmp[..count].copy_from_slice(&bytes[..count]);
                    if count != 0 {
                        commit = ReadCommit::Timer {
                            timer: Rc::clone(timer),
                        };
                    }
                    count
                }
            }
            FdHandle::TcpListener { listener, pending } => {
                if pending.is_none() {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            stream
                                .set_nonblocking(false)
                                .map_err(|err| format!("READ_FD fd{fd} stream blocking: {err}"))?;
                            *pending = Some(stream);
                        }
                        Err(err) if err.kind() == io::ErrorKind::WouldBlock => {}
                        Err(err) => {
                            return Err(format!("READ_FD fd{fd} accept: {err}"));
                        }
                    };
                }
                if let Some(stream) = pending {
                    stream
                        .read(&mut tmp)
                        .map_err(|err| format!("READ_FD fd{fd} stream: {err}"))?
                } else {
                    0
                }
            }
            FdHandle::TcpStream(stream) => match stream.read(&mut tmp) {
                Ok(count) => count,
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => 0,
                Err(err) => return Err(format!("READ_FD fd{fd} TCP stream: {err}")),
            },
            FdHandle::Stdout
            | FdHandle::Stderr
            | FdHandle::MessageEndpoint
            | FdHandle::Dir { .. }
            | FdHandle::PipeWriter(_)
            | FdHandle::TcpSocket { .. }
            | FdHandle::DmaBuffer { .. }
            | FdHandle::CallGate { .. }
            | FdHandle::ClassifierTable(_)
            | FdHandle::ServiceletProgram(_)
            | FdHandle::Closed => 0,
        };
        self.write_bytes(addr, &tmp[..count])?;
        match commit {
            ReadCommit::None => {}
            ReadCommit::Pipe { buffer, count } => {
                let mut buffer = buffer.borrow_mut();
                for _ in 0..count {
                    buffer.bytes.pop_front();
                }
                drop(buffer);
                self.poll_fd_waiters();
            }
            ReadCommit::EventCounter { value, semaphore } => {
                let mut value = value.borrow_mut();
                if semaphore {
                    *value = value.saturating_sub(1);
                } else {
                    *value = 0;
                }
            }
            ReadCommit::MemoryObject { end } => match &mut self.process_mut()?.fds[fd] {
                FdHandle::MemoryObject { pos, .. } => *pos = end,
                _ => return Err("fd is not a memory object".to_string()),
            },
            ReadCommit::Timer { timer } => {
                timer.borrow_mut().expirations = 0;
            }
        }
        Ok(Some(count))
    }

    fn pread_fd_index(
        &mut self,
        fd: usize,
        addr: u64,
        len: usize,
        offset: u64,
    ) -> Result<(), String> {
        if self.ensure_fd_right(fd, CAP_RIGHT_READ).is_err() {
            return Ok(());
        }
        self.ensure_mapped(addr, len, true)?;
        let mut tmp = vec![0; len];
        let result = match &mut self.process_mut()?.fds[fd] {
            FdHandle::File(file) => file.read_at(&mut tmp, offset),
            _ => Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "fd does not support offset reads",
            )),
        };
        match result {
            Ok(count) => {
                self.write_bytes(addr, &tmp[..count])?;
                self.complete_ok(count as u64)?;
            }
            Err(err) => self.set_status_io_error(err)?,
        }
        Ok(())
    }

    fn object_ctl(&mut self, result: Reg, argblock: u64) -> Result<(), String> {
        Self::ensure_result_reg_writable(result)?;
        let op = self.load_u64_offset(argblock, 0)?;
        let value = match op {
            OBJECT_OP_CREATE => self.object_ctl_create(argblock),
            OBJECT_OP_SOCKET_BIND => self.object_ctl_socket_bind(argblock),
            OBJECT_OP_SOCKET_LISTEN => self.object_ctl_socket_listen(argblock),
            OBJECT_OP_SOCKET_CONNECT => self.object_ctl_socket_connect(argblock),
            OBJECT_OP_SOCKET_ACCEPT => self.object_ctl_socket_accept(argblock),
            OBJECT_OP_SOCKET_GETSOCKNAME => self.object_ctl_socket_getsockname(argblock),
            OBJECT_OP_SOCKET_GETSOCKOPT => self.object_ctl_socket_getsockopt(argblock),
            OBJECT_OP_SOCKET_SETSOCKOPT => self.object_ctl_socket_setsockopt(argblock),
            OBJECT_OP_CLASSIFY => self.object_ctl_classify(argblock),
            OBJECT_OP_CLASSIFIER_QUERY => self.object_ctl_classifier_query(argblock),
            _ => Err(22),
        };
        match value {
            Ok(value) => self.complete_reg_ok(result, value),
            Err(errno) => self.complete_reg_err(result, errno),
        }
    }

    fn dma_ctl(&mut self, result: Reg, argblock: u64) -> Result<(), String> {
        Self::ensure_result_reg_writable(result)?;
        if !self.current_domain_dma_allowed()? {
            return self.complete_reg_err(result, 1);
        }
        let op = self.load_u64_offset(argblock, 0)?;
        let dst = self.load_u64_offset(argblock, 8)?;
        let src_or_value = self.load_u64_offset(argblock, 16)?;
        let len = self.load_u64_offset(argblock, 24)? as usize;
        let dma_buffer = self.load_u64_offset(argblock, 32)?;
        if dma_buffer != 0 {
            let validation = self.validate_dma_buffer(dma_buffer, op, dst, src_or_value, len);
            if let Err(errno) = validation {
                return self.complete_reg_err(result, errno);
            }
        }
        let outcome: Result<u64, u64> = match op {
            DMA_OP_COPY => {
                if self.ensure_mapped(dst, len, true).is_err() {
                    Err(14)
                } else {
                    self.read_bytes(src_or_value, len)
                        .map_err(|_| 14u64)
                        .and_then(|bytes| {
                            self.write_bytes(dst, &bytes)
                                .map(|_| len as u64)
                                .map_err(|_| 14u64)
                        })
                }
            }
            DMA_OP_FILL => {
                if self.ensure_mapped(dst, len, true).is_err() {
                    Err(14)
                } else {
                    let bytes = vec![src_or_value as u8; len];
                    self.write_bytes(dst, &bytes)
                        .map(|_| len as u64)
                        .map_err(|_| 14u64)
                }
            }
            _ => Err(22),
        };
        match outcome {
            Ok(count) => self.complete_reg_ok(result, count),
            Err(errno) => self.complete_reg_err(result, errno),
        }
    }

    fn validate_dma_buffer(
        &mut self,
        fd_value: u64,
        op: u64,
        dst: u64,
        src_or_value: u64,
        len: usize,
    ) -> Result<(), u64> {
        let fd = self.decode_fd_value(fd_value)?;
        let required_rights = match op {
            DMA_OP_COPY => CAP_RIGHT_READ | CAP_RIGHT_WRITE,
            DMA_OP_FILL => CAP_RIGHT_WRITE,
            _ => return Err(22),
        };
        self.fd_right_errno(fd, required_rights)?;
        let process = self.process().map_err(|_| 3u64)?;
        let (addr, buffer_len) = match process.fds.get(fd).ok_or(9u64)? {
            FdHandle::DmaBuffer { addr, len } => (*addr, *len),
            _ => return Err(9),
        };
        if !range_within(addr, buffer_len, dst, len) {
            return Err(14);
        }
        if op == DMA_OP_COPY && !range_within(addr, buffer_len, src_or_value, len) {
            return Err(14);
        }
        Ok(())
    }

    fn cap_send(&mut self, result: Reg, argblock: u64) -> Result<(), String> {
        Self::ensure_result_reg_writable(result)?;
        let channel_value = self.load_u64_offset(argblock, 0)?;
        let src_value = self.load_u64_offset(argblock, 8)?;
        let flags = self.load_u64_offset(argblock, 24)?;
        let value = self.cap_send_inner(channel_value, src_value, flags);
        match value {
            Ok(count) => self.complete_reg_ok(result, count),
            Err(errno) => self.complete_reg_err(result, errno),
        }
    }

    fn cap_send_inner(
        &mut self,
        channel_value: u64,
        src_value: u64,
        flags: u64,
    ) -> Result<u64, u64> {
        if flags & !CAP_SEND_FLAG_MOVE != 0 {
            return Err(22);
        }
        let channel = self.decode_fd_value(channel_value)?;
        let src = self.decode_fd_value(src_value)?;
        self.fd_right_errno(channel, CAP_RIGHT_WRITE | CAP_RIGHT_TRANSFER)?;
        self.fd_right_errno(src, CAP_RIGHT_TRANSFER)?;

        let (queue, payload) = {
            let process = self.process().map_err(|_| 3u64)?;
            let queue = match process.fds.get(channel).ok_or(9u64)? {
                FdHandle::PipeWriter(queue) => Rc::clone(queue),
                _ => return Err(9),
            };
            let handle = process
                .fds
                .get(src)
                .ok_or(9u64)?
                .clone_handle()
                .map_err(|_| 9u64)?;
            let capability = process.fd_capabilities.get(src).copied().ok_or(9u64)?;
            if capability.revoked {
                return Err(116);
            }
            (queue, CapabilityPayload { handle, capability })
        };

        queue.borrow_mut().push_capability(payload)?;
        if flags & CAP_SEND_FLAG_MOVE != 0 {
            self.close_fd_index(src).map_err(|_| 9u64)?;
        }
        self.poll_fd_waiters();
        Ok(1)
    }

    fn cap_recv(&mut self, result: Reg, argblock: u64) -> Result<(), String> {
        Self::ensure_result_reg_writable(result)?;
        let channel_value = self.load_u64_offset(argblock, 0)?;
        let dst_req = self.load_u64_offset(argblock, 8)?;
        let rights_req = self.load_u64_offset(argblock, 16)?;
        let flags = self.load_u64_offset(argblock, 24)?;
        let value = self.cap_recv_inner(channel_value, dst_req, rights_req, flags);
        match value {
            Ok(token) => self.complete_reg_ok(result, token),
            Err(errno) => self.complete_reg_err(result, errno),
        }
    }

    fn cap_recv_inner(
        &mut self,
        channel_value: u64,
        dst_req: u64,
        rights_req: u64,
        flags: u64,
    ) -> Result<u64, u64> {
        if flags != 0 {
            return Err(22);
        }
        let channel = self.decode_fd_value(channel_value)?;
        self.fd_right_errno(channel, CAP_RIGHT_READ | CAP_RIGHT_TRANSFER)?;
        let queue = {
            let process = self.process().map_err(|_| 3u64)?;
            match process.fds.get(channel).ok_or(9u64)? {
                FdHandle::PipeReader(queue) => Rc::clone(queue),
                _ => return Err(9),
            }
        };

        let source_capability = queue
            .borrow()
            .capabilities
            .front()
            .map(|payload| payload.capability)
            .ok_or(11u64)?;
        if source_capability.revoked {
            return Err(116);
        }
        let rights = if rights_req == 0 {
            source_capability.rights
        } else {
            rights_req
        };
        if rights & !source_capability.rights != 0 {
            return Err(1);
        }
        if rights != source_capability.rights && !source_capability.narrowable {
            return Err(1);
        }

        let dst = if dst_req == 0 {
            self.ensure_domain_budget_errno(0, 0, 0, 1)?;
            let process = self.process().map_err(|_| 3u64)?;
            process
                .fds
                .iter()
                .enumerate()
                .find(|(idx, candidate)| {
                    *idx != MESSAGE_ENDPOINT_FD && matches!(candidate, FdHandle::Closed)
                })
                .map(|(idx, _)| idx)
                .ok_or(24u64)?
        } else {
            if dst_req as usize >= FDR_COUNT || dst_req as usize == MESSAGE_ENDPOINT_FD {
                return Err(9);
            }
            let fd = dst_req as usize;
            let delta = self.fd_slot_delta(fd).map_err(|_| 9u64)?;
            self.ensure_domain_budget_errno(0, 0, 0, delta)?;
            fd
        };

        self.release_fd_locks_for_replacement(dst)
            .map_err(|_| 9u64)?;
        let mut payload = queue.borrow_mut().capabilities.pop_front().ok_or(11u64)?;
        payload.capability.rights = rights;
        payload.capability.narrowable = payload.capability.narrowable && !payload.capability.sealed;
        self.install_fd_capability(dst, payload.capability)
            .map_err(|_| 9u64)?;
        self.bump_fd_generation(dst).map_err(|_| 9u64)?;
        self.process_mut().map_err(|_| 3u64)?.fds[dst] = payload.handle;
        self.poll_fd_waiters();
        self.fd_token(dst).map_err(|_| 9u64)
    }

    fn cap_dup(&mut self, result: Reg, argblock: u64) -> Result<(), String> {
        Self::ensure_result_reg_writable(result)?;
        let src_value = self.load_u64_offset(argblock, 0)?;
        let dst_req = self.load_u64_offset(argblock, 8)?;
        let rights_req = self.load_u64_offset(argblock, 16)?;
        let flags = self.load_u64_offset(argblock, 24)?;
        let value = self.cap_dup_inner(src_value, dst_req, rights_req, flags);
        match value {
            Ok(token) => self.complete_reg_ok(result, token),
            Err(errno) => self.complete_reg_err(result, errno),
        }
    }

    fn cap_dup_inner(
        &mut self,
        src_value: u64,
        dst_req: u64,
        rights_req: u64,
        flags: u64,
    ) -> Result<u64, u64> {
        if flags & !CAP_DUP_FLAG_SEAL != 0 {
            return Err(22);
        }
        let src = self.decode_fd_value(src_value)?;
        let source_cap = self
            .process()
            .map_err(|_| 3u64)?
            .fd_capabilities
            .get(src)
            .copied()
            .ok_or(9u64)?;
        let rights = if rights_req == 0 {
            source_cap.rights
        } else {
            rights_req
        };
        let sealed = flags & CAP_DUP_FLAG_SEAL != 0;
        let handle = self
            .process()
            .map_err(|_| 3u64)?
            .fds
            .get(src)
            .ok_or(9u64)?
            .clone_handle()
            .map_err(|_| 9u64)?;
        let dst = if dst_req == 0 {
            self.ensure_domain_budget_errno(0, 0, 0, 1)?;
            let process = self.process().map_err(|_| 3u64)?;
            process
                .fds
                .iter()
                .enumerate()
                .find(|(idx, candidate)| {
                    *idx != MESSAGE_ENDPOINT_FD && matches!(candidate, FdHandle::Closed)
                })
                .map(|(idx, _)| idx)
                .ok_or(24u64)?
        } else {
            if dst_req as usize >= FDR_COUNT || dst_req as usize == MESSAGE_ENDPOINT_FD {
                return Err(9);
            }
            let fd = dst_req as usize;
            let delta = self.fd_slot_delta(fd).map_err(|_| 9u64)?;
            self.ensure_domain_budget_errno(0, 0, 0, delta)?;
            fd
        };
        if dst != src {
            self.release_fd_locks_for_replacement(dst)
                .map_err(|_| 9u64)?;
        }
        self.duplicate_fd_capability(src, dst, rights, sealed)?;
        self.bump_fd_generation(dst).map_err(|_| 9u64)?;
        self.process_mut().map_err(|_| 3u64)?.fds[dst] = handle;
        self.fd_token(dst).map_err(|_| 9u64)
    }

    fn cap_revoke(&mut self, result: Reg, argblock: u64) -> Result<(), String> {
        Self::ensure_result_reg_writable(result)?;
        let src_value = self.load_u64_offset(argblock, 0)?;
        let value = self.cap_revoke_inner(src_value);
        match value {
            Ok(count) => self.complete_reg_ok(result, count),
            Err(errno) => self.complete_reg_err(result, errno),
        }
    }

    fn cap_revoke_inner(&mut self, src_value: u64) -> Result<u64, u64> {
        let src = self.decode_fd_value(src_value)?;
        let source = self
            .process()
            .map_err(|_| 3u64)?
            .fd_capabilities
            .get(src)
            .copied()
            .ok_or(9u64)?;
        if source.revoked {
            return Err(116);
        }
        if !source.revocable || source.rights & CAP_RIGHT_REVOKE == 0 {
            return Err(1);
        }
        let lineage = source.lineage;
        let targets = self
            .process()
            .map_err(|_| 3u64)?
            .fd_capabilities
            .iter()
            .enumerate()
            .filter_map(|(idx, cap)| (cap.lineage == lineage && !cap.revoked).then_some(idx))
            .collect::<Vec<_>>();
        for fd in &targets {
            self.process_mut().map_err(|_| 3u64)?.fd_capabilities[*fd].revoked = true;
            self.bump_fd_generation(*fd).map_err(|_| 9u64)?;
        }
        Ok(targets.len() as u64)
    }

    fn object_ctl_create(&mut self, argblock: u64) -> NativeResult<u64> {
        self.require_domain_cap_errno(DOMAIN_CAP_OBJECT | DOMAIN_CAP_FDR)?;
        let kind_code = self.load_u64_offset(argblock, 8).map_err(|_| 14u64)?;
        let profile_code = self.load_u64_offset(argblock, 16).map_err(|_| 14u64)?;
        let kind = ObjectKind::from_code(kind_code).ok_or(22u64)?;
        let profile = ObjectProfile::from_code_for_kind(kind, profile_code).ok_or(22u64)?;
        let fd0_req = self.load_u64_offset(argblock, 24).map_err(|_| 14u64)?;
        let fd1_req = self.load_u64_offset(argblock, 32).map_err(|_| 14u64)?;
        let arg = self.load_u64_offset(argblock, 40).map_err(|_| 14u64)?;
        match (kind, profile) {
            (ObjectKind::Queue, ObjectProfile::Pipe) => {
                self.validate_object_fd_request(fd0_req)?;
                self.validate_object_fd_request(fd1_req)?;
                if fd0_req != 0 && fd0_req == fd1_req {
                    return Err(22);
                }
                let read_excluded = (fd1_req != 0).then_some(fd1_req as usize);
                let (read_fd, read_delta) = self.plan_object_fd_slot(fd0_req, read_excluded)?;
                let (write_fd, write_delta) = self.plan_object_fd_slot(fd1_req, Some(read_fd))?;
                self.ensure_domain_budget_errno(0, 0, 0, read_delta + write_delta)?;
                self.prevalidate_object_create_outputs(argblock, 2)?;
                let buffer = Rc::new(RefCell::new(PipeBuffer::default()));
                let read_fd = self
                    .install_object_fd(read_fd as u64, FdHandle::PipeReader(Rc::clone(&buffer)))?;
                let write_fd =
                    self.install_object_fd(write_fd as u64, FdHandle::PipeWriter(buffer))?;
                self.store_u64_offset(argblock, 24, read_fd as u64)
                    .map_err(|_| 14u64)?;
                self.store_u64_offset(argblock, 32, write_fd as u64)
                    .map_err(|_| 14u64)?;
                Ok(0)
            }
            (ObjectKind::Queue, ObjectProfile::CallGate) => {
                self.require_domain_cap_errno(DOMAIN_CAP_CALL)?;
                let target_domain = if fd1_req == 0 {
                    self.current_domain_id().map_err(|_| 3u64)?
                } else {
                    self.domain_ref(fd1_req, 0)?
                };
                let target_generation = self.domains.get(&target_domain).ok_or(3u64)?.generation;
                if !self.domain_is_descendant_or_self(
                    target_domain,
                    self.current_domain_id().map_err(|_| 3u64)?,
                ) {
                    return Err(1);
                }
                let mode = self.load_u64_offset(argblock, 48).map_err(|_| 14u64)?;
                let completion_fd = self.load_u64_offset(argblock, 56).map_err(|_| 14u64)?;
                let flags = self.load_u64_offset(argblock, 64).map_err(|_| 14u64)?;
                if !matches!(mode, CALL_MODE_SYNC | CALL_MODE_ASYNC | CALL_MODE_HANDOFF) {
                    return Err(22);
                }
                if flags & !CALL_GATE_FLAG_CAP_PASS != 0 {
                    return Err(22);
                }
                let (completion_fd, completion_generation) = if completion_fd == 0 {
                    (None, None)
                } else if completion_fd as usize >= FDR_COUNT {
                    return Err(9);
                } else {
                    let fd = completion_fd as usize;
                    self.fd_right_errno(fd, CAP_RIGHT_WRITE)?;
                    if !self.fd_accepts_call_completion(fd).map_err(|_| 9u64)? {
                        return Err(22);
                    }
                    (Some(fd), Some(self.fd_generation(fd).map_err(|_| 9u64)?))
                };
                if mode == CALL_MODE_ASYNC && completion_fd.is_none() {
                    return Err(22);
                }
                self.prevalidate_object_create_outputs(argblock, 1)?;
                let fd = self.install_object_fd(
                    fd0_req,
                    FdHandle::CallGate {
                        entry: arg as usize,
                        domain_id: target_domain,
                        domain_generation: target_generation,
                        mode,
                        completion_fd,
                        completion_generation,
                        flags,
                    },
                )?;
                self.store_u64_offset(argblock, 24, fd as u64)
                    .map_err(|_| 14u64)?;
                Ok(fd as u64)
            }
            (ObjectKind::Counter, ObjectProfile::EventFd) => {
                let flags = self.load_u64_offset(argblock, 48).map_err(|_| 14u64)?;
                if flags & !(EVENTFD_SEMAPHORE | EVENTFD_NONBLOCK) != 0 {
                    return Err(22);
                }
                self.prevalidate_object_create_outputs(argblock, 1)?;
                let fd = self.install_object_fd(
                    fd0_req,
                    FdHandle::EventCounter {
                        value: Rc::new(RefCell::new(arg)),
                        semaphore: flags & EVENTFD_SEMAPHORE != 0,
                    },
                )?;
                self.store_u64_offset(argblock, 24, fd as u64)
                    .map_err(|_| 14u64)?;
                Ok(fd as u64)
            }
            (ObjectKind::Counter, _) => {
                self.prevalidate_object_create_outputs(argblock, 1)?;
                let fd =
                    self.install_object_fd(fd0_req, FdHandle::Counter(Rc::new(RefCell::new(arg))))?;
                self.store_u64_offset(argblock, 24, fd as u64)
                    .map_err(|_| 14u64)?;
                Ok(fd as u64)
            }
            (ObjectKind::MemoryObject, _) => {
                if arg == 0 {
                    return Err(22);
                }
                if arg > MEMORY_SIZE as u64 {
                    return Err(12);
                }
                self.ensure_domain_budget_errno(arg, 0, 0, 0)?;
                let len = usize::try_from(arg).map_err(|_| 12u64)?;
                self.prevalidate_object_create_outputs(argblock, 1)?;
                let fd = self.install_object_fd(
                    fd0_req,
                    FdHandle::MemoryObject {
                        data: Rc::new(RefCell::new(vec![0; len])),
                        pos: 0,
                    },
                )?;
                self.store_u64_offset(argblock, 24, fd as u64)
                    .map_err(|_| 14u64)?;
                Ok(fd as u64)
            }
            (ObjectKind::Timer, _) => {
                self.prevalidate_object_create_outputs(argblock, 1)?;
                let fd = self.install_object_fd(
                    fd0_req,
                    FdHandle::Timer(Rc::new(RefCell::new(TimerState::default()))),
                )?;
                self.store_u64_offset(argblock, 24, fd as u64)
                    .map_err(|_| 14u64)?;
                Ok(fd as u64)
            }
            (ObjectKind::Classifier, ObjectProfile::ClassifierTable) => {
                let rules_ptr = arg;
                let rule_count = self.load_u64_offset(argblock, 48).map_err(|_| 14u64)? as usize;
                let allowed_ptr = self.load_u64_offset(argblock, 56).map_err(|_| 14u64)?;
                let allowed_count = self.load_u64_offset(argblock, 64).map_err(|_| 14u64)? as usize;
                if rule_count > CLASSIFIER_MAX_RULES
                    || allowed_count > CLASSIFIER_MAX_ALLOWED_QUEUES
                {
                    return Err(22);
                }
                let allowed_queues =
                    self.read_classifier_allowed_queues(allowed_ptr, allowed_count)?;
                let rules = self.read_classifier_rules(rules_ptr, rule_count, &allowed_queues)?;
                self.prevalidate_object_create_outputs(argblock, 1)?;
                let fd = self.install_object_fd(
                    fd0_req,
                    FdHandle::ClassifierTable(Rc::new(RefCell::new(ClassifierTable {
                        rules,
                        allowed_queues,
                        counters: ClassifierCounters::default(),
                    }))),
                )?;
                self.store_u64_offset(argblock, 24, fd as u64)
                    .map_err(|_| 14u64)?;
                Ok(fd as u64)
            }
            (ObjectKind::Servicelet, ObjectProfile::ServiceletProgram) => {
                let program = self.verify_servicelet_program(arg)?;
                self.prevalidate_object_create_outputs(argblock, 1)?;
                let fd =
                    self.install_object_fd(fd0_req, FdHandle::ServiceletProgram(Rc::new(program)))?;
                self.store_u64_offset(argblock, 24, fd as u64)
                    .map_err(|_| 14u64)?;
                Ok(fd as u64)
            }
            (ObjectKind::DmaBuffer, _) => {
                let len = self.load_u64_offset(argblock, 48).map_err(|_| 14u64)?;
                if len == 0 {
                    return Err(22);
                }
                self.ensure_mapped(arg, len as usize, false)
                    .map_err(|_| 14u64)?;
                self.ensure_mapped(arg, len as usize, true)
                    .map_err(|_| 14u64)?;
                self.prevalidate_object_create_outputs(argblock, 1)?;
                let fd = self.install_object_fd(fd0_req, FdHandle::DmaBuffer { addr: arg, len })?;
                self.store_u64_offset(argblock, 24, fd as u64)
                    .map_err(|_| 14u64)?;
                Ok(fd as u64)
            }
            (ObjectKind::Endpoint, ObjectProfile::TcpStream) => {
                let sock_type = self.load_u64_offset(argblock, 48).map_err(|_| 14u64)?;
                let protocol = self.load_u64_offset(argblock, 56).map_err(|_| 14u64)?;
                if arg != SOCKET_AF_INET
                    || sock_type != SOCKET_TYPE_STREAM
                    || !(protocol == 0 || protocol == SOCKET_LEVEL_IPPROTO_TCP)
                {
                    return Err(22);
                }
                self.prevalidate_object_create_outputs(argblock, 1)?;
                let fd = self.install_object_fd(
                    fd0_req,
                    FdHandle::TcpSocket {
                        domain: arg,
                        sock_type,
                        protocol,
                        bound_addr: None,
                    },
                )?;
                self.store_u64_offset(argblock, 24, fd as u64)
                    .map_err(|_| 14u64)?;
                Ok(fd as u64)
            }
            _ => Err(22),
        }
    }

    fn prevalidate_object_create_outputs(
        &mut self,
        argblock: u64,
        slots: usize,
    ) -> NativeResult<()> {
        let addr = argblock.checked_add(24).ok_or(14u64)?;
        self.ensure_mapped(addr, slots * 8, true).map_err(|_| 14u64)
    }

    fn plan_object_fd_slot(
        &self,
        requested: u64,
        excluded: Option<usize>,
    ) -> Result<(usize, u64), u64> {
        if requested != 0 {
            let fd = requested as usize;
            if Some(fd) == excluded {
                return Err(22);
            }
            let delta = self.fd_slot_delta(fd).map_err(|_| 9u64)?;
            return Ok((fd, delta));
        }
        let process = self.process().map_err(|_| 3u64)?;
        process
            .fds
            .iter()
            .enumerate()
            .find(|(idx, candidate)| {
                *idx != MESSAGE_ENDPOINT_FD
                    && Some(*idx) != excluded
                    && matches!(candidate, FdHandle::Closed)
            })
            .map(|(idx, _)| (idx, 1))
            .ok_or(24)
    }

    fn read_classifier_rules(
        &mut self,
        rules_ptr: u64,
        rule_count: usize,
        allowed_queues: &[ClassifierAllowedQueue],
    ) -> Result<Vec<ClassifierRule>, u64> {
        if rule_count == 0 {
            return Ok(Vec::new());
        }
        if rules_ptr == 0 {
            return Err(14);
        }
        let mut rules = Vec::with_capacity(rule_count);
        for idx in 0..rule_count as u64 {
            let base = Self::checked_record_base(rules_ptr, idx, CLASSIFIER_RULE_SIZE)
                .map_err(|_| 14u64)?;
            let rule = ClassifierRule {
                kind: self.load_u64_offset(base, 0).map_err(|_| 14u64)?,
                field: self.load_u64_offset(base, 8).map_err(|_| 14u64)?,
                value: self.load_u64_offset(base, 16).map_err(|_| 14u64)?,
                mask_or_end: self.load_u64_offset(base, 24).map_err(|_| 14u64)?,
                action: self.load_u64_offset(base, 32).map_err(|_| 14u64)?,
                action_arg: self.load_u64_offset(base, 40).map_err(|_| 14u64)?,
                hash_mod: self.load_u64_offset(base, 48).map_err(|_| 14u64)?,
            };
            if !matches!(
                rule.kind,
                CLASSIFY_RULE_EXACT
                    | CLASSIFY_RULE_MASKED
                    | CLASSIFY_RULE_RANGE
                    | CLASSIFY_RULE_HASH
            ) || !matches!(
                rule.field,
                CLASSIFY_FIELD_SERVICE_ID
                    | CLASSIFY_FIELD_DST_PORT
                    | CLASSIFY_FIELD_SRC_IPV4
                    | CLASSIFY_FIELD_DST_IPV4
                    | CLASSIFY_FIELD_HASH
                    | CLASSIFY_FIELD_PROFILE
                    | CLASSIFY_FIELD_DOMAIN_ID
                    | CLASSIFY_FIELD_INLINE0
                    | CLASSIFY_FIELD_INLINE1
                    | CLASSIFY_FIELD_INLINE2
            ) || !matches!(
                rule.action,
                CLASSIFY_ACTION_MARK
                    | CLASSIFY_ACTION_COUNT
                    | CLASSIFY_ACTION_DROP
                    | CLASSIFY_ACTION_ROUTE
                    | CLASSIFY_ACTION_NEEDS_SOFTWARE
            ) {
                return Err(22);
            }
            if rule.action == CLASSIFY_ACTION_ROUTE
                && !allowed_queues
                    .iter()
                    .any(|queue| queue.token == rule.action_arg)
            {
                return Err(1);
            }
            if rule.kind == CLASSIFY_RULE_RANGE && rule.value > rule.mask_or_end {
                return Err(22);
            }
            if rule.kind == CLASSIFY_RULE_HASH && rule.hash_mod == 0 {
                return Err(22);
            }
            rules.push(rule);
        }
        Ok(rules)
    }

    fn read_classifier_allowed_queues(
        &mut self,
        allowed_ptr: u64,
        allowed_count: usize,
    ) -> Result<Vec<ClassifierAllowedQueue>, u64> {
        if allowed_count == 0 {
            return Ok(Vec::new());
        }
        if allowed_ptr == 0 {
            return Err(14);
        }
        let mut allowed = Vec::with_capacity(allowed_count);
        for idx in 0..allowed_count as u64 {
            let token = self
                .load_u64(Self::checked_record_base(allowed_ptr, idx, 8).map_err(|_| 14u64)?)
                .map_err(|_| 14u64)?;
            if token & FDR_TOKEN_MARKER == 0 {
                return Err(9);
            }
            let fd = self.decode_fd_value(token)?;
            self.fd_right_errno(fd, CAP_RIGHT_WRITE)?;
            if !matches!(
                self.process().map_err(|_| 3u64)?.fds[fd],
                FdHandle::PipeWriter(_)
            ) {
                return Err(9);
            }
            let generation = self
                .process()
                .map_err(|_| 3u64)?
                .fd_generations
                .get(fd)
                .copied()
                .ok_or(9u64)?;
            allowed.push(ClassifierAllowedQueue {
                token,
                fd,
                generation,
            });
        }
        Ok(allowed)
    }

    fn verify_servicelet_program(&mut self, envelope: u64) -> NativeResult<ServiceletProgram> {
        if envelope == 0 {
            return Err(14);
        }
        let version = self.load_u64_offset(envelope, 0).map_err(|_| 14u64)?;
        if version != SERVICELET_VERIFY_VERSION {
            return Err(22);
        }
        let program_len = self.load_u64_offset(envelope, 8).map_err(|_| 14u64)?;
        let isa_subset = self.load_u64_offset(envelope, 16).map_err(|_| 14u64)?;
        let instruction_limit = self.load_u64_offset(envelope, 24).map_err(|_| 14u64)?;
        let cycle_limit = self.load_u64_offset(envelope, 32).map_err(|_| 14u64)?;
        let record_read_limit = self.load_u64_offset(envelope, 40).map_err(|_| 14u64)?;
        let action_write_limit = self.load_u64_offset(envelope, 48).map_err(|_| 14u64)?;
        let flags = self.load_u64_offset(envelope, 56).map_err(|_| 14u64)?;
        let owner_domain_id = self.load_u64_offset(envelope, 64).map_err(|_| 14u64)?;
        let owner_generation = self.load_u64_offset(envelope, 72).map_err(|_| 14u64)?;

        if program_len == 0
            || program_len > SERVICELET_MAX_PROGRAM_BYTES
            || instruction_limit == 0
            || instruction_limit > SERVICELET_MAX_INSTRUCTIONS
            || cycle_limit == 0
            || cycle_limit > SERVICELET_MAX_CYCLES
            || record_read_limit > SERVICELET_MAX_RECORD_BYTES
            || action_write_limit > SERVICELET_MAX_ACTION_BYTES
            || isa_subset == 0
            || isa_subset & !SERVICELET_ALLOWED_ISA_MASK != 0
            || flags & !SERVICELET_FLAG_ALLOW_STATIC_LOOPS != 0
        {
            return Err(22);
        }
        if owner_generation == 0 {
            return Err(116);
        }
        let owner = self.domain_ref(owner_domain_id, owner_generation)?;
        if !self.domain_is_descendant_or_self(owner, self.current_domain_id().map_err(|_| 3u64)?) {
            return Err(1);
        }

        Ok(ServiceletProgram {
            program_len,
            isa_subset,
            instruction_limit,
            cycle_limit,
            record_read_limit,
            action_write_limit,
            flags,
            owner_domain_id: owner,
            owner_generation,
        })
    }

    fn object_ctl_classify(&mut self, argblock: u64) -> NativeResult<u64> {
        let classifier_value = self.load_u64_offset(argblock, 8).map_err(|_| 14u64)?;
        let envelope_ptr = self.load_u64_offset(argblock, 16).map_err(|_| 14u64)?;
        let result_ptr = self.load_u64_offset(argblock, 24).map_err(|_| 14u64)?;
        if envelope_ptr == 0 {
            return Err(14);
        }
        self.ensure_mapped(envelope_ptr, CLASSIFY_ENVELOPE_SIZE as usize, false)
            .map_err(|_| 14u64)?;
        if result_ptr != 0 {
            self.ensure_mapped(result_ptr, CLASSIFY_RESULT_SIZE as usize, true)
                .map_err(|_| 14u64)?;
        }
        let classifier_fd = self.decode_fd_value(classifier_value)?;
        self.fd_right_errno(classifier_fd, CAP_RIGHT_CALL)?;
        let table = match self.process().map_err(|_| 3u64)?.fds.get(classifier_fd) {
            Some(FdHandle::ClassifierTable(table)) => Rc::clone(table),
            _ => return Err(9),
        };
        let envelope = ClassifierEnvelope {
            profile: self.load_u64_offset(envelope_ptr, 0).map_err(|_| 14u64)?,
            source: self.load_u64_offset(envelope_ptr, 8).map_err(|_| 14u64)?,
            source_generation: self.load_u64_offset(envelope_ptr, 16).map_err(|_| 14u64)?,
            domain_id: self.load_u64_offset(envelope_ptr, 24).map_err(|_| 14u64)?,
            record_ptr: self.load_u64_offset(envelope_ptr, 32).map_err(|_| 14u64)?,
            record_len: self.load_u64_offset(envelope_ptr, 40).map_err(|_| 14u64)? as usize,
            inline0: self.load_u64_offset(envelope_ptr, 48).map_err(|_| 14u64)?,
            inline1: self.load_u64_offset(envelope_ptr, 56).map_err(|_| 14u64)?,
            inline2: self.load_u64_offset(envelope_ptr, 64).map_err(|_| 14u64)?,
        };
        if !matches!(
            envelope.profile,
            CLASSIFY_PROFILE_PACKET
                | CLASSIFY_PROFILE_IPC
                | CLASSIFY_PROFILE_EVENT
                | CLASSIFY_PROFILE_DMA_COMPLETION
                | CLASSIFY_PROFILE_STORAGE_COMPLETION
                | CLASSIFY_PROFILE_TRACE
                | CLASSIFY_PROFILE_RUNTIME_TASK
        ) {
            return Err(22);
        }
        if envelope.domain_id != 0
            && envelope.domain_id != self.current_domain_id().map_err(|_| 3u64)?
        {
            return Err(1);
        }
        self.validate_classifier_source(&envelope)?;
        let packet = if envelope.profile == CLASSIFY_PROFILE_PACKET {
            match self.parse_classifier_packet(&envelope) {
                Ok(parsed) => parsed,
                Err(ClassifyParseError::Malformed) => {
                    let mut table = table.borrow_mut();
                    table.counters.malformed = table.counters.malformed.saturating_add(1);
                    table.counters.fallback = table.counters.fallback.saturating_add(1);
                    self.write_classifier_result(
                        result_ptr,
                        CLASSIFY_ACTION_NEEDS_SOFTWARE,
                        0,
                        0,
                        u64::MAX,
                    )?;
                    return Ok(CLASSIFY_ACTION_NEEDS_SOFTWARE);
                }
                Err(ClassifyParseError::NeedsSoftware) => ClassifierParsedFields {
                    needs_software: true,
                    ..ClassifierParsedFields::default()
                },
            }
        } else {
            ClassifierParsedFields::default()
        };
        if packet.needs_software {
            let mut table = table.borrow_mut();
            table.counters.fallback = table.counters.fallback.saturating_add(1);
            self.write_classifier_result(
                result_ptr,
                CLASSIFY_ACTION_NEEDS_SOFTWARE,
                0,
                0,
                u64::MAX,
            )?;
            return Ok(CLASSIFY_ACTION_NEEDS_SOFTWARE);
        }
        let selected = {
            let table_ref = table.borrow();
            table_ref.rules.iter().enumerate().find_map(|(idx, rule)| {
                self.classifier_rule_matches(rule, &envelope, &packet)
                    .then(|| (idx as u64, rule.clone()))
            })
        };
        let Some((rule_idx, rule)) = selected else {
            let mut table = table.borrow_mut();
            table.counters.fallback = table.counters.fallback.saturating_add(1);
            self.write_classifier_result(
                result_ptr,
                CLASSIFY_ACTION_NEEDS_SOFTWARE,
                0,
                0,
                u64::MAX,
            )?;
            return Ok(CLASSIFY_ACTION_NEEDS_SOFTWARE);
        };
        let route_token = if rule.action == CLASSIFY_ACTION_ROUTE {
            self.classifier_route(&table, rule.action_arg, &envelope)?
        } else {
            0
        };
        {
            let mut table = table.borrow_mut();
            table.counters.hits = table.counters.hits.saturating_add(1);
            match rule.action {
                CLASSIFY_ACTION_DROP => {
                    table.counters.drops = table.counters.drops.saturating_add(1);
                }
                CLASSIFY_ACTION_ROUTE => {
                    table.counters.routes = table.counters.routes.saturating_add(1);
                }
                CLASSIFY_ACTION_NEEDS_SOFTWARE => {
                    table.counters.fallback = table.counters.fallback.saturating_add(1);
                }
                _ => {}
            }
        }
        self.write_classifier_result(
            result_ptr,
            rule.action,
            rule.action_arg,
            route_token,
            rule_idx,
        )?;
        Ok(rule.action)
    }

    fn object_ctl_classifier_query(&mut self, argblock: u64) -> NativeResult<u64> {
        let classifier_value = self.load_u64_offset(argblock, 8).map_err(|_| 14u64)?;
        let out_ptr = self.load_u64_offset(argblock, 16).map_err(|_| 14u64)?;
        if out_ptr == 0 {
            return Err(14);
        }
        self.ensure_mapped(out_ptr, CLASSIFIER_COUNTERS_SIZE as usize, true)
            .map_err(|_| 14u64)?;
        let classifier_fd = self.decode_fd_value(classifier_value)?;
        self.fd_right_errno(classifier_fd, CAP_RIGHT_STAT)?;
        let counters = match self.process().map_err(|_| 3u64)?.fds.get(classifier_fd) {
            Some(FdHandle::ClassifierTable(table)) => {
                let table = table.borrow();
                [
                    table.counters.hits,
                    table.counters.drops,
                    table.counters.routes,
                    table.counters.malformed,
                    table.counters.fallback,
                ]
            }
            _ => return Err(9),
        };
        for (idx, value) in counters.into_iter().enumerate() {
            self.store_u64_offset(out_ptr, idx as u64 * 8, value)
                .map_err(|_| 14u64)?;
        }
        Ok(CLASSIFIER_COUNTERS_SIZE)
    }

    fn validate_classifier_source(&self, envelope: &ClassifierEnvelope) -> Result<(), u64> {
        if envelope.source == 0 {
            return Err(9);
        }
        let source_fd = self.decode_fd_value(envelope.source)?;
        self.fd_right_errno(source_fd, CAP_RIGHT_READ)?;
        if envelope.source_generation == 0 {
            return Err(116);
        }
        let generation = self
            .process()
            .map_err(|_| 3u64)?
            .fd_generations
            .get(source_fd)
            .copied()
            .ok_or(9u64)?;
        if generation != envelope.source_generation {
            return Err(116);
        }
        Ok(())
    }

    fn classifier_rule_matches(
        &self,
        rule: &ClassifierRule,
        envelope: &ClassifierEnvelope,
        packet: &ClassifierParsedFields,
    ) -> bool {
        let Some(value) = self.classifier_field_value(rule.field, envelope, packet) else {
            return false;
        };
        match rule.kind {
            CLASSIFY_RULE_EXACT => value == rule.value,
            CLASSIFY_RULE_MASKED => (value & rule.mask_or_end) == (rule.value & rule.mask_or_end),
            CLASSIFY_RULE_RANGE => value >= rule.value && value <= rule.mask_or_end,
            CLASSIFY_RULE_HASH => {
                let modulus = rule.hash_mod.max(rule.mask_or_end).max(1);
                value % modulus == rule.value
            }
            _ => false,
        }
    }

    fn classifier_field_value(
        &self,
        field: u64,
        envelope: &ClassifierEnvelope,
        packet: &ClassifierParsedFields,
    ) -> Option<u64> {
        match field {
            CLASSIFY_FIELD_SERVICE_ID => Some(envelope.inline0),
            CLASSIFY_FIELD_DST_PORT => packet.dst_port,
            CLASSIFY_FIELD_SRC_IPV4 => packet.src_ipv4,
            CLASSIFY_FIELD_DST_IPV4 => packet.dst_ipv4,
            CLASSIFY_FIELD_HASH => {
                Some(packet.hash ^ envelope.inline0 ^ envelope.inline1 ^ envelope.inline2)
            }
            CLASSIFY_FIELD_PROFILE => Some(envelope.profile),
            CLASSIFY_FIELD_DOMAIN_ID => Some(envelope.domain_id),
            CLASSIFY_FIELD_INLINE0 => Some(envelope.inline0),
            CLASSIFY_FIELD_INLINE1 => Some(envelope.inline1),
            CLASSIFY_FIELD_INLINE2 => Some(envelope.inline2),
            _ => None,
        }
    }

    fn parse_classifier_packet(
        &mut self,
        envelope: &ClassifierEnvelope,
    ) -> Result<ClassifierParsedFields, ClassifyParseError> {
        if envelope.record_len > CLASSIFIER_MAX_ROUTE_BYTES {
            return Err(ClassifyParseError::NeedsSoftware);
        }
        if envelope.record_ptr == 0 || envelope.record_len < 14 {
            return Err(ClassifyParseError::Malformed);
        }
        let bytes = self
            .read_bytes(envelope.record_ptr, envelope.record_len)
            .map_err(|_| ClassifyParseError::Malformed)?;
        let mut offset = 14usize;
        let mut ethertype = u16::from_be_bytes([bytes[12], bytes[13]]);
        if matches!(ethertype, 0x8100 | 0x88a8) {
            if bytes.len() < 18 {
                return Err(ClassifyParseError::Malformed);
            }
            ethertype = u16::from_be_bytes([bytes[16], bytes[17]]);
            offset = 18;
        }
        match ethertype {
            0x0800 => self.parse_classifier_ipv4(&bytes, offset),
            0x86dd => self.parse_classifier_ipv6(&bytes, offset),
            _ => Err(ClassifyParseError::NeedsSoftware),
        }
    }

    fn parse_classifier_ipv4(
        &self,
        bytes: &[u8],
        offset: usize,
    ) -> Result<ClassifierParsedFields, ClassifyParseError> {
        if bytes.len() < offset + 20 {
            return Err(ClassifyParseError::Malformed);
        }
        let version = bytes[offset] >> 4;
        let ihl = (bytes[offset] & 0x0f) as usize * 4;
        if version != 4 || ihl < 20 || bytes.len() < offset + ihl {
            return Err(ClassifyParseError::Malformed);
        }
        let total_len = u16::from_be_bytes([bytes[offset + 2], bytes[offset + 3]]) as usize;
        if total_len == 0 {
            let mut parsed = ClassifierParsedFields::default();
            parsed.needs_software = true;
            return Ok(parsed);
        }
        if total_len < ihl {
            return Err(ClassifyParseError::Malformed);
        }
        if bytes.len() < offset + total_len {
            return Err(ClassifyParseError::Malformed);
        }
        let protocol = bytes[offset + 9];
        let src_ipv4 = u32::from_be_bytes([
            bytes[offset + 12],
            bytes[offset + 13],
            bytes[offset + 14],
            bytes[offset + 15],
        ]) as u64;
        let dst_ipv4 = u32::from_be_bytes([
            bytes[offset + 16],
            bytes[offset + 17],
            bytes[offset + 18],
            bytes[offset + 19],
        ]) as u64;
        let mut parsed = ClassifierParsedFields {
            src_ipv4: Some(src_ipv4),
            dst_ipv4: Some(dst_ipv4),
            hash: src_ipv4 ^ dst_ipv4 ^ protocol as u64,
            ..ClassifierParsedFields::default()
        };
        let fragment = u16::from_be_bytes([bytes[offset + 6], bytes[offset + 7]]);
        if fragment & 0xbfff != 0 {
            parsed.needs_software = true;
            return Ok(parsed);
        }
        if matches!(protocol, 6 | 17) {
            let port_offset = offset + ihl;
            let packet_len = total_len;
            if packet_len < ihl + 4 {
                return Err(ClassifyParseError::Malformed);
            }
            if bytes.len() < port_offset + 4 {
                return Err(ClassifyParseError::Malformed);
            }
            let src_port = u16::from_be_bytes([bytes[port_offset], bytes[port_offset + 1]]) as u64;
            let dst_port =
                u16::from_be_bytes([bytes[port_offset + 2], bytes[port_offset + 3]]) as u64;
            parsed.src_port = Some(src_port);
            parsed.dst_port = Some(dst_port);
            parsed.hash ^= src_port ^ dst_port;
        } else {
            parsed.needs_software = true;
        }
        Ok(parsed)
    }

    fn parse_classifier_ipv6(
        &self,
        bytes: &[u8],
        offset: usize,
    ) -> Result<ClassifierParsedFields, ClassifyParseError> {
        if bytes.len() < offset + 40 {
            return Err(ClassifyParseError::Malformed);
        }
        if bytes[offset] >> 4 != 6 {
            return Err(ClassifyParseError::Malformed);
        }
        let payload_len = u16::from_be_bytes([bytes[offset + 4], bytes[offset + 5]]) as usize;
        let next_header = bytes[offset + 6];
        let mut hash = next_header as u64;
        for byte in &bytes[offset + 8..offset + 40] {
            hash = hash.rotate_left(5) ^ *byte as u64;
        }
        let mut parsed = ClassifierParsedFields {
            hash,
            ..ClassifierParsedFields::default()
        };
        if payload_len == 0 {
            parsed.needs_software = true;
            return Ok(parsed);
        }
        if bytes.len() < offset + 40 + payload_len {
            return Err(ClassifyParseError::Malformed);
        }
        if matches!(next_header, 6 | 17) {
            let port_offset = offset + 40;
            if payload_len < 4 {
                return Err(ClassifyParseError::Malformed);
            }
            if bytes.len() < port_offset + 4 {
                return Err(ClassifyParseError::Malformed);
            }
            let src_port = u16::from_be_bytes([bytes[port_offset], bytes[port_offset + 1]]) as u64;
            let dst_port =
                u16::from_be_bytes([bytes[port_offset + 2], bytes[port_offset + 3]]) as u64;
            parsed.src_port = Some(src_port);
            parsed.dst_port = Some(dst_port);
            parsed.hash ^= src_port ^ dst_port;
        } else {
            parsed.needs_software = true;
        }
        Ok(parsed)
    }

    fn classifier_route(
        &mut self,
        table: &Rc<RefCell<ClassifierTable>>,
        queue_token: u64,
        envelope: &ClassifierEnvelope,
    ) -> NativeResult<u64> {
        let (queue_fd, generation) = {
            let table = table.borrow();
            let Some(queue) = table
                .allowed_queues
                .iter()
                .find(|queue| queue.token == queue_token)
            else {
                return Err(1);
            };
            (queue.fd, queue.generation)
        };
        if !self
            .fd_generation_matches(queue_fd, generation)
            .map_err(|_| 9u64)?
        {
            return Err(116);
        }
        if self.decode_fd_value(queue_token)? != queue_fd {
            return Err(116);
        }
        self.fd_right_errno(queue_fd, CAP_RIGHT_WRITE)?;
        let queue = match self.process().map_err(|_| 3u64)?.fds.get(queue_fd) {
            Some(FdHandle::PipeWriter(queue)) => Rc::clone(queue),
            _ => return Err(9),
        };
        if envelope.record_len > CLASSIFIER_MAX_ROUTE_BYTES {
            return Err(75);
        }
        let payload = if envelope.record_ptr != 0 && envelope.record_len != 0 {
            self.read_bytes(envelope.record_ptr, envelope.record_len)
                .map_err(|_| 14u64)?
        } else {
            envelope.inline0.to_le_bytes().to_vec()
        };
        {
            queue.borrow_mut().push_bytes(&payload)?;
        }
        self.poll_fd_waiters();
        Ok(queue_token)
    }

    fn write_classifier_result(
        &mut self,
        result_ptr: u64,
        action: u64,
        action_arg: u64,
        route_token: u64,
        rule_idx: u64,
    ) -> Result<(), u64> {
        if result_ptr == 0 {
            return Ok(());
        }
        self.store_u64_offset(result_ptr, 0, action)
            .map_err(|_| 14u64)?;
        self.store_u64_offset(result_ptr, 8, action_arg)
            .map_err(|_| 14u64)?;
        self.store_u64_offset(result_ptr, 16, route_token)
            .map_err(|_| 14u64)?;
        self.store_u64_offset(result_ptr, 24, rule_idx)
            .map_err(|_| 14u64)?;
        Ok(())
    }

    fn object_ctl_socket_bind(&mut self, argblock: u64) -> Result<u64, u64> {
        let fd_value = self.load_u64_offset(argblock, 24).map_err(|_| 14u64)?;
        let addr_ptr = self.load_u64_offset(argblock, 40).map_err(|_| 14u64)?;
        let fd = self.decode_fd_value(fd_value)?;
        self.fd_right_errno(fd, CAP_RIGHT_WRITE)?;
        let addr = self.read_c_string(addr_ptr).map_err(|_| 14u64)?;
        let addr = addr.parse::<SocketAddr>().map_err(|_| 22u64)?.to_string();
        match &mut self.process_mut().map_err(|_| 3u64)?.fds[fd] {
            FdHandle::TcpSocket { bound_addr, .. } => {
                *bound_addr = Some(addr);
                Ok(0)
            }
            _ => Err(22),
        }
    }

    fn object_ctl_socket_listen(&mut self, argblock: u64) -> Result<u64, u64> {
        let fd_value = self.load_u64_offset(argblock, 24).map_err(|_| 14u64)?;
        let fd = self.decode_fd_value(fd_value)?;
        self.fd_right_errno(fd, CAP_RIGHT_WRITE | CAP_RIGHT_POLL)?;
        let addr = match &self.process().map_err(|_| 3u64)?.fds[fd] {
            FdHandle::TcpSocket {
                bound_addr: Some(addr),
                ..
            } => addr.clone(),
            _ => return Err(22),
        };
        let listener = TcpListener::bind(&addr).map_err(|_| 98u64)?;
        listener.set_nonblocking(true).map_err(|_| 5u64)?;
        self.process_mut().map_err(|_| 3u64)?.fds[fd] = FdHandle::TcpListener {
            listener,
            pending: None,
        };
        self.bump_fd_generation(fd).map_err(|_| 9u64)?;
        Ok(0)
    }

    fn object_ctl_socket_connect(&mut self, argblock: u64) -> Result<u64, u64> {
        let fd_value = self.load_u64_offset(argblock, 24).map_err(|_| 14u64)?;
        let addr_ptr = self.load_u64_offset(argblock, 40).map_err(|_| 14u64)?;
        let fd = self.decode_fd_value(fd_value)?;
        self.fd_right_errno(fd, CAP_RIGHT_READ | CAP_RIGHT_WRITE | CAP_RIGHT_POLL)?;
        if !matches!(
            self.process().map_err(|_| 3u64)?.fds.get(fd),
            Some(FdHandle::TcpSocket { .. })
        ) {
            return Err(22);
        }
        let addr = self.read_c_string(addr_ptr).map_err(|_| 14u64)?;
        let addr = addr.parse::<SocketAddr>().map_err(|_| 22u64)?;
        let stream = TcpStream::connect(addr).map_err(|_| 111u64)?;
        stream.set_nonblocking(true).map_err(|_| 5u64)?;
        self.process_mut().map_err(|_| 3u64)?.fds[fd] = FdHandle::TcpStream(stream);
        self.bump_fd_generation(fd).map_err(|_| 9u64)?;
        Ok(0)
    }

    fn object_ctl_socket_accept(&mut self, argblock: u64) -> Result<u64, u64> {
        let listener_value = self.load_u64_offset(argblock, 24).map_err(|_| 14u64)?;
        let accepted_req = self.load_u64_offset(argblock, 32).map_err(|_| 14u64)?;
        self.validate_object_fd_request(accepted_req)?;
        let listener_fd = self.decode_fd_value(listener_value)?;
        self.fd_right_errno(listener_fd, CAP_RIGHT_READ | CAP_RIGHT_POLL)?;
        let (accepted_fd, accepted_delta) = self.plan_object_fd_slot(accepted_req, None)?;
        self.ensure_domain_budget_errno(0, 0, 0, accepted_delta)?;
        self.ensure_mapped(argblock.checked_add(32).ok_or(14u64)?, 8, true)
            .map_err(|_| 14u64)?;
        let stream = {
            let process = self.process_mut().map_err(|_| 3u64)?;
            match &mut process.fds[listener_fd] {
                FdHandle::TcpListener { listener, pending } => {
                    if let Some(stream) = pending.take() {
                        stream
                    } else {
                        match listener.accept() {
                            Ok((stream, _)) => stream,
                            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                                return Err(11);
                            }
                            Err(_) => return Err(5),
                        }
                    }
                }
                _ => return Err(22),
            }
        };
        stream.set_nonblocking(true).map_err(|_| 5u64)?;
        let accepted_fd =
            self.install_object_fd(accepted_fd as u64, FdHandle::TcpStream(stream))?;
        self.store_u64_offset(argblock, 32, accepted_fd as u64)
            .map_err(|_| 14u64)?;
        Ok(accepted_fd as u64)
    }

    fn object_ctl_socket_getsockname(&mut self, argblock: u64) -> Result<u64, u64> {
        let fd_value = self.load_u64_offset(argblock, 24).map_err(|_| 14u64)?;
        let addr_ptr = self.load_u64_offset(argblock, 40).map_err(|_| 14u64)?;
        let len_ptr = self.load_u64_offset(argblock, 48).map_err(|_| 14u64)?;
        let fd = self.decode_fd_value(fd_value)?;
        self.fd_right_errno(fd, CAP_RIGHT_STAT)?;
        let addr = match &self.process().map_err(|_| 3u64)?.fds[fd] {
            FdHandle::TcpListener { listener, .. } => listener.local_addr().map_err(|_| 5u64)?,
            FdHandle::TcpStream(stream) => stream.local_addr().map_err(|_| 5u64)?,
            _ => return Err(22),
        };
        let mut bytes = addr.to_string().into_bytes();
        bytes.push(0);
        self.ensure_mapped(addr_ptr, bytes.len(), true)
            .map_err(|_| 14u64)?;
        if len_ptr != 0 {
            let capacity = self.load_u64(len_ptr).map_err(|_| 14u64)?;
            if capacity < bytes.len() as u64 {
                return Err(22);
            }
            self.store_u64_offset(len_ptr, 0, bytes.len() as u64)
                .map_err(|_| 14u64)?;
        }
        self.write_bytes_offset(addr_ptr, 0, &bytes)
            .map_err(|_| 14u64)?;
        Ok(0)
    }

    fn object_ctl_socket_getsockopt(&mut self, argblock: u64) -> Result<u64, u64> {
        let fd_value = self.load_u64_offset(argblock, 24).map_err(|_| 14u64)?;
        let level = self.load_u64_offset(argblock, 40).map_err(|_| 14u64)?;
        let optname = self.load_u64_offset(argblock, 48).map_err(|_| 14u64)?;
        let optval = self.load_u64_offset(argblock, 56).map_err(|_| 14u64)?;
        let optlen = self.load_u64_offset(argblock, 64).map_err(|_| 14u64)?;
        let fd = self.decode_fd_value(fd_value)?;
        self.fd_right_errno(fd, CAP_RIGHT_STAT)?;
        self.ensure_socket_fd(fd)?;
        if !self.socket_getsockopt_supported(level, optname) {
            return Err(22);
        }
        if optval != 0 {
            self.ensure_mapped(optval, 8, true).map_err(|_| 14u64)?;
        }
        if optlen != 0 {
            let capacity = self.load_u64(optlen).map_err(|_| 14u64)?;
            if capacity < 8 {
                return Err(22);
            }
            self.store_u64_offset(optlen, 0, 8).map_err(|_| 14u64)?;
        }
        if optval != 0 {
            self.store_u64_offset(optval, 0, 0).map_err(|_| 14u64)?;
        }
        Ok(0)
    }

    fn object_ctl_socket_setsockopt(&mut self, argblock: u64) -> Result<u64, u64> {
        let fd_value = self.load_u64_offset(argblock, 24).map_err(|_| 14u64)?;
        let level = self.load_u64_offset(argblock, 40).map_err(|_| 14u64)?;
        let optname = self.load_u64_offset(argblock, 48).map_err(|_| 14u64)?;
        let optval = self.load_u64_offset(argblock, 56).map_err(|_| 14u64)?;
        let optlen = self.load_u64_offset(argblock, 64).map_err(|_| 14u64)?;
        let fd = self.decode_fd_value(fd_value)?;
        self.fd_right_errno(fd, CAP_RIGHT_WRITE)?;
        self.ensure_socket_fd(fd)?;
        if !self.socket_setsockopt_supported(level, optname) {
            return Err(22);
        }
        if optval != 0 && optlen != 0 {
            self.ensure_mapped(optval, optlen as usize, false)
                .map_err(|_| 14u64)?;
        }
        Ok(0)
    }

    fn socket_getsockopt_supported(&self, level: u64, optname: u64) -> bool {
        matches!(
            (level, optname),
            (SOCKET_LEVEL_SOL_SOCKET, SOCKET_OPT_SO_ERROR)
        )
    }

    fn socket_setsockopt_supported(&self, level: u64, optname: u64) -> bool {
        matches!(
            (level, optname),
            (
                SOCKET_LEVEL_SOL_SOCKET,
                SOCKET_OPT_SO_REUSEADDR
                    | SOCKET_OPT_SO_BROADCAST
                    | SOCKET_OPT_SO_SNDBUF
                    | SOCKET_OPT_SO_RCVBUF
                    | SOCKET_OPT_SO_KEEPALIVE
            ) | (SOCKET_LEVEL_IPPROTO_TCP, SOCKET_OPT_TCP_NODELAY)
        )
    }

    fn ensure_socket_fd(&self, fd: usize) -> Result<(), u64> {
        match self.process().map_err(|_| 3u64)?.fds.get(fd) {
            Some(FdHandle::TcpSocket { .. })
            | Some(FdHandle::TcpListener { .. })
            | Some(FdHandle::TcpStream(_)) => Ok(()),
            _ => Err(22),
        }
    }

    fn validate_object_fd_request(&self, requested: u64) -> Result<(), u64> {
        if requested != 0
            && (requested as usize >= FDR_COUNT || requested as usize == MESSAGE_ENDPOINT_FD)
        {
            return Err(9);
        }
        Ok(())
    }

    fn install_object_fd(&mut self, requested: u64, handle: FdHandle) -> Result<usize, u64> {
        if requested != 0 {
            if requested as usize >= FDR_COUNT || requested as usize == MESSAGE_ENDPOINT_FD {
                return Err(9);
            }
            let fd = requested as usize;
            let delta = self.fd_slot_delta(fd).map_err(|_| 9u64)?;
            self.ensure_domain_budget_errno(0, 0, 0, delta)?;
            if !matches!(
                self.process().map_err(|_| 3u64)?.fds.get(fd),
                Some(FdHandle::Closed)
            ) {
                self.release_process_file_locks_for_fd(fd)
                    .map_err(|_| 9u64)?;
            }
            self.bump_fd_generation(fd).map_err(|_| 9u64)?;
            self.process_mut().map_err(|_| 3u64)?.fds[fd] = handle;
            let capability = self.fresh_fd_capability();
            self.install_fd_capability(fd, capability)
                .map_err(|_| 9u64)?;
            return Ok(fd);
        }
        self.ensure_domain_budget_errno(0, 0, 0, 1)?;
        let fd = {
            let process = self.process_mut().map_err(|_| 3u64)?;
            process
                .fds
                .iter()
                .enumerate()
                .find(|(idx, candidate)| {
                    *idx != MESSAGE_ENDPOINT_FD && matches!(candidate, FdHandle::Closed)
                })
                .map(|(idx, _)| idx)
        };
        let Some(fd) = fd else {
            return Err(24);
        };
        self.bump_fd_generation(fd).map_err(|_| 9u64)?;
        self.process_mut().map_err(|_| 3u64)?.fds[fd] = handle;
        let capability = self.fresh_fd_capability();
        self.install_fd_capability(fd, capability)
            .map_err(|_| 9u64)?;
        Ok(fd)
    }

    fn call_cap(
        &mut self,
        result: Reg,
        call_gate_fd: usize,
        arg0: u64,
        arg1: u64,
    ) -> Result<(), String> {
        Self::ensure_result_reg_writable(result)?;
        self.require_domain_cap(DOMAIN_CAP_CALL)?;
        if let Err(errno) = self.fd_right_errno(call_gate_fd, CAP_RIGHT_CALL) {
            return self.complete_reg_err(result, errno);
        }
        let (
            entry,
            domain_id,
            domain_generation,
            mode,
            completion_fd,
            completion_generation,
            flags,
        ) = match &self.process()?.fds[call_gate_fd] {
            FdHandle::CallGate {
                entry,
                domain_id,
                domain_generation,
                mode,
                completion_fd,
                completion_generation,
                flags,
            } => (
                *entry,
                *domain_id,
                *domain_generation,
                *mode,
                *completion_fd,
                *completion_generation,
                *flags,
            ),
            _ => {
                return self.complete_reg_err(result, 9);
            }
        };
        if self.domain_ref(domain_id, domain_generation).is_err() {
            return self.complete_reg_err(result, 116);
        }
        if (arg0 & CALL_ARG_CAP_MARKER != 0 || arg1 & CALL_ARG_CAP_MARKER != 0)
            && flags & CALL_GATE_FLAG_CAP_PASS == 0
        {
            return self.complete_reg_err(result, 1);
        }
        if self.domain_is_frozen_or_destroyed(domain_id) {
            return self.complete_reg_err(result, 11);
        }
        if !self.check_call_cpu_budget(domain_id)? {
            let errno = self.process()?.errno;
            return self.complete_reg_err(result, if errno == 0 { 11 } else { errno });
        }
        match mode {
            CALL_MODE_SYNC => self.call_cap_sync(result, entry, domain_id, arg0, arg1),
            CALL_MODE_ASYNC => {
                self.call_cap_async(result, completion_fd.zip(completion_generation), arg0, arg1)
            }
            CALL_MODE_HANDOFF => self.call_cap_handoff(result, entry, domain_id, arg0, arg1),
            _ => self.complete_reg_err(result, 22),
        }
    }

    fn call_cap_sync(
        &mut self,
        result: Reg,
        entry: usize,
        domain_id: u64,
        arg0: u64,
        arg1: u64,
    ) -> Result<(), String> {
        if self.thread()?.cap_call_stack.len() >= MAX_CAP_CALL_DEPTH {
            return self.complete_reg_err(result, 11);
        }
        let caller_domain_id = self.current_domain_id()?;
        let return_ip = self.thread()?.ip;
        self.thread_mut()?.cap_call_stack.push(CallContinuation {
            return_ip,
            result_reg: result,
            caller_domain_id,
        });
        self.process_mut()?.domain_id = domain_id;
        self.write_reg(Reg(1), arg0)?;
        self.write_reg(Reg(2), arg1)?;
        self.thread_mut()?.ip = entry;
        Ok(())
    }

    fn call_cap_async(
        &mut self,
        result: Reg,
        completion: Option<(usize, u64)>,
        arg0: u64,
        arg1: u64,
    ) -> Result<(), String> {
        if let Some((fd, generation)) = completion {
            if !self.fd_generation_matches(fd, generation)? {
                return self.complete_reg_err(result, 116);
            }
            if let Err(errno) = self.fd_right_errno(fd, CAP_RIGHT_WRITE) {
                return self.complete_reg_err(result, errno);
            }
        }
        let op_id = self.next_call_op_id;
        self.next_call_op_id = self.next_call_op_id.saturating_add(1);
        if let Some((fd, _)) = completion {
            if self.complete_call_fd(fd, op_id, arg0, arg1).is_err() {
                let errno = self.process()?.errno;
                return self.complete_reg_err(result, if errno == 0 { 5 } else { errno });
            }
            self.poll_fd_waiters();
        }
        self.complete_reg_ok(result, op_id)
    }

    fn call_cap_handoff(
        &mut self,
        result: Reg,
        entry: usize,
        domain_id: u64,
        arg0: u64,
        arg1: u64,
    ) -> Result<(), String> {
        self.process_mut()?.domain_id = domain_id;
        self.complete_reg_ok(result, 0)?;
        self.write_reg(Reg(1), arg0)?;
        self.write_reg(Reg(2), arg1)?;
        self.thread_mut()?.ip = entry;
        Ok(())
    }

    fn complete_call_fd(
        &mut self,
        fd: usize,
        op_id: u64,
        value0: u64,
        value1: u64,
    ) -> Result<(), String> {
        enum CompletionTarget {
            Counter(Rc<RefCell<u64>>),
            EventCounter(Rc<RefCell<u64>>),
            Queue(Rc<RefCell<PipeBuffer>>),
        }

        let target = {
            let process = self.process()?;
            match process.fds.get(fd) {
                Some(FdHandle::Counter(value)) => CompletionTarget::Counter(Rc::clone(value)),
                Some(FdHandle::EventCounter { value, .. }) => {
                    CompletionTarget::EventCounter(Rc::clone(value))
                }
                Some(FdHandle::PipeWriter(queue)) => CompletionTarget::Queue(Rc::clone(queue)),
                _ => {
                    self.set_status_errno(9)?;
                    return Err("CALL_CAP async completion target is not waitable".to_string());
                }
            }
        };

        match target {
            CompletionTarget::Counter(value) => {
                *value.borrow_mut() = op_id;
                Ok(())
            }
            CompletionTarget::EventCounter(value) => {
                let mut value = value.borrow_mut();
                *value = value.saturating_add(op_id);
                Ok(())
            }
            CompletionTarget::Queue(queue) => {
                let mut payload = [0u8; 24];
                payload[0..8].copy_from_slice(&op_id.to_le_bytes());
                payload[8..16].copy_from_slice(&value0.to_le_bytes());
                payload[16..24].copy_from_slice(&value1.to_le_bytes());
                if let Err(errno) = queue.borrow_mut().push_bytes(&payload) {
                    self.set_status_errno(errno)?;
                    return Err("CALL_CAP async completion queue is full".to_string());
                }
                Ok(())
            }
        }
    }

    fn ret_cap(&mut self, result: Reg, value0: u64, value1: u64) -> Result<(), String> {
        Self::ensure_result_reg_writable(result)?;
        let Some(continuation) = self.thread()?.cap_call_stack.last().cloned() else {
            return self.complete_reg_err(result, 22);
        };
        Self::ensure_result_reg_writable(continuation.result_reg)?;
        if self.domain_is_frozen_or_destroyed(continuation.caller_domain_id) {
            return self.complete_reg_err(result, 116);
        }
        let Some(caller) = self.domains.get(&continuation.caller_domain_id) else {
            return self.complete_reg_err(result, 116);
        };
        if caller.capability_mask & DOMAIN_CAP_CALL == 0 {
            return self.complete_reg_err(result, 1);
        }
        self.thread_mut()?.cap_call_stack.pop();
        self.process_mut()?.domain_id = continuation.caller_domain_id;
        self.thread_mut()?.ip = continuation.return_ip;
        self.set_errno(0)?;
        self.write_reg(continuation.result_reg, value0)?;
        self.write_reg(Reg(30), value1)?;
        self.write_reg(result, 0)
    }

    fn domain_ctl(&mut self, result: Reg, argblock: u64) -> Result<(), String> {
        Self::ensure_result_reg_writable(result)?;
        let op = self.load_u64_offset(argblock, 0)?;
        let value = match op {
            DOMAIN_OP_CREATE => self.domain_ctl_create(argblock),
            DOMAIN_OP_CONFIGURE => self.domain_ctl_configure(argblock),
            DOMAIN_OP_QUERY => self.domain_ctl_query(argblock),
            DOMAIN_OP_FREEZE => self.domain_ctl_set_frozen(argblock, true),
            DOMAIN_OP_RESUME => self.domain_ctl_set_frozen(argblock, false),
            DOMAIN_OP_DESTROY => self.domain_ctl_destroy(argblock),
            DOMAIN_OP_ATTACH_SELF => self.domain_ctl_attach_self(argblock),
            DOMAIN_OP_DETACH_SELF => self.domain_ctl_detach_self(),
            _ => Err(22),
        };
        match value {
            Ok(value) => self.complete_reg_ok(result, value),
            Err(errno) => self.complete_reg_err(result, errno),
        }
    }

    fn ns_ctl(&mut self, result: Reg, argblock: u64) -> Result<(), String> {
        Self::ensure_result_reg_writable(result)?;
        let value = self.ns_ctl_record(argblock);
        match value {
            Ok(value) => self.complete_reg_ok(result, value),
            Err(errno) => self.complete_reg_err(result, errno),
        }
    }

    fn ns_ctl_record(&mut self, argblock: u64) -> Result<u64, u64> {
        let op = self.load_u64_offset(argblock, 0).map_err(|_| 14u64)?;
        let version = self.load_u64_offset(argblock, 8).map_err(|_| 14u64)?;
        if version != NS_CTL_VERSION {
            return Err(22);
        }
        match op {
            NS_OP_RESOLVE => self.ns_ctl_resolve(argblock),
            _ => Err(22),
        }
    }

    fn ns_ctl_resolve(&mut self, argblock: u64) -> Result<u64, u64> {
        let dir_value = self.load_u64_offset(argblock, 16).map_err(|_| 14u64)?;
        let path_ptr = self.load_u64_offset(argblock, 24).map_err(|_| 14u64)?;
        let out_ptr = self.load_u64_offset(argblock, 32).map_err(|_| 14u64)?;
        let out_len = self.load_u64_offset(argblock, 40).map_err(|_| 14u64)? as usize;
        let flags = self.load_u64_offset(argblock, 48).map_err(|_| 14u64)?;
        if out_len == 0 {
            return Err(22);
        }
        if flags & !NS_RESOLVE_FLAG_NOFOLLOW_FINAL != 0 {
            return Err(22);
        }
        let path = self.read_c_string(path_ptr).map_err(|_| 14u64)?;
        let resolved = self.resolve_process_path_at_raw(
            dir_value,
            &path,
            flags & NS_RESOLVE_FLAG_NOFOLLOW_FINAL != 0,
        )?;
        let bytes = resolved.as_bytes();
        let required_len = bytes.len().checked_add(1).ok_or(34u64)?;
        if required_len > out_len {
            return Err(34);
        }
        self.ensure_mapped(out_ptr, required_len, true)
            .map_err(|_| 14u64)?;
        self.write_bytes_offset(out_ptr, 0, bytes)
            .map_err(|_| 14u64)?;
        self.write_bytes_offset(out_ptr, bytes.len() as u64, &[0])
            .map_err(|_| 14u64)?;
        Ok(bytes.len() as u64)
    }

    fn domain_ctl_create(&mut self, argblock: u64) -> Result<u64, u64> {
        let parent_id = self.domain_arg_id(argblock)?;
        let parent_generation = self.load_u64_offset(argblock, 16).map_err(|_| 14u64)?;
        self.domain_ref(parent_id, parent_generation)?;
        let caller = self.current_domain_id().map_err(|_| 3u64)?;
        if !self.domain_is_descendant_or_self(parent_id, caller) {
            return Err(1);
        }
        if self.live_domain_count() >= MAX_RESOURCE_DOMAINS {
            return Err(28);
        }
        let parent_depth = self.domain_depth(parent_id).ok_or(3u64)?;
        if parent_depth + 1 > MAX_DOMAIN_DEPTH {
            return Err(40);
        }

        let parent = self.domains.get(&parent_id).ok_or(3u64)?;
        let parent_limits = parent.limits;
        let parent_caps = parent.capability_mask;
        let parent_upcalls = parent.upcall_mask;
        let parent_security = parent.security;
        let requested_cpu = self.load_u64_offset(argblock, 32).map_err(|_| 14u64)?;
        let requested_memory = self.load_u64_offset(argblock, 40).map_err(|_| 14u64)?;
        let requested_pids = self.load_u64_offset(argblock, 48).map_err(|_| 14u64)?;
        let requested_fdrs = self.load_u64_offset(argblock, 56).map_err(|_| 14u64)?;
        let profile = self.load_u64_offset(argblock, 24).map_err(|_| 14u64)?;
        let requested_caps = self.load_u64_offset(argblock, 64).map_err(|_| 14u64)?;
        let requested_upcalls = self.load_u64_offset(argblock, 72).map_err(|_| 14u64)?;
        let limits = DomainLimits {
            cpu: Self::delegate_limit(requested_cpu, parent_limits.cpu)?,
            memory: Self::delegate_limit(requested_memory, parent_limits.memory)?,
            pids: Self::delegate_limit(requested_pids, parent_limits.pids)?,
            fdrs: Self::delegate_limit(requested_fdrs, parent_limits.fdrs)?,
        };
        let capability_mask = if requested_caps == 0 {
            parent_caps
        } else if requested_caps & !parent_caps == 0 {
            requested_caps
        } else {
            return Err(1);
        };
        let upcall_mask = if requested_upcalls == 0 {
            parent_upcalls
        } else if requested_upcalls & !parent_upcalls == 0 {
            requested_upcalls
        } else {
            return Err(1);
        };
        let security = self.domain_security_from_arg(argblock, parent_security, parent_security)?;

        self.ensure_mapped(argblock, DOMAIN_QUERY_SIZE as usize, true)
            .map_err(|_| 14u64)?;
        let id = self.next_domain_id;
        self.next_domain_id += 1;
        let domain = ResourceDomain {
            id,
            generation: 1,
            parent: Some(parent_id),
            children: Vec::new(),
            profile,
            limits,
            capability_mask,
            upcall_mask,
            security,
            frozen: false,
            destroyed: false,
            cpu_ticks: 0,
        };
        self.domains.insert(id, domain);
        if let Some(parent) = self.domains.get_mut(&parent_id) {
            parent.children.push(id);
        }
        self.store_u64_offset(argblock, 8, id).map_err(|_| 14u64)?;
        self.store_u64_offset(argblock, 16, 1).map_err(|_| 14u64)?;
        self.store_u64_offset(argblock, 120, parent_id)
            .map_err(|_| 14u64)?;
        self.store_u64_offset(argblock, 128, parent_depth + 1)
            .map_err(|_| 14u64)?;
        Ok(id)
    }

    fn domain_ctl_configure(&mut self, argblock: u64) -> Result<u64, u64> {
        let id = self.domain_ref_from_arg(argblock)?;
        self.ensure_domain_control(id)?;
        if self.domain_is_frozen_or_destroyed(id) {
            return Err(11);
        }
        let parent_id = self.domains.get(&id).and_then(|domain| domain.parent);
        let parent_limits = parent_id
            .and_then(|parent| self.domains.get(&parent).map(|domain| domain.limits))
            .unwrap_or_else(DomainLimits::root);
        let child_limits = self.max_direct_child_limits(id);

        let current_limits = self.domains.get(&id).ok_or(3u64)?.limits;
        let requested_cpu = self.load_u64_offset(argblock, 32).map_err(|_| 14u64)?;
        let requested_memory = self.load_u64_offset(argblock, 40).map_err(|_| 14u64)?;
        let requested_pids = self.load_u64_offset(argblock, 48).map_err(|_| 14u64)?;
        let requested_fdrs = self.load_u64_offset(argblock, 56).map_err(|_| 14u64)?;
        let limits = DomainLimits {
            cpu: Self::configure_limit(
                requested_cpu,
                current_limits.cpu,
                parent_limits.cpu,
                child_limits.cpu,
            )?,
            memory: Self::configure_limit(
                requested_memory,
                current_limits.memory,
                parent_limits.memory,
                child_limits.memory,
            )?,
            pids: Self::configure_limit(
                requested_pids,
                current_limits.pids,
                parent_limits.pids,
                child_limits.pids,
            )?,
            fdrs: Self::configure_limit(
                requested_fdrs,
                current_limits.fdrs,
                parent_limits.fdrs,
                child_limits.fdrs,
            )?,
        };

        let parent_caps = parent_id
            .and_then(|parent| {
                self.domains
                    .get(&parent)
                    .map(|domain| domain.capability_mask)
            })
            .unwrap_or(u64::MAX);
        let parent_upcalls = parent_id
            .and_then(|parent| self.domains.get(&parent).map(|domain| domain.upcall_mask))
            .unwrap_or(u64::MAX);
        let parent_security = parent_id
            .and_then(|parent| self.domains.get(&parent).map(|domain| domain.security))
            .unwrap_or_else(DomainSecurityPolicy::root);
        let current_security = self.domains.get(&id).ok_or(3u64)?.security;
        let security =
            self.domain_security_from_arg(argblock, parent_security, current_security)?;
        let profile = self.load_u64_offset(argblock, 24).map_err(|_| 14u64)?;
        let caps = self.load_u64_offset(argblock, 64).map_err(|_| 14u64)?;
        let upcalls = self.load_u64_offset(argblock, 72).map_err(|_| 14u64)?;

        if caps != 0 {
            if caps & !parent_caps != 0 {
                return Err(1);
            }
        }
        if upcalls != 0 {
            if upcalls & !parent_upcalls != 0 {
                return Err(1);
            }
        }
        let domain = self.domains.get_mut(&id).ok_or(3u64)?;
        if profile != 0 {
            domain.profile = profile;
        }
        domain.limits = limits;
        if caps != 0 {
            domain.capability_mask = caps;
        }
        if upcalls != 0 {
            domain.upcall_mask = upcalls;
        }
        domain.security = security;
        if caps != 0 {
            self.mask_descendant_capabilities(id, caps);
        }
        if upcalls != 0 {
            self.mask_descendant_upcalls(id, upcalls);
        }
        self.clamp_descendant_security(id);
        Ok(0)
    }

    fn domain_ctl_query(&mut self, argblock: u64) -> Result<u64, u64> {
        let id = self.domain_ref_from_arg(argblock)?;
        self.ensure_domain_control(id)?;
        self.ensure_mapped(argblock, DOMAIN_QUERY_SIZE as usize, true)
            .map_err(|_| 14u64)?;
        let usage = self.domain_usage(id);
        let domain = self.domains.get(&id).ok_or(3u64)?;
        let state = if domain.destroyed {
            DOMAIN_STATE_DESTROYED
        } else if domain.frozen {
            DOMAIN_STATE_FROZEN
        } else {
            DOMAIN_STATE_ACTIVE
        };
        let fields = [
            (8, domain.id),
            (16, domain.generation),
            (24, domain.profile),
            (32, domain.limits.cpu),
            (40, domain.limits.memory),
            (48, domain.limits.pids),
            (56, domain.limits.fdrs),
            (64, domain.capability_mask),
            (72, domain.upcall_mask),
            (80, usage.cpu),
            (88, usage.memory),
            (96, usage.pids),
            (104, usage.fdrs),
            (112, state),
            (120, domain.parent.unwrap_or(0)),
            (128, self.domain_depth(id).unwrap_or(0)),
            (
                136,
                domain
                    .children
                    .iter()
                    .filter(|child| self.domain_is_live(**child))
                    .count() as u64,
            ),
            (
                DOMAIN_SECURITY_ASLR_ENABLED,
                u64::from(domain.security.aslr_enabled),
            ),
            (
                DOMAIN_SECURITY_ALLOW_WX,
                u64::from(domain.security.allow_wx),
            ),
            (
                DOMAIN_SECURITY_ALLOW_JIT_TRANSITION,
                u64::from(domain.security.allow_jit_transition),
            ),
            (DOMAIN_SECURITY_ENTROPY_QUOTA, domain.security.entropy_quota),
            (
                DOMAIN_SECURITY_DMA_ALLOWED,
                u64::from(domain.security.dma_allowed),
            ),
            (
                DOMAIN_SECURITY_HARDENING_PROFILE,
                domain.security.hardening_profile,
            ),
            (
                DOMAIN_SECURITY_EXEC_SOURCE_POLICY,
                domain.security.executable_source_policy,
            ),
        ];
        for (offset, value) in fields {
            self.store_u64_offset(argblock, offset, value)
                .map_err(|_| 14u64)?;
        }
        Ok(DOMAIN_QUERY_SIZE)
    }

    fn domain_ctl_set_frozen(&mut self, argblock: u64, frozen: bool) -> Result<u64, u64> {
        let id = self.domain_ref_from_arg(argblock)?;
        self.ensure_domain_control(id)?;
        self.set_domain_frozen_recursive(id, frozen);
        if frozen {
            self.park_domain_threads(id);
        } else {
            self.release_domain_threads();
        }
        Ok(0)
    }

    fn domain_ctl_destroy(&mut self, argblock: u64) -> Result<u64, u64> {
        let id = self.domain_ref_from_arg(argblock)?;
        self.ensure_domain_control(id)?;
        if id == ROOT_DOMAIN_ID {
            return Err(1);
        }
        if self.domain_usage(id).pids != 0 {
            return Err(16);
        }
        let parent_id = self.domains.get(&id).and_then(|domain| domain.parent);
        if let Some(parent_id) = parent_id {
            if let Some(parent) = self.domains.get_mut(&parent_id) {
                parent.children.retain(|child| *child != id);
            }
        }
        self.destroy_domain_recursive(id);
        Ok(0)
    }

    fn domain_ctl_attach_self(&mut self, argblock: u64) -> Result<u64, u64> {
        let id = self.domain_ref_from_arg(argblock)?;
        self.ensure_domain_control(id)?;
        if self.domain_is_frozen_or_destroyed(id) {
            return Err(11);
        }
        let pid = self.thread().map_err(|_| 3u64)?.pid;
        let current_domain = self
            .processes
            .get(&pid)
            .map(|process| process.domain_id)
            .ok_or(3u64)?;
        let usage = self.process_usage(pid).ok_or(3u64)?;
        self.ensure_attach_budget(id, current_domain, &usage)?;
        if let Some(process) = self.processes.get_mut(&pid) {
            process.domain_id = id;
        }
        Ok(0)
    }

    fn domain_ctl_detach_self(&mut self) -> Result<u64, u64> {
        let pid = self.thread().map_err(|_| 3u64)?.pid;
        let current = self
            .processes
            .get(&pid)
            .map(|process| process.domain_id)
            .ok_or(3u64)?;
        let parent = self
            .domains
            .get(&current)
            .and_then(|domain| domain.parent)
            .unwrap_or(ROOT_DOMAIN_ID);
        if let Some(process) = self.processes.get_mut(&pid) {
            process.domain_id = parent;
        }
        Ok(parent)
    }

    fn domain_arg_id(&mut self, argblock: u64) -> Result<u64, u64> {
        let id = self.load_u64_offset(argblock, 8).map_err(|_| 14u64)?;
        if id == 0 {
            self.current_domain_id().map_err(|_| 3u64)
        } else {
            Ok(id)
        }
    }

    fn domain_ref_from_arg(&mut self, argblock: u64) -> Result<u64, u64> {
        let id = self.domain_arg_id(argblock)?;
        let generation = self.load_u64_offset(argblock, 16).map_err(|_| 14u64)?;
        self.domain_ref(id, generation)
    }

    fn domain_ref(&self, id: u64, generation: u64) -> Result<u64, u64> {
        let Some(domain) = self.domains.get(&id) else {
            return Err(3);
        };
        if domain.destroyed || (generation != 0 && domain.generation != generation) {
            return Err(116);
        }
        Ok(id)
    }

    fn ensure_domain_control(&self, id: u64) -> Result<(), u64> {
        let caller = self.current_domain_id().map_err(|_| 3u64)?;
        if self.domain_is_descendant_or_self(id, caller) {
            Ok(())
        } else {
            Err(1)
        }
    }

    fn current_domain_id(&self) -> Result<u64, String> {
        Ok(self.process()?.domain_id)
    }

    fn require_domain_cap(&mut self, mask: u64) -> Result<(), String> {
        match self.domain_has_cap_errno(mask) {
            Ok(true) => Ok(()),
            Ok(false) => {
                self.set_status_errno(1)?;
                Err("resource domain capability denied".to_string())
            }
            Err(errno) => {
                self.set_status_errno(errno)?;
                Err("resource domain inactive".to_string())
            }
        }
    }

    fn require_domain_cap_errno(&self, mask: u64) -> Result<(), u64> {
        if self.domain_has_cap_errno(mask)? {
            Ok(())
        } else {
            Err(1)
        }
    }

    fn domain_has_cap_errno(&self, mask: u64) -> Result<bool, u64> {
        let domain_id = self.current_domain_id().map_err(|_| 3u64)?;
        let Some(domain) = self.domains.get(&domain_id) else {
            return Err(3);
        };
        if self.domain_is_frozen_or_destroyed(domain_id) {
            return Err(11);
        }
        Ok(domain.capability_mask & mask == mask)
    }

    fn fd_slot_delta(&self, fd: usize) -> Result<u64, String> {
        if fd >= FDR_COUNT || fd == MESSAGE_ENDPOINT_FD {
            return Err(format!("fd index out of range: {fd}"));
        }
        Ok(match self.process()?.fds.get(fd) {
            Some(FdHandle::Closed) => 1,
            Some(_) => 0,
            None => return Err(format!("fd index out of range: {fd}")),
        })
    }

    fn check_domain_budget(
        &mut self,
        memory_delta: u64,
        _vma_delta: u64,
        pids_delta: u64,
        fdrs_delta: u64,
    ) -> Result<bool, String> {
        match self.ensure_domain_budget_errno(memory_delta, _vma_delta, pids_delta, fdrs_delta) {
            Ok(()) => Ok(true),
            Err(errno) => {
                self.set_status_errno(errno)?;
                Ok(false)
            }
        }
    }

    fn ensure_domain_budget_errno(
        &self,
        memory_delta: u64,
        _vma_delta: u64,
        pids_delta: u64,
        fdrs_delta: u64,
    ) -> Result<(), u64> {
        let current = self.current_domain_id().map_err(|_| 3u64)?;
        let mut cursor = Some(current);
        while let Some(domain_id) = cursor {
            let Some(domain) = self.domains.get(&domain_id) else {
                return Err(3);
            };
            let usage = self.domain_usage(domain_id);
            if usage.memory.saturating_add(memory_delta) > domain.limits.memory
                || usage.pids.saturating_add(pids_delta) > domain.limits.pids
                || usage.fdrs.saturating_add(fdrs_delta) > domain.limits.fdrs
            {
                return Err(12);
            }
            cursor = domain.parent;
        }
        Ok(())
    }

    fn check_call_cpu_budget(&mut self, callee_domain_id: u64) -> Result<bool, String> {
        let caller_domain_id = self.current_domain_id()?;
        for domain_id in [caller_domain_id, callee_domain_id] {
            let Some(domain) = self.domains.get(&domain_id) else {
                self.set_status_errno(3)?;
                return Ok(false);
            };
            if self.domain_usage(domain_id).cpu >= domain.limits.cpu {
                self.set_status_errno(11)?;
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn delegate_limit(requested: u64, parent_limit: u64) -> Result<u64, u64> {
        let limit = if requested == 0 {
            parent_limit
        } else {
            requested
        };
        if limit <= parent_limit {
            Ok(limit)
        } else {
            Err(1)
        }
    }

    fn configure_limit(
        requested: u64,
        current: u64,
        parent_limit: u64,
        child_floor: u64,
    ) -> Result<u64, u64> {
        let limit = if requested == 0 { current } else { requested };
        if limit > parent_limit || limit < child_floor {
            Err(1)
        } else {
            Ok(limit)
        }
    }

    fn domain_security_from_arg(
        &mut self,
        argblock: u64,
        parent: DomainSecurityPolicy,
        current: DomainSecurityPolicy,
    ) -> Result<DomainSecurityPolicy, u64> {
        let aslr_enabled = Self::decode_domain_bool(
            self.load_u64_offset(argblock, DOMAIN_SECURITY_ASLR_ENABLED)
                .map_err(|_| 14u64)?,
            current.aslr_enabled,
        )?;
        let allow_wx = Self::decode_domain_bool(
            self.load_u64_offset(argblock, DOMAIN_SECURITY_ALLOW_WX)
                .map_err(|_| 14u64)?,
            current.allow_wx,
        )?;
        let allow_jit_transition = Self::decode_domain_bool(
            self.load_u64_offset(argblock, DOMAIN_SECURITY_ALLOW_JIT_TRANSITION)
                .map_err(|_| 14u64)?,
            current.allow_jit_transition,
        )?;
        let entropy_quota = match self
            .load_u64_offset(argblock, DOMAIN_SECURITY_ENTROPY_QUOTA)
            .map_err(|_| 14u64)?
        {
            0 => current.entropy_quota,
            quota => quota,
        };
        let dma_allowed = Self::decode_domain_bool(
            self.load_u64_offset(argblock, DOMAIN_SECURITY_DMA_ALLOWED)
                .map_err(|_| 14u64)?,
            current.dma_allowed,
        )?;
        let hardening_profile = match self
            .load_u64_offset(argblock, DOMAIN_SECURITY_HARDENING_PROFILE)
            .map_err(|_| 14u64)?
        {
            0 => current.hardening_profile,
            profile => profile,
        };
        let executable_source_policy = match self
            .load_u64_offset(argblock, DOMAIN_SECURITY_EXEC_SOURCE_POLICY)
            .map_err(|_| 14u64)?
        {
            0 => current.executable_source_policy,
            policy => policy,
        };
        let candidate = DomainSecurityPolicy {
            aslr_enabled,
            allow_wx,
            allow_jit_transition,
            entropy_quota,
            dma_allowed,
            hardening_profile,
            executable_source_policy,
        };
        Self::validate_domain_security(parent, candidate)?;
        Ok(candidate)
    }

    fn decode_domain_bool(request: u64, current: bool) -> Result<bool, u64> {
        match request {
            DOMAIN_BOOL_INHERIT => Ok(current),
            DOMAIN_BOOL_ENABLE => Ok(true),
            DOMAIN_BOOL_DISABLE => Ok(false),
            _ => Err(22),
        }
    }

    fn validate_domain_security(
        parent: DomainSecurityPolicy,
        child: DomainSecurityPolicy,
    ) -> Result<(), u64> {
        if parent.aslr_enabled && !child.aslr_enabled {
            return Err(1);
        }
        if child.allow_wx && !parent.allow_wx {
            return Err(1);
        }
        if child.allow_jit_transition && !parent.allow_jit_transition {
            return Err(1);
        }
        if child.entropy_quota > parent.entropy_quota {
            return Err(1);
        }
        if child.dma_allowed && !parent.dma_allowed {
            return Err(1);
        }
        if child.hardening_profile < parent.hardening_profile {
            return Err(1);
        }
        if child.executable_source_policy & !parent.executable_source_policy != 0 {
            return Err(1);
        }
        Ok(())
    }

    fn clamp_security_to_parent(
        parent: DomainSecurityPolicy,
        child: DomainSecurityPolicy,
    ) -> DomainSecurityPolicy {
        DomainSecurityPolicy {
            aslr_enabled: child.aslr_enabled || parent.aslr_enabled,
            allow_wx: child.allow_wx && parent.allow_wx,
            allow_jit_transition: child.allow_jit_transition && parent.allow_jit_transition,
            entropy_quota: child.entropy_quota.min(parent.entropy_quota),
            dma_allowed: child.dma_allowed && parent.dma_allowed,
            hardening_profile: child.hardening_profile.max(parent.hardening_profile),
            executable_source_policy: child.executable_source_policy
                & parent.executable_source_policy,
        }
    }

    fn domain_allows_prot(&self, prot: u64) -> Result<bool, String> {
        let domain_id = self.current_domain_id()?;
        let Some(domain) = self.domains.get(&domain_id) else {
            return Err(format!("missing resource domain {domain_id}"));
        };
        let wants_wx = prot & 0b110 == 0b110;
        Ok(!wants_wx || domain.security.allow_wx)
    }

    fn domain_allows_executable_source(
        &self,
        prot: u64,
        file_backed: bool,
    ) -> Result<bool, String> {
        if prot & 0b100 == 0 {
            return Ok(true);
        }
        let domain_id = self.current_domain_id()?;
        let Some(domain) = self.domains.get(&domain_id) else {
            return Err(format!("missing resource domain {domain_id}"));
        };
        let source = if file_backed {
            EXEC_SOURCE_FILE_MAPPING
        } else {
            EXEC_SOURCE_ANONYMOUS_JIT
        };
        Ok(domain.security.allow_jit_transition
            && domain.security.executable_source_policy & source != 0)
    }

    fn current_domain_dma_allowed(&self) -> Result<bool, String> {
        let domain_id = self.current_domain_id()?;
        let Some(domain) = self.domains.get(&domain_id) else {
            return Err(format!("missing resource domain {domain_id}"));
        };
        Ok(domain.security.dma_allowed)
    }

    fn consume_domain_entropy(&mut self, bytes: u64) -> Result<(), String> {
        if bytes == 0 {
            return Ok(());
        }
        let mut domain_ids = Vec::new();
        let mut cursor = Some(self.current_domain_id()?);
        while let Some(domain_id) = cursor {
            let Some(domain) = self.domains.get(&domain_id) else {
                return Err(format!("missing resource domain {domain_id}"));
            };
            if domain.security.entropy_quota < bytes {
                return Err("resource domain entropy quota exceeded".to_string());
            }
            domain_ids.push(domain_id);
            cursor = domain.parent;
        }
        for domain_id in domain_ids {
            if let Some(domain) = self.domains.get_mut(&domain_id) {
                domain.security.entropy_quota = domain.security.entropy_quota.saturating_sub(bytes);
            }
        }
        Ok(())
    }

    fn next_random_u64(&mut self) -> u64 {
        let mut x = self.random_state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.random_state = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn random_bytes(&mut self, len: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(len);
        while out.len() < len {
            let value = self.next_random_u64().to_le_bytes();
            let remaining = len - out.len();
            out.extend_from_slice(&value[..remaining.min(value.len())]);
        }
        out
    }

    fn env_get(
        &mut self,
        result: Reg,
        key_reg: Reg,
        index_or_buf_reg: Reg,
        len_or_flags_reg: Reg,
    ) -> Result<(), String> {
        Self::ensure_result_reg_writable(result)?;
        let key = self.read_reg(key_reg)?;
        let index_or_buf = self.read_reg(index_or_buf_reg)?;
        let len_or_flags = self.read_reg(len_or_flags_reg)?;
        if key == ENV_KEY_AUXV_ENTRY && result.0 == 30 {
            return Err(
                "ENV_GET AUXV_ENTRY result aliases secondary return register r30".to_string(),
            );
        }
        let scalar = match key {
            ENV_KEY_ISA_VERSION => Some(ENV_ISA_VERSION),
            ENV_KEY_PAGE_SIZE => Some(ASLR_PAGE),
            ENV_KEY_CACHE_LINE_SIZE => Some(ENV_CACHE_LINE_SIZE),
            ENV_KEY_TIMEBASE_HZ => Some(ENV_TIMEBASE_HZ),
            ENV_KEY_HWCAP0 => Some(
                ENV_HWCAP0_RANDOM
                    | ENV_HWCAP0_CAPABILITIES
                    | ENV_HWCAP0_RESOURCE_DOMAINS
                    | ENV_HWCAP0_DMA
                    | ENV_HWCAP0_FUTEX
                    | ENV_HWCAP0_OBJECTS
                    | ENV_HWCAP0_CALL_CAP
                    | ENV_HWCAP0_CLASSIFIER,
            ),
            ENV_KEY_HWCAP1 => Some(0),
            ENV_KEY_IMPLEMENTATION_PROFILE => Some(ENV_IMPLEMENTATION_PROFILE_REFERENCE),
            ENV_KEY_DMA_ALIGNMENT => Some(ENV_DMA_ALIGNMENT),
            ENV_KEY_TIMER_GRANULARITY_NS => Some(ENV_TIMER_GRANULARITY_NS),
            ENV_KEY_MONOTONIC_COUNTER_BITS => Some(64),
            ENV_KEY_TIME_BEHAVIOR_FLAGS => Some(ENV_TIME_FLAG_MONOTONIC | ENV_TIME_FLAG_REALTIME),
            ENV_KEY_OPCODE_FEATURE_BITS => Some(
                ENV_OPCODE_FEATURE_BASE_ISA
                    | ENV_OPCODE_FEATURE_FDR
                    | ENV_OPCODE_FEATURE_OBJECT_CTL
                    | ENV_OPCODE_FEATURE_DOMAIN_CTL
                    | ENV_OPCODE_FEATURE_DMA_CTL
                    | ENV_OPCODE_FEATURE_CALL_CAP
                    | ENV_OPCODE_FEATURE_ENV_GET
                    | ENV_OPCODE_FEATURE_RANDOM
                    | ENV_OPCODE_FEATURE_AWAIT
                    | ENV_OPCODE_FEATURE_NS_CTL,
            ),
            ENV_KEY_OBJECT_PROFILE_BITS => Some(
                ENV_OBJECT_PROFILE_COUNTER
                    | ENV_OBJECT_PROFILE_QUEUE
                    | ENV_OBJECT_PROFILE_MEMORY_OBJECT
                    | ENV_OBJECT_PROFILE_DMA_BUFFER
                    | ENV_OBJECT_PROFILE_ENDPOINT
                    | ENV_OBJECT_PROFILE_TIMER
                    | ENV_OBJECT_PROFILE_CALL_GATE
                    | ENV_OBJECT_PROFILE_CLASSIFIER_TABLE
                    | ENV_OBJECT_PROFILE_SERVICELET_PROGRAM,
            ),
            ENV_KEY_DOMAIN_FEATURE_BITS => Some(
                ENV_DOMAIN_FEATURE_NESTED
                    | ENV_DOMAIN_FEATURE_BUDGETS
                    | ENV_DOMAIN_FEATURE_SECURITY_POLICY
                    | ENV_DOMAIN_FEATURE_LIFECYCLE,
            ),
            ENV_KEY_SECURITY_PROFILE_BITS => Some(ENV_SECURITY_PROFILE_ALL),
            ENV_KEY_SCHEDULER_FEATURE_BITS => Some(ENV_SCHEDULER_FEATURE_ALL),
            ENV_KEY_CLASSIFIER_FEATURE_BITS => Some(ENV_CLASSIFIER_FEATURE_ALL),
            ENV_KEY_TOPOLOGY_RECORD_COUNT => Some(ENV_TOPOLOGY_RECORD_COUNT),
            ENV_KEY_TOPOLOGY_RECORD_FORMAT => Some(ENV_TOPOLOGY_RECORD_FORMAT),
            ENV_KEY_ARCH_THREAD_LIMIT => Some(ENV_THREAD_LIMIT),
            ENV_KEY_PROCESS_LIMIT => Some(ENV_PROCESS_LIMIT),
            ENV_KEY_RESOURCE_DOMAIN_LIMIT => Some(MAX_RESOURCE_DOMAINS as u64),
            ENV_KEY_DEFAULT_FDR_LIMIT => Some(FDR_COUNT as u64),
            ENV_KEY_EVENT_QUEUE_LIMIT => Some(ENV_EVENT_QUEUE_LIMIT),
            ENV_KEY_FUTEX_BUCKET_COUNT => Some(ENV_FUTEX_BUCKET_COUNT),
            ENV_KEY_DMA_MAX_DESCRIPTORS => Some(128),
            ENV_KEY_CLASSIFIER_ENTRY_LIMIT => Some(CLASSIFIER_MAX_RULES as u64),
            ENV_KEY_CLASSIFIER_ALLOWED_QUEUE_LIMIT => Some(CLASSIFIER_MAX_ALLOWED_QUEUES as u64),
            ENV_KEY_CLASSIFIER_ROUTE_BYTE_LIMIT => Some(CLASSIFIER_MAX_ROUTE_BYTES as u64),
            ENV_KEY_SIGNAL_NUMBER_LIMIT => Some(SIGNAL_NUMBER_LIMIT),
            ENV_KEY_STARTUP_METADATA_PTR => Some(ARG_BASE),
            ENV_KEY_STARTUP_METADATA_LEN => Some(ARG_SIZE),
            ENV_KEY_STARTUP_METADATA_FORMAT => Some(ENV_STARTUP_METADATA_FORMAT),
            ENV_KEY_STARTUP_METADATA_VERSION => Some(ENV_STARTUP_METADATA_VERSION),
            ENV_KEY_SERVICELET_VERIFY_VERSION => Some(SERVICELET_VERIFY_VERSION),
            ENV_KEY_SERVICELET_PROGRAM_LIMIT => Some(SERVICELET_MAX_PROGRAM_BYTES),
            ENV_KEY_SERVICELET_INSTRUCTION_LIMIT => Some(SERVICELET_MAX_INSTRUCTIONS),
            ENV_KEY_SERVICELET_CYCLE_LIMIT => Some(SERVICELET_MAX_CYCLES),
            ENV_KEY_SERVICELET_RECORD_LIMIT => Some(SERVICELET_MAX_RECORD_BYTES),
            ENV_KEY_SERVICELET_ACTION_LIMIT => Some(SERVICELET_MAX_ACTION_BYTES),
            ENV_KEY_SERVICELET_ISA_MASK => Some(SERVICELET_ALLOWED_ISA_MASK),
            ENV_KEY_SERVICELET_FLAG_MASK => Some(SERVICELET_FLAG_ALLOW_STATIC_LOOPS),
            ENV_KEY_ARGC => Some(self.env_argc()?),
            ENV_KEY_ARGV_BASE => Some(ARG_BASE + 8),
            ENV_KEY_ENVP_BASE => Some(self.env_envp_base()?),
            ENV_KEY_AUXV_BASE => Some(self.env_auxv_base()?),
            ENV_KEY_AUXV_ENTRY => {
                let (kind, value) = self.env_auxv_entry(index_or_buf);
                self.write_reg(Reg(30), value)?;
                Some(kind)
            }
            ENV_KEY_PERSONALITY_ID | ENV_KEY_BOOT_MANIFEST_FLAGS => Some(0),
            ENV_KEY_PROCESS_ENTRY_RECORD => {
                return self.env_get_process_entry_record(result, index_or_buf, len_or_flags);
            }
            ENV_KEY_TOPOLOGY_RECORD => {
                return self.env_get_topology_records(result, index_or_buf, len_or_flags);
            }
            _ => None,
        };

        if let Some(value) = scalar {
            self.complete_reg_ok(result, value)
        } else {
            self.complete_reg_err(result, 22)
        }
    }

    fn env_argc(&mut self) -> Result<u64, String> {
        self.load_u64(ARG_BASE)
    }

    fn env_envp_base(&mut self) -> Result<u64, String> {
        let argv_slots = self
            .env_argc()?
            .checked_add(1)
            .ok_or_else(|| "startup argv count overflow".to_string())?;
        Self::checked_record_base(ARG_BASE + 8, argv_slots, 8)
    }

    fn env_auxv_base(&mut self) -> Result<u64, String> {
        let env_slots = self
            .env_count()?
            .checked_add(1)
            .ok_or_else(|| "startup env count overflow".to_string())?;
        let envp_base = self.env_envp_base()?;
        Self::checked_record_base(envp_base, env_slots, 8)
    }

    fn env_count(&mut self) -> Result<u64, String> {
        let envp = self.env_envp_base()?;
        for idx in 0..256u64 {
            if self.load_u64(Self::checked_record_base(envp, idx, 8)?)? == 0 {
                return Ok(idx);
            }
        }
        Err("envp is not null-terminated within 256 entries".to_string())
    }

    fn env_auxv_entry(&self, index: u64) -> (u64, u64) {
        match index {
            0 => (AT_PAGESZ, ASLR_PAGE),
            1 => (AT_HWCAP, ENV_HWCAP0_RANDOM | ENV_HWCAP0_CAPABILITIES),
            2 => (AT_CLKTCK, 100),
            3 => (AT_UID, 0),
            4 => (AT_EUID, 0),
            5 => (AT_GID, 0),
            6 => (AT_EGID, 0),
            7 => (AT_RANDOM, 0),
            _ => (0, 0),
        }
    }

    fn env_get_process_entry_record(
        &mut self,
        result: Reg,
        buf: u64,
        len: u64,
    ) -> Result<(), String> {
        let mut record = Vec::with_capacity(32);
        for value in [
            self.env_argc()?,
            ARG_BASE + 8,
            self.env_envp_base()?,
            self.env_auxv_base()?,
        ] {
            record.extend_from_slice(&value.to_le_bytes());
        }
        let count = (len as usize).min(record.len());
        if count == 0 {
            return self.complete_reg_ok(result, 0);
        }
        if self.write_bytes_offset(buf, 0, &record[..count]).is_err() {
            return self.complete_reg_err(result, 14);
        }
        self.complete_reg_ok(result, count as u64)
    }

    fn env_get_topology_records(&mut self, result: Reg, buf: u64, len: u64) -> Result<(), String> {
        let records = self.env_topology_records();
        let count = (len as usize).min(records.len());
        if count == 0 {
            return self.complete_reg_ok(result, 0);
        }
        if self.write_bytes_offset(buf, 0, &records[..count]).is_err() {
            return self.complete_reg_err(result, 14);
        }
        self.complete_reg_ok(result, count as u64)
    }

    fn env_topology_records(&self) -> Vec<u8> {
        let mut records =
            Vec::with_capacity(ENV_TOPOLOGY_RECORD_SIZE * ENV_TOPOLOGY_RECORD_COUNT as usize);
        for fields in [
            [
                1,
                0,
                ROOT_DOMAIN_ID,
                ENV_THREAD_LIMIT,
                ENV_CACHE_LINE_SIZE,
                ENV_TIMEBASE_HZ,
                ENV_SCHEDULER_FEATURE_ALL,
                0,
            ],
            [
                2,
                0,
                0,
                MEMORY_SIZE as u64,
                ASLR_PAGE,
                ENV_DMA_ALIGNMENT,
                ENV_SECURITY_PROFILE_ALL,
                0,
            ],
            [
                3,
                0,
                0,
                MEMORY_SIZE as u64,
                ENV_CACHE_LINE_SIZE,
                1,
                ENV_HWCAP0_DMA,
                0,
            ],
            [
                4,
                0,
                0,
                CLASSIFIER_MAX_RULES as u64,
                CLASSIFIER_MAX_ALLOWED_QUEUES as u64,
                CLASSIFIER_MAX_ROUTE_BYTES as u64,
                ENV_CLASSIFIER_FEATURE_ALL,
                0,
            ],
            [
                5,
                SERVICELET_VERIFY_VERSION,
                SERVICELET_MAX_PROGRAM_BYTES,
                SERVICELET_MAX_INSTRUCTIONS,
                SERVICELET_MAX_CYCLES,
                SERVICELET_MAX_RECORD_BYTES,
                SERVICELET_MAX_ACTION_BYTES,
                SERVICELET_ALLOWED_ISA_MASK,
            ],
        ] {
            for value in fields {
                records.extend_from_slice(&value.to_le_bytes());
            }
        }
        debug_assert_eq!(
            records.len(),
            ENV_TOPOLOGY_RECORD_COUNT as usize * ENV_TOPOLOGY_RECORD_SIZE
        );
        records
    }

    fn max_direct_child_limits(&self, id: u64) -> DomainLimits {
        let mut out = DomainLimits {
            cpu: 0,
            memory: 0,
            pids: 0,
            fdrs: 0,
        };
        if let Some(domain) = self.domains.get(&id) {
            for child in &domain.children {
                if let Some(child) = self.domains.get(child).filter(|child| !child.destroyed) {
                    out.cpu = out.cpu.max(child.limits.cpu);
                    out.memory = out.memory.max(child.limits.memory);
                    out.pids = out.pids.max(child.limits.pids);
                    out.fdrs = out.fdrs.max(child.limits.fdrs);
                }
            }
        }
        out
    }

    fn mask_descendant_capabilities(&mut self, id: u64, mask: u64) {
        let children = self
            .domains
            .get(&id)
            .map(|domain| domain.children.clone())
            .unwrap_or_default();
        for child_id in children {
            if let Some(child) = self.domains.get_mut(&child_id) {
                child.capability_mask &= mask;
            }
            self.mask_descendant_capabilities(child_id, mask);
        }
    }

    fn mask_descendant_upcalls(&mut self, id: u64, mask: u64) {
        let children = self
            .domains
            .get(&id)
            .map(|domain| domain.children.clone())
            .unwrap_or_default();
        for child_id in children {
            if let Some(child) = self.domains.get_mut(&child_id) {
                child.upcall_mask &= mask;
            }
            self.mask_descendant_upcalls(child_id, mask);
        }
    }

    fn clamp_descendant_security(&mut self, id: u64) {
        let Some(parent_security) = self.domains.get(&id).map(|domain| domain.security) else {
            return;
        };
        let children = self
            .domains
            .get(&id)
            .map(|domain| domain.children.clone())
            .unwrap_or_default();
        for child_id in children {
            if let Some(child) = self.domains.get_mut(&child_id) {
                child.security = Self::clamp_security_to_parent(parent_security, child.security);
            }
            self.clamp_descendant_security(child_id);
        }
    }

    fn set_domain_frozen_recursive(&mut self, id: u64, frozen: bool) {
        if let Some(domain) = self.domains.get_mut(&id) {
            domain.frozen = frozen;
            let children = domain.children.clone();
            for child in children {
                self.set_domain_frozen_recursive(child, frozen);
            }
        }
    }

    fn destroy_domain_recursive(&mut self, id: u64) {
        let children = self
            .domains
            .get(&id)
            .map(|domain| domain.children.clone())
            .unwrap_or_default();
        for child in children {
            self.destroy_domain_recursive(child);
        }
        if let Some(domain) = self.domains.get_mut(&id) {
            domain.children.clear();
            domain.destroyed = true;
            domain.frozen = true;
            domain.generation = domain.generation.saturating_add(1);
        }
    }

    fn park_domain_threads(&mut self, id: u64) {
        let mut kept_ready = VecDeque::new();
        while let Some(tid) = self.ready.pop_front() {
            let parked = self
                .threads
                .get(&tid)
                .and_then(|thread| self.processes.get(&thread.pid))
                .is_some_and(|process| self.domain_is_descendant_or_self(process.domain_id, id));
            if parked {
                if !self.domain_parked.contains(&tid) {
                    self.domain_parked.push_back(tid);
                }
            } else {
                kept_ready.push_back(tid);
            }
        }
        self.ready = kept_ready;
    }

    fn release_domain_threads(&mut self) {
        let parked = std::mem::take(&mut self.domain_parked);
        for tid in parked {
            match self.thread_domain_frozen(tid) {
                Ok(false) => self.wake_thread(tid),
                Ok(true) if self.threads.contains_key(&tid) => self.domain_parked.push_back(tid),
                _ => {}
            }
        }
    }

    fn thread_domain_frozen(&self, tid: u64) -> Result<bool, String> {
        let Some(thread) = self.threads.get(&tid) else {
            return Ok(false);
        };
        let Some(process) = self.processes.get(&thread.pid) else {
            return Ok(false);
        };
        Ok(self.domain_is_frozen_or_destroyed(process.domain_id))
    }

    fn domain_is_frozen_or_destroyed(&self, id: u64) -> bool {
        let mut cursor = Some(id);
        while let Some(domain_id) = cursor {
            let Some(domain) = self.domains.get(&domain_id) else {
                return true;
            };
            if domain.frozen || domain.destroyed {
                return true;
            }
            cursor = domain.parent;
        }
        false
    }

    fn domain_is_live(&self, id: u64) -> bool {
        self.domains
            .get(&id)
            .is_some_and(|domain| !domain.destroyed)
    }

    fn live_domain_count(&self) -> usize {
        self.domains
            .values()
            .filter(|domain| !domain.destroyed)
            .count()
    }

    fn domain_depth(&self, id: u64) -> Option<u64> {
        let mut depth = 0;
        let mut cursor = self.domains.get(&id)?.parent;
        while let Some(parent) = cursor {
            depth += 1;
            cursor = self.domains.get(&parent)?.parent;
        }
        Some(depth)
    }

    fn domain_is_descendant_or_self(&self, id: u64, ancestor: u64) -> bool {
        let mut cursor = Some(id);
        while let Some(domain_id) = cursor {
            if domain_id == ancestor {
                return true;
            }
            cursor = self
                .domains
                .get(&domain_id)
                .and_then(|domain| domain.parent);
        }
        false
    }

    fn domain_usage(&self, id: u64) -> DomainUsage {
        let mut usage = DomainUsage::default();
        for domain in self.domains.values() {
            if !domain.destroyed && self.domain_is_descendant_or_self(domain.id, id) {
                usage.cpu = usage.cpu.saturating_add(domain.cpu_ticks);
            }
        }
        for process in self.processes.values() {
            if self.domain_is_descendant_or_self(process.domain_id, id) {
                usage.memory = usage
                    .memory
                    .saturating_add(Self::process_memory_usage(process));
                usage.fdrs = usage.fdrs.saturating_add(
                    process
                        .fds
                        .iter()
                        .enumerate()
                        .filter(|(idx, fd)| {
                            *idx != MESSAGE_ENDPOINT_FD && !matches!(fd, FdHandle::Closed)
                        })
                        .count() as u64,
                );
            }
        }
        for thread in self.threads.values() {
            if let Some(process) = self.processes.get(&thread.pid) {
                if self.domain_is_descendant_or_self(process.domain_id, id) {
                    usage.pids = usage.pids.saturating_add(1);
                }
            }
        }
        usage
    }

    fn process_usage(&self, pid: u64) -> Option<DomainUsage> {
        let process = self.processes.get(&pid)?;
        let mut usage = DomainUsage::default();
        usage.memory = Self::process_memory_usage(process);
        usage.fdrs = process
            .fds
            .iter()
            .enumerate()
            .filter(|(idx, fd)| *idx != MESSAGE_ENDPOINT_FD && !matches!(fd, FdHandle::Closed))
            .count() as u64;
        usage.pids = self
            .threads
            .values()
            .filter(|thread| thread.pid == pid)
            .count() as u64;
        Some(usage)
    }

    fn process_memory_usage(process: &Process) -> u64 {
        process.fds.iter().fold(
            process.vmas.iter().map(|vma| vma.len).sum::<u64>(),
            |usage, fd| {
                if let FdHandle::MemoryObject { data, .. } = fd {
                    usage.saturating_add(data.borrow().len() as u64)
                } else {
                    usage
                }
            },
        )
    }

    fn ensure_attach_budget(
        &self,
        target_domain: u64,
        current_domain: u64,
        process_usage: &DomainUsage,
    ) -> Result<(), u64> {
        let mut cursor = Some(target_domain);
        while let Some(domain_id) = cursor {
            if domain_id == current_domain {
                break;
            }
            let Some(domain) = self.domains.get(&domain_id) else {
                return Err(3);
            };
            let usage = self.domain_usage(domain_id);
            if usage.memory.saturating_add(process_usage.memory) > domain.limits.memory
                || usage.pids.saturating_add(process_usage.pids) > domain.limits.pids
                || usage.fdrs.saturating_add(process_usage.fdrs) > domain.limits.fdrs
            {
                return Err(12);
            }
            cursor = domain.parent;
        }
        Ok(())
    }

    fn charge_cpu_tick(&mut self) -> Result<(), String> {
        let mut cursor = Some(self.current_domain_id()?);
        while let Some(domain_id) = cursor {
            let Some(domain) = self.domains.get_mut(&domain_id) else {
                return Err(format!("missing resource domain {domain_id}"));
            };
            domain.cpu_ticks = domain.cpu_ticks.saturating_add(1);
            cursor = domain.parent;
        }
        Ok(())
    }

    fn read_reg(&self, reg: Reg) -> Result<u64, String> {
        Ok(if reg.0 == 0 {
            0
        } else {
            self.thread()?.regs[reg.0]
        })
    }

    fn write_reg(&mut self, reg: Reg, value: u64) -> Result<(), String> {
        if reg.0 == 31 {
            return Err("write to hardware-locked stack pointer r31".to_string());
        }
        if reg.0 != 0 {
            self.thread_mut()?.regs[reg.0] = value;
        }
        Ok(())
    }

    fn ensure_result_reg_writable(reg: Reg) -> Result<(), String> {
        if reg.0 == 31 {
            Err("write to hardware-locked stack pointer r31".to_string())
        } else {
            Ok(())
        }
    }

    fn read_f64(&self, reg: FReg) -> Result<f64, String> {
        Ok(f64::from_bits(self.thread()?.fregs[reg.0]))
    }

    fn write_freg(&mut self, reg: FReg, value: f64) -> Result<(), String> {
        self.thread_mut()?.fregs[reg.0] = value.to_bits();
        Ok(())
    }

    fn condition(&self, condition: Condition) -> Result<bool, String> {
        let flags = self.thread()?.flags;
        Ok(match condition {
            Condition::Eq => flags.zero,
            Condition::Ne => !flags.zero,
            Condition::Lt => flags.negative,
            Condition::Gt => flags.greater,
            Condition::Le => flags.zero || flags.negative,
            Condition::Ge => flags.zero || flags.greater,
        })
    }

    fn resolve_value(&self, value: Value) -> Result<u64, String> {
        match value {
            Value::Imm(v) => Ok(v as u64),
            Value::Label(label) => {
                if let Some(addr) = self.process()?.program.data_labels.get(&label) {
                    Ok(*addr)
                } else if let Some(ip) = self.process()?.program.labels.get(&label) {
                    Ok(*ip as u64)
                } else {
                    Err(format!("unknown label {label:?}"))
                }
            }
        }
    }

    fn resolve_target(&self, target: Target) -> Result<usize, String> {
        match target {
            Target::Address(ip) => Ok(ip),
            Target::Label(label) => self
                .process()?
                .program
                .labels
                .get(&label)
                .copied()
                .ok_or_else(|| format!("unknown code label {label:?}")),
        }
    }

    fn resolve_mem(&self, mem: MemRef) -> Result<u64, String> {
        match mem {
            MemRef::BaseOffset(base, offset) => {
                Ok(self.read_reg(base)?.wrapping_add(offset as u64))
            }
            MemRef::Label(label) => self
                .process()?
                .program
                .data_labels
                .get(&label)
                .copied()
                .ok_or_else(|| format!("unknown data label {label:?}")),
        }
    }

    fn load_width(&mut self, addr: u64, width: Width) -> Result<u64, String> {
        let bytes = self.read_bytes(addr, width.bytes())?;
        Ok(match width {
            Width::Byte => bytes[0] as u64,
            Width::Word => u32::from_le_bytes(bytes.try_into().unwrap()) as u64,
            Width::Double => u64::from_le_bytes(bytes.try_into().unwrap()),
        })
    }

    fn store_width(&mut self, addr: u64, value: u64, width: Width) -> Result<(), String> {
        match width {
            Width::Byte => self.write_bytes(addr, &[value as u8]),
            Width::Word => self.write_bytes(addr, &(value as u32).to_le_bytes()),
            Width::Double => self.write_bytes(addr, &value.to_le_bytes()),
        }
    }

    fn load_u64(&mut self, addr: u64) -> Result<u64, String> {
        self.load_width(addr, Width::Double)
    }

    fn store_u64(&mut self, addr: u64, value: u64) -> Result<(), String> {
        self.store_width(addr, value, Width::Double)
    }

    fn read_bytes(&mut self, addr: u64, len: usize) -> Result<Vec<u8>, String> {
        self.ensure_mapped(addr, len, false)?;
        let process = self.process()?;
        let start = addr as usize;
        let end = start
            .checked_add(len)
            .ok_or_else(|| format!("memory range overflow at 0x{addr:x}"))?;
        Ok(process.memory[start..end].to_vec())
    }

    fn write_bytes(&mut self, addr: u64, data: &[u8]) -> Result<(), String> {
        self.ensure_mapped(addr, data.len(), true)?;
        let process = self.process_mut()?;
        let start = addr as usize;
        let end = start
            .checked_add(data.len())
            .ok_or_else(|| format!("memory range overflow at 0x{addr:x}"))?;
        process.memory[start..end].copy_from_slice(data);
        Ok(())
    }

    fn ensure_mapped(&mut self, addr: u64, len: usize, write: bool) -> Result<(), String> {
        let process = self.process_mut()?;
        let start = usize::try_from(addr).map_err(|_| {
            format!(
                "hardware SIGSEGV: unmapped address 0x{addr:x} + {len} (outside process memory)"
            )
        })?;
        let end = start.checked_add(len).ok_or_else(|| {
            format!("hardware SIGSEGV: unmapped address 0x{addr:x} + {len} (memory range overflow)")
        })?;
        if end > process.memory.len() {
            return Err(format!(
                "hardware SIGSEGV: unmapped address 0x{addr:x} + {len} (outside process memory)"
            ));
        }
        let idx = process
            .vmas
            .iter()
            .position(|vma| vma.contains(addr, len))
            .ok_or_else(|| format!("hardware SIGSEGV: unmapped address 0x{addr:x} + {len}"))?;
        if process.vmas[idx].guard {
            return Err(format!("hardware SIGSEGV: guard page access at 0x{addr:x}"));
        }
        if process.vmas[idx].prot == 0 {
            return Err(format!("hardware SIGSEGV: no-access VMA at 0x{addr:x}"));
        }
        if write && process.vmas[idx].prot & 0b10 == 0 {
            return Err(format!("hardware SIGSEGV: write denied at 0x{addr:x}"));
        }
        if !write && process.vmas[idx].prot & 0b01 == 0 {
            return Err(format!("hardware SIGSEGV: read denied at 0x{addr:x}"));
        }
        if !process.vmas[idx].resident {
            let (start, vma_len, file_offset) = {
                let vma = &process.vmas[idx];
                (vma.start, vma.len, vma.file_offset)
            };
            let vma_len = usize::try_from(vma_len)
                .map_err(|_| "VMA length exceeds host usize".to_string())?;
            let start =
                usize::try_from(start).map_err(|_| "VMA start exceeds host usize".to_string())?;
            let end = start
                .checked_add(vma_len)
                .ok_or_else(|| "VMA page-in range overflow".to_string())?;
            if end > process.memory.len() {
                return Err("VMA page-in exceeds process memory".to_string());
            }
            process.memory[start..end].fill(0);
            if let Some(file) = &mut process.vmas[idx].file {
                file.seek(SeekFrom::Start(file_offset))
                    .map_err(|err| format!("file-backed VMA seek failed: {err}"))?;
                let mut tmp = vec![0; vma_len];
                let count = file
                    .read(&mut tmp)
                    .map_err(|err| format!("file-backed VMA page-in failed: {err}"))?;
                let end = start
                    .checked_add(count)
                    .ok_or_else(|| "file-backed VMA page-in range overflow".to_string())?;
                process.memory[start..end].copy_from_slice(&tmp[..count]);
            }
            process.vmas[idx].resident = true;
        }
        Ok(())
    }

    fn instruction_fetch_fault(&self, addr: u64) -> Result<Option<String>, String> {
        let process = self.process()?;
        let start = usize::try_from(addr).map_err(|_| {
            format!("hardware SIGSEGV: unmapped address 0x{addr:x} + 1 (outside process memory)")
        })?;
        if start >= process.memory.len() {
            return Ok(Some(format!(
                "hardware SIGSEGV: unmapped address 0x{addr:x} + 1 (outside process memory)"
            )));
        }
        let Some(vma) = process.vmas.iter().find(|vma| vma.contains(addr, 1)) else {
            return Ok(None);
        };
        if vma.guard {
            return Ok(Some(format!(
                "hardware SIGSEGV: guard page execute at 0x{addr:x}"
            )));
        }
        if vma.prot == 0 {
            return Ok(Some(format!(
                "hardware SIGSEGV: no-access VMA execute at 0x{addr:x}"
            )));
        }
        if vma.prot & 0b100 == 0 {
            return Ok(Some(format!(
                "hardware SIGSEGV: execute denied at 0x{addr:x}"
            )));
        }
        Ok(Some(format!(
            "hardware SIGSEGV: dynamic instruction fetch is not modeled at 0x{addr:x}"
        )))
    }

    fn read_c_string(&mut self, addr: u64) -> Result<String, String> {
        let mut bytes = Vec::new();
        let mut pos = addr;
        loop {
            let byte = self.load_width(pos, Width::Byte)? as u8;
            if byte == 0 {
                break;
            }
            bytes.push(byte);
            pos = pos.checked_add(1).ok_or_else(|| {
                format!("unterminated string overflows address space at 0x{addr:x}")
            })?;
        }
        String::from_utf8(bytes).map_err(|err| format!("invalid utf-8 string at 0x{addr:x}: {err}"))
    }

    fn collect_exec_args(&mut self, path: &str, argv: u64) -> Result<Vec<String>, String> {
        if argv == 0 {
            return Ok(vec![path.to_string()]);
        }
        self.collect_exec_string_vector(argv, "argv")
    }

    fn collect_exec_env(&mut self, envp: u64) -> Result<Vec<String>, String> {
        if envp == 0 {
            return Ok(Vec::new());
        }
        self.collect_exec_string_vector(envp, "envp")
    }

    fn collect_exec_string_vector(
        &mut self,
        vector: u64,
        name: &str,
    ) -> Result<Vec<String>, String> {
        let mut values = Vec::new();
        for idx in 0..256u64 {
            let ptr = self.load_u64(Self::checked_record_base(vector, idx, 8)?)?;
            if ptr == 0 {
                return Ok(values);
            }
            values.push(self.read_c_string(ptr)?);
        }
        Err(format!(
            "EXEC {name} is not null-terminated within 256 entries"
        ))
    }

    fn read_pcr(&self, pcr: Pcr) -> Result<u64, String> {
        let process = self.process()?;
        Ok(match pcr {
            Pcr::Pid => process.pid,
            Pcr::Ppid => process.parent_pid.unwrap_or(0),
            Pcr::Tid => self.thread()?.tid,
            Pcr::Uid => process.uid,
            Pcr::Gid => process.gid,
            Pcr::Tp => self.thread()?.thread_pointer,
            Pcr::Sigmask => process.sigmask,
            Pcr::Sigpending => process
                .pending_events
                .iter()
                .filter_map(|event| event.signal_number())
                .filter(|signum| Self::valid_signal_number(*signum))
                .fold(0u64, |mask, signum| mask | (1u64 << signum.min(63))),
            Pcr::RealtimeSec => {
                let now = Self::system_time_to_host_timespec(SystemTime::now());
                now.tv_sec as u64
            }
            Pcr::RealtimeNsec => {
                let now = Self::system_time_to_host_timespec(SystemTime::now());
                now.tv_nsec as u64
            }
        })
    }

    fn write_pcr(&mut self, pcr: Pcr, value: u64) -> Result<(), String> {
        let process = self.process_mut()?;
        match pcr {
            Pcr::Pid
            | Pcr::Ppid
            | Pcr::Tid
            | Pcr::Sigpending
            | Pcr::RealtimeSec
            | Pcr::RealtimeNsec => Err("selected PCR is read-only".to_string()),
            Pcr::Tp => {
                self.thread_mut()?.thread_pointer = value;
                Ok(())
            }
            Pcr::Uid if process.uid != 0 => {
                Err("SET_PCR UID denied: current UID is not 0".to_string())
            }
            Pcr::Uid => {
                process.uid = value;
                Ok(())
            }
            Pcr::Gid => {
                process.gid = value;
                Ok(())
            }
            Pcr::Sigmask => {
                process.sigmask = value;
                Ok(())
            }
        }
    }

    fn exit_current(&mut self, code: i32) -> Result<(), String> {
        let tid = self.current_tid;
        let pid = self.thread()?.pid;
        let parent_pid = self.process()?.parent_pid;
        self.threads.remove(&tid);
        self.completed_threads.insert(tid, code as u64);
        if let Some(waiters) = self.thread_join_waiters.remove(&tid) {
            for waiter in waiters {
                self.wake_thread(waiter);
            }
        }
        self.remove_thread_wait_state(tid);
        self.last_exit = code;
        if !self.threads.values().any(|thread| thread.pid == pid) {
            self.advisory_locks.retain(|_, lock| lock.owner_pid != pid);
            self.processes.remove(&pid);
            if let Some(parent_pid) = parent_pid {
                if let Some(parent) = self.processes.get_mut(&parent_pid) {
                    self.completed_children.insert((parent_pid, pid), code);
                    Self::enqueue_pending_event(parent, NativeEvent::child_signal(SIGCHLD));
                }
                if let Some(waiters) = self.child_waiters.remove(&parent_pid) {
                    for waiter in waiters {
                        self.wake_thread(waiter);
                    }
                }
            }
        }
        Ok(())
    }

    fn remove_thread_wait_state(&mut self, tid: u64) {
        self.ready.retain(|ready_tid| *ready_tid != tid);
        self.domain_parked.retain(|parked_tid| *parked_tid != tid);
        self.sleepers.retain(|(sleep_tid, _)| *sleep_tid != tid);
        self.fd_waiters.retain(|waiter| waiter.tid != tid);
        for waiters in self.futex_waiters.values_mut() {
            waiters.retain(|waiter_tid| *waiter_tid != tid);
        }
        self.futex_waiters.retain(|_, waiters| !waiters.is_empty());
        for waiters in self.thread_join_waiters.values_mut() {
            waiters.retain(|waiter_tid| *waiter_tid != tid);
        }
        self.thread_join_waiters
            .retain(|_, waiters| !waiters.is_empty());
        for waiters in self.child_waiters.values_mut() {
            waiters.retain(|waiter_tid| *waiter_tid != tid);
        }
        self.child_waiters.retain(|_, waiters| !waiters.is_empty());
    }

    fn push_unique_waiter(waiters: &mut VecDeque<u64>, tid: u64) {
        if !waiters.contains(&tid) {
            waiters.push_back(tid);
        }
    }

    fn wake_thread(&mut self, tid: u64) {
        if self.threads.contains_key(&tid) && !self.ready.contains(&tid) {
            self.sleepers.retain(|(sleep_tid, _)| *sleep_tid != tid);
            self.ready.push_back(tid);
        }
    }

    fn tick_sleepers(&mut self) {
        let mut woke = Vec::new();
        for (tid, ticks) in &mut self.sleepers {
            *ticks = ticks.saturating_sub(1);
            if *ticks == 0 {
                woke.push(*tid);
            }
        }
        self.sleepers.retain(|(_, ticks)| *ticks != 0);
        for tid in woke {
            self.wake_thread(tid);
        }
    }

    fn tick_alarms(&mut self) {
        let mut expired = Vec::new();
        for (pid, ticks) in &mut self.alarms {
            *ticks = ticks.saturating_sub(1);
            if *ticks == 0 {
                expired.push(*pid);
            }
        }
        self.alarms.retain(|(_, ticks)| *ticks != 0);
        for pid in expired {
            self.queue_process_event(pid, NativeEvent::timer_signal(SIGALRM));
        }
    }

    fn tick_timers(&mut self) {
        let mut seen: Vec<*const RefCell<TimerState>> = Vec::new();
        for process in self.processes.values_mut() {
            for handle in &mut process.fds {
                let FdHandle::Timer(timer) = handle else {
                    continue;
                };
                let ptr = Rc::as_ptr(timer);
                if seen.contains(&ptr) {
                    continue;
                }
                seen.push(ptr);
                let mut timer = timer.borrow_mut();
                if timer.remaining == 0 {
                    continue;
                }
                timer.remaining = timer.remaining.saturating_sub(1);
                if timer.remaining == 0 {
                    timer.expirations = timer.expirations.saturating_add(1);
                    if timer.interval != 0 {
                        timer.remaining = timer.interval;
                    }
                }
            }
        }
    }

    fn queue_process_event(&mut self, pid: u64, event: NativeEvent) -> bool {
        if let Some(process) = self.processes.get_mut(&pid) {
            if !Self::enqueue_pending_event(process, event) {
                return false;
            }
            if let Some(tid) = self
                .threads
                .values()
                .find(|thread| thread.pid == pid)
                .map(|thread| thread.tid)
            {
                self.wake_thread(tid);
            }
            true
        } else {
            false
        }
    }

    fn poll_fd_waiters(&mut self) {
        let waiters = std::mem::take(&mut self.fd_waiters);
        for waiter in waiters {
            let state = self
                .with_thread_process(waiter.tid, |machine| {
                    if !machine.fd_generation_matches(waiter.fd, waiter.generation)? {
                        return Ok(FdWaiterState::Stale);
                    }
                    if let Err(errno) = machine.fd_right_errno(waiter.fd, CAP_RIGHT_POLL) {
                        return Ok(FdWaiterState::Error(errno));
                    }
                    if machine.fd_ready_for_mask(waiter.fd, waiter.mask)? {
                        Ok(FdWaiterState::Ready)
                    } else {
                        Ok(FdWaiterState::Pending)
                    }
                })
                .unwrap_or(FdWaiterState::Stale);
            match state {
                FdWaiterState::Ready => {
                    let _ = self.with_thread_process(waiter.tid, |machine| {
                        machine.set_errno(0)?;
                        if let Some(result) = waiter.result {
                            machine.write_reg(result, 0)?;
                        }
                        Ok(())
                    });
                    self.wake_thread(waiter.tid);
                }
                FdWaiterState::Error(errno) => {
                    let _ = self.with_thread_process(waiter.tid, |machine| {
                        machine.set_errno(errno)?;
                        if let Some(result) = waiter.result {
                            machine.write_reg(result, -1i64 as u64)?;
                        }
                        Ok(())
                    });
                    self.wake_thread(waiter.tid);
                }
                FdWaiterState::Pending if self.threads.contains_key(&waiter.tid) => {
                    self.fd_waiters.push(waiter);
                }
                FdWaiterState::Stale => {
                    let _ = self.with_thread_process(waiter.tid, |machine| {
                        machine.set_errno(116)?;
                        if let Some(result) = waiter.result {
                            machine.write_reg(result, -1i64 as u64)?;
                        }
                        Ok(())
                    });
                    self.wake_thread(waiter.tid);
                }
                FdWaiterState::Pending => {}
            }
        }
    }

    fn push_fd_waiter(&mut self, fd: usize, mask: u64, result: Option<Reg>) -> Result<(), String> {
        if let Some(result) = result {
            Self::ensure_result_reg_writable(result)?;
        }
        let generation = self.fd_generation(fd)?;
        self.fd_waiters.push(FdWaiter {
            tid: self.current_tid,
            fd,
            generation,
            mask,
            result,
        });
        Ok(())
    }

    fn fd_generation(&self, fd: usize) -> Result<u64, String> {
        self.process()?
            .fd_generations
            .get(fd)
            .copied()
            .ok_or_else(|| format!("fd index out of range: {fd}"))
    }

    fn fd_generation_matches(&self, fd: usize, generation: u64) -> Result<bool, String> {
        let process = self.process()?;
        Ok(process.fd_generations.get(fd).copied() == Some(generation)
            && !matches!(process.fds.get(fd), Some(FdHandle::Closed) | None))
    }

    fn fd_accepts_call_completion(&self, fd: usize) -> Result<bool, String> {
        Ok(matches!(
            self.process()?.fds.get(fd),
            Some(FdHandle::Counter(_) | FdHandle::EventCounter { .. } | FdHandle::PipeWriter(_))
        ))
    }

    fn with_thread_process<T>(
        &mut self,
        tid: u64,
        f: impl FnOnce(&mut Self) -> Result<T, String>,
    ) -> Result<T, String> {
        let saved = self.current_tid;
        self.current_tid = tid;
        let result = f(self);
        self.current_tid = saved;
        result
    }

    fn poll_fd_mask(&mut self, fd: u64, events: u64) -> Result<u64, String> {
        let revents = match self.decode_fd_value(fd) {
            Ok(fd) => self.poll_fd_index_mask_raw(fd, events)?,
            Err(_) => POLLNVAL_MASK,
        };
        self.set_errno(0)?;
        Ok(revents)
    }

    fn poll_fd_index_mask(&mut self, fd: usize, events: u64) -> Result<u64, String> {
        let revents = self.poll_fd_index_mask_raw(fd, events)?;
        self.set_errno(0)?;
        Ok(revents)
    }

    fn poll_fd_index_mask_raw(&mut self, fd: usize, events: u64) -> Result<u64, String> {
        if fd >= FDR_COUNT {
            return Ok(POLLNVAL_MASK);
        }
        if self.ensure_fd_right(fd, CAP_RIGHT_POLL).is_err() {
            return Ok(POLLNVAL_MASK);
        }
        if matches!(self.process()?.fds[fd], FdHandle::Closed) {
            return Ok(POLLNVAL_MASK);
        }
        let mut revents = 0;
        if events & POLLIN_MASK != 0 && self.fd_read_ready(fd)? {
            revents |= POLLIN_MASK;
        }
        if events & POLLOUT_MASK != 0 && self.fd_write_ready(fd)? {
            revents |= POLLOUT_MASK;
        }
        Ok(revents)
    }

    fn fd_ready_for_mask(&mut self, fd: usize, mask: u64) -> Result<bool, String> {
        if mask == 0 {
            self.fd_ready(fd)
        } else {
            Ok(self.poll_fd_index_mask_raw(fd, mask)? != 0)
        }
    }

    fn await_fd_ready_or_error(
        &mut self,
        result: Reg,
        fd: usize,
        mask: u64,
    ) -> Result<Option<bool>, String> {
        if fd >= FDR_COUNT {
            self.complete_reg_err(result, 9)?;
            return Ok(None);
        }
        if let Err(errno) = self.fd_right_errno(fd, CAP_RIGHT_POLL) {
            self.complete_reg_err(result, errno)?;
            return Ok(None);
        }
        if matches!(self.process()?.fds[fd], FdHandle::Closed) {
            self.complete_reg_err(result, 9)?;
            return Ok(None);
        }
        if mask == 0 {
            return self.fd_ready(fd).map(Some);
        }
        Ok(Some(self.poll_fd_index_mask_raw(fd, mask)? != 0))
    }

    fn fd_read_ready(&mut self, fd: usize) -> Result<bool, String> {
        if fd == MESSAGE_ENDPOINT_FD {
            return Ok(!self.process()?.inbox.is_empty());
        }
        let handle = &mut self.process_mut()?.fds[fd];
        match handle {
            FdHandle::Stdin
            | FdHandle::File(_)
            | FdHandle::Dir { .. }
            | FdHandle::Counter(_)
            | FdHandle::MemoryObject { .. } => Ok(true),
            FdHandle::EventCounter { value, .. } => Ok(*value.borrow() != 0),
            FdHandle::Timer(timer) => Ok(timer.borrow().expirations != 0),
            FdHandle::PipeReader(buffer) => {
                let buffer = buffer.borrow();
                Ok(!buffer.bytes.is_empty() || !buffer.capabilities.is_empty())
            }
            FdHandle::TcpListener { listener, pending } => {
                if pending.is_some() {
                    return Ok(true);
                }
                match listener.accept() {
                    Ok((stream, _)) => {
                        stream
                            .set_nonblocking(false)
                            .map_err(|err| format!("TCP accepted stream blocking failed: {err}"))?;
                        *pending = Some(stream);
                        Ok(true)
                    }
                    Err(err) if err.kind() == io::ErrorKind::WouldBlock => Ok(false),
                    Err(err) => Err(format!("TCP accept failed: {err}")),
                }
            }
            FdHandle::TcpStream(stream) => {
                let mut byte = [0u8; 1];
                match stream.peek(&mut byte) {
                    Ok(count) => Ok(count > 0),
                    Err(err) if err.kind() == io::ErrorKind::WouldBlock => Ok(false),
                    Err(_) => Ok(true),
                }
            }
            FdHandle::Stdout
            | FdHandle::Stderr
            | FdHandle::MessageEndpoint
            | FdHandle::PipeWriter(_)
            | FdHandle::TcpSocket { .. }
            | FdHandle::DmaBuffer { .. }
            | FdHandle::CallGate { .. }
            | FdHandle::ClassifierTable(_)
            | FdHandle::ServiceletProgram(_)
            | FdHandle::Closed => Ok(false),
        }
    }

    fn fd_write_ready(&self, fd: usize) -> Result<bool, String> {
        let handle = &self.process()?.fds[fd];
        match handle {
            FdHandle::PipeWriter(buffer) => Ok(buffer.borrow().can_push_bytes(1)),
            FdHandle::Stdout
            | FdHandle::Stderr
            | FdHandle::File(_)
            | FdHandle::Counter(_)
            | FdHandle::EventCounter { .. }
            | FdHandle::MemoryObject { .. }
            | FdHandle::Timer(_)
            | FdHandle::TcpStream(_) => Ok(true),
            _ => Ok(false),
        }
    }

    fn fd_ready(&mut self, fd: usize) -> Result<bool, String> {
        if fd == MESSAGE_ENDPOINT_FD {
            return Ok(!self.process()?.inbox.is_empty());
        }
        let handle = &mut self.process_mut()?.fds[fd];
        match handle {
            FdHandle::Stdin
            | FdHandle::Stdout
            | FdHandle::Stderr
            | FdHandle::File(_)
            | FdHandle::Dir { .. }
            | FdHandle::Counter(_)
            | FdHandle::MemoryObject { .. }
            | FdHandle::CallGate { .. }
            | FdHandle::ClassifierTable(_)
            | FdHandle::ServiceletProgram(_)
            | FdHandle::TcpStream(_) => Ok(true),
            FdHandle::PipeWriter(buffer) => {
                let buffer = buffer.borrow();
                Ok(buffer.can_push_bytes(1) || buffer.can_push_capability())
            }
            FdHandle::EventCounter { value, .. } => Ok(*value.borrow() != 0),
            FdHandle::Timer(timer) => Ok(timer.borrow().expirations != 0),
            FdHandle::MessageEndpoint | FdHandle::TcpSocket { .. } => Ok(false),
            FdHandle::PipeReader(buffer) => {
                let buffer = buffer.borrow();
                Ok(!buffer.bytes.is_empty() || !buffer.capabilities.is_empty())
            }
            FdHandle::DmaBuffer { .. } | FdHandle::Closed => Ok(false),
            FdHandle::TcpListener { listener, pending } => {
                if pending.is_some() {
                    return Ok(true);
                }
                match listener.accept() {
                    Ok((stream, _)) => {
                        stream
                            .set_nonblocking(false)
                            .map_err(|err| format!("TCP accepted stream blocking failed: {err}"))?;
                        *pending = Some(stream);
                        Ok(true)
                    }
                    Err(err) if err.kind() == io::ErrorKind::WouldBlock => Ok(false),
                    Err(err) => Err(format!("TCP accept failed: {err}")),
                }
            }
        }
    }

    fn raise_current_signal(&mut self, signum: u64) -> Result<(), String> {
        let pid = self.thread()?.pid;
        self.queue_process_event(pid, NativeEvent::fault_signal(signum));
        Ok(())
    }

    fn deliver_signal_if_needed(&mut self) -> Result<(), String> {
        if !self.thread()?.signal_stack.is_empty() {
            return Ok(());
        }
        let pid = self.thread()?.pid;
        let signum = {
            let Some(process) = self.processes.get_mut(&pid) else {
                return Ok(());
            };
            let Some(pos) = process.pending_events.iter().position(|event| {
                matches!(
                    event,
                    NativeEvent::Signal {
                        source: EventSource::HardwareFault,
                        ..
                    }
                ) || event
                    .signal_number()
                    .is_some_and(|sig| process.sigmask & (1u64 << sig.min(63)) == 0)
            }) else {
                return Ok(());
            };
            process.pending_events.remove(pos)
        };
        let Some(event) = signum else {
            return Ok(());
        };
        let Some(signum) = event.signal_number() else {
            return Ok(());
        };
        match self.process()?.signal_handlers.get(&signum).copied() {
            Some(SignalDisposition::Ignore) => {}
            Some(SignalDisposition::Handler(handler)) => {
                let saved = {
                    let thread = self.thread()?;
                    SavedSignalContext {
                        ip: thread.ip,
                        regs: thread.regs,
                        flags: thread.flags,
                    }
                };
                let thread = self.thread_mut()?;
                thread.signal_stack.push(saved);
                thread.ip = handler;
            }
            None => {
                if signum != SIGCHLD {
                    self.exit_current(128 + signum as i32)?;
                }
            }
        }
        Ok(())
    }

    fn load_microcode(&mut self, blob: &[u8]) -> Result<(), String> {
        let text = String::from_utf8_lossy(blob);
        let mut updates = Vec::new();
        for line in text.lines() {
            let parts = line.split_whitespace().collect::<Vec<_>>();
            if parts.is_empty() {
                continue;
            }
            if parts.len() != 3 || !parts[0].eq_ignore_ascii_case("PORT") {
                return Err(format!("invalid microcode directive {line:?}"));
            }
            let port = parse_num(parts[1])?;
            let value = parse_num(parts[2])?;
            if value > 255 {
                return Err(format!("microcode port value out of range: {value}"));
            }
            updates.push((port, value as u8));
        }
        for (port, value) in updates {
            self.process_mut()?.ucode_ports.insert(port, value);
        }
        Ok(())
    }
}

fn align_up(value: u64, align: u64) -> u64 {
    (value + align - 1) & !(align - 1)
}

fn checked_align_up(value: u64, align: u64) -> Option<u64> {
    let mask = align.checked_sub(1)?;
    Some(value.checked_add(mask)? & !mask)
}

fn normalize_path_lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => out.push(Path::new("/")),
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(part) => out.push(part),
        }
    }
    if out.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        out
    }
}

fn range_within(base: u64, base_len: u64, addr: u64, len: usize) -> bool {
    let Some(base_end) = base.checked_add(base_len) else {
        return false;
    };
    let Some(end) = addr.checked_add(len as u64) else {
        return false;
    };
    addr >= base && end <= base_end
}

fn parse_num(text: &str) -> Result<u64, String> {
    if let Some(hex) = text.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).map_err(|_| format!("invalid number {text:?}"))
    } else {
        text.parse::<u64>()
            .map_err(|_| format!("invalid number {text:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestRng(u64);

    impl TestRng {
        fn new(seed: u64) -> Self {
            Self(seed)
        }

        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            self.0
        }

        fn below(&mut self, limit: u64) -> u64 {
            self.next() % limit
        }
    }

    fn empty_program() -> Program {
        Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap()
    }

    #[test]
    fn call_preserves_sp_and_ip_when_return_slot_unmapped() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.thread_mut().unwrap().ip = 7;
        machine.thread_mut().unwrap().regs[31] = 0;

        let err = machine.exec(Instr::Call(Target::Address(0))).unwrap_err();

        assert!(err.contains("unmapped address"), "{err}");
        assert_eq!(machine.thread().unwrap().regs[31], 0);
        assert_eq!(machine.thread().unwrap().ip, 7);

        machine.thread_mut().unwrap().ip = 9;
        machine.thread_mut().unwrap().regs[2] = 0;

        let err = machine.exec(Instr::CallReg(Reg(2))).unwrap_err();

        assert!(err.contains("unmapped address"), "{err}");
        assert_eq!(machine.thread().unwrap().regs[31], 0);
        assert_eq!(machine.thread().unwrap().ip, 9);
    }

    fn create_pipe_pair(machine: &mut Machine, read_fd: u64, write_fd: u64) -> (u64, u64) {
        let arg = ARG_BASE;
        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::Queue.code())
            .unwrap();
        machine
            .store_u64(arg + 16, ObjectProfile::Pipe.code())
            .unwrap();
        machine.store_u64(arg + 24, read_fd).unwrap();
        machine.store_u64(arg + 32, write_fd).unwrap();
        machine.store_u64(arg + 40, 0).unwrap();
        machine.object_ctl(Reg(2), arg).unwrap();
        (
            machine.fd_token(read_fd as usize).unwrap(),
            machine.fd_token(write_fd as usize).unwrap(),
        )
    }

    fn create_memory_source(machine: &mut Machine, fd: u64) -> u64 {
        let arg = ARG_BASE + 0x100;
        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::MemoryObject.code())
            .unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, fd).unwrap();
        machine.store_u64(arg + 40, 64).unwrap();
        machine.object_ctl(Reg(2), arg).unwrap();
        machine.fd_token(fd as usize).unwrap()
    }

    fn write_classifier_rule(
        machine: &mut Machine,
        base: u64,
        kind: u64,
        field: u64,
        value: u64,
        mask_or_end: u64,
        action: u64,
        action_arg: u64,
        hash_mod: u64,
    ) {
        machine.store_u64(base, kind).unwrap();
        machine.store_u64(base + 8, field).unwrap();
        machine.store_u64(base + 16, value).unwrap();
        machine.store_u64(base + 24, mask_or_end).unwrap();
        machine.store_u64(base + 32, action).unwrap();
        machine.store_u64(base + 40, action_arg).unwrap();
        machine.store_u64(base + 48, hash_mod).unwrap();
    }

    fn create_classifier(
        machine: &mut Machine,
        fd: u64,
        rules_ptr: u64,
        rule_count: u64,
        allowed_ptr: u64,
        allowed_count: u64,
    ) -> u64 {
        assert_ne!(
            try_create_classifier(
                machine,
                fd,
                rules_ptr,
                rule_count,
                allowed_ptr,
                allowed_count
            ),
            -1i64 as u64
        );
        machine.fd_token(fd as usize).unwrap()
    }

    fn try_create_classifier(
        machine: &mut Machine,
        fd: u64,
        rules_ptr: u64,
        rule_count: u64,
        allowed_ptr: u64,
        allowed_count: u64,
    ) -> u64 {
        let arg = ARG_BASE + 0x200;
        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::Classifier.code())
            .unwrap();
        machine
            .store_u64(arg + 16, ObjectProfile::ClassifierTable.code())
            .unwrap();
        machine.store_u64(arg + 24, fd).unwrap();
        machine.store_u64(arg + 40, rules_ptr).unwrap();
        machine.store_u64(arg + 48, rule_count).unwrap();
        machine.store_u64(arg + 56, allowed_ptr).unwrap();
        machine.store_u64(arg + 64, allowed_count).unwrap();
        machine.object_ctl(Reg(2), arg).unwrap();
        machine.thread().unwrap().regs[2]
    }

    fn write_servicelet_envelope(
        machine: &mut Machine,
        base: u64,
        program_len: u64,
        isa_subset: u64,
        instruction_limit: u64,
        cycle_limit: u64,
        record_read_limit: u64,
        action_write_limit: u64,
        flags: u64,
    ) {
        machine.store_u64(base, SERVICELET_VERIFY_VERSION).unwrap();
        machine.store_u64(base + 8, program_len).unwrap();
        machine.store_u64(base + 16, isa_subset).unwrap();
        machine.store_u64(base + 24, instruction_limit).unwrap();
        machine.store_u64(base + 32, cycle_limit).unwrap();
        machine.store_u64(base + 40, record_read_limit).unwrap();
        machine.store_u64(base + 48, action_write_limit).unwrap();
        machine.store_u64(base + 56, flags).unwrap();
        machine.store_u64(base + 64, ROOT_DOMAIN_ID).unwrap();
        machine
            .store_u64(base + 72, machine.domains[&ROOT_DOMAIN_ID].generation)
            .unwrap();
    }

    fn add_test_domain(machine: &mut Machine, id: u64, parent: u64) {
        machine.domains.insert(
            id,
            ResourceDomain {
                id,
                generation: 1,
                parent: Some(parent),
                children: Vec::new(),
                profile: 4,
                limits: DomainLimits::root(),
                capability_mask: u64::MAX,
                upcall_mask: u64::MAX,
                security: DomainSecurityPolicy::root(),
                frozen: false,
                destroyed: false,
                cpu_ticks: 0,
            },
        );
        machine.domains.get_mut(&parent).unwrap().children.push(id);
        machine.next_domain_id = machine.next_domain_id.max(id + 1);
    }

    fn try_create_servicelet(machine: &mut Machine, fd: u64, envelope: u64) -> u64 {
        let arg = ARG_BASE + 0x280;
        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::Servicelet.code())
            .unwrap();
        machine
            .store_u64(arg + 16, ObjectProfile::ServiceletProgram.code())
            .unwrap();
        machine.store_u64(arg + 24, fd).unwrap();
        machine.store_u64(arg + 40, envelope).unwrap();
        machine.object_ctl(Reg(2), arg).unwrap();
        machine.thread().unwrap().regs[2]
    }

    fn classify(machine: &mut Machine, classifier: u64, envelope: u64, result: u64) -> u64 {
        let arg = ARG_BASE + 0x300;
        machine.store_u64(arg, OBJECT_OP_CLASSIFY).unwrap();
        machine.store_u64(arg + 8, classifier).unwrap();
        machine.store_u64(arg + 16, envelope).unwrap();
        machine.store_u64(arg + 24, result).unwrap();
        machine.object_ctl(Reg(9), arg).unwrap();
        machine.thread().unwrap().regs[9]
    }

    fn write_envelope(
        machine: &mut Machine,
        base: u64,
        profile: u64,
        source: u64,
        record_ptr: u64,
        record_len: u64,
        inline0: u64,
        inline1: u64,
        inline2: u64,
    ) {
        let source_fd = machine.decode_fd_value(source).unwrap();
        let source_generation = machine.fd_generation(source_fd).unwrap();
        machine.store_u64(base, profile).unwrap();
        machine.store_u64(base + 8, source).unwrap();
        machine.store_u64(base + 16, source_generation).unwrap();
        machine.store_u64(base + 24, ROOT_DOMAIN_ID).unwrap();
        machine.store_u64(base + 32, record_ptr).unwrap();
        machine.store_u64(base + 40, record_len).unwrap();
        machine.store_u64(base + 48, inline0).unwrap();
        machine.store_u64(base + 56, inline1).unwrap();
        machine.store_u64(base + 64, inline2).unwrap();
    }

    fn ipv4_udp_packet(src: [u8; 4], dst: [u8; 4], src_port: u16, dst_port: u16) -> Vec<u8> {
        let mut bytes = vec![0u8; 14 + 20 + 8];
        bytes[12] = 0x08;
        bytes[13] = 0x00;
        let ip = 14;
        bytes[ip] = 0x45;
        bytes[ip + 2..ip + 4].copy_from_slice(&(28u16).to_be_bytes());
        bytes[ip + 9] = 17;
        bytes[ip + 12..ip + 16].copy_from_slice(&src);
        bytes[ip + 16..ip + 20].copy_from_slice(&dst);
        let udp = ip + 20;
        bytes[udp..udp + 2].copy_from_slice(&src_port.to_be_bytes());
        bytes[udp + 2..udp + 4].copy_from_slice(&dst_port.to_be_bytes());
        bytes
    }

    fn ipv6_udp_packet(payload_len: u16, src_port: u16, dst_port: u16) -> Vec<u8> {
        let mut bytes = vec![0u8; 14 + 40 + 8];
        bytes[12] = 0x86;
        bytes[13] = 0xdd;
        let ip = 14;
        bytes[ip] = 0x60;
        bytes[ip + 4..ip + 6].copy_from_slice(&payload_len.to_be_bytes());
        bytes[ip + 6] = 17;
        bytes[ip + 8..ip + 24]
            .copy_from_slice(&[0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1]);
        bytes[ip + 24..ip + 40]
            .copy_from_slice(&[0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 2]);
        let udp = ip + 40;
        bytes[udp..udp + 2].copy_from_slice(&src_port.to_be_bytes());
        bytes[udp + 2..udp + 4].copy_from_slice(&dst_port.to_be_bytes());
        bytes
    }

    fn query_classifier_counters(machine: &mut Machine, classifier: u64, out: u64) {
        let arg = ARG_BASE + 0x380;
        machine.store_u64(arg, OBJECT_OP_CLASSIFIER_QUERY).unwrap();
        machine.store_u64(arg + 8, classifier).unwrap();
        machine.store_u64(arg + 16, out).unwrap();
        machine.object_ctl(Reg(10), arg).unwrap();
    }

    #[test]
    fn memory_object_creation_rejects_zero_length_without_replacing_fd() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let arg = ARG_BASE;

        let retained = Rc::new(RefCell::new(77));
        {
            let process = machine.process_mut().unwrap();
            process.fds[5] = FdHandle::Counter(retained.clone());
            process.fd_capabilities[5] = FdCapability::full(5);
        }

        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::MemoryObject.code())
            .unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 5).unwrap();
        machine.store_u64(arg + 40, 0).unwrap();
        machine.object_ctl(Reg(2), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[2], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        match &machine.process().unwrap().fds[5] {
            FdHandle::Counter(value) => {
                assert!(Rc::ptr_eq(value, &retained));
                assert_eq!(*value.borrow(), 77);
            }
            _ => panic!("expected retained counter fd"),
        }
    }

    #[test]
    fn memory_object_creation_rejects_oversized_without_replacing_fd() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let arg = ARG_BASE;

        let retained = Rc::new(RefCell::new(77));
        {
            let process = machine.process_mut().unwrap();
            process.fds[5] = FdHandle::Counter(retained.clone());
            process.fd_capabilities[5] = FdCapability::full(5);
        }

        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::MemoryObject.code())
            .unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 5).unwrap();
        machine.store_u64(arg + 40, MEMORY_SIZE as u64 + 1).unwrap();
        machine.object_ctl(Reg(2), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[2], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 12);
        match &machine.process().unwrap().fds[5] {
            FdHandle::Counter(value) => {
                assert!(Rc::ptr_eq(value, &retained));
                assert_eq!(*value.borrow(), 77);
            }
            _ => panic!("expected retained counter fd"),
        }
    }

    #[test]
    fn memory_object_creation_counts_backing_storage_in_domain_usage() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let arg = ARG_BASE;
        let baseline = machine.domain_usage(ROOT_DOMAIN_ID);

        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::MemoryObject.code())
            .unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 5).unwrap();
        machine.store_u64(arg + 40, 64).unwrap();
        machine.object_ctl(Reg(2), arg).unwrap();

        assert_ne!(machine.thread().unwrap().regs[2], -1i64 as u64);
        assert_eq!(
            machine.domain_usage(ROOT_DOMAIN_ID).memory,
            baseline.memory + 64
        );

        machine.close_fd_index(5).unwrap();
        assert_eq!(machine.domain_usage(ROOT_DOMAIN_ID).memory, baseline.memory);
    }

    #[test]
    fn memory_object_write_rejects_growth_past_maximum_size() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let object = Rc::new(RefCell::new(vec![0; 8]));
        {
            let process = machine.process_mut().unwrap();
            process.fds[5] = FdHandle::MemoryObject {
                data: object.clone(),
                pos: MEMORY_SIZE - 1,
            };
            process.fd_capabilities[5] = FdCapability::full(5);
        }

        let payload = ARG_BASE + 0x100;
        machine.write_bytes(payload, b"xy").unwrap();
        machine.write_fd_index(5, payload, 2).unwrap();

        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 12);
        assert_eq!(object.borrow().len(), 8);
        match &machine.process().unwrap().fds[5] {
            FdHandle::MemoryObject { pos, .. } => assert_eq!(*pos, MEMORY_SIZE - 1),
            _ => panic!("expected memory object fd"),
        }
    }

    #[test]
    fn memory_object_write_growth_obeys_domain_budget() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let object = Rc::new(RefCell::new(vec![0; 8]));
        {
            let process = machine.process_mut().unwrap();
            process.fds[5] = FdHandle::MemoryObject {
                data: object.clone(),
                pos: 8,
            };
            process.fd_capabilities[5] = FdCapability::full(5);
        }
        let current_usage = machine.domain_usage(ROOT_DOMAIN_ID).memory;
        machine
            .domains
            .get_mut(&ROOT_DOMAIN_ID)
            .unwrap()
            .limits
            .memory = current_usage;

        let payload = ARG_BASE + 0x100;
        machine.write_bytes(payload, b"z").unwrap();
        machine.write_fd_index(5, payload, 1).unwrap();

        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 12);
        assert_eq!(object.borrow().len(), 8);
        match &machine.process().unwrap().fds[5] {
            FdHandle::MemoryObject { pos, .. } => assert_eq!(*pos, 8),
            _ => panic!("expected memory object fd"),
        }
    }

    #[test]
    fn zero_length_reads_do_not_consume_event_sources() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let event_counter = Rc::new(RefCell::new(3));
        let timer = Rc::new(RefCell::new(TimerState {
            remaining: 0,
            interval: 0,
            expirations: 2,
        }));
        {
            let process = machine.process_mut().unwrap();
            process.fds[3] = FdHandle::EventCounter {
                value: Rc::clone(&event_counter),
                semaphore: false,
            };
            process.fd_capabilities[3] = FdCapability::full(3);
            process.fds[4] = FdHandle::Timer(Rc::clone(&timer));
            process.fd_capabilities[4] = FdCapability::full(4);
        }

        assert_eq!(machine.read_fd_index(3, ARG_BASE, 0).unwrap(), Some(0));
        assert_eq!(*event_counter.borrow(), 3);
        assert_eq!(machine.read_fd_index(4, ARG_BASE, 0).unwrap(), Some(0));
        assert_eq!(timer.borrow().expirations, 2);

        assert_eq!(machine.read_fd_index(3, ARG_BASE, 8).unwrap(), Some(8));
        assert_eq!(*event_counter.borrow(), 0);
        assert_eq!(machine.read_fd_index(4, ARG_BASE, 8).unwrap(), Some(8));
        assert_eq!(timer.borrow().expirations, 0);
    }

    #[test]
    fn pipe_writer_rejects_full_byte_queue_and_reports_not_writable() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        create_pipe_pair(&mut machine, 3, 4);
        let queue = match &machine.process().unwrap().fds[4] {
            FdHandle::PipeWriter(queue) => Rc::clone(queue),
            _ => panic!("expected pipe writer"),
        };
        queue.borrow_mut().bytes = vec![0; PIPE_BUFFER_BYTE_LIMIT].into();

        assert_eq!(machine.poll_fd_index_mask_raw(4, POLLOUT_MASK).unwrap(), 0);
        let payload = ARG_BASE + 0x100;
        machine.write_bytes(payload, b"x").unwrap();
        machine.write_fd_index(4, payload, 1).unwrap();

        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 11);
        assert_eq!(queue.borrow().bytes.len(), PIPE_BUFFER_BYTE_LIMIT);
        queue.borrow_mut().bytes.pop_front();
        assert_eq!(
            machine.poll_fd_index_mask_raw(4, POLLOUT_MASK).unwrap(),
            POLLOUT_MASK
        );
    }

    #[test]
    fn pipe_read_wakes_writer_waiting_for_byte_queue_space() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        create_pipe_pair(&mut machine, 3, 4);
        let queue = match &machine.process().unwrap().fds[4] {
            FdHandle::PipeWriter(queue) => Rc::clone(queue),
            _ => panic!("expected pipe writer"),
        };
        queue.borrow_mut().bytes = vec![0x5a; PIPE_BUFFER_BYTE_LIMIT].into();
        machine
            .push_fd_waiter(4, POLLOUT_MASK, Some(Reg(8)))
            .unwrap();
        machine.ready.retain(|tid| *tid != 1);

        assert_eq!(machine.read_fd_index(3, ARG_BASE, 1).unwrap(), Some(1));

        assert!(machine.ready.contains(&1));
        assert!(machine.fd_waiters.is_empty());
        assert_eq!(machine.thread().unwrap().regs[8], 0);
        assert_eq!(machine.process().unwrap().errno, 0);
        assert_eq!(
            machine.poll_fd_index_mask_raw(4, POLLOUT_MASK).unwrap(),
            POLLOUT_MASK
        );
    }

    #[test]
    fn pipe_write_wakes_reader_waiting_for_byte_payload() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        create_pipe_pair(&mut machine, 3, 4);
        machine
            .push_fd_waiter(3, POLLIN_MASK, Some(Reg(8)))
            .unwrap();
        machine.ready.retain(|tid| *tid != 1);

        let payload = ARG_BASE + 0x100;
        machine.write_bytes(payload, b"x").unwrap();
        machine.write_fd_index(4, payload, 1).unwrap();

        assert!(machine.ready.contains(&1));
        assert!(machine.fd_waiters.is_empty());
        assert_eq!(machine.thread().unwrap().regs[1], 1);
        assert_eq!(machine.thread().unwrap().regs[8], 0);
        assert_eq!(machine.process().unwrap().errno, 0);
        assert_eq!(
            machine.poll_fd_index_mask_raw(3, POLLIN_MASK).unwrap(),
            POLLIN_MASK
        );
    }

    #[test]
    fn event_counter_write_wakes_reader_waiting_for_nonzero_value() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let event_counter = Rc::new(RefCell::new(0));
        {
            let process = machine.process_mut().unwrap();
            process.fds[3] = FdHandle::EventCounter {
                value: Rc::clone(&event_counter),
                semaphore: false,
            };
            process.fd_capabilities[3] = FdCapability::full(3);
        }
        machine
            .push_fd_waiter(3, POLLIN_MASK, Some(Reg(8)))
            .unwrap();
        machine.ready.retain(|tid| *tid != 1);

        let payload = ARG_BASE + 0x100;
        machine.store_u64(payload, 2).unwrap();
        machine.write_fd_index(3, payload, 8).unwrap();

        assert_eq!(*event_counter.borrow(), 2);
        assert!(machine.ready.contains(&1));
        assert!(machine.fd_waiters.is_empty());
        assert_eq!(machine.thread().unwrap().regs[1], 8);
        assert_eq!(machine.thread().unwrap().regs[8], 0);
        assert_eq!(machine.process().unwrap().errno, 0);
        assert_eq!(
            machine.poll_fd_index_mask_raw(3, POLLIN_MASK).unwrap(),
            POLLIN_MASK
        );
    }

    #[test]
    fn timer_expiration_wakes_reader_waiting_for_tick() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let timer = Rc::new(RefCell::new(TimerState {
            remaining: 1,
            interval: 0,
            expirations: 0,
        }));
        {
            let process = machine.process_mut().unwrap();
            process.fds[3] = FdHandle::Timer(Rc::clone(&timer));
            process.fd_capabilities[3] = FdCapability::full(3);
        }
        machine
            .push_fd_waiter(3, POLLIN_MASK, Some(Reg(8)))
            .unwrap();
        machine.ready.retain(|tid| *tid != 1);

        machine.tick_timers();
        machine.poll_fd_waiters();

        assert_eq!(timer.borrow().expirations, 1);
        assert!(machine.ready.contains(&1));
        assert!(machine.fd_waiters.is_empty());
        assert_eq!(machine.thread().unwrap().regs[8], 0);
        assert_eq!(machine.process().unwrap().errno, 0);
        assert_eq!(
            machine.poll_fd_index_mask_raw(3, POLLIN_MASK).unwrap(),
            POLLIN_MASK
        );
    }

    #[test]
    fn push_rejects_locked_result_register_before_queue_write() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        create_pipe_pair(&mut machine, 3, 4);
        let payload = ARG_BASE + 0x100;
        machine.write_bytes(payload, b"x").unwrap();
        machine.thread_mut().unwrap().regs[2] = payload;
        machine.thread_mut().unwrap().regs[3] = 1;

        let err = machine
            .exec(Instr::Push(Reg(31), FdReg(4), Reg(2), Reg(3)))
            .unwrap_err();

        assert!(err.contains("hardware-locked stack pointer"), "{err}");
        assert!(!machine.fd_read_ready(3).unwrap());
    }

    #[test]
    fn cap_send_rejects_full_capability_queue_without_moving_source() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        create_pipe_pair(&mut machine, 3, 4);
        let queue = match &machine.process().unwrap().fds[4] {
            FdHandle::PipeWriter(queue) => Rc::clone(queue),
            _ => panic!("expected pipe writer"),
        };
        {
            let mut queue = queue.borrow_mut();
            for idx in 0..PIPE_CAPABILITY_LIMIT {
                queue.capabilities.push_back(CapabilityPayload {
                    handle: FdHandle::Counter(Rc::new(RefCell::new(idx as u64))),
                    capability: FdCapability::full(1000 + idx as u64),
                });
            }
        }
        {
            let process = machine.process_mut().unwrap();
            process.fds[5] = FdHandle::Counter(Rc::new(RefCell::new(99)));
            process.fd_capabilities[5] = FdCapability::full(5);
        }

        let arg = ARG_BASE;
        machine.store_u64(arg, 4).unwrap();
        machine.store_u64(arg + 8, 5).unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, CAP_SEND_FLAG_MOVE).unwrap();
        machine.cap_send(Reg(6), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[6], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 11);
        assert_eq!(queue.borrow().capabilities.len(), PIPE_CAPABILITY_LIMIT);
        assert!(matches!(
            machine.process().unwrap().fds[5],
            FdHandle::Counter(_)
        ));
    }

    #[test]
    fn cap_send_rejects_locked_result_before_queue_or_errno_side_effects() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        create_pipe_pair(&mut machine, 3, 4);
        {
            let process = machine.process_mut().unwrap();
            process.fds[5] = FdHandle::Counter(Rc::new(RefCell::new(99)));
            process.fd_capabilities[5] = FdCapability::full(5);
        }
        machine.set_errno(123).unwrap();
        let arg = ARG_BASE;
        machine.store_u64(arg, 4).unwrap();
        machine.store_u64(arg + 8, 5).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();

        let err = machine.cap_send(Reg(31), arg).unwrap_err();

        assert!(err.contains("stack pointer"), "{err}");
        assert_eq!(machine.process().unwrap().errno, 123);
        let queue = match &machine.process().unwrap().fds[4] {
            FdHandle::PipeWriter(queue) => Rc::clone(queue),
            _ => panic!("expected pipe writer"),
        };
        assert_eq!(queue.borrow().capabilities.len(), 0);
    }

    #[test]
    fn cap_recv_rejects_locked_result_before_dequeue_or_errno_side_effects() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        create_pipe_pair(&mut machine, 3, 4);
        {
            let process = machine.process_mut().unwrap();
            process.fds[5] = FdHandle::Counter(Rc::new(RefCell::new(99)));
            process.fd_capabilities[5] = FdCapability::full(5);
        }
        let arg = ARG_BASE;
        machine.store_u64(arg, 4).unwrap();
        machine.store_u64(arg + 8, 5).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_send(Reg(6), arg).unwrap();
        machine.set_errno(123).unwrap();
        machine.store_u64(arg, 3).unwrap();
        machine.store_u64(arg + 8, 7).unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();

        let err = machine.cap_recv(Reg(31), arg).unwrap_err();

        assert!(err.contains("stack pointer"), "{err}");
        assert_eq!(machine.process().unwrap().errno, 123);
        assert!(matches!(
            machine.process().unwrap().fds[7],
            FdHandle::Closed
        ));
        let queue = match &machine.process().unwrap().fds[3] {
            FdHandle::PipeReader(queue) => Rc::clone(queue),
            _ => panic!("expected pipe reader"),
        };
        assert_eq!(queue.borrow().capabilities.len(), 1);
    }

    #[test]
    fn cap_transfer_rejects_unknown_flags_without_queue_mutation() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        create_pipe_pair(&mut machine, 3, 4);
        {
            let process = machine.process_mut().unwrap();
            process.fds[5] = FdHandle::Counter(Rc::new(RefCell::new(99)));
            process.fd_capabilities[5] = FdCapability::full(5);
        }
        let arg = ARG_BASE;
        machine.store_u64(arg, 4).unwrap();
        machine.store_u64(arg + 8, 5).unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 1 << 9).unwrap();
        machine.cap_send(Reg(6), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[6], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        let queue = match &machine.process().unwrap().fds[4] {
            FdHandle::PipeWriter(queue) => Rc::clone(queue),
            _ => panic!("expected pipe writer"),
        };
        assert!(queue.borrow().capabilities.is_empty());
        assert!(matches!(
            machine.process().unwrap().fds[5],
            FdHandle::Counter(_)
        ));

        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_send(Reg(7), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[7], 1);
        assert_eq!(queue.borrow().capabilities.len(), 1);

        machine.store_u64(arg, 3).unwrap();
        machine.store_u64(arg + 8, 7).unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 1 << 9).unwrap();
        machine.cap_recv(Reg(8), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[8], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        assert_eq!(queue.borrow().capabilities.len(), 1);
        assert!(matches!(
            machine.process().unwrap().fds[7],
            FdHandle::Closed
        ));
    }

    #[test]
    fn object_create_rejects_cross_kind_profiles_without_installing_fd() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let arg = ARG_BASE;

        let invalid_cases = [
            (ObjectKind::Counter, ObjectProfile::TcpStream),
            (ObjectKind::MemoryObject, ObjectProfile::TcpStream),
            (ObjectKind::Timer, ObjectProfile::TcpStream),
        ];
        for (kind, profile) in invalid_cases {
            machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
            machine.store_u64(arg + 8, kind.code()).unwrap();
            machine.store_u64(arg + 16, profile.code()).unwrap();
            machine.store_u64(arg + 24, 7).unwrap();
            machine.store_u64(arg + 40, 64).unwrap();
            machine.object_ctl(Reg(2), arg).unwrap();

            assert_eq!(machine.thread().unwrap().regs[2], -1i64 as u64);
            assert_eq!(machine.process().unwrap().errno, 22);
            assert!(matches!(
                machine.process().unwrap().fds[7],
                FdHandle::Closed
            ));
        }

        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::Endpoint.code())
            .unwrap();
        machine
            .store_u64(arg + 16, ObjectProfile::TcpStream.code())
            .unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine.store_u64(arg + 40, SOCKET_AF_INET).unwrap();
        machine.store_u64(arg + 48, SOCKET_TYPE_STREAM).unwrap();
        machine.store_u64(arg + 56, 0).unwrap();
        machine.object_ctl(Reg(3), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[3], 7);
    }

    #[test]
    fn object_ctl_rejects_locked_result_register_before_create_side_effects() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let arg = ARG_BASE;

        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::Counter.code())
            .unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine.store_u64(arg + 40, 123).unwrap();

        let err = machine.object_ctl(Reg(31), arg).unwrap_err();

        assert!(err.contains("hardware-locked stack pointer"), "{err}");
        assert_eq!(machine.load_u64(arg + 24).unwrap(), 7);
        assert!(matches!(
            machine.process().unwrap().fds[7],
            FdHandle::Closed
        ));
    }

    #[test]
    fn object_ctl_create_prevalidates_result_slots_before_installing_fd() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let arg = ARG_BASE;

        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::Counter.code())
            .unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine.store_u64(arg + 40, 99).unwrap();
        let original_generation = machine.fd_generation(7).unwrap();
        {
            let process = machine.process_mut().unwrap();
            let vma = process
                .vmas
                .iter_mut()
                .find(|vma| vma.contains(arg, 48))
                .expect("argblock VMA");
            vma.prot = 0b01;
        }

        machine.object_ctl(Reg(5), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[5], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 14);
        assert!(matches!(
            machine.process().unwrap().fds[7],
            FdHandle::Closed
        ));
        assert_eq!(machine.fd_generation(7).unwrap(), original_generation);
    }

    #[test]
    fn pipe_create_prevalidates_fd_pair_before_installing_either_end() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let arg = ARG_BASE;

        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::Queue.code())
            .unwrap();
        machine
            .store_u64(arg + 16, ObjectProfile::Pipe.code())
            .unwrap();
        machine.store_u64(arg + 24, 5).unwrap();
        machine
            .store_u64(arg + 32, MESSAGE_ENDPOINT_FD as u64)
            .unwrap();
        machine.store_u64(arg + 40, 0).unwrap();
        machine.object_ctl(Reg(2), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[2], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 9);
        assert!(matches!(
            machine.process().unwrap().fds[5],
            FdHandle::Closed
        ));

        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::Queue.code())
            .unwrap();
        machine
            .store_u64(arg + 16, ObjectProfile::Pipe.code())
            .unwrap();
        machine.store_u64(arg + 24, 5).unwrap();
        machine.store_u64(arg + 32, 5).unwrap();
        machine.store_u64(arg + 40, 0).unwrap();
        machine.object_ctl(Reg(3), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[3], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        assert!(matches!(
            machine.process().unwrap().fds[5],
            FdHandle::Closed
        ));
    }

    #[test]
    fn pipe_create_prevalidates_combined_fdr_budget_before_installing() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let arg = ARG_BASE;
        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::Queue.code())
            .unwrap();
        machine
            .store_u64(arg + 16, ObjectProfile::Pipe.code())
            .unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.store_u64(arg + 32, 0).unwrap();
        machine.store_u64(arg + 40, 0).unwrap();
        let usage = machine.domain_usage(ROOT_DOMAIN_ID);
        machine
            .domains
            .get_mut(&ROOT_DOMAIN_ID)
            .unwrap()
            .limits
            .fdrs = usage.fdrs + 1;

        machine.object_ctl(Reg(2), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[2], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 12);
        assert!(matches!(
            machine.process().unwrap().fds[3],
            FdHandle::Closed
        ));
        assert!(matches!(
            machine.process().unwrap().fds[4],
            FdHandle::Closed
        ));
    }

    #[test]
    fn msg_send_reports_missing_target_and_full_inbox() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[2] = 99;
        machine.thread_mut().unwrap().regs[3] = 10;
        machine.thread_mut().unwrap().regs[4] = 20;

        machine
            .exec(Instr::MsgSend(Reg(2), Reg(3), Reg(4)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 3);
        assert!(machine.process().unwrap().inbox.is_empty());

        for _ in 0..PROCESS_INBOX_LIMIT {
            machine.process_mut().unwrap().inbox.push_back((1, 2));
        }
        machine.thread_mut().unwrap().regs[2] = 1;
        machine
            .exec(Instr::MsgSend(Reg(2), Reg(3), Reg(4)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 11);
        assert_eq!(machine.process().unwrap().inbox.len(), PROCESS_INBOX_LIMIT);

        machine.process_mut().unwrap().inbox.clear();
        machine.set_errno(123).unwrap();
        machine
            .exec(Instr::MsgSend(Reg(2), Reg(3), Reg(4)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[1], 0);
        assert_eq!(machine.process().unwrap().errno, 0);
        assert_eq!(machine.process().unwrap().inbox.front(), Some(&(10, 20)));
    }

    #[test]
    fn message_endpoint_pull_success_clears_errno_and_sets_secondary_result() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.process_mut().unwrap().inbox.push_back((10, 20));
        machine.set_errno(123).unwrap();

        let keep_ready = machine
            .exec(Instr::Pull(
                Reg(5),
                FdReg(MESSAGE_ENDPOINT_FD),
                Reg(0),
                Reg(0),
            ))
            .unwrap();

        assert!(keep_ready);
        assert_eq!(machine.thread().unwrap().regs[5], 10);
        assert_eq!(machine.thread().unwrap().regs[30], 20);
        assert_eq!(machine.process().unwrap().errno, 0);
        assert!(machine.process().unwrap().inbox.is_empty());
    }

    #[test]
    fn classifier_routes_ipc_record_by_service_id_and_wakes_queue() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let (_reader_token, writer_token) = create_pipe_pair(&mut machine, 3, 4);
        let source = create_memory_source(&mut machine, 5);
        let rules = ARG_BASE + 0x1000;
        let allowed = ARG_BASE + 0x1800;
        let payload = ARG_BASE + 0x1900;
        let envelope = ARG_BASE + 0x1a00;
        let result = ARG_BASE + 0x1b00;
        machine.write_bytes(payload, b"ipc").unwrap();
        machine.store_u64(allowed, writer_token).unwrap();
        write_classifier_rule(
            &mut machine,
            rules,
            CLASSIFY_RULE_EXACT,
            CLASSIFY_FIELD_SERVICE_ID,
            42,
            0,
            CLASSIFY_ACTION_ROUTE,
            writer_token,
            0,
        );
        let classifier = create_classifier(&mut machine, 6, rules, 1, allowed, 1);

        write_envelope(
            &mut machine,
            envelope,
            CLASSIFY_PROFILE_IPC,
            source,
            payload,
            3,
            42,
            0,
            0,
        );
        machine.push_fd_waiter(3, POLLIN_MASK, None).unwrap();
        let current_tid = machine.current_tid;
        machine.ready.retain(|tid| *tid != current_tid);
        assert!(!machine.ready.contains(&current_tid));

        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            CLASSIFY_ACTION_ROUTE
        );
        assert_eq!(machine.load_u64(result).unwrap(), CLASSIFY_ACTION_ROUTE);
        assert!(machine.fd_waiters.is_empty());
        assert!(machine.ready.contains(&current_tid));
        assert!(machine.fd_read_ready(3).unwrap());
        let mut out = [0u8; 3];
        assert_eq!(
            machine.read_fd_index(3, ARG_BASE + 0x1c00, 3).unwrap(),
            Some(3)
        );
        out.copy_from_slice(&machine.read_bytes(ARG_BASE + 0x1c00, 3).unwrap());
        assert_eq!(&out, b"ipc");
    }

    #[test]
    fn classifier_rejects_oversized_routed_records_before_queueing() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let (_reader_token, writer_token) = create_pipe_pair(&mut machine, 3, 4);
        let source = create_memory_source(&mut machine, 5);
        let rules = ARG_BASE + 0x1000;
        let allowed = ARG_BASE + 0x1800;
        let payload = ARG_BASE + 0x3000;
        let envelope = ARG_BASE + 0x1a00;
        let result = ARG_BASE + 0x1b00;
        let oversized = vec![0x5au8; CLASSIFIER_MAX_ROUTE_BYTES + 1];
        machine.write_bytes(payload, &oversized).unwrap();
        machine.store_u64(allowed, writer_token).unwrap();
        write_classifier_rule(
            &mut machine,
            rules,
            CLASSIFY_RULE_EXACT,
            CLASSIFY_FIELD_SERVICE_ID,
            42,
            0,
            CLASSIFY_ACTION_ROUTE,
            writer_token,
            0,
        );
        let classifier = create_classifier(&mut machine, 6, rules, 1, allowed, 1);
        write_envelope(
            &mut machine,
            envelope,
            CLASSIFY_PROFILE_IPC,
            source,
            payload,
            oversized.len() as u64,
            42,
            0,
            0,
        );

        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 75);
        assert!(!machine.fd_read_ready(3).unwrap());
    }

    #[test]
    fn classifier_rejects_full_destination_queue_without_success_counters() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let (_reader_token, writer_token) = create_pipe_pair(&mut machine, 3, 4);
        let queue = match &machine.process().unwrap().fds[4] {
            FdHandle::PipeWriter(queue) => Rc::clone(queue),
            _ => panic!("expected pipe writer"),
        };
        queue.borrow_mut().bytes = vec![0; PIPE_BUFFER_BYTE_LIMIT].into();
        let source = create_memory_source(&mut machine, 5);
        let rules = ARG_BASE + 0x1000;
        let allowed = ARG_BASE + 0x1800;
        let payload = ARG_BASE + 0x1900;
        let envelope = ARG_BASE + 0x1a00;
        let result = ARG_BASE + 0x1b00;
        let counters = ARG_BASE + 0x1c00;
        machine.write_bytes(payload, b"x").unwrap();
        machine.store_u64(allowed, writer_token).unwrap();
        for offset in [0, 8, 16, 24] {
            machine.store_u64(result + offset, 0xfeed_face).unwrap();
        }
        write_classifier_rule(
            &mut machine,
            rules,
            CLASSIFY_RULE_EXACT,
            CLASSIFY_FIELD_SERVICE_ID,
            42,
            0,
            CLASSIFY_ACTION_ROUTE,
            writer_token,
            0,
        );
        let classifier = create_classifier(&mut machine, 6, rules, 1, allowed, 1);
        write_envelope(
            &mut machine,
            envelope,
            CLASSIFY_PROFILE_IPC,
            source,
            payload,
            1,
            42,
            0,
            0,
        );

        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 11);
        assert_eq!(queue.borrow().bytes.len(), PIPE_BUFFER_BYTE_LIMIT);
        for offset in [0, 8, 16, 24] {
            assert_eq!(machine.load_u64(result + offset).unwrap(), 0xfeed_face);
        }
        query_classifier_counters(&mut machine, classifier, counters);
        assert_eq!(machine.load_u64(counters).unwrap(), 0);
        assert_eq!(machine.load_u64(counters + 16).unwrap(), 0);
    }

    #[test]
    fn classifier_routes_packets_by_port_subnet_and_hash() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let (_reader_token, writer_token) = create_pipe_pair(&mut machine, 3, 4);
        let source = create_memory_source(&mut machine, 5);
        let allowed = ARG_BASE + 0x1000;
        machine.store_u64(allowed, writer_token).unwrap();
        let packet = ipv4_udp_packet([10, 1, 2, 3], [192, 168, 1, 44], 1000, 8080);
        let packet_ptr = ARG_BASE + 0x1800;
        let envelope = ARG_BASE + 0x1a00;
        let result = ARG_BASE + 0x1b00;
        machine.write_bytes(packet_ptr, &packet).unwrap();
        write_envelope(
            &mut machine,
            envelope,
            CLASSIFY_PROFILE_PACKET,
            source,
            packet_ptr,
            packet.len() as u64,
            0,
            0,
            0,
        );

        let rule_sets = [
            (
                CLASSIFY_RULE_EXACT,
                CLASSIFY_FIELD_DST_PORT,
                8080,
                0,
                0,
                ARG_BASE + 0x2000,
            ),
            (
                CLASSIFY_RULE_MASKED,
                CLASSIFY_FIELD_DST_IPV4,
                0xc0a8_0100,
                0xffff_ff00,
                0,
                ARG_BASE + 0x2100,
            ),
            (
                CLASSIFY_RULE_HASH,
                CLASSIFY_FIELD_HASH,
                ((0x0a01_0203u64 ^ 0xc0a8_012cu64 ^ 17 ^ 1000 ^ 8080) % 4),
                0,
                4,
                ARG_BASE + 0x2200,
            ),
            (
                CLASSIFY_RULE_RANGE,
                CLASSIFY_FIELD_DST_PORT,
                8000,
                9000,
                0,
                ARG_BASE + 0x2300,
            ),
        ];
        for (kind, field, value, mask, hash_mod, rules) in rule_sets {
            write_classifier_rule(
                &mut machine,
                rules,
                kind,
                field,
                value,
                mask,
                CLASSIFY_ACTION_ROUTE,
                writer_token,
                hash_mod,
            );
            let classifier = create_classifier(&mut machine, 6, rules, 1, allowed, 1);
            assert_eq!(
                classify(&mut machine, classifier, envelope, result),
                CLASSIFY_ACTION_ROUTE
            );
            machine.close_fd_index(6).unwrap();
        }
    }

    #[test]
    fn classifier_ipv6_zero_payload_length_needs_software_without_routing() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let (_reader_token, writer_token) = create_pipe_pair(&mut machine, 3, 4);
        let source = create_memory_source(&mut machine, 5);
        let allowed = ARG_BASE + 0x1000;
        let rules = ARG_BASE + 0x1100;
        let packet_ptr = ARG_BASE + 0x1800;
        let envelope = ARG_BASE + 0x1a00;
        let result = ARG_BASE + 0x1b00;
        let counters = ARG_BASE + 0x1c00;
        machine.store_u64(allowed, writer_token).unwrap();
        write_classifier_rule(
            &mut machine,
            rules,
            CLASSIFY_RULE_EXACT,
            CLASSIFY_FIELD_DST_PORT,
            8080,
            0,
            CLASSIFY_ACTION_ROUTE,
            writer_token,
            0,
        );
        let classifier = create_classifier(&mut machine, 6, rules, 1, allowed, 1);
        let packet = ipv6_udp_packet(0, 1000, 8080);
        machine.write_bytes(packet_ptr, &packet).unwrap();
        write_envelope(
            &mut machine,
            envelope,
            CLASSIFY_PROFILE_PACKET,
            source,
            packet_ptr,
            packet.len() as u64,
            0,
            0,
            0,
        );

        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            CLASSIFY_ACTION_NEEDS_SOFTWARE
        );
        assert_eq!(
            machine.load_u64(result).unwrap(),
            CLASSIFY_ACTION_NEEDS_SOFTWARE
        );
        assert!(!machine.fd_read_ready(3).unwrap());
        query_classifier_counters(&mut machine, classifier, counters);
        assert_eq!(machine.load_u64(counters + 16).unwrap(), 0);
        assert_eq!(machine.load_u64(counters + 32).unwrap(), 1);
    }

    #[test]
    fn classifier_ipv4_zero_total_length_needs_software_without_routing() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let (_reader_token, writer_token) = create_pipe_pair(&mut machine, 3, 4);
        let source = create_memory_source(&mut machine, 5);
        let allowed = ARG_BASE + 0x1000;
        let rules = ARG_BASE + 0x1100;
        let packet_ptr = ARG_BASE + 0x1800;
        let envelope = ARG_BASE + 0x1a00;
        let result = ARG_BASE + 0x1b00;
        let counters = ARG_BASE + 0x1c00;
        machine.store_u64(allowed, writer_token).unwrap();
        write_classifier_rule(
            &mut machine,
            rules,
            CLASSIFY_RULE_EXACT,
            CLASSIFY_FIELD_DST_PORT,
            8080,
            0,
            CLASSIFY_ACTION_ROUTE,
            writer_token,
            0,
        );
        let classifier = create_classifier(&mut machine, 6, rules, 1, allowed, 1);
        let mut packet = ipv4_udp_packet([10, 1, 2, 3], [192, 168, 1, 44], 1000, 8080);
        packet[14 + 2] = 0;
        packet[14 + 3] = 0;
        machine.write_bytes(packet_ptr, &packet).unwrap();
        write_envelope(
            &mut machine,
            envelope,
            CLASSIFY_PROFILE_PACKET,
            source,
            packet_ptr,
            packet.len() as u64,
            0,
            0,
            0,
        );

        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            CLASSIFY_ACTION_NEEDS_SOFTWARE
        );
        assert!(!machine.fd_read_ready(3).unwrap());
        query_classifier_counters(&mut machine, classifier, counters);
        assert_eq!(machine.load_u64(counters + 16).unwrap(), 0);
        assert_eq!(machine.load_u64(counters + 32).unwrap(), 1);
    }

    #[test]
    fn classifier_oversized_packet_records_need_software_without_routing() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let (_reader_token, writer_token) = create_pipe_pair(&mut machine, 3, 4);
        let source = create_memory_source(&mut machine, 5);
        let allowed = ARG_BASE + 0x1000;
        let rules = ARG_BASE + 0x1100;
        let packet_ptr = ARG_BASE + 0x1800;
        let envelope = ARG_BASE + 0x1a00;
        let result = ARG_BASE + 0x1b00;
        let counters = ARG_BASE + 0x1c00;
        machine.store_u64(allowed, writer_token).unwrap();
        write_classifier_rule(
            &mut machine,
            rules,
            CLASSIFY_RULE_EXACT,
            CLASSIFY_FIELD_DST_PORT,
            8080,
            0,
            CLASSIFY_ACTION_ROUTE,
            writer_token,
            0,
        );
        let classifier = create_classifier(&mut machine, 6, rules, 1, allowed, 1);
        let packet = ipv4_udp_packet([10, 1, 2, 3], [192, 168, 1, 44], 1000, 8080);
        machine.write_bytes(packet_ptr, &packet).unwrap();
        write_envelope(
            &mut machine,
            envelope,
            CLASSIFY_PROFILE_PACKET,
            source,
            packet_ptr,
            CLASSIFIER_MAX_ROUTE_BYTES as u64 + 1,
            0,
            0,
            0,
        );

        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            CLASSIFY_ACTION_NEEDS_SOFTWARE
        );
        assert!(!machine.fd_read_ready(3).unwrap());
        query_classifier_counters(&mut machine, classifier, counters);
        assert_eq!(machine.load_u64(counters + 16).unwrap(), 0);
        assert_eq!(machine.load_u64(counters + 32).unwrap(), 1);
    }

    #[test]
    fn classifier_supports_mark_and_count_actions_without_queue_authority() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let source = create_memory_source(&mut machine, 5);
        let rules = ARG_BASE + 0x1000;
        let envelope = ARG_BASE + 0x1a00;
        let result = ARG_BASE + 0x1b00;
        let counters = ARG_BASE + 0x1c00;
        write_classifier_rule(
            &mut machine,
            rules,
            CLASSIFY_RULE_EXACT,
            CLASSIFY_FIELD_SERVICE_ID,
            55,
            0,
            CLASSIFY_ACTION_MARK,
            0xabcd,
            0,
        );
        write_classifier_rule(
            &mut machine,
            rules + CLASSIFIER_RULE_SIZE,
            CLASSIFY_RULE_EXACT,
            CLASSIFY_FIELD_SERVICE_ID,
            77,
            0,
            CLASSIFY_ACTION_COUNT,
            0,
            0,
        );
        let classifier = create_classifier(&mut machine, 6, rules, 2, 0, 0);

        write_envelope(
            &mut machine,
            envelope,
            CLASSIFY_PROFILE_IPC,
            source,
            0,
            0,
            55,
            0,
            0,
        );
        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            CLASSIFY_ACTION_MARK
        );
        assert_eq!(machine.load_u64(result + 8).unwrap(), 0xabcd);

        write_envelope(
            &mut machine,
            envelope,
            CLASSIFY_PROFILE_IPC,
            source,
            0,
            0,
            77,
            0,
            0,
        );
        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            CLASSIFY_ACTION_COUNT
        );
        query_classifier_counters(&mut machine, classifier, counters);
        assert_eq!(machine.load_u64(counters).unwrap(), 2);
        assert_eq!(machine.load_u64(counters + 8).unwrap(), 0);
        assert_eq!(machine.load_u64(counters + 16).unwrap(), 0);
        assert_eq!(machine.load_u64(counters + 24).unwrap(), 0);
        assert_eq!(machine.load_u64(counters + 32).unwrap(), 0);
    }

    #[test]
    fn classifier_routes_generic_event_records_with_inline_fields() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let (_reader_token, writer_token) = create_pipe_pair(&mut machine, 3, 4);
        let source = create_memory_source(&mut machine, 5);
        let rules = ARG_BASE + 0x1000;
        let allowed = ARG_BASE + 0x1800;
        let payload = ARG_BASE + 0x1900;
        let envelope = ARG_BASE + 0x1a00;
        let result = ARG_BASE + 0x1b00;
        machine.write_bytes(payload, b"evt").unwrap();
        machine.store_u64(allowed, writer_token).unwrap();
        write_classifier_rule(
            &mut machine,
            rules,
            CLASSIFY_RULE_EXACT,
            CLASSIFY_FIELD_DST_PORT,
            99,
            0,
            CLASSIFY_ACTION_DROP,
            0,
            0,
        );
        write_classifier_rule(
            &mut machine,
            rules + CLASSIFIER_RULE_SIZE,
            CLASSIFY_RULE_EXACT,
            CLASSIFY_FIELD_INLINE1,
            99,
            0,
            CLASSIFY_ACTION_ROUTE,
            writer_token,
            0,
        );
        let classifier = create_classifier(&mut machine, 6, rules, 2, allowed, 1);

        write_envelope(
            &mut machine,
            envelope,
            CLASSIFY_PROFILE_EVENT,
            source,
            payload,
            3,
            0,
            99,
            0,
        );

        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            CLASSIFY_ACTION_ROUTE
        );
        assert_eq!(machine.load_u64(result).unwrap(), CLASSIFY_ACTION_ROUTE);
        assert_eq!(machine.load_u64(result + 24).unwrap(), 1);
        assert!(machine.fd_read_ready(3).unwrap());
        assert_eq!(
            machine.read_fd_index(3, ARG_BASE + 0x1c00, 3).unwrap(),
            Some(3)
        );
        assert_eq!(
            machine.read_bytes(ARG_BASE + 0x1c00, 3).unwrap(),
            b"evt".to_vec()
        );
    }

    #[test]
    fn classifier_fallback_malformed_and_drop_counters_are_reported() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let (_reader_token, writer_token) = create_pipe_pair(&mut machine, 3, 4);
        let source = create_memory_source(&mut machine, 5);
        let rules = ARG_BASE + 0x1000;
        let allowed = ARG_BASE + 0x1800;
        let envelope = ARG_BASE + 0x1a00;
        let result = ARG_BASE + 0x1b00;
        let counters = ARG_BASE + 0x1c00;
        let packet_ptr = ARG_BASE + 0x1d00;
        machine.store_u64(allowed, writer_token).unwrap();
        write_classifier_rule(
            &mut machine,
            rules,
            CLASSIFY_RULE_EXACT,
            CLASSIFY_FIELD_SERVICE_ID,
            7,
            0,
            CLASSIFY_ACTION_DROP,
            0,
            0,
        );
        let classifier = create_classifier(&mut machine, 6, rules, 1, allowed, 1);

        write_envelope(
            &mut machine,
            envelope,
            CLASSIFY_PROFILE_IPC,
            source,
            0,
            0,
            7,
            0,
            0,
        );
        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            CLASSIFY_ACTION_DROP
        );

        write_envelope(
            &mut machine,
            envelope,
            CLASSIFY_PROFILE_IPC,
            source,
            0,
            0,
            99,
            0,
            0,
        );
        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            CLASSIFY_ACTION_NEEDS_SOFTWARE
        );

        machine.write_bytes(packet_ptr, &[1, 2, 3]).unwrap();
        write_envelope(
            &mut machine,
            envelope,
            CLASSIFY_PROFILE_PACKET,
            source,
            packet_ptr,
            3,
            0,
            0,
            0,
        );
        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            CLASSIFY_ACTION_NEEDS_SOFTWARE
        );

        let mut fragmented_packet = ipv4_udp_packet([10, 0, 0, 1], [10, 0, 0, 2], 100, 200);
        fragmented_packet[14 + 6] = 0x20;
        machine.write_bytes(packet_ptr, &fragmented_packet).unwrap();
        write_envelope(
            &mut machine,
            envelope,
            CLASSIFY_PROFILE_PACKET,
            source,
            packet_ptr,
            fragmented_packet.len() as u64,
            0,
            0,
            0,
        );
        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            CLASSIFY_ACTION_NEEDS_SOFTWARE
        );

        let mut reserved_flag_packet = ipv4_udp_packet([10, 0, 0, 1], [10, 0, 0, 2], 100, 200);
        reserved_flag_packet[14 + 6] = 0x80;
        machine
            .write_bytes(packet_ptr, &reserved_flag_packet)
            .unwrap();
        write_envelope(
            &mut machine,
            envelope,
            CLASSIFY_PROFILE_PACKET,
            source,
            packet_ptr,
            reserved_flag_packet.len() as u64,
            0,
            0,
            0,
        );
        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            CLASSIFY_ACTION_NEEDS_SOFTWARE
        );

        query_classifier_counters(&mut machine, classifier, counters);
        assert_eq!(machine.load_u64(counters).unwrap(), 1);
        assert_eq!(machine.load_u64(counters + 8).unwrap(), 1);
        assert_eq!(machine.load_u64(counters + 16).unwrap(), 0);
        assert_eq!(machine.load_u64(counters + 24).unwrap(), 1);
        assert_eq!(machine.load_u64(counters + 32).unwrap(), 4);
    }

    #[test]
    fn classifier_rejects_unauthorized_stale_and_revoked_routes() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let (_reader_token, writer_token) = create_pipe_pair(&mut machine, 3, 4);
        let (_other_reader, other_writer) = create_pipe_pair(&mut machine, 7, 8);
        let source = create_memory_source(&mut machine, 5);
        let rules = ARG_BASE + 0x1000;
        let allowed = ARG_BASE + 0x1800;
        let envelope = ARG_BASE + 0x1a00;
        let result = ARG_BASE + 0x1b00;
        machine.store_u64(allowed, writer_token).unwrap();
        write_classifier_rule(
            &mut machine,
            rules,
            CLASSIFY_RULE_EXACT,
            CLASSIFY_FIELD_SERVICE_ID,
            1,
            0,
            CLASSIFY_ACTION_ROUTE,
            other_writer,
            0,
        );
        assert_eq!(
            try_create_classifier(&mut machine, 6, rules, 1, allowed, 1),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 1);
        write_envelope(
            &mut machine,
            envelope,
            CLASSIFY_PROFILE_IPC,
            source,
            0,
            0,
            1,
            0,
            0,
        );

        write_classifier_rule(
            &mut machine,
            rules,
            CLASSIFY_RULE_EXACT,
            CLASSIFY_FIELD_SERVICE_ID,
            1,
            0,
            CLASSIFY_ACTION_ROUTE,
            writer_token,
            0,
        );
        let classifier = create_classifier(&mut machine, 6, rules, 1, allowed, 1);
        machine.store_u64(envelope + 16, 0).unwrap();
        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 116);
        machine.store_u64(envelope + 16, 999).unwrap();
        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 116);
        write_envelope(
            &mut machine,
            envelope,
            CLASSIFY_PROFILE_IPC,
            source,
            0,
            0,
            1,
            0,
            0,
        );
        machine.close_fd_index(5).unwrap();
        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 116);

        let source = create_memory_source(&mut machine, 5);
        write_envelope(
            &mut machine,
            envelope,
            CLASSIFY_PROFILE_IPC,
            source,
            0,
            0,
            1,
            0,
            0,
        );
        machine.close_fd_index(4).unwrap();
        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 116);

        let (_reader_token, writer_token) = create_pipe_pair(&mut machine, 7, 8);
        machine.store_u64(allowed, writer_token).unwrap();
        write_classifier_rule(
            &mut machine,
            rules,
            CLASSIFY_RULE_EXACT,
            CLASSIFY_FIELD_SERVICE_ID,
            1,
            0,
            CLASSIFY_ACTION_ROUTE,
            writer_token,
            0,
        );
        machine.close_fd_index(6).unwrap();
        let classifier = create_classifier(&mut machine, 6, rules, 1, allowed, 1);
        let source = create_memory_source(&mut machine, 5);
        write_envelope(
            &mut machine,
            envelope,
            CLASSIFY_PROFILE_IPC,
            source,
            0,
            0,
            1,
            0,
            0,
        );
        machine.store_u64(ARG_BASE + 0x1e00, source).unwrap();
        machine.cap_revoke(Reg(11), ARG_BASE + 0x1e00).unwrap();
        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 116);

        let (_reader_token, writer_token) = create_pipe_pair(&mut machine, 9, 10);
        machine.store_u64(allowed, writer_token).unwrap();
        write_classifier_rule(
            &mut machine,
            rules,
            CLASSIFY_RULE_EXACT,
            CLASSIFY_FIELD_SERVICE_ID,
            1,
            0,
            CLASSIFY_ACTION_ROUTE,
            writer_token,
            0,
        );
        machine.close_fd_index(6).unwrap();
        let classifier = create_classifier(&mut machine, 6, rules, 1, allowed, 1);
        let source = create_memory_source(&mut machine, 5);
        write_envelope(
            &mut machine,
            envelope,
            CLASSIFY_PROFILE_IPC,
            source,
            0,
            0,
            1,
            0,
            0,
        );
        machine.store_u64(ARG_BASE + 0x1f00, writer_token).unwrap();
        machine.cap_revoke(Reg(11), ARG_BASE + 0x1f00).unwrap();
        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 116);
    }

    #[test]
    fn classifier_cap_source_and_queue_rights_are_enforced() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let (_reader_token, writer_token) = create_pipe_pair(&mut machine, 3, 4);
        let source = create_memory_source(&mut machine, 5);
        let rules = ARG_BASE + 0x1000;
        let allowed = ARG_BASE + 0x1800;
        let envelope = ARG_BASE + 0x1a00;
        let result = ARG_BASE + 0x1b00;
        machine.store_u64(allowed, writer_token).unwrap();
        write_classifier_rule(
            &mut machine,
            rules,
            CLASSIFY_RULE_EXACT,
            CLASSIFY_FIELD_SERVICE_ID,
            1,
            0,
            CLASSIFY_ACTION_ROUTE,
            writer_token,
            0,
        );
        let classifier = create_classifier(&mut machine, 6, rules, 1, allowed, 1);
        write_envelope(
            &mut machine,
            envelope,
            CLASSIFY_PROFILE_IPC,
            source,
            0,
            0,
            1,
            0,
            0,
        );

        machine.processes.get_mut(&1).unwrap().fd_capabilities[6].rights &= !CAP_RIGHT_CALL;
        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 1);
        machine.processes.get_mut(&1).unwrap().fd_capabilities[6].rights |= CAP_RIGHT_CALL;

        machine.processes.get_mut(&1).unwrap().fd_capabilities[5].rights &= !CAP_RIGHT_READ;
        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 1);
        machine.processes.get_mut(&1).unwrap().fd_capabilities[5].rights |= CAP_RIGHT_READ;

        machine.processes.get_mut(&1).unwrap().fd_capabilities[4].rights &= !CAP_RIGHT_WRITE;
        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 1);
    }

    #[test]
    fn classifier_table_capability_is_generation_checked_and_revocable() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let (_reader_token, writer_token) = create_pipe_pair(&mut machine, 3, 4);
        let source = create_memory_source(&mut machine, 5);
        let rules = ARG_BASE + 0x1000;
        let allowed = ARG_BASE + 0x1800;
        let envelope = ARG_BASE + 0x1a00;
        let result = ARG_BASE + 0x1b00;
        machine.store_u64(allowed, writer_token).unwrap();
        write_classifier_rule(
            &mut machine,
            rules,
            CLASSIFY_RULE_EXACT,
            CLASSIFY_FIELD_SERVICE_ID,
            1,
            0,
            CLASSIFY_ACTION_ROUTE,
            writer_token,
            0,
        );
        let classifier = create_classifier(&mut machine, 6, rules, 1, allowed, 1);
        write_envelope(
            &mut machine,
            envelope,
            CLASSIFY_PROFILE_IPC,
            source,
            0,
            0,
            1,
            0,
            0,
        );

        machine.close_fd_index(6).unwrap();
        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 116);

        let classifier = create_classifier(&mut machine, 6, rules, 1, allowed, 1);
        machine.store_u64(ARG_BASE + 0x1c00, classifier).unwrap();
        machine.cap_revoke(Reg(11), ARG_BASE + 0x1c00).unwrap();
        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 116);
    }

    #[test]
    fn classifier_table_creation_is_bounded() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        assert_eq!(
            try_create_classifier(&mut machine, 6, 0, CLASSIFIER_MAX_RULES as u64 + 1, 0, 0),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 22);
        assert_eq!(
            try_create_classifier(
                &mut machine,
                6,
                0,
                0,
                0,
                CLASSIFIER_MAX_ALLOWED_QUEUES as u64 + 1
            ),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 22);
        assert_eq!(
            try_create_classifier(&mut machine, 6, 0, 1, 0, 0),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 14);
        assert_eq!(
            try_create_classifier(&mut machine, 6, 0, 0, 0, 1),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 14);
    }

    #[test]
    fn classifier_table_create_failures_do_not_install_requested_fd() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let rules = ARG_BASE + 0x1000;

        assert_eq!(
            try_create_classifier(&mut machine, 6, 0, CLASSIFIER_MAX_RULES as u64 + 1, 0, 0),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 22);
        assert!(matches!(
            machine.process().unwrap().fds[6],
            FdHandle::Closed
        ));

        write_classifier_rule(
            &mut machine,
            rules,
            99,
            CLASSIFY_FIELD_SERVICE_ID,
            1,
            0,
            CLASSIFY_ACTION_COUNT,
            0,
            0,
        );
        assert_eq!(
            try_create_classifier(&mut machine, 6, rules, 1, 0, 0),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 22);
        assert!(matches!(
            machine.process().unwrap().fds[6],
            FdHandle::Closed
        ));

        let retained = Rc::new(RefCell::new(77));
        {
            let process = machine.process_mut().unwrap();
            process.fds[6] = FdHandle::Counter(retained.clone());
            process.fd_capabilities[6] = FdCapability::full(6);
        }

        assert_eq!(
            try_create_classifier(&mut machine, 6, 0, 1, 0, 0),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 14);
        match &machine.process().unwrap().fds[6] {
            FdHandle::Counter(value) => {
                assert!(Rc::ptr_eq(value, &retained));
                assert_eq!(*value.borrow(), 77);
            }
            _ => panic!("expected retained counter fd"),
        }
    }

    #[test]
    fn classifier_table_rejects_invalid_rule_descriptors() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let rules = ARG_BASE + 0x1000;
        let invalid_cases = [
            (99, CLASSIFY_FIELD_SERVICE_ID, CLASSIFY_ACTION_COUNT),
            (CLASSIFY_RULE_EXACT, 99, CLASSIFY_ACTION_COUNT),
            (CLASSIFY_RULE_EXACT, CLASSIFY_FIELD_SERVICE_ID, 99),
        ];

        for (kind, field, action) in invalid_cases {
            write_classifier_rule(&mut machine, rules, kind, field, 1, 0, action, 0, 0);
            assert_eq!(
                try_create_classifier(&mut machine, 6, rules, 1, 0, 0),
                -1i64 as u64
            );
            assert_eq!(machine.process().unwrap().errno, 22);
        }

        write_classifier_rule(
            &mut machine,
            rules,
            CLASSIFY_RULE_RANGE,
            CLASSIFY_FIELD_SERVICE_ID,
            10,
            5,
            CLASSIFY_ACTION_COUNT,
            0,
            0,
        );
        assert_eq!(
            try_create_classifier(&mut machine, 6, rules, 1, 0, 0),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 22);

        write_classifier_rule(
            &mut machine,
            rules,
            CLASSIFY_RULE_HASH,
            CLASSIFY_FIELD_HASH,
            0,
            0,
            CLASSIFY_ACTION_COUNT,
            0,
            0,
        );
        assert_eq!(
            try_create_classifier(&mut machine, 6, rules, 1, 0, 0),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 22);
    }

    #[test]
    fn classifier_rejects_invalid_profile_and_foreign_domain_envelopes() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let source = create_memory_source(&mut machine, 5);
        let classifier = create_classifier(&mut machine, 6, 0, 0, 0, 0);
        let envelope = ARG_BASE + 0x1000;
        let result = ARG_BASE + 0x1100;

        write_envelope(&mut machine, envelope, 99, source, 0, 0, 0, 0, 0);
        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 22);

        write_envelope(
            &mut machine,
            envelope,
            CLASSIFY_PROFILE_IPC,
            source,
            0,
            0,
            0,
            0,
            0,
        );
        machine
            .store_u64(envelope + 24, ROOT_DOMAIN_ID + 1)
            .unwrap();
        assert_eq!(
            classify(&mut machine, classifier, envelope, result),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 1);
    }

    #[test]
    fn classifier_control_records_reject_bad_pointers_handles_and_rights() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let (_reader_token, writer_token) = create_pipe_pair(&mut machine, 3, 4);
        let source = create_memory_source(&mut machine, 5);
        let classifier = create_classifier(&mut machine, 6, 0, 0, 0, 0);
        let envelope = ARG_BASE + 0x1000;
        let result = ARG_BASE + 0x1100;
        let counters = ARG_BASE + 0x1200;

        assert_eq!(classify(&mut machine, classifier, 0, result), -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 14);

        write_envelope(
            &mut machine,
            envelope,
            CLASSIFY_PROFILE_IPC,
            source,
            0,
            0,
            1,
            0,
            0,
        );
        assert_eq!(
            classify(&mut machine, writer_token, envelope, result),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 9);

        let arg = ARG_BASE + 0x300;
        machine.store_u64(arg, OBJECT_OP_CLASSIFIER_QUERY).unwrap();
        machine.store_u64(arg + 8, classifier).unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.object_ctl(Reg(10), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[10], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 14);

        machine.store_u64(arg + 8, writer_token).unwrap();
        machine.store_u64(arg + 16, counters).unwrap();
        machine.object_ctl(Reg(10), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[10], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 9);

        machine.store_u64(arg + 8, classifier).unwrap();
        machine.processes.get_mut(&1).unwrap().fd_capabilities[6].rights &= !CAP_RIGHT_STAT;
        machine.object_ctl(Reg(10), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[10], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
    }

    #[test]
    fn servicelet_program_creation_verifies_bounds() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let envelope = ARG_BASE + 0x1000;
        write_servicelet_envelope(&mut machine, envelope, 64, 0x03, 32, 128, 64, 32, 0);

        assert_ne!(
            try_create_servicelet(&mut machine, 7, envelope),
            -1i64 as u64
        );
        match &machine.process().unwrap().fds[7] {
            FdHandle::ServiceletProgram(program) => {
                assert_eq!(program.program_len, 64);
                assert_eq!(program.isa_subset, 0x03);
                assert_eq!(program.instruction_limit, 32);
                assert_eq!(program.cycle_limit, 128);
                assert_eq!(program.record_read_limit, 64);
                assert_eq!(program.action_write_limit, 32);
                assert_eq!(program.flags, 0);
                assert_eq!(program.owner_domain_id, ROOT_DOMAIN_ID);
                assert_eq!(
                    program.owner_generation,
                    machine.domains[&ROOT_DOMAIN_ID].generation
                );
            }
            _ => panic!("expected servicelet program fd"),
        }
        assert!(machine.fd_token(7).unwrap() & FDR_TOKEN_MARKER != 0);
    }

    #[test]
    fn servicelet_verifier_accepts_advertised_static_loop_flag() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let envelope = ARG_BASE + 0x1000;
        write_servicelet_envelope(
            &mut machine,
            envelope,
            64,
            0x01,
            32,
            128,
            64,
            32,
            SERVICELET_FLAG_ALLOW_STATIC_LOOPS,
        );

        assert_ne!(
            try_create_servicelet(&mut machine, 7, envelope),
            -1i64 as u64
        );
        match &machine.process().unwrap().fds[7] {
            FdHandle::ServiceletProgram(program) => {
                assert_eq!(program.flags, SERVICELET_FLAG_ALLOW_STATIC_LOOPS);
            }
            _ => panic!("expected servicelet program fd"),
        }
    }

    #[test]
    fn servicelet_program_owner_must_be_current_domain_or_descendant() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        add_test_domain(&mut machine, 2, ROOT_DOMAIN_ID);
        add_test_domain(&mut machine, 3, ROOT_DOMAIN_ID);
        machine.processes.get_mut(&1).unwrap().domain_id = 2;
        let envelope = ARG_BASE + 0x1000;
        write_servicelet_envelope(&mut machine, envelope, 64, 0x01, 32, 128, 64, 32, 0);
        machine.store_u64(envelope + 64, 3).unwrap();
        machine.store_u64(envelope + 72, 1).unwrap();

        assert_eq!(
            try_create_servicelet(&mut machine, 7, envelope),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 1);

        add_test_domain(&mut machine, 4, 2);
        machine.store_u64(envelope + 64, 4).unwrap();
        machine.store_u64(envelope + 72, 1).unwrap();
        assert_ne!(
            try_create_servicelet(&mut machine, 7, envelope),
            -1i64 as u64
        );
    }

    #[test]
    fn servicelet_program_verifier_rejects_bad_envelopes() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let envelope = ARG_BASE + 0x1000;

        assert_eq!(try_create_servicelet(&mut machine, 7, 0), -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 14);

        write_servicelet_envelope(&mut machine, envelope, 64, 0x01, 32, 128, 64, 32, 0);
        machine
            .store_u64(envelope, SERVICELET_VERIFY_VERSION + 1)
            .unwrap();
        assert_eq!(
            try_create_servicelet(&mut machine, 7, envelope),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 22);

        write_servicelet_envelope(&mut machine, envelope, 0, 0x01, 32, 128, 64, 32, 0);
        assert_eq!(
            try_create_servicelet(&mut machine, 7, envelope),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 22);

        write_servicelet_envelope(
            &mut machine,
            envelope,
            SERVICELET_MAX_PROGRAM_BYTES + 1,
            0x01,
            32,
            128,
            64,
            32,
            0,
        );
        assert_eq!(
            try_create_servicelet(&mut machine, 7, envelope),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 22);

        write_servicelet_envelope(
            &mut machine,
            envelope,
            64,
            SERVICELET_ALLOWED_ISA_MASK << 1,
            32,
            128,
            64,
            32,
            0,
        );
        assert_eq!(
            try_create_servicelet(&mut machine, 7, envelope),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 22);

        write_servicelet_envelope(&mut machine, envelope, 64, 0x01, 0, 128, 64, 32, 0);
        assert_eq!(
            try_create_servicelet(&mut machine, 7, envelope),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 22);

        write_servicelet_envelope(
            &mut machine,
            envelope,
            64,
            0x01,
            SERVICELET_MAX_INSTRUCTIONS + 1,
            128,
            64,
            32,
            0,
        );
        assert_eq!(
            try_create_servicelet(&mut machine, 7, envelope),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 22);

        write_servicelet_envelope(&mut machine, envelope, 64, 0x01, 32, 0, 64, 32, 0);
        assert_eq!(
            try_create_servicelet(&mut machine, 7, envelope),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 22);

        write_servicelet_envelope(
            &mut machine,
            envelope,
            64,
            0x01,
            32,
            SERVICELET_MAX_CYCLES + 1,
            64,
            32,
            0,
        );
        assert_eq!(
            try_create_servicelet(&mut machine, 7, envelope),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 22);

        write_servicelet_envelope(
            &mut machine,
            envelope,
            64,
            0x01,
            32,
            128,
            SERVICELET_MAX_RECORD_BYTES + 1,
            32,
            0,
        );
        assert_eq!(
            try_create_servicelet(&mut machine, 7, envelope),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 22);

        write_servicelet_envelope(
            &mut machine,
            envelope,
            64,
            0x01,
            32,
            128,
            64,
            SERVICELET_MAX_ACTION_BYTES + 1,
            0,
        );
        assert_eq!(
            try_create_servicelet(&mut machine, 7, envelope),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 22);

        write_servicelet_envelope(&mut machine, envelope, 64, 0x01, 32, 128, 64, 32, 1 << 4);
        assert_eq!(
            try_create_servicelet(&mut machine, 7, envelope),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 22);

        write_servicelet_envelope(&mut machine, envelope, 64, 0x01, 32, 128, 64, 32, 0);
        machine.store_u64(envelope + 64, 999).unwrap();
        machine.store_u64(envelope + 72, 1).unwrap();
        assert_eq!(
            try_create_servicelet(&mut machine, 7, envelope),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 3);

        write_servicelet_envelope(&mut machine, envelope, 64, 0x01, 32, 128, 64, 32, 0);
        machine.store_u64(envelope + 72, 0).unwrap();
        assert_eq!(
            try_create_servicelet(&mut machine, 7, envelope),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 116);

        write_servicelet_envelope(&mut machine, envelope, 64, 0x01, 32, 128, 64, 32, 0);
        machine
            .store_u64(
                envelope + 72,
                machine.domains[&ROOT_DOMAIN_ID].generation + 1,
            )
            .unwrap();
        assert_eq!(
            try_create_servicelet(&mut machine, 7, envelope),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 116);
    }

    #[test]
    fn servicelet_verifier_rejections_do_not_install_requested_fd() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let envelope = ARG_BASE + 0x1000;

        assert_eq!(try_create_servicelet(&mut machine, 7, 0), -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 14);
        assert!(matches!(
            machine.process().unwrap().fds[7],
            FdHandle::Closed
        ));

        write_servicelet_envelope(&mut machine, envelope, 64, 0x01, 32, 128, 64, 32, 1 << 4);
        assert_eq!(
            try_create_servicelet(&mut machine, 7, envelope),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 22);
        assert!(matches!(
            machine.process().unwrap().fds[7],
            FdHandle::Closed
        ));

        let retained = Rc::new(RefCell::new(55));
        {
            let process = machine.process_mut().unwrap();
            process.fds[7] = FdHandle::Counter(retained.clone());
            process.fd_capabilities[7] = FdCapability::full(7);
        }

        write_servicelet_envelope(&mut machine, envelope, 64, 0x01, 32, 128, 64, 32, 0);
        machine
            .store_u64(
                envelope + 72,
                machine.domains[&ROOT_DOMAIN_ID].generation + 1,
            )
            .unwrap();
        assert_eq!(
            try_create_servicelet(&mut machine, 7, envelope),
            -1i64 as u64
        );
        assert_eq!(machine.process().unwrap().errno, 116);
        match &machine.process().unwrap().fds[7] {
            FdHandle::Counter(value) => {
                assert!(Rc::ptr_eq(value, &retained));
                assert_eq!(*value.borrow(), 55);
            }
            _ => panic!("expected retained counter fd"),
        }
    }

    #[test]
    fn socket_endpoint_object_controls_enforce_capability_rights() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let arg = ARG_BASE + 0x1000;
        let addr = ARG_BASE + 0x1100;
        let optval = ARG_BASE + 0x1200;
        let optlen = ARG_BASE + 0x1300;
        machine.write_bytes(addr, b"127.0.0.1:0\0").unwrap();
        machine.store_u64(optval, 1).unwrap();
        machine.store_u64(optlen, 8).unwrap();

        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::Endpoint.code())
            .unwrap();
        machine
            .store_u64(arg + 16, ObjectProfile::TcpStream.code())
            .unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine.store_u64(arg + 40, 2).unwrap();
        machine.store_u64(arg + 48, 1).unwrap();
        machine.store_u64(arg + 56, 0).unwrap();
        machine.object_ctl(Reg(1), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[1], 7);

        machine.processes.get_mut(&1).unwrap().fd_capabilities[7].rights &= !CAP_RIGHT_WRITE;
        machine.store_u64(arg, OBJECT_OP_SOCKET_BIND).unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine.store_u64(arg + 40, addr).unwrap();
        machine.object_ctl(Reg(2), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[2], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
        machine.processes.get_mut(&1).unwrap().fd_capabilities[7].rights |= CAP_RIGHT_WRITE;
        machine.object_ctl(Reg(3), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[3], 0);

        machine.processes.get_mut(&1).unwrap().fd_capabilities[7].rights &= !CAP_RIGHT_POLL;
        machine.store_u64(arg, OBJECT_OP_SOCKET_LISTEN).unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine.object_ctl(Reg(10), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[10], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
        machine.processes.get_mut(&1).unwrap().fd_capabilities[7].rights |= CAP_RIGHT_POLL;

        machine.processes.get_mut(&1).unwrap().fd_capabilities[7].rights &= !CAP_RIGHT_READ;
        machine.store_u64(arg, OBJECT_OP_SOCKET_CONNECT).unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine.store_u64(arg + 40, addr).unwrap();
        machine.object_ctl(Reg(11), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[11], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
        machine.processes.get_mut(&1).unwrap().fd_capabilities[7].rights |= CAP_RIGHT_READ;

        machine.processes.get_mut(&1).unwrap().fd_capabilities[7].rights &= !CAP_RIGHT_POLL;
        machine.store_u64(arg, OBJECT_OP_SOCKET_ACCEPT).unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine.store_u64(arg + 32, 0).unwrap();
        machine.object_ctl(Reg(12), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[12], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
        machine.processes.get_mut(&1).unwrap().fd_capabilities[7].rights |= CAP_RIGHT_POLL;

        machine.processes.get_mut(&1).unwrap().fd_capabilities[7].rights &= !CAP_RIGHT_STAT;
        machine.store_u64(arg, OBJECT_OP_SOCKET_GETSOCKOPT).unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine
            .store_u64(arg + 40, SOCKET_LEVEL_SOL_SOCKET)
            .unwrap();
        machine.store_u64(arg + 48, SOCKET_OPT_SO_ERROR).unwrap();
        machine.store_u64(arg + 56, optval).unwrap();
        machine.store_u64(arg + 64, optlen).unwrap();
        machine.object_ctl(Reg(4), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
        machine.processes.get_mut(&1).unwrap().fd_capabilities[7].rights |= CAP_RIGHT_STAT;
        machine.object_ctl(Reg(5), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[5], 0);
        assert_eq!(machine.load_u64(optlen).unwrap(), 8);

        machine.processes.get_mut(&1).unwrap().fd_capabilities[7].rights &= !CAP_RIGHT_WRITE;
        machine.store_u64(arg, OBJECT_OP_SOCKET_SETSOCKOPT).unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine
            .store_u64(arg + 40, SOCKET_LEVEL_SOL_SOCKET)
            .unwrap();
        machine
            .store_u64(arg + 48, SOCKET_OPT_SO_REUSEADDR)
            .unwrap();
        machine.store_u64(arg + 56, optval).unwrap();
        machine.store_u64(arg + 64, 8).unwrap();
        machine.object_ctl(Reg(6), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[6], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
    }

    #[test]
    fn socket_endpoint_rejects_invalid_addresses_without_state_change() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let arg = ARG_BASE + 0x1000;
        let addr = ARG_BASE + 0x1100;
        machine.write_bytes(addr, b"localhost:41065\0").unwrap();

        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::Endpoint.code())
            .unwrap();
        machine
            .store_u64(arg + 16, ObjectProfile::TcpStream.code())
            .unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine.store_u64(arg + 40, SOCKET_AF_INET).unwrap();
        machine.store_u64(arg + 48, SOCKET_TYPE_STREAM).unwrap();
        machine.store_u64(arg + 56, 0).unwrap();
        machine.object_ctl(Reg(1), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[1], 7);

        machine.store_u64(arg, OBJECT_OP_SOCKET_BIND).unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine.store_u64(arg + 40, addr).unwrap();
        machine.object_ctl(Reg(2), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[2], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        match &machine.process().unwrap().fds[7] {
            FdHandle::TcpSocket {
                bound_addr,
                domain,
                sock_type,
                protocol,
            } => {
                assert!(bound_addr.is_none());
                assert_eq!(*domain, SOCKET_AF_INET);
                assert_eq!(*sock_type, SOCKET_TYPE_STREAM);
                assert_eq!(*protocol, 0);
            }
            _ => panic!("expected unbound TCP socket"),
        }

        machine.store_u64(arg, OBJECT_OP_SOCKET_CONNECT).unwrap();
        machine.object_ctl(Reg(3), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[3], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        assert!(matches!(
            machine.process().unwrap().fds[7],
            FdHandle::TcpSocket { .. }
        ));
    }

    #[test]
    fn socket_accept_prevalidates_destination_before_taking_pending_stream() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let client = TcpStream::connect(addr).unwrap();
        let (server, _) = listener.accept().unwrap();

        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        {
            let process = machine.process_mut().unwrap();
            process.fds[7] = FdHandle::TcpListener {
                listener,
                pending: Some(server),
            };
            process.fd_capabilities[7] = FdCapability::full(7);
        }

        let arg = ARG_BASE + 0x1000;
        machine.store_u64(arg, OBJECT_OP_SOCKET_ACCEPT).unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine
            .store_u64(arg + 32, MESSAGE_ENDPOINT_FD as u64)
            .unwrap();
        machine.object_ctl(Reg(2), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[2], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 9);
        match &machine.process().unwrap().fds[7] {
            FdHandle::TcpListener {
                pending: Some(_), ..
            } => {}
            _ => panic!("expected pending stream to remain queued"),
        }
        drop(client);
    }

    #[test]
    fn socket_accept_prevalidates_output_slot_before_taking_pending_stream() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let client = TcpStream::connect(addr).unwrap();
        let (server, _) = listener.accept().unwrap();

        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        {
            let process = machine.process_mut().unwrap();
            process.fds[7] = FdHandle::TcpListener {
                listener,
                pending: Some(server),
            };
            process.fd_capabilities[7] = FdCapability::full(7);
        }

        let arg = ARG_BASE + 0x1000;
        machine.store_u64(arg, OBJECT_OP_SOCKET_ACCEPT).unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine.store_u64(arg + 32, 8).unwrap();
        {
            let process = machine.process_mut().unwrap();
            let vma = process
                .vmas
                .iter_mut()
                .find(|vma| vma.contains(arg, 40))
                .expect("socket accept argblock VMA");
            vma.prot = 0b01;
        }

        machine.object_ctl(Reg(2), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[2], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 14);
        match &machine.process().unwrap().fds[7] {
            FdHandle::TcpListener {
                pending: Some(_), ..
            } => {}
            _ => panic!("expected pending stream to remain queued"),
        }
        assert!(matches!(
            machine.process().unwrap().fds[8],
            FdHandle::Closed
        ));
        drop(client);
    }

    #[test]
    fn socket_endpoint_create_rejects_unsupported_profiles_without_installing_fd() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let arg = ARG_BASE + 0x1000;

        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::Endpoint.code())
            .unwrap();
        machine
            .store_u64(arg + 16, ObjectProfile::TcpStream.code())
            .unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine.store_u64(arg + 40, SOCKET_AF_INET).unwrap();
        machine.store_u64(arg + 48, 2).unwrap();
        machine.store_u64(arg + 56, 0).unwrap();
        machine.object_ctl(Reg(2), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[2], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        assert!(matches!(
            machine.process().unwrap().fds[7],
            FdHandle::Closed
        ));

        machine.store_u64(arg + 48, SOCKET_TYPE_STREAM).unwrap();
        machine.store_u64(arg + 56, 17).unwrap();
        machine.object_ctl(Reg(3), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[3], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        assert!(matches!(
            machine.process().unwrap().fds[7],
            FdHandle::Closed
        ));

        machine.store_u64(arg + 40, 10).unwrap();
        machine.store_u64(arg + 56, 0).unwrap();
        machine.object_ctl(Reg(4), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        assert!(matches!(
            machine.process().unwrap().fds[7],
            FdHandle::Closed
        ));
    }

    #[test]
    fn socket_endpoint_state_transitions_reject_stale_capability_tokens() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let arg = ARG_BASE + 0x1000;
        let addr = ARG_BASE + 0x1100;
        let out = ARG_BASE + 0x1200;
        let out_len = ARG_BASE + 0x1300;
        machine.write_bytes(addr, b"127.0.0.1:0\0").unwrap();

        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::Endpoint.code())
            .unwrap();
        machine
            .store_u64(arg + 16, ObjectProfile::TcpStream.code())
            .unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine.store_u64(arg + 40, 2).unwrap();
        machine.store_u64(arg + 48, 1).unwrap();
        machine.store_u64(arg + 56, 0).unwrap();
        machine.object_ctl(Reg(1), arg).unwrap();
        let socket_token = machine.fd_token(7).unwrap();

        machine.store_u64(arg, OBJECT_OP_SOCKET_BIND).unwrap();
        machine.store_u64(arg + 24, socket_token).unwrap();
        machine.store_u64(arg + 40, addr).unwrap();
        machine.object_ctl(Reg(2), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[2], 0);

        machine.store_u64(arg, OBJECT_OP_SOCKET_LISTEN).unwrap();
        machine.store_u64(arg + 24, socket_token).unwrap();
        machine.object_ctl(Reg(3), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[3], 0);
        let listener_token = machine.fd_token(7).unwrap();
        assert_ne!(listener_token, socket_token);

        machine
            .store_u64(arg, OBJECT_OP_SOCKET_GETSOCKNAME)
            .unwrap();
        machine.store_u64(arg + 24, socket_token).unwrap();
        machine.store_u64(arg + 40, out).unwrap();
        machine.store_u64(out_len, 64).unwrap();
        machine.store_u64(arg + 48, out_len).unwrap();
        machine.object_ctl(Reg(4), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 116);

        machine.store_u64(arg + 24, listener_token).unwrap();
        machine.object_ctl(Reg(5), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[5], 0);
        assert!(machine.load_u64(out_len).unwrap() > 0);
    }

    #[test]
    fn socket_getsockopt_prevalidates_value_before_len_update() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let arg = ARG_BASE + 0x1000;
        let optlen = ARG_BASE + 0x1300;

        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::Endpoint.code())
            .unwrap();
        machine
            .store_u64(arg + 16, ObjectProfile::TcpStream.code())
            .unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine.store_u64(arg + 40, SOCKET_AF_INET).unwrap();
        machine.store_u64(arg + 48, SOCKET_TYPE_STREAM).unwrap();
        machine.store_u64(arg + 56, 0).unwrap();
        machine.object_ctl(Reg(1), arg).unwrap();
        let socket_token = machine.fd_token(7).unwrap();

        machine.store_u64(optlen, 64).unwrap();
        machine.store_u64(arg, OBJECT_OP_SOCKET_GETSOCKOPT).unwrap();
        machine.store_u64(arg + 24, socket_token).unwrap();
        machine
            .store_u64(arg + 40, SOCKET_LEVEL_SOL_SOCKET)
            .unwrap();
        machine.store_u64(arg + 48, SOCKET_OPT_SO_ERROR).unwrap();
        machine.store_u64(arg + 56, MEMORY_SIZE as u64).unwrap();
        machine.store_u64(arg + 64, optlen).unwrap();
        machine.object_ctl(Reg(2), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[2], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 14);
        assert_eq!(machine.load_u64(optlen).unwrap(), 64);
    }

    #[test]
    fn socket_option_controls_reject_unsupported_options_before_buffers() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let arg = ARG_BASE + 0x1000;
        let optval = ARG_BASE + 0x1200;
        let optlen = ARG_BASE + 0x1300;

        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::Endpoint.code())
            .unwrap();
        machine
            .store_u64(arg + 16, ObjectProfile::TcpStream.code())
            .unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine.store_u64(arg + 40, SOCKET_AF_INET).unwrap();
        machine.store_u64(arg + 48, SOCKET_TYPE_STREAM).unwrap();
        machine.store_u64(arg + 56, 0).unwrap();
        machine.object_ctl(Reg(1), arg).unwrap();
        let socket_token = machine.fd_token(7).unwrap();

        machine.store_u64(optval, 0xfeed_face).unwrap();
        machine.store_u64(optlen, 64).unwrap();
        machine.store_u64(arg, OBJECT_OP_SOCKET_GETSOCKOPT).unwrap();
        machine.store_u64(arg + 24, socket_token).unwrap();
        machine
            .store_u64(arg + 40, SOCKET_LEVEL_SOL_SOCKET)
            .unwrap();
        machine.store_u64(arg + 48, 99).unwrap();
        machine.store_u64(arg + 56, optval).unwrap();
        machine.store_u64(arg + 64, optlen).unwrap();
        machine.object_ctl(Reg(2), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[2], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        assert_eq!(machine.load_u64(optval).unwrap(), 0xfeed_face);
        assert_eq!(machine.load_u64(optlen).unwrap(), 64);

        machine.store_u64(arg, OBJECT_OP_SOCKET_SETSOCKOPT).unwrap();
        machine.store_u64(arg + 48, 99).unwrap();
        machine.store_u64(arg + 56, MEMORY_SIZE as u64).unwrap();
        machine.store_u64(arg + 64, 8).unwrap();
        machine.object_ctl(Reg(3), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[3], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
    }

    #[test]
    fn socket_getsockname_rejects_short_buffer_without_writes() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let arg = ARG_BASE + 0x1000;
        let addr = ARG_BASE + 0x1100;
        let out = ARG_BASE + 0x1200;
        let out_len = ARG_BASE + 0x1300;
        machine.write_bytes(addr, b"127.0.0.1:0\0").unwrap();

        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::Endpoint.code())
            .unwrap();
        machine
            .store_u64(arg + 16, ObjectProfile::TcpStream.code())
            .unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine.store_u64(arg + 40, SOCKET_AF_INET).unwrap();
        machine.store_u64(arg + 48, SOCKET_TYPE_STREAM).unwrap();
        machine.store_u64(arg + 56, 0).unwrap();
        machine.object_ctl(Reg(1), arg).unwrap();

        machine.store_u64(arg, OBJECT_OP_SOCKET_BIND).unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine.store_u64(arg + 40, addr).unwrap();
        machine.object_ctl(Reg(2), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[2], 0);

        machine.store_u64(arg, OBJECT_OP_SOCKET_LISTEN).unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine.object_ctl(Reg(3), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[3], 0);
        let listener_token = machine.fd_token(7).unwrap();

        machine.write_bytes(out, b"sentinel\0").unwrap();
        machine.store_u64(out_len, 1).unwrap();
        machine
            .store_u64(arg, OBJECT_OP_SOCKET_GETSOCKNAME)
            .unwrap();
        machine.store_u64(arg + 24, listener_token).unwrap();
        machine.store_u64(arg + 40, out).unwrap();
        machine.store_u64(arg + 48, out_len).unwrap();
        machine.object_ctl(Reg(4), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        assert_eq!(machine.load_u64(out_len).unwrap(), 1);
        assert_eq!(machine.read_c_string(out).unwrap(), "sentinel");
    }

    #[test]
    fn socket_getsockname_prevalidates_output_before_len_update() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let arg = ARG_BASE + 0x1000;
        let addr = ARG_BASE + 0x1100;
        let out_len = ARG_BASE + 0x1300;
        machine.write_bytes(addr, b"127.0.0.1:0\0").unwrap();

        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::Endpoint.code())
            .unwrap();
        machine
            .store_u64(arg + 16, ObjectProfile::TcpStream.code())
            .unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine.store_u64(arg + 40, SOCKET_AF_INET).unwrap();
        machine.store_u64(arg + 48, SOCKET_TYPE_STREAM).unwrap();
        machine.store_u64(arg + 56, 0).unwrap();
        machine.object_ctl(Reg(1), arg).unwrap();

        machine.store_u64(arg, OBJECT_OP_SOCKET_BIND).unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine.store_u64(arg + 40, addr).unwrap();
        machine.object_ctl(Reg(2), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[2], 0);

        machine.store_u64(arg, OBJECT_OP_SOCKET_LISTEN).unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine.object_ctl(Reg(3), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[3], 0);
        let listener_token = machine.fd_token(7).unwrap();

        machine.store_u64(out_len, 64).unwrap();
        machine
            .store_u64(arg, OBJECT_OP_SOCKET_GETSOCKNAME)
            .unwrap();
        machine.store_u64(arg + 24, listener_token).unwrap();
        machine.store_u64(arg + 40, MEMORY_SIZE as u64).unwrap();
        machine.store_u64(arg + 48, out_len).unwrap();
        machine.object_ctl(Reg(4), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 14);
        assert_eq!(machine.load_u64(out_len).unwrap(), 64);
    }

    #[test]
    fn completion_helpers_are_errno_compatibility_boundary() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.complete_ok(123).unwrap();
        assert_eq!(machine.process().unwrap().errno, 0);
        assert_eq!(machine.thread().unwrap().regs[1], 123);

        machine.complete_err(22).unwrap();
        assert_eq!(machine.process().unwrap().errno, 22);
        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
    }

    #[test]
    fn clone_profiles_back_fork_and_spawn_entry() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;

        machine.set_errno(123).unwrap();
        machine
            .clone_with_profile(CloneProfile::NewProcessCow, Reg(5), None)
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[5], 2);
        assert_eq!(machine.process().unwrap().errno, 0);
        let child = machine
            .threads
            .values()
            .find(|thread| thread.pid == 2)
            .unwrap();
        assert_eq!(child.regs[5], 0);

        machine.set_errno(77).unwrap();
        machine
            .clone_with_profile(CloneProfile::NewThreadSharedVm, Reg(6), Some(0))
            .unwrap();
        assert!(machine.thread().unwrap().regs[6] >= 2);
        assert_eq!(machine.process().unwrap().errno, 0);
        assert!(machine.threads.len() >= 3);
    }

    #[test]
    fn clone_profile_failures_do_not_allocate_contexts() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let next_pid = machine.next_pid;
        let next_tid = machine.next_tid;
        let process_count = machine.processes.len();
        let thread_count = machine.threads.len();

        machine
            .clone_with_profile(CloneProfile::DomainTask, Reg(5), Some(0))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[5], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 38);
        assert_eq!(machine.next_pid, next_pid);
        assert_eq!(machine.next_tid, next_tid);
        assert_eq!(machine.processes.len(), process_count);
        assert_eq!(machine.threads.len(), thread_count);

        machine
            .clone_with_profile(CloneProfile::NewThreadSharedVm, Reg(6), None)
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[6], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        assert_eq!(machine.next_pid, next_pid);
        assert_eq!(machine.next_tid, next_tid);
        assert_eq!(machine.processes.len(), process_count);
        assert_eq!(machine.threads.len(), thread_count);

        machine.next_tid = 17;
        machine
            .clone_with_profile(CloneProfile::NewThreadSharedVm, Reg(7), Some(0))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[7], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 12);
        assert_eq!(machine.next_tid, 17);
        assert_eq!(machine.processes.len(), process_count);
        assert_eq!(machine.threads.len(), thread_count);

        let err = machine
            .clone_with_profile(CloneProfile::NewProcessCow, Reg(31), None)
            .unwrap_err();
        assert!(err.contains("hardware-locked stack pointer"), "{err}");
        assert_eq!(machine.next_pid, next_pid);
        assert_eq!(machine.next_tid, 17);
        assert_eq!(machine.processes.len(), process_count);
        assert_eq!(machine.threads.len(), thread_count);
    }

    #[test]
    fn signal_controls_reject_invalid_numbers_without_mutation() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine
            .process_mut()
            .unwrap()
            .signal_handlers
            .insert(2, SignalDisposition::Handler(7));

        machine.thread_mut().unwrap().regs[2] = 0;
        machine.thread_mut().unwrap().regs[3] = 99;
        assert!(machine.exec(Instr::Sigaction(Reg(2), Reg(3))).unwrap());
        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        assert!(!machine.process().unwrap().signal_handlers.contains_key(&0));
        assert!(matches!(
            machine.process().unwrap().signal_handlers.get(&2),
            Some(SignalDisposition::Handler(7))
        ));

        machine.thread_mut().unwrap().regs[2] = SIGNAL_NUMBER_LIMIT;
        machine.thread_mut().unwrap().regs[3] = SIG_IGN_HANDLER as u64;
        assert!(machine.exec(Instr::Sigaction(Reg(2), Reg(3))).unwrap());
        assert!(
            !machine
                .process()
                .unwrap()
                .signal_handlers
                .contains_key(&SIGNAL_NUMBER_LIMIT)
        );
        assert!(matches!(
            machine.process().unwrap().signal_handlers.get(&2),
            Some(SignalDisposition::Handler(7))
        ));

        machine.thread_mut().unwrap().regs[2] = SIGNAL_NUMBER_LIMIT - 1;
        machine.thread_mut().unwrap().regs[3] = SIG_IGN_HANDLER as u64;
        assert!(machine.exec(Instr::Sigaction(Reg(2), Reg(3))).unwrap());
        assert!(matches!(
            machine
                .process()
                .unwrap()
                .signal_handlers
                .get(&(SIGNAL_NUMBER_LIMIT - 1)),
            Some(SignalDisposition::Ignore)
        ));

        machine.thread_mut().unwrap().regs[4] = 1;
        machine.thread_mut().unwrap().regs[5] = SIGNAL_NUMBER_LIMIT;
        assert!(machine.exec(Instr::Kill(Reg(4), Reg(5))).unwrap());
        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        assert!(machine.process().unwrap().pending_events.is_empty());
    }

    #[test]
    fn invalid_signal_events_do_not_enter_pending_state() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;

        assert!(!machine.queue_process_event(1, NativeEvent::kill_signal(SIGNAL_NUMBER_LIMIT)));
        assert!(machine.process().unwrap().pending_events.is_empty());
        assert_eq!(machine.read_pcr(Pcr::Sigpending).unwrap(), 0);

        assert!(machine.queue_process_event(1, NativeEvent::kill_signal(SIGNAL_NUMBER_LIMIT - 1)));
        assert!(matches!(
            machine.process().unwrap().pending_events.front(),
            Some(NativeEvent::Signal { signum, .. }) if *signum == SIGNAL_NUMBER_LIMIT - 1
        ));
        assert_eq!(
            machine.read_pcr(Pcr::Sigpending).unwrap(),
            1u64 << (SIGNAL_NUMBER_LIMIT - 1)
        );
    }

    #[test]
    fn kill_reports_missing_target_and_full_event_queue() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;

        machine.thread_mut().unwrap().regs[4] = 99;
        machine.thread_mut().unwrap().regs[5] = 2;
        assert!(machine.exec(Instr::Kill(Reg(4), Reg(5))).unwrap());
        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 3);
        assert!(machine.process().unwrap().pending_events.is_empty());

        {
            let process = machine.process_mut().unwrap();
            for _ in 0..PROCESS_EVENT_QUEUE_LIMIT {
                process
                    .pending_events
                    .push_back(NativeEvent::timer_signal(SIGALRM));
            }
        }
        machine.thread_mut().unwrap().regs[4] = 1;
        machine.thread_mut().unwrap().regs[5] = 2;
        assert!(machine.exec(Instr::Kill(Reg(4), Reg(5))).unwrap());
        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 11);
        assert_eq!(
            machine.process().unwrap().pending_events.len(),
            PROCESS_EVENT_QUEUE_LIMIT
        );

        machine.process_mut().unwrap().pending_events.clear();
        machine.set_errno(123).unwrap();
        assert!(machine.exec(Instr::Kill(Reg(4), Reg(5))).unwrap());
        assert_eq!(machine.thread().unwrap().regs[1], 0);
        assert_eq!(machine.process().unwrap().errno, 0);
        assert!(matches!(
            machine.process().unwrap().pending_events.front(),
            Some(NativeEvent::Signal { signum: 2, .. })
        ));
    }

    #[test]
    fn alarm_rejects_locked_result_without_timer_mutation() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.alarms.push((1, 500));
        machine.thread_mut().unwrap().regs[2] = 7;

        let err = machine.exec(Instr::Alarm(Reg(31), Reg(2))).unwrap_err();

        assert!(err.contains("hardware-locked stack pointer"), "{err}");
        assert_eq!(machine.alarms, vec![(1, 500)]);
    }

    #[test]
    fn process_event_queue_rejects_overflow_without_replacing_pending_events() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        {
            let process = machine.process_mut().unwrap();
            for _ in 0..PROCESS_EVENT_QUEUE_LIMIT {
                process
                    .pending_events
                    .push_back(NativeEvent::timer_signal(SIGALRM));
            }
        }

        assert!(!machine.queue_process_event(1, NativeEvent::kill_signal(2)));
        assert_eq!(
            machine.process().unwrap().pending_events.len(),
            PROCESS_EVENT_QUEUE_LIMIT
        );
        assert!(
            machine
                .process()
                .unwrap()
                .pending_events
                .iter()
                .all(|event| matches!(
                    event,
                    NativeEvent::Signal {
                        signum: SIGALRM,
                        source: EventSource::Timer
                    }
                ))
        );
    }

    #[test]
    fn child_exit_signal_respects_parent_event_queue_limit() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine
            .clone_with_profile(CloneProfile::NewProcessCow, Reg(5), None)
            .unwrap();
        let child_pid = machine.thread().unwrap().regs[5];
        let child_tid = machine
            .threads
            .values()
            .find(|thread| thread.pid == child_pid)
            .unwrap()
            .tid;
        {
            let parent = machine.process_mut().unwrap();
            for _ in 0..PROCESS_EVENT_QUEUE_LIMIT {
                parent
                    .pending_events
                    .push_back(NativeEvent::timer_signal(SIGALRM));
            }
        }

        machine.current_tid = child_tid;
        machine.exit_current(7).unwrap();
        machine.current_tid = 1;

        assert_eq!(
            machine.process().unwrap().pending_events.len(),
            PROCESS_EVENT_QUEUE_LIMIT
        );
        assert!(
            !machine
                .process()
                .unwrap()
                .pending_events
                .iter()
                .any(|event| matches!(
                    event,
                    NativeEvent::Signal {
                        signum: SIGCHLD,
                        source: EventSource::ChildExit
                    }
                ))
        );
        assert_eq!(machine.completed_children.get(&(1, child_pid)), Some(&7));
    }

    #[test]
    fn signal_delivery_uses_native_event_queue_before_abi_frame() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine
            .process_mut()
            .unwrap()
            .signal_handlers
            .insert(2, SignalDisposition::Handler(7));

        machine.queue_process_event(1, NativeEvent::kill_signal(2));
        assert!(matches!(
            machine.process().unwrap().pending_events.front(),
            Some(NativeEvent::Signal { signum: 2, .. })
        ));

        machine.deliver_signal_if_needed().unwrap();
        assert!(machine.process().unwrap().pending_events.is_empty());
        assert_eq!(machine.thread().unwrap().ip, 7);
        assert_eq!(machine.thread().unwrap().signal_stack.len(), 1);
    }

    #[test]
    fn masked_signal_remains_pending_until_unmasked() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine
            .process_mut()
            .unwrap()
            .signal_handlers
            .insert(2, SignalDisposition::Handler(7));
        machine.process_mut().unwrap().sigmask = 1 << 2;

        machine.queue_process_event(1, NativeEvent::kill_signal(2));
        machine.deliver_signal_if_needed().unwrap();
        assert_eq!(machine.thread().unwrap().ip, 0);
        assert!(machine.thread().unwrap().signal_stack.is_empty());
        assert!(matches!(
            machine.process().unwrap().pending_events.front(),
            Some(NativeEvent::Signal { signum: 2, .. })
        ));

        machine.process_mut().unwrap().sigmask = 0;
        machine.deliver_signal_if_needed().unwrap();
        assert!(machine.process().unwrap().pending_events.is_empty());
        assert_eq!(machine.thread().unwrap().ip, 7);
        assert_eq!(machine.thread().unwrap().signal_stack.len(), 1);
    }

    #[test]
    fn hardware_fault_signal_bypasses_compatibility_signal_mask() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine
            .process_mut()
            .unwrap()
            .signal_handlers
            .insert(SIGSEGV, SignalDisposition::Handler(9));
        machine.process_mut().unwrap().sigmask = 1 << SIGSEGV;

        machine.queue_process_event(1, NativeEvent::kill_signal(SIGSEGV));
        machine.queue_process_event(1, NativeEvent::fault_signal(SIGSEGV));
        machine.deliver_signal_if_needed().unwrap();

        assert_eq!(machine.thread().unwrap().ip, 9);
        assert_eq!(machine.thread().unwrap().signal_stack.len(), 1);
        assert_eq!(machine.process().unwrap().pending_events.len(), 1);
        assert!(matches!(
            machine.process().unwrap().pending_events.front(),
            Some(NativeEvent::Signal {
                signum: SIGSEGV,
                source: EventSource::Kill,
            })
        ));
    }

    #[test]
    fn ignored_signal_and_default_sigchld_are_consumed_without_frame() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine
            .process_mut()
            .unwrap()
            .signal_handlers
            .insert(2, SignalDisposition::Ignore);

        machine.queue_process_event(1, NativeEvent::kill_signal(2));
        machine.deliver_signal_if_needed().unwrap();
        assert!(machine.process().unwrap().pending_events.is_empty());
        assert_eq!(machine.thread().unwrap().ip, 0);
        assert!(machine.thread().unwrap().signal_stack.is_empty());
        assert!(machine.processes.contains_key(&1));

        machine.queue_process_event(1, NativeEvent::child_signal(SIGCHLD));
        machine.deliver_signal_if_needed().unwrap();
        assert!(machine.process().unwrap().pending_events.is_empty());
        assert_eq!(machine.thread().unwrap().ip, 0);
        assert!(machine.thread().unwrap().signal_stack.is_empty());
        assert!(machine.processes.contains_key(&1));
    }

    #[test]
    fn default_fatal_signal_exits_through_native_event_path() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;

        machine.queue_process_event(1, NativeEvent::fault_signal(SIGSEGV));
        assert!(matches!(
            machine.process().unwrap().pending_events.front(),
            Some(NativeEvent::Signal {
                signum: SIGSEGV,
                ..
            })
        ));

        machine.deliver_signal_if_needed().unwrap();
        assert_eq!(machine.last_exit, 128 + SIGSEGV as i32);
        assert!(!machine.processes.contains_key(&1));
        assert!(!machine.threads.contains_key(&1));
    }

    #[test]
    fn divide_by_zero_queues_sigfpe_fault_event() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[2] = 99;
        machine.thread_mut().unwrap().regs[3] = 0;

        assert!(machine.exec(Instr::Div(Reg(1), Reg(2), Reg(3))).unwrap());

        assert!(matches!(
            machine.process().unwrap().pending_events.front(),
            Some(NativeEvent::Signal {
                signum: SIGFPE,
                source: EventSource::HardwareFault
            })
        ));
        assert_eq!(machine.thread().unwrap().regs[1], 0);
    }

    #[test]
    fn div_rejects_locked_result_before_fault_event() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[2] = 99;
        machine.thread_mut().unwrap().regs[3] = 0;

        let err = machine
            .exec(Instr::Div(Reg(31), Reg(2), Reg(3)))
            .unwrap_err();

        assert!(err.contains("hardware-locked stack pointer"), "{err}");
        assert!(machine.process().unwrap().pending_events.is_empty());
    }

    #[test]
    fn unprivileged_microcode_load_queues_fault_event_without_port_access() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.process_mut().unwrap().uid = 1000;
        let blob = ARG_BASE + 0x1800;
        machine.write_bytes(blob, b"PORT 7 9\n").unwrap();

        machine.thread_mut().unwrap().regs[2] = blob;
        machine.thread_mut().unwrap().regs[3] = 9;
        assert!(machine.exec(Instr::LoadUcode(Reg(2), Reg(3))).unwrap());

        assert!(machine.process().unwrap().ucode_ports.is_empty());
        assert!(matches!(
            machine.process().unwrap().pending_events.front(),
            Some(NativeEvent::Signal {
                signum: SIGSEGV,
                source: EventSource::HardwareFault,
            })
        ));

        machine.thread_mut().unwrap().regs[4] = 7;
        machine.thread_mut().unwrap().regs[5] = 0xaa;
        assert!(machine.exec(Instr::Outb(Reg(4), Reg(5))).unwrap());
        assert_eq!(machine.process().unwrap().pending_events.len(), 2);
        assert!(machine.process().unwrap().ucode_ports.is_empty());

        machine.thread_mut().unwrap().regs[6] = 0xdead_beef;
        assert!(machine.exec(Instr::Inb(Reg(6), Reg(4))).unwrap());
        assert_eq!(machine.process().unwrap().pending_events.len(), 3);
        assert_eq!(machine.thread().unwrap().regs[6], 0xdead_beef);
        assert!(machine.process().unwrap().ucode_ports.is_empty());
    }

    #[test]
    fn raw_port_hooks_require_io_domain_capability() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine
            .domains
            .get_mut(&ROOT_DOMAIN_ID)
            .unwrap()
            .capability_mask &= !DOMAIN_CAP_IO;

        machine.thread_mut().unwrap().regs[1] = 7;
        machine.thread_mut().unwrap().regs[2] = 0xaa;
        let err = machine.exec(Instr::Outb(Reg(1), Reg(2))).unwrap_err();
        assert!(err.contains("resource domain capability denied"), "{err}");
        assert!(machine.process().unwrap().ucode_ports.is_empty());

        machine.thread_mut().unwrap().regs[3] = 0xdead_beef;
        let err = machine.exec(Instr::Inb(Reg(3), Reg(1))).unwrap_err();
        assert!(err.contains("resource domain capability denied"), "{err}");
        assert_eq!(machine.thread().unwrap().regs[3], 0xdead_beef);
        assert!(machine.process().unwrap().ucode_ports.is_empty());

        let blob = ARG_BASE + 0x1a00;
        machine.write_bytes(blob, b"PORT 7 9\n").unwrap();
        machine.thread_mut().unwrap().regs[4] = blob;
        machine.thread_mut().unwrap().regs[5] = 9;
        let err = machine.exec(Instr::LoadUcode(Reg(4), Reg(5))).unwrap_err();
        assert!(err.contains("resource domain capability denied"), "{err}");
        assert!(machine.process().unwrap().ucode_ports.is_empty());
    }

    #[test]
    fn inb_rejects_locked_result_before_fault_or_port_access() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.process_mut().unwrap().uid = 1000;
        machine.thread_mut().unwrap().regs[4] = 7;

        let err = machine.exec(Instr::Inb(Reg(31), Reg(4))).unwrap_err();

        assert!(err.contains("hardware-locked stack pointer"), "{err}");
        assert!(machine.process().unwrap().pending_events.is_empty());
        assert!(machine.process().unwrap().ucode_ports.is_empty());
    }

    #[test]
    fn malformed_microcode_load_preserves_existing_port_state() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.process_mut().unwrap().ucode_ports.insert(1, 2);

        let err = machine
            .load_microcode(b"PORT 7 9\nNOT_A_DIRECTIVE\n")
            .unwrap_err();

        assert!(err.contains("invalid microcode directive"), "{err}");
        assert_eq!(machine.process().unwrap().ucode_ports.get(&1), Some(&2));
        assert!(!machine.process().unwrap().ucode_ports.contains_key(&7));
    }

    #[test]
    fn signal_delivery_defers_nested_frames_until_sigret() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine
            .process_mut()
            .unwrap()
            .signal_handlers
            .insert(2, SignalDisposition::Handler(7));
        machine
            .process_mut()
            .unwrap()
            .signal_handlers
            .insert(3, SignalDisposition::Handler(9));

        machine.queue_process_event(1, NativeEvent::kill_signal(2));
        machine.deliver_signal_if_needed().unwrap();
        assert_eq!(machine.thread().unwrap().ip, 7);
        assert_eq!(machine.thread().unwrap().signal_stack.len(), 1);

        machine.queue_process_event(1, NativeEvent::timer_signal(3));
        machine.deliver_signal_if_needed().unwrap();
        assert_eq!(machine.thread().unwrap().ip, 7);
        assert_eq!(machine.thread().unwrap().signal_stack.len(), 1);
        assert!(matches!(
            machine.process().unwrap().pending_events.front(),
            Some(NativeEvent::Signal { signum: 3, .. })
        ));

        machine.thread_mut().unwrap().signal_stack.clear();
        machine.deliver_signal_if_needed().unwrap();
        assert!(machine.process().unwrap().pending_events.is_empty());
        assert_eq!(machine.thread().unwrap().ip, 9);
        assert_eq!(machine.thread().unwrap().signal_stack.len(), 1);
    }

    #[test]
    fn object_ctl_rejects_invalid_records_and_missing_authority() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let arg = ARG_BASE;

        machine.store_u64(arg, 0xffff).unwrap();
        machine.object_ctl(Reg(2), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[2], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);

        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine.store_u64(arg + 8, 0xffff).unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.object_ctl(Reg(3), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[3], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);

        machine
            .store_u64(arg + 8, ObjectKind::Queue.code())
            .unwrap();
        machine.store_u64(arg + 16, 0xffff).unwrap();
        machine.object_ctl(Reg(4), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);

        machine
            .domains
            .get_mut(&ROOT_DOMAIN_ID)
            .unwrap()
            .capability_mask &= !DOMAIN_CAP_OBJECT;
        machine
            .store_u64(arg + 16, ObjectProfile::Pipe.code())
            .unwrap();
        machine.object_ctl(Reg(5), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[5], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);

        machine
            .domains
            .get_mut(&ROOT_DOMAIN_ID)
            .unwrap()
            .capability_mask = u64::MAX & !DOMAIN_CAP_FDR;
        machine.object_ctl(Reg(6), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[6], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
    }

    #[test]
    fn object_ctl_eventfd_rejects_unknown_flags_without_installing_fd() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let arg = ARG_BASE;

        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::Counter.code())
            .unwrap();
        machine
            .store_u64(arg + 16, ObjectProfile::EventFd.code())
            .unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine.store_u64(arg + 40, 1).unwrap();
        machine.store_u64(arg + 48, 1 << 4).unwrap();

        machine.object_ctl(Reg(2), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[2], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        assert!(matches!(
            machine.process().unwrap().fds[7],
            FdHandle::Closed
        ));
    }

    #[test]
    fn object_ctl_requested_fd_replacement_releases_old_file_locks() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("lnp64_lock_replace_{unique}"));
        fs::write(&path, b"locked").unwrap();

        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        {
            let process = machine.process_mut().unwrap();
            process.fds[7] = FdHandle::File(
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(&path)
                    .unwrap(),
            );
            process.fd_capabilities[7] = FdCapability::full(7);
        }

        let lock = ARG_BASE;
        machine.store_u64(lock, 1).unwrap();
        machine.fcntl_fd_index(7, 6, lock).unwrap();
        assert_eq!(machine.process().unwrap().errno, 0);
        assert_eq!(machine.advisory_locks.len(), 1);

        let arg = ARG_BASE + 0x100;
        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::Counter.code())
            .unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine.store_u64(arg + 40, 123).unwrap();
        machine.object_ctl(Reg(2), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[2], 7);
        assert_eq!(machine.process().unwrap().errno, 0);
        assert!(machine.advisory_locks.is_empty());
        assert!(matches!(
            machine.process().unwrap().fds[7],
            FdHandle::Counter(_)
        ));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn cap_dup_replacement_releases_old_file_locks() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("lnp64_cap_dup_lock_replace_{unique}"));
        fs::write(&path, b"locked").unwrap();

        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        {
            let process = machine.process_mut().unwrap();
            process.fds[5] = FdHandle::Counter(Rc::new(RefCell::new(99)));
            process.fd_capabilities[5] = FdCapability::full(5);
            process.fds[7] = FdHandle::File(
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(&path)
                    .unwrap(),
            );
            process.fd_capabilities[7] = FdCapability::full(7);
        }

        let lock = ARG_BASE;
        machine.store_u64(lock, 1).unwrap();
        machine.fcntl_fd_index(7, 6, lock).unwrap();
        assert_eq!(machine.advisory_locks.len(), 1);

        let arg = ARG_BASE + 0x100;
        machine.store_u64(arg, 5).unwrap();
        machine.store_u64(arg + 8, 7).unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_dup(Reg(2), arg).unwrap();

        assert_eq!(
            machine.thread().unwrap().regs[2],
            machine.fd_token(7).unwrap()
        );
        assert!(machine.advisory_locks.is_empty());
        assert!(matches!(
            machine.process().unwrap().fds[7],
            FdHandle::Counter(_)
        ));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn cap_dup_same_fd_preserves_existing_file_locks() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("lnp64_cap_dup_same_lock_{unique}"));
        fs::write(&path, b"locked").unwrap();

        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        {
            let process = machine.process_mut().unwrap();
            process.fds[7] = FdHandle::File(
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(&path)
                    .unwrap(),
            );
            process.fd_capabilities[7] = FdCapability::full(7);
        }

        let lock = ARG_BASE;
        machine.store_u64(lock, 1).unwrap();
        machine.fcntl_fd_index(7, 6, lock).unwrap();
        assert_eq!(machine.advisory_locks.len(), 1);

        let arg = ARG_BASE + 0x100;
        machine.store_u64(arg, 7).unwrap();
        machine.store_u64(arg + 8, 7).unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_dup(Reg(2), arg).unwrap();

        assert_eq!(
            machine.thread().unwrap().regs[2],
            machine.fd_token(7).unwrap()
        );
        assert_eq!(machine.advisory_locks.len(), 1);
        assert!(matches!(
            machine.process().unwrap().fds[7],
            FdHandle::File(_)
        ));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn cap_recv_replacement_releases_old_file_locks() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("lnp64_cap_recv_lock_replace_{unique}"));
        fs::write(&path, b"locked").unwrap();

        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        create_pipe_pair(&mut machine, 3, 4);
        {
            let process = machine.process_mut().unwrap();
            process.fds[5] = FdHandle::Counter(Rc::new(RefCell::new(123)));
            process.fd_capabilities[5] = FdCapability::full(5);
            process.fds[7] = FdHandle::File(
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(&path)
                    .unwrap(),
            );
            process.fd_capabilities[7] = FdCapability::full(7);
        }

        let lock = ARG_BASE;
        machine.store_u64(lock, 1).unwrap();
        machine.fcntl_fd_index(7, 6, lock).unwrap();
        assert_eq!(machine.advisory_locks.len(), 1);

        let arg = ARG_BASE + 0x100;
        machine.store_u64(arg, 4).unwrap();
        machine.store_u64(arg + 8, 5).unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_send(Reg(2), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[2], 1);

        machine.store_u64(arg, 3).unwrap();
        machine.store_u64(arg + 8, 7).unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_recv(Reg(6), arg).unwrap();

        assert_eq!(
            machine.thread().unwrap().regs[6],
            machine.fd_token(7).unwrap()
        );
        assert!(machine.advisory_locks.is_empty());
        assert!(matches!(
            machine.process().unwrap().fds[7],
            FdHandle::Counter(_)
        ));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn namespace_root_capability_is_required_for_path_resolution() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.process_mut().unwrap().namespace_root = None;
        assert!(machine.resolve_process_path("Cargo.toml").is_err());
    }

    #[test]
    fn chdir_without_namespace_root_preserves_cwd() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let original_cwd = machine.process().unwrap().cwd.clone();
        machine.process_mut().unwrap().namespace_root = None;
        let path = ARG_BASE + 0x1000;
        machine.write_bytes(path, b".\0").unwrap();
        machine.thread_mut().unwrap().regs[1] = path;

        machine.exec(Instr::ChdirPath(Reg(1))).unwrap();

        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 13);
        assert_eq!(machine.process().unwrap().cwd, original_cwd);
    }

    #[test]
    fn mkdir_without_namespace_root_does_not_create_host_dir() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let host_path = std::env::temp_dir().join(format!("lnp64_mkdir_no_root_{unique}"));
        let _ = fs::remove_dir_all(&host_path);
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.process_mut().unwrap().namespace_root = None;
        let path = ARG_BASE + 0x1000;
        let path_bytes = format!("{}\0", host_path.to_string_lossy());
        machine.write_bytes(path, path_bytes.as_bytes()).unwrap();
        machine.thread_mut().unwrap().regs[1] = path;

        machine.exec(Instr::MkdirPath(Reg(1), Reg(0))).unwrap();

        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 13);
        assert!(!host_path.exists());
        let _ = fs::remove_dir_all(host_path);
    }

    #[test]
    fn unlink_without_namespace_root_does_not_touch_host_file() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let host_path = std::env::temp_dir().join(format!("lnp64_unlink_no_root_{unique}"));
        fs::write(&host_path, b"keep").unwrap();
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.process_mut().unwrap().namespace_root = None;
        let path = ARG_BASE + 0x1000;
        let path_bytes = format!("{}\0", host_path.to_string_lossy());
        machine.write_bytes(path, path_bytes.as_bytes()).unwrap();
        machine.thread_mut().unwrap().regs[1] = path;

        machine.exec(Instr::UnlinkPath(Reg(1))).unwrap();

        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 13);
        assert!(host_path.exists());
        let _ = fs::remove_file(host_path);
    }

    #[test]
    fn rename_without_namespace_root_does_not_touch_host_files() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let old_path = std::env::temp_dir().join(format!("lnp64_rename_no_root_old_{unique}"));
        let new_path = std::env::temp_dir().join(format!("lnp64_rename_no_root_new_{unique}"));
        fs::write(&old_path, b"keep").unwrap();
        let _ = fs::remove_file(&new_path);
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.process_mut().unwrap().namespace_root = None;
        let old_addr = ARG_BASE + 0x1000;
        let new_addr = ARG_BASE + 0x1100;
        let old_bytes = format!("{}\0", old_path.to_string_lossy());
        let new_bytes = format!("{}\0", new_path.to_string_lossy());
        machine.write_bytes(old_addr, old_bytes.as_bytes()).unwrap();
        machine.write_bytes(new_addr, new_bytes.as_bytes()).unwrap();
        machine.thread_mut().unwrap().regs[1] = old_addr;
        machine.thread_mut().unwrap().regs[2] = new_addr;

        machine.exec(Instr::RenamePath(Reg(1), Reg(2))).unwrap();

        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 13);
        assert!(old_path.exists());
        assert!(!new_path.exists());
        let _ = fs::remove_file(old_path);
        let _ = fs::remove_file(new_path);
    }

    #[test]
    fn symlink_without_namespace_root_does_not_create_host_link() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let link_path = std::env::temp_dir().join(format!("lnp64_symlink_no_root_{unique}"));
        let _ = fs::remove_file(&link_path);
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.process_mut().unwrap().namespace_root = None;
        let target_addr = ARG_BASE + 0x1000;
        let link_addr = ARG_BASE + 0x1100;
        let link_bytes = format!("{}\0", link_path.to_string_lossy());
        machine
            .write_bytes(target_addr, b"target-payload\0")
            .unwrap();
        machine
            .write_bytes(link_addr, link_bytes.as_bytes())
            .unwrap();
        machine.thread_mut().unwrap().regs[1] = target_addr;
        machine.thread_mut().unwrap().regs[2] = link_addr;

        machine.exec(Instr::SymlinkPath(Reg(1), Reg(2))).unwrap();

        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 13);
        assert!(!link_path.exists());
        let _ = fs::remove_file(link_path);
    }

    #[test]
    fn hard_link_without_namespace_root_does_not_create_host_link() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let old_path = std::env::temp_dir().join(format!("lnp64_link_no_root_old_{unique}"));
        let new_path = std::env::temp_dir().join(format!("lnp64_link_no_root_new_{unique}"));
        fs::write(&old_path, b"keep").unwrap();
        let _ = fs::remove_file(&new_path);
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.process_mut().unwrap().namespace_root = None;
        let old_addr = ARG_BASE + 0x1000;
        let new_addr = ARG_BASE + 0x1100;
        let old_bytes = format!("{}\0", old_path.to_string_lossy());
        let new_bytes = format!("{}\0", new_path.to_string_lossy());
        machine.write_bytes(old_addr, old_bytes.as_bytes()).unwrap();
        machine.write_bytes(new_addr, new_bytes.as_bytes()).unwrap();
        machine.thread_mut().unwrap().regs[1] = old_addr;
        machine.thread_mut().unwrap().regs[2] = new_addr;
        machine.thread_mut().unwrap().regs[3] = 0;

        machine
            .exec(Instr::LinkPath(Reg(1), Reg(2), Reg(3)))
            .unwrap();

        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 13);
        assert!(old_path.exists());
        assert!(!new_path.exists());
        let _ = fs::remove_file(old_path);
        let _ = fs::remove_file(new_path);
    }

    #[test]
    fn stat_without_namespace_root_does_not_write_output_record() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.process_mut().unwrap().namespace_root = None;
        let path = ARG_BASE + 0x1000;
        let statbuf = ARG_BASE + 0x2000;
        let sentinel = vec![0xa5; LNP64_STAT_RECORD_SIZE];
        machine.write_bytes(path, b"Cargo.toml\0").unwrap();
        machine.write_bytes(statbuf, &sentinel).unwrap();
        machine.thread_mut().unwrap().regs[1] = path;
        machine.thread_mut().unwrap().regs[2] = statbuf;
        machine.thread_mut().unwrap().regs[3] = 0;

        machine
            .exec(Instr::StatPath(Reg(2), Reg(1), Reg(3)))
            .unwrap();

        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 13);
        assert_eq!(
            machine.read_bytes(statbuf, sentinel.len()).unwrap(),
            sentinel
        );
    }

    #[test]
    fn readlink_without_namespace_root_does_not_write_output_buffer() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.process_mut().unwrap().namespace_root = None;
        let path = ARG_BASE + 0x1000;
        let out = ARG_BASE + 0x2000;
        let sentinel = b"sentinel-buffer".to_vec();
        machine.write_bytes(path, b"link\0").unwrap();
        machine.write_bytes(out, &sentinel).unwrap();
        machine.thread_mut().unwrap().regs[1] = path;
        machine.thread_mut().unwrap().regs[2] = out;
        machine.thread_mut().unwrap().regs[3] = sentinel.len() as u64;

        machine
            .exec(Instr::ReadlinkPath(Reg(1), Reg(2), Reg(3)))
            .unwrap();

        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 13);
        assert_eq!(machine.read_bytes(out, sentinel.len()).unwrap(), sentinel);
    }

    #[test]
    fn open_fd_dyn_without_namespace_root_does_not_allocate_fdr() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.process_mut().unwrap().namespace_root = None;
        let generations = machine.process().unwrap().fd_generations.clone();
        let path = ARG_BASE + 0x1000;
        machine.write_bytes(path, b"Cargo.toml\0").unwrap();
        machine.thread_mut().unwrap().regs[1] = path;
        machine.thread_mut().unwrap().regs[2] = 0;

        machine
            .exec(Instr::OpenFdDyn(Reg(3), Reg(1), Reg(2)))
            .unwrap();

        assert_eq!(machine.thread().unwrap().regs[3], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 13);
        assert_eq!(machine.process().unwrap().fd_generations, generations);
        assert!(matches!(
            machine.process().unwrap().fds[3],
            FdHandle::Closed
        ));
    }

    #[test]
    fn openat_dyn_without_namespace_root_does_not_allocate_fdr_or_bypass_with_dir_cap() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.process_mut().unwrap().namespace_root = None;
        let generations = machine.process().unwrap().fd_generations.clone();
        let path = ARG_BASE + 0x1000;
        machine.write_bytes(path, b"Cargo.toml\0").unwrap();
        machine.thread_mut().unwrap().regs[1] = AT_FDCWD_VALUE;
        machine.thread_mut().unwrap().regs[2] = path;
        machine.thread_mut().unwrap().regs[3] = 0;

        machine
            .exec(Instr::OpenAtDyn(Reg(4), Reg(1), Reg(2), Reg(3)))
            .unwrap();

        assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 13);
        assert_eq!(machine.process().unwrap().fd_generations, generations);
        assert!(matches!(
            machine.process().unwrap().fds[3],
            FdHandle::Closed
        ));

        {
            let process = machine.process_mut().unwrap();
            process.fds[10] = FdHandle::Dir {
                path: std::env::current_dir()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                entries: Vec::new(),
                pos: 0,
            };
            process.fd_capabilities[10] = FdCapability::full(10);
        }
        machine.thread_mut().unwrap().regs[1] = 10;

        machine
            .exec(Instr::OpenAtDyn(Reg(5), Reg(1), Reg(2), Reg(3)))
            .unwrap();

        assert_eq!(machine.thread().unwrap().regs[5], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 13);
        assert_eq!(machine.process().unwrap().fd_generations, generations);
        assert!(matches!(
            machine.process().unwrap().fds[3],
            FdHandle::Closed
        ));
    }

    #[test]
    fn open_dir_dyn_without_namespace_root_does_not_allocate_fdr() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.process_mut().unwrap().namespace_root = None;
        let generations = machine.process().unwrap().fd_generations.clone();
        let path = ARG_BASE + 0x1000;
        machine.write_bytes(path, b".\0").unwrap();
        machine.thread_mut().unwrap().regs[1] = path;

        machine
            .exec(Instr::OpenDirDyn(Reg(3), Reg(1), Reg(0)))
            .unwrap();

        assert_eq!(machine.thread().unwrap().regs[3], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 13);
        assert_eq!(machine.process().unwrap().fd_generations, generations);
        assert!(matches!(
            machine.process().unwrap().fds[3],
            FdHandle::Closed
        ));
    }

    #[test]
    fn namespace_rejects_empty_paths_instead_of_bypassing_root() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.process_mut().unwrap().namespace_root = None;
        assert!(machine.resolve_process_path("").is_err());

        let root = PathBuf::from("/tmp/lnp64-ns-root");
        {
            let process = machine.process_mut().unwrap();
            process.namespace_root = Some(root.clone());
            process.cwd = root;
        }
        assert!(machine.resolve_process_path("").is_err());
        assert!(machine.resolve_process_path_no_follow_final("").is_err());
    }

    #[test]
    fn ns_ctl_resolve_requires_namespace_root_capability() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.process_mut().unwrap().namespace_root = None;
        let arg = ARG_BASE + 0x1000;
        let path = ARG_BASE + 0x1100;
        let out = ARG_BASE + 0x1200;
        machine.write_bytes(path, b"Cargo.toml\0").unwrap();
        machine.store_u64(arg, NS_OP_RESOLVE).unwrap();
        machine.store_u64(arg + 8, NS_CTL_VERSION).unwrap();
        machine.store_u64(arg + 16, AT_FDCWD_VALUE).unwrap();
        machine.store_u64(arg + 24, path).unwrap();
        machine.store_u64(arg + 32, out).unwrap();
        machine.store_u64(arg + 40, 256).unwrap();
        machine.store_u64(arg + 48, 0).unwrap();

        machine.ns_ctl(Reg(3), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[3], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 13);
    }

    #[test]
    fn namespace_root_rejects_lexical_escape() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let root = PathBuf::from("/tmp/lnp64-ns-root");
        let process = machine.process_mut().unwrap();
        process.namespace_root = Some(root.clone());
        process.cwd = root.join("subdir");
        assert!(machine.resolve_process_path("../../outside").is_err());
        assert_eq!(
            machine.resolve_process_path("inside").unwrap(),
            "/tmp/lnp64-ns-root/subdir/inside"
        );
        assert_eq!(
            machine.resolve_process_path("/etc/motd").unwrap(),
            "/tmp/lnp64-ns-root/etc/motd"
        );
    }

    #[test]
    fn namespace_root_rejects_symlink_escape() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("lnp64_ns_symlink_{unique}"));
        let root = base.join("root");
        let tmp = root.join("tmp");
        let outside = base.join("outside");
        let outside_file = outside.join("secret");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&tmp).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(&outside_file, b"host secret").unwrap();
        fs::write(tmp.join("inside"), b"inside").unwrap();
        std::os::unix::fs::symlink(&outside_file, tmp.join("secret_link")).unwrap();
        std::os::unix::fs::symlink(&outside, tmp.join("outside_dir")).unwrap();
        std::os::unix::fs::symlink(tmp.join("inside"), tmp.join("inside_link")).unwrap();

        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let process = machine.process_mut().unwrap();
        process.namespace_root = Some(root.clone());
        process.cwd = tmp.clone();

        assert!(machine.resolve_process_path("secret_link").is_err());
        assert!(
            machine
                .resolve_process_path("outside_dir/new_file")
                .is_err()
        );
        assert_eq!(
            machine.resolve_process_path("inside_link").unwrap(),
            tmp.join("inside_link").to_string_lossy()
        );

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn ns_ctl_resolve_uses_directory_capability() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("lnp64_ns_ctl_{unique}"));
        let root = base.join("root");
        let tmp = root.join("tmp");
        let outside = base.join("outside");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&tmp).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(tmp.join("inside"), b"inside").unwrap();

        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        {
            let process = machine.process_mut().unwrap();
            process.namespace_root = Some(root.clone());
            process.cwd = root.clone();
            process.fds[10] = FdHandle::Dir {
                path: tmp.to_string_lossy().into_owned(),
                entries: Vec::new(),
                pos: 0,
            };
            process.fd_capabilities[10] = FdCapability::full(10);
            process.fds[11] = FdHandle::File(File::open(tmp.join("inside")).unwrap());
            process.fd_capabilities[11] = FdCapability::full(11);
        }

        let arg = ARG_BASE + 0x1000;
        let path = ARG_BASE + 0x1100;
        let out = ARG_BASE + 0x1200;
        machine.write_bytes(path, b"inside\0").unwrap();
        machine.store_u64(arg, NS_OP_RESOLVE).unwrap();
        machine.store_u64(arg + 8, NS_CTL_VERSION).unwrap();
        machine.store_u64(arg + 16, 10).unwrap();
        machine.store_u64(arg + 24, path).unwrap();
        machine.store_u64(arg + 32, out).unwrap();
        machine.store_u64(arg + 40, 256).unwrap();
        machine.store_u64(arg + 48, 0).unwrap();
        machine.ns_ctl(Reg(4), arg).unwrap();
        let expected = tmp.join("inside").to_string_lossy().into_owned();
        assert_eq!(machine.thread().unwrap().regs[4], expected.len() as u64);
        assert_eq!(machine.process().unwrap().errno, 0);
        assert_eq!(machine.read_c_string(out).unwrap(), expected);

        machine.write_bytes(path, b"/tmp/inside\0").unwrap();
        machine.ns_ctl(Reg(9), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[9], expected.len() as u64);
        assert_eq!(machine.process().unwrap().errno, 0);
        assert_eq!(machine.read_c_string(out).unwrap(), expected);

        machine.write_bytes(path, b"inside\0").unwrap();
        machine.processes.get_mut(&1).unwrap().fd_capabilities[10].rights &= !CAP_RIGHT_READ;
        machine.ns_ctl(Reg(8), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[8], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
        machine.processes.get_mut(&1).unwrap().fd_capabilities[10].rights |= CAP_RIGHT_READ;

        machine.write_bytes(path, b"../../outside\0").unwrap();
        machine.ns_ctl(Reg(5), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[5], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 13);

        machine.write_bytes(path, b"inside\0").unwrap();
        machine.store_u64(arg + 16, 11).unwrap();
        machine.ns_ctl(Reg(6), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[6], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 20);

        machine.store_u64(arg + 16, 10).unwrap();
        machine.store_u64(arg + 48, 1 << 4).unwrap();
        machine.ns_ctl(Reg(12), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[12], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        machine.store_u64(arg + 48, 0).unwrap();

        machine.store_u64(arg + 8, NS_CTL_VERSION + 1).unwrap();
        machine.ns_ctl(Reg(7), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[7], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn ns_ctl_resolve_nofollow_final_preserves_in_root_symlink_path() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("lnp64_ns_ctl_nofollow_{unique}"));
        let root = base.join("root");
        let tmp = root.join("tmp");
        let outside = base.join("outside");
        let outside_file = outside.join("secret");
        let link = tmp.join("secret_link");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&tmp).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(&outside_file, b"secret").unwrap();
        std::os::unix::fs::symlink(&outside_file, &link).unwrap();

        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        {
            let process = machine.process_mut().unwrap();
            process.namespace_root = Some(root.clone());
            process.cwd = root.clone();
            process.fds[10] = FdHandle::Dir {
                path: tmp.to_string_lossy().into_owned(),
                entries: Vec::new(),
                pos: 0,
            };
            process.fd_capabilities[10] = FdCapability::full(10);
        }

        let arg = ARG_BASE + 0x1000;
        let path = ARG_BASE + 0x1100;
        let out = ARG_BASE + 0x1200;
        machine.write_bytes(path, b"secret_link\0").unwrap();
        machine.write_bytes(out, b"sentinel\0").unwrap();
        machine.store_u64(arg, NS_OP_RESOLVE).unwrap();
        machine.store_u64(arg + 8, NS_CTL_VERSION).unwrap();
        machine.store_u64(arg + 16, 10).unwrap();
        machine.store_u64(arg + 24, path).unwrap();
        machine.store_u64(arg + 32, out).unwrap();
        machine.store_u64(arg + 40, 256).unwrap();
        machine.store_u64(arg + 48, 0).unwrap();

        machine.ns_ctl(Reg(4), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 13);
        assert_eq!(machine.read_c_string(out).unwrap(), "sentinel");

        machine
            .store_u64(arg + 48, NS_RESOLVE_FLAG_NOFOLLOW_FINAL)
            .unwrap();
        machine.ns_ctl(Reg(5), arg).unwrap();
        let expected = link.to_string_lossy().into_owned();
        assert_eq!(machine.thread().unwrap().regs[5], expected.len() as u64);
        assert_eq!(machine.process().unwrap().errno, 0);
        assert_eq!(machine.read_c_string(out).unwrap(), expected);

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn ns_ctl_resolve_failures_do_not_write_output_buffer() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("lnp64_ns_ctl_fail_{unique}"));
        let root = base.join("root");
        let tmp = root.join("tmp");
        let outside = base.join("outside");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&tmp).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(tmp.join("inside"), b"inside").unwrap();

        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        {
            let process = machine.process_mut().unwrap();
            process.namespace_root = Some(root.clone());
            process.cwd = root.clone();
            process.fds[10] = FdHandle::Dir {
                path: tmp.to_string_lossy().into_owned(),
                entries: Vec::new(),
                pos: 0,
            };
            process.fd_capabilities[10] = FdCapability::full(10);
        }

        let arg = ARG_BASE + 0x1000;
        let path = ARG_BASE + 0x1100;
        let out = ARG_BASE + 0x1200;
        machine.write_bytes(path, b"inside\0").unwrap();
        machine.store_u64(arg, NS_OP_RESOLVE).unwrap();
        machine.store_u64(arg + 8, NS_CTL_VERSION).unwrap();
        machine.store_u64(arg + 16, 10).unwrap();
        machine.store_u64(arg + 24, path).unwrap();
        machine.store_u64(arg + 32, out).unwrap();
        machine.store_u64(arg + 40, 4).unwrap();
        machine.store_u64(arg + 48, 0).unwrap();

        machine.write_bytes(out, b"sentinel-a\0").unwrap();
        machine.ns_ctl(Reg(4), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 34);
        assert_eq!(machine.read_c_string(out).unwrap(), "sentinel-a");

        machine.store_u64(arg + 40, 256).unwrap();
        machine.store_u64(arg + 48, 1 << 4).unwrap();
        machine.write_bytes(out, b"sentinel-b\0").unwrap();
        machine.ns_ctl(Reg(5), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[5], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        assert_eq!(machine.read_c_string(out).unwrap(), "sentinel-b");

        machine.store_u64(arg + 48, 0).unwrap();
        machine.processes.get_mut(&1).unwrap().fd_capabilities[10].rights &= !CAP_RIGHT_READ;
        machine.write_bytes(out, b"sentinel-c\0").unwrap();
        machine.ns_ctl(Reg(6), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[6], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
        assert_eq!(machine.read_c_string(out).unwrap(), "sentinel-c");
        machine.processes.get_mut(&1).unwrap().fd_capabilities[10].rights |= CAP_RIGHT_READ;

        machine.write_bytes(path, b"../../outside\0").unwrap();
        machine.write_bytes(out, b"sentinel-d\0").unwrap();
        machine.ns_ctl(Reg(7), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[7], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 13);
        assert_eq!(machine.read_c_string(out).unwrap(), "sentinel-d");

        machine.write_bytes(path, b"inside\0").unwrap();
        machine.store_u64(arg + 48, 0).unwrap();
        let expected = tmp.join("inside").to_string_lossy().into_owned();
        let boundary_out = MEMORY_SIZE as u64 - expected.len() as u64;
        machine.process_mut().unwrap().vmas.push(Vma::anonymous(
            boundary_out,
            expected.len() as u64,
            0b11,
        ));
        machine.store_u64(arg + 32, boundary_out).unwrap();
        machine
            .store_u64(arg + 40, expected.len() as u64 + 1)
            .unwrap();
        machine.write_bytes(boundary_out, b"Z").unwrap();
        machine.ns_ctl(Reg(8), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[8], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 14);
        assert_eq!(machine.read_bytes(boundary_out, 1).unwrap(), b"Z".to_vec());

        machine.store_u64(arg, 99).unwrap();
        machine.store_u64(arg + 32, out).unwrap();
        machine.store_u64(arg + 40, 256).unwrap();
        machine.write_bytes(out, b"sentinel-e\0").unwrap();
        machine.ns_ctl(Reg(9), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[9], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        assert_eq!(machine.read_c_string(out).unwrap(), "sentinel-e");

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn getcwd_path_prevalidates_output_span_before_writing() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let cwd = "/tmp/lnp64_getcwd_boundary";
        machine.process_mut().unwrap().cwd = PathBuf::from(cwd);
        let boundary_out = MEMORY_SIZE as u64 - cwd.len() as u64;
        machine.process_mut().unwrap().vmas.push(Vma::anonymous(
            boundary_out,
            cwd.len() as u64,
            0b11,
        ));
        machine.write_bytes(boundary_out, b"Z").unwrap();
        machine.thread_mut().unwrap().regs[2] = boundary_out;
        machine.thread_mut().unwrap().regs[3] = cwd.len() as u64 + 1;

        machine.exec(Instr::GetcwdPath(Reg(2), Reg(3))).unwrap();

        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 14);
        assert_eq!(machine.read_bytes(boundary_out, 1).unwrap(), b"Z".to_vec());
    }

    #[test]
    fn readdir_write_failure_preserves_directory_position() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        {
            let process = machine.process_mut().unwrap();
            process.fds[10] = FdHandle::Dir {
                path: "/tmp".to_string(),
                entries: vec!["alpha".to_string()],
                pos: 0,
            };
            process.fd_capabilities[10] = FdCapability::full(10);
        }

        let err = machine.readdir_fd_index(10, u64::MAX - 1).unwrap_err();
        assert!(err.contains("unmapped address"), "{err}");
        assert!(matches!(
            machine.process().unwrap().fds.get(10),
            Some(FdHandle::Dir { pos: 0, .. })
        ));

        let out = ARG_BASE + 0x1400;
        machine.readdir_fd_index(10, out).unwrap();
        assert_eq!(machine.read_c_string(out).unwrap(), "alpha");
        assert!(matches!(
            machine.process().unwrap().fds.get(10),
            Some(FdHandle::Dir { pos: 1, .. })
        ));
    }

    #[test]
    fn runs_integer_loop() {
        let program = Program::parse(
            r#"
            .text
              LI r1, 5
              LI r2, 1
            loop:
              LI r3, 1
              CMP r1, r3
              BLE done
              MUL r2, r2, r1
              SUB r1, r1, r3
              JMP loop
            done:
              LI r3, 120
              CMP r2, r3
              BNE bad
              EXIT r0
            bad:
              LI r1, 1
              EXIT r1
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        assert_eq!(machine.run().unwrap(), 0);
    }

    #[test]
    fn exec_installs_process_entry_on_replacement_process() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let child_path = std::env::temp_dir().join(format!("lnp64_exec_entry_{unique}.s"));
        fs::write(
            &child_path,
            r#"
            .text
              LI r10, 0x700000
              LD r2, [r10, 0]
              LI r3, 2
              CMP r2, r3
              BNE bad
              LI r4, 0x700020
              LD r5, [r4, 0]
              CMP r5, r0
              BEQ bad
              LD.B r6, [r5, 0]
              LI r7, 75
              CMP r6, r7
              BNE bad
              LD r8, [r4, 8]
              CMP r8, r0
              BNE bad
              EXIT r0
            bad:
              LI r1, 1
              EXIT r1
            "#,
        )
        .unwrap();
        let child_path = child_path.to_string_lossy();
        let program = Program::parse(&format!(
            r#"
            .data
            path: .string "{child_path}"
            arg0: .string "child"
            arg1: .string "two"
            env0: .string "KEY=value"
            argv: .quad arg0
                  .quad arg1
                  .quad 0
            envp: .quad env0
                  .quad 0

            .text
              LI r1, path
              LI r2, argv
              LI r3, envp
              EXEC r1, r2, r3
              LI r3, 99
              EXIT r3
            "#
        ))
        .unwrap();
        let mut machine = Machine::new(program);
        let result = machine.run();
        let _ = fs::remove_file(child_path.as_ref());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn exec_rejection_preserves_old_image_before_commit() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let bad_path = std::env::temp_dir().join(format!("lnp64_bad_exec_plan_{unique}.s"));
        fs::write(&bad_path, "BAD_OPCODE r1\n").unwrap();
        let bad_path = bad_path.to_string_lossy();

        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let path_addr = ARG_BASE + 0x2000;
        machine.write_bytes(path_addr, bad_path.as_bytes()).unwrap();
        machine
            .write_bytes(path_addr + bad_path.len() as u64, &[0])
            .unwrap();
        machine.write_reg(Reg(1), path_addr).unwrap();
        machine.write_reg(Reg(2), 0).unwrap();
        machine.write_reg(Reg(3), 0).unwrap();
        machine.write_reg(Reg(9), 0xfeed_cafe).unwrap();
        machine.thread_mut().unwrap().ip = 0;

        assert!(machine.exec(Instr::Exec(Reg(1), Reg(2), Reg(3))).unwrap());
        let _ = fs::remove_file(bad_path.as_ref());
        assert_eq!(machine.process().unwrap().errno, 8);
        assert_eq!(machine.read_reg(Reg(1)).unwrap(), -1i64 as u64);
        assert!(matches!(
            machine.process().unwrap().program.instructions.first(),
            Some(Instr::Nop)
        ));
        assert_eq!(machine.thread().unwrap().tid, 1);
        assert_eq!(machine.thread().unwrap().ip, 0);
        assert_eq!(machine.read_reg(Reg(9)).unwrap(), 0xfeed_cafe);
        assert_eq!(machine.load_u64(ARG_BASE).unwrap(), 0);
    }

    #[test]
    fn exec_oversized_entry_metadata_preserves_old_image_before_commit() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let child_path = std::env::temp_dir().join(format!("lnp64_big_exec_argv_{unique}.s"));
        fs::write(&child_path, ".text\n  EXIT r0\n").unwrap();
        let child_path = child_path.to_string_lossy();

        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let path_addr = ARG_BASE + 0x2000;
        machine
            .write_bytes(path_addr, child_path.as_bytes())
            .unwrap();
        machine
            .write_bytes(path_addr + child_path.len() as u64, &[0])
            .unwrap();
        let oversized = machine.alloc_heap(ARG_SIZE as usize + 1, 8, false).unwrap();
        machine
            .write_bytes(oversized, &vec![b'x'; ARG_SIZE as usize])
            .unwrap();
        machine.write_bytes(oversized + ARG_SIZE, &[0]).unwrap();
        let argv = ARG_BASE + 0x100;
        machine.store_u64(argv, oversized).unwrap();
        machine.store_u64(argv + 8, 0).unwrap();
        machine.write_reg(Reg(1), path_addr).unwrap();
        machine.write_reg(Reg(2), argv).unwrap();
        machine.write_reg(Reg(3), 0).unwrap();
        machine.write_reg(Reg(9), 0xfeed_cafe).unwrap();
        machine.thread_mut().unwrap().ip = 0;

        let err = machine
            .exec(Instr::Exec(Reg(1), Reg(2), Reg(3)))
            .unwrap_err();

        let _ = fs::remove_file(child_path.as_ref());
        assert!(
            err.contains("argv data exceeds emulated argument page"),
            "{err}"
        );
        assert!(matches!(
            machine.process().unwrap().program.instructions.first(),
            Some(Instr::Nop)
        ));
        assert_eq!(machine.thread().unwrap().tid, 1);
        assert_eq!(machine.thread().unwrap().ip, 0);
        assert_eq!(machine.read_reg(Reg(9)).unwrap(), 0xfeed_cafe);
    }

    #[test]
    fn runs_system_primitive_subset() {
        let program = Program::parse(
            r#"
            .data
            pipe_msg: .string "hi"
            dup_msg: .string "!"
            obj_arg: .zero 64

            .text
              GET_PCR r1, PID
              LI r2, 1
              CMP r1, r2
              BNE bad

              LI r3, 16
              ALLOC r4, r3
              CMP r4, r0
              BEQ bad

              LI r5, 41
              ST [r4, 0], r5
              LI r6, 41
              LI r7, 42
              LOCK.CMPXCHG r8, r4, r6, r7
              LD r9, [r4, 0]
              CMP r9, r7
              BNE bad

              MSG_SEND r1, r6, r7
              AWAIT r0, fd255, r0
              PULL r10, fd255, r0, r0
              MOV r11, r30
              CMP r10, r6
              BNE bad
              CMP r11, r7
              BNE bad

              LI r12, pipe_msg
              LI r13, 2
              LI r18, obj_arg
              LI r19, 1
              ST [r18, 0], r19
              LI r19, 2
              ST [r18, 8], r19
              LI r19, 1
              ST [r18, 16], r19
              LI r19, 3
              ST [r18, 24], r19
              LI r19, 4
              ST [r18, 32], r19
              OBJECT_CTL r19, r18
              CMP r19, r0
              BNE bad
              PUSH r19, fd4, r12, r13
              LI r14, 2
              ALLOC r15, r14
              PULL r19, fd3, r15, r14
              CMP r19, r14
              BNE bad
              LD.B r16, [r15, 0]
              LI r17, 104
              CMP r16, r17
              BNE bad
              LD.B r16, [r15, 1]
              LI r17, 105
              CMP r16, r17
              BNE bad

              FD_DUP2 fd5, fd4
              CMP r1, r0
              BNE bad
              LI r12, dup_msg
              LI r13, 1
              WRITE_FD fd5, r12, r13
              READ_FD fd3, r15, r13
              CMP r1, r13
              BNE bad
              LD.B r16, [r15, 0]
              LI r17, 33
              CMP r16, r17
              BNE bad
              FREE r15

              LI r18, 0
              WAIT_PID r19, r18
              CMP r1, r0
              BNE bad
              FREE r4
              EXIT r0
            bad:
              LI r1, 1
              EXIT r1
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        assert_eq!(machine.run().unwrap(), 0);
    }

    #[test]
    fn offset_io_uses_explicit_file_offsets() {
        let path = format!("/tmp/lnp64_offset_io_{}.txt", std::process::id());
        fs::write(&path, b"abcdef").unwrap();

        let program = Program::parse(&format!(
            r#"
            .data
            path: .string "{path}"
            patch: .string "XY"

            .text
              LI r1, path
              LI r2, 4
              OPEN_FD fd3, r1, r2
              CMP r1, r0
              BNE bad

              LI r3, patch
              LI r4, 2
              LI r5, 2
              PWRITE_FD fd3, r3, r4, r5
              CMP r1, r4
              BNE bad

              LI r6, 6
              ALLOC r7, r6
              PREAD_FD fd3, r7, r6, r0
              CMP r1, r6
              BNE bad
              LD.B r8, [r7, 0]
              LI r9, 97
              CMP r8, r9
              BNE bad
              LD.B r8, [r7, 2]
              LI r9, 88
              CMP r8, r9
              BNE bad
              LD.B r8, [r7, 3]
              LI r9, 89
              CMP r8, r9
              BNE bad
              LD.B r8, [r7, 5]
              LI r9, 102
              CMP r8, r9
              BNE bad
              FREE r7
              EXIT r0
            bad:
              LI r1, 1
              EXIT r1
            "#,
        ))
        .unwrap();
        let mut machine = Machine::new(program);
        assert_eq!(machine.run().unwrap(), 0);
        assert_eq!(fs::read(&path).unwrap(), b"abXYef");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn timestamp_instructions_update_file_metadata() {
        let path = format!("/tmp/lnp64_utime_{}.txt", std::process::id());
        fs::write(&path, b"time").unwrap();

        let program = Program::parse(&format!(
            r#"
            .data
            path: .string "{path}"
            times: .quad 1
                   .quad 0
                   .quad 1
                   .quad 0

            .text
              LI r10, path
              LI r11, times
              LI r12, 0
              UTIME_PATH r10, r11, r12
              CMP r1, r0
              BNE bad

              LI r13, 4
              OPEN_FD fd3, r10, r13
              CMP r1, r0
              BNE bad

              UTIME_FD fd3, r11
              CMP r1, r0
              BNE bad

              LI r14, 3
              UTIME_FD_DYN r14, r11
              CMP r1, r0
              BNE bad

              EXIT r0
            bad:
              LI r1, 1
              EXIT r1
            "#,
        ))
        .unwrap();
        let mut machine = Machine::new(program);
        assert_eq!(machine.run().unwrap(), 0);
        let metadata = fs::metadata(&path).unwrap();
        assert_eq!(metadata.mtime(), 1);
        assert_eq!(metadata.mtime_nsec(), 0);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn store_u64_offset_rejects_address_overflow() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;

        let err = machine
            .store_u64_offset(u64::MAX - 4, 8, 0xfeed)
            .unwrap_err();

        assert!(err.contains("address overflow"), "{err}");
    }

    #[test]
    fn load_u64_offset_rejects_address_overflow() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;

        let err = machine.load_u64_offset(u64::MAX - 4, 8).unwrap_err();

        assert!(err.contains("address overflow"), "{err}");
    }

    #[test]
    fn checked_record_base_rejects_address_overflow() {
        let add_err = Machine::checked_record_base(u64::MAX - 8, 2, 8).unwrap_err();
        assert!(add_err.contains("address overflow"), "{add_err}");

        let mul_err = Machine::checked_record_base(0, u64::MAX, 2).unwrap_err();
        assert!(mul_err.contains("address overflow"), "{mul_err}");
    }

    #[test]
    fn write_bytes_offset_rejects_address_overflow() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;

        let err = machine
            .write_bytes_offset(u64::MAX - 1, 2, &[0xaa])
            .unwrap_err();

        assert!(err.contains("address overflow"), "{err}");
    }

    #[test]
    fn fork_exec_spawn_signal_futex_mmap_and_microcode_execute() {
        let exec_path = "/tmp/lnp64_exec_test.s";
        fs::write(
            exec_path,
            r#"
            .text
              LI r1, 0
              EXIT r1
            "#,
        )
        .unwrap();

        let program = Program::parse(&format!(
            r#"
            .data
            exec_path: .string "{exec_path}"
            ucode: .string "PORT 9 123\n"
            .text
              LI r1, handler
              LI r2, 10
              SIGACTION r2, r1
              GET_PCR r3, PID
              KILL r3, r2
              YIELD
              LD r20, sig_flag
              LI r4, 1
              CMP r20, r4
              LI r1, 2
              BNE bad

              LI r5, 16
              LI r25, 3
              MMAP r6, r0, r5, r25, fd0, r0
              LI r7, 99
              ST [r6, 0], r7
              LD r8, [r6, 0]
              CMP r8, r7
              LI r1, 3
              BNE bad

              LI r9, ucode
              LI r10, 11
              LOAD_UCODE r9, r10
              LI r11, 9
              INB r12, r11
              LI r13, 123
              CMP r12, r13
              LI r1, 4
              BNE bad

              LI r14, futex_word
              LI r15, 0
              LI r16, waiter
              SPAWN r17, r16
              YIELD
              LI r18, 1
              ST [r14, 0], r18
              FUTEX_WAKE r14, r18
              LI r26, 3
              SLEEP r26
              LD r19, [r14, 0]
              LI r21, 2
              CMP r19, r21
              MOV r1, r19
              BNE bad

              FORK r22
              CMP r22, r0
              BEQ child
              YIELD
              LI r23, exec_path
              EXEC r23, r0
            child:
              LI r24, 0
              EXIT r24

            waiter:
              FUTEX_WAIT r14, r15
              LI r18, 2
              ST [r14, 0], r18
              EXIT r0

            handler:
              LI r20, 1
              ST sig_flag, r20
              SIGRET

            bad:
              EXIT r1

            .data
            futex_word: .quad 0
            sig_flag: .quad 0
            "#
        ))
        .unwrap();
        let mut machine = Machine::new(program);
        assert_eq!(machine.run().unwrap(), 0);
        let _ = fs::remove_file(exec_path);
    }

    #[test]
    fn domain_ctl_manages_nested_resource_domains() {
        let program = Program::parse(
            r#"
            .data
            arg: .zero 208

            .text
              LI r10, arg
              LI r12, -1

              LI r1, 1
              ST [r10, 0], r1
              ST [r10, 8], r0
              ST [r10, 16], r0
              LI r1, 1
              ST [r10, 24], r1
              LI r1, 100
              ST [r10, 32], r1
              LI r1, 6000000
              ST [r10, 40], r1
              LI r1, 4
              ST [r10, 48], r1
              LI r1, 16
              ST [r10, 56], r1
              LI r1, 15
              ST [r10, 64], r1
              LI r1, 7
              ST [r10, 72], r1
              DOMAIN_CTL r20, r10
              CMP r20, r12
              BEQ bad

              LI r1, 1
              ST [r10, 0], r1
              ST [r10, 8], r20
              LI r1, 1
              ST [r10, 16], r1
              ST [r10, 24], r1
              LI r1, 40
              ST [r10, 32], r1
              LI r1, 5500000
              ST [r10, 40], r1
              LI r1, 2
              ST [r10, 48], r1
              LI r1, 8
              ST [r10, 56], r1
              LI r1, 3
              ST [r10, 64], r1
              LI r1, 1
              ST [r10, 72], r1
              DOMAIN_CTL r21, r10
              CMP r21, r12
              BEQ bad

              LI r1, 3
              ST [r10, 0], r1
              ST [r10, 8], r21
              LI r1, 1
              ST [r10, 16], r1
              DOMAIN_CTL r22, r10
              LI r1, 200
              CMP r22, r1
              BNE bad
              LD r23, [r10, 120]
              CMP r23, r20
              BNE bad
              LD r23, [r10, 128]
              LI r1, 2
              CMP r23, r1
              BNE bad

              LI r1, 1
              ST [r10, 0], r1
              ST [r10, 8], r21
              LI r1, 1
              ST [r10, 16], r1
              ST [r10, 24], r1
              LI r1, 41
              ST [r10, 32], r1
              LI r1, 5500000
              ST [r10, 40], r1
              LI r1, 2
              ST [r10, 48], r1
              LI r1, 8
              ST [r10, 56], r1
              LI r1, 3
              ST [r10, 64], r1
              LI r1, 1
              ST [r10, 72], r1
              DOMAIN_CTL r24, r10
              CMP r24, r12
              BNE bad

              LI r1, 4
              ST [r10, 0], r1
              ST [r10, 8], r21
              LI r1, 1
              ST [r10, 16], r1
              DOMAIN_CTL r24, r10
              CMP r24, r0
              BNE bad
              LI r1, 3
              ST [r10, 0], r1
              DOMAIN_CTL r24, r10
              LD r25, [r10, 112]
              LI r1, 1
              CMP r25, r1
              BNE bad
              LI r1, 5
              ST [r10, 0], r1
              DOMAIN_CTL r24, r10
              CMP r24, r0
              BNE bad

              LI r1, 7
              ST [r10, 0], r1
              ST [r10, 8], r20
              LI r1, 1
              ST [r10, 16], r1
              DOMAIN_CTL r24, r10
              CMP r24, r0
              BNE bad
              LI r1, 3
              ST [r10, 0], r1
              ST [r10, 8], r20
              DOMAIN_CTL r24, r10
              LD r25, [r10, 96]
              LI r1, 1
              CMP r25, r1
              BNE bad
              LI r1, 8
              ST [r10, 0], r1
              DOMAIN_CTL r24, r10
              LI r1, 1
              CMP r24, r1
              BNE bad

              LI r1, 6
              ST [r10, 0], r1
              ST [r10, 8], r21
              LI r1, 1
              ST [r10, 16], r1
              DOMAIN_CTL r24, r10
              CMP r24, r0
              BNE bad
              LI r1, 3
              ST [r10, 0], r1
              DOMAIN_CTL r24, r10
              CMP r24, r12
              BNE bad

              EXIT r0
            bad:
              LI r1, 1
              EXIT r1
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        assert_eq!(machine.run().unwrap(), 0);
    }

    #[test]
    fn domain_limits_are_enforced_by_ordinary_operations() {
        let program = Program::parse(
            r#"
            .data
            arg: .zero 208
            obj: .zero 80

            .text
              LI r10, arg
              LI r11, -1

              LI r1, 1
              ST [r10, 0], r1
              ST [r10, 8], r0
              ST [r10, 16], r0
              LI r1, 4
              ST [r10, 24], r1
              LI r1, 1000
              ST [r10, 32], r1
              LI r1, 5000000
              ST [r10, 40], r1
              LI r1, 1
              ST [r10, 48], r1
              LI r1, 5
              ST [r10, 56], r1
              LI r1, 63
              ST [r10, 64], r1
              ST [r10, 72], r1
              DOMAIN_CTL r20, r10
              CMP r20, r11
              BEQ bad

              LI r1, 7
              ST [r10, 0], r1
              ST [r10, 8], r20
              LI r1, 1
              ST [r10, 16], r1
              DOMAIN_CTL r21, r10
              CMP r21, r0
              BNE bad

              LI r1, 3
              ST [r10, 0], r1
              DOMAIN_CTL r21, r10
              LD r22, [r10, 88]
              LD r23, [r10, 104]

              LI r1, 64
              ALLOC r24, r1
              CMP r24, r11
              BEQ bad
              LI r1, 3
              ST [r10, 0], r1
              DOMAIN_CTL r21, r10
              LD r25, [r10, 88]
              CMP r25, r22
              BLE bad
              FREE r24
              LI r1, 3
              ST [r10, 0], r1
              DOMAIN_CTL r21, r10
              LD r25, [r10, 88]
              CMP r25, r22
              BNE bad

              LI r1, 1000000
              ALLOC r24, r1
              CMP r24, r11
              BNE bad

              LI r1, worker
              SPAWN r24, r1
              CMP r24, r11
              BNE bad

              LI r12, obj
              LI r1, 1
              ST [r12, 0], r1
              LI r1, 2
              ST [r12, 8], r1
              LI r1, 1
              ST [r12, 16], r1
              LI r1, 3
              ST [r12, 24], r1
              LI r1, 4
              ST [r12, 32], r1
              OBJECT_CTL r24, r12
              CMP r24, r0
              BNE bad
              LI r1, 3
              ST [r10, 0], r1
              DOMAIN_CTL r21, r10
              LD r25, [r10, 104]
              CMP r25, r23
              BLE bad

              FD_DUP2 fd5, fd4
              CMP r1, r11
              BNE bad
              FD_CLOSE fd3
              FD_CLOSE fd4
              LI r1, 3
              ST [r10, 0], r1
              DOMAIN_CTL r21, r10
              LD r25, [r10, 104]
              CMP r25, r23
              BNE bad

              EXIT r0

            worker:
              EXIT r0
            bad:
              LI r1, 1
              EXIT r1
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        assert_eq!(machine.run().unwrap(), 0);
    }

    #[test]
    fn domain_capability_scope_blocks_ordinary_ops() {
        let program = Program::parse(
            r#"
            .data
            arg: .zero 208

            .text
              LI r10, arg
              LI r1, 1
              ST [r10, 0], r1
              ST [r10, 8], r0
              ST [r10, 16], r0
              LI r1, 4
              ST [r10, 24], r1
              LI r1, 1000
              ST [r10, 32], r1
              LI r1, 5000000
              ST [r10, 40], r1
              LI r1, 1
              ST [r10, 48], r1
              LI r1, 8
              ST [r10, 56], r1
              LI r1, 13
              ST [r10, 64], r1
              ST [r10, 72], r1
              DOMAIN_CTL r20, r10

              LI r1, 7
              ST [r10, 0], r1
              ST [r10, 8], r20
              LI r1, 1
              ST [r10, 16], r1
              DOMAIN_CTL r21, r10

              LI r1, 64
              ALLOC r22, r1
              EXIT r0
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        let err = machine.run().unwrap_err();
        assert!(err.contains("resource domain capability denied"), "{err}");
    }

    #[test]
    fn domain_ctl_rejects_unknown_opcode_without_side_effects() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let arg = ARG_BASE;
        let next_domain_id = machine.next_domain_id;
        let domain_count = machine.domains.len();
        let root_children = machine.domains[&ROOT_DOMAIN_ID].children.clone();
        machine.store_u64(arg, 0xfeed_beef).unwrap();
        machine.store_u64(arg + 8, 0x1111).unwrap();
        machine.store_u64(arg + 16, 0x2222).unwrap();
        machine.set_errno(123).unwrap();

        machine.domain_ctl(Reg(5), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[5], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        assert_eq!(machine.next_domain_id, next_domain_id);
        assert_eq!(machine.domains.len(), domain_count);
        assert_eq!(machine.domains[&ROOT_DOMAIN_ID].children, root_children);
        assert_eq!(machine.load_u64(arg).unwrap(), 0xfeed_beef);
        assert_eq!(machine.load_u64(arg + 8).unwrap(), 0x1111);
        assert_eq!(machine.load_u64(arg + 16).unwrap(), 0x2222);
    }

    #[test]
    fn domain_security_policy_delegation_is_monotonic() {
        let mut machine = test_machine_with_child_domain();
        let arg = ARG_BASE;

        machine.store_u64(arg, DOMAIN_OP_CREATE).unwrap();
        machine.store_u64(arg + 8, ROOT_DOMAIN_ID).unwrap();
        machine.store_u64(arg + 16, 1).unwrap();
        machine
            .store_u64(arg + DOMAIN_SECURITY_ALLOW_WX, DOMAIN_BOOL_ENABLE)
            .unwrap();
        assert_eq!(machine.domain_ctl_create(arg), Err(1));

        machine
            .store_u64(arg + DOMAIN_SECURITY_ALLOW_WX, DOMAIN_BOOL_INHERIT)
            .unwrap();
        machine
            .store_u64(arg + DOMAIN_SECURITY_ASLR_ENABLED, DOMAIN_BOOL_DISABLE)
            .unwrap();
        assert_eq!(machine.domain_ctl_create(arg), Err(1));

        machine.domains.get_mut(&2).unwrap().security.aslr_enabled = false;
        machine.store_u64(arg + 8, 2).unwrap();
        machine
            .store_u64(arg + DOMAIN_SECURITY_ASLR_ENABLED, DOMAIN_BOOL_DISABLE)
            .unwrap();
        let id = machine.domain_ctl_create(arg).unwrap();
        assert!(!machine.domains[&id].security.aslr_enabled);

        machine.store_u64(arg, DOMAIN_OP_CONFIGURE).unwrap();
        machine.store_u64(arg + 8, 2).unwrap();
        machine.store_u64(arg + 16, 1).unwrap();
        machine.domains.get_mut(&2).unwrap().frozen = true;
        assert_eq!(machine.domain_ctl_configure(arg), Err(11));
    }

    #[test]
    fn domain_create_prevalidates_output_record_before_mutation() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let arg = ARG_BASE;
        machine.store_u64(arg, DOMAIN_OP_CREATE).unwrap();
        machine.store_u64(arg + 8, ROOT_DOMAIN_ID).unwrap();
        machine.store_u64(arg + 16, 1).unwrap();
        let next_domain_id = machine.next_domain_id;
        let domain_count = machine.domains.len();
        let parent_children = machine.domains[&ROOT_DOMAIN_ID].children.clone();
        {
            let process = machine.process_mut().unwrap();
            let vma = process
                .vmas
                .iter_mut()
                .find(|vma| vma.contains(arg, DOMAIN_QUERY_SIZE as usize))
                .expect("domain record VMA");
            vma.prot = 0b01;
        }

        assert_eq!(machine.domain_ctl_create(arg), Err(14));

        assert_eq!(machine.next_domain_id, next_domain_id);
        assert_eq!(machine.domains.len(), domain_count);
        assert_eq!(machine.domains[&ROOT_DOMAIN_ID].children, parent_children);
        assert!(!machine.domains.contains_key(&next_domain_id));
    }

    #[test]
    fn domain_security_rejects_invalid_boolean_selectors_without_mutation() {
        let mut machine = test_machine_with_child_domain();
        let arg = ARG_BASE;
        let next_domain_id = machine.next_domain_id;

        machine.store_u64(arg, DOMAIN_OP_CREATE).unwrap();
        machine.store_u64(arg + 8, ROOT_DOMAIN_ID).unwrap();
        machine.store_u64(arg + 16, 1).unwrap();
        machine
            .store_u64(arg + DOMAIN_SECURITY_ASLR_ENABLED, 99)
            .unwrap();
        assert_eq!(machine.domain_ctl_create(arg), Err(22));
        assert_eq!(machine.next_domain_id, next_domain_id);
        assert!(!machine.domains.contains_key(&next_domain_id));

        machine
            .store_u64(arg + DOMAIN_SECURITY_ASLR_ENABLED, DOMAIN_BOOL_INHERIT)
            .unwrap();
        machine.store_u64(arg, DOMAIN_OP_CONFIGURE).unwrap();
        machine.store_u64(arg + 8, 2).unwrap();
        machine.store_u64(arg + 16, 1).unwrap();
        machine
            .store_u64(arg + DOMAIN_SECURITY_DMA_ALLOWED, 99)
            .unwrap();
        let before = machine.domains[&2].security;
        assert_eq!(machine.domain_ctl_configure(arg), Err(22));
        let after = machine.domains[&2].security;
        assert_eq!(after.dma_allowed, before.dma_allowed);
        assert_eq!(after.aslr_enabled, before.aslr_enabled);
        assert_eq!(after.allow_wx, before.allow_wx);
        assert_eq!(after.allow_jit_transition, before.allow_jit_transition);
        assert_eq!(after.entropy_quota, before.entropy_quota);
        assert_eq!(after.hardening_profile, before.hardening_profile);
        assert_eq!(
            after.executable_source_policy,
            before.executable_source_policy
        );
    }

    #[test]
    fn domain_query_prevalidates_full_output_record_before_writing() {
        let mut machine = test_machine_with_child_domain();
        let arg = MEMORY_SIZE as u64 - 32;
        machine
            .process_mut()
            .unwrap()
            .vmas
            .push(Vma::anonymous(arg, 32, 0b11));
        machine.store_u64(arg + 8, ROOT_DOMAIN_ID).unwrap();
        machine
            .store_u64(arg + 16, machine.domains[&ROOT_DOMAIN_ID].generation)
            .unwrap();
        machine.store_u64(arg + 24, 0xfeed_face).unwrap();

        assert_eq!(machine.domain_ctl_query(arg), Err(14));
        assert_eq!(machine.load_u64(arg + 24).unwrap(), 0xfeed_face);
    }

    #[test]
    fn domain_configure_rejects_unauthorized_masks_without_mutation() {
        let mut machine = test_machine_with_child_domain();
        let arg = ARG_BASE;

        {
            let domain = machine.domains.get_mut(&2).unwrap();
            domain.profile = 4;
            domain.limits = DomainLimits {
                cpu: 100,
                memory: 200,
                pids: 3,
                fdrs: 8,
            };
            domain.capability_mask = DOMAIN_CAP_PROCESS | DOMAIN_CAP_MEMORY;
            domain.upcall_mask = 0b0011;
            domain.security.entropy_quota = 64;
        }

        let before_profile = machine.domains[&2].profile;
        let before_limits = machine.domains[&2].limits;
        let before_caps = machine.domains[&2].capability_mask;
        let before_upcalls = machine.domains[&2].upcall_mask;
        let before_security = machine.domains[&2].security;
        let assert_domain_unchanged = |domain: &ResourceDomain, before_limits: DomainLimits| {
            assert_eq!(domain.profile, before_profile);
            assert_eq!(domain.limits.cpu, before_limits.cpu);
            assert_eq!(domain.limits.memory, before_limits.memory);
            assert_eq!(domain.limits.pids, before_limits.pids);
            assert_eq!(domain.limits.fdrs, before_limits.fdrs);
            assert_eq!(domain.capability_mask, before_caps);
            assert_eq!(domain.upcall_mask, before_upcalls);
            assert_eq!(domain.security.aslr_enabled, before_security.aslr_enabled);
            assert_eq!(domain.security.allow_wx, before_security.allow_wx);
            assert_eq!(
                domain.security.allow_jit_transition,
                before_security.allow_jit_transition
            );
            assert_eq!(domain.security.entropy_quota, before_security.entropy_quota);
            assert_eq!(domain.security.dma_allowed, before_security.dma_allowed);
            assert_eq!(
                domain.security.hardening_profile,
                before_security.hardening_profile
            );
            assert_eq!(
                domain.security.executable_source_policy,
                before_security.executable_source_policy
            );
        };

        machine
            .domains
            .get_mut(&ROOT_DOMAIN_ID)
            .unwrap()
            .capability_mask = DOMAIN_CAP_PROCESS;
        machine.store_u64(arg, DOMAIN_OP_CONFIGURE).unwrap();
        machine.store_u64(arg + 8, 2).unwrap();
        machine.store_u64(arg + 16, 1).unwrap();
        machine.store_u64(arg + 24, 99).unwrap();
        machine.store_u64(arg + 32, 50).unwrap();
        machine.store_u64(arg + 40, 150).unwrap();
        machine.store_u64(arg + 48, 2).unwrap();
        machine.store_u64(arg + 56, 4).unwrap();
        machine
            .store_u64(arg + 64, DOMAIN_CAP_PROCESS | DOMAIN_CAP_MEMORY)
            .unwrap();

        assert_eq!(machine.domain_ctl_configure(arg), Err(1));
        assert_domain_unchanged(&machine.domains[&2], before_limits);

        machine
            .domains
            .get_mut(&ROOT_DOMAIN_ID)
            .unwrap()
            .capability_mask = u64::MAX;
        machine
            .domains
            .get_mut(&ROOT_DOMAIN_ID)
            .unwrap()
            .upcall_mask = 0b0001;
        machine.store_u64(arg + 64, 0).unwrap();
        machine.store_u64(arg + 72, 0b0010).unwrap();

        assert_eq!(machine.domain_ctl_configure(arg), Err(1));
        assert_domain_unchanged(&machine.domains[&2], before_limits);
    }

    #[test]
    fn domain_security_numeric_policy_delegation_is_monotonic() {
        let mut machine = test_machine_with_child_domain();
        let arg = ARG_BASE;
        {
            let parent = machine.domains.get_mut(&2).unwrap();
            parent.security.entropy_quota = 10;
            parent.security.hardening_profile = 5;
            parent.security.executable_source_policy = EXEC_SOURCE_FILE_MAPPING;
        }
        let next_domain_id = machine.next_domain_id;

        machine.store_u64(arg, DOMAIN_OP_CREATE).unwrap();
        machine.store_u64(arg + 8, 2).unwrap();
        machine.store_u64(arg + 16, 1).unwrap();
        machine
            .store_u64(arg + DOMAIN_SECURITY_ENTROPY_QUOTA, 11)
            .unwrap();
        assert_eq!(machine.domain_ctl_create(arg), Err(1));
        assert_eq!(machine.next_domain_id, next_domain_id);

        machine
            .store_u64(arg + DOMAIN_SECURITY_ENTROPY_QUOTA, 10)
            .unwrap();
        machine
            .store_u64(arg + DOMAIN_SECURITY_HARDENING_PROFILE, 4)
            .unwrap();
        assert_eq!(machine.domain_ctl_create(arg), Err(1));
        assert_eq!(machine.next_domain_id, next_domain_id);

        machine
            .store_u64(arg + DOMAIN_SECURITY_HARDENING_PROFILE, 5)
            .unwrap();
        machine
            .store_u64(
                arg + DOMAIN_SECURITY_EXEC_SOURCE_POLICY,
                EXEC_SOURCE_FILE_MAPPING | EXEC_SOURCE_ANONYMOUS_JIT,
            )
            .unwrap();
        assert_eq!(machine.domain_ctl_create(arg), Err(1));
        assert_eq!(machine.next_domain_id, next_domain_id);
        assert!(!machine.domains.contains_key(&next_domain_id));
    }

    #[test]
    fn inactive_current_domain_rejects_sensitive_operations() {
        let mut machine = test_machine_with_child_domain();
        machine.current_tid = 1;
        machine.processes.get_mut(&1).unwrap().domain_id = 2;
        machine.thread_mut().unwrap().regs[1] = 64;
        machine.domains.get_mut(&2).unwrap().frozen = true;
        let err = machine.exec(Instr::Alloc(Reg(2), Reg(1))).unwrap_err();
        assert!(err.contains("resource domain inactive"), "{err}");
        assert_eq!(machine.process().unwrap().errno, 11);

        let mut machine = test_machine_with_child_domain();
        machine.current_tid = 1;
        machine.processes.get_mut(&1).unwrap().domain_id = 2;
        machine.thread_mut().unwrap().regs[1] = 64;
        machine.destroy_domain_recursive(2);
        let err = machine.exec(Instr::Alloc(Reg(2), Reg(1))).unwrap_err();
        assert!(err.contains("resource domain inactive"), "{err}");
        assert_eq!(machine.process().unwrap().errno, 11);
    }

    #[test]
    fn mmap_success_clears_errno() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.set_errno(123).unwrap();
        machine.thread_mut().unwrap().regs[1] = 4096;
        machine.thread_mut().unwrap().regs[2] = 0b011;

        machine
            .exec(Instr::Mmap(
                Reg(3),
                Reg(0),
                Reg(1),
                Reg(2),
                FdReg(0),
                Reg(0),
            ))
            .unwrap();

        assert_ne!(machine.thread().unwrap().regs[3], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 0);
    }

    #[test]
    fn mmap_rejects_unknown_protection_bits_without_vma_side_effects() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let vma_count = machine.process().unwrap().vmas.len();
        let mmap_next = machine.process().unwrap().mmap_next;
        machine.thread_mut().unwrap().regs[1] = 4096;
        machine.thread_mut().unwrap().regs[2] = 0b1000;

        machine
            .exec(Instr::Mmap(
                Reg(3),
                Reg(0),
                Reg(1),
                Reg(2),
                FdReg(0),
                Reg(0),
            ))
            .unwrap();

        assert_eq!(machine.thread().unwrap().regs[3], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        assert_eq!(machine.process().unwrap().vmas.len(), vma_count);
        assert_eq!(machine.process().unwrap().mmap_next, mmap_next);
    }

    #[test]
    fn wx_mmap_and_mprotect_follow_domain_policy() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[6] = 4096;
        machine.thread_mut().unwrap().regs[7] = 0b110;
        machine
            .exec(Instr::Mmap(
                Reg(8),
                Reg(0),
                Reg(6),
                Reg(7),
                FdReg(0),
                Reg(0),
            ))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[8], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);

        machine.thread_mut().unwrap().regs[7] = 0b011;
        machine
            .exec(Instr::Mmap(
                Reg(8),
                Reg(0),
                Reg(6),
                Reg(7),
                FdReg(0),
                Reg(0),
            ))
            .unwrap();
        let addr = machine.thread().unwrap().regs[8];
        assert_ne!(addr, -1i64 as u64);

        machine.thread_mut().unwrap().regs[9] = addr;
        machine.thread_mut().unwrap().regs[10] = 4096;
        machine.thread_mut().unwrap().regs[11] = 0b110;
        machine
            .exec(Instr::Mprotect(Reg(9), Reg(10), Reg(11)))
            .unwrap();
        assert_eq!(machine.process().unwrap().errno, 1);
        assert_eq!(
            machine
                .process()
                .unwrap()
                .vmas
                .iter()
                .find(|vma| vma.start == addr)
                .unwrap()
                .prot,
            0b011
        );

        machine
            .domains
            .get_mut(&ROOT_DOMAIN_ID)
            .unwrap()
            .security
            .allow_wx = true;
        machine
            .exec(Instr::Mprotect(Reg(9), Reg(10), Reg(11)))
            .unwrap();
        assert_eq!(machine.process().unwrap().errno, 0);
        assert_eq!(
            machine
                .process()
                .unwrap()
                .vmas
                .iter()
                .find(|vma| vma.start == addr)
                .unwrap()
                .prot,
            0b110
        );
    }

    #[test]
    fn mprotect_rejects_overflow_and_unmapped_ranges() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[4] = 4096;
        machine.thread_mut().unwrap().regs[5] = 0b011;
        machine
            .exec(Instr::Mmap(
                Reg(6),
                Reg(0),
                Reg(4),
                Reg(5),
                FdReg(0),
                Reg(0),
            ))
            .unwrap();
        let mapped = machine.thread().unwrap().regs[6];
        let vma_count = machine.process().unwrap().vmas.len();

        machine.thread_mut().unwrap().regs[1] = mapped;
        machine.thread_mut().unwrap().regs[2] = 0;
        machine.thread_mut().unwrap().regs[3] = 0b001;
        machine
            .exec(Instr::Mprotect(Reg(1), Reg(2), Reg(3)))
            .unwrap();
        assert_eq!(machine.process().unwrap().errno, 22);
        assert_eq!(machine.process().unwrap().vmas.len(), vma_count);
        assert_eq!(
            machine
                .process()
                .unwrap()
                .vmas
                .iter()
                .find(|vma| vma.start == mapped)
                .unwrap()
                .prot,
            0b011
        );

        machine.thread_mut().unwrap().regs[1] = u64::MAX - 1;
        machine.thread_mut().unwrap().regs[2] = 8;
        machine.thread_mut().unwrap().regs[3] = 0b001;
        machine
            .exec(Instr::Mprotect(Reg(1), Reg(2), Reg(3)))
            .unwrap();
        assert_eq!(machine.process().unwrap().errno, 22);
        assert_eq!(machine.process().unwrap().vmas.len(), vma_count);
        assert_eq!(
            machine
                .process()
                .unwrap()
                .vmas
                .iter()
                .find(|vma| vma.start == mapped)
                .unwrap()
                .prot,
            0b011
        );

        machine.thread_mut().unwrap().regs[1] = mapped;
        machine.thread_mut().unwrap().regs[2] = 4096;
        machine.thread_mut().unwrap().regs[3] = 0b1000;
        machine
            .exec(Instr::Mprotect(Reg(1), Reg(2), Reg(3)))
            .unwrap();
        assert_eq!(machine.process().unwrap().errno, 22);
        assert_eq!(machine.process().unwrap().vmas.len(), vma_count);
        assert_eq!(
            machine
                .process()
                .unwrap()
                .vmas
                .iter()
                .find(|vma| vma.start == mapped)
                .unwrap()
                .prot,
            0b011
        );

        machine.thread_mut().unwrap().regs[1] = mapped + 128;
        machine.thread_mut().unwrap().regs[2] = 1024;
        machine.thread_mut().unwrap().regs[3] = 0b001;
        machine
            .exec(Instr::Mprotect(Reg(1), Reg(2), Reg(3)))
            .unwrap();
        assert_eq!(machine.process().unwrap().errno, 22);
        assert_eq!(machine.process().unwrap().vmas.len(), vma_count);
        assert_eq!(
            machine
                .process()
                .unwrap()
                .vmas
                .iter()
                .find(|vma| vma.start == mapped)
                .unwrap()
                .prot,
            0b011
        );

        machine.thread_mut().unwrap().regs[1] = 0xdead_0000;
        machine.thread_mut().unwrap().regs[2] = 4096;
        machine.thread_mut().unwrap().regs[3] = 0b001;
        machine
            .exec(Instr::Mprotect(Reg(1), Reg(2), Reg(3)))
            .unwrap();
        assert_eq!(machine.process().unwrap().errno, 12);
        assert_eq!(machine.process().unwrap().vmas.len(), vma_count);
        assert_eq!(
            machine
                .process()
                .unwrap()
                .vmas
                .iter()
                .find(|vma| vma.start == mapped)
                .unwrap()
                .prot,
            0b011
        );
    }

    #[test]
    fn mmap_rejects_overflow_and_out_of_range_hints_without_vmas() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let vma_count = machine.process().unwrap().vmas.len();

        machine.thread_mut().unwrap().regs[1] = 0;
        machine.thread_mut().unwrap().regs[2] = 0;
        machine.thread_mut().unwrap().regs[3] = 0b001;
        machine
            .exec(Instr::Mmap(
                Reg(4),
                Reg(0),
                Reg(1),
                Reg(3),
                FdReg(0),
                Reg(0),
            ))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        assert_eq!(machine.process().unwrap().vmas.len(), vma_count);

        machine.thread_mut().unwrap().regs[1] = u64::MAX - 1;
        machine.thread_mut().unwrap().regs[2] = 8;
        machine.thread_mut().unwrap().regs[3] = 0b001;
        machine
            .exec(Instr::Mmap(
                Reg(4),
                Reg(1),
                Reg(2),
                Reg(3),
                FdReg(0),
                Reg(0),
            ))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        assert_eq!(machine.process().unwrap().vmas.len(), vma_count);

        machine.thread_mut().unwrap().regs[1] = MEMORY_SIZE as u64 - 1024;
        machine.thread_mut().unwrap().regs[2] = 4096;
        machine
            .exec(Instr::Mmap(
                Reg(5),
                Reg(1),
                Reg(2),
                Reg(3),
                FdReg(0),
                Reg(0),
            ))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[5], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 12);
        assert_eq!(machine.process().unwrap().vmas.len(), vma_count);

        let mmap_next = machine.process().unwrap().mmap_next;
        machine.thread_mut().unwrap().regs[1] = 0;
        machine.thread_mut().unwrap().regs[2] = 4096;
        machine.thread_mut().unwrap().regs[3] = 0b001;
        let err = machine
            .exec(Instr::Mmap(
                Reg(31),
                Reg(1),
                Reg(2),
                Reg(3),
                FdReg(0),
                Reg(0),
            ))
            .unwrap_err();
        assert!(err.contains("hardware-locked stack pointer"), "{err}");
        assert_eq!(machine.process().unwrap().mmap_next, mmap_next);
        assert_eq!(machine.process().unwrap().vmas.len(), vma_count);
    }

    #[test]
    fn mmap_rejects_overlapping_hint_without_mutating_vmas() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[1] = 4096;
        machine.thread_mut().unwrap().regs[2] = 0b011;
        machine
            .exec(Instr::Mmap(
                Reg(3),
                Reg(0),
                Reg(1),
                Reg(2),
                FdReg(0),
                Reg(0),
            ))
            .unwrap();
        let addr = machine.thread().unwrap().regs[3];
        assert_ne!(addr, -1i64 as u64);
        let vma_count = machine.process().unwrap().vmas.len();

        machine.thread_mut().unwrap().regs[4] = addr + 128;
        machine.thread_mut().unwrap().regs[5] = 4096;
        machine.thread_mut().unwrap().regs[6] = 0b001;
        machine
            .exec(Instr::Mmap(
                Reg(7),
                Reg(4),
                Reg(5),
                Reg(6),
                FdReg(0),
                Reg(0),
            ))
            .unwrap();

        assert_eq!(machine.thread().unwrap().regs[7], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 12);
        assert_eq!(machine.process().unwrap().vmas.len(), vma_count);
        machine.write_bytes(addr, &[0xab]).unwrap();
        assert_eq!(machine.read_bytes(addr, 1).unwrap(), vec![0xab]);
    }

    #[test]
    fn hinted_mmap_does_not_rewind_default_mapping_cursor() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[1] = 4096;
        machine.thread_mut().unwrap().regs[2] = 0b011;
        machine
            .exec(Instr::Mmap(
                Reg(3),
                Reg(0),
                Reg(1),
                Reg(2),
                FdReg(0),
                Reg(0),
            ))
            .unwrap();
        let base = machine.thread().unwrap().regs[3];
        assert_ne!(base, -1i64 as u64);

        machine.thread_mut().unwrap().regs[4] = base + 8192;
        machine
            .exec(Instr::Mmap(
                Reg(5),
                Reg(4),
                Reg(1),
                Reg(2),
                FdReg(0),
                Reg(0),
            ))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[5], base + 8192);

        machine.thread_mut().unwrap().regs[6] = base + 4096;
        machine
            .exec(Instr::Mmap(
                Reg(7),
                Reg(6),
                Reg(1),
                Reg(2),
                FdReg(0),
                Reg(0),
            ))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[7], base + 4096);

        machine
            .exec(Instr::Mmap(
                Reg(8),
                Reg(0),
                Reg(1),
                Reg(2),
                FdReg(0),
                Reg(0),
            ))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[8], base + 12288);
        assert_eq!(machine.process().unwrap().errno, 0);
    }

    #[test]
    fn munmap_rejects_partial_or_interior_ranges_without_unmapping() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[1] = 4096;
        machine.thread_mut().unwrap().regs[2] = 0b011;
        machine
            .exec(Instr::Mmap(
                Reg(3),
                Reg(0),
                Reg(1),
                Reg(2),
                FdReg(0),
                Reg(0),
            ))
            .unwrap();
        let addr = machine.thread().unwrap().regs[3];
        assert_ne!(addr, -1i64 as u64);
        machine.write_bytes(addr, &[0xcc]).unwrap();
        let vma_count = machine.process().unwrap().vmas.len();

        machine.thread_mut().unwrap().regs[4] = addr;
        machine.thread_mut().unwrap().regs[5] = 0;
        machine.exec(Instr::Munmap(Reg(4), Reg(5))).unwrap();
        assert_eq!(machine.process().unwrap().errno, 22);
        assert_eq!(machine.process().unwrap().vmas.len(), vma_count);
        assert_eq!(machine.read_bytes(addr, 1).unwrap(), vec![0xcc]);

        machine.thread_mut().unwrap().regs[4] = addr + 128;
        machine.thread_mut().unwrap().regs[5] = 4096;
        machine.exec(Instr::Munmap(Reg(4), Reg(5))).unwrap();
        assert_eq!(machine.process().unwrap().errno, 22);
        assert_eq!(machine.process().unwrap().vmas.len(), vma_count);
        assert_eq!(machine.read_bytes(addr, 1).unwrap(), vec![0xcc]);

        machine.thread_mut().unwrap().regs[4] = addr;
        machine.thread_mut().unwrap().regs[5] = 2048;
        machine.exec(Instr::Munmap(Reg(4), Reg(5))).unwrap();
        assert_eq!(machine.process().unwrap().errno, 22);
        assert_eq!(machine.process().unwrap().vmas.len(), vma_count);
        assert_eq!(machine.read_bytes(addr, 1).unwrap(), vec![0xcc]);

        machine.thread_mut().unwrap().regs[5] = 4096;
        machine.exec(Instr::Munmap(Reg(4), Reg(5))).unwrap();
        assert_eq!(machine.process().unwrap().errno, 0);
        assert_eq!(machine.process().unwrap().vmas.len(), vma_count - 1);
        let err = machine.read_bytes(addr, 1).unwrap_err();
        assert!(err.contains("unmapped address"), "{err}");
    }

    #[test]
    fn isync_reports_success_and_canonical_range_errors() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[1] = 4096;
        machine.thread_mut().unwrap().regs[2] = 0b101;
        machine.thread_mut().unwrap().regs[7] = 0x230_000;
        machine
            .exec(Instr::Mmap(
                Reg(3),
                Reg(7),
                Reg(1),
                Reg(2),
                FdReg(0),
                Reg(7),
            ))
            .unwrap();
        let addr = machine.thread().unwrap().regs[3];
        assert_ne!(addr, -1i64 as u64);

        machine.thread_mut().unwrap().regs[4] = addr;
        machine.thread_mut().unwrap().regs[5] = 64;
        machine.exec(Instr::Isync(Reg(6), Reg(4), Reg(5))).unwrap();
        assert_eq!(machine.thread().unwrap().regs[6], 0);
        assert_eq!(machine.process().unwrap().errno, 0);

        machine.thread_mut().unwrap().regs[5] = 0;
        machine.exec(Instr::Isync(Reg(6), Reg(4), Reg(5))).unwrap();
        assert_eq!(machine.thread().unwrap().regs[6], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);

        machine.thread_mut().unwrap().regs[4] = 0xffff_0000;
        machine.thread_mut().unwrap().regs[5] = 64;
        machine.exec(Instr::Isync(Reg(6), Reg(4), Reg(5))).unwrap();
        assert_eq!(machine.thread().unwrap().regs[6], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 14);
    }

    #[test]
    fn completion_helpers_reject_locked_result_before_errno_update() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;

        machine.set_errno(123).unwrap();
        let err = machine.complete_reg_ok(Reg(31), 0).unwrap_err();
        assert!(err.contains("hardware-locked stack pointer"), "{err}");
        assert_eq!(machine.process().unwrap().errno, 123);

        machine.set_errno(124).unwrap();
        let err = machine.complete_reg_err(Reg(31), 22).unwrap_err();
        assert!(err.contains("hardware-locked stack pointer"), "{err}");
        assert_eq!(machine.process().unwrap().errno, 124);

        machine.thread_mut().unwrap().regs[4] = ARG_BASE;
        machine.thread_mut().unwrap().regs[5] = 0;
        machine.set_errno(125).unwrap();
        let err = machine
            .exec(Instr::Isync(Reg(31), Reg(4), Reg(5)))
            .unwrap_err();
        assert!(err.contains("hardware-locked stack pointer"), "{err}");
        assert_eq!(machine.process().unwrap().errno, 125);
    }

    #[test]
    fn executable_mappings_require_jit_transition_policy() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        machine
            .domains
            .get_mut(&ROOT_DOMAIN_ID)
            .unwrap()
            .security
            .allow_jit_transition = false;

        machine.thread_mut().unwrap().regs[1] = 4096;
        machine.thread_mut().unwrap().regs[2] = 0b101;
        machine.thread_mut().unwrap().regs[7] = 0x220_000;
        machine
            .exec(Instr::Mmap(
                Reg(3),
                Reg(7),
                Reg(1),
                Reg(2),
                FdReg(0),
                Reg(7),
            ))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[3], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);

        machine.thread_mut().unwrap().regs[1] = 4096;
        machine.thread_mut().unwrap().regs[2] = 0b011;
        machine.thread_mut().unwrap().regs[7] = 0x220_000;
        machine
            .exec(Instr::Mmap(
                Reg(3),
                Reg(7),
                Reg(1),
                Reg(2),
                FdReg(0),
                Reg(7),
            ))
            .unwrap();
        let addr = machine.thread().unwrap().regs[3];
        assert_ne!(addr, -1i64 as u64);

        machine.thread_mut().unwrap().regs[4] = addr;
        machine.thread_mut().unwrap().regs[5] = 4096;
        machine.thread_mut().unwrap().regs[6] = 0b101;
        machine
            .exec(Instr::Mprotect(Reg(4), Reg(5), Reg(6)))
            .unwrap();
        assert_eq!(machine.process().unwrap().errno, 1);
        assert_eq!(
            machine
                .process()
                .unwrap()
                .vmas
                .iter()
                .find(|vma| vma.start == addr)
                .unwrap()
                .prot,
            0b011
        );

        machine
            .domains
            .get_mut(&ROOT_DOMAIN_ID)
            .unwrap()
            .security
            .allow_jit_transition = true;
        machine.thread_mut().unwrap().regs[4] = addr;
        machine.thread_mut().unwrap().regs[5] = 4096;
        machine.thread_mut().unwrap().regs[6] = 0b101;
        machine
            .exec(Instr::Mprotect(Reg(4), Reg(5), Reg(6)))
            .unwrap();
        assert_eq!(machine.process().unwrap().errno, 0);
        assert_eq!(
            machine
                .process()
                .unwrap()
                .vmas
                .iter()
                .find(|vma| vma.start == addr)
                .unwrap()
                .prot,
            0b101
        );
    }

    #[test]
    fn nx_and_guard_instruction_fetches_fault() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[1] = 4096;
        machine.thread_mut().unwrap().regs[2] = 0b011;
        machine
            .exec(Instr::Mmap(
                Reg(3),
                Reg(0),
                Reg(1),
                Reg(2),
                FdReg(0),
                Reg(0),
            ))
            .unwrap();
        let rw_mapping = machine.thread().unwrap().regs[3];
        machine.thread_mut().unwrap().ip = rw_mapping as usize;
        let err = machine.run().unwrap_err();
        assert!(err.contains("execute denied"), "{err}");

        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[1] = 32;
        machine.thread_mut().unwrap().regs[2] = 64;
        machine
            .exec(Instr::AllocEx(Reg(3), Reg(1), Reg(2)))
            .unwrap();
        let guarded = machine.thread().unwrap().regs[3] - 1;
        machine.thread_mut().unwrap().ip = guarded as usize;
        let err = machine.run().unwrap_err();
        assert!(err.contains("guard page execute"), "{err}");
    }

    #[test]
    fn instruction_fetch_rejects_vma_outside_process_memory() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        let addr = MEMORY_SIZE as u64 + 8;
        machine
            .process_mut()
            .unwrap()
            .vmas
            .push(Vma::anonymous(addr, 8, 0b101));
        machine.thread_mut().unwrap().ip = addr as usize;

        let err = machine.run().unwrap_err();
        assert!(err.contains("outside process memory"), "{err}");
        assert!(!err.contains("dynamic instruction fetch"), "{err}");
    }

    #[test]
    fn jit_style_transition_reaches_rx_without_wx() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[1] = 4096;
        machine.thread_mut().unwrap().regs[2] = 0b011;
        machine
            .exec(Instr::Mmap(
                Reg(3),
                Reg(0),
                Reg(1),
                Reg(2),
                FdReg(0),
                Reg(0),
            ))
            .unwrap();
        let addr = machine.thread().unwrap().regs[3];
        machine.write_bytes(addr, &[0x90]).unwrap();

        machine.thread_mut().unwrap().regs[4] = addr;
        machine.thread_mut().unwrap().regs[5] = 4096;
        machine.thread_mut().unwrap().regs[6] = 0b101;
        machine
            .exec(Instr::Mprotect(Reg(4), Reg(5), Reg(6)))
            .unwrap();
        assert_eq!(machine.process().unwrap().errno, 0);
        assert_eq!(
            machine
                .process()
                .unwrap()
                .vmas
                .iter()
                .find(|vma| vma.start == addr)
                .unwrap()
                .prot,
            0b101
        );
        let err = machine.write_bytes(addr, &[0xcc]).unwrap_err();
        assert!(err.contains("write denied"), "{err}");
    }

    #[test]
    fn signal_frame_stack_area_is_non_executable() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let machine = Machine::new(program);
        let stack_top = machine.process().unwrap().stack_top;
        let stack_vma = machine
            .process()
            .unwrap()
            .vmas
            .iter()
            .find(|vma| vma.contains(stack_top - CALL_FRAME_SIZE, CALL_FRAME_SIZE as usize))
            .unwrap();
        assert_eq!(stack_vma.prot & 0b100, 0);
    }

    #[test]
    fn tcp_listener_endpoint_is_not_resolved_as_relative_path() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let machine = Machine::new(program);
        assert_eq!(
            machine
                .resolve_process_path("tcp-listen:127.0.0.1:0")
                .unwrap(),
            "tcp-listen:127.0.0.1:0"
        );
    }

    #[test]
    fn tcp_listener_endpoint_rejects_non_numeric_addresses() {
        let err = match Machine::open_fd_handle("tcp-listen:localhost:0", 0) {
            Ok(_) => panic!("expected non-numeric listener address rejection"),
            Err(err) => err,
        };
        assert!(err.contains("TCP listener address"), "{err}");
    }

    #[test]
    fn process_layout_aslr_is_deterministic_and_disableable() {
        let first = ProcessLayout::for_process(1, ROOT_DOMAIN_ID, true);
        let second = ProcessLayout::for_process(1, ROOT_DOMAIN_ID, true);
        let other_process = ProcessLayout::for_process(2, ROOT_DOMAIN_ID, true);
        let other_domain = ProcessLayout::for_process(1, 2, true);
        assert_eq!(first.stack_top, second.stack_top);
        assert_eq!(first.heap_base, second.heap_base);
        assert_eq!(first.mmap_base, second.mmap_base);
        assert_ne!(
            (first.stack_top, first.heap_base, first.mmap_base),
            (
                other_process.stack_top,
                other_process.heap_base,
                other_process.mmap_base
            )
        );
        assert_ne!(
            (first.stack_top, first.heap_base, first.mmap_base),
            (
                other_domain.stack_top,
                other_domain.heap_base,
                other_domain.mmap_base
            )
        );
        assert_ne!(first.stack_top, STACK_TOP);
        assert_ne!(first.heap_base, HEAP_BASE);
        assert_ne!(first.mmap_base, MMAP_BASE);
        assert_eq!(first.stack_top % ASLR_PAGE, 0);
        assert_eq!(first.heap_base % ASLR_PAGE, 0);
        assert_eq!(first.mmap_base % ASLR_PAGE, 0);

        let disabled = ProcessLayout::for_process(1, ROOT_DOMAIN_ID, false);
        assert_eq!(disabled.stack_top, STACK_TOP);
        assert_eq!(disabled.heap_base, HEAP_BASE);
        assert_eq!(disabled.mmap_base, MMAP_BASE);
        let disabled_child = ProcessLayout::for_process(1, 2, false);
        assert_eq!(disabled_child.stack_top, STACK_TOP);
        assert_eq!(disabled_child.heap_base, HEAP_BASE);
        assert_eq!(disabled_child.mmap_base, MMAP_BASE);
    }

    #[test]
    fn vma_contains_rejects_overflowing_region_bounds() {
        let vma = Vma::anonymous(u64::MAX - 8, 16, 0b011);

        assert!(!vma.contains(u64::MAX - 8, 1));
        assert!(!vma.contains(u64::MAX - 4, 8));

        let normal = Vma::anonymous(0x1000, 0x100, 0b011);
        assert!(normal.contains(0x1010, 0x20));
        assert!(!normal.contains(0x10f0, 0x20));
    }

    #[test]
    fn ensure_mapped_rejects_resident_vma_beyond_process_memory() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let addr = MEMORY_SIZE as u64 - 2;
        machine
            .process_mut()
            .unwrap()
            .vmas
            .push(Vma::anonymous(addr, 8, 0b011));

        let read_err = machine.read_bytes(addr, 8).unwrap_err();
        assert!(read_err.contains("outside process memory"), "{read_err}");
        let write_err = machine.write_bytes(addr, &[1, 2, 3, 4]).unwrap_err();
        assert!(write_err.contains("outside process memory"), "{write_err}");
    }

    #[test]
    fn allocation_alignment_rejects_wrapping_heap_cursor() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.process_mut().unwrap().heap_next = u64::MAX - 8;
        machine.thread_mut().unwrap().regs[1] = 1;

        let err = machine.exec(Instr::Alloc(Reg(2), Reg(1))).unwrap_err();

        assert!(err.contains("allocation overflow"), "{err}");
        assert_eq!(machine.thread().unwrap().regs[2], 0);
    }

    #[test]
    fn alloc_rejects_locked_result_register_before_vma_side_effects() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let heap_next = machine.process().unwrap().heap_next;
        let vma_count = machine.process().unwrap().vmas.len();
        machine.thread_mut().unwrap().regs[1] = 64;

        let err = machine.exec(Instr::Alloc(Reg(31), Reg(1))).unwrap_err();

        assert!(err.contains("hardware-locked stack pointer"), "{err}");
        assert_eq!(machine.process().unwrap().heap_next, heap_next);
        assert!(machine.process().unwrap().allocations.is_empty());
        assert_eq!(machine.process().unwrap().vmas.len(), vma_count);
    }

    #[test]
    fn allocation_success_clears_errno() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[1] = 64;
        machine.set_errno(123).unwrap();

        machine.exec(Instr::Alloc(Reg(2), Reg(1))).unwrap();

        let ptr = machine.thread().unwrap().regs[2];
        assert_ne!(ptr, 0);
        assert_eq!(machine.process().unwrap().errno, 0);

        machine.thread_mut().unwrap().regs[6] = ptr;
        machine.set_errno(55).unwrap();
        machine.exec(Instr::AllocSize(Reg(7), Reg(6))).unwrap();
        assert_eq!(machine.thread().unwrap().regs[7], 64);
        assert_eq!(machine.process().unwrap().errno, 0);

        machine.thread_mut().unwrap().regs[8] = ptr + 1;
        machine.set_errno(56).unwrap();
        machine.exec(Instr::AllocSize(Reg(9), Reg(8))).unwrap();
        assert_eq!(machine.thread().unwrap().regs[9], 0);
        assert_eq!(machine.process().unwrap().errno, 0);

        machine.thread_mut().unwrap().regs[3] = 128;
        machine.thread_mut().unwrap().regs[4] = 256;
        machine.set_errno(77).unwrap();

        machine
            .exec(Instr::AllocEx(Reg(5), Reg(3), Reg(4)))
            .unwrap();

        assert_ne!(machine.thread().unwrap().regs[5], 0);
        assert_eq!(machine.process().unwrap().errno, 0);
    }

    #[test]
    fn anonymous_mmap_rejects_wrapping_cursor_alignment() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.process_mut().unwrap().mmap_next = u64::MAX - 8;
        machine.thread_mut().unwrap().regs[1] = 4096;
        machine.thread_mut().unwrap().regs[2] = 0b011;

        machine
            .exec(Instr::Mmap(
                Reg(3),
                Reg(0),
                Reg(1),
                Reg(2),
                FdReg(0),
                Reg(0),
            ))
            .unwrap();

        assert_eq!(machine.thread().unwrap().regs[3], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
    }

    #[test]
    fn mprotect_ignores_overflowing_vma_bounds() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine
            .process_mut()
            .unwrap()
            .vmas
            .push(Vma::anonymous(u64::MAX - 8, 16, 0b011));

        machine.mprotect_range(u64::MAX - 8, 1, 0b001).unwrap();

        assert_eq!(machine.process().unwrap().errno, 12);
        assert_eq!(
            machine
                .process()
                .unwrap()
                .vmas
                .iter()
                .find(|vma| vma.start == u64::MAX - 8)
                .unwrap()
                .prot,
            0b011
        );
    }

    #[test]
    fn heap_and_anonymous_mmap_use_aslr_layout() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        let layout = ProcessLayout::for_process(1, ROOT_DOMAIN_ID, true);
        assert_eq!(machine.process().unwrap().heap_next, layout.heap_base);
        assert_eq!(machine.process().unwrap().mmap_next, layout.mmap_base);
        assert_eq!(machine.process().unwrap().stack_top, layout.stack_top);

        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[1] = 32;
        machine.exec(Instr::Alloc(Reg(2), Reg(1))).unwrap();
        assert_eq!(
            machine.thread().unwrap().regs[2],
            align_up(layout.heap_base, 64)
        );

        machine.thread_mut().unwrap().regs[3] = 4096;
        machine.thread_mut().unwrap().regs[4] = 0b011;
        machine
            .exec(Instr::Mmap(
                Reg(5),
                Reg(0),
                Reg(3),
                Reg(4),
                FdReg(0),
                Reg(0),
            ))
            .unwrap();
        assert_eq!(
            machine.thread().unwrap().regs[5],
            align_up(layout.mmap_base, 4096)
        );
    }

    #[test]
    fn dynamic_fd_tokens_reject_stale_reuse() {
        let program = Program::parse(
            r#"
            .data
            path: .string "Cargo.toml"

            .text
              NOP
            "#,
        )
        .unwrap();
        let path = program.data_labels["path"];
        let mut machine = Machine::new(program);
        machine.current_tid = 1;

        machine.thread_mut().unwrap().regs[1] = path;
        machine.thread_mut().unwrap().regs[2] = 0;
        machine
            .exec(Instr::OpenFdDyn(Reg(3), Reg(1), Reg(2)))
            .unwrap();
        let first_token = machine.thread().unwrap().regs[3];
        let first_fd = (first_token & FDR_TOKEN_INDEX_MASK) as usize;
        assert!(first_token >= FDR_COUNT as u64);

        machine.thread_mut().unwrap().regs[4] = first_token;
        machine.exec(Instr::FdCloseDyn(Reg(4))).unwrap();
        assert_eq!(machine.process().unwrap().errno, 0);

        machine.thread_mut().unwrap().regs[1] = path;
        machine.thread_mut().unwrap().regs[2] = 0;
        machine
            .exec(Instr::OpenFdDyn(Reg(5), Reg(1), Reg(2)))
            .unwrap();
        let second_token = machine.thread().unwrap().regs[5];
        assert_ne!(second_token, first_token);
        assert_eq!((second_token & FDR_TOKEN_INDEX_MASK) as usize, first_fd);

        machine.thread_mut().unwrap().regs[6] = first_token;
        machine.thread_mut().unwrap().regs[7] = ARG_BASE;
        machine.thread_mut().unwrap().regs[8] = 4;
        machine
            .exec(Instr::ReadFdDyn(Reg(6), Reg(7), Reg(8)))
            .unwrap();
        assert_eq!(machine.process().unwrap().errno, 116);
        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
    }

    #[test]
    fn open_fd_dyn_rejects_locked_result_register_before_allocating_fd() {
        let program = Program::parse(
            r#"
            .data
            path: .string "Cargo.toml"

            .text
              NOP
            "#,
        )
        .unwrap();
        let path = program.data_labels["path"];
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        let generations = machine.process().unwrap().fd_generations.clone();

        machine.thread_mut().unwrap().regs[1] = path;
        machine.thread_mut().unwrap().regs[2] = 0;
        let err = machine
            .exec(Instr::OpenFdDyn(Reg(31), Reg(1), Reg(2)))
            .unwrap_err();

        assert!(err.contains("hardware-locked stack pointer"), "{err}");
        assert_eq!(machine.process().unwrap().fd_generations, generations);
        assert!(matches!(
            machine.process().unwrap().fds[3],
            FdHandle::Closed
        ));
    }

    #[test]
    fn close_rejects_closed_fdr_slots_without_recycling_generation() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let original_generation = machine.fd_generation(7).unwrap();

        assert!(machine.exec(Instr::FdClose(FdReg(7))).unwrap());
        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 9);
        assert_eq!(machine.fd_generation(7).unwrap(), original_generation);

        machine.thread_mut().unwrap().regs[2] = 7;
        assert!(machine.exec(Instr::FdCloseDyn(Reg(2))).unwrap());
        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 9);
        assert_eq!(machine.fd_generation(7).unwrap(), original_generation);
    }

    #[test]
    fn stale_fd_waiter_rejects_reused_event_source_generation() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        let pipe = Rc::new(RefCell::new(PipeBuffer::default()));
        machine.processes.get_mut(&1).unwrap().fds[3] = FdHandle::PipeReader(pipe);
        machine.processes.get_mut(&1).unwrap().fd_capabilities[3] = FdCapability::full(3);
        machine.thread_mut().unwrap().regs[2] = POLLIN_MASK;

        let keep_ready = machine
            .exec(Instr::Await(Reg(5), FdReg(3), Reg(2)))
            .unwrap();
        assert!(!keep_ready);
        assert_eq!(machine.fd_waiters.len(), 1);

        machine.close_fd_index(3).unwrap();
        assert_eq!(
            machine
                .alloc_fd_handle(FdHandle::Counter(Rc::new(RefCell::new(1))))
                .unwrap(),
            Some(3)
        );
        machine.poll_fd_waiters();

        assert!(machine.ready.contains(&1));
        assert!(machine.fd_waiters.is_empty());
        assert_eq!(machine.thread().unwrap().regs[5], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 116);
    }

    #[test]
    fn fd_reads_reject_huge_destination_lengths_before_allocating() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        {
            let process = machine.process_mut().unwrap();
            process.fds[4] = FdHandle::Counter(Rc::new(RefCell::new(0xfeed)));
            process.fd_capabilities[4] = FdCapability::full(4);
        }

        let err = machine.read_fd_index(4, ARG_BASE, usize::MAX).unwrap_err();
        assert!(err.contains("unmapped address"), "{err}");

        let path = format!("/tmp/lnp64_pread_huge_{}.txt", std::process::id());
        fs::write(&path, b"data").unwrap();
        let file = OpenOptions::new().read(true).open(&path).unwrap();
        {
            let process = machine.process_mut().unwrap();
            process.fds[5] = FdHandle::File(file);
            process.fd_capabilities[5] = FdCapability::full(5);
        }
        let err = machine
            .pread_fd_index(5, ARG_BASE, usize::MAX, 0)
            .unwrap_err();
        assert!(err.contains("unmapped address"), "{err}");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn read_fd_denied_preserves_error_status() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        {
            let process = machine.process_mut().unwrap();
            process.fds[3] = FdHandle::Counter(Rc::new(RefCell::new(7)));
            process.fd_capabilities[3] = FdCapability::full(3);
            process.fd_capabilities[3].rights = CAP_RIGHT_WRITE;
        }

        assert_eq!(machine.read_fd_index(3, ARG_BASE, 8).unwrap(), None);
        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);

        machine.thread_mut().unwrap().regs[4] = 3;
        machine.thread_mut().unwrap().regs[5] = ARG_BASE;
        machine.thread_mut().unwrap().regs[6] = 8;
        machine.thread_mut().unwrap().regs[1] = 1234;
        machine
            .exec(Instr::ReadFdDyn(Reg(4), Reg(5), Reg(6)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
    }

    #[test]
    fn stat_fd_prevalidates_entire_output_record() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let statbuf = MEMORY_SIZE as u64 - 8;
        {
            let process = machine.process_mut().unwrap();
            process.vmas.push(Vma::anonymous(statbuf, 8, 0b11));
            process.fds[3] = FdHandle::Counter(Rc::new(RefCell::new(0)));
            process.fd_capabilities[3] = FdCapability::full(3);
        }
        let sentinel = 0xfeed_face_cafe_beefu64.to_le_bytes();
        machine.write_bytes(statbuf, &sentinel).unwrap();

        let err = machine.stat_fd_index(statbuf, 3).unwrap_err();

        assert!(err.contains("unmapped address"), "{err}");
        assert_eq!(machine.read_bytes(statbuf, 8).unwrap(), sentinel.to_vec());
    }

    #[test]
    fn stale_dynamic_fd_waiter_reports_error_result() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        create_pipe_pair(&mut machine, 3, 4);
        machine.thread_mut().unwrap().regs[2] = 3;
        machine.thread_mut().unwrap().regs[3] = POLLIN_MASK;

        let keep_ready = machine
            .exec(Instr::AwaitDyn(Reg(5), Reg(2), Reg(3)))
            .unwrap();
        assert!(!keep_ready);
        assert_eq!(machine.fd_waiters.len(), 1);

        machine.close_fd_index(3).unwrap();
        machine.poll_fd_waiters();

        assert!(machine.ready.contains(&1));
        assert!(machine.fd_waiters.is_empty());
        assert_eq!(machine.thread().unwrap().regs[5], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 116);
    }

    #[test]
    fn await_requires_poll_right_without_parking() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        create_pipe_pair(&mut machine, 3, 4);
        machine.processes.get_mut(&1).unwrap().fd_capabilities[3].rights &= !CAP_RIGHT_POLL;
        machine.thread_mut().unwrap().regs[2] = 0;

        let keep_ready = machine
            .exec(Instr::Await(Reg(5), FdReg(3), Reg(2)))
            .unwrap();
        assert!(keep_ready);
        assert!(machine.fd_waiters.is_empty());
        assert_eq!(machine.thread().unwrap().regs[5], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);

        machine.thread_mut().unwrap().regs[2] = POLLIN_MASK;
        let keep_ready = machine
            .exec(Instr::Await(Reg(6), FdReg(3), Reg(2)))
            .unwrap();
        assert!(keep_ready);
        assert!(machine.fd_waiters.is_empty());
        assert_eq!(machine.thread().unwrap().regs[6], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);

        machine.thread_mut().unwrap().regs[6] = 3;
        machine.thread_mut().unwrap().regs[7] = POLLIN_MASK;

        let keep_ready = machine
            .exec(Instr::AwaitDyn(Reg(8), Reg(6), Reg(7)))
            .unwrap();
        assert!(keep_ready);
        assert!(machine.fd_waiters.is_empty());
        assert_eq!(machine.thread().unwrap().regs[8], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
    }

    #[test]
    fn fd_waiter_reports_error_when_poll_right_revoked_before_wake() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        create_pipe_pair(&mut machine, 3, 4);
        machine.thread_mut().unwrap().regs[2] = POLLIN_MASK;

        let keep_ready = machine
            .exec(Instr::Await(Reg(5), FdReg(3), Reg(2)))
            .unwrap();
        assert!(!keep_ready);
        assert_eq!(machine.fd_waiters.len(), 1);

        machine.processes.get_mut(&1).unwrap().fd_capabilities[3].rights &= !CAP_RIGHT_POLL;
        machine.poll_fd_waiters();

        assert!(machine.ready.contains(&1));
        assert!(machine.fd_waiters.is_empty());
        assert_eq!(machine.thread().unwrap().regs[5], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
    }

    #[test]
    fn await_rejects_locked_result_register_without_parking() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        create_pipe_pair(&mut machine, 3, 4);
        machine.thread_mut().unwrap().regs[2] = POLLIN_MASK;

        let err = machine
            .exec(Instr::Await(Reg(31), FdReg(3), Reg(2)))
            .unwrap_err();

        assert!(err.contains("hardware-locked stack pointer"), "{err}");
        assert!(machine.fd_waiters.is_empty());
        assert!(machine.ready.contains(&1));
    }

    #[test]
    fn fd_waiter_helper_rejects_locked_result_register() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        create_pipe_pair(&mut machine, 3, 4);

        let err = machine
            .push_fd_waiter(3, POLLIN_MASK, Some(Reg(31)))
            .unwrap_err();

        assert!(err.contains("hardware-locked stack pointer"), "{err}");
        assert!(machine.fd_waiters.is_empty());
        assert!(machine.ready.contains(&1));
    }

    #[test]
    fn poll_fd_rejects_locked_result_register_without_errno_side_effects() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        create_pipe_pair(&mut machine, 3, 4);
        machine.thread_mut().unwrap().regs[2] = POLLIN_MASK;
        machine.set_errno(123).unwrap();

        let err = machine
            .exec(Instr::PollFd(Reg(31), FdReg(3), Reg(2)))
            .unwrap_err();

        assert!(err.contains("hardware-locked stack pointer"), "{err}");
        assert_eq!(machine.process().unwrap().errno, 123);
        assert!(machine.fd_waiters.is_empty());
        assert!(machine.ready.contains(&1));

        machine.thread_mut().unwrap().regs[4] = 3;
        machine.thread_mut().unwrap().regs[5] = POLLIN_MASK;
        machine.set_errno(124).unwrap();

        let err = machine
            .exec(Instr::PollFdDyn(Reg(31), Reg(4), Reg(5)))
            .unwrap_err();

        assert!(err.contains("hardware-locked stack pointer"), "{err}");
        assert_eq!(machine.process().unwrap().errno, 124);
        assert!(machine.fd_waiters.is_empty());
        assert!(machine.ready.contains(&1));
    }

    #[test]
    fn poll_fd_success_clears_errno() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        create_pipe_pair(&mut machine, 3, 4);
        machine.thread_mut().unwrap().regs[2] = POLLIN_MASK;
        machine.set_errno(123).unwrap();

        machine
            .exec(Instr::PollFd(Reg(5), FdReg(3), Reg(2)))
            .unwrap();

        assert_eq!(machine.thread().unwrap().regs[5], 0);
        assert_eq!(machine.process().unwrap().errno, 0);

        machine.thread_mut().unwrap().regs[6] = 3;
        machine.thread_mut().unwrap().regs[7] = POLLIN_MASK;
        machine.set_errno(124).unwrap();

        machine
            .exec(Instr::PollFdDyn(Reg(8), Reg(6), Reg(7)))
            .unwrap();

        assert_eq!(machine.thread().unwrap().regs[8], 0);
        assert_eq!(machine.process().unwrap().errno, 0);
    }

    #[test]
    fn await_rejects_closed_and_invalid_fds_without_parking() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[2] = POLLIN_MASK;

        let keep_ready = machine
            .exec(Instr::Await(Reg(5), FdReg(7), Reg(2)))
            .unwrap();
        assert!(keep_ready);
        assert!(machine.fd_waiters.is_empty());
        assert_eq!(machine.thread().unwrap().regs[5], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 9);

        let keep_ready = machine
            .exec(Instr::Await(Reg(6), FdReg(FDR_COUNT), Reg(2)))
            .unwrap();
        assert!(keep_ready);
        assert!(machine.fd_waiters.is_empty());
        assert_eq!(machine.thread().unwrap().regs[6], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 9);

        machine.thread_mut().unwrap().regs[7] = 7;
        machine.thread_mut().unwrap().regs[8] = POLLIN_MASK;
        let keep_ready = machine
            .exec(Instr::AwaitDyn(Reg(9), Reg(7), Reg(8)))
            .unwrap();
        assert!(keep_ready);
        assert!(machine.fd_waiters.is_empty());
        assert_eq!(machine.thread().unwrap().regs[9], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 9);
    }

    #[test]
    fn wait_on_fd_rejects_invalid_sources_without_parking() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;

        let keep_ready = machine
            .exec(Instr::WaitOnFd(FdReg(FDR_COUNT), Reg(0)))
            .unwrap();
        assert!(keep_ready);
        assert!(machine.ready.contains(&1));
        assert!(machine.fd_waiters.is_empty());
        assert_eq!(machine.process().unwrap().errno, 9);

        let keep_ready = machine.exec(Instr::WaitOnFd(FdReg(7), Reg(0))).unwrap();
        assert!(keep_ready);
        assert!(machine.ready.contains(&1));
        assert!(machine.fd_waiters.is_empty());
        assert_eq!(machine.process().unwrap().errno, 9);

        create_pipe_pair(&mut machine, 3, 4);
        machine.processes.get_mut(&1).unwrap().fd_capabilities[3].rights &= !CAP_RIGHT_POLL;
        let keep_ready = machine.exec(Instr::WaitOnFd(FdReg(3), Reg(0))).unwrap();
        assert!(keep_ready);
        assert!(machine.ready.contains(&1));
        assert!(machine.fd_waiters.is_empty());
        assert_eq!(machine.process().unwrap().errno, 1);
    }

    #[test]
    fn multi_source_fd_waiters_wake_only_ready_sources() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        create_pipe_pair(&mut machine, 3, 4);
        create_pipe_pair(&mut machine, 5, 6);
        machine.push_fd_waiter(3, POLLIN_MASK, None).unwrap();
        machine.push_fd_waiter(5, POLLIN_MASK, None).unwrap();
        machine.ready.retain(|tid| *tid != 1);

        let payload = ARG_BASE + 0x100;
        machine.write_bytes(payload, b"x").unwrap();
        machine.write_fd_index(4, payload, 1).unwrap();
        machine.poll_fd_waiters();

        assert!(machine.ready.contains(&1));
        assert_eq!(machine.fd_waiters.len(), 1);
        assert_eq!(machine.fd_waiters[0].fd, 5);
        assert!(machine.fd_read_ready(3).unwrap());
        assert!(!machine.fd_read_ready(5).unwrap());
    }

    #[test]
    fn ready_fd_waiter_completes_result_register_and_errno() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        create_pipe_pair(&mut machine, 3, 4);
        machine.thread_mut().unwrap().regs[2] = POLLIN_MASK;
        machine.thread_mut().unwrap().regs[5] = 0xdead_beef;

        let keep_ready = machine
            .exec(Instr::Await(Reg(5), FdReg(3), Reg(2)))
            .unwrap();
        assert!(!keep_ready);
        assert_eq!(machine.thread().unwrap().regs[5], 0xdead_beef);

        let payload = ARG_BASE + 0x140;
        machine.write_bytes(payload, b"x").unwrap();
        machine.set_errno(123).unwrap();
        machine.write_fd_index(4, payload, 1).unwrap();

        assert!(machine.ready.contains(&1));
        assert!(machine.fd_waiters.is_empty());
        assert_eq!(machine.thread().unwrap().regs[5], 0);
        assert_eq!(machine.process().unwrap().errno, 0);
    }

    #[test]
    fn fd_waiters_survive_repeated_pending_churn_until_ready() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        create_pipe_pair(&mut machine, 3, 4);
        create_pipe_pair(&mut machine, 5, 6);
        machine
            .push_fd_waiter(3, POLLIN_MASK, Some(Reg(7)))
            .unwrap();
        machine
            .push_fd_waiter(5, POLLIN_MASK, Some(Reg(8)))
            .unwrap();
        machine.ready.retain(|tid| *tid != 1);

        for _ in 0..4 {
            machine.poll_fd_waiters();
            assert!(!machine.ready.contains(&1));
            assert_eq!(machine.fd_waiters.len(), 2);
        }

        let payload = ARG_BASE + 0x120;
        machine.write_bytes(payload, b"y").unwrap();
        machine.write_fd_index(6, payload, 1).unwrap();
        machine.poll_fd_waiters();

        assert!(machine.ready.contains(&1));
        assert_eq!(machine.fd_waiters.len(), 1);
        assert_eq!(machine.fd_waiters[0].fd, 3);

        machine.ready.retain(|tid| *tid != 1);
        machine.write_bytes(payload, b"z").unwrap();
        machine.write_fd_index(4, payload, 1).unwrap();
        machine.poll_fd_waiters();

        assert!(machine.ready.contains(&1));
        assert!(machine.fd_waiters.is_empty());
        assert_eq!(machine.thread().unwrap().regs[7], 0);
        assert_eq!(machine.thread().unwrap().regs[8], 0);
    }

    #[test]
    fn zero_tick_sleep_parks_until_next_scheduler_tick() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[2] = 0;

        let keep_ready = machine.exec(Instr::Sleep(Reg(2))).unwrap();
        assert!(!keep_ready);
        assert!(!machine.ready.contains(&1));
        assert_eq!(machine.sleepers, vec![(1, 1)]);

        machine.tick_sleepers();
        assert!(machine.sleepers.is_empty());
        assert!(machine.ready.contains(&1));
    }

    #[test]
    fn repeated_sleep_replaces_existing_sleep_entry() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[2] = 5;

        machine.exec(Instr::Sleep(Reg(2))).unwrap();
        machine.thread_mut().unwrap().regs[2] = 9;
        machine.exec(Instr::Sleep(Reg(2))).unwrap();

        assert_eq!(machine.sleepers, vec![(1, 9)]);
        assert!(!machine.ready.contains(&1));
    }

    #[test]
    fn futex_wake_removes_empty_waiter_entry() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine
            .clone_with_profile(CloneProfile::NewThreadSharedVm, Reg(2), Some(0))
            .unwrap();
        let waiter_tid = machine.thread().unwrap().regs[2];
        machine.ready.retain(|tid| *tid != waiter_tid);
        machine
            .futex_waiters
            .entry(0x100)
            .or_default()
            .push_back(waiter_tid);
        machine.thread_mut().unwrap().regs[3] = 0x100;
        machine.thread_mut().unwrap().regs[4] = 1;

        machine.exec(Instr::FutexWake(Reg(3), Reg(4))).unwrap();

        assert!(!machine.futex_waiters.contains_key(&0x100));
        assert!(machine.ready.contains(&waiter_tid));
    }

    #[test]
    fn lock_cmpxchg_rejects_locked_result_before_store() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let addr = ARG_BASE + 0x180;
        machine.store_u64(addr, 7).unwrap();
        machine.thread_mut().unwrap().regs[2] = addr;
        machine.thread_mut().unwrap().regs[3] = 7;
        machine.thread_mut().unwrap().regs[4] = 9;

        let err = machine
            .exec(Instr::LockCmpxchg(Reg(31), Reg(2), Reg(3), Reg(4)))
            .unwrap_err();

        assert!(err.contains("hardware-locked stack pointer"), "{err}");
        assert_eq!(machine.load_u64(addr).unwrap(), 7);
    }

    #[test]
    fn thread_join_waiter_wakes_and_consumes_exit_status() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine
            .clone_with_profile(CloneProfile::NewThreadSharedVm, Reg(2), Some(0))
            .unwrap();
        let child_tid = machine.thread().unwrap().regs[2];
        let retval = ARG_BASE + 0x200;
        machine.thread_mut().unwrap().regs[3] = child_tid;
        machine.thread_mut().unwrap().regs[4] = retval;

        let keep_ready = machine
            .exec(Instr::ThreadJoin(Reg(5), Reg(3), Reg(4)))
            .unwrap();
        assert!(!keep_ready);
        assert!(!machine.ready.contains(&1));
        assert_eq!(
            machine
                .thread_join_waiters
                .get(&child_tid)
                .unwrap()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![1]
        );

        machine.current_tid = child_tid;
        machine.exit_current(77).unwrap();
        assert!(machine.ready.contains(&1));
        assert!(!machine.threads.contains_key(&child_tid));
        assert_eq!(machine.completed_threads.get(&child_tid), Some(&77));

        machine.current_tid = 1;
        machine
            .exec(Instr::ThreadJoin(Reg(5), Reg(3), Reg(4)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[5], 0);
        assert_eq!(machine.load_u64(retval).unwrap(), 77);
        assert!(!machine.completed_threads.contains_key(&child_tid));
    }

    #[test]
    fn thread_join_invalid_retval_preserves_completed_status() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine
            .clone_with_profile(CloneProfile::NewThreadSharedVm, Reg(2), Some(0))
            .unwrap();
        let child_tid = machine.thread().unwrap().regs[2];

        machine.current_tid = child_tid;
        machine.exit_current(77).unwrap();
        assert_eq!(machine.completed_threads.get(&child_tid), Some(&77));

        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[3] = child_tid;
        machine.thread_mut().unwrap().regs[4] = u64::MAX - 7;
        machine.thread_mut().unwrap().regs[5] = 0xdead_beef;

        let err = machine
            .exec(Instr::ThreadJoin(Reg(5), Reg(3), Reg(4)))
            .unwrap_err();
        assert!(err.contains("unmapped address"), "{err}");
        assert_eq!(machine.completed_threads.get(&child_tid), Some(&77));
        assert_eq!(machine.thread().unwrap().regs[5], 0xdead_beef);

        let retval = ARG_BASE + 0x240;
        machine.thread_mut().unwrap().regs[4] = retval;
        machine
            .exec(Instr::ThreadJoin(Reg(5), Reg(3), Reg(4)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[5], 0);
        assert_eq!(machine.load_u64(retval).unwrap(), 77);
        assert!(!machine.completed_threads.contains_key(&child_tid));
    }

    #[test]
    fn exiting_thread_is_removed_from_all_wait_queues() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.ready.push_back(1);
        machine.domain_parked.push_back(1);
        machine.sleepers.push((1, 10));
        machine.fd_waiters.push(FdWaiter {
            tid: 1,
            fd: 0,
            generation: machine.fd_generation(0).unwrap(),
            mask: 0,
            result: None,
        });
        machine.futex_waiters.entry(0x100).or_default().push_back(1);
        machine
            .thread_join_waiters
            .entry(99)
            .or_default()
            .push_back(1);
        machine.child_waiters.entry(1).or_default().push_back(1);

        machine.exit_current(9).unwrap();

        assert!(!machine.ready.contains(&1));
        assert!(!machine.domain_parked.contains(&1));
        assert!(machine.sleepers.is_empty());
        assert!(machine.fd_waiters.is_empty());
        assert!(machine.futex_waiters.is_empty());
        assert!(machine.thread_join_waiters.is_empty());
        assert!(machine.child_waiters.is_empty());
        assert!(!machine.threads.contains_key(&1));
        assert_eq!(machine.completed_threads.get(&1), Some(&9));
    }

    #[test]
    fn waitpid_specific_child_consumes_completed_status() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine
            .clone_with_profile(CloneProfile::NewProcessCow, Reg(2), None)
            .unwrap();
        let child_pid = machine.thread().unwrap().regs[2];
        let child_tid = machine
            .threads
            .values()
            .find(|thread| thread.pid == child_pid)
            .unwrap()
            .tid;

        machine.current_tid = child_tid;
        machine.exit_current(42).unwrap();
        assert_eq!(machine.completed_children.get(&(1, child_pid)), Some(&42));

        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[3] = child_pid;
        machine.exec(Instr::WaitPid(Reg(4), Reg(3))).unwrap();

        assert_eq!(machine.thread().unwrap().regs[4], 42);
        assert_eq!(machine.thread().unwrap().regs[1], 0);
        assert!(!machine.completed_children.contains_key(&(1, child_pid)));
    }

    #[test]
    fn waitpid_rejects_status_register_alias() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine
            .clone_with_profile(CloneProfile::NewProcessCow, Reg(2), None)
            .unwrap();
        let child_pid = machine.thread().unwrap().regs[2];
        let child_tid = machine
            .threads
            .values()
            .find(|thread| thread.pid == child_pid)
            .unwrap()
            .tid;

        machine.current_tid = child_tid;
        machine.exit_current(42).unwrap();
        assert_eq!(machine.completed_children.get(&(1, child_pid)), Some(&42));

        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[3] = child_pid;
        let err = machine.exec(Instr::WaitPid(Reg(1), Reg(3))).unwrap_err();

        assert!(err.contains("aliases status register"), "{err}");
        assert_eq!(machine.completed_children.get(&(1, child_pid)), Some(&42));
    }

    #[test]
    fn waitpid_live_child_parks_until_child_exit_event() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine
            .clone_with_profile(CloneProfile::NewProcessCow, Reg(2), None)
            .unwrap();
        let child_pid = machine.thread().unwrap().regs[2];
        let child_tid = machine
            .threads
            .values()
            .find(|thread| thread.pid == child_pid)
            .unwrap()
            .tid;

        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[3] = child_pid;
        let keep_ready = machine.exec(Instr::WaitPid(Reg(4), Reg(3))).unwrap();
        assert!(!keep_ready);
        assert!(!machine.ready.contains(&1));
        assert!(machine.sleepers.is_empty());
        assert_eq!(
            machine
                .child_waiters
                .get(&1)
                .unwrap()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![1]
        );

        machine.current_tid = child_tid;
        machine.exit_current(55).unwrap();
        assert!(machine.ready.contains(&1));

        machine.current_tid = 1;
        machine.exec(Instr::WaitPid(Reg(4), Reg(3))).unwrap();
        assert_eq!(machine.thread().unwrap().regs[4], 55);
        assert_eq!(machine.thread().unwrap().regs[1], 0);
    }

    #[test]
    fn repeated_parking_does_not_duplicate_wait_queue_entries() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine
            .clone_with_profile(CloneProfile::NewProcessCow, Reg(2), None)
            .unwrap();
        let child_pid = machine.thread().unwrap().regs[2];
        machine.thread_mut().unwrap().regs[3] = child_pid;

        machine.exec(Instr::WaitPid(Reg(4), Reg(3))).unwrap();
        machine.exec(Instr::WaitPid(Reg(4), Reg(3))).unwrap();
        assert_eq!(
            machine
                .child_waiters
                .get(&1)
                .unwrap()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![1]
        );

        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine
            .clone_with_profile(CloneProfile::NewThreadSharedVm, Reg(2), Some(0))
            .unwrap();
        let child_tid = machine.thread().unwrap().regs[2];
        machine.thread_mut().unwrap().regs[3] = child_tid;
        machine.thread_mut().unwrap().regs[4] = 0;

        machine
            .exec(Instr::ThreadJoin(Reg(5), Reg(3), Reg(4)))
            .unwrap();
        machine
            .exec(Instr::ThreadJoin(Reg(5), Reg(3), Reg(4)))
            .unwrap();
        assert_eq!(
            machine
                .thread_join_waiters
                .get(&child_tid)
                .unwrap()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![1]
        );

        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let futex_addr = ARG_BASE + 0x280;
        machine.store_u64(futex_addr, 7).unwrap();
        machine.thread_mut().unwrap().regs[2] = futex_addr;
        machine.thread_mut().unwrap().regs[3] = 7;

        machine.exec(Instr::FutexWait(Reg(2), Reg(3))).unwrap();
        machine.exec(Instr::FutexWait(Reg(2), Reg(3))).unwrap();
        assert_eq!(
            machine
                .futex_waiters
                .get(&futex_addr)
                .unwrap()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![1]
        );
    }

    #[test]
    fn waitpid_specific_non_child_reports_echild() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[2] = 999;

        machine.exec(Instr::WaitPid(Reg(4), Reg(2))).unwrap();

        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 10);
        assert_eq!(machine.thread().unwrap().regs[4], 0);
    }

    #[test]
    fn waitpid_any_child_prefers_completed_status_over_live_child() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine
            .clone_with_profile(CloneProfile::NewProcessCow, Reg(2), None)
            .unwrap();
        let completed_pid = machine.thread().unwrap().regs[2];
        machine.current_tid = 1;
        machine
            .clone_with_profile(CloneProfile::NewProcessCow, Reg(3), None)
            .unwrap();
        let live_pid = machine.thread().unwrap().regs[3];
        assert!(machine.processes.contains_key(&live_pid));
        let completed_tid = machine
            .threads
            .values()
            .find(|thread| thread.pid == completed_pid)
            .unwrap()
            .tid;

        machine.current_tid = completed_tid;
        machine.exit_current(33).unwrap();

        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[4] = 0;
        let keep_ready = machine.exec(Instr::WaitPid(Reg(5), Reg(4))).unwrap();

        assert!(keep_ready);
        assert_eq!(machine.thread().unwrap().regs[5], 33);
        assert_eq!(machine.thread().unwrap().regs[1], 0);
        assert!(machine.processes.contains_key(&live_pid));
        assert!(!machine.completed_children.contains_key(&(1, completed_pid)));
    }

    #[test]
    fn orphan_child_exit_does_not_record_unwaitable_status() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine
            .clone_with_profile(CloneProfile::NewProcessCow, Reg(2), None)
            .unwrap();
        let child_pid = machine.thread().unwrap().regs[2];
        let child_tid = machine
            .threads
            .values()
            .find(|thread| thread.pid == child_pid)
            .unwrap()
            .tid;

        machine.current_tid = 1;
        machine.exit_current(0).unwrap();
        assert!(!machine.processes.contains_key(&1));

        machine.current_tid = child_tid;
        machine.exit_current(7).unwrap();

        assert!(!machine.completed_children.contains_key(&(1, child_pid)));
    }

    #[test]
    fn unmapped_vma_rejects_stale_memory_access() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[1] = 4096;
        machine.thread_mut().unwrap().regs[2] = 0b011;
        machine
            .exec(Instr::Mmap(
                Reg(3),
                Reg(0),
                Reg(1),
                Reg(2),
                FdReg(0),
                Reg(0),
            ))
            .unwrap();
        let addr = machine.thread().unwrap().regs[3];
        machine.write_bytes(addr, &[1]).unwrap();

        machine.thread_mut().unwrap().regs[4] = addr;
        machine.thread_mut().unwrap().regs[5] = 4096;
        machine.exec(Instr::Munmap(Reg(4), Reg(5))).unwrap();

        let err = machine.read_bytes(addr, 1).unwrap_err();
        assert!(err.contains("unmapped address"), "{err}");
        let err = machine.write_bytes(addr, &[2]).unwrap_err();
        assert!(err.contains("unmapped address"), "{err}");
    }

    #[test]
    fn anonymous_mmap_remap_zero_fills_old_bytes() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[1] = 4096;
        machine.thread_mut().unwrap().regs[2] = 0b011;
        machine
            .exec(Instr::Mmap(
                Reg(3),
                Reg(0),
                Reg(1),
                Reg(2),
                FdReg(0),
                Reg(0),
            ))
            .unwrap();
        let addr = machine.thread().unwrap().regs[3];
        machine
            .write_bytes(addr, &[0xaa, 0xbb, 0xcc, 0xdd])
            .unwrap();

        machine.thread_mut().unwrap().regs[4] = addr;
        machine.thread_mut().unwrap().regs[5] = 4096;
        machine.exec(Instr::Munmap(Reg(4), Reg(5))).unwrap();

        machine.thread_mut().unwrap().regs[6] = addr;
        machine
            .exec(Instr::Mmap(
                Reg(7),
                Reg(6),
                Reg(1),
                Reg(2),
                FdReg(0),
                Reg(0),
            ))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[7], addr);
        assert_eq!(machine.read_bytes(addr, 4).unwrap(), vec![0, 0, 0, 0]);
    }

    #[test]
    fn file_backed_mmap_zero_fills_short_read_tail() {
        let path = format!("/tmp/lnp64_short_vma_pagein_{}.bin", std::process::id());
        fs::write(&path, b"ab").unwrap();
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let addr = 0x270_000;
        {
            let process = machine.process_mut().unwrap();
            process.memory[addr as usize..addr as usize + 4].copy_from_slice(&[9, 9, 9, 9]);
            process.vmas.push(Vma {
                start: addr,
                len: 4,
                prot: 0b001,
                file: Some(File::open(&path).unwrap()),
                file_offset: 0,
                resident: false,
                guard: false,
            });
        }

        assert_eq!(machine.read_bytes(addr, 4).unwrap(), vec![b'a', b'b', 0, 0]);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn cap_dup_can_only_narrow_rights() {
        let program = Program::parse(
            r#"
            .data
            path: .string "Cargo.toml"

            .text
              NOP
            "#,
        )
        .unwrap();
        let path = program.data_labels["path"];
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[1] = path;
        machine.thread_mut().unwrap().regs[2] = 0;
        machine
            .exec(Instr::OpenFdDyn(Reg(3), Reg(1), Reg(2)))
            .unwrap();
        let source = machine.thread().unwrap().regs[3];
        let arg = ARG_BASE;
        machine.store_u64(arg, source).unwrap();
        machine.store_u64(arg + 8, 0).unwrap();
        machine.store_u64(arg + 16, CAP_RIGHT_READ).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_dup(Reg(4), arg).unwrap();
        let readonly = machine.thread().unwrap().regs[4];
        assert_ne!(readonly, -1i64 as u64);

        machine.thread_mut().unwrap().regs[5] = readonly;
        machine.thread_mut().unwrap().regs[6] = ARG_BASE + 0x1000;
        machine.thread_mut().unwrap().regs[7] = 1;
        machine
            .exec(Instr::WriteFdDyn(Reg(5), Reg(6), Reg(7)))
            .unwrap();
        assert_eq!(machine.process().unwrap().errno, 1);

        machine.store_u64(arg, readonly).unwrap();
        machine.store_u64(arg + 8, 0).unwrap();
        machine
            .store_u64(arg + 16, CAP_RIGHT_READ | CAP_RIGHT_WRITE)
            .unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_dup(Reg(8), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[8], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);

        machine.store_u64(arg, readonly).unwrap();
        machine.store_u64(arg + 8, 8).unwrap();
        machine
            .store_u64(arg + 16, CAP_RIGHT_READ | CAP_RIGHT_WRITE)
            .unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_dup(Reg(9), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[9], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
        assert!(matches!(
            machine.process().unwrap().fds[8],
            FdHandle::Closed
        ));

        let retained = Rc::new(RefCell::new(99));
        {
            let process = machine.process_mut().unwrap();
            process.fds[8] = FdHandle::Counter(retained.clone());
            process.fd_capabilities[8] = FdCapability::full(8);
        }
        machine.cap_dup(Reg(10), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[10], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
        match &machine.process().unwrap().fds[8] {
            FdHandle::Counter(value) => {
                assert!(Rc::ptr_eq(value, &retained));
                assert_eq!(*value.borrow(), 99);
            }
            _ => panic!("expected retained counter fd"),
        }
    }

    #[test]
    fn cap_dup_rejects_unknown_flags_without_installing_destination() {
        let program = Program::parse(
            r#"
            .data
            path: .string "Cargo.toml"

            .text
              NOP
            "#,
        )
        .unwrap();
        let path = program.data_labels["path"];
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[1] = path;
        machine.thread_mut().unwrap().regs[2] = 0;
        machine
            .exec(Instr::OpenFdDyn(Reg(3), Reg(1), Reg(2)))
            .unwrap();
        let source = machine.thread().unwrap().regs[3];
        let arg = ARG_BASE;
        machine.store_u64(arg, source).unwrap();
        machine.store_u64(arg + 8, 8).unwrap();
        machine.store_u64(arg + 16, CAP_RIGHT_READ).unwrap();
        machine.store_u64(arg + 24, 1 << 4).unwrap();

        machine.cap_dup(Reg(4), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        assert!(matches!(
            machine.process().unwrap().fds[8],
            FdHandle::Closed
        ));
    }

    #[test]
    fn sealed_capability_can_be_used_but_not_duplicated_or_narrowed() {
        let program = Program::parse(
            r#"
            .data
            path: .string "Cargo.toml"

            .text
              NOP
            "#,
        )
        .unwrap();
        let path = program.data_labels["path"];
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[1] = path;
        machine.thread_mut().unwrap().regs[2] = 0;
        machine
            .exec(Instr::OpenFdDyn(Reg(3), Reg(1), Reg(2)))
            .unwrap();
        let source = machine.thread().unwrap().regs[3];
        let arg = ARG_BASE;
        machine.store_u64(arg, source).unwrap();
        machine.store_u64(arg + 8, 0).unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, CAP_DUP_FLAG_SEAL).unwrap();
        machine.cap_dup(Reg(4), arg).unwrap();
        let sealed = machine.thread().unwrap().regs[4];
        assert_ne!(sealed, -1i64 as u64);

        machine.thread_mut().unwrap().regs[6] = sealed;
        machine.thread_mut().unwrap().regs[7] = ARG_BASE + 0x1000;
        machine.thread_mut().unwrap().regs[8] = 4;
        machine
            .exec(Instr::ReadFdDyn(Reg(6), Reg(7), Reg(8)))
            .unwrap();
        assert_eq!(machine.process().unwrap().errno, 0);
        assert_eq!(machine.thread().unwrap().regs[1], 4);

        machine.store_u64(arg, sealed).unwrap();
        machine.store_u64(arg + 8, 0).unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_dup(Reg(5), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[5], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);

        machine.store_u64(arg, sealed).unwrap();
        machine.store_u64(arg + 8, 0).unwrap();
        machine.store_u64(arg + 16, CAP_RIGHT_READ).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_dup(Reg(9), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[9], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
    }

    #[test]
    fn cap_revoke_invalidates_descendant_capabilities() {
        let program = Program::parse(
            r#"
            .data
            path: .string "Cargo.toml"

            .text
              NOP
            "#,
        )
        .unwrap();
        let path = program.data_labels["path"];
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[1] = path;
        machine.thread_mut().unwrap().regs[2] = 0;
        machine
            .exec(Instr::OpenFdDyn(Reg(3), Reg(1), Reg(2)))
            .unwrap();
        let source = machine.thread().unwrap().regs[3];
        let arg = ARG_BASE;
        machine.store_u64(arg, source).unwrap();
        machine.store_u64(arg + 8, 0).unwrap();
        machine
            .store_u64(arg + 16, CAP_RIGHT_READ | CAP_RIGHT_REVOKE)
            .unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_dup(Reg(4), arg).unwrap();
        let child = machine.thread().unwrap().regs[4];

        machine.store_u64(arg, source).unwrap();
        machine.cap_revoke(Reg(5), arg).unwrap();
        assert!(machine.thread().unwrap().regs[5] >= 2);

        machine.thread_mut().unwrap().regs[6] = child;
        machine.thread_mut().unwrap().regs[7] = ARG_BASE + 0x1000;
        machine.thread_mut().unwrap().regs[8] = 1;
        machine
            .exec(Instr::ReadFdDyn(Reg(6), Reg(7), Reg(8)))
            .unwrap();
        assert_eq!(machine.process().unwrap().errno, 116);
        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
    }

    #[test]
    fn cap_dup_rejects_locked_result_before_install_or_errno_side_effects() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        {
            let process = machine.process_mut().unwrap();
            process.fds[5] = FdHandle::Counter(Rc::new(RefCell::new(99)));
            process.fd_capabilities[5] = FdCapability::full(5);
        }
        let source = machine.fd_token(5).unwrap();
        machine.set_errno(123).unwrap();
        let arg = ARG_BASE;
        machine.store_u64(arg, source).unwrap();
        machine.store_u64(arg + 8, 6).unwrap();
        machine.store_u64(arg + 16, CAP_RIGHT_READ).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();

        let err = machine.cap_dup(Reg(31), arg).unwrap_err();

        assert!(err.contains("stack pointer"), "{err}");
        assert_eq!(machine.process().unwrap().errno, 123);
        assert!(matches!(
            machine.process().unwrap().fds[6],
            FdHandle::Closed
        ));
    }

    #[test]
    fn cap_revoke_rejects_locked_result_before_revocation_or_errno_side_effects() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        {
            let process = machine.process_mut().unwrap();
            process.fds[5] = FdHandle::Counter(Rc::new(RefCell::new(99)));
            process.fd_capabilities[5] = FdCapability::full(5);
        }
        let source = machine.fd_token(5).unwrap();
        let arg = ARG_BASE;
        machine.store_u64(arg, source).unwrap();
        machine.store_u64(arg + 8, 6).unwrap();
        machine
            .store_u64(arg + 16, CAP_RIGHT_READ | CAP_RIGHT_REVOKE)
            .unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_dup(Reg(7), arg).unwrap();
        machine.set_errno(123).unwrap();
        machine.store_u64(arg, source).unwrap();

        let err = machine.cap_revoke(Reg(31), arg).unwrap_err();

        assert!(err.contains("stack pointer"), "{err}");
        assert_eq!(machine.process().unwrap().errno, 123);
        assert!(!machine.process().unwrap().fd_capabilities[5].revoked);
        assert!(!machine.process().unwrap().fd_capabilities[6].revoked);
    }

    #[test]
    fn cap_send_requires_transfer_right() {
        let program = Program::parse(
            r#"
            .data
            path: .string "Cargo.toml"

            .text
              NOP
            "#,
        )
        .unwrap();
        let path = program.data_labels["path"];
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        let pipe = Rc::new(RefCell::new(PipeBuffer::default()));
        machine.processes.get_mut(&1).unwrap().fds[3] = FdHandle::PipeReader(Rc::clone(&pipe));
        machine.processes.get_mut(&1).unwrap().fd_capabilities[3] = FdCapability::full(3);
        machine.processes.get_mut(&1).unwrap().fds[4] = FdHandle::PipeWriter(pipe);
        machine.processes.get_mut(&1).unwrap().fd_capabilities[4] = FdCapability::full(4);

        machine.thread_mut().unwrap().regs[1] = path;
        machine.thread_mut().unwrap().regs[2] = 0;
        machine
            .exec(Instr::OpenFdDyn(Reg(5), Reg(1), Reg(2)))
            .unwrap();
        let source = machine.thread().unwrap().regs[5];
        let arg = ARG_BASE;

        machine.processes.get_mut(&1).unwrap().fd_capabilities[4].rights &= !CAP_RIGHT_TRANSFER;
        machine.store_u64(arg, 4).unwrap();
        machine.store_u64(arg + 8, source).unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_send(Reg(6), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[6], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
        machine.processes.get_mut(&1).unwrap().fd_capabilities[4].rights |= CAP_RIGHT_TRANSFER;

        machine.store_u64(arg, source).unwrap();
        machine.store_u64(arg + 8, 0).unwrap();
        machine.store_u64(arg + 16, CAP_RIGHT_READ).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_dup(Reg(6), arg).unwrap();
        let no_transfer = machine.thread().unwrap().regs[6];

        machine.store_u64(arg, 4).unwrap();
        machine.store_u64(arg + 8, no_transfer).unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_send(Reg(7), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[7], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
    }

    #[test]
    fn cap_send_recv_transfers_narrowed_capability() {
        let program = Program::parse(
            r#"
            .data
            path: .string "Cargo.toml"

            .text
              NOP
            "#,
        )
        .unwrap();
        let path = program.data_labels["path"];
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        let pipe = Rc::new(RefCell::new(PipeBuffer::default()));
        machine.processes.get_mut(&1).unwrap().fds[3] = FdHandle::PipeReader(Rc::clone(&pipe));
        machine.processes.get_mut(&1).unwrap().fd_capabilities[3] = FdCapability::full(3);
        machine.processes.get_mut(&1).unwrap().fds[4] = FdHandle::PipeWriter(pipe);
        machine.processes.get_mut(&1).unwrap().fd_capabilities[4] = FdCapability::full(4);

        machine.thread_mut().unwrap().regs[1] = path;
        machine.thread_mut().unwrap().regs[2] = 0;
        machine
            .exec(Instr::OpenFdDyn(Reg(5), Reg(1), Reg(2)))
            .unwrap();
        let source = machine.thread().unwrap().regs[5];
        let arg = ARG_BASE;
        machine.store_u64(arg, 4).unwrap();
        machine.store_u64(arg + 8, source).unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_send(Reg(6), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[6], 1);

        machine.processes.get_mut(&1).unwrap().fd_capabilities[3].rights &= !CAP_RIGHT_TRANSFER;
        machine.store_u64(arg, 3).unwrap();
        machine.store_u64(arg + 8, 0).unwrap();
        machine.store_u64(arg + 16, CAP_RIGHT_READ).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_recv(Reg(7), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[7], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
        machine.processes.get_mut(&1).unwrap().fd_capabilities[3].rights |= CAP_RIGHT_TRANSFER;

        machine.store_u64(arg, 3).unwrap();
        machine.store_u64(arg + 8, 0).unwrap();
        machine.store_u64(arg + 16, CAP_RIGHT_READ).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_recv(Reg(7), arg).unwrap();
        let received = machine.thread().unwrap().regs[7];
        assert_ne!(received, -1i64 as u64);
        let received_fd = machine.decode_fd_value(received).unwrap();
        assert_eq!(
            machine.process().unwrap().fd_capabilities[received_fd].rights,
            CAP_RIGHT_READ
        );

        machine.thread_mut().unwrap().regs[8] = received;
        machine.thread_mut().unwrap().regs[9] = ARG_BASE + 0x1000;
        machine.thread_mut().unwrap().regs[10] = 1;
        machine
            .exec(Instr::WriteFdDyn(Reg(8), Reg(9), Reg(10)))
            .unwrap();
        assert_eq!(machine.process().unwrap().errno, 1);
    }

    #[test]
    fn cap_send_wakes_reader_waiting_for_capability_payload() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        create_pipe_pair(&mut machine, 3, 4);
        {
            let process = machine.process_mut().unwrap();
            process.fds[5] = FdHandle::Counter(Rc::new(RefCell::new(99)));
            process.fd_capabilities[5] = FdCapability::full(5);
        }
        machine
            .push_fd_waiter(3, POLLIN_MASK, Some(Reg(8)))
            .unwrap();
        machine.ready.retain(|tid| *tid != 1);

        let arg = ARG_BASE;
        machine.store_u64(arg, 4).unwrap();
        machine.store_u64(arg + 8, 5).unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_send(Reg(6), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[6], 1);
        assert!(machine.ready.contains(&1));
        assert!(machine.fd_waiters.is_empty());
        assert_eq!(machine.thread().unwrap().regs[8], 0);
        assert_eq!(machine.process().unwrap().errno, 0);
        assert!(machine.fd_read_ready(3).unwrap());
    }

    #[test]
    fn cap_recv_wakes_writer_waiting_for_capability_queue_space() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        create_pipe_pair(&mut machine, 3, 4);
        let queue = match &machine.process().unwrap().fds[4] {
            FdHandle::PipeWriter(queue) => Rc::clone(queue),
            _ => panic!("expected pipe writer"),
        };
        {
            let mut queue = queue.borrow_mut();
            queue.bytes = vec![0; PIPE_BUFFER_BYTE_LIMIT].into();
            for idx in 0..PIPE_CAPABILITY_LIMIT {
                queue.capabilities.push_back(CapabilityPayload {
                    handle: FdHandle::Counter(Rc::new(RefCell::new(idx as u64))),
                    capability: FdCapability::full(1000 + idx as u64),
                });
            }
        }
        machine.push_fd_waiter(4, 0, Some(Reg(8))).unwrap();
        machine.ready.retain(|tid| *tid != 1);

        let arg = ARG_BASE;
        machine.store_u64(arg, 3).unwrap();
        machine.store_u64(arg + 8, 6).unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_recv(Reg(7), arg).unwrap();

        assert_ne!(machine.thread().unwrap().regs[7], -1i64 as u64);
        assert!(machine.ready.contains(&1));
        assert!(machine.fd_waiters.is_empty());
        assert_eq!(machine.thread().unwrap().regs[8], 0);
        assert_eq!(machine.process().unwrap().errno, 0);
        assert!(machine.fd_ready(4).unwrap());
    }

    #[test]
    fn cap_send_move_closes_source_after_queueing_capability() {
        let program = Program::parse(
            r#"
            .data
            path: .string "Cargo.toml"

            .text
              NOP
            "#,
        )
        .unwrap();
        let path = program.data_labels["path"];
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        let pipe = Rc::new(RefCell::new(PipeBuffer::default()));
        machine.processes.get_mut(&1).unwrap().fds[3] = FdHandle::PipeReader(Rc::clone(&pipe));
        machine.processes.get_mut(&1).unwrap().fd_capabilities[3] = FdCapability::full(3);
        machine.processes.get_mut(&1).unwrap().fds[4] = FdHandle::PipeWriter(pipe);
        machine.processes.get_mut(&1).unwrap().fd_capabilities[4] = FdCapability::full(4);

        machine.thread_mut().unwrap().regs[1] = path;
        machine.thread_mut().unwrap().regs[2] = 0;
        machine
            .exec(Instr::OpenFdDyn(Reg(5), Reg(1), Reg(2)))
            .unwrap();
        let source = machine.thread().unwrap().regs[5];
        let source_fd = machine.decode_fd_value(source).unwrap();
        let arg = ARG_BASE;
        machine.store_u64(arg, 4).unwrap();
        machine.store_u64(arg + 8, source).unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, CAP_SEND_FLAG_MOVE).unwrap();
        machine.cap_send(Reg(6), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[6], 1);
        assert!(matches!(
            machine.process().unwrap().fds[source_fd],
            FdHandle::Closed
        ));

        machine.thread_mut().unwrap().regs[7] = source;
        machine.thread_mut().unwrap().regs[8] = ARG_BASE + 0x1000;
        machine.thread_mut().unwrap().regs[9] = 1;
        machine
            .exec(Instr::ReadFdDyn(Reg(7), Reg(8), Reg(9)))
            .unwrap();
        assert_eq!(machine.process().unwrap().errno, 116);

        machine.store_u64(arg, 3).unwrap();
        machine.store_u64(arg + 8, 0).unwrap();
        machine.store_u64(arg + 16, CAP_RIGHT_READ).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_recv(Reg(10), arg).unwrap();
        let received = machine.thread().unwrap().regs[10];
        assert_ne!(received, -1i64 as u64);

        machine.thread_mut().unwrap().regs[11] = received;
        machine.thread_mut().unwrap().regs[12] = ARG_BASE + 0x1100;
        machine.thread_mut().unwrap().regs[13] = 1;
        machine
            .exec(Instr::ReadFdDyn(Reg(11), Reg(12), Reg(13)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[1], 1);
        assert_eq!(machine.process().unwrap().errno, 0);
    }

    #[test]
    fn cap_recv_cannot_expand_transferred_rights() {
        let program = Program::parse(
            r#"
            .data
            path: .string "Cargo.toml"

            .text
              NOP
            "#,
        )
        .unwrap();
        let path = program.data_labels["path"];
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        let pipe = Rc::new(RefCell::new(PipeBuffer::default()));
        machine.processes.get_mut(&1).unwrap().fds[3] = FdHandle::PipeReader(Rc::clone(&pipe));
        machine.processes.get_mut(&1).unwrap().fd_capabilities[3] = FdCapability::full(3);
        machine.processes.get_mut(&1).unwrap().fds[4] = FdHandle::PipeWriter(pipe);
        machine.processes.get_mut(&1).unwrap().fd_capabilities[4] = FdCapability::full(4);

        machine.thread_mut().unwrap().regs[1] = path;
        machine.thread_mut().unwrap().regs[2] = 0;
        machine
            .exec(Instr::OpenFdDyn(Reg(5), Reg(1), Reg(2)))
            .unwrap();
        let source = machine.thread().unwrap().regs[5];
        let arg = ARG_BASE;
        machine.store_u64(arg, source).unwrap();
        machine.store_u64(arg + 8, 0).unwrap();
        machine
            .store_u64(arg + 16, CAP_RIGHT_READ | CAP_RIGHT_TRANSFER)
            .unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_dup(Reg(6), arg).unwrap();
        let transferable_read = machine.thread().unwrap().regs[6];

        machine.store_u64(arg, 4).unwrap();
        machine.store_u64(arg + 8, transferable_read).unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_send(Reg(7), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[7], 1);

        machine.store_u64(arg, 3).unwrap();
        machine.store_u64(arg + 8, 0).unwrap();
        machine
            .store_u64(arg + 16, CAP_RIGHT_READ | CAP_RIGHT_WRITE)
            .unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_recv(Reg(8), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[8], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);

        machine.store_u64(arg, 3).unwrap();
        machine.store_u64(arg + 8, 8).unwrap();
        machine
            .store_u64(arg + 16, CAP_RIGHT_READ | CAP_RIGHT_WRITE)
            .unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_recv(Reg(10), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[10], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
        assert!(matches!(
            machine.process().unwrap().fds[8],
            FdHandle::Closed
        ));

        let retained = Rc::new(RefCell::new(111));
        {
            let process = machine.process_mut().unwrap();
            process.fds[8] = FdHandle::Counter(retained.clone());
            process.fd_capabilities[8] = FdCapability::full(8);
        }
        machine.cap_recv(Reg(11), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[11], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
        match &machine.process().unwrap().fds[8] {
            FdHandle::Counter(value) => {
                assert!(Rc::ptr_eq(value, &retained));
                assert_eq!(*value.borrow(), 111);
            }
            _ => panic!("expected retained counter fd"),
        }

        machine.store_u64(arg + 16, CAP_RIGHT_READ).unwrap();
        machine.cap_recv(Reg(9), arg).unwrap();
        let received = machine.thread().unwrap().regs[9];
        assert_ne!(received, -1i64 as u64);
        let received_fd = machine.decode_fd_value(received).unwrap();
        assert_eq!(
            machine.process().unwrap().fd_capabilities[received_fd].rights,
            CAP_RIGHT_READ
        );
    }

    #[test]
    fn cap_revoke_invalidates_received_capability() {
        let program = Program::parse(
            r#"
            .data
            path: .string "Cargo.toml"

            .text
              NOP
            "#,
        )
        .unwrap();
        let path = program.data_labels["path"];
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        let pipe = Rc::new(RefCell::new(PipeBuffer::default()));
        machine.processes.get_mut(&1).unwrap().fds[3] = FdHandle::PipeReader(Rc::clone(&pipe));
        machine.processes.get_mut(&1).unwrap().fd_capabilities[3] = FdCapability::full(3);
        machine.processes.get_mut(&1).unwrap().fds[4] = FdHandle::PipeWriter(pipe);
        machine.processes.get_mut(&1).unwrap().fd_capabilities[4] = FdCapability::full(4);

        machine.thread_mut().unwrap().regs[1] = path;
        machine.thread_mut().unwrap().regs[2] = 0;
        machine
            .exec(Instr::OpenFdDyn(Reg(5), Reg(1), Reg(2)))
            .unwrap();
        let source = machine.thread().unwrap().regs[5];
        let arg = ARG_BASE;
        machine.store_u64(arg, 4).unwrap();
        machine.store_u64(arg + 8, source).unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_send(Reg(6), arg).unwrap();

        machine.store_u64(arg, 3).unwrap();
        machine.store_u64(arg + 8, 0).unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_recv(Reg(7), arg).unwrap();
        let received = machine.thread().unwrap().regs[7];

        machine.store_u64(arg, source).unwrap();
        machine.cap_revoke(Reg(8), arg).unwrap();
        assert!(machine.thread().unwrap().regs[8] >= 2);

        machine.thread_mut().unwrap().regs[9] = received;
        machine.thread_mut().unwrap().regs[10] = ARG_BASE + 0x1000;
        machine.thread_mut().unwrap().regs[11] = 1;
        machine
            .exec(Instr::ReadFdDyn(Reg(9), Reg(10), Reg(11)))
            .unwrap();
        assert_eq!(machine.process().unwrap().errno, 116);
        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
    }

    #[test]
    fn random_scalar_and_buffer_are_deterministic() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut first = Machine::new(program.clone());
        let mut second = Machine::new(program);
        first.current_tid = 1;
        second.current_tid = 1;

        first.exec(Instr::Random(Reg(1), Reg(0), Reg(0))).unwrap();
        second.exec(Instr::Random(Reg(1), Reg(0), Reg(0))).unwrap();
        assert_eq!(
            first.thread().unwrap().regs[1],
            second.thread().unwrap().regs[1]
        );
        assert_ne!(first.thread().unwrap().regs[1], 0);

        first.thread_mut().unwrap().regs[2] = ARG_BASE;
        first.thread_mut().unwrap().regs[3] = 16;
        second.thread_mut().unwrap().regs[2] = ARG_BASE;
        second.thread_mut().unwrap().regs[3] = 16;
        first.exec(Instr::Random(Reg(4), Reg(2), Reg(3))).unwrap();
        second.exec(Instr::Random(Reg(4), Reg(2), Reg(3))).unwrap();
        assert_eq!(first.thread().unwrap().regs[4], 16);
        assert_eq!(
            first.read_bytes(ARG_BASE, 16).unwrap(),
            second.read_bytes(ARG_BASE, 16).unwrap()
        );
        assert_ne!(first.read_bytes(ARG_BASE, 16).unwrap(), vec![0; 16]);
    }

    #[test]
    fn random_obeys_domain_entropy_quota() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        machine
            .domains
            .get_mut(&ROOT_DOMAIN_ID)
            .unwrap()
            .security
            .entropy_quota = 4;

        machine.exec(Instr::Random(Reg(1), Reg(0), Reg(0))).unwrap();
        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
        assert_eq!(machine.domains[&ROOT_DOMAIN_ID].security.entropy_quota, 4);

        machine.thread_mut().unwrap().regs[2] = ARG_BASE;
        machine.thread_mut().unwrap().regs[3] = 4;
        machine.exec(Instr::Random(Reg(4), Reg(2), Reg(3))).unwrap();
        assert_eq!(machine.thread().unwrap().regs[4], 4);
        assert_eq!(machine.domains[&ROOT_DOMAIN_ID].security.entropy_quota, 0);

        let before_denied = machine.read_bytes(ARG_BASE, 4).unwrap();
        machine.thread_mut().unwrap().regs[3] = 1;
        machine.exec(Instr::Random(Reg(5), Reg(2), Reg(3))).unwrap();
        assert_eq!(machine.thread().unwrap().regs[5], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
        assert_eq!(machine.read_bytes(ARG_BASE, 4).unwrap(), before_denied);
    }

    #[test]
    fn random_buffer_preflights_destination_before_entropy_or_allocation() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine
            .domains
            .get_mut(&ROOT_DOMAIN_ID)
            .unwrap()
            .security
            .entropy_quota = 64;
        machine.thread_mut().unwrap().regs[2] = ARG_BASE;
        machine.thread_mut().unwrap().regs[3] = ARG_SIZE + 1;

        let err = machine
            .exec(Instr::Random(Reg(4), Reg(2), Reg(3)))
            .unwrap_err();

        assert!(err.contains("unmapped address"), "{err}");
        assert_eq!(machine.domains[&ROOT_DOMAIN_ID].security.entropy_quota, 64);
        assert_eq!(machine.thread().unwrap().regs[4], 0);
    }

    #[test]
    fn random_rejects_locked_result_register_before_entropy_or_writes() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine
            .domains
            .get_mut(&ROOT_DOMAIN_ID)
            .unwrap()
            .security
            .entropy_quota = 64;
        let random_state = machine.random_state;
        machine.write_bytes(ARG_BASE, b"sentinel").unwrap();
        machine.thread_mut().unwrap().regs[2] = ARG_BASE;
        machine.thread_mut().unwrap().regs[3] = 8;

        let err = machine
            .exec(Instr::Random(Reg(31), Reg(2), Reg(3)))
            .unwrap_err();

        assert!(err.contains("hardware-locked stack pointer"), "{err}");
        assert_eq!(machine.domains[&ROOT_DOMAIN_ID].security.entropy_quota, 64);
        assert_eq!(machine.random_state, random_state);
        assert_eq!(
            machine.read_bytes(ARG_BASE, 8).unwrap(),
            b"sentinel".to_vec()
        );
    }

    #[test]
    fn set_process_entry_failure_preserves_argument_page() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.store_u64(ARG_BASE, 0xfeed_cafe).unwrap();
        let oversized = "x".repeat(ARG_SIZE as usize);

        let err = machine.set_process_entry(&[oversized], &[]).unwrap_err();

        assert!(
            err.contains("argv data exceeds emulated argument page"),
            "{err}"
        );
        assert_eq!(machine.load_u64(ARG_BASE).unwrap(), 0xfeed_cafe);
    }

    #[test]
    fn env_get_reports_public_scalar_metadata() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        machine
            .set_args(&["prog".to_string(), "arg".to_string()])
            .unwrap();

        machine.thread_mut().unwrap().regs[2] = ENV_KEY_PAGE_SIZE;
        machine
            .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(0), Reg(0)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[1], ASLR_PAGE);

        machine.thread_mut().unwrap().regs[2] = ENV_KEY_ARGC;
        machine
            .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(0), Reg(0)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[1], 2);

        machine.thread_mut().unwrap().regs[2] = ENV_KEY_ARGV_BASE;
        machine
            .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(0), Reg(0)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[1], ARG_BASE + 8);

        machine.thread_mut().unwrap().regs[2] = ENV_KEY_ENVP_BASE;
        machine
            .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(0), Reg(0)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[1], ARG_BASE + 8 + 3 * 8);

        machine.thread_mut().unwrap().regs[2] = ENV_KEY_IMPLEMENTATION_PROFILE;
        machine
            .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(0), Reg(0)))
            .unwrap();
        assert_eq!(
            machine.thread().unwrap().regs[1],
            ENV_IMPLEMENTATION_PROFILE_REFERENCE
        );

        machine.thread_mut().unwrap().regs[2] = ENV_KEY_DMA_ALIGNMENT;
        machine
            .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(0), Reg(0)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[1], ENV_DMA_ALIGNMENT);

        machine.thread_mut().unwrap().regs[2] = ENV_KEY_TIMER_GRANULARITY_NS;
        machine
            .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(0), Reg(0)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[1], ENV_TIMER_GRANULARITY_NS);

        machine.thread_mut().unwrap().regs[2] = ENV_KEY_HWCAP0;
        machine
            .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(0), Reg(0)))
            .unwrap();
        assert!(machine.thread().unwrap().regs[1] & ENV_HWCAP0_CLASSIFIER != 0);

        machine.thread_mut().unwrap().regs[2] = ENV_KEY_OPCODE_FEATURE_BITS;
        machine
            .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(0), Reg(0)))
            .unwrap();
        assert!(machine.thread().unwrap().regs[1] & ENV_OPCODE_FEATURE_NS_CTL != 0);

        machine.thread_mut().unwrap().regs[2] = ENV_KEY_OBJECT_PROFILE_BITS;
        machine
            .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(0), Reg(0)))
            .unwrap();
        assert!(machine.thread().unwrap().regs[1] & ENV_OBJECT_PROFILE_CLASSIFIER_TABLE != 0);
        assert!(machine.thread().unwrap().regs[1] & ENV_OBJECT_PROFILE_SERVICELET_PROGRAM != 0);

        machine.thread_mut().unwrap().regs[2] = ENV_KEY_SECURITY_PROFILE_BITS;
        machine
            .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(0), Reg(0)))
            .unwrap();
        assert!(machine.thread().unwrap().regs[1] & ENV_SECURITY_PROFILE_WX_DENY != 0);
        assert!(machine.thread().unwrap().regs[1] & ENV_SECURITY_PROFILE_NO_RAW_IRQ != 0);
        assert!(machine.thread().unwrap().regs[1] & ENV_SECURITY_PROFILE_NO_RAW_MMIO != 0);
        assert!(machine.thread().unwrap().regs[1] & ENV_SECURITY_PROFILE_NO_RAW_SYSCALL != 0);

        machine.thread_mut().unwrap().regs[2] = ENV_KEY_CLASSIFIER_FEATURE_BITS;
        machine
            .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(0), Reg(0)))
            .unwrap();
        assert!(machine.thread().unwrap().regs[1] & ENV_CLASSIFIER_FEATURE_HASH != 0);

        machine.thread_mut().unwrap().regs[2] = ENV_KEY_TOPOLOGY_RECORD_COUNT;
        machine
            .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(0), Reg(0)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[1], ENV_TOPOLOGY_RECORD_COUNT);

        machine.thread_mut().unwrap().regs[2] = ENV_KEY_TOPOLOGY_RECORD_FORMAT;
        machine
            .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(0), Reg(0)))
            .unwrap();
        assert_eq!(
            machine.thread().unwrap().regs[1],
            ENV_TOPOLOGY_RECORD_FORMAT
        );

        machine.thread_mut().unwrap().regs[2] = ENV_KEY_RESOURCE_DOMAIN_LIMIT;
        machine
            .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(0), Reg(0)))
            .unwrap();
        assert_eq!(
            machine.thread().unwrap().regs[1],
            MAX_RESOURCE_DOMAINS as u64
        );

        machine.thread_mut().unwrap().regs[2] = ENV_KEY_CLASSIFIER_ENTRY_LIMIT;
        machine
            .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(0), Reg(0)))
            .unwrap();
        assert_eq!(
            machine.thread().unwrap().regs[1],
            CLASSIFIER_MAX_RULES as u64
        );

        machine.thread_mut().unwrap().regs[2] = ENV_KEY_CLASSIFIER_ALLOWED_QUEUE_LIMIT;
        machine
            .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(0), Reg(0)))
            .unwrap();
        assert_eq!(
            machine.thread().unwrap().regs[1],
            CLASSIFIER_MAX_ALLOWED_QUEUES as u64
        );

        machine.thread_mut().unwrap().regs[2] = ENV_KEY_CLASSIFIER_ROUTE_BYTE_LIMIT;
        machine
            .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(0), Reg(0)))
            .unwrap();
        assert_eq!(
            machine.thread().unwrap().regs[1],
            CLASSIFIER_MAX_ROUTE_BYTES as u64
        );

        machine.thread_mut().unwrap().regs[2] = ENV_KEY_SIGNAL_NUMBER_LIMIT;
        machine
            .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(0), Reg(0)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[1], SIGNAL_NUMBER_LIMIT);

        machine.thread_mut().unwrap().regs[2] = ENV_KEY_STARTUP_METADATA_PTR;
        machine
            .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(0), Reg(0)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[1], ARG_BASE);

        machine.thread_mut().unwrap().regs[2] = ENV_KEY_STARTUP_METADATA_LEN;
        machine
            .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(0), Reg(0)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[1], ARG_SIZE);

        let servicelet_env = [
            (ENV_KEY_SERVICELET_VERIFY_VERSION, SERVICELET_VERIFY_VERSION),
            (
                ENV_KEY_SERVICELET_PROGRAM_LIMIT,
                SERVICELET_MAX_PROGRAM_BYTES,
            ),
            (
                ENV_KEY_SERVICELET_INSTRUCTION_LIMIT,
                SERVICELET_MAX_INSTRUCTIONS,
            ),
            (ENV_KEY_SERVICELET_CYCLE_LIMIT, SERVICELET_MAX_CYCLES),
            (ENV_KEY_SERVICELET_RECORD_LIMIT, SERVICELET_MAX_RECORD_BYTES),
            (ENV_KEY_SERVICELET_ACTION_LIMIT, SERVICELET_MAX_ACTION_BYTES),
            (ENV_KEY_SERVICELET_ISA_MASK, SERVICELET_ALLOWED_ISA_MASK),
            (
                ENV_KEY_SERVICELET_FLAG_MASK,
                SERVICELET_FLAG_ALLOW_STATIC_LOOPS,
            ),
        ];
        for (key, expected) in servicelet_env {
            machine.thread_mut().unwrap().regs[2] = key;
            machine
                .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(0), Reg(0)))
                .unwrap();
            assert_eq!(machine.thread().unwrap().regs[1], expected);
        }
    }

    #[test]
    fn env_get_copies_topology_records_and_faults_bad_buffers() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        let out = ARG_BASE + 0x900;
        let topology_len = ENV_TOPOLOGY_RECORD_COUNT * ENV_TOPOLOGY_RECORD_SIZE as u64;

        machine.thread_mut().unwrap().regs[2] = ENV_KEY_TOPOLOGY_RECORD;
        machine.thread_mut().unwrap().regs[3] = out;
        machine.thread_mut().unwrap().regs[4] = topology_len;
        machine
            .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(3), Reg(4)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[1], topology_len);
        assert_eq!(machine.load_u64(out).unwrap(), 1);
        assert_eq!(
            machine.load_u64(out + 48).unwrap(),
            ENV_SCHEDULER_FEATURE_ALL
        );
        assert_eq!(
            machine
                .load_u64(out + ENV_TOPOLOGY_RECORD_SIZE as u64)
                .unwrap(),
            2
        );
        let security_record = out + ENV_TOPOLOGY_RECORD_SIZE as u64;
        assert_eq!(
            machine.load_u64(security_record + 48).unwrap(),
            ENV_SECURITY_PROFILE_ALL
        );
        let classifier_record = out + 3 * ENV_TOPOLOGY_RECORD_SIZE as u64;
        assert_eq!(machine.load_u64(classifier_record).unwrap(), 4);
        assert_eq!(
            machine.load_u64(classifier_record + 24).unwrap(),
            CLASSIFIER_MAX_RULES as u64
        );
        assert_eq!(
            machine.load_u64(classifier_record + 32).unwrap(),
            CLASSIFIER_MAX_ALLOWED_QUEUES as u64
        );
        assert_eq!(
            machine.load_u64(classifier_record + 40).unwrap(),
            CLASSIFIER_MAX_ROUTE_BYTES as u64
        );
        assert_eq!(
            machine.load_u64(classifier_record + 48).unwrap(),
            ENV_CLASSIFIER_FEATURE_ALL
        );
        let servicelet_record = out + 4 * ENV_TOPOLOGY_RECORD_SIZE as u64;
        assert_eq!(machine.load_u64(servicelet_record).unwrap(), 5);
        assert_eq!(
            machine.load_u64(servicelet_record + 8).unwrap(),
            SERVICELET_VERIFY_VERSION
        );
        assert_eq!(
            machine.load_u64(servicelet_record + 16).unwrap(),
            SERVICELET_MAX_PROGRAM_BYTES
        );
        assert_eq!(
            machine.load_u64(servicelet_record + 24).unwrap(),
            SERVICELET_MAX_INSTRUCTIONS
        );
        assert_eq!(
            machine.load_u64(servicelet_record + 32).unwrap(),
            SERVICELET_MAX_CYCLES
        );
        assert_eq!(
            machine.load_u64(servicelet_record + 40).unwrap(),
            SERVICELET_MAX_RECORD_BYTES
        );
        assert_eq!(
            machine.load_u64(servicelet_record + 48).unwrap(),
            SERVICELET_MAX_ACTION_BYTES
        );
        assert_eq!(
            machine.load_u64(servicelet_record + 56).unwrap(),
            SERVICELET_ALLOWED_ISA_MASK
        );

        machine.thread_mut().unwrap().regs[3] = 0xffff_ffff;
        machine.thread_mut().unwrap().regs[4] = ENV_TOPOLOGY_RECORD_SIZE as u64;
        machine
            .exec(Instr::EnvGet(Reg(5), Reg(2), Reg(3), Reg(4)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[5], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 14);
    }

    #[test]
    fn env_get_copies_process_entry_record_and_faults_bad_buffers() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        machine.set_args(&["prog".to_string()]).unwrap();

        machine.thread_mut().unwrap().regs[2] = ENV_KEY_PROCESS_ENTRY_RECORD;
        machine.thread_mut().unwrap().regs[3] = ARG_BASE + 0x800;
        machine.thread_mut().unwrap().regs[4] = 32;
        machine
            .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(3), Reg(4)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[1], 32);
        assert_eq!(machine.load_u64(ARG_BASE + 0x800).unwrap(), 1);
        assert_eq!(machine.load_u64(ARG_BASE + 0x808).unwrap(), ARG_BASE + 8);
        assert_eq!(machine.load_u64(ARG_BASE + 0x810).unwrap(), ARG_BASE + 24);
        assert_eq!(machine.load_u64(ARG_BASE + 0x818).unwrap(), ARG_BASE + 32);

        machine.thread_mut().unwrap().regs[3] = 0xffff_ffff;
        machine
            .exec(Instr::EnvGet(Reg(5), Reg(2), Reg(3), Reg(4)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[5], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 14);
    }

    #[test]
    fn env_get_record_copies_are_length_bounded() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        machine
            .set_args(&["prog".to_string(), "arg".to_string()])
            .unwrap();
        let out = ARG_BASE + 0x900;
        let sentinel = vec![0xa5; 64];

        machine.write_bytes(out, &sentinel).unwrap();
        machine.thread_mut().unwrap().regs[2] = ENV_KEY_PROCESS_ENTRY_RECORD;
        machine.thread_mut().unwrap().regs[3] = out;
        machine.thread_mut().unwrap().regs[4] = 0;
        machine
            .exec(Instr::EnvGet(Reg(7), Reg(2), Reg(3), Reg(4)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[7], 0);
        assert_eq!(machine.read_bytes(out, 64).unwrap(), sentinel);

        machine.set_errno(123).unwrap();
        machine.thread_mut().unwrap().regs[2] = ENV_KEY_PROCESS_ENTRY_RECORD;
        machine.thread_mut().unwrap().regs[3] = 0xffff_ffff;
        machine.thread_mut().unwrap().regs[4] = 0;
        machine
            .exec(Instr::EnvGet(Reg(9), Reg(2), Reg(3), Reg(4)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[9], 0);
        assert_eq!(machine.process().unwrap().errno, 0);

        machine.write_bytes(out, &sentinel).unwrap();
        machine.thread_mut().unwrap().regs[2] = ENV_KEY_PROCESS_ENTRY_RECORD;
        machine.thread_mut().unwrap().regs[3] = out;
        machine.thread_mut().unwrap().regs[4] = 16;
        machine
            .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(3), Reg(4)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[1], 16);
        assert_eq!(machine.load_u64(out).unwrap(), 2);
        assert_eq!(machine.load_u64(out + 8).unwrap(), ARG_BASE + 8);
        assert_eq!(machine.read_bytes(out + 16, 48).unwrap(), vec![0xa5; 48]);

        machine.write_bytes(out, &sentinel).unwrap();
        machine.thread_mut().unwrap().regs[2] = ENV_KEY_TOPOLOGY_RECORD;
        machine.thread_mut().unwrap().regs[3] = out;
        machine.thread_mut().unwrap().regs[4] = 0;
        machine
            .exec(Instr::EnvGet(Reg(8), Reg(2), Reg(3), Reg(4)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[8], 0);
        assert_eq!(machine.read_bytes(out, 64).unwrap(), sentinel);

        machine.set_errno(123).unwrap();
        machine.thread_mut().unwrap().regs[2] = ENV_KEY_TOPOLOGY_RECORD;
        machine.thread_mut().unwrap().regs[3] = 0xffff_ffff;
        machine.thread_mut().unwrap().regs[4] = 0;
        machine
            .exec(Instr::EnvGet(Reg(10), Reg(2), Reg(3), Reg(4)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[10], 0);
        assert_eq!(machine.process().unwrap().errno, 0);

        machine.write_bytes(out, &sentinel).unwrap();
        machine.thread_mut().unwrap().regs[2] = ENV_KEY_TOPOLOGY_RECORD;
        machine.thread_mut().unwrap().regs[3] = out;
        machine.thread_mut().unwrap().regs[4] = 24;
        machine
            .exec(Instr::EnvGet(Reg(5), Reg(2), Reg(3), Reg(4)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[5], 24);
        assert_eq!(machine.load_u64(out).unwrap(), 1);
        assert_eq!(machine.load_u64(out + 8).unwrap(), 0);
        assert_eq!(machine.load_u64(out + 16).unwrap(), ROOT_DOMAIN_ID);
        assert_eq!(machine.read_bytes(out + 24, 40).unwrap(), vec![0xa5; 40]);
    }

    #[test]
    fn env_get_record_rejects_locked_result_before_buffer_or_errno_side_effects() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.set_args(&["prog".to_string()]).unwrap();
        machine.set_errno(123).unwrap();
        let out = ARG_BASE + 0x900;
        let sentinel = vec![0xa5; 64];
        machine.write_bytes(out, &sentinel).unwrap();
        machine.thread_mut().unwrap().regs[2] = ENV_KEY_PROCESS_ENTRY_RECORD;
        machine.thread_mut().unwrap().regs[3] = out;
        machine.thread_mut().unwrap().regs[4] = 32;

        let err = machine
            .exec(Instr::EnvGet(Reg(31), Reg(2), Reg(3), Reg(4)))
            .unwrap_err();

        assert!(err.contains("hardware-locked stack pointer"), "{err}");
        assert_eq!(machine.process().unwrap().errno, 123);
        assert_eq!(machine.read_bytes(out, 64).unwrap(), sentinel);
    }

    #[test]
    fn env_get_rejects_bad_keys_and_never_exposes_random_material() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        machine.current_tid = 1;

        machine.thread_mut().unwrap().regs[2] = ENV_KEY_AUXV_ENTRY;
        machine.thread_mut().unwrap().regs[3] = 7;
        machine
            .exec(Instr::EnvGet(Reg(1), Reg(2), Reg(3), Reg(0)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[1], AT_RANDOM);
        assert_eq!(machine.thread().unwrap().regs[30], 0);

        machine.thread_mut().unwrap().regs[30] = 0xfeed_face;
        machine.thread_mut().unwrap().regs[3] = 7;
        let err = machine
            .exec(Instr::EnvGet(Reg(31), Reg(2), Reg(3), Reg(0)))
            .unwrap_err();
        assert!(err.contains("hardware-locked stack pointer"), "{err}");
        assert_eq!(machine.thread().unwrap().regs[30], 0xfeed_face);

        machine.thread_mut().unwrap().regs[30] = 0xfeed_face;
        machine.thread_mut().unwrap().regs[3] = 99;
        machine
            .exec(Instr::EnvGet(Reg(3), Reg(2), Reg(3), Reg(0)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[3], 0);
        assert_eq!(machine.thread().unwrap().regs[30], 0);
        assert_eq!(machine.process().unwrap().errno, 0);

        machine.thread_mut().unwrap().regs[2] = 0xfeed_beef;
        machine
            .exec(Instr::EnvGet(Reg(4), Reg(2), Reg(0), Reg(0)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);

        machine.exec(Instr::Random(Reg(5), Reg(0), Reg(0))).unwrap();
        assert_ne!(machine.thread().unwrap().regs[5], 0);
        machine.thread_mut().unwrap().regs[2] = ENV_KEY_AUXV_ENTRY;
        machine.thread_mut().unwrap().regs[3] = 7;
        machine
            .exec(Instr::EnvGet(Reg(6), Reg(2), Reg(3), Reg(0)))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[6], AT_RANDOM);
        assert_eq!(machine.thread().unwrap().regs[30], 0);
    }

    #[test]
    fn env_get_auxv_rejects_result_aliasing_secondary_return_register() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[2] = ENV_KEY_AUXV_ENTRY;
        machine.thread_mut().unwrap().regs[3] = 1;
        machine.thread_mut().unwrap().regs[30] = 0xfeed_face;

        let err = machine
            .exec(Instr::EnvGet(Reg(30), Reg(2), Reg(3), Reg(0)))
            .unwrap_err();

        assert!(err.contains("aliases secondary return register"), "{err}");
        assert_eq!(machine.thread().unwrap().regs[30], 0xfeed_face);
        assert_eq!(machine.process().unwrap().errno, 0);
    }

    #[test]
    fn no_access_mmap_faults_on_load_and_store() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[1] = 4096;
        machine.thread_mut().unwrap().regs[2] = 0;
        machine
            .exec(Instr::Mmap(
                Reg(3),
                Reg(0),
                Reg(1),
                Reg(2),
                FdReg(0),
                Reg(0),
            ))
            .unwrap();
        let addr = machine.thread().unwrap().regs[3];
        assert_ne!(addr, -1i64 as u64);
        let read_err = machine.read_bytes(addr, 1).unwrap_err();
        assert!(read_err.contains("no-access VMA"), "{read_err}");
        let write_err = machine.write_bytes(addr, &[1]).unwrap_err();
        assert!(write_err.contains("no-access VMA"), "{write_err}");
    }

    #[test]
    fn dma_ctl_copy_and_fill_use_vma_permissions() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        let arg = ARG_BASE;
        let src = ARG_BASE + 0x1000;
        let dst = ARG_BASE + 0x1100;
        machine.write_bytes(src, &[1, 2, 3, 4]).unwrap();

        machine.store_u64(arg, DMA_OP_COPY).unwrap();
        machine.store_u64(arg + 8, dst).unwrap();
        machine.store_u64(arg + 16, src).unwrap();
        machine.store_u64(arg + 24, 4).unwrap();
        machine.dma_ctl(Reg(1), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[1], 4);
        assert_eq!(machine.read_bytes(dst, 4).unwrap(), vec![1, 2, 3, 4]);

        machine.store_u64(arg, DMA_OP_FILL).unwrap();
        machine.store_u64(arg + 8, dst).unwrap();
        machine.store_u64(arg + 16, 0xab).unwrap();
        machine.store_u64(arg + 24, 3).unwrap();
        machine.dma_ctl(Reg(2), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[2], 3);
        assert_eq!(
            machine.read_bytes(dst, 4).unwrap(),
            vec![0xab, 0xab, 0xab, 4]
        );

        machine.write_bytes(dst, &[9, 9, 9, 9]).unwrap();
        machine.store_u64(arg, 99).unwrap();
        machine.store_u64(arg + 8, dst).unwrap();
        machine.store_u64(arg + 16, 0xee).unwrap();
        machine.store_u64(arg + 24, 4).unwrap();
        machine.store_u64(arg + 32, 0).unwrap();
        machine.dma_ctl(Reg(3), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[3], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        assert_eq!(machine.read_bytes(dst, 4).unwrap(), vec![9, 9, 9, 9]);
    }

    #[test]
    fn dma_copy_prevalidates_destination_before_source_page_in() {
        let path = format!("/tmp/lnp64_dma_pagein_{}.bin", std::process::id());
        fs::write(&path, b"abcd").unwrap();
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let src = 0x260_000;
        {
            let process = machine.process_mut().unwrap();
            process.vmas.push(Vma {
                start: src,
                len: 4,
                prot: 0b001,
                file: Some(File::open(&path).unwrap()),
                file_offset: 0,
                resident: false,
                guard: false,
            });
        }
        let arg = ARG_BASE;
        machine.store_u64(arg, DMA_OP_COPY).unwrap();
        machine.store_u64(arg + 8, MEMORY_SIZE as u64).unwrap();
        machine.store_u64(arg + 16, src).unwrap();
        machine.store_u64(arg + 24, 4).unwrap();
        machine.store_u64(arg + 32, 0).unwrap();

        machine.dma_ctl(Reg(3), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[3], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 14);
        assert!(
            !machine
                .process()
                .unwrap()
                .vmas
                .iter()
                .find(|vma| vma.start == src)
                .unwrap()
                .resident
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn dma_ctl_rejects_locked_result_before_errno_or_memory_side_effects() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.set_errno(123).unwrap();
        let arg = ARG_BASE;
        let dst = ARG_BASE + 0x1000;
        machine.write_bytes(dst, &[0x55, 0x55]).unwrap();

        machine.store_u64(arg, DMA_OP_FILL).unwrap();
        machine.store_u64(arg + 8, dst).unwrap();
        machine.store_u64(arg + 16, 0xaa).unwrap();
        machine.store_u64(arg + 24, 2).unwrap();
        machine.store_u64(arg + 32, 0).unwrap();

        let err = machine.dma_ctl(Reg(31), arg).unwrap_err();

        assert!(err.contains("stack pointer"), "{err}");
        assert_eq!(machine.process().unwrap().errno, 123);
        assert_eq!(machine.read_bytes(dst, 2).unwrap(), vec![0x55, 0x55]);
    }

    #[test]
    fn dma_ctl_rejects_guard_unmapped_and_disallowed_domain() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        let arg = ARG_BASE;

        machine.thread_mut().unwrap().regs[1] = 8;
        machine.thread_mut().unwrap().regs[2] = 64;
        machine
            .exec(Instr::AllocEx(Reg(3), Reg(1), Reg(2)))
            .unwrap();
        let guarded = machine.thread().unwrap().regs[3] + 8;

        machine.store_u64(arg, DMA_OP_FILL).unwrap();
        machine.store_u64(arg + 8, guarded).unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 1).unwrap();
        machine.dma_ctl(Reg(4), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 14);

        machine.store_u64(arg + 8, 0x7f_0000).unwrap();
        machine.dma_ctl(Reg(5), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[5], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 14);

        machine
            .domains
            .get_mut(&ROOT_DOMAIN_ID)
            .unwrap()
            .security
            .dma_allowed = false;
        let denied_target = ARG_BASE + 0x1000;
        machine.write_bytes(denied_target, &[7, 7]).unwrap();
        machine.store_u64(arg + 8, denied_target).unwrap();
        machine.store_u64(arg + 16, 0xff).unwrap();
        machine.store_u64(arg + 24, 2).unwrap();
        machine.store_u64(arg + 32, 0).unwrap();
        machine.dma_ctl(Reg(6), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[6], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
        assert_eq!(machine.read_bytes(denied_target, 2).unwrap(), vec![7, 7]);
    }

    #[test]
    fn dma_ctl_fill_rejects_huge_unmapped_length_before_allocating() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        let arg = ARG_BASE;
        let dst = ARG_BASE + 0x1000;
        machine.write_bytes(dst, &[0x55]).unwrap();

        machine.store_u64(arg, DMA_OP_FILL).unwrap();
        machine.store_u64(arg + 8, dst).unwrap();
        machine.store_u64(arg + 16, 0xaa).unwrap();
        machine.store_u64(arg + 24, u64::MAX).unwrap();
        machine.store_u64(arg + 32, 0).unwrap();
        machine.dma_ctl(Reg(1), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 14);
        assert_eq!(machine.read_bytes(dst, 1).unwrap(), vec![0x55]);
    }

    #[test]
    fn dma_ctl_uses_dma_buffer_capability_scope() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[1] = 16;
        machine.exec(Instr::Alloc(Reg(2), Reg(1))).unwrap();
        let buffer = machine.thread().unwrap().regs[2];
        let arg = ARG_BASE;

        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::DmaBuffer.code())
            .unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.store_u64(arg + 32, 0).unwrap();
        machine.store_u64(arg + 40, buffer).unwrap();
        machine.store_u64(arg + 48, 16).unwrap();
        machine.object_ctl(Reg(3), arg).unwrap();
        let fd = machine.thread().unwrap().regs[3] as usize;
        let token = machine.fd_token(fd).unwrap();

        machine.store_u64(arg, DMA_OP_FILL).unwrap();
        machine.store_u64(arg + 8, buffer + 4).unwrap();
        machine.store_u64(arg + 16, 0xcd).unwrap();
        machine.store_u64(arg + 24, 4).unwrap();
        machine.store_u64(arg + 32, token).unwrap();
        machine.dma_ctl(Reg(4), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[4], 4);
        assert_eq!(
            machine.read_bytes(buffer + 4, 4).unwrap(),
            vec![0xcd, 0xcd, 0xcd, 0xcd]
        );

        machine.store_u64(arg + 8, buffer + 15).unwrap();
        machine.store_u64(arg + 24, 2).unwrap();
        machine.dma_ctl(Reg(5), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[5], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 14);
    }

    #[test]
    fn dma_buffer_object_creation_requires_writable_mapped_range() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[1] = 4096;
        machine.thread_mut().unwrap().regs[2] = 0b001;
        machine
            .exec(Instr::Mmap(
                Reg(3),
                Reg(0),
                Reg(1),
                Reg(2),
                FdReg(0),
                Reg(0),
            ))
            .unwrap();
        let readonly = machine.thread().unwrap().regs[3];
        let arg = ARG_BASE;

        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::DmaBuffer.code())
            .unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 7).unwrap();
        machine.store_u64(arg + 40, readonly).unwrap();
        machine.store_u64(arg + 48, 0).unwrap();
        machine.object_ctl(Reg(4), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        assert!(matches!(
            machine.process().unwrap().fds[7],
            FdHandle::Closed
        ));

        machine.store_u64(arg + 48, 16).unwrap();
        machine.object_ctl(Reg(4), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 14);
        assert!(matches!(
            machine.process().unwrap().fds[7],
            FdHandle::Closed
        ));

        let retained = Rc::new(RefCell::new(88));
        {
            let process = machine.process_mut().unwrap();
            process.fds[7] = FdHandle::Counter(retained.clone());
            process.fd_capabilities[7] = FdCapability::full(7);
        }
        machine.object_ctl(Reg(5), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[5], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 14);
        match &machine.process().unwrap().fds[7] {
            FdHandle::Counter(value) => {
                assert!(Rc::ptr_eq(value, &retained));
                assert_eq!(*value.borrow(), 88);
            }
            _ => panic!("expected retained counter fd"),
        }
    }

    #[test]
    fn dma_ctl_rejects_stale_and_revoked_dma_buffers() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[1] = 16;
        machine.exec(Instr::Alloc(Reg(2), Reg(1))).unwrap();
        let buffer = machine.thread().unwrap().regs[2];
        let arg = ARG_BASE;

        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::DmaBuffer.code())
            .unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.store_u64(arg + 32, 0).unwrap();
        machine.store_u64(arg + 40, buffer).unwrap();
        machine.store_u64(arg + 48, 16).unwrap();
        machine.object_ctl(Reg(3), arg).unwrap();
        let fd = machine.thread().unwrap().regs[3] as usize;
        let stale_token = machine.fd_token(fd).unwrap();

        machine.thread_mut().unwrap().regs[4] = stale_token;
        machine.exec(Instr::FdCloseDyn(Reg(4))).unwrap();
        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::DmaBuffer.code())
            .unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, fd as u64).unwrap();
        machine.store_u64(arg + 32, 0).unwrap();
        machine.store_u64(arg + 40, buffer).unwrap();
        machine.store_u64(arg + 48, 16).unwrap();
        machine.object_ctl(Reg(5), arg).unwrap();

        machine.store_u64(arg, DMA_OP_FILL).unwrap();
        machine.store_u64(arg + 8, buffer).unwrap();
        machine.store_u64(arg + 16, 0xee).unwrap();
        machine.store_u64(arg + 24, 1).unwrap();
        machine.store_u64(arg + 32, stale_token).unwrap();
        machine.dma_ctl(Reg(6), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[6], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 116);

        let live_token = machine.fd_token(fd).unwrap();
        machine.store_u64(arg, live_token).unwrap();
        machine.cap_revoke(Reg(7), arg).unwrap();
        machine.store_u64(arg, DMA_OP_FILL).unwrap();
        machine.store_u64(arg + 8, buffer).unwrap();
        machine.store_u64(arg + 16, 0xee).unwrap();
        machine.store_u64(arg + 24, 1).unwrap();
        machine.store_u64(arg + 32, live_token).unwrap();
        machine.dma_ctl(Reg(8), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[8], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 116);
    }

    #[test]
    fn dma_ctl_v1_completes_before_revoke_and_blocks_later_submits() {
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[1] = 16;
        machine.exec(Instr::Alloc(Reg(2), Reg(1))).unwrap();
        let buffer = machine.thread().unwrap().regs[2];
        let arg = ARG_BASE;

        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::DmaBuffer.code())
            .unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.store_u64(arg + 32, 0).unwrap();
        machine.store_u64(arg + 40, buffer).unwrap();
        machine.store_u64(arg + 48, 16).unwrap();
        machine.object_ctl(Reg(3), arg).unwrap();
        let fd = machine.thread().unwrap().regs[3] as usize;
        let owner_token = machine.fd_token(fd).unwrap();

        machine.store_u64(arg, owner_token).unwrap();
        machine.store_u64(arg + 8, 0).unwrap();
        machine.store_u64(arg + 16, CAP_RIGHT_WRITE).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_dup(Reg(4), arg).unwrap();
        let writer_token = machine.thread().unwrap().regs[4];
        assert_ne!(writer_token, -1i64 as u64);

        machine.store_u64(arg, DMA_OP_FILL).unwrap();
        machine.store_u64(arg + 8, buffer + 4).unwrap();
        machine.store_u64(arg + 16, 0xaa).unwrap();
        machine.store_u64(arg + 24, 4).unwrap();
        machine.store_u64(arg + 32, writer_token).unwrap();
        machine.dma_ctl(Reg(5), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[5], 4);
        assert_eq!(
            machine.read_bytes(buffer, 12).unwrap(),
            vec![0, 0, 0, 0, 0xaa, 0xaa, 0xaa, 0xaa, 0, 0, 0, 0]
        );

        machine.store_u64(arg, owner_token).unwrap();
        machine.cap_revoke(Reg(6), arg).unwrap();
        assert!(machine.thread().unwrap().regs[6] >= 2);

        machine.store_u64(arg, DMA_OP_FILL).unwrap();
        machine.store_u64(arg + 8, buffer + 8).unwrap();
        machine.store_u64(arg + 16, 0xbb).unwrap();
        machine.store_u64(arg + 24, 4).unwrap();
        machine.store_u64(arg + 32, writer_token).unwrap();
        machine.dma_ctl(Reg(7), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[7], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 116);
        assert_eq!(
            machine.read_bytes(buffer, 12).unwrap(),
            vec![0, 0, 0, 0, 0xaa, 0xaa, 0xaa, 0xaa, 0, 0, 0, 0]
        );
    }

    #[test]
    fn alloc_ex_creates_and_frees_guard_regions() {
        let program = empty_program();
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[1] = 32;
        machine.thread_mut().unwrap().regs[2] = 64;
        machine
            .exec(Instr::AllocEx(Reg(3), Reg(1), Reg(2)))
            .unwrap();
        let ptr = machine.thread().unwrap().regs[3];
        assert_eq!(ptr % 64, 0);
        assert_eq!(machine.process().unwrap().allocations[&ptr].len, 32);

        machine.write_bytes(ptr, &[7]).unwrap();
        assert_eq!(machine.read_bytes(ptr, 1).unwrap(), vec![7]);

        let guard_before = ptr - 4096;
        let guard_after = ptr + 32;
        let before_err = machine.read_bytes(ptr - 1, 1).unwrap_err();
        assert!(before_err.contains("guard page"), "{before_err}");
        let after_err = machine.write_bytes(guard_after, &[1]).unwrap_err();
        assert!(after_err.contains("guard page"), "{after_err}");
        assert!(
            machine
                .process()
                .unwrap()
                .vmas
                .iter()
                .any(|vma| vma.start == guard_before && vma.guard)
        );
        assert!(
            machine
                .process()
                .unwrap()
                .vmas
                .iter()
                .any(|vma| vma.start == guard_after && vma.guard)
        );

        machine.thread_mut().unwrap().regs[4] = ptr;
        machine.exec(Instr::Free(Reg(4))).unwrap();
        assert!(!machine.process().unwrap().allocations.contains_key(&ptr));
        assert!(
            !machine
                .process()
                .unwrap()
                .vmas
                .iter()
                .any(|vma| vma.start == guard_before
                    || vma.start == ptr
                    || vma.start == guard_after)
        );
        let stale_read = machine.read_bytes(ptr, 1).unwrap_err();
        assert!(stale_read.contains("unmapped address"), "{stale_read}");
        let stale_write = machine.write_bytes(ptr, &[1]).unwrap_err();
        assert!(stale_write.contains("unmapped address"), "{stale_write}");
    }

    #[test]
    fn randomized_mmap_mprotect_and_guard_stress_preserves_permissions() {
        let mut rng = TestRng::new(0x5150_f00d_dead_beef);
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let mut live_allocs = Vec::new();

        for i in 0..48 {
            let len = 4096;
            let prot = match rng.below(4) {
                0 => 0,
                1 => 0b001,
                2 => 0b010,
                _ => 0b011,
            };
            machine.thread_mut().unwrap().regs[1] = len;
            machine.thread_mut().unwrap().regs[2] = prot;
            machine
                .exec(Instr::Mmap(
                    Reg(3),
                    Reg(0),
                    Reg(1),
                    Reg(2),
                    FdReg(0),
                    Reg(0),
                ))
                .unwrap();
            let addr = machine.thread().unwrap().regs[3];
            assert_ne!(addr, -1i64 as u64);

            if prot & 0b001 != 0 {
                assert_eq!(machine.read_bytes(addr, 1).unwrap(), vec![0]);
            } else {
                let err = machine.read_bytes(addr, 1).unwrap_err();
                assert!(
                    err.contains("no-access VMA") || err.contains("read denied"),
                    "{err}"
                );
            }
            if prot & 0b010 != 0 {
                machine.write_bytes(addr, &[i as u8]).unwrap();
            } else {
                let err = machine.write_bytes(addr, &[i as u8]).unwrap_err();
                assert!(
                    err.contains("no-access VMA") || err.contains("write denied"),
                    "{err}"
                );
            }

            let new_prot = match rng.below(4) {
                0 => 0,
                1 => 0b001,
                2 => 0b010,
                _ => 0b011,
            };
            machine.thread_mut().unwrap().regs[4] = addr;
            machine.thread_mut().unwrap().regs[5] = len;
            machine.thread_mut().unwrap().regs[6] = new_prot;
            machine
                .exec(Instr::Mprotect(Reg(4), Reg(5), Reg(6)))
                .unwrap();
            assert_eq!(machine.process().unwrap().errno, 0);

            if new_prot & 0b010 != 0 {
                machine.write_bytes(addr, &[0xaa]).unwrap();
            } else {
                let err = machine.write_bytes(addr, &[0xaa]).unwrap_err();
                assert!(
                    err.contains("no-access VMA") || err.contains("write denied"),
                    "{err}"
                );
            }

            if rng.below(3) == 0 {
                let alloc_len = 16 + rng.below(96) as usize;
                machine.thread_mut().unwrap().regs[7] = alloc_len as u64;
                machine.thread_mut().unwrap().regs[8] = 64;
                machine
                    .exec(Instr::AllocEx(Reg(9), Reg(7), Reg(8)))
                    .unwrap();
                let ptr = machine.thread().unwrap().regs[9];
                assert_eq!(ptr % 64, 0);
                assert!(
                    machine
                        .read_bytes(ptr - 1, 1)
                        .unwrap_err()
                        .contains("guard page")
                );
                assert!(
                    machine
                        .write_bytes(ptr + alloc_len as u64, &[1])
                        .unwrap_err()
                        .contains("guard page")
                );
                live_allocs.push(ptr);
            }

            machine.thread_mut().unwrap().regs[10] = addr;
            machine.thread_mut().unwrap().regs[11] = len;
            machine.exec(Instr::Munmap(Reg(10), Reg(11))).unwrap();
            assert!(
                machine
                    .read_bytes(addr, 1)
                    .unwrap_err()
                    .contains("unmapped address")
            );

            if !live_allocs.is_empty() && rng.below(2) == 0 {
                let idx = rng.below(live_allocs.len() as u64) as usize;
                let ptr = live_allocs.swap_remove(idx);
                machine.thread_mut().unwrap().regs[12] = ptr;
                machine.exec(Instr::Free(Reg(12))).unwrap();
                assert!(
                    machine
                        .read_bytes(ptr, 1)
                        .unwrap_err()
                        .contains("unmapped address")
                );
            }
        }
    }

    #[test]
    fn randomized_capability_delegation_stress_preserves_authority() {
        let mut rng = TestRng::new(0xc0ff_ee00_cafe_f00d);
        let program = Program::parse(
            r#"
            .data
            path: .string "Cargo.toml"

            .text
              NOP
            "#,
        )
        .unwrap();
        let path = program.data_labels["path"];
        let mut machine = Machine::new(program);
        machine.current_tid = 1;
        let pipe = Rc::new(RefCell::new(PipeBuffer::default()));
        machine.processes.get_mut(&1).unwrap().fds[10] = FdHandle::PipeReader(Rc::clone(&pipe));
        machine.processes.get_mut(&1).unwrap().fd_capabilities[10] = FdCapability::full(10);
        machine.processes.get_mut(&1).unwrap().fds[11] = FdHandle::PipeWriter(pipe);
        machine.processes.get_mut(&1).unwrap().fd_capabilities[11] = FdCapability::full(11);

        for _ in 0..32 {
            machine.thread_mut().unwrap().regs[1] = path;
            machine.thread_mut().unwrap().regs[2] = 0;
            machine
                .exec(Instr::OpenFdDyn(Reg(3), Reg(1), Reg(2)))
                .unwrap();
            let source = machine.thread().unwrap().regs[3];
            assert_ne!(source, -1i64 as u64);

            let mut rights = CAP_RIGHT_READ;
            if rng.below(2) == 0 {
                rights |= CAP_RIGHT_DUP;
            }
            if rng.below(2) == 0 {
                rights |= CAP_RIGHT_REVOKE;
            }
            if rng.below(2) == 0 {
                rights |= CAP_RIGHT_TRANSFER;
            }
            let seal = rng.below(2) == 0;
            let arg = ARG_BASE;
            machine.store_u64(arg, source).unwrap();
            machine.store_u64(arg + 8, 0).unwrap();
            machine.store_u64(arg + 16, rights).unwrap();
            machine
                .store_u64(arg + 24, if seal { CAP_DUP_FLAG_SEAL } else { 0 })
                .unwrap();
            machine.cap_dup(Reg(4), arg).unwrap();
            let child = machine.thread().unwrap().regs[4];
            assert_ne!(child, -1i64 as u64);

            machine.thread_mut().unwrap().regs[5] = child;
            machine.thread_mut().unwrap().regs[6] = ARG_BASE + 0x4000;
            machine.thread_mut().unwrap().regs[7] = 1;
            machine
                .exec(Instr::ReadFdDyn(Reg(5), Reg(6), Reg(7)))
                .unwrap();
            assert_eq!(machine.process().unwrap().errno, 0);
            assert_eq!(machine.thread().unwrap().regs[1], 1);

            machine.store_u64(arg, child).unwrap();
            machine.store_u64(arg + 8, 0).unwrap();
            machine.store_u64(arg + 16, CAP_RIGHT_READ).unwrap();
            machine.store_u64(arg + 24, 0).unwrap();
            machine.cap_dup(Reg(8), arg).unwrap();
            if seal || rights & CAP_RIGHT_DUP == 0 {
                assert_eq!(machine.thread().unwrap().regs[8], -1i64 as u64);
                assert_eq!(machine.process().unwrap().errno, 1);
            } else {
                assert_ne!(machine.thread().unwrap().regs[8], -1i64 as u64);
            }

            machine.store_u64(arg, 11).unwrap();
            machine.store_u64(arg + 8, child).unwrap();
            machine.store_u64(arg + 16, 0).unwrap();
            machine.store_u64(arg + 24, 0).unwrap();
            machine.cap_send(Reg(9), arg).unwrap();
            if rights & CAP_RIGHT_TRANSFER == 0 {
                assert_eq!(machine.thread().unwrap().regs[9], -1i64 as u64);
                assert_eq!(machine.process().unwrap().errno, 1);
            } else {
                assert_eq!(machine.thread().unwrap().regs[9], 1);
                machine.store_u64(arg, 10).unwrap();
                machine.store_u64(arg + 8, 0).unwrap();
                machine.store_u64(arg + 16, CAP_RIGHT_READ).unwrap();
                machine.store_u64(arg + 24, 0).unwrap();
                machine.cap_recv(Reg(12), arg).unwrap();
                let received = machine.thread().unwrap().regs[12];
                assert_ne!(received, -1i64 as u64);
            }

            machine.store_u64(arg, source).unwrap();
            machine.cap_revoke(Reg(13), arg).unwrap();
            assert!(machine.thread().unwrap().regs[13] >= 1);
            machine.thread_mut().unwrap().regs[14] = child;
            machine.thread_mut().unwrap().regs[15] = ARG_BASE + 0x5000;
            machine.thread_mut().unwrap().regs[16] = 1;
            machine
                .exec(Instr::ReadFdDyn(Reg(14), Reg(15), Reg(16)))
                .unwrap();
            assert_eq!(machine.process().unwrap().errno, 116);
            assert_eq!(machine.thread().unwrap().regs[1], -1i64 as u64);
        }
    }

    #[test]
    fn randomized_domain_lifecycle_stress_rejects_stale_handles() {
        let mut rng = TestRng::new(0xd0aa_1ade_c001_beef);
        let mut machine = Machine::new(empty_program());
        machine.current_tid = 1;
        let arg = ARG_BASE;
        let mut live = Vec::new();
        let mut destroyed = Vec::new();

        for _ in 0..96 {
            for offset in (0..=160).step_by(8) {
                machine.store_u64(arg + offset, 0).unwrap();
            }

            let op = if live.is_empty() { 0 } else { rng.below(4) };
            match op {
                0 if live.len() < 24 => {
                    machine.store_u64(arg, DOMAIN_OP_CREATE).unwrap();
                    machine.store_u64(arg + 8, ROOT_DOMAIN_ID).unwrap();
                    machine.store_u64(arg + 16, 1).unwrap();
                    let id = machine.domain_ctl_create(arg).unwrap();
                    let generation = machine.domains[&id].generation;
                    assert_eq!(generation, 1);
                    assert!(machine.domain_ref(id, generation).is_ok());
                    live.push((id, generation));
                }
                1 if !live.is_empty() => {
                    let idx = rng.below(live.len() as u64) as usize;
                    let (id, generation) = live[idx];
                    machine.store_u64(arg + 8, id).unwrap();
                    machine.store_u64(arg + 16, generation).unwrap();
                    machine.domain_ctl_set_frozen(arg, true).unwrap();
                    assert!(machine.domains[&id].frozen);
                    machine.domain_ctl_set_frozen(arg, false).unwrap();
                    assert!(!machine.domains[&id].frozen);
                    assert!(machine.domain_ref(id, generation).is_ok());
                }
                2 if !live.is_empty() => {
                    let idx = rng.below(live.len() as u64) as usize;
                    let (id, generation) = live.swap_remove(idx);
                    machine.store_u64(arg + 8, id).unwrap();
                    machine.store_u64(arg + 16, generation).unwrap();
                    machine.domain_ctl_destroy(arg).unwrap();
                    assert!(machine.domains[&id].destroyed);
                    assert_eq!(machine.domain_ref(id, generation), Err(116));
                    destroyed.push((id, generation));
                }
                _ => {
                    if let Some(&(id, generation)) =
                        destroyed.get(rng.below(destroyed.len().max(1) as u64) as usize)
                    {
                        machine.store_u64(arg + 8, id).unwrap();
                        machine.store_u64(arg + 16, generation).unwrap();
                        assert_eq!(machine.domain_ctl_query(arg), Err(116));
                    }
                }
            }
        }

        if destroyed.is_empty() {
            let (id, generation) = live.pop().expect("domain stress created no domains");
            machine.store_u64(arg + 8, id).unwrap();
            machine.store_u64(arg + 16, generation).unwrap();
            machine.domain_ctl_destroy(arg).unwrap();
            destroyed.push((id, generation));
        }

        for (id, generation) in destroyed {
            assert_eq!(machine.domain_ref(id, generation), Err(116));
        }
    }

    #[test]
    fn domain_usage_rolls_up_and_release_is_live() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        let parent = ResourceDomain {
            id: 2,
            generation: 1,
            parent: Some(ROOT_DOMAIN_ID),
            children: vec![3],
            profile: 4,
            limits: DomainLimits {
                cpu: u64::MAX,
                memory: u64::MAX,
                pids: u64::MAX,
                fdrs: u64::MAX,
            },
            capability_mask: u64::MAX,
            upcall_mask: u64::MAX,
            security: DomainSecurityPolicy::root(),
            frozen: false,
            destroyed: false,
            cpu_ticks: 0,
        };
        let child = ResourceDomain {
            id: 3,
            generation: 1,
            parent: Some(2),
            children: Vec::new(),
            profile: 4,
            limits: parent.limits,
            capability_mask: u64::MAX,
            upcall_mask: u64::MAX,
            security: DomainSecurityPolicy::root(),
            frozen: false,
            destroyed: false,
            cpu_ticks: 0,
        };
        machine
            .domains
            .get_mut(&ROOT_DOMAIN_ID)
            .unwrap()
            .children
            .push(2);
        machine.domains.insert(2, parent);
        machine.domains.insert(3, child);
        machine.processes.get_mut(&1).unwrap().domain_id = 3;

        let child_before = machine.domain_usage(3);
        let parent_before = machine.domain_usage(2);
        assert_eq!(child_before.memory, parent_before.memory);
        assert_eq!(child_before.pids, parent_before.pids);
        assert_eq!(child_before.fdrs, parent_before.fdrs);

        machine.current_tid = 1;
        machine.thread_mut().unwrap().regs[1] = 64;
        machine.exec(Instr::Alloc(Reg(2), Reg(1))).unwrap();
        let ptr = machine.thread().unwrap().regs[2];
        let child_after_alloc = machine.domain_usage(3);
        let parent_after_alloc = machine.domain_usage(2);
        assert_eq!(child_after_alloc.memory, parent_after_alloc.memory);
        assert!(child_after_alloc.memory > child_before.memory);

        machine.thread_mut().unwrap().regs[3] = ptr;
        machine.exec(Instr::Free(Reg(3))).unwrap();
        let child_after_free = machine.domain_usage(3);
        let parent_after_free = machine.domain_usage(2);
        assert_eq!(child_after_free.memory, child_before.memory);
        assert_eq!(parent_after_free.memory, parent_before.memory);
    }

    #[test]
    fn failed_budgeted_operations_do_not_leak_accounting() {
        let mut machine = test_machine_with_child_domain();
        machine.processes.get_mut(&1).unwrap().domain_id = 2;
        machine.current_tid = 1;
        let baseline = machine.domain_usage(2);

        machine.domains.get_mut(&2).unwrap().limits.pids = baseline.pids;
        machine.exec(Instr::Fork(Reg(5))).unwrap();
        assert_eq!(machine.thread().unwrap().regs[5], -1i64 as u64);
        assert_eq!(machine.domain_usage(2), baseline);
        assert_eq!(machine.threads.len(), 1);

        machine.domains.get_mut(&2).unwrap().limits.memory = baseline.memory;
        machine.thread_mut().unwrap().regs[6] = 4096;
        machine.thread_mut().unwrap().regs[7] = 3;
        machine
            .exec(Instr::Mmap(
                Reg(8),
                Reg(0),
                Reg(6),
                Reg(7),
                FdReg(0),
                Reg(0),
            ))
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[8], -1i64 as u64);
        assert_eq!(machine.domain_usage(2), baseline);

        machine.domains.get_mut(&2).unwrap().limits.fdrs = baseline.fdrs;
        let arg = ARG_BASE;
        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::Counter.code())
            .unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 4).unwrap();
        machine.store_u64(arg + 40, 0).unwrap();
        machine.object_ctl(Reg(9), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[9], -1i64 as u64);
        assert!(matches!(
            machine.process().unwrap().fds[4],
            FdHandle::Closed
        ));
        assert_eq!(machine.domain_usage(2), baseline);

        machine.domains.get_mut(&2).unwrap().frozen = true;
        let stack_before = machine.thread().unwrap().cap_call_stack.len();
        let domain_before = machine.process().unwrap().domain_id;
        let err = machine.call_cap(Reg(10), 3, 1, 2).unwrap_err();
        assert!(err.contains("resource domain inactive"), "{err}");
        assert_eq!(machine.thread().unwrap().cap_call_stack.len(), stack_before);
        assert_eq!(machine.process().unwrap().domain_id, domain_before);
        assert_eq!(machine.domain_usage(2), baseline);
        machine.domains.get_mut(&2).unwrap().frozen = false;

        machine.domains.insert(
            3,
            ResourceDomain {
                id: 3,
                generation: 1,
                parent: Some(2),
                children: Vec::new(),
                profile: 4,
                limits: DomainLimits {
                    cpu: u64::MAX,
                    memory: 1,
                    pids: 1,
                    fdrs: 1,
                },
                capability_mask: u64::MAX,
                upcall_mask: u64::MAX,
                security: DomainSecurityPolicy::root(),
                frozen: false,
                destroyed: false,
                cpu_ticks: 0,
            },
        );
        machine.domains.get_mut(&2).unwrap().children.push(3);
        machine.store_u64(arg + 8, 3).unwrap();
        machine.store_u64(arg + 16, 1).unwrap();
        assert_eq!(machine.domain_ctl_attach_self(arg), Err(12));
        assert_eq!(machine.process().unwrap().domain_id, 2);
        assert_eq!(machine.domain_usage(2), baseline);
    }

    #[test]
    fn call_cap_sync_returns_across_domain_gate() {
        let program = Program::parse(
            r#"
            .data
            dom: .zero 208
            obj: .zero 80

            .text
              LI r10, dom
              LI r11, -1
              LI r1, 1
              ST [r10, 0], r1
              ST [r10, 8], r0
              ST [r10, 16], r0
              LI r1, 4
              ST [r10, 24], r1
              LI r1, 1000
              ST [r10, 32], r1
              LI r1, 5000000
              ST [r10, 40], r1
              LI r1, 2
              ST [r10, 48], r1
              LI r1, 8
              ST [r10, 56], r1
              LI r1, 63
              ST [r10, 64], r1
              ST [r10, 72], r1
              DOMAIN_CTL r20, r10
              CMP r20, r11
              BEQ bad

              LI r12, obj
              LI r1, 1
              ST [r12, 0], r1
              LI r1, 2
              ST [r12, 8], r1
              LI r1, 4
              ST [r12, 16], r1
              LI r1, 3
              ST [r12, 24], r1
              ST [r12, 32], r20
              LI r1, service
              ST [r12, 40], r1
              OBJECT_CTL r21, r12
              CMP r21, r11
              BEQ bad

              LI r1, 7
              LI r2, 9
              CALL_CAP r22, fd3, r1, r2
              LI r23, 16
              CMP r22, r23
              BNE bad
              LI r23, 9
              CMP r30, r23
              BNE bad
              EXIT r0

            service:
              ADD r3, r1, r2
              RET_CAP r0, r3, r2

            bad:
              LI r1, 1
              EXIT r1
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        assert_eq!(machine.run().unwrap(), 0);
    }

    fn test_machine_with_child_domain() -> Machine {
        let program = Program::parse(
            r#"
            .text
              NOP
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        machine.domains.insert(
            2,
            ResourceDomain {
                id: 2,
                generation: 1,
                parent: Some(ROOT_DOMAIN_ID),
                children: Vec::new(),
                profile: 4,
                limits: DomainLimits::root(),
                capability_mask: u64::MAX,
                upcall_mask: u64::MAX,
                security: DomainSecurityPolicy::root(),
                frozen: false,
                destroyed: false,
                cpu_ticks: 0,
            },
        );
        machine
            .domains
            .get_mut(&ROOT_DOMAIN_ID)
            .unwrap()
            .children
            .push(2);
        machine.next_domain_id = 3;
        machine.processes.get_mut(&1).unwrap().fds[3] = FdHandle::CallGate {
            entry: 1,
            domain_id: 2,
            domain_generation: 1,
            mode: CALL_MODE_SYNC,
            completion_fd: None,
            completion_generation: None,
            flags: 0,
        };
        machine.processes.get_mut(&1).unwrap().fd_capabilities[3] = FdCapability::full(3);
        machine
    }

    #[test]
    fn object_ctl_call_gate_rejects_unknown_flags_without_installing_fd() {
        let mut machine = test_machine_with_child_domain();
        machine.current_tid = 1;
        let arg = ARG_BASE;

        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::Queue.code())
            .unwrap();
        machine
            .store_u64(arg + 16, ObjectProfile::CallGate.code())
            .unwrap();
        machine.store_u64(arg + 24, 4).unwrap();
        machine.store_u64(arg + 32, 2).unwrap();
        machine.store_u64(arg + 40, 1).unwrap();
        machine.store_u64(arg + 48, CALL_MODE_SYNC).unwrap();
        machine.store_u64(arg + 56, 0).unwrap();
        machine.store_u64(arg + 64, 1 << 4).unwrap();

        machine.object_ctl(Reg(5), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[5], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        assert!(matches!(
            machine.process().unwrap().fds[4],
            FdHandle::Closed
        ));
    }

    #[test]
    fn object_ctl_call_gate_requires_completion_write_authority() {
        let mut machine = test_machine_with_child_domain();
        machine.current_tid = 1;
        machine.processes.get_mut(&1).unwrap().fds[4] = FdHandle::Counter(Rc::new(RefCell::new(0)));
        machine.processes.get_mut(&1).unwrap().fd_capabilities[4] = FdCapability::full(4);
        machine.processes.get_mut(&1).unwrap().fd_capabilities[4].rights &= !CAP_RIGHT_WRITE;
        let arg = ARG_BASE;

        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::Queue.code())
            .unwrap();
        machine
            .store_u64(arg + 16, ObjectProfile::CallGate.code())
            .unwrap();
        machine.store_u64(arg + 24, 5).unwrap();
        machine.store_u64(arg + 32, 2).unwrap();
        machine.store_u64(arg + 40, 1).unwrap();
        machine.store_u64(arg + 48, CALL_MODE_ASYNC).unwrap();
        machine.store_u64(arg + 56, 4).unwrap();
        machine.store_u64(arg + 64, 0).unwrap();

        machine.object_ctl(Reg(6), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[6], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
        assert!(matches!(
            machine.process().unwrap().fds[5],
            FdHandle::Closed
        ));
    }

    #[test]
    fn object_ctl_call_gate_rejects_non_waitable_completion_target() {
        let mut machine = test_machine_with_child_domain();
        machine.current_tid = 1;
        let arg = ARG_BASE;

        machine.store_u64(arg, OBJECT_OP_CREATE).unwrap();
        machine
            .store_u64(arg + 8, ObjectKind::Queue.code())
            .unwrap();
        machine
            .store_u64(arg + 16, ObjectProfile::CallGate.code())
            .unwrap();
        machine.store_u64(arg + 24, 5).unwrap();
        machine.store_u64(arg + 32, 2).unwrap();
        machine.store_u64(arg + 40, 1).unwrap();
        machine.store_u64(arg + 48, CALL_MODE_ASYNC).unwrap();
        machine.store_u64(arg + 56, 1).unwrap();
        machine.store_u64(arg + 64, 0).unwrap();

        machine.object_ctl(Reg(6), arg).unwrap();

        assert_eq!(machine.thread().unwrap().regs[6], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);
        assert!(matches!(
            machine.process().unwrap().fds[5],
            FdHandle::Closed
        ));
    }

    #[test]
    fn call_cap_negative_corner_cases() {
        let mut machine = test_machine_with_child_domain();

        machine.domains.get_mut(&2).unwrap().generation = 2;
        machine.call_cap(Reg(4), 3, 1, 2).unwrap();
        assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 116);

        machine = test_machine_with_child_domain();
        machine
            .call_cap(Reg(4), 3, CALL_ARG_CAP_MARKER | 1, 2)
            .unwrap();
        assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);

        machine = test_machine_with_child_domain();
        machine.domains.get_mut(&2).unwrap().frozen = true;
        machine.call_cap(Reg(4), 3, 1, 2).unwrap();
        assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 11);

        machine = test_machine_with_child_domain();
        machine.domains.get_mut(&2).unwrap().limits.cpu = 0;
        machine.call_cap(Reg(4), 3, 1, 2).unwrap();
        assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 11);

        machine = test_machine_with_child_domain();
        machine.domains.get_mut(&ROOT_DOMAIN_ID).unwrap().limits.cpu = 0;
        machine.call_cap(Reg(4), 3, 1, 2).unwrap();
        assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 11);

        machine = test_machine_with_child_domain();
        machine.thread_mut().unwrap().cap_call_stack.resize(
            MAX_CAP_CALL_DEPTH,
            CallContinuation {
                return_ip: 0,
                result_reg: Reg(4),
                caller_domain_id: ROOT_DOMAIN_ID,
            },
        );
        machine.call_cap(Reg(4), 3, 1, 2).unwrap();
        assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);

        machine = test_machine_with_child_domain();
        machine.ret_cap(Reg(4), 1, 2).unwrap();
        assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 22);

        machine = test_machine_with_child_domain();
        machine.processes.get_mut(&1).unwrap().domain_id = 2;
        machine
            .thread_mut()
            .unwrap()
            .cap_call_stack
            .push(CallContinuation {
                return_ip: 0,
                result_reg: Reg(5),
                caller_domain_id: ROOT_DOMAIN_ID,
            });
        machine
            .domains
            .get_mut(&ROOT_DOMAIN_ID)
            .unwrap()
            .capability_mask &= !DOMAIN_CAP_CALL;
        machine.ret_cap(Reg(4), 1, 2).unwrap();
        assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);
        assert_eq!(machine.process().unwrap().domain_id, 2);

        machine = test_machine_with_child_domain();
        machine.domains.insert(
            3,
            ResourceDomain {
                id: 3,
                generation: 1,
                parent: Some(ROOT_DOMAIN_ID),
                children: Vec::new(),
                profile: 4,
                limits: DomainLimits::root(),
                capability_mask: u64::MAX,
                upcall_mask: u64::MAX,
                security: DomainSecurityPolicy::root(),
                frozen: false,
                destroyed: false,
                cpu_ticks: 0,
            },
        );
        machine.processes.get_mut(&1).unwrap().domain_id = 3;
        machine
            .thread_mut()
            .unwrap()
            .cap_call_stack
            .push(CallContinuation {
                return_ip: 0,
                result_reg: Reg(5),
                caller_domain_id: 2,
            });
        machine.destroy_domain_recursive(2);
        machine.ret_cap(Reg(4), 1, 2).unwrap();
        assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);
        assert_eq!(machine.process().unwrap().domain_id, 3);
    }

    #[test]
    fn ret_cap_rejects_locked_result_registers_without_popping_or_switching() {
        let mut machine = test_machine_with_child_domain();
        machine.current_tid = 1;
        machine.processes.get_mut(&1).unwrap().domain_id = 2;
        machine.thread_mut().unwrap().ip = 77;
        machine
            .thread_mut()
            .unwrap()
            .cap_call_stack
            .push(CallContinuation {
                return_ip: 123,
                result_reg: Reg(5),
                caller_domain_id: ROOT_DOMAIN_ID,
            });

        let err = machine.ret_cap(Reg(31), 10, 20).unwrap_err();
        assert!(err.contains("hardware-locked stack pointer"), "{err}");
        assert_eq!(machine.process().unwrap().domain_id, 2);
        assert_eq!(machine.thread().unwrap().ip, 77);
        assert_eq!(machine.thread().unwrap().cap_call_stack.len(), 1);
        assert_eq!(machine.thread().unwrap().regs[5], 0);
        assert_eq!(machine.thread().unwrap().regs[30], 0);

        machine.thread_mut().unwrap().cap_call_stack[0].result_reg = Reg(31);

        let err = machine.ret_cap(Reg(5), 10, 20).unwrap_err();
        assert!(err.contains("hardware-locked stack pointer"), "{err}");
        assert_eq!(machine.process().unwrap().domain_id, 2);
        assert_eq!(machine.thread().unwrap().ip, 77);
        assert_eq!(machine.thread().unwrap().cap_call_stack.len(), 1);
        assert_eq!(machine.thread().unwrap().regs[5], 0);
        assert_eq!(machine.thread().unwrap().regs[30], 0);
    }

    #[test]
    fn call_cap_validation_failures_preserve_caller_context() {
        let assert_failed_call_preserved =
            |machine: &mut Machine, arg0: u64, expected_errno: u64| {
                let stack_len = machine.thread().unwrap().cap_call_stack.len();
                let domain_id = machine.process().unwrap().domain_id;
                let ip = machine.thread().unwrap().ip;

                machine.call_cap(Reg(4), 3, arg0, 2).unwrap();

                assert_eq!(machine.thread().unwrap().regs[4], -1i64 as u64);
                assert_eq!(machine.process().unwrap().errno, expected_errno);
                assert_eq!(machine.thread().unwrap().cap_call_stack.len(), stack_len);
                assert_eq!(machine.process().unwrap().domain_id, domain_id);
                assert_eq!(machine.thread().unwrap().ip, ip);
            };

        let mut machine = test_machine_with_child_domain();
        machine.domains.get_mut(&2).unwrap().generation = 2;
        assert_failed_call_preserved(&mut machine, 1, 116);

        let mut machine = test_machine_with_child_domain();
        assert_failed_call_preserved(&mut machine, CALL_ARG_CAP_MARKER | 1, 1);

        let mut machine = test_machine_with_child_domain();
        machine.domains.get_mut(&2).unwrap().frozen = true;
        assert_failed_call_preserved(&mut machine, 1, 11);

        let mut machine = test_machine_with_child_domain();
        machine.domains.get_mut(&2).unwrap().limits.cpu = 0;
        assert_failed_call_preserved(&mut machine, 1, 11);

        let mut machine = test_machine_with_child_domain();
        machine.thread_mut().unwrap().cap_call_stack.resize(
            MAX_CAP_CALL_DEPTH,
            CallContinuation {
                return_ip: 0,
                result_reg: Reg(4),
                caller_domain_id: ROOT_DOMAIN_ID,
            },
        );
        assert_failed_call_preserved(&mut machine, 1, 11);
    }

    #[test]
    fn call_cap_async_and_handoff_modes_execute_minimally() {
        let mut machine = test_machine_with_child_domain();
        machine.processes.get_mut(&1).unwrap().fds[4] = FdHandle::Counter(Rc::new(RefCell::new(0)));
        machine.processes.get_mut(&1).unwrap().fd_capabilities[4] = FdCapability::full(4);
        machine.processes.get_mut(&1).unwrap().fds[3] = FdHandle::CallGate {
            entry: 1,
            domain_id: 2,
            domain_generation: 1,
            mode: CALL_MODE_ASYNC,
            completion_fd: Some(4),
            completion_generation: Some(1),
            flags: 0,
        };
        let err = machine.call_cap(Reg(31), 3, 10, 20).unwrap_err();
        assert!(err.contains("hardware-locked stack pointer"), "{err}");
        assert_eq!(machine.next_call_op_id, 1);
        match &machine.process().unwrap().fds[4] {
            FdHandle::Counter(value) => assert_eq!(*value.borrow(), 0),
            _ => panic!("expected completion counter"),
        }

        machine.call_cap(Reg(6), 3, 10, 20).unwrap();
        assert_eq!(machine.thread().unwrap().regs[6], 1);
        match &machine.process().unwrap().fds[4] {
            FdHandle::Counter(value) => assert_eq!(*value.borrow(), 1),
            _ => panic!("expected completion counter"),
        }
        assert!(machine.thread().unwrap().cap_call_stack.is_empty());
        assert_eq!(machine.process().unwrap().domain_id, ROOT_DOMAIN_ID);

        machine.processes.get_mut(&1).unwrap().fds[3] = FdHandle::CallGate {
            entry: 1,
            domain_id: 2,
            domain_generation: 1,
            mode: CALL_MODE_HANDOFF,
            completion_fd: None,
            completion_generation: None,
            flags: 0,
        };
        machine.call_cap(Reg(6), 3, 33, 44).unwrap();
        assert_eq!(machine.thread().unwrap().regs[6], 0);
        assert_eq!(machine.thread().unwrap().regs[1], 33);
        assert_eq!(machine.thread().unwrap().regs[2], 44);
        assert_eq!(machine.process().unwrap().domain_id, 2);
        assert_eq!(machine.thread().unwrap().ip, 1);
        assert!(machine.thread().unwrap().cap_call_stack.is_empty());
    }

    #[test]
    fn async_call_completion_wakes_waiting_event_queue_reader() {
        let mut machine = test_machine_with_child_domain();
        machine.current_tid = 1;
        create_pipe_pair(&mut machine, 4, 5);
        let completion_generation = machine.fd_generation(5).unwrap();
        machine.processes.get_mut(&1).unwrap().fds[3] = FdHandle::CallGate {
            entry: 1,
            domain_id: 2,
            domain_generation: 1,
            mode: CALL_MODE_ASYNC,
            completion_fd: Some(5),
            completion_generation: Some(completion_generation),
            flags: 0,
        };
        machine
            .push_fd_waiter(4, POLLIN_MASK, Some(Reg(8)))
            .unwrap();
        machine.ready.retain(|tid| *tid != 1);

        machine.call_cap(Reg(6), 3, 10, 20).unwrap();

        assert!(machine.ready.contains(&1));
        assert!(machine.fd_waiters.is_empty());
        assert_eq!(machine.thread().unwrap().regs[6], 1);
        assert!(machine.fd_read_ready(4).unwrap());
    }

    #[test]
    fn async_call_completion_wakes_event_counter_waiter() {
        let mut machine = test_machine_with_child_domain();
        machine.current_tid = 1;
        let counter = Rc::new(RefCell::new(0));
        machine.processes.get_mut(&1).unwrap().fds[4] = FdHandle::EventCounter {
            value: counter.clone(),
            semaphore: false,
        };
        machine.processes.get_mut(&1).unwrap().fd_capabilities[4] = FdCapability::full(4);
        machine.processes.get_mut(&1).unwrap().fds[3] = FdHandle::CallGate {
            entry: 1,
            domain_id: 2,
            domain_generation: 1,
            mode: CALL_MODE_ASYNC,
            completion_fd: Some(4),
            completion_generation: Some(1),
            flags: 0,
        };
        machine
            .push_fd_waiter(4, POLLIN_MASK, Some(Reg(8)))
            .unwrap();
        machine.ready.retain(|tid| *tid != 1);

        machine.call_cap(Reg(6), 3, 10, 20).unwrap();

        assert!(machine.ready.contains(&1));
        assert!(machine.fd_waiters.is_empty());
        assert_eq!(machine.thread().unwrap().regs[6], 1);
        assert_eq!(*counter.borrow(), 1);
    }

    #[test]
    fn async_call_completion_full_queue_reports_errno_without_trapping() {
        let mut machine = test_machine_with_child_domain();
        machine.current_tid = 1;
        create_pipe_pair(&mut machine, 4, 5);
        let queue = match &machine.process().unwrap().fds[5] {
            FdHandle::PipeWriter(queue) => Rc::clone(queue),
            _ => panic!("expected pipe writer"),
        };
        queue.borrow_mut().bytes = vec![0; PIPE_BUFFER_BYTE_LIMIT].into();
        let completion_generation = machine.fd_generation(5).unwrap();
        machine.processes.get_mut(&1).unwrap().fds[3] = FdHandle::CallGate {
            entry: 1,
            domain_id: 2,
            domain_generation: 1,
            mode: CALL_MODE_ASYNC,
            completion_fd: Some(5),
            completion_generation: Some(completion_generation),
            flags: 0,
        };

        machine.call_cap(Reg(6), 3, 10, 20).unwrap();

        assert_eq!(machine.thread().unwrap().regs[6], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 11);
        assert_eq!(queue.borrow().bytes.len(), PIPE_BUFFER_BYTE_LIMIT);
        assert_eq!(machine.next_call_op_id, 2);
    }

    #[test]
    fn async_call_completion_rejects_reused_completion_slot() {
        let mut machine = test_machine_with_child_domain();
        machine.current_tid = 1;
        let original = Rc::new(RefCell::new(0));
        machine.processes.get_mut(&1).unwrap().fds[4] = FdHandle::Counter(original);
        machine.processes.get_mut(&1).unwrap().fd_capabilities[4] = FdCapability::full(4);
        machine.processes.get_mut(&1).unwrap().fds[3] = FdHandle::CallGate {
            entry: 1,
            domain_id: 2,
            domain_generation: 1,
            mode: CALL_MODE_ASYNC,
            completion_fd: Some(4),
            completion_generation: Some(1),
            flags: 0,
        };

        machine.close_fd_index(4).unwrap();
        let replacement = Rc::new(RefCell::new(77));
        machine.processes.get_mut(&1).unwrap().fds[4] = FdHandle::Counter(replacement.clone());
        machine.processes.get_mut(&1).unwrap().fd_capabilities[4] = FdCapability::full(4);

        machine.call_cap(Reg(6), 3, 10, 20).unwrap();

        assert_eq!(machine.thread().unwrap().regs[6], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 116);
        assert_eq!(*replacement.borrow(), 77);
        assert_eq!(machine.next_call_op_id, 1);
    }

    #[test]
    fn domain_property_invariants_cover_edge_cases() {
        let program = Program::parse(
            r#"
            .text
              NOP
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);

        let mut live = vec![ROOT_DOMAIN_ID];
        for idx in 0..24u64 {
            let parent_id = live[(idx as usize * 7 + 3) % live.len()];
            let parent = machine.domains.get(&parent_id).unwrap();
            let id = idx + 2;
            let limits = DomainLimits {
                cpu: parent.limits.cpu.saturating_sub(idx + 1),
                memory: parent.limits.memory.saturating_sub((idx + 1) * 4096),
                pids: parent.limits.pids.saturating_sub(1),
                fdrs: parent.limits.fdrs.saturating_sub(1),
            };
            machine.domains.insert(
                id,
                ResourceDomain {
                    id,
                    generation: 1,
                    parent: Some(parent_id),
                    children: Vec::new(),
                    profile: idx % 9,
                    limits,
                    capability_mask: u64::MAX >> (idx % 8),
                    upcall_mask: u64::MAX >> (idx % 8),
                    security: DomainSecurityPolicy::root(),
                    frozen: false,
                    destroyed: false,
                    cpu_ticks: 0,
                },
            );
            machine
                .domains
                .get_mut(&parent_id)
                .unwrap()
                .children
                .push(id);
            live.push(id);
        }

        for id in &live {
            let mut seen = Vec::new();
            let mut cursor = Some(*id);
            while let Some(domain_id) = cursor {
                assert!(!seen.contains(&domain_id), "cycle at domain {domain_id}");
                seen.push(domain_id);
                cursor = machine.domains.get(&domain_id).unwrap().parent;
            }
            assert!(seen.contains(&ROOT_DOMAIN_ID));
            assert!(machine.domain_depth(*id).unwrap() < MAX_DOMAIN_DEPTH);
            if let Some(parent_id) = machine.domains[id].parent {
                let child = &machine.domains[id];
                let parent = &machine.domains[&parent_id];
                assert!(child.limits.cpu <= parent.limits.cpu);
                assert!(child.limits.memory <= parent.limits.memory);
                assert!(child.limits.pids <= parent.limits.pids);
                assert!(child.limits.fdrs <= parent.limits.fdrs);
            }
        }

        let leaf = *live.last().unwrap();
        machine.destroy_domain_recursive(leaf);
        assert_eq!(machine.domain_ref(leaf, 1), Err(116));

        let parent_id = machine.domains[&2].parent.unwrap();
        assert_eq!(Machine::delegate_limit(101, 100), Err(1));

        machine.processes.get_mut(&1).unwrap().domain_id = 2;
        assert_eq!(machine.domain_ctl_detach_self(), Ok(parent_id));
        machine.processes.get_mut(&1).unwrap().domain_id = 2;
        machine.destroy_domain_recursive(2);
        let arg = ARG_BASE;
        machine.store_u64(arg + 8, 2).unwrap();
        machine.store_u64(arg + 16, 1).unwrap();
        assert_eq!(machine.domain_ctl_attach_self(arg), Err(116));

        machine.domains.insert(
            100,
            ResourceDomain {
                id: 100,
                generation: 1,
                parent: Some(ROOT_DOMAIN_ID),
                children: Vec::new(),
                profile: 4,
                limits: DomainLimits::root(),
                capability_mask: u64::MAX,
                upcall_mask: u64::MAX,
                security: DomainSecurityPolicy::root(),
                frozen: false,
                destroyed: false,
                cpu_ticks: 0,
            },
        );
        machine
            .domains
            .get_mut(&ROOT_DOMAIN_ID)
            .unwrap()
            .children
            .push(100);
        machine.processes.get_mut(&1).unwrap().domain_id = 100;
        machine.ready.clear();
        machine.ready.push_back(1);
        machine.set_domain_frozen_recursive(100, true);
        machine.park_domain_threads(100);
        assert!(machine.ready.is_empty());
        assert_eq!(machine.domain_parked.front(), Some(&1));
        machine.set_domain_frozen_recursive(100, false);
        machine.release_domain_threads();
        assert!(machine.ready.contains(&1));

        machine.domains.insert(
            101,
            ResourceDomain {
                id: 101,
                generation: 1,
                parent: Some(100),
                children: Vec::new(),
                profile: 8,
                limits: DomainLimits::root(),
                capability_mask: u64::MAX,
                upcall_mask: u64::MAX,
                security: DomainSecurityPolicy::root(),
                frozen: false,
                destroyed: false,
                cpu_ticks: 0,
            },
        );
        machine.domains.get_mut(&100).unwrap().children.push(101);
        machine.domain_parked.push_back(1);
        machine.mask_descendant_capabilities(100, DOMAIN_CAP_IO | DOMAIN_CAP_FDR);
        assert_eq!(
            machine.domains[&101].capability_mask,
            DOMAIN_CAP_IO | DOMAIN_CAP_FDR
        );
    }

    #[test]
    fn uses_dedicated_fpu_and_vector_register_files() {
        let program = Program::parse(
            r#"
            .text
              FADD f3, f1, f2
              VADD.32 v3, v1, v2
              EXIT r0
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        {
            let thread = machine.threads.get_mut(&1).unwrap();
            thread.fregs[1] = 1.5f64.to_bits();
            thread.fregs[2] = 2.25f64.to_bits();
            thread.vregs[1] = 1 | (2 << 32) | (3 << 64) | (4 << 96);
            thread.vregs[2] = 10 | (20 << 32) | (30 << 64) | (40 << 96);
        }
        assert_eq!(machine.run().unwrap(), 0);
        let thread = machine.threads.get(&1);
        assert!(thread.is_none(), "thread exits after verification run");

        let program = Program::parse(
            r#"
            .text
              FADD f3, f1, f2
              VADD.32 v3, v1, v2
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        {
            let thread = machine.threads.get_mut(&1).unwrap();
            thread.fregs[1] = 1.5f64.to_bits();
            thread.fregs[2] = 2.25f64.to_bits();
            thread.vregs[1] = 1 | (2 << 32) | (3 << 64) | (4 << 96);
            thread.vregs[2] = 10 | (20 << 32) | (30 << 64) | (40 << 96);
        }
        machine.current_tid = 1;
        let fadd = machine.processes[&1].program.instructions[0].clone();
        machine.exec(fadd).unwrap();
        let vadd = machine.processes[&1].program.instructions[1].clone();
        machine.exec(vadd).unwrap();
        let thread = machine.threads.get(&1).unwrap();
        assert_eq!(f64::from_bits(thread.fregs[3]), 3.75);
        assert_eq!(thread.vregs[3], 11 | (22 << 32) | (33 << 64) | (44 << 96));
    }

    #[test]
    fn rejects_writes_to_locked_stack_pointer() {
        let program = Program::parse(
            r#"
            .text
              LI r1, 123
              MOV r31, r1
              EXIT r0
            "#,
        )
        .unwrap();
        let mut machine = Machine::new(program);
        let err = machine.run().unwrap_err();
        assert!(err.contains("hardware-locked stack pointer"), "{err}");
    }
}

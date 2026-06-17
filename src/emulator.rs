use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::os::fd::AsRawFd;
use std::os::raw::{c_char, c_int};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::asm::Program;
use crate::isa::*;

const STACK_SIZE: u64 = 4 * 1024 * 1024;
const CALL_FRAME_SIZE: u64 = 32 * 1024;
const THREAD_STACK_STRIDE: u64 = 0x80_000;
const MMAP_BASE: u64 = 0x200_000;
const ASLR_PAGE: u64 = 4096;
const ASLR_HEAP_PAGES: u64 = 16;
const ASLR_MMAP_PAGES: u64 = 16;
const ASLR_STACK_PAGES: u64 = 16;
const SIGCHLD: u64 = 17;
const SIGALRM: u64 = 14;
const SIGSEGV: u64 = 11;
const MESSAGE_ENDPOINT_FD: usize = FDR_COUNT - 1;
const UTIME_NOW_LNP64: i64 = 1_073_741_823;
const UTIME_OMIT_LNP64: i64 = 1_073_741_822;
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
const ENV_KEY_PROCESS_ENTRY_RECORD: u64 = 64;
const ENV_ISA_VERSION: u64 = 1;
const ENV_HWCAP0_RANDOM: u64 = 1 << 0;
const ENV_HWCAP0_CAPABILITIES: u64 = 1 << 1;
const ENV_HWCAP0_RESOURCE_DOMAINS: u64 = 1 << 2;
const ENV_HWCAP0_DMA: u64 = 1 << 3;
const ENV_HWCAP0_FUTEX: u64 = 1 << 4;
const ENV_CACHE_LINE_SIZE: u64 = 64;
const ENV_TIMEBASE_HZ: u64 = 1_000_000_000;
const ENV_THREAD_LIMIT: u64 = 4096;
const ENV_PROCESS_LIMIT: u64 = 4096;
const ENV_EVENT_QUEUE_LIMIT: u64 = 4096;
const ENV_FUTEX_BUCKET_COUNT: u64 = 4096;
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
const OBJECT_KIND_COUNTER: u64 = 1;
const OBJECT_KIND_QUEUE: u64 = 2;
const OBJECT_KIND_MEMORY_OBJECT: u64 = 3;
const OBJECT_KIND_DMA_BUFFER: u64 = 4;
const OBJECT_KIND_ENDPOINT: u64 = 5;
const OBJECT_KIND_TIMER: u64 = 6;
const OBJECT_PROFILE_PIPE: u64 = 1;
const OBJECT_PROFILE_TCP_STREAM: u64 = 2;
const OBJECT_PROFILE_CALL_GATE: u64 = 4;
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
    MemoryObject {
        data: Rc<RefCell<Vec<u8>>>,
        pos: usize,
    },
    Timer(Rc<RefCell<TimerState>>),
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
            FdHandle::MemoryObject { data, pos } => Ok(FdHandle::MemoryObject {
                data: Rc::clone(data),
                pos: *pos,
            }),
            FdHandle::Timer(timer) => Ok(FdHandle::Timer(Rc::clone(timer))),
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
                flags,
            } => Ok(FdHandle::CallGate {
                entry: *entry,
                domain_id: *domain_id,
                domain_generation: *domain_generation,
                mode: *mode,
                completion_fd: *completion_fd,
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

#[derive(Default)]
struct TimerState {
    remaining: u64,
    interval: u64,
    expirations: u64,
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
        addr >= self.start && end <= self.start + self.len
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
    signal_handlers: HashMap<u64, usize>,
    pending_signals: VecDeque<u64>,
    inbox: VecDeque<(u64, u64)>,
    ucode_ports: HashMap<u64, u8>,
    errno: u64,
    cwd: PathBuf,
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
            pending_signals: VecDeque::new(),
            inbox: VecDeque::new(),
            ucode_ports: HashMap::new(),
            errno: 0,
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
            pending_signals: VecDeque::new(),
            inbox: VecDeque::new(),
            ucode_ports: self.ucode_ports.clone(),
            errno: self.errno,
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
    completed_threads: HashMap<u64, u64>,
    fd_waiters: Vec<FdWaiter>,
    current_tid: u64,
    next_pid: u64,
    next_tid: u64,
    next_domain_id: u64,
    next_call_op_id: u64,
    next_cap_lineage: u64,
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
            completed_threads: HashMap::new(),
            fd_waiters: Vec::new(),
            current_tid: root_tid,
            next_pid: 2,
            next_tid: 2,
            next_domain_id: 2,
            next_call_op_id: 1,
            next_cap_lineage: FDR_COUNT as u64 + 1,
            random_state: 0x4d59_5df4_d0f3_3173,
            last_exit: 0,
        }
    }

    pub fn set_args(&mut self, args: &[String]) -> Result<(), String> {
        self.set_process_entry(args, &[])
    }

    pub fn set_process_entry(&mut self, args: &[String], env: &[String]) -> Result<(), String> {
        let pid = self.thread()?.pid;
        let process = self
            .processes
            .get_mut(&pid)
            .ok_or_else(|| format!("missing process {pid}"))?;
        let argc_addr = ARG_BASE as usize;
        let argv_addr = (ARG_BASE + 8) as usize;
        let envp_addr = argv_addr + (args.len() + 1) * 8;
        let mut str_addr = ARG_BASE + 0x1000;
        let arg_page_start = ARG_BASE as usize;
        let arg_page_end = (ARG_BASE + ARG_SIZE) as usize;
        process.memory[arg_page_start..arg_page_end].fill(0);
        if envp_addr + (env.len() + 1) * 8 > str_addr as usize {
            return Err("process entry pointer table exceeds reserved argument area".to_string());
        }
        process.memory[argc_addr..argc_addr + 8]
            .copy_from_slice(&(args.len() as u64).to_le_bytes());
        for (idx, arg) in args.iter().enumerate() {
            let ptr_slot = argv_addr + idx * 8;
            process.memory[ptr_slot..ptr_slot + 8].copy_from_slice(&str_addr.to_le_bytes());
            let bytes = arg.as_bytes();
            let start = str_addr as usize;
            let end = start + bytes.len();
            if end + 1 >= (ARG_BASE + ARG_SIZE) as usize {
                return Err("argv data exceeds emulated argument page".to_string());
            }
            process.memory[start..end].copy_from_slice(bytes);
            process.memory[end] = 0;
            str_addr += bytes.len() as u64 + 1;
        }
        let null_slot = argv_addr + args.len() * 8;
        process.memory[null_slot..null_slot + 8].copy_from_slice(&0u64.to_le_bytes());
        for (idx, item) in env.iter().enumerate() {
            let ptr_slot = envp_addr + idx * 8;
            process.memory[ptr_slot..ptr_slot + 8].copy_from_slice(&str_addr.to_le_bytes());
            let bytes = item.as_bytes();
            let start = str_addr as usize;
            let end = start + bytes.len();
            if end + 1 >= (ARG_BASE + ARG_SIZE) as usize {
                return Err("envp data exceeds emulated argument page".to_string());
            }
            process.memory[start..end].copy_from_slice(bytes);
            process.memory[end] = 0;
            str_addr += bytes.len() as u64 + 1;
        }
        let null_slot = envp_addr + env.len() * 8;
        process.memory[null_slot..null_slot + 8].copy_from_slice(&0u64.to_le_bytes());
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
                let divisor = self.read_reg(b)?;
                if divisor == 0 {
                    self.raise_current_signal(8)?;
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
                self.thread_mut()?.regs[31] = sp;
                self.store_u64(sp, ret)?;
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
                self.thread_mut()?.regs[31] = sp;
                self.store_u64(sp, ret)?;
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
                self.require_domain_cap(DOMAIN_CAP_IO)?;
                if fd.0 == MESSAGE_ENDPOINT_FD {
                    let Some((v1, v2)) = self.process_mut()?.inbox.pop_front() else {
                        self.thread_mut()?.ip = self.thread()?.ip.saturating_sub(1);
                        self.ready.retain(|tid| *tid != self.current_tid);
                        return Ok(false);
                    };
                    self.set_errno(0)?;
                    self.write_reg(result, v1)?;
                    self.write_reg(Reg(30), v2)?;
                } else {
                    let addr = self.read_reg(buf)?;
                    let len = self.read_reg(len)? as usize;
                    let count = self.read_fd_index(fd.0, addr, len)?;
                    self.set_errno(0)?;
                    self.write_reg(result, count as u64)?;
                }
            }
            Instr::Push(result, fd, buf, len) => {
                self.require_domain_cap(DOMAIN_CAP_IO)?;
                let addr = self.read_reg(buf)?;
                let len = self.read_reg(len)? as usize;
                self.write_fd_index(fd.0, addr, len)?;
                self.write_reg(result, self.read_reg(Reg(1))?)?;
            }
            Instr::Await(result, fd, mask) => {
                let mask = self.read_reg(mask)?;
                if !self.fd_ready_for_mask(fd.0, mask)? {
                    self.push_fd_waiter(fd.0, mask, Some(result))?;
                    self.ready.retain(|tid| *tid != self.current_tid);
                    return Ok(false);
                }
                self.set_errno(0)?;
                self.write_reg(result, 0)?;
            }
            Instr::AwaitDyn(result, fd_reg, mask) => {
                let fd = self.read_reg(fd_reg)?;
                let mask = self.read_reg(mask)?;
                let Some(fd) = self.checked_fd_index(fd)? else {
                    self.write_reg(result, -1i64 as u64)?;
                    return Ok(true);
                };
                if !self.fd_ready_for_mask(fd, mask)? {
                    self.push_fd_waiter(fd, mask, Some(result))?;
                    self.ready.retain(|tid| *tid != self.current_tid);
                    return Ok(false);
                }
                self.set_errno(0)?;
                self.write_reg(result, 0)?;
            }
            Instr::PollFd(result, fd, events) => {
                let events = self.read_reg(events)?;
                let revents = self.poll_fd_mask(fd.0 as u64, events)?;
                self.write_reg(result, revents)?;
            }
            Instr::PollFdDyn(result, fd_reg, events) => {
                let fd = self.read_reg(fd_reg)?;
                let events = self.read_reg(events)?;
                let revents = match self.checked_fd_index(fd)? {
                    Some(fd) => self.poll_fd_index_mask(fd, events)?,
                    None => POLLNVAL_MASK,
                };
                self.set_errno(0)?;
                self.write_reg(result, revents)?;
            }
            Instr::Alloc(dst, bytes_reg) => {
                let len = (self.read_reg(bytes_reg)? as usize).max(1);
                let addr = self.alloc_heap(len, 64, false)?;
                self.write_reg(dst, addr)?;
            }
            Instr::AllocEx(dst, bytes_reg, align_reg) => {
                let len = (self.read_reg(bytes_reg)? as usize).max(1);
                let align = self.read_reg(align_reg)?.clamp(1, 4096).next_power_of_two();
                let addr = self.alloc_heap(len, align, true)?;
                self.write_reg(dst, addr)?;
            }
            Instr::AllocSize(dst, ptr_reg) => {
                let ptr = self.read_reg(ptr_reg)?;
                let size = self
                    .process()?
                    .allocations
                    .get(&ptr)
                    .map(|allocation| allocation.len)
                    .unwrap_or(0);
                self.write_reg(dst, size as u64)?;
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
                let path = self.resolve_process_path(&path)?;
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
                self.require_domain_cap(DOMAIN_CAP_FDR)?;
                if !self.check_domain_budget(0, 0, 0, 1)? {
                    self.write_reg(dst_reg, -1i64 as u64)?;
                    return Ok(true);
                }
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let path = self.resolve_process_path(&path)?;
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
                let path = self.resolve_process_path(&path)?;
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
                self.require_domain_cap(DOMAIN_CAP_FDR)?;
                if !self.check_domain_budget(0, 0, 0, 1)? {
                    self.write_reg(dst_reg, -1i64 as u64)?;
                    return Ok(true);
                }
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let path = self.resolve_process_path(&path)?;
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
                let count = self.read_fd_index(fd.0, addr, len)?;
                self.write_reg(Reg(1), count as u64)?;
            }
            Instr::ReadFdDyn(fd_reg, buf, len) => {
                self.require_domain_cap(DOMAIN_CAP_IO)?;
                let fd = self.read_reg(fd_reg)?;
                let addr = self.read_reg(buf)?;
                let len = self.read_reg(len)? as usize;
                if let Some(fd) = self.checked_fd_index(fd)? {
                    let count = self.read_fd_index(fd, addr, len)?;
                    self.write_reg(Reg(1), count as u64)?;
                } else {
                    self.write_reg(Reg(1), 0)?;
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
                } else {
                    self.write_reg(Reg(1), 0)?;
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
                } else {
                    self.write_reg(Reg(1), 0)?;
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
                } else {
                    self.write_reg(Reg(1), 0)?;
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
                } else {
                    self.write_reg(Reg(1), 0)?;
                }
            }
            Instr::MkdirPath(path_reg, _mode_reg) => {
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let path = self.resolve_process_path(&path)?;
                match fs::create_dir(&path) {
                    Ok(()) => self.set_status_ok()?,
                    Err(err) => self.set_status_io_error(err)?,
                }
            }
            Instr::UnlinkPath(path_reg) => {
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let path = self.resolve_process_path(&path)?;
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
                let old = self.resolve_process_path(&old)?;
                let new = self.resolve_process_path(&new)?;
                match fs::rename(&old, &new) {
                    Ok(()) => self.set_status_ok()?,
                    Err(err) => self.set_status_io_error(err)?,
                }
            }
            Instr::LinkPath(old_reg, new_reg, flags_reg) => {
                let old = self.read_c_string(self.read_reg(old_reg)?)?;
                let new = self.read_c_string(self.read_reg(new_reg)?)?;
                let flags = self.read_reg(flags_reg)?;
                let old_path = self.resolve_process_path(&old)?;
                let new = self.resolve_process_path(&new)?;
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
            Instr::SymlinkPath(target_reg, link_reg) => {
                let target = self.read_c_string(self.read_reg(target_reg)?)?;
                let link = self.read_c_string(self.read_reg(link_reg)?)?;
                let link = self.resolve_process_path(&link)?;
                match std::os::unix::fs::symlink(&target, &link) {
                    Ok(()) => self.set_status_ok()?,
                    Err(err) => self.set_status_io_error(err)?,
                }
            }
            Instr::ReadlinkPath(path_reg, buf_reg, len_reg) => {
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let path = self.resolve_process_path(&path)?;
                let buf = self.read_reg(buf_reg)?;
                let len = self.read_reg(len_reg)? as usize;
                match fs::read_link(&path) {
                    Ok(target) => {
                        let bytes = target.to_string_lossy();
                        let data = bytes.as_bytes();
                        let count = data.len().min(len);
                        self.write_bytes(buf, &data[..count])?;
                        self.set_errno(0)?;
                        self.write_reg(Reg(1), count as u64)?;
                    }
                    Err(err) => self.set_status_io_error(err)?,
                }
            }
            Instr::ChdirPath(path_reg) => {
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let path = self.resolve_process_path(&path)?;
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
                if len == 0 || bytes.len() + 1 > len {
                    self.set_status_errno(34)?;
                } else {
                    self.write_bytes(buf, bytes)?;
                    self.write_bytes(buf + bytes.len() as u64, &[0])?;
                    self.set_errno(0)?;
                    self.write_reg(Reg(1), buf)?;
                }
            }
            Instr::ChmodPath(path_reg, mode_reg, _flags_reg) => {
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let path = self.resolve_process_path(&path)?;
                let mode = self.read_reg(mode_reg)? as u32;
                match fs::set_permissions(&path, fs::Permissions::from_mode(mode)) {
                    Ok(()) => self.set_status_ok()?,
                    Err(err) => self.set_status_io_error(err)?,
                }
            }
            Instr::ChownPath(path_reg, uid_reg, gid_reg, _flags_reg) => {
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let path = self.resolve_process_path(&path)?;
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
                let path = self.resolve_process_path(&path)?;
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
                let path = self.resolve_process_path(&path)?;
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
            Instr::FdClose(fd) => {
                self.close_fd_index(fd.0)?;
                self.set_status_ok()?;
            }
            Instr::FdCloseDyn(fd_reg) => {
                let fd = self.read_reg(fd_reg)?;
                if let Some(fd) = self.checked_fd_index(fd)? {
                    self.close_fd_index(fd)?;
                    self.set_status_ok()?;
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
                let pid = self.read_reg(pid_reg)?;
                let current_pid = self.thread()?.pid;
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
                    self.sleepers.push((self.current_tid, 1));
                    self.ready.retain(|tid| *tid != self.current_tid);
                    return Ok(false);
                }
                if pid == 0 || !self.processes.contains_key(&pid) {
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
                let len = self.read_reg(len_reg)?;
                let bytes = if len == 0 { 8 } else { len };
                if self.consume_domain_entropy(bytes).is_err() {
                    self.set_status_errno(1)?;
                    self.write_reg(result, -1i64 as u64)?;
                    return Ok(true);
                }
                if len == 0 {
                    let value = self.next_random_u64();
                    self.set_errno(0)?;
                    self.write_reg(result, value)?;
                } else {
                    let addr = self.read_reg(buf)?;
                    let data = self.random_bytes(len as usize);
                    self.write_bytes(addr, &data)?;
                    self.set_errno(0)?;
                    self.write_reg(result, len)?;
                }
            }
            Instr::Fork(dst) => {
                self.require_domain_cap(DOMAIN_CAP_PROCESS)?;
                if !self.check_domain_budget(0, 0, 1, 0)? {
                    self.write_reg(dst, -1i64 as u64)?;
                    return Ok(true);
                }
                let child_pid = self.next_pid;
                self.next_pid += 1;
                let child_tid = self.next_tid;
                self.next_tid += 1;

                let parent_pid = self.thread()?.pid;
                let child_process = self.process()?.fork_clone(child_pid)?;
                let mut child_thread = self.thread()?.clone();
                child_thread.pid = child_pid;
                child_thread.tid = child_tid;
                if dst.0 != 0 && dst.0 != 31 {
                    child_thread.regs[dst.0] = 0;
                }
                self.processes.insert(child_pid, child_process);
                self.threads.insert(child_tid, child_thread);
                self.ready.push_back(child_tid);
                self.write_reg(dst, child_pid)?;
                let _ = parent_pid;
            }
            Instr::Exec(path_reg, argv_reg, envp_reg) => {
                let path = self.read_c_string(self.read_reg(path_reg)?)?;
                let argv = self.read_reg(argv_reg)?;
                let envp = self.read_reg(envp_reg)?;
                let args = self.collect_exec_args(&path, argv)?;
                let env = self.collect_exec_env(envp)?;
                let source = fs::read_to_string(&path)
                    .map_err(|err| format!("EXEC failed to read {path:?}: {err}"))?;
                let program = Program::parse(&source)
                    .map_err(|err| format!("EXEC failed to assemble {path:?}: {err}"))?;
                let pid = self.thread()?.pid;
                let domain_id = self.process()?.domain_id;
                let aslr_enabled = self
                    .domains
                    .get(&domain_id)
                    .map(|domain| domain.security.aslr_enabled)
                    .unwrap_or(true);
                let layout = ProcessLayout::for_process(pid, domain_id, aslr_enabled);
                self.process_mut()?.exec(program, layout);
                let pid = self.thread()?.pid;
                let tid = self.thread()?.tid;
                *self.thread_mut()? = Thread::new(tid, pid, layout.stack_top);
                self.set_process_entry(&args, &env)?;
            }
            Instr::Spawn(dst, entry) => {
                self.require_domain_cap(DOMAIN_CAP_PROCESS)?;
                if !self.check_domain_budget(0, 0, 1, 0)? {
                    self.write_reg(dst, -1i64 as u64)?;
                    return Ok(true);
                }
                let tid = self.next_tid;
                self.next_tid += 1;
                let mut child = self.thread()?.clone();
                child.tid = tid;
                child.thread_pointer = 0;
                child.ip = self.read_reg(entry)? as usize;
                let stack_top = self.process()?.stack_top;
                child.regs[31] = stack_top - CALL_FRAME_SIZE - ((tid - 1) * THREAD_STACK_STRIDE);
                self.threads.insert(tid, child);
                self.ready.push_back(tid);
                self.write_reg(dst, tid)?;
            }
            Instr::ThreadJoin(result, tid_reg, retval_reg) => {
                let tid = self.read_reg(tid_reg)?;
                let retval_ptr = self.read_reg(retval_reg)?;
                if tid == self.current_tid {
                    self.write_reg(result, 35)?;
                } else if let Some(value) = self.completed_threads.remove(&tid) {
                    if retval_ptr != 0 {
                        self.store_u64(retval_ptr, value)?;
                    }
                    self.write_reg(result, 0)?;
                } else if self.threads.contains_key(&tid) {
                    self.thread_mut()?.ip = self.thread()?.ip.saturating_sub(1);
                    self.thread_join_waiters
                        .entry(tid)
                        .or_default()
                        .push_back(self.current_tid);
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
                let len = self.read_reg(len)?.max(1);
                self.require_domain_cap(DOMAIN_CAP_MEMORY)?;
                if !self.check_domain_budget(len, 1, 0, 0)? {
                    self.write_reg(dst, -1i64 as u64)?;
                    return Ok(true);
                }
                let prot = self.read_reg(prot)?;
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
                let addr = {
                    let process = self.process_mut()?;
                    let addr = if hint != 0 {
                        hint
                    } else {
                        align_up(process.mmap_next, 4096)
                    };
                    let end = addr
                        .checked_add(len)
                        .ok_or_else(|| "MMAP range overflow".to_string())?;
                    if end as usize >= process.memory.len() {
                        return Err(format!("MMAP out of range: 0x{addr:x} + {len}"));
                    }
                    process.mmap_next = end;
                    process.vmas.push(Vma {
                        start: addr,
                        len,
                        prot,
                        file,
                        file_offset: offset,
                        resident: false,
                        guard: false,
                    });
                    addr
                };
                self.write_reg(dst, addr)?;
            }
            Instr::Munmap(addr, _len) => {
                let addr = self.read_reg(addr)?;
                self.process_mut()?.vmas.retain(|vma| vma.start != addr);
            }
            Instr::Mprotect(addr, len, prot) => {
                let addr = self.read_reg(addr)?;
                let len = self.read_reg(len)?;
                let prot = self.read_reg(prot)?;
                self.mprotect_range(addr, len, prot)?;
            }
            Instr::Sigaction(signum, handler) => {
                let signum = self.read_reg(signum)?;
                let handler = self.read_reg(handler)? as usize;
                self.process_mut()?.signal_handlers.insert(signum, handler);
            }
            Instr::SigmaskSet(mask) => {
                let mask = self.read_reg(mask)?;
                self.process_mut()?.sigmask = mask;
            }
            Instr::Alarm(dst, seconds) => {
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
                self.raise_process_signal(pid, signum);
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
                    self.futex_waiters
                        .entry(addr)
                        .or_default()
                        .push_back(self.current_tid);
                    self.ready.retain(|tid| *tid != self.current_tid);
                    return Ok(false);
                }
            }
            Instr::FutexWake(addr_reg, count_reg) => {
                let addr = self.read_reg(addr_reg)?;
                let count = self.read_reg(count_reg)?;
                let mut to_wake = Vec::new();
                if let Some(waiters) = self.futex_waiters.get_mut(&addr) {
                    for _ in 0..count {
                        let Some(tid) = waiters.pop_front() else {
                            break;
                        };
                        to_wake.push(tid);
                    }
                }
                for tid in to_wake {
                    self.wake_thread(tid);
                }
            }
            Instr::Inb(dst, port) => {
                let value = self
                    .process()?
                    .ucode_ports
                    .get(&self.read_reg(port)?)
                    .copied()
                    .unwrap_or(0);
                self.write_reg(dst, value as u64)?;
            }
            Instr::Outb(port, src) => {
                let port = self.read_reg(port)?;
                let value = self.read_reg(src)? as u8;
                self.process_mut()?.ucode_ports.insert(port, value);
            }
            Instr::LoadUcode(buf, len) => {
                if self.process()?.uid != 0 {
                    self.raise_current_signal(SIGSEGV)?;
                    return Ok(true);
                }
                let blob = self.read_bytes(self.read_reg(buf)?, self.read_reg(len)? as usize)?;
                self.load_microcode(&blob)?;
            }
            Instr::MsgSend(pid, v1, v2) => {
                let pid = self.read_reg(pid)?;
                let msg = (self.read_reg(v1)?, self.read_reg(v2)?);
                if let Some(process) = self.processes.get_mut(&pid) {
                    process.inbox.push_back(msg);
                    if let Some(tid) = self
                        .threads
                        .values()
                        .find(|thread| thread.pid == pid)
                        .map(|thread| thread.tid)
                    {
                        self.wake_thread(tid);
                    }
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

    fn set_errno(&mut self, errno: u64) -> Result<(), String> {
        self.process_mut()?.errno = errno;
        Ok(())
    }

    fn set_status_ok(&mut self) -> Result<(), String> {
        self.set_errno(0)?;
        self.write_reg(Reg(1), 0)
    }

    fn set_status_errno(&mut self, errno: u64) -> Result<(), String> {
        self.set_errno(errno)?;
        self.write_reg(Reg(1), -1i64 as u64)
    }

    fn set_status_io_error(&mut self, err: io::Error) -> Result<(), String> {
        self.set_status_errno(Self::errno_from_io(&err))
    }

    fn resolve_process_path(&self, path: &str) -> Result<String, String> {
        if path.is_empty() || path.starts_with("tcp-listen:") || Path::new(path).is_absolute() {
            return Ok(path.to_string());
        }
        Ok(self
            .process()?
            .cwd
            .join(path)
            .to_string_lossy()
            .into_owned())
    }

    fn write_lnp64_stat(&mut self, addr: u64, metadata: &fs::Metadata) -> Result<(), String> {
        let fields = [
            (0, metadata.mode() as u64),
            (8, metadata.size()),
            (16, metadata.dev()),
            (24, metadata.ino()),
            (32, metadata.mtime() as u64),
            (40, metadata.mtime_nsec() as u64),
            (48, metadata.nlink()),
            (56, metadata.uid() as u64),
            (64, metadata.gid() as u64),
            (72, metadata.atime() as u64),
            (80, metadata.atime_nsec() as u64),
            (88, metadata.ctime() as u64),
            (96, metadata.ctime_nsec() as u64),
        ];
        for (offset, value) in fields {
            self.store_u64(addr + offset, value)?;
        }
        Ok(())
    }

    fn write_synthetic_stat(&mut self, addr: u64, mode: u64, size: u64) -> Result<(), String> {
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
            self.store_u64(addr + offset, value)?;
        }
        Ok(())
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
                align_up(
                    process
                        .heap_next
                        .checked_add(guard_len)
                        .ok_or_else(|| "allocation overflow".to_string())?,
                    align,
                )
            } else {
                align_up(process.heap_next, align)
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
        self.bump_fd_generation(fd)?;
        self.process_mut()?.fds[fd] = FdHandle::Closed;
        let lineage = self.fresh_fd_capability().lineage;
        self.install_fd_capability(fd, FdCapability::closed(lineage))?;
        Ok(())
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
                buffer.borrow_mut().bytes.extend(data.iter().copied());
                Ok(())
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
            FdHandle::MemoryObject { data: object, pos } => {
                let mut object = object.borrow_mut();
                let end = pos.saturating_add(data.len());
                if end > object.len() {
                    object.resize(end, 0);
                }
                object[*pos..end].copy_from_slice(&data);
                *pos = end;
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
            | FdHandle::TcpSocket { .. }
            | FdHandle::DmaBuffer { .. }
            | FdHandle::CallGate { .. }
            | FdHandle::Closed => Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "fd is not writable",
            )),
        };
        match result {
            Ok(()) => {
                self.set_errno(0)?;
                self.write_reg(Reg(1), data.len() as u64)?;
            }
            Err(err) => self.set_status_io_error(err)?,
        }
        Ok(())
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
            Ok(()) => {
                self.set_errno(0)?;
                self.write_reg(Reg(1), data.len() as u64)?;
            }
            Err(err) => self.set_status_io_error(err)?,
        }
        Ok(())
    }

    fn readdir_fd_index(&mut self, fd: usize, addr: u64) -> Result<(), String> {
        let entry = match &mut self.process_mut()?.fds[fd] {
            FdHandle::Dir { entries, pos, .. } => {
                if *pos >= entries.len() {
                    None
                } else {
                    let entry = entries[*pos].clone();
                    *pos += 1;
                    Some(entry)
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
        let mtime = self.host_timespec_at(times_ptr + 16, now)?;
        Ok(Some([atime, mtime]))
    }

    fn host_timespec_at(&mut self, addr: u64, now: HostTimespec) -> Result<HostTimespec, String> {
        let sec = self.load_u64(addr)? as i64;
        let nsec = self.load_u64(addr + 8)? as i64;
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
                Ok(pos) => {
                    self.set_errno(0)?;
                    self.write_reg(Reg(1), pos)?;
                }
                Err(err) => self.set_status_io_error(err)?,
            }
        } else {
            self.set_status_errno(22)?;
        }
        Ok(())
    }

    fn mprotect_range(&mut self, addr: u64, len: u64, prot: u64) -> Result<(), String> {
        if !self.domain_allows_prot(prot)? {
            self.set_status_errno(1)?;
            return Ok(());
        }
        let len = len.max(1);
        let Some(end) = addr.checked_add(len) else {
            self.set_status_errno(22)?;
            return Ok(());
        };
        let Some(idx) = self.process()?.vmas.iter().position(|vma| {
            let vma_end = vma.start + vma.len;
            addr >= vma.start && end <= vma_end
        }) else {
            self.set_status_errno(12)?;
            return Ok(());
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

    fn read_fd_index(&mut self, fd: usize, addr: u64, len: usize) -> Result<usize, String> {
        if self.ensure_fd_right(fd, CAP_RIGHT_READ).is_err() {
            return Ok(0);
        }
        let mut tmp = vec![0; len];
        let count = match &mut self.process_mut()?.fds[fd] {
            FdHandle::Stdin => io::stdin()
                .read(&mut tmp)
                .map_err(|err| format!("READ_FD fd0: {err}"))?,
            FdHandle::File(file) => file
                .read(&mut tmp)
                .map_err(|err| format!("READ_FD fd{fd}: {err}"))?,
            FdHandle::PipeReader(buffer) => {
                let mut buffer = buffer.borrow_mut();
                let mut count = 0;
                while count < len {
                    let Some(byte) = buffer.bytes.pop_front() else {
                        break;
                    };
                    tmp[count] = byte;
                    count += 1;
                }
                count
            }
            FdHandle::Counter(value) => {
                let bytes = value.borrow().to_le_bytes();
                let count = len.min(bytes.len());
                tmp[..count].copy_from_slice(&bytes[..count]);
                count
            }
            FdHandle::MemoryObject { data, pos } => {
                let data = data.borrow();
                let available = data.len().saturating_sub(*pos);
                let count = len.min(available);
                tmp[..count].copy_from_slice(&data[*pos..*pos + count]);
                *pos += count;
                count
            }
            FdHandle::Timer(timer) => {
                let mut timer = timer.borrow_mut();
                if timer.expirations == 0 {
                    0
                } else {
                    let bytes = timer.expirations.to_le_bytes();
                    let count = len.min(bytes.len());
                    tmp[..count].copy_from_slice(&bytes[..count]);
                    timer.expirations = 0;
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
            | FdHandle::Closed => 0,
        };
        self.write_bytes(addr, &tmp[..count])?;
        Ok(count)
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
                self.set_errno(0)?;
                self.write_reg(Reg(1), count as u64)?;
            }
            Err(err) => self.set_status_io_error(err)?,
        }
        Ok(())
    }

    fn object_ctl(&mut self, result: Reg, argblock: u64) -> Result<(), String> {
        let op = self.load_u64(argblock)?;
        let value = match op {
            OBJECT_OP_CREATE => self.object_ctl_create(argblock),
            OBJECT_OP_SOCKET_BIND => self.object_ctl_socket_bind(argblock),
            OBJECT_OP_SOCKET_LISTEN => self.object_ctl_socket_listen(argblock),
            OBJECT_OP_SOCKET_CONNECT => self.object_ctl_socket_connect(argblock),
            OBJECT_OP_SOCKET_ACCEPT => self.object_ctl_socket_accept(argblock),
            OBJECT_OP_SOCKET_GETSOCKNAME => self.object_ctl_socket_getsockname(argblock),
            OBJECT_OP_SOCKET_GETSOCKOPT => self.object_ctl_socket_getsockopt(argblock),
            OBJECT_OP_SOCKET_SETSOCKOPT => self.object_ctl_socket_setsockopt(argblock),
            _ => Err(22),
        };
        match value {
            Ok(value) => {
                self.set_errno(0)?;
                self.write_reg(result, value)
            }
            Err(errno) => {
                self.set_errno(errno)?;
                self.write_reg(result, -1i64 as u64)
            }
        }
    }

    fn dma_ctl(&mut self, result: Reg, argblock: u64) -> Result<(), String> {
        if !self.current_domain_dma_allowed()? {
            self.set_status_errno(1)?;
            self.write_reg(result, -1i64 as u64)?;
            return Ok(());
        }
        let op = self.load_u64(argblock)?;
        let dst = self.load_u64(argblock + 8)?;
        let src_or_value = self.load_u64(argblock + 16)?;
        let len = self.load_u64(argblock + 24)? as usize;
        let dma_buffer = self.load_u64(argblock + 32)?;
        if dma_buffer != 0 {
            let validation = self.validate_dma_buffer(dma_buffer, op, dst, src_or_value, len);
            if let Err(errno) = validation {
                self.set_status_errno(errno)?;
                self.write_reg(result, -1i64 as u64)?;
                return Ok(());
            }
        }
        let outcome: Result<u64, u64> = match op {
            DMA_OP_COPY => self
                .read_bytes(src_or_value, len)
                .map_err(|_| 14u64)
                .and_then(|bytes| {
                    self.write_bytes(dst, &bytes)
                        .map(|_| len as u64)
                        .map_err(|_| 14u64)
                }),
            DMA_OP_FILL => {
                let bytes = vec![src_or_value as u8; len];
                self.write_bytes(dst, &bytes)
                    .map(|_| len as u64)
                    .map_err(|_| 14u64)
            }
            _ => Err(22),
        };
        match outcome {
            Ok(count) => {
                self.set_errno(0)?;
                self.write_reg(result, count)
            }
            Err(errno) => {
                self.set_status_errno(errno)?;
                self.write_reg(result, -1i64 as u64)
            }
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
        let channel_value = self.load_u64(argblock)?;
        let src_value = self.load_u64(argblock + 8)?;
        let flags = self.load_u64(argblock + 24)?;
        let value = self.cap_send_inner(channel_value, src_value, flags);
        match value {
            Ok(count) => {
                self.set_errno(0)?;
                self.write_reg(result, count)
            }
            Err(errno) => {
                self.set_status_errno(errno)?;
                self.write_reg(result, -1i64 as u64)
            }
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

        queue.borrow_mut().capabilities.push_back(payload);
        if flags & CAP_SEND_FLAG_MOVE != 0 {
            self.close_fd_index(src).map_err(|_| 9u64)?;
        }
        Ok(1)
    }

    fn cap_recv(&mut self, result: Reg, argblock: u64) -> Result<(), String> {
        let channel_value = self.load_u64(argblock)?;
        let dst_req = self.load_u64(argblock + 8)?;
        let rights_req = self.load_u64(argblock + 16)?;
        let flags = self.load_u64(argblock + 24)?;
        let value = self.cap_recv_inner(channel_value, dst_req, rights_req, flags);
        match value {
            Ok(token) => {
                self.set_errno(0)?;
                self.write_reg(result, token)
            }
            Err(errno) => {
                self.set_status_errno(errno)?;
                self.write_reg(result, -1i64 as u64)
            }
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

        let mut payload = queue.borrow_mut().capabilities.pop_front().ok_or(11u64)?;
        payload.capability.rights = rights;
        payload.capability.narrowable = payload.capability.narrowable && !payload.capability.sealed;
        self.install_fd_capability(dst, payload.capability)
            .map_err(|_| 9u64)?;
        self.bump_fd_generation(dst).map_err(|_| 9u64)?;
        self.process_mut().map_err(|_| 3u64)?.fds[dst] = payload.handle;
        self.fd_token(dst).map_err(|_| 9u64)
    }

    fn cap_dup(&mut self, result: Reg, argblock: u64) -> Result<(), String> {
        let src_value = self.load_u64(argblock)?;
        let dst_req = self.load_u64(argblock + 8)?;
        let rights_req = self.load_u64(argblock + 16)?;
        let flags = self.load_u64(argblock + 24)?;
        let value = self.cap_dup_inner(src_value, dst_req, rights_req, flags);
        match value {
            Ok(token) => {
                self.set_errno(0)?;
                self.write_reg(result, token)
            }
            Err(errno) => {
                self.set_status_errno(errno)?;
                self.write_reg(result, -1i64 as u64)
            }
        }
    }

    fn cap_dup_inner(
        &mut self,
        src_value: u64,
        dst_req: u64,
        rights_req: u64,
        flags: u64,
    ) -> Result<u64, u64> {
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
        self.duplicate_fd_capability(src, dst, rights, sealed)?;
        self.bump_fd_generation(dst).map_err(|_| 9u64)?;
        self.process_mut().map_err(|_| 3u64)?.fds[dst] = handle;
        self.fd_token(dst).map_err(|_| 9u64)
    }

    fn cap_revoke(&mut self, result: Reg, argblock: u64) -> Result<(), String> {
        let src_value = self.load_u64(argblock)?;
        let value = self.cap_revoke_inner(src_value);
        match value {
            Ok(count) => {
                self.set_errno(0)?;
                self.write_reg(result, count)
            }
            Err(errno) => {
                self.set_status_errno(errno)?;
                self.write_reg(result, -1i64 as u64)
            }
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

    fn object_ctl_create(&mut self, argblock: u64) -> Result<u64, u64> {
        self.require_domain_cap_errno(DOMAIN_CAP_OBJECT | DOMAIN_CAP_FDR)?;
        let kind = self.load_u64(argblock + 8).map_err(|_| 14u64)?;
        let profile = self.load_u64(argblock + 16).map_err(|_| 14u64)?;
        let fd0_req = self.load_u64(argblock + 24).map_err(|_| 14u64)?;
        let fd1_req = self.load_u64(argblock + 32).map_err(|_| 14u64)?;
        let arg = self.load_u64(argblock + 40).map_err(|_| 14u64)?;
        match (kind, profile) {
            (OBJECT_KIND_QUEUE, OBJECT_PROFILE_PIPE) => {
                let buffer = Rc::new(RefCell::new(PipeBuffer::default()));
                let read_fd =
                    self.install_object_fd(fd0_req, FdHandle::PipeReader(Rc::clone(&buffer)))?;
                let write_fd = self.install_object_fd(fd1_req, FdHandle::PipeWriter(buffer))?;
                self.store_u64(argblock + 24, read_fd as u64)
                    .map_err(|_| 14u64)?;
                self.store_u64(argblock + 32, write_fd as u64)
                    .map_err(|_| 14u64)?;
                Ok(0)
            }
            (OBJECT_KIND_QUEUE, OBJECT_PROFILE_CALL_GATE) => {
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
                let mode = self.load_u64(argblock + 48).map_err(|_| 14u64)?;
                let completion_fd = self.load_u64(argblock + 56).map_err(|_| 14u64)?;
                let flags = self.load_u64(argblock + 64).map_err(|_| 14u64)?;
                if !matches!(mode, CALL_MODE_SYNC | CALL_MODE_ASYNC | CALL_MODE_HANDOFF) {
                    return Err(22);
                }
                let completion_fd = if completion_fd == 0 {
                    None
                } else if completion_fd as usize >= FDR_COUNT {
                    return Err(9);
                } else {
                    Some(completion_fd as usize)
                };
                if mode == CALL_MODE_ASYNC && completion_fd.is_none() {
                    return Err(22);
                }
                let fd = self.install_object_fd(
                    fd0_req,
                    FdHandle::CallGate {
                        entry: arg as usize,
                        domain_id: target_domain,
                        domain_generation: target_generation,
                        mode,
                        completion_fd,
                        flags,
                    },
                )?;
                self.store_u64(argblock + 24, fd as u64)
                    .map_err(|_| 14u64)?;
                Ok(fd as u64)
            }
            (OBJECT_KIND_COUNTER, _) => {
                let fd =
                    self.install_object_fd(fd0_req, FdHandle::Counter(Rc::new(RefCell::new(arg))))?;
                self.store_u64(argblock + 24, fd as u64)
                    .map_err(|_| 14u64)?;
                Ok(fd as u64)
            }
            (OBJECT_KIND_MEMORY_OBJECT, _) => {
                let len = arg.max(1) as usize;
                let fd = self.install_object_fd(
                    fd0_req,
                    FdHandle::MemoryObject {
                        data: Rc::new(RefCell::new(vec![0; len])),
                        pos: 0,
                    },
                )?;
                self.store_u64(argblock + 24, fd as u64)
                    .map_err(|_| 14u64)?;
                Ok(fd as u64)
            }
            (OBJECT_KIND_TIMER, _) => {
                let fd = self.install_object_fd(
                    fd0_req,
                    FdHandle::Timer(Rc::new(RefCell::new(TimerState::default()))),
                )?;
                self.store_u64(argblock + 24, fd as u64)
                    .map_err(|_| 14u64)?;
                Ok(fd as u64)
            }
            (OBJECT_KIND_DMA_BUFFER, _) => {
                let len = self.load_u64(argblock + 48).map_err(|_| 14u64)?.max(1);
                self.ensure_mapped(arg, len as usize, false)
                    .map_err(|_| 14u64)?;
                self.ensure_mapped(arg, len as usize, true)
                    .map_err(|_| 14u64)?;
                let fd = self.install_object_fd(fd0_req, FdHandle::DmaBuffer { addr: arg, len })?;
                self.store_u64(argblock + 24, fd as u64)
                    .map_err(|_| 14u64)?;
                Ok(fd as u64)
            }
            (OBJECT_KIND_ENDPOINT, OBJECT_PROFILE_TCP_STREAM) => {
                let sock_type = self.load_u64(argblock + 48).map_err(|_| 14u64)?;
                let protocol = self.load_u64(argblock + 56).map_err(|_| 14u64)?;
                let fd = self.install_object_fd(
                    fd0_req,
                    FdHandle::TcpSocket {
                        domain: arg,
                        sock_type,
                        protocol,
                        bound_addr: None,
                    },
                )?;
                self.store_u64(argblock + 24, fd as u64)
                    .map_err(|_| 14u64)?;
                Ok(fd as u64)
            }
            _ => Err(22),
        }
    }

    fn object_ctl_socket_bind(&mut self, argblock: u64) -> Result<u64, u64> {
        let fd_value = self.load_u64(argblock + 24).map_err(|_| 14u64)?;
        let addr_ptr = self.load_u64(argblock + 40).map_err(|_| 14u64)?;
        let fd = self.decode_fd_value(fd_value)?;
        self.fd_right_errno(fd, CAP_RIGHT_WRITE)?;
        let addr = self.read_c_string(addr_ptr).map_err(|_| 14u64)?;
        match &mut self.process_mut().map_err(|_| 3u64)?.fds[fd] {
            FdHandle::TcpSocket { bound_addr, .. } => {
                *bound_addr = Some(addr);
                Ok(0)
            }
            _ => Err(22),
        }
    }

    fn object_ctl_socket_listen(&mut self, argblock: u64) -> Result<u64, u64> {
        let fd_value = self.load_u64(argblock + 24).map_err(|_| 14u64)?;
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
        let fd_value = self.load_u64(argblock + 24).map_err(|_| 14u64)?;
        let addr_ptr = self.load_u64(argblock + 40).map_err(|_| 14u64)?;
        let fd = self.decode_fd_value(fd_value)?;
        self.fd_right_errno(fd, CAP_RIGHT_READ | CAP_RIGHT_WRITE | CAP_RIGHT_POLL)?;
        if !matches!(
            self.process().map_err(|_| 3u64)?.fds.get(fd),
            Some(FdHandle::TcpSocket { .. })
        ) {
            return Err(22);
        }
        let addr = self.read_c_string(addr_ptr).map_err(|_| 14u64)?;
        let stream = TcpStream::connect(&addr).map_err(|_| 111u64)?;
        stream.set_nonblocking(true).map_err(|_| 5u64)?;
        self.process_mut().map_err(|_| 3u64)?.fds[fd] = FdHandle::TcpStream(stream);
        self.bump_fd_generation(fd).map_err(|_| 9u64)?;
        Ok(0)
    }

    fn object_ctl_socket_accept(&mut self, argblock: u64) -> Result<u64, u64> {
        let listener_value = self.load_u64(argblock + 24).map_err(|_| 14u64)?;
        let accepted_req = self.load_u64(argblock + 32).map_err(|_| 14u64)?;
        let listener_fd = self.decode_fd_value(listener_value)?;
        self.fd_right_errno(listener_fd, CAP_RIGHT_READ | CAP_RIGHT_POLL)?;
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
        let accepted_fd = self.install_object_fd(accepted_req, FdHandle::TcpStream(stream))?;
        self.store_u64(argblock + 32, accepted_fd as u64)
            .map_err(|_| 14u64)?;
        Ok(accepted_fd as u64)
    }

    fn object_ctl_socket_getsockname(&mut self, argblock: u64) -> Result<u64, u64> {
        let fd_value = self.load_u64(argblock + 24).map_err(|_| 14u64)?;
        let addr_ptr = self.load_u64(argblock + 40).map_err(|_| 14u64)?;
        let len_ptr = self.load_u64(argblock + 48).map_err(|_| 14u64)?;
        let fd = self.decode_fd_value(fd_value)?;
        self.fd_right_errno(fd, CAP_RIGHT_STAT)?;
        let addr = match &self.process().map_err(|_| 3u64)?.fds[fd] {
            FdHandle::TcpListener { listener, .. } => listener.local_addr().map_err(|_| 5u64)?,
            FdHandle::TcpStream(stream) => stream.local_addr().map_err(|_| 5u64)?,
            _ => return Err(22),
        };
        let mut bytes = addr.to_string().into_bytes();
        bytes.push(0);
        if len_ptr != 0 {
            let capacity = self.load_u64(len_ptr).map_err(|_| 14u64)?;
            if capacity < bytes.len() as u64 {
                return Err(22);
            }
            self.store_u64(len_ptr, bytes.len() as u64)
                .map_err(|_| 14u64)?;
        }
        self.write_bytes(addr_ptr, &bytes).map_err(|_| 14u64)?;
        Ok(0)
    }

    fn object_ctl_socket_getsockopt(&mut self, argblock: u64) -> Result<u64, u64> {
        let fd_value = self.load_u64(argblock + 24).map_err(|_| 14u64)?;
        let optval = self.load_u64(argblock + 56).map_err(|_| 14u64)?;
        let optlen = self.load_u64(argblock + 64).map_err(|_| 14u64)?;
        let fd = self.decode_fd_value(fd_value)?;
        self.fd_right_errno(fd, CAP_RIGHT_STAT)?;
        self.ensure_socket_fd(fd)?;
        if optlen != 0 {
            let capacity = self.load_u64(optlen).map_err(|_| 14u64)?;
            if capacity < 8 {
                return Err(22);
            }
            self.store_u64(optlen, 8).map_err(|_| 14u64)?;
        }
        if optval != 0 {
            self.store_u64(optval, 0).map_err(|_| 14u64)?;
        }
        Ok(0)
    }

    fn object_ctl_socket_setsockopt(&mut self, argblock: u64) -> Result<u64, u64> {
        let fd_value = self.load_u64(argblock + 24).map_err(|_| 14u64)?;
        let optval = self.load_u64(argblock + 56).map_err(|_| 14u64)?;
        let optlen = self.load_u64(argblock + 64).map_err(|_| 14u64)?;
        let fd = self.decode_fd_value(fd_value)?;
        self.fd_right_errno(fd, CAP_RIGHT_WRITE)?;
        self.ensure_socket_fd(fd)?;
        if optval != 0 && optlen != 0 {
            self.ensure_mapped(optval, optlen as usize, false)
                .map_err(|_| 14u64)?;
        }
        Ok(0)
    }

    fn ensure_socket_fd(&self, fd: usize) -> Result<(), u64> {
        match self.process().map_err(|_| 3u64)?.fds.get(fd) {
            Some(FdHandle::TcpSocket { .. })
            | Some(FdHandle::TcpListener { .. })
            | Some(FdHandle::TcpStream(_)) => Ok(()),
            _ => Err(22),
        }
    }

    fn install_object_fd(&mut self, requested: u64, handle: FdHandle) -> Result<usize, u64> {
        if requested != 0 {
            if requested as usize >= FDR_COUNT || requested as usize == MESSAGE_ENDPOINT_FD {
                return Err(9);
            }
            let fd = requested as usize;
            let delta = self.fd_slot_delta(fd).map_err(|_| 9u64)?;
            self.ensure_domain_budget_errno(0, 0, 0, delta)?;
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
        self.require_domain_cap(DOMAIN_CAP_CALL)?;
        if self.ensure_fd_right(call_gate_fd, CAP_RIGHT_CALL).is_err() {
            self.write_reg(result, -1i64 as u64)?;
            return Ok(());
        }
        let (entry, domain_id, domain_generation, mode, completion_fd, flags) =
            match &self.process()?.fds[call_gate_fd] {
                FdHandle::CallGate {
                    entry,
                    domain_id,
                    domain_generation,
                    mode,
                    completion_fd,
                    flags,
                } => (
                    *entry,
                    *domain_id,
                    *domain_generation,
                    *mode,
                    *completion_fd,
                    *flags,
                ),
                _ => {
                    self.set_status_errno(9)?;
                    self.write_reg(result, -1i64 as u64)?;
                    return Ok(());
                }
            };
        if self.domain_ref(domain_id, domain_generation).is_err() {
            self.set_status_errno(116)?;
            self.write_reg(result, -1i64 as u64)?;
            return Ok(());
        }
        if (arg0 & CALL_ARG_CAP_MARKER != 0 || arg1 & CALL_ARG_CAP_MARKER != 0)
            && flags & CALL_GATE_FLAG_CAP_PASS == 0
        {
            self.set_status_errno(1)?;
            self.write_reg(result, -1i64 as u64)?;
            return Ok(());
        }
        if self.domain_is_frozen_or_destroyed(domain_id) {
            self.set_status_errno(11)?;
            self.write_reg(result, -1i64 as u64)?;
            return Ok(());
        }
        if !self.check_call_cpu_budget(domain_id)? {
            self.write_reg(result, -1i64 as u64)?;
            return Ok(());
        }
        match mode {
            CALL_MODE_SYNC => self.call_cap_sync(result, entry, domain_id, arg0, arg1),
            CALL_MODE_ASYNC => self.call_cap_async(result, completion_fd, arg0, arg1),
            CALL_MODE_HANDOFF => self.call_cap_handoff(result, entry, domain_id, arg0, arg1),
            _ => {
                self.set_status_errno(22)?;
                self.write_reg(result, -1i64 as u64)?;
                return Ok(());
            }
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
            self.set_status_errno(11)?;
            self.write_reg(result, -1i64 as u64)?;
            return Ok(());
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
        completion_fd: Option<usize>,
        arg0: u64,
        arg1: u64,
    ) -> Result<(), String> {
        let op_id = self.next_call_op_id;
        self.next_call_op_id = self.next_call_op_id.saturating_add(1);
        if let Some(fd) = completion_fd {
            self.complete_call_fd(fd, op_id, arg0, arg1)?;
        }
        self.set_errno(0)?;
        self.write_reg(result, op_id)
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
        self.set_errno(0)?;
        self.write_reg(result, 0)?;
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
        match &mut self.process_mut()?.fds[fd] {
            FdHandle::Counter(value) => {
                *value.borrow_mut() = op_id;
                Ok(())
            }
            FdHandle::PipeWriter(queue) => {
                let mut queue = queue.borrow_mut();
                queue.bytes.extend(op_id.to_le_bytes());
                queue.bytes.extend(value0.to_le_bytes());
                queue.bytes.extend(value1.to_le_bytes());
                Ok(())
            }
            _ => {
                self.set_status_errno(9)?;
                Err("CALL_CAP async completion target is not waitable".to_string())
            }
        }
    }

    fn ret_cap(&mut self, result: Reg, value0: u64, value1: u64) -> Result<(), String> {
        let Some(continuation) = self.thread_mut()?.cap_call_stack.pop() else {
            self.set_status_errno(22)?;
            self.write_reg(result, -1i64 as u64)?;
            return Ok(());
        };
        if self.domain_is_frozen_or_destroyed(continuation.caller_domain_id) {
            self.set_status_errno(116)?;
            self.write_reg(result, -1i64 as u64)?;
            return Ok(());
        }
        let Some(caller) = self.domains.get(&continuation.caller_domain_id) else {
            self.set_status_errno(116)?;
            self.write_reg(result, -1i64 as u64)?;
            return Ok(());
        };
        if caller.capability_mask & DOMAIN_CAP_CALL == 0 {
            self.set_status_errno(1)?;
            self.write_reg(result, -1i64 as u64)?;
            return Ok(());
        }
        self.process_mut()?.domain_id = continuation.caller_domain_id;
        self.thread_mut()?.ip = continuation.return_ip;
        self.set_errno(0)?;
        self.write_reg(continuation.result_reg, value0)?;
        self.write_reg(Reg(30), value1)?;
        self.write_reg(result, 0)
    }

    fn domain_ctl(&mut self, result: Reg, argblock: u64) -> Result<(), String> {
        let op = self.load_u64(argblock)?;
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
            Ok(value) => {
                self.set_errno(0)?;
                self.write_reg(result, value)
            }
            Err(errno) => {
                self.set_errno(errno)?;
                self.write_reg(result, -1i64 as u64)
            }
        }
    }

    fn domain_ctl_create(&mut self, argblock: u64) -> Result<u64, u64> {
        let parent_id = self.domain_arg_id(argblock)?;
        let parent_generation = self.load_u64(argblock + 16).map_err(|_| 14u64)?;
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
        let requested_cpu = self.load_u64(argblock + 32).map_err(|_| 14u64)?;
        let requested_memory = self.load_u64(argblock + 40).map_err(|_| 14u64)?;
        let requested_pids = self.load_u64(argblock + 48).map_err(|_| 14u64)?;
        let requested_fdrs = self.load_u64(argblock + 56).map_err(|_| 14u64)?;
        let profile = self.load_u64(argblock + 24).map_err(|_| 14u64)?;
        let requested_caps = self.load_u64(argblock + 64).map_err(|_| 14u64)?;
        let requested_upcalls = self.load_u64(argblock + 72).map_err(|_| 14u64)?;
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
        self.store_u64(argblock + 8, id).map_err(|_| 14u64)?;
        self.store_u64(argblock + 16, 1).map_err(|_| 14u64)?;
        self.store_u64(argblock + 120, parent_id)
            .map_err(|_| 14u64)?;
        self.store_u64(argblock + 128, parent_depth + 1)
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
        let requested_cpu = self.load_u64(argblock + 32).map_err(|_| 14u64)?;
        let requested_memory = self.load_u64(argblock + 40).map_err(|_| 14u64)?;
        let requested_pids = self.load_u64(argblock + 48).map_err(|_| 14u64)?;
        let requested_fdrs = self.load_u64(argblock + 56).map_err(|_| 14u64)?;
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
        let profile = self.load_u64(argblock + 24).map_err(|_| 14u64)?;
        let caps = self.load_u64(argblock + 64).map_err(|_| 14u64)?;
        let upcalls = self.load_u64(argblock + 72).map_err(|_| 14u64)?;

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
            self.store_u64(argblock + offset, value)
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
        let id = self.load_u64(argblock + 8).map_err(|_| 14u64)?;
        if id == 0 {
            self.current_domain_id().map_err(|_| 3u64)
        } else {
            Ok(id)
        }
    }

    fn domain_ref_from_arg(&mut self, argblock: u64) -> Result<u64, u64> {
        let id = self.domain_arg_id(argblock)?;
        let generation = self.load_u64(argblock + 16).map_err(|_| 14u64)?;
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
            self.load_u64(argblock + DOMAIN_SECURITY_ASLR_ENABLED)
                .map_err(|_| 14u64)?,
            current.aslr_enabled,
        )?;
        let allow_wx = Self::decode_domain_bool(
            self.load_u64(argblock + DOMAIN_SECURITY_ALLOW_WX)
                .map_err(|_| 14u64)?,
            current.allow_wx,
        )?;
        let allow_jit_transition = Self::decode_domain_bool(
            self.load_u64(argblock + DOMAIN_SECURITY_ALLOW_JIT_TRANSITION)
                .map_err(|_| 14u64)?,
            current.allow_jit_transition,
        )?;
        let entropy_quota = match self
            .load_u64(argblock + DOMAIN_SECURITY_ENTROPY_QUOTA)
            .map_err(|_| 14u64)?
        {
            0 => current.entropy_quota,
            quota => quota,
        };
        let dma_allowed = Self::decode_domain_bool(
            self.load_u64(argblock + DOMAIN_SECURITY_DMA_ALLOWED)
                .map_err(|_| 14u64)?,
            current.dma_allowed,
        )?;
        let hardening_profile = match self
            .load_u64(argblock + DOMAIN_SECURITY_HARDENING_PROFILE)
            .map_err(|_| 14u64)?
        {
            0 => current.hardening_profile,
            profile => profile,
        };
        let executable_source_policy = match self
            .load_u64(argblock + DOMAIN_SECURITY_EXEC_SOURCE_POLICY)
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
        let key = self.read_reg(key_reg)?;
        let index_or_buf = self.read_reg(index_or_buf_reg)?;
        let len_or_flags = self.read_reg(len_or_flags_reg)?;
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
                    | ENV_HWCAP0_FUTEX,
            ),
            ENV_KEY_HWCAP1 => Some(0),
            ENV_KEY_ARCH_THREAD_LIMIT => Some(ENV_THREAD_LIMIT),
            ENV_KEY_PROCESS_LIMIT => Some(ENV_PROCESS_LIMIT),
            ENV_KEY_DEFAULT_FDR_LIMIT => Some(FDR_COUNT as u64),
            ENV_KEY_EVENT_QUEUE_LIMIT => Some(ENV_EVENT_QUEUE_LIMIT),
            ENV_KEY_FUTEX_BUCKET_COUNT => Some(ENV_FUTEX_BUCKET_COUNT),
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
            _ => None,
        };

        if let Some(value) = scalar {
            self.set_errno(0)?;
            self.write_reg(result, value)
        } else {
            self.set_status_errno(22)?;
            self.write_reg(result, -1i64 as u64)
        }
    }

    fn env_argc(&mut self) -> Result<u64, String> {
        self.load_u64(ARG_BASE)
    }

    fn env_envp_base(&mut self) -> Result<u64, String> {
        Ok(ARG_BASE + 8 + (self.env_argc()?.saturating_add(1) * 8))
    }

    fn env_auxv_base(&mut self) -> Result<u64, String> {
        Ok(self.env_envp_base()? + ((self.env_count()? + 1) * 8))
    }

    fn env_count(&mut self) -> Result<u64, String> {
        let envp = self.env_envp_base()?;
        for idx in 0..256u64 {
            if self.load_u64(envp + idx * 8)? == 0 {
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
        if self.write_bytes(buf, &record[..count]).is_err() {
            self.set_status_errno(14)?;
            self.write_reg(result, -1i64 as u64)?;
            return Ok(());
        }
        self.set_errno(0)?;
        self.write_reg(result, count as u64)
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
                    .saturating_add(process.vmas.iter().map(|vma| vma.len).sum::<u64>());
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
        usage.memory = process.vmas.iter().map(|vma| vma.len).sum::<u64>();
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
            if let Some(file) = &mut process.vmas[idx].file {
                file.seek(SeekFrom::Start(file_offset))
                    .map_err(|err| format!("file-backed VMA seek failed: {err}"))?;
                let mut tmp = vec![0; vma_len as usize];
                let count = file
                    .read(&mut tmp)
                    .map_err(|err| format!("file-backed VMA page-in failed: {err}"))?;
                let start = start as usize;
                process.memory[start..start + count].copy_from_slice(&tmp[..count]);
            }
            process.vmas[idx].resident = true;
        }
        Ok(())
    }

    fn instruction_fetch_fault(&self, addr: u64) -> Result<Option<String>, String> {
        let process = self.process()?;
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
            pos += 1;
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
            let ptr = self.load_u64(vector + idx * 8)?;
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
            Pcr::Pid | Pcr::Ppid | Pcr::Tid | Pcr::RealtimeSec | Pcr::RealtimeNsec => {
                Err("selected PCR is read-only".to_string())
            }
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
        self.last_exit = code;
        if !self.threads.values().any(|thread| thread.pid == pid) {
            self.processes.remove(&pid);
            if let Some(parent_pid) = parent_pid {
                if let Some(parent) = self.processes.get_mut(&parent_pid) {
                    parent.pending_signals.push_back(SIGCHLD);
                }
            }
        }
        Ok(())
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
            self.raise_process_signal(pid, SIGALRM);
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

    fn raise_process_signal(&mut self, pid: u64, signum: u64) {
        if let Some(process) = self.processes.get_mut(&pid) {
            process.pending_signals.push_back(signum);
            if let Some(tid) = self
                .threads
                .values()
                .find(|thread| thread.pid == pid)
                .map(|thread| thread.tid)
            {
                self.wake_thread(tid);
            }
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
                    if machine.fd_ready_for_mask(waiter.fd, waiter.mask)? {
                        Ok(FdWaiterState::Ready)
                    } else {
                        Ok(FdWaiterState::Pending)
                    }
                })
                .unwrap_or(FdWaiterState::Stale);
            match state {
                FdWaiterState::Ready => self.wake_thread(waiter.tid),
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
            | FdHandle::Closed => Ok(false),
        }
    }

    fn fd_write_ready(&self, fd: usize) -> Result<bool, String> {
        let handle = &self.process()?.fds[fd];
        Ok(matches!(
            handle,
            FdHandle::Stdout
                | FdHandle::Stderr
                | FdHandle::File(_)
                | FdHandle::PipeWriter(_)
                | FdHandle::Counter(_)
                | FdHandle::MemoryObject { .. }
                | FdHandle::Timer(_)
                | FdHandle::TcpStream(_)
        ))
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
            | FdHandle::PipeWriter(_)
            | FdHandle::Counter(_)
            | FdHandle::MemoryObject { .. }
            | FdHandle::CallGate { .. }
            | FdHandle::TcpStream(_) => Ok(true),
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
        self.process_mut()?.pending_signals.push_back(signum);
        Ok(())
    }

    fn deliver_signal_if_needed(&mut self) -> Result<(), String> {
        let pid = self.thread()?.pid;
        let signum = {
            let Some(process) = self.processes.get_mut(&pid) else {
                return Ok(());
            };
            let Some(pos) = process
                .pending_signals
                .iter()
                .position(|sig| process.sigmask & (1u64 << sig.min(&63)) == 0)
            else {
                return Ok(());
            };
            process.pending_signals.remove(pos)
        };
        let Some(signum) = signum else {
            return Ok(());
        };
        let handler = self.process()?.signal_handlers.get(&signum).copied();
        if let Some(handler) = handler {
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
        } else {
            if signum != SIGCHLD {
                self.exit_current(128 + signum as i32)?;
            }
        }
        Ok(())
    }

    fn load_microcode(&mut self, blob: &[u8]) -> Result<(), String> {
        let text = String::from_utf8_lossy(blob);
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
            self.process_mut()?.ucode_ports.insert(port, value as u8);
        }
        Ok(())
    }
}

fn align_up(value: u64, align: u64) -> u64 {
    (value + align - 1) & !(align - 1)
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
        assert_eq!(machine.thread().unwrap().regs[1], 0);
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
    }

    #[test]
    fn sealed_capability_cannot_be_duplicated() {
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

        machine.store_u64(arg, sealed).unwrap();
        machine.store_u64(arg + 8, 0).unwrap();
        machine.store_u64(arg + 16, 0).unwrap();
        machine.store_u64(arg + 24, 0).unwrap();
        machine.cap_dup(Reg(5), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[5], -1i64 as u64);
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
        assert_eq!(machine.thread().unwrap().regs[1], 0);
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
        assert_eq!(machine.thread().unwrap().regs[1], 0);
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

        machine.thread_mut().unwrap().regs[3] = 1;
        machine.exec(Instr::Random(Reg(5), Reg(2), Reg(3))).unwrap();
        assert_eq!(machine.thread().unwrap().regs[5], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
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
        machine.store_u64(arg + 8, ARG_BASE + 0x1000).unwrap();
        machine.dma_ctl(Reg(6), arg).unwrap();
        assert_eq!(machine.thread().unwrap().regs[6], -1i64 as u64);
        assert_eq!(machine.process().unwrap().errno, 1);
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
        machine.store_u64(arg + 8, OBJECT_KIND_DMA_BUFFER).unwrap();
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
        machine.store_u64(arg + 8, OBJECT_KIND_DMA_BUFFER).unwrap();
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
        machine.store_u64(arg + 8, OBJECT_KIND_DMA_BUFFER).unwrap();
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
    fn alloc_ex_creates_and_frees_guard_regions() {
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
        machine.store_u64(arg + 8, OBJECT_KIND_COUNTER).unwrap();
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
            flags: 0,
        };
        machine.processes.get_mut(&1).unwrap().fd_capabilities[3] = FdCapability::full(3);
        machine
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
            flags: 0,
        };
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

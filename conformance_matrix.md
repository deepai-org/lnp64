# LNP64 POSIX and Libc Conformance Matrix

This matrix is the compatibility ledger for the emulator, C compiler, and libc
shim surface. It should be updated whenever a real package exposes a missing
runtime, libc, ISA, or ABI behavior.

Status values:

- `implemented`: the API lowers to LNP64 instructions or emulator primitives.
- `tested`: the API has a repository test or scripted real-program smoke test.
- `partial`: useful subset exists, but known POSIX/libc behavior is missing.
- `unsupported`: intentionally outside the current v1 compatibility surface.
- `native extension`: LNP64-specific API or instruction surface.

| API or Scope | Status | Evidence | Known Gaps / Compatibility Bugs |
| --- | --- | --- | --- |
| `_start`, `main(argc, argv, envp)`, `environ` | tested | `c_start_symbol_overrides_main_entry`, `c_main_receives_argc_argv_and_envp_from_startup_page`, `c_main_environ_points_at_startup_envp` | No standalone crt object files yet; startup is compiler/runtime modeled. `COMPAT-ABI-001` tracks psABI/crt packaging. |
| `getauxval`, auxv metadata, process entry metadata | tested | `c_getauxval_startup_metadata_surface_runs`, `ENV_GET` emulator tests | Dynamic loader auxv contract is not frozen. Covered by `COMPAT-ABI-002`. |
| TLS/thread pointer, `errno`, `strerror` | tested | `c_thread_pointer_and_specific_storage_are_per_thread`, `c_errno_location_shim_tracks_hardware_errno`, `c_strerror_returns_static_errno_messages` | TLS layout needs psABI text. |
| `atexit`, `exit`, `_exit` | tested | `c_atexit_handlers_run_before_main_return`, `c_exit_runs_atexit_but_exit_bypasses_it` | No shared-object destructor model. |
| Environment APIs: `getenv`, `setenv`, `unsetenv` | tested | `c_environment_surface_stores_and_finds_values` | Locale/environment inheritance across `exec` needs more real-program coverage. |
| Basic file descriptors: `open`, `openat`-style lowering, `close`, `read`, `write`, `lseek` | tested | `lowers_file_builtins_to_fd_instructions`, `scripts/run_sbase.sh` | Host-backed file model, not a boot image VFS yet. |
| Positioned and vectored I/O: `pread`, `pwrite`, `readv`, `writev` | tested | `c_readv_writev_surface_uses_dynamic_fdr_io`, emulator fd dispatch tests | Scatter/gather edge cases need real package coverage. |
| Descriptor duplication: `dup`, `dup2`, `fcntl`-style capability duplication | tested | `c_posix_descriptor_dup_surface_runs_on_cap_dup`, dynamic FDR token tests | Full `fcntl` command matrix is partial. `COMPAT-LIBC-001`. |
| Metadata and filesystem mutation: `stat`, `fstat`, `chmod`, `chown`, `link`, `symlink`, `mkdir`, `unlink`, `rename`, `utime`/`touch` | partial | `scripts/run_sbase.sh` covers `chmod`, `chown`, `ln`, `mkdir`, `rm`, `mv`, `touch`, `find`, `ls` | Symlink/path corner cases, permissions, hard-link errors, and full `stat` fields need conformance tests. `COMPAT-FS-001`. |
| Directory iteration: `opendir`, `readdir`, `rewinddir`, `closedir` | tested | `c_directory_iteration_surface_reads_entries`, `scripts/run_sbase.sh` (`ls`, `find`) | Concurrent directory mutation semantics are not specified. |
| stdio streams: `fopen`, `freopen`, `fread`, `fwrite`, `fgets`, `fprintf`, `fflush`, `fclose`, `fseek`, `ftell`, `tmpfile`, `setvbuf` | tested | `c_fgets_reads_lines_from_descriptor_stream`, `c_fprintf_writes_formatted_output_to_descriptor_stream`, `c_stdio_*`, `c_tmpfile_returns_read_write_unlinked_stream`, `c_freopen_replaces_descriptor_stream` | Buffered stream error flags and wide I/O are partial. `COMPAT-STDIO-001`. |
| Formatting and string/memory helpers: `printf` subset, `snprintf` subset, `str*`, `mem*`, `ctype` | partial | Lua portability/string tests, `c_standard_ctype_surface_runs`, sbase/jsmn scripts | Full format grammar, floating formatting, locale collation, and wide chars are partial. `COMPAT-LIBC-002`. |
| Math/libm used by real packages | partial | `c_libm_integer_model_shims_run` | Integer-model shims only; real floating libm remains incomplete. `COMPAT-LIBM-001`. |
| Allocation: `malloc`, `free`, `realloc`, `calloc`, `aligned_alloc`, `posix_memalign` | tested | `c_allocator_libc_surface_uses_native_heap_metadata`, allocator demos | Multi-threaded allocator stress is not yet in the suite. `COMPAT-STRESS-004`. |
| `brk`, `sbrk` | tested | `c_brk_sbrk_compat_surface_uses_native_heap` | Compatibility cursor is modeled over native allocations, not a contiguous Unix heap. |
| `mmap`, `munmap`, `mprotect` | tested | `c_mmap_mprotect_and_munmap_surface_runs`, VMA security tests | File-backed shared mapping coherence needs real package coverage. `COMPAT-MM-001`. |
| W^X, NX, guard pages, ASLR, `RANDOM` | tested | `wx_mmap_and_mprotect_follow_domain_policy`, `nx_and_guard_instruction_fetches_fault`, `heap_and_anonymous_mmap_use_aslr_layout`, `random_scalar_and_buffer_are_deterministic` | Broader randomized mmap/munmap/mprotect fuzzing is missing. `COMPAT-STRESS-003`. |
| `getentropy`, `getrandom`, `arc4random`, `arc4random_buf` | tested | `c_entropy_surface_lowers_to_random_instruction`, `random_obeys_domain_entropy_quota` | Cryptographic entropy source is intentionally not modeled; deterministic emulator RNG is for architecture behavior. |
| `pthread_create`, `pthread_join`, `pthread_self`, thread-specific storage | tested | `c_thread_pointer_and_specific_storage_are_per_thread`, pthread tests | `pthread_detach`, cancellation, robust mutexes, and scheduler fairness are partial. `COMPAT-PTHREAD-001`. |
| Mutexes, condvars, rwlocks, `pthread_once`, semaphores | tested | `c_pthread_mutex_condvar_surface_runs_on_futex_primitives`, `c_semaphore_and_once_surface_runs_on_futex_primitives`, `c_rwlock_surface_runs_on_futex_primitives` | Timed waits and process-shared behavior are partial. `COMPAT-PTHREAD-002`. |
| C11 atomics and futex-backed waits | tested | `c_c11_atomic_surface_runs_on_lock_cmpxchg`, emulator futex tests | Memory-order conformance beyond the modeled primitives needs stress tests. |
| `poll`, `select`, `epoll_create1`, `epoll_ctl`, `epoll_wait` | tested | `c_select_fdset_surface_lowers_to_readiness_probe_and_runs`, `c_select_blocks_with_dynamic_await_and_runs`, `c_poll_*`, `c_epoll_surface_lowers_to_native_readiness_probe_and_runs` | Race coverage for event before arm, during arm, and timeout is incomplete. `COMPAT-STRESS-005`. |
| `eventfd`, `timerfd_create`, `timerfd_settime`, `timerfd_gettime` | tested | `c_eventfd_surface_uses_counter_object_profile`, `c_timerfd_surface_uses_object_timer_profile` | Full Linux flag and clock-id behavior is partial. |
| `clock_gettime`, `gettimeofday`, `time`, `nanosleep`, `usleep`, `alarm` | tested | `c_time_surface_uses_realtime_pcrs_and_sleep`, `c_usleep_and_alarm_surface_runs` | Time precision is emulator-coarse. |
| `fork`, `exec*`, `wait`, `waitpid`, `getpid`, `getppid` | tested | `c_posix_process_and_signal_mask_surface_runs`, `c_wait_and_getppid_surface_runs_after_fork`, `c_exec_family_lowers_to_native_exec`, emulator clone/exec tests | `exec` loads LNP64 assembly today; binary format loader work is tracked as `COMPAT-BIN-001`. |
| Signals: `sigaction`, `signal`, `sigprocmask`, `raise`, default/ignore dispositions, signal frames | tested | `c_sigaction_accepts_posix_action_struct`, `c_signal_default_and_ignore_dispositions_run`, `signal_frame_stack_area_is_non_executable` | Full POSIX signal queueing, altstack, and per-thread delivery semantics are partial. `COMPAT-SIGNAL-001`. |
| Sockets: `socket`, `bind`, `listen`, `connect`, `accept`, `getsockname`, `getsockopt`, `setsockopt`, `send`, `recv` | tested | `c_socket_surface_lowers_to_endpoint_object_controls_and_runs`, `demos/netcat.c`, `demos/httpd.c` in `scripts/run_demos.sh` | Nonblocking sockets, descriptor passing, UDP, and real network-driver service integration are partial. `COMPAT-SOCK-001`. |
| Optional dynamic loading and subprocess streams: `dlopen`, `dlsym`, `dlclose`, `popen`, `pclose` | unsupported | `c_optional_dynamic_loading_and_popen_fail_cleanly` | Intentionally fail cleanly until binary/dynamic-loader work exists. |
| Locale, wide chars, iconv, regex | partial | sbase text utilities exercise simple byte-string behavior | Full locale/wide/regex conformance is not implemented. `COMPAT-LIBC-003`. |
| LNP64 `__lnp_*` shim layer | native extension | `c_private_lnp_shim_layer_lowers_to_native_primitives`, `c_private_lnp_shim_layer_accepts_dynamic_fdr_tokens` | ABI names need psABI stabilization. |
| Resource Domain APIs | native extension | `c_domain_lifecycle_surface_runs_on_domain_ctl`, `c_domain_limit_failure_runs`, domain emulator tests | Random create/freeze/resume/destroy stress is missing. `COMPAT-STRESS-002`. |
| Capability APIs: send, receive, duplicate, narrow, seal, revoke, generation checks | native extension | `c_capability_transfer_surface_runs_on_native_cap_ops`, capability emulator tests | Randomized passing/revocation fuzzing is missing. `COMPAT-STRESS-001`. |
| Object APIs: queue/counter/memory object, pipe lowering, message receive | native extension | `c_object_creation_surface_runs_on_object_ctl`, `c_pipe_lowers_to_object_queue_and_runs`, `c_message_receive_lowers_to_await_pull_and_runs` | Multi-producer/multi-consumer race coverage is partial. |
| Call gates: sync, async, handoff | native extension | `c_sync_call_gate_runs`, `call_cap_sync_returns_across_domain_gate`, `call_cap_async_and_handoff_modes_execute_minimally` | psABI register contract for cross-domain calls needs formal text. |
| DMA APIs and DMA buffers | native extension | `dma_ctl_copy_and_fill_use_vma_permissions`, `dma_ctl_rejects_guard_unmapped_and_disallowed_domain`, `dma_ctl_uses_dma_buffer_capability_scope`, `dma_ctl_rejects_stale_and_revoked_dma_buffers` | Pending-operation revoke stress is not modeled yet. `COMPAT-STRESS-006`. |
| ELF objects, relocations, dynamic linker | unsupported | Design docs mention loader requirements; no implementation test covers ELF loading | Define binary format details. `COMPAT-BIN-001`. |
| Bootable userland image: `/sbin/init`, shell-like runner, filesystem image, `/dev` namespace | unsupported | Demos run individually through host CLI | Build minimal userland image. `COMPAT-USERLAND-001`. |

## Real Program Targets

These targets are the current compatibility gates. A target only moves to
`passing` when it builds and runs through a checked-in script without package-
specific compiler/runtime special casing.

| Target | Status | Evidence | Compatibility Bugs |
| --- | --- | --- | --- |
| sbase subset | passing | `third_party/sbase/*`, `scripts/run_sbase.sh` | Expand command coverage and edge cases under `COMPAT-FS-001`, `COMPAT-STDIO-001`, `COMPAT-LIBC-002`. |
| jsmn | passing | `third_party/jsmn/example/simple.c`, `third_party/jsmn/test/tests.c`, `scripts/run_sbase.sh` | None known beyond broader C parser/runtime coverage. |
| small HTTP server | passing | `demos/httpd.c`, `scripts/run_demos.sh` | Socket nonblocking and network-service semantics tracked by `COMPAT-SOCK-001`. |
| netcat-like socket demo | passing | `demos/netcat.c`, `scripts/run_demos.sh` | Socket nonblocking and descriptor passing tracked by `COMPAT-SOCK-001`. |
| sqlite-lite demo | passing | `demos/sqlite_lite.c`, `demos/sqlite_lite.db`, `scripts/run_demos.sh` | This is not upstream SQLite; full SQLite remains `COMPAT-PKG-003`. |
| Lua upstream | failing / not checked in | Lua-targeted compiler tests exist; no checked-in full Lua package gate | Remove Lua-specific normalizer pressure by fixing generic C semantics. `COMPAT-PKG-001`. |
| zlib upstream | not started | No checked-in zlib target | Add package gate. `COMPAT-PKG-002`. |
| SQLite upstream | not started | No checked-in SQLite target | Add package gate. `COMPAT-PKG-003`. |
| libpng upstream | not started | No checked-in libpng target | Add package gate after zlib. `COMPAT-PKG-004`. |
| musl tests subset | not started | No checked-in musl test gate | Add focused libc conformance harness. `COMPAT-PKG-005`. |

## Open Compatibility Bugs

| Bug | Requirement | Current State | Next Concrete Step |
| --- | --- | --- | --- |
| `COMPAT-ABI-001` | psABI/crt packaging | Startup is modeled in compiler/runtime tests. | Write `psABI.md` and define whether v1 ships crt objects or compiler-emitted startup. |
| `COMPAT-ABI-002` | auxv/dynamic-loader contract | `ENV_GET` and `getauxval` exist. | Freeze auxv keys, stack layout, and dynamic loader expectations. |
| `COMPAT-BIN-001` | Binary/object format | Assembly program loading exists; ELF details are design-only. | Define static v1 ELF relocation model and executable mapping rules. |
| `COMPAT-USERLAND-001` | Minimal userland image | Individual demos run through host CLI. | Add init program, command runner, boot manifest, and basic filesystem image script. |
| `COMPAT-PKG-001` | Upstream Lua | Lua compatibility is covered by targeted compiler tests only. | Add a reproducible upstream Lua package script and convert failures into generic compiler/runtime bugs. |
| `COMPAT-PKG-002` | zlib | Not started. | Vendor or fetch a small zlib release and add build/run smoke tests. |
| `COMPAT-PKG-003` | Upstream SQLite | Only sqlite-lite demo exists. | Add upstream amalgamation smoke once C/runtime gaps are known. |
| `COMPAT-PKG-004` | libpng | Not started. | Add after zlib passes. |
| `COMPAT-PKG-005` | musl tests | Not started. | Pick a small libc-test subset that avoids unsupported dynamic linking first. |
| `COMPAT-FS-001` | Filesystem/path conformance | sbase covers common commands. | Add negative/error path tests for symlinks, permissions, `stat`, and rename/link corner cases. |
| `COMPAT-STDIO-001` | stdio conformance | Common descriptor-backed streams pass. | Add EOF/error flag, buffering, append, and mixed read/write tests. |
| `COMPAT-LIBC-001` | Full `fcntl` command surface | Descriptor duplication path exists. | Define supported commands and add tests for unsupported errno behavior. |
| `COMPAT-LIBC-002` | Formatting/string/locale | Byte-string subset passes real demos. | Expand printf/scanf, locale, and collation tests. |
| `COMPAT-LIBC-003` | Wide char/regex/iconv | Mostly unsupported. | Decide v1 boundary and add fail-cleanly tests. |
| `COMPAT-LIBM-001` | Floating libm | Integer-model shims exist. | Add real double representation or document static integer-only v1 limit. |
| `COMPAT-MM-001` | File-backed mmap/shared coherence | Anonymous mapping security is well tested. | Add file mapping and shared mapping smoke tests. |
| `COMPAT-PTHREAD-001` | Full pthread lifecycle | Core create/join/TLS works. | Add detach, cancellation boundary, and join error tests. |
| `COMPAT-PTHREAD-002` | Timed/process-shared sync | Futex-backed primitives work. | Add timed waits and unsupported process-shared behavior tests. |
| `COMPAT-SIGNAL-001` | Full signal semantics | Default/ignore/action/mask subset works. | Add queueing, nested delivery, altstack boundary, and per-thread delivery tests. |
| `COMPAT-SOCK-001` | Full socket semantics | TCP-like local endpoint subset works. | Add nonblocking, UDP boundary, accepted endpoint inheritance, and descriptor passing plan. |
| `COMPAT-STRESS-001` | Capability fuzzing | Deterministic unit tests exist. | Add randomized cap send/dup/seal/revoke sequence test. |
| `COMPAT-STRESS-002` | Domain lifecycle fuzzing | Deterministic unit tests exist. | Add random create/freeze/resume/destroy sequence test. |
| `COMPAT-STRESS-003` | VMA fuzzing | Deterministic guard/NX/W^X tests exist. | Add random mmap/munmap/mprotect/guard test. |
| `COMPAT-STRESS-004` | Allocator pressure | Single-thread allocator tests exist. | Add multi-threaded allocation/free/realloc pressure test. |
| `COMPAT-STRESS-005` | poll/epoll races | Blocking and readiness tests exist. | Add before-arm, during-arm, after-timeout race tests. |
| `COMPAT-STRESS-006` | DMA pending revoke | Synchronous revoke rejection tests exist. | Model or explicitly document no-pending-DMA v1 policy, then test it. |

# LNP64 Real Toolchain Roadmap

This roadmap is the transition plan from the in-repo toy C compiler to a real
LLVM/Clang/lld based LNP64 toolchain capable of building NetBSD-derived libc,
libpthread, userland commands, and rump service components.

`toolchain/lnp64_target.manifest` is the checked, machine-readable seed of the
future LLVM target contract. The Rust test
`llvm_target_manifest_records_required_backend_contract` keeps its first
backend-facing fields synchronized with this roadmap.

`toolchain/lnp64_transition.manifest` records the broader transition checklist:
toy compiler retirement, real target contracts, the first LLVM/Clang path,
libc/runtime shims, software loader/exec-plan work, NetBSD personality layering,
and conformance gates. The Rust test
`toolchain_transition_manifest_records_layered_deliverables` keeps those
roadmap deliverables tied to concrete files and gates.

`toolchain/lnp64_llvm_bootstrap.manifest` names the first Clang-built program
set that must replace toy-compiler smoke coverage: hello, arithmetic, memory,
calls, and simple libc. The hello, arithmetic, memory, and calls entries are
now tested through the real LLVM/lld/software-loader path; `simple_libc`
remains partial until the Clang-built libc/runtime path replaces the smoke-only
shim.
`toolchain/lnp64_llvm_gates.manifest` records the concrete planned command
shapes for compiling, linking, exec-plan inspection, and emulator execution.
`scripts/run_llvm_bootstrap_gates.sh --dry-run` prints those planned commands
from the manifest; actual execution is blocked behind an explicit opt-in until
the LNP64 LLVM backend exists.
`Dockerfile.llvm` and `scripts/run_real_llvm_tblgen_docker.sh` provide the
first real LLVM infrastructure gate: it builds a container with LLVM tools and
runs `llvm-tblgen` over the LNP64 TableGen target files, writing generated
includes under `target/real-llvm-tblgen`.
`scripts/run_real_llvm_lnp64_docker.sh` overlays the backend, Clang target-info
hook, and lld arch hook into upstream LLVM 14 and builds real
`clang`/`llc`/`llvm-mc`/`llvm-objdump`/`lld` tools in Docker; its smoke now
verifies trivial LNP64 IR codegen, real Clang compiles of scalar C,
`demos/hello.c`, `demos/factorial.c`, `demos/allocator.c`,
`demos/fibonacci.c`, `demos/pcr.c`, `demos/cat.c`,
`demos/json_parser.c`, `demos/rot13.c`, `demos/producer_consumer.c`, and
`demos/parallel_hash.c`, `demos/sqlite_lite.c`, `demos/ping_pong.c`,
`demos/netcat.c`, and `demos/httpd.c` to target objects, plus generated
`natsort`, `jsmn`, string-only `inih`, and `cwalk` package smokes over real package
sources. It also covers
indirect calls, inline
asm, exit/argc, scalar arithmetic immediates and native remainders,
high-multiply primitives, scalar extension primitives, bit-count/rotate/byte-swap primitives, signed and
unsigned compares, conditional selects, signed loads, wide constants, stack
aggregate addresses, signed 32-bit return extension, minilibc
string/compare/`memmove` calls, minilibc
`calloc`/`realloc`, and minilibc `read` smokes. The gate assembles the checked
crt0 and minimal libc smoke stubs,
disassembles emitted objects, statically links crt0 plus an assembler-built
`main`, and statically links each Clang-built demo/probe object with crt0 plus
the checked C shim objects. The Docker wrapper submits those linked ELFs through
`elf-plan`/`run-elf` and requires expected output or exit status after exec-plan
validation and commit.
`toolchain/lnp64_static.ld` is the initial checked static linker-script
contract for lld-produced ELF inputs.
`toolchain/crt0_lnp64.s` is the initial checked crt0 startup stub for the
future LLVM/lld path.
`toolchain/liblnp64_min.s` is a checked smoke-only legacy assembler libc object
kept to prove llvm-mc/objdump coverage for the native libc opcodes while the
full libc/runtime is not ready. The string/memory/ctype, numeric conversion,
path helper, search helper, sort helper, `calloc`/`realloc`, `read`, `errno`,
`exit`, and real Clang demo link/run smokes now link against Clang-built tiny C
implementations instead of this assembler shim, keeping those slices on the
real C toolchain path.
`toolchain/liblnp64_string_min.c`, `toolchain/liblnp64_convert_min.c`,
`toolchain/liblnp64_path_min.c`, `toolchain/liblnp64_search_min.c`,
`toolchain/liblnp64_sort_min.c`, `toolchain/liblnp64_alloc_min.c`,
`toolchain/liblnp64_fd_min.c`, `toolchain/liblnp64_process_min.c`,
`toolchain/liblnp64_errno_min.c`, and `toolchain/liblnp64_poll_min.c` are the
checked source files for those tiny C shim objects.
`toolchain/lnp64_intrinsics.h` is the initial checked private C shim header for
native `__lnp_*` calls.
`toolchain/lnp64_clang_driver.manifest` records the planned Clang/lld driver
defaults for the first backend bring-up.
Until LNP64 jump-table lowering is implemented, the checked real LLVM Clang
gates compile freestanding C with `-fno-jump-tables`; this avoids accidental
SelectionDAG jump-table emission while keeping ordinary branch lowering under
test.
`toolchain/lnp64_llvm_filemap.manifest` records the first llvm-project source
surface for the backend, MC layer, Clang target info, driver, lld relocation
handler, and smoke tests.
`toolchain/lnp64_registers.manifest` records the backend-facing register
classes, reserved registers, hidden compare flags, allocatable sets, and
debug/unwind role names.
The first llvm-project backend files now exist under `llvm/lib/Target/LNP64/`:
`CMakeLists.txt`, `LNP64.td`, `LNP64RegisterInfo.td`,
`LNP64CallingConv.td`, `LNP64InstrInfo.td`, `TargetInfo/LNP64TargetInfo.cpp`,
initial `MCTargetDesc` registration/code-emitter files, and the first
`TargetMachine`/`Subtarget`/`ISelLowering`/`FrameLowering` class skeletons.
The first Clang target-info, Clang driver-arch, lld ELF arch, and MC
AsmBackend/object-writer source files also exist under the matching
llvm-project paths, with the LNP64 triple, freestanding driver defaults,
inline-asm constraint surface, and relocation names pinned to the checked
manifests. The MC AsmParser, code emitter, InstPrinter, and Disassembler
components now cover the first fixed32 integer, memory, branch/call, and return
subset for future `llvm-mc` round trips.
Scaffolded llvm-project lit tests now exist for `llc` hello/native-intrinsic
codegen, `llvm-mc` basic assembly, and Clang driver command shape; they remain
marked `XFAIL` until the target is integrated into a buildable llvm-project
tree.
The MC code emitter now has concrete fixed32 paths for `NOP`, `RET`, `LI`,
`MOV`, integer ALU/compare operations, branch/call/return opcodes,
byte/halfword/word/doubleword `LD`/`ST`, and native heap opcodes
`ALLOC`/`ALLOC_EX`/`ALLOC_SIZE`/`FREE`. The atomics/wait subset now includes
real LLVM MC round trips for AMOs, compare-exchange, and `FUTEX_WAIT`/
`FUTEX_WAKE`; the barrier subset now round-trips `FENCE` aliases and explicit
`ISYNC`. The first typed-control opcodes
`OBJECT_CTL` and `DOMAIN_CTL` also have real MC and committed-exec coverage
through private `__lnp_object_ctl`/`__lnp_domain_ctl` Clang smokes. Other
opcodes remain blocked until operand encodings are implemented.
`OPEN_AT`, `PULL`, `PUSH`, `GET_PCR`, and the `CLONE.SPAWN`/`THREAD_JOIN`
native thread-control slice now have real LLVM MC/object/link/`run-elf`
coverage through the private intrinsic header, so POSIX-shaped `openat`,
`read`, `write`, `pid`, and `getpid` live in libc shim code rather than in the
Rust compiler frontend. `toolchain/lnp64_intrinsic_lowering.manifest` now
separates real LLVM call
lowerings, inline-asm shims, and declared-but-pending native intrinsics so the
backend cannot silently lower compatibility-critical calls such as `MMAP` while
dropping source capability, flags, offset, or argument-block state. The private
intrinsic header now exposes object-smoked inline futex wait/wake shims over
the real MC futex opcodes, `toolchain/liblnp64_futex_min.c` provides the first
Clang-built minilibc futex object for libc/pthread bring-up,
`toolchain/liblnp64_poll_min.c` starts the Clang-built
poll/select/epoll/kqueue shim over `__lnp_await`, and
`toolchain/liblnp64_signal_min.c` starts the Clang-built signal compatibility
shim over native signal/gate aliases, and `toolchain/liblnp64_socket_min.c`
starts the Clang-built socket compatibility shim over endpoint object controls.
The
disassembler decodes that same initial fixed32 subset for future `llvm-mc`
round trips. The MC layer now has target fixup kinds and object-writer
relocation mapping for branch/data relocations, and the lld scaffold can patch
`R_LNP64_BRANCH26` into the aligned signed branch field.
The first SelectionDAG patterns now select signed-16 constant materialization
through `LI`, simple i64 ALU operations (`add`/`sub`/`mul`/signed `div`,
bitwise ops including `xor allones` to `NOT`, and shifts), and i64
base+signed-14-offset loads/stores onto the fixed32 opcodes. LLVM
unconditional `br` now selects the architectural `JMP`
through a machine-basic-block branch target operand. Non-varargs calls now
lower register arguments through `CC_LNP64`, emit `CALL` for global or external
callees, emit `CALL_REG` for register callees, and copy register results through
`RetCC_LNP64`. LLVM `BR_CC` now lowers signed integer comparisons through
condition-specific pseudo branches that expand to `CMP` plus `BEQ`/`BNE`/`BLT`/
`BGT`/`BLE`/`BGE`. The DAG selector now folds direct frame-index stack-slot
loads/stores for 64-bit and 32-bit `int` locals into `LD`/`ST` and
`LD_W`/`ST_W`, accepting Clang-emitted sign-extending and any-extending 32-bit
stack-load nodes onto the current word-load bring-up opcode. The real Clang
factorial object gate now exercises stack locals through the same SP-relative
frame-index elimination path as spills/reloads; the allocator object gate adds
external `alloc`/`free`/`write` call references, stack-local pointer storage,
and string-address relocations without claiming runtime allocator execution;
the Fibonacci object gate covers recursive direct calls, returns, local
spill-slot traffic, and multi-function object emission.
Fixed integer stack-passed call operands now lower through call-frame pseudos,
SP-relative stores/loads, and non-leaf `LR_GET`/`LR_SET` spill/restore.
Varargs, stack returns, richer aggregate ABI cases, unaligned/large-offset
address expansion, fuller global/constant-pool models, and the remaining
`__lnp_*` native shim coverage remain bring-up blockers.
Narrow memory selection covers zero-extending byte/half/word loads and
truncating byte/half/word stores through `LD_B`/`LD_H`/`LD_W` and
`ST_B`/`ST_H`/`ST_W`.
The first private native shim lowerings recognize `__lnp_pull`, `__lnp_push`,
`__lnp_await`, `__lnp_call`, `__lnp_domain_ctl`, and `__lnp_object_ctl` and
emit native `PULL`/`PUSH`/`AWAIT`/`GATE_CALL`/`DOMAIN_CTL`/`OBJECT_CTL`
operations directly, treating the C ABI `lnp64_cap_t` as a GPR capability
handle. `__lnp_push`, `__lnp_await`, `__lnp_call`, `__lnp_gate_return`,
`__lnp_domain_ctl`, and `__lnp_object_ctl` now have Clang/lld/`run-elf`
execution smokes. The remaining `__lnp_*` shims still need backend nodes or
runtime call fallbacks.
Return lowering now maps the LLVM return value path through `RetCC_LNP64` into
`r1` and selects a target `RET_FLAG` DAG node to the architectural `RET`;
formal argument lowering maps register arguments from `CC_LNP64` live-ins, and
fixed integer stack arguments use reserved call-frame pseudos plus SP-relative
stores/loads. Varargs, stack returns, aggregate ABI corners, and broader
call-frame stress remain bring-up blockers.
Control-flow opcodes now carry TableGen instruction properties for branches,
calls, link-register definition/use, returns, terminators, and barriers, so
later call/return lowering and verifier work can rely on instruction metadata.
`CMP` now defines the hidden backend `FLAGS` state and conditional branches use
it, giving the future LLVM `icmp`/conditional-`br` lowering a real dependency
model without exposing flags to the C ABI or debug/unwind state.
`LNP64InstrInfo` now lowers GPR-to-GPR physical register copies through `MOV`,
giving instruction selection and register allocation a concrete copy path.
It also emits first GPR-only stack-slot spills/reloads through `ST`/`LD` with a
frame-index base and zero offset; full frame layout and prologue/epilogue
emission remain blockers.
`LNP64RegisterInfo` now rewrites frame-index operands to `R31` plus the stack
object offset and frame size, giving those stack-slot instructions a first
SP-relative lowering path.
Frame lowering now reserves `r30` as a backend scratch register and emits
signed-16 stack adjustments with `LI r30, size` plus `SUB`/`ADD` on `r31`.
`LNP64InstrInfo.td` now carries operand-bearing TableGen classes for integer
RRR/RR/RI, branch, memory, atomic, and native-capability instruction shapes
instead of name-only opcode stubs.
They are scaffolded source files for the real port, not a buildable code
generator yet.
`toolchain/lnp64_mc_encoding.manifest` records the initial MC format classes,
opcode coverage, relocation hooks, and llvm-project surfaces for the first
assembler/emitter/disassembler path.
`toolchain/lnp64_run_elf.manifest` records the boundary between the existing
ELF-to-exec-plan loader probe, committed-image text execution, and the still
unfinished stdout/libc runtime work.
`toolchain/lnp64_toy_compiler_policy.manifest` records the checked policy that
keeps the in-repo C compiler as a bootstrap smoke generator rather than the
platform-defining toolchain.

The current Rust assembler, emulator, and C compiler remain useful bootstrap
and architecture smoke-test tools. They are not the long-term application
toolchain.

## Target Boundary

The production path is:

```text
Clang C/C++ frontend
  -> LLVM IR
  -> LLVM LNP64 backend
  -> ELF64 LNP64 relocatable objects
  -> lld static executable or static PIE
  -> LNP64 software loader
  -> bounded EXEC plan
  -> hardware/emulator EXEC commit
```

Hardware still does not parse ELF, shebangs, archives, dynamic-linker state, or
NetBSD policy. Those remain loader, libc, and personality responsibilities.

## LLVM Backend Milestones

1. Register the target skeleton.
   - Triple: `lnp64-unknown-none` for native static programs.
   - Data layout: little-endian LP64, 64-bit pointer width, natural 64-bit GPR
     alignment.
   - Object format: ELF64 with provisional `EM_LNP64` from `object_format.md`.
   - Initial MC layer: asm parser/printer enough to round-trip core integer
     instructions, with format and relocation hooks from
     `toolchain/lnp64_mc_encoding.manifest`.

2. Define registers and calling convention.
   - GPR `r0`-`r31`, FDR capability registers `fd0`-`fd255`, PCR names, and
     dedicated FPU/vector register files from
     `toolchain/lnp64_registers.manifest`.
   - psABI argument/return rules from `psABI.md`.
   - Stack layout, call frame, callee-save set, TLS pointer, and startup
     metadata access.
   - Debug/unwind minimums from `toolchain/lnp64_debug_unwind.manifest`:
     DWARF line/register info, CFI for non-leaf frames, `LR` return-address
     state, and no language exception runtime in v0.

3. Lower normal code.
   - Integer ALU, compares, branches, calls/returns.
   - Loads/stores with byte/half/word/dword widths; i64 base+offset,
     zero-extending narrow loads, and truncating narrow stores exist first,
     while signed narrow loads and large-offset expansion remain pending.
   - Global addresses, constant pools, stack slots, and frame lowering.
   - Atomics and fences needed by libc/libpthread.

4. Add native primitive access.
   - Inline asm constraints for GPR/FDR/FPR/VR/PCR operands from
     `toolchain/lnp64_inline_asm.manifest`.
   - `CLONE` is a backend-visible native primitive with profile operands
     `new_process_cow`, `new_thread_shared_vm`, `spawn_entry`, and
     `domain_task`. POSIX `fork()` and `pthread_create()` remain libc/runtime
     lowerings to the first two profiles rather than compiler special cases.
   - Backend builtins or Clang intrinsics for private `__lnp_*` shims:
     `__lnp_openat`, `__lnp_pull`, `__lnp_push`, `__lnp_mmap`,
     `__lnp_await`, `__lnp_gate_call`, `__lnp_call`,
     `__lnp_gate_return`, `__lnp_domain_ctl`, `__lnp_domain_create`,
     `__lnp_object_ctl`, `__lnp_object_create`, `__lnp_cap_dup`,
     `__lnp_cap_send`, `__lnp_cap_recv`, and `__lnp_cap_revoke`.
   - Application C should normally call libc/POSIX APIs, not raw primitives.

5. Implement relocations and lld integration.
   - Absolute, PC-relative branch/call, GOT-like static PIE, TLS, and data
     relocations.
   - Static archives and ordinary `ar`/lld workflows.
   - Linker script defaults from `toolchain/lnp64_static.ld` that match
     `object_format.md` mapping policy.

6. Build the software loader path.
   - Parse ELF in a loader/personality component, not in hardware.
   - Validate segment permissions, relocations, TLS, auxv, startup descriptors,
     FDR grants, and executable provenance.
   - Produce bounded EXEC plan records and prove old image preservation on
     pre-commit failure.
   - Current repository code starts this in `src/loader.rs`: it parses static
     ELF64 LNP64 program headers, builds bounded exec-plan VMA records, applies
     checked `R_LNP64_RELATIVE` RELA entries with an explicit load bias, applies
     symbol-less `R_LNP64_ABS64`, `R_LNP64_ABS32`, and `R_LNP64_GLOB_DAT` slots
     that a static linker has already resolved, applies symbol-less
     `R_LNP64_TLS_TPREL64` and `R_LNP64_TLS_DTPREL64` offsets against `PT_TLS`,
     resolves symbol-less `R_LNP64_FDR_DESC64` references against authorized
     startup FDR descriptor records, and parses `LNP64ST\0` startup/FDR
     descriptor notes. It can materialize VMA byte images with file-backed
     contents plus zero-fill tails. Symbolful relocation resolution and richer
     startup authority installation remain blocked until the fuller lld/loader
     path exists.

## Clang, Libc, And Runtime Milestones

1. Clang driver support for `--target=lnp64-unknown-none`.
2. Private libc/syscall shim layer over `__lnp_*` intrinsics.
3. Minimal crt objects or documented compiler-emitted startup transition pinned
   by `toolchain/lnp64_crt_startup.manifest` and `toolchain/crt0_lnp64.s`.
4. libc surfaces for file descriptors, paths, memory mapping, time, signals,
   pthreads, sockets, and Resource Domain controls.
5. libpthread over `CLONE`, futexes, TLS, timers, and event queues.
6. libm strategy: integer-only bootstrap first, real floating-point libm later.

## NetBSD Bring-Up Layers

1. Build small NetBSD-derived libc/libpthread/libm pieces with Clang.
2. Build tiny userland commands as static LNP64 ELF objects.
3. Bring up rump filesystem and networking services over FDR block/network
   capabilities.
4. Replace fixed-record smoke fixtures with loader-produced EXEC plans.
5. Move the NetBSD personality system gate from toy-compiled `.s` programs to
   Clang/lld produced ELF inputs.
6. Consider a fuller `evb-lnp64` machine port only after rump-style services
   and static userland are credible.

## Toy Compiler Freeze Policy

The in-repo C compiler is frozen as a bootstrap/test artifact once an LLVM
target skeleton can compile and run a comparable hello-world, syscall-shim, and
NetBSD personality smoke. After that point:

- New application/package compatibility work should target Clang first.
- The toy compiler should receive only small fixes needed to keep existing smoke
  tests useful.
- Any remaining `lnp64 cc` use must pass `--toy-bootstrap` so the Rust compiler
  stays visibly legacy and cannot become the implicit C path again.
- New native primitives should be exposed through libc/private `__lnp_*` shims
  and LLVM intrinsics, not new ad hoc C compiler builtins unless needed for a
  temporary architecture test.

## First Acceptance Gates

The first real-toolchain gate should prove all of the following:

- `clang --target=lnp64-unknown-none` compiles a freestanding `hello.c`.
- `llvm-mc` or the integrated assembler emits an ELF64 LNP64 object.
- `lld` links a static executable with the LNP64 ELF machine id.
- The software loader converts that executable to an EXEC plan.
- The emulator runs the program without the toy C compiler.
- A private libc shim can call at least `OPEN_AT`, `PULL`, `PUSH`, `AWAIT`,
  `OBJECT_CTL`, `DOMAIN_CTL`, and `CAP_*` through backend-supported intrinsics
  or inline asm.

The concrete first-program set is pinned in
`toolchain/lnp64_llvm_bootstrap.manifest`:

- `hello`: freestanding output/exit path.
- `arithmetic`: integer ALU, compares, branches, calls, and stack locals.
- `memory`: loads/stores, global data, allocation, and errno/TLS interaction.
- `calls`: call/return, `LR`, and spill-slot behavior.
- `simple_libc`: startup, descriptor I/O, errno/TLS, strings/memory,
  pthread/futex, event waits, mmap/signal/socket subset, and static linking.

The first four gates now run through real Clang/lld output and the software
loader without the toy compiler. Full replacement remains partial until the
simple-libc gate no longer depends on the smoke-only shim.

The planned command shapes are pinned in
`toolchain/lnp64_llvm_gates.manifest`, the dry-run gate driver is
`scripts/run_llvm_bootstrap_gates.sh --dry-run`, and the driver defaults are
pinned in `toolchain/lnp64_clang_driver.manifest`. The implementation file
surface is pinned in `toolchain/lnp64_llvm_filemap.manifest`. `Dockerfile.llvm`
and `scripts/run_real_llvm_tblgen_docker.sh` pin the first real LLVM tool gate:
the Docker script runs host-independent `llvm-tblgen` over the LNP64 TableGen
backend files and writes generated includes under `target/real-llvm-tblgen`.
`scripts/run_real_llvm_lnp64_docker.sh` pins the next real LLVM gate by building
upstream LLVM 14 `clang`, `llc`, `llvm-mc`, `llvm-objdump`, and an ELF-only
`lld` smoke driver with LNP64 registered, then proving IR codegen, real Clang
scalar C, hello object, factorial object, allocator object, and Fibonacci calls
object compilation, C `__lnp_push` native-intrinsic and `_exit` objects,
crt0/minilibc assembly, disassembly, assembler-main static linking, intrinsic
and exit static linking, per-demo Clang-object static linking, linked
hello/factorial/allocator/Fibonacci `run-elf` execution smokes, and direct
intrinsic/exit `run-elf` execution smokes through those real tools.
The Clang compile gates must include `toolchain/lnp64_intrinsics.h`, the crt
gate must assemble `toolchain/crt0_lnp64.s`, the static link gate must use
`toolchain/lnp64_static.ld`, and all gates must stay Clang/lld/loader based: no
gate in that manifest or driver script may invoke `lnp64 cc`, `cargo run -- cc`,
or the in-repo toy C compiler.
The `run_without_toy_compiler` gate is partial: linked hello, factorial,
allocator, Fibonacci, PCR, cat/openat, JSON parser, rot13, producer/consumer,
parallel hash, sqlite lite, and ping-pong smokes now run through real
Clang/lld output, while the full
gate remains open until the Clang-built libc/runtime path replaces the
smoke-only shim.

## Checked Transition Deliverables

The transition is intentionally layered so the toy compiler stops defining the
platform while remaining useful as a smoke generator:

| Phase | Current Artifact | Gate |
| --- | --- | --- |
| Toy compiler retirement | `toolchain_roadmap.md`, `toolchain/lnp64_toy_compiler_policy.manifest`, `src/c_compiler.rs`, and private `__lnp_*` shim tests keep new native work out of ad hoc POSIX-shaped compiler features. | `toy_compiler_policy_manifest_freezes_bootstrap_role`, `c_private_lnp_manifest_intrinsics_lower_and_run` |
| Real toolchain target | `toolchain/lnp64_target.manifest`, register-class, psABI, relocation, MC encoding, object-format, crt, inline-asm, debug/unwind, intrinsic, isel, and exec-plan manifests. | `toolchain_contract_index_is_complete`, `register_manifest_records_backend_classes`, `mc_encoding_manifest_covers_initial_backend_opcodes` |
| Minimal LLVM/Clang path | `toolchain/lnp64_llvm_bootstrap.manifest` pins tested hello, arithmetic, memory, calls, PCR, cat/openat, json parser, rot13, producer/consumer, parallel hash, sqlite lite, and ping-pong replacement gates plus partial netcat, httpd, and simple-libc replacement gates; `toolchain/lnp64_llvm_gates.manifest` and `scripts/run_llvm_bootstrap_gates.sh --dry-run` pin the remaining Clang/lld/loader command shapes that replace `lnp64 cc`; `Dockerfile.llvm`, `scripts/run_real_llvm_tblgen_docker.sh`, and `scripts/run_real_llvm_lnp64_docker.sh` pin the real LLVM TableGen, Clang scalar, hello object, factorial object, allocator object, Fibonacci calls object, PCR demo object, cat demo object, json parser demo object, rot13 demo object, producer/consumer demo object, parallel hash demo object, sqlite lite demo object, ping-pong demo object, netcat/httpd demo object/static links, natsort, jsmn, inih, and cwalk package object/link/run-elf smokes, Clang-built string/memory/ctype, numeric conversion, path helper, search helper, sort helper, allocation, fd, process, errno, futex, poll/select/epoll/kqueue, signal, and socket libc objects, legacy smoke libc assembly, codegen, MC, disassembly, GET_PCR and OPEN_AT intrinsics, lld static-link, and run-elf smoke gates; `toolchain/lnp64_run_elf.manifest` pins the execution boundary between loader commit and real ELF text execution; `toolchain/lnp64_clang_driver.manifest` pins the driver defaults and `toolchain/include/{assert,ctype,stdio,string}.h` pins the first target C header root; `toolchain/lnp64_llvm_filemap.manifest` pins the llvm-project source surface; `toolchain/lnp64_static.ld` pins the first lld static layout; `toolchain/crt0_lnp64.s` pins the first crt0 startup stub; `toolchain/liblnp64_min.s` pins the legacy assembler libc opcode smoke; `toolchain/liblnp64_string_min.c`, `toolchain/liblnp64_convert_min.c`, `toolchain/liblnp64_path_min.c`, `toolchain/liblnp64_search_min.c`, `toolchain/liblnp64_sort_min.c`, `toolchain/liblnp64_alloc_min.c`, `toolchain/liblnp64_fd_min.c`, `toolchain/liblnp64_process_min.c`, `toolchain/liblnp64_errno_min.c`, `toolchain/liblnp64_poll_min.c`, `toolchain/liblnp64_signal_min.c`, and `toolchain/liblnp64_socket_min.c` pin the checked C shim sources; `toolchain/lnp64_intrinsics.h` pins the private C shim header; `toolchain/lnp64_mc_encoding.manifest` pins the first MC encoding and relocation hooks. | `llvm_bootstrap_manifest_names_first_clang_gate`, `llvm_gate_manifest_pins_non_toy_clang_commands`, `run_elf_manifest_records_execution_boundary`, `clang_driver_manifest_matches_llvm_gates`, `llvm_filemap_manifest_names_backend_source_surface`, `mc_encoding_manifest_covers_initial_backend_opcodes`, `crt0_startup_stub_matches_crt_contract`, and `intrinsic_header_matches_intrinsic_manifest` |
| Libc/runtime shim layer | `libc_roadmap.md`, `toolchain/lnp64_libc_shim.manifest`, crt/startup manifest, intrinsic manifest, and private intrinsic header define startup, TLS/errno, allocation, FDR I/O, pthread/futex, event waits, mmap, signal, and socket lowering. | `libc_shim_manifest_covers_runtime_surfaces` plus `scripts/run_software_gates.sh` |
| Software loader and exec plan | `src/loader.rs`, `src/emulator.rs`, `object_format.md`, `toolchain/lnp64_exec_plan.manifest`, and `toolchain/lnp64_loader_security.manifest` define the initial ELF64 parser, encoded exec-plan records, emulator-side descriptor validation, committed entry/TLS/startup metadata, W^X/NX/ASLR/provenance coverage, and atomic memory-image commit probe for the bounded `EXEC` boundary. | `exec_plan_manifest_matches_loader_boundary_contract`, `loader_security_manifest_covers_exec_plan_security`, plus the `exec_descriptor` test filter |
| NetBSD personality layering | `netbsd_personality_abi.md`, `toolchain/lnp64_netbsd_layers.manifest`, this roadmap, and the NetBSD system script keep the personality layered over native services. | `netbsd_layers_manifest_preserves_personality_order` plus `scripts/run_netbsd_personality_system.sh` |
| Conformance gates | `conformance_matrix.md`, `toolchain/lnp64_conformance_gates.manifest`, `scripts/run_software_gates.sh`, and `scripts/run_all_gates.sh` enumerate asm demos, C tests, randomized emulator tests, adversarial/fault tests, package tests, NetBSD personality gates, partial LLVM-built versions, RTL/proof, and whitespace gates. | `conformance_gate_manifest_covers_required_layers`, `scripts/run_software_gates.sh`, and `scripts/run_all_gates.sh` |

The `minimal_llvm_clang_path` row is now partial. It is not complete until the
Clang/lld/software-loader path covers the simple-libc replacement gate without
the smoke-only libc shim or toy compiler.

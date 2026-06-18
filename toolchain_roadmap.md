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
calls, and simple libc. The entries are marked `planned` until a real
LLVM/lld/software-loader path runs them without the toy compiler.
`toolchain/lnp64_llvm_gates.manifest` records the concrete planned command
shapes for compiling, linking, exec-plan inspection, and emulator execution.
`scripts/run_llvm_bootstrap_gates.sh --dry-run` prints those planned commands
from the manifest; actual execution is blocked behind an explicit opt-in until
the LNP64 LLVM backend exists.
`toolchain/lnp64_static.ld` is the initial checked static linker-script
contract for lld-produced ELF inputs.
`toolchain/crt0_lnp64.s` is the initial checked crt0 startup stub for the
future LLVM/lld path.
`toolchain/lnp64_intrinsics.h` is the initial checked private C shim header for
native `__lnp_*` calls.
`toolchain/lnp64_clang_driver.manifest` records the planned Clang/lld driver
defaults for the first backend bring-up.
`toolchain/lnp64_llvm_filemap.manifest` records the first llvm-project source
surface for the backend, MC layer, Clang target info, driver, lld relocation
handler, and smoke tests.
`toolchain/lnp64_registers.manifest` records the backend-facing register
classes, reserved registers, allocatable sets, and debug/unwind role names.
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
`MOV`, integer ALU/compare operations, branch/call/return opcodes, and
byte/halfword/word/doubleword `LD`/`ST`; other opcodes remain blocked until
operand encodings are implemented. The disassembler decodes that same initial
fixed32 subset for future `llvm-mc` round trips. The MC layer now has target
fixup kinds and object-writer relocation mapping for branch/data relocations,
and the lld scaffold can patch `R_LNP64_BRANCH26` into the aligned signed
branch field.
The first SelectionDAG patterns now select signed-16 constant materialization
through `LI`, simple i64 ALU operations (`add`/`sub`/`mul`/signed `div`,
bitwise ops, and shifts), and i64 base+signed-14-offset loads/stores onto the
fixed32 opcodes. LLVM unconditional `br` now selects the architectural `JMP`
through a machine-basic-block branch target operand. Direct, non-varargs calls
now lower register arguments through `CC_LNP64`, emit `CALL` for global or
external callees, and copy register results through `RetCC_LNP64`. Conditional
branch lowering, indirect calls, stack call operands/results, call-frame
pseudos, signed narrow loads, unaligned/large-offset address expansion, and
globals remain bring-up blockers.
Narrow memory selection covers zero-extending byte/half/word loads and
truncating byte/half/word stores through `LD_B`/`LD_H`/`LD_W` and
`ST_B`/`ST_H`/`ST_W`.
The first private native shim lowerings recognize `__lnp_pull`, `__lnp_push`,
and `__lnp_call` and emit native `PULL`/`PUSH`/`GATE_CALL` operations directly,
treating the C ABI `lnp64_cap_t` as a GPR capability handle. The remaining
`__lnp_*` shims still need backend nodes or runtime call fallbacks.
Return lowering now maps the LLVM return value path through `RetCC_LNP64` into
`r1` and selects a target `RET_FLAG` DAG node to the architectural `RET`;
formal argument lowering maps register arguments from `CC_LNP64` live-ins.
This first call-convention lowering is intentionally register-only: varargs,
stack arguments, stack returns, indirect calls, and call-frame pseudo handling
remain bring-up blockers.
Control-flow opcodes now carry TableGen instruction properties for branches,
calls, link-register definition/use, returns, terminators, and barriers, so
later call/return lowering and verifier work can rely on instruction metadata.
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
ELF-to-exec-plan loader probe and the still-planned no-toy-compiler execution
gate.
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

Until those gates exist, the checked NetBSD personality system remains the
bootstrap compatibility target and the toy compiler remains on the critical
path.

The planned command shapes are pinned in
`toolchain/lnp64_llvm_gates.manifest`, the dry-run gate driver is
`scripts/run_llvm_bootstrap_gates.sh --dry-run`, and the driver defaults are
pinned in `toolchain/lnp64_clang_driver.manifest`. The implementation file
surface is pinned in `toolchain/lnp64_llvm_filemap.manifest`. The Clang compile
gates must include `toolchain/lnp64_intrinsics.h`, the crt gate must assemble
`toolchain/crt0_lnp64.s`, the static link gate must use
`toolchain/lnp64_static.ld`, and all gates must stay Clang/lld/loader based: no
gate in that manifest or driver script may invoke `lnp64 cc`, `cargo run -- cc`,
or the in-repo toy C compiler.
The `run_without_toy_compiler` gate remains planned until
`toolchain/lnp64_run_elf.manifest` moves text fetch/decode, stdout/exit, and
no-toy-compiler stages out of `planned`.

## Checked Transition Deliverables

The transition is intentionally layered so the toy compiler stops defining the
platform while remaining useful as a smoke generator:

| Phase | Current Artifact | Gate |
| --- | --- | --- |
| Toy compiler retirement | `toolchain_roadmap.md`, `toolchain/lnp64_toy_compiler_policy.manifest`, `src/c_compiler.rs`, and private `__lnp_*` shim tests keep new native work out of ad hoc POSIX-shaped compiler features. | `toy_compiler_policy_manifest_freezes_bootstrap_role`, `c_private_lnp_manifest_intrinsics_lower_and_run` |
| Real toolchain target | `toolchain/lnp64_target.manifest`, register-class, psABI, relocation, MC encoding, object-format, crt, inline-asm, debug/unwind, intrinsic, isel, and exec-plan manifests. | `toolchain_contract_index_is_complete`, `register_manifest_records_backend_classes`, `mc_encoding_manifest_covers_initial_backend_opcodes` |
| Minimal LLVM/Clang path | `toolchain/lnp64_llvm_bootstrap.manifest` pins the planned hello, arithmetic, memory, calls, and simple-libc replacement gates for the toy-compiler smoke path; `toolchain/lnp64_llvm_gates.manifest` and `scripts/run_llvm_bootstrap_gates.sh --dry-run` pin the Clang/lld/loader command shapes that replace `lnp64 cc`; `toolchain/lnp64_run_elf.manifest` pins the execution boundary between loader commit and real ELF text execution; `toolchain/lnp64_clang_driver.manifest` pins the driver defaults; `toolchain/lnp64_llvm_filemap.manifest` pins the llvm-project source surface; `toolchain/lnp64_static.ld` pins the first lld static layout; `toolchain/crt0_lnp64.s` pins the first crt0 startup stub; `toolchain/lnp64_intrinsics.h` pins the private C shim header; `toolchain/lnp64_mc_encoding.manifest` pins the first MC encoding and relocation hooks. | `llvm_bootstrap_manifest_names_first_clang_gate`, `llvm_gate_manifest_pins_non_toy_clang_commands`, `run_elf_manifest_records_execution_boundary`, `clang_driver_manifest_matches_llvm_gates`, `llvm_filemap_manifest_names_backend_source_surface`, `mc_encoding_manifest_covers_initial_backend_opcodes`, `crt0_startup_stub_matches_crt_contract`, and `intrinsic_header_matches_intrinsic_manifest` |
| Libc/runtime shim layer | `libc_roadmap.md`, `toolchain/lnp64_libc_shim.manifest`, crt/startup manifest, intrinsic manifest, and private intrinsic header define startup, TLS/errno, allocation, FDR I/O, pthread/futex, event waits, mmap, signal, and socket lowering. | `libc_shim_manifest_covers_runtime_surfaces` plus `scripts/run_software_gates.sh` |
| Software loader and exec plan | `src/loader.rs`, `src/emulator.rs`, `object_format.md`, `toolchain/lnp64_exec_plan.manifest`, and `toolchain/lnp64_loader_security.manifest` define the initial ELF64 parser, encoded exec-plan records, emulator-side descriptor validation, committed entry/TLS/startup metadata, W^X/NX/ASLR/provenance coverage, and atomic memory-image commit probe for the bounded `EXEC` boundary. | `exec_plan_manifest_matches_loader_boundary_contract`, `loader_security_manifest_covers_exec_plan_security`, plus the `exec_descriptor` test filter |
| NetBSD personality layering | `netbsd_personality_abi.md`, `toolchain/lnp64_netbsd_layers.manifest`, this roadmap, and the NetBSD system script keep the personality layered over native services. | `netbsd_layers_manifest_preserves_personality_order` plus `scripts/run_netbsd_personality_system.sh` |
| Conformance gates | `conformance_matrix.md`, `toolchain/lnp64_conformance_gates.manifest`, `scripts/run_software_gates.sh`, and `scripts/run_all_gates.sh` enumerate asm demos, C tests, randomized emulator tests, adversarial/fault tests, package tests, NetBSD personality gates, planned LLVM-built versions, RTL/proof, and whitespace gates. | `conformance_gate_manifest_covers_required_layers`, `scripts/run_software_gates.sh`, and `scripts/run_all_gates.sh` |

The `minimal_llvm_clang_path` row is still marked planned. It is not complete
until Clang/lld can build and the software loader can run the small programs
listed in the first real-toolchain gate without the toy C compiler.

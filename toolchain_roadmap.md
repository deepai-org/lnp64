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
     instructions.

2. Define registers and calling convention.
   - GPR `r0`-`r31`, FDR capability registers `fd0`-`fd255`, PCR names, and
     dedicated FPU/vector register files.
   - psABI argument/return rules from `psABI.md`.
   - Stack layout, call frame, callee-save set, TLS pointer, and startup
     metadata access.
   - Debug/unwind minimums from `toolchain/lnp64_debug_unwind.manifest`:
     DWARF line/register info, CFI for non-leaf frames, `LR` return-address
     state, and no language exception runtime in v0.

3. Lower normal code.
   - Integer ALU, compares, branches, calls/returns.
   - Loads/stores with byte/half/word/dword widths.
   - Global addresses, constant pools, stack slots, and frame lowering.
   - Atomics and fences needed by libc/libpthread.

4. Add native primitive access.
   - Inline asm constraints for GPR/FDR/PCR operands from
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
   - Linker script defaults that match `object_format.md` mapping policy.

6. Build the software loader path.
   - Parse ELF in a loader/personality component, not in hardware.
   - Validate segment permissions, relocations, TLS, auxv, startup descriptors,
     FDR grants, and executable provenance.
   - Produce bounded EXEC plan records and prove old image preservation on
     pre-commit failure.
   - Current repository code starts this in `src/loader.rs`: it parses static
     ELF64 LNP64 program headers, builds bounded exec-plan VMA records, applies
     checked `R_LNP64_RELATIVE` RELA entries with an explicit load bias, and
     rejects unsupported relocations until the fuller lld/loader path exists.

## Clang, Libc, And Runtime Milestones

1. Clang driver support for `--target=lnp64-unknown-none`.
2. Private libc/syscall shim layer over `__lnp_*` intrinsics.
3. Minimal crt objects or documented compiler-emitted startup transition pinned
   by `toolchain/lnp64_crt_startup.manifest`.
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

## Checked Transition Deliverables

The transition is intentionally layered so the toy compiler stops defining the
platform while remaining useful as a smoke generator:

| Phase | Current Artifact | Gate |
| --- | --- | --- |
| Toy compiler retirement | `toolchain_roadmap.md`, `src/c_compiler.rs`, and private `__lnp_*` shim tests keep new native work out of ad hoc POSIX-shaped compiler features. | `c_private_lnp_manifest_intrinsics_lower_and_run` |
| Real toolchain target | `toolchain/lnp64_target.manifest`, psABI, relocation, object-format, crt, inline-asm, debug/unwind, intrinsic, isel, and exec-plan manifests. | `toolchain_contract_index_is_complete` |
| Minimal LLVM/Clang path | `toolchain/lnp64_llvm_bootstrap.manifest` pins the planned hello, arithmetic, memory, calls, and simple-libc replacement gates for the toy-compiler smoke path. | `llvm_bootstrap_manifest_names_first_clang_gate` |
| Libc/runtime shim layer | `libc_roadmap.md`, crt/startup manifest, and intrinsic manifest define startup, TLS/errno, allocation, FDR I/O, pthread/futex, event waits, mmap, signal, and socket lowering. | `scripts/run_software_gates.sh` |
| Software loader and exec plan | `src/loader.rs`, `object_format.md`, and `toolchain/lnp64_exec_plan.manifest` define the initial ELF64 parser and `EXEC` as a bounded hardware commit descriptor. | `exec_plan_manifest_matches_loader_boundary_contract` |
| NetBSD personality layering | `netbsd_personality_abi.md`, this roadmap, and the NetBSD system script keep the personality layered over native services. | `scripts/run_netbsd_personality_system.sh` |
| Conformance gates | `conformance_matrix.md`, `scripts/run_software_gates.sh`, and `scripts/run_all_gates.sh` enumerate host software, package, personality, RTL/proof, and whitespace gates. | `scripts/run_software_gates.sh` plus `scripts/run_all_gates.sh` |

The `minimal_llvm_clang_path` row is still marked planned. It is not complete
until Clang/lld can build and the software loader can run the small programs
listed in the first real-toolchain gate without the toy C compiler.

# LNP64 psABI v0

This document records the current emulator, LLVM/Clang, lld, loader, and
libc/runtime process ABI. It is a compatibility contract for repository tests
and package bring-up, not yet a final hardware ABI.

## Scope

The v0 psABI covers:

- integer calling convention used by the C compiler.
- stack and local storage convention used by compiled C.
- process entry page layout for `argc`, `argv`, and `envp`.
- thread pointer, `errno`, POSIX signal/gate-delivery frame, and auxv
  conventions.
- FDR inheritance and native capability surfaces.
- current dynamic loader and binary-format expectations.

## Register Model

| Register Class | Count | Width | psABI Role |
| --- | --- | --- | --- |
| GPR `r0`-`r31` | 32 | 64-bit | Integer, pointer, status, and temporary registers. |
| FDR `fd0`-`fd255` | 256 | capability/FDR slot | Hardware-owned descriptor and capability slots. |
| FPR `f0`-`f31` | 32 | implementation-defined | Present in the ISA; not part of the current C ABI. |
| VR `v0`-`v15` | 16 | implementation-defined | Present in the ISA; not part of the current C ABI. |

`r0` reads as zero. Writes to `r0` are ignored.

`r1` is the return address (`ra`). v2 dissolves the v1 `LR` special register and
`FLAGS`; the link lives in an ordinary GPR. `CALL sym` is `JAL r1, sym`
(`r1 = PC + 8`), `CALL rs` is `JALR r1, rs, 0`, and `RET` is `JALR r0, r1, 0`.
Non-leaf functions spill and reload `r1` with ordinary `SD`/`LD` like any GPR;
there is no `LR_GET`/`LR_SET` bridge.

`r30` is the dedicated backend scratch register: it is reserved (not
register-allocated) and is used by the compiler to materialize stack-adjustment
immediates and frame addresses in prologues, epilogues, and frame-index
expansion. Because it is caller-clobbered and not callee-saved, no value that
must survive a call is kept in `r30`.

`r31` is the stack pointer for compiler-generated locals and spills. The current
emulator protects `r31` from ordinary `write_reg` updates; the hardware design
allows it to be ordinary thread state with MMU-enforced bounds. Code that needs
portable v0 behavior should treat `r31` as runtime-owned and should not write it
directly.

There is no architectural `FLAGS`/condition-code register: comparisons write a
GPR (`SLT`/`SLTU`/`SLTI`/`SLTIU`) and branches read GPRs
(`BEQ`/`BNE`/`BLT`/`BGE`/`BLTU`/`BGEU`). All instructions are fixed 64-bit words;
the PC advances by 8. Atomics use hardware `LR.D`/`SC.D` load-reserved /
store-conditional, not faked load/op/store.

## Data Model

The LNP64 C ABI uses an LP64-like integer/pointer model with 64-bit words as
the default scalar storage unit:

| C Surface | v0 Size |
| --- | --- |
| pointer | 8 bytes |
| `long`, `size_t`, `time_t`, descriptor tokens | 8 bytes |
| default local scalar slot | 8 bytes |
| byte loads/stores | 1 byte |
| word loads/stores | 4 bytes |
| doubleword loads/stores | 8 bytes |

Aggregate layout is ABI-defined only for explicitly documented public records
in v0 and is still being hardened by real
package tests. Public psABI structs should be documented explicitly before they
are used across object or domain boundaries.

## Calling Convention

Integer and pointer arguments are passed in `r2` through `r9`.

LLVM lowering assigns at most the first eight integer or pointer arguments to
`r2..r9` before `JAL`/`JALR` (the `CALL` pseudo).
Additional fixed arguments are passed in 16-byte-aligned stack argument slots.
For variadic calls, non-fixed
arguments are copied into the caller stack argument area for the callee's
`va_start` and `va_arg` support.

Return values are placed in `r2`. Multi-register returns are not part of the C
ABI yet. Cross-domain gate profiles use bounded return registers through native
`GATE_RETURN`; `RET_CAP` is the source-level call-profile spelling.

The v2 C ABI defines a callee-saved (preserved) GPR set `s0`-`s9` =
`r18`-`r27`: a callee that uses any of these must save it on entry and restore
it before returning, so the register allocator may keep values live across a
call in an `s`-register instead of spilling them to the stack. Every other
caller-visible GPR is caller-clobbered: `r2`-`r17` and `r28`-`r29` (the integer
argument/return and temporary registers), plus the `ra` register `r1` and the
`r30` backend scratch register. Callers that need a caller-clobbered register's
value to survive a call must spill it explicitly. `r0` (zero) and `r31` (sp)
are not part of either set.

## Nonlocal Jumps

`setjmp` and `longjmp` are libc/psABI operations over ordinary user-visible
thread context. A plain `jmp_buf` may save only the state required to resume
the same live C thread image: `r31` as the stack pointer, `r1` as the return
link (`ra`), the callee-preserved GPRs `s0`-`s9` = `r18`-`r27` (so that a
`longjmp` reinstates the caller's saved register state), and user-visible FDR or
capability register values only if a future psABI explicitly makes them part
of the calling convention.

`jmp_buf` must not save or restore Resource Domain authority state, scheduler
state, waitable membership, gate continuation tokens unless they are explicitly
ordinary user-visible capabilities, engine operation ownership, in-flight
command state, raw VMA/MMU/TLB state, debug/trace/attestation authority,
privileged PCRs, hidden hardware delivery frames, or reset/recovery epochs.
`longjmp` is therefore just a user-context restore and branch within the same
live thread and process image. It has no supported meaning across thread,
domain, or `exec` boundaries.

The libc ABI reserves validation-cookie words in `jmp_buf` for thread
generation, process/image generation, and stack-bounds generation checks. When
stable selectors expose those generations, `setjmp` must capture them and
`longjmp` must cheaply validate them before restoring context; validation
failure aborts or faults rather than continuing. In v0 those words are reserved
and zeroed by the minimal shim, while `r31` and `r1` (the `ra` return link) are
the active restored state.

Corrupting `jmp_buf` remains C undefined behavior, but hardware must not treat
arbitrary bytes as freshly minted authority. Restored capability-like values are
only existing typed register/FDR/capability references; `longjmp` cannot mint,
widen, unseal, or refresh generations. `sigsetjmp(env, 1)` additionally saves
and restores the compatibility signal-delivery mask through the normal
`GET_PCR`/`SET_PCR` `SIGMASK` or gate-mask ABI. Plain `setjmp` does not save or
restore a signal mask.

## Address Materialization

v2 instructions are fixed 64-bit words, so a full 32-bit immediate is inline.
Compiler-generated materialization uses:

- small signed constant: `LI rd, imm32` (the assembler spelling of
  `ADDI rd, r0, imm32`); full 64-bit literal: `LI rd, lo32` then
  `LIU rd, rd, hi32`;
- direct address / PC-relative: `AUIPC rd, imm32` (one word,
  `rd = PC + sext32(imm32)`, relocated by `R_LNP64_AUIPC`) followed by an
  `ADDI`/`LD` against the formed base;
- descriptor or TLS offset through a slot: `AUIPC tmp, slot` followed by
  `LD rd, tmp, off`.

There is no `LA` instruction and no two-word `AUIPC`/`LI32`. LLVM, lld, object
tests, and the loader use the `R_LNP64_AUIPC` / instruction-count
`R_LNP64_BRANCH` / `R_LNP64_JUMP` relocations from `object_format.md`; they must
not create a second pseudo-address contract.

## Stack and Local Storage

`r31` points at the current thread's stack/local region. Compiler-generated
locals are addressed at positive offsets from `r31`; offset `8` is the first
ordinary local slot. Local scalars use 8-byte slots unless the compiler has
package-specific aggregate layout metadata for a larger object.

The emulator starts each thread with a stack top derived from the process
layout. ASLR can randomize stack placement; deterministic domain policy can
disable that randomization for tests.

The v0 ABI uses `r31` as the stack pointer and `LR` as the hardware return
link. Calls use register arguments first and stack argument slots for overflow
or variadic arguments; non-leaf functions spill `LR` through `LR_GET`/`LR_SET`.

## Debug and Unwind Minimum

The initial LLVM target should emit DWARF line tables and register mappings for
GPR `r0`-`r31` and `TP`. DWARF register numbers are `r0`-`r31` as
`0`-`31`, `r1` (ra) as DWARF register `1`, and `TP` as `33`. (v2 dissolved the
v1 `LR` special register; the return address lives in `r1`.) Non-leaf functions
should carry call-frame information sufficient to recover
`r31` as the CFA stack register and `r1` as the return address.
There is no v0 language exception runtime and
`.eh_frame` is not required for the first static C target. POSIX signal and
gate-delivery frames unwind through the psABI signal frame described below.

## Process Entry

The current emulator reserves the process entry page at `0x700000` with size
`0x20000`.

| Address | Content |
| --- | --- |
| `0x700000` | `argc` as a 64-bit little-endian word. |
| `0x700008` | `argv[0]` pointer. |
| `0x700008 + argc * 8` | Last `argv` pointer. |
| `0x700008 + (argc + 1) * 8` | Null `argv` terminator. |
| Next slot | `envp[0]` pointer. |
| After final environment pointer | Null `envp` terminator. |
| `0x701000` onward | NUL-terminated argument and environment strings. |

The static crt0 startup stub initializes C `main` parameters from this page:

- `argc` is loaded from `0x700000`.
- `argv` is the pointer table beginning at `0x700008`.
- `envp` is the first pointer slot after the null `argv` terminator.
- `environ`, when referenced, is initialized to the same `envp` pointer.

Static Clang/lld driver defaults use
`target/lnp64-sysroot/usr/lib/lnp64/crt0.o` as the packaged crt0 object.
`toolchain/crt0_lnp64.s` remains the checked source contract for that object.
It defines `_start`, loads `argc`, `argv`, and `envp` from the startup page,
clears TLS errno state, calls `main`, and exits through `EXIT`. Custom runtime
profiles may still provide their own `_start`, but hosted C coverage is
crt0/libc/runtime modeled rather than compiler-emitted startup.

## Auxv and Environment Metadata

`ENV_GET` is the architectural metadata path for machine facts and the opaque
startup metadata block. libc/runtime code implements `getauxval` by combining
direct `ENV_GET` machine keys with auxv records that the loader/personality
placed in that startup metadata block. Hardware does not index auxv entries.

`AT_RANDOM` intentionally returns zero through `getauxval`; secret entropy must
come from `RANDOM` or libc wrappers such as `getentropy`, `getrandom`, and
`arc4random_buf`.

Auxv key numbers and the dynamic-loader contract are not frozen in v0.

## TLS and Errno

The thread pointer is read and written through the `TP` PCR. `TLS_BASE` is the
compiler-facing PCR spelling for the same architectural value. LLVM may model it
as a target thread-pointer register after an explicit `GET_PCR TLS_BASE`
materialization or as a fixed live-in when the process-entry contract guarantees
the value is already resident.

Local-exec TLS is the required first TLS model:

- `GET_PCR r_base, TLS_BASE` materializes the thread pointer.
- If the TP-relative offset is encodable, the backend may use an immediate add.
- Otherwise the backend loads a linker-filled TP-relative offset slot using the
  canonical AUIPC+LD sequence and adds it to `r_base`.
- `R_LNP64_TLS_TPREL64` fills 64-bit TP-relative offset slots;
  `R_LNP64_TLS_TPREL_SLOT64` marks slots intended for the canonical local-exec
  materialization path.

General-dynamic and initial-exec TLS are future loader/personality models, not
v0 backend requirements.

`GET_PCR` and `SET_PCR` are compiler-visible control operations, not ambient
runtime calls. `GET_PCR r_result, selector` returns the selected value.
`SET_PCR r_result, selector, r_src` returns `0` on success or a negative
architectural error on failure. Writes to read-only selectors fail with `-EPERM`
without trapping or mutating state. The first LLVM backend must model `SET_PCR`
as defining the encoded result register and clobbering only the selected PCR
when the operation succeeds.

`errno` is hardware-thread-local through `ERRNO_GET` and `ERRNO_SET`, but it is
a C/POSIX compatibility view rather than the native ISA error channel. Native
operations return negative architectural errors in their encoded result register.
The C runtime translates those native errors to POSIX `-1` plus thread-local
`errno` at public API boundaries. A global `errno` symbol, when present in C
source, is synchronized with that hardware-thread-local errno path by compiler
lowering.

## Native Capability Call ABI

Native capability calls are compiler-visible intrinsics over `GATE_CALL` and
`GATE_RETURN`, not syscalls.

- Call instruction shape: `GATE_CALL r_result, r_gate_fd, r_arg0, r_arg1`.
- `r_gate_fd` holds a call-gate FDR/capability handle.
- `r_arg0` and `r_arg1` carry the two fast scalar argument words. Larger payloads,
  buffers, and capabilities are passed through pre-authorized FDRs or typed
  argument records named by those registers.
- Synchronous success returns a nonnegative result or small operation value in
  `r_result`; failure returns a negative architectural error.
- Asynchronous success returns a nonnegative operation id, event token, or
  completion handle as defined by the gate profile. Completion payloads are read
  from the configured event/counter/queue object with `AWAIT_EX` and `PULL` or
  profile-specific `GET_META`.
- Handoff success returns zero or a nonnegative handoff token when the caller
  remains live; otherwise the current activation is ended according to the gate
  profile.
- `GATE_RETURN r_result, r_value0, r_value1` returns two fast scalar result words
  to a trusted continuation. `r_result` reports the return commit status using
  the same nonnegative-success/negative-error convention.
- All caller-visible GPRs except the encoded result register are caller-clobbered
  unless a future psABI revision defines a callee-save set. `LR`, `TP`, and stack
  pointer semantics follow the ordinary function ABI unless the gate profile
  explicitly enters a separate domain/thread context.

## Signals

LNP64 v1 freezes a small Unix-signal ABI subset as a profile over native
Gate/Continuation delivery. Signal handlers receive the signal number in `r1`,
matching the first integer argument register. `SIGRET` is the POSIX spelling of
`GATE_RETURN` and restores the Gate/Continuation Engine's saved interrupted
context for the current thread.

The hardware design requires a saved gate-delivery context containing at least the
saved context token/generation, faulting PC, signal number/code, bad address
where relevant, trapped instruction word where relevant, source PID/TID/domain
where permitted, event/fault id, and the GPR/FPR/VR state needed by this psABI.
The emulator implements the POSIX signal profile and keeps signal-frame stack
areas non-executable.

The frozen subset includes handler/default/ignore dispositions, per-thread
masks, process-directed and thread-directed pending state, fault-to-signal
mapping, `raise`/`kill`-style software injection, `alarm`, fixed handler entry,
and `SIGRET`. User-visible frame memory is diagnostic/runtime ABI data;
`SIGRET`/`GATE_RETURN` uses the Gate/Continuation Engine-owned context
token/generation.

Full POSIX realtime queueing, OS-specific syscall restart behavior, arbitrary
`sigaltstack` ABI variants, and Linux/BSD-specific delivery corner cases remain
outside the frozen v0 psABI. A libc or Unix personality may emulate them over
event queues and compatibility metadata.

Native async runtimes should use event queues, counters, queues, cancellation
objects, and gate profiles directly instead of POSIX signal delivery.

## FDR Inheritance and Capabilities

FDR slots are authority-bearing hardware entries, not Unix integers alone. The
C/POSIX layer may expose dynamic descriptor tokens for descriptors that cannot
be encoded as a static `fdN` operand. Tokens include generation information so
stale descriptor reuse fails.

`fork`-like cloning is the `CLONE profile=posix_fork` compatibility profile.
It creates a new PID with exactly one child thread. FDR entries are inherited
only according to their descriptor inheritance flags and retain object
generation, rights, event masks, transfer rights, and Resource Domain scope.
In-flight operation ownership is not copied.

`exec` preserves only descriptors not marked close-on-exec plus explicit startup
FDR grants from the exec-plan descriptor. It installs a fresh startup metadata
block and does not reinterpret inherited descriptors as ambient authority.
The exec-plan is prepared by a loader/runtime/personality; hardware validates
and commits it atomically. If validation fails before the hardware commit point,
the old process image remains active.

## Native Clone Profiles

`CLONE` is a native primitive with explicit profiles. POSIX process and thread
APIs are compatibility lowerings onto these profiles; native runtimes should name
the profile they need instead of calling a POSIX-shaped surface.

| Profile | Compatibility Surface | Semantics |
| --- | --- | --- |
| `new_process_cow` | `fork()` | New PID, one child thread, copy-on-write address-space snapshot, inherited FDRs according to descriptor metadata. |
| `new_thread_shared_vm` | `pthread_create()` | New thread in the same process/domain with shared VM, FDR table, and process metadata. |
| `spawn_entry` | native runtime spawn | New thread-like execution context starting at an explicit entry PC. |
| `domain_task` | native domain scheduler | Domain-owned task profile; not exposed through the C compatibility layer in v0. |

The backend-facing target manifest records these exact spellings in
`clone_profiles`. Loader, libc, and runtime code may map higher-level process,
thread, servicelet, or domain-task abstractions onto them, but the hardware
profile selection is explicit.

Native capability movement uses `CAP_DUP`, `CAP_SEND`, `CAP_RECV`, and
`CAP_REVOKE`. Delegation may narrow rights, transfer permission, ranges, event
masks, and mapping permissions. Sealed capabilities may be used or transferred
according to their rights but cannot be duplicated or narrowed by ordinary
receivers.

Capability tokens and native extension APIs must preserve the architectural
lineage model: object generation, capability generation, lineage root, lineage
epoch, rights, range, event mask, mapping permissions, transfer/seal/narrow
flags, and Resource Domain scope. Revocation advances the lineage or
revocation-root epoch. Operations issued before their commit point fail with
`EREVOKED` or the object-specific stale-reference error; operations after commit
complete according to the object's documented teardown rule, and later use sees
the stale generation/epoch.

## Dynamic Loader Expectations

There is no dynamic linker ABI in v0. Optional dynamic loading APIs such as
`dlopen`, `dlsym`, and `dlclose` fail cleanly in the current libc surface.

The v0 package bring-up path is static Clang/lld-linked LNP64 ELF plus small
compiler-emitted or hand-written LNP64 assembly where the real backend still
needs an explicit contract smoke.
Future dynamic loading is a software loader/personality contract, not a
hardware `EXEC` contract. Hardware accepts a bounded exec-plan descriptor and
opaque startup metadata; it does not parse ELF, dynamic-linker state,
relocations, interpreters, shebangs, library graphs, or Unix credential
transition policy. Loader/personality code owns ASLR layout choices, relocation
correctness, startup metadata shape, auxv contents, and FDR grant selection;
hardware owns capability/generation/lineage checks, W^X/NX/provenance checks,
Resource Domain policy checks, and atomic image commit. Future dynamic loading
must define:

- auxv keys consumed by the loader.
- relocation records and symbol binding rules.
- executable mapping source policy.
- FDR/capability startup descriptors.
- TLS module layout.
- destructor/fini ordering.

## Binary and Object Format v0

The emulator currently loads LNP64 assembly programs, not ELF binaries.
`object_format.md` defines the target static v1 software-loader ELF profile,
relocation model, executable mapping permissions, ASLR loader behavior,
dynamic-linking boundary, startup descriptor records, and the boundary between
loader-owned format policy and hardware `EXEC` commit.

Until those details are complete, real-package gates should continue to compile
through the real Clang/lld path and run through the software loader or explicit
object/static-link gates. The deleted in-repo C compiler is not a package
bring-up path.

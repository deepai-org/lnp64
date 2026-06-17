# LNP64 psABI v0

This document records the current emulator and C compiler process ABI. It is a
compatibility contract for repository tests and package bring-up, not yet a
final hardware ABI.

## Scope

The v0 psABI covers:

- integer calling convention used by the C compiler.
- stack and local storage convention used by compiled C.
- process entry page layout for `argc`, `argv`, and `envp`.
- thread pointer, `errno`, signal frame, and auxv conventions.
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

`r31` is the stack pointer for compiler-generated locals and spills. The current
emulator protects `r31` from ordinary `write_reg` updates; the hardware design
allows it to be ordinary thread state with MMU-enforced bounds. Code that needs
portable v0 behavior should treat `r31` as runtime-owned and should not write it
directly.

`LR` is a thread-local link register outside the numbered GPR file. `CALL` and
`CALL_REG` set `LR = PC + 8`; `RET` jumps to `LR`.

## Data Model

The current C compiler uses an LP64-like integer/pointer model with 64-bit words
as the default scalar storage unit:

| C Surface | v0 Size |
| --- | --- |
| pointer | 8 bytes |
| `long`, `size_t`, `time_t`, descriptor tokens | 8 bytes |
| default local scalar slot | 8 bytes |
| byte loads/stores | 1 byte |
| word loads/stores | 4 bytes |
| doubleword loads/stores | 8 bytes |

Aggregate layout is compiler-defined in v0 and is still being hardened by real
package tests. Public psABI structs should be documented explicitly before they
are used across object or domain boundaries.

## Calling Convention

Integer and pointer arguments are passed in `r1` through `r6`.

The current compiler evaluates at most the first six arguments into temporary
spill slots, reloads them, and moves them into `r1..r6` before `CALL` or
`CALL_REG`. Additional C varargs are copied into a compiler-managed varargs
area for the callee's `va_start`/`va_arg` support; they are not passed on a
hardware stack by generic call lowering.

Return values are placed in `r1`. Multi-register returns are not part of the C
ABI yet. Cross-domain call gates use their own bounded return registers through
`RET_CAP`.

The current compiler treats GPRs other than `r0` and `r31` as caller-clobbered.
There is no callee-saved GPR set in the v0 compiler ABI. Runtimes that need
stable register state across calls must spill it explicitly.

## Stack and Local Storage

`r31` points at the current thread's stack/local region. Compiler-generated
locals are addressed at positive offsets from `r31`; offset `8` is the first
ordinary local slot. Local scalars use 8-byte slots unless the compiler has
package-specific aggregate layout metadata for a larger object.

The emulator starts each thread with a stack top derived from the process
layout. ASLR can randomize stack placement; deterministic domain policy can
disable that randomization for tests.

The v0 compiler does not build a conventional downward-growing C call stack for
arguments. Function calls rely on registers, compiler-managed spill slots, and
the hardware `LR`.

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

For C `main`, the compiler initializes parameters specially:

- `argc` is loaded from `0x700000`.
- `argv` is the pointer table beginning at `0x700008`.
- `envp` is the first pointer slot after the null `argv` terminator.
- `environ`, when referenced, is initialized to the same `envp` pointer.

If a source file defines `_start`, the compiler emits `_start` as the entry
symbol before `main`; otherwise it emits `main` first. In v0 there are no
standalone crt object files. Startup is compiler/runtime modeled.

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

The thread pointer is read and written through the `TP` PCR. The C compatibility
surface uses this for thread-specific storage tests.

`errno` is hardware-thread-local through `ERRNO_GET` and `ERRNO_SET`. A global
`errno` symbol, when present in C source, is synchronized with the hardware
errno path by compiler lowering.

## Signals

LNP64 v1 freezes a small Unix-signal ABI subset. Signal handlers receive the
signal number in `r1`, matching the first integer argument register. `SIGRET`
restores the Signal Engine's saved interrupted context for the current thread.

The hardware design requires a saved signal context containing at least the
saved context token/generation, faulting PC, signal number/code, bad address
where relevant, trapped instruction word where relevant, source PID/TID/domain
where permitted, event/fault id, and the GPR/FPR/VR state needed by this psABI.
The emulator implements signal delivery and keeps signal-frame stack areas
non-executable.

The frozen subset includes handler/default/ignore dispositions, per-thread
masks, process-directed and thread-directed pending state, fault-to-signal
mapping, `raise`/`kill`-style software injection, `alarm`, fixed handler entry,
and `SIGRET`. User-visible frame memory is diagnostic/runtime ABI data;
`SIGRET` uses the Signal Engine-owned context token/generation.

Full POSIX realtime queueing, OS-specific syscall restart behavior, arbitrary
`sigaltstack` ABI variants, and Linux/BSD-specific delivery corner cases remain
outside the frozen v0 psABI. A libc or Unix personality may emulate them over
event queues and compatibility metadata.

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

The v0 package bring-up path is static or compiler-emitted LNP64 assembly.
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

Until those details are implemented, real-package gates should compile through
the repository C compiler to LNP64 assembly.

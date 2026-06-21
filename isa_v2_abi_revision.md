# LNP64 ISA v2 — ABI revision proposal: a callee-saved register class

Status: **proposed**. This is a follow-up to the v2 migration, flagging the one
ABI weakness that surfaced as both a correctness bug and a codegen-efficiency
problem during the LLVM backend bring-up.

## The problem (evidence from the migration)

The v2 ABI as shipped designates **no callee-saved registers** — every GPR is
caller-saved (the psABI literally records "no callee-saved set"). Two concrete
consequences showed up while getting the toolchain to a working executable:

1. **Correctness.** With `r1` (ra) allocatable and no callee-saved convention,
   the register allocator reused `r1` as a scratch in leaf functions (which do
   not spill it) and clobbered the return address — `ret` jumped to garbage. We
   had to reserve `r1` outright. That works, but it is a symptom of the missing
   convention.

2. **Efficiency (the real cost).** With zero callee-saved registers, **every
   value live across a call must be spilled to the stack** — there is nowhere
   else to hold it. In the trivial `write()` smoke, `main` spilled three values
   around a single call:

   ```
   sd r1, 16(r31)     # ra
   sd r4, 0(r31)      # len, live across the call
   jal r1, write
   ld  r3, 0(r31)     # reload
   sd  r2, 8(r31)     # return value, spilled again
   ...
   ```

   Real code calls constantly; this convention taxes every non-leaf function
   with stack traffic it should not need. It looks fine on toy tests and
   silently slows everything.

## The proposal: a RISC-V-style register classification

Partition the 32 GPRs into the conventional roles (exact split is tunable; this
is a sane default):

| Regs | Role | Saver |
| --- | --- | --- |
| `r0` | hardwired zero | — (reserved) |
| `r1` | `ra` — return address / link | dedicated (reserved) |
| `r2`–`r9` | integer args `a0`–`a7`; `r2` also return value | caller |
| `r10`–`r17` | temporaries `t0`–`t7` | caller |
| **`r18`–`r27`** | **saved `s0`–`s9`** | **callee** ← new |
| `r28`–`r29` | temporaries `t8`–`t9` | caller |
| `r30` | backend scratch / `gp` | reserved |
| `r31` | stack pointer `sp` | reserved |

This yields **10 callee-saved registers**, 8 argument registers, and ~10
caller-saved temporaries — comfortable for an integer ISA.

## Why it fixes both problems

- **No more forced cross-call spills.** A value live across a call is allocated
  to an `s`-register and simply survives the call; the callee that wants to use
  that `s`-register saves/restores it once in its own prologue/epilogue. The
  generic LLVM prologue-epilogue inserter handles this automatically once
  `getCalleeSavedRegs` returns the `s`-set — no manual frame code.
- **`r1` becomes cleanly handleable.** With a proper convention in place, the
  return-address register is the standard `ra` case the backend already knows
  how to model; the leaf-clobber hazard goes away by construction rather than by
  reserving `r1` as a special case (though keeping `r1` dedicated is also fine
  and simplest).
- **Familiar codegen path.** This is exactly the RISC-V/Alpha shape LLVM is most
  battle-tested on; less custom backend code, fewer surprises.

## Migration impact (small, and isolated to software)

The ABI is a software convention — **the emulator and RTL do not change** (the
hardware does not care which registers are callee-saved). Affected layers:

- **LLVM backend:** `getCalleeSavedRegs` returns `r18`–`r27`; `LNP64CallingConv.td`
  marks the `CSR` set and the caller-saved/arg/temp split; the allocator then
  prefers callee-saved registers for cross-call values and PEI auto-emits the
  save/restore. The manual `r1` spill in `LNP64FrameLowering` can stay or fold
  into the CSR machinery.
- **psABI** (`psABI.md` + `toolchain/lnp64_psabi.manifest` + `lnp64_registers.manifest`):
  replace "no callee-saved set" with the table above; update the conformance
  tests that pin the register roles.
- **Runtime that hand-saves registers:** `toolchain/liblnp64_setjmp_min.s` must
  save/restore the **callee-saved set** (`r18`–`r27`) in `jmp_buf`, not just
  `sp`+`ra` — a `longjmp` has to restore the caller's `s`-registers. This is the
  main hand-written change. `crt0` is unaffected (it makes one call and exits).
- **DWARF:** the CFI for callee-saved spills (already emitted per-register by
  PEI) just covers more registers.

## Recommendation

Lock this in **before** more software and compiled artifacts harden around the
current "everything caller-saved" model. It is a one-time, well-understood ABI
decision with a large, permanent payoff in code quality, and it is far cheaper
to land now (one libc shim + the psABI + the backend register info) than after
a larger userland exists. The `setjmp` save-set is the only non-mechanical part.

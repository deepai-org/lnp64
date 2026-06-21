# LNP64 ISA + ABI — master change list

Single consolidated list of every ISA/ABI change identified during the v2
migration, so none are forgotten while the architecture is still malleable.
Status: **done** / **pending (implement)** / **decision needed**.

## A. ABI — register conventions

| # | Change | Status |
| --- | --- | --- |
| A1 | `r0`=zero, `r31`=sp, `r30`=backend scratch, `r1`=ra (dedicated link reg, reserved) | **done** |
| A2 | Integer args in `r2`–`r9`; return value in `r2` | **done** (LLVM); **pending** in hand-asm (see C) |
| A3 | Syscall/native-instruction results go to the instruction's `rd` (default `r2`), **never `r1`** | **done** (emulator + RTL + LLVM) |
| A4 | **Callee-saved register class** — e.g. `r18`–`r27` as `s0`–`s9`; eliminates forced cross-call spills; lets PEI manage save/restore | **decision needed** → then implement (see `isa_v2_abi_revision.md`) |
| A5 | **Signal-handler argument register** — handler currently receives the signum in `r1` (v1-flavored, in both emulator+RTL). Inconsistent with the C handler ABI (`void h(int)` → `r2`/a0) and with A3 | **decision needed** |
| A6 | `exit`/`_exit` status code in `r2` (a0), not `r1` | **done** in crt0; **pending** in `liblnp64_min.s` (C) |

## B. ISA / microarchitecture (from the design discussion)

| # | Change | Status |
| --- | --- | --- |
| B1 | Eliminate `FLAGS`; compares→GPR (`SLT*`), reg-compare branches | **done** |
| B2 | Fixed 64-bit, one-word decode; LR/SC atomics; LIU/AUIPC; sign-ext loads | **done** |
| B3 | **Uniform instruction timing / no speculation** — in-order, non-speculative; remove the RTL return-address/branch predictor (`return_stack` as a *predictor*); a `ret` just reads `r1` at fixed cost. Performance comes from instruction count, not prediction | **decision needed** |
| B4 | **Memory model** — keep timing honest by exposing the hierarchy: scratchpad/TCM with fixed load/store latency + explicit DMA, rather than a transparent cache (which reintroduces data-dependent timing). RTL already has a flat `DATA_SRAM` region as the seed | **decision needed** |
| B5 | **Capability/gate ABI freeze** — freeze the `GATE_CALL`/`GATE_RETURN`/capability-transfer convention in silicon (the high-leverage, philosophy-consistent freeze for a hardware-OS). Partly there already | **decision needed (scope)** |

## C. Hand-written runtime / asm — bring to the v2 ABI

| # | Item | Status |
| --- | --- | --- |
| C1 | `toolchain/liblnp64_min.s` — written in the **v1 ABI** (uses `r1` as arg0/return *and* `jal r1` for the link → broken under v2). Full rewrite to args `r2`–`r9`, return `r2`, save/restore `r1`(ra) across nested calls | **pending** |
| C2 | `exit r1`→`exit r2` in `liblnp64_min.s` + the decode-smoke greps (`run_real_llvm_lnp64.sh:530,5020`, `run_real_llvm_bootstrap_smokes.sh:72`) + assertions (`lowering.rs:2921,5793`) | **pending** |
| C3 | Audit `demos/*.s` for v1-ABI arg/return-in-`r1` (mnemonics were ported; ABI usage not fully audited) | **pending (audit)** |
| C4 | `liblnp64_setjmp_min.s` must save/restore the **callee-saved set** in `jmp_buf` once A4 lands (currently saves only sp+ra) | **pending (after A4)** |

## E. LLVM cleanliness (top priority)

Making the LLVM backend *extremely clean* is a stated top priority. It resolves
several decisions above and adds its own:

| # | Change | Status |
| --- | --- | --- |
| E1 | **Callee-saved register class (A4) — do it.** This is the single biggest LLVM-cleanliness lever: it removes the reserved-`r1` hack, the forced cross-call spills, and the leaf-clobber bug class, and lets the *generic* PEI manage save/restore. It is the standard RISC-V-shaped model LLVM is built around. | **decision → yes (recommended)** |
| E2 | **TableGen-declarative MC layer.** Replace the hand-written `LNP64MCCodeEmitter` (200-line switch) and `LNP64AsmParser` (`StringSwitch`) with `bits<64> Inst` fields + the generated encoder / disassembler / `AsmMatcher`. One declarative source of truth for encoding; far less hand code; also the source the Coq `decode` can be generated from. | **pending (deferred in migration; elevate)** |
| E3 | **Minimize custom inserters.** Audit the remaining `EmitInstrWithCustomInserter` users (`PseudoLI64`, `PseudoSELECT_CC`, LR/SC emit). Keep only what truly cannot be a plain pattern. | **pending (audit)** |
| E4 | **No instruction duality / special cases** (D1 bootstrap forms) — fewer opcodes the backend must special-case = cleaner tables. | see D1 |
| E5 | Standard frame lowering — once E1 lands, the manual `r1` prologue spill folds into the generic callee-saved-spill path; delete the bespoke code. | **pending (after E1)** |

## D. Smaller warts

| # | Item | Status |
| --- | --- | --- |
| D1 | Bootstrap-instruction duality (`mmap`/`mmap_bootstrap`, `munmap`/`mprotect` bootstrap forms) — decide whether both forms stay first-class or unify | **decision needed (low priority)** |
| D2 | Two assembler memory syntaxes (`[base,off]` in the Rust asm vs `off(base)` in LLVM) — unify on one grammar | **pending** |
| D3 | `MULHSU` defined but pattern-less (unselectable) — wire a pattern or drop it | **pending (minor)** |

## Decisions (locked)

- **A4 / E1 — Callee-saved class: YES.** `r2`–`r9` args, `r10`–`r17` + `r28`–`r29`
  temps, **`r18`–`r27` callee-saved (`s0`–`s9`)**. setjmp save-set updated (C4).
- **E2 — TableGen-declarative MC layer: YES, now.** Retire the hand-written
  `MCCodeEmitter`/`AsmParser`; `bits<64> Inst` → generated encode/decode/match.
- **A5 — Signal-handler arg → `r2`** (consistent C handler ABI + A3).
- **B3 — Uniform timing: YES.** In-order, non-speculative; remove the RTL
  return-address/branch predictor. `ret` reads `r1` at fixed cost.
- **B4 — Memory model: transparent cache.** Scratchpad-explicit would need a
  compiler that doesn't exist. Accept the split: **deterministic non-speculative
  core + cached memory** (loads vary, pipeline does not). Cache side-channel
  partitioning is a later hardening pass, not a v1 blocker. (No immediate RTL
  change — the current flat-SRAM model is the placeholder until a cache is
  designed; the decision is the *direction*, away from scratchpad.)

## Already validated end-to-end
Full v2 toolchain builds, the libc sysroot compiles + links, and the `write()`
sysroot smoke runs to `exit=0` (no SIGSEGV); `cargo` 471/2 (2 pre-existing);
RTL↔emulator cosim byte-exact. The pending items above do not regress that
baseline.

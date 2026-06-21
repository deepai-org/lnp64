# LNP64 ISA v2 — Migration File Inventory

Concrete file/component checklist for executing the v2 ISA migration. This is
the companion to [`isa_v2_design.md`](isa_v2_design.md) (the spec) and
[`isa_v2_design.md` §7](isa_v2_design.md) (the layer order). Where §7 lists the
*order*, this lists the actual *files* to touch, grounded in the current tree.

Each layer is split into **regenerated** (falls out of the TableGen `.td`
rewrite — cheap) vs **hand-ported** (real work). The gating rule from §7 holds:
no layer lands until the layer before it passes its gate against the emulator
oracle.

## 1. Spec — done

- `isa_v2_design.md` — the source of truth.

## 2. LLVM backend — `llvm/lib/Target/LNP64/` (center of gravity)

Hand-rewrite:

- **`LNP64InstrInfo.td`** — 64-bit formats, `bits<64> Inst` per instruction;
  delete the 40 `CSET*`/`CSEL*`/`Pseudo*`; add `SLT*` / `BEQ…BGEU` / `LI` /
  `LIU` / `AUIPC` / `JAL` / `JALR` / `LR` / `SC` / sign-extending loads.
- **`LNP64ISelLowering.cpp`** — delete the `EmitInstrWithCustomInserter`
  compare/select/branch glue and `PseudoLI64`/`PseudoLINeg32`; wire
  `AtomicExpand` → LR/SC; `setcc`/`br_cc` become plain patterns.
- **`LNP64RegisterInfo.td` / `.cpp`** — delete `LR`/`FLAGS`/`SPECIAL`; `ra=r1`;
  reclaim `r30`; fix `AltOrders` to `(sub GPR, R0, R31)`; delete the
  large-offset address-materialization spill path.
- **`LNP64InstrInfo.cpp`** — `copyPhysReg` / spill slots (drop the LR special
  case; GPR↔GPR `MOV` only).
- **`LNP64FrameLowering.cpp`** — delete the `LR_GET`/`LR_SET`→`r30` bounce;
  `Size=8`; `pc+8` returns.
- **`LNP64CallingConv.td`** — `ra=r1`, call-clobber sets.
- **`MCTargetDesc/LNP64MCAsmBackend.cpp`** — **fixup kinds change**: v1
  `branch26` / `abs32` / `pcrel32` → new widths/positions for 64-bit words and
  `<<3` instruction-count offsets. (Easy to miss; correctness-critical.)
- **`MCTargetDesc/LNP64MCAsmInfo.cpp`** — instruction width / code-pointer
  settings.
- **`LNP64TargetMachine.cpp` / `LNP64Subtarget.cpp`** — datalayout, if it
  changes.

Retire → TableGen-generated (stop being hand-written):

- `MCTargetDesc/LNP64MCCodeEmitter.cpp` → generated encoder.
- `AsmParser/LNP64AsmParser.cpp` → generated `AsmMatcher`.
- `Disassembler/LNP64Disassembler.cpp` → generated.
- `InstPrinter/LNP64InstPrinter.cpp` → generated.

## 3. Clang frontend

- `clang/lib/Basic/Targets/LNP64.cpp` / `.h` — register names for inline-asm
  constraints (`r1`/`r30` role change), builtins, datalayout string.
- `clang/lib/Driver/ToolChains/Arch/LNP64.cpp`.

## 4. Linker — `lld/ELF/Arch/LNP64.cpp`

Relocation handling must match the new MC fixup kinds and 64-bit instruction
encoding. Small but correctness-critical; hand-ported.

## 5. Emulator — `src/emulator.rs` (single file, ~910 KB) — the reference oracle

- Decode: `decode`, `load_exec_u32`, the opcode `match` → 64-bit fixed fetch,
  no second-word path.
- Execute: per-`Instr` arms → `FLAGS`→GPR compares, LR/SC + `reservation_addr`,
  `LIU` / `AUIPC`, sign-extending loads.

Biggest hand-port; gates every layer below it.

## 6. Hand-written assembly + runtime (none of this regenerates)

- `toolchain/crt0_lnp64.s` — startup; uses v1 mnemonics / `LR` / `FLAGS`.
- `toolchain/liblnp64_min.s` — native runtime shim.
- `toolchain/liblnp64_setjmp_min.s` — **hard-codes the register save set** →
  `ra=r1` change.
- `demos/*.s` — ~10 hand-written assembly demos.
- `toolchain/liblnp64_*_min.c` — C; just recompile (cheap).
- `psABI.md`, `netbsd_personality_abi.md` — `ra=r1`, save/restore sets, 8-byte
  instruction width.

## 7. RTL — `rtl/`

- `rtl/include/lnp64_pkg.sv` — opcode enum (`LNP64_OP_*`).
- `rtl/core/lnp64_decode.sv` — decode FSM → 64-bit fixed fetch (where the
  "trivial bounded-progress decode" win lands).
- `rtl/core/lnp64_core_tile.sv`, `rtl/top/lnp64_top.sv` — fetch width, PC+8.
- `rtl/formal/*`, `rtl/sim/*` — mostly M-engine mediation semantics (not scalar
  decode), but their embedded testbench programs use ISA encodings, so the
  `tests/rtl/*.f` filelists and programs ripple.

## 8. Koika — `proofs/koika/lnp64_mediation.v`

**Verified abstract (no v1 ISA trace).** 113 lines; models capability
registers / mediation, not scalar instruction decode or opcodes. Nothing to
remove for the v1→v2 migration. (Renumbering or extending it is Phase-1
refinement work, not v1 cleanup.)

## 9. Coq — `proofs/coq/`

**Verified abstract (no v1 ISA trace).** `CapSpec.v` / `CapImpl.v` model
capabilities as `{lo, hi, w}` with four abstract ops (Write/Derive/Revoke/Nop).
There is **no concrete instruction encoding, register file, `FLAGS`, `LR`, or
`decode` function** in the Coq layer, so there is nothing v1-specific to remove.
Design §5's "regenerate decode / `MachineState` delta (remove flags/LR, add
reservation_addr)" describes a **future concrete model that does not exist
yet** — it is not part of the v1→v2 mechanical migration.

Genuine v2 Coq work, all **additive / deferred Phase-1 scope** (out of scope for
"leave no trace of v1"):

- Extend the `Cap` record with read/execute permission to support §3.2's
  PCC-less literal-load authorization (the gap found during PCC verification).
- **New** `scripts/td_to_coq_decode.py` — generate a Coq `decode` from the `.td`
  once a concrete decoder model is introduced.

These are tracked here for completeness but are NOT required to call the v1→v2
migration complete.

## 10. Conformance harness + witnesses

- `toolchain/lnp64_conformance_gates.manifest`, `conformance_matrix.md`.
- `scripts/run_real_llvm_lnp64*.sh` (docker `llvm-mc`/objects gates),
  `scripts/run_rtl_top_*.sh` smokes, `scripts/check_rtl_*_witness.py`, and the
  committed typed-trace witnesses.

## 11. Docs

- `hardware_design.md` §6 — replace with the v2 §2 encoding (retire both the
  legacy 64-bit-word format *and* the §6.0 32-bit description).
- `feature_readiness.md` and the roadmap `.md`s.

## Effort & risk summary

- **Cheap (regenerated from the `.td`):** encoder, disassembler, parser,
  instr-info.
- **Expensive, hand-ported, gating:** `emulator.rs` (oracle),
  `LNP64InstrInfo.td` + `LNP64ISelLowering.cpp`, RTL decode, Coq, and — easy to
  underestimate — the hand-written `.s` runtime/demos (crt0, setjmp,
  liblnp64_min) plus `lld` relocations / MC fixups.
- **Highest-risk-to-forget (compiles clean, breaks at link/run time):** MC
  fixup kinds (§2), lld relocations (§4), clang inline-asm register roles (§3),
  and the hand `.s` save-sets (§6).

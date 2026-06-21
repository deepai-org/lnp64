# LNP64 ISA v2 — Design Specification

Status: **proposed / locked for review**. This document is the single source of
truth for the v2 instruction set. Every other layer — reference emulator
(`src/emulator.rs`), LLVM backend (`llvm/lib/Target/LNP64`), RTL
(`rtl/`), Koika mediation core, and the Coq/Kami proofs — derives from it.
Where any layer disagrees with this document, this document wins; the layer is
the bug.

v2 is a deliberate, one-batch break from v1. v1 is not yet frozen by released
software, and Phase 1 formal proofs have not started, so we pay the migration
cost once, now, and let Phase 1 prove against v2 instead of throwing v1 proofs
away.

## 1. Design goals

v2 removes every v1 construct that forced compiler workarounds *and* complicated
the formal model. The guiding principle is **RISC-V-class regularity**:

1. **Pure 32-bit fixed-width decode.** No instruction is ever wider than one
   32-bit word. Instruction fetch/decode is a total 1-to-1 function of a single
   word — no stateful "fetch the second literal word" FSM. This is the single
   most important property for proving bounded-progress in the decoder.
2. **No implicit architectural condition state.** No `FLAGS` register.
   Comparisons write GPRs; branches read GPRs.
3. **One uniform register file.** The return address is a normal GPR. No
   special link register, no permanently reserved compiler scratch register.
4. **Honest atomics.** Hardware-supported LR/SC; the compiler never fakes
   atomicity.
5. **Declarative encoding.** Instruction bit layouts live in TableGen
   `bits<32> Inst` fields and are the source from which the Coq `decode`
   function is mechanically generated.

## 2. Instruction encoding

All instructions are exactly 32 bits, little-endian in memory. The opcode is
the high byte. Register fields are 5 bits (GPR 0..31). Six formats:

```
 31    24 23  19 18  14 13   9 8       0
+--------+------+------+------+----------+
| opcode |  rd  | rs1  | rs2  |  funct9  |   R-type   (reg, reg, reg)
+--------+------+------+------+----------+
| opcode |  rd  | rs1  |     imm14       |   I-type   (reg, reg, simm14)
+--------+------+------+------+----------+
| opcode |imm_hi| rs1  | rs2  |  imm_lo  |   S-type   (store: simm14 = {imm_hi[4:0], imm_lo[8:0]})
+--------+------+------+------+----------+
| opcode |imm_hi| rs1  | rs2  |  imm_lo  |   B-type   (branch: simm14 = {imm_hi, imm_lo}, scaled x4)
+--------+------+------+-----------------+
| opcode |  rd  |        imm19           |   U-type   (LUI, AUIPC)
+--------+------+-----------------------+
| opcode |  rd  |        imm19           |   J-type   (JAL; offset = simm19 x4)
+--------+--------------------------------+
| opcode |            imm24              |   Jump-only (JMP; offset = simm24 x4)
+--------+--------------------------------+
```

Field notes:

- **I-type immediate** is `imm[13:0]`, sign-extended to 64 bits. Range
  `[-8192, 8191]`.
- **S/B-type immediate** is split (`imm_hi` = bits[23:19], `imm_lo` =
  bits[8:0]) to keep `rs1`/`rs2` in their canonical slots. Reassembled to a
  14-bit signed value. B-type is scaled by 4 (word-aligned targets), giving a
  conditional-branch reach of **±32 KiB**.
- **U-type immediate** is `imm[18:0]` (19 bits), placed in the high bits:
  `LUI rd, imm19` sets `rd = sext64(imm19 << 13)`.
- **J-type** offset = `sext(imm19) * 4` → **±1 MiB** (call/jump-and-link).
- **Jump-only** offset = `sext(imm24) * 4` → **±32 MiB** (the long
  unconditional jump used as the branch-relaxation trampoline).

### 2.1 Constant materialization (kills v1's 8-byte `LI32`/`LA`/`AUIPC`)

- **≤14-bit signed:** `ADDI rd, r0, imm14`.
- **32-bit:** `LUI rd, hi19` ; `ADDI rd, rd, lo14` with the standard hi/lo carry
  adjustment (if `lo14`'s sign bit is set, increment `hi19`). The 19-bit upper
  and 14-bit lower overlap at bit 13 and tile a full 32-bit value.
- **64-bit and address-of-global:** PC-relative literal-pool load —
  `AUIPC rt, hi19` ; `LD rd, lo14(rt)` against a constant emitted to `.rodata`.
  This replaces the v1 `PseudoLI64` shift/or staircase and the 8-byte `LA`.

There are **no** multi-word instructions. `LI32`, `LA`, and the 8-byte `AUIPC`
are deleted.

## 3. Resolved architectural decisions

### 3.1 Condition handling — eliminate `FLAGS` (RISC-V/Alpha model)

`FLAGS` and `CMP`/`CMPU`/`CSET.*`/`CSEL.*`/`Bcc` are **removed**. Replaced by:

- `SLT rd, rs1, rs2` — `rd = (rs1 <s rs2) ? 1 : 0`
- `SLTU rd, rs1, rs2` — unsigned
- `SLTI rd, rs1, imm14` / `SLTIU rd, rs1, imm14`
- `BEQ rs1, rs2, off` `BNE` `BLT` `BGE` `BLTU` `BGEU`

`BGT`/`BLE` (and unsigned) are assembler pseudo-spellings that swap operands.
`SEQ`/`SNE` into a GPR are synthesized as `SLTU rd, r0, (rs1 xor rs2)` /
`SLTIU rd, (xor), 1` per the RISC-V idioms.

Backend impact: deletes all **40** `PseudoCSET*/CSET*I/CSEL*/PseudoB*`
definitions, the entire `EmitInstrWithCustomInserter` compare/select/branch
glue, and the `SELECT`→`SELECT_CC` expansion. `setcc`/`br_cc`/`select_cc`
become ordinary pattern matches. Formal impact: `FLAGS` leaves `MachineState`
entirely.

### 3.2 PC-relative loads and capabilities — Option A (CHERI-style PCC)

The Program-Counter Capability (PCC) carries **both Execute and Read**
permission and bounds covering `.text` + adjacent `.rodata` literal pools. An
`AUIPC`-formed address that falls within PCC bounds authorizes the subsequent
`LD`; no separate `.rodata` capability register is required. The loader/OS must
grant PCC read+execute over the code+rodata range at image entry.

Rationale: drastically simpler LLVM backend (literal pools are "just" PC-relative
loads, the standard LLVM constant-island mechanism), and the bounds check is a
single PCC interval test in the proof. (Option B — execute-only PCC with an
explicit `.rodata` capability register — is recorded as a future hardening step
but is **not** v2.)

Note: the PCC is a *new* piece of architectural state — there is no `PCC`
register in the v1 register files. Its representation (implicit, not a numbered
register) is specified in §4.5.

### 3.3 Branch relaxation — trust LLVM's generic pass

The ±32 KiB conditional reach (§2) means large functions overflow B-type. We do
**not** hand-roll relaxation. We implement the four target hooks
`analyzeBranch`, `insertBranch`, `removeBranch`, `reverseBranchCondition`, and
enable the generic `BranchRelaxation` pass, which rewrites an out-of-range
`Bcc target` into `Bcc.inverted skip ; JMP target` (the ±32 MiB jump-only form).

Trust boundary, stated explicitly: the Coq/Kami proofs establish that the
**hardware** executes the emitted instruction stream correctly. That a *correct*
relaxation preserves source semantics is a separate compiler-verification
problem (the CompCert-shaped gap) and is **out of scope** for the hardware
proof. We trust LLVM's mature generic pass here, exactly as a non-CompCert
toolchain would.

### 3.4 Atomics — delete `AMO_*`, implement LR/SC

The v1 `AMO_*` and `LOCK_CMPXCHG` opcodes are **removed**, and the backend's
fake load/op/store lowering is removed with them. v2 has:

- `LR.d rd, (rs1)` — load-reserved: loads, records a reservation on the
  address.
- `SC.d rd, rs2, (rs1)` — store-conditional: stores `rs2` if the reservation is
  still valid; writes 0/1 success into `rd`.

LLVM expands every `atomicrmw` / `cmpxchg` into an `LR/SC` retry loop (the
standard RISC-V `AtomicExpand` path). This also fixes a **present** v1 bug: the
ISA already has `CLONE_SPAWN`, `THREAD_JOIN`, `FUTEX_WAIT`, `FUTEX_WAKE`, i.e.
real shared-memory concurrency, so v1's "single coherent domain → fake atomics"
justification was already false.

Formal impact: `MachineState` gains a single `reservation_addr : option addr`,
set by `LR`, checked-and-consumed by `SC`, and **cleared on any store to the
reserved address and on any trap/context-switch**. This is far cheaper to prove
than an AMO ALU in the memory controller.

### 3.5 Sign-extending sub-word loads

Add `LB`/`LH`/`LW` (sign-extending) alongside `LBU`/`LHU`/`LWU` (zero-extending)
and `LD` (64-bit). Deletes the v1 `PseudoLD_SB/SH/SW` → `LD_B`+`SEXT_B`
two-instruction custom inserter. Stores: `SB`/`SH`/`SW`/`SD`.

## 4. Register files and ABI

LNP64 is a large register machine with **five** architectural register classes.
v2 only restructures the GPR file and deletes the two `SPECIAL` registers
(`LR`, `FLAGS`); the capability (FDR), control (PCR), and unimplemented
(FPR/VR) files are explicitly accounted for below so nothing is silently
dropped. Inventory (v1 → v2):

| Class | v1 members | Width | v2 disposition |
| --- | --- | --- | --- |
| GPR | `r0`-`r31` | 64 | restructured ABI (§4.1) |
| FDR | `fd0`-`fd255` (256) | capability slot | **retained, unchanged** (§4.2) |
| PCR | 12 control regs | 64 | **retained, unchanged** (§4.3) |
| FPR | `f0`-`f31` | 64 | **retained, deferred** — no instructions (§4.4) |
| VR | `v0`-`v15` | 128 | **retained, deferred** — no instructions (§4.4) |
| SPECIAL | `LR`, `TP`, `FLAGS` | 64 | **dissolved** — `LR`→`r1`, `FLAGS` deleted, `TP` is a PCR (§4.1/4.3) |

### 4.1 GPR file and integer ABI

One uniform GPR file `r0..r31`, all allocatable except the fixed roles below.
The v1 `LR` and `FLAGS` registers are deleted from the register info, so the
`SPECIAL` register class is removed entirely.

| Reg | v2 role | v1 role | Change |
| --- | --- | --- | --- |
| `r0` | hardwired zero (writes ignored) | hardwired zero | unchanged |
| `r1` | **return address (`ra`)** | temporary | now the link register |
| `r30` | **general allocatable** | reserved backend scratch | **reclaimed** |
| `r31` | stack pointer (`sp`) | stack pointer | unchanged |
| `LR` (special) | *deleted* | thread-local link reg | folded into `r1` |
| `FLAGS` (special) | *deleted* | condition codes | removed (§3.1) |

The `GPR` register class's `AltOrders` changes from `(sub GPR, R0, R30, R31)`
to `(sub GPR, R0, R31)` — `r30` rejoins the allocation order, and `r0`/`r31`
stay out. `r1` remains allocatable (it is caller-saved / clobbered by calls
like RISC-V `ra`, not reserved).

Call/return become ordinary, allocator-visible operations:

- `CALL sym` → `JAL r1, sym` (J-type; saves `pc+4` into `r1`).
- `CALL rs` → `JALR r1, rs, 0` (I-type).
- `RET` → `JALR r0, r1, 0` (jump to `r1`, discard link into the zero reg).

The prologue/epilogue spill `r1` with a normal `SD`/`LD` like any
callee-saved GPR — the v1 `LR_GET`/`LR_SET`→`r30` bounce is deleted, and
`copyPhysReg` no longer needs a special case (it's GPR↔GPR `MOV` only).

> ABI note: this changes the psABI (`r1` reserved as `ra`). `psABI.md` and the
> minimal C runtime's save/restore set must be updated in the same batch. `sp`
> stays `r31` to minimize churn in the rest of the backend.

### 4.2 FDR — capability / descriptor file (the part v2 must NOT break)

`fd0`-`fd255` are the 256 hardware-owned capability and descriptor slots. They
are **not** integer/pointer GPRs, are **not** part of the C integer ABI, and
are never targeted by ordinary codegen — they are produced and consumed only by
the capability/descriptor instructions (`OPEN_AT`, `PULL`, `PUSH`, `CAP_DUP`,
`CAP_SEND`, `CAP_RECV`, `CAP_REVOKE`, `GATE_CALL`, …). v2 leaves the FDR file,
its width, and those instructions **unchanged**. The condition-code and
link-register surgery does not touch them.

Two interactions to pin down explicitly because the capability file is where
the security model lives:

- **LR/SC (§3.4) operates on GPR-addressed memory, not on FDR slots.** The
  reservation is over a data address reachable through an ordinary memory
  capability; capability-slot mutation continues to go through the dedicated
  `CAP_*` instructions, which are not atomic-RMW and need no reservation.
- **PCC (see §4.5).** The capability authorizing PC-relative literal loads is
  the program-counter capability, which is *not* one of the FDR slots — see
  §4.5, which closes the gap that §3.2 left open.

### 4.3 PCR — process / control registers

The 12 control registers — `PID`, `PPID`, `TID`, `TP`, `UID`, `GID`,
`SIGMASK`, `SIGPENDING`, `REALTIME_SEC`, `REALTIME_NSEC`, `CRED_PROFILE`,
`CRED_HANDLE` — are **retained unchanged** and remain accessed only through
`GET_PCR rd, pcr` and `SET_PCR rd, pcr, rs`. In the v2 32-bit encoding these
carry a 5-bit PCR selector in the `rs1` slot (`[18:14]`); 12 of 32 selector
values are defined, the rest reserved. `TP` (thread pointer) is read/written
here, as the psABI TLS section already assumes; it is no longer also exposed
via the dissolved `SPECIAL` class.

### 4.4 FPR / VR — present but unimplemented

`f0`-`f31` (FPR, 64-bit) and `v0`-`v15` (VR, 128-bit) exist in the register
info but have **no instructions** in v1 and are not in the C ABI. v2 keeps the
register classes as reserved namespace and defines **no** FP or vector
instructions; a future extension owns them. They are listed here only so the
decoder/proof can treat their encoding space as reserved rather than undefined.

### 4.5 PCC — program-counter capability (resolves the §3.2 gap)

§3.2 referenced a PCC that has **no register definition in v1** — there is no
`PCC` in any register file today. v2 introduces the PCC as **implicit
architectural state** (alongside the PC itself), *not* a numbered or
GPR-allocatable register:

- The PCC bounds + permissions gate instruction fetch and authorize
  `AUIPC`-relative literal loads that fall within its range (§3.2, Option A:
  read+execute).
- It is set by the loader/OS at image entry and on capability-domain transfer
  (`GATE_CALL`/`GATE_RETURN`); it is not written by ordinary instructions.
- In `MachineState` it is a single record `{ base; bound; perms }` (§5), not an
  entry in `regs`. This keeps the GPR file proof unchanged and the PCC check a
  single interval+permission test.

## 5. Formal-model deltas (Coq/Kami `MachineState`)

- **Remove** `flags`.
- **Remove** the separate `LR` field; return address lives in `regs[1]`.
- **Add** `reservation_addr : option word` (§3.4).
- **Add** `pcc : { base; bound; perms }` as implicit state next to `pc` (§4.5),
  if not already modeled.
- **Decode** becomes a total function `word -> option instr` with no second-word
  dependency; bounded-progress in the decoder FSM follows directly.
- **Unchanged register state:** the GPR file (`regs[0..31]`, with `regs[0]`
  fixed to zero), the FDR capability file, the 12 PCRs, and the reserved
  FPR/VR namespace all carry over from v1 untouched. Only `flags` and `LR`
  leave; only `reservation_addr` and the explicit `pcc` record arrive.

## 6. TableGen as the single encoding source

Every instruction defines its `bits<32> Inst` in the `.td`. The custom
hand-written `LNP64MCCodeEmitter` switch **and** the hand-written
`LNP64AsmParser` `StringSwitch` are both retired in favor of TableGen-generated
encoder, disassembler, and `AsmMatcher`. A small extraction script
(`scripts/td_to_coq_decode.py`, new) parses the generated instruction-info
tables and emits the Coq `decode` definition, guaranteeing the compiler and the
hardware proof speak about identical bit patterns.

## 7. Migration batch — execution order

One coherent batch, but layered so each layer is validated against the one
before it (the emulator is the reference oracle):

1. **This spec** — locked (review gate here).
2. **Reference emulator** (`src/emulator.rs`) — new decode/execute for the v2
   encoding; becomes the executable oracle. Update the committed-exec typed
   traces.
3. **LLVM backend** — rewrite `LNP64InstrInfo.td` (formats + `bits<32> Inst`),
   delete the 40 pseudos + custom inserter, add `SLT*`/branch hooks +
   `BranchRelaxation`, switch constant materialization to `LUI`/`ADDI`/literal
   pools, fold `LR` into `r1`, reclaim `r30`, wire `AtomicExpand` to LR/SC,
   add sign-extending loads, and move encoding/parsing to TableGen.
4. **psABI + minimal C runtime** — `ra=r1`, save/restore sets, crt0.
5. **Conformance** — assemble/run the existing smokes through the docker
   `llvm-mc`/emulator gates; diff against the v2 emulator oracle.
6. **RTL** (`rtl/`) + **Koika core** — update the decode/opcode tables and the
   mediation core to the v2 encoding.
7. **Coq/Kami** — regenerate `decode` from the `.td`; update `MachineState`
   (remove `flags`, add `reservation_addr`); this is the Phase 1 starting point.
8. **Docs** — fold §6.0 of `hardware_design.md` into this spec's encoding and
   retire the legacy 64-bit format section.

Gating rule: no layer N+1 lands until layer N passes its gate against the
emulator oracle, so the layers can never silently desync (the failure mode that
produced the v1 spec-vs-impl drift).

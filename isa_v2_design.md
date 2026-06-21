# LNP64 ISA v2 — Design Specification

Status: **proposed / locked for review**. This document is the single source of
truth for the v2 instruction set. Every other layer — reference emulator
(`src/emulator.rs`), LLVM backend (`llvm/lib/Target/LNP64`), RTL (`rtl/`), Koika
mediation core, and the Coq/Kami proofs — derives from it. Where any layer
disagrees with this document, this document wins; the layer is the bug.

v2 is a deliberate, one-batch break from v1. v1 is not yet frozen by released
software, and Phase 1 formal proofs have not started, so we pay the migration
cost once, now, and let Phase 1 prove against v2 instead of throwing v1 proofs
away.

## 0. The width decision: fixed 64-bit instructions

v1 tried to be a fixed-32-bit ISA but wasn't: `LI32`/`LA`/`AUIPC` were 8 bytes,
`Status5` was 8 bytes, and the 5-operand path syscalls (`LINK_PATH_AT`,
`CHOWN_PATH_AT`) emitted a trailing word. The root cause is structural: LNP64
puts OS primitives in hardware, and complex primitives need high operand
bandwidth. `8-bit opcode + 5×5-bit registers = 33 bits` — a 5-operand
instruction **mathematically cannot** fit in 32 bits.

There were two ways out:

- **Argument blocks (rejected).** Keep 32-bit words; wide operands come from an
  in-memory block via a pointer. This keeps *decode* simple but makes *execute*
  a microcoded operand-fetch state machine — and a memory read mid-instruction
  can fault on a capability boundary or race another thread. That destroys the
  "one instruction = one atomic state transition" mapping in `CapImpl.v`.
- **Fixed 64-bit words (chosen).** All operands live in the register file; the
  instruction executes as a single atomic transition. Decode is a total
  function of one 64-bit word — no variable length, no second-word FSM,
  trivially bounded-progress. (eBPF made the same choice for the same reason.)

v2 is therefore a **fixed 64-bit, one-major-opcode-per-instruction CISC-leaning
RISC**. The I-cache footprint roughly doubles; on a cloud-grade server part that
is cheap, and mathematical certainty is not.

## 1. Design goals

1. **Pure 64-bit fixed-width decode.** Every instruction is exactly one 64-bit
   word. `decode : word64 -> option instr` is total with no inter-word state.
2. **Atomic execution.** Every operand is in a register before execute begins;
   no instruction reads its operands from memory. One instruction maps to one
   `MachineState` transition.
3. **No implicit architectural condition state.** No `FLAGS` register;
   comparisons write GPRs, branches read GPRs.
4. **One uniform register file.** Return address is a normal GPR; no special
   link register, no permanently reserved scratch register.
5. **Honest atomics.** Hardware LR/SC; the compiler never fakes atomicity.
6. **Declarative encoding.** Layouts live in TableGen `bits<64> Inst` fields and
   are the source from which the Coq `decode` function is generated.

## 2. Instruction encoding

All instructions are exactly 64 bits, little-endian. The opcode is the high
byte; one major opcode per instruction (no `funct` sub-decode). Register fields
are 5 bits (GPR 0..31). The PC advances by 8; all instruction addresses are
8-byte aligned. Six formats, all exactly one word:

```
 63    56 55  51 50  46 45  41 40  36 35  31 30                          0
+--------+------+------+------+------+------+-----------------------------+
| opcode |  rd  | rs1  | rs2  | rs3  | rs4  |  rs5(5) | reserved-zero(26) |  R-type (up to 6 regs)
+--------+------+------+------+------+------+-----------------------------+
| opcode |  rd  | rs1  |              imm32 (sign-extended)  | resv(14)   |  I-type (rd, rs1, imm32)
+--------+------+------+-------------------------------------+------------+
| opcode | rs1(base) | rs2(src) |        imm32 (sign-extended) | resv(14) |  S-type (store)
+--------+-----------+----------+-----------------------------+----------+
| opcode | rs1  | rs2  |     imm32 (instr-count offset, <<3)  | resv(14)  |  B-type (branch)
+--------+------+------+-------------------------------------+-----------+
| opcode |  rd  |              imm32 (sign-extended)        |  reserved   |  U-type (AUIPC)
+--------+------+-------------------------------------------+-------------+
| opcode |  rd  |        imm32 (instr-count offset, <<3)    |  reserved   |  J-type (JAL)
+--------+------+-------------------------------------------+-------------+
```

Field rules:

- Register slots occupy fixed positions: `rd[55:51]`, `rs1[50:46]`,
  `rs2[45:41]`, `rs3[40:36]`, `rs4[35:31]`, `rs5[30:26]`. A format that does not
  use a slot leaves it **reserved-zero**.
- `imm32` is a 32-bit field, sign-extended to 64 bits for arithmetic / offsets.
- **Control-transfer offsets are instruction-counts, not bytes.** Because every
  instruction is exactly 8 bytes and 8-aligned, the low 3 bits of any valid
  target offset are always zero. B-type and J-type therefore encode
  `offset >> 3`: the hardware target is `PC + (sext64(imm32) << 3)`, giving a
  **±16 GiB** reach and wasting no immediate bits on the always-zero low bits.
  (`AUIPC` is the exception — see §3.2: it forms *data* addresses with byte
  granularity and adds `sext64(imm32)` directly.)
- All **reserved** bits must be zero; the decoder rejects non-zero reserved bits
  (this keeps the encoding space closed for the proof and free for future use).
- The R-type's 6 register slots cover every wide-operand instruction natively —
  including the 5-operand `LINK_PATH_AT` / `CHOWN_PATH_AT` — so **no instruction
  emits a trailing word**.

### 2.1 Constant materialization (kills v1's 8-byte forms and hi/lo logic)

A 64-bit word holds a full 32-bit immediate inline, so:

- **≤32-bit signed:** `LI rd, imm32` — one instruction, the assembler spelling
  of `ADDI rd, r0, imm32` (`rd = sext64(imm32)`). (Replaces v1's `LI` + 8-byte
  `LI32`; there is no `LUI`+`ADDI` carry dance, and no `LUI` instruction at all.)
- **Full 64-bit literal:** `LI rd, lo32` ; `LIU rd, rd, hi32` — two
  instructions, each one word. `LIU` is an **I-type** instruction with an
  explicit source:
  `LIU rd, rs1, imm32  →  rd = (rs1 & 0x0000_0000_FFFF_FFFF) | (uint64(imm32) << 32)`.
  It reads `rs1` through the normal source-operand port and writes `rd` — there
  is **no** read-modify-write of the destination, so sources stay sources and
  destinations stay destinations (clean pipeline, clean Coq `Execute`). Used
  only for non-address 64-bit constants.
- **Address-of-global / PC-relative 64-bit:** `AUIPC rt, imm32` ;
  `LD rd, lo(rt)` against a `.rodata` literal, or `AUIPC`+`ADDI`-equivalent for
  in-range PIC. This is the common path for pointers.

Load/store displacements are **32-bit signed** (I/S-type `imm32`), so frame
offsets never overflow in practice — v1's `r30`-scratch address-materialization
path (`LNP64RegisterInfo.cpp:80-92`) is **deleted**.

## 3. Resolved architectural decisions

### 3.1 Condition handling — eliminate `FLAGS` (RISC-V/Alpha model)

`FLAGS`, `CMP`/`CMPU`, `CSET.*`, `CSEL.*`, and `Bcc` are **removed**. Replaced
by compare-into-GPR and compare-and-branch:

- `SLT rd, rs1, rs2` / `SLTU` — `rd = (rs1 < rs2) ? 1 : 0` (signed / unsigned).
- `SLTI rd, rs1, imm32` / `SLTIU` — set-less-than immediate.
- `BEQ rs1, rs2, off` / `BNE` / `BLT` / `BGE` / `BLTU` / `BGEU`.

`BGT`/`BLE` (and unsigned) are assembler pseudo-spellings that swap operands.
`SEQ`/`SNE` into a GPR use the RISC-V `SLTU rd, r0, (xor)` / `SLTIU` idioms.

Backend impact: deletes all **40** `PseudoCSET*/CSET*I/CSEL*/PseudoB*`, the
entire `EmitInstrWithCustomInserter` compare/select/branch glue, and the
`SELECT`→`SELECT_CC` expansion. `setcc`/`br_cc`/`select_cc` become ordinary
pattern matches. Formal impact: `FLAGS` leaves `MachineState`.

### 3.2 PC-relative loads and capabilities — Option A (read+execute literal pools)

`AUIPC rd, imm32` is defined as `rd = PC + sext64(imm32)` (PC of the AUIPC
instruction). The subsequent `LD` of a literal is authorized by the **Read
permission on the literal-pool region** — *not* by a program-counter capability.

> Verified against the implementation (2026-06-21). There is **no PCC / program-
> counter-capability register** anywhere in LNP64 — `grep` over the emulator,
> RTL, and Coq finds none. Authorization is POSIX-style per-region `R/W/X`:
> - **Emulator** (the reference oracle): instruction fetch checks the execute
>   bit (`emulator.rs:9628`); a data load checks the read bit (`:9557`). The
>   loader enforces **W^X** — it rejects write∧execute (`:1337`) but **permits
>   read∧execute**. So a region mapped `R+X` serves both fetch and `LD`, and a
>   literal-pool load succeeds on the strength of the read bit alone.
> - **RTL**: an 8-bit `permissions` field with `load_permitted` /
>   `permission_faulted` (`lnp64_pkg.sv:647,660,695`) — a per-descriptor
>   permission model that expresses the same R/X distinction.

v2 therefore mandates: the literal-pool region (in `.text` mapped `R+X`, or an
adjacent `.rodata` mapped `R`) carries **Read** permission, and the **W^X**
invariant is preserved (code + rodata are never writable). No new architectural
register is introduced. Backend impact is nil — LLVM emits the ordinary
`AUIPC + LD` constant-island sequence.

(Option B — execute-only code with literals confined to a separate
read-only `.rodata`, no read permission on `.text` — is also expressible in the
same R/W/X model and is recorded as future hardening, **not** v2.)

> Phase-1 proof gap (record now): the Coq capability model (`CapSpec.v`) proves
> **write** confinement only — "an address range plus a write permission"
> (`CapSpec.v:16`). It models no read or fetch permission. Covering PC-relative
> literal loads in the formal security argument requires extending the Coq
> `Step`/`MachineState` with read + fetch permission. This is new proof scope,
> not a v2 encoding detail.

### 3.3 Branch range — relaxation no longer required

With instruction-count branch/jump offsets (§2, `imm32 << 3`), conditional
branches and calls reach ±16 GiB directly. The generic
`BranchRelaxation` pass and its semantics-preservation proof obligation are
**not needed**. We still implement `analyzeBranch` / `insertBranch` /
`removeBranch` / `reverseBranchCondition` (LLVM requires them for basic
codegen), but no relaxation/long-branch trampoline machinery.

### 3.4 Atomics — delete `AMO_*`, implement LR/SC

The v1 `AMO_*` and `LOCK_CMPXCHG` opcodes are **removed**, along with the
backend's fake load/op/store lowering. v2 has:

- `LR.d rd, (rs1)` — load-reserved: loads, records a reservation on the address.
- `SC.d rd, rs2, (rs1)` — store-conditional: stores `rs2` iff the reservation
  is still valid; writes 0/1 success into `rd`.

LLVM expands every `atomicrmw` / `cmpxchg` into an LR/SC retry loop (standard
RISC-V `AtomicExpand`). This also fixes a **present** v1 bug: the ISA already
has `CLONE_SPAWN`, `THREAD_JOIN`, `FUTEX_WAIT`, `FUTEX_WAKE` — real shared-memory
concurrency — so v1's "single coherent domain → fake atomics" was already false.

Formal impact: `MachineState` gains `reservation_addr : option word`, set by
`LR`, checked-and-consumed by `SC`, and **cleared on any store to the reserved
address and on any trap / context switch**. Far cheaper to prove than an AMO ALU
in the memory controller.

### 3.5 Sign-extending sub-word loads

Add `LB`/`LH`/`LW` (sign-extending) alongside `LBU`/`LHU`/`LWU`
(zero-extending) and `LD` (64-bit). Deletes the v1 `PseudoLD_SB/SH/SW` →
`LD_B`+`SEXT_B` two-instruction custom inserter. Stores: `SB`/`SH`/`SW`/`SD`.

## 4. Register files and ABI

LNP64 has **five** architectural register classes. v2 only restructures the GPR
file and deletes the two `SPECIAL` registers (`LR`, `FLAGS`); the capability
(FDR), control (PCR), and unimplemented (FPR/VR) files are explicitly accounted
for so nothing is silently dropped.

| Class | v1 members | Width | v2 disposition |
| --- | --- | --- | --- |
| GPR | `r0`-`r31` | 64 | restructured ABI (§4.1) |
| FDR | `fd0`-`fd255` (256) | capability slot | **retained, unchanged** (§4.2) |
| PCR | 12 control regs | 64 | **retained, unchanged** (§4.3) |
| FPR | `f0`-`f31` | 64 | **retained, deferred** — no instructions (§4.4) |
| VR | `v0`-`v15` | 128 | **retained, deferred** — no instructions (§4.4) |
| SPECIAL | `LR`, `TP`, `FLAGS` | 64 | **dissolved** — `LR`→`r1`, `FLAGS` deleted, `TP` is a PCR |

### 4.1 GPR file and integer ABI

One uniform GPR file `r0..r31`. The v1 `LR` and `FLAGS` registers are deleted, so
the `SPECIAL` register class is removed entirely.

| Reg | v2 role | v1 role | Change |
| --- | --- | --- | --- |
| `r0` | hardwired zero (writes ignored) | hardwired zero | unchanged |
| `r1` | **return address (`ra`)** | temporary | now the link register |
| `r30` | **general allocatable** | reserved backend scratch | **reclaimed** |
| `r31` | stack pointer (`sp`) | stack pointer | unchanged |
| `LR` (special) | *deleted* | thread-local link reg | folded into `r1` |
| `FLAGS` (special) | *deleted* | condition codes | removed (§3.1) |

`GPR` `AltOrders` changes from `(sub GPR, R0, R30, R31)` to
`(sub GPR, R0, R31)` — `r30` rejoins allocation; `r0`/`r31` stay out; `r1`
remains allocatable (caller-saved / call-clobbered like RISC-V `ra`).

Call/return are ordinary allocator-visible operations:

- `CALL sym` → `JAL r1, sym` (J-type; saves `pc+8` into `r1`).
- `CALL rs` → `JALR r1, rs, 0` (I-type).
- `RET` → `JALR r0, r1, 0`.

Prologue/epilogue spill `r1` with a normal `SD`/`LD` like any callee-saved GPR —
the v1 `LR_GET`/`LR_SET`→`r30` bounce is deleted, and `copyPhysReg` no longer
needs a special case (GPR↔GPR `MOV` only). Note `pc+8` (not `pc+4`) for the
64-bit instruction width.

### 4.2 FDR — capability / descriptor file (must NOT break)

`fd0`-`fd255` are the 256 hardware-owned capability/descriptor slots. They are
not integer/pointer GPRs, not part of the C integer ABI, and never targeted by
ordinary codegen — produced/consumed only by capability/descriptor instructions
(`OPEN_AT`, `PULL`, `PUSH`, `CAP_DUP`, `CAP_SEND`, `CAP_RECV`, `CAP_REVOKE`,
`GATE_CALL`, …). v2 leaves the FDR file, its width, and those instructions
**unchanged** — and the 64-bit word now encodes their (≤6) register operands
natively in one word, removing the v1 trailing-word hack. Interactions:

- **LR/SC operates on GPR-addressed memory, not FDR slots.** Capability-slot
  mutation stays in the `CAP_*` instructions (not atomic-RMW; no reservation).
- **PCC** is not an FDR slot — see §4.5.

### 4.3 PCR — process / control registers

`PID`, `PPID`, `TID`, `TP`, `UID`, `GID`, `SIGMASK`, `SIGPENDING`,
`REALTIME_SEC`, `REALTIME_NSEC`, `CRED_PROFILE`, `CRED_HANDLE` are **retained
unchanged**, accessed via `GET_PCR rd, pcr` / `SET_PCR rd, pcr, rs` with a 5-bit
PCR selector in the `rs1` slot (12 of 32 values defined, rest reserved). `TP`
is read/written here, as the psABI TLS section assumes; it is no longer also
exposed via the dissolved `SPECIAL` class.

### 4.4 FPR / VR — present but unimplemented

`f0`-`f31` (FPR, 64-bit) and `v0`-`v15` (VR, 128-bit) exist in the register info
but have no instructions and are not in the C ABI. v2 keeps them as reserved
namespace and defines **no** FP/vector instructions; a future extension owns
them. Listed here so the decoder/proof treats their space as reserved, not
undefined.

### 4.5 Code/rodata authorization — no PCC; reuse the R/W/X model

v2 introduces **no** program-counter capability and **no** new register. As
verified in §3.2, LNP64 has no PCC; instruction fetch and PC-relative literal
loads are authorized by the existing per-region `R/W/X` permissions:

- The code + literal-pool region is mapped `R+X` (or code `X` with literals in
  an adjacent `R` `.rodata`).
- Fetch requires the execute bit; the literal `LD` requires the read bit; the
  loader's **W^X** invariant keeps the region non-writable.

This keeps the GPR file and the register-file proof completely unchanged. The
only formal-model work is extending the capability model with read/fetch
permission (§3.2 Phase-1 gap), which lives in the existing capability state, not
in a new PC-bound register.

## 5. Formal-model deltas (Coq/Kami `MachineState`)

- **Remove** `flags`.
- **Remove** the separate `LR` field; return address lives in `regs[1]`.
- **Add** `reservation_addr : option word` (§3.4).
- **Extend the capability model with read + fetch permission.** Today
  `CapSpec.v` proves write confinement only (an address range + a write
  permission); covering instruction fetch and PC-relative literal loads (§3.2)
  needs read/fetch permission added to the existing capability state. **No PCC
  register is added** — there is none in the machine (§4.5).
- **Decode** is a total function `word64 -> option instr`, fixed width, no
  inter-word state — bounded-progress in the fetch/decode FSM is immediate.
- **Execute** reads all operands from registers; one instruction = one
  transition (a blocking/parking primitive transitions into the parked/trap
  state, still a single step).
- **Unchanged register state:** GPR file (`regs[0..31]`, `regs[0]` fixed zero),
  FDR capability file, the 12 PCRs, and the reserved FPR/VR namespace all carry
  over from v1. Only `flags`/`LR` leave; only `reservation_addr` and the
  explicit `pcc` record arrive.

## 6. TableGen as the single encoding source

Every instruction defines its `bits<64> Inst` in the `.td`. The custom
hand-written `LNP64MCCodeEmitter` switch **and** the hand-written
`LNP64AsmParser` `StringSwitch` are retired in favor of TableGen-generated
encoder, disassembler, and `AsmMatcher`. A small extraction script
(`scripts/td_to_coq_decode.py`, new) parses the generated instruction-info
tables and emits the Coq `decode` definition, so compiler and hardware proof
speak about identical bit patterns. One-major-opcode-per-instruction makes that
Coq decoder a single flat pattern match.

## 7. Migration batch — execution order

One coherent batch, layered so each layer validates against the one before it
(the emulator is the reference oracle):

1. **This spec** — locked (review gate here).
2. **Reference emulator** (`src/emulator.rs`) — new 64-bit decode/execute;
   becomes the executable oracle. Update committed-exec typed traces.
3. **LLVM backend** — rewrite `LNP64InstrInfo.td` (64-bit formats +
   `bits<64> Inst`), delete the 40 pseudos + custom inserter, add `SLT*` +
   compare-branch patterns + `analyzeBranch`/`insertBranch` hooks (no
   relaxation), switch constants to `LI imm32` / `LIU` / `AUIPC`+literal-pool,
   fold `LR`→`r1`, reclaim `r30`, delete the large-offset spill path, wire
   `AtomicExpand` to LR/SC, add sign-extending loads, set instruction `Size=8`
   and `pc+8` returns, and move encoding/parsing to TableGen.
4. **psABI + minimal C runtime** — `ra=r1`, save/restore sets, crt0, 8-byte
   instruction width.
5. **Conformance** — assemble/run the existing smokes through the docker
   `llvm-mc`/emulator gates; diff against the v2 emulator oracle.
6. **RTL** (`rtl/`) + **Koika core** — 64-bit fetch/decode and the v2 opcode
   table; update the mediation core.
7. **Coq/Kami** — regenerate `decode` from the `.td`; update `MachineState`
   (remove `flags`/`LR`, add `reservation_addr`, extend the capability model
   with read/fetch permission — no PCC register). Phase 1 starting point.
8. **Docs** — replace `hardware_design.md` §6 (both the legacy 64-bit-word
   format *and* the §6.0 32-bit description) with this spec's §2 encoding.

Gating rule: no layer N+1 lands until layer N passes its gate against the
emulator oracle, so the layers can never silently desync (the failure mode that
produced the v1 spec-vs-impl drift).

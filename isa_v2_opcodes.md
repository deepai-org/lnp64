# LNP64 ISA v2 — Canonical Opcode & Encoding Table

Authoritative bit-pattern contract for v2. Every layer — emulator, LLVM `.td`,
RTL `lnp64_pkg.sv`/`lnp64_decode.sv`, and the generated Coq `decode` — MUST
implement exactly this table. Companion to [`isa_v2_design.md`](isa_v2_design.md)
(formats §2) and [`isa_v2_migration_inventory.md`](isa_v2_migration_inventory.md).

All instructions are **64 bits**, little-endian, one major opcode in `[63:56]`.

**Positional convention** — register slots occupy fixed bit positions
regardless of role; a slot an instruction does not use is reserved-zero:

```
rd  = [55:51]   rs1 = [50:46]   rs2 = [45:41]
rs3 = [40:36]   rs4 = [35:31]   rs5 = [30:26]
```

The 32-bit immediate occupies the 32 bits immediately below the **lowest
register slot the format uses**, so its position is fixed per format:

| Fmt | Uses rd? | Register slots | `imm32` bits | reserved |
| --- | --- | --- | --- | --- |
| R | yes | rd, rs1, rs2, rs3, rs4, rs5 | — | `[25:0]` |
| I | yes | rd, rs1 | `[45:14]` | `[13:0]` |
| S | no (rd-slot=0) | rs1(base), rs2(src) | `[40:9]` | `[8:0]` |
| B | no (rd-slot=0) | rs1, rs2 | `[40:9]` (`off = imm32<<3`) | `[8:0]` |
| U | yes | rd | `[50:19]` | `[18:0]` |
| J | yes | rd | `[50:19]` (`off = imm32<<3`) | `[18:0]` |

`rd = (w>>51)&0x1f`, `rs1 = (w>>46)&0x1f`, `rs2 = (w>>41)&0x1f` are computed the
same way for every instruction; only the immediate offset varies by format.
Unused slots and all reserved bits are zero; the decoder rejects non-zero
reserved bits.

Opcode-stability rule: an instruction whose **semantics are identical** to v1
keeps its v1 opcode (absorbed into v2, re-encoded as one 64-bit word). v1-only
instructions are removed; their opcodes are either reused by a v2 addition
(noted) or left free.

## Formats

| Fmt | Operands | Slots used |
| --- | --- | --- |
| R | `rd, rs1[, rs2[, rs3[, rs4[, rs5]]]]` | up to 6 register slots |
| I | `rd, rs1, imm32` | rd, rs1, imm32 |
| S | `rs1(base), rs2(src), imm32` | rs1, rs2, imm32 |
| B | `rs1, rs2, off` | rs1, rs2, imm32 (`off = imm32<<3`) |
| U | `rd, imm32` | rd, imm32 |
| J | `rd, off` | rd, imm32 (`off = imm32<<3`) |

## 1. Integer ALU — R-type (`rd, rs1, rs2`)

| Op | Mnemonic | Semantics | v1 |
| --- | --- | --- | --- |
| 0x10 | `add` | rd = rs1 + rs2 | = |
| 0x11 | `sub` | rd = rs1 - rs2 | = |
| 0x12 | `mul` | rd = rs1 * rs2 | = |
| 0x13 | `div` | rd = rs1 /s rs2 | = |
| 0xa7 | `udiv` | rd = rs1 /u rs2 | = |
| 0xa8 | `srem` | rd = rs1 %s rs2 | = |
| 0xa9 | `urem` | rd = rs1 %u rs2 | = |
| 0xaa | `mulh` | signed high | = |
| 0xab | `mulhu` | unsigned high | = |
| 0xac | `mulhsu` | signed×unsigned high | = |
| 0x14 | `and` | rd = rs1 & rs2 | = |
| 0x15 | `or` | rd = rs1 \| rs2 | = |
| 0x16 | `xor` | rd = rs1 ^ rs2 | = |
| 0x18 | `sll` | rd = rs1 << rs2 | v1 `lsl` |
| 0x19 | `srl` | rd = rs1 >>u rs2 | v1 `lsr` |
| 0x1a | `sra` | rd = rs1 >>s rs2 | v1 `asr` |
| 0xb6 | `rol` | rotate left | = |
| 0xb7 | `ror` | rotate right | = |
| **0x1b** | **`slt`** | rd = (rs1 <s rs2) | **NEW** (reuses v1 `cmp` slot) |
| **0x1c** | **`sltu`** | rd = (rs1 <u rs2) | **NEW** (reuses v1 `cmpu` slot) |

## 2. Unary — R-type (`rd, rs1`)

| Op | Mnemonic | v1 |
| --- | --- | --- |
| 0x17 | `not` | = |
| 0xad | `sext.b` | = |
| 0xae | `sext.h` | = |
| 0xaf | `sext.w` | = |
| 0xb0 | `zext.b` | = |
| 0xb1 | `zext.h` | = |
| 0xb2 | `zext.w` | = |
| 0xb3 | `clz` | = |
| 0xb4 | `ctz` | = |
| 0xb5 | `popcnt` | = |
| 0xb8 | `bswap16` | = |
| 0xb9 | `bswap32` | = |
| 0xba | `bswap64` | = |

## 3. Register-immediate — I-type (`rd, rs1, imm32`)

| Op | Mnemonic | Semantics | v1 |
| --- | --- | --- | --- |
| 0xa0 | `addi` | rd = rs1 + sext(imm32) | v1 simm14 → simm32 |
| 0xa1 | `andi` | rd = rs1 & sext(imm32) | widened |
| 0xa2 | `ori` | rd = rs1 \| sext(imm32) | widened |
| 0xa3 | `xori` | rd = rs1 ^ sext(imm32) | widened |
| 0xa4 | `slli` | rd = rs1 << (imm32 & 63) | v1 `lsli` |
| 0xa5 | `srli` | rd = rs1 >>u (imm32 & 63) | v1 `lsri` |
| 0xa6 | `srai` | rd = rs1 >>s (imm32 & 63) | v1 `asri` |
| **0x1d** | **`slti`** | rd = (rs1 <s sext(imm32)) | **NEW** |
| **0x1e** | **`sltiu`** | rd = (rs1 <u sext(imm32)) | **NEW** |
| **0x04** | **`liu`** | rd = (rs1 & 0xFFFFFFFF) \| (uint(imm32)<<32) | **NEW** (reuses v1 `li32` slot) |

`li rd, imm32` is an assembler alias for `addi rd, r0, imm32`.
64-bit constant: `li rd, lo32 ; liu rd, rd, hi32`.

## 4. Loads — I-type (`rd, rs1(base), imm32`) / Stores — S-type (`rs1(base), rs2(src), imm32`)

| Op | Mnemonic | Semantics | v1 |
| --- | --- | --- | --- |
| 0x30 | `ld` | rd = mem64 | = |
| 0x31 | `lwu` | rd = zext mem32 | v1 `ld.w` |
| 0x32 | `lbu` | rd = zext mem8 | v1 `ld.b` |
| 0x36 | `lhu` | rd = zext mem16 | v1 `ld.h` |
| **0x05** | **`lw`** | rd = sext mem32 | **NEW** |
| **0x08** | **`lb`** | rd = sext mem8 | **NEW** (reuses v1 errno-get-region free slot; see §note) |
| **0x09** | **`lh`** | rd = sext mem16 | **NEW** |
| 0x33 | `sd` | mem64 = rs2 | v1 `st` |
| 0x34 | `sw` | mem32 = rs2 | v1 `st.w` |
| 0x35 | `sb` | mem8 = rs2 | v1 `st.b` |
| 0x37 | `sh` | mem16 = rs2 | v1 `st.h` |

> Free-slot note: v2 reuses low opcodes vacated by removed v1 ops. `0x05`,
> `0x08`, `0x09` were unassigned/`inb`/`outb`-adjacent in v1; final assignment is
> fixed here and the emulator/RTL/`.td` must match exactly. (`inb`/`outb`/
> `load_ucode` move — see §9.)

## 5. Atomics — LR/SC (replaces all `AMO_*`/`LOCK_CMPXCHG`)

| Op | Mnemonic | Form | Semantics |
| --- | --- | --- | --- |
| **0xc5** | **`lr.d`** | `rd, (rs1)` | rd = mem64[rs1]; reservation_addr = rs1 |
| **0xc6** | **`sc.d`** | `rd, rs2, (rs1)` | if reservation valid: mem64[rs1]=rs2, rd=0; else rd=1; reservation cleared |
| 0xcd | `fence` | — | memory fence |
| 0xce | `isync` | `rs1, rs2, rs3` | instruction-stream sync (= v1) |

Removed: `amo.swap/add/and/or/xor` (0xc5-0xc8,0xca), `lock.cmpxchg` (0xc9).
0xc5/0xc6 reused by `lr.d`/`sc.d`; 0xc7-0xca left free.

## 6. Control transfer

| Op | Mnemonic | Fmt | Semantics | v1 |
| --- | --- | --- | --- | --- |
| 0x20 | `jmp` | J | pc += off (rd=r0) | = (unconditional) |
| 0x27 | `jal` | J | rd = pc+8; pc += off | v1 `call` |
| 0x28 | `jalr` | I | rd = pc+8; pc = rs1 + sext(imm32) | v1 `call_reg` |
| 0x21 | `beq` | B | if rs1==rs2: pc += off | v1 flag-branch |
| 0x22 | `bne` | B | if rs1!=rs2 | v1 flag-branch |
| 0x23 | `blt` | B | if rs1 <s rs2 | v1 flag-branch |
| 0x24 | `bge` | B | if rs1 >=s rs2 | v1 flag-branch |
| 0x25 | `bltu` | B | if rs1 <u rs2 | **NEW** |
| 0x26 | `bgeu` | B | if rs1 >=u rs2 | **NEW** |
| 0x40 | `sel.eq` | R4 | rd = (ra==rb) ? rt : rf | **NEW** (fused compare-select) |
| 0x41 | `sel.ne` | R4 | rd = (ra!=rb) ? rt : rf | **NEW** |
| 0x42 | `sel.lt` | R4 | rd = (ra <s rb) ? rt : rf | **NEW** |
| 0x43 | `sel.ge` | R4 | rd = (ra >=s rb) ? rt : rf | **NEW** |
| 0x44 | `sel.ltu` | R4 | rd = (ra <u rb) ? rt : rf | **NEW** |
| 0x45 | `sel.geu` | R4 | rd = (ra >=u rb) ? rt : rf | **NEW** |

`sel.<cc> rd, ra, rb, rt, rf` is the fused compare-and-select (a branch-free,
Class-A datapath mux): rd[55:51], ra=rs1[50:46], rb=rs2[45:41], rt=rs3[40:36],
rf=rs4[35:31]. Conditions mirror the branch family (0x21-0x26). It is the
hardware target for LLVM `ISD::SELECT_CC` (and plain `select` via
`sel.ne rd, cond, r0, t, f`), replacing the v1 compare + branch-diamond lowering.

`ret` = `jalr r0, r1, 0`. `call sym` = `jal r1, sym`. `bgt/ble/bgtu/bleu` are
assembler aliases (operand swap). Removed: `ret`(0x1f), `lr_get`(0x29),
`lr_set`(0x2a) — the `LR` register is gone; the link lives in `r1`.

## 7. Constants / PC-relative

| Op | Mnemonic | Fmt | Semantics | v1 |
| --- | --- | --- | --- | --- |
| 0xd0 | `auipc` | U | rd = pc + sext(imm32) | v1 (was 8-byte; now 1 word) |

Removed: `la`(0x03), `li32`(0x04 → reused by `liu`). `li` is `addi rd,r0,imm32`.

## 8. PCR access

| Op | Mnemonic | Form | v1 |
| --- | --- | --- | --- |
| 0x54 | `get_pcr` | `rd, pcr` (selector in rs1 slot) | = |
| 0x55 | `set_pcr` | `rd, pcr, rs` | = |

PCR selectors (rs1 slot, 5-bit): 0 PID, 1 PPID, 2 TID, 3 TP, 4 UID, 5 GID,
6 SIGMASK, 7 SIGPENDING, 8 REALTIME_SEC, 9 REALTIME_NSEC, 10 CRED_PROFILE,
11 CRED_HANDLE.

## 9. System / capability / FDR / path primitives (semantics = v1; re-encoded 64-bit)

All of the following keep their v1 opcodes and semantics and are re-encoded as a
single 64-bit word; **the v1 trailing-word hack is removed** — 5-register forms
place the 5th operand in the `rs4` slot.

| Op | Mnemonic | Op | Mnemonic |
| --- | --- | --- | --- |
| 0x07 | `sleep` | 0x57 | `write_fd` |
| 0x2b | `pull` | 0x58 | `open_at_dyn` |
| 0x2c | `push` | 0x59 | `clone.spawn` |
| 0x2d | `read_fd` | 0x5a | `thread_join` |
| 0x2e | `await` | 0x5b | `dma_ctl` |
| 0x2f | `call_cap` | 0x5c | `stat_path_at` |
| 0x38 | `errno_get` | 0x5d | `stat_fd_dyn` |
| 0x39 | `errno_set` | 0x5e | `utime_path_at` |
| 0x3a | `exit` | 0x5f | `utime_fd_dyn` |
| 0x3b | `pull_dyn` | 0x60 | `mmap_bootstrap` |
| 0x3c | `push_dyn` | 0x61 | `munmap_bootstrap` |
| 0x47 | `alloc` | 0x62 | `sigaction` |
| 0x48 | `alloc_size` | 0x63 | `sigmask_set` |
| 0x49 | `free` | 0x64 | `kill` |
| 0x4a | `alloc_ex` | 0x65 | `sigret` |
| 0x4b | `object_ctl` | 0x66 | `mprotect_bootstrap` |
| 0x4c | `domain_ctl` | 0x67 | `fcntl_fd_dyn` |
| 0x4d | `await_dyn` | 0x68 | `alarm` |
| 0x4e | `call_cap_dyn` | 0x69 | `fd_seek_dyn` |
| 0x4f | `ret_cap` | 0x6a | `mmap` (now single word: fd→rs4, off→rs5) |
| 0x50 | `cap_dup` | 0x6b | `unlink_path_at` |
| 0x51 | `cap_send` | 0x6c | `mprotect` |
| 0x52 | `cap_recv` | 0x6d | `open_fd_dyn` |
| 0x53 | `cap_revoke` | 0x6e | `fd_close_dyn` |
| 0x56 | `env_get` | 0x6f | `waitable_probe` |
| 0x70 | `waitable_probe_dyn` | 0x79 | `chdir_path` |
| 0x71 | `await_ex` | 0x7a | `getcwd_path` |
| 0x72 | `await_ex_dyn` | 0x7b | `chmod_path_at` |
| 0x73 | `open_dir_dyn` | 0x7c | `chown_path_at` (5th reg→rs4; no trailing word) |
| 0x74 | `mkdir_path_at` | 0x7d | `fork` |
| 0x75 | `rename_path_at` | 0x7e | `wait_pid` |
| 0x76 | `link_path_at` (5th reg→rs4; no trailing word) | 0x7f | `exec` |
| 0x77 | `symlink_path_at` | 0x80 | `inb` |
| 0x78 | `readlink_path_at` | 0x81 | `outb` |
| 0x82 | `load_ucode` | 0xcb | `futex_wait` |
| 0xcc | `futex_wake` | 0xcf | `readdir_fd_dyn` |

## 10. Unified endpoint IPC (Phase 3, `unified_object_model.md`)

The four verbs `send`/`recv`/`gate_call`/`wait` operate **only** on endpoints + a
`(bytes, caps)` message. Operands are GPRs holding handle/pointer **values**
(post-FDR→GPR migration). An endpoint's behavior is its **type** —
`Backing {Thread, Memory, Register} × Producer {software, hardware}` — fixed at
create time, **not** a per-op flag. **There are no ring/SQE/async opcodes:** an
io_uring-style ring is a *Memory-backed endpoint* you `send`/`recv`/`wait` on, so a
verb is never encoded twice (the endpoint *type* selects buffered-vs-rendezvous).
Landing oracle-first; **not frozen into RTL** until the bounded Memory-backed-endpoint
latency + cap-safety proofs (E4) pass.

| Op | Mnemonic | Form | Semantics | Status |
| --- | --- | --- | --- | --- |
| 0x83 | `send` | `rd, rs1(ep), rs2(msgdesc)` | rendezvous w/ ep backing — Thread→block-till-consumer, Memory→enqueue&return (EAGAIN if full), Register→update; rd=bytes or -errno | **emulator** |
| 0x84 | `recv` | `rd, rs1(ep), rs2(msgdesc)` | take one message; install caps; rd=bytes or -errno (EAGAIN if empty) | **emulator** |
| 0x88 | `endpoint_create` | `rd, rs1(type/hint)` | mint an endpoint cap of a `Backing×Producer` type; rd=handle | **emulator** |
| 0x86 | `wait` | `rd, rs1(waitset), rs2(timeout)` | poll/block until an edge in the set fires; rd=#ready | **emulator** |
| 0x2f | `gate_call` | (existing) | the `call` verb — Thread-backed rendezvous (migrating gate); completion = `send` to a Continuation Endpoint | **built + M2-proven** |

The `call` verb is the existing `GATE_CALL` (`call_cap`) at 0x2f — no new opcode.
`0x85` and `0x87` stay **free**: the ring needs no instruction (it is a Memory-backed
endpoint; the formerly-reserved `ring_enter` is **dropped**).

**Message descriptor** (frozen, in guest memory): `[0]=bytes_ptr`,
`[8]=bytes_len` (send in; recv buffer-cap in / actual out), `[16]=caps_ptr`
(array of u64 cap handles), `[24]=caps_len` (send #caps; recv buffer-cap in /
actual out). Caps in a message are cap-table handles resolved against the
*sender's* table and installed into the *receiver's* by the engine.

## Removed in v2 (no trace retained)

- Condition-code machine: `cmp`(0x1b→`slt`), `cmpu`(0x1c→`sltu`), `cset.*`
  (0x3d-0x46), `csel.*` (0xbb-0xc4). The `FLAGS` register is deleted.
- Link register: `ret`(0x1f), `lr_get`(0x29), `lr_set`(0x2a). `LR` deleted;
  link is `r1`.
- Wide-immediate forms: `la`(0x03), `li32`(0x04→`liu`). 8-byte `auipc`→1-word.
- Atomics: `amo.*`(0xc5-0xc8,0xca→`lr.d`/`sc.d` reuse 0xc5/0xc6),
  `lock.cmpxchg`(0xc9).

Free opcode slots after migration: 0x0a-0x0f, 0x1f, 0x29, 0x2a, 0x3d-0x46,
0x85, 0x87, 0x89-0x9f, 0xbb-0xc4, 0xc7-0xca, 0xd1-0xff. (0x83/0x84/0x86/0x88
reserved for the unified-endpoint verbs `send`/`recv`/`wait`/`endpoint_create`, §10;
the `call` verb reuses the existing gate at 0x2f; no ring opcode — the ring is a
Memory-backed endpoint.)

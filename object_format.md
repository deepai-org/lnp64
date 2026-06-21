# LNP64 Binary and Object Format v1

This document defines the first software-loader ELF profile for LNP64. The
repository now supports both legacy assembly smoke execution and real
Clang/lld-produced static ELF execution through `run-elf`; this format is the
contract between the package toolchain, loader, and bounded exec-plan path.

Hardware does not parse this format. ELF headers, program headers, dynamic
linking, relocations, interpreters, shebang handling, library search, and
credential-transition policy belong to loader services, libc runtimes, or Unix
personality domains. A loader that accepts this format produces a bounded LNP64
exec-plan descriptor; hardware `EXEC` validates that descriptor's capabilities
and commits the address-space replacement atomically.

The boundary is intentional: this file is a software-loader contract, not a
hardware executable ABI. Linux, NetBSD, native static loaders, boot-manifest
tools, and future dynamic loaders may all produce the same exec-plan shape after
applying their own format and policy rules.

## ELF Profile

LNP64 v1 uses ELF64, little-endian, two's complement, LP64-style objects.

| ELF Field | Value |
| --- | --- |
| `EI_CLASS` | `ELFCLASS64` |
| `EI_DATA` | `ELFDATA2LSB` |
| `e_machine` | `EM_LNP64`, provisional value `0x6c64` until registered |
| `e_ident[EI_OSABI]` | `ELFOSABI_NONE` for native static programs |
| `e_type` | `ET_EXEC` or `ET_DYN`; `ET_DYN` means static PIE only in this v1 profile |
| `p_align` | page-aligned `PT_LOAD` segments use at least 4096-byte alignment |

The canonical toolchain path for v1 is static linking to an executable image.
Relocatable `ET_REL` objects are a toolchain format, not an emulator input.
Accepting `ET_DYN` for static PIE is a software-loader policy decision; it does
not imply a hardware dynamic linker or a mandatory shared-library ABI.

## Sections and Segments

The software loader consumes program headers. Section headers are optional at
runtime and may be stripped.

Required program-header behavior:

- `PT_LOAD` segments are mapped with permissions derived from `p_flags`.
- `PT_PHDR`, if present, is exposed through auxv metadata.
- `PT_NOTE` may carry LNP64 startup descriptor notes.
- `PT_TLS` describes the initial TLS image for the main executable.
- `PT_DYNAMIC` is rejected by static v1 unless the process domain has opted into
  a future dynamic-loader profile.

Recommended sections for linked objects:

| Section | Purpose |
| --- | --- |
| `.text` | executable code, mapped RX |
| `.rodata` | constants, mapped R |
| `.data` | initialized writable data, mapped RW |
| `.bss` | zero-fill writable data, mapped RW |
| `.tdata`, `.tbss` | initial TLS image and zero-fill TLS |
| `.rela.*` | addend-bearing relocation records for link-time and PIE fixups |
| `.note.lnp64.startup` | startup descriptor metadata |
| `.note.lnp64.capreq` | requested startup FDR/capability descriptors |

## Executable Mapping Permissions

The loader must never create a writable-plus-executable mapping in normal v1
mode.

| Segment Flags | VMA Protection |
| --- | --- |
| `PF_R` | read |
| `PF_W` | read/write, non-executable |
| `PF_X` | read/execute, non-writable |
| `PF_W | PF_X` | rejected unless the Resource Domain has explicit loader/JIT W^X exception authority |

Data, heap, stack, TLS, signal frame, FDR object, DMA buffer, and anonymous
mappings default NX. Executable mappings must come from an executable image
object or an authorized loader/JIT transition. If relocations must patch code,
the loader maps the affected page writable and non-executable, applies the
relocation, then transitions it to executable and non-writable with `MPROTECT`
and instruction-cache synchronization.

## ASLR Loader Behavior

Static non-PIE `ET_EXEC` images are loaded at their fixed virtual addresses
unless the executable was linked with an LNP64 relocation table that permits
rebasing.

PIE-capable `ET_DYN` images use a randomized load bias when the current Resource
Domain has ASLR enabled. The load bias must:

- preserve each segment's `p_align`.
- keep all `PT_LOAD` segments non-overlapping.
- avoid stack, heap, argument page, and reserved FDR/runtime areas.
- use deterministic emulator entropy when deterministic test mode is active.
- be reported through auxv as `AT_BASE` or the LNP64-specific equivalent when a
  dynamic loader is introduced.

Stack, heap arena, anonymous `MMAP`, and guarded allocation randomization remain
owned by the emulator's process layout and VMA engines, not by ELF headers.

## Relocation Model

LNP64 uses RELA relocations: each relocation carries an explicit signed addend.
Static v1 linkers should resolve all relocations before producing `ET_EXEC`
unless the image is PIE-capable.

Provisional relocation numbers:

| Number | Name | Calculation |
| --- | --- | --- |
| 0 | `R_LNP64_NONE` | no relocation |
| 1 | `R_LNP64_ABS64` | `S + A` |
| 2 | `R_LNP64_ABS32` | low 32 bits of `S + A`; overflow is an error |
| 3 | `R_LNP64_PC32` | `S + A - P`; overflow is an error |
| 4 | `R_LNP64_GOT64` | address of GOT entry for `S + A` |
| 5 | `R_LNP64_GLOB_DAT` | `S + A` for data/code pointer slots |
| 6 | `R_LNP64_RELATIVE` | `B + A` |
| 7 | `R_LNP64_TLS_TPREL64` | TLS offset from thread pointer |
| 8 | `R_LNP64_TLS_DTPREL64` | TLS offset from module TLS base |
| 9 | `R_LNP64_FDR_DESC64` | startup FDR descriptor-table index plus addend |
| 10 | `R_LNP64_CAP_DESC64` | startup capability descriptor-table index plus addend |
| 11 | `R_LNP64_CALLGATE64` | call-gate descriptor-table index plus addend |
| 12 | `R_LNP64_TLS_TPREL_SLOT64` | 64-bit local-exec TLS offset slot loaded through AUIPC+LD |
| 13 | `R_LNP64_AUIPC` | v2 `AUIPC rd, imm32`: `rd = P + sext32(S + A - P)`; one 64-bit word, byte-granular pc-relative high part |
| 14 | `R_LNP64_BRANCH` | v2 B-type: instruction-count displacement `(S + A - P) >> 3`; must be 8-byte aligned and in range |
| 15 | `R_LNP64_JUMP` | v2 J-type: instruction-count displacement `(S + A - P) >> 3`; must be 8-byte aligned and in range |

Variables:

- `S`: resolved symbol value.
- `A`: relocation addend.
- `P`: place being relocated.
- `B`: load bias for PIE images.

Relocations that name FDRs, capabilities, or call gates do not forge authority.
They resolve to descriptor indexes or metadata records. The loader installs
actual FDR capabilities only from startup descriptors authorized by the parent
domain or boot manifest.

Canonical symbol materialization:

- Direct symbol address: `AUIPC rd, R_LNP64_PCREL_HI20(symbol+addend)`;
  `ADDI rd, rd, R_LNP64_PCREL_LO12_I(symbol+addend)`.
- Symbol address or constant via slot: `AUIPC tmp, R_LNP64_PCREL_HI20(slot)`;
  `LD rd, tmp, R_LNP64_PCREL_LO12_LD(slot)`, where the slot contains an
  `ABS64`, `GLOB_DAT`, `RELATIVE`, descriptor, or TLS offset relocation.
- Assembler `LA` is only a source macro for the direct two-instruction sequence.
  Object files, LLVM backend code, lld, and loader tests must use the explicit
  AUIPC relocation pair.
- Linker pair binding for `R_LNP64_PCREL_LO12_I` and
  `R_LNP64_PCREL_LO12_LD` is not finalized yet. Until the psABI chooses
  whether the low relocation binds by a named high relocation, a relocation
  group, or a nearest-preceding `R_LNP64_PCREL_HI20` rule, lld must reject the
  split PC-relative forms instead of applying an approximate per-relocation
  `S + A - P` calculation.
- Local-exec TLS address: `GET_PCR r_base, TLS_BASE`; materialize a signed
  TP-relative offset either directly when the backend can prove it fits the
  immediate form, or via an `R_LNP64_TLS_TPREL_SLOT64` slot loaded with
  AUIPC+LD; then add the offset to `r_base`. `R_LNP64_TLS_TPREL64` remains the
  64-bit relocation used to fill such slots.

## Static-Only v1 Policy

Native v1 loader policy is static-only for this profile:

- `dlopen`, `dlsym`, and `dlclose` remain fail-cleanly libc surfaces.
- `PT_DYNAMIC` and lazy binding are rejected by the native static loader.
- PIE is allowed only when all needed relocations are present in the image and
  can be applied before user code runs.
- Shared libraries may be represented in a future profile as prelinked image
  objects or by a user-space dynamic loader launched with explicit domain
  policy.

This keeps W^X, capability startup, and deterministic test behavior tractable
while real package compatibility is still being expanded. It is a loader policy,
not a hardware limitation on future executable formats.

## Startup Descriptors

LNP64 startup descriptors pass authority-bearing objects into a process without
encoding ambient privilege in memory addresses.

The loader accepts descriptor metadata from the boot manifest or parent domain
and publishes public metadata through `ENV_GET`/auxv. Descriptor payloads live in
hardware FDR slots. Hardware sees only the resulting startup metadata pointer
and FDR grants recorded in the exec-plan descriptor.

`.note.lnp64.startup` contains:

| Field | Size | Meaning |
| --- | --- | --- |
| magic | 8 | ASCII `LNP64ST\0` |
| version | 8 | note version, currently `1` |
| flags | 8 | startup behavior flags |
| argc_addr | 8 | process entry argc address or zero for default |
| argv_addr | 8 | argv table address or zero for default |
| envp_addr | 8 | envp table address or zero for default |
| auxv_addr | 8 | auxv table address or zero for `ENV_GET`-only metadata |
| fdr_count | 8 | number of descriptor records following |

Each descriptor record contains:

| Field | Size | Meaning |
| --- | --- | --- |
| slot | 8 | requested FDR slot, or `UINT64_MAX` for loader choice |
| kind | 8 | file, directory, queue, counter, socket, call gate, memory object, DMA buffer, device |
| rights | 8 | LNP64 capability rights mask |
| flags | 8 | inheritance, close-on-exec, sealed, transfer, revocable |
| object_id | 8 | manifest object id, zero when loader creates the object |
| generation | 8 | expected object generation, zero for newly created objects |
| name_offset | 8 | string-table offset for diagnostics or path-backed descriptors |
| reserved | 8 | must be zero |

The initial process receives:

- `fd0`, `fd1`, and `fd2` according to the boot manifest, usually console
  endpoints.
- executable image FDR metadata if the loader keeps the image object open.
- root/cwd namespace descriptors when the boot profile delegates them.
- device, event, timer, call-gate, or DMA descriptors only when explicitly named
  by the boot manifest or parent domain.

Descriptor inheritance across `exec` follows FDR capability metadata. A
close-on-exec flag prevents inheritance. Generation mismatches fail the exec
before user code starts.

## Exec-Plan Descriptor Boundary

The exec-plan descriptor is the only object consumed by hardware `EXEC`.
It is a bounded architecture record containing:

- descriptor version, total length, bounded record counts, flags, expected
  domain/process generations, and expected lineage epoch.
- entry PC, initial SP, optional TLS base, and startup metadata pointer.
- VMA records: target virtual address, length, protection, memory type,
  executable provenance class, source object/FDR capability, source offset,
  source generation, lineage epoch, zero-fill length, and mapping flags
  (reserved zero in v1).
- startup FDR grants and close-on-exec/preserve decisions.
- optional image measurement references for measured boot, audit, or
  attestation records.

The loader must choose ASLR layout, apply relocations, resolve static PIE
placement, build startup metadata, prepare source/memory capabilities, and
select FDR grants before calling `EXEC`. Hardware validates the descriptor's
shape, capabilities, generations, lineage, W^X/NX policy, executable
provenance, guard pages, memory type, Resource Domain policy, and FDR grant
authority. It does not validate ELF program-header intent or relocation
correctness directly.

If `EXEC` rejects the descriptor before its commit point, the old image remains
runnable and the caller receives an errno-style status. Once hardware commits,
the old address space and sibling thread contexts are gone and subsequent
startup/page/fetch faults belong to the new image.

## Loader Failure Rules

The software loader must fail before submitting `EXEC` when:

- an ELF header field is unsupported.
- a `PT_LOAD` segment overflows, overlaps another segment, or violates alignment.
- a segment requests unauthorized W+X permissions.
- a relocation overflows or names an unavailable symbol/descriptor.
- startup descriptors request undelegated rights.
- ASLR cannot find a non-overlapping aligned load range.

Failures report a deterministic errno-style status to the caller or boot
diagnostics path.

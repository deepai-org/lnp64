# Emulator Security Roadmap

This document lists emulator changes needed to implement and test the native
LNP64 security model: W^X, NX data, ASLR, guard pages, entropy, generation
checks, revocation, sealed/narrowed capabilities, and DMA isolation.

## 1. Resource Domain Security Policy

Extend Resource Domain state with security policy fields:

- `aslr_enabled`
- `allow_wx`
- `allow_jit_transition`
- entropy quota or entropy permission
- `dma_allowed`
- hardening profile
- executable-source policy

Enforce monotonic delegation. A child domain can become stricter than its
parent, but cannot enable broader executable-memory, DMA, entropy, device, or
capability-transfer authority than its parent delegated.

Tests:

- child cannot enable W+X if parent disallows it.
- child can disable ASLR for a deterministic test profile only when parent
  permits it.
- frozen, revoked, or destroyed domains reject security-sensitive operations.

## 2. VMA Protection Enforcement

Implement enforcement in `MMAP` and `MPROTECT`:

- reject `PROT_WRITE | PROT_EXEC` unless the current domain permits it.
- default anonymous, heap, stack, shared-memory, DMA, and device mappings to NX.
- add guard/no-access VMA support.
- make load, store, fetch, and DMA pin against guard pages fault.
- make instruction fetch from NX pages fault through the emulator's existing
  signal/fault path.

Tests:

- anonymous RW mapping cannot execute.
- executable mapping cannot be made writable without policy.
- JIT flow works: RW mapping, write code, `MPROTECT` to RX, then `ISYNC`.
- guard page faults on load, store, and fetch.
- `ALLOC_EX guard_before/guard_after` creates faulting guard regions.

## 3. `RANDOM`

Add opcode, assembler, and emulator support for `RANDOM`.

For deterministic tests, seed the emulator RNG from a fixed config value. The
emulator does not need cryptographic entropy, but it should preserve the
architectural behavior and failure modes.

Tests:

- scalar `RANDOM` writes a value to the destination register.
- buffer form fills the requested byte range.
- domain entropy quota or permission failures are reported correctly.
- `ENV_GET` does not expose secret random material.

## 4. ASLR

Make ASLR emulator-visible for:

- process stacks.
- heap arenas.
- anonymous `MMAP` with a null hint.
- executable/load segments if modeled by the loader.
- call-gate trampolines if represented.

Keep deterministic mode available for tests.

Tests:

- two execs or processes get different stack, heap, or mmap bases when ASLR is
  enabled.
- a child domain with ASLR disabled gets stable bases.
- randomized mappings remain aligned and non-overlapping.

## 5. Generation Checks

Make generation checks systematic for:

- FDR entries.
- domains.
- VMAs and mapped objects.
- call gates.
- event sources.
- DMA buffers.
- heap arenas or allocation slots in hardened mode.

Tests:

- stale FDR after close/reopen fails.
- stale domain handle fails.
- stale call gate fails.
- stale event source fails after source object destruction/recreation.
- stale VMA/object mapping cannot be used after revoke or unmap.

## 6. Sealed and Narrowed Capabilities

Extend capability metadata with:

- rights mask.
- transfer rights.
- narrowable bit.
- sealed bit.
- revocable lineage id.
- allowed range and mapping permissions where relevant.

Enforce:

- narrowing can only remove rights.
- sealed capabilities cannot be duplicated, narrowed, or reminted unless their
  rights explicitly allow it.
- `CAP_SEND` obeys transfer permission.
- `CAP_REVOKE` invalidates descendants.

Tests:

- read-only capability cannot be broadened to write.
- sealed capability can be used but not duplicated or narrowed.
- capability without transfer right cannot be sent.
- revoked child capability fails immediately.
- cached descriptor path observes revocation or generation mismatch.

## 7. DMA Isolation

Route `DMA_CTL` and device-style DMA through emulator checks even if the final
operation is a simple memory copy:

- VMA exists.
- target/source is not guard or no-access.
- direction permission is valid.
- FDR/object generation matches.
- domain DMA budget and permission allow it.
- device-visible DMA requires a `dma_buffer` FDR.
- revoked DMA buffers reject new work.

Tests:

- DMA to a guard page fails.
- DMA from an unmapped range fails.
- DMA using stale or revoked buffer fails.
- PCIe/device DMA only works through exported `dma_buffer` capabilities.
- revoke waits for or cancels in-flight descriptors according to the chosen
  emulator policy.

## 8. Fault and Signal Coverage

Security features should have stable failure behavior.

Tests:

- NX execute fault.
- W^X violation returns `EACCES` or `EPERM`.
- guard page load/store fault.
- invalid DMA emits a completion fault or event.
- stale capability returns `EBADF` or the chosen stale-reference error.
- signal frame lives in a non-executable protected stack area.

## Recommended Order

1. Add Resource Domain security policy fields.
2. Implement VMA W^X, NX, and guard enforcement.
3. Add `RANDOM` and deterministic ASLR.
4. Add capability sealing, narrowing, and revocation tests.
5. Make generation checks systematic across stale handles.
6. Add DMA isolation checks.
7. Add end-to-end demos: secure JIT, sandboxed domain, revoked DMA buffer, and
   guarded heap overflow.

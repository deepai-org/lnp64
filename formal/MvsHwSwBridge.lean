/- LNP64 MVS hardware/software contract bridge.

This file closes the loop between the Yosys SVA proofs on the RTL and the
software guarantee a kernel cares about. The properties model-checked on the
actual SystemVerilog (see scripts/run_rtl_mvs_formal.sh) are taken here as
*axioms* (a `HwContract`), and Lean proves that GIVEN those axioms, a tiny
two-tenant kernel that grants each tenant a disjoint capability is memory-safe:
neither tenant can ever write into the other's memory.

The axioms correspond exactly to the discharged RTL gates:
  * `mediated`     <- mvs mediation  (a write occurs only with an authorizing,
                                      write-capable, page-confined capability)
  * `unforgeable`  <- mvs derivation (the capability used is within the
                                      kernel-granted root -- no forged authority)
Revocation, non-interference, reset isolation and bounded progress are the
temporal/availability companions also proven on the RTL.

Kernel tactics only (no sorry/admit/native_decide).
-/

namespace Lnp64.MvsBridge

/-- 8-bit address; its page is the top nibble (matches the RTL `addr[7:4]`). -/
def addrPage (a : Nat) : Nat := a / 16

/-- A capability: a single writable page (a bounded region in the full design). -/
structure Cap where
  page : Nat
  canWrite : Bool
deriving DecidableEq, Repr

/-- The kernel's capability setup: tenant 0 owns page 1, tenant 1 owns page 2,
    everyone else holds nothing. The two tenant pages are disjoint. -/
def kernelCap : Nat -> Cap
  | 0 => { page := 1, canWrite := true }
  | 1 => { page := 2, canWrite := true }
  | _ => { page := 0, canWrite := false }

/-- The hardware contract: the facts proven about the RTL by the Yosys gates,
    assumed here as axioms. `writeOccurs i a c` reads "initiator `i` commits a
    memory write to address `a` while holding capability `c`". -/
structure HwContract (writeOccurs : Nat -> Nat -> Cap -> Prop) : Prop where
  /-- Mediation (mvs mediation gate): a committed write is authorized by a
      write-capable capability and is confined to that capability's page. -/
  mediated : ∀ i a c, writeOccurs i a c -> c.canWrite = true ∧ addrPage a = c.page
  /-- No forged authority (mvs derivation gate): the capability actually used is
      within the kernel-granted root for that initiator. -/
  unforgeable : ∀ i a c, writeOccurs i a c -> c.page = (kernelCap i).page

variable {writeOccurs : Nat -> Nat -> Cap -> Prop}

/-- Every committed write lands on the initiator's kernel-granted page. -/
theorem write_confined_to_kernel_page (hw : HwContract writeOccurs)
    (i a : Nat) (c : Cap) (h : writeOccurs i a c) :
    addrPage a = (kernelCap i).page := by
  have hmed := (hw.mediated i a c h).2      -- addrPage a = c.page
  have hcap := hw.unforgeable i a c h        -- c.page = (kernelCap i).page
  rw [hmed, hcap]

/-- Tenant isolation: tenant 0 can never write into tenant 1's memory (page 2),
    and tenant 1 can never write into tenant 0's memory (page 1). -/
theorem tenant0_cannot_write_tenant1 (hw : HwContract writeOccurs)
    (a : Nat) (c : Cap) (h : writeOccurs 0 a c) :
    addrPage a ≠ (kernelCap 1).page := by
  have := write_confined_to_kernel_page hw 0 a c h   -- addrPage a = page 1
  simp [kernelCap, addrPage] at this ⊢
  omega

theorem tenant1_cannot_write_tenant0 (hw : HwContract writeOccurs)
    (a : Nat) (c : Cap) (h : writeOccurs 1 a c) :
    addrPage a ≠ (kernelCap 0).page := by
  have := write_confined_to_kernel_page hw 1 a c h   -- addrPage a = page 2
  simp [kernelCap, addrPage] at this ⊢
  omega

/-- The two-tenant memory-safety theorem: given the RTL hardware contract, no
    write by one tenant ever lands in the other tenant's disjoint page. -/
theorem two_tenant_memory_safety (hw : HwContract writeOccurs)
    (a : Nat) (c : Cap) :
    (writeOccurs 0 a c -> addrPage a ≠ (kernelCap 1).page) ∧
    (writeOccurs 1 a c -> addrPage a ≠ (kernelCap 0).page) :=
  ⟨tenant0_cannot_write_tenant1 hw a c, tenant1_cannot_write_tenant0 hw a c⟩

end Lnp64.MvsBridge

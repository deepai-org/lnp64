# Planned Refactor: Processes Are Uniform Resource Domains

Status: **planned design refactor**, not yet implemented. This is an architecture
change (touches `design.md`, `hardware_design.md`, `formal_theorems.md`, the
domain/capability engine, the shared schema, and the M-series confinement
invariants). It is *not* an OS/personality change and must not move any currently
frozen POSIX mechanism out of silicon.

## The idea

Collapse `process`, `container`, `VM`, `sandbox`, and `supervisor` into **one
recursive primitive: the Resource Domain.** Today `design.md` treats a *process*
(PID/PPID/PID-group, `CLONE profile=posix_fork`, per-process context) as a
distinct notion alongside Resource Domains (nested containment + accounting).
This refactor makes them the same object at the mechanism layer.

A **Resource Domain is one uniform node** that may *hold* any of:

- zero or more **threads** (the schedulable leaves — threads stay a separate axis,
  not domains),
- zero or more **address-space capabilities** (an address space is a *held
  capability*, not intrinsic to the node),
- zero or more **child domains**,
- other capabilities (FDRs, device/DMA authority, gates, …),
- a **budget** / accounting record.

There is **no `process` vs `container` type tag** and **no leaf-vs-interior
variant.** What a domain *is* (process, container, VM) is just which slots it
populates: a "process" is a domain that holds an address-space capability + one
or more threads; a "container/VM" is a domain that holds child domains; a node
can do both at once (a shell runs threads *and* spawns children, like real Unix).

## Why this is a simplification, not a complication

- **Removes a hardware object type.** One uniform node instead of process-struct
  + domain. Silicon shrinks.
- **One lifecycle, accounting, and confinement, proven once.** create / freeze /
  resume / destroy / death-event, budgets, attach/detach, capability scope —
  defined for domains, inherited by "processes." rlimits and cgroup limits become
  one budget mechanism at different nesting depths.
- **fork/exec/clone fall out of capability rules.** `fork` = spawn a child domain
  that COW-inherits the parent's address-space capability + narrowed FDRs; `exec`
  = replace the image held by a domain; the whole `clone()` spectrum (share VM?
  fds? signal handlers?) = *which capabilities are shared* with the child domain.
  The process/thread continuum stops being a special case.
- **Address space as a held capability** makes shared-VM clones and vfork natural
  (two domains referencing the same address-space object) instead of special
  cases — strictly more expressive at no extra concept count.
- **Recursion is free.** "A process sub-delegates to child sandboxes" is the same
  nesting as the guest-can-be-a-host hypervisor model; no new machinery.

This completes a thesis already in the README: Resource Domains are "one nested
containment primitive" for containers/VMs/supervisors/cgroups. The process was
the missing case; this folds it in.

## Have cake and eat it: keep POSIX frozen in silicon

This refactor must **not** push more into the OS than is already there. Freezing a
stable POSIX subset in hardware is on-thesis (the core LNP64 bet). The refactor is
a **consolidation of hardware objects, not a migration of policy to software.**

- Every currently frozen POSIX mechanism stays frozen, now **domain-bound** instead
  of bound to a separate process struct: signal delivery via gates
  (`GATE_*`/`SIGRET`), futex wait/wake, FDR read/write, `CLONE`/`EXEC`/`EXIT`,
  `AWAIT_EX`, scheduler runqueue admission. Nothing leaves the chip.
- Process bookkeeping is **already domain-keyed** in the current design (the
  per-domain PID counter checked by `domain_nested_test`, and PCRs `PID`/`PPID`).
  This refactor extends that, it does not relocate it.
- The thin process service (path resolution, namespace policy) stays exactly as
  thin as today. No new OS surface.

## The one invariant: names are data, authority is capability

The Unix process bits that are *not* tree-shaped — PID-as-name, process
groups, sessions, wait/zombie/reaping, signal targeting by pgrp — **stay frozen in
silicon as domain-keyed indices/relations classified as data/observability, never
as authority.** This is already required by the **evidence-honesty** severe goal
("observability/naming are data paths only; cannot become hidden authority").

The single discipline that keeps the abstraction clean while POSIX stays frozen:

> A frozen name (PID, pgrp, session) is **addressing only**. The authority to act
> through it always rides a held capability. `kill(pid, sig)` resolves the PID in
> hardware **and** checks a capability to the target domain/group. No act-by-
> integer ambient authority, ever.

Hold that line and you get clean abstraction + frozen POSIX + compatibility
simultaneously. The *only* way to lose cleanliness is to let a frozen PID become
ambient authority the way stock Unix does — which this design refuses by an
existing severe goal.

## The conscious grafts (named, not rediscovered later)

A capability machine emulating Unix has a few impedance points. They are accepted
grafts, handled at the level already frozen, unchanged by this refactor:

- **setuid/setgid exec = authority amplification** (opposite of monotone
  capability narrowing). Handled as today: the loader confers a capability derived
  from the file's setuid bit; the binary's domain is pre-granted that authority.
- **Orphan reparenting to init/subreaper = tree mutation on death.** Modeled as a
  domain death-event plus a reparent policy, not arbitrary re-homing of authority.
- **PID reuse / `getpid` stability / kill-by-pid** = the ambient-naming graft,
  contained by the names-are-data invariant above.

## Work items (when scheduled)

1. `design.md`: replace the separate "process" notion with "a process is a
   Resource Domain holding an address-space capability + threads"; define the
   uniform node and its held slots; state the names-are-data invariant.
2. `hardware_design.md` + shared schema: one uniform domain record (drop any
   process/domain split); address space as a held capability; domain-keyed
   PID/pgrp/session/reaping indices marked data-only.
3. `formal_theorems.md` / M-series: re-confirm domain-confinement invariants hold
   when `fork`/`exec`/`signal`/`wait` are expressed as domain operations (more
   proof *leverage* — prove containment once — but real schema/Lean churn).
4. Cost guard: a **lightweight leaf profile** so fork-heavy workloads
   (`make -j`, pipelines, fork storms) do not pay container-weight cost or exhaust
   a finite domain table; define fail-closed behavior on slot exhaustion.

## Non-goals

- Not moving any frozen POSIX mechanism into the OS/personality.
- Not making threads into domains (threads stay the scheduling axis).
- Not adding ambient (capability-free) authority via PIDs/pgrps.

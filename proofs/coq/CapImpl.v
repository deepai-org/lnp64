(* LNP64 unpipelined mediation implementation + refinement to the spec.

   Phase 1 vertical slice. Following the standard refinement discipline, the
   implementation is kept STRUCTURALLY IDENTICAL to the abstract spec
   (CapSpec.v): same state shape, one cycle = one operation, no pipelining and
   no split transactions yet. The simulation relation is therefore equality, and
   the refinement theorem is: the implementation's DETERMINISTIC cycle function
   is, for every input, a legal abstract Step. Hence every implementation-
   reachable state is spec-reachable and the spec's security theorems
   (no forged authority, write confinement, two-tenant isolation) transfer to
   the implementation.

   The implementation uses DECIDABLE (bit-level / hardware-realizable) checks --
   a bounds comparison for the capability subset and a permission+range check
   for authorization -- and we prove those decidable checks imply the abstract
   (authorization-based) predicates. This is the deterministic denotation of a
   single unpipelined Koika rule; encoding it in the Koika DSL and extracting
   Verilog is the mechanical connecting step. *)

From Coq Require Import Arith Lia Bool.
Require Import CapSpec.

Section Impl.

Variable root : nat -> Cap.

(* ---- Decidable, hardware-realizable checks ---- *)

Definition authorizedb (s : State) (i a : nat) : bool :=
  w (cap s i) && (lo (cap s i) <=? a) && (a <=? hi (cap s i)).

Lemma authorizedb_true : forall s i a,
  authorizedb s i a = true <-> authorized s i a.
Proof.
  intros s i a. unfold authorizedb, authorized, inRange.
  rewrite !andb_true_iff, Nat.leb_le, Nat.leb_le. tauto.
Qed.

Lemma authorizedb_false : forall s i a,
  authorizedb s i a = false -> ~ authorized s i a.
Proof.
  intros s i a Hf Hc. apply authorizedb_true in Hc. rewrite Hc in Hf. discriminate.
Qed.

(* Hardware checks a bounds subset (decidable); it implies the abstract
   authorization-based subset. *)
Definition capSubsetb (c1 c2 : Cap) : bool :=
  (lo c2 <=? lo c1) && (hi c1 <=? hi c2) && (negb (w c1) || w c2).

Lemma capSubsetb_sound : forall c1 c2,
  capSubsetb c1 c2 = true -> capSubset c1 c2.
Proof.
  intros c1 c2 H. unfold capSubsetb in H.
  rewrite !andb_true_iff, !Nat.leb_le in H.
  destruct H as [[Hlo Hhi] Hw].
  unfold capSubset. split.
  - intros a [Ha1 Ha2]. unfold inRange. lia.
  - intro Hw1. apply orb_true_iff in Hw. destruct Hw as [Hn | Hy].
    + rewrite Hw1 in Hn. discriminate.
    + exact Hy.
Qed.

(* ---- The deterministic, unpipelined cycle function ---- *)

Definition impl_step (s : State) (op : Op) : State :=
  match op with
  | OpWrite i a v =>
      if authorizedb s i a
      then {| cap := cap s; mem := upd_mem (mem s) a (i, v) |}
      else s
  | OpDerive i c =>
      if capSubsetb c (cap s i)
      then {| cap := upd_cap (cap s) i c; mem := mem s |}
      else s
  | OpRevoke i =>
      {| cap := upd_cap (cap s) i emptyCap; mem := mem s |}
  | OpNop => s
  end.

(* ---- Refinement: every implementation cycle is a legal abstract step ---- *)

Theorem impl_simulates : forall s op, exists op', Step s op' (impl_step s op).
Proof.
  intros s op. destruct op as [i a v | i c | i |].
  - (* write *) destruct (authorizedb s i a) eqn:E.
    + exists (OpWrite i a v). simpl. rewrite E.
      apply StepWriteOk. apply authorizedb_true; exact E.
    + exists (OpWrite i a v). simpl. rewrite E.
      apply StepWriteDenied. apply authorizedb_false; exact E.
  - (* derive *) destruct (capSubsetb c (cap s i)) eqn:E.
    + exists (OpDerive i c). simpl. rewrite E.
      apply StepDerive. apply capSubsetb_sound; exact E.
    + exists OpNop. simpl. rewrite E. apply StepNop.
  - (* revoke *) exists (OpRevoke i). simpl. apply StepRevoke.
  - (* nop *) exists OpNop. simpl. apply StepNop.
Qed.

(* ---- Implementation reachability and transfer of the security theorems ---- *)

Inductive ImplReachable : State -> Prop :=
| IReachInit : ImplReachable (init root)
| IReachStep : forall s op, ImplReachable s -> ImplReachable (impl_step s op).

(* Every implementation-reachable state is reachable in the spec. *)
Theorem impl_refines_spec : forall s, ImplReachable s -> Reachable root s.
Proof.
  intros s H. induction H.
  - apply ReachInit.
  - destruct (impl_simulates s op) as [op' Hstep].
    eapply ReachStep; eauto.
Qed.

(* The spec's security theorems therefore hold of the implementation. *)
Theorem impl_no_forged_authority : forall s i,
  ImplReachable s -> capSubset (cap s i) (root i).
Proof.
  intros s i H. apply (no_forged_authority root). apply impl_refines_spec; exact H.
Qed.

Theorem impl_writes_confined_to_root : forall s a i v,
  ImplReachable s -> mem s a = Some (i, v) -> inRange a (root i).
Proof.
  intros s a i v H He.
  apply (writes_confined_to_root root s a i v); [apply impl_refines_spec; exact H | exact He].
Qed.

Theorem impl_two_tenant_isolation : forall s a v,
  ImplReachable s ->
  Disjoint (root 0) (root 1) ->
  (mem s a = Some (0, v) -> ~ inRange a (root 1)) /\
  (mem s a = Some (1, v) -> ~ inRange a (root 0)).
Proof.
  intros s a v H Hd.
  apply (two_tenant_isolation root s a v); [apply impl_refines_spec; exact H | exact Hd].
Qed.

End Impl.

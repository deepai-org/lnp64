(* LNP64 abstract capability-machine specification (foundational, Coq).

   This is the *abstract spec* the hardware must refine: a capability machine
   whose only way to change memory is an authorized, page-confined write, whose
   capabilities can only ever be narrowed (no forged authority), and whose two
   tenants -- granted disjoint root capabilities by the kernel -- are therefore
   memory-isolated.

   The security theorems are proven here in Coq's kernel over ALL reachable
   states (an inductive invariant), independent of any solver. The next layer
   (a Koika implementation) is proven to refine THIS machine, so the same
   theorems transfer to the extracted Verilog. *)

From Coq Require Import Arith Lia.

(* ---- Capabilities: an address range plus a write permission ---- *)

Record Cap := { lo : nat; hi : nat; w : bool }.

Definition inRange (a : nat) (c : Cap) : Prop := lo c <= a /\ a <= hi c.

(* A capability c1 is a subset of c2 when it authorizes no address c2 does not
   and carries no more permission. (Authorization-based, so the empty capability
   is a subset of every capability -- the right semantics for revoke/derive.) *)
Definition capSubset (c1 c2 : Cap) : Prop :=
  (forall a, inRange a c1 -> inRange a c2) /\ (w c1 = true -> w c2 = true).

Lemma capSubset_refl : forall c, capSubset c c.
Proof. intro c. unfold capSubset. split; auto. Qed.

Lemma capSubset_trans : forall a b c,
  capSubset a b -> capSubset b c -> capSubset a c.
Proof.
  intros a b c [Hr1 Hw1] [Hr2 Hw2].
  unfold capSubset. split.
  - intros x Hx. apply Hr2, Hr1, Hx.
  - intro Ha. apply Hw2, Hw1, Ha.
Qed.

(* A subset capability authorizes no address its parent does not. *)
Lemma inRange_subset : forall a c1 c2,
  capSubset c1 c2 -> inRange a c1 -> inRange a c2.
Proof.
  intros a c1 c2 [Hr _] Ha. apply Hr, Ha.
Qed.

(* ---- The abstract capability machine ---- *)

Section CapMachine.

(* Initiators are numbered; each has a fixed kernel-granted root capability. *)
Variable root : nat -> Cap.

Record State := {
  cap : nat -> Cap;                  (* the capability each initiator holds *)
  mem : nat -> option (nat * nat)    (* per cell: (writer id, value), if written *)
}.

Inductive Op :=
| OpWrite (i a v : nat)
| OpDerive (i : nat) (c : Cap)
| OpRevoke (i : nat)
| OpNop.

Definition emptyCap : Cap := {| lo := 1; hi := 0; w := false |}.  (* lo>hi: empty *)

Definition upd_cap (f : nat -> Cap) (i : nat) (c : Cap) : nat -> Cap :=
  fun j => if Nat.eqb j i then c else f j.

Definition upd_mem (f : nat -> option (nat * nat)) (a : nat) (e : nat * nat)
  : nat -> option (nat * nat) :=
  fun b => if Nat.eqb b a then Some e else f b.

(* Authorization: the holder's capability permits writing this address. *)
Definition authorized (s : State) (i a : nat) : Prop :=
  w (cap s i) = true /\ inRange a (cap s i).

Inductive Step : State -> Op -> State -> Prop :=
| StepWriteOk : forall s i a v,
    authorized s i a ->
    Step s (OpWrite i a v)
      {| cap := cap s; mem := upd_mem (mem s) a (i, v) |}
| StepWriteDenied : forall s i a v,
    ~ authorized s i a ->
    Step s (OpWrite i a v) s            (* fail-closed: no memory change *)
| StepDerive : forall s i c,
    capSubset c (cap s i) ->            (* derive only a subset of what is held *)
    Step s (OpDerive i c)
      {| cap := upd_cap (cap s) i c; mem := mem s |}
| StepRevoke : forall s i,
    Step s (OpRevoke i)
      {| cap := upd_cap (cap s) i emptyCap; mem := mem s |}
| StepNop : forall s, Step s OpNop s.

(* The kernel's initial state: each initiator holds exactly its root cap; memory
   is empty. *)
Definition init : State := {| cap := root; mem := fun _ => None |}.

Inductive Reachable : State -> Prop :=
| ReachInit : Reachable init
| ReachStep : forall s o t, Reachable s -> Step s o t -> Reachable t.

(* ---- Invariants ---- *)

(* I1: every held capability is a subset of its kernel-granted root
       (no forged authority -- capabilities only ever narrow). *)
Definition InvCap (s : State) : Prop :=
  forall i, capSubset (cap s i) (root i).

(* I2: every written memory cell lies within its writer's root capability
       (mediation/confinement, carried in the state). *)
Definition InvMem (s : State) : Prop :=
  forall a i v, mem s a = Some (i, v) -> inRange a (root i).

Definition Inv (s : State) : Prop := InvCap s /\ InvMem s.

Lemma Inv_init : Inv init.
Proof.
  split.
  - intro i. simpl. apply capSubset_refl.
  - intros a i v H. simpl in H. discriminate.
Qed.

Lemma upd_cap_same : forall f i c, upd_cap f i c i = c.
Proof. intros. unfold upd_cap. now rewrite Nat.eqb_refl. Qed.

Lemma upd_cap_other : forall f i c j, j <> i -> upd_cap f i c j = f j.
Proof. intros. unfold upd_cap. now rewrite (proj2 (Nat.eqb_neq j i) H). Qed.

Lemma Inv_step : forall s o t, Inv s -> Step s o t -> Inv t.
Proof.
  intros s o t [Hcap Hmem] Hstep.
  destruct Hstep.
  - (* StepWriteOk: caps unchanged; new cell is within writer's root *)
    split.
    + exact Hcap.
    + intros a' i' v' He. simpl in He. unfold upd_mem in He.
      destruct (Nat.eqb a' a) eqn:E.
      * apply Nat.eqb_eq in E. subst a'. inversion He; subst i' v'.
        destruct H as [Hw Hin].
        apply (inRange_subset a (cap s i) (root i)); [apply Hcap | exact Hin].
      * exact (Hmem a' i' v' He).
  - (* StepWriteDenied: nothing changes *)
    split; assumption.
  - (* StepDerive: cap stays a subset of root; mem unchanged *)
    split.
    + intro j. destruct (Nat.eq_dec j i) as [->|Hne].
      * simpl. rewrite upd_cap_same.
        apply (capSubset_trans c (cap s i) (root i)); [assumption | apply Hcap].
      * simpl. rewrite upd_cap_other by assumption. apply Hcap.
    + exact Hmem.
  - (* StepRevoke: empty cap is a subset of anything; mem unchanged *)
    split.
    + intro j. destruct (Nat.eq_dec j i) as [->|Hne].
      * simpl. rewrite upd_cap_same. split.
        -- intros x Hx. unfold inRange, emptyCap in Hx. simpl in Hx. lia.
        -- intro Hc. unfold emptyCap in Hc. simpl in Hc. discriminate.
      * simpl. rewrite upd_cap_other by assumption. apply Hcap.
    + exact Hmem.
  - (* StepNop *)
    split; assumption.
Qed.

Theorem reachable_Inv : forall s, Reachable s -> Inv s.
Proof.
  intros s H. induction H.
  - apply Inv_init.
  - eapply Inv_step; eauto.
Qed.

(* ---- Security theorems ---- *)

(* No forged authority: in every reachable state, every capability is within its
   root. *)
Theorem no_forged_authority : forall s i,
  Reachable s -> capSubset (cap s i) (root i).
Proof. intros s i H. exact (proj1 (reachable_Inv s H) i). Qed.

(* Mediation/confinement: every written cell is within its writer's root. *)
Theorem writes_confined_to_root : forall s a i v,
  Reachable s -> mem s a = Some (i, v) -> inRange a (root i).
Proof. intros s a i v H He. exact (proj2 (reachable_Inv s H) a i v He). Qed.

(* Two-tenant isolation: if tenants 0 and 1 are granted disjoint roots, then no
   cell ever written by tenant 0 lies in tenant 1's region, and vice versa --
   neither tenant can ever touch the other's memory. *)
Definition Disjoint (c1 c2 : Cap) : Prop :=
  forall a, inRange a c1 -> ~ inRange a c2.

Theorem two_tenant_isolation : forall s a v,
  Reachable s ->
  Disjoint (root 0) (root 1) ->
  (mem s a = Some (0, v) -> ~ inRange a (root 1)) /\
  (mem s a = Some (1, v) -> ~ inRange a (root 0)).
Proof.
  intros s a v Hr Hdisj. split; intro He.
  - apply Hdisj. apply (writes_confined_to_root s a 0 v Hr He).
  - intro Hin1.
    pose proof (writes_confined_to_root s a 1 v Hr He) as Hin1'.
    apply (Hdisj a Hin1 Hin1').
Qed.

End CapMachine.

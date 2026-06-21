(*! LNP64 mediation choke point in Koika (extractable to Verilog).

   This is the Koika realization of the unpipelined write path proven in
   proofs/coq/CapImpl.v (impl_step's OpWrite case): a single rule that gates a
   memory write strobe on the capability check -- write permission AND the
   address within [cap_lo, cap_hi]. Koika's rule semantics make the write atomic;
   extracting this with cuttlec yields the Verilog whose mediation property is
   then cross-checked by the SVA harness. The capability registers are revocable
   (cap_w cleared) but never re-validated in this slice, matching the spec. *)

Require Import Koika.Frontend.

Module LNP64Mediation.
  Definition ext_fn_t := empty_ext_fn_t.

  Inductive reg_t :=
  | cap_lo        (* capability lower bound *)
  | cap_hi        (* capability upper bound *)
  | cap_w         (* capability write permission *)
  | req_valid     (* a memory request is presented *)
  | req_we        (* the request is a write *)
  | req_addr      (* the request address *)
  | out_we        (* mediated write strobe (the single choke point) *)
  | out_addr.     (* address of the mediated write *)

  Definition R r :=
    match r with
    | cap_lo => bits_t 8
    | cap_hi => bits_t 8
    | cap_w => bits_t 1
    | req_valid => bits_t 1
    | req_we => bits_t 1
    | req_addr => bits_t 8
    | out_we => bits_t 1
    | out_addr => bits_t 8
    end.

  Definition init_r idx : R idx :=
    match idx with
    | cap_lo => Bits.of_nat 8 16     (* boot capability: page 0x10..0x7f *)
    | cap_hi => Bits.of_nat 8 127
    | cap_w => Ob~1
    | req_valid => Bits.zero
    | req_we => Bits.zero
    | req_addr => Bits.zero
    | out_we => Bits.zero
    | out_addr => Bits.zero
    end.

  (* The single mediation rule: the memory write strobe is the AND of the
     request being a valid write and the capability authorizing the address. *)
  Definition mediate : uaction reg_t ext_fn_t :=
    {{
        let v := read0(req_valid) in
        let we := read0(req_we) in
        let a := read0(req_addr) in
        let lo := read0(cap_lo) in
        let hi := read0(cap_hi) in
        let cw := read0(cap_w) in
        let authorized :=
          v && we && cw && (lo <= a) && (a <= hi) in
        write0(out_we, authorized);
        write0(out_addr, a)
    }}.

  Inductive rule_name_t := do_mediate.

  Definition rules :=
    tc_rules R empty_Sigma
             (fun rl => match rl with do_mediate => mediate end).

  Definition sched : scheduler := do_mediate |> done.

  Definition package :=
    {| ip_koika := {| koika_reg_types := R;
                      koika_reg_init := init_r;
                      koika_ext_fn_types := empty_Sigma;
                      koika_rules := rules;
                      koika_rule_external _ := false;
                      koika_scheduler := sched;
                      koika_module_name := "lnp64_mediation" |};
       ip_sim := {| sp_ext_fn_specs := empty_ext_fn_props;
                    sp_prelude := None |};
       ip_verilog := {| vp_ext_fn_specs := empty_ext_fn_props; |} |}.
End LNP64Mediation.

Definition prog := Interop.Backends.register LNP64Mediation.package.
Extraction "lnp64_mediation.ml" prog.

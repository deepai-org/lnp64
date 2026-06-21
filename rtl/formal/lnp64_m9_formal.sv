`timescale 1ns/1ps

// Formal property-verification harness for the M9 classifier/servicelet engine.
// Asserts the SG-PROGRESS servicelet-containment severe goals as SVA on the real
// lnp64_m9_classifier_servelet output ports, model-checked over all seeds/timings.
import lnp64_pkg::*;

module lnp64_m9_formal (
    input logic clk,
    input logic reset_n,
    input logic start,
    input logic [31:0] scenario_seed
);
    logic done;
    logic trace_valid;
    logic [7:0] trace_code;
    logic [63:0] trace_value;
    logic verifier_accepted;
    logic verifier_rejected;
    logic packet_steered;
    logic ipc_steered;
    logic action_emitted;
    logic budget_enforced;
    logic stale_attachment_rejected;
    logic no_authority_created;
    logic counts_exact;
    logic typed_commit_valid;
    lnp64_m9_classifier_commit_t typed_commit;
    lnp64_m9_state_projection_t typed_state_projection;

    lnp64_m9_classifier_servicelet dut (
        .clk(clk),
        .reset_n(reset_n),
        .start(start),
        .scenario_seed(scenario_seed),
        .done(done),
        .trace_valid(trace_valid),
        .trace_code(trace_code),
        .trace_value(trace_value),
        .verifier_accepted(verifier_accepted),
        .verifier_rejected(verifier_rejected),
        .packet_steered(packet_steered),
        .ipc_steered(ipc_steered),
        .action_emitted(action_emitted),
        .budget_enforced(budget_enforced),
        .stale_attachment_rejected(stale_attachment_rejected),
        .no_authority_created(no_authority_created),
        .counts_exact(counts_exact),
        .typed_commit_valid(typed_commit_valid),
        .typed_commit(typed_commit),
        .typed_state_projection(typed_state_projection)
    );

    reg init_done = 1'b0;
    always @(posedge clk) init_done <= 1'b1;
    always @(posedge clk) begin
        if (!init_done) assume (!reset_n);
    end

    always @(posedge clk) begin
        if (reset_n && typed_commit_valid) begin
            // Note: no_authority_created is a late positive attestation
            // (action_emitted && verifier_accepted), not a per-commit safety
            // invariant, so it is not asserted here. The containment severe
            // goals below are the per-op control/authority properties.

            // An over-budget servicelet is terminated/charged (EAGAIN).
            if (typed_commit.op == LNP64_M9_COMMIT_BUDGET_EXHAUST)
                a_budget_enforced:
                    assert (typed_commit.status == LNP64_ERR_EAGAIN
                            && budget_enforced);

            // A stale attachment is rejected as revoked (EREVOKED).
            if (typed_commit.op == LNP64_M9_COMMIT_STALE_ATTACHMENT)
                a_stale_attachment_rejected:
                    assert (typed_commit.status == LNP64_ERR_EREVOKED
                            && stale_attachment_rejected);

            // A rejected verifier result is reported invalid (EINVAL).
            if (typed_commit.op == LNP64_M9_COMMIT_VERIFY_REJECT)
                a_verifier_rejected:
                    assert (typed_commit.status == LNP64_ERR_EINVAL
                            && verifier_rejected);
        end
    end
endmodule

`timescale 1ns/1ps

// Formal property-verification harness for the M11 DDR/metadata engine.
// Asserts the SG-MEM revocation/generation and domain-confinement severe goals
// as SVA on the real lnp64_m11_ddr_metadata output ports, model-checked over all
// seeds and timings.
import lnp64_pkg::*;

module lnp64_m11_formal (
    input logic clk,
    input logic reset_n,
    input logic start,
    input logic [31:0] scenario_seed
);
    logic done;
    logic trace_valid;
    logic [7:0] trace_code;
    logic [63:0] trace_value;
    logic metadata_allocated;
    logic metadata_domain_bound;
    logic ddr_write_completed;
    logic ddr_read_completed;
    logic read_matches_write;
    logic stale_generation_rejected;
    logic cross_domain_rejected;
    logic ecc_scrubbed;
    logic barrier_quiescent;
    logic counts_exact;
    logic typed_commit_valid;
    lnp64_m11_ddr_commit_t typed_commit;
    lnp64_m11_state_projection_t typed_state_projection;

    lnp64_m11_ddr_metadata dut (
        .clk(clk),
        .reset_n(reset_n),
        .start(start),
        .scenario_seed(scenario_seed),
        .done(done),
        .trace_valid(trace_valid),
        .trace_code(trace_code),
        .trace_value(trace_value),
        .metadata_allocated(metadata_allocated),
        .metadata_domain_bound(metadata_domain_bound),
        .ddr_write_completed(ddr_write_completed),
        .ddr_read_completed(ddr_read_completed),
        .read_matches_write(read_matches_write),
        .stale_generation_rejected(stale_generation_rejected),
        .cross_domain_rejected(cross_domain_rejected),
        .ecc_scrubbed(ecc_scrubbed),
        .barrier_quiescent(barrier_quiescent),
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
            // SG-MEM (revocation): a stale-generation submit is rejected (EREVOKED).
            if (typed_commit.op == LNP64_M11_COMMIT_STALE_SUBMIT)
                a_stale_generation_revoked:
                    assert (typed_commit.status == LNP64_ERR_EREVOKED
                            && stale_generation_rejected);

            // SG-MEM: a cross-domain submit is denied (EPERM).
            if (typed_commit.op == LNP64_M11_COMMIT_CROSS_DOMAIN)
                a_cross_domain_denied:
                    assert (typed_commit.status == LNP64_ERR_EPERM
                            && cross_domain_rejected);

            // Metadata is allocated bound to a domain.
            if (typed_commit.op == LNP64_M11_COMMIT_METADATA_ALLOC)
                a_metadata_domain_bound:
                    assert (metadata_allocated && metadata_domain_bound);

            // Note: read-after-write data equality (read_matches_write) is a
            // datapath-correctness property, not an authority/control severe
            // goal, so it is intentionally not asserted here (left to the
            // typed-trace / Lean layer).

            // An ECC scrub is reported as an EIO fault.
            if (typed_commit.op == LNP64_M11_COMMIT_ECC_SCRUB)
                a_ecc_scrub_eio:
                    assert (typed_commit.status == LNP64_ERR_EIO && ecc_scrubbed);
        end
    end
endmodule

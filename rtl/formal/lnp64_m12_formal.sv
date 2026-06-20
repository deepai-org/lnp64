`timescale 1ns/1ps

// Formal property-verification harness for the M12 storage-barrier engine.
//
// Instantiates the real lnp64_m12_storage_barrier RTL and asserts the SG-MEM
// storage severe goals as SVA on its output ports. start/scenario_seed are free
// inputs and reset is a power-on discipline, so SymbiYosys explores all seeds and
// timings -- a proof about the hardware over all behaviours, not one trace.
import lnp64_pkg::*;

module lnp64_m12_formal (
    input logic clk,
    input logic reset_n,
    input logic start,
    input logic [31:0] scenario_seed
);
    logic done;
    logic trace_valid;
    logic [7:0] trace_code;
    logic [63:0] trace_value;
    logic boot_image_visible;
    logic block_object_authorized;
    logic block_write_completed;
    logic storage_barrier_issued;
    logic storage_barrier_quiescent;
    logic stale_object_rejected;
    logic cross_domain_rejected;
    logic media_fault_terminal;
    logic no_raw_device_authority;
    logic counts_exact;
    logic typed_commit_valid;
    lnp64_m12_storage_commit_t typed_commit;
    lnp64_m12_state_projection_t typed_state_projection;

    lnp64_m12_storage_barrier dut (
        .clk(clk),
        .reset_n(reset_n),
        .start(start),
        .scenario_seed(scenario_seed),
        .done(done),
        .trace_valid(trace_valid),
        .trace_code(trace_code),
        .trace_value(trace_value),
        .boot_image_visible(boot_image_visible),
        .block_object_authorized(block_object_authorized),
        .block_write_completed(block_write_completed),
        .storage_barrier_issued(storage_barrier_issued),
        .storage_barrier_quiescent(storage_barrier_quiescent),
        .stale_object_rejected(stale_object_rejected),
        .cross_domain_rejected(cross_domain_rejected),
        .media_fault_terminal(media_fault_terminal),
        .no_raw_device_authority(no_raw_device_authority),
        .counts_exact(counts_exact),
        .typed_commit_valid(typed_commit_valid),
        .typed_commit(typed_commit),
        .typed_state_projection(typed_state_projection)
    );

    // Power-on reset discipline (reset_n asserted on the first cycle).
    reg init_done = 1'b0;
    always @(posedge clk) init_done <= 1'b1;
    always @(posedge clk) begin
        if (!init_done) assume (!reset_n);
    end

    always @(posedge clk) begin
        if (reset_n) begin
            // SG-MEM: raw block-device authority is NEVER exposed.
            a_no_raw_device_authority:
                assert (no_raw_device_authority);

            if (typed_commit_valid) begin
                a_projection_no_raw:
                    assert (typed_state_projection.no_raw_device_authority);

                // A stale object submit is rejected as revoked (EREVOKED).
                if (typed_commit.op == LNP64_M12_COMMIT_STALE_OBJECT)
                    a_stale_object_revoked:
                        assert (typed_commit.status == LNP64_ERR_EREVOKED
                                && stale_object_rejected);

                // A cross-domain submit is denied (EPERM).
                if (typed_commit.op == LNP64_M12_COMMIT_CROSS_DOMAIN)
                    a_cross_domain_denied:
                        assert (typed_commit.status == LNP64_ERR_EPERM
                                && cross_domain_rejected);

                // A media fault is terminal (EIO).
                if (typed_commit.op == LNP64_M12_COMMIT_MEDIA_FAULT)
                    a_media_fault_terminal:
                        assert (typed_commit.status == LNP64_ERR_EIO
                                && media_fault_terminal);

                // A block write only commits against an authorized object.
                if (typed_commit.op == LNP64_M12_COMMIT_BLOCK_WRITE)
                    a_block_write_authorized:
                        assert (block_object_authorized && block_write_completed);
            end
        end
    end
endmodule

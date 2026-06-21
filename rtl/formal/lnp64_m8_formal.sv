`timescale 1ns/1ps

// Formal property-verification harness for the M8 heap engine.
// Asserts the SG-MEM heap-safety severe goals as SVA on the real lnp64_m8_heap
// output ports, model-checked over all seeds and timings.
import lnp64_pkg::*;

module lnp64_m8_formal (
    input logic clk,
    input logic reset_n,
    input logic start,
    input logic [31:0] scenario_seed
);
    logic done;
    logic trace_valid;
    logic [7:0] trace_code;
    logic [63:0] trace_value;
    logic alloc_completed;
    logic alloc_size_reported;
    logic free_completed;
    logic reuse_completed;
    logic double_free_rejected;
    logic stale_pointer_rejected;
    logic cross_thread_handoff;
    logic guard_faulted;
    logic quarantine_observed;
    logic heap_count_exact;
    logic typed_commit_valid;
    lnp64_m8_heap_commit_t typed_commit;
    lnp64_m8_state_projection_t typed_state_projection;

    lnp64_m8_heap dut (
        .clk(clk),
        .reset_n(reset_n),
        .start(start),
        .scenario_seed(scenario_seed),
        .done(done),
        .trace_valid(trace_valid),
        .trace_code(trace_code),
        .trace_value(trace_value),
        .alloc_completed(alloc_completed),
        .alloc_size_reported(alloc_size_reported),
        .free_completed(free_completed),
        .reuse_completed(reuse_completed),
        .double_free_rejected(double_free_rejected),
        .stale_pointer_rejected(stale_pointer_rejected),
        .cross_thread_handoff(cross_thread_handoff),
        .guard_faulted(guard_faulted),
        .quarantine_observed(quarantine_observed),
        .heap_count_exact(heap_count_exact),
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
            // SG-MEM: a double free is rejected (EINVAL).
            if (typed_commit.op == LNP64_M8_COMMIT_DOUBLE_FREE)
                a_double_free_rejected:
                    assert (typed_commit.status == LNP64_ERR_EINVAL
                            && double_free_rejected);

            // SG-MEM (revocation): a stale pointer free is rejected (EREVOKED).
            if (typed_commit.op == LNP64_M8_COMMIT_STALE_FREE)
                a_stale_pointer_rejected:
                    assert (typed_commit.status == LNP64_ERR_EREVOKED
                            && stale_pointer_rejected);

            // SG-MEM: a guard-page access faults (EFAULT) and is quarantined.
            if (typed_commit.op == LNP64_M8_COMMIT_GUARD_FAULT)
                a_guard_fault_quarantined:
                    assert (typed_commit.status == LNP64_ERR_EFAULT
                            && guard_faulted && quarantine_observed);
        end
    end
endmodule

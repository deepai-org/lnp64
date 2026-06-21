`timescale 1ns/1ps

// Formal property-verification harness for the M15 object-profiles engine.
// Asserts the SG-OBJECT severe goals as SVA on the real lnp64_m15_object_profiles
// output ports, model-checked over all seeds and timings.
import lnp64_pkg::*;

module lnp64_m15_formal (
    input logic clk,
    input logic reset_n,
    input logic start,
    input logic [31:0] scenario_seed
);
    logic done;
    logic trace_valid;
    logic [7:0] trace_code;
    logic [63:0] trace_value;
    logic counter_threshold_event;
    logic queue_rights_valid;
    logic queue_overflow_explicit;
    logic event_source_generation_safe;
    logic gate_continuation_unique;
    logic counts_exact;
    logic typed_commit_valid;
    lnp64_m15_object_commit_t typed_commit;
    lnp64_m15_state_projection_t typed_state_projection;

    lnp64_m15_object_profiles dut (
        .clk(clk),
        .reset_n(reset_n),
        .start(start),
        .scenario_seed(scenario_seed),
        .done(done),
        .trace_valid(trace_valid),
        .trace_code(trace_code),
        .trace_value(trace_value),
        .counter_threshold_event(counter_threshold_event),
        .queue_rights_valid(queue_rights_valid),
        .queue_overflow_explicit(queue_overflow_explicit),
        .event_source_generation_safe(event_source_generation_safe),
        .gate_continuation_unique(gate_continuation_unique),
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
            // SG-OBJECT (no lost events): a queue overflow is signalled
            // explicitly as EAGAIN, never silently dropped.
            if (typed_commit.op == LNP64_M15_COMMIT_QUEUE_OVERFLOW)
                a_overflow_explicit:
                    assert (typed_commit.status == LNP64_ERR_EAGAIN
                            && queue_overflow_explicit);

            // SG-OBJECT (revocation): a stale event source is rejected (EREVOKED).
            if (typed_commit.op == LNP64_M15_COMMIT_STALE_EVENT)
                a_stale_event_revoked:
                    assert (typed_commit.status == LNP64_ERR_EREVOKED
                            && event_source_generation_safe);

            // A replayed gate continuation is rejected (uniqueness, EREVOKED).
            if (typed_commit.op == LNP64_M15_COMMIT_GATE_PROFILE)
                a_gate_continuation_unique:
                    assert (typed_commit.status == LNP64_ERR_EREVOKED
                            && gate_continuation_unique);

            // The counter threshold raises an event.
            if (typed_commit.op == LNP64_M15_COMMIT_COUNTER)
                a_counter_threshold_event:
                    assert (counter_threshold_event);

            // A queue push only commits with valid rights.
            if (typed_commit.op == LNP64_M15_COMMIT_QUEUE_PUSH)
                a_queue_rights_valid:
                    assert (queue_rights_valid);
        end
    end
endmodule

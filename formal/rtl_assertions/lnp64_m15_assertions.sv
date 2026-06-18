`timescale 1ns/1ps

module lnp64_m15_assertions (
    input logic clk,
    input logic reset_n,
    input logic done,
    input logic counter_threshold_event,
    input logic queue_rights_valid,
    input logic queue_overflow_explicit,
    input logic event_source_generation_safe,
    input logic gate_continuation_unique,
    input logic counts_exact
);
    always_ff @(posedge clk) begin
        if (reset_n && done) begin
            assert (counter_threshold_event)
                else $fatal(1, "M15 counter threshold did not emit an event");
            assert (queue_rights_valid)
                else $fatal(1, "M15 queue push did not require rights");
            assert (queue_overflow_explicit)
                else $fatal(1, "M15 queue overflow was not explicit");
            assert (event_source_generation_safe)
                else $fatal(1, "M15 stale event source was not rejected");
            assert (gate_continuation_unique)
                else $fatal(1, "M15 gate continuation was not unique");
            assert (counts_exact)
                else $fatal(1, "M15 failure/event counts were not exact");
        end
    end
endmodule

`timescale 1ns/1ps

module lnp64_m8_assertions (
    input logic clk,
    input logic reset_n,
    input logic done,
    input logic alloc_completed,
    input logic alloc_size_reported,
    input logic free_completed,
    input logic reuse_completed,
    input logic double_free_rejected,
    input logic stale_pointer_rejected,
    input logic cross_thread_handoff,
    input logic guard_faulted,
    input logic quarantine_observed,
    input logic heap_count_exact
);
    always_ff @(posedge clk) begin
        if (reset_n && done) begin
            assert (alloc_completed)
                else $fatal(1, "M8 allocation did not complete");
            assert (alloc_size_reported)
                else $fatal(1, "M8 allocation size was not reported");
            assert (free_completed)
                else $fatal(1, "M8 free did not complete");
            assert (reuse_completed)
                else $fatal(1, "M8 reuse did not complete");
            assert (double_free_rejected)
                else $fatal(1, "M8 double-free was not rejected");
            assert (stale_pointer_rejected)
                else $fatal(1, "M8 stale pointer was not rejected");
            assert (cross_thread_handoff)
                else $fatal(1, "M8 cross-thread free handoff did not occur");
            assert (guard_faulted)
                else $fatal(1, "M8 guard fault was not observed");
            assert (quarantine_observed)
                else $fatal(1, "M8 quarantine was not observed");
            assert (heap_count_exact)
                else $fatal(1, "M8 heap counts were not exact");
        end
    end
endmodule

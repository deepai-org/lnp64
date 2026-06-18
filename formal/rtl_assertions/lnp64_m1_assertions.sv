`timescale 1ns/1ps

module lnp64_m1_assertions (
    input logic clk,
    input logic reset_n,
    input logic done,
    input logic no_forged_fdr,
    input logic no_lost_wakeup,
    input logic exactly_one_scheduler_location,
    input logic stale_generation_rejected,
    input logic queue_full_explicit
);
    always_ff @(posedge clk) begin
        if (reset_n) begin
            assert (exactly_one_scheduler_location || !done)
                else $fatal(1, "M1 scheduler location invariant failed");
            if (done) begin
                assert (no_forged_fdr)
                    else $fatal(1, "M1 allowed forged FDR authority");
                assert (no_lost_wakeup)
                    else $fatal(1, "M1 lost queue wakeup");
                assert (stale_generation_rejected)
                    else $fatal(1, "M1 stale generation was not rejected");
                assert (queue_full_explicit)
                    else $fatal(1, "M1 queue full behavior was not explicit");
            end
        end
    end
endmodule

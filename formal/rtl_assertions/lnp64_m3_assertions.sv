`timescale 1ns/1ps

module lnp64_m3_assertions (
    input logic clk,
    input logic reset_n,
    input logic done,
    input logic clone_created,
    input logic child_exit_signaled,
    input logic parent_join_completed,
    input logic exec_barrier_stopped_sibling,
    input logic stale_join_rejected,
    input logic exec_cancel_terminal,
    input logic exactly_one_thread_location
);
    always_ff @(posedge clk) begin
        if (reset_n) begin
            assert (exactly_one_thread_location || !done)
                else $fatal(1, "M3 thread location invariant failed");
            if (done) begin
                assert (clone_created)
                    else $fatal(1, "M3 clone was not created");
                assert (child_exit_signaled)
                    else $fatal(1, "M3 child exit waitable was not signaled");
                assert (parent_join_completed)
                    else $fatal(1, "M3 parent join did not complete");
                assert (exec_barrier_stopped_sibling)
                    else $fatal(1, "M3 exec barrier did not stop sibling work");
                assert (stale_join_rejected)
                    else $fatal(1, "M3 stale join was not rejected");
                assert (exec_cancel_terminal)
                    else $fatal(1, "M3 exec cancellation was not terminal");
            end
        end
    end
endmodule

`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m3_tb;
    logic clk;
    logic reset_n;
    logic start;
    logic [31:0] scenario_seed;
    logic done;
    logic trace_valid;
    logic [7:0] trace_code;
    logic [63:0] trace_value;
    logic clone_created;
    logic child_exit_signaled;
    logic parent_join_completed;
    logic exec_barrier_stopped_sibling;
    logic stale_join_rejected;
    logic exec_cancel_terminal;
    logic exactly_one_thread_location;

    lnp64_m3_process dut(
        .clk(clk),
        .reset_n(reset_n),
        .start(start),
        .scenario_seed(scenario_seed),
        .done(done),
        .trace_valid(trace_valid),
        .trace_code(trace_code),
        .trace_value(trace_value),
        .clone_created(clone_created),
        .child_exit_signaled(child_exit_signaled),
        .parent_join_completed(parent_join_completed),
        .exec_barrier_stopped_sibling(exec_barrier_stopped_sibling),
        .stale_join_rejected(stale_join_rejected),
        .exec_cancel_terminal(exec_cancel_terminal),
        .exactly_one_thread_location(exactly_one_thread_location)
    );

    lnp64_m3_assertions assertions_i(
        .clk(clk),
        .reset_n(reset_n),
        .done(done),
        .clone_created(clone_created),
        .child_exit_signaled(child_exit_signaled),
        .parent_join_completed(parent_join_completed),
        .exec_barrier_stopped_sibling(exec_barrier_stopped_sibling),
        .stale_join_rejected(stale_join_rejected),
        .exec_cancel_terminal(exec_cancel_terminal),
        .exactly_one_thread_location(exactly_one_thread_location)
    );

    always #5 clk = ~clk;

    task automatic require(input logic condition, input string message);
        if (!condition) begin
            $fatal(1, "%s", message);
        end
    endtask

    always_ff @(posedge clk) begin
        if (trace_valid) begin
            unique case (trace_code)
                8'd1: $display(
                    "TRACE boot parent=%0d child_slot=free exec_epoch=%0d",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd2: $display(
                    "TRACE clone parent=%0d child=%0d state=runnable",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd3: $display(
                    "TRACE exit child=%0d code=%0d waitable=signaled",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd4: $display(
                    "TRACE join parent=%0d child=%0d code=%0d",
                    trace_value[63:48],
                    trace_value[47:32],
                    trace_value[31:0]
                );
                8'd5: $display(
                    "TRACE exec_barrier epoch=%0d siblings_stopped=%0d",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd6: $display("TRACE stale_join errno=%0d", trace_value[15:0]);
                8'd7: $display("TRACE exec_cancel errno=%0d", trace_value[15:0]);
                8'd8: $display(
                    "TRACE done live_threads=%0d exec_epoch=%0d",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                default: $display("TRACE unknown code=%0d value=%0d", trace_code, trace_value);
            endcase
        end
    end

    initial begin
        if (!$value$plusargs("seed=%d", scenario_seed)) begin
            scenario_seed = 32'd0;
        end
        clk = 1'b0;
        reset_n = 1'b0;
        start = 1'b0;

        repeat (4) @(posedge clk);
        reset_n = 1'b1;
        @(posedge clk);
        start = 1'b1;
        @(posedge clk);
        start = 1'b0;

        repeat (32) @(posedge clk);
        require(done, "M3 process slice did not complete");
        require(clone_created, "M3 clone did not create a child");
        require(child_exit_signaled, "M3 child exit did not signal waitable");
        require(parent_join_completed, "M3 parent join did not complete");
        require(exec_barrier_stopped_sibling, "M3 exec barrier did not stop sibling work");
        require(stale_join_rejected, "M3 stale join was not rejected");
        require(exec_cancel_terminal, "M3 exec cancellation did not reach a terminal path");
        require(exactly_one_thread_location, "M3 exactly-one thread location invariant did not hold");
        $display("LNP64-RTL-M3 PASS");
        $finish;
    end
endmodule

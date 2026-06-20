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
    logic typed_commit_valid;
    lnp64_m3_process_commit_t typed_commit;
    lnp64_m3_state_projection_t typed_state_projection;

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
        .exactly_one_thread_location(exactly_one_thread_location),
        .typed_commit_valid(typed_commit_valid),
        .typed_commit(typed_commit),
        .typed_state_projection(typed_state_projection)
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
        if (typed_commit_valid) begin
            $display(
                "TTRACE_M3 {\"record\":\"m3_process_commit\",\"op\":%0d,\"status\":%0d,\"parent_tid\":%0d,\"child_tid\":%0d,\"child_generation\":%0d,\"join_generation\":%0d,\"exec_epoch\":%0d,\"exit_code\":%0d}",
                typed_commit.op,
                typed_commit.status,
                typed_commit.parent_tid,
                typed_commit.child_tid,
                typed_commit.child_generation,
                typed_commit.join_generation,
                typed_commit.exec_epoch,
                typed_commit.exit_code
            );
            $display(
                "TTRACE_M3_BITS {\"record\":\"m3_process_commit_bits\",\"width\":%0d,\"bits\":\"%0h\"}",
                $bits(lnp64_m3_process_commit_t),
                typed_commit
            );
            $display(
                "TTRACE_M3_STATE {\"record\":\"m3_state_projection\",\"op\":%0d,\"status\":%0d,\"parent_state\":%0d,\"child_state\":%0d,\"parent_tid\":%0d,\"child_tid\":%0d,\"child_generation\":%0d,\"join_generation\":%0d,\"exec_epoch\":%0d,\"clone_created\":%0d,\"child_exit_signaled\":%0d,\"parent_join_completed\":%0d,\"exec_barrier_stopped_sibling\":%0d,\"stale_join_rejected\":%0d,\"exec_cancel_terminal\":%0d,\"exactly_one_thread_location\":%0d}",
                typed_state_projection.op,
                typed_state_projection.status,
                typed_state_projection.parent_state,
                typed_state_projection.child_state,
                typed_state_projection.parent_tid,
                typed_state_projection.child_tid,
                typed_state_projection.child_generation,
                typed_state_projection.join_generation,
                typed_state_projection.exec_epoch,
                typed_state_projection.clone_created,
                typed_state_projection.child_exit_signaled,
                typed_state_projection.parent_join_completed,
                typed_state_projection.exec_barrier_stopped_sibling,
                typed_state_projection.stale_join_rejected,
                typed_state_projection.exec_cancel_terminal,
                typed_state_projection.exactly_one_thread_location
            );
            $display(
                "TTRACE_M3_STATE_BITS {\"record\":\"m3_state_projection_bits\",\"width\":%0d,\"bits\":\"%0h\"}",
                $bits(lnp64_m3_state_projection_t),
                typed_state_projection
            );
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

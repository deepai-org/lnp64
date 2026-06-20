`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m8_tb;
    logic clk;
    logic reset_n;
    logic start;
    logic [31:0] scenario_seed;
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

    lnp64_m8_heap dut(
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

    lnp64_m8_assertions assertions_i(
        .clk(clk),
        .reset_n(reset_n),
        .done(done),
        .alloc_completed(alloc_completed),
        .alloc_size_reported(alloc_size_reported),
        .free_completed(free_completed),
        .reuse_completed(reuse_completed),
        .double_free_rejected(double_free_rejected),
        .stale_pointer_rejected(stale_pointer_rejected),
        .cross_thread_handoff(cross_thread_handoff),
        .guard_faulted(guard_faulted),
        .quarantine_observed(quarantine_observed),
        .heap_count_exact(heap_count_exact)
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
                    "TRACE boot root_domain=%0d heap_gen=%0d",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd2: $display(
                    "TRACE alloc tid=%0d ptr=%0d size=%0d class=%0d",
                    trace_value[63:48],
                    trace_value[31:0],
                    trace_value[47:32],
                    trace_value[47:32]
                );
                8'd3: $display(
                    "TRACE alloc_size ptr=%0d size=%0d",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd4: $display(
                    "TRACE free tid=%0d ptr=%0d quarantine=%0d",
                    trace_value[15:0],
                    trace_value[63:32],
                    trace_value[31:16]
                );
                8'd5: $display(
                    "TRACE reuse tid=%0d ptr=%0d generation=%0d",
                    trace_value[63:48],
                    trace_value[31:0],
                    trace_value[47:32]
                );
                8'd6: $display("TRACE double_free errno=%0d", trace_value[15:0]);
                8'd7: $display("TRACE stale_free errno=%0d", trace_value[15:0]);
                8'd8: $display(
                    "TRACE cross_thread_free owner=%0d freer=%0d handoff=%0d",
                    trace_value[63:48],
                    trace_value[47:32],
                    trace_value[31:0]
                );
                8'd9: $display("TRACE guard_fault errno=%0d", trace_value[15:0]);
                8'd10: $display(
                    "TRACE done allocs=%0d frees=%0d",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                default: $display("TRACE unknown code=%0d value=%0d", trace_code, trace_value);
            endcase
        end
        if (typed_commit_valid) begin
            $display(
                "TTRACE_M8 {\"record\":\"m8_heap_commit\",\"op\":%0d,\"status\":%0d,\"owner_tid\":%0d,\"pointer_generation\":%0d,\"heap_generation\":%0d,\"size_class\":%0d,\"heap_ptr\":%0d}",
                typed_commit.op,
                typed_commit.status,
                typed_commit.owner_tid,
                typed_commit.pointer_generation,
                typed_commit.heap_generation,
                typed_commit.size_class,
                typed_commit.heap_ptr
            );
            $display(
                "TTRACE_M8_BITS {\"record\":\"m8_heap_commit_bits\",\"width\":%0d,\"bits\":\"%0h\"}",
                $bits(lnp64_m8_heap_commit_t),
                typed_commit
            );
            $display(
                "TTRACE_M8_STATE {\"record\":\"m8_state_projection\",\"op\":%0d,\"status\":%0d,\"pointer_generation\":%0d,\"owner_tid\":%0d,\"allocations\":%0d,\"frees\":%0d,\"allocated\":%0d,\"quarantined\":%0d,\"alloc_completed\":%0d,\"alloc_size_reported\":%0d,\"free_completed\":%0d,\"reuse_completed\":%0d,\"double_free_rejected\":%0d,\"stale_pointer_rejected\":%0d,\"cross_thread_handoff\":%0d,\"guard_faulted\":%0d,\"quarantine_observed\":%0d,\"heap_count_exact\":%0d}",
                typed_state_projection.op,
                typed_state_projection.status,
                typed_state_projection.pointer_generation,
                typed_state_projection.owner_tid,
                typed_state_projection.allocations,
                typed_state_projection.frees,
                typed_state_projection.allocated,
                typed_state_projection.quarantined,
                typed_state_projection.alloc_completed,
                typed_state_projection.alloc_size_reported,
                typed_state_projection.free_completed,
                typed_state_projection.reuse_completed,
                typed_state_projection.double_free_rejected,
                typed_state_projection.stale_pointer_rejected,
                typed_state_projection.cross_thread_handoff,
                typed_state_projection.guard_faulted,
                typed_state_projection.quarantine_observed,
                typed_state_projection.heap_count_exact
            );
            $display(
                "TTRACE_M8_STATE_BITS {\"record\":\"m8_state_projection_bits\",\"width\":%0d,\"bits\":\"%0h\"}",
                $bits(lnp64_m8_state_projection_t),
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

        repeat (40) @(posedge clk);
        require(done, "M8 heap slice did not complete");
        require(alloc_completed, "M8 allocation did not complete");
        require(alloc_size_reported, "M8 allocation size was not reported");
        require(free_completed, "M8 free did not complete");
        require(reuse_completed, "M8 reuse did not complete");
        require(double_free_rejected, "M8 double-free was not rejected");
        require(stale_pointer_rejected, "M8 stale pointer was not rejected");
        require(cross_thread_handoff, "M8 cross-thread free handoff did not occur");
        require(guard_faulted, "M8 guard fault was not observed");
        require(quarantine_observed, "M8 quarantine was not observed");
        require(heap_count_exact, "M8 heap counts were not exact");
        $display("LNP64-RTL-M8 PASS");
        $finish;
    end
endmodule

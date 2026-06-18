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
        .heap_count_exact(heap_count_exact)
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

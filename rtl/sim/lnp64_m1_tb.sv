`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m1_tb;
    logic clk;
    logic reset_n;
    logic start;
    logic done;
    logic trace_valid;
    logic [7:0] trace_code;
    logic [63:0] trace_value;
    logic no_forged_fdr;
    logic no_lost_wakeup;
    logic exactly_one_scheduler_location;
    logic stale_generation_rejected;
    logic queue_full_explicit;

    lnp64_m1_pingpong dut(
        .clk(clk),
        .reset_n(reset_n),
        .start(start),
        .done(done),
        .trace_valid(trace_valid),
        .trace_code(trace_code),
        .trace_value(trace_value),
        .no_forged_fdr(no_forged_fdr),
        .no_lost_wakeup(no_lost_wakeup),
        .exactly_one_scheduler_location(exactly_one_scheduler_location),
        .stale_generation_rejected(stale_generation_rejected),
        .queue_full_explicit(queue_full_explicit)
    );

    lnp64_m1_assertions assertions_i(
        .clk(clk),
        .reset_n(reset_n),
        .done(done),
        .no_forged_fdr(no_forged_fdr),
        .no_lost_wakeup(no_lost_wakeup),
        .exactly_one_scheduler_location(exactly_one_scheduler_location),
        .stale_generation_rejected(stale_generation_rejected),
        .queue_full_explicit(queue_full_explicit)
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
                8'd1: $display("TRACE boot root_domain=%0d queue_gen=%0d", 1, trace_value[31:0]);
                8'd2: $display("TRACE cap_dup dst=consumer rights=0x%016h", trace_value);
                8'd3: $display("TRACE await tid=2 queue=empty state=parked");
                8'd4: $display("TRACE push tid=1 value=%0d wake=2", trace_value);
                8'd5: $display("TRACE pull tid=2 value=%0d", trace_value);
                8'd6: $display("TRACE queue_refill value=%0d", trace_value);
                8'd7: $display("TRACE push_full errno=%0d", trace_value);
                8'd8: $display("TRACE stale_pull errno=%0d", trace_value);
                8'd9: $display("TRACE done events=%0d", trace_value[31:0]);
                default: $display("TRACE unknown code=%0d value=%0d", trace_code, trace_value);
            endcase
        end
    end

    initial begin
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
        require(done, "M1 ping-pong did not complete");
        require(no_forged_fdr, "M1 no-forged-FDR invariant did not hold");
        require(no_lost_wakeup, "M1 wakeup was lost");
        require(exactly_one_scheduler_location, "M1 exactly-one scheduler invariant did not hold");
        require(stale_generation_rejected, "M1 stale generation was not rejected");
        require(queue_full_explicit, "M1 queue full behavior was not explicit");
        $display("LNP64-RTL-M1 PASS");
        $finish;
    end
endmodule

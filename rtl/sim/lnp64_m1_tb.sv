`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m1_tb;
    logic clk;
    logic reset_n;
    logic start;
    logic deny_dup;
    logic [31:0] scenario_seed;
    logic done;
    logic trace_valid;
    logic [7:0] trace_code;
    logic [63:0] trace_value;
    logic no_forged_fdr;
    logic no_lost_wakeup;
    logic exactly_one_scheduler_location;
    logic stale_generation_rejected;
    logic queue_full_explicit;
    logic typed_commit_valid;
    lnp64_m1_cap_commit_t typed_commit;

    lnp64_m1_pingpong dut(
        .clk(clk),
        .reset_n(reset_n),
        .start(start),
        .deny_dup(deny_dup),
        .scenario_seed(scenario_seed),
        .done(done),
        .trace_valid(trace_valid),
        .trace_code(trace_code),
        .trace_value(trace_value),
        .no_forged_fdr(no_forged_fdr),
        .no_lost_wakeup(no_lost_wakeup),
        .exactly_one_scheduler_location(exactly_one_scheduler_location),
        .stale_generation_rejected(stale_generation_rejected),
        .queue_full_explicit(queue_full_explicit),
        .typed_commit_valid(typed_commit_valid),
        .typed_commit(typed_commit)
    );

    lnp64_m1_assertions assertions_i(
        .clk(clk),
        .reset_n(reset_n),
        .done(done),
        .no_forged_fdr(no_forged_fdr),
        .no_lost_wakeup(no_lost_wakeup),
        .exactly_one_scheduler_location(exactly_one_scheduler_location),
        .stale_generation_rejected(stale_generation_rejected),
        .queue_full_explicit(queue_full_explicit),
        .expect_denied(deny_dup),
        .typed_commit_valid(typed_commit_valid),
        .typed_commit(typed_commit)
    );

    always #5 clk = ~clk;

    task automatic require(input logic condition, input string message);
        if (!condition) begin
            $fatal(1, "%s", message);
        end
    endtask

    always_ff @(posedge clk) begin
        if (typed_commit_valid) begin
            $display(
                "TTRACE_M1 {\"record\":\"m1_cap_commit\",\"op\":%0d,\"object_id\":%0d,\"object_gen\":%0d,\"fdr_gen\":%0d,\"domain_id\":%0d,\"domain_gen\":%0d,\"rights_mask\":%0d,\"lineage_epoch\":%0d,\"sealed\":%0d,\"status\":%0d}",
                typed_commit.op,
                typed_commit.object_id,
                typed_commit.object_gen,
                typed_commit.fdr_gen,
                typed_commit.domain_id,
                typed_commit.domain_gen,
                typed_commit.rights_mask,
                typed_commit.lineage_epoch,
                typed_commit.sealed,
                typed_commit.status
            );
        end
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
        if (!$value$plusargs("seed=%d", scenario_seed)) begin
            scenario_seed = 32'd0;
        end
        deny_dup = $test$plusargs("deny_dup");
        clk = 1'b0;
        reset_n = 1'b0;
        start = 1'b0;

        repeat (4) @(posedge clk);
        reset_n = 1'b1;
        @(posedge clk);
        start = 1'b1;
        @(posedge clk);
        start = 1'b0;

        for (int unsigned cycle = 0; cycle < 20 && !done; cycle++) begin
            @(posedge clk);
        end
        require(done, "M1 ping-pong did not complete");
        require(no_forged_fdr, "M1 no-forged-FDR invariant did not hold");
        require(exactly_one_scheduler_location, "M1 exactly-one scheduler invariant did not hold");
        if (!deny_dup) begin
            require(no_lost_wakeup, "M1 wakeup was lost");
            require(stale_generation_rejected, "M1 stale generation was not rejected");
            require(queue_full_explicit, "M1 queue full behavior was not explicit");
        end
        $display("LNP64-RTL-M1 PASS");
        $finish;
    end
endmodule

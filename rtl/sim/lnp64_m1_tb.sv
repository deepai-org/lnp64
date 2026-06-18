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
    lnp64_m1_state_projection_t typed_state_projection;

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
        .typed_commit(typed_commit),
        .typed_state_projection(typed_state_projection)
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
        .typed_commit(typed_commit),
        .typed_state_projection(typed_state_projection),
        .rtl_state_projection(dut.state),
        .queue_generation(typed_state_projection.object_gen)
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
            $display(
                "TTRACE_M1_BITS {\"record\":\"m1_cap_commit_bits\",\"bits\":\"%0h\"}",
                typed_commit
            );
            $display(
                "TTRACE_M1_STATE {\"record\":\"m1_state_projection\",\"op\":%0d,\"status\":%0d,\"object_gen\":%0d,\"created_object_created\":%0d,\"created_object_gen\":%0d,\"root_object_id\":%0d,\"root_generation\":%0d,\"root_domain_id\":%0d,\"root_lineage_epoch\":%0d,\"root_sealed\":%0d,\"root_rights\":%0d,\"consumer_object_id\":%0d,\"consumer_generation\":%0d,\"consumer_domain_id\":%0d,\"consumer_lineage_epoch\":%0d,\"consumer_sealed\":%0d,\"consumer_rights\":%0d,\"sent_valid\":%0d,\"sent_object_id\":%0d,\"sent_generation\":%0d,\"sent_domain_id\":%0d,\"sent_lineage_epoch\":%0d,\"sent_sealed\":%0d,\"sent_rights\":%0d,\"minted_valid\":%0d,\"minted_object_id\":%0d,\"minted_generation\":%0d,\"minted_domain_id\":%0d,\"minted_lineage_epoch\":%0d,\"minted_sealed\":%0d,\"minted_rights\":%0d,\"wake_pending\":%0d,\"transfer_valid\":%0d,\"stale_rejected\":%0d,\"revoked_rejected\":%0d,\"failed_no_authority\":%0d,\"full_was_explicit\":%0d,\"has_revoked_generation\":%0d,\"revoked_generation\":%0d}",
                typed_state_projection.op,
                typed_state_projection.status,
                typed_state_projection.object_gen,
                typed_state_projection.created_object_created,
                typed_state_projection.created_object_gen,
                typed_state_projection.root_object_id,
                typed_state_projection.root_generation,
                typed_state_projection.root_domain_id,
                typed_state_projection.root_lineage_epoch,
                typed_state_projection.root_sealed,
                typed_state_projection.root_rights,
                typed_state_projection.consumer_object_id,
                typed_state_projection.consumer_generation,
                typed_state_projection.consumer_domain_id,
                typed_state_projection.consumer_lineage_epoch,
                typed_state_projection.consumer_sealed,
                typed_state_projection.consumer_rights,
                typed_state_projection.sent_valid,
                typed_state_projection.sent_object_id,
                typed_state_projection.sent_generation,
                typed_state_projection.sent_domain_id,
                typed_state_projection.sent_lineage_epoch,
                typed_state_projection.sent_sealed,
                typed_state_projection.sent_rights,
                typed_state_projection.minted_valid,
                typed_state_projection.minted_object_id,
                typed_state_projection.minted_generation,
                typed_state_projection.minted_domain_id,
                typed_state_projection.minted_lineage_epoch,
                typed_state_projection.minted_sealed,
                typed_state_projection.minted_rights,
                typed_state_projection.wake_pending,
                typed_state_projection.transfer_valid,
                typed_state_projection.stale_rejected,
                typed_state_projection.revoked_rejected,
                typed_state_projection.failed_no_authority,
                typed_state_projection.full_was_explicit,
                typed_state_projection.has_revoked_generation,
                typed_state_projection.revoked_generation
            );
            $display(
                "TTRACE_M1_STATE_BITS {\"record\":\"m1_state_projection_bits\",\"bits\":\"%0h\"}",
                typed_state_projection
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

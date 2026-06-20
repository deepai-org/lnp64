`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m11_tb;
    logic clk;
    logic reset_n;
    logic start;
    logic [31:0] scenario_seed;
    logic done;
    logic trace_valid;
    logic [7:0] trace_code;
    logic [63:0] trace_value;
    logic metadata_allocated;
    logic metadata_domain_bound;
    logic ddr_write_completed;
    logic ddr_read_completed;
    logic read_matches_write;
    logic stale_generation_rejected;
    logic cross_domain_rejected;
    logic ecc_scrubbed;
    logic barrier_quiescent;
    logic counts_exact;
    logic typed_commit_valid;
    lnp64_m11_ddr_commit_t typed_commit;
    lnp64_m11_state_projection_t typed_state_projection;

    lnp64_m11_ddr_metadata dut(
        .clk(clk),
        .reset_n(reset_n),
        .start(start),
        .scenario_seed(scenario_seed),
        .done(done),
        .trace_valid(trace_valid),
        .trace_code(trace_code),
        .trace_value(trace_value),
        .metadata_allocated(metadata_allocated),
        .metadata_domain_bound(metadata_domain_bound),
        .ddr_write_completed(ddr_write_completed),
        .ddr_read_completed(ddr_read_completed),
        .read_matches_write(read_matches_write),
        .stale_generation_rejected(stale_generation_rejected),
        .cross_domain_rejected(cross_domain_rejected),
        .ecc_scrubbed(ecc_scrubbed),
        .barrier_quiescent(barrier_quiescent),
        .counts_exact(counts_exact),
        .typed_commit_valid(typed_commit_valid),
        .typed_commit(typed_commit),
        .typed_state_projection(typed_state_projection)
    );

    lnp64_m11_assertions assertions_i(
        .clk(clk),
        .reset_n(reset_n),
        .done(done),
        .metadata_allocated(metadata_allocated),
        .metadata_domain_bound(metadata_domain_bound),
        .ddr_write_completed(ddr_write_completed),
        .ddr_read_completed(ddr_read_completed),
        .read_matches_write(read_matches_write),
        .stale_generation_rejected(stale_generation_rejected),
        .cross_domain_rejected(cross_domain_rejected),
        .ecc_scrubbed(ecc_scrubbed),
        .barrier_quiescent(barrier_quiescent),
        .counts_exact(counts_exact)
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
                    "TRACE boot root_domain=%0d ddr_window=%0d metadata_epoch=%0d",
                    trace_value[63:32],
                    trace_value[31:16],
                    trace_value[15:0]
                );
                8'd2: $display(
                    "TRACE metadata_alloc line=%0d gen=%0d domain=%0d epoch=%0d",
                    trace_value[63:48],
                    trace_value[47:32],
                    trace_value[31:16],
                    trace_value[15:0]
                );
                8'd3: $display(
                    "TRACE ddr_write line=%0d bytes=%0d data=%0d",
                    trace_value[63:48],
                    trace_value[47:32],
                    trace_value[31:0]
                );
                8'd4: $display(
                    "TRACE ddr_read line=%0d data=%0d visible=%0d",
                    trace_value[63:48],
                    trace_value[47:16],
                    trace_value[15:0]
                );
                8'd5: $display(
                    "TRACE stale_submit gen=%0d errno=%0d",
                    trace_value[47:32],
                    trace_value[15:0]
                );
                8'd6: $display(
                    "TRACE cross_domain domain=%0d errno=%0d",
                    trace_value[63:32],
                    trace_value[15:0]
                );
                8'd7: $display(
                    "TRACE ecc_scrub corrections=%0d errno=%0d",
                    trace_value[31:16],
                    trace_value[15:0]
                );
                8'd8: $display(
                    "TRACE barrier line=%0d quiescent=%0d",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd9: $display(
                    "TRACE done completions=%0d faults=%0d epoch=%0d",
                    trace_value[63:48],
                    trace_value[47:32],
                    trace_value[31:0]
                );
                default: $display("TRACE unknown code=%0d value=%0d", trace_code, trace_value);
            endcase
        end
        if (typed_commit_valid) begin
            $display(
                "TTRACE_M11 {\"record\":\"m11_ddr_commit\",\"op\":%0d,\"status\":%0d,\"line_id\":%0d,\"line_generation\":%0d,\"domain_id\":%0d,\"metadata_epoch\":%0d,\"byte_len\":%0d,\"data_value\":%0d}",
                typed_commit.op,
                typed_commit.status,
                typed_commit.line_id,
                typed_commit.line_generation,
                typed_commit.domain_id,
                typed_commit.metadata_epoch,
                typed_commit.byte_len,
                typed_commit.data_value
            );
            $display(
                "TTRACE_M11_BITS {\"record\":\"m11_ddr_commit_bits\",\"width\":%0d,\"bits\":\"%0h\"}",
                $bits(lnp64_m11_ddr_commit_t),
                typed_commit
            );
            $display(
                "TTRACE_M11_STATE {\"record\":\"m11_state_projection\",\"op\":%0d,\"status\":%0d,\"completions\":%0d,\"faults\":%0d,\"metadata_allocated\":%0d,\"metadata_domain_bound\":%0d,\"ddr_write_completed\":%0d,\"ddr_read_completed\":%0d,\"read_matches_write\":%0d,\"stale_generation_rejected\":%0d,\"cross_domain_rejected\":%0d,\"ecc_scrubbed\":%0d,\"barrier_quiescent\":%0d,\"counts_exact\":%0d}",
                typed_state_projection.op,
                typed_state_projection.status,
                typed_state_projection.completions,
                typed_state_projection.faults,
                typed_state_projection.metadata_allocated,
                typed_state_projection.metadata_domain_bound,
                typed_state_projection.ddr_write_completed,
                typed_state_projection.ddr_read_completed,
                typed_state_projection.read_matches_write,
                typed_state_projection.stale_generation_rejected,
                typed_state_projection.cross_domain_rejected,
                typed_state_projection.ecc_scrubbed,
                typed_state_projection.barrier_quiescent,
                typed_state_projection.counts_exact
            );
            $display(
                "TTRACE_M11_STATE_BITS {\"record\":\"m11_state_projection_bits\",\"width\":%0d,\"bits\":\"%0h\"}",
                $bits(lnp64_m11_state_projection_t),
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
        require(done, "M11 DDR/metadata slice did not complete");
        require(metadata_allocated, "M11 metadata allocation did not occur");
        require(metadata_domain_bound, "M11 metadata was not domain-bound");
        require(ddr_write_completed, "M11 DDR write did not complete");
        require(ddr_read_completed, "M11 DDR read did not complete");
        require(read_matches_write, "M11 DDR read did not match write");
        require(stale_generation_rejected, "M11 stale generation was not rejected");
        require(cross_domain_rejected, "M11 cross-domain access was not rejected");
        require(ecc_scrubbed, "M11 ECC scrub did not occur");
        require(barrier_quiescent, "M11 metadata barrier was not quiescent");
        require(counts_exact, "M11 counts were not exact");
        $display("LNP64-RTL-M11 PASS");
        $finish;
    end
endmodule

`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m5_tb;
    logic clk;
    logic reset_n;
    logic start;
    logic [31:0] scenario_seed;
    logic done;
    logic trace_valid;
    logic [7:0] trace_code;
    logic [63:0] trace_value;
    logic pin_completed;
    logic unpin_completed;
    logic copy_completed;
    logic fill_completed;
    logic permission_faulted;
    logic revoke_rejected;
    logic domain_isolation_enforced;
    logic coherence_observed;
    logic completions_exact;
    logic typed_commit_valid;
    lnp64_m5_dma_commit_t typed_commit;
    lnp64_m5_state_projection_t typed_state_projection;

    lnp64_m5_dma dut(
        .clk(clk),
        .reset_n(reset_n),
        .start(start),
        .scenario_seed(scenario_seed),
        .done(done),
        .trace_valid(trace_valid),
        .trace_code(trace_code),
        .trace_value(trace_value),
        .pin_completed(pin_completed),
        .unpin_completed(unpin_completed),
        .copy_completed(copy_completed),
        .fill_completed(fill_completed),
        .permission_faulted(permission_faulted),
        .revoke_rejected(revoke_rejected),
        .domain_isolation_enforced(domain_isolation_enforced),
        .coherence_observed(coherence_observed),
        .completions_exact(completions_exact),
        .typed_commit_valid(typed_commit_valid),
        .typed_commit(typed_commit),
        .typed_state_projection(typed_state_projection)
    );

    lnp64_m5_assertions assertions_i(
        .clk(clk),
        .reset_n(reset_n),
        .done(done),
        .pin_completed(pin_completed),
        .unpin_completed(unpin_completed),
        .copy_completed(copy_completed),
        .fill_completed(fill_completed),
        .permission_faulted(permission_faulted),
        .revoke_rejected(revoke_rejected),
        .domain_isolation_enforced(domain_isolation_enforced),
        .coherence_observed(coherence_observed),
        .completions_exact(completions_exact)
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
                    "TRACE boot root_domain=%0d dma_buffers=%0d",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd2: $display(
                    "TRACE dma_pin buffer=%0d pinned=%0d",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd3: $display(
                    "TRACE dma_copy src=%0d dst=%0d bytes=%0d completion=1",
                    trace_value[63:48],
                    trace_value[47:32],
                    trace_value[31:0]
                );
                8'd4: $display(
                    "TRACE dma_fill dst=%0d value=%0d bytes=%0d completion=2",
                    trace_value[63:48],
                    trace_value[47:32],
                    trace_value[31:0]
                );
                8'd5: $display(
                    "TRACE dma_unpin buffer=%0d pinned=%0d",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd6: $display("TRACE permission_fault errno=%0d op=write", trace_value[15:0]);
                8'd7: $display("TRACE revoked_submit errno=%0d", trace_value[15:0]);
                8'd8: $display("TRACE domain_isolation errno=%0d", trace_value[15:0]);
                8'd9: $display(
                    "TRACE coherence_flush buffer=%0d visible=%0d",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd10: $display("TRACE done completions=%0d", trace_value[31:0]);
                default: $display("TRACE unknown code=%0d value=%0d", trace_code, trace_value);
            endcase
        end
        if (typed_commit_valid) begin
            $display(
                "TTRACE_M5 {\"record\":\"m5_dma_commit\",\"op\":%0d,\"status\":%0d,\"src_buffer_id\":%0d,\"dst_buffer_id\":%0d,\"dst_generation\":%0d,\"requester_domain\":%0d,\"dst_domain\":%0d,\"dst_rights\":%0d}",
                typed_commit.op,
                typed_commit.status,
                typed_commit.src_buffer_id,
                typed_commit.dst_buffer_id,
                typed_commit.dst_generation,
                typed_commit.requester_domain,
                typed_commit.dst_domain,
                typed_commit.dst_rights
            );
            $display(
                "TTRACE_M5_BITS {\"record\":\"m5_dma_commit_bits\",\"width\":%0d,\"bits\":\"%0h\"}",
                $bits(lnp64_m5_dma_commit_t),
                typed_commit
            );
            $display(
                "TTRACE_M5_STATE {\"record\":\"m5_state_projection\",\"op\":%0d,\"status\":%0d,\"dst_buffer_id\":%0d,\"dst_generation\":%0d,\"requester_domain\":%0d,\"dst_domain\":%0d,\"dst_rights\":%0d,\"dst_pinned\":%0d,\"completions\":%0d,\"dst_visible\":%0d,\"pin_completed\":%0d,\"unpin_completed\":%0d,\"copy_completed\":%0d,\"fill_completed\":%0d,\"permission_faulted\":%0d,\"revoke_rejected\":%0d,\"domain_isolation_enforced\":%0d,\"coherence_observed\":%0d,\"completions_exact\":%0d}",
                typed_state_projection.op,
                typed_state_projection.status,
                typed_state_projection.dst_buffer_id,
                typed_state_projection.dst_generation,
                typed_state_projection.requester_domain,
                typed_state_projection.dst_domain,
                typed_state_projection.dst_rights,
                typed_state_projection.dst_pinned,
                typed_state_projection.completions,
                typed_state_projection.dst_visible,
                typed_state_projection.pin_completed,
                typed_state_projection.unpin_completed,
                typed_state_projection.copy_completed,
                typed_state_projection.fill_completed,
                typed_state_projection.permission_faulted,
                typed_state_projection.revoke_rejected,
                typed_state_projection.domain_isolation_enforced,
                typed_state_projection.coherence_observed,
                typed_state_projection.completions_exact
            );
            $display(
                "TTRACE_M5_STATE_BITS {\"record\":\"m5_state_projection_bits\",\"width\":%0d,\"bits\":\"%0h\"}",
                $bits(lnp64_m5_state_projection_t),
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
        require(done, "M5 DMA slice did not complete");
        require(pin_completed, "M5 DMA buffer pin did not complete");
        require(copy_completed, "M5 copy did not complete");
        require(fill_completed, "M5 fill did not complete");
        require(unpin_completed, "M5 DMA buffer unpin did not complete");
        require(permission_faulted, "M5 permission fault did not occur");
        require(revoke_rejected, "M5 revoked submit was not rejected");
        require(domain_isolation_enforced, "M5 domain isolation was not enforced");
        require(coherence_observed, "M5 coherence visibility was not observed");
        require(completions_exact, "M5 completion count was not exact");
        $display("LNP64-RTL-M5 PASS");
        $finish;
    end
endmodule

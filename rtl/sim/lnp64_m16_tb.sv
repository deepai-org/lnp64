`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m16_tb;
    logic clk;
    logic reset_n;
    logic start;
    logic [31:0] scenario_seed;
    logic done;
    logic trace_valid;
    logic [7:0] trace_code;
    logic [63:0] trace_value;
    logic bounded_depth_le_capacity;
    logic drain_bounded_by_capacity;
    logic full_fails_closed;
    logic empty_fails_closed;
    logic oversize_fails_closed;
    logic no_block_except_wait;
    logic caps_resolve_sender_only;
    logic caps_reject_out_of_range;
    logic install_no_amplify;
    logic framing_one_send_one_recv;
    logic notify_raises_register_edge;
    logic counts_exact;
    logic typed_commit_valid;
    lnp64_m16_endpoint_commit_t typed_commit;
    lnp64_m16_state_projection_t typed_state_projection;

    lnp64_m16_endpoint dut(
        .clk(clk),
        .reset_n(reset_n),
        .start(start),
        .scenario_seed(scenario_seed),
        .done(done),
        .trace_valid(trace_valid),
        .trace_code(trace_code),
        .trace_value(trace_value),
        .bounded_depth_le_capacity(bounded_depth_le_capacity),
        .drain_bounded_by_capacity(drain_bounded_by_capacity),
        .full_fails_closed(full_fails_closed),
        .empty_fails_closed(empty_fails_closed),
        .oversize_fails_closed(oversize_fails_closed),
        .no_block_except_wait(no_block_except_wait),
        .caps_resolve_sender_only(caps_resolve_sender_only),
        .caps_reject_out_of_range(caps_reject_out_of_range),
        .install_no_amplify(install_no_amplify),
        .framing_one_send_one_recv(framing_one_send_one_recv),
        .notify_raises_register_edge(notify_raises_register_edge),
        .counts_exact(counts_exact),
        .typed_commit_valid(typed_commit_valid),
        .typed_commit(typed_commit),
        .typed_state_projection(typed_state_projection)
    );

    lnp64_m16_assertions assertions_i(
        .clk(clk),
        .reset_n(reset_n),
        .done(done),
        .bounded_depth_le_capacity(bounded_depth_le_capacity),
        .drain_bounded_by_capacity(drain_bounded_by_capacity),
        .full_fails_closed(full_fails_closed),
        .empty_fails_closed(empty_fails_closed),
        .oversize_fails_closed(oversize_fails_closed),
        .no_block_except_wait(no_block_except_wait),
        .caps_resolve_sender_only(caps_resolve_sender_only),
        .caps_reject_out_of_range(caps_reject_out_of_range),
        .install_no_amplify(install_no_amplify),
        .framing_one_send_one_recv(framing_one_send_one_recv),
        .notify_raises_register_edge(notify_raises_register_edge),
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
                8'd1: $display("TRACE create endpoint=%0d capacity=%0d backing=memory",
                    trace_value[63:32], trace_value[31:0]);
                8'd2: $display("TRACE send bytes=%0d depth=%0d", trace_value[63:32], trace_value[31:0]);
                8'd3: $display("TRACE recv bytes=%0d depth=%0d", trace_value[63:32], trace_value[31:0]);
                8'd4: $display("TRACE send_full capacity=%0d errno=%0d", trace_value[63:32], trace_value[15:0]);
                8'd5: $display("TRACE recv_empty errno=%0d", trace_value[15:0]);
                8'd6: $display("TRACE oversize bytes=%0d errno=%0d", trace_value[63:32], trace_value[15:0]);
                8'd7: $display("TRACE cap_send rights=0x%08h caps=%0d", trace_value[63:32], trace_value[31:0]);
                8'd8: $display("TRACE cap_reject handle=0x%08h errno=%0d", trace_value[63:32], trace_value[15:0]);
                8'd9: $display("TRACE notify register_edge=%0d", trace_value[63:32]);
                8'd10: $display("TRACE done failures=%0d events=%0d", trace_value[63:32], trace_value[31:0]);
                default: $display("TRACE unknown code=%0d value=%0d", trace_code, trace_value);
            endcase
        end
        if (typed_commit_valid) begin
            $display(
                "TTRACE_M16 {\"record\":\"m16_endpoint_commit\",\"op\":%0d,\"status\":%0d,\"endpoint_id\":%0d,\"endpoint_gen\":%0d,\"backing\":%0d,\"bytes_len\":%0d,\"caps_len\":%0d,\"depth\":%0d,\"capacity\":%0d,\"caps_resolved\":%0d,\"caps_installed\":%0d,\"sender_domain_id\":%0d,\"sender_domain_gen\":%0d,\"receiver_domain_id\":%0d,\"receiver_domain_gen\":%0d}",
                typed_commit.op,
                typed_commit.status,
                typed_commit.endpoint_id,
                typed_commit.endpoint_gen,
                typed_commit.backing,
                typed_commit.bytes_len,
                typed_commit.caps_len,
                typed_commit.depth,
                typed_commit.capacity,
                typed_commit.caps_resolved,
                typed_commit.caps_installed,
                typed_commit.sender_domain_id,
                typed_commit.sender_domain_gen,
                typed_commit.receiver_domain_id,
                typed_commit.receiver_domain_gen
            );
            $display(
                "TTRACE_M16_BITS {\"record\":\"m16_endpoint_commit_bits\",\"width\":%0d,\"bits\":\"%0h\"}",
                $bits(lnp64_m16_endpoint_commit_t),
                typed_commit
            );
            $display(
                "TTRACE_M16_STATE {\"record\":\"m16_state_projection\",\"op\":%0d,\"status\":%0d,\"depth\":%0d,\"capacity\":%0d,\"failures\":%0d,\"events\":%0d,\"bounded_depth_le_capacity\":%0d,\"drain_bounded_by_capacity\":%0d,\"full_fails_closed\":%0d,\"empty_fails_closed\":%0d,\"oversize_fails_closed\":%0d,\"no_block_except_wait\":%0d,\"caps_resolve_sender_only\":%0d,\"caps_reject_out_of_range\":%0d,\"install_no_amplify\":%0d,\"framing_one_send_one_recv\":%0d,\"notify_raises_register_edge\":%0d,\"counts_exact\":%0d}",
                typed_state_projection.op,
                typed_state_projection.status,
                typed_state_projection.depth,
                typed_state_projection.capacity,
                typed_state_projection.failures,
                typed_state_projection.events,
                typed_state_projection.bounded_depth_le_capacity,
                typed_state_projection.drain_bounded_by_capacity,
                typed_state_projection.full_fails_closed,
                typed_state_projection.empty_fails_closed,
                typed_state_projection.oversize_fails_closed,
                typed_state_projection.no_block_except_wait,
                typed_state_projection.caps_resolve_sender_only,
                typed_state_projection.caps_reject_out_of_range,
                typed_state_projection.install_no_amplify,
                typed_state_projection.framing_one_send_one_recv,
                typed_state_projection.notify_raises_register_edge,
                typed_state_projection.counts_exact
            );
            $display(
                "TTRACE_M16_STATE_BITS {\"record\":\"m16_state_projection_bits\",\"width\":%0d,\"bits\":\"%0h\"}",
                $bits(lnp64_m16_state_projection_t),
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
        require(done, "M16 endpoint slice did not complete");
        require(bounded_depth_le_capacity, "M16 depth exceeded capacity");
        require(drain_bounded_by_capacity, "M16 drain not bounded by capacity");
        require(full_fails_closed, "M16 full send did not fail closed");
        require(empty_fails_closed, "M16 empty recv did not fail closed");
        require(oversize_fails_closed, "M16 oversize did not fail closed");
        require(no_block_except_wait, "M16 a non-wait op blocked");
        require(caps_resolve_sender_only, "M16 cap did not resolve sender-only");
        require(caps_reject_out_of_range, "M16 out-of-range cap not rejected");
        require(install_no_amplify, "M16 cap install amplified rights");
        require(framing_one_send_one_recv, "M16 framing broke");
        require(notify_raises_register_edge, "M16 notify did not raise edge");
        require(counts_exact, "M16 counts were not exact");
        $display("LNP64-RTL-M16 PASS");
        $finish;
    end
endmodule

`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m2_tb;
    logic clk;
    logic reset_n;
    logic start;
    logic [31:0] scenario_seed;
    logic done;
    logic trace_valid;
    logic [7:0] trace_code;
    logic [63:0] trace_value;
    logic continuation_unique;
    logic sync_roundtrip_ok;
    logic async_delivery_ok;
    logic handoff_delivery_ok;
    logic stale_continuation_rejected;
    logic fault_delivery_gate_ok;
    logic signal_compatibility_ok;
    logic typed_commit_valid;
    lnp64_m2_gate_commit_t typed_commit;
    lnp64_m2_state_projection_t typed_state_projection;

    lnp64_m2_gate dut(
        .clk(clk),
        .reset_n(reset_n),
        .start(start),
        .scenario_seed(scenario_seed),
        .done(done),
        .trace_valid(trace_valid),
        .trace_code(trace_code),
        .trace_value(trace_value),
        .continuation_unique(continuation_unique),
        .sync_roundtrip_ok(sync_roundtrip_ok),
        .async_delivery_ok(async_delivery_ok),
        .handoff_delivery_ok(handoff_delivery_ok),
        .stale_continuation_rejected(stale_continuation_rejected),
        .fault_delivery_gate_ok(fault_delivery_gate_ok),
        .signal_compatibility_ok(signal_compatibility_ok),
        .typed_commit_valid(typed_commit_valid),
        .typed_commit(typed_commit),
        .typed_state_projection(typed_state_projection)
    );

    lnp64_m2_assertions assertions_i(
        .clk(clk),
        .reset_n(reset_n),
        .done(done),
        .continuation_unique(continuation_unique),
        .sync_roundtrip_ok(sync_roundtrip_ok),
        .async_delivery_ok(async_delivery_ok),
        .handoff_delivery_ok(handoff_delivery_ok),
        .stale_continuation_rejected(stale_continuation_rejected),
        .fault_delivery_gate_ok(fault_delivery_gate_ok),
        .signal_compatibility_ok(signal_compatibility_ok)
    );

    always #5 clk = ~clk;

    task automatic require(input logic condition, input string message);
        if (!condition) begin
            $fatal(1, "%s", message);
        end
    endtask

    function automatic string mode_name(input logic [15:0] mode);
        unique case (mode)
            16'd0: mode_name = "sync";
            16'd1: mode_name = "async";
            16'd2: mode_name = "handoff";
            default: mode_name = "unknown";
        endcase
    endfunction

    always_ff @(posedge clk) begin
        if (trace_valid) begin
            unique case (trace_code)
                8'd1: $display("TRACE boot root_domain=%0d gate_gen=%0d", 1, trace_value[31:0]);
                8'd2: $display(
                    "TRACE gate_call mode=%s target=%0d continuation=%0d",
                    mode_name(trace_value[15:0]),
                    trace_value[63:48],
                    trace_value[47:16]
                );
                8'd3: $display("TRACE gate_return continuation=%0d wake=1", trace_value[31:0]);
                8'd4: $display(
                    "TRACE gate_call mode=%s target=%0d completion=none",
                    mode_name(trace_value[15:0]),
                    trace_value[47:16]
                );
                8'd5: $display(
                    "TRACE gate_call mode=%s target=%0d transfer=running",
                    mode_name(trace_value[15:0]),
                    trace_value[47:16]
                );
                8'd6: $display("TRACE stale_return errno=%0d", trace_value[15:0]);
                8'd7: $display("TRACE fault_delivery errno=%0d target=fault_gate", trace_value[15:0]);
                8'd8: $display("TRACE signal_compat mask=honored authority=0");
                8'd9: $display("TRACE done delivered_faults=%0d", trace_value[31:0]);
                default: $display("TRACE unknown code=%0d value=%0d", trace_code, trace_value);
            endcase
        end
        if (typed_commit_valid) begin
            $display(
                "TTRACE_M2 {\"record\":\"m2_gate_commit\",\"op\":%0d,\"status\":%0d,\"continuation_id\":%0d,\"continuation_generation\":%0d,\"caller_tid\":%0d,\"callee_tid\":%0d,\"mode\":%0d}",
                typed_commit.op,
                typed_commit.status,
                typed_commit.continuation_id,
                typed_commit.continuation_generation,
                typed_commit.caller_tid,
                typed_commit.callee_tid,
                typed_commit.mode
            );
            $display(
                "TTRACE_M2_BITS {\"record\":\"m2_gate_commit_bits\",\"width\":%0d,\"bits\":\"%0h\"}",
                $bits(lnp64_m2_gate_commit_t),
                typed_commit
            );
            $display(
                "TTRACE_M2_STATE {\"record\":\"m2_state_projection\",\"op\":%0d,\"status\":%0d,\"caller_loc\":%0d,\"callee_loc\":%0d,\"continuation_valid\":%0d,\"continuation_id\":%0d,\"continuation_generation\":%0d,\"delivered_faults\":%0d,\"continuation_unique\":%0d,\"sync_roundtrip_ok\":%0d,\"async_delivery_ok\":%0d,\"handoff_delivery_ok\":%0d,\"stale_continuation_rejected\":%0d,\"fault_delivery_gate_ok\":%0d,\"signal_compatibility_ok\":%0d}",
                typed_state_projection.op,
                typed_state_projection.status,
                typed_state_projection.caller_loc,
                typed_state_projection.callee_loc,
                typed_state_projection.continuation_valid,
                typed_state_projection.continuation_id,
                typed_state_projection.continuation_generation,
                typed_state_projection.delivered_faults,
                typed_state_projection.continuation_unique,
                typed_state_projection.sync_roundtrip_ok,
                typed_state_projection.async_delivery_ok,
                typed_state_projection.handoff_delivery_ok,
                typed_state_projection.stale_continuation_rejected,
                typed_state_projection.fault_delivery_gate_ok,
                typed_state_projection.signal_compatibility_ok
            );
            $display(
                "TTRACE_M2_STATE_BITS {\"record\":\"m2_state_projection_bits\",\"width\":%0d,\"bits\":\"%0h\"}",
                $bits(lnp64_m2_state_projection_t),
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
        require(done, "M2 gate slice did not complete");
        require(continuation_unique, "M2 continuation uniqueness invariant did not hold");
        require(sync_roundtrip_ok, "M2 sync gate roundtrip did not complete");
        require(async_delivery_ok, "M2 async gate delivery did not complete");
        require(handoff_delivery_ok, "M2 handoff gate delivery did not complete");
        require(stale_continuation_rejected, "M2 stale continuation was not rejected");
        require(fault_delivery_gate_ok, "M2 fault delivery gate did not run");
        require(signal_compatibility_ok, "M2 signal compatibility path did not run");
        $display("LNP64-RTL-M2 PASS");
        $finish;
    end
endmodule

`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m6_tb;
    logic clk;
    logic reset_n;
    logic start;
    logic [31:0] scenario_seed;
    logic done;
    logic trace_valid;
    logic [7:0] trace_code;
    logic [63:0] trace_value;
    logic envelope_validated;
    logic namespace_dispatched;
    logic service_continuation_created;
    logic cap_return_installed;
    logic returned_cap_narrowed;
    logic cancel_terminal;
    logic stale_service_rejected;
    logic crash_completed;
    logic typed_commit_valid;
    lnp64_m6_service_commit_t typed_commit;
    lnp64_m6_state_projection_t typed_state_projection;

    lnp64_m6_service dut(
        .clk(clk),
        .reset_n(reset_n),
        .start(start),
        .scenario_seed(scenario_seed),
        .done(done),
        .trace_valid(trace_valid),
        .trace_code(trace_code),
        .trace_value(trace_value),
        .envelope_validated(envelope_validated),
        .namespace_dispatched(namespace_dispatched),
        .service_continuation_created(service_continuation_created),
        .cap_return_installed(cap_return_installed),
        .returned_cap_narrowed(returned_cap_narrowed),
        .cancel_terminal(cancel_terminal),
        .stale_service_rejected(stale_service_rejected),
        .crash_completed(crash_completed),
        .typed_commit_valid(typed_commit_valid),
        .typed_commit(typed_commit),
        .typed_state_projection(typed_state_projection)
    );

    lnp64_m6_assertions assertions_i(
        .clk(clk),
        .reset_n(reset_n),
        .done(done),
        .envelope_validated(envelope_validated),
        .namespace_dispatched(namespace_dispatched),
        .service_continuation_created(service_continuation_created),
        .cap_return_installed(cap_return_installed),
        .returned_cap_narrowed(returned_cap_narrowed),
        .cancel_terminal(cancel_terminal),
        .stale_service_rejected(stale_service_rejected),
        .crash_completed(crash_completed)
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
                    "TRACE boot root_domain=%0d namespace_root=%0d",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd2: $display(
                    "TRACE envelope op=open_at version=%0d profile=namespace valid=%0d",
                    trace_value[63:48],
                    trace_value[31:0]
                );
                8'd3: $display(
                    "TRACE ns_dispatch selector=%0d path_len=%0d service=%0d",
                    trace_value[63:48],
                    trace_value[47:32],
                    trace_value[31:0]
                );
                8'd4: $display(
                    "TRACE service_request op_id=%0d continuation=%0d state=pending",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd5: $display(
                    "TRACE cap_proposal object=%0d rights=0x%016h installed=1",
                    trace_value[63:32],
                    {32'd0, trace_value[31:0]}
                );
                8'd6: $display(
                    "TRACE service_cancel continuation=%0d errno=%0d",
                    trace_value[63:32],
                    trace_value[15:0]
                );
                8'd7: $display("TRACE stale_service errno=%0d", trace_value[15:0]);
                8'd8: $display("TRACE crash_completion errno=%0d", trace_value[15:0]);
                8'd9: $display(
                    "TRACE done installed_caps=%0d completions=%0d",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                default: $display("TRACE unknown code=%0d value=%0d", trace_code, trace_value);
            endcase
        end
        if (typed_commit_valid) begin
            $display(
                "TTRACE_M6 {\"record\":\"m6_service_commit\",\"op\":%0d,\"status\":%0d,\"service_id\":%0d,\"op_id\":%0d,\"continuation_generation\":%0d,\"service_generation\":%0d,\"requested_rights\":%0d,\"returned_rights\":%0d}",
                typed_commit.op,
                typed_commit.status,
                typed_commit.service_id,
                typed_commit.op_id,
                typed_commit.continuation_generation,
                typed_commit.service_generation,
                typed_commit.requested_rights,
                typed_commit.returned_rights
            );
            $display(
                "TTRACE_M6_BITS {\"record\":\"m6_service_commit_bits\",\"width\":%0d,\"bits\":\"%0h\"}",
                $bits(lnp64_m6_service_commit_t),
                typed_commit
            );
            $display(
                "TTRACE_M6_STATE {\"record\":\"m6_state_projection\",\"op\":%0d,\"status\":%0d,\"service_generation\":%0d,\"continuation_generation\":%0d,\"installed_caps\":%0d,\"completions\":%0d,\"envelope_validated\":%0d,\"namespace_dispatched\":%0d,\"service_continuation_created\":%0d,\"cap_return_installed\":%0d,\"returned_cap_narrowed\":%0d,\"cancel_terminal\":%0d,\"stale_service_rejected\":%0d,\"crash_completed\":%0d}",
                typed_state_projection.op,
                typed_state_projection.status,
                typed_state_projection.service_generation,
                typed_state_projection.continuation_generation,
                typed_state_projection.installed_caps,
                typed_state_projection.completions,
                typed_state_projection.envelope_validated,
                typed_state_projection.namespace_dispatched,
                typed_state_projection.service_continuation_created,
                typed_state_projection.cap_return_installed,
                typed_state_projection.returned_cap_narrowed,
                typed_state_projection.cancel_terminal,
                typed_state_projection.stale_service_rejected,
                typed_state_projection.crash_completed
            );
            $display(
                "TTRACE_M6_STATE_BITS {\"record\":\"m6_state_projection_bits\",\"width\":%0d,\"bits\":\"%0h\"}",
                $bits(lnp64_m6_state_projection_t),
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
        require(done, "M6 service slice did not complete");
        require(envelope_validated, "M6 envelope validation did not complete");
        require(namespace_dispatched, "M6 namespace dispatch did not complete");
        require(service_continuation_created, "M6 service continuation was not created");
        require(cap_return_installed, "M6 returned capability was not installed");
        require(returned_cap_narrowed, "M6 returned capability was not narrowed");
        require(cancel_terminal, "M6 cancellation was not terminal");
        require(stale_service_rejected, "M6 stale service was not rejected");
        require(crash_completed, "M6 service crash did not complete");
        $display("LNP64-RTL-M6 PASS");
        $finish;
    end
endmodule

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
        .crash_completed(crash_completed)
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

`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m9_tb;
    logic clk;
    logic reset_n;
    logic start;
    logic [31:0] scenario_seed;
    logic done;
    logic trace_valid;
    logic [7:0] trace_code;
    logic [63:0] trace_value;
    logic verifier_accepted;
    logic verifier_rejected;
    logic packet_steered;
    logic ipc_steered;
    logic action_emitted;
    logic budget_enforced;
    logic stale_attachment_rejected;
    logic no_authority_created;
    logic counts_exact;

    lnp64_m9_classifier_servicelet dut(
        .clk(clk),
        .reset_n(reset_n),
        .start(start),
        .scenario_seed(scenario_seed),
        .done(done),
        .trace_valid(trace_valid),
        .trace_code(trace_code),
        .trace_value(trace_value),
        .verifier_accepted(verifier_accepted),
        .verifier_rejected(verifier_rejected),
        .packet_steered(packet_steered),
        .ipc_steered(ipc_steered),
        .action_emitted(action_emitted),
        .budget_enforced(budget_enforced),
        .stale_attachment_rejected(stale_attachment_rejected),
        .no_authority_created(no_authority_created),
        .counts_exact(counts_exact)
    );

    lnp64_m9_assertions assertions_i(
        .clk(clk),
        .reset_n(reset_n),
        .done(done),
        .verifier_accepted(verifier_accepted),
        .verifier_rejected(verifier_rejected),
        .packet_steered(packet_steered),
        .ipc_steered(ipc_steered),
        .action_emitted(action_emitted),
        .budget_enforced(budget_enforced),
        .stale_attachment_rejected(stale_attachment_rejected),
        .no_authority_created(no_authority_created),
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
                    "TRACE boot root_domain=%0d classifier_table=%0d",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd2: $display(
                    "TRACE verifier program=%0d instructions=%0d accepted=%0d",
                    trace_value[63:32],
                    trace_value[31:16],
                    trace_value[15:0]
                );
                8'd3: $display("TRACE verifier_reject reason=blocking errno=%0d", trace_value[15:0]);
                8'd4: $display(
                    "TRACE packet_steer rule=%0d queue=%0d mark=%0d",
                    trace_value[63:48],
                    trace_value[47:32],
                    trace_value[31:0]
                );
                8'd5: $display(
                    "TRACE ipc_steer service=%0d gate=%0d",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd6: $display(
                    "TRACE action_emit kind=needs_software authorized=%0d",
                    trace_value[31:0]
                );
                8'd7: $display(
                    "TRACE budget_exhaust errno=%0d cycles=%0d",
                    trace_value[15:0],
                    trace_value[47:16]
                );
                8'd8: $display("TRACE stale_attachment errno=%0d", trace_value[15:0]);
                8'd9: $display(
                    "TRACE done packets=%0d ipc=%0d rejects=%0d",
                    trace_value[63:48],
                    trace_value[47:32],
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
        require(done, "M9 classifier/servicelet slice did not complete");
        require(verifier_accepted, "M9 verifier did not accept bounded servicelet");
        require(verifier_rejected, "M9 verifier did not reject invalid servicelet");
        require(packet_steered, "M9 packet steering did not occur");
        require(ipc_steered, "M9 IPC steering did not occur");
        require(action_emitted, "M9 action record was not emitted");
        require(budget_enforced, "M9 servicelet budget was not enforced");
        require(stale_attachment_rejected, "M9 stale attachment was not rejected");
        require(no_authority_created, "M9 action path created authority");
        require(counts_exact, "M9 counts were not exact");
        $display("LNP64-RTL-M9 PASS");
        $finish;
    end
endmodule

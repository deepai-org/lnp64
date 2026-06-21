`timescale 1ns/1ps

// "Does what it says" formal harness for the lnp64_policy_engine shell.
// Contract: the policy engine never grants (decision_allow) unless a request was
// presented the previous cycle -- no spurious authorization. request_valid is a
// free input, so this holds for every request stream on the real RTL.
import lnp64_pkg::*;

module lnp64_policy_engine_formal (
    input logic clk,
    input logic reset_n,
    input logic request_valid
);
    logic decision_allow;

    lnp64_policy_engine dut (
        .clk(clk),
        .reset_n(reset_n),
        .request_valid(request_valid),
        .decision_allow(decision_allow)
    );

    reg init_done = 1'b0;
    always @(posedge clk) init_done <= 1'b1;
    always @(posedge clk) begin
        if (!init_done) assume (!reset_n);
    end

    reg prev_request_valid = 1'b0;
    always @(posedge clk) prev_request_valid <= request_valid;

    always @(posedge clk) begin
        if (reset_n) begin
            // No grant without a request the previous cycle.
            a_no_spurious_grant:
                assert (!decision_allow || prev_request_valid);
        end
    end
endmodule

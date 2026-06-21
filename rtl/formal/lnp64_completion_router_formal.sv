`timescale 1ns/1ps

// "Does what it says" formal harness for the lnp64_completion_router shell.
// Contract: a completion is emitted exactly one cycle after a response, and it
// faithfully forwards that response's status, errno and result value (the router
// neither fabricates nor alters completion outcomes). rsp/rsp_valid are free.
import lnp64_pkg::*;

module lnp64_completion_router_formal (
    input logic clk,
    input logic reset_n,
    input logic rsp_valid,
    input lnp64_rsp_t rsp
);
    lnp64_completion_t completion;
    logic completion_valid;

    lnp64_completion_router dut (
        .clk(clk),
        .reset_n(reset_n),
        .rsp_valid(rsp_valid),
        .rsp(rsp),
        .completion(completion),
        .completion_valid(completion_valid)
    );

    reg init_done = 1'b0;
    always @(posedge clk) init_done <= 1'b1;
    always @(posedge clk) begin
        if (!init_done) assume (!reset_n);
    end

    // One-cycle history of the response, reset-aware to match the DUT's own
    // reset behaviour (during reset the DUT ignores rsp, so neither must our
    // shadow of it carry a stale rsp_valid across the reset boundary).
    reg prev_rsp_valid = 1'b0;
    lnp64_rsp_t prev_rsp;
    always @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            prev_rsp_valid <= 1'b0;
            prev_rsp <= '0;
        end else begin
            prev_rsp_valid <= rsp_valid;
            prev_rsp <= rsp;
        end
    end

    always @(posedge clk) begin
        if (reset_n) begin
            // A completion is valid iff a response was valid the previous cycle.
            a_completion_tracks_rsp:
                assert (completion_valid == prev_rsp_valid);

            // The completion faithfully forwards the response outcome.
            if (completion_valid) begin
                a_status_forwarded:    assert (completion.status == prev_rsp.status);
                a_errno_forwarded:     assert (completion.errno_value == prev_rsp.errno_value);
                a_value_forwarded:     assert (completion.value == prev_rsp.result_value);
            end
        end
    end
endmodule

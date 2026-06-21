`timescale 1ns/1ps

// Reset-ISOLATION proof (two-copy miter) for the MVS reset-iso core.
//
// Two instances are driven with IDENTICAL inputs except initiator 0's soft
// reset (srst0_a vs srst0_b, free and independent). The assertions prove that
// however initiator 0 is reset, initiator 1's local state, capability, and
// writes are bit-identical -- resetting one component cannot corrupt another's
// authority. Proven on the actual RTL over all input sequences.
module lnp64_mvs_reset_iso_formal (
    input logic       clk,
    input logic       rst_n,
    input logic       srst0_a,
    input logic       srst0_b,
    input logic       srst1,
    input logic       req0,
    input logic [7:0] addr0,
    input logic       req1,
    input logic [7:0] addr1
);
    logic [1:0] ph0_a, ph1_a, ph0_b, ph1_b;
    logic       cv0_a, cv1_a, we0_a, we1_a;
    logic       cv0_b, cv1_b, we0_b, we1_b;

    lnp64_mvs_reset_iso inst_a (
        .clk(clk), .rst_n(rst_n), .srst0(srst0_a), .srst1(srst1),
        .req0(req0), .addr0(addr0), .req1(req1), .addr1(addr1),
        .phase0(ph0_a), .phase1(ph1_a), .cap0_valid(cv0_a), .cap1_valid(cv1_a),
        .we0(we0_a), .we1(we1_a)
    );

    lnp64_mvs_reset_iso inst_b (
        .clk(clk), .rst_n(rst_n), .srst0(srst0_b), .srst1(srst1),
        .req0(req0), .addr0(addr0), .req1(req1), .addr1(addr1),
        .phase0(ph0_b), .phase1(ph1_b), .cap0_valid(cv0_b), .cap1_valid(cv1_b),
        .we0(we0_b), .we1(we1_b)
    );

    reg init_done = 1'b0;
    always @(posedge clk) init_done <= 1'b1;
    always @(posedge clk) begin
        if (!init_done) assume (!rst_n);
    end

    always @(posedge clk) begin
        if (rst_n) begin
            // Initiator 0's reset cannot change initiator 1's local state...
            a_phase1_isolated:      assert (ph1_a == ph1_b);
            // ...its capability...
            a_cap1_isolated:        assert (cv1_a == cv1_b);
            // ...or its writes.
            a_write1_isolated:      assert (we1_a == we1_b);
        end
    end
endmodule

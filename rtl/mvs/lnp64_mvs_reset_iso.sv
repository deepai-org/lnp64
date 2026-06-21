`timescale 1ns/1ps

// LNP64 MVS reset-isolation core.
//
// Two initiators, each with a small local pipeline FSM (phase) and its own
// revocable capability, plus an out-of-band per-initiator soft reset (e.g. the
// debug agent resetting a stuck component). A soft reset of one initiator must
// affect ONLY that initiator's local state and capability -- never the other
// initiator's phase, capability, or writes. This module isolates each soft
// reset structurally; the reset-isolation proof confirms there is no cross path.
module lnp64_mvs_reset_iso (
    input  logic       clk,
    input  logic       rst_n,

    input  logic       srst0,        // soft reset for initiator 0
    input  logic       srst1,        // soft reset for initiator 1

    input  logic       req0,
    input  logic [7:0] addr0,
    input  logic       req1,
    input  logic [7:0] addr1,

    output logic [1:0] phase0,
    output logic [1:0] phase1,
    output logic       cap0_valid,
    output logic       cap1_valid,
    output logic       we0,
    output logic       we1
);
    logic [1:0] ph0, ph1;
    logic       cv0, cv1;

    assign phase0 = ph0;
    assign phase1 = ph1;
    assign cap0_valid = cv0;
    assign cap1_valid = cv1;

    // Each initiator writes only within its own page and with a valid capability.
    assign we0 = req0 && cv0 && (addr0[7:4] == 4'h1);
    assign we1 = req1 && cv1 && (addr1[7:4] == 4'h2);

    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            ph0 <= 2'd0; cv0 <= 1'b1;
            ph1 <= 2'd0; cv1 <= 1'b1;
        end else begin
            // Initiator 0 local state: its own soft reset touches ONLY ph0/cv0.
            if (srst0) begin
                ph0 <= 2'd0;
                cv0 <= 1'b0;       // a reset component loses its own capability
            end else if (req0) begin
                ph0 <= ph0 + 2'd1;
            end
            // Initiator 1 local state: its own soft reset touches ONLY ph1/cv1.
            if (srst1) begin
                ph1 <= 2'd0;
                cv1 <= 1'b0;
            end else if (req1) begin
                ph1 <= ph1 + 2'd1;
            end
        end
    end
endmodule

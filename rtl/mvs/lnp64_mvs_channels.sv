`timescale 1ns/1ps

// LNP64 MVS two-channel interconnect core (closes the protocol-deadlock gap).
//
// A real fabric has separate Request and Response channels; deadlock arises when
// a response cannot be sent because it (transitively) waits on a request being
// accepted, forming a cycle. This core breaks the cycle by construction with a
// strict acyclic ordering:
//   * Responses drain INDEPENDENTLY of the request channel: a response is
//     offered whenever one is pending, never gated on request acceptance.
//   * Requests are admitted only when there is RESPONSE-buffer space, so every
//     accepted request has a reserved response slot (the request side depends on
//     the response side, never the reverse).
// With this ordering the protocol cannot deadlock: a drained response channel
// always makes progress, and full response buffering only back-pressures new
// requests (correct flow control, not a cycle).
module lnp64_mvs_channels (
    input  logic clk,
    input  logic rst_n,

    // Request channel in (free adversarial producer).
    input  logic req_valid,
    output logic req_ready,

    // Response channel out (free adversarial consumer readiness).
    output logic rsp_valid,
    input  logic rsp_ready,

    output logic [1:0] resp_count
);
    localparam logic [1:0] CAP = 2'd2;  // response buffer depth

    logic [1:0] cnt;
    assign resp_count = cnt;

    // Responses are offered whenever pending -- independent of req_ready.
    assign rsp_valid = (cnt != 2'd0);

    // Requests admitted iff there is room for the response (resp-space flow
    // control) -- this is the ONLY coupling, and it is request-depends-on-
    // response, never the reverse.
    assign req_ready = (cnt != CAP);

    logic do_accept, do_send;
    assign do_accept = req_valid && req_ready;
    assign do_send   = rsp_valid && rsp_ready;

    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            cnt <= 2'd0;
        end else begin
            unique case ({do_accept, do_send})
                2'b10: cnt <= cnt + 2'd1;  // a new request reserves a response slot
                2'b01: cnt <= cnt - 2'd1;  // a response drained
                default: cnt <= cnt;        // both or neither: net zero
            endcase
        end
    end
endmodule

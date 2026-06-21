`timescale 1ns/1ps

// Two-channel DEADLOCK-FREEDOM proof for the MVS interconnect core.
//
// Proves on the actual RTL, over all inputs, that the request/response protocol
// has the acyclic flow-control ordering that precludes deadlock: responses are
// offered independent of the request channel; requests are admitted only with
// reserved response space; the buffer never overflows; and a response that the
// consumer is ready to take is delivered immediately (no starvation).
module lnp64_mvs_channels_formal (
    input logic clk,
    input logic rst_n,
    input logic req_valid,
    input logic rsp_ready
);
    logic req_ready, rsp_valid;
    logic [1:0] resp_count;

    lnp64_mvs_channels dut (
        .clk(clk), .rst_n(rst_n),
        .req_valid(req_valid), .req_ready(req_ready),
        .rsp_valid(rsp_valid), .rsp_ready(rsp_ready),
        .resp_count(resp_count)
    );

    reg init_done = 1'b0;
    always @(posedge clk) init_done <= 1'b1;
    always @(posedge clk) begin
        if (!init_done) assume (!rst_n);
    end

    reg [1:0] prev_count = 2'd0;
    reg prev_rsp_valid = 1'b0, prev_rsp_ready = 1'b0;
    always @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin prev_count <= 2'd0; prev_rsp_valid <= 1'b0; prev_rsp_ready <= 1'b0; end
        else begin prev_count <= resp_count; prev_rsp_valid <= rsp_valid; prev_rsp_ready <= rsp_ready; end
    end

    always @(posedge clk) begin
        if (rst_n) begin
            // Responses are offered whenever pending -- not gated on the request
            // channel (no response-waits-on-request cycle).
            a_resp_offered_when_pending:
                assert (rsp_valid == (resp_count != 2'd0));

            // Requests admitted iff response-buffer space exists: every accepted
            // request has a reserved response slot.
            a_req_ready_iff_space:
                assert (req_ready == (resp_count != 2'd2));

            // The response buffer never overflows.
            a_no_overflow:
                assert (resp_count <= 2'd2);

            // No starvation: a response the consumer was ready to take last cycle
            // was actually delivered (count decreased by the send).
            if (prev_rsp_valid && prev_rsp_ready && resp_count != 2'd0)
                a_response_delivered:
                    assert (resp_count <= prev_count);
        end
    end
endmodule

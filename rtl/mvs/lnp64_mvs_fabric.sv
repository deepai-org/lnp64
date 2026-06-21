`timescale 1ns/1ps

// LNP64 MVS pipelined-fabric core (closes the in-flight revocation gap).
//
// A real interconnect is pipelined: an initiator enqueues a write request that
// sits in a fabric FIFO for several cycles before it reaches memory. A naive
// design that checks the capability at the INITIATOR boundary lets an in-flight
// write commit after the capability has been revoked. This core instead places
// the capability checker at the MEMORY ENDPOINT: the write strobe is gated by
// the CURRENT capability state at the moment the request is dequeued, so a
// revocation that happens while a request is in flight still blocks it.
//
// Depth-3 shift FIFO of {id,addr,we} requests; capability table revocable.
module lnp64_mvs_fabric (
    input  logic       clk,
    input  logic       rst_n,

    // Initiator boundary: enqueue a write request (free adversarial inputs).
    input  logic       enq_valid,
    input  logic [1:0] enq_id,
    input  logic [7:0] enq_addr,
    input  logic       enq_we,

    // Revocation (may fire at any time, including while requests are in flight).
    input  logic       revoke_valid,
    input  logic [1:0] revoke_id,

    // Memory endpoint ready/valid backpressure.
    input  logic       deq_ready,

    output logic       mem_we,
    output logic [7:0] mem_waddr,
    output logic [1:0] mem_wid,
    output logic       cpu_cap_valid,
    output logic       dma_cap_valid,
    output logic [1:0] fifo_count
);
    // ---- Revocable capability table (boot valid for id0/id1; id2 none) ----
    logic [1:0] cap_valid;
    assign cpu_cap_valid = cap_valid[0];
    assign dma_cap_valid = cap_valid[1];

    function automatic logic cap_live(input [1:0] id);
        unique case (id)
            2'd0:    cap_live = cap_valid[0];
            2'd1:    cap_live = cap_valid[1];
            default: cap_live = 1'b0;
        endcase
    endfunction

    function automatic logic id_has_page(input [1:0] id, input [7:0] addr);
        unique case (id)
            2'd0:    id_has_page = (addr[7:4] == 4'h1);
            2'd1:    id_has_page = (addr[7:4] == 4'h2);
            default: id_has_page = 1'b0;
        endcase
    endfunction

    // ---- Depth-3 shift FIFO of in-flight requests {id, addr, we} ----
    // entry layout: {id[1:0], addr[7:0], we} = 11 bits
    logic [10:0] e0, e1, e2;
    logic [1:0]  cnt;
    assign fifo_count = cnt;

    logic        head_valid;
    logic [1:0]  head_id;
    logic [7:0]  head_addr;
    logic        head_we;
    assign head_valid = (cnt != 2'd0);
    assign head_id    = e0[10:9];
    assign head_addr  = e0[8:1];
    assign head_we    = e0[0];

    logic do_enq, do_deq;
    assign do_enq = enq_valid && (cnt != 2'd3);
    assign do_deq = head_valid && deq_ready;

    // ---- Endpoint capability check: re-evaluated with CURRENT cap state ----
    assign mem_we    = do_deq && head_we && id_has_page(head_id, head_addr) && cap_live(head_id);
    assign mem_waddr = head_addr;
    assign mem_wid   = head_id;

    logic [10:0] enq_word;
    assign enq_word = {enq_id, enq_addr, enq_we};

    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            e0 <= 11'd0; e1 <= 11'd0; e2 <= 11'd0;
            cnt <= 2'd0;
            cap_valid <= 2'b11;
        end else begin
            // Revocation is permanent (clear-only); takes effect immediately at
            // the endpoint for any request not yet dequeued.
            if (revoke_valid && revoke_id == 2'd0) cap_valid[0] <= 1'b0;
            if (revoke_valid && revoke_id == 2'd1) cap_valid[1] <= 1'b0;

            // Shift FIFO maintenance.
            unique case ({do_enq, do_deq})
                2'b10: begin // enqueue only
                    unique case (cnt)
                        2'd0: e0 <= enq_word;
                        2'd1: e1 <= enq_word;
                        default: e2 <= enq_word;
                    endcase
                    cnt <= cnt + 2'd1;
                end
                2'b01: begin // dequeue only: shift down
                    e0 <= e1; e1 <= e2; e2 <= 11'd0;
                    cnt <= cnt - 2'd1;
                end
                2'b11: begin // simultaneous enq + deq: shift and insert at new tail
                    e0 <= e1; e1 <= e2;
                    unique case (cnt)
                        2'd1: e0 <= enq_word;
                        2'd2: e1 <= enq_word;
                        default: e2 <= enq_word;
                    endcase
                    // cnt unchanged
                end
                default: ; // idle
            endcase
        end
    end
endmodule

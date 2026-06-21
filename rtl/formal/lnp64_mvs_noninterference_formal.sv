`timescale 1ns/1ps

// Non-interference (two-copy miter) proof for the LNP64 MVS.
//
// Two identical MVS instances are driven with IDENTICAL in-band initiator inputs
// (CPU, DMA) and identical legitimate out-of-band timing inputs (dbg_stall,
// dbg_req -- the debug agent is allowed to request and to halt arbitration).
// They differ ONLY in the debug agent's tamper content: what (dbg_we) and where
// (dbg_addr) it attempts to write. The assertions prove that this tamper content
// cannot flow into the authority decision or the memory write -- i.e. debug
// state does not flow into authority state. Combined with the mediation proof
// (debug can never write at all), this is the covert-channel / non-interference
// guarantee for the debug/reset agent, on the actual RTL over all inputs.
module lnp64_mvs_noninterference_formal (
    input logic        clk,
    input logic        rst_n,
    // shared in-band initiators
    input logic        cpu_req,
    input logic        cpu_we,
    input logic [15:0] cpu_addr,
    input logic        dma_req,
    input logic        dma_we,
    input logic [15:0] dma_addr,
    // shared legitimate out-of-band timing controls
    input logic        dbg_stall,
    input logic        dbg_req,
    // independent debug tamper content per instance
    input logic        dbg_we_a,
    input logic [15:0] dbg_addr_a,
    input logic        dbg_we_b,
    input logic [15:0] dbg_addr_b
);
    logic        we_a, cap_a, gval_a;
    logic [15:0] waddr_a;
    logic [1:0]  gid_a, rr_a;
    logic        we_b, cap_b, gval_b;
    logic [15:0] waddr_b;
    logic [1:0]  gid_b, rr_b;

    lnp64_mvs inst_a (
        .clk(clk), .rst_n(rst_n),
        .cpu_req(cpu_req), .cpu_we(cpu_we), .cpu_addr(cpu_addr),
        .dma_req(dma_req), .dma_we(dma_we), .dma_addr(dma_addr),
        .dbg_stall(dbg_stall), .dbg_req(dbg_req), .dbg_we(dbg_we_a), .dbg_addr(dbg_addr_a),
        .mem_we(we_a), .mem_waddr(waddr_a), .granted_id(gid_a),
        .grant_valid(gval_a), .cap_authorized(cap_a), .arb_rr(rr_a)
    );

    lnp64_mvs inst_b (
        .clk(clk), .rst_n(rst_n),
        .cpu_req(cpu_req), .cpu_we(cpu_we), .cpu_addr(cpu_addr),
        .dma_req(dma_req), .dma_we(dma_we), .dma_addr(dma_addr),
        .dbg_stall(dbg_stall), .dbg_req(dbg_req), .dbg_we(dbg_we_b), .dbg_addr(dbg_addr_b),
        .mem_we(we_b), .mem_waddr(waddr_b), .granted_id(gid_b),
        .grant_valid(gval_b), .cap_authorized(cap_b), .arb_rr(rr_b)
    );

    reg init_done = 1'b0;
    always @(posedge clk) init_done <= 1'b1;
    always @(posedge clk) begin
        if (!init_done) assume (!rst_n);
    end

    always @(posedge clk) begin
        if (rst_n) begin
            // Strengthening invariant: both instances run the arbiter in
            // lock-step (identical timing inputs => identical pointer), which
            // makes the non-interference properties inductive.
            a_arbiter_lockstep:
                assert (rr_a == rr_b);

            // Debug tamper content cannot change the authority decision...
            a_authority_noninterference:
                assert (cap_a == cap_b);

            // ...nor whether a memory write occurs...
            a_write_noninterference:
                assert (we_a == we_b);

            // ...nor the arbitration outcome (timing inputs are shared).
            a_grant_noninterference:
                assert (gval_a == gval_b && gid_a == gid_b);

            // ...nor, when a write occurs, its address.
            if (we_a)
                a_waddr_noninterference: assert (waddr_a == waddr_b);
        end
    end
endmodule

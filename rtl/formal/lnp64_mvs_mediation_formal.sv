`timescale 1ns/1ps

// Whole-chip MEDIATION proof harness for the LNP64 MVS.
//
// All initiator and debug ports are free inputs (an arbitrary adversary every
// cycle). The assertions prove, on the actual RTL over all input sequences, that
// every memory write is mediated by the capability checker and confined to the
// granted initiator's authorized region -- and that the out-of-band debug agent
// can never produce a write.
module lnp64_mvs_mediation_formal (
    input logic        clk,
    input logic        rst_n,
    input logic        cpu_req,
    input logic        cpu_we,
    input logic [15:0] cpu_addr,
    input logic        dma_req,
    input logic        dma_we,
    input logic [15:0] dma_addr,
    input logic        dbg_stall,
    input logic        dbg_req,
    input logic        dbg_we,
    input logic [15:0] dbg_addr
);
    logic        mem_we;
    logic [15:0] mem_waddr;
    logic [1:0]  granted_id;
    logic        grant_valid;
    logic        cap_authorized;

    lnp64_mvs dut (
        .clk(clk), .rst_n(rst_n),
        .cpu_req(cpu_req), .cpu_we(cpu_we), .cpu_addr(cpu_addr),
        .dma_req(dma_req), .dma_we(dma_we), .dma_addr(dma_addr),
        .dbg_stall(dbg_stall), .dbg_req(dbg_req), .dbg_we(dbg_we), .dbg_addr(dbg_addr),
        .mem_we(mem_we), .mem_waddr(mem_waddr),
        .granted_id(granted_id), .grant_valid(grant_valid), .cap_authorized(cap_authorized)
    );

    // Power-on reset discipline.
    reg init_done = 1'b0;
    always @(posedge clk) init_done <= 1'b1;
    always @(posedge clk) begin
        if (!init_done) assume (!rst_n);
    end

    always @(posedge clk) begin
        if (rst_n) begin
            // Strict mediation: a memory write occurs only if the capability
            // checker authorized it this cycle.
            a_write_implies_authorized:
                assert (!mem_we || cap_authorized);

            // The out-of-band debug agent can NEVER cause a memory write.
            a_debug_cannot_write:
                assert (!(mem_we && granted_id == 2'd2));

            // Address confinement: each initiator can only write its own page.
            if (mem_we && granted_id == 2'd0)
                a_cpu_confined: assert (mem_waddr[15:8] == 8'h01);
            if (mem_we && granted_id == 2'd1)
                a_dma_confined: assert (mem_waddr[15:8] == 8'h02);

            // A write is always attributed to an in-band initiator.
            if (mem_we)
                a_write_has_initiator: assert (grant_valid && granted_id != 2'd2);
        end
    end
endmodule

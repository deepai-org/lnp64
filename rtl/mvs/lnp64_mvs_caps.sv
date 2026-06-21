`timescale 1ns/1ps

// LNP64 MVS capability-revocation core.
//
// Extends the MVS mediation core with per-initiator capability-valid registers
// and a revoke path, to prove temporal isolation: once a capability is revoked
// it can never authorize a write again (revocation is permanent and effective).
// CPU (id0) and DMA (id1) hold revocable write capabilities; the out-of-band
// debug agent (id2) holds none. Authorization requires both the fixed page
// right AND a currently-valid capability. There is no re-validation path in
// hardware (re-granting authority is a separate, unmodeled operation), so
// cap_valid is monotonically non-increasing.
module lnp64_mvs_caps (
    input  logic        clk,
    input  logic        rst_n,

    input  logic        cpu_req,
    input  logic        cpu_we,
    input  logic [15:0] cpu_addr,

    input  logic        dma_req,
    input  logic        dma_we,
    input  logic [15:0] dma_addr,

    input  logic        dbg_stall,
    input  logic        dbg_req,
    input  logic        dbg_we,
    input  logic [15:0] dbg_addr,

    // Revocation port: when revoke_valid, clear the target initiator's cap.
    input  logic        revoke_valid,
    input  logic [1:0]  revoke_id,

    output logic        mem_we,
    output logic [15:0] mem_waddr,
    output logic [1:0]  granted_id,
    output logic        grant_valid,
    output logic        cap_authorized,
    output logic        cpu_cap_valid,
    output logic        dma_cap_valid
);
    // ---- Per-initiator capability-valid state (boot valid; revoke-only) ----
    logic [1:0] cap_valid;  // index 0 = cpu, 1 = dma
    assign cpu_cap_valid = cap_valid[0];
    assign dma_cap_valid = cap_valid[1];

    // ---- Round-robin arbiter (shared with the base MVS design) ----
    logic [1:0] rr;
    logic [2:0] reqv;
    assign reqv = {dbg_req, dma_req, cpu_req};

    function automatic [1:0] inc3(input [1:0] x);
        inc3 = (x == 2'd2) ? 2'd0 : x + 2'd1;
    endfunction

    logic [1:0] o0, o1, o2;
    assign o0 = rr;
    assign o1 = inc3(rr);
    assign o2 = inc3(o1);

    logic [1:0] gidx;
    logic       gval;
    always_comb begin
        gidx = 2'd0;
        gval = 1'b0;
        if (!dbg_stall) begin
            if (reqv[o0])      begin gidx = o0; gval = 1'b1; end
            else if (reqv[o1]) begin gidx = o1; gval = 1'b1; end
            else if (reqv[o2]) begin gidx = o2; gval = 1'b1; end
        end
    end

    assign granted_id  = gidx;
    assign grant_valid = gval;

    logic        sel_we;
    logic [15:0] sel_addr;
    always_comb begin
        unique case (gidx)
            2'd0:    begin sel_we = cpu_we; sel_addr = cpu_addr; end
            2'd1:    begin sel_we = dma_we; sel_addr = dma_addr; end
            default: begin sel_we = dbg_we; sel_addr = dbg_addr; end
        endcase
    end

    // base (page) rights keyed on hardwired id
    function automatic logic id_has_page(input [1:0] id, input [15:0] addr);
        unique case (id)
            2'd0:    id_has_page = (addr[15:8] == 8'h01);
            2'd1:    id_has_page = (addr[15:8] == 8'h02);
            default: id_has_page = 1'b0;
        endcase
    endfunction

    // currently-valid capability for the granted id (debug never has one)
    logic id_cap_live;
    always_comb begin
        unique case (gidx)
            2'd0:    id_cap_live = cap_valid[0];
            2'd1:    id_cap_live = cap_valid[1];
            default: id_cap_live = 1'b0;
        endcase
    end

    // ---- Choke point: page right AND a live (un-revoked) capability ----
    assign cap_authorized = gval && sel_we && id_has_page(gidx, sel_addr) && id_cap_live;
    assign mem_we    = cap_authorized;
    assign mem_waddr = sel_addr;

    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            rr <= 2'd0;
            cap_valid <= 2'b11;   // both capabilities valid at boot
        end else begin
            if (gval)
                rr <= inc3(gidx);
            // Revocation is permanent: clear, never set. (No re-validation path.)
            if (revoke_valid && revoke_id == 2'd0) cap_valid[0] <= 1'b0;
            if (revoke_valid && revoke_id == 2'd1) cap_valid[1] <= 1'b0;
        end
    end
endmodule

`timescale 1ns/1ps

// LNP64 Minimal Viable System (MVS) for whole-chip mediation / progress proofs.
//
// Provability-first minimal core: two in-band initiators (CPU id0, DMA id1) and
// one OUT-OF-BAND debug/reset agent (id2) all contend for a single memory target
// through a round-robin arbiter and a single, unbypassable capability-checker
// choke point. The capability policy is keyed on the HARDWIRED initiator id (not
// a value the requester supplies), so an initiator cannot claim authority it
// does not structurally have. The debug agent may stall arbitration and may
// drive request wires, but the policy grants it no write rights and its wires
// have no path to the write-enable except through the same checker.
//
// Self-contained (no package), no large memory array -- mediation is a property
// of the write-enable / address / granted-id signals, so the model stays tiny
// and BMC/k-induction dispatch instantly. All request inputs are free, i.e. an
// arbitrary adversary on every port every cycle.
module lnp64_mvs (
    input  logic        clk,
    input  logic        rst_n,

    // CPU initiator (hardwired id 0)
    input  logic        cpu_req,
    input  logic        cpu_we,
    input  logic [15:0] cpu_addr,

    // DMA initiator (hardwired id 1)
    input  logic        dma_req,
    input  logic        dma_we,
    input  logic [15:0] dma_addr,

    // Out-of-band debug/reset agent (hardwired id 2): may stall, may drive a
    // would-be write; it holds no capability.
    input  logic        dbg_stall,
    input  logic        dbg_req,
    input  logic        dbg_we,
    input  logic [15:0] dbg_addr,

    // Observation points for the proof harness.
    output logic        mem_we,        // the single memory write strobe
    output logic [15:0] mem_waddr,
    output logic [1:0]  granted_id,
    output logic        grant_valid,
    output logic        cap_authorized
);
    // ---- Round-robin arbiter (3 -> 1) with debug stall as backpressure ----
    logic [1:0] rr;  // index to favor first this round

    // request vector: index 0 = cpu, 1 = dma, 2 = debug
    logic [2:0] reqv;
    assign reqv = {dbg_req, dma_req, cpu_req};

    // rotating candidate order rr, rr+1, rr+2 (mod 3)
    function automatic [1:0] inc3(input [1:0] x);
        inc3 = (x == 2'd2) ? 2'd0 : x + 2'd1;
    endfunction

    logic [1:0] o0, o1, o2;
    assign o0 = rr;
    assign o1 = inc3(rr);
    assign o2 = inc3(o1);

    logic stalled;
    assign stalled = dbg_stall;

    logic [1:0] gidx;
    logic       gval;
    always_comb begin
        gidx = 2'd0;
        gval = 1'b0;
        if (!stalled) begin
            if (reqv[o0])      begin gidx = o0; gval = 1'b1; end
            else if (reqv[o1]) begin gidx = o1; gval = 1'b1; end
            else if (reqv[o2]) begin gidx = o2; gval = 1'b1; end
        end
    end

    assign granted_id  = gidx;
    assign grant_valid = gval;

    // selected request fields (by hardwired id, not requester-supplied identity)
    logic        sel_we;
    logic [15:0] sel_addr;
    always_comb begin
        unique case (gidx)
            2'd0:    begin sel_we = cpu_we; sel_addr = cpu_addr; end
            2'd1:    begin sel_we = dma_we; sel_addr = dma_addr; end
            default: begin sel_we = dbg_we; sel_addr = dbg_addr; end
        endcase
    end

    // ---- Capability checker: the fixed mediation policy ----
    // Rights keyed on the hardwired initiator id:
    //   id0 (CPU)   may write page 0x01xx
    //   id1 (DMA)   may write page 0x02xx
    //   id2 (DEBUG) holds no write capability
    function automatic logic id_has_rights(input [1:0] id, input [15:0] addr);
        unique case (id)
            2'd0:    id_has_rights = (addr[15:8] == 8'h01);
            2'd1:    id_has_rights = (addr[15:8] == 8'h02);
            default: id_has_rights = 1'b0;
        endcase
    endfunction

    assign cap_authorized = gval && sel_we && id_has_rights(gidx, sel_addr);

    // ---- The single unbypassable choke point ----
    // Memory write-enable is STRUCTURALLY the capability decision. No debug wire
    // reaches this except through the checker above.
    assign mem_we    = cap_authorized;
    assign mem_waddr = sel_addr;

    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            rr <= 2'd0;
        end else if (gval) begin
            rr <= inc3(gidx);  // rotate fairness past the served initiator
        end
    end
endmodule

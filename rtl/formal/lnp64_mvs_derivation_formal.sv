`timescale 1ns/1ps

// Capability DERIVATION / no-forged-authority proof for the MVS derive core.
//
// Proves on the actual RTL, over all inputs, that an initiator can never forge
// authority during execution: the held capability is always a subset of its
// root, derivation is strictly non-widening, and every memory write is confined
// to the root authority.
module lnp64_mvs_derivation_formal (
    input logic       clk,
    input logic       rst_n,
    input logic       derive_valid,
    input logic [7:0] req_lo,
    input logic [7:0] req_hi,
    input logic       req_w,
    input logic       acc_req,
    input logic       acc_w,
    input logic [7:0] acc_addr
);
    localparam logic [7:0] ROOT_LO = 8'h10;
    localparam logic [7:0] ROOT_HI = 8'h7f;
    localparam logic       ROOT_W  = 1'b1;

    logic [7:0] cap_lo, cap_hi, mem_waddr;
    logic       cap_w, derive_accepted, mem_we;

    lnp64_mvs_derive dut (
        .clk(clk), .rst_n(rst_n),
        .derive_valid(derive_valid), .req_lo(req_lo), .req_hi(req_hi), .req_w(req_w),
        .acc_req(acc_req), .acc_w(acc_w), .acc_addr(acc_addr),
        .cap_lo(cap_lo), .cap_hi(cap_hi), .cap_w(cap_w),
        .derive_accepted(derive_accepted), .mem_we(mem_we), .mem_waddr(mem_waddr)
    );

    reg init_done = 1'b0;
    always @(posedge clk) init_done <= 1'b1;
    always @(posedge clk) begin
        if (!init_done) assume (!rst_n);
    end

    // Shadow the previous capability to express monotone narrowing.
    reg [7:0] p_lo, p_hi;
    reg       p_w;
    always @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            p_lo <= ROOT_LO; p_hi <= ROOT_HI; p_w <= ROOT_W;
        end else begin
            p_lo <= cap_lo; p_hi <= cap_hi; p_w <= cap_w;
        end
    end

    always @(posedge clk) begin
        if (rst_n) begin
            // No forged authority: the held capability never exceeds its root.
            a_cap_within_root:
                assert (cap_lo >= ROOT_LO && cap_hi <= ROOT_HI && (!cap_w || ROOT_W)
                        && cap_lo <= cap_hi);

            // Derivation is strictly non-widening (subset of what was held).
            a_derive_non_widening:
                assert (cap_lo >= p_lo && cap_hi <= p_hi && (!cap_w || p_w));

            // Every memory write is confined to the root authority and needs the
            // write capability.
            if (mem_we)
                a_write_confined_to_root:
                    assert (mem_waddr >= ROOT_LO && mem_waddr <= ROOT_HI && cap_w
                            && mem_waddr >= cap_lo && mem_waddr <= cap_hi);
        end
    end
endmodule

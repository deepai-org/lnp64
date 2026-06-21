`timescale 1ns/1ps

// Capability REVOCATION / temporal-isolation proof for the LNP64 MVS caps core.
//
// Proves on the actual RTL, over all inputs, that revocation works and is
// permanent: a write only ever occurs while the granting initiator's capability
// is valid, the capability state is monotonically non-increasing (once revoked,
// never re-validated), and therefore a revoked initiator can never write again.
module lnp64_mvs_revocation_formal (
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
    input logic [15:0] dbg_addr,
    input logic        revoke_valid,
    input logic [1:0]  revoke_id
);
    logic        mem_we, grant_valid, cap_authorized, cpu_cap_valid, dma_cap_valid;
    logic [15:0] mem_waddr;
    logic [1:0]  granted_id;

    lnp64_mvs_caps dut (
        .clk(clk), .rst_n(rst_n),
        .cpu_req(cpu_req), .cpu_we(cpu_we), .cpu_addr(cpu_addr),
        .dma_req(dma_req), .dma_we(dma_we), .dma_addr(dma_addr),
        .dbg_stall(dbg_stall), .dbg_req(dbg_req), .dbg_we(dbg_we), .dbg_addr(dbg_addr),
        .revoke_valid(revoke_valid), .revoke_id(revoke_id),
        .mem_we(mem_we), .mem_waddr(mem_waddr), .granted_id(granted_id),
        .grant_valid(grant_valid), .cap_authorized(cap_authorized),
        .cpu_cap_valid(cpu_cap_valid), .dma_cap_valid(dma_cap_valid)
    );

    reg init_done = 1'b0;
    always @(posedge clk) init_done <= 1'b1;
    always @(posedge clk) begin
        if (!init_done) assume (!rst_n);
    end

    // Shadow the previous capability bits to express monotonicity.
    reg prev_cpu_cap = 1'b1;
    reg prev_dma_cap = 1'b1;
    always @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            prev_cpu_cap <= 1'b1;
            prev_dma_cap <= 1'b1;
        end else begin
            prev_cpu_cap <= cpu_cap_valid;
            prev_dma_cap <= dma_cap_valid;
        end
    end

    always @(posedge clk) begin
        if (rst_n) begin
            // A write only occurs while the granting initiator's cap is valid.
            if (mem_we && granted_id == 2'd0)
                a_cpu_write_requires_cap: assert (cpu_cap_valid);
            if (mem_we && granted_id == 2'd1)
                a_dma_write_requires_cap: assert (dma_cap_valid);

            // Revocation is permanent: capability bits never go 0 -> 1.
            a_cpu_cap_monotonic: assert (!(prev_cpu_cap == 1'b0 && cpu_cap_valid == 1'b1));
            a_dma_cap_monotonic: assert (!(prev_dma_cap == 1'b0 && dma_cap_valid == 1'b1));

            // Therefore a revoked initiator can never produce a write.
            if (!cpu_cap_valid)
                a_revoked_cpu_cannot_write: assert (!(mem_we && granted_id == 2'd0));
            if (!dma_cap_valid)
                a_revoked_dma_cannot_write: assert (!(mem_we && granted_id == 2'd1));
        end
    end
endmodule

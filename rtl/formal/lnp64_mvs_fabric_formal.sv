`timescale 1ns/1ps

// In-flight REVOCATION proof for the pipelined-fabric MVS core.
//
// Proves on the actual RTL, over all inputs, that placing the capability check
// at the memory endpoint makes revocation effective even against requests that
// are already in flight in the fabric FIFO: a revoked initiator can never
// produce a memory write, no matter how many of its requests were enqueued
// before the revoke.
module lnp64_mvs_fabric_formal (
    input logic       clk,
    input logic       rst_n,
    input logic       enq_valid,
    input logic [1:0] enq_id,
    input logic [7:0] enq_addr,
    input logic       enq_we,
    input logic       revoke_valid,
    input logic [1:0] revoke_id,
    input logic       deq_ready
);
    logic       mem_we;
    logic [7:0] mem_waddr;
    logic [1:0] mem_wid;
    logic       cpu_cap_valid, dma_cap_valid;
    logic [1:0] fifo_count;

    lnp64_mvs_fabric dut (
        .clk(clk), .rst_n(rst_n),
        .enq_valid(enq_valid), .enq_id(enq_id), .enq_addr(enq_addr), .enq_we(enq_we),
        .revoke_valid(revoke_valid), .revoke_id(revoke_id),
        .deq_ready(deq_ready),
        .mem_we(mem_we), .mem_waddr(mem_waddr), .mem_wid(mem_wid),
        .cpu_cap_valid(cpu_cap_valid), .dma_cap_valid(dma_cap_valid),
        .fifo_count(fifo_count)
    );

    reg init_done = 1'b0;
    always @(posedge clk) init_done <= 1'b1;
    always @(posedge clk) begin
        if (!init_done) assume (!rst_n);
    end

    reg prev_cpu = 1'b1, prev_dma = 1'b1;
    always @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin prev_cpu <= 1'b1; prev_dma <= 1'b1; end
        else        begin prev_cpu <= cpu_cap_valid; prev_dma <= dma_cap_valid; end
    end

    always @(posedge clk) begin
        if (rst_n) begin
            // A committed write is endpoint-mediated: confined to the writer's
            // page and requires a CURRENTLY-live capability.
            if (mem_we && mem_wid == 2'd0)
                a_cpu_endpoint_mediated: assert (cpu_cap_valid && mem_waddr[7:4] == 4'h1);
            if (mem_we && mem_wid == 2'd1)
                a_dma_endpoint_mediated: assert (dma_cap_valid && mem_waddr[7:4] == 4'h2);

            // The debug id (2) can never produce a write, in flight or not.
            a_debug_never_writes: assert (!(mem_we && mem_wid == 2'd2));

            // In-flight revocation: once revoked, that initiator can NEVER write,
            // even though earlier requests may still be sitting in the FIFO.
            if (!cpu_cap_valid)
                a_revoked_cpu_no_inflight_write: assert (!(mem_we && mem_wid == 2'd0));
            if (!dma_cap_valid)
                a_revoked_dma_no_inflight_write: assert (!(mem_we && mem_wid == 2'd1));

            // Revocation is permanent.
            a_cpu_revoke_sticky: assert (!(prev_cpu == 1'b0 && cpu_cap_valid == 1'b1));
            a_dma_revoke_sticky: assert (!(prev_dma == 1'b0 && dma_cap_valid == 1'b1));
        end
    end
endmodule

`timescale 1ns/1ps

// Bounded-PROGRESS / totality proof harness for the LNP64 MVS.
//
// Bounded liveness encoded as safety: a watchdog counter tracks consecutive
// cycles in which the CPU is requesting, arbitration is not halted by the debug
// agent, yet the CPU has not been granted. The round-robin arbiter (3 ports)
// must grant the CPU within 3 such cycles, so the counter can never exceed that
// bound. Also proves debug halt actually halts (no grant while stalled) and the
// arbiter never grants two initiators at once. All ports are free inputs.
module lnp64_mvs_progress_formal (
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

    reg init_done = 1'b0;
    always @(posedge clk) init_done <= 1'b1;
    always @(posedge clk) begin
        if (!init_done) assume (!rst_n);
    end

    wire cpu_granted = grant_valid && (granted_id == 2'd0);

    // Watchdog: consecutive cycles CPU requests + not halted + not yet served.
    reg [3:0] wait_cnt;
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n)
            wait_cnt <= 4'd0;
        else if (cpu_granted || dbg_stall || !cpu_req)
            wait_cnt <= 4'd0;            // served, halted, or idle: no obligation
        else
            wait_cnt <= wait_cnt + 4'd1; // requesting, unhalted, ungranted
    end

    always @(posedge clk) begin
        if (rst_n) begin
            // Bounded progress: an unhalted CPU request is granted within 3
            // cycles (round-robin over 3 ports), so the wait never exceeds it.
            a_bounded_cpu_grant:
                assert (wait_cnt <= 4'd3);

            // Debug halt actually halts: no grant is issued while stalled.
            a_no_grant_while_stalled:
                assert (!(dbg_stall && grant_valid));

            // A granted id is always a real port id (totality of the decode).
            if (grant_valid)
                a_grant_id_valid: assert (granted_id <= 2'd2);
        end
    end
endmodule

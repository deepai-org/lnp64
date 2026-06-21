`timescale 1ns/1ps

// LNP64 MVS hardware wait/wake core (the OS-in-silicon scheduler primitive).
//
// Two threads and one event source. Each thread is READY or PARKED. A ready
// thread that requests park parks UNLESS a wake for it is already pending, in
// which case it atomically consumes the wake and stays ready (this atomic
// park-or-consume is what prevents lost wakeups). An event targeted at a thread
// wakes it if parked, or is recorded as a pending wake if it is ready. The
// scheduler only ever selects a READY thread.
module lnp64_mvs_waitwake (
    input  logic clk,
    input  logic rst_n,

    input  logic park_req0,     // thread 0 requests to park
    input  logic park_req1,     // thread 1 requests to park
    input  logic event_fire,    // an event fires this cycle
    input  logic event_target,  // for thread 0 (0) or thread 1 (1)
    input  logic sched_tick,    // scheduler arbitration tick

    output logic parked0,
    output logic parked1,
    output logic pending0,      // pending (un-consumed) wake for thread 0
    output logic pending1,
    output logic sched_valid,
    output logic sched_id
);
    logic p0, p1;   // parked
    logic w0, w1;   // pending wake

    assign parked0  = p0;
    assign parked1  = p1;
    assign pending0 = w0;
    assign pending1 = w1;

    // Scheduler selects a READY (not parked) thread, thread 0 preferred.
    assign sched_valid = sched_tick && (!p0 || !p1);
    assign sched_id    = (!p0) ? 1'b0 : 1'b1;

    // Per-thread next-state (event handled before park, so a same-cycle event is
    // consumed by a parking thread -> no lost wakeup).
    logic ev0, ev1;
    assign ev0 = event_fire && (event_target == 1'b0);
    assign ev1 = event_fire && (event_target == 1'b1);

    logic n_p0, n_w0, n_p1, n_w1;
    always_comb begin
        // thread 0
        n_p0 = p0;
        n_w0 = w0;
        if (ev0) begin
            if (p0) n_p0 = 1'b0;
            else    n_w0 = 1'b1;
        end
        if (park_req0 && !p0) begin
            if (n_w0) n_w0 = 1'b0;
            else      n_p0 = 1'b1;
        end
        // thread 1
        n_p1 = p1;
        n_w1 = w1;
        if (ev1) begin
            if (p1) n_p1 = 1'b0;
            else    n_w1 = 1'b1;
        end
        if (park_req1 && !p1) begin
            if (n_w1) n_w1 = 1'b0;
            else      n_p1 = 1'b1;
        end
    end

    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            p0 <= 1'b0; w0 <= 1'b0;
            p1 <= 1'b0; w1 <= 1'b0;
        end else begin
            p0 <= n_p0; w0 <= n_w0;
            p1 <= n_p1; w1 <= n_w1;
        end
    end
endmodule

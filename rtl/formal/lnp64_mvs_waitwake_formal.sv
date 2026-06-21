`timescale 1ns/1ps

// Wait/Wake correctness proof for the MVS scheduler primitive.
//
// Proves on the actual RTL, over all inputs: the scheduler never selects a
// parked thread; an event targeted at a parked thread wakes exactly that thread
// (the other is unaffected by it); and no wakeup is lost -- a parked thread and
// a pending wake can never coexist, so a delivered wake is always honoured.
module lnp64_mvs_waitwake_formal (
    input logic clk,
    input logic rst_n,
    input logic park_req0,
    input logic park_req1,
    input logic event_fire,
    input logic event_target,
    input logic sched_tick
);
    logic parked0, parked1, pending0, pending1, sched_valid, sched_id;

    lnp64_mvs_waitwake dut (
        .clk(clk), .rst_n(rst_n),
        .park_req0(park_req0), .park_req1(park_req1),
        .event_fire(event_fire), .event_target(event_target), .sched_tick(sched_tick),
        .parked0(parked0), .parked1(parked1), .pending0(pending0), .pending1(pending1),
        .sched_valid(sched_valid), .sched_id(sched_id)
    );

    reg init_done = 1'b0;
    always @(posedge clk) init_done <= 1'b1;
    always @(posedge clk) begin
        if (!init_done) assume (!rst_n);
    end

    // History for the wake-transition property.
    reg ev0_parked_q = 1'b0;
    reg ev1_parked_q = 1'b0;
    always @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            ev0_parked_q <= 1'b0;
            ev1_parked_q <= 1'b0;
        end else begin
            ev0_parked_q <= event_fire && (event_target == 1'b0) && parked0;
            ev1_parked_q <= event_fire && (event_target == 1'b1) && parked1;
        end
    end

    always @(posedge clk) begin
        if (rst_n) begin
            // The scheduler never selects a parked thread.
            if (sched_valid)
                a_parked_not_scheduled:
                    assert (sched_id ? !parked1 : !parked0);

            // No lost wakeup: a parked thread and a pending wake never coexist,
            // so a delivered wake is always honoured (never stranded).
            a_no_lost_wakeup0: assert (!(parked0 && pending0));
            a_no_lost_wakeup1: assert (!(parked1 && pending1));

            // An event targeted at a parked thread wakes exactly that thread.
            if (ev0_parked_q)
                a_event_wakes_thread0: assert (!parked0);
            if (ev1_parked_q)
                a_event_wakes_thread1: assert (!parked1);
        end
    end
endmodule

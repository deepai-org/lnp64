`timescale 1ns/1ps

// Pipelined check-then-sleep race proof for the MVS wait pipeline.
//
// Proves on the actual RTL, over all inputs, that an event arriving anywhere in
// the evaluate->commit->park pipeline is never lost: a parked thread and a
// pending event never coexist, and an event during the commit window aborts the
// park rather than being dropped. This closes the lost-wakeup race that a
// single-cycle (atomic-park) model assumes away.
module lnp64_mvs_waitpipe_formal (
    input logic clk,
    input logic rst_n,
    input logic wait_issue,
    input logic event_fire
);
    logic [1:0] state;
    logic       pending_event, parked;

    lnp64_mvs_waitpipe dut (
        .clk(clk), .rst_n(rst_n),
        .wait_issue(wait_issue), .event_fire(event_fire),
        .state(state), .pending_event(pending_event), .parked(parked)
    );

    reg init_done = 1'b0;
    always @(posedge clk) init_done <= 1'b1;
    always @(posedge clk) begin
        if (!init_done) assume (!rst_n);
    end

    // Track that last cycle we were in the commit window with an event present.
    reg commit_event_q = 1'b0;
    always @(posedge clk or negedge rst_n) begin
        if (!rst_n) commit_event_q <= 1'b0;
        else        commit_event_q <= (state == 2'd1) && (event_fire || pending_event);
    end

    always @(posedge clk) begin
        if (rst_n) begin
            // No lost wakeup across the pipeline: a parked thread and a pending
            // event never coexist (a pending event always woke/aborted first).
            a_no_lost_wakeup_pipeline:
                assert (!(parked && pending_event));

            // An event during the commit window aborts the park: the thread is
            // not PARKED the next cycle (the Wait is retried, event observed).
            if (commit_event_q)
                a_commit_event_aborts_park:
                    assert (state != 2'd2);
        end
    end
endmodule

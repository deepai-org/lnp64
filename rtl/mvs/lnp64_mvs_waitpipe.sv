`timescale 1ns/1ps

// LNP64 MVS pipelined wait/wake core (closes the check-then-sleep race gap).
//
// A real CPU does not park atomically: it evaluates a condition, then commits a
// Wait instruction down a pipeline before the thread is actually PARKED. The
// classic lost-wakeup bug is an event arriving in the window between evaluate
// and park. This core makes that window explicit (READY -> COMMITTING -> PARKED)
// and buffers events in a pending-event register, so an event arriving during
// the COMMITTING window aborts the park (the wait is retried with the event
// observed) and is never dropped.
module lnp64_mvs_waitpipe (
    input  logic clk,
    input  logic rst_n,

    input  logic wait_issue,   // thread evaluated its condition and issues Wait
    input  logic event_fire,   // the awaited event fires this cycle

    output logic [1:0] state,        // 0=READY, 1=COMMITTING, 2=PARKED
    output logic       pending_event,
    output logic       parked
);
    localparam logic [1:0] S_READY  = 2'd0;
    localparam logic [1:0] S_COMMIT = 2'd1;
    localparam logic [1:0] S_PARKED = 2'd2;

    logic [1:0] st;
    logic       pe;
    assign state         = st;
    assign pending_event = pe;
    assign parked        = (st == S_PARKED);

    logic [1:0] n_st;
    logic       n_pe;
    always_comb begin
        n_st = st;
        n_pe = pe;
        unique case (st)
            S_READY: begin
                if (event_fire) n_pe = 1'b1;      // record the event
                if (wait_issue) n_st = S_COMMIT;  // enter the commit pipeline
            end
            S_COMMIT: begin
                // Event during the commit window (this cycle or buffered) aborts
                // the park and is consumed -- it is NOT lost.
                if (event_fire || pe) begin
                    n_st = S_READY;
                    n_pe = 1'b0;
                end else begin
                    n_st = S_PARKED;
                end
            end
            S_PARKED: begin
                if (event_fire || pe) begin       // wake
                    n_st = S_READY;
                    n_pe = 1'b0;
                end
            end
            default: n_st = S_READY;
        endcase
    end

    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            st <= S_READY;
            pe <= 1'b0;
        end else begin
            st <= n_st;
            pe <= n_pe;
        end
    end
endmodule

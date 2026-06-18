`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m7_assertions (
    input logic clk,
    input logic reset_n,
    input logic done,
    input logic cmpxchg_success,
    input logic cmpxchg_failure_explicit,
    input logic futex_wait_parked,
    input logic futex_wake_delivered,
    input logic timer_wait_parked,
    input logic timer_expired,
    input logic bucket_spill_preserved,
    input logic stale_address_rejected,
    input logic no_lost_wakeup,
    input logic atomic_count_exact,
    input logic typed_commit_valid,
    input lnp64_m7_sched_commit_t typed_commit,
    input lnp64_m7_state_projection_t typed_state_projection,
    input logic [31:0] atomic_word,
    input logic [31:0] atomic_count,
    input logic waiter_parked,
    input logic [31:0] wait_generation,
    input logic [31:0] address_generation,
    input logic [31:0] stale_address_generation,
    input logic wake_pending
);
    localparam logic [31:0] M7_TID = 32'd2;
    localparam logic [15:0] M7_LOC_RUNNABLE = 16'd1;
    localparam logic [15:0] M7_LOC_PARKED = 16'd3;
    localparam logic [31:0] M7_DOMAIN_BUDGET = 32'd1;
    localparam logic [31:0] M7_WAIT_COST = 32'd1;

    function automatic logic [15:0] projected_location(input logic parked);
        return parked ? M7_LOC_PARKED : M7_LOC_RUNNABLE;
    endfunction

    always_ff @(posedge clk) begin
        if (reset_n) begin
            assert (typed_state_projection.tid == M7_TID)
                else $fatal(1, "M7 typed projection TID drifted");
            assert (typed_state_projection.location == projected_location(waiter_parked))
                else $fatal(1, "M7 typed projection location drifted from waiter state");
            assert (typed_state_projection.wait_generation == wait_generation)
                else $fatal(1, "M7 typed projection wait generation drifted");
            assert (typed_state_projection.atomic_word == atomic_word)
                else $fatal(1, "M7 typed projection atomic word drifted");
            assert (typed_state_projection.atomic_count == atomic_count)
                else $fatal(1, "M7 typed projection atomic count drifted");
            assert (typed_state_projection.cmpxchg_failure_explicit == cmpxchg_failure_explicit)
                else $fatal(1, "M7 typed projection cmpxchg failure flag drifted");
            assert (typed_state_projection.address_generation == address_generation)
                else $fatal(1, "M7 typed projection address generation drifted");
            assert (typed_state_projection.stale_address_generation == stale_address_generation)
                else $fatal(1, "M7 typed projection stale address generation drifted");
            assert (typed_state_projection.domain_budget == M7_DOMAIN_BUDGET)
                else $fatal(1, "M7 typed projection domain budget drifted");
            assert (typed_state_projection.wait_cost == M7_WAIT_COST)
                else $fatal(1, "M7 typed projection wait cost drifted");
            assert (typed_state_projection.wake_pending == wake_pending)
                else $fatal(1, "M7 typed projection wake-pending bit drifted");
            assert (typed_state_projection.futex_wake_delivered == futex_wake_delivered)
                else $fatal(1, "M7 typed projection futex wake witness drifted");
            assert (typed_state_projection.timer_wake_delivered == timer_expired)
                else $fatal(1, "M7 typed projection timer wake witness drifted");
            assert (typed_state_projection.stale_address_rejected == stale_address_rejected)
                else $fatal(1, "M7 typed projection stale rejection witness drifted");

            if (typed_commit_valid) begin
                assert (typed_state_projection.op == typed_commit.op &&
                        typed_state_projection.status == typed_commit.status)
                    else $fatal(1, "M7 typed state projection transition tag drifted from commit");
                assert (typed_commit.tid == M7_TID)
                    else $fatal(1, "M7 typed commit TID drifted");
                assert (typed_commit.before_location == M7_LOC_RUNNABLE ||
                        typed_commit.before_location == M7_LOC_PARKED)
                    else $fatal(1, "M7 typed commit used unknown before location");
                assert (typed_commit.after_location == M7_LOC_RUNNABLE ||
                        typed_commit.after_location == M7_LOC_PARKED)
                    else $fatal(1, "M7 typed commit used unknown after location");
                assert (typed_commit.status == LNP64_ERR_OK ||
                        typed_commit.status == LNP64_ERR_EAGAIN ||
                        typed_commit.status == LNP64_ERR_EREVOKED)
                    else $fatal(1, "M7 typed commit used unexpected status");

                unique case (typed_commit.op)
                    LNP64_M7_COMMIT_CMPXCHG_SUCCESS,
                    LNP64_M7_COMMIT_FUTEX_WAIT,
                    LNP64_M7_COMMIT_FUTEX_WAKE,
                    LNP64_M7_COMMIT_TIMER_WAIT,
                    LNP64_M7_COMMIT_TIMER_EXPIRE,
                    LNP64_M7_COMMIT_CONSUME_WAKE:
                        assert (typed_commit.status == LNP64_ERR_OK)
                            else $fatal(1, "M7 OK-only transition emitted non-OK status");
                    LNP64_M7_COMMIT_CMPXCHG_FAIL:
                        assert (typed_commit.status == LNP64_ERR_EAGAIN)
                            else $fatal(1, "M7 cmpxchg failure did not emit EAGAIN");
                    LNP64_M7_COMMIT_REJECT_STALE_ADDRESS:
                        assert (typed_commit.status == LNP64_ERR_EREVOKED)
                            else $fatal(1, "M7 stale address rejection did not emit EREVOKED");
                    default:
                        assert (1'b0)
                            else $fatal(1, "M7 typed commit used unknown operation");
                endcase
            end

            if (done) begin
                assert (cmpxchg_success)
                    else $fatal(1, "M7 compare-exchange success was not observed");
                assert (cmpxchg_failure_explicit)
                    else $fatal(1, "M7 compare-exchange failure was not explicit");
                assert (futex_wait_parked)
                    else $fatal(1, "M7 futex wait did not park");
                assert (futex_wake_delivered)
                    else $fatal(1, "M7 futex wake was not delivered");
                assert (timer_wait_parked)
                    else $fatal(1, "M7 timer wait did not park");
                assert (timer_expired)
                    else $fatal(1, "M7 timer expiry did not wake the waiter");
                assert (bucket_spill_preserved)
                    else $fatal(1, "M7 futex bucket spill did not preserve identity");
                assert (stale_address_rejected)
                    else $fatal(1, "M7 stale futex address was not rejected");
                assert (no_lost_wakeup)
                    else $fatal(1, "M7 lost wakeup invariant failed");
                assert (atomic_count_exact)
                    else $fatal(1, "M7 atomic count was not exact");
            end
        end
    end
endmodule

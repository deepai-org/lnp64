`timescale 1ns/1ps

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
    input logic atomic_count_exact
);
    always_ff @(posedge clk) begin
        if (reset_n && done) begin
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
endmodule

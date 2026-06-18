`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m7_tb;
    logic clk;
    logic reset_n;
    logic start;
    logic [31:0] scenario_seed;
    logic done;
    logic trace_valid;
    logic [7:0] trace_code;
    logic [63:0] trace_value;
    logic cmpxchg_success;
    logic cmpxchg_failure_explicit;
    logic futex_wait_parked;
    logic futex_wake_delivered;
    logic timer_wait_parked;
    logic timer_expired;
    logic bucket_spill_preserved;
    logic stale_address_rejected;
    logic no_lost_wakeup;
    logic atomic_count_exact;

    lnp64_m7_futex_atomic dut(
        .clk(clk),
        .reset_n(reset_n),
        .start(start),
        .scenario_seed(scenario_seed),
        .done(done),
        .trace_valid(trace_valid),
        .trace_code(trace_code),
        .trace_value(trace_value),
        .cmpxchg_success(cmpxchg_success),
        .cmpxchg_failure_explicit(cmpxchg_failure_explicit),
        .futex_wait_parked(futex_wait_parked),
        .futex_wake_delivered(futex_wake_delivered),
        .timer_wait_parked(timer_wait_parked),
        .timer_expired(timer_expired),
        .bucket_spill_preserved(bucket_spill_preserved),
        .stale_address_rejected(stale_address_rejected),
        .no_lost_wakeup(no_lost_wakeup),
        .atomic_count_exact(atomic_count_exact)
    );

    lnp64_m7_assertions assertions_i(
        .clk(clk),
        .reset_n(reset_n),
        .done(done),
        .cmpxchg_success(cmpxchg_success),
        .cmpxchg_failure_explicit(cmpxchg_failure_explicit),
        .futex_wait_parked(futex_wait_parked),
        .futex_wake_delivered(futex_wake_delivered),
        .timer_wait_parked(timer_wait_parked),
        .timer_expired(timer_expired),
        .bucket_spill_preserved(bucket_spill_preserved),
        .stale_address_rejected(stale_address_rejected),
        .no_lost_wakeup(no_lost_wakeup),
        .atomic_count_exact(atomic_count_exact)
    );

    always #5 clk = ~clk;

    task automatic require(input logic condition, input string message);
        if (!condition) begin
            $fatal(1, "%s", message);
        end
    endtask

    always_ff @(posedge clk) begin
        if (trace_valid) begin
            unique case (trace_code)
                8'd1: $display(
                    "TRACE boot root_domain=%0d atomic_word=%0d",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd2: $display(
                    "TRACE cmpxchg expected=%0d desired=%0d old=%0d result=ok",
                    trace_value[63:48],
                    trace_value[47:32],
                    trace_value[31:0]
                );
                8'd3: $display(
                    "TRACE cmpxchg expected=%0d desired=%0d old=%0d errno=%0d",
                    trace_value[63:48],
                    trace_value[47:32],
                    trace_value[31:16],
                    trace_value[15:0]
                );
                8'd4: $display(
                    "TRACE futex_wait addr=%0d expected=%0d state=parked",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd5: $display(
                    "TRACE futex_wake addr=%0d woken=%0d",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd6: $display(
                    "TRACE timer_wait deadline=%0d state=parked",
                    trace_value[63:32]
                );
                8'd7: $display(
                    "TRACE timer_expire deadline=%0d woken=%0d",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd8: $display(
                    "TRACE bucket_spill bucket=%0d preserved=%0d",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd9: $display("TRACE stale_futex errno=%0d", trace_value[15:0]);
                8'd10: $display(
                    "TRACE done wakes=%0d atomics=%0d",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                default: $display("TRACE unknown code=%0d value=%0d", trace_code, trace_value);
            endcase
        end
    end

    initial begin
        if (!$value$plusargs("seed=%d", scenario_seed)) begin
            scenario_seed = 32'd0;
        end
        clk = 1'b0;
        reset_n = 1'b0;
        start = 1'b0;

        repeat (4) @(posedge clk);
        reset_n = 1'b1;
        @(posedge clk);
        start = 1'b1;
        @(posedge clk);
        start = 1'b0;

        repeat (32) @(posedge clk);
        require(done, "M7 futex/atomic slice did not complete");
        require(cmpxchg_success, "M7 compare-exchange success did not occur");
        require(cmpxchg_failure_explicit, "M7 compare-exchange failure was not explicit");
        require(futex_wait_parked, "M7 futex wait did not park");
        require(futex_wake_delivered, "M7 futex wake was not delivered");
        require(timer_wait_parked, "M7 timer wait did not park");
        require(timer_expired, "M7 timer expiry did not wake the waiter");
        require(bucket_spill_preserved, "M7 bucket spill did not preserve identity");
        require(stale_address_rejected, "M7 stale futex address was not rejected");
        require(no_lost_wakeup, "M7 no-lost-wakeup invariant did not hold");
        require(atomic_count_exact, "M7 atomic count was not exact");
        $display("LNP64-RTL-M7 PASS");
        $finish;
    end
endmodule

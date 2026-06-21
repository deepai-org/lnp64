`timescale 1ns/1ps

// "Does what it says" formal harness for the lnp64_watchdog shell.
//
// Contract: once the watchdog degrades it stays degraded (fail-safe latch, never
// silently recovers), and any fault it raises is the DEGRADED fault attributed
// to the watchdog engine. tile_id/inject_stuck/fault_ready are free inputs, so
// the proof holds for every stimulus on the real RTL.
import lnp64_pkg::*;

module lnp64_watchdog_formal (
    input logic clk,
    input logic reset_n,
    input logic [31:0] tile_id,
    input logic inject_stuck,
    input logic fault_ready
);
    logic degraded;
    logic fault_valid;
    lnp64_fault_t fault;

    lnp64_watchdog dut (
        .clk(clk),
        .reset_n(reset_n),
        .tile_id(tile_id),
        .inject_stuck(inject_stuck),
        .degraded(degraded),
        .fault_valid(fault_valid),
        .fault_ready(fault_ready),
        .fault(fault)
    );

    // Power-on reset discipline.
    reg init_done = 1'b0;
    always @(posedge clk) init_done <= 1'b1;
    always @(posedge clk) begin
        if (!init_done) assume (!reset_n);
    end

    // Track the previous degraded value to express monotonicity.
    reg prev_degraded = 1'b0;
    always @(posedge clk) prev_degraded <= degraded;

    always @(posedge clk) begin
        if (reset_n) begin
            // Fail-safe latch: degraded never returns to 0 once set.
            a_degraded_sticky:
                assert (!(prev_degraded && !degraded));

            // Any raised fault is the watchdog's DEGRADED fault.
            if (fault_valid) begin
                a_fault_is_degraded:
                    assert (fault.fault_code == LNP64_STATUS_DEGRADED
                            && fault.source == LNP64_ENGINE_WATCHDOG);
            end
        end
    end
endmodule

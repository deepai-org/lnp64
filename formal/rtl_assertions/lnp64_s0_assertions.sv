`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_s0_assertions (
    input logic clk,
    input logic reset_n,
    input logic boot_stable,
    input logic pid1_exactly_one_location,
    input logic pid1_completed,
    input logic env_get_ok,
    input logic unsupported_failed_closed,
    input logic stub_failed_closed,
    input logic event_woke_thread,
    input logic structured_fault_seen,
    input logic watchdog_degraded_seen,
    input logic no_raw_authority_visible,
    input logic coherence_paths_live
);
    always_ff @(posedge clk) begin
        if (reset_n) begin
            assert (no_raw_authority_visible)
                else $fatal(1, "software-visible raw authority path became visible");
            if (boot_stable && !pid1_completed) begin
                assert (pid1_exactly_one_location)
                    else $fatal(1, "PID 1 lost exactly-one scheduler location invariant");
            end
            if (pid1_completed) begin
                assert (env_get_ok)
                    else $fatal(1, "PID 1 completed without ENV_GET feature evidence");
                assert (unsupported_failed_closed)
                    else $fatal(1, "unsupported native command did not fail closed");
                assert (stub_failed_closed)
                    else $fatal(1, "stubbed resource command did not fail closed");
                assert (coherence_paths_live)
                    else $fatal(1, "coherence/TLB/DMA visibility stub paths were not live");
            end
            if (event_woke_thread) begin
                assert (boot_stable)
                    else $fatal(1, "event wake observed before valid boot state");
            end
            if (structured_fault_seen || watchdog_degraded_seen) begin
                assert (boot_stable)
                    else $fatal(1, "fault/degraded event observed before valid boot state");
            end
        end
    end
endmodule

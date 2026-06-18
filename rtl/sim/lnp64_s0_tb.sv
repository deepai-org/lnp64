`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_s0_tb;
    logic clk;
    logic reset_n;
    logic force_boot_fault;
    logic sim_event_inject;
    logic sim_fault_inject;
    logic sim_watchdog_inject;

    logic boot_stable;
    logic pid1_exactly_one_location;
    logic pid1_completed;
    logic [31:0] retired_count;
    logic env_get_ok;
    logic sram_ldst_ok;
    logic unsupported_failed_closed;
    logic stub_failed_closed;
    logic uart_seen;
    logic [7:0] uart_byte_seen;
    logic event_woke_thread;
    logic structured_fault_seen;
    logic watchdog_degraded_seen;
    logic no_raw_authority_visible;
    logic coherence_paths_live;

    lnp64_top dut(
        .clk(clk),
        .reset_n(reset_n),
        .force_boot_fault(force_boot_fault),
        .sim_event_inject(sim_event_inject),
        .sim_fault_inject(sim_fault_inject),
        .sim_watchdog_inject(sim_watchdog_inject),
        .boot_stable(boot_stable),
        .pid1_exactly_one_location(pid1_exactly_one_location),
        .pid1_completed(pid1_completed),
        .retired_count(retired_count),
        .env_get_ok(env_get_ok),
        .sram_ldst_ok(sram_ldst_ok),
        .unsupported_failed_closed(unsupported_failed_closed),
        .stub_failed_closed(stub_failed_closed),
        .uart_seen(uart_seen),
        .uart_byte_seen(uart_byte_seen),
        .event_woke_thread(event_woke_thread),
        .structured_fault_seen(structured_fault_seen),
        .watchdog_degraded_seen(watchdog_degraded_seen),
        .no_raw_authority_visible(no_raw_authority_visible),
        .coherence_paths_live(coherence_paths_live)
    );

    lnp64_s0_assertions assertions_i(
        .clk(clk),
        .reset_n(reset_n),
        .boot_stable(boot_stable),
        .pid1_exactly_one_location(pid1_exactly_one_location),
        .pid1_completed(pid1_completed),
        .env_get_ok(env_get_ok),
        .unsupported_failed_closed(unsupported_failed_closed),
        .stub_failed_closed(stub_failed_closed),
        .event_woke_thread(event_woke_thread),
        .structured_fault_seen(structured_fault_seen),
        .watchdog_degraded_seen(watchdog_degraded_seen),
        .no_raw_authority_visible(no_raw_authority_visible),
        .coherence_paths_live(coherence_paths_live)
    );

    always #5 clk = ~clk;

    task automatic require(input logic condition, input string message);
        if (!condition) begin
            $fatal(1, "%s", message);
        end
    endtask

    initial begin
        clk = 1'b0;
        reset_n = 1'b0;
        force_boot_fault = 1'b0;
        sim_event_inject = 1'b0;
        sim_fault_inject = 1'b0;
        sim_watchdog_inject = 1'b0;

        repeat (4) @(posedge clk);
        reset_n = 1'b1;

        wait (boot_stable);
        repeat (2) @(posedge clk);
        require(uart_seen && uart_byte_seen == 8'h53, "UART boot/status byte was not observed");
        require(no_raw_authority_visible, "raw authority path was visible after boot");

        repeat (8) @(posedge clk);
        sim_event_inject = 1'b1;
        repeat (24) @(posedge clk);
        sim_event_inject = 1'b0;

        sim_fault_inject = 1'b1;
        @(posedge clk);
        sim_fault_inject = 1'b0;

        sim_watchdog_inject = 1'b1;
        repeat (8) @(posedge clk);
        sim_watchdog_inject = 1'b0;

        repeat (80) @(posedge clk);

        require(pid1_completed, "PID 1 did not complete the S0 ROM");
        require(retired_count >= 32'd13, "PID 1 retired too few S0 instructions");
        require(env_get_ok, "ENV_GET did not report expected S0 feature bits");
        require(sram_ldst_ok, "SRAM LD/ST path did not roundtrip the ALU value");
        require(unsupported_failed_closed, "unsupported opcode did not return canonical ENOTSUP");
        require(stub_failed_closed, "stub resource operation did not fail closed");
        require(event_woke_thread, "synthetic event did not wake or mark the parked thread");
        require(structured_fault_seen, "synthetic stub-engine fault did not emit a structured fault");
        require(watchdog_degraded_seen, "watchdog-injected stuck command did not reach degraded/fault state");
        require(no_raw_authority_visible, "raw physical interrupt/address/DMA/device authority became visible");
        require(coherence_paths_live, "coherence/TLB/DMA visibility stub paths were not live");

        $display("LNP64-RTL-S0 PASS retired=%0d features=0x%016h", retired_count, dut.env_features_seen);
        $finish;
    end
endmodule

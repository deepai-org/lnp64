`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_top_program_tb;
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
    logic multicore_no_duplicate_tid;
    logic tile_reset_stable_all;
    logic tile1_observable_idle;
    logic cross_tile_wake_one;
    logic tile_fault_isolated;
    logic [31:0] topology_tile_count_seen;
    logic [63:0] topology_enabled_tile_mask_seen;
    logic [31:0] topology_coherence_domain_seen;
    logic [31:0] topology_active_window_base_seen;
    logic [31:0] topology_active_window_count_seen;

    lnp64_top #(
        .CORE_TILE_COUNT(2)
    ) dut(
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
        .coherence_paths_live(coherence_paths_live),
        .multicore_no_duplicate_tid(multicore_no_duplicate_tid),
        .tile_reset_stable_all(tile_reset_stable_all),
        .tile1_observable_idle(tile1_observable_idle),
        .cross_tile_wake_one(cross_tile_wake_one),
        .tile_fault_isolated(tile_fault_isolated),
        .topology_tile_count_seen(topology_tile_count_seen),
        .topology_enabled_tile_mask_seen(topology_enabled_tile_mask_seen),
        .topology_coherence_domain_seen(topology_coherence_domain_seen),
        .topology_active_window_base_seen(topology_active_window_base_seen),
        .topology_active_window_count_seen(topology_active_window_count_seen)
    );

    always #5 clk = ~clk;

    task automatic require(input logic condition, input string message);
        if (!condition) begin
            $fatal(1, "%s", message);
        end
    endtask

    always @(posedge clk) begin
        #1;
        if (dut.retire_submit_valid_vec[0]) begin
            $display(
                "RTL_RETIRE {\"pc\":%0d,\"opcode\":%0d}",
                dut.retire_submit_record_vec[0].pc,
                dut.core_tiles[0].core_i.program_rom[dut.retire_submit_record_vec[0].pc][31:24]
            );
        end
    end

    initial begin
        clk = 1'b0;
        reset_n = 1'b0;
        force_boot_fault = 1'b0;
        sim_event_inject = 1'b0;
        sim_fault_inject = 1'b0;
        sim_watchdog_inject = 1'b0;

        repeat (4) @(posedge clk);
        reset_n = 1'b1;

        repeat (200) begin
            @(posedge clk);
            if (pid1_completed) begin
                break;
            end
        end

        require(boot_stable, "top-level program boot did not stabilize");
        require(pid1_completed, "top-level program did not reach EXIT");
        require(tile_reset_stable_all, "top-level program did not reset both tiles");
        require(tile1_observable_idle, "tile 1 was not observable/idled during top-level program");
        require(multicore_no_duplicate_tid, "top-level program duplicated PID 1 across tiles");
        require(retired_count == 32'd6, "top-level program retired an unexpected instruction count");
        require(dut.core_tiles[0].core_i.gpr[3] == 64'd12, "ADD result did not reach r3");
        require(dut.core_tiles[0].core_i.gpr[4] == 64'd12, "LD result did not reach r4");
        require(dut.core_tiles[0].core_i.sram[0] == 64'd12, "ST result did not reach SRAM[0]");

        $display(
            "RTL_FINAL {\"retired\":%0d,\"exit_reg\":%0d,\"r3\":%0d,\"r4\":%0d,\"mem0\":%0d}",
            retired_count,
            dut.core_tiles[0].core_i.gpr[4],
            dut.core_tiles[0].core_i.gpr[3],
            dut.core_tiles[0].core_i.gpr[4],
            dut.core_tiles[0].core_i.sram[0]
        );
        $display("LNP64-RTL-TOP-PROGRAM PASS");
        $finish;
    end
endmodule

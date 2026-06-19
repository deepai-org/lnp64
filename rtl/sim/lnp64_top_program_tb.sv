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
    int unsigned max_cycles;
    logic [7:0] retire_opcode;
    logic [4:0] retire_raw_result_reg;
    logic retire_result_valid;
    logic [4:0] retire_result_reg;

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

    function automatic logic flat_result_valid(input logic [7:0] opcode);
        unique case (opcode)
            8'h00, 8'h1b, 8'h1c, 8'h1f, 8'h20, 8'h21, 8'h22, 8'h23,
            8'h24, 8'h25, 8'h26, 8'h27, 8'h28, 8'h2a, 8'h33, 8'h34,
            8'h35, 8'h37, 8'h39, 8'h3a, 8'h49, 8'h55, 8'h61, 8'h63,
            8'h64, 8'h65, 8'h68, 8'hcb, 8'hcc, 8'hcd: flat_result_valid = 1'b0;
            default: flat_result_valid = 1'b1;
        endcase
    endfunction

    always_comb begin
        retire_opcode = dut.core_tiles[0].core_i.program_rom[dut.retire_submit_record_vec[0].pc][31:24];
        retire_raw_result_reg =
            dut.core_tiles[0].core_i.program_rom[dut.retire_submit_record_vec[0].pc][23:19];
        retire_result_valid = flat_result_valid(retire_opcode);
        retire_result_reg = retire_opcode == 8'h57 ? 5'd1 : retire_raw_result_reg;
    end

    task automatic require(input logic condition, input string message);
        if (!condition) begin
            $fatal(1, "%s", message);
        end
    endtask

    always @(posedge clk) begin
        #1;
        if (dut.retire_submit_valid_vec[0]) begin
            $display(
                "RTL_RETIRE {\"pc\":%0d,\"opcode\":%0d,\"tile_id\":%0d,\"pid\":%0d,\"tid\":%0d,\"domain_id\":%0d,\"domain_gen\":%0d,\"action\":%0d,\"result_valid\":%0d,\"result_reg\":%0d,\"result_value\":%0d,\"errno\":%0d,\"status\":%0d}",
                dut.retire_submit_record_vec[0].pc,
                retire_opcode,
                dut.retire_submit_record_vec[0].tile_id,
                dut.retire_submit_record_vec[0].pid,
                dut.retire_submit_record_vec[0].tid,
                32'd1,
                32'd1,
                dut.retire_submit_record_vec[0].action,
                retire_result_valid,
                retire_result_valid ? retire_result_reg : 5'd0,
                retire_result_valid ? dut.core_tiles[0].core_i.gpr[retire_result_reg] : 64'd0,
                dut.core_tiles[0].core_i.errno_reg,
                dut.core_tiles[0].core_i.errno_reg == LNP64_ERR_OK ? 16'd0 : 16'd1
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

        if (!$value$plusargs("lnp64_max_cycles=%d", max_cycles)) begin
            max_cycles = 200;
        end

        repeat (max_cycles) begin
            @(posedge clk);
            if (pid1_completed) begin
                break;
            end
        end

        require(boot_stable, "top-level program boot did not stabilize");
        if (!pid1_completed) begin
            $fatal(1, "top-level program did not reach EXIT within %0d cycles", max_cycles);
        end
        require(tile_reset_stable_all, "top-level program did not reset both tiles");
        require(tile1_observable_idle, "tile 1 was not observable/idled during top-level program");
        require(multicore_no_duplicate_tid, "top-level program duplicated PID 1 across tiles");
        require(retired_count > 32'd0, "top-level program retired no instructions");

        $display(
            "RTL_FINAL {\"retired\":%0d,\"exit_reg\":%0d,\"r3\":%0d,\"r4\":%0d,\"r5\":%0d,\"env_page\":%0d,\"mem0\":%0d,\"errno\":%0d}",
            retired_count,
            dut.core_tiles[0].core_i.gpr[
                dut.core_tiles[0].core_i.program_rom[dut.retire_submit_record_vec[0].pc][23:19]
            ],
            dut.core_tiles[0].core_i.gpr[3],
            dut.core_tiles[0].core_i.gpr[4],
            dut.core_tiles[0].core_i.gpr[5],
            dut.core_tiles[0].core_i.gpr[6],
            dut.core_tiles[0].core_i.sram[0],
            dut.core_tiles[0].core_i.errno_reg
        );
        $display("LNP64-RTL-TOP-PROGRAM PASS");
        $finish;
    end
endmodule

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
    logic stress_boot_stable;
    logic stress_tile_reset_stable_all;
    logic [31:0] stress_topology_tile_count_seen;
    logic object_route_seen;
    logic unsupported_route_seen;

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

    lnp64_top #(
        .CORE_TILE_COUNT(4),
        .CORE_THREAD_CONTEXT_COUNT(4)
    ) stress_dut(
        .clk(clk),
        .reset_n(reset_n),
        .force_boot_fault(force_boot_fault),
        .sim_event_inject(sim_event_inject),
        .sim_fault_inject(sim_fault_inject),
        .sim_watchdog_inject(sim_watchdog_inject),
        .boot_stable(stress_boot_stable),
        .pid1_exactly_one_location(),
        .pid1_completed(),
        .retired_count(),
        .env_get_ok(),
        .sram_ldst_ok(),
        .unsupported_failed_closed(),
        .stub_failed_closed(),
        .uart_seen(),
        .uart_byte_seen(),
        .event_woke_thread(),
        .structured_fault_seen(),
        .watchdog_degraded_seen(),
        .no_raw_authority_visible(),
        .coherence_paths_live(),
        .multicore_no_duplicate_tid(),
        .tile_reset_stable_all(stress_tile_reset_stable_all),
        .tile1_observable_idle(),
        .cross_tile_wake_one(),
        .tile_fault_isolated(),
        .topology_tile_count_seen(stress_topology_tile_count_seen),
        .topology_enabled_tile_mask_seen(),
        .topology_coherence_domain_seen(),
        .topology_active_window_base_seen(),
        .topology_active_window_count_seen()
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
        .coherence_paths_live(coherence_paths_live),
        .multicore_no_duplicate_tid(multicore_no_duplicate_tid),
        .tile_reset_stable_all(tile_reset_stable_all),
        .tile_fault_isolated(tile_fault_isolated),
        .cross_tile_wake_one(cross_tile_wake_one)
    );

    always #5 clk = ~clk;

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            object_route_seen <= 1'b0;
            unsupported_route_seen <= 1'b0;
        end else begin
            if (dut.object_cmd_valid && dut.object_cmd_ready && dut.object_cmd.opcode == LNP64_OP_OBJECT_CTL) begin
                object_route_seen <= 1'b1;
            end
            if (dut.engine_router_i.unsupported_cmd_valid &&
                dut.engine_router_i.unsupported_cmd_ready &&
                dut.core_cmd.opcode == LNP64_OP_UNSUPPORTED) begin
                unsupported_route_seen <= 1'b1;
            end
        end
    end

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
        force_boot_fault = 1'b1;
        reset_n = 1'b1;

        wait (dut.boot_fault_valid);
        repeat (2) @(posedge clk);
        require(!boot_stable, "forced boot fault created a stable boot state");
        require(!dut.release_core, "forced boot fault released the core");
        require(dut.boot_fault.fault_code == LNP64_ERR_EFAULT, "forced boot fault used the wrong canonical fault code");
        require(
            dut.boot_fault.source == 16'hb007 && dut.boot_fault.detail == 64'hb007_fa11,
            "forced boot fault did not emit measured/audited boot fault"
        );

        reset_n = 1'b0;
        force_boot_fault = 1'b0;
        repeat (4) @(posedge clk);
        reset_n = 1'b1;

        wait (boot_stable);
        repeat (2) @(posedge clk);
        require(uart_seen && uart_byte_seen == 8'h53, "UART boot/status byte was not observed");
        require(no_raw_authority_visible, "raw authority path was visible after boot");
        require(tile_reset_stable_all, "not every enabled tile reached reset-stable");
        require(tile1_observable_idle, "tile 1 was not observable, schedulable, and idle");
        require(multicore_no_duplicate_tid, "one TID was issued to two tiles");

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
        require(dut.retire_submit_record_vec[0].tile_id == 32'd0, "tile 0 did not run PID 1");
        require(retired_count >= 32'd13, "PID 1 retired too few S0 instructions");
        require(env_get_ok, "ENV_GET did not report expected S0 feature bits");
        require(topology_tile_count_seen == 32'd2, "ENV_GET did not report the two-tile topology");
        require(topology_enabled_tile_mask_seen == 64'h3, "ENV_GET did not report the enabled tile mask");
        require(topology_coherence_domain_seen == 32'd1, "ENV_GET did not report the coherence domain id");
        require(topology_active_window_base_seen == 32'd0 && topology_active_window_count_seen == 32'd2,
            "ENV_GET did not report the active-window shape");
        require(sram_ldst_ok, "SRAM LD/ST path did not roundtrip the ALU value");
        require(object_route_seen, "OBJECT_CTL did not route through top-level object engine lane");
        require(unsupported_failed_closed, "unsupported opcode did not return canonical ENOTSUP");
        require(unsupported_route_seen, "unsupported command did not route through default fail-closed lane");
        require(stub_failed_closed, "stub resource operation did not fail closed");
        require(event_woke_thread, "synthetic event did not wake or mark the parked thread");
        require(cross_tile_wake_one, "cross-tile wake did not produce exactly one wake");
        require(structured_fault_seen, "synthetic stub-engine fault did not emit a structured fault");
        require(tile_fault_isolated, "tile-local fault corrupted another tile's scheduler state");
        require(watchdog_degraded_seen, "watchdog-injected stuck command did not reach degraded/fault state");
        require(no_raw_authority_visible, "raw physical interrupt/address/DMA/device authority became visible");
        require(coherence_paths_live, "coherence/TLB/DMA visibility stub paths were not live");
        require(stress_boot_stable && stress_tile_reset_stable_all && stress_topology_tile_count_seen == 32'd4,
            "4-tile stress configuration did not reach reset-stable");

        $display(
            "TTRACE {\"record\":\"lnp64_retire_submit_t\",\"op_id\":%0d,\"pid\":%0d,\"tid\":%0d,\"tile_id\":%0d,\"domain_id\":%0d,\"domain_gen\":%0d,\"pc\":%0d,\"opcode\":%0d,\"arch_opcode\":%0d,\"action\":%0d,\"operand_rd\":%0d,\"operand_rs1\":%0d,\"operand_rs2\":%0d,\"operand_rs3\":%0d,\"operand_imm\":%0d,\"result_valid\":%0d,\"result_reg\":%0d,\"result_value\":%0d,\"errno\":%0d,\"status\":%0d,\"latency_class\":%0d,\"wait_source\":%0d,\"event_id\":%0d,\"fault_id\":%0d}",
            dut.retire_submit_record_vec[0].op_id,
            dut.retire_submit_record_vec[0].pid,
            dut.retire_submit_record_vec[0].tid,
            dut.retire_submit_record_vec[0].tile_id,
            dut.retire_submit_record_vec[0].domain_id,
            dut.retire_submit_record_vec[0].domain_gen,
            dut.retire_submit_record_vec[0].pc,
            dut.retire_submit_record_vec[0].opcode,
            dut.retire_submit_record_vec[0].arch_opcode,
            dut.retire_submit_record_vec[0].action,
            dut.retire_submit_record_vec[0].operand_rd,
            dut.retire_submit_record_vec[0].operand_rs1,
            dut.retire_submit_record_vec[0].operand_rs2,
            dut.retire_submit_record_vec[0].operand_rs3,
            dut.retire_submit_record_vec[0].operand_imm,
            dut.retire_submit_record_vec[0].result_valid,
            dut.retire_submit_record_vec[0].result_reg,
            dut.retire_submit_record_vec[0].result_value,
            dut.retire_submit_record_vec[0].errno,
            dut.retire_submit_record_vec[0].status,
            dut.retire_submit_record_vec[0].latency_class,
            dut.retire_submit_record_vec[0].wait_source,
            dut.retire_submit_record_vec[0].event_id,
            dut.retire_submit_record_vec[0].fault_id
        );
        $display(
            "TTRACE {\"record\":\"lnp64_thread_sched_t\",\"pid\":%0d,\"tid\":%0d,\"tile_id\":%0d,\"domain_id\":%0d,\"domain_gen\":%0d,\"state\":%0d,\"latency_class\":%0d,\"wait_generation\":%0d,\"weight_index\":%0d,\"virtual_deadline\":%0d,\"active_location\":%0d}",
            dut.park_submit_record_vec[0].pid,
            dut.park_submit_record_vec[0].tid,
            dut.park_submit_record_vec[0].tile_id,
            dut.park_submit_record_vec[0].domain_id,
            dut.park_submit_record_vec[0].domain_gen,
            dut.park_submit_record_vec[0].state,
            dut.park_submit_record_vec[0].latency_class,
            dut.park_submit_record_vec[0].wait_generation,
            dut.park_submit_record_vec[0].weight_index,
            dut.park_submit_record_vec[0].virtual_deadline,
            dut.park_submit_record_vec[0].active_location
        );
        $display(
            "TTRACE {\"record\":\"lnp64_event_t\",\"event_id\":%0d,\"tile_id\":%0d,\"op_id\":%0d,\"pid\":%0d,\"tid\":%0d,\"domain_id\":%0d,\"domain_gen\":%0d,\"event_mask\":%0d,\"source\":%0d,\"status\":%0d}",
            dut.event_record.event_id,
            dut.event_record.tile_id,
            dut.event_record.op_id,
            dut.event_record.pid,
            dut.event_record.tid,
            dut.event_record.domain_id,
            dut.event_record.domain_gen,
            dut.event_record.event_mask,
            dut.event_record.source,
            dut.event_record.status
        );
        $display(
            "TTRACE {\"record\":\"lnp64_fault_t\",\"fault_id\":%0d,\"tile_id\":%0d,\"op_id\":%0d,\"pid\":%0d,\"tid\":%0d,\"domain_id\":%0d,\"domain_gen\":%0d,\"fault_code\":%0d,\"source\":%0d,\"detail\":%0d}",
            dut.fault_record.fault_id,
            dut.fault_record.tile_id,
            dut.fault_record.op_id,
            dut.fault_record.pid,
            dut.fault_record.tid,
            dut.fault_record.domain_id,
            dut.fault_record.domain_gen,
            dut.fault_record.fault_code,
            dut.fault_record.source,
            dut.fault_record.detail
        );
        $display(
            "TTRACE {\"record\":\"lnp64_tlb_cache_invalidate_t\",\"invalidate_id\":1,\"tile_id\":0,\"domain_id\":1,\"domain_generation\":1,\"virtual_base\":0,\"byte_len\":4096,\"scope\":1}"
        );
        $display(
            "TTRACE {\"record\":\"lnp64_coherence_txn_t\",\"txn_id\":1,\"tile_id\":0,\"domain_id\":1,\"domain_generation\":1,\"address\":0,\"byte_len\":8,\"memory_type\":1,\"ordering\":1}"
        );
        $display(
            "TTRACE {\"record\":\"lnp64_trace_t\",\"trace_id\":1,\"tile_id\":0,\"domain_id\":1,\"domain_gen\":1,\"source\":%0d,\"severity\":1,\"counter_value\":%0d,\"payload_hash\":%0d}",
            LNP64_ENGINE_WATCHDOG,
            retired_count,
            topology_enabled_tile_mask_seen[31:0]
        );

        $display("LNP64-RTL-S0 PASS retired=%0d features=0x%016h", retired_count, dut.env_features_seen);
        $finish;
    end
endmodule

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
    logic [31:0] retire_instr;
    lnp64_decode_t retire_dec;
    lnp64_m1_state_projection_t sampled_m1_pre_state;
    logic [4:0] retire_raw_result_reg;
    logic retire_result_valid;
    logic [4:0] retire_result_reg;
    logic [31:0] retire_operand_imm;
    logic [63:0] final_mem_checksum;
    logic unsupported_retired_seen;

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

    lnp64_decode retire_decode_i(
        .instr(retire_instr),
        .dec(retire_dec)
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

    function automatic logic [31:0] flat_operand_imm(
        input logic [7:0] opcode,
        input logic [31:0] instr,
        input logic [31:0] literal
    );
        unique case (opcode)
            8'h03, 8'h04, 8'hd0: flat_operand_imm = literal;
            8'h01: flat_operand_imm = {{16{instr[15]}}, instr[15:0]};
            8'h20, 8'h21, 8'h22, 8'h23, 8'h24, 8'h25, 8'h26, 8'h27:
                flat_operand_imm = {{8{instr[23]}}, instr[23:0]};
            default: flat_operand_imm = {{18{instr[13]}}, instr[13:0]};
        endcase
    endfunction

    function automatic logic [63:0] rtl_memory_checksum;
        logic [63:0] checksum;
        begin
            checksum = 64'h6c6e_7036_345f_7331;
            for (int word_idx = 0; word_idx < 96; word_idx = word_idx + 1) begin
                checksum = {checksum[56:0], checksum[63:57]} ^
                    {checksum[2:0], checksum[63:3]} ^
                    dut.core_tiles[0].core_i.sram[word_idx] ^
                    {57'd0, word_idx[6:0]};
            end
            rtl_memory_checksum = checksum;
        end
    endfunction

    always_comb begin
        retire_instr = dut.core_tiles[0].core_i.program_rom[dut.retire_submit_record_vec[0].pc];
        retire_opcode = retire_instr[31:24];
        retire_raw_result_reg =
            retire_instr[23:19];
        retire_result_valid = flat_result_valid(retire_opcode);
        retire_result_reg = retire_opcode == 8'h57 ? 5'd1 : retire_raw_result_reg;
        retire_operand_imm = flat_operand_imm(
            retire_opcode,
            retire_instr,
            dut.core_tiles[0].core_i.program_rom[dut.retire_submit_record_vec[0].pc + 32'd1]
        );
        final_mem_checksum = rtl_memory_checksum();
    end

    task automatic require(input logic condition, input string message);
        if (!condition) begin
            $fatal(1, "%s", message);
        end
    endtask

    task automatic display_m1_top_state(
        input string prefix,
        input lnp64_m1_state_projection_t state,
        input logic [31:0] pc,
        input logic [31:0] tile_id
    );
        $display(
            "%s {\"record\":\"m1_state_projection\",\"op\":%0d,\"status\":%0d,\"object_gen\":%0d,\"created_object_created\":%0d,\"created_object_gen\":%0d,\"root_object_id\":%0d,\"root_generation\":%0d,\"root_domain_id\":%0d,\"root_lineage_epoch\":%0d,\"root_sealed\":%0d,\"root_rights\":%0d,\"consumer_object_id\":%0d,\"consumer_generation\":%0d,\"consumer_domain_id\":%0d,\"consumer_lineage_epoch\":%0d,\"consumer_sealed\":%0d,\"consumer_rights\":%0d,\"sent_valid\":%0d,\"sent_object_id\":%0d,\"sent_generation\":%0d,\"sent_domain_id\":%0d,\"sent_lineage_epoch\":%0d,\"sent_sealed\":%0d,\"sent_rights\":%0d,\"minted_valid\":%0d,\"minted_object_id\":%0d,\"minted_generation\":%0d,\"minted_domain_id\":%0d,\"minted_lineage_epoch\":%0d,\"minted_sealed\":%0d,\"minted_rights\":%0d,\"wake_pending\":%0d,\"transfer_valid\":%0d,\"stale_rejected\":%0d,\"revoked_rejected\":%0d,\"failed_no_authority\":%0d,\"full_was_explicit\":%0d,\"has_revoked_generation\":%0d,\"revoked_generation\":%0d,\"pc\":%0d,\"tile_id\":%0d}",
            prefix,
            state.op,
            state.status,
            state.object_gen,
            state.created_object_created,
            state.created_object_gen,
            state.root_object_id,
            state.root_generation,
            state.root_domain_id,
            state.root_lineage_epoch,
            state.root_sealed,
            state.root_rights,
            state.consumer_object_id,
            state.consumer_generation,
            state.consumer_domain_id,
            state.consumer_lineage_epoch,
            state.consumer_sealed,
            state.consumer_rights,
            state.sent_valid,
            state.sent_object_id,
            state.sent_generation,
            state.sent_domain_id,
            state.sent_lineage_epoch,
            state.sent_sealed,
            state.sent_rights,
            state.minted_valid,
            state.minted_object_id,
            state.minted_generation,
            state.minted_domain_id,
            state.minted_lineage_epoch,
            state.minted_sealed,
            state.minted_rights,
            state.wake_pending,
            state.transfer_valid,
            state.stale_rejected,
            state.revoked_rejected,
            state.failed_no_authority,
            state.full_was_explicit,
            state.has_revoked_generation,
            state.revoked_generation,
            pc,
            tile_id
        );
    endtask

    task automatic display_m1_top_state_bits(
        input string prefix,
        input lnp64_m1_state_projection_t state,
        input logic [31:0] pc,
        input logic [31:0] tile_id
    );
        $display(
            "%s {\"record\":\"m1_state_projection_bits\",\"width\":%0d,\"bits\":\"%0h\",\"pc\":%0d,\"tile_id\":%0d}",
            prefix,
            $bits(lnp64_m1_state_projection_t),
            state,
            pc,
            tile_id
        );
    endtask

    always @(posedge clk) begin
        sampled_m1_pre_state = dut.m1_pre_state_projection_vec[0];
        #1;
        if (dut.retire_submit_valid_vec[0]) begin
            if (retire_opcode == 8'hff) begin
                unsupported_retired_seen = 1'b1;
            end
            $display(
                "RTL_RETIRE {\"pc\":%0d,\"opcode\":%0d,\"arch_opcode\":%0d,\"tile_id\":%0d,\"pid\":%0d,\"tid\":%0d,\"domain_id\":%0d,\"domain_gen\":%0d,\"action\":%0d,\"operand_rd\":%0d,\"operand_rs1\":%0d,\"operand_rs2\":%0d,\"operand_rs3\":%0d,\"operand_imm\":%0d,\"result_valid\":%0d,\"result_reg\":%0d,\"result_value\":%0d,\"errno\":%0d,\"status\":%0d,\"event_id\":%0d,\"fault_id\":%0d}",
                dut.retire_submit_record_vec[0].pc,
                retire_opcode,
                retire_dec.opcode,
                dut.retire_submit_record_vec[0].tile_id,
                dut.retire_submit_record_vec[0].pid,
                dut.retire_submit_record_vec[0].tid,
                32'd1,
                32'd1,
                dut.retire_submit_record_vec[0].action,
                retire_instr[23:19],
                retire_instr[18:14],
                retire_instr[13:9],
                retire_instr[8:4],
                retire_operand_imm,
                retire_result_valid,
                retire_result_valid ? retire_result_reg : 5'd0,
                retire_result_valid ? dut.core_tiles[0].core_i.gpr[retire_result_reg] : 64'd0,
                dut.core_tiles[0].core_i.errno_reg,
                dut.core_tiles[0].core_i.errno_reg == LNP64_ERR_OK ? 16'd0 : 16'd1,
                32'd0,
                32'd0
            );
            if (dut.m1_commit_valid_vec[0]) begin
                $display(
                    "RTL_M1_TOP_COMMIT {\"record\":\"m1_cap_commit\",\"op\":%0d,\"object_id\":%0d,\"object_gen\":%0d,\"fdr_gen\":%0d,\"domain_id\":%0d,\"domain_gen\":%0d,\"rights_mask\":%0d,\"lineage_epoch\":%0d,\"sealed\":%0d,\"status\":%0d,\"pc\":%0d,\"tile_id\":%0d}",
                    dut.m1_commit_vec[0].op,
                    dut.m1_commit_vec[0].object_id,
                    dut.m1_commit_vec[0].object_gen,
                    dut.m1_commit_vec[0].fdr_gen,
                    dut.m1_commit_vec[0].domain_id,
                    dut.m1_commit_vec[0].domain_gen,
                    dut.m1_commit_vec[0].rights_mask,
                    dut.m1_commit_vec[0].lineage_epoch,
                    dut.m1_commit_vec[0].sealed,
                    dut.m1_commit_vec[0].status,
                    dut.retire_submit_record_vec[0].pc,
                    dut.retire_submit_record_vec[0].tile_id
                );
                $display(
                    "RTL_M1_TOP_COMMIT_BITS {\"record\":\"m1_cap_commit_bits\",\"width\":%0d,\"bits\":\"%0h\",\"pc\":%0d,\"tile_id\":%0d}",
                    $bits(lnp64_m1_cap_commit_t),
                    dut.m1_commit_vec[0],
                    dut.retire_submit_record_vec[0].pc,
                    dut.retire_submit_record_vec[0].tile_id
                );
                display_m1_top_state(
                    "RTL_M1_TOP_PRE_STATE",
                    sampled_m1_pre_state,
                    dut.retire_submit_record_vec[0].pc,
                    dut.retire_submit_record_vec[0].tile_id
                );
                display_m1_top_state_bits(
                    "RTL_M1_TOP_PRE_STATE_BITS",
                    sampled_m1_pre_state,
                    dut.retire_submit_record_vec[0].pc,
                    dut.retire_submit_record_vec[0].tile_id
                );
                display_m1_top_state(
                    "RTL_M1_TOP_STATE",
                    dut.m1_state_projection_vec[0],
                    dut.retire_submit_record_vec[0].pc,
                    dut.retire_submit_record_vec[0].tile_id
                );
                display_m1_top_state_bits(
                    "RTL_M1_TOP_STATE_BITS",
                    dut.m1_state_projection_vec[0],
                    dut.retire_submit_record_vec[0].pc,
                    dut.retire_submit_record_vec[0].tile_id
                );
            end
        end
    end

    initial begin
        clk = 1'b0;
        reset_n = 1'b0;
        force_boot_fault = 1'b0;
        sim_event_inject = 1'b0;
        sim_fault_inject = 1'b0;
        sim_watchdog_inject = 1'b0;
        unsupported_retired_seen = 1'b0;

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
        if (unsupported_retired_seen || retire_opcode == 8'hff) begin
            require(unsupported_failed_closed, "unsupported top-level program did not fail closed canonically");
        end

        $display(
            "RTL_FINAL {\"retired\":%0d,\"exit_reg\":%0d,\"regs\":[%0d,%0d,%0d,%0d,%0d,%0d,%0d,%0d,%0d,%0d,%0d,%0d,%0d,%0d,%0d,%0d,%0d,%0d,%0d,%0d,%0d,%0d,%0d,%0d,%0d,%0d,%0d,%0d,%0d,%0d,%0d,%0d],\"r3\":%0d,\"r4\":%0d,\"r5\":%0d,\"env_page\":%0d,\"mem0\":%0d,\"mem_checksum\":%0d,\"errno\":%0d}",
            retired_count,
            dut.core_tiles[0].core_i.gpr[
                dut.core_tiles[0].core_i.program_rom[dut.retire_submit_record_vec[0].pc][23:19]
            ],
            dut.core_tiles[0].core_i.gpr[0],
            dut.core_tiles[0].core_i.gpr[1],
            dut.core_tiles[0].core_i.gpr[2],
            dut.core_tiles[0].core_i.gpr[3],
            dut.core_tiles[0].core_i.gpr[4],
            dut.core_tiles[0].core_i.gpr[5],
            dut.core_tiles[0].core_i.gpr[6],
            dut.core_tiles[0].core_i.gpr[7],
            dut.core_tiles[0].core_i.gpr[8],
            dut.core_tiles[0].core_i.gpr[9],
            dut.core_tiles[0].core_i.gpr[10],
            dut.core_tiles[0].core_i.gpr[11],
            dut.core_tiles[0].core_i.gpr[12],
            dut.core_tiles[0].core_i.gpr[13],
            dut.core_tiles[0].core_i.gpr[14],
            dut.core_tiles[0].core_i.gpr[15],
            dut.core_tiles[0].core_i.gpr[16],
            dut.core_tiles[0].core_i.gpr[17],
            dut.core_tiles[0].core_i.gpr[18],
            dut.core_tiles[0].core_i.gpr[19],
            dut.core_tiles[0].core_i.gpr[20],
            dut.core_tiles[0].core_i.gpr[21],
            dut.core_tiles[0].core_i.gpr[22],
            dut.core_tiles[0].core_i.gpr[23],
            dut.core_tiles[0].core_i.gpr[24],
            dut.core_tiles[0].core_i.gpr[25],
            dut.core_tiles[0].core_i.gpr[26],
            dut.core_tiles[0].core_i.gpr[27],
            dut.core_tiles[0].core_i.gpr[28],
            dut.core_tiles[0].core_i.gpr[29],
            dut.core_tiles[0].core_i.gpr[30],
            dut.core_tiles[0].core_i.gpr[31],
            dut.core_tiles[0].core_i.gpr[3],
            dut.core_tiles[0].core_i.gpr[4],
            dut.core_tiles[0].core_i.gpr[5],
            dut.core_tiles[0].core_i.gpr[6],
            dut.core_tiles[0].core_i.sram[0],
            final_mem_checksum,
            dut.core_tiles[0].core_i.errno_reg
        );
        $display("LNP64-RTL-TOP-PROGRAM PASS");
        $finish;
    end
endmodule

`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_core_tile #(
    parameter int TILE_ID = 0
) (
    input  logic clk,
    input  logic reset_n,
    input  logic tile_enable,
    input  logic release_core,
    input  logic [31:0] topology_tile_count,
    input  logic [63:0] topology_enabled_tile_mask,
    input  logic [31:0] topology_coherence_domain_id,
    input  logic [31:0] topology_active_window_base,
    input  logic [31:0] topology_active_window_count,

    output logic cmd_valid,
    input  logic cmd_ready,
    output lnp64_cmd_t cmd,
    input  logic rsp_valid,
    output logic rsp_ready,
    input  lnp64_rsp_t rsp,

    output logic yielded,
    input  logic wake_valid,
    output logic done,
    output logic tile_reset_stable,
    output logic tile_idle,
    output logic tile_running,
    output logic tile_parked,
    output logic tile_faulted,
    output logic [31:0] tile_telemetry_counter,
    output logic [31:0] tile_fault_counter,
    output logic retire_submit_valid,
    output lnp64_retire_submit_t retire_submit_record,
    output logic park_submit_valid,
    output lnp64_thread_sched_t park_submit_record,
    output logic submit_valid,
    output lnp64_thread_sched_t submit_record,
    output logic icache_invalidate,
    input  logic icache_invalidate_ack,
    output logic dcache_writeback,
    input  logic dcache_writeback_ack,
    output logic tlb_invalidate,
    input  logic tlb_invalidate_ack,

    output logic pid1_runnable,
    output logic pid1_parked,
    output logic [31:0] retired_count,
    output logic [63:0] env_features_seen,
    output logic [31:0] env_tile_count_seen,
    output logic [63:0] env_enabled_tile_mask_seen,
    output logic [31:0] env_coherence_domain_seen,
    output logic [31:0] env_active_window_base_seen,
    output logic [31:0] env_active_window_count_seen,
    output logic [63:0] ld_value_seen,
    output logic object_stub_failed_closed,
    output logic unsupported_failed_closed,
    output logic raw_authority_visible
);
    typedef enum logic [2:0] {
        CORE_RESET,
        CORE_WAIT_RELEASE,
        CORE_EXEC,
        CORE_SEND_CMD,
        CORE_WAIT_RSP,
        CORE_DONE
    } core_state_e;

    core_state_e state;
    lnp64_decode_t dec;
    logic [31:0] instr;
    logic [31:0] pc;
    logic [31:0] next_op_id;
    logic [63:0] gpr [0:31];
    logic [63:0] sram [0:15];
    logic [15:0] errno_reg;
    logic pending_unsupported;
    logic [31:0] command_pc;
    lnp64_retire_submit_t retire_submit_next;
    lnp64_thread_sched_t thread_submit_next;

    lnp64_decode decode_i(.instr(instr), .dec(dec));

    function automatic logic [31:0] rom(input logic [31:0] index);
        case (index)
            32'd0:  rom = {LNP64_OP_NOP[7:0],        8'd0, 8'd0, 8'd0};
            32'd1:  rom = {LNP64_OP_LI32[7:0],       8'd1, 8'd0, 8'd7};
            32'd2:  rom = {LNP64_OP_LI32[7:0],       8'd2, 8'd0, 8'd5};
            32'd3:  rom = {LNP64_OP_ADD[7:0],        8'd3, 8'd1, 8'd2};
            32'd4:  rom = {LNP64_OP_ST[7:0],         8'd0, 8'd3, 8'd0};
            32'd5:  rom = {LNP64_OP_LD[7:0],         8'd4, 8'd0, 8'd0};
            32'd6:  rom = {LNP64_OP_JMP[7:0],        8'd0, 8'd0, 8'd1};
            32'd7:  rom = {LNP64_OP_LI32[7:0],       8'd5, 8'd0, 8'd99};
            32'd8:  rom = {LNP64_OP_YIELD[7:0],      8'd0, 8'd0, 8'd0};
            32'd9:  rom = {LNP64_OP_ENV_GET[7:0],    8'd6, 8'd0, 8'd0};
            32'd10: rom = {LNP64_OP_SET_ERRNO[7:0],  8'd0, 8'd0, 8'd13};
            32'd11: rom = {LNP64_OP_GET_ERRNO[7:0],  8'd7, 8'd0, 8'd0};
            32'd12: rom = {LNP64_OP_OBJECT_CTL[7:0], 8'd8, 8'd0, 8'd0};
            32'd13: rom = {LNP64_OP_UNSUPPORTED[7:0],8'd9, 8'd0, 8'd0};
            default: rom = {LNP64_OP_NOP[7:0],       8'd0, 8'd0, 8'd0};
        endcase
    endfunction

    always_comb begin
        instr = rom(pc);
    end

    always_comb begin
        cmd.op_id = next_op_id;
        cmd.tile_id = TILE_ID[31:0];
        cmd.opcode = pending_unsupported ? LNP64_OP_UNSUPPORTED : dec.opcode;
        cmd.profile = 16'd0;
        cmd.pid = 32'd1;
        cmd.tid = 32'd1;
        cmd.domain_id = 32'd1;
        cmd.domain_gen = 32'd1;
        cmd.credential_snapshot_id = 32'd1;
        cmd.result_reg = dec.rd;
        cmd.rights_mask = 64'd0;
        cmd.flags = 64'd0;
        cmd.arg0 = 64'd0;
        cmd.arg1 = 64'd0;
        cmd.arg2 = 64'd0;
        cmd.arg3 = 64'd0;
        cmd.arg_block_ptr = 64'd0;
        cmd.arg_block_len = 64'd0;
        cmd.cancel_class = 16'd0;
        cmd.completion_target = 16'd1;
    end

    always_comb begin
        tile_idle = tile_enable && (state == CORE_WAIT_RELEASE || state == CORE_DONE);
        tile_running = tile_enable && (state == CORE_EXEC || state == CORE_SEND_CMD || state == CORE_WAIT_RSP);
        tile_parked = tile_enable && pid1_parked;
        tile_faulted = 1'b0;
        tile_telemetry_counter = retired_count;
        tile_fault_counter = 32'd0;

        retire_submit_next = '0;
        retire_submit_next.op_id = next_op_id;
        retire_submit_next.tile_id = TILE_ID[31:0];
        retire_submit_next.pid = 32'd1;
        retire_submit_next.tid = 32'd1;
        retire_submit_next.pc = pc;
        retire_submit_next.action = 16'd1;

        thread_submit_next = '0;
        thread_submit_next.pid = 32'd1;
        thread_submit_next.tid = 32'd1;
        thread_submit_next.tile_id = TILE_ID[31:0];
        thread_submit_next.domain_id = 32'd1;
        thread_submit_next.domain_gen = 32'd1;
        thread_submit_next.state = pid1_parked ? 16'd2 : (pid1_runnable ? 16'd1 : 16'd0);
        thread_submit_next.wait_generation = 32'd1;
        thread_submit_next.active_location = TILE_ID[31:0];
    end

    integer i;
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            state <= CORE_RESET;
            pc <= 32'd0;
            next_op_id <= 32'd1;
            cmd_valid <= 1'b0;
            rsp_ready <= 1'b0;
            yielded <= 1'b0;
            done <= 1'b0;
            tile_reset_stable <= 1'b0;
            retire_submit_valid <= 1'b0;
            retire_submit_record <= '0;
            park_submit_valid <= 1'b0;
            park_submit_record <= '0;
            submit_valid <= 1'b0;
            submit_record <= '0;
            icache_invalidate <= 1'b0;
            dcache_writeback <= 1'b0;
            tlb_invalidate <= 1'b0;
            pid1_runnable <= 1'b0;
            pid1_parked <= 1'b0;
            retired_count <= 32'd0;
            env_features_seen <= 64'd0;
            env_tile_count_seen <= 32'd0;
            env_enabled_tile_mask_seen <= 64'd0;
            env_coherence_domain_seen <= 32'd0;
            env_active_window_base_seen <= 32'd0;
            env_active_window_count_seen <= 32'd0;
            ld_value_seen <= 64'd0;
            object_stub_failed_closed <= 1'b0;
            unsupported_failed_closed <= 1'b0;
            raw_authority_visible <= 1'b0;
            errno_reg <= LNP64_ERR_OK;
            pending_unsupported <= 1'b0;
            command_pc <= 32'd0;
            for (i = 0; i < 32; i = i + 1) begin
                gpr[i] <= 64'd0;
            end
            for (i = 0; i < 16; i = i + 1) begin
                sram[i] <= 64'd0;
            end
        end else begin
            retire_submit_valid <= 1'b0;
            park_submit_valid <= 1'b0;
            submit_valid <= 1'b0;
            if (icache_invalidate && icache_invalidate_ack) begin
                icache_invalidate <= 1'b0;
            end
            if (dcache_writeback && dcache_writeback_ack) begin
                dcache_writeback <= 1'b0;
            end
            if (tlb_invalidate && tlb_invalidate_ack) begin
                tlb_invalidate <= 1'b0;
            end
            case (state)
                CORE_RESET: begin
                    state <= CORE_WAIT_RELEASE;
                    pid1_runnable <= 1'b0;
                    tile_reset_stable <= tile_enable;
                end
                CORE_WAIT_RELEASE: begin
                    if (tile_enable && release_core) begin
                        state <= CORE_EXEC;
                        pid1_runnable <= 1'b1;
                        submit_valid <= 1'b1;
                        submit_record <= thread_submit_next;
                        icache_invalidate <= 1'b1;
                    end
                end
                CORE_EXEC: begin
                    cmd_valid <= 1'b0;
                    rsp_ready <= 1'b0;
                    if (!dec.supported) begin
                        pending_unsupported <= 1'b1;
                        command_pc <= pc;
                        state <= CORE_SEND_CMD;
                    end else begin
                        unique case (dec.opcode)
                            LNP64_OP_NOP: begin
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_LI32: begin
                                gpr[dec.rd] <= {{32{dec.imm[31]}}, dec.imm};
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_ADD: begin
                                gpr[dec.rd] <= gpr[dec.rs1] + gpr[dec.rs2];
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_JMP: begin
                                pc <= pc + 32'd1 + dec.imm;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_LD: begin
                                gpr[dec.rd] <= sram[dec.imm[3:0]];
                                ld_value_seen <= sram[dec.imm[3:0]];
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_ST: begin
                                sram[dec.imm[3:0]] <= gpr[dec.rs1];
                                dcache_writeback <= 1'b1;
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_YIELD: begin
                                yielded <= 1'b1;
                                pid1_runnable <= 1'b0;
                                pid1_parked <= 1'b1;
                                park_submit_valid <= 1'b1;
                                park_submit_record <= thread_submit_next;
                                if (wake_valid) begin
                                    pid1_runnable <= 1'b1;
                                    pid1_parked <= 1'b0;
                                    pc <= pc + 32'd1;
                                    retired_count <= retired_count + 32'd1;
                                    retire_submit_valid <= 1'b1;
                                    retire_submit_record <= retire_submit_next;
                                end
                            end
                            LNP64_OP_ENV_GET: begin
                                gpr[dec.rd] <= LNP64_S0_FEATURES;
                                env_features_seen <= LNP64_S0_FEATURES;
                                env_tile_count_seen <= topology_tile_count;
                                env_enabled_tile_mask_seen <= topology_enabled_tile_mask;
                                env_coherence_domain_seen <= topology_coherence_domain_id;
                                env_active_window_base_seen <= topology_active_window_base;
                                env_active_window_count_seen <= topology_active_window_count;
                                tlb_invalidate <= 1'b1;
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_SET_ERRNO: begin
                                errno_reg <= dec.imm[15:0];
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_GET_ERRNO: begin
                                gpr[dec.rd] <= {48'd0, errno_reg};
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_OBJECT_CTL: begin
                                pending_unsupported <= 1'b0;
                                command_pc <= pc;
                                state <= CORE_SEND_CMD;
                            end
                            default: begin
                                pending_unsupported <= 1'b1;
                                command_pc <= pc;
                                state <= CORE_SEND_CMD;
                            end
                        endcase
                    end
                end
                CORE_SEND_CMD: begin
                    if (!cmd_valid) begin
                        cmd_valid <= 1'b1;
                    end else if (cmd_ready) begin
                        cmd_valid <= 1'b0;
                        rsp_ready <= 1'b1;
                        state <= CORE_WAIT_RSP;
                    end
                end
                CORE_WAIT_RSP: begin
                    rsp_ready <= 1'b1;
                    if (rsp_valid) begin
                        rsp_ready <= 1'b0;
                        gpr[rsp.result_reg] <= rsp.result_value;
                        errno_reg <= rsp.errno_value;
                        if (!pending_unsupported &&
                            rsp.status == LNP64_STATUS_ERROR &&
                            rsp.errno_value == LNP64_ERR_EPERM) begin
                            object_stub_failed_closed <= 1'b1;
                        end
                        if (pending_unsupported &&
                            rsp.status == LNP64_STATUS_UNSUPPORTED &&
                            rsp.errno_value == LNP64_ERR_ENOTSUP) begin
                            unsupported_failed_closed <= 1'b1;
                        end
                        pending_unsupported <= 1'b0;
                        retired_count <= retired_count + 32'd1;
                        retire_submit_valid <= 1'b1;
                        retire_submit_record <= retire_submit_next;
                        pc <= command_pc + 32'd1;
                        next_op_id <= next_op_id + 32'd1;
                        if (command_pc == 32'd13) begin
                            done <= 1'b1;
                            pid1_runnable <= 1'b0;
                            pid1_parked <= 1'b0;
                            state <= CORE_DONE;
                        end else begin
                            state <= CORE_EXEC;
                        end
                    end
                end
                CORE_DONE: begin
                    cmd_valid <= 1'b0;
                    rsp_ready <= 1'b0;
                end
                default: state <= CORE_RESET;
            endcase
            gpr[0] <= 64'd0;
        end
    end
endmodule

module lnp64_issue_retire (
    input  logic clk,
    input  logic reset_n,
    input  logic retire_valid,
    output logic [31:0] retire_counter
);
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            retire_counter <= 32'd0;
        end else if (retire_valid) begin
            retire_counter <= retire_counter + 32'd1;
        end
    end
endmodule

module lnp64_thread_context (
    input  logic clk,
    input  logic reset_n,
    input  logic boot_valid,
    output lnp64_thread_sched_t pid1_context
);
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            pid1_context <= '0;
        end else if (boot_valid) begin
            pid1_context.pid <= 32'd1;
            pid1_context.tid <= 32'd1;
            pid1_context.tile_id <= 32'd0;
            pid1_context.domain_id <= 32'd1;
            pid1_context.domain_gen <= 32'd1;
            pid1_context.state <= 16'd1;
            pid1_context.latency_class <= 16'd0;
            pid1_context.wait_generation <= 32'd1;
            pid1_context.active_location <= 32'd1;
        end
    end
endmodule

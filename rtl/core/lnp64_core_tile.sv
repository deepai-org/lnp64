`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_core_tile #(
    parameter int TILE_ID = 0,
    parameter int PROGRAM_WORDS = 64
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
    logic [31:0] program_rom [0:PROGRAM_WORDS-1];
    localparam logic [63:0] HEAP_ARCH_BASE = 64'h0000_0000_0010_f000;
    localparam logic [3:0] HEAP_SRAM_BASE_WORD = 4'd12;
    logic [15:0] errno_reg;
    logic cmp_zero;
    logic cmp_negative;
    logic cmp_greater;
    logic [31:0] return_stack [0:7];
    logic [3:0] return_stack_depth;
    logic pending_unsupported;
    logic [31:0] command_pc;
    logic [63:0] mem_addr;
    logic [63:0] heap_next;
    lnp64_retire_submit_t retire_submit_next;
    lnp64_thread_sched_t thread_submit_next;

    lnp64_decode decode_i(.instr(instr), .dec(dec));

    function automatic logic [31:0] enc_ri(
        input logic [7:0] opcode,
        input logic [4:0] rd,
        input logic signed [15:0] imm
    );
        enc_ri = {opcode, rd, 3'd0, imm[15:0]};
    endfunction

    function automatic logic [31:0] enc_rrr(
        input logic [7:0] opcode,
        input logic [4:0] rd,
        input logic [4:0] rs1,
        input logic [4:0] rs2
    );
        enc_rrr = {opcode, rd, rs1, rs2, 9'd0};
    endfunction

    function automatic logic [31:0] enc_rrrr(
        input logic [7:0] opcode,
        input logic [4:0] rd,
        input logic [4:0] rs1,
        input logic [4:0] rs2,
        input logic [4:0] rs3
    );
        enc_rrrr = {opcode, rd, rs1, rs2, rs3, 4'd0};
    endfunction

    function automatic logic [31:0] enc_mem(
        input logic [7:0] opcode,
        input logic [4:0] reg_a,
        input logic [4:0] base,
        input logic signed [13:0] imm
    );
        enc_mem = {opcode, reg_a, base, imm[13:0]};
    endfunction

    function automatic logic [31:0] enc_reg(
        input logic [7:0] opcode,
        input logic [4:0] reg_a
    );
        enc_reg = {opcode, reg_a, 19'd0};
    endfunction

    function automatic logic [31:0] enc_branch(
        input logic [7:0] opcode,
        input logic signed [23:0] delta_words
    );
        enc_branch = {opcode, delta_words[23:0]};
    endfunction

    function automatic logic [3:0] sram_word_index(input logic [63:0] addr);
        if (addr >= HEAP_ARCH_BASE) begin
            sram_word_index = HEAP_SRAM_BASE_WORD + addr[6:3];
        end else begin
            sram_word_index = addr[6:3];
        end
    endfunction

    string program_hex_path;
    integer rom_i;
    initial begin
        for (rom_i = 0; rom_i < PROGRAM_WORDS; rom_i = rom_i + 1) begin
            program_rom[rom_i] = enc_reg(8'h00, 5'd0);
        end
        program_rom[0]  = enc_reg(8'h00, 5'd0);
        program_rom[1]  = enc_ri(8'h01, 5'd1, 16'sd7);
        program_rom[2]  = enc_ri(8'h01, 5'd2, 16'sd5);
        program_rom[3]  = enc_rrr(8'h10, 5'd3, 5'd1, 5'd2);
        program_rom[4]  = enc_mem(8'h33, 5'd3, 5'd0, 14'sd0);
        program_rom[5]  = enc_mem(8'h30, 5'd4, 5'd0, 14'sd0);
        program_rom[6]  = enc_branch(8'h20, 24'sd2);
        program_rom[7]  = enc_ri(8'h01, 5'd5, 16'sd99);
        program_rom[8]  = enc_reg(8'h06, 5'd0);
        program_rom[9]  = enc_rrrr(8'h56, 5'd6, 5'd0, 5'd0, 5'd0);
        program_rom[10] = enc_ri(8'h01, 5'd9, 16'sd13);
        program_rom[11] = enc_reg(8'h39, 5'd9);
        program_rom[12] = enc_reg(8'h38, 5'd7);
        program_rom[13] = enc_rrr(8'h4b, 5'd8, 5'd0, 5'd0);
        program_rom[14] = enc_reg(8'hff, 5'd9);
`ifndef SYNTHESIS
        if ($value$plusargs("lnp64_program_hex=%s", program_hex_path)) begin
            $readmemh(program_hex_path, program_rom);
        end
`endif
    end

    always_comb begin
        if (pc < PROGRAM_WORDS[31:0]) begin
            instr = program_rom[pc];
        end else begin
            instr = enc_reg(8'hff, 5'd0);
        end
        mem_addr = gpr[dec.rs1] + {{32{dec.imm[31]}}, dec.imm};
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
            cmp_zero <= 1'b0;
            cmp_negative <= 1'b0;
            cmp_greater <= 1'b0;
            return_stack_depth <= 4'd0;
            pending_unsupported <= 1'b0;
            command_pc <= 32'd0;
            heap_next <= HEAP_ARCH_BASE;
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
                            LNP64_OP_LI32_LITERAL: begin
                                gpr[dec.rd] <= {32'd0, program_rom[pc + 32'd1]};
                                pc <= pc + 32'd2;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_MOV: begin
                                gpr[dec.rd] <= gpr[dec.rs1];
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
                            LNP64_OP_ADDI: begin
                                gpr[dec.rd] <= gpr[dec.rs1] + {{32{dec.imm[31]}}, dec.imm};
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_SUB: begin
                                gpr[dec.rd] <= gpr[dec.rs1] - gpr[dec.rs2];
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_MUL: begin
                                gpr[dec.rd] <= gpr[dec.rs1] * gpr[dec.rs2];
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_DIV: begin
                                gpr[dec.rd] <= gpr[dec.rs2] == 64'd0 ? 64'd0 : $signed(gpr[dec.rs1]) / $signed(gpr[dec.rs2]);
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_AND: begin
                                gpr[dec.rd] <= gpr[dec.rs1] & gpr[dec.rs2];
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_ANDI: begin
                                gpr[dec.rd] <= gpr[dec.rs1] & {{32{dec.imm[31]}}, dec.imm};
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_OR: begin
                                gpr[dec.rd] <= gpr[dec.rs1] | gpr[dec.rs2];
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_ORI: begin
                                gpr[dec.rd] <= gpr[dec.rs1] | {{32{dec.imm[31]}}, dec.imm};
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_XOR: begin
                                gpr[dec.rd] <= gpr[dec.rs1] ^ gpr[dec.rs2];
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_XORI: begin
                                gpr[dec.rd] <= gpr[dec.rs1] ^ {{32{dec.imm[31]}}, dec.imm};
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_NOT: begin
                                gpr[dec.rd] <= ~gpr[dec.rs1];
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_LSL: begin
                                gpr[dec.rd] <= gpr[dec.rs1] << gpr[dec.rs2][5:0];
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_LSR: begin
                                gpr[dec.rd] <= gpr[dec.rs1] >> gpr[dec.rs2][5:0];
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_ASR: begin
                                gpr[dec.rd] <= $signed(gpr[dec.rs1]) >>> gpr[dec.rs2][5:0];
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_LSLI: begin
                                gpr[dec.rd] <= gpr[dec.rs1] << dec.imm[5:0];
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_LSRI: begin
                                gpr[dec.rd] <= gpr[dec.rs1] >> dec.imm[5:0];
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_ASRI: begin
                                gpr[dec.rd] <= $signed(gpr[dec.rs1]) >>> dec.imm[5:0];
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_SEXT_B: begin
                                gpr[dec.rd] <= {{56{gpr[dec.rs1][7]}}, gpr[dec.rs1][7:0]};
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_SEXT_H: begin
                                gpr[dec.rd] <= {{48{gpr[dec.rs1][15]}}, gpr[dec.rs1][15:0]};
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_SEXT_W: begin
                                gpr[dec.rd] <= {{32{gpr[dec.rs1][31]}}, gpr[dec.rs1][31:0]};
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_ZEXT_B: begin
                                gpr[dec.rd] <= {56'd0, gpr[dec.rs1][7:0]};
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_ZEXT_H: begin
                                gpr[dec.rd] <= {48'd0, gpr[dec.rs1][15:0]};
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_ZEXT_W: begin
                                gpr[dec.rd] <= {32'd0, gpr[dec.rs1][31:0]};
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_UDIV: begin
                                gpr[dec.rd] <= gpr[dec.rs2] == 64'd0 ? 64'd0 : gpr[dec.rs1] / gpr[dec.rs2];
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_UREM: begin
                                gpr[dec.rd] <= gpr[dec.rs2] == 64'd0 ? 64'd0 : gpr[dec.rs1] % gpr[dec.rs2];
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_SREM: begin
                                gpr[dec.rd] <= gpr[dec.rs2] == 64'd0 ? 64'd0 : $signed(gpr[dec.rs1]) % $signed(gpr[dec.rs2]);
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_CMP: begin
                                cmp_zero <= gpr[dec.rd] == gpr[dec.rs1];
                                cmp_negative <= $signed(gpr[dec.rd]) < $signed(gpr[dec.rs1]);
                                cmp_greater <= $signed(gpr[dec.rd]) > $signed(gpr[dec.rs1]);
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_JMP: begin
                                pc <= pc + dec.imm;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_BRANCH_EQ: begin
                                pc <= cmp_zero ? pc + dec.imm : pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_BRANCH_NE: begin
                                pc <= !cmp_zero ? pc + dec.imm : pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_BRANCH_LT: begin
                                pc <= cmp_negative ? pc + dec.imm : pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_BRANCH_GT: begin
                                pc <= cmp_greater ? pc + dec.imm : pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_BRANCH_LE: begin
                                pc <= (cmp_zero || cmp_negative) ? pc + dec.imm : pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_BRANCH_GE: begin
                                pc <= (cmp_zero || cmp_greater) ? pc + dec.imm : pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_CALL: begin
                                if (return_stack_depth < 4'd8) begin
                                    return_stack[return_stack_depth[2:0]] <= pc + 32'd1;
                                    return_stack_depth <= return_stack_depth + 4'd1;
                                end
                                pc <= pc + dec.imm;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_RET: begin
                                if (return_stack_depth != 4'd0) begin
                                    pc <= return_stack[return_stack_depth[2:0] - 3'd1];
                                    return_stack_depth <= return_stack_depth - 4'd1;
                                end else begin
                                    pc <= pc + 32'd1;
                                end
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_ALLOC: begin
                                gpr[dec.rd] <= heap_next;
                                heap_next <= heap_next + gpr[dec.rs1];
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_LD: begin
                                gpr[dec.rd] <= sram[sram_word_index(mem_addr)];
                                ld_value_seen <= sram[sram_word_index(mem_addr)];
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_LD_B: begin
                                unique case (mem_addr[2:0])
                                    3'd0: gpr[dec.rd] <= {56'd0, sram[sram_word_index(mem_addr)][7:0]};
                                    3'd1: gpr[dec.rd] <= {56'd0, sram[sram_word_index(mem_addr)][15:8]};
                                    3'd2: gpr[dec.rd] <= {56'd0, sram[sram_word_index(mem_addr)][23:16]};
                                    3'd3: gpr[dec.rd] <= {56'd0, sram[sram_word_index(mem_addr)][31:24]};
                                    3'd4: gpr[dec.rd] <= {56'd0, sram[sram_word_index(mem_addr)][39:32]};
                                    3'd5: gpr[dec.rd] <= {56'd0, sram[sram_word_index(mem_addr)][47:40]};
                                    3'd6: gpr[dec.rd] <= {56'd0, sram[sram_word_index(mem_addr)][55:48]};
                                    default: gpr[dec.rd] <= {56'd0, sram[sram_word_index(mem_addr)][63:56]};
                                endcase
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_ST: begin
                                sram[sram_word_index(mem_addr)] <= gpr[dec.rd];
                                dcache_writeback <= 1'b1;
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_ST_B: begin
                                unique case (mem_addr[2:0])
                                    3'd0: sram[sram_word_index(mem_addr)][7:0] <= gpr[dec.rd][7:0];
                                    3'd1: sram[sram_word_index(mem_addr)][15:8] <= gpr[dec.rd][7:0];
                                    3'd2: sram[sram_word_index(mem_addr)][23:16] <= gpr[dec.rd][7:0];
                                    3'd3: sram[sram_word_index(mem_addr)][31:24] <= gpr[dec.rd][7:0];
                                    3'd4: sram[sram_word_index(mem_addr)][39:32] <= gpr[dec.rd][7:0];
                                    3'd5: sram[sram_word_index(mem_addr)][47:40] <= gpr[dec.rd][7:0];
                                    3'd6: sram[sram_word_index(mem_addr)][55:48] <= gpr[dec.rd][7:0];
                                    default: sram[sram_word_index(mem_addr)][63:56] <= gpr[dec.rd][7:0];
                                endcase
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
                                case (gpr[dec.rs1])
                                    64'd0: gpr[dec.rd] <= LNP64_S0_FEATURES;
                                    64'd2: gpr[dec.rd] <= 64'd4096;
                                    64'd5: gpr[dec.rd] <= LNP64_S0_FEATURES;
                                    64'd30: gpr[dec.rd] <= 64'd1;
                                    64'd53: gpr[dec.rd] <= {32'd0, topology_active_window_count};
                                    default: gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                endcase
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
                                errno_reg <= gpr[dec.rd][15:0];
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
                            LNP64_OP_EXIT: begin
                                done <= 1'b1;
                                pid1_runnable <= 1'b0;
                                pid1_parked <= 1'b0;
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                                state <= CORE_DONE;
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
                        if (pending_unsupported) begin
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

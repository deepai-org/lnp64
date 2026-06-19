`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_core_tile #(
    parameter int TILE_ID = 0,
    parameter int PROGRAM_WORDS = 256,
    parameter int SRAM_WORDS = 128,
    parameter int RETURN_STACK_DEPTH = 64
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
    logic [63:0] sram [0:SRAM_WORDS-1];
    logic [63:0] initial_sram [0:SRAM_WORDS-1];
    logic [63:0] initial_data_sram [0:SRAM_WORDS-1];
    logic [31:0] program_rom [0:PROGRAM_WORDS-1];
    localparam logic [63:0] HEAP_ARCH_BASE = 64'h0000_0000_0010_f000;
    localparam logic [63:0] FLAT_DATA_BASE_ADDR = 64'h0000_0000_0001_0000;
    localparam logic [63:0] FLAT_EXEC_BASE_ADDR = 64'h0000_0000_0000_1000;
    localparam logic [63:0] OBJECT_OP_CREATE = 64'd1;
    localparam logic [63:0] OBJECT_KIND_DMA_BUFFER = 64'd4;
    localparam logic [63:0] RTL_DMA_BUFFER_DEFAULT_FD = 64'd3;
    localparam logic [63:0] RTL_DMA_BUFFER_TOKEN = 64'h4000_0000_0000_0203;
    localparam logic [15:0] RTL_ERR_ESTALE = 16'd116;
    localparam int unsigned DATA_SRAM_BASE_WORD = 16;
    localparam int unsigned HEAP_SRAM_BASE_WORD = 96;
    logic [15:0] errno_reg;
    logic cmp_zero;
    logic cmp_negative;
    logic cmp_greater;
    logic cmp_below;
    logic cmp_above;
    localparam int RETURN_STACK_INDEX_WIDTH = $clog2(RETURN_STACK_DEPTH);
    localparam logic [RETURN_STACK_INDEX_WIDTH:0] RETURN_STACK_DEPTH_VALUE = RETURN_STACK_DEPTH;
    logic [31:0] return_stack [0:RETURN_STACK_DEPTH-1];
    logic [RETURN_STACK_INDEX_WIDTH:0] return_stack_depth;
    logic [63:0] link_register;
    logic pending_unsupported;
    logic [31:0] command_pc;
    logic [63:0] mem_addr;
    logic [63:0] dma_argblock_addr;
    logic [63:0] dma_op;
    logic [63:0] dma_dst;
    logic [63:0] dma_src_or_value;
    logic [63:0] dma_len;
    logic [63:0] dma_buffer;
    logic dma_dst_heap_valid;
    logic dma_src_heap_valid;
    logic dma_scope_valid;
    int unsigned dma_dst_next_sram_word_index;
    logic [63:0] object_argblock_addr;
    logic [63:0] object_op;
    logic [63:0] object_kind;
    logic [63:0] object_fd_req;
    logic [63:0] object_dma_addr;
    logic [63:0] object_dma_len;
    logic [63:0] object_fd_store_addr;
    int unsigned object_fd_store_word_index;
    int unsigned object_fd_store_next_word_index;
    logic [2:0] object_fd_store_byte_lane;
    logic dma_buffer_object_valid;
    logic dma_buffer_object_revoked;
    logic [63:0] dma_buffer_object_fd;
    logic [63:0] dma_buffer_object_addr;
    logic [63:0] dma_buffer_object_len;
    logic [63:0] heap_next;
    logic [63:0] heap_alloc_ptr [0:3];
    logic [63:0] heap_alloc_size [0:3];
    logic heap_alloc_valid [0:3];
    logic [1:0] heap_alloc_next_slot;
    logic topology_record_valid;
    logic [63:0] topology_record_base;
    int unsigned mem_sram_word_index;
    int unsigned mem_sram_next_word_index;
    int unsigned dma_dst_sram_word_index;
    logic [2:0] mem_byte_lane;
    logic [2:0] dma_dst_byte_lane;
    logic [1:0] mem_half_lane;
    logic mem_word_upper;
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

    function automatic int unsigned sram_word_index(input logic [63:0] addr);
        logic [63:0] rel_addr;
        begin
            if (addr >= HEAP_ARCH_BASE) begin
                rel_addr = addr - HEAP_ARCH_BASE;
                sram_word_index = HEAP_SRAM_BASE_WORD + rel_addr[7:3];
            end else if (addr >= FLAT_DATA_BASE_ADDR) begin
                rel_addr = addr - FLAT_DATA_BASE_ADDR;
                sram_word_index = DATA_SRAM_BASE_WORD + rel_addr[8:3];
            end else begin
                sram_word_index = addr[8:3];
            end
        end
    endfunction

    function automatic logic [63:0] flat_exec_addr(input logic [31:0] pc_word);
        flat_exec_addr = FLAT_EXEC_BASE_ADDR + {30'd0, pc_word, 2'd0};
    endfunction

    function automatic logic [31:0] flat_exec_pc_word(input logic [63:0] addr);
        if (addr >= FLAT_EXEC_BASE_ADDR) begin
            flat_exec_pc_word = (addr - FLAT_EXEC_BASE_ADDR) >> 2;
        end else begin
            flat_exec_pc_word = addr[31:0];
        end
    endfunction

    function automatic logic [63:0] sram_read_word_or_zero(input logic [63:0] addr);
        int unsigned idx;
        begin
            idx = sram_word_index(addr);
            if (idx < SRAM_WORDS) begin
                sram_read_word_or_zero = sram[idx];
            end else begin
                sram_read_word_or_zero = 64'd0;
            end
        end
    endfunction

    function automatic logic [63:0] load_double_unaligned(input logic [63:0] addr);
        logic [2:0] lane;
        logic [63:0] low_word;
        logic [63:0] high_word;
        begin
            lane = addr[2:0];
            low_word = sram_read_word_or_zero(addr);
            high_word = sram_read_word_or_zero(addr + (64'd8 - {61'd0, lane}));
            if (lane == 3'd0) begin
                load_double_unaligned = low_word;
            end else begin
                load_double_unaligned = (low_word >> ({3'd0, lane} * 6'd8)) |
                    (high_word << ((6'd8 - {3'd0, lane}) * 6'd8));
            end
        end
    endfunction

    function automatic logic [63:0] store_double_low_word(
        input logic [63:0] word,
        input logic [2:0] lane,
        input logic [63:0] value
    );
        begin
            store_double_low_word = word;
            for (int byte_idx = 0; byte_idx < 8; byte_idx = byte_idx + 1) begin
                if (byte_idx >= lane) begin
                    store_double_low_word[byte_idx * 8 +: 8] = value[(byte_idx - lane) * 8 +: 8];
                end
            end
        end
    endfunction

    function automatic logic [63:0] store_double_high_word(
        input logic [63:0] word,
        input logic [2:0] lane,
        input logic [63:0] value
    );
        begin
            store_double_high_word = word;
            for (int byte_idx = 0; byte_idx < 8; byte_idx = byte_idx + 1) begin
                if (byte_idx < lane) begin
                    store_double_high_word[byte_idx * 8 +: 8] =
                        value[(8 - lane + byte_idx) * 8 +: 8];
                end
            end
        end
    endfunction

    function automatic logic [63:0] load_word_lane(input logic [63:0] word, input logic upper);
        load_word_lane = upper ? {32'd0, word[63:32]} : {32'd0, word[31:0]};
    endfunction

    function automatic logic [63:0] load_half_lane(input logic [63:0] word, input logic [1:0] lane);
        begin
            unique case (lane)
                2'd0: load_half_lane = {48'd0, word[15:0]};
                2'd1: load_half_lane = {48'd0, word[31:16]};
                2'd2: load_half_lane = {48'd0, word[47:32]};
                default: load_half_lane = {48'd0, word[63:48]};
            endcase
        end
    endfunction

    function automatic logic [63:0] load_byte_lane(input logic [63:0] word, input logic [2:0] lane);
        begin
            unique case (lane)
                3'd0: load_byte_lane = {56'd0, word[7:0]};
                3'd1: load_byte_lane = {56'd0, word[15:8]};
                3'd2: load_byte_lane = {56'd0, word[23:16]};
                3'd3: load_byte_lane = {56'd0, word[31:24]};
                3'd4: load_byte_lane = {56'd0, word[39:32]};
                3'd5: load_byte_lane = {56'd0, word[47:40]};
                3'd6: load_byte_lane = {56'd0, word[55:48]};
                default: load_byte_lane = {56'd0, word[63:56]};
            endcase
        end
    endfunction

    function automatic logic [63:0] store_word_lane(input logic [63:0] word, input logic upper, input logic [31:0] value);
        begin
            store_word_lane = word;
            if (upper) begin
                store_word_lane[63:32] = value;
            end else begin
                store_word_lane[31:0] = value;
            end
        end
    endfunction

    function automatic logic [63:0] store_half_lane(input logic [63:0] word, input logic [1:0] lane, input logic [15:0] value);
        begin
            store_half_lane = word;
            unique case (lane)
                2'd0: store_half_lane[15:0] = value;
                2'd1: store_half_lane[31:16] = value;
                2'd2: store_half_lane[47:32] = value;
                default: store_half_lane[63:48] = value;
            endcase
        end
    endfunction

    function automatic logic [63:0] store_byte_lane(input logic [63:0] word, input logic [2:0] lane, input logic [7:0] value);
        begin
            store_byte_lane = word;
            unique case (lane)
                3'd0: store_byte_lane[7:0] = value;
                3'd1: store_byte_lane[15:8] = value;
                3'd2: store_byte_lane[23:16] = value;
                3'd3: store_byte_lane[31:24] = value;
                3'd4: store_byte_lane[39:32] = value;
                3'd5: store_byte_lane[47:40] = value;
                3'd6: store_byte_lane[55:48] = value;
                default: store_byte_lane[63:56] = value;
            endcase
        end
    endfunction

    function automatic logic [63:0] min_u64(input logic [63:0] lhs, input logic [63:0] rhs);
        min_u64 = lhs < rhs ? lhs : rhs;
    endfunction

    function automatic logic [63:0] align_up_u64(input logic [63:0] value, input logic [63:0] align);
        logic [63:0] mask;
        begin
            if (align <= 64'd1) begin
                align_up_u64 = value;
            end else begin
                mask = align - 64'd1;
                align_up_u64 = (value + mask) & ~mask;
            end
        end
    endfunction

    function automatic logic [63:0] alloc_len_u64(input logic [63:0] len);
        begin
            alloc_len_u64 = len == 64'd0 ? 64'd1 : len;
        end
    endfunction

    function automatic logic [63:0] alloc_align_u64(input logic [63:0] requested);
        logic [63:0] clamped;
        logic [63:0] rounded;
        begin
            if (requested < 64'd1) begin
                clamped = 64'd1;
            end else if (requested > 64'd4096) begin
                clamped = 64'd4096;
            end else begin
                clamped = requested;
            end
            rounded = 64'd1;
            for (int align_bit = 0; align_bit < 12; align_bit = align_bit + 1) begin
                if (rounded < clamped) begin
                    rounded = rounded << 1;
                end
            end
            alloc_align_u64 = rounded;
        end
    endfunction

    function automatic logic heap_range_valid(input logic [63:0] addr, input logic [63:0] len);
        begin
            heap_range_valid = 1'b0;
            for (int slot = 0; slot < 4; slot = slot + 1) begin
                if (heap_alloc_valid[slot] &&
                    addr >= heap_alloc_ptr[slot] &&
                    len <= heap_alloc_size[slot] &&
                    (addr - heap_alloc_ptr[slot]) <= (heap_alloc_size[slot] - len)) begin
                    heap_range_valid = 1'b1;
                end
            end
        end
    endfunction

    function automatic logic range_within(
        input logic [63:0] base,
        input logic [63:0] extent,
        input logic [63:0] addr,
        input logic [63:0] len
    );
        begin
            range_within = addr >= base &&
                len <= extent &&
                (addr - base) <= (extent - len);
        end
    endfunction

    function automatic logic dma_buffer_ref_matches(input logic [63:0] value);
        begin
            dma_buffer_ref_matches = dma_buffer_object_valid &&
                (value == dma_buffer_object_fd || value == RTL_DMA_BUFFER_TOKEN);
        end
    endfunction

    function automatic logic dma_buffer_ref_revoked(input logic [63:0] value);
        begin
            dma_buffer_ref_revoked = dma_buffer_ref_matches(value) && dma_buffer_object_revoked;
        end
    endfunction

    function automatic logic [63:0] clz64(input logic [63:0] value);
        integer bit_idx;
        logic seen_one;
        begin
            clz64 = 64'd0;
            seen_one = 1'b0;
            for (bit_idx = 63; bit_idx >= 0; bit_idx = bit_idx - 1) begin
                if (!seen_one && value[bit_idx]) begin
                    seen_one = 1'b1;
                end else if (!seen_one) begin
                    clz64 = clz64 + 64'd1;
                end
            end
        end
    endfunction

    function automatic logic [63:0] ctz64(input logic [63:0] value);
        integer bit_idx;
        logic seen_one;
        begin
            ctz64 = 64'd0;
            seen_one = 1'b0;
            for (bit_idx = 0; bit_idx < 64; bit_idx = bit_idx + 1) begin
                if (!seen_one && value[bit_idx]) begin
                    seen_one = 1'b1;
                end else if (!seen_one) begin
                    ctz64 = ctz64 + 64'd1;
                end
            end
        end
    endfunction

    function automatic logic [63:0] popcnt64(input logic [63:0] value);
        integer bit_idx;
        begin
            popcnt64 = 64'd0;
            for (bit_idx = 0; bit_idx < 64; bit_idx = bit_idx + 1) begin
                popcnt64 = popcnt64 + {63'd0, value[bit_idx]};
            end
        end
    endfunction

    function automatic logic [63:0] bswap64(input logic [63:0] value);
        begin
            bswap64 = {
                value[7:0],
                value[15:8],
                value[23:16],
                value[31:24],
                value[39:32],
                value[47:40],
                value[55:48],
                value[63:56]
            };
        end
    endfunction

    function automatic logic [63:0] mulh_signed(input logic [63:0] lhs, input logic [63:0] rhs);
        logic signed [127:0] lhs_ext;
        logic signed [127:0] rhs_ext;
        logic signed [127:0] product;
        begin
            lhs_ext = {{64{lhs[63]}}, lhs};
            rhs_ext = {{64{rhs[63]}}, rhs};
            product = lhs_ext * rhs_ext;
            mulh_signed = product[127:64];
        end
    endfunction

    function automatic logic [63:0] mulh_unsigned(input logic [63:0] lhs, input logic [63:0] rhs);
        logic [127:0] lhs_ext;
        logic [127:0] rhs_ext;
        logic [127:0] product;
        begin
            lhs_ext = {64'd0, lhs};
            rhs_ext = {64'd0, rhs};
            product = lhs_ext * rhs_ext;
            mulh_unsigned = product[127:64];
        end
    endfunction

    function automatic logic [63:0] mulh_signed_unsigned(input logic [63:0] lhs, input logic [63:0] rhs);
        logic signed [127:0] lhs_ext;
        logic signed [127:0] rhs_ext;
        logic signed [127:0] product;
        begin
            lhs_ext = {{64{lhs[63]}}, lhs};
            rhs_ext = {64'd0, rhs};
            product = lhs_ext * rhs_ext;
            mulh_signed_unsigned = product[127:64];
        end
    endfunction

    function automatic logic csel_condition(input logic [15:0] opcode);
        begin
            unique case (opcode)
                LNP64_OP_CSEL_EQ, LNP64_OP_CSET_EQ: csel_condition = cmp_zero;
                LNP64_OP_CSEL_NE, LNP64_OP_CSET_NE: csel_condition = !cmp_zero;
                LNP64_OP_CSEL_LT, LNP64_OP_CSET_LT: csel_condition = cmp_negative;
                LNP64_OP_CSEL_GT, LNP64_OP_CSET_GT: csel_condition = cmp_greater;
                LNP64_OP_CSEL_LE, LNP64_OP_CSET_LE: csel_condition = cmp_zero || cmp_negative;
                LNP64_OP_CSEL_GE, LNP64_OP_CSET_GE: csel_condition = cmp_zero || cmp_greater;
                LNP64_OP_CSEL_ULT, LNP64_OP_CSET_ULT: csel_condition = cmp_below;
                LNP64_OP_CSEL_UGT, LNP64_OP_CSET_UGT: csel_condition = cmp_above;
                LNP64_OP_CSEL_ULE, LNP64_OP_CSET_ULE: csel_condition = cmp_zero || cmp_below;
                LNP64_OP_CSEL_UGE, LNP64_OP_CSET_UGE: csel_condition = cmp_zero || cmp_above;
                default: csel_condition = 1'b0;
            endcase
        end
    endfunction

    string program_hex_path;
    string data_hex_path;
    integer rom_i;
    integer sram_i;
    initial begin
        for (rom_i = 0; rom_i < PROGRAM_WORDS; rom_i = rom_i + 1) begin
            program_rom[rom_i] = enc_reg(8'h00, 5'd0);
        end
        for (sram_i = 0; sram_i < SRAM_WORDS; sram_i = sram_i + 1) begin
            initial_sram[sram_i] = 64'd0;
            initial_data_sram[sram_i] = 64'd0;
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
        if ($value$plusargs("lnp64_data_hex=%s", data_hex_path)) begin
            $readmemh(data_hex_path, initial_data_sram);
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
        mem_sram_word_index = sram_word_index(mem_addr);
        mem_sram_next_word_index = sram_word_index(mem_addr + (64'd8 - {61'd0, mem_addr[2:0]}));
        mem_byte_lane = mem_addr[2:0];
        mem_half_lane = mem_addr[2:1];
        mem_word_upper = mem_addr[2];
        dma_argblock_addr = gpr[dec.rs1];
        dma_op = load_double_unaligned(dma_argblock_addr);
        dma_dst = load_double_unaligned(dma_argblock_addr + 64'd8);
        dma_src_or_value = load_double_unaligned(dma_argblock_addr + 64'd16);
        dma_len = load_double_unaligned(dma_argblock_addr + 64'd24);
        dma_buffer = load_double_unaligned(dma_argblock_addr + 64'd32);
        dma_dst_sram_word_index = sram_word_index(dma_dst);
        dma_dst_next_sram_word_index = sram_word_index(dma_dst + (64'd8 - {61'd0, dma_dst[2:0]}));
        dma_dst_byte_lane = dma_dst[2:0];
        dma_dst_heap_valid = heap_range_valid(dma_dst, dma_len);
        dma_src_heap_valid = heap_range_valid(dma_src_or_value, dma_len);
        dma_scope_valid = dma_buffer_ref_matches(dma_buffer) &&
            range_within(dma_buffer_object_addr, dma_buffer_object_len, dma_dst, dma_len) &&
            (dma_op != 64'd1 ||
                range_within(dma_buffer_object_addr, dma_buffer_object_len, dma_src_or_value, dma_len));
        object_argblock_addr = gpr[dec.rs1];
        object_op = load_double_unaligned(object_argblock_addr);
        object_kind = load_double_unaligned(object_argblock_addr + 64'd8);
        object_fd_req = load_double_unaligned(object_argblock_addr + 64'd24);
        object_dma_addr = load_double_unaligned(object_argblock_addr + 64'd40);
        object_dma_len = load_double_unaligned(object_argblock_addr + 64'd48);
        object_fd_store_addr = object_argblock_addr + 64'd24;
        object_fd_store_word_index = sram_word_index(object_fd_store_addr);
        object_fd_store_next_word_index = sram_word_index(
            object_fd_store_addr + (64'd8 - {61'd0, object_fd_store_addr[2:0]})
        );
        object_fd_store_byte_lane = object_fd_store_addr[2:0];
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
            cmp_below <= 1'b0;
            cmp_above <= 1'b0;
            return_stack_depth <= '0;
            link_register <= 64'd0;
            pending_unsupported <= 1'b0;
            command_pc <= 32'd0;
            heap_next <= HEAP_ARCH_BASE;
            heap_alloc_next_slot <= 2'd0;
            dma_buffer_object_valid <= 1'b0;
            dma_buffer_object_revoked <= 1'b0;
            dma_buffer_object_fd <= RTL_DMA_BUFFER_DEFAULT_FD;
            dma_buffer_object_addr <= 64'd0;
            dma_buffer_object_len <= 64'd0;
            topology_record_valid <= 1'b0;
            topology_record_base <= 64'd0;
            for (i = 0; i < 32; i = i + 1) begin
                gpr[i] <= 64'd0;
            end
            for (i = 0; i < 4; i = i + 1) begin
                heap_alloc_ptr[i] <= 64'd0;
                heap_alloc_size[i] <= 64'd0;
                heap_alloc_valid[i] <= 1'b0;
            end
            for (i = 0; i < SRAM_WORDS; i = i + 1) begin
                sram[i] = initial_sram[i];
            end
            for (i = 0; i < SRAM_WORDS - DATA_SRAM_BASE_WORD; i = i + 1) begin
                sram[DATA_SRAM_BASE_WORD + i] = initial_data_sram[i];
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
                            LNP64_OP_AUIPC_LITERAL: begin
                                gpr[dec.rd] <= FLAT_EXEC_BASE_ADDR + {30'd0, pc, 2'd0} + {{32{program_rom[pc + 32'd1][31]}}, program_rom[pc + 32'd1]};
                                pc <= pc + 32'd2;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_FENCE: begin
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_ISYNC: begin
                                gpr[dec.rd] <= 64'd0;
                                pc <= pc + 32'd1;
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
                            LNP64_OP_MULH: begin
                                gpr[dec.rd] <= mulh_signed(gpr[dec.rs1], gpr[dec.rs2]);
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_MULHU: begin
                                gpr[dec.rd] <= mulh_unsigned(gpr[dec.rs1], gpr[dec.rs2]);
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_MULHSU: begin
                                gpr[dec.rd] <= mulh_signed_unsigned(gpr[dec.rs1], gpr[dec.rs2]);
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
                            LNP64_OP_CLZ: begin
                                gpr[dec.rd] <= clz64(gpr[dec.rs1]);
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_CTZ: begin
                                gpr[dec.rd] <= ctz64(gpr[dec.rs1]);
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_POPCNT: begin
                                gpr[dec.rd] <= popcnt64(gpr[dec.rs1]);
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_ROL: begin
                                gpr[dec.rd] <= (gpr[dec.rs1] << gpr[dec.rs2][5:0]) | (gpr[dec.rs1] >> (64 - gpr[dec.rs2][5:0]));
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_ROR: begin
                                gpr[dec.rd] <= (gpr[dec.rs1] >> gpr[dec.rs2][5:0]) | (gpr[dec.rs1] << (64 - gpr[dec.rs2][5:0]));
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_BSWAP16: begin
                                gpr[dec.rd] <= {48'd0, gpr[dec.rs1][7:0], gpr[dec.rs1][15:8]};
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_BSWAP32: begin
                                gpr[dec.rd] <= {32'd0, gpr[dec.rs1][7:0], gpr[dec.rs1][15:8], gpr[dec.rs1][23:16], gpr[dec.rs1][31:24]};
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_BSWAP64: begin
                                gpr[dec.rd] <= bswap64(gpr[dec.rs1]);
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
                                cmp_below <= gpr[dec.rd] < gpr[dec.rs1];
                                cmp_above <= gpr[dec.rd] > gpr[dec.rs1];
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_CMPU: begin
                                cmp_zero <= gpr[dec.rd] == gpr[dec.rs1];
                                cmp_negative <= 1'b0;
                                cmp_greater <= 1'b0;
                                cmp_below <= gpr[dec.rd] < gpr[dec.rs1];
                                cmp_above <= gpr[dec.rd] > gpr[dec.rs1];
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_CSEL_EQ,
                            LNP64_OP_CSEL_NE,
                            LNP64_OP_CSEL_LT,
                            LNP64_OP_CSEL_GT,
                            LNP64_OP_CSEL_LE,
                            LNP64_OP_CSEL_GE,
                            LNP64_OP_CSEL_ULT,
                            LNP64_OP_CSEL_UGT,
                            LNP64_OP_CSEL_ULE,
                            LNP64_OP_CSEL_UGE: begin
                                gpr[dec.rd] <= csel_condition(dec.opcode) ? gpr[dec.rs1] : gpr[dec.rs2];
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_CSET_EQ,
                            LNP64_OP_CSET_NE,
                            LNP64_OP_CSET_LT,
                            LNP64_OP_CSET_GT,
                            LNP64_OP_CSET_LE,
                            LNP64_OP_CSET_GE,
                            LNP64_OP_CSET_ULT,
                            LNP64_OP_CSET_UGT,
                            LNP64_OP_CSET_ULE,
                            LNP64_OP_CSET_UGE: begin
                                gpr[dec.rd] <= {63'd0, csel_condition(dec.opcode)};
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
                                if (return_stack_depth < RETURN_STACK_DEPTH_VALUE) begin
                                    return_stack[return_stack_depth[RETURN_STACK_INDEX_WIDTH-1:0]] <= pc + 32'd1;
                                    return_stack_depth <= return_stack_depth + {{RETURN_STACK_INDEX_WIDTH{1'b0}}, 1'b1};
                                end
                                link_register <= flat_exec_addr(pc + 32'd1);
                                pc <= pc + dec.imm;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_CALL_REG: begin
                                if (return_stack_depth < RETURN_STACK_DEPTH_VALUE) begin
                                    return_stack[return_stack_depth[RETURN_STACK_INDEX_WIDTH-1:0]] <= pc + 32'd1;
                                    return_stack_depth <= return_stack_depth + {{RETURN_STACK_INDEX_WIDTH{1'b0}}, 1'b1};
                                end
                                link_register <= flat_exec_addr(pc + 32'd1);
                                pc <= flat_exec_pc_word(gpr[dec.rd]);
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_LR_GET: begin
                                gpr[dec.rd] <= link_register;
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_LR_SET: begin
                                link_register <= gpr[dec.rd];
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_RET: begin
                                if (return_stack_depth != '0) begin
                                    pc <= return_stack[return_stack_depth[RETURN_STACK_INDEX_WIDTH-1:0] - {{(RETURN_STACK_INDEX_WIDTH-1){1'b0}}, 1'b1}];
                                    return_stack_depth <= return_stack_depth - {{RETURN_STACK_INDEX_WIDTH{1'b0}}, 1'b1};
                                end else begin
                                    pc <= flat_exec_pc_word(link_register);
                                end
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_ALLOC: begin
                                gpr[dec.rd] <= align_up_u64(heap_next, 64'd64);
                                heap_alloc_ptr[heap_alloc_next_slot] <= align_up_u64(heap_next, 64'd64);
                                heap_alloc_size[heap_alloc_next_slot] <= alloc_len_u64(gpr[dec.rs1]);
                                heap_alloc_valid[heap_alloc_next_slot] <= 1'b1;
                                heap_alloc_next_slot <= heap_alloc_next_slot + 2'd1;
                                heap_next <= align_up_u64(heap_next, 64'd64) + alloc_len_u64(gpr[dec.rs1]);
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_ALLOC_EX: begin
                                gpr[dec.rd] <= align_up_u64(heap_next + 64'd4096, alloc_align_u64(gpr[dec.rs2]));
                                heap_alloc_ptr[heap_alloc_next_slot] <= align_up_u64(heap_next + 64'd4096, alloc_align_u64(gpr[dec.rs2]));
                                heap_alloc_size[heap_alloc_next_slot] <= alloc_len_u64(gpr[dec.rs1]);
                                heap_alloc_valid[heap_alloc_next_slot] <= 1'b1;
                                heap_alloc_next_slot <= heap_alloc_next_slot + 2'd1;
                                heap_next <= align_up_u64(heap_next + 64'd4096, alloc_align_u64(gpr[dec.rs2])) +
                                    alloc_len_u64(gpr[dec.rs1]) + 64'd4096;
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_ALLOC_SIZE: begin
                                gpr[dec.rd] <= 64'd0;
                                for (i = 0; i < 4; i = i + 1) begin
                                    if (heap_alloc_valid[i] && heap_alloc_ptr[i] == gpr[dec.rs1]) begin
                                        gpr[dec.rd] <= heap_alloc_size[i];
                                    end
                                end
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_FREE: begin
                                for (i = 0; i < 4; i = i + 1) begin
                                    if (heap_alloc_valid[i] && heap_alloc_ptr[i] == gpr[dec.rd]) begin
                                        heap_alloc_valid[i] <= 1'b0;
                                    end
                                end
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_LD: begin
                                if (topology_record_valid && mem_addr == topology_record_base) begin
                                    gpr[dec.rd] <= 64'd1;
                                    ld_value_seen <= 64'd1;
                                end else if (topology_record_valid && mem_addr == topology_record_base + 64'd192) begin
                                    gpr[dec.rd] <= 64'd4;
                                    ld_value_seen <= 64'd4;
                                end else if (topology_record_valid && mem_addr == topology_record_base + 64'd232) begin
                                    gpr[dec.rd] <= 64'd4096;
                                    ld_value_seen <= 64'd4096;
                                end else if (topology_record_valid && mem_addr == topology_record_base + 64'd256) begin
                                    gpr[dec.rd] <= 64'd5;
                                    ld_value_seen <= 64'd5;
                                end else if (topology_record_valid && mem_addr == topology_record_base + 64'd272) begin
                                    gpr[dec.rd] <= 64'd4096;
                                    ld_value_seen <= 64'd4096;
                                end else begin
                                    gpr[dec.rd] <= load_double_unaligned(mem_addr);
                                    ld_value_seen <= load_double_unaligned(mem_addr);
                                end
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_LD_W: begin
                                gpr[dec.rd] <= load_word_lane(sram[mem_sram_word_index], mem_word_upper);
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_LD_H: begin
                                gpr[dec.rd] <= load_half_lane(sram[mem_sram_word_index], mem_half_lane);
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_LD_B: begin
                                gpr[dec.rd] <= load_byte_lane(sram[mem_sram_word_index], mem_byte_lane);
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_ST: begin
                                sram[mem_sram_word_index] <= store_double_low_word(
                                    sram[mem_sram_word_index],
                                    mem_byte_lane,
                                    gpr[dec.rd]
                                );
                                if (mem_byte_lane != 3'd0) begin
                                    sram[mem_sram_next_word_index] <= store_double_high_word(
                                        sram[mem_sram_next_word_index],
                                        mem_byte_lane,
                                        gpr[dec.rd]
                                    );
                                end
                                dcache_writeback <= 1'b1;
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_ST_W: begin
                                sram[mem_sram_word_index] <= store_word_lane(sram[mem_sram_word_index], mem_word_upper, gpr[dec.rd][31:0]);
                                dcache_writeback <= 1'b1;
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_ST_H: begin
                                sram[mem_sram_word_index] <= store_half_lane(sram[mem_sram_word_index], mem_half_lane, gpr[dec.rd][15:0]);
                                dcache_writeback <= 1'b1;
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_ST_B: begin
                                sram[mem_sram_word_index] <= store_byte_lane(sram[mem_sram_word_index], mem_byte_lane, gpr[dec.rd][7:0]);
                                dcache_writeback <= 1'b1;
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_AMO_SWAP: begin
                                gpr[dec.rd] <= sram[sram_word_index(gpr[dec.rs1])];
                                sram[sram_word_index(gpr[dec.rs1])] <= gpr[dec.rs2];
                                dcache_writeback <= 1'b1;
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_AMO_ADD: begin
                                gpr[dec.rd] <= sram[sram_word_index(gpr[dec.rs1])];
                                sram[sram_word_index(gpr[dec.rs1])] <= sram[sram_word_index(gpr[dec.rs1])] + gpr[dec.rs2];
                                dcache_writeback <= 1'b1;
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_AMO_AND: begin
                                gpr[dec.rd] <= sram[sram_word_index(gpr[dec.rs1])];
                                sram[sram_word_index(gpr[dec.rs1])] <= sram[sram_word_index(gpr[dec.rs1])] & gpr[dec.rs2];
                                dcache_writeback <= 1'b1;
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_AMO_OR: begin
                                gpr[dec.rd] <= sram[sram_word_index(gpr[dec.rs1])];
                                sram[sram_word_index(gpr[dec.rs1])] <= sram[sram_word_index(gpr[dec.rs1])] | gpr[dec.rs2];
                                dcache_writeback <= 1'b1;
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_AMO_XOR: begin
                                gpr[dec.rd] <= sram[sram_word_index(gpr[dec.rs1])];
                                sram[sram_word_index(gpr[dec.rs1])] <= sram[sram_word_index(gpr[dec.rs1])] ^ gpr[dec.rs2];
                                dcache_writeback <= 1'b1;
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_LOCK_CMPXCHG: begin
                                gpr[dec.rd] <= sram[sram_word_index(gpr[dec.rs1])];
                                if (sram[sram_word_index(gpr[dec.rs1])] == gpr[dec.rs2]) begin
                                    sram[sram_word_index(gpr[dec.rs1])] <= gpr[dec.rs3];
                                    dcache_writeback <= 1'b1;
                                end
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_DMA_CTL: begin
                                if (dma_buffer != 64'd0 && !dma_buffer_ref_matches(dma_buffer)) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EBADF;
                                end else if (dma_buffer != 64'd0 && dma_buffer_ref_revoked(dma_buffer)) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= RTL_ERR_ESTALE;
                                end else if (dma_buffer != 64'd0 && !dma_scope_valid) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EFAULT;
                                end else if (dma_op == 64'd2) begin
                                    if (dma_buffer == 64'd0 && !dma_dst_heap_valid) begin
                                        gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                        errno_reg <= LNP64_ERR_EFAULT;
                                    end else if (dma_len == 64'd0) begin
                                        gpr[dec.rd] <= 64'd0;
                                        errno_reg <= LNP64_ERR_OK;
                                    end else if (dma_len == 64'd1) begin
                                        sram[dma_dst_sram_word_index] <= store_byte_lane(
                                            sram[dma_dst_sram_word_index],
                                            dma_dst_byte_lane,
                                            dma_src_or_value[7:0]
                                        );
                                        dcache_writeback <= 1'b1;
                                        gpr[dec.rd] <= 64'd1;
                                        errno_reg <= LNP64_ERR_OK;
                                    end else begin
                                        gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                        errno_reg <= LNP64_ERR_ENOTSUP;
                                    end
                                end else if (dma_op == 64'd1) begin
                                    if (dma_buffer == 64'd0 && (!dma_dst_heap_valid || !dma_src_heap_valid)) begin
                                        gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                        errno_reg <= LNP64_ERR_EFAULT;
                                    end else if (dma_len == 64'd0) begin
                                        gpr[dec.rd] <= 64'd0;
                                        errno_reg <= LNP64_ERR_OK;
                                    end else if (dma_len == 64'd8) begin
                                        sram[dma_dst_sram_word_index] <= store_double_low_word(
                                            sram[dma_dst_sram_word_index],
                                            dma_dst_byte_lane,
                                            load_double_unaligned(dma_src_or_value)
                                        );
                                        if (dma_dst_byte_lane != 3'd0) begin
                                            sram[dma_dst_next_sram_word_index] <= store_double_high_word(
                                                sram[dma_dst_next_sram_word_index],
                                                dma_dst_byte_lane,
                                                load_double_unaligned(dma_src_or_value)
                                            );
                                        end
                                        dcache_writeback <= 1'b1;
                                        gpr[dec.rd] <= 64'd8;
                                        errno_reg <= LNP64_ERR_OK;
                                    end else begin
                                        gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                        errno_reg <= LNP64_ERR_ENOTSUP;
                                    end
                                end else begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EINVAL;
                                end
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
                                    64'd1: gpr[dec.rd] <= 64'd1;
                                    64'd2: gpr[dec.rd] <= 64'd4096;
                                    64'd5: gpr[dec.rd] <= 64'd255;
                                    64'd27: gpr[dec.rd] <= 64'd511;
                                    64'd29: gpr[dec.rd] <= 64'd511;
                                    64'd30: gpr[dec.rd] <= 64'd5;
                                    64'd31: gpr[dec.rd] <= 64'd1;
                                    64'd53: gpr[dec.rd] <= {32'd0, topology_active_window_count};
                                    64'd65: begin
                                        gpr[dec.rd] <= min_u64(gpr[dec.rs3], 64'd320);
                                        topology_record_valid <= gpr[dec.rs3] >= 64'd280;
                                        topology_record_base <= gpr[dec.rs2];
                                    end
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
                            LNP64_OP_WRITE_FD: begin
                                gpr[1] <= gpr[dec.rs2];
                                errno_reg <= LNP64_ERR_OK;
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
                                if (object_op == OBJECT_OP_CREATE &&
                                    object_kind == OBJECT_KIND_DMA_BUFFER &&
                                    object_dma_len != 64'd0 &&
                                    heap_range_valid(object_dma_addr, object_dma_len) &&
                                    (object_fd_req == 64'd0 || object_fd_req == RTL_DMA_BUFFER_DEFAULT_FD)) begin
                                    dma_buffer_object_valid <= 1'b1;
                                    dma_buffer_object_revoked <= 1'b0;
                                    dma_buffer_object_fd <= RTL_DMA_BUFFER_DEFAULT_FD;
                                    dma_buffer_object_addr <= object_dma_addr;
                                    dma_buffer_object_len <= object_dma_len;
                                    sram[object_fd_store_word_index] <= store_double_low_word(
                                        sram[object_fd_store_word_index],
                                        object_fd_store_byte_lane,
                                        RTL_DMA_BUFFER_DEFAULT_FD
                                    );
                                    if (object_fd_store_byte_lane != 3'd0) begin
                                        sram[object_fd_store_next_word_index] <= store_double_high_word(
                                            sram[object_fd_store_next_word_index],
                                            object_fd_store_byte_lane,
                                            RTL_DMA_BUFFER_DEFAULT_FD
                                        );
                                    end
                                    dcache_writeback <= 1'b1;
                                    gpr[dec.rd] <= RTL_DMA_BUFFER_DEFAULT_FD;
                                    errno_reg <= LNP64_ERR_OK;
                                    pc <= pc + 32'd1;
                                    retired_count <= retired_count + 32'd1;
                                    retire_submit_valid <= 1'b1;
                                    retire_submit_record <= retire_submit_next;
                                end else if (object_op == OBJECT_OP_CREATE &&
                                    object_kind == OBJECT_KIND_DMA_BUFFER &&
                                    object_dma_len == 64'd0) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EINVAL;
                                    pc <= pc + 32'd1;
                                    retired_count <= retired_count + 32'd1;
                                    retire_submit_valid <= 1'b1;
                                    retire_submit_record <= retire_submit_next;
                                end else if (object_op == OBJECT_OP_CREATE &&
                                    object_kind == OBJECT_KIND_DMA_BUFFER) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EFAULT;
                                    pc <= pc + 32'd1;
                                    retired_count <= retired_count + 32'd1;
                                    retire_submit_valid <= 1'b1;
                                    retire_submit_record <= retire_submit_next;
                                end else begin
                                    pending_unsupported <= 1'b0;
                                    command_pc <= pc;
                                    state <= CORE_SEND_CMD;
                                end
                            end
                            LNP64_OP_CAP_REVOKE: begin
                                if (dma_buffer_ref_matches(object_op) && !dma_buffer_object_revoked) begin
                                    dma_buffer_object_revoked <= 1'b1;
                                    gpr[dec.rd] <= 64'd1;
                                    errno_reg <= LNP64_ERR_OK;
                                end else if (dma_buffer_ref_matches(object_op) && dma_buffer_object_revoked) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= RTL_ERR_ESTALE;
                                end else begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EBADF;
                                end
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
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

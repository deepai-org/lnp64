`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_core_tile #(
    parameter int TILE_ID = 0,
    parameter int PROGRAM_WORDS = 1024,
    parameter int SRAM_WORDS = 2176,
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
    output logic m1_commit_valid,
    output lnp64_m1_cap_commit_t m1_commit,
    output lnp64_m1_state_projection_t m1_pre_state_projection,
    output lnp64_m1_state_projection_t m1_state_projection,
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
        CORE_SWITCH,
        CORE_DONE
    } core_state_e;

    core_state_e state;
    lnp64_decode_t dec;
    logic [31:0] instr;
    logic [7:0] raw_opcode;
    logic [31:0] pc;
    logic [31:0] next_op_id;
    localparam int RETURN_STACK_INDEX_WIDTH = $clog2(RETURN_STACK_DEPTH);
    logic [63:0] gpr [0:31];
    logic [63:0] thread_gpr [0:1][0:31];
    logic [31:0] thread_pc [0:1];
    logic [31:0] thread_return_stack [0:1][0:RETURN_STACK_DEPTH-1];
    logic [RETURN_STACK_INDEX_WIDTH:0] thread_return_stack_depth [0:1];
    logic [63:0] thread_link_register [0:1];
    logic thread_cmp_zero [0:1];
    logic thread_cmp_negative [0:1];
    logic thread_cmp_greater [0:1];
    logic thread_cmp_below [0:1];
    logic thread_cmp_above [0:1];
    logic thread_active [0:1];
    logic thread_completed [0:1];
    logic [63:0] thread_exit_code [0:1];
    logic active_thread_slot;
    logic next_thread_slot;
    logic [31:0] active_tid;
    logic [63:0] sram [0:SRAM_WORDS-1];
    logic [63:0] initial_sram [0:SRAM_WORDS-1];
    logic [63:0] initial_data_sram [0:SRAM_WORDS-1];
    logic [31:0] program_rom [0:PROGRAM_WORDS-1];
    localparam logic [63:0] HEAP_ARCH_BASE = 64'h0000_0000_0010_f000;
    localparam logic [63:0] MMAP_ARCH_BASE = 64'h0000_0000_0020_e000;
    localparam logic [63:0] FLAT_DATA_BASE_ADDR = 64'h0000_0000_0001_0000;
    localparam logic [63:0] FLAT_EXEC_BASE_ADDR = 64'h0000_0000_0000_1000;
    localparam logic [63:0] FLAT_EXEC_INITIAL_SP = 64'h0000_0000_0067_2000;
    localparam logic [63:0] FLAT_EXEC_STACK_BASE_ADDR = 64'h0000_0000_0067_0000;
    localparam logic [63:0] FLAT_EXEC_STACK_WINDOW_BYTES = 64'd16384;
    localparam logic [63:0] FLAT_EXEC_CALL_FRAME_BYTES = 64'd1024;
    localparam logic [63:0] FLAT_EXEC_CHILD_SP = 64'h0000_0000_0067_3000;
    localparam logic [63:0] OBJECT_OP_CREATE = 64'd1;
    localparam logic [63:0] OBJECT_KIND_COUNTER = 64'd1;
    localparam logic [63:0] OBJECT_KIND_QUEUE = 64'd2;
    localparam logic [63:0] OBJECT_KIND_MEMORY_OBJECT = 64'd3;
    localparam logic [63:0] OBJECT_KIND_DMA_BUFFER = 64'd4;
    localparam logic [63:0] OBJECT_KIND_TIMER = 64'd6;
    localparam logic [63:0] OBJECT_PROFILE_PIPE = 64'd1;
    localparam logic [63:0] OBJECT_PROFILE_CALL_GATE = 64'd4;
    localparam logic [63:0] CALL_MODE_SYNC = 64'd0;
    localparam logic [63:0] CALL_MODE_ASYNC = 64'd1;
    localparam logic [63:0] CALL_MODE_HANDOFF = 64'd2;
    localparam logic [63:0] DOMAIN_OP_CREATE = 64'd1;
    localparam logic [63:0] DOMAIN_OP_CONFIGURE = 64'd2;
    localparam logic [63:0] DOMAIN_OP_QUERY = 64'd3;
    localparam logic [63:0] DOMAIN_OP_FREEZE = 64'd4;
    localparam logic [63:0] DOMAIN_OP_RESUME = 64'd5;
    localparam logic [63:0] DOMAIN_OP_DESTROY = 64'd6;
    localparam logic [63:0] DOMAIN_ROOT_ID = 64'd1;
    localparam logic [63:0] DOMAIN_QUERY_SIZE = 64'd200;
    localparam logic [63:0] DOMAIN_STATE_ACTIVE = 64'd0;
    localparam logic [63:0] DOMAIN_STATE_DESTROYED = 64'd2;
    localparam logic [63:0] RTL_DMA_BUFFER_DEFAULT_FD = 64'd3;
    localparam logic [63:0] RTL_DMA_BUFFER_TOKEN = 64'h4000_0000_0000_0203;
    localparam logic [63:0] FDR_TOKEN_MARKER = 64'h4000_0000_0000_0000;
    localparam logic [63:0] FDR_TOKEN_INDEX_MASK = 64'h0000_0000_0000_00ff;
    localparam logic [63:0] CAP_DUP_FLAG_SEAL = 64'd1;
    localparam logic [63:0] CAP_RIGHT_ALL = 64'h0000_0000_0000_01ff;
    localparam logic [63:0] CAP_RIGHT_CALL = 64'h0000_0000_0000_0020;
    localparam logic [63:0] CAP_RIGHT_DUP = 64'h0000_0000_0000_0040;
    localparam logic [63:0] CAP_RIGHT_REVOKE = 64'h0000_0000_0000_0080;
    localparam logic [63:0] CAP_RIGHT_TRANSFER = 64'h0000_0000_0000_0100;
    localparam logic [2:0] FDR_KIND_CLOSED = 3'd0;
    localparam logic [2:0] FDR_KIND_GENERIC = 3'd1;
    localparam logic [2:0] FDR_KIND_PIPE_READER = 3'd2;
    localparam logic [2:0] FDR_KIND_PIPE_WRITER = 3'd3;
    localparam logic [2:0] FDR_KIND_CALL_GATE = 3'd4;
    localparam logic [15:0] RTL_ERR_ESTALE = 16'd116;
    localparam int unsigned FDR_SLOT_COUNT = 10;
    localparam int unsigned DOMAIN_SLOT_COUNT = 8;
    localparam int unsigned DATA_SRAM_BASE_WORD = 16;
    localparam int unsigned HEAP_SRAM_BASE_WORD = 96;
    localparam int unsigned STACK_SRAM_BASE_WORD = 128;
    logic [15:0] errno_reg;
    logic cap_engine_shadow_enabled;
    logic cmp_zero;
    logic cmp_negative;
    logic cmp_greater;
    logic cmp_below;
    logic cmp_above;
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
    logic [63:0] object_profile;
    logic [63:0] object_fd_req;
    logic [63:0] object_fd1_req;
    logic [63:0] object_dma_addr;
    logic [63:0] object_dma_len;
    logic [63:0] object_gate_domain;
    logic [63:0] object_gate_entry;
    logic [63:0] object_gate_mode;
    logic [63:0] object_gate_completion_fd;
    logic [63:0] object_gate_flags;
    logic [63:0] domain_argblock_addr;
    logic [63:0] domain_op;
    logic [63:0] domain_parent_req;
    logic [63:0] domain_generation_req;
    logic [63:0] domain_profile_req;
    logic [63:0] domain_cpu_req;
    logic [63:0] domain_memory_req;
    logic [63:0] domain_pids_req;
    logic [63:0] domain_fdrs_req;
    logic [63:0] domain_caps_req;
    logic [63:0] domain_upcalls_req;
    logic [63:0] domain_ref_id;
    int unsigned domain_ref_slot;
    int unsigned domain_create_slot;
    logic domain_ref_in_range;
    logic domain_ref_live;
    logic domain_create_slot_in_range;
    logic [63:0] object_fd_store_addr;
    logic [63:0] object_fd1_store_addr;
    int unsigned object_fd_store_word_index;
    int unsigned object_fd_store_next_word_index;
    int unsigned object_fd1_store_word_index;
    logic [2:0] object_fd_store_byte_lane;
    logic [2:0] object_fd1_store_byte_lane;
    int unsigned object_memory_fd;
    logic object_memory_fd_in_range;
    int unsigned object_single_fd;
    logic object_single_fd_in_range;
    logic dma_buffer_object_valid;
    logic dma_buffer_object_revoked;
    logic [63:0] dma_buffer_object_fd;
    logic [63:0] dma_buffer_object_addr;
    logic [63:0] dma_buffer_object_len;
    logic [63:0] heap_next;
    logic [63:0] mmap_next;
    logic [63:0] heap_alloc_ptr [0:3];
    logic [63:0] heap_alloc_size [0:3];
    logic heap_alloc_valid [0:3];
    logic [1:0] heap_alloc_next_slot;
    logic fdr_valid [0:FDR_SLOT_COUNT-1];
    logic fdr_revoked [0:FDR_SLOT_COUNT-1];
    logic [63:0] fdr_generation [0:FDR_SLOT_COUNT-1];
    logic [63:0] fdr_rights [0:FDR_SLOT_COUNT-1];
    logic [63:0] fdr_lineage [0:FDR_SLOT_COUNT-1];
    logic [2:0] fdr_kind [0:FDR_SLOT_COUNT-1];
    logic call_gate_valid [0:FDR_SLOT_COUNT-1];
    logic [63:0] call_gate_entry [0:FDR_SLOT_COUNT-1];
    logic [63:0] call_gate_mode [0:FDR_SLOT_COUNT-1];
    logic [63:0] call_gate_completion_fd [0:FDR_SLOT_COUNT-1];
    logic call_continuation_valid;
    logic [31:0] call_continuation_return_pc;
    logic [7:0] call_continuation_result_reg;
    logic [63:0] call_next_op_id;
    logic [63:0] domain_next_id;
    logic domain_valid [0:DOMAIN_SLOT_COUNT-1];
    logic domain_destroyed [0:DOMAIN_SLOT_COUNT-1];
    logic [63:0] domain_generation [0:DOMAIN_SLOT_COUNT-1];
    logic [63:0] domain_parent [0:DOMAIN_SLOT_COUNT-1];
    logic [63:0] domain_depth [0:DOMAIN_SLOT_COUNT-1];
    logic [63:0] domain_profile [0:DOMAIN_SLOT_COUNT-1];
    logic [63:0] domain_cpu_limit [0:DOMAIN_SLOT_COUNT-1];
    logic [63:0] domain_memory_limit [0:DOMAIN_SLOT_COUNT-1];
    logic [63:0] domain_pids_limit [0:DOMAIN_SLOT_COUNT-1];
    logic [63:0] domain_fdrs_limit [0:DOMAIN_SLOT_COUNT-1];
    logic [63:0] domain_cap_mask [0:DOMAIN_SLOT_COUNT-1];
    logic [63:0] domain_upcall_mask [0:DOMAIN_SLOT_COUNT-1];
    logic [63:0] domain_child_count [0:DOMAIN_SLOT_COUNT-1];
    logic domain_frozen [0:DOMAIN_SLOT_COUNT-1];
    logic cap_queue_valid;
    logic [63:0] cap_queue_rights;
    logic [63:0] cap_queue_lineage;
    logic [63:0] cap_queue_generation;
    logic cap_queue_revoked;
    logic pipe_payload_valid [0:FDR_SLOT_COUNT-1];
    logic [63:0] pipe_payload_value [0:FDR_SLOT_COUNT-1];
    logic [3:0] pipe_payload_len [0:FDR_SLOT_COUNT-1];
    logic memory_object_valid;
    logic [63:0] memory_object_lineage;
    logic [63:0] memory_object_len;
    logic memory_object_byte_valid;
    logic [7:0] memory_object_byte_value;
    logic event_counter_valid;
    logic [63:0] event_counter_lineage;
    logic [63:0] event_counter_value;
    logic timer_valid;
    logic [63:0] timer_lineage;
    logic [63:0] timer_expirations;
    logic topology_record_valid;
    logic [63:0] topology_record_base;
    int unsigned mem_sram_word_index;
    int unsigned mem_sram_next_word_index;
    int unsigned dma_dst_sram_word_index;
    logic [2:0] mem_byte_lane;
    logic [2:0] dma_dst_byte_lane;
    logic [1:0] mem_half_lane;
    logic mem_word_upper;
    logic [63:0] cap_src_value;
    logic [63:0] cap_dst_req;
    logic [63:0] cap_rights_req;
    logic [63:0] cap_flags;
    int unsigned cap_src_fd;
    int unsigned cap_arg1_fd;
    int unsigned cap_dst_fd;
    logic cap_src_fd_in_range;
    logic cap_dst_fd_in_range;
    logic cap_src_is_token;
    logic cap_src_token_shape_valid;
    logic cap_src_generation_matches;
    logic cap_src_live;
    logic cap_src_stale;
    logic [63:0] cap_dup_rights;
    logic cap_dup_rights_subset;
    logic [63:0] cap_revoke_count;
    logic [63:0] cap_recv_rights;
    logic cap_recv_rights_subset;
    int unsigned pipe_fd;
    int unsigned pipe_queue_slot;
    logic pipe_fd_in_range;
    logic pipe_fd_is_token;
    logic pipe_fd_token_shape_valid;
    logic pipe_fd_generation_matches;
    logic pipe_fd_token_stale;
    int unsigned pipe_alloc_reader_fd;
    int unsigned pipe_alloc_writer_fd;
    logic pipe_alloc_available;
    logic [3:0] pipe_pull_len;
    int unsigned close_fd;
    logic close_fd_in_range;
    logic close_fd_is_token;
    logic close_fd_token_shape_valid;
    logic close_fd_generation_matches;
    logic close_fd_token_stale;
    int unsigned file_open_fd;
    logic file_open_available;
    logic [63:0] dynamic_file_read_count;
    int unsigned static_fd;
    logic static_fd_in_range;
    logic static_fd_is_memory_object;
    logic static_fd_is_event_counter;
    logic static_fd_is_timer;
    int unsigned static_fd_buf_word_index;
    int unsigned static_fd_buf_next_word_index;
    logic [2:0] static_fd_buf_byte_lane;
    int unsigned await_fd;
    logic await_fd_in_range;
    logic await_fd_ready;
    int unsigned call_gate_fd;
    logic call_gate_fd_in_range;
    logic call_gate_fd_live;
    logic call_gate_completion_fd_in_range;
    logic call_gate_completion_is_event_counter;
    int unsigned pipe_buf_word_index;
    logic [2:0] pipe_buf_byte_lane;
    lnp64_m1_cap_commit_t m1_commit_next;
    int unsigned m1_projection_root_fd;
    int unsigned m1_projection_consumer_fd;
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
            if (addr >= FLAT_EXEC_STACK_BASE_ADDR &&
                addr < (FLAT_EXEC_STACK_BASE_ADDR + FLAT_EXEC_STACK_WINDOW_BYTES)) begin
                rel_addr = addr - FLAT_EXEC_STACK_BASE_ADDR;
                sram_word_index = STACK_SRAM_BASE_WORD + rel_addr[12:3];
            end else if (addr >= HEAP_ARCH_BASE) begin
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

    task automatic store_double_unaligned_next(
        input logic [63:0] addr,
        input logic [63:0] value
    );
        int unsigned word_index;
        int unsigned next_word_index;
        logic [2:0] lane;
        begin
            lane = addr[2:0];
            word_index = sram_word_index(addr);
            next_word_index = sram_word_index(addr + (64'd8 - {61'd0, lane}));
            sram[word_index] <= store_double_low_word(sram[word_index], lane, value);
            if (lane != 3'd0) begin
                sram[next_word_index] <= store_double_high_word(sram[next_word_index], lane, value);
            end
        end
    endtask

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

    function automatic logic [63:0] low_bytes_mask(input logic [3:0] len);
        begin
            unique case (len)
                4'd0: low_bytes_mask = 64'h0000_0000_0000_0000;
                4'd1: low_bytes_mask = 64'h0000_0000_0000_00ff;
                4'd2: low_bytes_mask = 64'h0000_0000_0000_ffff;
                4'd3: low_bytes_mask = 64'h0000_0000_00ff_ffff;
                4'd4: low_bytes_mask = 64'h0000_0000_ffff_ffff;
                4'd5: low_bytes_mask = 64'h0000_00ff_ffff_ffff;
                4'd6: low_bytes_mask = 64'h0000_ffff_ffff_ffff;
                4'd7: low_bytes_mask = 64'h00ff_ffff_ffff_ffff;
                default: low_bytes_mask = 64'hffff_ffff_ffff_ffff;
            endcase
        end
    endfunction

    function automatic logic [63:0] store_payload_low_word(
        input logic [63:0] word,
        input logic [2:0] lane,
        input logic [63:0] value,
        input logic [3:0] len
    );
        begin
            store_payload_low_word = word;
            for (int byte_idx = 0; byte_idx < 8; byte_idx = byte_idx + 1) begin
                if (byte_idx >= lane && (byte_idx - lane) < len) begin
                    store_payload_low_word[byte_idx * 8 +: 8] =
                        value[(byte_idx - lane) * 8 +: 8];
                end
            end
        end
    endfunction

    function automatic logic [63:0] store_payload_high_word(
        input logic [63:0] word,
        input logic [2:0] lane,
        input logic [63:0] value,
        input logic [3:0] len
    );
        begin
            store_payload_high_word = word;
            for (int byte_idx = 0; byte_idx < 8; byte_idx = byte_idx + 1) begin
                if (byte_idx < lane && (8 - lane + byte_idx) < len) begin
                    store_payload_high_word[byte_idx * 8 +: 8] =
                        value[(8 - lane + byte_idx) * 8 +: 8];
                end
            end
        end
    endfunction

    task automatic store_payload_unaligned_next(
        input logic [63:0] addr,
        input logic [63:0] value,
        input logic [3:0] len
    );
        int unsigned word_index;
        int unsigned next_word_index;
        logic [2:0] lane;
        begin
            lane = addr[2:0];
            word_index = sram_word_index(addr);
            next_word_index = sram_word_index(addr + (64'd8 - {61'd0, lane}));
            sram[word_index] <= store_payload_low_word(sram[word_index], lane, value, len);
            if ({1'b0, lane} + len > 4'd8) begin
                sram[next_word_index] <= store_payload_high_word(sram[next_word_index], lane, value, len);
            end
        end
    endtask

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

    function automatic int unsigned fdr_value_fd(input logic [63:0] value);
        logic [63:0] fd_bits;
        begin
            fd_bits = (value < 64'd256) ? value : (value & FDR_TOKEN_INDEX_MASK);
            fdr_value_fd = fd_bits[7:0];
        end
    endfunction

    function automatic logic [63:0] fdr_token(input int unsigned fd);
        begin
            fdr_token = FDR_TOKEN_MARKER | (fdr_generation[fd] << 8) | {56'd0, fd[7:0]};
        end
    endfunction

    function automatic int unsigned m1_current_consumer_fd;
        begin
            unique case (dec.opcode)
                LNP64_OP_CAP_SEND: begin
                    m1_current_consumer_fd = (cap_arg1_fd < FDR_SLOT_COUNT) ? cap_arg1_fd : 0;
                end
                LNP64_OP_CAP_DUP, LNP64_OP_CAP_RECV: begin
                    m1_current_consumer_fd = cap_dst_fd_in_range ? cap_dst_fd : 0;
                end
                default: begin
                    m1_current_consumer_fd = cap_src_fd_in_range ? cap_src_fd : 0;
                end
            endcase
        end
    endfunction

    function automatic lnp64_m1_state_projection_t build_m1_state_projection(
        input logic [7:0] op,
        input logic [15:0] status,
        input int unsigned root_fd,
        input int unsigned consumer_fd
    );
        lnp64_m1_state_projection_t projection;
        begin
            projection = '0;
            projection.op = op;
            projection.status = status;
            if (root_fd < FDR_SLOT_COUNT) begin
                projection.object_gen = fdr_generation[root_fd][31:0];
                projection.root_object_id = fdr_lineage[root_fd][31:0];
                projection.root_generation = fdr_generation[root_fd][31:0];
                projection.root_domain_id = 32'd1;
                projection.root_lineage_epoch = fdr_lineage[root_fd][31:0];
                projection.root_sealed = 1'b0;
                projection.root_rights = (fdr_valid[root_fd] && !fdr_revoked[root_fd]) ?
                    fdr_rights[root_fd] : 64'd0;
                projection.has_revoked_generation = fdr_revoked[root_fd];
                projection.revoked_generation = fdr_revoked[root_fd] ?
                    fdr_generation[root_fd][31:0] : 32'd0;
            end
            if (consumer_fd < FDR_SLOT_COUNT) begin
                projection.consumer_object_id = fdr_lineage[consumer_fd][31:0];
                projection.consumer_generation = fdr_generation[consumer_fd][31:0];
                projection.consumer_domain_id = 32'd2;
                projection.consumer_lineage_epoch = fdr_lineage[consumer_fd][31:0];
                projection.consumer_sealed = 1'b0;
                projection.consumer_rights = (fdr_valid[consumer_fd] && !fdr_revoked[consumer_fd]) ?
                    fdr_rights[consumer_fd] : 64'd0;
            end
            projection.sent_valid = cap_queue_valid && !cap_queue_revoked;
            if (projection.sent_valid) begin
                projection.sent_object_id = cap_queue_lineage[31:0];
                projection.sent_generation = cap_queue_generation[31:0];
                projection.sent_domain_id = 32'd2;
                projection.sent_lineage_epoch = cap_queue_lineage[31:0];
                projection.sent_sealed = 1'b0;
                projection.sent_rights = cap_queue_rights;
            end
            projection.transfer_valid = projection.sent_valid;
            projection.stale_rejected = status == RTL_ERR_ESTALE;
            projection.revoked_rejected = status == RTL_ERR_ESTALE;
            projection.failed_no_authority =
                status == LNP64_ERR_EPERM || status == LNP64_ERR_EBADF;
            projection.full_was_explicit = status == LNP64_ERR_EAGAIN;
            build_m1_state_projection = projection;
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

    function automatic logic [63:0] div_signed64(input logic [63:0] lhs, input logic [63:0] rhs);
        logic [63:0] lhs_abs;
        logic [63:0] rhs_abs;
        logic [63:0] quotient_abs;
        begin
            if (rhs == 64'd0) begin
                div_signed64 = 64'd0;
            end else begin
                lhs_abs = lhs[63] ? (~lhs + 64'd1) : lhs;
                rhs_abs = rhs[63] ? (~rhs + 64'd1) : rhs;
                quotient_abs = lhs_abs / rhs_abs;
                div_signed64 = lhs[63] ^ rhs[63] ? (~quotient_abs + 64'd1) : quotient_abs;
            end
        end
    endfunction

    function automatic logic [63:0] rem_signed64(input logic [63:0] lhs, input logic [63:0] rhs);
        logic [63:0] lhs_abs;
        logic [63:0] rhs_abs;
        logic [63:0] remainder_abs;
        begin
            if (rhs == 64'd0) begin
                rem_signed64 = 64'd0;
            end else begin
                lhs_abs = lhs[63] ? (~lhs + 64'd1) : lhs;
                rhs_abs = rhs[63] ? (~rhs + 64'd1) : rhs;
                remainder_abs = lhs_abs % rhs_abs;
                rem_signed64 = lhs[63] ? (~remainder_abs + 64'd1) : remainder_abs;
            end
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
            raw_opcode = program_rom[pc][31:24];
        end else begin
            instr = enc_reg(8'hff, 5'd0);
            raw_opcode = 8'hff;
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
        object_profile = load_double_unaligned(object_argblock_addr + 64'd16);
        object_fd_req = load_double_unaligned(object_argblock_addr + 64'd24);
        object_fd1_req = load_double_unaligned(object_argblock_addr + 64'd32);
        object_dma_addr = load_double_unaligned(object_argblock_addr + 64'd40);
        object_dma_len = load_double_unaligned(object_argblock_addr + 64'd48);
        object_gate_domain = load_double_unaligned(object_argblock_addr + 64'd32);
        object_gate_entry = load_double_unaligned(object_argblock_addr + 64'd40);
        object_gate_mode = load_double_unaligned(object_argblock_addr + 64'd48);
        object_gate_completion_fd = load_double_unaligned(object_argblock_addr + 64'd56);
        object_gate_flags = load_double_unaligned(object_argblock_addr + 64'd64);
        domain_argblock_addr = gpr[dec.rs1];
        domain_op = load_double_unaligned(domain_argblock_addr);
        domain_parent_req = load_double_unaligned(domain_argblock_addr + 64'd8);
        domain_generation_req = load_double_unaligned(domain_argblock_addr + 64'd16);
        domain_profile_req = load_double_unaligned(domain_argblock_addr + 64'd24);
        domain_cpu_req = load_double_unaligned(domain_argblock_addr + 64'd32);
        domain_memory_req = load_double_unaligned(domain_argblock_addr + 64'd40);
        domain_pids_req = load_double_unaligned(domain_argblock_addr + 64'd48);
        domain_fdrs_req = load_double_unaligned(domain_argblock_addr + 64'd56);
        domain_caps_req = load_double_unaligned(domain_argblock_addr + 64'd64);
        domain_upcalls_req = load_double_unaligned(domain_argblock_addr + 64'd72);
        domain_ref_id = domain_parent_req == 64'd0 ? DOMAIN_ROOT_ID : domain_parent_req;
        domain_ref_slot = domain_ref_id > 64'd0 ? int'(domain_ref_id - 64'd1) : 0;
        domain_create_slot = domain_next_id > 64'd0 ? int'(domain_next_id - 64'd1) : 0;
        domain_ref_in_range = domain_ref_id > 64'd0 && domain_ref_slot < DOMAIN_SLOT_COUNT;
        domain_ref_live = domain_ref_in_range &&
            domain_valid[domain_ref_slot] &&
            !domain_destroyed[domain_ref_slot] &&
            (domain_generation_req == 64'd0 ||
                domain_generation_req == domain_generation[domain_ref_slot]);
        domain_create_slot_in_range = domain_next_id > 64'd0 && domain_create_slot < DOMAIN_SLOT_COUNT;
        object_fd_store_addr = object_argblock_addr + 64'd24;
        object_fd1_store_addr = object_argblock_addr + 64'd32;
        object_fd_store_word_index = sram_word_index(object_fd_store_addr);
        object_fd_store_next_word_index = sram_word_index(
            object_fd_store_addr + (64'd8 - {61'd0, object_fd_store_addr[2:0]})
        );
        object_fd1_store_word_index = sram_word_index(object_fd1_store_addr);
        object_fd_store_byte_lane = object_fd_store_addr[2:0];
        object_fd1_store_byte_lane = object_fd1_store_addr[2:0];
        object_memory_fd = object_fd_req == 64'd0 ? 5 : object_fd_req[31:0];
        object_memory_fd_in_range = object_fd_req == 64'd0 || object_fd_req < FDR_SLOT_COUNT;
        object_single_fd = object_fd_req == 64'd0 ? 3 : object_fd_req[31:0];
        object_single_fd_in_range = object_fd_req == 64'd0 || object_fd_req < FDR_SLOT_COUNT;
        cap_src_value = load_double_unaligned(gpr[dec.rs1]);
        cap_dst_req = load_double_unaligned(gpr[dec.rs1] + 64'd8);
        cap_rights_req = load_double_unaligned(gpr[dec.rs1] + 64'd16);
        cap_flags = load_double_unaligned(gpr[dec.rs1] + 64'd24);
        cap_src_fd = fdr_value_fd(cap_src_value);
        cap_arg1_fd = fdr_value_fd(cap_dst_req);
        cap_dst_fd = cap_dst_req == 64'd0 ? 3 : fdr_value_fd(cap_dst_req);
        cap_src_fd_in_range = cap_src_fd < FDR_SLOT_COUNT;
        cap_dst_fd_in_range = cap_dst_fd < FDR_SLOT_COUNT;
        cap_src_is_token = cap_src_value >= 64'd256;
        cap_src_token_shape_valid = (cap_src_value & FDR_TOKEN_MARKER) != 64'd0 &&
            cap_src_value[7:0] < FDR_SLOT_COUNT[7:0] &&
            ((cap_src_value & ~FDR_TOKEN_MARKER) >> 8) != 64'd0;
        cap_src_generation_matches = cap_src_fd_in_range &&
            (((cap_src_value & ~FDR_TOKEN_MARKER) >> 8) == fdr_generation[cap_src_fd]);
        cap_src_live = cap_src_fd_in_range && fdr_valid[cap_src_fd] && !fdr_revoked[cap_src_fd] &&
            (!cap_src_is_token || (cap_src_token_shape_valid && cap_src_generation_matches));
        cap_src_stale = cap_src_is_token && cap_src_token_shape_valid && cap_src_fd_in_range &&
            (!fdr_valid[cap_src_fd] || fdr_revoked[cap_src_fd] || !cap_src_generation_matches);
        cap_dup_rights = (cap_rights_req == 64'd0 && cap_src_fd_in_range) ? fdr_rights[cap_src_fd] : cap_rights_req;
        cap_dup_rights_subset = cap_src_fd_in_range && ((cap_dup_rights & ~fdr_rights[cap_src_fd]) == 64'd0);
        cap_recv_rights = cap_rights_req == 64'd0 ? cap_queue_rights : cap_rights_req;
        cap_recv_rights_subset = (cap_recv_rights & ~cap_queue_rights) == 64'd0;
        if (raw_opcode == 8'h2b || raw_opcode == 8'h2c) begin
            pipe_fd = dec.rs1;
        end else begin
            pipe_fd = fdr_value_fd(gpr[dec.rs1]);
        end
        if (pipe_fd == 4) begin
            pipe_queue_slot = 3;
        end else if (pipe_fd == 6) begin
            pipe_queue_slot = 5;
        end else if (pipe_fd == 8) begin
            pipe_queue_slot = 7;
        end else begin
            pipe_queue_slot = pipe_fd;
        end
        pipe_fd_in_range = pipe_fd < FDR_SLOT_COUNT;
        pipe_fd_is_token = gpr[dec.rs1] >= 64'd256;
        pipe_fd_token_shape_valid = (gpr[dec.rs1] & FDR_TOKEN_MARKER) != 64'd0 &&
            gpr[dec.rs1][7:0] < FDR_SLOT_COUNT[7:0] &&
            ((gpr[dec.rs1] & ~FDR_TOKEN_MARKER) >> 8) != 64'd0;
        pipe_fd_generation_matches = pipe_fd_in_range &&
            (((gpr[dec.rs1] & ~FDR_TOKEN_MARKER) >> 8) == fdr_generation[pipe_fd]);
        pipe_fd_token_stale = pipe_fd_is_token && pipe_fd_token_shape_valid && pipe_fd_in_range &&
            (!fdr_valid[pipe_fd] || fdr_revoked[pipe_fd] || !pipe_fd_generation_matches);
        pipe_pull_len = 4'd0;
        if (pipe_queue_slot < FDR_SLOT_COUNT && pipe_payload_valid[pipe_queue_slot]) begin
            if (gpr[dec.rs3] < {60'd0, pipe_payload_len[pipe_queue_slot]}) begin
                pipe_pull_len = gpr[dec.rs3][3:0];
            end else begin
                pipe_pull_len = pipe_payload_len[pipe_queue_slot];
            end
        end
        close_fd = fdr_value_fd(gpr[dec.rd]);
        close_fd_in_range = close_fd < FDR_SLOT_COUNT;
        close_fd_is_token = gpr[dec.rd] >= 64'd256;
        close_fd_token_shape_valid = (gpr[dec.rd] & FDR_TOKEN_MARKER) != 64'd0 &&
            gpr[dec.rd][7:0] < FDR_SLOT_COUNT[7:0] &&
            ((gpr[dec.rd] & ~FDR_TOKEN_MARKER) >> 8) != 64'd0;
        close_fd_generation_matches = close_fd_in_range &&
            (((gpr[dec.rd] & ~FDR_TOKEN_MARKER) >> 8) == fdr_generation[close_fd]);
        close_fd_token_stale = close_fd_is_token && close_fd_token_shape_valid && close_fd_in_range &&
            (!fdr_valid[close_fd] || fdr_revoked[close_fd] || !close_fd_generation_matches);
        file_open_fd = 3;
        file_open_available = 1'b0;
        for (int slot = 3; slot < FDR_SLOT_COUNT; slot = slot + 1) begin
            if (!file_open_available && !fdr_valid[slot]) begin
                file_open_fd = slot;
                file_open_available = 1'b1;
            end
        end
        pipe_alloc_reader_fd = 3;
        pipe_alloc_writer_fd = 4;
        pipe_alloc_available = 1'b0;
        for (int slot = 3; slot + 1 < FDR_SLOT_COUNT; slot = slot + 2) begin
            if (!pipe_alloc_available && !fdr_valid[slot] && !fdr_valid[slot + 1]) begin
                pipe_alloc_reader_fd = slot;
                pipe_alloc_writer_fd = slot + 1;
                pipe_alloc_available = 1'b1;
            end
        end
        dynamic_file_read_count = min_u64(gpr[dec.rs3], 64'd8);
        static_fd = dec.rd;
        static_fd_in_range = static_fd < FDR_SLOT_COUNT;
        static_fd_is_memory_object = 1'b0;
        static_fd_is_event_counter = 1'b0;
        static_fd_is_timer = 1'b0;
        if (static_fd_in_range) begin
            static_fd_is_memory_object = memory_object_valid &&
                fdr_lineage[static_fd] == memory_object_lineage;
            static_fd_is_event_counter = event_counter_valid &&
                fdr_lineage[static_fd] == event_counter_lineage;
            static_fd_is_timer = timer_valid &&
                fdr_lineage[static_fd] == timer_lineage;
        end
        static_fd_buf_word_index = sram_word_index(gpr[dec.rs1]);
        static_fd_buf_next_word_index = sram_word_index(gpr[dec.rs1] + (64'd8 - {61'd0, gpr[dec.rs1][2:0]}));
        static_fd_buf_byte_lane = gpr[dec.rs1][2:0];
        if (raw_opcode == 8'h2e) begin
            await_fd = dec.rs1;
        end else begin
            await_fd = fdr_value_fd(gpr[dec.rs1]);
        end
        await_fd_in_range = await_fd < FDR_SLOT_COUNT;
        await_fd_ready = 1'b0;
        if (await_fd_in_range && fdr_valid[await_fd] && !fdr_revoked[await_fd]) begin
            await_fd_ready =
                (timer_valid && fdr_lineage[await_fd] == timer_lineage && timer_expirations != 64'd0) ||
                (event_counter_valid && fdr_lineage[await_fd] == event_counter_lineage &&
                    event_counter_value != 64'd0);
        end
        if (raw_opcode == 8'h2f) begin
            call_gate_fd = dec.rs1;
        end else begin
            call_gate_fd = fdr_value_fd(gpr[dec.rs1]);
        end
        call_gate_fd_in_range = call_gate_fd < FDR_SLOT_COUNT;
        call_gate_fd_live = call_gate_fd_in_range &&
            fdr_valid[call_gate_fd] &&
            !fdr_revoked[call_gate_fd] &&
            fdr_kind[call_gate_fd] == FDR_KIND_CALL_GATE &&
            call_gate_valid[call_gate_fd] &&
            ((fdr_rights[call_gate_fd] & CAP_RIGHT_CALL) != 64'd0);
        call_gate_completion_fd_in_range = call_gate_fd_live &&
            call_gate_completion_fd[call_gate_fd] < FDR_SLOT_COUNT;
        call_gate_completion_is_event_counter = call_gate_completion_fd_in_range &&
            event_counter_valid &&
            fdr_valid[call_gate_completion_fd[call_gate_fd]] &&
            !fdr_revoked[call_gate_completion_fd[call_gate_fd]] &&
            fdr_lineage[call_gate_completion_fd[call_gate_fd]] == event_counter_lineage;
        pipe_buf_word_index = sram_word_index(gpr[dec.rs2]);
        pipe_buf_byte_lane = gpr[dec.rs2][2:0];
        cap_revoke_count = 64'd0;
        if (cap_src_fd_in_range) begin
            for (int slot = 0; slot < FDR_SLOT_COUNT; slot = slot + 1) begin
                if (fdr_valid[slot] && !fdr_revoked[slot] && fdr_lineage[slot] == fdr_lineage[cap_src_fd]) begin
                    cap_revoke_count = cap_revoke_count + 64'd1;
                end
            end
            if (cap_queue_valid && !cap_queue_revoked && cap_queue_lineage == fdr_lineage[cap_src_fd]) begin
                cap_revoke_count = cap_revoke_count + 64'd1;
            end
        end
    end

    always_comb begin
        active_tid = {31'd0, active_thread_slot} + 32'd1;
        if (active_thread_slot == 1'b0 && thread_active[1]) begin
            next_thread_slot = 1'b1;
        end else if (active_thread_slot == 1'b1 && thread_active[0]) begin
            next_thread_slot = 1'b0;
        end else begin
            next_thread_slot = active_thread_slot;
        end

        cmd.op_id = next_op_id;
        cmd.tile_id = TILE_ID[31:0];
        cmd.opcode = pending_unsupported ? LNP64_OP_UNSUPPORTED : dec.opcode;
        cmd.profile = 16'd0;
        cmd.pid = 32'd1;
        cmd.tid = active_tid;
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
        unique case (dec.opcode)
            LNP64_OP_CAP_DUP: begin
                cmd.rights_mask = cap_rights_req;
                cmd.flags = cap_flags;
                cmd.arg0 = cap_src_value;
                cmd.arg1 = cap_dst_req;
                cmd.arg_block_ptr = gpr[dec.rs1];
                cmd.arg_block_len = 64'd32;
            end
            default: begin
            end
        endcase
    end

    always_comb begin
        m1_commit_next = '0;
        unique case (dec.opcode)
            LNP64_OP_CAP_DUP: m1_commit_next.op = LNP64_M1_COMMIT_CAP_DUP;
            LNP64_OP_CAP_SEND: m1_commit_next.op = LNP64_M1_COMMIT_CAP_SEND;
            LNP64_OP_CAP_RECV: m1_commit_next.op = LNP64_M1_COMMIT_CAP_RECV;
            LNP64_OP_CAP_REVOKE: m1_commit_next.op = LNP64_M1_COMMIT_CAP_REVOKE;
            default: m1_commit_next.op = 8'd0;
        endcase
        m1_commit_next.domain_id = 32'd1;
        m1_commit_next.domain_gen = 32'd1;
        m1_commit_next.status = errno_reg;

        unique case (dec.opcode)
            LNP64_OP_CAP_DUP: begin
                if (cap_flags & ~CAP_DUP_FLAG_SEAL) begin
                    m1_commit_next.status = LNP64_ERR_EINVAL;
                end else if (cap_src_stale) begin
                    m1_commit_next.status = RTL_ERR_ESTALE;
                end else if (!cap_src_live || !cap_dst_fd_in_range) begin
                    m1_commit_next.status = LNP64_ERR_EBADF;
                end else if ((fdr_rights[cap_src_fd] & CAP_RIGHT_DUP) == 64'd0 ||
                    !cap_dup_rights_subset) begin
                    m1_commit_next.status = LNP64_ERR_EPERM;
                end else begin
                    m1_commit_next.status = LNP64_ERR_OK;
                    m1_commit_next.object_gen = fdr_generation[cap_dst_fd] + 64'd1;
                    m1_commit_next.fdr_gen = fdr_generation[cap_dst_fd] + 64'd1;
                    m1_commit_next.object_id = fdr_lineage[cap_src_fd][31:0];
                    m1_commit_next.rights_mask = cap_dup_rights;
                    m1_commit_next.lineage_epoch = fdr_lineage[cap_src_fd][31:0];
                    m1_commit_next.sealed = (cap_flags & CAP_DUP_FLAG_SEAL) != 64'd0;
                end
                if (m1_commit_next.status != LNP64_ERR_OK && cap_src_fd_in_range) begin
                    m1_commit_next.object_id = fdr_lineage[cap_src_fd][31:0];
                    m1_commit_next.object_gen = fdr_generation[cap_src_fd];
                    m1_commit_next.fdr_gen = fdr_generation[cap_src_fd];
                    m1_commit_next.rights_mask = fdr_rights[cap_src_fd];
                    m1_commit_next.lineage_epoch = fdr_lineage[cap_src_fd][31:0];
                end
            end
            LNP64_OP_CAP_SEND: begin
                if (cap_flags != 64'd0) begin
                    m1_commit_next.status = LNP64_ERR_EINVAL;
                end else if (!cap_src_live || !cap_src_fd_in_range ||
                    fdr_kind[cap_src_fd] != FDR_KIND_PIPE_WRITER ||
                    ((fdr_rights[cap_src_fd] & (CAP_RIGHT_TRANSFER | 64'd2)) != (CAP_RIGHT_TRANSFER | 64'd2))) begin
                    m1_commit_next.status = LNP64_ERR_EBADF;
                end else if (cap_arg1_fd >= FDR_SLOT_COUNT ||
                    !fdr_valid[cap_arg1_fd] || fdr_revoked[cap_arg1_fd]) begin
                    m1_commit_next.status = LNP64_ERR_EBADF;
                end else if ((fdr_rights[cap_arg1_fd] & CAP_RIGHT_TRANSFER) == 64'd0) begin
                    m1_commit_next.status = LNP64_ERR_EPERM;
                end else if (cap_queue_valid) begin
                    m1_commit_next.status = LNP64_ERR_EAGAIN;
                end else begin
                    m1_commit_next.status = LNP64_ERR_OK;
                end
                if (cap_arg1_fd < FDR_SLOT_COUNT) begin
                    m1_commit_next.object_id = fdr_lineage[cap_arg1_fd][31:0];
                    m1_commit_next.object_gen = fdr_generation[cap_arg1_fd];
                    m1_commit_next.fdr_gen = fdr_generation[cap_arg1_fd];
                    m1_commit_next.rights_mask = fdr_rights[cap_arg1_fd];
                    m1_commit_next.lineage_epoch = fdr_lineage[cap_arg1_fd][31:0];
                end
            end
            LNP64_OP_CAP_RECV: begin
                if (cap_flags != 64'd0) begin
                    m1_commit_next.status = LNP64_ERR_EINVAL;
                end else if (!cap_src_live || !cap_src_fd_in_range ||
                    fdr_kind[cap_src_fd] != FDR_KIND_PIPE_READER ||
                    ((fdr_rights[cap_src_fd] & (CAP_RIGHT_TRANSFER | 64'd1)) != (CAP_RIGHT_TRANSFER | 64'd1))) begin
                    m1_commit_next.status = LNP64_ERR_EBADF;
                end else if (!cap_queue_valid) begin
                    m1_commit_next.status = LNP64_ERR_EAGAIN;
                end else if (cap_queue_revoked) begin
                    m1_commit_next.status = RTL_ERR_ESTALE;
                end else if (!cap_recv_rights_subset) begin
                    m1_commit_next.status = LNP64_ERR_EPERM;
                end else if (!cap_dst_fd_in_range) begin
                    m1_commit_next.status = LNP64_ERR_EBADF;
                end else begin
                    m1_commit_next.status = LNP64_ERR_OK;
                    m1_commit_next.object_gen = fdr_generation[cap_dst_fd] + 64'd1;
                    m1_commit_next.fdr_gen = fdr_generation[cap_dst_fd] + 64'd1;
                    m1_commit_next.object_id = cap_queue_lineage[31:0];
                    m1_commit_next.rights_mask = cap_recv_rights;
                    m1_commit_next.lineage_epoch = cap_queue_lineage[31:0];
                end
                if (m1_commit_next.status != LNP64_ERR_OK && cap_queue_valid) begin
                    m1_commit_next.object_id = cap_queue_lineage[31:0];
                    m1_commit_next.rights_mask = cap_queue_rights;
                    m1_commit_next.lineage_epoch = cap_queue_lineage[31:0];
                end
            end
            LNP64_OP_CAP_REVOKE: begin
                if (cap_src_stale) begin
                    m1_commit_next.status = RTL_ERR_ESTALE;
                end else if (cap_src_live && ((fdr_rights[cap_src_fd] & CAP_RIGHT_REVOKE) != 64'd0)) begin
                    m1_commit_next.status = LNP64_ERR_OK;
                    m1_commit_next.object_gen = fdr_generation[cap_src_fd] + 64'd1;
                    m1_commit_next.fdr_gen = fdr_generation[cap_src_fd] + 64'd1;
                end else if (cap_src_fd_in_range && fdr_valid[cap_src_fd] &&
                    !fdr_revoked[cap_src_fd] &&
                    ((fdr_rights[cap_src_fd] & CAP_RIGHT_REVOKE) == 64'd0)) begin
                    m1_commit_next.status = LNP64_ERR_EPERM;
                end else if (dma_buffer_ref_matches(object_op) && !dma_buffer_object_revoked) begin
                    m1_commit_next.status = LNP64_ERR_OK;
                end else if (dma_buffer_ref_matches(object_op) && dma_buffer_object_revoked) begin
                    m1_commit_next.status = RTL_ERR_ESTALE;
                end else begin
                    m1_commit_next.status = LNP64_ERR_EBADF;
                end
                if (cap_src_fd_in_range) begin
                    m1_commit_next.object_id = fdr_lineage[cap_src_fd][31:0];
                    if (m1_commit_next.object_gen == 32'd0) begin
                        m1_commit_next.object_gen = fdr_generation[cap_src_fd];
                        m1_commit_next.fdr_gen = fdr_generation[cap_src_fd];
                    end
                    m1_commit_next.rights_mask = fdr_rights[cap_src_fd];
                    m1_commit_next.lineage_epoch = fdr_lineage[cap_src_fd][31:0];
                end
            end
            default: begin
            end
        endcase
    end

    always_comb begin
        m1_pre_state_projection = build_m1_state_projection(
            m1_commit_next.op,
            m1_commit_next.status,
            cap_src_fd_in_range ? cap_src_fd : 0,
            m1_current_consumer_fd()
        );
        m1_state_projection = build_m1_state_projection(
            m1_commit.op,
            m1_commit.status,
            m1_projection_root_fd,
            m1_projection_consumer_fd
        );
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
        retire_submit_next.tid = active_tid;
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
    integer j;
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
            m1_commit_valid <= 1'b0;
            m1_commit <= '0;
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
            cap_engine_shadow_enabled <= 1'b1;
            cmp_zero <= 1'b0;
            cmp_negative <= 1'b0;
            cmp_greater <= 1'b0;
            cmp_below <= 1'b0;
            cmp_above <= 1'b0;
            return_stack_depth <= '0;
            link_register <= 64'd0;
            active_thread_slot <= 1'b0;
            for (i = 0; i < 2; i = i + 1) begin
                thread_pc[i] <= 32'd0;
                thread_return_stack_depth[i] <= '0;
                thread_link_register[i] <= 64'd0;
                thread_cmp_zero[i] <= 1'b0;
                thread_cmp_negative[i] <= 1'b0;
                thread_cmp_greater[i] <= 1'b0;
                thread_cmp_below[i] <= 1'b0;
                thread_cmp_above[i] <= 1'b0;
                thread_active[i] <= i == 0;
                thread_completed[i] <= 1'b0;
                thread_exit_code[i] <= 64'd0;
                for (j = 0; j < 32; j = j + 1) begin
                    thread_gpr[i][j] <= 64'd0;
                end
                for (j = 0; j < RETURN_STACK_DEPTH; j = j + 1) begin
                    thread_return_stack[i][j] <= 32'd0;
                end
            end
            pending_unsupported <= 1'b0;
            command_pc <= 32'd0;
            heap_next <= HEAP_ARCH_BASE;
            mmap_next <= MMAP_ARCH_BASE;
            heap_alloc_next_slot <= 2'd0;
            dma_buffer_object_valid <= 1'b0;
            dma_buffer_object_revoked <= 1'b0;
            dma_buffer_object_fd <= RTL_DMA_BUFFER_DEFAULT_FD;
            dma_buffer_object_addr <= 64'd0;
            dma_buffer_object_len <= 64'd0;
            cap_queue_valid <= 1'b0;
            cap_queue_rights <= 64'd0;
            cap_queue_lineage <= 64'd0;
            cap_queue_generation <= 64'd0;
            cap_queue_revoked <= 1'b0;
            call_continuation_valid <= 1'b0;
            call_continuation_return_pc <= 32'd0;
            call_continuation_result_reg <= 8'd0;
            call_next_op_id <= 64'd1;
            memory_object_valid <= 1'b0;
            memory_object_lineage <= 64'd0;
            memory_object_len <= 64'd0;
            memory_object_byte_valid <= 1'b0;
            memory_object_byte_value <= 8'd0;
            event_counter_valid <= 1'b0;
            event_counter_lineage <= 64'd0;
            event_counter_value <= 64'd0;
            timer_valid <= 1'b0;
            timer_lineage <= 64'd0;
            timer_expirations <= 64'd0;
            domain_next_id <= 64'd2;
            m1_projection_root_fd <= 0;
            m1_projection_consumer_fd <= 0;
            topology_record_valid <= 1'b0;
            topology_record_base <= 64'd0;
            for (i = 0; i < 32; i = i + 1) begin
                gpr[i] <= 64'd0;
            end
            gpr[31] <= FLAT_EXEC_INITIAL_SP;
            for (i = 0; i < 4; i = i + 1) begin
                heap_alloc_ptr[i] <= 64'd0;
                heap_alloc_size[i] <= 64'd0;
                heap_alloc_valid[i] <= 1'b0;
            end
            for (i = 0; i < FDR_SLOT_COUNT; i = i + 1) begin
                fdr_generation[i] <= 64'd1;
                fdr_valid[i] <= i < 3;
                fdr_revoked[i] <= 1'b0;
                fdr_rights[i] <= i < 3 ? CAP_RIGHT_ALL : 64'd0;
                fdr_lineage[i] <= {32'd0, i[31:0]} + 64'd1;
                fdr_kind[i] <= i < 3 ? FDR_KIND_GENERIC : FDR_KIND_CLOSED;
                pipe_payload_valid[i] <= 1'b0;
                pipe_payload_value[i] <= 64'd0;
                pipe_payload_len[i] <= 4'd0;
                call_gate_valid[i] <= 1'b0;
                call_gate_entry[i] <= 64'd0;
                call_gate_mode[i] <= 64'd0;
                call_gate_completion_fd[i] <= 64'd0;
            end
            for (i = 0; i < DOMAIN_SLOT_COUNT; i = i + 1) begin
                domain_valid[i] <= i == 0;
                domain_destroyed[i] <= 1'b0;
                domain_generation[i] <= 64'd1;
                domain_parent[i] <= i == 0 ? 64'd0 : DOMAIN_ROOT_ID;
                domain_depth[i] <= i == 0 ? 64'd0 : 64'd1;
                domain_profile[i] <= 64'd0;
                domain_cpu_limit[i] <= i == 0 ? 64'hffff_ffff_ffff_ffff : 64'd0;
                domain_memory_limit[i] <= i == 0 ? 64'hffff_ffff_ffff_ffff : 64'd0;
                domain_pids_limit[i] <= i == 0 ? 64'hffff_ffff_ffff_ffff : 64'd0;
                domain_fdrs_limit[i] <= i == 0 ? 64'hffff_ffff_ffff_ffff : 64'd0;
                domain_cap_mask[i] <= i == 0 ? 64'hffff_ffff_ffff_ffff : 64'd0;
                domain_upcall_mask[i] <= i == 0 ? 64'hffff_ffff_ffff_ffff : 64'd0;
                domain_child_count[i] <= 64'd0;
                domain_frozen[i] <= 1'b0;
            end
            for (i = 0; i < SRAM_WORDS; i = i + 1) begin
                sram[i] = initial_sram[i];
            end
            for (i = 0; i < SRAM_WORDS - DATA_SRAM_BASE_WORD; i = i + 1) begin
                sram[DATA_SRAM_BASE_WORD + i] = initial_data_sram[i];
            end
        end else begin
            retire_submit_valid <= 1'b0;
            m1_commit_valid <= 1'b0;
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
                    state <= CORE_SWITCH;
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
                            LNP64_OP_LA_LITERAL: begin
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
                                gpr[dec.rd] <= div_signed64(gpr[dec.rs1], gpr[dec.rs2]);
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
                                gpr[dec.rd] <= rem_signed64(gpr[dec.rs1], gpr[dec.rs2]);
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
                                gpr[31] <= gpr[31] - FLAT_EXEC_CALL_FRAME_BYTES;
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
                                gpr[31] <= gpr[31] - FLAT_EXEC_CALL_FRAME_BYTES;
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
                                gpr[31] <= gpr[31] + FLAT_EXEC_CALL_FRAME_BYTES;
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
                            LNP64_OP_MMAP: begin
                                if (gpr[dec.rs2] == 64'd0 || (gpr[dec.rs3] & ~64'd7) != 64'd0) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EINVAL;
                                end else if ((gpr[dec.rs3] & 64'd6) == 64'd6) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EPERM;
                                end else if (program_rom[pc + 32'd1][23:19] != 5'd0 ||
                                    gpr[program_rom[pc + 32'd1][18:14]] != 64'd0) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EBADF;
                                end else begin
                                    gpr[dec.rd] <= gpr[dec.rs1] != 64'd0 ?
                                        gpr[dec.rs1] : align_up_u64(mmap_next, 64'd4096);
                                    heap_alloc_ptr[heap_alloc_next_slot] <= gpr[dec.rs1] != 64'd0 ?
                                        gpr[dec.rs1] : align_up_u64(mmap_next, 64'd4096);
                                    heap_alloc_size[heap_alloc_next_slot] <= gpr[dec.rs2];
                                    heap_alloc_valid[heap_alloc_next_slot] <= 1'b1;
                                    heap_alloc_next_slot <= heap_alloc_next_slot + 2'd1;
                                    mmap_next <= (gpr[dec.rs1] != 64'd0 ?
                                        gpr[dec.rs1] : align_up_u64(mmap_next, 64'd4096)) + gpr[dec.rs2];
                                    errno_reg <= LNP64_ERR_OK;
                                end
                                pc <= pc + 32'd2;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_MPROTECT: begin
                                if (gpr[dec.rs1] == 64'd0 || (gpr[dec.rs2] & ~64'd7) != 64'd0) begin
                                    gpr[1] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EINVAL;
                                end else if ((gpr[dec.rs2] & 64'd6) == 64'd6) begin
                                    gpr[1] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EPERM;
                                end else if (!heap_range_valid(gpr[dec.rd], gpr[dec.rs1])) begin
                                    gpr[1] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EINVAL;
                                end else begin
                                    gpr[1] <= 64'd0;
                                    errno_reg <= LNP64_ERR_OK;
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
                            LNP64_OP_SLEEP: begin
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
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
                            LNP64_OP_OPEN_FD: begin
                                cap_engine_shadow_enabled <= 1'b0;
                                if (!file_open_available) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= 16'd24;
                                end else begin
                                    fdr_valid[file_open_fd] <= 1'b1;
                                    fdr_revoked[file_open_fd] <= 1'b0;
                                    fdr_generation[file_open_fd] <= fdr_generation[file_open_fd] + 64'd1;
                                    fdr_rights[file_open_fd] <= CAP_RIGHT_ALL;
                                    fdr_lineage[file_open_fd] <= 64'd1792 + {56'd0, file_open_fd[7:0]};
                                    fdr_kind[file_open_fd] <= FDR_KIND_GENERIC;
                                    gpr[dec.rd] <= FDR_TOKEN_MARKER |
                                        ((fdr_generation[file_open_fd] + 64'd1) << 8) |
                                        {56'd0, file_open_fd[7:0]};
                                    gpr[1] <= FDR_TOKEN_MARKER |
                                        ((fdr_generation[file_open_fd] + 64'd1) << 8) |
                                        {56'd0, file_open_fd[7:0]};
                                    errno_reg <= LNP64_ERR_OK;
                                end
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_FD_CLOSE: begin
                                cap_engine_shadow_enabled <= 1'b0;
                                if (close_fd_token_stale) begin
                                    errno_reg <= RTL_ERR_ESTALE;
                                end else if (!close_fd_in_range || !fdr_valid[close_fd]) begin
                                    errno_reg <= LNP64_ERR_EBADF;
                                end else begin
                                    fdr_valid[close_fd] <= 1'b0;
                                    fdr_revoked[close_fd] <= 1'b0;
                                    fdr_generation[close_fd] <= fdr_generation[close_fd] + 64'd1;
                                    fdr_rights[close_fd] <= 64'd0;
                                    fdr_kind[close_fd] <= FDR_KIND_CLOSED;
                                    errno_reg <= LNP64_ERR_OK;
                                end
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_READ_FD: begin
                                if (gpr[dec.rs2] == 64'd0) begin
                                    gpr[1] <= 64'd0;
                                    errno_reg <= LNP64_ERR_OK;
                                end else if (!static_fd_in_range || !fdr_valid[static_fd] ||
                                    !(static_fd_is_memory_object || static_fd_is_event_counter ||
                                        static_fd_is_timer)) begin
                                    gpr[1] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EBADF;
                                end else if (fdr_revoked[static_fd]) begin
                                    gpr[1] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= RTL_ERR_ESTALE;
                                end else if ((fdr_rights[static_fd] & 64'd1) == 64'd0) begin
                                    gpr[1] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EPERM;
                                end else if (static_fd_is_event_counter) begin
                                    sram[static_fd_buf_word_index] <= store_double_low_word(
                                        sram[static_fd_buf_word_index],
                                        static_fd_buf_byte_lane,
                                        event_counter_value
                                    );
                                    if (static_fd_buf_byte_lane != 3'd0) begin
                                        sram[static_fd_buf_next_word_index] <= store_double_high_word(
                                            sram[static_fd_buf_next_word_index],
                                            static_fd_buf_byte_lane,
                                            event_counter_value
                                        );
                                    end
                                    event_counter_value <= 64'd0;
                                    dcache_writeback <= 1'b1;
                                    gpr[1] <= gpr[dec.rs2] < 64'd8 ? gpr[dec.rs2] : 64'd8;
                                    errno_reg <= LNP64_ERR_OK;
                                end else if (static_fd_is_timer) begin
                                    sram[static_fd_buf_word_index] <= store_double_low_word(
                                        sram[static_fd_buf_word_index],
                                        static_fd_buf_byte_lane,
                                        timer_expirations
                                    );
                                    if (static_fd_buf_byte_lane != 3'd0) begin
                                        sram[static_fd_buf_next_word_index] <= store_double_high_word(
                                            sram[static_fd_buf_next_word_index],
                                            static_fd_buf_byte_lane,
                                            timer_expirations
                                        );
                                    end
                                    timer_expirations <= 64'd0;
                                    dcache_writeback <= 1'b1;
                                    gpr[1] <= gpr[dec.rs2] < 64'd8 ? gpr[dec.rs2] : 64'd8;
                                    errno_reg <= LNP64_ERR_OK;
                                end else if (!memory_object_byte_valid || memory_object_len == 64'd0) begin
                                    gpr[1] <= 64'd0;
                                    errno_reg <= LNP64_ERR_OK;
                                end else begin
                                    sram[static_fd_buf_word_index] <= store_byte_lane(
                                        sram[static_fd_buf_word_index],
                                        static_fd_buf_byte_lane,
                                        memory_object_byte_value
                                    );
                                    dcache_writeback <= 1'b1;
                                    gpr[1] <= 64'd1;
                                    errno_reg <= LNP64_ERR_OK;
                                end
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_WRITE_FD: begin
                                if (gpr[dec.rs2] == 64'd0) begin
                                    gpr[1] <= 64'd0;
                                    errno_reg <= LNP64_ERR_OK;
                                end else if (static_fd_is_event_counter) begin
                                    if (!fdr_valid[static_fd]) begin
                                        gpr[1] <= 64'hffff_ffff_ffff_ffff;
                                        errno_reg <= LNP64_ERR_EBADF;
                                    end else if (fdr_revoked[static_fd]) begin
                                        gpr[1] <= 64'hffff_ffff_ffff_ffff;
                                        errno_reg <= RTL_ERR_ESTALE;
                                    end else if ((fdr_rights[static_fd] & 64'd2) == 64'd0) begin
                                        gpr[1] <= 64'hffff_ffff_ffff_ffff;
                                        errno_reg <= LNP64_ERR_EPERM;
                                    end else begin
                                        event_counter_value <= event_counter_value +
                                            load_double_unaligned(gpr[dec.rs1]);
                                        gpr[1] <= gpr[dec.rs2] < 64'd8 ? gpr[dec.rs2] : 64'd8;
                                        errno_reg <= LNP64_ERR_OK;
                                    end
                                end else if (static_fd_is_timer) begin
                                    if (!fdr_valid[static_fd]) begin
                                        gpr[1] <= 64'hffff_ffff_ffff_ffff;
                                        errno_reg <= LNP64_ERR_EBADF;
                                    end else if (fdr_revoked[static_fd]) begin
                                        gpr[1] <= 64'hffff_ffff_ffff_ffff;
                                        errno_reg <= RTL_ERR_ESTALE;
                                    end else if ((fdr_rights[static_fd] & 64'd2) == 64'd0) begin
                                        gpr[1] <= 64'hffff_ffff_ffff_ffff;
                                        errno_reg <= LNP64_ERR_EPERM;
                                    end else begin
                                        timer_expirations <= load_double_unaligned(gpr[dec.rs1]) == 64'd0 ?
                                            64'd0 : 64'd1;
                                        gpr[1] <= gpr[dec.rs2] < 64'd8 ? gpr[dec.rs2] : 64'd8;
                                        errno_reg <= LNP64_ERR_OK;
                                    end
                                end else if (static_fd_is_memory_object) begin
                                    if (!fdr_valid[static_fd]) begin
                                        gpr[1] <= 64'hffff_ffff_ffff_ffff;
                                        errno_reg <= LNP64_ERR_EBADF;
                                    end else if (fdr_revoked[static_fd]) begin
                                        gpr[1] <= 64'hffff_ffff_ffff_ffff;
                                        errno_reg <= RTL_ERR_ESTALE;
                                    end else if ((fdr_rights[static_fd] & 64'd2) == 64'd0) begin
                                        gpr[1] <= 64'hffff_ffff_ffff_ffff;
                                        errno_reg <= LNP64_ERR_EPERM;
                                    end else begin
                                        memory_object_byte_valid <= 1'b1;
                                        memory_object_byte_value <= load_byte_lane(
                                            sram[static_fd_buf_word_index],
                                            static_fd_buf_byte_lane
                                        );
                                        gpr[1] <= 64'd1;
                                        errno_reg <= LNP64_ERR_OK;
                                    end
                                end else begin
                                    gpr[1] <= gpr[dec.rs2];
                                    errno_reg <= LNP64_ERR_OK;
                                end
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_AWAIT: begin
                                if (!await_fd_in_range || !fdr_valid[await_fd]) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EBADF;
                                end else if (fdr_revoked[await_fd]) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= RTL_ERR_ESTALE;
                                end else if ((fdr_rights[await_fd] & 64'd16) == 64'd0) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EPERM;
                                end else if (!await_fd_ready) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EAGAIN;
                                end else begin
                                    gpr[dec.rd] <= 64'd0;
                                    errno_reg <= LNP64_ERR_OK;
                                end
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_GATE_CALL: begin
                                if (!call_gate_fd_live) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EBADF;
                                    pc <= pc + 32'd1;
                                end else if (call_gate_mode[call_gate_fd] == CALL_MODE_ASYNC) begin
                                    if (!call_gate_completion_is_event_counter) begin
                                        gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                        errno_reg <= LNP64_ERR_EINVAL;
                                    end else begin
                                        event_counter_value <= event_counter_value + call_next_op_id;
                                        gpr[dec.rd] <= call_next_op_id;
                                        call_next_op_id <= call_next_op_id + 64'd1;
                                        errno_reg <= LNP64_ERR_OK;
                                    end
                                    pc <= pc + 32'd1;
                                end else if (call_gate_mode[call_gate_fd] == CALL_MODE_HANDOFF) begin
                                    gpr[dec.rd] <= 64'd0;
                                    gpr[1] <= gpr[dec.rs2];
                                    gpr[2] <= gpr[dec.rs3];
                                    errno_reg <= LNP64_ERR_OK;
                                    pc <= flat_exec_pc_word(call_gate_entry[call_gate_fd]);
                                end else if (call_gate_mode[call_gate_fd] == CALL_MODE_SYNC) begin
                                    if (call_continuation_valid) begin
                                        gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                        errno_reg <= LNP64_ERR_EAGAIN;
                                        pc <= pc + 32'd1;
                                    end else begin
                                        call_continuation_valid <= 1'b1;
                                        call_continuation_return_pc <= pc + 32'd1;
                                        call_continuation_result_reg <= dec.rd;
                                        gpr[1] <= gpr[dec.rs2];
                                        gpr[2] <= gpr[dec.rs3];
                                        errno_reg <= LNP64_ERR_OK;
                                        pc <= flat_exec_pc_word(call_gate_entry[call_gate_fd]);
                                    end
                                end else begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EINVAL;
                                    pc <= pc + 32'd1;
                                end
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_GATE_RETURN: begin
                                if (!call_continuation_valid) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EINVAL;
                                    pc <= pc + 32'd1;
                                end else begin
                                    gpr[call_continuation_result_reg[4:0]] <= gpr[dec.rs1];
                                    gpr[30] <= gpr[dec.rs2];
                                    gpr[dec.rd] <= 64'd0;
                                    call_continuation_valid <= 1'b0;
                                    errno_reg <= LNP64_ERR_OK;
                                    pc <= call_continuation_return_pc;
                                end
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_PUSH: begin
                                if (gpr[dec.rs3] == 64'd0) begin
                                    gpr[1] <= 64'd0;
                                    gpr[dec.rd] <= 64'd0;
                                    errno_reg <= LNP64_ERR_OK;
                                end else if (!pipe_fd_in_range || !fdr_valid[pipe_fd] ||
                                    fdr_revoked[pipe_fd] ||
                                    fdr_kind[pipe_fd] != FDR_KIND_PIPE_WRITER ||
                                    ((fdr_rights[pipe_fd] & 64'd2) == 64'd0)) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EBADF;
                                end else if (gpr[dec.rs3] > 64'd8) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EINVAL;
                                end else if (pipe_payload_valid[pipe_queue_slot]) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EAGAIN;
                                end else begin
                                    pipe_payload_valid[pipe_queue_slot] <= 1'b1;
                                    pipe_payload_value[pipe_queue_slot] <=
                                        load_double_unaligned(gpr[dec.rs2]) &
                                        low_bytes_mask(gpr[dec.rs3][3:0]);
                                    pipe_payload_len[pipe_queue_slot] <= gpr[dec.rs3][3:0];
                                    gpr[1] <= gpr[dec.rs3];
                                    gpr[dec.rd] <= gpr[dec.rs3];
                                    errno_reg <= LNP64_ERR_OK;
                                end
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_PULL: begin
                                if (gpr[dec.rs3] == 64'd0) begin
                                    gpr[1] <= 64'd0;
                                    gpr[dec.rd] <= 64'd0;
                                    errno_reg <= LNP64_ERR_OK;
                                end else if (raw_opcode == 8'h3b && pipe_fd_token_stale) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= RTL_ERR_ESTALE;
                                end else if (raw_opcode == 8'h3b && pipe_fd_in_range &&
                                    fdr_valid[pipe_fd] && !fdr_revoked[pipe_fd] &&
                                    fdr_kind[pipe_fd] == FDR_KIND_GENERIC &&
                                    ((fdr_rights[pipe_fd] & 64'd1) != 64'd0)) begin
                                    store_double_unaligned_next(gpr[dec.rs2], 64'h0000_0000_6361_705b);
                                    dcache_writeback <= 1'b1;
                                    gpr[1] <= dynamic_file_read_count;
                                    gpr[dec.rd] <= dynamic_file_read_count;
                                    errno_reg <= LNP64_ERR_OK;
                                end else if (!pipe_fd_in_range || !fdr_valid[pipe_fd] ||
                                    fdr_revoked[pipe_fd] ||
                                    fdr_kind[pipe_fd] != FDR_KIND_PIPE_READER ||
                                    ((fdr_rights[pipe_fd] & 64'd1) == 64'd0)) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EBADF;
                                end else if (gpr[dec.rs3] > 64'd8) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EINVAL;
                                end else if (!pipe_payload_valid[pipe_queue_slot]) begin
                                    gpr[1] <= 64'd0;
                                    gpr[dec.rd] <= 64'd0;
                                    errno_reg <= LNP64_ERR_OK;
                                end else begin
                                    store_payload_unaligned_next(
                                        gpr[dec.rs2],
                                        pipe_payload_value[pipe_queue_slot],
                                        pipe_pull_len
                                    );
                                    if (pipe_pull_len == pipe_payload_len[pipe_queue_slot]) begin
                                        pipe_payload_valid[pipe_queue_slot] <= 1'b0;
                                        pipe_payload_value[pipe_queue_slot] <= 64'd0;
                                        pipe_payload_len[pipe_queue_slot] <= 4'd0;
                                    end else begin
                                        pipe_payload_value[pipe_queue_slot] <=
                                            pipe_payload_value[pipe_queue_slot] >>
                                            ({2'd0, pipe_pull_len} * 6'd8);
                                        pipe_payload_len[pipe_queue_slot] <=
                                            pipe_payload_len[pipe_queue_slot] - pipe_pull_len;
                                    end
                                    gpr[1] <= {60'd0, pipe_pull_len};
                                    gpr[dec.rd] <= {60'd0, pipe_pull_len};
                                    errno_reg <= LNP64_ERR_OK;
                                end
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
                            LNP64_OP_CLONE: begin
                                if (active_thread_slot == 1'b0 && !thread_active[1]) begin
                                    for (i = 0; i < 32; i = i + 1) begin
                                        thread_gpr[1][i] <= gpr[i];
                                    end
                                    thread_gpr[1][1] <= gpr[dec.rs2];
                                    thread_gpr[1][31] <= FLAT_EXEC_CHILD_SP;
                                    thread_gpr[1][0] <= 64'd0;
                                    thread_pc[1] <= flat_exec_pc_word(gpr[dec.rs1]);
                                    thread_return_stack_depth[1] <= '0;
                                    for (i = 0; i < RETURN_STACK_DEPTH; i = i + 1) begin
                                        thread_return_stack[1][i] <= 32'd0;
                                    end
                                    thread_link_register[1] <= 64'd0;
                                    thread_cmp_zero[1] <= 1'b0;
                                    thread_cmp_negative[1] <= 1'b0;
                                    thread_cmp_greater[1] <= 1'b0;
                                    thread_cmp_below[1] <= 1'b0;
                                    thread_cmp_above[1] <= 1'b0;
                                    thread_active[1] <= 1'b1;
                                    thread_completed[1] <= 1'b0;
                                    thread_exit_code[1] <= 64'd0;
                                    gpr[dec.rd] <= 64'd2;
                                    errno_reg <= LNP64_ERR_OK;
                                end else begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EAGAIN;
                                end
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_JOIN: begin
                                if (gpr[dec.rs1] == active_tid) begin
                                    gpr[dec.rd] <= 64'd35;
                                    errno_reg <= LNP64_ERR_OK;
                                end else if (gpr[dec.rs1] == 64'd2 && thread_completed[1]) begin
                                    if (gpr[dec.rs2] != 64'd0) begin
                                        store_double_unaligned_next(gpr[dec.rs2], thread_exit_code[1]);
                                    end
                                    thread_completed[1] <= 1'b0;
                                    gpr[dec.rd] <= 64'd0;
                                    errno_reg <= LNP64_ERR_OK;
                                end else if (gpr[dec.rs1] == 64'd2 && thread_active[1]) begin
                                    gpr[dec.rd] <= 64'd11;
                                    errno_reg <= LNP64_ERR_EAGAIN;
                                end else begin
                                    gpr[dec.rd] <= 64'd3;
                                    errno_reg <= LNP64_ERR_OK;
                                end
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_EXIT: begin
                                if (active_thread_slot == 1'b0) begin
                                    done <= 1'b1;
                                    pid1_runnable <= 1'b0;
                                    pid1_parked <= 1'b0;
                                    state <= CORE_DONE;
                                end else begin
                                    thread_active[active_thread_slot] <= 1'b0;
                                    thread_completed[active_thread_slot] <= 1'b1;
                                    thread_exit_code[active_thread_slot] <= gpr[dec.rd];
                                    state <= CORE_SWITCH;
                                end
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_OBJECT_CTL: begin
                                cap_engine_shadow_enabled <= 1'b0;
                                if (object_op == OBJECT_OP_CREATE &&
                                    object_kind == OBJECT_KIND_QUEUE &&
                                    object_profile == OBJECT_PROFILE_CALL_GATE &&
                                    object_single_fd_in_range &&
                                    (object_gate_mode == CALL_MODE_SYNC ||
                                        object_gate_mode == CALL_MODE_ASYNC ||
                                        object_gate_mode == CALL_MODE_HANDOFF) &&
                                    object_gate_flags == 64'd0 &&
                                    (object_gate_mode != CALL_MODE_ASYNC ||
                                        (object_gate_completion_fd < FDR_SLOT_COUNT &&
                                            fdr_valid[object_gate_completion_fd] &&
                                            !fdr_revoked[object_gate_completion_fd] &&
                                            event_counter_valid &&
                                            fdr_lineage[object_gate_completion_fd] == event_counter_lineage))) begin
                                    call_gate_valid[object_single_fd] <= 1'b1;
                                    call_gate_entry[object_single_fd] <= object_gate_entry;
                                    call_gate_mode[object_single_fd] <= object_gate_mode;
                                    call_gate_completion_fd[object_single_fd] <= object_gate_completion_fd;
                                    fdr_valid[object_single_fd] <= 1'b1;
                                    fdr_revoked[object_single_fd] <= 1'b0;
                                    fdr_generation[object_single_fd] <= fdr_generation[object_single_fd] + 64'd1;
                                    fdr_rights[object_single_fd] <= CAP_RIGHT_ALL;
                                    fdr_lineage[object_single_fd] <= 64'd1281 + {56'd0, object_single_fd[7:0]};
                                    fdr_kind[object_single_fd] <= FDR_KIND_CALL_GATE;
                                    sram[object_fd_store_word_index] <= store_double_low_word(
                                        sram[object_fd_store_word_index],
                                        object_fd_store_byte_lane,
                                        {56'd0, object_single_fd[7:0]}
                                    );
                                    if (object_fd_store_byte_lane != 3'd0) begin
                                        sram[object_fd_store_next_word_index] <= store_double_high_word(
                                            sram[object_fd_store_next_word_index],
                                            object_fd_store_byte_lane,
                                            {56'd0, object_single_fd[7:0]}
                                        );
                                    end
                                    gpr[dec.rd] <= {56'd0, object_single_fd[7:0]};
                                    errno_reg <= LNP64_ERR_OK;
                                    pc <= pc + 32'd1;
                                    retired_count <= retired_count + 32'd1;
                                    retire_submit_valid <= 1'b1;
                                    retire_submit_record <= retire_submit_next;
                                end else if (object_op == OBJECT_OP_CREATE &&
                                    object_kind == OBJECT_KIND_QUEUE &&
                                    object_profile == OBJECT_PROFILE_CALL_GATE) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EINVAL;
                                    pc <= pc + 32'd1;
                                    retired_count <= retired_count + 32'd1;
                                    retire_submit_valid <= 1'b1;
                                    retire_submit_record <= retire_submit_next;
                                end else if (object_op == OBJECT_OP_CREATE &&
                                    object_kind == OBJECT_KIND_QUEUE &&
                                    object_profile == OBJECT_PROFILE_PIPE &&
                                    ((object_fd_req == 64'd0 && object_fd1_req == 64'd0 &&
                                        pipe_alloc_available) ||
                                     (object_fd_req != 64'd0 && object_fd_req < FDR_SLOT_COUNT &&
                                        object_fd1_req == object_fd_req + 64'd1 &&
                                        object_fd1_req < FDR_SLOT_COUNT &&
                                        !fdr_valid[object_fd_req[31:0]] &&
                                        !fdr_valid[object_fd1_req[31:0]]))) begin
                                    fdr_valid[pipe_alloc_reader_fd] <= 1'b1;
                                    fdr_revoked[pipe_alloc_reader_fd] <= 1'b0;
                                    fdr_generation[pipe_alloc_reader_fd] <= fdr_generation[pipe_alloc_reader_fd] + 64'd1;
                                    fdr_rights[pipe_alloc_reader_fd] <= CAP_RIGHT_ALL;
                                    fdr_lineage[pipe_alloc_reader_fd] <= 64'd257 + {32'd0, pipe_alloc_reader_fd[31:0]};
                                    fdr_kind[pipe_alloc_reader_fd] <= FDR_KIND_PIPE_READER;
                                    fdr_valid[pipe_alloc_writer_fd] <= 1'b1;
                                    fdr_revoked[pipe_alloc_writer_fd] <= 1'b0;
                                    fdr_generation[pipe_alloc_writer_fd] <= fdr_generation[pipe_alloc_writer_fd] + 64'd1;
                                    fdr_rights[pipe_alloc_writer_fd] <= CAP_RIGHT_ALL;
                                    fdr_lineage[pipe_alloc_writer_fd] <= 64'd257 + {32'd0, pipe_alloc_writer_fd[31:0]};
                                    fdr_kind[pipe_alloc_writer_fd] <= FDR_KIND_PIPE_WRITER;
                                    sram[object_fd_store_word_index] <= store_double_low_word(
                                        sram[object_fd_store_word_index],
                                        object_fd_store_byte_lane,
                                        {32'd0, pipe_alloc_reader_fd[31:0]}
                                    );
                                    if (object_fd_store_byte_lane != 3'd0) begin
                                        sram[object_fd_store_next_word_index] <= store_double_high_word(
                                            sram[object_fd_store_next_word_index],
                                            object_fd_store_byte_lane,
                                            {32'd0, pipe_alloc_reader_fd[31:0]}
                                        );
                                    end
                                    sram[object_fd1_store_word_index] <= store_double_low_word(
                                        sram[object_fd1_store_word_index],
                                        object_fd1_store_byte_lane,
                                        {32'd0, pipe_alloc_writer_fd[31:0]}
                                    );
                                    gpr[dec.rd] <= 64'd0;
                                    errno_reg <= LNP64_ERR_OK;
                                    pc <= pc + 32'd1;
                                    retired_count <= retired_count + 32'd1;
                                    retire_submit_valid <= 1'b1;
                                    retire_submit_record <= retire_submit_next;
                                end else if (object_op == OBJECT_OP_CREATE &&
                                    object_kind == OBJECT_KIND_COUNTER &&
                                    (object_profile == 64'd0 || object_profile == 64'd1) &&
                                    object_single_fd_in_range) begin
                                    event_counter_valid <= 1'b1;
                                    event_counter_lineage <= 64'd769;
                                    event_counter_value <= object_dma_addr;
                                    fdr_valid[object_single_fd] <= 1'b1;
                                    fdr_revoked[object_single_fd] <= 1'b0;
                                    fdr_generation[object_single_fd] <= fdr_generation[object_single_fd] + 64'd1;
                                    fdr_rights[object_single_fd] <= CAP_RIGHT_ALL;
                                    fdr_lineage[object_single_fd] <= 64'd769;
                                    fdr_kind[object_single_fd] <= FDR_KIND_GENERIC;
                                    sram[object_fd_store_word_index] <= store_double_low_word(
                                        sram[object_fd_store_word_index],
                                        object_fd_store_byte_lane,
                                        {56'd0, object_single_fd[7:0]}
                                    );
                                    if (object_fd_store_byte_lane != 3'd0) begin
                                        sram[object_fd_store_next_word_index] <= store_double_high_word(
                                            sram[object_fd_store_next_word_index],
                                            object_fd_store_byte_lane,
                                            {56'd0, object_single_fd[7:0]}
                                        );
                                    end
                                    gpr[dec.rd] <= {56'd0, object_single_fd[7:0]};
                                    errno_reg <= LNP64_ERR_OK;
                                    pc <= pc + 32'd1;
                                    retired_count <= retired_count + 32'd1;
                                    retire_submit_valid <= 1'b1;
                                    retire_submit_record <= retire_submit_next;
                                end else if (object_op == OBJECT_OP_CREATE &&
                                    object_kind == OBJECT_KIND_TIMER &&
                                    object_single_fd_in_range) begin
                                    timer_valid <= 1'b1;
                                    timer_lineage <= 64'd1025;
                                    timer_expirations <= 64'd0;
                                    fdr_valid[object_single_fd] <= 1'b1;
                                    fdr_revoked[object_single_fd] <= 1'b0;
                                    fdr_generation[object_single_fd] <= fdr_generation[object_single_fd] + 64'd1;
                                    fdr_rights[object_single_fd] <= CAP_RIGHT_ALL;
                                    fdr_lineage[object_single_fd] <= 64'd1025;
                                    fdr_kind[object_single_fd] <= FDR_KIND_GENERIC;
                                    sram[object_fd_store_word_index] <= store_double_low_word(
                                        sram[object_fd_store_word_index],
                                        object_fd_store_byte_lane,
                                        {56'd0, object_single_fd[7:0]}
                                    );
                                    if (object_fd_store_byte_lane != 3'd0) begin
                                        sram[object_fd_store_next_word_index] <= store_double_high_word(
                                            sram[object_fd_store_next_word_index],
                                            object_fd_store_byte_lane,
                                            {56'd0, object_single_fd[7:0]}
                                        );
                                    end
                                    gpr[dec.rd] <= {56'd0, object_single_fd[7:0]};
                                    errno_reg <= LNP64_ERR_OK;
                                    pc <= pc + 32'd1;
                                    retired_count <= retired_count + 32'd1;
                                    retire_submit_valid <= 1'b1;
                                    retire_submit_record <= retire_submit_next;
                                end else if (object_op == OBJECT_OP_CREATE &&
                                    object_kind == OBJECT_KIND_MEMORY_OBJECT &&
                                    object_dma_addr != 64'd0 &&
                                    object_memory_fd_in_range) begin
                                    memory_object_valid <= 1'b1;
                                    memory_object_lineage <= 64'd513;
                                    memory_object_len <= object_dma_addr;
                                    memory_object_byte_valid <= 1'b0;
                                    memory_object_byte_value <= 8'd0;
                                    fdr_valid[object_memory_fd] <= 1'b1;
                                    fdr_revoked[object_memory_fd] <= 1'b0;
                                    fdr_generation[object_memory_fd] <= fdr_generation[object_memory_fd] + 64'd1;
                                    fdr_rights[object_memory_fd] <= CAP_RIGHT_ALL;
                                    fdr_lineage[object_memory_fd] <= 64'd513;
                                    fdr_kind[object_memory_fd] <= FDR_KIND_GENERIC;
                                    sram[object_fd_store_word_index] <= store_double_low_word(
                                        sram[object_fd_store_word_index],
                                        object_fd_store_byte_lane,
                                        {56'd0, object_memory_fd[7:0]}
                                    );
                                    if (object_fd_store_byte_lane != 3'd0) begin
                                        sram[object_fd_store_next_word_index] <= store_double_high_word(
                                            sram[object_fd_store_next_word_index],
                                            object_fd_store_byte_lane,
                                            {56'd0, object_memory_fd[7:0]}
                                        );
                                    end
                                    gpr[dec.rd] <= {56'd0, object_memory_fd[7:0]};
                                    errno_reg <= LNP64_ERR_OK;
                                    pc <= pc + 32'd1;
                                    retired_count <= retired_count + 32'd1;
                                    retire_submit_valid <= 1'b1;
                                    retire_submit_record <= retire_submit_next;
                                end else if (object_op == OBJECT_OP_CREATE &&
                                    object_kind == OBJECT_KIND_MEMORY_OBJECT) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EINVAL;
                                    pc <= pc + 32'd1;
                                    retired_count <= retired_count + 32'd1;
                                    retire_submit_valid <= 1'b1;
                                    retire_submit_record <= retire_submit_next;
                                end else if (object_op == OBJECT_OP_CREATE &&
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
                            LNP64_OP_DOMAIN_CTL: begin
                                if (domain_op == DOMAIN_OP_CREATE &&
                                    domain_ref_live &&
                                    domain_create_slot_in_range &&
                                    !domain_valid[domain_create_slot] &&
                                    (domain_cpu_req == 64'd0 || domain_cpu_req <= domain_cpu_limit[domain_ref_slot]) &&
                                    (domain_memory_req == 64'd0 ||
                                        domain_memory_req <= domain_memory_limit[domain_ref_slot]) &&
                                    (domain_pids_req == 64'd0 ||
                                        domain_pids_req <= domain_pids_limit[domain_ref_slot]) &&
                                    (domain_fdrs_req == 64'd0 ||
                                        domain_fdrs_req <= domain_fdrs_limit[domain_ref_slot]) &&
                                    (domain_caps_req == 64'd0 ||
                                        ((domain_caps_req & ~domain_cap_mask[domain_ref_slot]) == 64'd0)) &&
                                    (domain_upcalls_req == 64'd0 ||
                                        ((domain_upcalls_req & ~domain_upcall_mask[domain_ref_slot]) == 64'd0))) begin
                                    domain_valid[domain_create_slot] <= 1'b1;
                                    domain_destroyed[domain_create_slot] <= 1'b0;
                                    domain_generation[domain_create_slot] <= 64'd1;
                                    domain_parent[domain_create_slot] <= domain_ref_id;
                                    domain_depth[domain_create_slot] <= domain_depth[domain_ref_slot] + 64'd1;
                                    domain_profile[domain_create_slot] <= domain_profile_req;
                                    domain_cpu_limit[domain_create_slot] <= domain_cpu_req == 64'd0 ?
                                        domain_cpu_limit[domain_ref_slot] : domain_cpu_req;
                                    domain_memory_limit[domain_create_slot] <= domain_memory_req == 64'd0 ?
                                        domain_memory_limit[domain_ref_slot] : domain_memory_req;
                                    domain_pids_limit[domain_create_slot] <= domain_pids_req == 64'd0 ?
                                        domain_pids_limit[domain_ref_slot] : domain_pids_req;
                                    domain_fdrs_limit[domain_create_slot] <= domain_fdrs_req == 64'd0 ?
                                        domain_fdrs_limit[domain_ref_slot] : domain_fdrs_req;
                                    domain_cap_mask[domain_create_slot] <= domain_caps_req == 64'd0 ?
                                        domain_cap_mask[domain_ref_slot] : domain_caps_req;
                                    domain_upcall_mask[domain_create_slot] <= domain_upcalls_req == 64'd0 ?
                                        domain_upcall_mask[domain_ref_slot] : domain_upcalls_req;
                                    domain_child_count[domain_create_slot] <= 64'd0;
                                    domain_frozen[domain_create_slot] <= 1'b0;
                                    domain_child_count[domain_ref_slot] <= domain_child_count[domain_ref_slot] + 64'd1;
                                    store_double_unaligned_next(domain_argblock_addr + 64'd8, domain_next_id);
                                    store_double_unaligned_next(domain_argblock_addr + 64'd16, 64'd1);
                                    store_double_unaligned_next(domain_argblock_addr + 64'd120, domain_ref_id);
                                    store_double_unaligned_next(
                                        domain_argblock_addr + 64'd128,
                                        domain_depth[domain_ref_slot] + 64'd1
                                    );
                                    gpr[dec.rd] <= domain_next_id;
                                    domain_next_id <= domain_next_id + 64'd1;
                                    errno_reg <= LNP64_ERR_OK;
                                end else if (domain_op == DOMAIN_OP_QUERY && domain_ref_live) begin
                                    store_double_unaligned_next(domain_argblock_addr + 64'd8, domain_ref_id);
                                    store_double_unaligned_next(
                                        domain_argblock_addr + 64'd16,
                                        domain_generation[domain_ref_slot]
                                    );
                                    store_double_unaligned_next(
                                        domain_argblock_addr + 64'd24,
                                        domain_profile[domain_ref_slot]
                                    );
                                    store_double_unaligned_next(
                                        domain_argblock_addr + 64'd32,
                                        domain_cpu_limit[domain_ref_slot]
                                    );
                                    store_double_unaligned_next(
                                        domain_argblock_addr + 64'd40,
                                        domain_memory_limit[domain_ref_slot]
                                    );
                                    store_double_unaligned_next(
                                        domain_argblock_addr + 64'd48,
                                        domain_pids_limit[domain_ref_slot]
                                    );
                                    store_double_unaligned_next(
                                        domain_argblock_addr + 64'd56,
                                        domain_fdrs_limit[domain_ref_slot]
                                    );
                                    store_double_unaligned_next(
                                        domain_argblock_addr + 64'd64,
                                        domain_cap_mask[domain_ref_slot]
                                    );
                                    store_double_unaligned_next(
                                        domain_argblock_addr + 64'd72,
                                        domain_upcall_mask[domain_ref_slot]
                                    );
                                    store_double_unaligned_next(domain_argblock_addr + 64'd80, retired_count);
                                    store_double_unaligned_next(domain_argblock_addr + 64'd88, 64'd0);
                                    store_double_unaligned_next(domain_argblock_addr + 64'd96, 64'd1);
                                    store_double_unaligned_next(domain_argblock_addr + 64'd104, 64'd3);
                                    store_double_unaligned_next(
                                        domain_argblock_addr + 64'd112,
                                        domain_destroyed[domain_ref_slot] ? DOMAIN_STATE_DESTROYED :
                                            (domain_frozen[domain_ref_slot] ? 64'd1 : DOMAIN_STATE_ACTIVE)
                                    );
                                    store_double_unaligned_next(
                                        domain_argblock_addr + 64'd120,
                                        domain_parent[domain_ref_slot]
                                    );
                                    store_double_unaligned_next(
                                        domain_argblock_addr + 64'd128,
                                        domain_depth[domain_ref_slot]
                                    );
                                    store_double_unaligned_next(
                                        domain_argblock_addr + 64'd136,
                                        domain_child_count[domain_ref_slot]
                                    );
                                    gpr[dec.rd] <= DOMAIN_QUERY_SIZE;
                                    errno_reg <= LNP64_ERR_OK;
                                end else if (domain_op == DOMAIN_OP_CONFIGURE && domain_ref_live &&
                                    !domain_frozen[domain_ref_slot] &&
                                    (domain_cpu_req == 64'd0 || domain_cpu_req <= domain_cpu_limit[domain_ref_slot]) &&
                                    (domain_memory_req == 64'd0 ||
                                        domain_memory_req <= domain_memory_limit[domain_ref_slot]) &&
                                    (domain_pids_req == 64'd0 ||
                                        domain_pids_req <= domain_pids_limit[domain_ref_slot]) &&
                                    (domain_fdrs_req == 64'd0 ||
                                        domain_fdrs_req <= domain_fdrs_limit[domain_ref_slot]) &&
                                    (domain_caps_req == 64'd0 ||
                                        ((domain_caps_req & ~domain_cap_mask[domain_ref_slot]) == 64'd0)) &&
                                    (domain_upcalls_req == 64'd0 ||
                                        ((domain_upcalls_req & ~domain_upcall_mask[domain_ref_slot]) == 64'd0))) begin
                                    if (domain_profile_req != 64'd0) begin
                                        domain_profile[domain_ref_slot] <= domain_profile_req;
                                    end
                                    if (domain_cpu_req != 64'd0) begin
                                        domain_cpu_limit[domain_ref_slot] <= domain_cpu_req;
                                    end
                                    if (domain_memory_req != 64'd0) begin
                                        domain_memory_limit[domain_ref_slot] <= domain_memory_req;
                                    end
                                    if (domain_pids_req != 64'd0) begin
                                        domain_pids_limit[domain_ref_slot] <= domain_pids_req;
                                    end
                                    if (domain_fdrs_req != 64'd0) begin
                                        domain_fdrs_limit[domain_ref_slot] <= domain_fdrs_req;
                                    end
                                    if (domain_caps_req != 64'd0) begin
                                        domain_cap_mask[domain_ref_slot] <= domain_caps_req;
                                        for (i = 0; i < DOMAIN_SLOT_COUNT; i = i + 1) begin
                                            if (domain_valid[i] && domain_parent[i] == domain_ref_id) begin
                                                domain_cap_mask[i] <= domain_cap_mask[i] & domain_caps_req;
                                            end
                                        end
                                    end
                                    if (domain_upcalls_req != 64'd0) begin
                                        domain_upcall_mask[domain_ref_slot] <= domain_upcalls_req;
                                        for (i = 0; i < DOMAIN_SLOT_COUNT; i = i + 1) begin
                                            if (domain_valid[i] && domain_parent[i] == domain_ref_id) begin
                                                domain_upcall_mask[i] <= domain_upcall_mask[i] & domain_upcalls_req;
                                            end
                                        end
                                    end
                                    gpr[dec.rd] <= 64'd0;
                                    errno_reg <= LNP64_ERR_OK;
                                end else if (domain_op == DOMAIN_OP_FREEZE && domain_ref_live) begin
                                    domain_frozen[domain_ref_slot] <= 1'b1;
                                    for (i = 0; i < DOMAIN_SLOT_COUNT; i = i + 1) begin
                                        if (domain_valid[i] && domain_parent[i] == domain_ref_id) begin
                                            domain_frozen[i] <= 1'b1;
                                        end
                                    end
                                    gpr[dec.rd] <= 64'd0;
                                    errno_reg <= LNP64_ERR_OK;
                                end else if (domain_op == DOMAIN_OP_RESUME && domain_ref_live) begin
                                    domain_frozen[domain_ref_slot] <= 1'b0;
                                    for (i = 0; i < DOMAIN_SLOT_COUNT; i = i + 1) begin
                                        if (domain_valid[i] && domain_parent[i] == domain_ref_id) begin
                                            domain_frozen[i] <= 1'b0;
                                        end
                                    end
                                    gpr[dec.rd] <= 64'd0;
                                    errno_reg <= LNP64_ERR_OK;
                                end else if (domain_op == DOMAIN_OP_DESTROY && domain_ref_live &&
                                    domain_ref_id != DOMAIN_ROOT_ID) begin
                                    domain_destroyed[domain_ref_slot] <= 1'b1;
                                    domain_generation[domain_ref_slot] <= domain_generation[domain_ref_slot] + 64'd1;
                                    if (domain_parent[domain_ref_slot] > 64'd0 &&
                                        (domain_parent[domain_ref_slot] - 64'd1) < DOMAIN_SLOT_COUNT &&
                                        domain_child_count[domain_parent[domain_ref_slot] - 64'd1] != 64'd0) begin
                                        domain_child_count[domain_parent[domain_ref_slot] - 64'd1] <=
                                            domain_child_count[domain_parent[domain_ref_slot] - 64'd1] - 64'd1;
                                    end
                                    gpr[dec.rd] <= 64'd0;
                                    errno_reg <= LNP64_ERR_OK;
                                end else if (domain_op == DOMAIN_OP_CREATE) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EPERM;
                                end else if (domain_op == DOMAIN_OP_QUERY || domain_op == DOMAIN_OP_DESTROY) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= RTL_ERR_ESTALE;
                                end else if (domain_op == DOMAIN_OP_CONFIGURE ||
                                    domain_op == DOMAIN_OP_FREEZE ||
                                    domain_op == DOMAIN_OP_RESUME) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EPERM;
                                end else begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EINVAL;
                                end
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                            end
                            LNP64_OP_CAP_DUP: begin
                                if (cap_engine_shadow_enabled) begin
                                    pending_unsupported <= 1'b0;
                                    command_pc <= pc;
                                    state <= CORE_SEND_CMD;
                                end else begin
                                    if (cap_flags & ~CAP_DUP_FLAG_SEAL) begin
                                        gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                        errno_reg <= LNP64_ERR_EINVAL;
                                    end else if (cap_src_stale) begin
                                        gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                        errno_reg <= RTL_ERR_ESTALE;
                                    end else if (!cap_src_live || !cap_dst_fd_in_range) begin
                                        gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                        errno_reg <= LNP64_ERR_EBADF;
                                    end else if ((fdr_rights[cap_src_fd] & CAP_RIGHT_DUP) == 64'd0 ||
                                        !cap_dup_rights_subset) begin
                                        gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                        errno_reg <= LNP64_ERR_EPERM;
                                    end else begin
                                        fdr_valid[cap_dst_fd] <= 1'b1;
                                        fdr_revoked[cap_dst_fd] <= 1'b0;
                                        fdr_generation[cap_dst_fd] <= fdr_generation[cap_dst_fd] + 64'd1;
                                        fdr_rights[cap_dst_fd] <= cap_dup_rights;
                                        fdr_lineage[cap_dst_fd] <= fdr_lineage[cap_src_fd];
                                        fdr_kind[cap_dst_fd] <= fdr_kind[cap_src_fd];
                                        gpr[dec.rd] <= FDR_TOKEN_MARKER |
                                            ((fdr_generation[cap_dst_fd] + 64'd1) << 8) |
                                            {56'd0, cap_dst_fd[7:0]};
                                        errno_reg <= LNP64_ERR_OK;
                                    end
                                    pc <= pc + 32'd1;
                                    retired_count <= retired_count + 32'd1;
                                    retire_submit_valid <= 1'b1;
                                    retire_submit_record <= retire_submit_next;
                                    m1_commit_valid <= 1'b1;
                                    m1_commit <= m1_commit_next;
                                    m1_projection_root_fd <= cap_src_fd_in_range ? cap_src_fd : 0;
                                    m1_projection_consumer_fd <= m1_current_consumer_fd();
                                end
                            end
                            LNP64_OP_CAP_SEND: begin
                                cap_engine_shadow_enabled <= 1'b0;
                                if (cap_flags != 64'd0) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EINVAL;
                                end else if (!cap_src_live || !cap_src_fd_in_range ||
                                    fdr_kind[cap_src_fd] != FDR_KIND_PIPE_WRITER ||
                                    ((fdr_rights[cap_src_fd] & (CAP_RIGHT_TRANSFER | 64'd2)) != (CAP_RIGHT_TRANSFER | 64'd2))) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EBADF;
                                end else if (cap_arg1_fd >= FDR_SLOT_COUNT ||
                                    !fdr_valid[cap_arg1_fd] || fdr_revoked[cap_arg1_fd]) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EBADF;
                                end else if ((fdr_rights[cap_arg1_fd] & CAP_RIGHT_TRANSFER) == 64'd0) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EPERM;
                                end else if (cap_queue_valid) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EAGAIN;
                                end else begin
                                    cap_queue_valid <= 1'b1;
                                    cap_queue_rights <= fdr_rights[cap_arg1_fd];
                                    cap_queue_lineage <= fdr_lineage[cap_arg1_fd];
                                    cap_queue_generation <= fdr_generation[cap_arg1_fd];
                                    cap_queue_revoked <= fdr_revoked[cap_arg1_fd];
                                    gpr[dec.rd] <= 64'd1;
                                    errno_reg <= LNP64_ERR_OK;
                                end
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                                m1_commit_valid <= 1'b1;
                                m1_commit <= m1_commit_next;
                                m1_projection_root_fd <= cap_src_fd_in_range ? cap_src_fd : 0;
                                m1_projection_consumer_fd <= m1_current_consumer_fd();
                            end
                            LNP64_OP_CAP_RECV: begin
                                cap_engine_shadow_enabled <= 1'b0;
                                if (cap_flags != 64'd0) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EINVAL;
                                end else if (!cap_src_live || !cap_src_fd_in_range ||
                                    fdr_kind[cap_src_fd] != FDR_KIND_PIPE_READER ||
                                    ((fdr_rights[cap_src_fd] & (CAP_RIGHT_TRANSFER | 64'd1)) != (CAP_RIGHT_TRANSFER | 64'd1))) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EBADF;
                                end else if (!cap_queue_valid) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EAGAIN;
                                end else if (cap_queue_revoked) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= RTL_ERR_ESTALE;
                                end else if (!cap_recv_rights_subset) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EPERM;
                                end else if (!cap_dst_fd_in_range) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EBADF;
                                end else begin
                                    cap_queue_valid <= 1'b0;
                                    cap_queue_generation <= 64'd0;
                                    fdr_valid[cap_dst_fd] <= 1'b1;
                                    fdr_revoked[cap_dst_fd] <= 1'b0;
                                    fdr_generation[cap_dst_fd] <= fdr_generation[cap_dst_fd] + 64'd1;
                                    fdr_rights[cap_dst_fd] <= cap_recv_rights;
                                    fdr_lineage[cap_dst_fd] <= cap_queue_lineage;
                                    fdr_kind[cap_dst_fd] <= FDR_KIND_GENERIC;
                                    gpr[dec.rd] <= FDR_TOKEN_MARKER |
                                        ((fdr_generation[cap_dst_fd] + 64'd1) << 8) |
                                        {56'd0, cap_dst_fd[7:0]};
                                    errno_reg <= LNP64_ERR_OK;
                                end
                                pc <= pc + 32'd1;
                                retired_count <= retired_count + 32'd1;
                                retire_submit_valid <= 1'b1;
                                retire_submit_record <= retire_submit_next;
                                m1_commit_valid <= 1'b1;
                                m1_commit <= m1_commit_next;
                                m1_projection_root_fd <= cap_src_fd_in_range ? cap_src_fd : 0;
                                m1_projection_consumer_fd <= m1_current_consumer_fd();
                            end
                            LNP64_OP_CAP_REVOKE: begin
                                cap_engine_shadow_enabled <= 1'b0;
                                if (cap_src_stale) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= RTL_ERR_ESTALE;
                                end else if (cap_src_live && ((fdr_rights[cap_src_fd] & CAP_RIGHT_REVOKE) != 64'd0)) begin
                                    for (i = 0; i < FDR_SLOT_COUNT; i = i + 1) begin
                                        if (fdr_valid[i] && !fdr_revoked[i] && fdr_lineage[i] == fdr_lineage[cap_src_fd]) begin
                                            fdr_revoked[i] <= 1'b1;
                                            fdr_generation[i] <= fdr_generation[i] + 64'd1;
                                        end
                                    end
                                    if (cap_queue_valid && !cap_queue_revoked &&
                                        cap_queue_lineage == fdr_lineage[cap_src_fd]) begin
                                        cap_queue_revoked <= 1'b1;
                                    end
                                    gpr[dec.rd] <= cap_revoke_count;
                                    errno_reg <= LNP64_ERR_OK;
                                end else if (cap_src_fd_in_range && fdr_valid[cap_src_fd] &&
                                    !fdr_revoked[cap_src_fd] &&
                                    ((fdr_rights[cap_src_fd] & CAP_RIGHT_REVOKE) == 64'd0)) begin
                                    gpr[dec.rd] <= 64'hffff_ffff_ffff_ffff;
                                    errno_reg <= LNP64_ERR_EPERM;
                                end else if (dma_buffer_ref_matches(object_op) && !dma_buffer_object_revoked) begin
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
                                m1_commit_valid <= 1'b1;
                                m1_commit <= m1_commit_next;
                                m1_projection_root_fd <= cap_src_fd_in_range ? cap_src_fd : 0;
                                m1_projection_consumer_fd <= m1_current_consumer_fd();
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
                        if (!pending_unsupported && dec.opcode == LNP64_OP_CAP_DUP) begin
                            if (rsp.status == LNP64_STATUS_OK &&
                                rsp.errno_value == LNP64_ERR_OK &&
                                cap_src_fd_in_range &&
                                cap_dst_fd_in_range) begin
                                fdr_valid[cap_dst_fd] <= 1'b1;
                                fdr_revoked[cap_dst_fd] <= 1'b0;
                                fdr_generation[cap_dst_fd] <= fdr_generation[cap_dst_fd] + 64'd1;
                                fdr_rights[cap_dst_fd] <= cap_dup_rights;
                                fdr_lineage[cap_dst_fd] <= fdr_lineage[cap_src_fd];
                                fdr_kind[cap_dst_fd] <= fdr_kind[cap_src_fd];
                            end
                            m1_commit_valid <= 1'b1;
                            m1_commit <= m1_commit_next;
                            m1_projection_root_fd <= cap_src_fd_in_range ? cap_src_fd : 0;
                            m1_projection_consumer_fd <= m1_current_consumer_fd();
                        end
                        if (!pending_unsupported &&
                            dec.opcode == LNP64_OP_OBJECT_CTL &&
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
                            state <= CORE_SWITCH;
                        end
                    end
                end
                CORE_SWITCH: begin
                    cmd_valid <= 1'b0;
                    rsp_ready <= 1'b0;
                    thread_pc[active_thread_slot] <= pc;
                    thread_return_stack_depth[active_thread_slot] <= return_stack_depth;
                    thread_link_register[active_thread_slot] <= link_register;
                    thread_cmp_zero[active_thread_slot] <= cmp_zero;
                    thread_cmp_negative[active_thread_slot] <= cmp_negative;
                    thread_cmp_greater[active_thread_slot] <= cmp_greater;
                    thread_cmp_below[active_thread_slot] <= cmp_below;
                    thread_cmp_above[active_thread_slot] <= cmp_above;
                    for (i = 0; i < 32; i = i + 1) begin
                        thread_gpr[active_thread_slot][i] <= gpr[i];
                    end
                    for (i = 0; i < RETURN_STACK_DEPTH; i = i + 1) begin
                        thread_return_stack[active_thread_slot][i] <= return_stack[i];
                    end
                    if (next_thread_slot != active_thread_slot) begin
                        active_thread_slot <= next_thread_slot;
                        pc <= thread_pc[next_thread_slot];
                        return_stack_depth <= thread_return_stack_depth[next_thread_slot];
                        link_register <= thread_link_register[next_thread_slot];
                        cmp_zero <= thread_cmp_zero[next_thread_slot];
                        cmp_negative <= thread_cmp_negative[next_thread_slot];
                        cmp_greater <= thread_cmp_greater[next_thread_slot];
                        cmp_below <= thread_cmp_below[next_thread_slot];
                        cmp_above <= thread_cmp_above[next_thread_slot];
                        for (i = 0; i < 32; i = i + 1) begin
                            gpr[i] <= thread_gpr[next_thread_slot][i];
                        end
                        for (i = 0; i < RETURN_STACK_DEPTH; i = i + 1) begin
                            return_stack[i] <= thread_return_stack[next_thread_slot][i];
                        end
                    end
                    state <= CORE_EXEC;
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

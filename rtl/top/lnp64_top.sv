`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_top #(
    parameter int CORE_TILE_COUNT = 2,
    parameter int MAX_SUPPORTED_TILE_COUNT = 4
) (
    input  logic clk,
    input  logic reset_n,
    input  logic force_boot_fault,
    input  logic sim_event_inject,
    input  logic sim_fault_inject,
    input  logic sim_watchdog_inject,

    output logic boot_stable,
    output logic pid1_exactly_one_location,
    output logic pid1_completed,
    output logic [31:0] retired_count,
    output logic env_get_ok,
    output logic sram_ldst_ok,
    output logic unsupported_failed_closed,
    output logic stub_failed_closed,
    output logic uart_seen,
    output logic [7:0] uart_byte_seen,
    output logic event_woke_thread,
    output logic structured_fault_seen,
    output logic watchdog_degraded_seen,
    output logic no_raw_authority_visible,
    output logic coherence_paths_live,
    output logic multicore_no_duplicate_tid,
    output logic tile_reset_stable_all,
    output logic tile1_observable_idle,
    output logic cross_tile_wake_one,
    output logic tile_fault_isolated,
    output logic [31:0] topology_tile_count_seen,
    output logic [63:0] topology_enabled_tile_mask_seen,
    output logic [31:0] topology_coherence_domain_seen,
    output logic [31:0] topology_active_window_base_seen,
    output logic [31:0] topology_active_window_count_seen
);
    localparam logic [63:0] ENABLED_TILE_MASK =
        CORE_TILE_COUNT >= 64 ? 64'hffff_ffff_ffff_ffff : ((64'd1 << CORE_TILE_COUNT) - 64'd1);
    localparam logic [31:0] COHERENCE_DOMAIN_ID = 32'd1;
    localparam logic [31:0] ACTIVE_WINDOW_BASE = 32'd0;
    localparam logic [31:0] ACTIVE_WINDOW_COUNT = CORE_TILE_COUNT[31:0];

    initial begin
        if (CORE_TILE_COUNT < 1 || CORE_TILE_COUNT > MAX_SUPPORTED_TILE_COUNT) begin
            $fatal(1, "CORE_TILE_COUNT must be in the supported 1..4 S0/stress range");
        end
    end

    logic logic_reset_n;
    logic boot_valid;
    logic release_core;
    logic boot_fault_valid;
    lnp64_fault_t boot_fault;
    lnp64_domain_t root_domain;
    lnp64_cap_t root_fdr;
    logic [31:0] boot_pid1;
    logic [31:0] boot_tid1;
    lnp64_thread_sched_t pid1_context;

    logic [CORE_TILE_COUNT-1:0] tile_enable;
    logic [CORE_TILE_COUNT-1:0] tile_reset_stable;
    logic [CORE_TILE_COUNT-1:0] core_tile_idle;
    logic [CORE_TILE_COUNT-1:0] core_tile_running;
    logic [CORE_TILE_COUNT-1:0] core_tile_parked;
    logic [CORE_TILE_COUNT-1:0] core_tile_faulted;
    logic [CORE_TILE_COUNT-1:0] scheduler_tile_faulted;
    logic [CORE_TILE_COUNT-1:0] sched_issue_valid;
    logic [CORE_TILE_COUNT*32-1:0] sched_issue_tid_flat;
    logic sched_no_duplicate_issue;
    logic sched_tile1_schedulable_idle;
    logic sched_tile_fault_isolated;

    logic [CORE_TILE_COUNT-1:0] core_cmd_valid_vec;
    logic [CORE_TILE_COUNT-1:0] core_cmd_ready_vec;
    lnp64_cmd_t core_cmd_vec [CORE_TILE_COUNT];
    logic [CORE_TILE_COUNT-1:0] core_rsp_valid_vec;
    logic [CORE_TILE_COUNT-1:0] core_rsp_ready_vec;
    lnp64_rsp_t core_rsp_vec [CORE_TILE_COUNT];
    logic [CORE_TILE_COUNT-1:0] core_yielded_vec;
    logic [CORE_TILE_COUNT-1:0] core_pid1_runnable_vec;
    logic [CORE_TILE_COUNT-1:0] core_pid1_parked_vec;
    logic [CORE_TILE_COUNT-1:0] core_done_vec;
    logic [CORE_TILE_COUNT-1:0] retire_submit_valid_vec;
    logic [CORE_TILE_COUNT-1:0] m1_commit_valid_vec;
    logic [CORE_TILE_COUNT-1:0] park_submit_valid_vec;
    logic [CORE_TILE_COUNT-1:0] submit_valid_vec;
    lnp64_retire_submit_t retire_submit_record_vec [CORE_TILE_COUNT];
    lnp64_m1_cap_commit_t core_m1_commit_vec [CORE_TILE_COUNT];
    lnp64_m1_cap_commit_t m1_commit_vec [CORE_TILE_COUNT];
    lnp64_m1_cap_commit_t cap_m1_commit_latched_vec [CORE_TILE_COUNT];
    logic [CORE_TILE_COUNT-1:0] cap_m1_commit_latched_valid_vec;
    lnp64_m1_state_projection_t core_m1_pre_state_projection_vec [CORE_TILE_COUNT];
    lnp64_m1_state_projection_t core_m1_state_projection_vec [CORE_TILE_COUNT];
    lnp64_m1_state_projection_t m1_pre_state_projection_vec [CORE_TILE_COUNT];
    lnp64_m1_state_projection_t m1_state_projection_vec [CORE_TILE_COUNT];
    lnp64_m1_state_projection_t cap_m1_pre_state_projection_latched_vec [CORE_TILE_COUNT];
    lnp64_m1_state_projection_t cap_m1_state_projection_latched_vec [CORE_TILE_COUNT];
    lnp64_thread_sched_t park_submit_record_vec [CORE_TILE_COUNT];
    lnp64_thread_sched_t submit_record_vec [CORE_TILE_COUNT];
    logic [CORE_TILE_COUNT-1:0] icache_invalidate_vec;
    logic [CORE_TILE_COUNT-1:0] dcache_writeback_vec;
    logic [CORE_TILE_COUNT-1:0] tlb_invalidate_vec;
    logic [CORE_TILE_COUNT-1:0] icache_invalidate_seen;
    logic [CORE_TILE_COUNT-1:0] dcache_writeback_seen;
    logic [CORE_TILE_COUNT-1:0] tlb_invalidate_seen;

    logic core_cmd_valid;
    logic core_cmd_ready;
    lnp64_cmd_t core_cmd;
    logic selected_cmd_valid;
    lnp64_cmd_t selected_cmd;
    logic core_rsp_valid;
    logic core_rsp_ready;
    lnp64_rsp_t core_rsp;
    logic [31:0] selected_cmd_tile;

    logic [31:0] retired_count_vec [CORE_TILE_COUNT];
    logic [63:0] env_features_seen_vec [CORE_TILE_COUNT];
    logic [31:0] env_tile_count_seen_vec [CORE_TILE_COUNT];
    logic [63:0] env_enabled_tile_mask_seen_vec [CORE_TILE_COUNT];
    logic [31:0] env_coherence_domain_seen_vec [CORE_TILE_COUNT];
    logic [31:0] env_active_window_base_seen_vec [CORE_TILE_COUNT];
    logic [31:0] env_active_window_count_seen_vec [CORE_TILE_COUNT];
    logic [63:0] ld_value_seen_vec [CORE_TILE_COUNT];
    logic [CORE_TILE_COUNT-1:0] object_stub_failed_closed_vec;
    logic [CORE_TILE_COUNT-1:0] unsupported_failed_closed_vec;
    logic [CORE_TILE_COUNT-1:0] core_raw_authority_visible_vec;
    logic [63:0] env_features_seen;
    logic [63:0] ld_value_seen;
    logic core_raw_authority_visible;

    logic sched_pid1_runnable;
    logic sched_pid1_parked;
    logic event_valid;
    logic wake_valid;
    lnp64_event_t event_record;
    logic [31:0] event_counter;
    logic cross_tile_wake_valid;
    logic [31:0] event_wake_counter;
    logic cross_tile_wake_observed;
    logic cross_tile_duplicate_wake;
    logic fault_valid;
    lnp64_fault_t fault_record;
    logic routed_fault_valid;
    lnp64_fault_t routed_fault;
    logic watchdog_fault_valid;
    lnp64_fault_t watchdog_fault;
    logic watchdog_degraded;
    logic uart_valid;
    logic [7:0] uart_byte;
    lnp64_completion_t completion;
    logic completion_valid;
    logic [15:0] errno_value;
    lnp64_quote_t quote;
    logic [63:0] env_feature_bits;
    logic [31:0] env_limit_threads;
    logic cap_cmd_valid;
    logic cap_cmd_ready;
    lnp64_cmd_t cap_cmd;
    logic cap_rsp_valid;
    logic cap_rsp_ready;
    lnp64_rsp_t cap_rsp;
    logic cap_m1_commit_valid;
    lnp64_m1_cap_commit_t cap_m1_commit;
    lnp64_m1_state_projection_t cap_m1_pre_state_projection;
    lnp64_m1_state_projection_t cap_m1_state_projection;
    logic object_cmd_valid;
    logic object_cmd_ready;
    lnp64_cmd_t object_cmd;
    logic object_rsp_valid;
    logic object_rsp_ready;
    lnp64_rsp_t object_rsp;
    logic domain_cmd_valid;
    logic domain_cmd_ready;
    lnp64_cmd_t domain_cmd;
    logic domain_rsp_valid;
    logic domain_rsp_ready;
    lnp64_rsp_t domain_rsp;
    logic heap_cmd_valid;
    logic heap_cmd_ready;
    lnp64_cmd_t heap_cmd;
    logic heap_rsp_valid;
    logic heap_rsp_ready;
    lnp64_rsp_t heap_rsp;
    logic vma_cmd_valid;
    logic vma_cmd_ready;
    lnp64_cmd_t vma_cmd;
    logic vma_rsp_valid;
    logic vma_rsp_ready;
    lnp64_rsp_t vma_rsp;
    logic dma_cmd_valid;
    logic dma_cmd_ready;
    lnp64_cmd_t dma_cmd;
    logic dma_rsp_valid;
    logic dma_rsp_ready;
    lnp64_rsp_t dma_rsp;
    logic object_cap_sync_valid;
    logic [31:0] object_cap_sync_reader_fd;
    logic [31:0] object_cap_sync_writer_fd;
    logic object_cap_sync_single_valid;
    logic [31:0] object_cap_sync_single_fd;
    logic [2:0] object_cap_sync_single_kind;
    logic [63:0] object_cap_sync_single_lineage;
    logic page_allocator_idle;
    logic metadata_idle;
    logic futex_idle;
    logic memory_visibility_live;
    logic dma_visibility_live;
    logic memory_raw_pa_visible;
    logic dma_raw_visible;
    logic storage_raw_visible;
    logic eth_irq_visible;
    logic pcie_dma_visible;
    logic pcie_irq_visible;
    logic policy_allow;
    logic typed_control_idle;
    logic namespace_idle;
    logic stream_frontend_idle;
    logic ddr_absent_or_idle;
    logic sd_spi_absent_or_idle;
    logic boot_image_idle;

    localparam logic [63:0] REQUIRED_S0_FEATURE_MASK =
        LNP64_FEATURE_CORE_TILE |
        LNP64_FEATURE_DECODE |
        LNP64_FEATURE_ENV_GET |
        LNP64_FEATURE_SCHEDULER_STUB |
        LNP64_FEATURE_EVENT_STUB |
        LNP64_FEATURE_CAP_STUB |
        LNP64_FEATURE_DOMAIN_STUB |
        LNP64_FEATURE_RAS_STUB |
        LNP64_FEATURE_UART_STUB |
        LNP64_FEATURE_VMA_ABSENT |
        LNP64_FEATURE_DMA_ABSENT |
        LNP64_FEATURE_HEAP_STUB |
        LNP64_FEATURE_FUTEX_STUB |
        LNP64_FEATURE_CLASSIFIER_STUB |
        LNP64_FEATURE_STORAGE_STUB |
        LNP64_FEATURE_ETH_STUB |
        LNP64_FEATURE_PCIE_STUB;
    lnp64_cmd_t zero_cmd;

    assign zero_cmd = '0;
    assign tile_enable = ENABLED_TILE_MASK[CORE_TILE_COUNT-1:0];
    assign retired_count = retired_count_vec[0];
    assign env_features_seen = env_features_seen_vec[0];
    assign ld_value_seen = ld_value_seen_vec[0];
    assign stub_failed_closed = object_stub_failed_closed_vec[0];
    assign unsupported_failed_closed = unsupported_failed_closed_vec[0];
    assign core_raw_authority_visible = |core_raw_authority_visible_vec;
    assign multicore_no_duplicate_tid = sched_no_duplicate_issue;
    assign tile_reset_stable_all = &tile_reset_stable;
    assign cross_tile_wake_one = cross_tile_wake_observed && !cross_tile_duplicate_wake;
    assign tile_fault_isolated = sched_tile_fault_isolated;
    assign topology_tile_count_seen = env_tile_count_seen_vec[0];
    assign topology_enabled_tile_mask_seen = env_enabled_tile_mask_seen_vec[0];
    assign topology_coherence_domain_seen = env_coherence_domain_seen_vec[0];
    assign topology_active_window_base_seen = env_active_window_base_seen_vec[0];
    assign topology_active_window_count_seen = env_active_window_count_seen_vec[0];

    lnp64_clock_reset clock_reset_i(
        .clk(clk),
        .reset_n(reset_n),
        .logic_reset_n(logic_reset_n)
    );

    lnp64_reset_boot reset_boot_i(
        .clk(clk),
        .reset_n(logic_reset_n),
        .force_boot_fault(force_boot_fault),
        .boot_valid(boot_valid),
        .release_core(release_core),
        .boot_fault_valid(boot_fault_valid),
        .boot_fault(boot_fault),
        .root_domain(root_domain),
        .root_fdr(root_fdr),
        .pid1(boot_pid1),
        .tid1(boot_tid1)
    );

    lnp64_thread_context thread_context_i(
        .clk(clk),
        .reset_n(logic_reset_n),
        .boot_valid(boot_valid),
        .pid1_context(pid1_context)
    );

    genvar tile_id;
    generate
        for (tile_id = 0; tile_id < CORE_TILE_COUNT; tile_id = tile_id + 1) begin : core_tiles
            lnp64_core_tile #(
                .TILE_ID(tile_id)
            ) core_i (
                .clk(clk),
                .reset_n(logic_reset_n),
                .tile_enable(tile_enable[tile_id]),
                .release_core(
                    release_core &&
                    sched_issue_valid[tile_id] &&
                    sched_issue_tid_flat[tile_id*32 +: 32] == 32'd1
                ),
                .topology_tile_count(CORE_TILE_COUNT[31:0]),
                .topology_enabled_tile_mask(ENABLED_TILE_MASK),
                .topology_coherence_domain_id(COHERENCE_DOMAIN_ID),
                .topology_active_window_base(ACTIVE_WINDOW_BASE),
                .topology_active_window_count(ACTIVE_WINDOW_COUNT),
                .cmd_valid(core_cmd_valid_vec[tile_id]),
                .cmd_ready(core_cmd_ready_vec[tile_id]),
                .cmd(core_cmd_vec[tile_id]),
                .rsp_valid(core_rsp_valid_vec[tile_id]),
                .rsp_ready(core_rsp_ready_vec[tile_id]),
                .rsp(core_rsp_vec[tile_id]),
                .yielded(core_yielded_vec[tile_id]),
                .wake_valid(wake_valid && tile_id == 0),
                .done(core_done_vec[tile_id]),
                .tile_reset_stable(tile_reset_stable[tile_id]),
                .tile_idle(core_tile_idle[tile_id]),
                .tile_running(core_tile_running[tile_id]),
                .tile_parked(core_tile_parked[tile_id]),
                .tile_faulted(core_tile_faulted[tile_id]),
                .tile_telemetry_counter(),
                .tile_fault_counter(),
                .retire_submit_valid(retire_submit_valid_vec[tile_id]),
                .retire_submit_record(retire_submit_record_vec[tile_id]),
                .m1_commit_valid(m1_commit_valid_vec[tile_id]),
                .m1_commit(core_m1_commit_vec[tile_id]),
                .m1_pre_state_projection(core_m1_pre_state_projection_vec[tile_id]),
                .m1_state_projection(core_m1_state_projection_vec[tile_id]),
                .park_submit_valid(park_submit_valid_vec[tile_id]),
                .park_submit_record(park_submit_record_vec[tile_id]),
                .submit_valid(submit_valid_vec[tile_id]),
                .submit_record(submit_record_vec[tile_id]),
                .icache_invalidate(icache_invalidate_vec[tile_id]),
                .icache_invalidate_ack(1'b1),
                .dcache_writeback(dcache_writeback_vec[tile_id]),
                .dcache_writeback_ack(1'b1),
                .tlb_invalidate(tlb_invalidate_vec[tile_id]),
                .tlb_invalidate_ack(1'b1),
                .pid1_runnable(core_pid1_runnable_vec[tile_id]),
                .pid1_parked(core_pid1_parked_vec[tile_id]),
                .retired_count(retired_count_vec[tile_id]),
                .env_features_seen(env_features_seen_vec[tile_id]),
                .env_tile_count_seen(env_tile_count_seen_vec[tile_id]),
                .env_enabled_tile_mask_seen(env_enabled_tile_mask_seen_vec[tile_id]),
                .env_coherence_domain_seen(env_coherence_domain_seen_vec[tile_id]),
                .env_active_window_base_seen(env_active_window_base_seen_vec[tile_id]),
                .env_active_window_count_seen(env_active_window_count_seen_vec[tile_id]),
                .ld_value_seen(ld_value_seen_vec[tile_id]),
                .object_stub_failed_closed(object_stub_failed_closed_vec[tile_id]),
                .unsupported_failed_closed(unsupported_failed_closed_vec[tile_id]),
                .raw_authority_visible(core_raw_authority_visible_vec[tile_id])
            );
            logic cap_m1_projection_live;

            assign cap_m1_projection_live =
                cap_rsp_valid && cap_rsp_ready && cap_m1_commit_valid &&
                cap_rsp.tile_id == tile_id[31:0];
            assign m1_commit_vec[tile_id] = cap_m1_commit_latched_valid_vec[tile_id] ?
                cap_m1_commit_latched_vec[tile_id] : '0;
            assign m1_pre_state_projection_vec[tile_id] =
                cap_m1_projection_live ? cap_m1_pre_state_projection :
                cap_m1_commit_latched_valid_vec[tile_id] ?
                    cap_m1_pre_state_projection_latched_vec[tile_id] :
                    '0;
            assign m1_state_projection_vec[tile_id] =
                cap_m1_projection_live ? cap_m1_state_projection :
                cap_m1_commit_latched_valid_vec[tile_id] ?
                    cap_m1_state_projection_latched_vec[tile_id] :
                    '0;

            lnp64_issue_retire issue_retire_i(
                .clk(clk),
                .reset_n(logic_reset_n),
                .retire_valid(retire_submit_valid_vec[tile_id]),
                .retire_counter()
            );
        end
    endgenerate

    integer arb_i;
    always_comb begin
        selected_cmd_tile = 32'd0;
        selected_cmd_valid = 1'b0;
        selected_cmd = '0;
        for (arb_i = 0; arb_i < CORE_TILE_COUNT; arb_i = arb_i + 1) begin
            core_cmd_ready_vec[arb_i] = 1'b0;
            if (!selected_cmd_valid && core_cmd_valid_vec[arb_i]) begin
                selected_cmd_tile = arb_i[31:0];
                selected_cmd_valid = 1'b1;
                selected_cmd = core_cmd_vec[arb_i];
            end
        end
        for (arb_i = 0; arb_i < CORE_TILE_COUNT; arb_i = arb_i + 1) begin
            core_cmd_ready_vec[arb_i] =
                !core_cmd_valid && selected_cmd_valid && selected_cmd_tile == arb_i[31:0];
        end
    end

    always_ff @(posedge clk or negedge logic_reset_n) begin
        if (!logic_reset_n) begin
            core_cmd_valid <= 1'b0;
            core_cmd <= '0;
        end else begin
            if (core_cmd_valid && core_cmd_ready) begin
                core_cmd_valid <= 1'b0;
            end
            if (!core_cmd_valid && selected_cmd_valid) begin
                core_cmd_valid <= 1'b1;
                core_cmd <= selected_cmd;
            end
        end
    end

    integer rsp_i;
    always_comb begin
        core_rsp_ready = 1'b0;
        for (rsp_i = 0; rsp_i < CORE_TILE_COUNT; rsp_i = rsp_i + 1) begin
            core_rsp_vec[rsp_i] = core_rsp;
            core_rsp_valid_vec[rsp_i] = core_rsp_valid && core_rsp.tile_id == rsp_i[31:0];
            if (core_rsp_valid_vec[rsp_i]) begin
                core_rsp_ready = core_rsp_ready | core_rsp_ready_vec[rsp_i];
            end
        end
    end

    integer m1_latch_i;
    always_ff @(posedge clk or negedge logic_reset_n) begin
        if (!logic_reset_n) begin
            cap_m1_commit_latched_valid_vec <= '0;
            for (m1_latch_i = 0; m1_latch_i < CORE_TILE_COUNT; m1_latch_i = m1_latch_i + 1) begin
                cap_m1_commit_latched_vec[m1_latch_i] <= '0;
                cap_m1_pre_state_projection_latched_vec[m1_latch_i] <= '0;
                cap_m1_state_projection_latched_vec[m1_latch_i] <= '0;
            end
        end else if (cap_rsp_valid && cap_rsp_ready && cap_m1_commit_valid &&
            cap_rsp.tile_id < CORE_TILE_COUNT[31:0]) begin
            cap_m1_commit_latched_valid_vec[cap_rsp.tile_id] <= 1'b1;
            cap_m1_commit_latched_vec[cap_rsp.tile_id] <= cap_m1_commit;
            cap_m1_pre_state_projection_latched_vec[cap_rsp.tile_id] <= cap_m1_pre_state_projection;
            cap_m1_state_projection_latched_vec[cap_rsp.tile_id] <= cap_m1_state_projection;
        end
    end

`ifndef SYNTHESIS
    function automatic logic top_m1_authority_projection_slots_match(
        input lnp64_m1_state_projection_t left,
        input lnp64_m1_state_projection_t right
    );
        begin
            top_m1_authority_projection_slots_match =
                left.object_gen == right.object_gen &&
                left.created_object_created == right.created_object_created &&
                left.created_object_gen == right.created_object_gen &&
                left.root_object_id == right.root_object_id &&
                left.root_generation == right.root_generation &&
                left.root_domain_id == right.root_domain_id &&
                left.root_lineage_epoch == right.root_lineage_epoch &&
                left.root_sealed == right.root_sealed &&
                left.root_rights == right.root_rights &&
                left.consumer_object_id == right.consumer_object_id &&
                left.consumer_generation == right.consumer_generation &&
                left.consumer_domain_id == right.consumer_domain_id &&
                left.consumer_lineage_epoch == right.consumer_lineage_epoch &&
                left.consumer_sealed == right.consumer_sealed &&
                left.consumer_rights == right.consumer_rights &&
                left.sent_valid == right.sent_valid &&
                left.sent_object_id == right.sent_object_id &&
                left.sent_generation == right.sent_generation &&
                left.sent_domain_id == right.sent_domain_id &&
                left.sent_lineage_epoch == right.sent_lineage_epoch &&
                left.sent_sealed == right.sent_sealed &&
                left.sent_rights == right.sent_rights &&
                left.minted_valid == right.minted_valid &&
                left.minted_object_id == right.minted_object_id &&
                left.minted_generation == right.minted_generation &&
                left.minted_domain_id == right.minted_domain_id &&
                left.minted_lineage_epoch == right.minted_lineage_epoch &&
                left.minted_sealed == right.minted_sealed &&
                left.minted_rights == right.minted_rights &&
                left.wake_pending == right.wake_pending &&
                left.transfer_valid == right.transfer_valid &&
                left.has_revoked_generation == right.has_revoked_generation &&
                left.revoked_generation == right.revoked_generation;
        end
    endfunction

    function automatic logic top_m1_non_ok_failure_witness_matches(
        input lnp64_m1_cap_commit_t commit,
        input lnp64_m1_state_projection_t state
    );
        begin
            unique case (commit.status)
                LNP64_ERR_ESTALE:
                    top_m1_non_ok_failure_witness_matches =
                        state.stale_rejected && state.revoked_rejected;
                LNP64_ERR_EPERM,
                LNP64_ERR_EBADF:
                    top_m1_non_ok_failure_witness_matches = state.failed_no_authority;
                LNP64_ERR_EAGAIN:
                    top_m1_non_ok_failure_witness_matches = state.full_was_explicit;
                default:
                    top_m1_non_ok_failure_witness_matches = 1'b1;
            endcase
        end
    endfunction

    integer m1_assert_i;
    always_ff @(posedge clk or negedge logic_reset_n) begin
        if (!logic_reset_n) begin
        end else begin
            for (m1_assert_i = 0; m1_assert_i < CORE_TILE_COUNT; m1_assert_i = m1_assert_i + 1) begin
                if (m1_commit_valid_vec[m1_assert_i]) begin
                    assert (retire_submit_valid_vec[m1_assert_i])
                        else $fatal(1, "SG-AUTH M1 commit was not tied to a tile-local retired instruction");
                    assert (retire_submit_record_vec[m1_assert_i].tile_id == m1_assert_i[31:0])
                        else $fatal(1, "SG-AUTH M1 retire tile id drifted from top-level tile vector");
                    assert (cap_m1_commit_latched_valid_vec[m1_assert_i])
                        else $fatal(1, "SG-AUTH M1 retire lacked cap-engine-owned commit");
                    assert (m1_pre_state_projection_vec[m1_assert_i].op == m1_commit_vec[m1_assert_i].op)
                        else $fatal(1, "SG-AUTH M1 pre-state projection op drifted from cap-engine commit");
                    assert (m1_pre_state_projection_vec[m1_assert_i].status == m1_commit_vec[m1_assert_i].status)
                        else $fatal(1, "SG-AUTH M1 pre-state projection status drifted from cap-engine commit");
                    assert (m1_state_projection_vec[m1_assert_i].op == m1_commit_vec[m1_assert_i].op)
                        else $fatal(1, "SG-AUTH M1 state projection op drifted from cap-engine commit");
                    assert (m1_state_projection_vec[m1_assert_i].status == m1_commit_vec[m1_assert_i].status)
                        else $fatal(1, "SG-AUTH M1 state projection status drifted from cap-engine commit");
                    if (m1_commit_vec[m1_assert_i].status == LNP64_ERR_OK) begin
                        unique case (m1_commit_vec[m1_assert_i].op)
                            LNP64_M1_COMMIT_CAP_DUP,
                            LNP64_M1_COMMIT_CAP_RECV: begin
                                assert (m1_state_projection_vec[m1_assert_i].consumer_object_id ==
                                    m1_commit_vec[m1_assert_i].object_id)
                                    else $fatal(1, "SG-AUTH M1 consumer object id drifted from cap-engine commit");
                                assert (m1_state_projection_vec[m1_assert_i].consumer_generation ==
                                    m1_commit_vec[m1_assert_i].fdr_gen)
                                    else $fatal(1, "SG-AUTH M1 consumer generation drifted from cap-engine commit");
                                assert (m1_state_projection_vec[m1_assert_i].consumer_rights ==
                                    m1_commit_vec[m1_assert_i].rights_mask)
                                    else $fatal(1, "SG-AUTH M1 consumer rights drifted from cap-engine commit");
                                assert (m1_state_projection_vec[m1_assert_i].consumer_lineage_epoch ==
                                    m1_commit_vec[m1_assert_i].lineage_epoch)
                                    else $fatal(1, "SG-AUTH M1 consumer lineage drifted from cap-engine commit");
                            end
                            LNP64_M1_COMMIT_CAP_SEND: begin
                                assert (m1_state_projection_vec[m1_assert_i].sent_valid)
                                    else $fatal(1, "SG-AUTH M1 capSend did not project a sent cap");
                                assert (m1_state_projection_vec[m1_assert_i].sent_object_id ==
                                    m1_commit_vec[m1_assert_i].object_id)
                                    else $fatal(1, "SG-AUTH M1 sent object id drifted from cap-engine commit");
                                assert (m1_state_projection_vec[m1_assert_i].sent_generation ==
                                    m1_commit_vec[m1_assert_i].fdr_gen)
                                    else $fatal(1, "SG-AUTH M1 sent generation drifted from cap-engine commit");
                                assert (m1_state_projection_vec[m1_assert_i].sent_rights ==
                                    m1_commit_vec[m1_assert_i].rights_mask)
                                    else $fatal(1, "SG-AUTH M1 sent rights drifted from cap-engine commit");
                                assert (m1_state_projection_vec[m1_assert_i].sent_lineage_epoch ==
                                    m1_commit_vec[m1_assert_i].lineage_epoch)
                                    else $fatal(1, "SG-AUTH M1 sent lineage drifted from cap-engine commit");
                            end
                            LNP64_M1_COMMIT_CAP_REVOKE: begin
                                assert (m1_state_projection_vec[m1_assert_i].root_object_id ==
                                    m1_commit_vec[m1_assert_i].object_id)
                                    else $fatal(1, "SG-AUTH M1 revoked root object id drifted from cap-engine commit");
                                assert (m1_state_projection_vec[m1_assert_i].root_generation ==
                                    m1_commit_vec[m1_assert_i].fdr_gen)
                                    else $fatal(1, "SG-AUTH M1 revoked root generation drifted from cap-engine commit");
                                assert (m1_state_projection_vec[m1_assert_i].root_lineage_epoch ==
                                    m1_commit_vec[m1_assert_i].lineage_epoch)
                                    else $fatal(1, "SG-AUTH M1 revoked root lineage drifted from cap-engine commit");
                            end
                            default: begin
                            end
                        endcase
                    end else begin
                        assert (top_m1_authority_projection_slots_match(
                            m1_pre_state_projection_vec[m1_assert_i],
                            m1_state_projection_vec[m1_assert_i]
                        )) else $fatal(1, "SG-AUTH M1 non-OK top-level commit changed authority projection slots");
                        assert (top_m1_non_ok_failure_witness_matches(
                            m1_commit_vec[m1_assert_i],
                            m1_state_projection_vec[m1_assert_i]
                        )) else $fatal(1, "SG-AUTH M1 non-OK top-level commit lacked matching failure witness");
                    end
                end
            end
        end
    end
`endif

    lnp64_engine_router engine_router_i(
        .clk(clk),
        .reset_n(logic_reset_n),
        .cmd_valid(core_cmd_valid),
        .cmd_ready(core_cmd_ready),
        .cmd(core_cmd),
        .rsp_valid(core_rsp_valid),
        .rsp_ready(core_rsp_ready),
        .rsp(core_rsp),
        .cap_cmd_valid(cap_cmd_valid),
        .cap_cmd_ready(cap_cmd_ready),
        .cap_cmd(cap_cmd),
        .cap_rsp_valid(cap_rsp_valid),
        .cap_rsp_ready(cap_rsp_ready),
        .cap_rsp(cap_rsp),
        .object_cmd_valid(object_cmd_valid),
        .object_cmd_ready(object_cmd_ready),
        .object_cmd(object_cmd),
        .object_rsp_valid(object_rsp_valid),
        .object_rsp_ready(object_rsp_ready),
        .object_rsp(object_rsp),
        .domain_cmd_valid(domain_cmd_valid),
        .domain_cmd_ready(domain_cmd_ready),
        .domain_cmd(domain_cmd),
        .domain_rsp_valid(domain_rsp_valid),
        .domain_rsp_ready(domain_rsp_ready),
        .domain_rsp(domain_rsp),
        .heap_cmd_valid(heap_cmd_valid),
        .heap_cmd_ready(heap_cmd_ready),
        .heap_cmd(heap_cmd),
        .heap_rsp_valid(heap_rsp_valid),
        .heap_rsp_ready(heap_rsp_ready),
        .heap_rsp(heap_rsp),
        .vma_cmd_valid(vma_cmd_valid),
        .vma_cmd_ready(vma_cmd_ready),
        .vma_cmd(vma_cmd),
        .vma_rsp_valid(vma_rsp_valid),
        .vma_rsp_ready(vma_rsp_ready),
        .vma_rsp(vma_rsp),
        .dma_cmd_valid(dma_cmd_valid),
        .dma_cmd_ready(dma_cmd_ready),
        .dma_cmd(dma_cmd),
        .dma_rsp_valid(dma_rsp_valid),
        .dma_rsp_ready(dma_rsp_ready),
        .dma_rsp(dma_rsp),
        .fault_valid(routed_fault_valid),
        .fault_ready(1'b1),
        .fault(routed_fault),
        .routed_counter()
    );

    lnp64_completion_router completion_router_i(
        .clk(clk),
        .reset_n(logic_reset_n),
        .rsp_valid(core_rsp_valid),
        .rsp(core_rsp),
        .completion(completion),
        .completion_valid(completion_valid)
    );

    lnp64_errno_writeback errno_writeback_i(
        .clk(clk),
        .reset_n(logic_reset_n),
        .rsp_valid(core_rsp_valid),
        .rsp(core_rsp),
        .errno_value(errno_value)
    );

    lnp64_scheduler #(
        .CORE_TILE_COUNT(CORE_TILE_COUNT)
    ) scheduler_i(
        .clk(clk),
        .reset_n(logic_reset_n),
        .boot_valid(boot_valid),
        .park_pid1(core_yielded_vec[0] && core_pid1_parked_vec[0]),
        .wake_pid1(wake_valid),
        .tile_idle(core_tile_idle),
        .tile_running(core_tile_running),
        .tile_parked(core_tile_parked),
        .tile_faulted(scheduler_tile_faulted),
        .issue_valid(sched_issue_valid),
        .issue_tid_flat(sched_issue_tid_flat),
        .exactly_one_location(pid1_exactly_one_location),
        .pid1_runnable(sched_pid1_runnable),
        .pid1_parked(sched_pid1_parked),
        .no_duplicate_issue(sched_no_duplicate_issue),
        .tile1_schedulable_idle(sched_tile1_schedulable_idle),
        .tile_fault_isolated(sched_tile_fault_isolated)
    );

    lnp64_event_router event_router_i(
        .clk(clk),
        .reset_n(logic_reset_n),
        .synthetic_event(sim_event_inject),
        .source_tile_id(32'd1),
        .target_tile_id(32'd0),
        .pid1_parked(core_pid1_parked_vec[0] || sched_pid1_parked),
        .wake_valid(wake_valid),
        .event_valid(event_valid),
        .event_ready(1'b1),
        .event_record(event_record),
        .event_counter(event_counter),
        .cross_tile_wake_valid(cross_tile_wake_valid),
        .wake_counter(event_wake_counter)
    );

    lnp64_fault_telemetry fault_telemetry_i(
        .clk(clk),
        .reset_n(logic_reset_n),
        .tile_id(32'd1),
        .inject_fault(sim_fault_inject),
        .fault_valid(fault_valid),
        .fault_ready(1'b1),
        .fault(fault_record),
        .fault_counter()
    );

    lnp64_watchdog watchdog_i(
        .clk(clk),
        .reset_n(logic_reset_n),
        .tile_id(32'd0),
        .inject_stuck(sim_watchdog_inject),
        .degraded(watchdog_degraded),
        .fault_valid(watchdog_fault_valid),
        .fault_ready(1'b1),
        .fault(watchdog_fault)
    );

    lnp64_measurement_attestation measurement_i(
        .clk(clk),
        .reset_n(logic_reset_n),
        .boot_valid(boot_valid),
        .quote(quote)
    );

    lnp64_policy_engine policy_i(
        .clk(clk),
        .reset_n(logic_reset_n),
        .request_valid(boot_valid),
        .decision_allow(policy_allow)
    );

    lnp64_typed_control_validator typed_control_i(.clk(clk), .reset_n(logic_reset_n), .idle(typed_control_idle), .telemetry_counter(), .fault_counter());
    lnp64_namespace_dispatch namespace_i(.clk(clk), .reset_n(logic_reset_n), .idle(namespace_idle), .telemetry_counter(), .fault_counter());
    lnp64_stream_frontend stream_frontend_i(.clk(clk), .reset_n(logic_reset_n), .idle(stream_frontend_idle), .telemetry_counter(), .fault_counter());
    lnp64_ddr_controller ddr_i(.clk(clk), .reset_n(logic_reset_n), .absent_or_idle(ddr_absent_or_idle), .telemetry_counter(), .fault_counter());
    lnp64_sd_spi_flash sd_spi_i(.clk(clk), .reset_n(logic_reset_n), .absent_or_idle(sd_spi_absent_or_idle), .telemetry_counter(), .fault_counter());
    lnp64_boot_image_storage boot_image_i(.clk(clk), .reset_n(logic_reset_n), .idle(boot_image_idle), .telemetry_counter(), .fault_counter());

    lnp64_cap_engine cap_i(.clk(clk), .reset_n(logic_reset_n), .cmd_valid(cap_cmd_valid), .cmd_ready(cap_cmd_ready), .cmd(cap_cmd), .object_cap_sync_valid(object_cap_sync_valid), .object_cap_sync_reader_fd(object_cap_sync_reader_fd), .object_cap_sync_writer_fd(object_cap_sync_writer_fd), .object_cap_sync_single_valid(object_cap_sync_single_valid), .object_cap_sync_single_fd(object_cap_sync_single_fd), .object_cap_sync_single_kind(object_cap_sync_single_kind), .object_cap_sync_single_lineage(object_cap_sync_single_lineage), .rsp_valid(cap_rsp_valid), .rsp_ready(cap_rsp_ready), .rsp(cap_rsp), .m1_commit_valid(cap_m1_commit_valid), .m1_commit(cap_m1_commit), .m1_pre_state_projection(cap_m1_pre_state_projection), .m1_state_projection(cap_m1_state_projection), .telemetry_counter(), .fault_counter());
    lnp64_domain_engine domain_i(.clk(clk), .reset_n(logic_reset_n), .cmd_valid(domain_cmd_valid), .cmd_ready(domain_cmd_ready), .cmd(domain_cmd), .rsp_valid(domain_rsp_valid), .rsp_ready(domain_rsp_ready), .rsp(domain_rsp), .telemetry_counter(), .fault_counter());
    lnp64_object_engine object_i(.clk(clk), .reset_n(logic_reset_n), .cmd_valid(object_cmd_valid), .cmd_ready(object_cmd_ready), .cmd(object_cmd), .rsp_valid(object_rsp_valid), .rsp_ready(object_rsp_ready), .rsp(object_rsp), .cap_sync_valid(object_cap_sync_valid), .cap_sync_reader_fd(object_cap_sync_reader_fd), .cap_sync_writer_fd(object_cap_sync_writer_fd), .cap_sync_single_valid(object_cap_sync_single_valid), .cap_sync_single_fd(object_cap_sync_single_fd), .cap_sync_single_kind(object_cap_sync_single_kind), .cap_sync_single_lineage(object_cap_sync_single_lineage), .telemetry_counter(), .fault_counter());
    lnp64_gate_engine gate_i(.clk(clk), .reset_n(logic_reset_n), .cmd_valid(1'b0), .cmd_ready(), .cmd(zero_cmd), .rsp_valid(), .rsp_ready(1'b1), .rsp(), .telemetry_counter(), .fault_counter());
    lnp64_process_engine process_i(.clk(clk), .reset_n(logic_reset_n), .cmd_valid(1'b0), .cmd_ready(), .cmd(zero_cmd), .rsp_valid(), .rsp_ready(1'b1), .rsp(), .telemetry_counter(), .fault_counter());
    lnp64_vma_engine vma_i(.clk(clk), .reset_n(logic_reset_n), .cmd_valid(vma_cmd_valid), .cmd_ready(vma_cmd_ready), .cmd(vma_cmd), .rsp_valid(vma_rsp_valid), .rsp_ready(vma_rsp_ready), .rsp(vma_rsp), .telemetry_counter(), .fault_counter());
    lnp64_page_allocator page_allocator_i(.clk(clk), .reset_n(logic_reset_n), .idle(page_allocator_idle), .telemetry_counter(), .fault_counter());
    lnp64_memory_fabric memory_fabric_i(.clk(clk), .reset_n(logic_reset_n), .coherence_event_path_live(memory_visibility_live), .raw_physical_address_visible(memory_raw_pa_visible), .telemetry_counter(), .fault_counter());
    lnp64_metadata_broker metadata_i(.clk(clk), .reset_n(logic_reset_n), .idle(metadata_idle), .telemetry_counter(), .fault_counter());
    lnp64_dma_fabric dma_i(.clk(clk), .reset_n(logic_reset_n), .cmd_valid(dma_cmd_valid), .cmd_ready(dma_cmd_ready), .cmd(dma_cmd), .rsp_valid(dma_rsp_valid), .rsp_ready(dma_rsp_ready), .rsp(dma_rsp), .visibility_event_path_live(dma_visibility_live), .raw_dma_authority_visible(dma_raw_visible), .telemetry_counter(), .fault_counter());
    lnp64_service_boundary service_i(.clk(clk), .reset_n(logic_reset_n), .cmd_valid(1'b0), .cmd_ready(), .cmd(zero_cmd), .rsp_valid(), .rsp_ready(1'b1), .rsp(), .telemetry_counter(), .fault_counter());
    lnp64_futex_atomic futex_i(.clk(clk), .reset_n(logic_reset_n), .idle(futex_idle), .telemetry_counter(), .fault_counter());
    lnp64_heap_engine heap_i(.clk(clk), .reset_n(logic_reset_n), .cmd_valid(heap_cmd_valid), .cmd_ready(heap_cmd_ready), .cmd(heap_cmd), .rsp_valid(heap_rsp_valid), .rsp_ready(heap_rsp_ready), .rsp(heap_rsp), .telemetry_counter(), .fault_counter());
    lnp64_classifier_servicelet classifier_i(.clk(clk), .reset_n(logic_reset_n), .cmd_valid(1'b0), .cmd_ready(), .cmd(zero_cmd), .rsp_valid(), .rsp_ready(1'b1), .rsp(), .telemetry_counter(), .fault_counter());
    lnp64_entropy_env entropy_env_i(.clk(clk), .reset_n(logic_reset_n), .feature_bits(env_feature_bits), .limit_threads(env_limit_threads));
    lnp64_uart uart_i(.clk(clk), .reset_n(logic_reset_n), .boot_valid(boot_valid), .uart_valid(uart_valid), .uart_byte(uart_byte));
    lnp64_storage_stub storage_i(.clk(clk), .reset_n(logic_reset_n), .raw_device_authority_visible(storage_raw_visible), .telemetry_counter(), .fault_counter());
    lnp64_eth_stub eth_i(.clk(clk), .reset_n(logic_reset_n), .raw_interrupt_visible(eth_irq_visible), .telemetry_counter(), .fault_counter());
    lnp64_pcie_stub pcie_i(.clk(clk), .reset_n(logic_reset_n), .raw_dma_authority_visible(pcie_dma_visible), .raw_interrupt_visible(pcie_irq_visible), .telemetry_counter(), .fault_counter());

    integer status_i;
    always_comb begin
        scheduler_tile_faulted = core_tile_faulted;
        if (CORE_TILE_COUNT > 1 && structured_fault_seen) begin
            scheduler_tile_faulted[1] = 1'b1;
        end
    end

    always_ff @(posedge clk or negedge logic_reset_n) begin
        if (!logic_reset_n) begin
            boot_stable <= 1'b0;
            pid1_completed <= 1'b0;
            env_get_ok <= 1'b0;
            sram_ldst_ok <= 1'b0;
            uart_seen <= 1'b0;
            uart_byte_seen <= 8'd0;
            event_woke_thread <= 1'b0;
            structured_fault_seen <= 1'b0;
            watchdog_degraded_seen <= 1'b0;
            no_raw_authority_visible <= 1'b0;
            coherence_paths_live <= 1'b0;
            icache_invalidate_seen <= '0;
            dcache_writeback_seen <= '0;
            tlb_invalidate_seen <= '0;
            tile1_observable_idle <= 1'b0;
            cross_tile_wake_observed <= 1'b0;
            cross_tile_duplicate_wake <= 1'b0;
        end else begin
            if (boot_valid && root_domain.domain_id == 32'd1 && root_domain.domain_gen == 32'd1 &&
                root_fdr.object_id == 32'd1 && boot_pid1 == 32'd1 && boot_tid1 == 32'd1) begin
                boot_stable <= 1'b1;
            end
            if (core_done_vec[0]) begin
                pid1_completed <= 1'b1;
            end
            if ((env_features_seen & REQUIRED_S0_FEATURE_MASK) == REQUIRED_S0_FEATURE_MASK &&
                env_tile_count_seen_vec[0] == CORE_TILE_COUNT[31:0] &&
                env_enabled_tile_mask_seen_vec[0] == ENABLED_TILE_MASK &&
                env_coherence_domain_seen_vec[0] == COHERENCE_DOMAIN_ID &&
                env_active_window_base_seen_vec[0] == ACTIVE_WINDOW_BASE &&
                env_active_window_count_seen_vec[0] == ACTIVE_WINDOW_COUNT) begin
                env_get_ok <= 1'b1;
            end
            if (ld_value_seen == 64'd12) begin
                sram_ldst_ok <= 1'b1;
            end
            if (uart_valid) begin
                uart_seen <= 1'b1;
                uart_byte_seen <= uart_byte;
            end
            if (event_valid && event_record.status == LNP64_STATUS_EVENT && wake_valid && event_record.tile_id == 32'd0) begin
                event_woke_thread <= 1'b1;
            end
            if ((fault_valid && fault_record.fault_code == LNP64_ERR_EFAULT && fault_record.tile_id == 32'd1) ||
                (routed_fault_valid && routed_fault.fault_code == LNP64_ERR_EFAULT)) begin
                structured_fault_seen <= 1'b1;
            end
            if (watchdog_degraded || watchdog_fault_valid) begin
                watchdog_degraded_seen <= 1'b1;
            end
            icache_invalidate_seen <= icache_invalidate_seen | icache_invalidate_vec;
            dcache_writeback_seen <= dcache_writeback_seen | dcache_writeback_vec;
            tlb_invalidate_seen <= tlb_invalidate_seen | tlb_invalidate_vec;
            coherence_paths_live <=
                memory_visibility_live &&
                dma_visibility_live &&
                (|icache_invalidate_seen) &&
                (|dcache_writeback_seen) &&
                (|tlb_invalidate_seen);
            no_raw_authority_visible <=
                !core_raw_authority_visible &&
                !memory_raw_pa_visible &&
                !dma_raw_visible &&
                !storage_raw_visible &&
                !eth_irq_visible &&
                !pcie_dma_visible &&
                !pcie_irq_visible;
            if (CORE_TILE_COUNT > 1 && tile_reset_stable[1] && sched_tile1_schedulable_idle) begin
                tile1_observable_idle <= 1'b1;
            end
            if (cross_tile_wake_valid && !cross_tile_wake_observed) begin
                cross_tile_wake_observed <= 1'b1;
            end else if (cross_tile_wake_valid) begin
                cross_tile_duplicate_wake <= 1'b1;
            end
        end
    end
endmodule

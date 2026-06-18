`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_top (
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
    output logic coherence_paths_live
);
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

    logic core_cmd_valid;
    logic core_cmd_ready;
    lnp64_cmd_t core_cmd;
    logic core_rsp_valid;
    logic core_rsp_ready;
    lnp64_rsp_t core_rsp;
    logic core_yielded;
    logic core_pid1_runnable;
    logic core_pid1_parked;
    logic core_done;
    logic [63:0] env_features_seen;
    logic [63:0] ld_value_seen;
    logic core_raw_authority_visible;

    logic sched_pid1_runnable;
    logic sched_pid1_parked;
    logic event_valid;
    logic wake_valid;
    lnp64_event_t event_record;
    logic [31:0] event_counter;
    logic fault_valid;
    lnp64_fault_t fault_record;
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

    lnp64_core_tile core_i(
        .clk(clk),
        .reset_n(logic_reset_n),
        .release_core(release_core),
        .cmd_valid(core_cmd_valid),
        .cmd_ready(core_cmd_ready),
        .cmd(core_cmd),
        .rsp_valid(core_rsp_valid),
        .rsp_ready(core_rsp_ready),
        .rsp(core_rsp),
        .yielded(core_yielded),
        .wake_valid(wake_valid),
        .done(core_done),
        .pid1_runnable(core_pid1_runnable),
        .pid1_parked(core_pid1_parked),
        .retired_count(retired_count),
        .env_features_seen(env_features_seen),
        .ld_value_seen(ld_value_seen),
        .object_stub_failed_closed(stub_failed_closed),
        .unsupported_failed_closed(unsupported_failed_closed),
        .raw_authority_visible(core_raw_authority_visible)
    );

    lnp64_issue_retire issue_retire_i(
        .clk(clk),
        .reset_n(logic_reset_n),
        .retire_valid(core_done),
        .retire_counter()
    );

    lnp64_engine_router engine_router_i(
        .clk(clk),
        .reset_n(logic_reset_n),
        .cmd_valid(core_cmd_valid),
        .cmd_ready(core_cmd_ready),
        .cmd(core_cmd),
        .rsp_valid(core_rsp_valid),
        .rsp_ready(core_rsp_ready),
        .rsp(core_rsp),
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

    lnp64_scheduler scheduler_i(
        .clk(clk),
        .reset_n(logic_reset_n),
        .boot_valid(boot_valid),
        .park_pid1(core_yielded && core_pid1_parked),
        .wake_pid1(wake_valid),
        .exactly_one_location(pid1_exactly_one_location),
        .pid1_runnable(sched_pid1_runnable),
        .pid1_parked(sched_pid1_parked)
    );

    lnp64_event_router event_router_i(
        .clk(clk),
        .reset_n(logic_reset_n),
        .synthetic_event(sim_event_inject),
        .pid1_parked(core_pid1_parked || sched_pid1_parked),
        .wake_valid(wake_valid),
        .event_valid(event_valid),
        .event_ready(1'b1),
        .event_record(event_record),
        .event_counter(event_counter)
    );

    lnp64_fault_telemetry fault_telemetry_i(
        .clk(clk),
        .reset_n(logic_reset_n),
        .inject_fault(sim_fault_inject),
        .fault_valid(fault_valid),
        .fault_ready(1'b1),
        .fault(fault_record),
        .fault_counter()
    );

    lnp64_watchdog watchdog_i(
        .clk(clk),
        .reset_n(logic_reset_n),
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

    lnp64_cap_engine cap_i(.clk(clk), .reset_n(logic_reset_n), .cmd_valid(1'b0), .cmd_ready(), .cmd(zero_cmd), .rsp_valid(), .rsp_ready(1'b1), .rsp(), .telemetry_counter(), .fault_counter());
    lnp64_domain_engine domain_i(.clk(clk), .reset_n(logic_reset_n), .cmd_valid(1'b0), .cmd_ready(), .cmd(zero_cmd), .rsp_valid(), .rsp_ready(1'b1), .rsp(), .telemetry_counter(), .fault_counter());
    lnp64_object_engine object_i(.clk(clk), .reset_n(logic_reset_n), .cmd_valid(1'b0), .cmd_ready(), .cmd(zero_cmd), .rsp_valid(), .rsp_ready(1'b1), .rsp(), .telemetry_counter(), .fault_counter());
    lnp64_gate_engine gate_i(.clk(clk), .reset_n(logic_reset_n), .cmd_valid(1'b0), .cmd_ready(), .cmd(zero_cmd), .rsp_valid(), .rsp_ready(1'b1), .rsp(), .telemetry_counter(), .fault_counter());
    lnp64_process_engine process_i(.clk(clk), .reset_n(logic_reset_n), .cmd_valid(1'b0), .cmd_ready(), .cmd(zero_cmd), .rsp_valid(), .rsp_ready(1'b1), .rsp(), .telemetry_counter(), .fault_counter());
    lnp64_vma_engine vma_i(.clk(clk), .reset_n(logic_reset_n), .cmd_valid(1'b0), .cmd_ready(), .cmd(zero_cmd), .rsp_valid(), .rsp_ready(1'b1), .rsp(), .telemetry_counter(), .fault_counter());
    lnp64_page_allocator page_allocator_i(.clk(clk), .reset_n(logic_reset_n), .idle(page_allocator_idle), .telemetry_counter(), .fault_counter());
    lnp64_memory_fabric memory_fabric_i(.clk(clk), .reset_n(logic_reset_n), .coherence_event_path_live(memory_visibility_live), .raw_physical_address_visible(memory_raw_pa_visible), .telemetry_counter(), .fault_counter());
    lnp64_metadata_broker metadata_i(.clk(clk), .reset_n(logic_reset_n), .idle(metadata_idle), .telemetry_counter(), .fault_counter());
    lnp64_dma_fabric dma_i(.clk(clk), .reset_n(logic_reset_n), .visibility_event_path_live(dma_visibility_live), .raw_dma_authority_visible(dma_raw_visible), .telemetry_counter(), .fault_counter());
    lnp64_service_boundary service_i(.clk(clk), .reset_n(logic_reset_n), .cmd_valid(1'b0), .cmd_ready(), .cmd(zero_cmd), .rsp_valid(), .rsp_ready(1'b1), .rsp(), .telemetry_counter(), .fault_counter());
    lnp64_futex_atomic futex_i(.clk(clk), .reset_n(logic_reset_n), .idle(futex_idle), .telemetry_counter(), .fault_counter());
    lnp64_heap_engine heap_i(.clk(clk), .reset_n(logic_reset_n), .cmd_valid(1'b0), .cmd_ready(), .cmd(zero_cmd), .rsp_valid(), .rsp_ready(1'b1), .rsp(), .telemetry_counter(), .fault_counter());
    lnp64_classifier_servicelet classifier_i(.clk(clk), .reset_n(logic_reset_n), .cmd_valid(1'b0), .cmd_ready(), .cmd(zero_cmd), .rsp_valid(), .rsp_ready(1'b1), .rsp(), .telemetry_counter(), .fault_counter());
    lnp64_entropy_env entropy_env_i(.clk(clk), .reset_n(logic_reset_n), .feature_bits(env_feature_bits), .limit_threads(env_limit_threads));
    lnp64_uart uart_i(.clk(clk), .reset_n(logic_reset_n), .boot_valid(boot_valid), .uart_valid(uart_valid), .uart_byte(uart_byte));
    lnp64_storage_stub storage_i(.clk(clk), .reset_n(logic_reset_n), .raw_device_authority_visible(storage_raw_visible), .telemetry_counter(), .fault_counter());
    lnp64_eth_stub eth_i(.clk(clk), .reset_n(logic_reset_n), .raw_interrupt_visible(eth_irq_visible), .telemetry_counter(), .fault_counter());
    lnp64_pcie_stub pcie_i(.clk(clk), .reset_n(logic_reset_n), .raw_dma_authority_visible(pcie_dma_visible), .raw_interrupt_visible(pcie_irq_visible), .telemetry_counter(), .fault_counter());

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
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
        end else begin
            if (boot_valid && root_domain.domain_id == 32'd1 && root_domain.domain_gen == 32'd1 &&
                root_fdr.object_id == 32'd1 && boot_pid1 == 32'd1 && boot_tid1 == 32'd1) begin
                boot_stable <= 1'b1;
            end
            if (core_done) begin
                pid1_completed <= 1'b1;
            end
            if ((env_features_seen & REQUIRED_S0_FEATURE_MASK) == REQUIRED_S0_FEATURE_MASK) begin
                env_get_ok <= 1'b1;
            end
            if (ld_value_seen == 64'd12) begin
                sram_ldst_ok <= 1'b1;
            end
            if (uart_valid) begin
                uart_seen <= 1'b1;
                uart_byte_seen <= uart_byte;
            end
            if (event_valid && event_record.status == LNP64_STATUS_EVENT && wake_valid) begin
                event_woke_thread <= 1'b1;
            end
            if (fault_valid && fault_record.fault_code == LNP64_ERR_EFAULT) begin
                structured_fault_seen <= 1'b1;
            end
            if (watchdog_degraded || watchdog_fault_valid) begin
                watchdog_degraded_seen <= 1'b1;
            end
            coherence_paths_live <= memory_visibility_live && dma_visibility_live;
            no_raw_authority_visible <=
                !core_raw_authority_visible &&
                !memory_raw_pa_visible &&
                !dma_raw_visible &&
                !storage_raw_visible &&
                !eth_irq_visible &&
                !pcie_dma_visible &&
                !pcie_irq_visible;
        end
    end
endmodule

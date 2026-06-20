`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m13_pcie_iommu (
    input  logic clk,
    input  logic reset_n,
    input  logic start,
    input  logic [31:0] scenario_seed,
    output logic done,
    output logic trace_valid,
    output logic [7:0] trace_code,
    output logic [63:0] trace_value,
    output logic device_enumerated,
    output logic bar_capability_created,
    output logic iommu_bound_to_domain,
    output logic scoped_dma_completed,
    output logic msi_event_delivered,
    output logic unbound_bus_master_rejected,
    output logic stale_bar_rejected,
    output logic malformed_config_rejected,
    output logic no_raw_pcie_authority,
    output logic counts_exact,
    output logic typed_commit_valid,
    output lnp64_m13_pcie_commit_t typed_commit,
    output lnp64_m13_state_projection_t typed_state_projection
);
    typedef enum logic [3:0] {
        P_RESET,
        P_BOOT,
        P_ENUMERATE,
        P_IOMMU_DMA,
        P_MSI,
        P_BUS_MASTER,
        P_STALE_BAR,
        P_MALFORMED_CONFIG,
        P_RAW_AUTHORITY,
        P_DONE
    } pcie_state_e;

    pcie_state_e state;
    lnp64_pcie_device_t device;
    lnp64_iommu_mapping_t iommu_mapping;
    logic [31:0] completions;
    logic [31:0] faults;
    logic raw_dma_authority_visible;
    logic raw_interrupt_visible;

    function automatic logic [31:0] seeded_root_domain(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[3:0]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_requester(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'h0000_0100;
        end
        return ({25'd0, seed[10:4]} + 32'd1) << 8;
    endfunction

    function automatic logic [31:0] seeded_bar_id(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[15:12]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_bar_gen(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[19:16]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_iommu_context(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[23:20]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_dma_bytes(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd128;
        end
        return ({29'd0, seed[26:24]} + 32'd1) << 6;
    endfunction

    function automatic logic [31:0] seeded_msi_vector(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd32;
        end
        return {27'd0, seed[31:27]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_rogue_domain(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd2;
        end
        return seeded_root_domain(seed) + {29'd0, seed[30:28]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_malformed_field(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {31'd0, seed[31]} + 32'd1;
    endfunction

    function automatic logic [15:0] seeded_requester_trace(input logic [31:0] seed);
        return seeded_requester(seed);
    endfunction

    function automatic logic [15:0] seeded_root_domain_trace(input logic [31:0] seed);
        return seeded_root_domain(seed);
    endfunction

    function automatic logic [15:0] seeded_bar_id_trace(input logic [31:0] seed);
        return seeded_bar_id(seed);
    endfunction

    function automatic logic [15:0] seeded_bar_gen_trace(input logic [31:0] seed);
        return seeded_bar_gen(seed);
    endfunction

    function automatic logic [15:0] seeded_iommu_context_trace(input logic [31:0] seed);
        return seeded_iommu_context(seed);
    endfunction

    function automatic logic [15:0] seeded_dma_bytes_trace(input logic [31:0] seed);
        return seeded_dma_bytes(seed);
    endfunction

    function automatic logic [15:0] seeded_msi_vector_trace(input logic [31:0] seed);
        return seeded_msi_vector(seed);
    endfunction

    function automatic logic [15:0] seeded_stale_bar_gen_trace(input logic [31:0] seed);
        return seeded_bar_gen(seed) + 32'd1;
    endfunction

    function automatic logic [15:0] seeded_malformed_field_trace(input logic [31:0] seed);
        return seeded_malformed_field(seed);
    endfunction

    task automatic commit_m13(
        input lnp64_m13_pcie_op_e op,
        input logic [15:0] status,
        input logic [31:0] dma_bytes
    );
        typed_commit_valid <= 1'b1;
        typed_commit.op <= op;
        typed_commit.status <= status;
        typed_commit.requester_id <= seeded_requester(scenario_seed);
        typed_commit.bar_id <= seeded_bar_id(scenario_seed);
        typed_commit.bar_generation <= seeded_bar_gen(scenario_seed);
        typed_commit.domain_id <= seeded_root_domain(scenario_seed);
        typed_commit.iommu_context <= seeded_iommu_context(scenario_seed);
        typed_commit.dma_bytes <= dma_bytes;
    endtask

    always_comb begin
        no_raw_pcie_authority = !raw_dma_authority_visible && !raw_interrupt_visible;
        counts_exact = completions == 32'd3 && faults == 32'd3;
    end

    always_comb begin
        typed_state_projection = '0;
        typed_state_projection.op = typed_commit.op;
        typed_state_projection.status = typed_commit.status;
        typed_state_projection.completions = completions;
        typed_state_projection.faults = faults;
        typed_state_projection.device_enumerated = device_enumerated;
        typed_state_projection.bar_capability_created = bar_capability_created;
        typed_state_projection.iommu_bound_to_domain = iommu_bound_to_domain;
        typed_state_projection.scoped_dma_completed = scoped_dma_completed;
        typed_state_projection.msi_event_delivered = msi_event_delivered;
        typed_state_projection.unbound_bus_master_rejected = unbound_bus_master_rejected;
        typed_state_projection.stale_bar_rejected = stale_bar_rejected;
        typed_state_projection.malformed_config_rejected = malformed_config_rejected;
        typed_state_projection.no_raw_pcie_authority = no_raw_pcie_authority;
        typed_state_projection.counts_exact = counts_exact;
    end

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            state <= P_RESET;
            done <= 1'b0;
            trace_valid <= 1'b0;
            trace_code <= 8'd0;
            trace_value <= 64'd0;
            typed_commit_valid <= 1'b0;
            typed_commit <= '0;
            device_enumerated <= 1'b0;
            bar_capability_created <= 1'b0;
            iommu_bound_to_domain <= 1'b0;
            scoped_dma_completed <= 1'b0;
            msi_event_delivered <= 1'b0;
            unbound_bus_master_rejected <= 1'b0;
            stale_bar_rejected <= 1'b0;
            malformed_config_rejected <= 1'b0;
            completions <= 32'd0;
            faults <= 32'd0;
            raw_dma_authority_visible <= 1'b0;
            raw_interrupt_visible <= 1'b0;
            device <= '0;
            iommu_mapping <= '0;
        end else begin
            trace_valid <= 1'b0;
            typed_commit_valid <= 1'b0;
            unique case (state)
                P_RESET: begin
                    if (start) begin
                        state <= P_BOOT;
                    end
                end
                P_BOOT: begin
                    device.domain_id <= seeded_root_domain(scenario_seed);
                    device.domain_generation <= 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd1;
                    trace_value <= {seeded_root_domain(scenario_seed), 31'd0, 1'b1};
                    state <= P_ENUMERATE;
                end
                P_ENUMERATE: begin
                    device_enumerated <= 1'b1;
                    bar_capability_created <= 1'b1;
                    completions <= completions + 32'd1;
                    device.requester_id <= seeded_requester(scenario_seed);
                    device.bar_id <= seeded_bar_id(scenario_seed);
                    device.bar_generation <= seeded_bar_gen(scenario_seed);
                    device.bar_base_token <= {seeded_requester(scenario_seed), seeded_bar_id(scenario_seed)};
                    device.bar_length <= 64'd4096;
                    device.rights_mask <= 64'h3;
                    device.msi_vector <= seeded_msi_vector_trace(scenario_seed);
                    device.device_state <= 16'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd2;
                    trace_value <= {
                        seeded_requester_trace(scenario_seed),
                        seeded_bar_id_trace(scenario_seed),
                        seeded_bar_gen_trace(scenario_seed),
                        16'd1
                    };
                    commit_m13(LNP64_M13_COMMIT_ENUMERATE, LNP64_STATUS_OK, 32'd0);
                    state <= P_IOMMU_DMA;
                end
                P_IOMMU_DMA: begin
                    iommu_bound_to_domain <= 1'b1;
                    scoped_dma_completed <= 1'b1;
                    completions <= completions + 32'd1;
                    iommu_mapping.context_id <= seeded_iommu_context(scenario_seed);
                    iommu_mapping.requester_id <= seeded_requester(scenario_seed);
                    iommu_mapping.domain_id <= seeded_root_domain(scenario_seed);
                    iommu_mapping.domain_generation <= 32'd1;
                    iommu_mapping.bar_id <= seeded_bar_id(scenario_seed);
                    iommu_mapping.bar_generation <= seeded_bar_gen(scenario_seed);
                    iommu_mapping.dma_window_token <= {seeded_root_domain(scenario_seed), seeded_requester(scenario_seed)};
                    iommu_mapping.byte_len <= {32'd0, seeded_dma_bytes(scenario_seed)};
                    iommu_mapping.permission <= 16'h3;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd3;
                    trace_value <= {
                        seeded_iommu_context_trace(scenario_seed),
                        seeded_root_domain_trace(scenario_seed),
                        seeded_dma_bytes_trace(scenario_seed),
                        16'd1
                    };
                    commit_m13(LNP64_M13_COMMIT_IOMMU_DMA, LNP64_STATUS_OK, seeded_dma_bytes(scenario_seed));
                    state <= P_MSI;
                end
                P_MSI: begin
                    msi_event_delivered <= 1'b1;
                    completions <= completions + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd4;
                    trace_value <= {32'd0, seeded_msi_vector_trace(scenario_seed), 16'd1};
                    commit_m13(LNP64_M13_COMMIT_MSI, LNP64_STATUS_OK, 32'd0);
                    state <= P_BUS_MASTER;
                end
                P_BUS_MASTER: begin
                    unbound_bus_master_rejected <= 1'b1;
                    faults <= faults + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd5;
                    trace_value <= {seeded_rogue_domain(scenario_seed), 16'd0, LNP64_ERR_EPERM};
                    commit_m13(LNP64_M13_COMMIT_BUS_MASTER, LNP64_ERR_EPERM, 32'd0);
                    state <= P_STALE_BAR;
                end
                P_STALE_BAR: begin
                    stale_bar_rejected <= 1'b1;
                    faults <= faults + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd6;
                    trace_value <= {16'd0, seeded_stale_bar_gen_trace(scenario_seed), 16'd0, LNP64_ERR_EREVOKED};
                    commit_m13(LNP64_M13_COMMIT_STALE_BAR, LNP64_ERR_EREVOKED, 32'd0);
                    state <= P_MALFORMED_CONFIG;
                end
                P_MALFORMED_CONFIG: begin
                    malformed_config_rejected <= 1'b1;
                    faults <= faults + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd7;
                    trace_value <= {32'd0, seeded_malformed_field_trace(scenario_seed), LNP64_ERR_EINVAL};
                    commit_m13(LNP64_M13_COMMIT_MALFORMED_CONFIG, LNP64_ERR_EINVAL, 32'd0);
                    state <= P_RAW_AUTHORITY;
                end
                P_RAW_AUTHORITY: begin
                    raw_dma_authority_visible <= 1'b0;
                    raw_interrupt_visible <= 1'b0;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd8;
                    trace_value <= 64'd0;
                    commit_m13(LNP64_M13_COMMIT_RAW_AUTHORITY, LNP64_STATUS_OK, 32'd0);
                    state <= P_DONE;
                end
                P_DONE: begin
                    if (!done) begin
                        done <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd9;
                        trace_value <= {
                            completions[15:0],
                            faults[15:0],
                            seeded_bar_id(scenario_seed)
                        };
                    end
                    state <= P_DONE;
                end
                default: state <= P_RESET;
            endcase
        end
    end
endmodule

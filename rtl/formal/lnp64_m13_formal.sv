`timescale 1ns/1ps

// Formal property-verification harness for the M13 PCIe/IOMMU engine.
//
// Instantiates the real lnp64_m13_pcie_iommu RTL and asserts the SG-IO severe
// goals as SVA properties on its output ports. clk/reset_n/start/scenario_seed
// are free inputs, so SymbiYosys explores ALL input sequences (every seed, every
// reset/start timing) -- a proof about the hardware over all behaviours, not a
// single simulation trace.
import lnp64_pkg::*;

module lnp64_m13_formal (
    input logic clk,
    input logic reset_n,
    input logic start,
    input logic [31:0] scenario_seed
);
    logic done;
    logic trace_valid;
    logic [7:0] trace_code;
    logic [63:0] trace_value;
    logic device_enumerated;
    logic bar_capability_created;
    logic iommu_bound_to_domain;
    logic scoped_dma_completed;
    logic msi_event_delivered;
    logic unbound_bus_master_rejected;
    logic stale_bar_rejected;
    logic malformed_config_rejected;
    logic no_raw_pcie_authority;
    logic counts_exact;
    logic typed_commit_valid;
    lnp64_m13_pcie_commit_t typed_commit;
    lnp64_m13_state_projection_t typed_state_projection;

    lnp64_m13_pcie_iommu dut (
        .clk(clk),
        .reset_n(reset_n),
        .start(start),
        .scenario_seed(scenario_seed),
        .done(done),
        .trace_valid(trace_valid),
        .trace_code(trace_code),
        .trace_value(trace_value),
        .device_enumerated(device_enumerated),
        .bar_capability_created(bar_capability_created),
        .iommu_bound_to_domain(iommu_bound_to_domain),
        .scoped_dma_completed(scoped_dma_completed),
        .msi_event_delivered(msi_event_delivered),
        .unbound_bus_master_rejected(unbound_bus_master_rejected),
        .stale_bar_rejected(stale_bar_rejected),
        .malformed_config_rejected(malformed_config_rejected),
        .no_raw_pcie_authority(no_raw_pcie_authority),
        .counts_exact(counts_exact),
        .typed_commit_valid(typed_commit_valid),
        .typed_commit(typed_commit),
        .typed_state_projection(typed_state_projection)
    );

    // Reset discipline: constrain the model to a real power-on reset (reset_n
    // asserted on the first cycle), so the proof starts from the defined reset
    // state. After that reset_n, start and scenario_seed are all free inputs, so
    // the model checker still explores every seed and every start/reset timing.
    reg init_done = 1'b0;
    always @(posedge clk) init_done <= 1'b1;
    always @(posedge clk) begin
        if (!init_done) assume (!reset_n);
    end

    // Severe-goal properties as immediate assertions sampled each clock once the
    // engine is out of reset. Every property here is same-cycle, so an immediate
    // assert in a clocked block is equivalent to the concurrent SVA form and is
    // robustly supported by the yosys formal frontend.
    always @(posedge clk) begin
        if (reset_n) begin
            // SG-IO: raw PCIe DMA/interrupt authority is NEVER exposed.
            a_no_raw_pcie_authority:
                assert (no_raw_pcie_authority);

            if (typed_commit_valid) begin
                // The state projection agrees with the no-raw-authority output.
                a_projection_no_raw:
                    assert (typed_state_projection.no_raw_pcie_authority);

                // An unbound bus master is denied (EPERM) and recorded rejected.
                if (typed_commit.op == LNP64_M13_COMMIT_BUS_MASTER)
                    a_bus_master_denied:
                        assert (typed_commit.status == LNP64_ERR_EPERM
                                && unbound_bus_master_rejected);

                // A stale BAR submit is rejected as revoked (EREVOKED).
                if (typed_commit.op == LNP64_M13_COMMIT_STALE_BAR)
                    a_stale_bar_revoked:
                        assert (typed_commit.status == LNP64_ERR_EREVOKED
                                && stale_bar_rejected);

                // A malformed config submit is rejected as invalid (EINVAL).
                if (typed_commit.op == LNP64_M13_COMMIT_MALFORMED_CONFIG)
                    a_malformed_einval:
                        assert (typed_commit.status == LNP64_ERR_EINVAL
                                && malformed_config_rejected);

                // An IOMMU-scoped DMA only commits while bound to a domain.
                if (typed_commit.op == LNP64_M13_COMMIT_IOMMU_DMA)
                    a_dma_iommu_bound:
                        assert (iommu_bound_to_domain && scoped_dma_completed);
            end
        end
    end
endmodule

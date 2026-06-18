`timescale 1ns/1ps

module lnp64_m13_assertions (
    input logic clk,
    input logic reset_n,
    input logic done,
    input logic device_enumerated,
    input logic bar_capability_created,
    input logic iommu_bound_to_domain,
    input logic scoped_dma_completed,
    input logic msi_event_delivered,
    input logic unbound_bus_master_rejected,
    input logic stale_bar_rejected,
    input logic malformed_config_rejected,
    input logic no_raw_pcie_authority,
    input logic counts_exact
);
    always_ff @(posedge clk) begin
        if (reset_n && done) begin
            assert (device_enumerated);
            assert (bar_capability_created);
            assert (iommu_bound_to_domain);
            assert (scoped_dma_completed);
            assert (msi_event_delivered);
            assert (unbound_bus_master_rejected);
            assert (stale_bar_rejected);
            assert (malformed_config_rejected);
            assert (no_raw_pcie_authority);
            assert (counts_exact);
        end
    end
endmodule

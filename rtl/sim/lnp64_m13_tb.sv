`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m13_tb;
    logic clk;
    logic reset_n;
    logic start;
    logic [31:0] scenario_seed;
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

    lnp64_m13_pcie_iommu dut(
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
        .counts_exact(counts_exact)
    );

    lnp64_m13_assertions assertions_i(
        .clk(clk),
        .reset_n(reset_n),
        .done(done),
        .device_enumerated(device_enumerated),
        .bar_capability_created(bar_capability_created),
        .iommu_bound_to_domain(iommu_bound_to_domain),
        .scoped_dma_completed(scoped_dma_completed),
        .msi_event_delivered(msi_event_delivered),
        .unbound_bus_master_rejected(unbound_bus_master_rejected),
        .stale_bar_rejected(stale_bar_rejected),
        .malformed_config_rejected(malformed_config_rejected),
        .no_raw_pcie_authority(no_raw_pcie_authority),
        .counts_exact(counts_exact)
    );

    always #5 clk = ~clk;

    task automatic require(input logic condition, input string message);
        if (!condition) begin
            $fatal(1, "%s", message);
        end
    endtask

    always_ff @(posedge clk) begin
        if (trace_valid) begin
            unique case (trace_code)
                8'd1: $display(
                    "TRACE boot root_domain=%0d pcie_stub=%0d",
                    trace_value[63:32],
                    trace_value[0]
                );
                8'd2: $display(
                    "TRACE enumerate requester=%0d bar=%0d gen=%0d cap=%0d",
                    trace_value[63:48],
                    trace_value[47:32],
                    trace_value[31:16],
                    trace_value[15:0]
                );
                8'd3: $display(
                    "TRACE iommu_dma context=%0d domain=%0d bytes=%0d completion=%0d",
                    trace_value[63:48],
                    trace_value[47:32],
                    trace_value[31:16],
                    trace_value[15:0]
                );
                8'd4: $display(
                    "TRACE msi vector=%0d event=%0d",
                    trace_value[31:16],
                    trace_value[15:0]
                );
                8'd5: $display(
                    "TRACE bus_master domain=%0d errno=%0d",
                    trace_value[63:32],
                    trace_value[15:0]
                );
                8'd6: $display(
                    "TRACE stale_bar gen=%0d errno=%0d",
                    trace_value[47:32],
                    trace_value[15:0]
                );
                8'd7: $display(
                    "TRACE malformed_config field=%0d errno=%0d",
                    trace_value[31:16],
                    trace_value[15:0]
                );
                8'd8: $display(
                    "TRACE raw_pcie dma=%0d interrupt=%0d",
                    trace_value[1],
                    trace_value[0]
                );
                8'd9: $display(
                    "TRACE done completions=%0d faults=%0d bar=%0d",
                    trace_value[63:48],
                    trace_value[47:32],
                    trace_value[31:0]
                );
                default: $display("TRACE unknown code=%0d value=%0d", trace_code, trace_value);
            endcase
        end
    end

    initial begin
        if (!$value$plusargs("seed=%d", scenario_seed)) begin
            scenario_seed = 32'd0;
        end
        clk = 1'b0;
        reset_n = 1'b0;
        start = 1'b0;

        repeat (4) @(posedge clk);
        reset_n = 1'b1;
        @(posedge clk);
        start = 1'b1;
        @(posedge clk);
        start = 1'b0;

        repeat (40) @(posedge clk);
        require(done, "M13 PCIe/IOMMU slice did not complete");
        require(device_enumerated, "M13 PCIe device was not enumerated");
        require(bar_capability_created, "M13 BAR capability was not created");
        require(iommu_bound_to_domain, "M13 IOMMU mapping was not domain-bound");
        require(scoped_dma_completed, "M13 scoped DMA did not complete");
        require(msi_event_delivered, "M13 MSI event was not delivered");
        require(unbound_bus_master_rejected, "M13 unbound bus master was not rejected");
        require(stale_bar_rejected, "M13 stale BAR was not rejected");
        require(malformed_config_rejected, "M13 malformed config was not rejected");
        require(no_raw_pcie_authority, "M13 exposed raw PCIe authority");
        require(counts_exact, "M13 counts were not exact");
        $display("LNP64-RTL-M13 PASS");
        $finish;
    end
endmodule

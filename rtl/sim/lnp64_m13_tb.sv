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
    logic typed_commit_valid;
    lnp64_m13_pcie_commit_t typed_commit;
    lnp64_m13_state_projection_t typed_state_projection;

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
        .counts_exact(counts_exact),
        .typed_commit_valid(typed_commit_valid),
        .typed_commit(typed_commit),
        .typed_state_projection(typed_state_projection)
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
        if (typed_commit_valid) begin
            $display(
                "TTRACE_M13 {\"record\":\"m13_pcie_commit\",\"op\":%0d,\"status\":%0d,\"requester_id\":%0d,\"bar_id\":%0d,\"bar_generation\":%0d,\"domain_id\":%0d,\"iommu_context\":%0d,\"dma_bytes\":%0d}",
                typed_commit.op,
                typed_commit.status,
                typed_commit.requester_id,
                typed_commit.bar_id,
                typed_commit.bar_generation,
                typed_commit.domain_id,
                typed_commit.iommu_context,
                typed_commit.dma_bytes
            );
            $display(
                "TTRACE_M13_BITS {\"record\":\"m13_pcie_commit_bits\",\"width\":%0d,\"bits\":\"%0h\"}",
                $bits(lnp64_m13_pcie_commit_t),
                typed_commit
            );
            $display(
                "TTRACE_M13_STATE {\"record\":\"m13_state_projection\",\"op\":%0d,\"status\":%0d,\"completions\":%0d,\"faults\":%0d,\"device_enumerated\":%0d,\"bar_capability_created\":%0d,\"iommu_bound_to_domain\":%0d,\"scoped_dma_completed\":%0d,\"msi_event_delivered\":%0d,\"unbound_bus_master_rejected\":%0d,\"stale_bar_rejected\":%0d,\"malformed_config_rejected\":%0d,\"no_raw_pcie_authority\":%0d,\"counts_exact\":%0d}",
                typed_state_projection.op,
                typed_state_projection.status,
                typed_state_projection.completions,
                typed_state_projection.faults,
                typed_state_projection.device_enumerated,
                typed_state_projection.bar_capability_created,
                typed_state_projection.iommu_bound_to_domain,
                typed_state_projection.scoped_dma_completed,
                typed_state_projection.msi_event_delivered,
                typed_state_projection.unbound_bus_master_rejected,
                typed_state_projection.stale_bar_rejected,
                typed_state_projection.malformed_config_rejected,
                typed_state_projection.no_raw_pcie_authority,
                typed_state_projection.counts_exact
            );
            $display(
                "TTRACE_M13_STATE_BITS {\"record\":\"m13_state_projection_bits\",\"width\":%0d,\"bits\":\"%0h\"}",
                $bits(lnp64_m13_state_projection_t),
                typed_state_projection
            );
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

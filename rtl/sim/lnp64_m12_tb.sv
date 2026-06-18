`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m12_tb;
    logic clk;
    logic reset_n;
    logic start;
    logic [31:0] scenario_seed;
    logic done;
    logic trace_valid;
    logic [7:0] trace_code;
    logic [63:0] trace_value;
    logic boot_image_visible;
    logic block_object_authorized;
    logic block_write_completed;
    logic storage_barrier_issued;
    logic storage_barrier_quiescent;
    logic stale_object_rejected;
    logic cross_domain_rejected;
    logic media_fault_terminal;
    logic no_raw_device_authority;
    logic counts_exact;

    lnp64_m12_storage_barrier dut(
        .clk(clk),
        .reset_n(reset_n),
        .start(start),
        .scenario_seed(scenario_seed),
        .done(done),
        .trace_valid(trace_valid),
        .trace_code(trace_code),
        .trace_value(trace_value),
        .boot_image_visible(boot_image_visible),
        .block_object_authorized(block_object_authorized),
        .block_write_completed(block_write_completed),
        .storage_barrier_issued(storage_barrier_issued),
        .storage_barrier_quiescent(storage_barrier_quiescent),
        .stale_object_rejected(stale_object_rejected),
        .cross_domain_rejected(cross_domain_rejected),
        .media_fault_terminal(media_fault_terminal),
        .no_raw_device_authority(no_raw_device_authority),
        .counts_exact(counts_exact)
    );

    lnp64_m12_assertions assertions_i(
        .clk(clk),
        .reset_n(reset_n),
        .done(done),
        .boot_image_visible(boot_image_visible),
        .block_object_authorized(block_object_authorized),
        .block_write_completed(block_write_completed),
        .storage_barrier_issued(storage_barrier_issued),
        .storage_barrier_quiescent(storage_barrier_quiescent),
        .stale_object_rejected(stale_object_rejected),
        .cross_domain_rejected(cross_domain_rejected),
        .media_fault_terminal(media_fault_terminal),
        .no_raw_device_authority(no_raw_device_authority),
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
                    "TRACE boot root_domain=%0d storage_stub=%0d",
                    trace_value[63:32],
                    trace_value[0]
                );
                8'd2: $display(
                    "TRACE boot_image block=%0d bytes=%0d visible=%0d",
                    trace_value[63:32],
                    trace_value[31:16],
                    trace_value[15:0]
                );
                8'd3: $display(
                    "TRACE block_write object=%0d gen=%0d block=%0d data=%0d",
                    trace_value[63:48],
                    trace_value[47:32],
                    trace_value[31:16],
                    trace_value[15:0]
                );
                8'd4: $display(
                    "TRACE barrier barrier=%0d object=%0d quiescent=%0d",
                    trace_value[63:32],
                    trace_value[31:16],
                    trace_value[15:0]
                );
                8'd5: $display(
                    "TRACE stale_object gen=%0d errno=%0d",
                    trace_value[47:32],
                    trace_value[15:0]
                );
                8'd6: $display(
                    "TRACE cross_domain domain=%0d errno=%0d",
                    trace_value[63:32],
                    trace_value[15:0]
                );
                8'd7: $display(
                    "TRACE media_fault status=%0d errno=%0d",
                    trace_value[31:16],
                    trace_value[15:0]
                );
                8'd8: $display(
                    "TRACE raw_authority visible=%0d",
                    trace_value[0]
                );
                8'd9: $display(
                    "TRACE done completions=%0d faults=%0d barrier=%0d",
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
        require(done, "M12 storage-barrier slice did not complete");
        require(boot_image_visible, "M12 boot image read was not visible");
        require(block_object_authorized, "M12 block object was not authorized");
        require(block_write_completed, "M12 block write did not complete");
        require(storage_barrier_issued, "M12 storage barrier was not issued");
        require(storage_barrier_quiescent, "M12 storage barrier was not quiescent");
        require(stale_object_rejected, "M12 stale object was not rejected");
        require(cross_domain_rejected, "M12 cross-domain access was not rejected");
        require(media_fault_terminal, "M12 media fault was not terminal");
        require(no_raw_device_authority, "M12 exposed raw device authority");
        require(counts_exact, "M12 counts were not exact");
        $display("LNP64-RTL-M12 PASS");
        $finish;
    end
endmodule

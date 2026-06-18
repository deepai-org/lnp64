`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m4_tb;
    logic clk;
    logic reset_n;
    logic start;
    logic [31:0] scenario_seed;
    logic done;
    logic trace_valid;
    logic [7:0] trace_code;
    logic [63:0] trace_value;
    logic mapping_created;
    logic load_permitted;
    logic store_rejected;
    logic nx_faulted;
    logic guard_faulted;
    logic stale_vma_rejected;
    logic tlb_invalidation_observed;
    logic wx_enforced;

    lnp64_m4_vma dut(
        .clk(clk),
        .reset_n(reset_n),
        .start(start),
        .scenario_seed(scenario_seed),
        .done(done),
        .trace_valid(trace_valid),
        .trace_code(trace_code),
        .trace_value(trace_value),
        .mapping_created(mapping_created),
        .load_permitted(load_permitted),
        .store_rejected(store_rejected),
        .nx_faulted(nx_faulted),
        .guard_faulted(guard_faulted),
        .stale_vma_rejected(stale_vma_rejected),
        .tlb_invalidation_observed(tlb_invalidation_observed),
        .wx_enforced(wx_enforced)
    );

    lnp64_m4_assertions assertions_i(
        .clk(clk),
        .reset_n(reset_n),
        .done(done),
        .mapping_created(mapping_created),
        .load_permitted(load_permitted),
        .store_rejected(store_rejected),
        .nx_faulted(nx_faulted),
        .guard_faulted(guard_faulted),
        .stale_vma_rejected(stale_vma_rejected),
        .tlb_invalidation_observed(tlb_invalidation_observed),
        .wx_enforced(wx_enforced)
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
                8'd1: $display("TRACE boot root_domain=%0d vma_table=empty", trace_value[31:0]);
                8'd2: $display(
                    "TRACE mmap vma=%0d pages=%0d perms=rx guard=1",
                    trace_value[63:48],
                    trace_value[47:16]
                );
                8'd3: $display("TRACE load addr=0x%016h result=ok", trace_value);
                8'd4: $display("TRACE store_denied errno=%0d invariant=wx", trace_value[15:0]);
                8'd5: $display("TRACE exec_fault errno=%0d reason=nx", trace_value[15:0]);
                8'd6: $display("TRACE guard_fault errno=%0d page=guard", trace_value[15:0]);
                8'd7: $display("TRACE stale_vma errno=%0d", trace_value[15:0]);
                8'd8: $display(
                    "TRACE tlb_invalidate vma=%0d tlb_valid=%0d",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd9: $display(
                    "TRACE done mappings=%0d vma_gen=%0d",
                    trace_value[63:32],
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

        repeat (32) @(posedge clk);
        require(done, "M4 VMA slice did not complete");
        require(mapping_created, "M4 mapping was not created");
        require(load_permitted, "M4 permitted load did not complete");
        require(store_rejected, "M4 write to non-writable mapping was not rejected");
        require(nx_faulted, "M4 NX execute fault did not occur");
        require(guard_faulted, "M4 guard fault did not occur");
        require(stale_vma_rejected, "M4 stale VMA generation was not rejected");
        require(tlb_invalidation_observed, "M4 TLB invalidation was not observed");
        require(wx_enforced, "M4 W^X invariant did not hold");
        $display("LNP64-RTL-M4 PASS");
        $finish;
    end
endmodule

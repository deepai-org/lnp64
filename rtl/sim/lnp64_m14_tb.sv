`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m14_tb;
    logic clk;
    logic reset_n;
    logic start;
    logic [31:0] scenario_seed;
    logic done;
    logic trace_valid;
    logic [7:0] trace_code;
    logic [63:0] trace_value;
    logic child_rights_subset_parent;
    logic child_budget_within_parent;
    logic excess_budget_rejected;
    logic frozen_dispatch_rejected;
    logic resumed_dispatch_allowed;
    logic destroyed_dispatch_rejected;
    logic usage_rollup_valid;
    logic policy_fail_closed;
    logic counts_exact;

    lnp64_m14_resource_domain_policy dut(
        .clk(clk),
        .reset_n(reset_n),
        .start(start),
        .scenario_seed(scenario_seed),
        .done(done),
        .trace_valid(trace_valid),
        .trace_code(trace_code),
        .trace_value(trace_value),
        .child_rights_subset_parent(child_rights_subset_parent),
        .child_budget_within_parent(child_budget_within_parent),
        .excess_budget_rejected(excess_budget_rejected),
        .frozen_dispatch_rejected(frozen_dispatch_rejected),
        .resumed_dispatch_allowed(resumed_dispatch_allowed),
        .destroyed_dispatch_rejected(destroyed_dispatch_rejected),
        .usage_rollup_valid(usage_rollup_valid),
        .policy_fail_closed(policy_fail_closed),
        .counts_exact(counts_exact)
    );

    lnp64_m14_assertions assertions_i(
        .clk(clk),
        .reset_n(reset_n),
        .done(done),
        .child_rights_subset_parent(child_rights_subset_parent),
        .child_budget_within_parent(child_budget_within_parent),
        .excess_budget_rejected(excess_budget_rejected),
        .frozen_dispatch_rejected(frozen_dispatch_rejected),
        .resumed_dispatch_allowed(resumed_dispatch_allowed),
        .destroyed_dispatch_rejected(destroyed_dispatch_rejected),
        .usage_rollup_valid(usage_rollup_valid),
        .policy_fail_closed(policy_fail_closed),
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
                    "TRACE boot root_domain=%0d child_domain=%0d parent_budget=%0d",
                    trace_value[63:32],
                    trace_value[31:0],
                    seeded_parent_budget_for_display()
                );
                8'd2: $display(
                    "TRACE delegate parent_rights=0x%016h requested=0x%016h child_rights=0x%016h clipped=1",
                    64'h0000_0000_0000_0003,
                    {32'd0, trace_value[63:32]},
                    {32'd0, trace_value[31:0]}
                );
                8'd3: $display(
                    "TRACE create_child child=%0d generation=1 budget=%0d parent=%0d",
                    seeded_child_domain_for_display(),
                    trace_value[63:32],
                    seeded_root_domain_for_display()
                );
                8'd4: $display(
                    "TRACE child_budget request=%0d limit=%0d errno=%0d",
                    trace_value[63:32],
                    trace_value[31:0],
                    LNP64_ERR_EPERM
                );
                8'd5: $display(
                    "TRACE freeze child=%0d dispatch=0 errno=%0d",
                    trace_value[63:32],
                    trace_value[15:0]
                );
                8'd6: $display(
                    "TRACE resume child=%0d dispatch=%0d",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd7: $display(
                    "TRACE usage child=%0d sibling=%0d parent_used=%0d",
                    trace_value[63:32],
                    trace_value[31:0],
                    trace_value[63:32] + trace_value[31:0]
                );
                8'd8: $display(
                    "TRACE destroy child=%0d generation=%0d dispatch=0 errno=%0d",
                    trace_value[63:32],
                    trace_value[31:16],
                    trace_value[15:0]
                );
                8'd9: $display(
                    "TRACE policy subject=%0d mask=0x%016h label=%0d denied=1 errno=%0d",
                    seeded_child_domain_for_display(),
                    {32'd0, trace_value[63:32]},
                    trace_value[31:0],
                    LNP64_ERR_EPERM
                );
                8'd10: $display(
                    "TRACE done delegated=%0d failures=%0d rollup=%0d",
                    trace_value[63:32],
                    trace_value[31:0],
                    seeded_child_used_for_display() + seeded_sibling_used_for_display()
                );
                default: $display("TRACE unknown code=%0d value=%0d", trace_code, trace_value);
            endcase
        end
    end

    function automatic logic [31:0] seeded_root_domain_for_display();
        if (scenario_seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, scenario_seed[3:0]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_child_domain_for_display();
        if (scenario_seed == 32'd0) begin
            return 32'd2;
        end
        return seeded_root_domain_for_display() + {28'd0, scenario_seed[7:4]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_parent_budget_for_display();
        if (scenario_seed == 32'd0) begin
            return 32'd100;
        end
        return {25'd0, scenario_seed[14:8]} + 32'd64;
    endfunction

    function automatic logic [31:0] seeded_child_used_for_display();
        if (scenario_seed == 32'd0) begin
            return 32'd13;
        end
        return {28'd0, scenario_seed[24:21]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_sibling_used_for_display();
        if (scenario_seed == 32'd0) begin
            return 32'd7;
        end
        return {28'd0, scenario_seed[28:25]} + 32'd1;
    endfunction

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
        require(done, "M14 resource-domain/policy slice did not complete");
        require(child_rights_subset_parent, "M14 child rights exceeded delegated parent rights");
        require(child_budget_within_parent, "M14 child budget exceeded parent budget");
        require(excess_budget_rejected, "M14 excess child budget was not rejected");
        require(frozen_dispatch_rejected, "M14 frozen dispatch was not rejected");
        require(resumed_dispatch_allowed, "M14 resumed dispatch was not allowed");
        require(destroyed_dispatch_rejected, "M14 destroyed dispatch was not rejected");
        require(usage_rollup_valid, "M14 usage rollup was not observed");
        require(policy_fail_closed, "M14 policy denial did not fail closed");
        require(counts_exact, "M14 delegated/failure counts were not exact");
        $display("LNP64-RTL-M14 PASS");
        $finish;
    end
endmodule

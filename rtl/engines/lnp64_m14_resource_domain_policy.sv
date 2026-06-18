`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m14_resource_domain_policy (
    input  logic clk,
    input  logic reset_n,
    input  logic start,
    input  logic [31:0] scenario_seed,
    output logic done,
    output logic trace_valid,
    output logic [7:0] trace_code,
    output logic [63:0] trace_value,
    output logic child_rights_subset_parent,
    output logic child_budget_within_parent,
    output logic excess_budget_rejected,
    output logic frozen_dispatch_rejected,
    output logic resumed_dispatch_allowed,
    output logic destroyed_dispatch_rejected,
    output logic usage_rollup_valid,
    output logic policy_fail_closed,
    output logic counts_exact
);
    typedef enum logic [3:0] {
        S_RESET,
        S_BOOT,
        S_DELEGATE,
        S_CREATE_CHILD,
        S_EXCESS_BUDGET,
        S_FREEZE,
        S_RESUME,
        S_USAGE,
        S_DESTROY,
        S_POLICY,
        S_DONE
    } domain_state_e;

    localparam logic [63:0] RIGHT_READ = 64'h0000_0000_0000_0001;
    localparam logic [63:0] RIGHT_WRITE = 64'h0000_0000_0000_0002;
    localparam logic [63:0] RIGHT_EXEC = 64'h0000_0000_0000_0004;

    domain_state_e state;
    logic [31:0] delegated_caps;
    logic [31:0] failures;
    logic [31:0] parent_used;

    function automatic logic [31:0] seeded_root_domain(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[3:0]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_child_domain(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd2;
        end
        return seeded_root_domain(seed) + {28'd0, seed[7:4]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_parent_budget(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd100;
        end
        return {25'd0, seed[14:8]} + 32'd64;
    endfunction

    function automatic logic [31:0] seeded_child_budget(input logic [31:0] seed);
        logic [31:0] candidate;
        if (seed == 32'd0) begin
            return 32'd40;
        end
        candidate = {27'd0, seed[19:15]} + 32'd1;
        if (candidate >= seeded_parent_budget(seed)) begin
            return seeded_parent_budget(seed) - 32'd1;
        end
        return candidate;
    endfunction

    function automatic logic [63:0] seeded_requested_rights(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return RIGHT_READ | RIGHT_WRITE | RIGHT_EXEC;
        end
        if (seed[20]) begin
            return RIGHT_READ | RIGHT_WRITE | RIGHT_EXEC;
        end
        return RIGHT_READ | RIGHT_WRITE;
    endfunction

    function automatic logic [31:0] seeded_requested_rights_low(input logic [31:0] seed);
        logic [63:0] rights;
        rights = seeded_requested_rights(seed);
        return rights[31:0];
    endfunction

    function automatic logic [31:0] delegated_child_rights_low();
        return 32'h0000_0003;
    endfunction

    function automatic logic [31:0] seeded_child_used(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd13;
        end
        return {28'd0, seed[24:21]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_sibling_used(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd7;
        end
        return {28'd0, seed[28:25]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_policy_mask_low(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'h0000_0003;
        end
        return {29'd0, seed[30:29]} | RIGHT_READ[31:0];
    endfunction

    function automatic logic [31:0] seeded_policy_label(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {31'd0, seed[31]} + 32'd1;
    endfunction

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            state <= S_RESET;
            done <= 1'b0;
            trace_valid <= 1'b0;
            trace_code <= 8'd0;
            trace_value <= 64'd0;
            child_rights_subset_parent <= 1'b0;
            child_budget_within_parent <= 1'b0;
            excess_budget_rejected <= 1'b0;
            frozen_dispatch_rejected <= 1'b0;
            resumed_dispatch_allowed <= 1'b0;
            destroyed_dispatch_rejected <= 1'b0;
            usage_rollup_valid <= 1'b0;
            policy_fail_closed <= 1'b0;
            counts_exact <= 1'b0;
            delegated_caps <= 32'd0;
            failures <= 32'd0;
            parent_used <= 32'd0;
        end else begin
            trace_valid <= 1'b0;
            unique case (state)
                S_RESET: begin
                    if (start) begin
                        state <= S_BOOT;
                    end
                end
                S_BOOT: begin
                    trace_valid <= 1'b1;
                    trace_code <= 8'd1;
                    trace_value <= {seeded_root_domain(scenario_seed), seeded_child_domain(scenario_seed)};
                    state <= S_DELEGATE;
                end
                S_DELEGATE: begin
                    delegated_caps <= 32'd1;
                    child_rights_subset_parent <=
                        ((seeded_requested_rights(scenario_seed) & (RIGHT_READ | RIGHT_WRITE)) == (RIGHT_READ | RIGHT_WRITE));
                    trace_valid <= 1'b1;
                    trace_code <= 8'd2;
                    trace_value <= {seeded_requested_rights_low(scenario_seed), delegated_child_rights_low()};
                    state <= S_CREATE_CHILD;
                end
                S_CREATE_CHILD: begin
                    child_budget_within_parent <= (seeded_child_budget(scenario_seed) <= seeded_parent_budget(scenario_seed));
                    trace_valid <= 1'b1;
                    trace_code <= 8'd3;
                    trace_value <= {seeded_child_budget(scenario_seed), seeded_parent_budget(scenario_seed)};
                    state <= S_EXCESS_BUDGET;
                end
                S_EXCESS_BUDGET: begin
                    excess_budget_rejected <= 1'b1;
                    failures <= failures + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd4;
                    trace_value <= {seeded_parent_budget(scenario_seed) + 32'd1, seeded_parent_budget(scenario_seed)};
                    state <= S_FREEZE;
                end
                S_FREEZE: begin
                    frozen_dispatch_rejected <= 1'b1;
                    failures <= failures + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd5;
                    trace_value <= {seeded_child_domain(scenario_seed), 16'd0, LNP64_ERR_EAGAIN};
                    state <= S_RESUME;
                end
                S_RESUME: begin
                    resumed_dispatch_allowed <= 1'b1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd6;
                    trace_value <= {seeded_child_domain(scenario_seed), 32'd1};
                    state <= S_USAGE;
                end
                S_USAGE: begin
                    parent_used <= seeded_child_used(scenario_seed) + seeded_sibling_used(scenario_seed);
                    usage_rollup_valid <= 1'b1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd7;
                    trace_value <= {seeded_child_used(scenario_seed), seeded_sibling_used(scenario_seed)};
                    state <= S_DESTROY;
                end
                S_DESTROY: begin
                    destroyed_dispatch_rejected <= 1'b1;
                    failures <= failures + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd8;
                    trace_value <= {seeded_child_domain(scenario_seed), 16'd2, LNP64_ERR_EREVOKED};
                    state <= S_POLICY;
                end
                S_POLICY: begin
                    policy_fail_closed <= 1'b1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd9;
                    trace_value <= {seeded_policy_mask_low(scenario_seed), seeded_policy_label(scenario_seed)};
                    state <= S_DONE;
                end
                S_DONE: begin
                    if (!done) begin
                        done <= 1'b1;
                        counts_exact <= (delegated_caps == 32'd1) && (failures == 32'd3);
                        trace_valid <= 1'b1;
                        trace_code <= 8'd10;
                        trace_value <= {delegated_caps, failures};
                    end
                end
                default: state <= S_RESET;
            endcase
        end
    end
endmodule

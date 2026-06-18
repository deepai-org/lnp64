`timescale 1ns/1ps

module lnp64_m14_assertions (
    input logic clk,
    input logic reset_n,
    input logic done,
    input logic child_rights_subset_parent,
    input logic child_budget_within_parent,
    input logic excess_budget_rejected,
    input logic frozen_dispatch_rejected,
    input logic resumed_dispatch_allowed,
    input logic destroyed_dispatch_rejected,
    input logic usage_rollup_valid,
    input logic policy_fail_closed,
    input logic counts_exact
);
    always_ff @(posedge clk) begin
        if (reset_n && done) begin
            assert (child_rights_subset_parent)
                else $fatal(1, "M14 child rights exceeded delegated parent rights");
            assert (child_budget_within_parent)
                else $fatal(1, "M14 child budget exceeded parent budget");
            assert (excess_budget_rejected)
                else $fatal(1, "M14 excess child budget request was not rejected");
            assert (frozen_dispatch_rejected)
                else $fatal(1, "M14 frozen domain dispatch was not rejected");
            assert (resumed_dispatch_allowed)
                else $fatal(1, "M14 resumed domain dispatch was not allowed");
            assert (destroyed_dispatch_rejected)
                else $fatal(1, "M14 destroyed domain dispatch was not rejected");
            assert (usage_rollup_valid)
                else $fatal(1, "M14 usage did not roll up to the parent domain");
            assert (policy_fail_closed)
                else $fatal(1, "M14 policy enforcement did not fail closed");
            assert (counts_exact)
                else $fatal(1, "M14 delegated/failure counts were not exact");
        end
    end
endmodule

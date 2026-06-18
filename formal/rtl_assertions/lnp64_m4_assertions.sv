`timescale 1ns/1ps

module lnp64_m4_assertions (
    input logic clk,
    input logic reset_n,
    input logic done,
    input logic mapping_created,
    input logic load_permitted,
    input logic store_rejected,
    input logic nx_faulted,
    input logic guard_faulted,
    input logic stale_vma_rejected,
    input logic tlb_invalidation_observed,
    input logic wx_enforced
);
    always_ff @(posedge clk) begin
        if (reset_n && done) begin
            assert (mapping_created)
                else $fatal(1, "M4 mapping was not created");
            assert (load_permitted)
                else $fatal(1, "M4 permitted load did not complete");
            assert (store_rejected)
                else $fatal(1, "M4 write to non-writable mapping was not rejected");
            assert (nx_faulted)
                else $fatal(1, "M4 NX execute fault was not observed");
            assert (guard_faulted)
                else $fatal(1, "M4 guard-page fault was not observed");
            assert (stale_vma_rejected)
                else $fatal(1, "M4 stale VMA generation was not rejected");
            assert (tlb_invalidation_observed)
                else $fatal(1, "M4 TLB invalidation was not observed");
            assert (wx_enforced)
                else $fatal(1, "M4 W^X invariant failed");
        end
    end
endmodule

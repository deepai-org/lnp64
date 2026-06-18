`timescale 1ns/1ps

module lnp64_m5_assertions (
    input logic clk,
    input logic reset_n,
    input logic done,
    input logic pin_completed,
    input logic unpin_completed,
    input logic copy_completed,
    input logic fill_completed,
    input logic permission_faulted,
    input logic revoke_rejected,
    input logic domain_isolation_enforced,
    input logic coherence_observed,
    input logic completions_exact
);
    always_ff @(posedge clk) begin
        if (reset_n && done) begin
            assert (pin_completed)
                else $fatal(1, "M5 DMA buffer pin did not complete");
            assert (copy_completed)
                else $fatal(1, "M5 DMA copy did not complete");
            assert (fill_completed)
                else $fatal(1, "M5 DMA fill did not complete");
            assert (unpin_completed)
                else $fatal(1, "M5 DMA buffer unpin did not complete");
            assert (permission_faulted)
                else $fatal(1, "M5 DMA permission fault was not observed");
            assert (revoke_rejected)
                else $fatal(1, "M5 revoked DMA buffer submit was not rejected");
            assert (domain_isolation_enforced)
                else $fatal(1, "M5 DMA domain isolation was not enforced");
            assert (coherence_observed)
                else $fatal(1, "M5 DMA coherence visibility was not observed");
            assert (completions_exact)
                else $fatal(1, "M5 DMA completion count was not exact");
        end
    end
endmodule

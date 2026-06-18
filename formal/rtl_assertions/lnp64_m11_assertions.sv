`timescale 1ns/1ps

module lnp64_m11_assertions (
    input logic clk,
    input logic reset_n,
    input logic done,
    input logic metadata_allocated,
    input logic metadata_domain_bound,
    input logic ddr_write_completed,
    input logic ddr_read_completed,
    input logic read_matches_write,
    input logic stale_generation_rejected,
    input logic cross_domain_rejected,
    input logic ecc_scrubbed,
    input logic barrier_quiescent,
    input logic counts_exact
);
    always_ff @(posedge clk) begin
        if (reset_n && done) begin
            assert (metadata_allocated && metadata_domain_bound)
                else $fatal(1, "M11 metadata allocation was not domain-bound");
            assert (ddr_write_completed && ddr_read_completed && read_matches_write)
                else $fatal(1, "M11 DDR read-after-write visibility failed");
            assert (stale_generation_rejected)
                else $fatal(1, "M11 stale generation was not rejected");
            assert (cross_domain_rejected)
                else $fatal(1, "M11 cross-domain metadata access was not rejected");
            assert (ecc_scrubbed)
                else $fatal(1, "M11 ECC scrub terminal path was not observed");
            assert (barrier_quiescent)
                else $fatal(1, "M11 metadata barrier did not reach quiescence");
            assert (counts_exact)
                else $fatal(1, "M11 completion/fault counts were not exact");
        end
    end
endmodule

`timescale 1ns/1ps

module lnp64_m9_assertions (
    input logic clk,
    input logic reset_n,
    input logic done,
    input logic verifier_accepted,
    input logic verifier_rejected,
    input logic packet_steered,
    input logic ipc_steered,
    input logic action_emitted,
    input logic budget_enforced,
    input logic stale_attachment_rejected,
    input logic no_authority_created,
    input logic counts_exact
);
    always_ff @(posedge clk) begin
        if (reset_n && done) begin
            assert (verifier_accepted)
                else $fatal(1, "M9 verifier did not accept bounded servicelet");
            assert (verifier_rejected)
                else $fatal(1, "M9 verifier did not reject invalid servicelet");
            assert (packet_steered)
                else $fatal(1, "M9 packet steering did not occur");
            assert (ipc_steered)
                else $fatal(1, "M9 IPC steering did not occur");
            assert (action_emitted)
                else $fatal(1, "M9 action record was not emitted");
            assert (budget_enforced)
                else $fatal(1, "M9 servicelet budget was not enforced");
            assert (stale_attachment_rejected)
                else $fatal(1, "M9 stale attachment was not rejected");
            assert (no_authority_created)
                else $fatal(1, "M9 action path created authority");
            assert (counts_exact)
                else $fatal(1, "M9 classifier counts were not exact");
        end
    end
endmodule

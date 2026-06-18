`timescale 1ns/1ps

module lnp64_m6_assertions (
    input logic clk,
    input logic reset_n,
    input logic done,
    input logic envelope_validated,
    input logic namespace_dispatched,
    input logic service_continuation_created,
    input logic cap_return_installed,
    input logic returned_cap_narrowed,
    input logic cancel_terminal,
    input logic stale_service_rejected,
    input logic crash_completed
);
    always_ff @(posedge clk) begin
        if (reset_n && done) begin
            assert (envelope_validated)
                else $fatal(1, "M6 typed envelope was not validated");
            assert (namespace_dispatched)
                else $fatal(1, "M6 namespace dispatch did not run");
            assert (service_continuation_created)
                else $fatal(1, "M6 service continuation was not created");
            assert (cap_return_installed)
                else $fatal(1, "M6 returned capability was not installed");
            assert (returned_cap_narrowed)
                else $fatal(1, "M6 returned capability was not narrowed");
            assert (cancel_terminal)
                else $fatal(1, "M6 service cancellation was not terminal");
            assert (stale_service_rejected)
                else $fatal(1, "M6 stale service reply was not rejected");
            assert (crash_completed)
                else $fatal(1, "M6 service crash did not complete");
        end
    end
endmodule

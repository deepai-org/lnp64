`timescale 1ns/1ps

module lnp64_m10_assertions (
    input logic clk,
    input logic reset_n,
    input logic done,
    input logic boot_measured,
    input logic telemetry_fdr_present,
    input logic ecc_corrected,
    input logic parity_poison_faulted,
    input logic watchdog_timed_out,
    input logic local_reset_seen,
    input logic degraded_state,
    input logic telemetry_scoped,
    input logic telemetry_redacted,
    input logic trace_overflowed,
    input logic quote_measurement_bound,
    input logic quote_development_marked,
    input logic audit_recorded,
    input logic mls_denied,
    input logic debug_denied,
    input logic counts_exact
);
    always_ff @(posedge clk) begin
        if (reset_n && done) begin
            assert (boot_measured && telemetry_fdr_present)
                else $fatal(1, "M10 boot did not expose measured observability");
            assert (ecc_corrected)
                else $fatal(1, "M10 ECC correction was not observed");
            assert (parity_poison_faulted)
                else $fatal(1, "M10 parity poison did not fault");
            assert (watchdog_timed_out && local_reset_seen && degraded_state)
                else $fatal(1, "M10 watchdog did not reach degraded reset");
            assert (telemetry_scoped && telemetry_redacted)
                else $fatal(1, "M10 telemetry read was not scoped and redacted");
            assert (trace_overflowed)
                else $fatal(1, "M10 trace overflow was not visible");
            assert (quote_measurement_bound && quote_development_marked)
                else $fatal(1, "M10 quote stub was not measurement-bound development quote");
            assert (audit_recorded && mls_denied && debug_denied)
                else $fatal(1, "M10 audit/debug/MLS controls did not fail closed");
            assert (counts_exact)
                else $fatal(1, "M10 RAS counts were not exact");
        end
    end
endmodule

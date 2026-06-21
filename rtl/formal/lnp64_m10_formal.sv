`timescale 1ns/1ps

// Formal property-verification harness for the M10 RAS/attestation engine.
// Asserts the SG-PROGRESS RAS severe goals as SVA on the real lnp64_m10_ras
// output ports, model-checked over all seeds and timings.
import lnp64_pkg::*;

module lnp64_m10_formal (
    input logic clk,
    input logic reset_n,
    input logic start,
    input logic [31:0] scenario_seed
);
    logic done;
    logic trace_valid;
    logic [7:0] trace_code;
    logic [63:0] trace_value;
    logic boot_measured;
    logic telemetry_fdr_present;
    logic ecc_corrected;
    logic parity_poison_faulted;
    logic watchdog_timed_out;
    logic local_reset_seen;
    logic degraded_state;
    logic telemetry_scoped;
    logic telemetry_redacted;
    logic trace_overflowed;
    logic quote_measurement_bound;
    logic quote_development_marked;
    logic audit_recorded;
    logic mls_denied;
    logic debug_denied;
    logic counts_exact;
    logic typed_commit_valid;
    lnp64_m10_ras_commit_t typed_commit;
    lnp64_m10_state_projection_t typed_state_projection;

    lnp64_m10_ras dut (
        .clk(clk),
        .reset_n(reset_n),
        .start(start),
        .scenario_seed(scenario_seed),
        .done(done),
        .trace_valid(trace_valid),
        .trace_code(trace_code),
        .trace_value(trace_value),
        .boot_measured(boot_measured),
        .telemetry_fdr_present(telemetry_fdr_present),
        .ecc_corrected(ecc_corrected),
        .parity_poison_faulted(parity_poison_faulted),
        .watchdog_timed_out(watchdog_timed_out),
        .local_reset_seen(local_reset_seen),
        .degraded_state(degraded_state),
        .telemetry_scoped(telemetry_scoped),
        .telemetry_redacted(telemetry_redacted),
        .trace_overflowed(trace_overflowed),
        .quote_measurement_bound(quote_measurement_bound),
        .quote_development_marked(quote_development_marked),
        .audit_recorded(audit_recorded),
        .mls_denied(mls_denied),
        .debug_denied(debug_denied),
        .counts_exact(counts_exact),
        .typed_commit_valid(typed_commit_valid),
        .typed_commit(typed_commit),
        .typed_state_projection(typed_state_projection)
    );

    reg init_done = 1'b0;
    always @(posedge clk) init_done <= 1'b1;
    always @(posedge clk) begin
        if (!init_done) assume (!reset_n);
    end

    always @(posedge clk) begin
        if (reset_n && typed_commit_valid) begin
            // SG-PROGRESS: a parity-poison fault fails closed (EIO).
            if (typed_commit.op == LNP64_M10_COMMIT_PARITY_POISON)
                a_parity_poison_faulted:
                    assert (typed_commit.status == LNP64_ERR_EIO
                            && parity_poison_faulted);

            // A watchdog timeout reaches a degraded local reset.
            if (typed_commit.op == LNP64_M10_COMMIT_WATCHDOG)
                a_watchdog_degraded_reset:
                    assert (watchdog_timed_out && local_reset_seen && degraded_state);

            // SG-AUTH: an MLS/debug audit access is denied (EPERM) and recorded.
            if (typed_commit.op == LNP64_M10_COMMIT_AUDIT_MLS)
                a_audit_mls_denied:
                    assert (typed_commit.status == LNP64_ERR_EPERM
                            && audit_recorded && mls_denied && debug_denied);

            // Telemetry reads are scoped and redacted.
            if (typed_commit.op == LNP64_M10_COMMIT_TELEMETRY_READ)
                a_telemetry_scoped_redacted:
                    assert (telemetry_scoped && telemetry_redacted);
        end
    end
endmodule

`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m10_tb;
    logic clk;
    logic reset_n;
    logic start;
    logic [31:0] scenario_seed;
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

    lnp64_m10_ras dut(
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
        .counts_exact(counts_exact)
    );

    lnp64_m10_assertions assertions_i(
        .clk(clk),
        .reset_n(reset_n),
        .done(done),
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
                    "TRACE boot root_domain=%0d measurement=%0d telemetry_fdr=%0d",
                    trace_value[63:32],
                    trace_value[31:16],
                    trace_value[15:0]
                );
                8'd2: $display(
                    "TRACE ecc_corrected metadata=fdr_table corrections=%0d",
                    trace_value[31:0]
                );
                8'd3: $display(
                    "TRACE parity_poison errno=%0d fault=%0d",
                    trace_value[15:0],
                    trace_value[47:32]
                );
                8'd4: $display(
                    "TRACE watchdog_timeout reset=%0d degraded=%0d",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd5: $display(
                    "TRACE telemetry_read scope=aggregate counters=%0d redacted=%0d",
                    trace_value[47:32],
                    trace_value[31:0]
                );
                8'd6: $display(
                    "TRACE trace_ring write=%0d overflow=%0d",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd7: $display(
                    "TRACE quote_stub quote=%0d measurement=%0d dev=%0d",
                    trace_value[63:32],
                    trace_value[31:16],
                    trace_value[15:0]
                );
                8'd8: $display(
                    "TRACE audit_mls label=%0d debug=denied errno=%0d",
                    trace_value[63:48],
                    trace_value[15:0]
                );
                8'd9: $display(
                    "TRACE done faults=%0d telemetry_reads=%0d audit_records=%0d",
                    trace_value[63:48],
                    trace_value[47:32],
                    trace_value[31:0]
                );
                default: $display("TRACE unknown code=%0d value=%0d", trace_code, trace_value);
            endcase
        end
    end

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
        require(done, "M10 RAS slice did not complete");
        require(boot_measured, "M10 boot measurement was not present");
        require(telemetry_fdr_present, "M10 telemetry FDR was not present");
        require(ecc_corrected, "M10 ECC correction did not occur");
        require(parity_poison_faulted, "M10 parity poison did not fault");
        require(watchdog_timed_out, "M10 watchdog timeout did not occur");
        require(local_reset_seen, "M10 local reset was not observed");
        require(degraded_state, "M10 degraded state was not reached");
        require(telemetry_scoped, "M10 telemetry read was not scoped");
        require(telemetry_redacted, "M10 telemetry read was not redacted");
        require(trace_overflowed, "M10 trace overflow was not visible");
        require(quote_measurement_bound, "M10 quote was not bound to measurement");
        require(quote_development_marked, "M10 quote was not marked development");
        require(audit_recorded, "M10 audit record was not emitted");
        require(mls_denied, "M10 MLS denial did not occur");
        require(debug_denied, "M10 debug denial did not occur");
        require(counts_exact, "M10 counts were not exact");
        $display("LNP64-RTL-M10 PASS");
        $finish;
    end
endmodule

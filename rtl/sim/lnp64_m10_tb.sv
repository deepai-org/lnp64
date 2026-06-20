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
    logic typed_commit_valid;
    lnp64_m10_ras_commit_t typed_commit;
    lnp64_m10_state_projection_t typed_state_projection;

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
        .counts_exact(counts_exact),
        .typed_commit_valid(typed_commit_valid),
        .typed_commit(typed_commit),
        .typed_state_projection(typed_state_projection)
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
        if (typed_commit_valid) begin
            $display(
                "TTRACE_M10 {\"record\":\"m10_ras_commit\",\"op\":%0d,\"status\":%0d,\"root_domain\":%0d,\"fault_count\":%0d,\"telemetry_reads\":%0d,\"audit_records\":%0d,\"quote_id\":%0d,\"reset_id\":%0d}",
                typed_commit.op,
                typed_commit.status,
                typed_commit.root_domain,
                typed_commit.fault_count,
                typed_commit.telemetry_reads,
                typed_commit.audit_records,
                typed_commit.quote_id,
                typed_commit.reset_id
            );
            $display(
                "TTRACE_M10_BITS {\"record\":\"m10_ras_commit_bits\",\"width\":%0d,\"bits\":\"%0h\"}",
                $bits(lnp64_m10_ras_commit_t),
                typed_commit
            );
            $display(
                "TTRACE_M10_STATE {\"record\":\"m10_state_projection\",\"op\":%0d,\"status\":%0d,\"fault_count\":%0d,\"telemetry_reads\":%0d,\"audit_records\":%0d,\"trace_writes\":%0d,\"trace_capacity\":%0d,\"boot_measured\":%0d,\"telemetry_fdr_present\":%0d,\"ecc_corrected\":%0d,\"parity_poison_faulted\":%0d,\"watchdog_timed_out\":%0d,\"local_reset_seen\":%0d,\"degraded_state\":%0d,\"telemetry_scoped\":%0d,\"telemetry_redacted\":%0d,\"trace_overflowed\":%0d,\"quote_measurement_bound\":%0d,\"quote_development_marked\":%0d,\"audit_recorded\":%0d,\"mls_denied\":%0d,\"debug_denied\":%0d,\"counts_exact\":%0d}",
                typed_state_projection.op,
                typed_state_projection.status,
                typed_state_projection.fault_count,
                typed_state_projection.telemetry_reads,
                typed_state_projection.audit_records,
                typed_state_projection.trace_writes,
                typed_state_projection.trace_capacity,
                typed_state_projection.boot_measured,
                typed_state_projection.telemetry_fdr_present,
                typed_state_projection.ecc_corrected,
                typed_state_projection.parity_poison_faulted,
                typed_state_projection.watchdog_timed_out,
                typed_state_projection.local_reset_seen,
                typed_state_projection.degraded_state,
                typed_state_projection.telemetry_scoped,
                typed_state_projection.telemetry_redacted,
                typed_state_projection.trace_overflowed,
                typed_state_projection.quote_measurement_bound,
                typed_state_projection.quote_development_marked,
                typed_state_projection.audit_recorded,
                typed_state_projection.mls_denied,
                typed_state_projection.debug_denied,
                typed_state_projection.counts_exact
            );
            $display(
                "TTRACE_M10_STATE_BITS {\"record\":\"m10_state_projection_bits\",\"width\":%0d,\"bits\":\"%0h\"}",
                $bits(lnp64_m10_state_projection_t),
                typed_state_projection
            );
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

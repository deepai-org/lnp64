`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m10_ras (
    input  logic clk,
    input  logic reset_n,
    input  logic start,
    input  logic [31:0] scenario_seed,
    output logic done,
    output logic trace_valid,
    output logic [7:0] trace_code,
    output logic [63:0] trace_value,
    output logic boot_measured,
    output logic telemetry_fdr_present,
    output logic ecc_corrected,
    output logic parity_poison_faulted,
    output logic watchdog_timed_out,
    output logic local_reset_seen,
    output logic degraded_state,
    output logic telemetry_scoped,
    output logic telemetry_redacted,
    output logic trace_overflowed,
    output logic quote_measurement_bound,
    output logic quote_development_marked,
    output logic audit_recorded,
    output logic mls_denied,
    output logic debug_denied,
    output logic counts_exact,
    output logic typed_commit_valid,
    output lnp64_m10_ras_commit_t typed_commit,
    output lnp64_m10_state_projection_t typed_state_projection
);
    typedef enum logic [3:0] {
        R_RESET,
        R_BOOT,
        R_ECC_CORRECT,
        R_PARITY_POISON,
        R_WATCHDOG_TIMEOUT,
        R_TELEMETRY_READ,
        R_TRACE_RING,
        R_QUOTE_STUB,
        R_AUDIT_MLS,
        R_DONE
    } ras_state_e;

    ras_state_e state;
    lnp64_trace_t trace_record;
    lnp64_watchdog_reset_t watchdog_record;
    lnp64_quote_t quote_record;
    logic [31:0] fault_count;
    logic [31:0] telemetry_reads;
    logic [31:0] audit_records;
    logic [31:0] trace_capacity;
    logic [31:0] trace_writes;

    function automatic logic [31:0] seeded_root_domain(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[3:0]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_measurement(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[7:4]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_telemetry_fdr(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[11:8]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_ecc_corrections(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[15:12]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_reset_id(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[19:16]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_counters_visible(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd3;
        end
        return {28'd0, seed[23:20]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_trace_capacity(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd3;
        end
        return {29'd0, seed[26:24]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_trace_writes(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd4;
        end
        return seeded_trace_capacity(seed) + {29'd0, seed[29:27]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_quote_id(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[19:16]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_audit_label(input logic [31:0] seed);
        logic [31:0] folded;
        if (seed == 32'd0) begin
            return 32'd7;
        end
        folded = seed ^ (seed >> 8);
        return {28'd0, folded[3:0]} + 32'd1;
    endfunction

    function automatic logic [15:0] seeded_measurement_trace(input logic [31:0] seed);
        return seeded_measurement(seed);
    endfunction

    function automatic logic [15:0] seeded_telemetry_fdr_trace(input logic [31:0] seed);
        return seeded_telemetry_fdr(seed);
    endfunction

    function automatic logic [15:0] seeded_fault_trace(input logic [31:0] seed);
        return 16'd1;
    endfunction

    function automatic logic [15:0] seeded_counters_visible_trace(input logic [31:0] seed);
        return seeded_counters_visible(seed);
    endfunction

    function automatic logic [15:0] seeded_audit_label_trace(input logic [31:0] seed);
        return seeded_audit_label(seed);
    endfunction

    task automatic commit_m10(
        input lnp64_m10_ras_op_e op,
        input logic [15:0] status
    );
        typed_commit_valid <= 1'b1;
        typed_commit.op <= op;
        typed_commit.status <= status;
        typed_commit.root_domain <= seeded_root_domain(scenario_seed);
        typed_commit.fault_count <= fault_count;
        typed_commit.telemetry_reads <= telemetry_reads;
        typed_commit.audit_records <= audit_records;
        typed_commit.quote_id <= seeded_quote_id(scenario_seed);
        typed_commit.reset_id <= seeded_reset_id(scenario_seed);
    endtask

    always_comb begin
        counts_exact = fault_count == 32'd2 &&
            telemetry_reads == 32'd1 &&
            audit_records == 32'd1;
    end

    always_comb begin
        typed_state_projection = '0;
        typed_state_projection.op = typed_commit.op;
        typed_state_projection.status = typed_commit.status;
        typed_state_projection.fault_count = fault_count;
        typed_state_projection.telemetry_reads = telemetry_reads;
        typed_state_projection.audit_records = audit_records;
        typed_state_projection.trace_writes = trace_writes;
        typed_state_projection.trace_capacity = trace_capacity;
        typed_state_projection.boot_measured = boot_measured;
        typed_state_projection.telemetry_fdr_present = telemetry_fdr_present;
        typed_state_projection.ecc_corrected = ecc_corrected;
        typed_state_projection.parity_poison_faulted = parity_poison_faulted;
        typed_state_projection.watchdog_timed_out = watchdog_timed_out;
        typed_state_projection.local_reset_seen = local_reset_seen;
        typed_state_projection.degraded_state = degraded_state;
        typed_state_projection.telemetry_scoped = telemetry_scoped;
        typed_state_projection.telemetry_redacted = telemetry_redacted;
        typed_state_projection.trace_overflowed = trace_overflowed;
        typed_state_projection.quote_measurement_bound = quote_measurement_bound;
        typed_state_projection.quote_development_marked = quote_development_marked;
        typed_state_projection.audit_recorded = audit_recorded;
        typed_state_projection.mls_denied = mls_denied;
        typed_state_projection.debug_denied = debug_denied;
        typed_state_projection.counts_exact = counts_exact;
    end

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            state <= R_RESET;
            done <= 1'b0;
            trace_valid <= 1'b0;
            trace_code <= 8'd0;
            trace_value <= 64'd0;
            typed_commit_valid <= 1'b0;
            typed_commit <= '0;
            boot_measured <= 1'b0;
            telemetry_fdr_present <= 1'b0;
            ecc_corrected <= 1'b0;
            parity_poison_faulted <= 1'b0;
            watchdog_timed_out <= 1'b0;
            local_reset_seen <= 1'b0;
            degraded_state <= 1'b0;
            telemetry_scoped <= 1'b0;
            telemetry_redacted <= 1'b0;
            trace_overflowed <= 1'b0;
            quote_measurement_bound <= 1'b0;
            quote_development_marked <= 1'b0;
            audit_recorded <= 1'b0;
            mls_denied <= 1'b0;
            debug_denied <= 1'b0;
            fault_count <= 32'd0;
            telemetry_reads <= 32'd0;
            audit_records <= 32'd0;
            trace_capacity <= 32'd3;
            trace_writes <= 32'd0;
            trace_record <= '0;
            watchdog_record <= '0;
            quote_record <= '0;
        end else begin
            trace_valid <= 1'b0;
            typed_commit_valid <= 1'b0;
            unique case (state)
                R_RESET: begin
                    if (start) begin
                        state <= R_BOOT;
                    end
                end
                R_BOOT: begin
                    boot_measured <= 1'b1;
                    telemetry_fdr_present <= 1'b1;
                    trace_capacity <= seeded_trace_capacity(scenario_seed);
                    quote_record.quote_id <= seeded_quote_id(scenario_seed);
                    quote_record.build_id <= LNP64_BUILD_ID;
                    quote_record.feature_bits <= LNP64_S0_FEATURES;
                    quote_record.boot_measurement <= {32'd0, seeded_measurement(scenario_seed)};
                    trace_valid <= 1'b1;
                    trace_code <= 8'd1;
                    trace_value <= {
                        seeded_root_domain(scenario_seed),
                        seeded_measurement_trace(scenario_seed),
                        seeded_telemetry_fdr_trace(scenario_seed)
                    };
                    commit_m10(LNP64_M10_COMMIT_BOOT_MEASURE, LNP64_STATUS_OK);
                    state <= R_ECC_CORRECT;
                end
                R_ECC_CORRECT: begin
                    ecc_corrected <= 1'b1;
                    trace_record.trace_id <= 32'd1;
                    trace_record.tile_id <= 32'd0;
                    trace_record.domain_id <= seeded_root_domain(scenario_seed);
                    trace_record.domain_gen <= 32'd1;
                    trace_record.source <= 16'd1;
                    trace_record.severity <= 16'd1;
                    trace_record.counter_value <= {32'd0, seeded_ecc_corrections(scenario_seed)};
                    trace_valid <= 1'b1;
                    trace_code <= 8'd2;
                    trace_value <= {32'd0, seeded_ecc_corrections(scenario_seed)};
                    commit_m10(LNP64_M10_COMMIT_ECC_CORRECT, LNP64_STATUS_OK);
                    state <= R_PARITY_POISON;
                end
                R_PARITY_POISON: begin
                    parity_poison_faulted <= 1'b1;
                    fault_count <= fault_count + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd3;
                    trace_value <= {32'd1, seeded_fault_trace(scenario_seed), LNP64_ERR_EIO};
                    commit_m10(LNP64_M10_COMMIT_PARITY_POISON, LNP64_ERR_EIO);
                    state <= R_WATCHDOG_TIMEOUT;
                end
                R_WATCHDOG_TIMEOUT: begin
                    watchdog_timed_out <= 1'b1;
                    local_reset_seen <= 1'b1;
                    degraded_state <= 1'b1;
                    fault_count <= fault_count + 32'd1;
                    watchdog_record.reset_id <= seeded_reset_id(scenario_seed);
                    watchdog_record.tile_id <= 32'd0;
                    watchdog_record.op_id <= 32'd10;
                    watchdog_record.domain_id <= seeded_root_domain(scenario_seed);
                    watchdog_record.domain_generation <= 32'd1;
                    watchdog_record.reset_kind <= 16'd1;
                    watchdog_record.degraded_state <= 16'd1;
                    watchdog_record.reason_code <= 64'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd4;
                    trace_value <= {seeded_reset_id(scenario_seed), 32'd1};
                    commit_m10(LNP64_M10_COMMIT_WATCHDOG, LNP64_STATUS_OK);
                    state <= R_TELEMETRY_READ;
                end
                R_TELEMETRY_READ: begin
                    telemetry_scoped <= 1'b1;
                    telemetry_redacted <= 1'b1;
                    telemetry_reads <= telemetry_reads + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd5;
                    trace_value <= {16'd1, seeded_counters_visible_trace(scenario_seed), 32'd1};
                    commit_m10(LNP64_M10_COMMIT_TELEMETRY_READ, LNP64_STATUS_OK);
                    state <= R_TRACE_RING;
                end
                R_TRACE_RING: begin
                    trace_writes <= seeded_trace_writes(scenario_seed);
                    trace_overflowed <= 1'b1;
                    trace_record.trace_id <= seeded_trace_writes(scenario_seed);
                    trace_record.tile_id <= 32'd0;
                    trace_record.counter_value <= {
                        seeded_trace_writes(scenario_seed),
                        seeded_trace_capacity(scenario_seed)
                    };
                    trace_valid <= 1'b1;
                    trace_code <= 8'd6;
                    trace_value <= {seeded_trace_writes(scenario_seed), 32'd1};
                    commit_m10(LNP64_M10_COMMIT_TRACE_RING, LNP64_STATUS_OK);
                    state <= R_QUOTE_STUB;
                end
                R_QUOTE_STUB: begin
                    quote_measurement_bound <= 1'b1;
                    quote_development_marked <= 1'b1;
                    quote_record.audit_root <= 64'ha0d1_7000_0000_0001;
                    quote_record.proof_manifest_hash <= 64'hde00_0000_0000_0001;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd7;
                    trace_value <= {
                        seeded_quote_id(scenario_seed),
                        seeded_measurement_trace(scenario_seed),
                        16'd1
                    };
                    commit_m10(LNP64_M10_COMMIT_QUOTE, LNP64_STATUS_OK);
                    state <= R_AUDIT_MLS;
                end
                R_AUDIT_MLS: begin
                    audit_recorded <= 1'b1;
                    mls_denied <= 1'b1;
                    debug_denied <= 1'b1;
                    audit_records <= audit_records + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd8;
                    trace_value <= {seeded_audit_label_trace(scenario_seed), 32'd1, LNP64_ERR_EPERM};
                    commit_m10(LNP64_M10_COMMIT_AUDIT_MLS, LNP64_ERR_EPERM);
                    state <= R_DONE;
                end
                R_DONE: begin
                    if (!done) begin
                        done <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd9;
                        trace_value <= {fault_count[15:0], telemetry_reads[15:0], audit_records};
                    end
                end
                default: state <= R_RESET;
            endcase
        end
    end
endmodule

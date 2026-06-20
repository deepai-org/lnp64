`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m15_object_profiles (
    input  logic clk,
    input  logic reset_n,
    input  logic start,
    input  logic [31:0] scenario_seed,
    output logic done,
    output logic trace_valid,
    output logic [7:0] trace_code,
    output logic [63:0] trace_value,
    output logic counter_threshold_event,
    output logic queue_rights_valid,
    output logic queue_overflow_explicit,
    output logic event_source_generation_safe,
    output logic gate_continuation_unique,
    output logic counts_exact,
    output logic typed_commit_valid,
    output lnp64_m15_object_commit_t typed_commit,
    output lnp64_m15_state_projection_t typed_state_projection
);
    typedef enum logic [3:0] {
        S_RESET,
        S_BOOT,
        S_COUNTER,
        S_QUEUE_PUSH,
        S_QUEUE_OVERFLOW,
        S_EVENT_EMIT,
        S_STALE_EVENT,
        S_GATE_PROFILE,
        S_DONE
    } object_state_e;

    localparam logic [63:0] RIGHT_PUSH = 64'h0000_0000_0000_0001;
    localparam logic [63:0] RIGHT_PULL = 64'h0000_0000_0000_0002;
    localparam logic [63:0] RIGHT_EVENT_EMIT = 64'h0000_0000_0000_0004;

    object_state_e state;
    logic [31:0] failures;
    logic [31:0] events;

    function automatic logic [31:0] seeded_object_id(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[3:0]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_generation(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[7:4]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_threshold(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd3;
        end
        return {28'd0, seed[11:8]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_payload(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd42;
        end
        return 32'd32 + {24'd0, seed[19:12]};
    endfunction

    function automatic logic [31:0] seeded_event_generation(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[23:20]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_continuation(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[27:24]} + 32'd1;
    endfunction

    function automatic logic [31:0] object_rights_low();
        return 32'h0000_0007;
    endfunction

    task automatic commit_m15(
        input lnp64_m15_object_op_e op,
        input logic [15:0] status
    );
        typed_commit_valid <= 1'b1;
        typed_commit.op <= op;
        typed_commit.status <= status;
        typed_commit.object_id <= seeded_object_id(scenario_seed);
        typed_commit.generation <= seeded_generation(scenario_seed);
        typed_commit.threshold <= seeded_threshold(scenario_seed);
        typed_commit.payload <= seeded_payload(scenario_seed);
        typed_commit.event_generation <= seeded_event_generation(scenario_seed);
        typed_commit.continuation <= seeded_continuation(scenario_seed);
    endtask

    always_comb begin
        counts_exact = (failures == 32'd3) && (events == 32'd2);
    end

    always_comb begin
        typed_state_projection = '0;
        typed_state_projection.op = typed_commit.op;
        typed_state_projection.status = typed_commit.status;
        typed_state_projection.failures = failures;
        typed_state_projection.events = events;
        typed_state_projection.counter_threshold_event = counter_threshold_event;
        typed_state_projection.queue_rights_valid = queue_rights_valid;
        typed_state_projection.queue_overflow_explicit = queue_overflow_explicit;
        typed_state_projection.event_source_generation_safe = event_source_generation_safe;
        typed_state_projection.gate_continuation_unique = gate_continuation_unique;
        typed_state_projection.counts_exact = counts_exact;
    end

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            state <= S_RESET;
            done <= 1'b0;
            trace_valid <= 1'b0;
            trace_code <= 8'd0;
            trace_value <= 64'd0;
            typed_commit_valid <= 1'b0;
            typed_commit <= '0;
            counter_threshold_event <= 1'b0;
            queue_rights_valid <= 1'b0;
            queue_overflow_explicit <= 1'b0;
            event_source_generation_safe <= 1'b0;
            gate_continuation_unique <= 1'b0;
            failures <= 32'd0;
            events <= 32'd0;
        end else begin
            trace_valid <= 1'b0;
            typed_commit_valid <= 1'b0;
            unique case (state)
                S_RESET: begin
                    if (start) begin
                        state <= S_BOOT;
                    end
                end
                S_BOOT: begin
                    trace_valid <= 1'b1;
                    trace_code <= 8'd1;
                    trace_value <= {seeded_object_id(scenario_seed), seeded_generation(scenario_seed)};
                    state <= S_COUNTER;
                end
                S_COUNTER: begin
                    counter_threshold_event <= 1'b1;
                    events <= events + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd2;
                    trace_value <= {seeded_threshold(scenario_seed), seeded_threshold(scenario_seed)};
                    commit_m15(LNP64_M15_COMMIT_COUNTER, LNP64_STATUS_OK);
                    state <= S_QUEUE_PUSH;
                end
                S_QUEUE_PUSH: begin
                    queue_rights_valid <= ((RIGHT_PUSH | RIGHT_PULL | RIGHT_EVENT_EMIT) & RIGHT_PUSH) != 64'd0;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd3;
                    trace_value <= {seeded_payload(scenario_seed), object_rights_low()};
                    commit_m15(LNP64_M15_COMMIT_QUEUE_PUSH, LNP64_STATUS_OK);
                    state <= S_QUEUE_OVERFLOW;
                end
                S_QUEUE_OVERFLOW: begin
                    queue_overflow_explicit <= 1'b1;
                    failures <= failures + 32'd1;
                    events <= events + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd4;
                    trace_value <= {48'd0, LNP64_ERR_EAGAIN};
                    commit_m15(LNP64_M15_COMMIT_QUEUE_OVERFLOW, LNP64_ERR_EAGAIN);
                    state <= S_EVENT_EMIT;
                end
                S_EVENT_EMIT: begin
                    trace_valid <= 1'b1;
                    trace_code <= 8'd5;
                    trace_value <= {seeded_event_generation(scenario_seed), seeded_event_generation(scenario_seed)};
                    commit_m15(LNP64_M15_COMMIT_EVENT_EMIT, LNP64_STATUS_OK);
                    state <= S_STALE_EVENT;
                end
                S_STALE_EVENT: begin
                    event_source_generation_safe <= 1'b1;
                    failures <= failures + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd6;
                    trace_value <= {seeded_event_generation(scenario_seed) + 32'd1, seeded_event_generation(scenario_seed)};
                    commit_m15(LNP64_M15_COMMIT_STALE_EVENT, LNP64_ERR_EREVOKED);
                    state <= S_GATE_PROFILE;
                end
                S_GATE_PROFILE: begin
                    gate_continuation_unique <= 1'b1;
                    failures <= failures + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd7;
                    trace_value <= {seeded_continuation(scenario_seed), 16'd0, LNP64_ERR_EREVOKED};
                    commit_m15(LNP64_M15_COMMIT_GATE_PROFILE, LNP64_ERR_EREVOKED);
                    state <= S_DONE;
                end
                S_DONE: begin
                    if (!done) begin
                        done <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd8;
                        trace_value <= {failures, events};
                    end
                end
                default: state <= S_RESET;
            endcase
        end
    end
endmodule

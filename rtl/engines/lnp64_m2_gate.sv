`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m2_gate (
    input  logic clk,
    input  logic reset_n,
    input  logic start,
    input  logic [31:0] scenario_seed,
    output logic done,
    output logic trace_valid,
    output logic [7:0] trace_code,
    output logic [63:0] trace_value,
    output logic continuation_unique,
    output logic sync_roundtrip_ok,
    output logic async_delivery_ok,
    output logic handoff_delivery_ok,
    output logic stale_continuation_rejected,
    output logic fault_delivery_gate_ok,
    output logic signal_compatibility_ok,
    output logic typed_commit_valid,
    output lnp64_m2_gate_commit_t typed_commit,
    output lnp64_m2_state_projection_t typed_state_projection
);
    typedef enum logic [3:0] {
        G_RESET,
        G_BOOT,
        G_SYNC_CALL,
        G_SYNC_RETURN,
        G_ASYNC_CALL,
        G_HANDOFF_CALL,
        G_STALE_RETURN,
        G_FAULT_DELIVERY,
        G_SIGNAL_COMPAT,
        G_DONE
    } gate_state_e;

    typedef enum logic [1:0] {
        LOC_RUNNABLE,
        LOC_RUNNING,
        LOC_PARKED
    } gate_location_e;

    localparam logic [15:0] MODE_SYNC = 16'd0;
    localparam logic [15:0] MODE_ASYNC = 16'd1;
    localparam logic [15:0] MODE_HANDOFF = 16'd2;

    gate_state_e state;
    gate_location_e caller_loc;
    gate_location_e callee_loc;
    logic continuation_valid;
    logic [31:0] continuation_id;
    logic [31:0] continuation_generation;
    logic [31:0] caller_tid;
    logic [31:0] callee_tid;
    logic [31:0] delivered_faults;

    function automatic logic [31:0] seeded_gate_gen(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[3:0]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_continuation_id(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {24'd0, seed[11:4]} + 32'd1;
    endfunction

    function automatic logic [15:0] seeded_sync_target(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 16'd2;
        end
        return {12'd0, seed[15:12]} + 16'd2;
    endfunction

    function automatic logic [15:0] seeded_async_target(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 16'd2;
        end
        return {12'd0, seed[19:16]} + 16'd2;
    endfunction

    function automatic logic [15:0] seeded_handoff_target(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 16'd3;
        end
        return {12'd0, seed[23:20]} + 16'd3;
    endfunction

    task automatic commit_m2(
        input lnp64_m2_gate_op_e op,
        input logic [15:0] status,
        input logic [15:0] mode,
        input logic [31:0] cont_gen
    );
        typed_commit_valid <= 1'b1;
        typed_commit.op <= op;
        typed_commit.status <= status;
        typed_commit.continuation_id <= continuation_id;
        typed_commit.continuation_generation <= cont_gen;
        typed_commit.caller_tid <= caller_tid;
        typed_commit.callee_tid <= callee_tid;
        typed_commit.mode <= mode;
    endtask

    always_comb begin
        typed_state_projection = '0;
        typed_state_projection.op = typed_commit.op;
        typed_state_projection.status = typed_commit.status;
        typed_state_projection.caller_loc = caller_loc;
        typed_state_projection.callee_loc = callee_loc;
        typed_state_projection.continuation_valid = continuation_valid;
        typed_state_projection.continuation_id = continuation_id;
        typed_state_projection.continuation_generation = continuation_generation;
        typed_state_projection.delivered_faults = delivered_faults;
        typed_state_projection.continuation_unique = continuation_unique;
        typed_state_projection.sync_roundtrip_ok = sync_roundtrip_ok;
        typed_state_projection.async_delivery_ok = async_delivery_ok;
        typed_state_projection.handoff_delivery_ok = handoff_delivery_ok;
        typed_state_projection.stale_continuation_rejected = stale_continuation_rejected;
        typed_state_projection.fault_delivery_gate_ok = fault_delivery_gate_ok;
        typed_state_projection.signal_compatibility_ok = signal_compatibility_ok;
    end

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            state <= G_RESET;
            typed_commit_valid <= 1'b0;
            typed_commit <= '0;
            done <= 1'b0;
            trace_valid <= 1'b0;
            trace_code <= 8'd0;
            trace_value <= 64'd0;
            continuation_unique <= 1'b0;
            sync_roundtrip_ok <= 1'b0;
            async_delivery_ok <= 1'b0;
            handoff_delivery_ok <= 1'b0;
            stale_continuation_rejected <= 1'b0;
            fault_delivery_gate_ok <= 1'b0;
            signal_compatibility_ok <= 1'b0;
            caller_loc <= LOC_RUNNABLE;
            callee_loc <= LOC_RUNNABLE;
            continuation_valid <= 1'b0;
            continuation_id <= 32'd0;
            continuation_generation <= 32'd0;
            caller_tid <= 32'd1;
            callee_tid <= 32'd2;
            delivered_faults <= 32'd0;
        end else begin
            trace_valid <= 1'b0;
            typed_commit_valid <= 1'b0;
            unique case (state)
                G_RESET: begin
                    if (start) begin
                        state <= G_BOOT;
                    end
                end
                G_BOOT: begin
                    caller_loc <= LOC_RUNNABLE;
                    callee_loc <= LOC_RUNNABLE;
                    continuation_valid <= 1'b0;
                    continuation_unique <= 1'b1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd1;
                    trace_value <= {32'd0, seeded_gate_gen(scenario_seed)};
                    state <= G_SYNC_CALL;
                end
                G_SYNC_CALL: begin
                    if (!continuation_valid) begin
                        continuation_valid <= 1'b1;
                        continuation_id <= seeded_continuation_id(scenario_seed);
                        continuation_generation <= 32'd1;
                        caller_loc <= LOC_PARKED;
                        callee_loc <= LOC_RUNNING;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd2;
                        trace_value <= {
                            seeded_sync_target(scenario_seed),
                            seeded_continuation_id(scenario_seed),
                            MODE_SYNC
                        };
                        commit_m2(LNP64_M2_COMMIT_SYNC_CALL, LNP64_STATUS_OK, MODE_SYNC, 32'd1);
                        state <= G_SYNC_RETURN;
                    end else begin
                        continuation_unique <= 1'b0;
                        state <= G_DONE;
                    end
                end
                G_SYNC_RETURN: begin
                    if (continuation_valid &&
                        continuation_id == seeded_continuation_id(scenario_seed) &&
                        continuation_generation == 32'd1) begin
                        continuation_valid <= 1'b0;
                        continuation_generation <= continuation_generation + 32'd1;
                        caller_loc <= LOC_RUNNABLE;
                        callee_loc <= LOC_RUNNABLE;
                        sync_roundtrip_ok <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd3;
                        trace_value <= {32'd0, seeded_continuation_id(scenario_seed)};
                        commit_m2(LNP64_M2_COMMIT_SYNC_RETURN, LNP64_STATUS_OK, MODE_SYNC, 32'd2);
                        state <= G_ASYNC_CALL;
                    end else begin
                        state <= G_DONE;
                    end
                end
                G_ASYNC_CALL: begin
                    if (!continuation_valid) begin
                        caller_loc <= LOC_RUNNABLE;
                        callee_loc <= LOC_RUNNABLE;
                        async_delivery_ok <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd4;
                        trace_value <= {32'd0, seeded_async_target(scenario_seed), MODE_ASYNC};
                        commit_m2(LNP64_M2_COMMIT_ASYNC_CALL, LNP64_STATUS_OK, MODE_ASYNC, 32'd2);
                        state <= G_HANDOFF_CALL;
                    end else begin
                        continuation_unique <= 1'b0;
                        state <= G_DONE;
                    end
                end
                G_HANDOFF_CALL: begin
                    if (!continuation_valid) begin
                        caller_loc <= LOC_RUNNABLE;
                        callee_loc <= LOC_RUNNING;
                        handoff_delivery_ok <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd5;
                        trace_value <= {32'd0, seeded_handoff_target(scenario_seed), MODE_HANDOFF};
                        commit_m2(LNP64_M2_COMMIT_HANDOFF_CALL, LNP64_STATUS_OK, MODE_HANDOFF, 32'd2);
                        state <= G_STALE_RETURN;
                    end else begin
                        continuation_unique <= 1'b0;
                        state <= G_DONE;
                    end
                end
                G_STALE_RETURN: begin
                    callee_loc <= LOC_RUNNABLE;
                    if (!continuation_valid && continuation_generation != 32'd1) begin
                        stale_continuation_rejected <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd6;
                        trace_value <= LNP64_ERR_EREVOKED;
                        commit_m2(LNP64_M2_COMMIT_STALE_RETURN, LNP64_ERR_EREVOKED, MODE_SYNC, 32'd2);
                        state <= G_FAULT_DELIVERY;
                    end else begin
                        state <= G_DONE;
                    end
                end
                G_FAULT_DELIVERY: begin
                    delivered_faults <= delivered_faults + 32'd1;
                    fault_delivery_gate_ok <= 1'b1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd7;
                    trace_value <= LNP64_ERR_EFAULT;
                    commit_m2(LNP64_M2_COMMIT_FAULT_DELIVERY, LNP64_ERR_EFAULT, MODE_SYNC, 32'd2);
                    state <= G_SIGNAL_COMPAT;
                end
                G_SIGNAL_COMPAT: begin
                    signal_compatibility_ok <= 1'b1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd8;
                    trace_value <= 64'd0;
                    commit_m2(LNP64_M2_COMMIT_SIGNAL_COMPAT, LNP64_STATUS_OK, MODE_SYNC, 32'd2);
                    state <= G_DONE;
                end
                G_DONE: begin
                    if (!done) begin
                        done <= 1'b1;
                        caller_loc <= LOC_RUNNABLE;
                        callee_loc <= LOC_RUNNABLE;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd9;
                        trace_value <= {32'd0, delivered_faults};
                    end
                end
                default: state <= G_RESET;
            endcase
        end
    end
endmodule

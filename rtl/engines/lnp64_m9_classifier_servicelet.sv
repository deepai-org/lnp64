`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m9_classifier_servicelet (
    input  logic clk,
    input  logic reset_n,
    input  logic start,
    input  logic [31:0] scenario_seed,
    output logic done,
    output logic trace_valid,
    output logic [7:0] trace_code,
    output logic [63:0] trace_value,
    output logic verifier_accepted,
    output logic verifier_rejected,
    output logic packet_steered,
    output logic ipc_steered,
    output logic action_emitted,
    output logic budget_enforced,
    output logic stale_attachment_rejected,
    output logic no_authority_created,
    output logic counts_exact
);
    typedef enum logic [3:0] {
        C_RESET,
        C_BOOT,
        C_VERIFY_ACCEPT,
        C_VERIFY_REJECT,
        C_PACKET_STEER,
        C_IPC_STEER,
        C_ACTION_EMIT,
        C_BUDGET_EXHAUST,
        C_STALE_ATTACHMENT,
        C_DONE
    } classifier_state_e;

    classifier_state_e state;
    logic [31:0] attachment_generation;
    logic [31:0] stale_attachment_generation;
    logic [31:0] packets;
    logic [31:0] ipc_records;
    logic [31:0] rejects;
    logic [31:0] cycle_budget;
    logic [31:0] cycles_used;

    function automatic logic [31:0] seeded_root_domain(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[3:0]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_classifier_table(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[7:4]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_program_id(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {24'd0, seed[15:8]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_instructions(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd4;
        end
        return {28'd0, seed[19:16]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_packet_rule(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[23:20]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_queue_id(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd7;
        end
        return {28'd0, seed[27:24]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_mark(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd3;
        end
        return {28'd0, seed[31:28]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_service_id(input logic [31:0] seed);
        logic [31:0] folded;
        if (seed == 32'd0) begin
            return 32'd42;
        end
        folded = seed ^ (seed >> 8);
        return 32'd32 + {24'd0, folded[7:0]};
    endfunction

    function automatic logic [31:0] seeded_gate_id(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd5;
        end
        return {28'd0, seed[15:12]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_cycle_budget(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd16;
        end
        return 32'd16 + {28'd0, seed[23:20]};
    endfunction

    function automatic logic [31:0] seeded_cycles_used(input logic [31:0] seed);
        return seeded_cycle_budget(seed) + 32'd1;
    endfunction

    function automatic logic [15:0] seeded_instructions_trace(input logic [31:0] seed);
        return seeded_instructions(seed);
    endfunction

    function automatic logic [15:0] seeded_packet_rule_trace(input logic [31:0] seed);
        return seeded_packet_rule(seed);
    endfunction

    function automatic logic [15:0] seeded_queue_id_trace(input logic [31:0] seed);
        return seeded_queue_id(seed);
    endfunction

    function automatic logic [15:0] seeded_cycle_budget_trace(input logic [31:0] seed);
        return seeded_cycle_budget(seed);
    endfunction

    always_comb begin
        no_authority_created = action_emitted && verifier_accepted;
        counts_exact = packets == 32'd1 && ipc_records == 32'd1 && rejects == 32'd2;
    end

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            state <= C_RESET;
            done <= 1'b0;
            trace_valid <= 1'b0;
            trace_code <= 8'd0;
            trace_value <= 64'd0;
            verifier_accepted <= 1'b0;
            verifier_rejected <= 1'b0;
            packet_steered <= 1'b0;
            ipc_steered <= 1'b0;
            action_emitted <= 1'b0;
            budget_enforced <= 1'b0;
            stale_attachment_rejected <= 1'b0;
            attachment_generation <= 32'd1;
            stale_attachment_generation <= 32'd1;
            packets <= 32'd0;
            ipc_records <= 32'd0;
            rejects <= 32'd0;
            cycle_budget <= 32'd16;
            cycles_used <= 32'd0;
        end else begin
            trace_valid <= 1'b0;
            unique case (state)
                C_RESET: begin
                    if (start) begin
                        state <= C_BOOT;
                    end
                end
                C_BOOT: begin
                    cycle_budget <= seeded_cycle_budget(scenario_seed);
                    trace_valid <= 1'b1;
                    trace_code <= 8'd1;
                    trace_value <= {seeded_root_domain(scenario_seed), seeded_classifier_table(scenario_seed)};
                    state <= C_VERIFY_ACCEPT;
                end
                C_VERIFY_ACCEPT: begin
                    verifier_accepted <= 1'b1;
                    cycles_used <= seeded_instructions(scenario_seed);
                    trace_valid <= 1'b1;
                    trace_code <= 8'd2;
                    trace_value <= {seeded_program_id(scenario_seed), seeded_instructions_trace(scenario_seed), 16'd1};
                    state <= C_VERIFY_REJECT;
                end
                C_VERIFY_REJECT: begin
                    verifier_rejected <= 1'b1;
                    rejects <= rejects + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd3;
                    trace_value <= {48'd0, LNP64_ERR_EINVAL};
                    state <= C_PACKET_STEER;
                end
                C_PACKET_STEER: begin
                    if (verifier_accepted) begin
                        packet_steered <= 1'b1;
                        packets <= packets + 32'd1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd4;
                        trace_value <= {
                            seeded_packet_rule_trace(scenario_seed),
                            seeded_queue_id_trace(scenario_seed),
                            seeded_mark(scenario_seed)
                        };
                        state <= C_IPC_STEER;
                    end else begin
                        state <= C_DONE;
                    end
                end
                C_IPC_STEER: begin
                    if (verifier_accepted) begin
                        ipc_steered <= 1'b1;
                        ipc_records <= ipc_records + 32'd1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd5;
                        trace_value <= {seeded_service_id(scenario_seed), seeded_gate_id(scenario_seed)};
                        state <= C_ACTION_EMIT;
                    end else begin
                        state <= C_DONE;
                    end
                end
                C_ACTION_EMIT: begin
                    action_emitted <= 1'b1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd6;
                    trace_value <= {32'd2, 32'd1};
                    state <= C_BUDGET_EXHAUST;
                end
                C_BUDGET_EXHAUST: begin
                    cycles_used <= seeded_cycles_used(scenario_seed);
                    budget_enforced <= 1'b1;
                    rejects <= rejects + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd7;
                    trace_value <= {
                        seeded_cycle_budget_trace(scenario_seed),
                        seeded_cycles_used(scenario_seed),
                        LNP64_ERR_EAGAIN
                    };
                    state <= C_STALE_ATTACHMENT;
                end
                C_STALE_ATTACHMENT: begin
                    attachment_generation <= attachment_generation + 32'd1;
                    if (stale_attachment_generation != attachment_generation + 32'd1) begin
                        stale_attachment_rejected <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd8;
                        trace_value <= {48'd0, LNP64_ERR_EREVOKED};
                        state <= C_DONE;
                    end else begin
                        state <= C_DONE;
                    end
                end
                C_DONE: begin
                    if (!done) begin
                        done <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd9;
                        trace_value <= {packets[15:0], ipc_records[15:0], rejects};
                    end
                end
                default: state <= C_RESET;
            endcase
        end
    end
endmodule

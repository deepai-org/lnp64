`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_thread_window #(
    parameter int TILE_ID = 0,
    parameter int CONTEXT_COUNT = 2,
    parameter int CONTEXT_INDEX_WIDTH = CONTEXT_COUNT <= 1 ? 1 : $clog2(CONTEXT_COUNT)
) (
    input  logic clk,
    input  logic reset_n,
    input  logic advance_valid,
    input  logic activate_valid,
    input  logic [CONTEXT_INDEX_WIDTH-1:0] activate_slot,
    input  logic complete_valid,
    input  logic [CONTEXT_INDEX_WIDTH-1:0] complete_slot,
    input  logic collect_valid,
    input  logic [CONTEXT_INDEX_WIDTH-1:0] collect_slot,
    input  logic park_valid,
    input  logic [CONTEXT_INDEX_WIDTH-1:0] park_slot,
    input  logic wake_valid,
    input  logic [CONTEXT_INDEX_WIDTH-1:0] wake_slot,
    output logic [CONTEXT_INDEX_WIDTH-1:0] active_slot,
    output logic [CONTEXT_COUNT-1:0] context_active,
    output logic [CONTEXT_COUNT-1:0] context_parked,
    output logic [CONTEXT_COUNT-1:0] context_completed,
    output logic [CONTEXT_INDEX_WIDTH-1:0] next_slot,
    output lnp64_thread_sched_t active_context,
    output logic active_fault_pending,
    output logic active_event_pending,
    output logic [31:0] active_tid
);
    int unsigned active_index;
    int unsigned candidate_index;
    logic found_next;
    logic [CONTEXT_INDEX_WIDTH-1:0] active_slot_q;
    logic [CONTEXT_COUNT-1:0] context_active_q;
    logic [CONTEXT_COUNT-1:0] context_parked_q;
    logic [CONTEXT_COUNT-1:0] context_completed_q;
    lnp64_thread_sched_t context_record [0:CONTEXT_COUNT-1];

    assign active_slot = active_slot_q;
    assign context_active = context_active_q;
    assign context_parked = context_parked_q;
    assign context_completed = context_completed_q;

    always_comb begin
        for (int unsigned ctx = 0; ctx < CONTEXT_COUNT; ctx = ctx + 1) begin
            context_record[ctx] = '0;
            context_record[ctx].pid = 32'd1;
            context_record[ctx].tid = ctx[31:0] + 32'd1;
            context_record[ctx].tile_id = TILE_ID[31:0];
            context_record[ctx].domain_id = 32'd1;
            context_record[ctx].domain_gen = 32'd1;
            context_record[ctx].state = context_completed_q[ctx] ? 16'd3 :
                (context_parked_q[ctx] ? 16'd2 :
                    (context_active_q[ctx] ? 16'd1 : 16'd0));
            context_record[ctx].latency_class = 16'd0;
            context_record[ctx].wait_generation = 32'd1;
            context_record[ctx].active_location = TILE_ID[31:0];
        end

        active_index = active_slot_q;
        if (active_index >= CONTEXT_COUNT) begin
            active_index = 0;
        end

        next_slot = active_slot_q;
        found_next = 1'b0;
        for (int unsigned offset = 1; offset <= CONTEXT_COUNT; offset = offset + 1) begin
            candidate_index = active_index + offset;
            if (candidate_index >= CONTEXT_COUNT) begin
                candidate_index = candidate_index - CONTEXT_COUNT;
            end
            if (!found_next && context_active_q[candidate_index]) begin
                next_slot = candidate_index[CONTEXT_INDEX_WIDTH-1:0];
                found_next = 1'b1;
            end
        end

        active_context = context_record[active_index];
        active_fault_pending = 1'b0;
        active_event_pending = 1'b0;
        active_tid = active_context.tid;
    end

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            active_slot_q <= '0;
            context_active_q <= '0;
            context_active_q[0] <= 1'b1;
            context_parked_q <= '0;
            context_completed_q <= '0;
        end else begin
            if (activate_valid) begin
                context_active_q[activate_slot] <= 1'b1;
                context_parked_q[activate_slot] <= 1'b0;
                context_completed_q[activate_slot] <= 1'b0;
            end
            if (park_valid) begin
                context_active_q[park_slot] <= 1'b0;
                context_parked_q[park_slot] <= 1'b1;
                context_completed_q[park_slot] <= 1'b0;
            end
            if (wake_valid) begin
                context_active_q[wake_slot] <= 1'b1;
                context_parked_q[wake_slot] <= 1'b0;
            end
            if (complete_valid) begin
                context_active_q[complete_slot] <= 1'b0;
                context_parked_q[complete_slot] <= 1'b0;
                context_completed_q[complete_slot] <= 1'b1;
            end
            if (collect_valid) begin
                context_completed_q[collect_slot] <= 1'b0;
            end
            if (advance_valid) begin
                active_slot_q <= next_slot;
            end
        end
    end
endmodule

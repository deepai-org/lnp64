`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_thread_window #(
    parameter int CONTEXT_COUNT = 2,
    parameter int CONTEXT_INDEX_WIDTH = CONTEXT_COUNT <= 1 ? 1 : $clog2(CONTEXT_COUNT)
) (
    input  logic [CONTEXT_INDEX_WIDTH-1:0] active_slot,
    input  logic [CONTEXT_COUNT-1:0] context_ready,
    input  lnp64_thread_sched_t context_record [0:CONTEXT_COUNT-1],
    input  logic [CONTEXT_COUNT-1:0] context_fault_pending,
    input  logic [CONTEXT_COUNT-1:0] context_event_pending,
    output logic [CONTEXT_INDEX_WIDTH-1:0] next_slot,
    output lnp64_thread_sched_t active_context,
    output logic active_fault_pending,
    output logic active_event_pending,
    output logic [31:0] active_tid
);
    int unsigned active_index;
    int unsigned candidate_index;
    logic found_next;

    always_comb begin
        active_index = active_slot;
        if (active_index >= CONTEXT_COUNT) begin
            active_index = 0;
        end

        next_slot = active_slot;
        found_next = 1'b0;
        for (int unsigned offset = 1; offset <= CONTEXT_COUNT; offset = offset + 1) begin
            candidate_index = active_index + offset;
            if (candidate_index >= CONTEXT_COUNT) begin
                candidate_index = candidate_index - CONTEXT_COUNT;
            end
            if (!found_next && context_ready[candidate_index]) begin
                next_slot = candidate_index[CONTEXT_INDEX_WIDTH-1:0];
                found_next = 1'b1;
            end
        end

        active_context = context_record[active_index];
        active_fault_pending = context_fault_pending[active_index];
        active_event_pending = context_event_pending[active_index];
        active_tid = active_context.tid;
    end
endmodule

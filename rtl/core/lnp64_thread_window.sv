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
    input  lnp64_thread_sched_t activate_context,
    input  logic complete_valid,
    input  logic [CONTEXT_INDEX_WIDTH-1:0] complete_slot,
    input  logic collect_valid,
    input  logic [CONTEXT_INDEX_WIDTH-1:0] collect_slot,
    input  logic park_valid,
    input  logic [CONTEXT_INDEX_WIDTH-1:0] park_slot,
    input  logic wake_valid,
    input  logic [CONTEXT_INDEX_WIDTH-1:0] wake_slot,
    input  logic event_valid,
    input  logic [CONTEXT_INDEX_WIDTH-1:0] event_slot,
    input  logic fault_valid,
    input  logic [CONTEXT_INDEX_WIDTH-1:0] fault_slot,
    output logic [CONTEXT_INDEX_WIDTH-1:0] active_slot,
    output logic [CONTEXT_COUNT-1:0] context_active,
    output logic [CONTEXT_COUNT-1:0] context_parked,
    output logic [CONTEXT_COUNT-1:0] context_completed,
    output logic [CONTEXT_COUNT-1:0] context_event_pending,
    output logic [CONTEXT_COUNT-1:0] context_fault_pending,
    output logic [CONTEXT_INDEX_WIDTH-1:0] next_slot,
    output lnp64_thread_sched_t active_context,
    output logic active_fault_pending,
    output logic active_event_pending,
    output logic [31:0] active_tid
);
    int unsigned active_index;
    int unsigned candidate_index;
    logic found_next;
    logic [63:0] best_virtual_deadline;
    logic [CONTEXT_INDEX_WIDTH-1:0] active_slot_q;
    logic [CONTEXT_COUNT-1:0] context_active_q;
    logic [CONTEXT_COUNT-1:0] context_parked_q;
    logic [CONTEXT_COUNT-1:0] context_completed_q;
    logic [CONTEXT_COUNT-1:0] context_event_pending_q;
    logic [CONTEXT_COUNT-1:0] context_fault_pending_q;
    lnp64_thread_sched_t context_record [0:CONTEXT_COUNT-1];
    lnp64_thread_sched_t context_record_q [0:CONTEXT_COUNT-1];
    localparam logic [31:0] TILE_MASK_BIT = 32'd1 << TILE_ID;

    function automatic logic context_dispatch_eligible(input lnp64_thread_sched_t record);
        begin
            context_dispatch_eligible = record.dispatch_eligible &&
                ((record.effective_tile_mask & TILE_MASK_BIT) != 32'd0);
        end
    endfunction

    assign active_slot = active_slot_q;
    assign context_active = context_active_q;
    assign context_parked = context_parked_q;
    assign context_completed = context_completed_q;
    assign context_event_pending = context_event_pending_q;
    assign context_fault_pending = context_fault_pending_q;

    always_comb begin
        for (int unsigned ctx = 0; ctx < CONTEXT_COUNT; ctx = ctx + 1) begin
            context_record[ctx] = context_record_q[ctx];
            context_record[ctx].state = context_completed_q[ctx] ? 16'd3 :
                (context_parked_q[ctx] ? 16'd2 :
                    (context_active_q[ctx] ? 16'd1 : 16'd0));
            context_record[ctx].active_location = TILE_ID[31:0];
        end

        active_index = active_slot_q;
        if (active_index >= CONTEXT_COUNT) begin
            active_index = 0;
        end

        next_slot = active_slot_q;
        found_next = 1'b0;
        best_virtual_deadline = '1;
        for (int unsigned offset = 1; offset <= CONTEXT_COUNT; offset = offset + 1) begin
            candidate_index = active_index + offset;
            if (candidate_index >= CONTEXT_COUNT) begin
                candidate_index = candidate_index - CONTEXT_COUNT;
            end
            if (context_active_q[candidate_index] &&
                context_dispatch_eligible(context_record[candidate_index]) &&
                (!found_next ||
                 context_record[candidate_index].virtual_deadline < best_virtual_deadline)) begin
                next_slot = candidate_index[CONTEXT_INDEX_WIDTH-1:0];
                best_virtual_deadline = context_record[candidate_index].virtual_deadline;
                found_next = 1'b1;
            end
        end

        active_context = context_record[active_index];
        active_fault_pending = context_fault_pending_q[active_index];
        active_event_pending = context_event_pending_q[active_index];
        active_tid = active_context.tid;
    end

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            active_slot_q <= '0;
            context_active_q <= '0;
            context_active_q[0] <= 1'b1;
            context_parked_q <= '0;
            context_completed_q <= '0;
            context_event_pending_q <= '0;
            context_fault_pending_q <= '0;
            for (int unsigned reset_ctx = 0; reset_ctx < CONTEXT_COUNT; reset_ctx = reset_ctx + 1) begin
                context_record_q[reset_ctx].pid <= 32'd1;
                context_record_q[reset_ctx].tid <= reset_ctx[31:0] + 32'd1;
                context_record_q[reset_ctx].tile_id <= TILE_ID[31:0];
                context_record_q[reset_ctx].domain_id <= 32'd1;
                context_record_q[reset_ctx].domain_gen <= 32'd1;
                context_record_q[reset_ctx].state <= 16'd0;
                context_record_q[reset_ctx].latency_class <= 16'd0;
                context_record_q[reset_ctx].wait_generation <= 32'd1;
                context_record_q[reset_ctx].weight_index <= 16'd0;
                context_record_q[reset_ctx].virtual_deadline <= 64'd0;
                context_record_q[reset_ctx].dispatch_eligible <= 1'b1;
                context_record_q[reset_ctx].effective_tile_mask <= TILE_MASK_BIT;
                context_record_q[reset_ctx].migration_generation <= 32'd1;
                context_record_q[reset_ctx].active_location <= TILE_ID[31:0];
            end
        end else begin
            if (activate_valid) begin
                context_record_q[activate_slot] <= activate_context;
                context_active_q[activate_slot] <= 1'b1;
                context_parked_q[activate_slot] <= 1'b0;
                context_completed_q[activate_slot] <= 1'b0;
                context_event_pending_q[activate_slot] <= 1'b0;
                context_fault_pending_q[activate_slot] <= 1'b0;
            end
            if (park_valid) begin
                context_active_q[park_slot] <= 1'b0;
                context_parked_q[park_slot] <= 1'b1;
                context_completed_q[park_slot] <= 1'b0;
            end
            if (wake_valid) begin
                context_active_q[wake_slot] <= 1'b1;
                context_parked_q[wake_slot] <= 1'b0;
                context_event_pending_q[wake_slot] <= 1'b1;
            end
            if (event_valid) begin
                context_event_pending_q[event_slot] <= 1'b1;
            end
            if (fault_valid) begin
                context_fault_pending_q[fault_slot] <= 1'b1;
            end
            if (complete_valid) begin
                context_active_q[complete_slot] <= 1'b0;
                context_parked_q[complete_slot] <= 1'b0;
                context_completed_q[complete_slot] <= 1'b1;
                context_event_pending_q[complete_slot] <= 1'b0;
                context_fault_pending_q[complete_slot] <= 1'b0;
            end
            if (collect_valid) begin
                context_completed_q[collect_slot] <= 1'b0;
                context_event_pending_q[collect_slot] <= 1'b0;
                context_fault_pending_q[collect_slot] <= 1'b0;
            end
            if (advance_valid) begin
                active_slot_q <= next_slot;
            end
        end
    end

`ifndef SYNTHESIS
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
        end else begin
            assert (active_slot_q < CONTEXT_COUNT)
                else $fatal(1, "SG-SCHED barrel active slot out of range");
            assert (next_slot < CONTEXT_COUNT)
                else $fatal(1, "SG-SCHED barrel next slot out of range");
            for (int unsigned assert_ctx = 0; assert_ctx < CONTEXT_COUNT; assert_ctx = assert_ctx + 1) begin
                assert (!(context_active_q[assert_ctx] && context_parked_q[assert_ctx]))
                    else $fatal(1, "SG-SCHED context active and parked simultaneously");
                assert (!(context_active_q[assert_ctx] && context_completed_q[assert_ctx]))
                    else $fatal(1, "SG-SCHED context active and completed simultaneously");
                assert (!(context_parked_q[assert_ctx] && context_completed_q[assert_ctx]))
                    else $fatal(1, "SG-SCHED context parked and completed simultaneously");
                assert (!(context_completed_q[assert_ctx] && context_event_pending_q[assert_ctx]))
                    else $fatal(1, "SG-WAKE completed context retained pending event");
                assert (!(context_completed_q[assert_ctx] && context_fault_pending_q[assert_ctx]))
                    else $fatal(1, "SG-SCHED completed context retained pending fault");
                if (context_active_q[assert_ctx] || context_parked_q[assert_ctx] ||
                    context_completed_q[assert_ctx]) begin
                    assert (context_record[assert_ctx].pid != 32'd0 &&
                        context_record[assert_ctx].tid != 32'd0 &&
                        context_record[assert_ctx].domain_id != 32'd0 &&
                        context_record[assert_ctx].domain_gen != 32'd0)
                        else $fatal(1, "SG-SCHED live context missing architectural metadata");
                    assert (context_record[assert_ctx].effective_tile_mask != 32'd0)
                        else $fatal(1, "SG-SCHED live context missing effective tile mask");
                    assert (context_record[assert_ctx].migration_generation != 32'd0)
                        else $fatal(1, "SG-SCHED live context missing migration generation");
                end
                if (context_active_q[assert_ctx]) begin
                    assert (context_dispatch_eligible(context_record[assert_ctx]))
                        else $fatal(1, "SG-SCHED resident context not eligible for this tile");
                    assert (context_record[next_slot].virtual_deadline <=
                        context_record[assert_ctx].virtual_deadline)
                        else $fatal(1, "SG-SCHED barrel skipped earlier virtual deadline");
                end
            end
            if (context_active_q != '0) begin
                assert (context_active_q[next_slot] && context_dispatch_eligible(context_record[next_slot]))
                    else $fatal(1, "SG-SCHED barrel selected a non-eligible context");
            end
        end
    end
`endif
endmodule

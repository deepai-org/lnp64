`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m8_heap (
    input  logic clk,
    input  logic reset_n,
    input  logic start,
    input  logic [31:0] scenario_seed,
    output logic done,
    output logic trace_valid,
    output logic [7:0] trace_code,
    output logic [63:0] trace_value,
    output logic alloc_completed,
    output logic alloc_size_reported,
    output logic free_completed,
    output logic reuse_completed,
    output logic double_free_rejected,
    output logic stale_pointer_rejected,
    output logic cross_thread_handoff,
    output logic guard_faulted,
    output logic quarantine_observed,
    output logic heap_count_exact,
    output logic typed_commit_valid,
    output lnp64_m8_heap_commit_t typed_commit,
    output lnp64_m8_state_projection_t typed_state_projection
);
    typedef enum logic [3:0] {
        H_RESET,
        H_BOOT,
        H_ALLOC,
        H_ALLOC_SIZE,
        H_FREE,
        H_REUSE,
        H_DOUBLE_FREE,
        H_STALE_FREE,
        H_CROSS_THREAD_FREE,
        H_GUARD_FAULT,
        H_DONE
    } heap_state_e;

    localparam logic [31:0] OWNER_TID = 32'd1;
    localparam logic [31:0] FREER_TID = 32'd2;
    localparam logic [63:0] HEAP_PTR = 64'h0000_0000_0000_1000;
    localparam logic [31:0] SIZE_CLASS = 32'd32;

    heap_state_e state;
    logic [31:0] heap_generation;
    logic [31:0] pointer_generation;
    logic [31:0] stale_pointer_generation;
    logic [31:0] owner_tid;
    logic [31:0] allocations;
    logic [31:0] frees;
    logic allocated;
    logic quarantined;

    function automatic logic [31:0] seeded_root_domain(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[3:0]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_heap_generation(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[7:4]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_heap_ptr(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return HEAP_PTR[31:0];
        end
        return HEAP_PTR[31:0] + ({28'd0, seed[11:8]} << 5);
    endfunction

    function automatic logic [31:0] seeded_size_class(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return SIZE_CLASS;
        end
        return ({29'd0, seed[14:12]} + 32'd1) << 4;
    endfunction

    function automatic logic [31:0] seeded_owner_tid(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return OWNER_TID;
        end
        return {28'd0, seed[19:16]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_freer_tid(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return FREER_TID;
        end
        return seeded_owner_tid(seed) + {29'd0, seed[22:20]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_pointer_generation(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[27:24]} + 32'd1;
    endfunction

    function automatic logic [15:0] seeded_owner_tid_trace(input logic [31:0] seed);
        return seeded_owner_tid(seed);
    endfunction

    function automatic logic [15:0] seeded_freer_tid_trace(input logic [31:0] seed);
        return seeded_freer_tid(seed);
    endfunction

    function automatic logic [15:0] seeded_size_class_trace(input logic [31:0] seed);
        return seeded_size_class(seed);
    endfunction

    function automatic logic [15:0] seeded_pointer_generation_trace(input logic [31:0] seed);
        return seeded_pointer_generation(seed) + 32'd1;
    endfunction

    task automatic commit_m8(
        input lnp64_m8_heap_op_e op,
        input logic [15:0] status,
        input logic [31:0] size_class,
        input logic [63:0] heap_ptr
    );
        typed_commit_valid <= 1'b1;
        typed_commit.op <= op;
        typed_commit.status <= status;
        typed_commit.owner_tid <= owner_tid;
        typed_commit.pointer_generation <= pointer_generation;
        typed_commit.heap_generation <= heap_generation;
        typed_commit.size_class <= size_class;
        typed_commit.heap_ptr <= heap_ptr;
    endtask

    always_comb begin
        heap_count_exact = allocations == 32'd2 && frees == 32'd2;
    end

    always_comb begin
        typed_state_projection = '0;
        typed_state_projection.op = typed_commit.op;
        typed_state_projection.status = typed_commit.status;
        typed_state_projection.pointer_generation = pointer_generation;
        typed_state_projection.owner_tid = owner_tid;
        typed_state_projection.allocations = allocations;
        typed_state_projection.frees = frees;
        typed_state_projection.allocated = allocated;
        typed_state_projection.quarantined = quarantined;
        typed_state_projection.alloc_completed = alloc_completed;
        typed_state_projection.alloc_size_reported = alloc_size_reported;
        typed_state_projection.free_completed = free_completed;
        typed_state_projection.reuse_completed = reuse_completed;
        typed_state_projection.double_free_rejected = double_free_rejected;
        typed_state_projection.stale_pointer_rejected = stale_pointer_rejected;
        typed_state_projection.cross_thread_handoff = cross_thread_handoff;
        typed_state_projection.guard_faulted = guard_faulted;
        typed_state_projection.quarantine_observed = quarantine_observed;
        typed_state_projection.heap_count_exact = heap_count_exact;
    end

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            state <= H_RESET;
            done <= 1'b0;
            trace_valid <= 1'b0;
            trace_code <= 8'd0;
            trace_value <= 64'd0;
            typed_commit_valid <= 1'b0;
            typed_commit <= '0;
            alloc_completed <= 1'b0;
            alloc_size_reported <= 1'b0;
            free_completed <= 1'b0;
            reuse_completed <= 1'b0;
            double_free_rejected <= 1'b0;
            stale_pointer_rejected <= 1'b0;
            cross_thread_handoff <= 1'b0;
            guard_faulted <= 1'b0;
            quarantine_observed <= 1'b0;
            heap_generation <= 32'd1;
            pointer_generation <= 32'd1;
            stale_pointer_generation <= 32'd1;
            owner_tid <= OWNER_TID;
            allocations <= 32'd0;
            frees <= 32'd0;
            allocated <= 1'b0;
            quarantined <= 1'b0;
        end else begin
            trace_valid <= 1'b0;
            typed_commit_valid <= 1'b0;
            unique case (state)
                H_RESET: begin
                    if (start) begin
                        state <= H_BOOT;
                    end
                end
                H_BOOT: begin
                    heap_generation <= seeded_heap_generation(scenario_seed);
                    pointer_generation <= seeded_pointer_generation(scenario_seed);
                    stale_pointer_generation <= seeded_pointer_generation(scenario_seed);
                    trace_valid <= 1'b1;
                    trace_code <= 8'd1;
                    trace_value <= {seeded_root_domain(scenario_seed), seeded_heap_generation(scenario_seed)};
                    state <= H_ALLOC;
                end
                H_ALLOC: begin
                    allocated <= 1'b1;
                    quarantined <= 1'b0;
                    owner_tid <= seeded_owner_tid(scenario_seed);
                    allocations <= allocations + 32'd1;
                    alloc_completed <= 1'b1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd2;
                    trace_value <= {
                        seeded_owner_tid_trace(scenario_seed),
                        seeded_size_class_trace(scenario_seed),
                        seeded_heap_ptr(scenario_seed)
                    };
                    commit_m8(LNP64_M8_COMMIT_ALLOC, LNP64_STATUS_OK, seeded_size_class(scenario_seed), {32'd0, seeded_heap_ptr(scenario_seed)});
                    state <= H_ALLOC_SIZE;
                end
                H_ALLOC_SIZE: begin
                    if (allocated && pointer_generation == seeded_pointer_generation(scenario_seed)) begin
                        alloc_size_reported <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd3;
                        trace_value <= {seeded_heap_ptr(scenario_seed), seeded_size_class(scenario_seed)};
                        commit_m8(LNP64_M8_COMMIT_ALLOC_SIZE, LNP64_STATUS_OK, seeded_size_class(scenario_seed), {32'd0, seeded_heap_ptr(scenario_seed)});
                        state <= H_FREE;
                    end else begin
                        state <= H_DONE;
                    end
                end
                H_FREE: begin
                    if (allocated) begin
                        allocated <= 1'b0;
                        quarantined <= 1'b1;
                        pointer_generation <= pointer_generation + 32'd1;
                        frees <= frees + 32'd1;
                        free_completed <= 1'b1;
                        quarantine_observed <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd4;
                        trace_value <= {seeded_heap_ptr(scenario_seed), 16'd1, seeded_owner_tid_trace(scenario_seed)};
                        commit_m8(LNP64_M8_COMMIT_FREE, LNP64_STATUS_OK, seeded_size_class(scenario_seed), {32'd0, seeded_heap_ptr(scenario_seed)});
                        state <= H_REUSE;
                    end else begin
                        state <= H_DONE;
                    end
                end
                H_REUSE: begin
                    if (quarantined && !allocated) begin
                        allocated <= 1'b1;
                        quarantined <= 1'b0;
                        owner_tid <= seeded_owner_tid(scenario_seed);
                        allocations <= allocations + 32'd1;
                        reuse_completed <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd5;
                        trace_value <= {
                            seeded_owner_tid_trace(scenario_seed),
                            seeded_pointer_generation_trace(scenario_seed),
                            seeded_heap_ptr(scenario_seed)
                        };
                        commit_m8(LNP64_M8_COMMIT_REUSE, LNP64_STATUS_OK, seeded_size_class(scenario_seed), {32'd0, seeded_heap_ptr(scenario_seed)});
                        state <= H_DOUBLE_FREE;
                    end else begin
                        state <= H_DONE;
                    end
                end
                H_DOUBLE_FREE: begin
                    double_free_rejected <= 1'b1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd6;
                    trace_value <= {48'd0, LNP64_ERR_EINVAL};
                    commit_m8(LNP64_M8_COMMIT_DOUBLE_FREE, LNP64_ERR_EINVAL, 32'd0, 64'd0);
                    state <= H_STALE_FREE;
                end
                H_STALE_FREE: begin
                    if (stale_pointer_generation != pointer_generation) begin
                        stale_pointer_rejected <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd7;
                        trace_value <= {48'd0, LNP64_ERR_EREVOKED};
                        commit_m8(LNP64_M8_COMMIT_STALE_FREE, LNP64_ERR_EREVOKED, 32'd0, 64'd0);
                        state <= H_CROSS_THREAD_FREE;
                    end else begin
                        state <= H_DONE;
                    end
                end
                H_CROSS_THREAD_FREE: begin
                    if (allocated && owner_tid == seeded_owner_tid(scenario_seed)) begin
                        allocated <= 1'b0;
                        quarantined <= 1'b1;
                        frees <= frees + 32'd1;
                        cross_thread_handoff <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd8;
                        trace_value <= {seeded_owner_tid_trace(scenario_seed), seeded_freer_tid_trace(scenario_seed), 32'd1};
                        commit_m8(LNP64_M8_COMMIT_CROSS_THREAD_FREE, LNP64_STATUS_OK, 32'd0, 64'd0);
                        state <= H_GUARD_FAULT;
                    end else begin
                        state <= H_DONE;
                    end
                end
                H_GUARD_FAULT: begin
                    guard_faulted <= 1'b1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd9;
                    trace_value <= {48'd0, LNP64_ERR_EFAULT};
                    commit_m8(LNP64_M8_COMMIT_GUARD_FAULT, LNP64_ERR_EFAULT, 32'd0, 64'd0);
                    state <= H_DONE;
                end
                H_DONE: begin
                    if (!done) begin
                        done <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd10;
                        trace_value <= {allocations, frees};
                    end
                end
                default: state <= H_RESET;
            endcase
        end
    end
endmodule

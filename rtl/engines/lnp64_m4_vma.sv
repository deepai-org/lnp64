`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m4_vma (
    input  logic clk,
    input  logic reset_n,
    input  logic start,
    input  logic [31:0] scenario_seed,
    output logic done,
    output logic trace_valid,
    output logic [7:0] trace_code,
    output logic [63:0] trace_value,
    output logic mapping_created,
    output logic load_permitted,
    output logic store_rejected,
    output logic nx_faulted,
    output logic guard_faulted,
    output logic stale_vma_rejected,
    output logic tlb_invalidation_observed,
    output logic wx_enforced,
    output logic typed_commit_valid,
    output lnp64_m4_vma_commit_t typed_commit,
    output lnp64_m4_state_projection_t typed_state_projection
);
    typedef enum logic [3:0] {
        V_RESET,
        V_BOOT,
        V_MAP,
        V_LOAD,
        V_STORE_DENIED,
        V_EXEC_FAULT,
        V_GUARD_FAULT,
        V_STALE_ACCESS,
        V_TLB_INVALIDATE,
        V_DONE
    } vma_state_e;

    localparam logic [3:0] PERM_R = 4'h1;
    localparam logic [3:0] PERM_W = 4'h2;
    localparam logic [3:0] PERM_X = 4'h4;
    localparam logic [63:0] VMA_BASE = 64'h0000_0000_0000_4000;

    vma_state_e state;
    logic [31:0] vma_id;
    logic [31:0] vma_generation;
    logic [31:0] stale_generation;
    logic [3:0] permissions;
    logic guard_page_valid;
    logic tlb_valid;

    function automatic logic [31:0] seeded_vma_id(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[3:0]} + 32'd1;
    endfunction

    function automatic logic [15:0] seeded_vma_id_trace(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 16'd1;
        end
        return {12'd0, seed[3:0]} + 16'd1;
    endfunction

    function automatic logic [31:0] seeded_pages(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd2;
        end
        return {29'd0, seed[6:4]} + 32'd1;
    endfunction

    function automatic logic [63:0] seeded_base(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return VMA_BASE;
        end
        return VMA_BASE + ({56'd0, seed[15:8]} << 12);
    endfunction

    function automatic logic [31:0] seeded_generation(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[19:16]} + 32'd1;
    endfunction

    task automatic commit_m4(
        input lnp64_m4_vma_op_e op,
        input logic [15:0] status,
        input logic [31:0] cid,
        input logic [31:0] cgen,
        input logic [7:0] perms,
        input logic [63:0] faddr
    );
        typed_commit_valid <= 1'b1;
        typed_commit.op <= op;
        typed_commit.status <= status;
        typed_commit.vma_id <= cid;
        typed_commit.vma_generation <= cgen;
        typed_commit.permissions <= perms;
        typed_commit.fault_addr <= faddr;
    endtask

    always_comb begin
        wx_enforced = (permissions & (PERM_W | PERM_X)) != (PERM_W | PERM_X);
    end

    always_comb begin
        typed_state_projection = '0;
        typed_state_projection.op = typed_commit.op;
        typed_state_projection.status = typed_commit.status;
        typed_state_projection.vma_id = vma_id;
        typed_state_projection.vma_generation = vma_generation;
        typed_state_projection.permissions = {4'd0, permissions};
        typed_state_projection.guard_page_valid = guard_page_valid;
        typed_state_projection.tlb_valid = tlb_valid;
        typed_state_projection.mapping_created = mapping_created;
        typed_state_projection.load_permitted = load_permitted;
        typed_state_projection.store_rejected = store_rejected;
        typed_state_projection.nx_faulted = nx_faulted;
        typed_state_projection.guard_faulted = guard_faulted;
        typed_state_projection.stale_vma_rejected = stale_vma_rejected;
        typed_state_projection.tlb_invalidation_observed = tlb_invalidation_observed;
        typed_state_projection.wx_enforced = wx_enforced;
    end

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            state <= V_RESET;
            done <= 1'b0;
            trace_valid <= 1'b0;
            trace_code <= 8'd0;
            trace_value <= 64'd0;
            typed_commit_valid <= 1'b0;
            typed_commit <= '0;
            mapping_created <= 1'b0;
            load_permitted <= 1'b0;
            store_rejected <= 1'b0;
            nx_faulted <= 1'b0;
            guard_faulted <= 1'b0;
            stale_vma_rejected <= 1'b0;
            tlb_invalidation_observed <= 1'b0;
            vma_id <= 32'd0;
            vma_generation <= 32'd0;
            stale_generation <= 32'd0;
            permissions <= 4'd0;
            guard_page_valid <= 1'b0;
            tlb_valid <= 1'b0;
        end else begin
            trace_valid <= 1'b0;
            typed_commit_valid <= 1'b0;
            unique case (state)
                V_RESET: begin
                    if (start) begin
                        state <= V_BOOT;
                    end
                end
                V_BOOT: begin
                    trace_valid <= 1'b1;
                    trace_code <= 8'd1;
                    trace_value <= 64'd1;
                    state <= V_MAP;
                end
                V_MAP: begin
                    vma_id <= seeded_vma_id(scenario_seed);
                    vma_generation <= seeded_generation(scenario_seed);
                    stale_generation <= seeded_generation(scenario_seed);
                    permissions <= PERM_R | PERM_X;
                    guard_page_valid <= 1'b1;
                    tlb_valid <= 1'b1;
                    mapping_created <= 1'b1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd2;
                    trace_value <= {seeded_vma_id_trace(scenario_seed), seeded_pages(scenario_seed), 12'd0, PERM_R | PERM_X};
                    commit_m4(LNP64_M4_COMMIT_MMAP, LNP64_STATUS_OK,
                        seeded_vma_id(scenario_seed), seeded_generation(scenario_seed),
                        {4'd0, PERM_R | PERM_X}, 64'd0);
                    state <= V_LOAD;
                end
                V_LOAD: begin
                    if (tlb_valid &&
                        vma_generation == seeded_generation(scenario_seed) &&
                        (permissions & PERM_R) != 4'd0) begin
                        load_permitted <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd3;
                        trace_value <= seeded_base(scenario_seed);
                        commit_m4(LNP64_M4_COMMIT_LOAD, LNP64_STATUS_OK,
                            seeded_vma_id(scenario_seed), seeded_generation(scenario_seed),
                            {4'd0, PERM_R | PERM_X}, seeded_base(scenario_seed));
                        state <= V_STORE_DENIED;
                    end else begin
                        state <= V_DONE;
                    end
                end
                V_STORE_DENIED: begin
                    if ((permissions & PERM_W) == 4'd0 && wx_enforced) begin
                        store_rejected <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd4;
                        trace_value <= {48'd0, LNP64_ERR_EACCES};
                        commit_m4(LNP64_M4_COMMIT_STORE_DENIED, LNP64_ERR_EACCES,
                            seeded_vma_id(scenario_seed), seeded_generation(scenario_seed),
                            {4'd0, PERM_R | PERM_X}, 64'd0);
                        state <= V_EXEC_FAULT;
                    end else begin
                        state <= V_DONE;
                    end
                end
                V_EXEC_FAULT: begin
                    permissions <= PERM_R;
                    if ((PERM_R & PERM_X) == 4'd0) begin
                        nx_faulted <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd5;
                        trace_value <= {48'd0, LNP64_ERR_EFAULT};
                        commit_m4(LNP64_M4_COMMIT_EXEC_FAULT, LNP64_ERR_EFAULT,
                            seeded_vma_id(scenario_seed), seeded_generation(scenario_seed),
                            {4'd0, PERM_R}, 64'd0);
                        state <= V_GUARD_FAULT;
                    end else begin
                        state <= V_DONE;
                    end
                end
                V_GUARD_FAULT: begin
                    if (guard_page_valid) begin
                        guard_faulted <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd6;
                        trace_value <= {48'd0, LNP64_ERR_EFAULT};
                        commit_m4(LNP64_M4_COMMIT_GUARD_FAULT, LNP64_ERR_EFAULT,
                            seeded_vma_id(scenario_seed), seeded_generation(scenario_seed),
                            {4'd0, PERM_R}, 64'd0);
                        state <= V_STALE_ACCESS;
                    end else begin
                        state <= V_DONE;
                    end
                end
                V_STALE_ACCESS: begin
                    vma_generation <= vma_generation + 32'd1;
                    if (stale_generation != vma_generation + 32'd1) begin
                        stale_vma_rejected <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd7;
                        trace_value <= {48'd0, LNP64_ERR_EREVOKED};
                        commit_m4(LNP64_M4_COMMIT_STALE_REJECT, LNP64_ERR_EREVOKED,
                            seeded_vma_id(scenario_seed), seeded_generation(scenario_seed) + 32'd1,
                            {4'd0, PERM_R}, 64'd0);
                        state <= V_TLB_INVALIDATE;
                    end else begin
                        state <= V_DONE;
                    end
                end
                V_TLB_INVALIDATE: begin
                    tlb_valid <= 1'b0;
                    tlb_invalidation_observed <= 1'b1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd8;
                    trace_value <= {seeded_vma_id(scenario_seed), 32'd0};
                    commit_m4(LNP64_M4_COMMIT_TLB_INVALIDATE, LNP64_STATUS_OK,
                        seeded_vma_id(scenario_seed), seeded_generation(scenario_seed) + 32'd1,
                        {4'd0, PERM_R}, 64'd0);
                    state <= V_DONE;
                end
                V_DONE: begin
                    if (!done) begin
                        done <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd9;
                        trace_value <= {32'd1, vma_generation};
                    end
                end
                default: state <= V_RESET;
            endcase
        end
    end
endmodule

`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m5_dma (
    input  logic clk,
    input  logic reset_n,
    input  logic start,
    input  logic [31:0] scenario_seed,
    output logic done,
    output logic trace_valid,
    output logic [7:0] trace_code,
    output logic [63:0] trace_value,
    output logic pin_completed,
    output logic unpin_completed,
    output logic copy_completed,
    output logic fill_completed,
    output logic permission_faulted,
    output logic revoke_rejected,
    output logic domain_isolation_enforced,
    output logic coherence_observed,
    output logic completions_exact,
    output logic typed_commit_valid,
    output lnp64_m5_dma_commit_t typed_commit,
    output lnp64_m5_state_projection_t typed_state_projection
);
    typedef enum logic [3:0] {
        D_RESET,
        D_BOOT,
        D_PIN,
        D_COPY,
        D_FILL,
        D_UNPIN,
        D_PERMISSION_FAULT,
        D_REVOKED_SUBMIT,
        D_DOMAIN_ISOLATION,
        D_COHERENCE,
        D_DONE
    } dma_state_e;

    localparam logic [3:0] RIGHT_READ = 4'h1;
    localparam logic [3:0] RIGHT_WRITE = 4'h2;
    dma_state_e state;
    logic [31:0] src_buffer_id;
    logic [31:0] dst_buffer_id;
    logic [31:0] dst_generation;
    logic [31:0] stale_dst_generation;
    logic [31:0] requester_domain;
    logic [31:0] dst_domain;
    logic [3:0] dst_rights;
    logic dst_pinned;
    logic [31:0] completions;
    logic dst_visible;

    function automatic logic [31:0] seeded_root_domain(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[3:0]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_src_buffer(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[7:4]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_dst_buffer(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd2;
        end
        return {28'd0, seed[11:8]} + 32'd2;
    endfunction

    function automatic logic [31:0] seeded_copy_bytes(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd16;
        end
        return ({28'd0, seed[15:12]} + 32'd1) << 2;
    endfunction

    function automatic logic [15:0] seeded_fill_value(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 16'd170;
        end
        return {8'd0, seed[23:16]} + 16'd1;
    endfunction

    function automatic logic [31:0] seeded_fill_bytes(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd8;
        end
        return ({29'd0, seed[26:24]} + 32'd1) << 2;
    endfunction

    function automatic logic [31:0] seeded_isolation_domain(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd2;
        end
        return seeded_root_domain(seed) + {30'd0, seed[28:27]} + 32'd1;
    endfunction

    function automatic logic [15:0] seeded_src_buffer_trace(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 16'd1;
        end
        return {12'd0, seed[7:4]} + 16'd1;
    endfunction

    function automatic logic [15:0] seeded_dst_buffer_trace(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 16'd2;
        end
        return {12'd0, seed[11:8]} + 16'd2;
    endfunction

    task automatic commit_m5(
        input lnp64_m5_dma_op_e op,
        input logic [15:0] status,
        input logic [7:0] rights,
        input logic [31:0] gen
    );
        typed_commit_valid <= 1'b1;
        typed_commit.op <= op;
        typed_commit.status <= status;
        typed_commit.src_buffer_id <= src_buffer_id;
        typed_commit.dst_buffer_id <= dst_buffer_id;
        typed_commit.dst_generation <= gen;
        typed_commit.requester_domain <= requester_domain;
        typed_commit.dst_domain <= dst_domain;
        typed_commit.dst_rights <= rights;
    endtask

    always_comb begin
        completions_exact = completions == 32'd2;
    end

    always_comb begin
        typed_state_projection = '0;
        typed_state_projection.op = typed_commit.op;
        typed_state_projection.status = typed_commit.status;
        typed_state_projection.dst_buffer_id = dst_buffer_id;
        typed_state_projection.dst_generation = dst_generation;
        typed_state_projection.requester_domain = requester_domain;
        typed_state_projection.dst_domain = dst_domain;
        typed_state_projection.dst_rights = dst_rights;
        typed_state_projection.dst_pinned = dst_pinned;
        typed_state_projection.completions = completions;
        typed_state_projection.dst_visible = dst_visible;
        typed_state_projection.pin_completed = pin_completed;
        typed_state_projection.unpin_completed = unpin_completed;
        typed_state_projection.copy_completed = copy_completed;
        typed_state_projection.fill_completed = fill_completed;
        typed_state_projection.permission_faulted = permission_faulted;
        typed_state_projection.revoke_rejected = revoke_rejected;
        typed_state_projection.domain_isolation_enforced = domain_isolation_enforced;
        typed_state_projection.coherence_observed = coherence_observed;
        typed_state_projection.completions_exact = completions_exact;
    end

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            state <= D_RESET;
            done <= 1'b0;
            trace_valid <= 1'b0;
            trace_code <= 8'd0;
            trace_value <= 64'd0;
            typed_commit_valid <= 1'b0;
            typed_commit <= '0;
            pin_completed <= 1'b0;
            unpin_completed <= 1'b0;
            copy_completed <= 1'b0;
            fill_completed <= 1'b0;
            permission_faulted <= 1'b0;
            revoke_rejected <= 1'b0;
            domain_isolation_enforced <= 1'b0;
            coherence_observed <= 1'b0;
            src_buffer_id <= 32'd1;
            dst_buffer_id <= 32'd2;
            dst_generation <= 32'd1;
            stale_dst_generation <= 32'd1;
            requester_domain <= 32'd1;
            dst_domain <= 32'd1;
            dst_rights <= RIGHT_READ | RIGHT_WRITE;
            dst_pinned <= 1'b0;
            completions <= 32'd0;
            dst_visible <= 1'b0;
        end else begin
            trace_valid <= 1'b0;
            typed_commit_valid <= 1'b0;
            unique case (state)
                D_RESET: begin
                    if (start) begin
                        state <= D_BOOT;
                    end
                end
                D_BOOT: begin
                    requester_domain <= seeded_root_domain(scenario_seed);
                    dst_domain <= seeded_root_domain(scenario_seed);
                    src_buffer_id <= seeded_src_buffer(scenario_seed);
                    dst_buffer_id <= seeded_dst_buffer(scenario_seed);
                    dst_rights <= RIGHT_READ | RIGHT_WRITE;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd1;
                    trace_value <= {seeded_root_domain(scenario_seed), 32'd2};
                    state <= D_PIN;
                end
                D_PIN: begin
                    if (requester_domain == dst_domain && (dst_rights & RIGHT_WRITE) != 4'd0) begin
                        dst_pinned <= 1'b1;
                        pin_completed <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd2;
                        trace_value <= {seeded_dst_buffer_trace(scenario_seed), 32'd1};
                        commit_m5(LNP64_M5_COMMIT_PIN, LNP64_STATUS_OK,
                            RIGHT_READ | RIGHT_WRITE, dst_generation);
                        state <= D_COPY;
                    end else begin
                        state <= D_DONE;
                    end
                end
                D_COPY: begin
                    if (dst_pinned && requester_domain == dst_domain && (dst_rights & RIGHT_WRITE) != 4'd0) begin
                        copy_completed <= 1'b1;
                        completions <= completions + 32'd1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd3;
                        trace_value <= {seeded_src_buffer_trace(scenario_seed), seeded_dst_buffer_trace(scenario_seed), seeded_copy_bytes(scenario_seed)};
                        commit_m5(LNP64_M5_COMMIT_COPY, LNP64_STATUS_OK,
                            RIGHT_READ | RIGHT_WRITE, dst_generation);
                        state <= D_FILL;
                    end else begin
                        state <= D_DONE;
                    end
                end
                D_FILL: begin
                    if (dst_pinned && requester_domain == dst_domain && (dst_rights & RIGHT_WRITE) != 4'd0) begin
                        fill_completed <= 1'b1;
                        completions <= completions + 32'd1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd4;
                        trace_value <= {seeded_dst_buffer_trace(scenario_seed), seeded_fill_value(scenario_seed), seeded_fill_bytes(scenario_seed)};
                        commit_m5(LNP64_M5_COMMIT_FILL, LNP64_STATUS_OK,
                            RIGHT_READ | RIGHT_WRITE, dst_generation);
                        state <= D_UNPIN;
                    end else begin
                        state <= D_DONE;
                    end
                end
                D_UNPIN: begin
                    if (dst_pinned) begin
                        dst_pinned <= 1'b0;
                        unpin_completed <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd5;
                        trace_value <= {seeded_dst_buffer_trace(scenario_seed), 32'd0};
                        commit_m5(LNP64_M5_COMMIT_UNPIN, LNP64_STATUS_OK,
                            RIGHT_READ | RIGHT_WRITE, dst_generation);
                        state <= D_PERMISSION_FAULT;
                    end else begin
                        state <= D_DONE;
                    end
                end
                D_PERMISSION_FAULT: begin
                    dst_rights <= RIGHT_READ;
                    permission_faulted <= 1'b1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd6;
                    trace_value <= {48'd0, LNP64_ERR_EACCES};
                    commit_m5(LNP64_M5_COMMIT_PERMISSION_FAULT, LNP64_ERR_EACCES,
                        RIGHT_READ, dst_generation);
                    state <= D_REVOKED_SUBMIT;
                end
                D_REVOKED_SUBMIT: begin
                    dst_generation <= dst_generation + 32'd1;
                    if (stale_dst_generation != dst_generation + 32'd1) begin
                        revoke_rejected <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd7;
                        trace_value <= {48'd0, LNP64_ERR_EREVOKED};
                        commit_m5(LNP64_M5_COMMIT_REVOKED_SUBMIT, LNP64_ERR_EREVOKED,
                            RIGHT_READ, dst_generation + 32'd1);
                        state <= D_DOMAIN_ISOLATION;
                    end else begin
                        state <= D_DONE;
                    end
                end
                D_DOMAIN_ISOLATION: begin
                    dst_domain <= seeded_isolation_domain(scenario_seed);
                    if (requester_domain != seeded_isolation_domain(scenario_seed)) begin
                        domain_isolation_enforced <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd8;
                        trace_value <= {48'd0, LNP64_ERR_EPERM};
                        commit_m5(LNP64_M5_COMMIT_DOMAIN_ISOLATION, LNP64_ERR_EPERM,
                            RIGHT_READ, dst_generation);
                        state <= D_COHERENCE;
                    end else begin
                        state <= D_DONE;
                    end
                end
                D_COHERENCE: begin
                    dst_visible <= 1'b1;
                    coherence_observed <= 1'b1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd9;
                    trace_value <= {seeded_dst_buffer(scenario_seed), 32'd1};
                    commit_m5(LNP64_M5_COMMIT_COHERENCE_FLUSH, LNP64_STATUS_OK,
                        RIGHT_READ, dst_generation);
                    state <= D_DONE;
                end
                D_DONE: begin
                    if (!done) begin
                        done <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd10;
                        trace_value <= {32'd0, completions};
                    end
                end
                default: state <= D_RESET;
            endcase
        end
    end
endmodule

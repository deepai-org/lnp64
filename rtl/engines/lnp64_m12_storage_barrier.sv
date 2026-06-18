`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m12_storage_barrier (
    input  logic clk,
    input  logic reset_n,
    input  logic start,
    input  logic [31:0] scenario_seed,
    output logic done,
    output logic trace_valid,
    output logic [7:0] trace_code,
    output logic [63:0] trace_value,
    output logic boot_image_visible,
    output logic block_object_authorized,
    output logic block_write_completed,
    output logic storage_barrier_issued,
    output logic storage_barrier_quiescent,
    output logic stale_object_rejected,
    output logic cross_domain_rejected,
    output logic media_fault_terminal,
    output logic no_raw_device_authority,
    output logic counts_exact
);
    typedef enum logic [3:0] {
        S_RESET,
        S_BOOT,
        S_BOOT_IMAGE,
        S_BLOCK_WRITE,
        S_BARRIER,
        S_STALE_OBJECT,
        S_CROSS_DOMAIN,
        S_MEDIA_FAULT,
        S_RAW_AUTHORITY,
        S_DONE
    } storage_state_e;

    storage_state_e state;
    lnp64_storage_barrier_t barrier;
    logic [31:0] completions;
    logic [31:0] faults;
    logic raw_device_authority_visible;

    function automatic logic [31:0] seeded_root_domain(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[3:0]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_object_id(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {24'd0, seed[11:4]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_object_gen(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[15:12]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_barrier_id(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {24'd0, seed[23:16]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_block_index(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd0;
        end
        return {24'd0, seed[31:24]};
    endfunction

    function automatic logic [31:0] seeded_byte_len(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd512;
        end
        return ({29'd0, seed[22:20]} + 32'd1) << 6;
    endfunction

    function automatic logic [31:0] seeded_data_value(input logic [31:0] seed);
        logic [31:0] value;
        if (seed == 32'd0) begin
            return 32'h0000_5a17;
        end
        value = seed * 32'd1664525 + 32'd1013904223;
        if (value == 32'd0) begin
            return 32'd1;
        end
        return value;
    endfunction

    function automatic logic [31:0] seeded_cross_domain(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd2;
        end
        return seeded_root_domain(seed) + {29'd0, seed[30:28]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_media_status(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {31'd0, seed[31]} + 32'd1;
    endfunction

    function automatic logic [15:0] seeded_object_id_trace(input logic [31:0] seed);
        return seeded_object_id(seed);
    endfunction

    function automatic logic [15:0] seeded_object_gen_trace(input logic [31:0] seed);
        return seeded_object_gen(seed);
    endfunction

    function automatic logic [15:0] seeded_block_index_trace(input logic [31:0] seed);
        return seeded_block_index(seed);
    endfunction

    function automatic logic [15:0] seeded_byte_len_trace(input logic [31:0] seed);
        return seeded_byte_len(seed);
    endfunction

    function automatic logic [15:0] seeded_data_value_trace(input logic [31:0] seed);
        return seeded_data_value(seed);
    endfunction

    function automatic logic [15:0] seeded_stale_object_gen_trace(input logic [31:0] seed);
        return seeded_object_gen(seed) + 32'd1;
    endfunction

    function automatic logic [15:0] seeded_media_status_trace(input logic [31:0] seed);
        return seeded_media_status(seed);
    endfunction

    always_comb begin
        no_raw_device_authority = !raw_device_authority_visible;
        counts_exact = completions == 32'd3 && faults == 32'd3;
    end

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            state <= S_RESET;
            done <= 1'b0;
            trace_valid <= 1'b0;
            trace_code <= 8'd0;
            trace_value <= 64'd0;
            boot_image_visible <= 1'b0;
            block_object_authorized <= 1'b0;
            block_write_completed <= 1'b0;
            storage_barrier_issued <= 1'b0;
            storage_barrier_quiescent <= 1'b0;
            stale_object_rejected <= 1'b0;
            cross_domain_rejected <= 1'b0;
            media_fault_terminal <= 1'b0;
            completions <= 32'd0;
            faults <= 32'd0;
            raw_device_authority_visible <= 1'b0;
            barrier <= '0;
        end else begin
            trace_valid <= 1'b0;
            unique case (state)
                S_RESET: begin
                    if (start) begin
                        state <= S_BOOT;
                    end
                end
                S_BOOT: begin
                    barrier.domain_id <= seeded_root_domain(scenario_seed);
                    barrier.domain_generation <= 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd1;
                    trace_value <= {seeded_root_domain(scenario_seed), 31'd0, 1'b1};
                    state <= S_BOOT_IMAGE;
                end
                S_BOOT_IMAGE: begin
                    boot_image_visible <= 1'b1;
                    completions <= completions + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd2;
                    trace_value <= {
                        seeded_block_index(scenario_seed),
                        seeded_byte_len_trace(scenario_seed),
                        16'd1
                    };
                    state <= S_BLOCK_WRITE;
                end
                S_BLOCK_WRITE: begin
                    block_object_authorized <= 1'b1;
                    block_write_completed <= 1'b1;
                    completions <= completions + 32'd1;
                    barrier.object_id <= seeded_object_id(scenario_seed);
                    barrier.object_generation <= seeded_object_gen(scenario_seed);
                    trace_valid <= 1'b1;
                    trace_code <= 8'd3;
                    trace_value <= {
                        seeded_object_id_trace(scenario_seed),
                        seeded_object_gen_trace(scenario_seed),
                        seeded_block_index_trace(scenario_seed),
                        seeded_data_value_trace(scenario_seed)
                    };
                    state <= S_BARRIER;
                end
                S_BARRIER: begin
                    storage_barrier_issued <= 1'b1;
                    storage_barrier_quiescent <= 1'b1;
                    completions <= completions + 32'd1;
                    barrier.barrier_id <= seeded_barrier_id(scenario_seed);
                    barrier.barrier_kind <= 16'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd4;
                    trace_value <= {
                        seeded_barrier_id(scenario_seed),
                        seeded_object_id_trace(scenario_seed),
                        16'd1
                    };
                    state <= S_STALE_OBJECT;
                end
                S_STALE_OBJECT: begin
                    stale_object_rejected <= 1'b1;
                    faults <= faults + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd5;
                    trace_value <= {
                        16'd0,
                        seeded_stale_object_gen_trace(scenario_seed),
                        16'd0,
                        LNP64_ERR_EREVOKED
                    };
                    state <= S_CROSS_DOMAIN;
                end
                S_CROSS_DOMAIN: begin
                    cross_domain_rejected <= 1'b1;
                    faults <= faults + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd6;
                    trace_value <= {
                        seeded_cross_domain(scenario_seed),
                        16'd0,
                        LNP64_ERR_EPERM
                    };
                    state <= S_MEDIA_FAULT;
                end
                S_MEDIA_FAULT: begin
                    media_fault_terminal <= 1'b1;
                    faults <= faults + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd7;
                    trace_value <= {
                        32'd0,
                        seeded_media_status_trace(scenario_seed),
                        LNP64_ERR_EIO
                    };
                    state <= S_RAW_AUTHORITY;
                end
                S_RAW_AUTHORITY: begin
                    raw_device_authority_visible <= 1'b0;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd8;
                    trace_value <= 64'd0;
                    state <= S_DONE;
                end
                S_DONE: begin
                    if (!done) begin
                        done <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd9;
                        trace_value <= {
                            completions[15:0],
                            faults[15:0],
                            seeded_barrier_id(scenario_seed)
                        };
                    end
                    state <= S_DONE;
                end
                default: state <= S_RESET;
            endcase
        end
    end
endmodule

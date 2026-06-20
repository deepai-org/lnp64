`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m11_ddr_metadata (
    input  logic clk,
    input  logic reset_n,
    input  logic start,
    input  logic [31:0] scenario_seed,
    output logic done,
    output logic trace_valid,
    output logic [7:0] trace_code,
    output logic [63:0] trace_value,
    output logic metadata_allocated,
    output logic metadata_domain_bound,
    output logic ddr_write_completed,
    output logic ddr_read_completed,
    output logic read_matches_write,
    output logic stale_generation_rejected,
    output logic cross_domain_rejected,
    output logic ecc_scrubbed,
    output logic barrier_quiescent,
    output logic counts_exact,
    output logic typed_commit_valid,
    output lnp64_m11_ddr_commit_t typed_commit,
    output lnp64_m11_state_projection_t typed_state_projection
);
    typedef enum logic [3:0] {
        D_RESET,
        D_BOOT,
        D_METADATA_ALLOC,
        D_DDR_WRITE,
        D_DDR_READ,
        D_STALE_SUBMIT,
        D_CROSS_DOMAIN,
        D_ECC_SCRUB,
        D_BARRIER,
        D_DONE
    } ddr_state_e;

    ddr_state_e state;
    lnp64_ddr_line_t ddr_line;
    lnp64_metadata_entry_t metadata_entry;
    logic [31:0] completions;
    logic [31:0] faults;

    function automatic logic [31:0] seeded_root_domain(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[3:0]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_line_id(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {24'd0, seed[11:4]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_line_gen(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[15:12]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_metadata_epoch(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[19:16]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_byte_len(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd64;
        end
        return ({29'd0, seed[22:20]} + 32'd1) << 3;
    endfunction

    function automatic logic [31:0] seeded_data_value(input logic [31:0] seed);
        logic [31:0] value;
        if (seed == 32'd0) begin
            return 32'h0000_1234;
        end
        value = seed * 32'd1103515245 + 32'd12345;
        if (value == 32'd0) begin
            return 32'd1;
        end
        return value;
    endfunction

    function automatic logic [31:0] seeded_cross_domain(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd2;
        end
        return seeded_root_domain(seed) + {29'd0, seed[25:23]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_ecc_corrections(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {29'd0, seed[28:26]} + 32'd1;
    endfunction

    function automatic logic [15:0] seeded_line_id_trace(input logic [31:0] seed);
        return seeded_line_id(seed);
    endfunction

    function automatic logic [15:0] seeded_root_domain_trace(input logic [31:0] seed);
        return seeded_root_domain(seed);
    endfunction

    function automatic logic [15:0] seeded_line_gen_trace(input logic [31:0] seed);
        return seeded_line_gen(seed);
    endfunction

    function automatic logic [15:0] seeded_metadata_epoch_trace(input logic [31:0] seed);
        return seeded_metadata_epoch(seed);
    endfunction

    function automatic logic [15:0] seeded_byte_len_trace(input logic [31:0] seed);
        return seeded_byte_len(seed);
    endfunction

    function automatic logic [15:0] seeded_stale_gen_trace(input logic [31:0] seed);
        return seeded_line_gen(seed) + 32'd1;
    endfunction

    function automatic logic [15:0] seeded_ecc_corrections_trace(input logic [31:0] seed);
        return seeded_ecc_corrections(seed);
    endfunction

    task automatic commit_m11(
        input lnp64_m11_ddr_op_e op,
        input logic [15:0] status,
        input logic [31:0] data_value
    );
        typed_commit_valid <= 1'b1;
        typed_commit.op <= op;
        typed_commit.status <= status;
        typed_commit.line_id <= seeded_line_id(scenario_seed);
        typed_commit.line_generation <= seeded_line_gen(scenario_seed);
        typed_commit.domain_id <= seeded_root_domain(scenario_seed);
        typed_commit.metadata_epoch <= seeded_metadata_epoch(scenario_seed);
        typed_commit.byte_len <= seeded_byte_len(scenario_seed);
        typed_commit.data_value <= data_value;
    endtask

    always_comb begin
        counts_exact = completions == 32'd2 && faults == 32'd3;
    end

    always_comb begin
        typed_state_projection = '0;
        typed_state_projection.op = typed_commit.op;
        typed_state_projection.status = typed_commit.status;
        typed_state_projection.completions = completions;
        typed_state_projection.faults = faults;
        typed_state_projection.metadata_allocated = metadata_allocated;
        typed_state_projection.metadata_domain_bound = metadata_domain_bound;
        typed_state_projection.ddr_write_completed = ddr_write_completed;
        typed_state_projection.ddr_read_completed = ddr_read_completed;
        typed_state_projection.read_matches_write = read_matches_write;
        typed_state_projection.stale_generation_rejected = stale_generation_rejected;
        typed_state_projection.cross_domain_rejected = cross_domain_rejected;
        typed_state_projection.ecc_scrubbed = ecc_scrubbed;
        typed_state_projection.barrier_quiescent = barrier_quiescent;
        typed_state_projection.counts_exact = counts_exact;
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
            metadata_allocated <= 1'b0;
            metadata_domain_bound <= 1'b0;
            ddr_write_completed <= 1'b0;
            ddr_read_completed <= 1'b0;
            read_matches_write <= 1'b0;
            stale_generation_rejected <= 1'b0;
            cross_domain_rejected <= 1'b0;
            ecc_scrubbed <= 1'b0;
            barrier_quiescent <= 1'b0;
            completions <= 32'd0;
            faults <= 32'd0;
            ddr_line <= '0;
            metadata_entry <= '0;
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
                    metadata_entry.domain_id <= seeded_root_domain(scenario_seed);
                    metadata_entry.domain_generation <= 32'd1;
                    metadata_entry.metadata_epoch <= seeded_metadata_epoch(scenario_seed);
                    trace_valid <= 1'b1;
                    trace_code <= 8'd1;
                    trace_value <= {
                        seeded_root_domain(scenario_seed),
                        16'd1,
                        seeded_metadata_epoch_trace(scenario_seed)
                    };
                    state <= D_METADATA_ALLOC;
                end
                D_METADATA_ALLOC: begin
                    metadata_allocated <= 1'b1;
                    metadata_domain_bound <= 1'b1;
                    metadata_entry.entry_id <= 32'd1;
                    metadata_entry.line_id <= seeded_line_id(scenario_seed);
                    metadata_entry.line_generation <= seeded_line_gen(scenario_seed);
                    metadata_entry.rights_mask <= 64'h3;
                    metadata_entry.integrity_state <= 16'd0;
                    ddr_line.line_id <= seeded_line_id(scenario_seed);
                    ddr_line.line_generation <= seeded_line_gen(scenario_seed);
                    ddr_line.domain_id <= seeded_root_domain(scenario_seed);
                    ddr_line.domain_generation <= 32'd1;
                    ddr_line.byte_address <= {32'd0, seeded_line_id(scenario_seed)} << 6;
                    ddr_line.byte_len <= {32'd0, seeded_byte_len(scenario_seed)};
                    trace_valid <= 1'b1;
                    trace_code <= 8'd2;
                    trace_value <= {
                        seeded_line_id_trace(scenario_seed),
                        seeded_line_gen_trace(scenario_seed),
                        seeded_root_domain_trace(scenario_seed),
                        seeded_metadata_epoch_trace(scenario_seed)
                    };
                    commit_m11(LNP64_M11_COMMIT_METADATA_ALLOC, LNP64_STATUS_OK, 32'd0);
                    state <= D_DDR_WRITE;
                end
                D_DDR_WRITE: begin
                    ddr_write_completed <= 1'b1;
                    completions <= completions + 32'd1;
                    ddr_line.data_value <= {32'd0, seeded_data_value(scenario_seed)};
                    trace_valid <= 1'b1;
                    trace_code <= 8'd3;
                    trace_value <= {
                        seeded_line_id_trace(scenario_seed),
                        seeded_byte_len_trace(scenario_seed),
                        seeded_data_value(scenario_seed)
                    };
                    commit_m11(LNP64_M11_COMMIT_DDR_WRITE, LNP64_STATUS_OK, seeded_data_value(scenario_seed));
                    state <= D_DDR_READ;
                end
                D_DDR_READ: begin
                    ddr_read_completed <= 1'b1;
                    read_matches_write <= ddr_line.data_value[31:0] == seeded_data_value(scenario_seed);
                    completions <= completions + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd4;
                    trace_value <= {
                        seeded_line_id_trace(scenario_seed),
                        seeded_data_value(scenario_seed),
                        16'd1
                    };
                    commit_m11(LNP64_M11_COMMIT_DDR_READ, LNP64_STATUS_OK, seeded_data_value(scenario_seed));
                    state <= D_STALE_SUBMIT;
                end
                D_STALE_SUBMIT: begin
                    stale_generation_rejected <= 1'b1;
                    faults <= faults + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd5;
                    trace_value <= {
                        16'd0,
                        seeded_stale_gen_trace(scenario_seed),
                        16'd0,
                        LNP64_ERR_EREVOKED
                    };
                    commit_m11(LNP64_M11_COMMIT_STALE_SUBMIT, LNP64_ERR_EREVOKED, 32'd0);
                    state <= D_CROSS_DOMAIN;
                end
                D_CROSS_DOMAIN: begin
                    cross_domain_rejected <= 1'b1;
                    faults <= faults + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd6;
                    trace_value <= {
                        seeded_cross_domain(scenario_seed),
                        16'd0,
                        LNP64_ERR_EPERM
                    };
                    commit_m11(LNP64_M11_COMMIT_CROSS_DOMAIN, LNP64_ERR_EPERM, 32'd0);
                    state <= D_ECC_SCRUB;
                end
                D_ECC_SCRUB: begin
                    ecc_scrubbed <= 1'b1;
                    faults <= faults + 32'd1;
                    metadata_entry.integrity_state <= 16'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd7;
                    trace_value <= {
                        32'd0,
                        seeded_ecc_corrections_trace(scenario_seed),
                        LNP64_ERR_EIO
                    };
                    commit_m11(LNP64_M11_COMMIT_ECC_SCRUB, LNP64_ERR_EIO, 32'd0);
                    state <= D_BARRIER;
                end
                D_BARRIER: begin
                    barrier_quiescent <= 1'b1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd8;
                    trace_value <= {seeded_line_id(scenario_seed), 32'd1};
                    commit_m11(LNP64_M11_COMMIT_BARRIER, LNP64_STATUS_OK, 32'd0);
                    state <= D_DONE;
                end
                D_DONE: begin
                    if (!done) begin
                        done <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd9;
                        trace_value <= {
                            completions[15:0],
                            faults[15:0],
                            seeded_metadata_epoch(scenario_seed)
                        };
                    end
                    state <= D_DONE;
                end
                default: state <= D_RESET;
            endcase
        end
    end
endmodule

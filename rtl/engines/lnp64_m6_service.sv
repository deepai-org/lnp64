`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m6_service (
    input  logic clk,
    input  logic reset_n,
    input  logic start,
    input  logic [31:0] scenario_seed,
    output logic done,
    output logic trace_valid,
    output logic [7:0] trace_code,
    output logic [63:0] trace_value,
    output logic envelope_validated,
    output logic namespace_dispatched,
    output logic service_continuation_created,
    output logic cap_return_installed,
    output logic returned_cap_narrowed,
    output logic cancel_terminal,
    output logic stale_service_rejected,
    output logic crash_completed
);
    typedef enum logic [3:0] {
        S_RESET,
        S_BOOT,
        S_ENVELOPE,
        S_NS_DISPATCH,
        S_SERVICE_REQUEST,
        S_CAP_RETURN,
        S_SERVICE_CANCEL,
        S_STALE_SERVICE,
        S_CRASH_COMPLETION,
        S_DONE
    } service_state_e;

    localparam logic [15:0] PROFILE_NAMESPACE = 16'd1;
    localparam logic [15:0] SELECTOR_OPEN = 16'd3;
    localparam logic [63:0] RIGHT_READ = 64'h0000_0000_0000_0001;
    localparam logic [63:0] RIGHT_WRITE = 64'h0000_0000_0000_0002;

    service_state_e state;
    logic [31:0] service_generation;
    logic [31:0] stale_service_generation;
    logic [31:0] continuation_generation;
    logic [31:0] installed_caps;
    logic [31:0] completions;
    logic [63:0] requested_rights;
    logic [63:0] returned_rights;

    function automatic logic [31:0] seeded_root_domain(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[3:0]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_namespace_root(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[7:4]} + 32'd1;
    endfunction

    function automatic logic [15:0] seeded_path_len(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 16'd8;
        end
        return {10'd0, seed[13:8]} + 16'd1;
    endfunction

    function automatic logic [31:0] seeded_service_id(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {26'd0, seed[19:14]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_op_id(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[23:20]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_continuation(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[27:24]} + 32'd1;
    endfunction

    function automatic logic [63:0] seeded_returned_rights(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return RIGHT_READ;
        end
        if (seed[28]) begin
            return RIGHT_WRITE;
        end
        return RIGHT_READ;
    endfunction

    function automatic logic [31:0] seeded_returned_rights_trace(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return RIGHT_READ[31:0];
        end
        if (seed[28]) begin
            return RIGHT_WRITE[31:0];
        end
        return RIGHT_READ[31:0];
    endfunction

    function automatic logic [31:0] seeded_cap_object(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd9;
        end
        return seeded_service_id(seed) + {29'd0, seed[31:29]} + 32'd8;
    endfunction

    function automatic logic [31:0] seeded_cancel_continuation(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd2;
        end
        return seeded_continuation(seed) + 32'd1;
    endfunction

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            state <= S_RESET;
            done <= 1'b0;
            trace_valid <= 1'b0;
            trace_code <= 8'd0;
            trace_value <= 64'd0;
            envelope_validated <= 1'b0;
            namespace_dispatched <= 1'b0;
            service_continuation_created <= 1'b0;
            cap_return_installed <= 1'b0;
            returned_cap_narrowed <= 1'b0;
            cancel_terminal <= 1'b0;
            stale_service_rejected <= 1'b0;
            crash_completed <= 1'b0;
            service_generation <= 32'd1;
            stale_service_generation <= 32'd1;
            continuation_generation <= 32'd0;
            installed_caps <= 32'd0;
            completions <= 32'd0;
            requested_rights <= 64'd0;
            returned_rights <= 64'd0;
        end else begin
            trace_valid <= 1'b0;
            unique case (state)
                S_RESET: begin
                    if (start) begin
                        state <= S_BOOT;
                    end
                end
                S_BOOT: begin
                    trace_valid <= 1'b1;
                    trace_code <= 8'd1;
                    trace_value <= {seeded_root_domain(scenario_seed), seeded_namespace_root(scenario_seed)};
                    state <= S_ENVELOPE;
                end
                S_ENVELOPE: begin
                    envelope_validated <= 1'b1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd2;
                    trace_value <= {16'd1, PROFILE_NAMESPACE, 32'd1};
                    state <= S_NS_DISPATCH;
                end
                S_NS_DISPATCH: begin
                    namespace_dispatched <= 1'b1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd3;
                    trace_value <= {SELECTOR_OPEN, seeded_path_len(scenario_seed), seeded_service_id(scenario_seed)};
                    state <= S_SERVICE_REQUEST;
                end
                S_SERVICE_REQUEST: begin
                    continuation_generation <= seeded_continuation(scenario_seed);
                    service_continuation_created <= 1'b1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd4;
                    trace_value <= {seeded_op_id(scenario_seed), seeded_continuation(scenario_seed)};
                    state <= S_CAP_RETURN;
                end
                S_CAP_RETURN: begin
                    requested_rights <= RIGHT_READ | RIGHT_WRITE;
                    returned_rights <= seeded_returned_rights(scenario_seed);
                    installed_caps <= installed_caps + 32'd1;
                    cap_return_installed <= 1'b1;
                    returned_cap_narrowed <=
                        ((seeded_returned_rights(scenario_seed) & ~(RIGHT_READ | RIGHT_WRITE)) == 64'd0)
                        && (seeded_returned_rights(scenario_seed) != (RIGHT_READ | RIGHT_WRITE));
                    trace_valid <= 1'b1;
                    trace_code <= 8'd5;
                    trace_value <= {seeded_cap_object(scenario_seed), seeded_returned_rights_trace(scenario_seed)};
                    state <= S_SERVICE_CANCEL;
                end
                S_SERVICE_CANCEL: begin
                    cancel_terminal <= 1'b1;
                    completions <= completions + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd6;
                    trace_value <= {seeded_cancel_continuation(scenario_seed), 16'd0, LNP64_ERR_ECANCELED};
                    state <= S_STALE_SERVICE;
                end
                S_STALE_SERVICE: begin
                    service_generation <= service_generation + 32'd1;
                    if (stale_service_generation != service_generation + 32'd1) begin
                        stale_service_rejected <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd7;
                        trace_value <= {48'd0, LNP64_ERR_EREVOKED};
                        state <= S_CRASH_COMPLETION;
                    end else begin
                        state <= S_DONE;
                    end
                end
                S_CRASH_COMPLETION: begin
                    crash_completed <= 1'b1;
                    completions <= completions + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd8;
                    trace_value <= {48'd0, LNP64_ERR_EIO};
                    state <= S_DONE;
                end
                S_DONE: begin
                    if (!done) begin
                        done <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd9;
                        trace_value <= {installed_caps, completions};
                    end
                end
                default: state <= S_RESET;
            endcase
        end
    end
endmodule

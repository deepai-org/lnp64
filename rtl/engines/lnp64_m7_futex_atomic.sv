`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m7_futex_atomic (
    input  logic clk,
    input  logic reset_n,
    input  logic start,
    input  logic [31:0] scenario_seed,
    output logic done,
    output logic trace_valid,
    output logic [7:0] trace_code,
    output logic [63:0] trace_value,
    output logic cmpxchg_success,
    output logic cmpxchg_failure_explicit,
    output logic futex_wait_parked,
    output logic futex_wake_delivered,
    output logic timer_wait_parked,
    output logic timer_expired,
    output logic bucket_spill_preserved,
    output logic stale_address_rejected,
    output logic no_lost_wakeup,
    output logic atomic_count_exact
);
    typedef enum logic [3:0] {
        A_RESET,
        A_BOOT,
        A_CMPXCHG_SUCCESS,
        A_CMPXCHG_FAIL,
        A_FUTEX_WAIT,
        A_FUTEX_WAKE,
        A_TIMER_WAIT,
        A_TIMER_EXPIRE,
        A_BUCKET_SPILL,
        A_STALE_ADDRESS,
        A_DONE
    } atomic_state_e;

    localparam logic [63:0] FUTEX_ADDR = 64'h0000_0000_0000_1000;

    atomic_state_e state;
    logic [31:0] atomic_word;
    logic [31:0] expected_value;
    logic [31:0] desired_value;
    logic [31:0] wait_generation;
    logic [31:0] address_generation;
    logic [31:0] stale_address_generation;
    logic waiter_parked;
    logic [31:0] wake_count;
    logic [31:0] timer_deadline;
    logic [31:0] timer_wake_count;
    logic [31:0] atomic_count;

    function automatic logic [31:0] seeded_root_domain(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[3:0]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_initial_atomic(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd0;
        end
        return {24'd0, seed[11:4]};
    endfunction

    function automatic logic [31:0] seeded_success_desired(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return seeded_initial_atomic(seed) + {28'd0, seed[15:12]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_fail_expected(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd0;
        end
        return seeded_success_desired(seed) + {28'd0, seed[19:16]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_fail_desired(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd2;
        end
        return seeded_success_desired(seed) + {28'd0, seed[23:20]} + 32'd2;
    endfunction

    function automatic logic [63:0] seeded_futex_addr(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return FUTEX_ADDR;
        end
        return FUTEX_ADDR + ({56'd0, seed[27:24]} << 3);
    endfunction

    function automatic logic [15:0] seeded_initial_atomic_trace(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 16'd0;
        end
        return {8'd0, seed[11:4]};
    endfunction

    function automatic logic [15:0] seeded_success_desired_trace(input logic [31:0] seed);
        return seeded_success_desired(seed);
    endfunction

    function automatic logic [15:0] seeded_fail_expected_trace(input logic [31:0] seed);
        return seeded_fail_expected(seed);
    endfunction

    function automatic logic [15:0] seeded_fail_desired_trace(input logic [31:0] seed);
        return seeded_fail_desired(seed);
    endfunction

    function automatic logic [31:0] seeded_futex_addr_trace(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return FUTEX_ADDR[31:0];
        end
        return FUTEX_ADDR[31:0] + ({24'd0, seed[27:24]} << 3);
    endfunction

    function automatic logic [31:0] seeded_bucket_id(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[31:28]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_timer_deadline(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd3;
        end
        return {28'd0, seed[7:4]} + 32'd3;
    endfunction

    always_comb begin
        no_lost_wakeup = (wake_count + timer_wake_count) == 32'd0 || !waiter_parked;
        atomic_count_exact = atomic_count == 32'd2;
    end

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            state <= A_RESET;
            done <= 1'b0;
            trace_valid <= 1'b0;
            trace_code <= 8'd0;
            trace_value <= 64'd0;
            cmpxchg_success <= 1'b0;
            cmpxchg_failure_explicit <= 1'b0;
            futex_wait_parked <= 1'b0;
            futex_wake_delivered <= 1'b0;
            timer_wait_parked <= 1'b0;
            timer_expired <= 1'b0;
            bucket_spill_preserved <= 1'b0;
            stale_address_rejected <= 1'b0;
            atomic_word <= 32'd0;
            expected_value <= 32'd0;
            desired_value <= 32'd0;
            wait_generation <= 32'd1;
            address_generation <= 32'd1;
            stale_address_generation <= 32'd1;
            waiter_parked <= 1'b0;
            wake_count <= 32'd0;
            timer_deadline <= 32'd0;
            timer_wake_count <= 32'd0;
            atomic_count <= 32'd0;
        end else begin
            trace_valid <= 1'b0;
            unique case (state)
                A_RESET: begin
                    if (start) begin
                        state <= A_BOOT;
                    end
                end
                A_BOOT: begin
                    atomic_word <= seeded_initial_atomic(scenario_seed);
                    trace_valid <= 1'b1;
                    trace_code <= 8'd1;
                    trace_value <= {seeded_root_domain(scenario_seed), seeded_initial_atomic(scenario_seed)};
                    state <= A_CMPXCHG_SUCCESS;
                end
                A_CMPXCHG_SUCCESS: begin
                    expected_value <= seeded_initial_atomic(scenario_seed);
                    desired_value <= seeded_success_desired(scenario_seed);
                    if (atomic_word == seeded_initial_atomic(scenario_seed)) begin
                        atomic_word <= seeded_success_desired(scenario_seed);
                        atomic_count <= atomic_count + 32'd1;
                        cmpxchg_success <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd2;
                        trace_value <= {
                            seeded_initial_atomic_trace(scenario_seed),
                            seeded_success_desired_trace(scenario_seed),
                            seeded_initial_atomic(scenario_seed)
                        };
                        state <= A_CMPXCHG_FAIL;
                    end else begin
                        state <= A_DONE;
                    end
                end
                A_CMPXCHG_FAIL: begin
                    expected_value <= seeded_fail_expected(scenario_seed);
                    desired_value <= seeded_fail_desired(scenario_seed);
                    atomic_count <= atomic_count + 32'd1;
                    cmpxchg_failure_explicit <= 1'b1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd3;
                    trace_value <= {
                        seeded_fail_expected_trace(scenario_seed),
                        seeded_fail_desired_trace(scenario_seed),
                        atomic_word[15:0],
                        LNP64_ERR_EAGAIN
                    };
                    state <= A_FUTEX_WAIT;
                end
                A_FUTEX_WAIT: begin
                    if (atomic_word == seeded_success_desired(scenario_seed)) begin
                        waiter_parked <= 1'b1;
                        futex_wait_parked <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd4;
                        trace_value <= {seeded_futex_addr_trace(scenario_seed), seeded_success_desired(scenario_seed)};
                        state <= A_FUTEX_WAKE;
                    end else begin
                        state <= A_DONE;
                    end
                end
                A_FUTEX_WAKE: begin
                    if (waiter_parked && wait_generation == 32'd1) begin
                        waiter_parked <= 1'b0;
                        wake_count <= wake_count + 32'd1;
                        futex_wake_delivered <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd5;
                        trace_value <= {seeded_futex_addr_trace(scenario_seed), 32'd1};
                        state <= A_TIMER_WAIT;
                    end else begin
                        state <= A_DONE;
                    end
                end
                A_TIMER_WAIT: begin
                    timer_deadline <= seeded_timer_deadline(scenario_seed);
                    waiter_parked <= 1'b1;
                    timer_wait_parked <= 1'b1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd6;
                    trace_value <= {seeded_timer_deadline(scenario_seed), 32'd1};
                    state <= A_TIMER_EXPIRE;
                end
                A_TIMER_EXPIRE: begin
                    if (waiter_parked && timer_deadline == seeded_timer_deadline(scenario_seed)) begin
                        waiter_parked <= 1'b0;
                        timer_wake_count <= timer_wake_count + 32'd1;
                        timer_expired <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd7;
                        trace_value <= {seeded_timer_deadline(scenario_seed), 32'd1};
                        state <= A_BUCKET_SPILL;
                    end else begin
                        state <= A_DONE;
                    end
                end
                A_BUCKET_SPILL: begin
                    bucket_spill_preserved <= 1'b1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd8;
                    trace_value <= {seeded_bucket_id(scenario_seed), 32'd1};
                    state <= A_STALE_ADDRESS;
                end
                A_STALE_ADDRESS: begin
                    address_generation <= address_generation + 32'd1;
                    if (stale_address_generation != address_generation + 32'd1) begin
                        stale_address_rejected <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd9;
                        trace_value <= {48'd0, LNP64_ERR_EREVOKED};
                        state <= A_DONE;
                    end else begin
                        state <= A_DONE;
                    end
                end
                A_DONE: begin
                    if (!done) begin
                        done <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd10;
                        trace_value <= {wake_count + timer_wake_count, atomic_count};
                    end
                end
                default: state <= A_RESET;
            endcase
        end
    end
endmodule

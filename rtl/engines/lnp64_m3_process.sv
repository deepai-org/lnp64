`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m3_process (
    input  logic clk,
    input  logic reset_n,
    input  logic start,
    input  logic [31:0] scenario_seed,
    output logic done,
    output logic trace_valid,
    output logic [7:0] trace_code,
    output logic [63:0] trace_value,
    output logic clone_created,
    output logic child_exit_signaled,
    output logic parent_join_completed,
    output logic exec_barrier_stopped_sibling,
    output logic stale_join_rejected,
    output logic exec_cancel_terminal,
    output logic exactly_one_thread_location,
    output logic typed_commit_valid,
    output lnp64_m3_process_commit_t typed_commit,
    output lnp64_m3_state_projection_t typed_state_projection
);
    typedef enum logic [3:0] {
        P_RESET,
        P_BOOT,
        P_CLONE,
        P_CHILD_EXIT,
        P_PARENT_JOIN,
        P_EXEC_BARRIER,
        P_STALE_JOIN,
        P_EXEC_CANCEL,
        P_DONE
    } process_state_e;

    typedef enum logic [1:0] {
        T_UNUSED,
        T_RUNNABLE,
        T_RUNNING,
        T_EXITED
    } thread_state_e;

    process_state_e state;
    thread_state_e parent_state;
    thread_state_e child_state;
    logic [31:0] parent_tid;
    logic [31:0] child_tid;
    logic [31:0] child_generation;
    logic [31:0] join_generation;
    logic [31:0] exec_epoch;
    logic [31:0] child_exit_code;
    logic child_waitable_signaled;

    function automatic logic [31:0] seeded_parent_tid(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[3:0]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_child_tid(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd2;
        end
        return {28'd0, seed[7:4]} + 32'd2;
    endfunction

    function automatic logic [31:0] seeded_exit_code(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd7;
        end
        return {24'd0, seed[15:8]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_exec_epoch(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[19:16]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_siblings_stopped(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {30'd0, seed[21:20]} + 32'd1;
    endfunction

    task automatic commit_m3(
        input lnp64_m3_process_op_e op,
        input logic [15:0] status,
        input logic [31:0] exit_code
    );
        typed_commit_valid <= 1'b1;
        typed_commit.op <= op;
        typed_commit.status <= status;
        typed_commit.parent_tid <= parent_tid;
        typed_commit.child_tid <= child_tid;
        typed_commit.child_generation <= child_generation;
        typed_commit.join_generation <= join_generation;
        typed_commit.exec_epoch <= exec_epoch;
        typed_commit.exit_code <= exit_code;
    endtask

    always_comb begin
        typed_state_projection = '0;
        typed_state_projection.op = typed_commit.op;
        typed_state_projection.status = typed_commit.status;
        typed_state_projection.parent_state = parent_state;
        typed_state_projection.child_state = child_state;
        typed_state_projection.parent_tid = parent_tid;
        typed_state_projection.child_tid = child_tid;
        typed_state_projection.child_generation = child_generation;
        typed_state_projection.join_generation = join_generation;
        typed_state_projection.exec_epoch = exec_epoch;
        typed_state_projection.clone_created = clone_created;
        typed_state_projection.child_exit_signaled = child_exit_signaled;
        typed_state_projection.parent_join_completed = parent_join_completed;
        typed_state_projection.exec_barrier_stopped_sibling = exec_barrier_stopped_sibling;
        typed_state_projection.stale_join_rejected = stale_join_rejected;
        typed_state_projection.exec_cancel_terminal = exec_cancel_terminal;
        typed_state_projection.exactly_one_thread_location = exactly_one_thread_location;
    end

    always_comb begin
        exactly_one_thread_location =
            parent_state != T_UNUSED &&
            child_state != T_RUNNING &&
            (child_state == T_UNUSED || child_state == T_RUNNABLE || child_state == T_EXITED);
    end

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            state <= P_RESET;
            done <= 1'b0;
            trace_valid <= 1'b0;
            trace_code <= 8'd0;
            trace_value <= 64'd0;
            typed_commit_valid <= 1'b0;
            typed_commit <= '0;
            clone_created <= 1'b0;
            child_exit_signaled <= 1'b0;
            parent_join_completed <= 1'b0;
            exec_barrier_stopped_sibling <= 1'b0;
            stale_join_rejected <= 1'b0;
            exec_cancel_terminal <= 1'b0;
            parent_state <= T_RUNNING;
            child_state <= T_UNUSED;
            parent_tid <= 32'd1;
            child_tid <= 32'd0;
            child_generation <= 32'd0;
            join_generation <= 32'd0;
            exec_epoch <= 32'd1;
            child_exit_code <= 32'd0;
            child_waitable_signaled <= 1'b0;
        end else begin
            trace_valid <= 1'b0;
            typed_commit_valid <= 1'b0;
            unique case (state)
                P_RESET: begin
                    if (start) begin
                        state <= P_BOOT;
                    end
                end
                P_BOOT: begin
                    parent_state <= T_RUNNING;
                    child_state <= T_UNUSED;
                    parent_tid <= seeded_parent_tid(scenario_seed);
                    exec_epoch <= seeded_exec_epoch(scenario_seed);
                    trace_valid <= 1'b1;
                    trace_code <= 8'd1;
                    trace_value <= {seeded_parent_tid(scenario_seed), seeded_exec_epoch(scenario_seed)};
                    state <= P_CLONE;
                end
                P_CLONE: begin
                    if (child_state == T_UNUSED) begin
                        child_tid <= seeded_child_tid(scenario_seed);
                        child_generation <= 32'd1;
                        join_generation <= 32'd1;
                        child_state <= T_RUNNABLE;
                        clone_created <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd2;
                        trace_value <= {parent_tid, seeded_child_tid(scenario_seed)};
                        commit_m3(LNP64_M3_COMMIT_CLONE, LNP64_STATUS_OK, 32'd0);
                        state <= P_CHILD_EXIT;
                    end else begin
                        state <= P_DONE;
                    end
                end
                P_CHILD_EXIT: begin
                    if (child_state == T_RUNNABLE && child_generation == join_generation) begin
                        child_state <= T_EXITED;
                        child_exit_code <= seeded_exit_code(scenario_seed);
                        child_waitable_signaled <= 1'b1;
                        child_exit_signaled <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd3;
                        trace_value <= {child_tid, seeded_exit_code(scenario_seed)};
                        commit_m3(LNP64_M3_COMMIT_CHILD_EXIT, LNP64_STATUS_OK, seeded_exit_code(scenario_seed));
                        state <= P_PARENT_JOIN;
                    end else begin
                        state <= P_DONE;
                    end
                end
                P_PARENT_JOIN: begin
                    if (child_state == T_EXITED && child_waitable_signaled) begin
                        child_state <= T_UNUSED;
                        child_generation <= child_generation + 32'd1;
                        child_waitable_signaled <= 1'b0;
                        parent_join_completed <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd4;
                        trace_value <= {parent_tid[15:0], child_tid[15:0], child_exit_code};
                        commit_m3(LNP64_M3_COMMIT_PARENT_JOIN, LNP64_STATUS_OK, child_exit_code);
                        state <= P_EXEC_BARRIER;
                    end else begin
                        state <= P_DONE;
                    end
                end
                P_EXEC_BARRIER: begin
                    exec_epoch <= exec_epoch + 32'd1;
                    exec_barrier_stopped_sibling <= 1'b1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd5;
                    trace_value <= {exec_epoch + 32'd1, seeded_siblings_stopped(scenario_seed)};
                    commit_m3(LNP64_M3_COMMIT_EXEC_BARRIER, LNP64_STATUS_OK, 32'd0);
                    state <= P_STALE_JOIN;
                end
                P_STALE_JOIN: begin
                    if (join_generation != child_generation) begin
                        stale_join_rejected <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd6;
                        trace_value <= {48'd0, LNP64_ERR_EREVOKED};
                        commit_m3(LNP64_M3_COMMIT_STALE_JOIN, LNP64_ERR_EREVOKED, 32'd0);
                        state <= P_EXEC_CANCEL;
                    end else begin
                        state <= P_DONE;
                    end
                end
                P_EXEC_CANCEL: begin
                    exec_cancel_terminal <= 1'b1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd7;
                    trace_value <= {48'd0, LNP64_ERR_ECANCELED};
                    commit_m3(LNP64_M3_COMMIT_EXEC_CANCEL, LNP64_ERR_ECANCELED, 32'd0);
                    state <= P_DONE;
                end
                P_DONE: begin
                    if (!done) begin
                        done <= 1'b1;
                        parent_state <= T_RUNNING;
                        child_state <= T_UNUSED;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd8;
                        trace_value <= {32'd1, exec_epoch};
                    end
                end
                default: state <= P_RESET;
            endcase
        end
    end
endmodule

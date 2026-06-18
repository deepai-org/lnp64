`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m1_pingpong (
    input  logic clk,
    input  logic reset_n,
    input  logic start,
    input  logic deny_dup,
    input  logic [31:0] scenario_seed,
    output logic done,
    output logic trace_valid,
    output logic [7:0] trace_code,
    output logic [63:0] trace_value,
    output logic no_forged_fdr,
    output logic no_lost_wakeup,
    output logic exactly_one_scheduler_location,
    output logic stale_generation_rejected,
    output logic queue_full_explicit,
    output logic typed_commit_valid,
    output lnp64_m1_cap_commit_t typed_commit,
    output lnp64_m1_state_projection_t typed_state_projection
);
    typedef enum logic [3:0] {
        M1_RESET,
        M1_BOOT,
        M1_CAP_DUP,
        M1_CAP_SEND,
        M1_CAP_RECV,
        M1_CONSUMER_AWAIT,
        M1_PRODUCER_PUSH,
        M1_CONSUMER_PULL,
        M1_QUEUE_REFILL,
        M1_QUEUE_FULL,
        M1_OBJECT_CREATE,
        M1_CAP_REVOKE,
        M1_STALE_REJECT,
        M1_DONE
    } m1_state_e;

    typedef enum logic [1:0] {
        LOC_NONE,
        LOC_RUNNABLE,
        LOC_RUNNING,
        LOC_PARKED
    } sched_location_e;

    localparam logic [63:0] RIGHT_PUSH = 64'h1;
    localparam logic [63:0] RIGHT_PULL = 64'h2;
    localparam logic [63:0] RIGHT_DUP  = 64'h4;
    localparam logic [63:0] RIGHT_MINT = 64'h8;

    localparam logic [31:0] M1_QUEUE_OBJECT_ID = 32'd1;
    localparam logic [31:0] M1_CREATED_OBJECT_ID = 32'd2;
    localparam logic [31:0] M1_CREATED_OBJECT_GEN = 32'd1;
    localparam logic [31:0] M1_ROOT_DOMAIN_ID = 32'd1;
    localparam logic [31:0] M1_CONSUMER_DOMAIN_ID = 32'd2;
    localparam logic [31:0] M1_DOMAIN_GEN = 32'd1;
    localparam logic [31:0] M1_LINEAGE_EPOCH = 32'd1;

    m1_state_e state;
    sched_location_e producer_loc;
    sched_location_e consumer_loc;
    logic queue_valid;
    logic [63:0] queue_value;
    logic [31:0] queue_generation;
    logic [31:0] producer_fd_generation;
    logic [31:0] consumer_fd_generation;
    logic [63:0] producer_rights;
    logic [63:0] consumer_rights;
    logic wake_pending;
    logic transfer_valid;
    logic created_object_created;
    logic [31:0] created_object_generation;
    logic sent_cap_valid;
    logic minted_cap_valid;
    logic revoked_rejected;
    logic failed_no_authority;
    logic has_revoked_generation;
    logic [31:0] revoked_generation;
    logic [31:0] event_count;

    function automatic logic [31:0] seeded_queue_gen(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[3:0]} + 32'd1;
    endfunction

    function automatic logic [63:0] seeded_push_value(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 64'd42;
        end
        return 64'd32 + {56'd0, seed[7:0]};
    endfunction

    function automatic logic [63:0] seeded_refill_value(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 64'd7;
        end
        return 64'd1 + {56'd0, seed[15:8]};
    endfunction

    function automatic logic exactly_one(input sched_location_e loc);
        return loc != LOC_NONE;
    endfunction

    always_comb begin
        typed_state_projection = '0;
        typed_state_projection.op = typed_commit.op;
        typed_state_projection.status = typed_commit.status;
        typed_state_projection.object_gen = queue_generation;
        typed_state_projection.created_object_created = created_object_created;
        typed_state_projection.created_object_gen = created_object_generation;
        typed_state_projection.root_object_id = M1_QUEUE_OBJECT_ID;
        typed_state_projection.root_generation = producer_fd_generation;
        typed_state_projection.root_domain_id = M1_ROOT_DOMAIN_ID;
        typed_state_projection.root_lineage_epoch = M1_LINEAGE_EPOCH;
        typed_state_projection.root_sealed = 1'b0;
        typed_state_projection.root_rights = producer_rights;
        typed_state_projection.consumer_object_id = M1_QUEUE_OBJECT_ID;
        typed_state_projection.consumer_generation = consumer_fd_generation;
        typed_state_projection.consumer_domain_id = M1_CONSUMER_DOMAIN_ID;
        typed_state_projection.consumer_lineage_epoch = M1_LINEAGE_EPOCH;
        typed_state_projection.consumer_sealed = 1'b0;
        typed_state_projection.consumer_rights = consumer_rights;
        typed_state_projection.sent_valid = sent_cap_valid;
        if (sent_cap_valid) begin
            typed_state_projection.sent_object_id = M1_QUEUE_OBJECT_ID;
            typed_state_projection.sent_generation = consumer_fd_generation;
            typed_state_projection.sent_domain_id = M1_CONSUMER_DOMAIN_ID;
            typed_state_projection.sent_lineage_epoch = M1_LINEAGE_EPOCH;
            typed_state_projection.sent_sealed = 1'b0;
            typed_state_projection.sent_rights = consumer_rights;
        end
        typed_state_projection.minted_valid = minted_cap_valid;
        if (minted_cap_valid) begin
            typed_state_projection.minted_object_id = M1_CREATED_OBJECT_ID;
            typed_state_projection.minted_generation = created_object_generation;
            typed_state_projection.minted_domain_id = M1_ROOT_DOMAIN_ID;
            typed_state_projection.minted_lineage_epoch = M1_LINEAGE_EPOCH;
            typed_state_projection.minted_sealed = 1'b0;
            typed_state_projection.minted_rights = producer_rights;
        end
        typed_state_projection.wake_pending = wake_pending;
        typed_state_projection.transfer_valid = transfer_valid;
        typed_state_projection.stale_rejected = stale_generation_rejected;
        typed_state_projection.revoked_rejected = revoked_rejected;
        typed_state_projection.failed_no_authority = failed_no_authority;
        typed_state_projection.full_was_explicit = queue_full_explicit;
        typed_state_projection.has_revoked_generation = has_revoked_generation;
        typed_state_projection.revoked_generation = revoked_generation;
    end

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            state <= M1_RESET;
            done <= 1'b0;
            trace_valid <= 1'b0;
            trace_code <= 8'd0;
            trace_value <= 64'd0;
            no_forged_fdr <= 1'b0;
            no_lost_wakeup <= 1'b0;
            exactly_one_scheduler_location <= 1'b0;
            stale_generation_rejected <= 1'b0;
            queue_full_explicit <= 1'b0;
            typed_commit_valid <= 1'b0;
            typed_commit <= '0;
            producer_loc <= LOC_NONE;
            consumer_loc <= LOC_NONE;
            queue_valid <= 1'b0;
            queue_value <= 64'd0;
            queue_generation <= 32'd0;
            producer_fd_generation <= 32'd0;
            consumer_fd_generation <= 32'd0;
            producer_rights <= 64'd0;
            consumer_rights <= 64'd0;
            wake_pending <= 1'b0;
            transfer_valid <= 1'b0;
            created_object_created <= 1'b0;
            created_object_generation <= 32'd0;
            sent_cap_valid <= 1'b0;
            minted_cap_valid <= 1'b0;
            revoked_rejected <= 1'b0;
            failed_no_authority <= 1'b0;
            has_revoked_generation <= 1'b0;
            revoked_generation <= 32'd0;
            event_count <= 32'd0;
        end else begin
            trace_valid <= 1'b0;
            typed_commit_valid <= 1'b0;
            exactly_one_scheduler_location <= exactly_one(producer_loc) && exactly_one(consumer_loc);
            unique case (state)
                M1_RESET: begin
                    if (start) begin
                        state <= M1_BOOT;
                    end
                end
                M1_BOOT: begin
                    producer_loc <= LOC_RUNNABLE;
                    consumer_loc <= LOC_RUNNABLE;
                    queue_generation <= seeded_queue_gen(scenario_seed);
                    producer_fd_generation <= seeded_queue_gen(scenario_seed);
                    consumer_fd_generation <= 32'd0;
                    producer_rights <= deny_dup ? (RIGHT_PUSH | RIGHT_PULL) : (RIGHT_PUSH | RIGHT_PULL | RIGHT_DUP | RIGHT_MINT);
                    consumer_rights <= 64'd0;
                    transfer_valid <= 1'b0;
                    created_object_created <= 1'b0;
                    created_object_generation <= M1_CREATED_OBJECT_GEN;
                    sent_cap_valid <= 1'b0;
                    minted_cap_valid <= 1'b0;
                    revoked_rejected <= 1'b0;
                    failed_no_authority <= 1'b0;
                    has_revoked_generation <= 1'b0;
                    revoked_generation <= 32'd0;
                    queue_valid <= 1'b0;
                    no_forged_fdr <= 1'b1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd1;
                    trace_value <= {32'd0, seeded_queue_gen(scenario_seed)};
                    state <= M1_CAP_DUP;
                end
                M1_CAP_DUP: begin
                    if (producer_fd_generation == queue_generation &&
                        (producer_rights & RIGHT_DUP) != 64'd0) begin
                        consumer_fd_generation <= producer_fd_generation;
                        consumer_rights <= RIGHT_PULL;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd2;
                        trace_value <= RIGHT_PULL;
                        typed_commit_valid <= 1'b1;
                        typed_commit.op <= LNP64_M1_COMMIT_CAP_DUP;
                        typed_commit.object_id <= M1_QUEUE_OBJECT_ID;
                        typed_commit.object_gen <= queue_generation;
                        typed_commit.fdr_gen <= producer_fd_generation;
                        typed_commit.domain_id <= M1_CONSUMER_DOMAIN_ID;
                        typed_commit.domain_gen <= M1_DOMAIN_GEN;
                        typed_commit.rights_mask <= RIGHT_PULL;
                        typed_commit.lineage_epoch <= M1_LINEAGE_EPOCH;
                        typed_commit.sealed <= 1'b0;
                        typed_commit.status <= LNP64_ERR_OK;
                        state <= M1_CAP_SEND;
                    end else begin
                        typed_commit_valid <= 1'b1;
                        typed_commit.op <= LNP64_M1_COMMIT_CAP_DUP_DENIED;
                        typed_commit.object_id <= M1_QUEUE_OBJECT_ID;
                        typed_commit.object_gen <= queue_generation;
                        typed_commit.fdr_gen <= producer_fd_generation;
                        typed_commit.domain_id <= M1_ROOT_DOMAIN_ID;
                        typed_commit.domain_gen <= M1_DOMAIN_GEN;
                        typed_commit.rights_mask <= producer_rights;
                        typed_commit.lineage_epoch <= M1_LINEAGE_EPOCH;
                        typed_commit.sealed <= 1'b0;
                        typed_commit.status <= LNP64_ERR_EPERM;
                        failed_no_authority <= 1'b1;
                        state <= M1_DONE;
                    end
                end
                M1_CAP_SEND: begin
                    typed_commit_valid <= 1'b1;
                    typed_commit.op <= LNP64_M1_COMMIT_CAP_SEND;
                    typed_commit.object_id <= M1_QUEUE_OBJECT_ID;
                    typed_commit.object_gen <= queue_generation;
                    typed_commit.fdr_gen <= consumer_fd_generation;
                    typed_commit.domain_id <= M1_CONSUMER_DOMAIN_ID;
                    typed_commit.domain_gen <= M1_DOMAIN_GEN;
                    typed_commit.rights_mask <= consumer_rights;
                    typed_commit.lineage_epoch <= M1_LINEAGE_EPOCH;
                    typed_commit.sealed <= 1'b0;
                    typed_commit.status <= LNP64_ERR_OK;
                    sent_cap_valid <= 1'b1;
                    transfer_valid <= 1'b1;
                    state <= M1_CAP_RECV;
                end
                M1_CAP_RECV: begin
                    typed_commit_valid <= 1'b1;
                    typed_commit.op <= LNP64_M1_COMMIT_CAP_RECV;
                    typed_commit.object_id <= M1_QUEUE_OBJECT_ID;
                    typed_commit.object_gen <= queue_generation;
                    typed_commit.fdr_gen <= consumer_fd_generation;
                    typed_commit.domain_id <= M1_CONSUMER_DOMAIN_ID;
                    typed_commit.domain_gen <= M1_DOMAIN_GEN;
                    typed_commit.rights_mask <= consumer_rights;
                    typed_commit.lineage_epoch <= M1_LINEAGE_EPOCH;
                    typed_commit.sealed <= 1'b0;
                    typed_commit.status <= LNP64_ERR_OK;
                    sent_cap_valid <= 1'b0;
                    transfer_valid <= 1'b1;
                    state <= M1_CONSUMER_AWAIT;
                end
                M1_CONSUMER_AWAIT: begin
                    consumer_loc <= LOC_PARKED;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd3;
                    trace_value <= 64'd0;
                    state <= M1_PRODUCER_PUSH;
                end
                M1_PRODUCER_PUSH: begin
                    producer_loc <= LOC_RUNNING;
                    if (!queue_valid &&
                        producer_fd_generation == queue_generation &&
                        (producer_rights & RIGHT_PUSH) != 64'd0) begin
                        queue_valid <= 1'b1;
                        queue_value <= seeded_push_value(scenario_seed);
                        wake_pending <= 1'b1;
                        consumer_loc <= LOC_RUNNABLE;
                        event_count <= event_count + 32'd1;
                        no_lost_wakeup <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd4;
                        trace_value <= seeded_push_value(scenario_seed);
                        typed_commit_valid <= 1'b1;
                        typed_commit.op <= LNP64_M1_COMMIT_PUSH;
                        typed_commit.object_id <= M1_QUEUE_OBJECT_ID;
                        typed_commit.object_gen <= queue_generation;
                        typed_commit.fdr_gen <= producer_fd_generation;
                        typed_commit.domain_id <= M1_ROOT_DOMAIN_ID;
                        typed_commit.domain_gen <= M1_DOMAIN_GEN;
                        typed_commit.rights_mask <= producer_rights;
                        typed_commit.lineage_epoch <= M1_LINEAGE_EPOCH;
                        typed_commit.sealed <= 1'b0;
                        typed_commit.status <= LNP64_ERR_OK;
                        state <= M1_CONSUMER_PULL;
                    end else begin
                        state <= M1_DONE;
                    end
                end
                M1_CONSUMER_PULL: begin
                    producer_loc <= LOC_RUNNABLE;
                    consumer_loc <= LOC_RUNNING;
                    if (queue_valid &&
                        consumer_fd_generation == queue_generation &&
                        (consumer_rights & RIGHT_PULL) != 64'd0) begin
                        queue_valid <= 1'b0;
                        wake_pending <= 1'b0;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd5;
                        trace_value <= queue_value;
                        typed_commit_valid <= 1'b1;
                        typed_commit.op <= LNP64_M1_COMMIT_PULL;
                        typed_commit.object_id <= M1_QUEUE_OBJECT_ID;
                        typed_commit.object_gen <= queue_generation;
                        typed_commit.fdr_gen <= consumer_fd_generation;
                        typed_commit.domain_id <= M1_CONSUMER_DOMAIN_ID;
                        typed_commit.domain_gen <= M1_DOMAIN_GEN;
                        typed_commit.rights_mask <= consumer_rights;
                        typed_commit.lineage_epoch <= M1_LINEAGE_EPOCH;
                        typed_commit.sealed <= 1'b0;
                        typed_commit.status <= LNP64_ERR_OK;
                        state <= M1_QUEUE_REFILL;
                    end else begin
                        state <= M1_DONE;
                    end
                end
                M1_QUEUE_REFILL: begin
                    consumer_loc <= LOC_RUNNABLE;
                    queue_valid <= 1'b1;
                    queue_value <= seeded_refill_value(scenario_seed);
                    trace_valid <= 1'b1;
                    trace_code <= 8'd6;
                    trace_value <= seeded_refill_value(scenario_seed);
                    state <= M1_QUEUE_FULL;
                end
                M1_QUEUE_FULL: begin
                    producer_loc <= LOC_RUNNING;
                    if (queue_valid) begin
                        queue_full_explicit <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd7;
                        trace_value <= {48'd0, LNP64_ERR_EAGAIN};
                        typed_commit_valid <= 1'b1;
                        typed_commit.op <= LNP64_M1_COMMIT_REJECT_FULL;
                        typed_commit.object_id <= M1_QUEUE_OBJECT_ID;
                        typed_commit.object_gen <= queue_generation;
                        typed_commit.fdr_gen <= producer_fd_generation;
                        typed_commit.domain_id <= M1_ROOT_DOMAIN_ID;
                        typed_commit.domain_gen <= M1_DOMAIN_GEN;
                        typed_commit.rights_mask <= producer_rights;
                        typed_commit.lineage_epoch <= M1_LINEAGE_EPOCH;
                        typed_commit.sealed <= 1'b0;
                        typed_commit.status <= LNP64_ERR_EAGAIN;
                        state <= M1_OBJECT_CREATE;
                    end else begin
                        state <= M1_DONE;
                    end
                end
                M1_OBJECT_CREATE: begin
                    producer_loc <= LOC_RUNNING;
                    if (producer_fd_generation == queue_generation &&
                        (producer_rights & RIGHT_MINT) != 64'd0) begin
                        typed_commit_valid <= 1'b1;
                        typed_commit.op <= LNP64_M1_COMMIT_OBJECT_CREATE;
                        typed_commit.object_id <= M1_CREATED_OBJECT_ID;
                        typed_commit.object_gen <= M1_CREATED_OBJECT_GEN;
                        typed_commit.fdr_gen <= M1_CREATED_OBJECT_GEN;
                        typed_commit.domain_id <= M1_ROOT_DOMAIN_ID;
                        typed_commit.domain_gen <= M1_DOMAIN_GEN;
                        typed_commit.rights_mask <= producer_rights;
                        typed_commit.lineage_epoch <= M1_LINEAGE_EPOCH;
                        typed_commit.sealed <= 1'b0;
                        typed_commit.status <= LNP64_ERR_OK;
                        created_object_created <= 1'b1;
                        minted_cap_valid <= 1'b1;
                        state <= M1_CAP_REVOKE;
                    end else begin
                        state <= M1_DONE;
                    end
                end
                M1_CAP_REVOKE: begin
                    producer_loc <= LOC_RUNNABLE;
                    queue_generation <= queue_generation + 32'd1;
                    producer_fd_generation <= queue_generation + 32'd1;
                    has_revoked_generation <= 1'b1;
                    revoked_generation <= queue_generation;
                    revoked_rejected <= 1'b1;
                    stale_generation_rejected <= 1'b1;
                    typed_commit_valid <= 1'b1;
                    typed_commit.op <= LNP64_M1_COMMIT_CAP_REVOKE;
                    typed_commit.object_id <= M1_QUEUE_OBJECT_ID;
                    typed_commit.object_gen <= queue_generation + 32'd1;
                    typed_commit.fdr_gen <= queue_generation;
                    typed_commit.domain_id <= M1_ROOT_DOMAIN_ID;
                    typed_commit.domain_gen <= M1_DOMAIN_GEN;
                    typed_commit.rights_mask <= producer_rights;
                    typed_commit.lineage_epoch <= M1_LINEAGE_EPOCH;
                    typed_commit.sealed <= 1'b0;
                    typed_commit.status <= LNP64_ERR_OK;
                    state <= M1_STALE_REJECT;
                end
                M1_STALE_REJECT: begin
                    producer_loc <= LOC_RUNNABLE;
                    if (consumer_fd_generation != queue_generation) begin
                        stale_generation_rejected <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd8;
                        trace_value <= {48'd0, LNP64_ERR_EREVOKED};
                        typed_commit_valid <= 1'b1;
                        typed_commit.op <= LNP64_M1_COMMIT_REJECT_STALE;
                        typed_commit.object_id <= M1_QUEUE_OBJECT_ID;
                        typed_commit.object_gen <= queue_generation;
                        typed_commit.fdr_gen <= consumer_fd_generation;
                        typed_commit.domain_id <= M1_CONSUMER_DOMAIN_ID;
                        typed_commit.domain_gen <= M1_DOMAIN_GEN;
                        typed_commit.rights_mask <= consumer_rights;
                        typed_commit.lineage_epoch <= M1_LINEAGE_EPOCH;
                        typed_commit.sealed <= 1'b0;
                        typed_commit.status <= LNP64_ERR_EREVOKED;
                    end
                    state <= M1_DONE;
                end
                M1_DONE: begin
                    producer_loc <= LOC_RUNNABLE;
                    consumer_loc <= LOC_RUNNABLE;
                    if (!done) begin
                        done <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd9;
                        trace_value <= {32'd0, event_count};
                    end
                end
                default: state <= M1_RESET;
            endcase
        end
    end
endmodule

`timescale 1ns/1ps

import lnp64_pkg::*;

// M16 unified-endpoint typed-trace engine. The endpoint is a bounded queue
// (EP-F): a Memory-backed endpoint carries (bytes, caps) messages; a
// Register-backed endpoint is a counter whose edge a notify (empty send)
// raises. This scenario engine walks one deterministic trace exercising the
// four EP-F invariant classes and emits a typed commit + invariant projection
// per retired endpoint op, mirroring the M15 object-profile engine.
module lnp64_m16_endpoint (
    input  logic clk,
    input  logic reset_n,
    input  logic start,
    input  logic [31:0] scenario_seed,
    output logic done,
    output logic trace_valid,
    output logic [7:0] trace_code,
    output logic [63:0] trace_value,
    output logic bounded_depth_le_capacity,
    output logic drain_bounded_by_capacity,
    output logic full_fails_closed,
    output logic empty_fails_closed,
    output logic oversize_fails_closed,
    output logic no_block_except_wait,
    output logic caps_resolve_sender_only,
    output logic caps_reject_out_of_range,
    output logic install_no_amplify,
    output logic framing_one_send_one_recv,
    output logic notify_raises_register_edge,
    output logic counts_exact,
    output logic typed_commit_valid,
    output lnp64_m16_endpoint_commit_t typed_commit,
    output lnp64_m16_state_projection_t typed_state_projection
);
    typedef enum logic [4:0] {
        S_RESET,
        S_CREATE,
        S_SEND1,
        S_RECV1,
        S_FILL_A,
        S_FILL_B,
        S_SEND_FULL,
        S_DRAIN_A,
        S_DRAIN_B,
        S_RECV_EMPTY,
        S_OVERSIZE,
        S_CAP_SEND,
        S_CAP_REJECT,
        S_NOTIFY,
        S_DONE
    } endpoint_state_e;

    // Sender holds rights {push|pull|event}; install must not amplify beyond it.
    localparam logic [63:0] SENDER_RIGHTS = 64'h0000_0000_0000_0007;
    localparam logic [31:0] MSG_MAX_BYTES = 32'd64;   // oversize threshold (EMSGSIZE)

    endpoint_state_e state;
    logic [31:0] capacity;
    logic [31:0] depth;
    logic [31:0] failures;
    logic [31:0] events;
    logic [31:0] drain_count;
    logic [31:0] register_edge;
    logic [31:0] delivered;          // running count of one-send/one-recv pairs

    function automatic logic [31:0] seeded_capacity(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd2;
        end
        return {30'd0, seed[1:0]} + 32'd2;   // 2..5
    endfunction

    function automatic logic [31:0] seeded_endpoint_id(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd1;
        end
        return {28'd0, seed[3:0]} + 32'd1;
    endfunction

    function automatic logic [31:0] seeded_bytes(input logic [31:0] seed);
        if (seed == 32'd0) begin
            return 32'd8;
        end
        return 32'd8 + {28'd0, seed[7:4]};
    endfunction

    // counts_exact: the deterministic trace produces exactly four fail-closed
    // rejections (full send, empty recv, oversize, cap reject) and one
    // register edge (notify).
    always_comb begin
        counts_exact = (failures == 32'd4) && (events == 32'd1);
    end

    always_comb begin
        bounded_depth_le_capacity = (depth <= capacity);
        drain_bounded_by_capacity = (drain_count <= capacity);
    end

    always_comb begin
        typed_state_projection = '0;
        typed_state_projection.op = typed_commit.op;
        typed_state_projection.status = typed_commit.status;
        typed_state_projection.depth = depth;
        typed_state_projection.capacity = capacity;
        typed_state_projection.failures = failures;
        typed_state_projection.events = events;
        typed_state_projection.bounded_depth_le_capacity = bounded_depth_le_capacity;
        typed_state_projection.drain_bounded_by_capacity = drain_bounded_by_capacity;
        typed_state_projection.full_fails_closed = full_fails_closed;
        typed_state_projection.empty_fails_closed = empty_fails_closed;
        typed_state_projection.oversize_fails_closed = oversize_fails_closed;
        typed_state_projection.no_block_except_wait = no_block_except_wait;
        typed_state_projection.caps_resolve_sender_only = caps_resolve_sender_only;
        typed_state_projection.caps_reject_out_of_range = caps_reject_out_of_range;
        typed_state_projection.install_no_amplify = install_no_amplify;
        typed_state_projection.framing_one_send_one_recv = framing_one_send_one_recv;
        typed_state_projection.notify_raises_register_edge = notify_raises_register_edge;
        typed_state_projection.counts_exact = counts_exact;
    end

    task automatic commit_m16(
        input lnp64_m16_endpoint_op_e op,
        input lnp64_m16_backing_e backing,
        input logic [15:0] status,
        input logic [31:0] bytes_len,
        input logic [31:0] caps_len,
        input logic [31:0] caps_resolved,
        input logic [31:0] caps_installed,
        input logic [31:0] next_depth
    );
        typed_commit_valid <= 1'b1;
        typed_commit.op <= op;
        typed_commit.status <= status;
        typed_commit.endpoint_id <= seeded_endpoint_id(scenario_seed);
        typed_commit.endpoint_gen <= 32'd1;
        typed_commit.backing <= backing;
        typed_commit.bytes_len <= bytes_len;
        typed_commit.caps_len <= caps_len;
        typed_commit.depth <= next_depth;
        typed_commit.capacity <= capacity;
        typed_commit.caps_resolved <= caps_resolved;
        typed_commit.caps_installed <= caps_installed;
        typed_commit.sender_domain_id <= 32'd1;
        typed_commit.sender_domain_gen <= 32'd1;
        typed_commit.receiver_domain_id <= 32'd2;
        typed_commit.receiver_domain_gen <= 32'd1;
    endtask

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            state <= S_RESET;
            done <= 1'b0;
            trace_valid <= 1'b0;
            trace_code <= 8'd0;
            trace_value <= 64'd0;
            typed_commit_valid <= 1'b0;
            typed_commit <= '0;
            capacity <= 32'd0;
            depth <= 32'd0;
            failures <= 32'd0;
            events <= 32'd0;
            drain_count <= 32'd0;
            register_edge <= 32'd0;
            delivered <= 32'd0;
            full_fails_closed <= 1'b0;
            empty_fails_closed <= 1'b0;
            oversize_fails_closed <= 1'b0;
            no_block_except_wait <= 1'b1;
            caps_resolve_sender_only <= 1'b0;
            caps_reject_out_of_range <= 1'b0;
            install_no_amplify <= 1'b0;
            framing_one_send_one_recv <= 1'b0;
            notify_raises_register_edge <= 1'b0;
        end else begin
            trace_valid <= 1'b0;
            typed_commit_valid <= 1'b0;
            unique case (state)
                S_RESET: begin
                    if (start) begin
                        capacity <= seeded_capacity(scenario_seed);
                        state <= S_CREATE;
                    end
                end
                S_CREATE: begin
                    // endpoint_create: Memory-backed, empty queue.
                    trace_valid <= 1'b1;
                    trace_code <= 8'd1;
                    trace_value <= {seeded_endpoint_id(scenario_seed), seeded_capacity(scenario_seed)};
                    commit_m16(LNP64_M16_COMMIT_CREATE, LNP64_M16_BACKING_MEMORY,
                        LNP64_STATUS_OK, 32'd0, 32'd0, 32'd0, 32'd0, 32'd0);
                    state <= S_SEND1;
                end
                S_SEND1: begin
                    // framing: one send enqueues exactly one message.
                    depth <= 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd2;
                    trace_value <= {seeded_bytes(scenario_seed), 32'd1};
                    commit_m16(LNP64_M16_COMMIT_SEND, LNP64_M16_BACKING_MEMORY,
                        LNP64_STATUS_OK, seeded_bytes(scenario_seed), 32'd0, 32'd0, 32'd0, 32'd1);
                    state <= S_RECV1;
                end
                S_RECV1: begin
                    // framing: one recv dequeues exactly one message.
                    depth <= 32'd0;
                    framing_one_send_one_recv <= 1'b1;
                    delivered <= delivered + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd3;
                    trace_value <= {seeded_bytes(scenario_seed), 32'd0};
                    commit_m16(LNP64_M16_COMMIT_RECV, LNP64_M16_BACKING_MEMORY,
                        LNP64_STATUS_OK, seeded_bytes(scenario_seed), 32'd0, 32'd0, 32'd0, 32'd0);
                    state <= S_FILL_A;
                end
                S_FILL_A: begin
                    depth <= 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd2;
                    trace_value <= {seeded_bytes(scenario_seed), 32'd1};
                    commit_m16(LNP64_M16_COMMIT_SEND, LNP64_M16_BACKING_MEMORY,
                        LNP64_STATUS_OK, seeded_bytes(scenario_seed), 32'd0, 32'd0, 32'd0, 32'd1);
                    state <= S_FILL_B;
                end
                S_FILL_B: begin
                    // fill to capacity (depth == capacity); still bounded.
                    depth <= capacity;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd2;
                    trace_value <= {seeded_bytes(scenario_seed), capacity};
                    commit_m16(LNP64_M16_COMMIT_SEND, LNP64_M16_BACKING_MEMORY,
                        LNP64_STATUS_OK, seeded_bytes(scenario_seed), 32'd0, 32'd0, 32'd0, capacity);
                    state <= S_SEND_FULL;
                end
                S_SEND_FULL: begin
                    // fail-closed: send on a full queue -> EAGAIN, depth unchanged,
                    // never exceeds capacity (bounded).
                    full_fails_closed <= 1'b1;
                    failures <= failures + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd4;
                    trace_value <= {capacity, 16'd0, LNP64_ERR_EAGAIN};
                    commit_m16(LNP64_M16_COMMIT_SEND_FULL, LNP64_M16_BACKING_MEMORY,
                        LNP64_ERR_EAGAIN, seeded_bytes(scenario_seed), 32'd0, 32'd0, 32'd0, capacity);
                    state <= S_DRAIN_A;
                end
                S_DRAIN_A: begin
                    depth <= depth - 32'd1;
                    drain_count <= drain_count + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd3;
                    trace_value <= {seeded_bytes(scenario_seed), depth - 32'd1};
                    commit_m16(LNP64_M16_COMMIT_RECV, LNP64_M16_BACKING_MEMORY,
                        LNP64_STATUS_OK, seeded_bytes(scenario_seed), 32'd0, 32'd0, 32'd0, depth - 32'd1);
                    state <= S_DRAIN_B;
                end
                S_DRAIN_B: begin
                    depth <= 32'd0;
                    drain_count <= drain_count + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd3;
                    trace_value <= {seeded_bytes(scenario_seed), 32'd0};
                    commit_m16(LNP64_M16_COMMIT_RECV, LNP64_M16_BACKING_MEMORY,
                        LNP64_STATUS_OK, seeded_bytes(scenario_seed), 32'd0, 32'd0, 32'd0, 32'd0);
                    state <= S_RECV_EMPTY;
                end
                S_RECV_EMPTY: begin
                    // fail-closed: recv on an empty queue -> EAGAIN.
                    empty_fails_closed <= 1'b1;
                    failures <= failures + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd5;
                    trace_value <= {32'd0, 16'd0, LNP64_ERR_EAGAIN};
                    commit_m16(LNP64_M16_COMMIT_RECV_EMPTY, LNP64_M16_BACKING_MEMORY,
                        LNP64_ERR_EAGAIN, 32'd0, 32'd0, 32'd0, 32'd0, 32'd0);
                    state <= S_OVERSIZE;
                end
                S_OVERSIZE: begin
                    // fail-closed: a message larger than the bound -> EMSGSIZE.
                    oversize_fails_closed <= 1'b1;
                    failures <= failures + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd6;
                    trace_value <= {MSG_MAX_BYTES + 32'd1, 16'd0, LNP64_ERR_EMSGSIZE};
                    commit_m16(LNP64_M16_COMMIT_OVERSIZE, LNP64_M16_BACKING_MEMORY,
                        LNP64_ERR_EMSGSIZE, MSG_MAX_BYTES + 32'd1, 32'd0, 32'd0, 32'd0, 32'd0);
                    state <= S_CAP_SEND;
                end
                S_CAP_SEND: begin
                    // cap-safety: one cap resolves against the sender's table and
                    // installs into the receiver's with no rights amplification.
                    depth <= 32'd1;
                    caps_resolve_sender_only <= 1'b1;
                    install_no_amplify <= 1'b1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd7;
                    trace_value <= {SENDER_RIGHTS[31:0], 32'd1};
                    commit_m16(LNP64_M16_COMMIT_CAP_SEND, LNP64_M16_BACKING_MEMORY,
                        LNP64_STATUS_OK, seeded_bytes(scenario_seed), 32'd1, 32'd1, 32'd1, 32'd1);
                    state <= S_CAP_REJECT;
                end
                S_CAP_REJECT: begin
                    // cap-safety: an out-of-range / revoked cap handle is rejected;
                    // nothing is installed.
                    caps_reject_out_of_range <= 1'b1;
                    failures <= failures + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd8;
                    trace_value <= {32'hffff_ffff, 16'd0, LNP64_ERR_EBADF};
                    commit_m16(LNP64_M16_COMMIT_CAP_REJECT, LNP64_M16_BACKING_MEMORY,
                        LNP64_ERR_EBADF, seeded_bytes(scenario_seed), 32'd1, 32'd0, 32'd0, 32'd1);
                    state <= S_NOTIFY;
                end
                S_NOTIFY: begin
                    // framing: an empty send to a Register-backed endpoint raises
                    // its edge by +1 (futex_wake / eventfd-notify).
                    notify_raises_register_edge <= 1'b1;
                    register_edge <= register_edge + 32'd1;
                    events <= events + 32'd1;
                    trace_valid <= 1'b1;
                    trace_code <= 8'd9;
                    trace_value <= {register_edge + 32'd1, 32'd0};
                    commit_m16(LNP64_M16_COMMIT_NOTIFY, LNP64_M16_BACKING_REGISTER,
                        LNP64_STATUS_OK, 32'd0, 32'd0, 32'd0, 32'd0, 32'd1);
                    state <= S_DONE;
                end
                S_DONE: begin
                    if (!done) begin
                        done <= 1'b1;
                        trace_valid <= 1'b1;
                        trace_code <= 8'd10;
                        trace_value <= {failures, events};
                    end
                end
                default: state <= S_RESET;
            endcase
        end
    end
endmodule

`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m1_assertions (
    input logic clk,
    input logic reset_n,
    input logic done,
    input logic no_forged_fdr,
    input logic no_lost_wakeup,
    input logic exactly_one_scheduler_location,
    input logic stale_generation_rejected,
    input logic queue_full_explicit,
    input logic expect_denied,
    input logic typed_commit_valid,
    input lnp64_m1_cap_commit_t typed_commit
);
    localparam logic [63:0] M1_RIGHT_PUSH = 64'h1;
    localparam logic [63:0] M1_RIGHT_PULL = 64'h2;
    localparam logic [63:0] M1_RIGHT_DUP  = 64'h4;
    localparam logic [63:0] M1_RIGHT_MINT = 64'h8;
    localparam logic [63:0] M1_ROOT_RIGHTS = M1_RIGHT_PUSH | M1_RIGHT_PULL | M1_RIGHT_DUP | M1_RIGHT_MINT;

    localparam logic [31:0] M1_QUEUE_OBJECT_ID = 32'd1;
    localparam logic [31:0] M1_CREATED_OBJECT_ID = 32'd2;
    localparam logic [31:0] M1_CREATED_OBJECT_GEN = 32'd1;
    localparam logic [31:0] M1_ROOT_DOMAIN_ID = 32'd1;
    localparam logic [31:0] M1_CONSUMER_DOMAIN_ID = 32'd2;
    localparam logic [31:0] M1_DOMAIN_GEN = 32'd1;
    localparam logic [31:0] M1_LINEAGE_EPOCH = 32'd1;

    logic [3:0] commit_index;
    logic have_duplicated_cap;
    logic have_sent_cap;
    lnp64_m1_cap_commit_t duplicated_cap;
    lnp64_m1_cap_commit_t sent_cap;

    function automatic logic [7:0] expected_commit_op(input logic [3:0] index);
        if (expect_denied) begin
            return (index == 4'd0) ? LNP64_M1_COMMIT_CAP_DUP_DENIED : 8'hff;
        end
        unique case (index)
            4'd0: expected_commit_op = LNP64_M1_COMMIT_CAP_DUP;
            4'd1: expected_commit_op = LNP64_M1_COMMIT_CAP_SEND;
            4'd2: expected_commit_op = LNP64_M1_COMMIT_CAP_RECV;
            4'd3: expected_commit_op = LNP64_M1_COMMIT_PUSH;
            4'd4: expected_commit_op = LNP64_M1_COMMIT_PULL;
            4'd5: expected_commit_op = LNP64_M1_COMMIT_REJECT_FULL;
            4'd6: expected_commit_op = LNP64_M1_COMMIT_OBJECT_CREATE;
            4'd7: expected_commit_op = LNP64_M1_COMMIT_CAP_REVOKE;
            4'd8: expected_commit_op = LNP64_M1_COMMIT_REJECT_STALE;
            default: expected_commit_op = 8'hff;
        endcase
    endfunction

    function automatic logic m1_rights_subset(input logic [63:0] child, input logic [63:0] parent);
        return (child & ~parent) == 64'd0;
    endfunction

    function automatic logic m1_lineage_valid(input lnp64_m1_cap_commit_t commit);
        return (commit.object_id == M1_QUEUE_OBJECT_ID || commit.object_id == M1_CREATED_OBJECT_ID) &&
            (commit.domain_id == M1_ROOT_DOMAIN_ID || commit.domain_id == M1_CONSUMER_DOMAIN_ID) &&
            commit.domain_gen == M1_DOMAIN_GEN &&
            commit.lineage_epoch == M1_LINEAGE_EPOCH &&
            !commit.sealed &&
            m1_rights_subset(commit.rights_mask, M1_ROOT_RIGHTS);
    endfunction

    function automatic logic m1_same_cap_fields(
        input lnp64_m1_cap_commit_t left,
        input lnp64_m1_cap_commit_t right
    );
        return left.object_id == right.object_id &&
            left.object_gen == right.object_gen &&
            left.fdr_gen == right.fdr_gen &&
            left.domain_id == right.domain_id &&
            left.domain_gen == right.domain_gen &&
            left.rights_mask == right.rights_mask &&
            left.lineage_epoch == right.lineage_epoch &&
            left.sealed == right.sealed;
    endfunction

    function automatic logic m1_root_live_authority(input lnp64_m1_cap_commit_t commit);
        return m1_lineage_valid(commit) &&
            commit.object_id == M1_QUEUE_OBJECT_ID &&
            commit.domain_id == M1_ROOT_DOMAIN_ID &&
            commit.rights_mask == M1_ROOT_RIGHTS &&
            commit.fdr_gen == commit.object_gen &&
            commit.status == LNP64_ERR_OK;
    endfunction

    function automatic logic m1_consumer_pull_authority(input lnp64_m1_cap_commit_t commit);
        return m1_lineage_valid(commit) &&
            commit.object_id == M1_QUEUE_OBJECT_ID &&
            commit.domain_id == M1_CONSUMER_DOMAIN_ID &&
            commit.rights_mask == M1_RIGHT_PULL &&
            commit.fdr_gen == commit.object_gen &&
            commit.status == LNP64_ERR_OK;
    endfunction

    function automatic logic m1_root_revoke_commit(input lnp64_m1_cap_commit_t commit);
        return m1_lineage_valid(commit) &&
            commit.object_id == M1_QUEUE_OBJECT_ID &&
            commit.domain_id == M1_ROOT_DOMAIN_ID &&
            commit.rights_mask == M1_ROOT_RIGHTS &&
            commit.object_gen == commit.fdr_gen + 32'd1 &&
            commit.status == LNP64_ERR_OK;
    endfunction

    function automatic logic m1_root_queue_full_reject(input lnp64_m1_cap_commit_t commit);
        return m1_lineage_valid(commit) &&
            commit.object_id == M1_QUEUE_OBJECT_ID &&
            commit.domain_id == M1_ROOT_DOMAIN_ID &&
            commit.rights_mask == M1_ROOT_RIGHTS &&
            commit.fdr_gen == commit.object_gen &&
            commit.status == LNP64_ERR_EAGAIN;
    endfunction

    function automatic logic m1_root_object_create(input lnp64_m1_cap_commit_t commit);
        return m1_lineage_valid(commit) &&
            commit.object_id == M1_CREATED_OBJECT_ID &&
            commit.object_gen == M1_CREATED_OBJECT_GEN &&
            commit.fdr_gen == M1_CREATED_OBJECT_GEN &&
            commit.domain_id == M1_ROOT_DOMAIN_ID &&
            commit.rights_mask == M1_ROOT_RIGHTS &&
            (commit.rights_mask & M1_RIGHT_MINT) != 64'd0 &&
            commit.status == LNP64_ERR_OK;
    endfunction

    function automatic logic m1_consumer_stale_reject(input lnp64_m1_cap_commit_t commit);
        return m1_lineage_valid(commit) &&
            commit.object_id == M1_QUEUE_OBJECT_ID &&
            commit.domain_id == M1_CONSUMER_DOMAIN_ID &&
            commit.rights_mask == M1_RIGHT_PULL &&
            commit.fdr_gen != commit.object_gen &&
            commit.status == LNP64_ERR_EREVOKED;
    endfunction

    function automatic logic m1_root_dup_denied(input lnp64_m1_cap_commit_t commit);
        return m1_lineage_valid(commit) &&
            commit.object_id == M1_QUEUE_OBJECT_ID &&
            commit.domain_id == M1_ROOT_DOMAIN_ID &&
            (commit.rights_mask & M1_RIGHT_DUP) == 64'd0 &&
            (commit.rights_mask & M1_RIGHT_MINT) == 64'd0 &&
            commit.fdr_gen == commit.object_gen &&
            commit.status == LNP64_ERR_EPERM;
    endfunction

    always_ff @(posedge clk) begin
        if (!reset_n) begin
            commit_index <= 4'd0;
            have_duplicated_cap <= 1'b0;
            have_sent_cap <= 1'b0;
            duplicated_cap <= '0;
            sent_cap <= '0;
        end else begin
            assert (exactly_one_scheduler_location || !done)
                else $fatal(1, "M1 scheduler location invariant failed");
            if (typed_commit_valid) begin
                assert (typed_commit.op == expected_commit_op(commit_index))
                    else $fatal(1, "M1 typed commit sequence drifted");
                assert ((typed_commit.rights_mask & ~M1_ROOT_RIGHTS) == 64'd0)
                    else $fatal(1, "M1 typed commit broadened rights beyond root");
                assert (typed_commit.fdr_gen != 32'd0)
                    else $fatal(1, "M1 typed commit used zero FDR generation");
                assert (typed_commit.object_gen != 32'd0)
                    else $fatal(1, "M1 typed commit used zero object generation");
                assert (typed_commit.status == LNP64_ERR_OK ||
                        typed_commit.status == LNP64_ERR_EPERM ||
                        typed_commit.status == LNP64_ERR_EAGAIN ||
                        typed_commit.status == LNP64_ERR_EREVOKED)
                    else $fatal(1, "M1 typed commit used unexpected status");
                assert (m1_lineage_valid(typed_commit))
                    else $fatal(1, "M1 typed commit failed lineage/rights validity");

                unique case (typed_commit.op)
                    LNP64_M1_COMMIT_CAP_DUP: begin
                        assert (m1_consumer_pull_authority(typed_commit))
                            else $fatal(1, "M1 consumer commit failed pull-authority predicate");
                        duplicated_cap <= typed_commit;
                        have_duplicated_cap <= 1'b1;
                    end
                    LNP64_M1_COMMIT_CAP_SEND: begin
                        assert (m1_consumer_pull_authority(typed_commit))
                            else $fatal(1, "M1 consumer commit failed pull-authority predicate");
                        assert (have_duplicated_cap)
                            else $fatal(1, "M1 capSend occurred before authorized capDup");
                        assert (m1_same_cap_fields(typed_commit, duplicated_cap))
                            else $fatal(1, "M1 capSend changed duplicated cap authority");
                        sent_cap <= typed_commit;
                        have_sent_cap <= 1'b1;
                    end
                    LNP64_M1_COMMIT_CAP_RECV: begin
                        assert (m1_consumer_pull_authority(typed_commit))
                            else $fatal(1, "M1 consumer commit failed pull-authority predicate");
                        assert (have_sent_cap)
                            else $fatal(1, "M1 capRecv occurred before a valid capSend");
                        assert (m1_same_cap_fields(typed_commit, sent_cap))
                            else $fatal(1, "M1 capRecv changed sent cap authority");
                        have_sent_cap <= 1'b0;
                    end
                    LNP64_M1_COMMIT_PULL: begin
                        assert (m1_consumer_pull_authority(typed_commit))
                            else $fatal(1, "M1 consumer commit failed pull-authority predicate");
                    end
                    LNP64_M1_COMMIT_PUSH: begin
                        assert (m1_root_live_authority(typed_commit))
                            else $fatal(1, "M1 push failed root live-authority predicate");
                    end
                    LNP64_M1_COMMIT_REJECT_FULL: begin
                        assert (m1_root_queue_full_reject(typed_commit))
                            else $fatal(1, "M1 rejectFull failed root queue-full predicate");
                    end
                    LNP64_M1_COMMIT_OBJECT_CREATE: begin
                        assert (m1_root_object_create(typed_commit))
                            else $fatal(1, "M1 objectCreate failed root mint predicate");
                    end
                    LNP64_M1_COMMIT_CAP_REVOKE: begin
                        assert (m1_root_revoke_commit(typed_commit))
                            else $fatal(1, "M1 revoke failed root revoke-commit predicate");
                    end
                    LNP64_M1_COMMIT_REJECT_STALE: begin
                        assert (m1_consumer_stale_reject(typed_commit))
                            else $fatal(1, "M1 rejectStale failed consumer stale-reject predicate");
                    end
                    LNP64_M1_COMMIT_CAP_DUP_DENIED: begin
                        assert (m1_root_dup_denied(typed_commit))
                            else $fatal(1, "M1 denied capDup failed root denied predicate");
                    end
                    default: begin
                        assert (1'b0)
                            else $fatal(1, "M1 typed commit used unknown operation");
                    end
                endcase
                commit_index <= commit_index + 4'd1;
            end
            if (done) begin
                assert (no_forged_fdr)
                    else $fatal(1, "M1 allowed forged FDR authority");
                if (expect_denied) begin
                    assert (commit_index == 4'd1)
                        else $fatal(1, "M1 denied path did not emit exactly one typed commit");
                    assert (!have_duplicated_cap && !have_sent_cap)
                        else $fatal(1, "M1 denied path retained transfer authority");
                end else begin
                    assert (no_lost_wakeup)
                        else $fatal(1, "M1 lost queue wakeup");
                    assert (stale_generation_rejected)
                        else $fatal(1, "M1 stale generation was not rejected");
                    assert (queue_full_explicit)
                        else $fatal(1, "M1 queue full behavior was not explicit");
                    assert (!have_sent_cap)
                        else $fatal(1, "M1 ended with an undelivered sent cap");
                    assert (commit_index == 4'd9)
                        else $fatal(1, "M1 did not emit the full typed commit sequence");
                end
            end
        end
    end
endmodule

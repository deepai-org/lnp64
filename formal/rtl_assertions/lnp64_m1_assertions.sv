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
    input lnp64_m1_cap_commit_t typed_commit,
    input lnp64_m1_state_projection_t typed_state_projection,
    input logic [3:0] rtl_state_projection,
    input logic [31:0] queue_generation,
    input logic [31:0] producer_fd_generation,
    input logic [31:0] consumer_fd_generation,
    input logic [63:0] producer_rights,
    input logic [63:0] consumer_rights,
    input logic sent_cap_valid,
    input logic minted_cap_valid,
    input lnp64_cap_t sent_cap_state,
    input lnp64_cap_t minted_cap_state,
    input logic created_object_created,
    input logic [31:0] created_object_generation,
    input logic wake_pending,
    input logic transfer_valid,
    input logic revoked_rejected,
    input logic failed_no_authority,
    input logic has_revoked_generation,
    input logic [31:0] revoked_generation
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
    localparam logic [3:0] M1_STATE_RESET = 4'd0;
    localparam logic [3:0] M1_STATE_BOOT = 4'd1;
    localparam logic [3:0] M1_STATE_CAP_DUP = 4'd2;
    localparam logic [3:0] M1_STATE_CAP_SEND = 4'd3;
    localparam logic [3:0] M1_STATE_CAP_RECV = 4'd4;
    localparam logic [3:0] M1_STATE_OBJECT_CREATE = 4'd10;
    localparam logic [3:0] M1_STATE_CAP_REVOKE = 4'd11;

    logic [3:0] commit_index;
    logic have_duplicated_cap;
    logic have_sent_cap;
    lnp64_m1_cap_commit_t duplicated_cap;
    lnp64_m1_cap_commit_t sent_cap;
    logic [3:0] previous_rtl_state_projection;
    logic [31:0] previous_producer_fd_generation;
    logic [31:0] previous_consumer_fd_generation;
    logic [31:0] previous_queue_generation;
    logic [63:0] previous_producer_rights;
    logic [63:0] previous_consumer_rights;
    lnp64_m1_state_projection_t previous_typed_state_projection;

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

    function automatic logic [15:0] expected_commit_status(input logic [7:0] op);
        unique case (op)
            LNP64_M1_COMMIT_REJECT_FULL: expected_commit_status = LNP64_ERR_EAGAIN;
            LNP64_M1_COMMIT_REJECT_STALE: expected_commit_status = LNP64_ERR_EREVOKED;
            LNP64_M1_COMMIT_CAP_DUP_DENIED: expected_commit_status = LNP64_ERR_EPERM;
            default: expected_commit_status = LNP64_ERR_OK;
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

    function automatic logic m1_ok_typed_commit(input logic [7:0] op);
        return typed_commit_valid &&
            typed_commit.op == op &&
            typed_commit.status == LNP64_ERR_OK;
    endfunction

    function automatic logic m1_zero_cap_projection(
        input logic [31:0] object_id,
        input logic [31:0] generation,
        input logic [31:0] domain_id,
        input logic [31:0] lineage_epoch,
        input logic sealed,
        input logic [63:0] rights
    );
        return object_id == 32'd0 &&
            generation == 32'd0 &&
            domain_id == 32'd0 &&
            lineage_epoch == 32'd0 &&
            !sealed &&
            rights == 64'd0;
    endfunction

    function automatic logic m1_cap_state_is_zero(input lnp64_cap_t cap);
        return cap.object_id == 32'd0 &&
            cap.object_gen == 32'd0 &&
            cap.fdr_gen == 32'd0 &&
            cap.domain_id == 32'd0 &&
            cap.domain_gen == 32'd0 &&
            cap.rights_mask == 64'd0 &&
            cap.lineage_epoch == 32'd0 &&
            !cap.sealed &&
            !cap.narrowable;
    endfunction

    function automatic logic m1_projection_matches_cap_state(
        input logic [31:0] object_id,
        input logic [31:0] generation,
        input logic [31:0] domain_id,
        input logic [31:0] lineage_epoch,
        input logic sealed,
        input logic [63:0] rights,
        input lnp64_cap_t cap
    );
        return object_id == cap.object_id &&
            generation == cap.fdr_gen &&
            cap.object_gen == cap.fdr_gen &&
            domain_id == cap.domain_id &&
            cap.domain_gen == M1_DOMAIN_GEN &&
            lineage_epoch == cap.lineage_epoch &&
            sealed == cap.sealed &&
            rights == cap.rights_mask &&
            cap.narrowable;
    endfunction

    function automatic logic m1_projection_cap_matches_commit(
        input logic [31:0] object_id,
        input logic [31:0] generation,
        input logic [31:0] domain_id,
        input logic [31:0] lineage_epoch,
        input logic sealed,
        input logic [63:0] rights,
        input lnp64_m1_cap_commit_t commit
    );
        return object_id == commit.object_id &&
            generation == commit.fdr_gen &&
            domain_id == commit.domain_id &&
            lineage_epoch == commit.lineage_epoch &&
            sealed == commit.sealed &&
            rights == commit.rights_mask;
    endfunction

    function automatic logic m1_sent_projection_slots_match(
        input lnp64_m1_state_projection_t left,
        input lnp64_m1_state_projection_t right
    );
        return left.sent_valid == right.sent_valid &&
            left.sent_object_id == right.sent_object_id &&
            left.sent_generation == right.sent_generation &&
            left.sent_domain_id == right.sent_domain_id &&
            left.sent_lineage_epoch == right.sent_lineage_epoch &&
            left.sent_sealed == right.sent_sealed &&
            left.sent_rights == right.sent_rights;
    endfunction

    function automatic logic m1_minted_projection_slots_match(
        input lnp64_m1_state_projection_t left,
        input lnp64_m1_state_projection_t right
    );
        return left.minted_valid == right.minted_valid &&
            left.minted_object_id == right.minted_object_id &&
            left.minted_generation == right.minted_generation &&
            left.minted_domain_id == right.minted_domain_id &&
            left.minted_lineage_epoch == right.minted_lineage_epoch &&
            left.minted_sealed == right.minted_sealed &&
            left.minted_rights == right.minted_rights;
    endfunction

    function automatic logic m1_authority_projection_slots_match(
        input lnp64_m1_state_projection_t left,
        input lnp64_m1_state_projection_t right
    );
        return left.root_object_id == right.root_object_id &&
            left.root_generation == right.root_generation &&
            left.root_domain_id == right.root_domain_id &&
            left.root_lineage_epoch == right.root_lineage_epoch &&
            left.root_sealed == right.root_sealed &&
            left.root_rights == right.root_rights &&
            left.consumer_object_id == right.consumer_object_id &&
            left.consumer_generation == right.consumer_generation &&
            left.consumer_domain_id == right.consumer_domain_id &&
            left.consumer_lineage_epoch == right.consumer_lineage_epoch &&
            left.consumer_sealed == right.consumer_sealed &&
            left.consumer_rights == right.consumer_rights &&
            left.sent_valid == right.sent_valid &&
            left.sent_object_id == right.sent_object_id &&
            left.sent_generation == right.sent_generation &&
            left.sent_domain_id == right.sent_domain_id &&
            left.sent_lineage_epoch == right.sent_lineage_epoch &&
            left.sent_sealed == right.sent_sealed &&
            left.sent_rights == right.sent_rights &&
            left.minted_valid == right.minted_valid &&
            left.minted_object_id == right.minted_object_id &&
            left.minted_generation == right.minted_generation &&
            left.minted_domain_id == right.minted_domain_id &&
            left.minted_lineage_epoch == right.minted_lineage_epoch &&
            left.minted_sealed == right.minted_sealed &&
            left.minted_rights == right.minted_rights;
    endfunction

    always_ff @(posedge clk) begin
        if (!reset_n) begin
            commit_index <= 4'd0;
            have_duplicated_cap <= 1'b0;
            have_sent_cap <= 1'b0;
            duplicated_cap <= '0;
            sent_cap <= '0;
            previous_rtl_state_projection <= M1_STATE_RESET;
            previous_producer_fd_generation <= 32'd0;
            previous_consumer_fd_generation <= 32'd0;
            previous_queue_generation <= 32'd0;
            previous_producer_rights <= 64'd0;
            previous_consumer_rights <= 64'd0;
            previous_typed_state_projection <= '0;
        end else begin
            // SG-AUTH: authority-bearing state changes are mediated by the M1 owner transitions.
            assert (typed_state_projection.object_gen == queue_generation)
                else $fatal(1, "M1 typed state projection object generation did not match RTL queue_generation");
            assert (typed_state_projection.root_object_id == M1_QUEUE_OBJECT_ID)
                else $fatal(1, "M1 typed state projection root cap object drifted");
            assert (typed_state_projection.root_generation == producer_fd_generation)
                else $fatal(1, "M1 typed state projection root generation did not match RTL producer_fd_generation");
            assert (typed_state_projection.root_domain_id == M1_ROOT_DOMAIN_ID)
                else $fatal(1, "M1 typed state projection root cap domain drifted");
            assert (typed_state_projection.root_lineage_epoch == M1_LINEAGE_EPOCH)
                else $fatal(1, "M1 typed state projection root cap lineage drifted");
            assert (!typed_state_projection.root_sealed)
                else $fatal(1, "M1 typed state projection root cap was sealed");
            assert (typed_state_projection.root_rights == producer_rights)
                else $fatal(1, "M1 typed state projection root rights did not match RTL producer_rights");
            assert (m1_rights_subset(typed_state_projection.root_rights, M1_ROOT_RIGHTS))
                else $fatal(1, "M1 typed state projection root rights broadened");
            assert (typed_state_projection.consumer_object_id == M1_QUEUE_OBJECT_ID)
                else $fatal(1, "M1 typed state projection consumer cap object drifted");
            assert (typed_state_projection.consumer_generation == consumer_fd_generation)
                else $fatal(1, "M1 typed state projection consumer generation did not match RTL consumer_fd_generation");
            assert (typed_state_projection.consumer_domain_id == M1_CONSUMER_DOMAIN_ID)
                else $fatal(1, "M1 typed state projection consumer cap domain drifted");
            assert (typed_state_projection.consumer_lineage_epoch == M1_LINEAGE_EPOCH)
                else $fatal(1, "M1 typed state projection consumer cap lineage drifted");
            assert (!typed_state_projection.consumer_sealed)
                else $fatal(1, "M1 typed state projection consumer cap was sealed");
            assert (typed_state_projection.consumer_rights == consumer_rights)
                else $fatal(1, "M1 typed state projection consumer rights did not match RTL consumer_rights");
            assert (m1_rights_subset(typed_state_projection.consumer_rights, M1_ROOT_RIGHTS))
                else $fatal(1, "M1 typed state projection consumer rights broadened");
            assert (typed_state_projection.sent_valid == sent_cap_valid)
                else $fatal(1, "M1 typed state projection sent_valid did not match RTL sent_cap_valid");
            assert (typed_state_projection.minted_valid == minted_cap_valid)
                else $fatal(1, "M1 typed state projection minted_valid did not match RTL minted_cap_valid");
            assert (typed_state_projection.created_object_created == created_object_created)
                else $fatal(1, "M1 typed state projection created_object_created did not match RTL created_object_created");
            assert (typed_state_projection.created_object_gen == created_object_generation)
                else $fatal(1, "M1 typed state projection created_object_gen did not match RTL created_object_generation");
            assert (typed_state_projection.wake_pending == wake_pending)
                else $fatal(1, "M1 typed state projection wake_pending did not match RTL wake_pending");
            assert (typed_state_projection.transfer_valid == transfer_valid)
                else $fatal(1, "M1 typed state projection transfer_valid did not match RTL transfer_valid");
            assert (typed_state_projection.stale_rejected == stale_generation_rejected)
                else $fatal(1, "M1 typed state projection stale_rejected did not match RTL stale_generation_rejected");
            assert (typed_state_projection.revoked_rejected == revoked_rejected)
                else $fatal(1, "M1 typed state projection revoked_rejected did not match RTL revoked_rejected");
            assert (typed_state_projection.failed_no_authority == failed_no_authority)
                else $fatal(1, "M1 typed state projection failed_no_authority did not match RTL failed_no_authority");
            assert (typed_state_projection.full_was_explicit == queue_full_explicit)
                else $fatal(1, "M1 typed state projection full_was_explicit did not match RTL queue_full_explicit");
            assert (typed_state_projection.has_revoked_generation == has_revoked_generation)
                else $fatal(1, "M1 typed state projection has_revoked_generation did not match RTL has_revoked_generation");
            assert (typed_state_projection.revoked_generation == revoked_generation)
                else $fatal(1, "M1 typed state projection revoked_generation did not match RTL revoked_generation");
            if (typed_state_projection.has_revoked_generation) begin
                assert (typed_state_projection.revoked_generation != 32'd0)
                    else $fatal(1, "M1 revoked-generation witness used zero generation");
            end else begin
                assert (typed_state_projection.revoked_generation == 32'd0)
                    else $fatal(1, "M1 revoked-generation projection was nonzero without a witness");
            end
            if (!typed_state_projection.sent_valid) begin
                assert (m1_cap_state_is_zero(sent_cap_state))
                    else $fatal(1, "M1 invalid sent-cap state retained authority bits");
                assert (m1_zero_cap_projection(
                    typed_state_projection.sent_object_id,
                    typed_state_projection.sent_generation,
                    typed_state_projection.sent_domain_id,
                    typed_state_projection.sent_lineage_epoch,
                    typed_state_projection.sent_sealed,
                    typed_state_projection.sent_rights
                )) else $fatal(1, "M1 invalid sent-cap projection carried authority fields");
            end else begin
                assert (m1_projection_matches_cap_state(
                    typed_state_projection.sent_object_id,
                    typed_state_projection.sent_generation,
                    typed_state_projection.sent_domain_id,
                    typed_state_projection.sent_lineage_epoch,
                    typed_state_projection.sent_sealed,
                    typed_state_projection.sent_rights,
                    sent_cap_state
                )) else $fatal(1, "M1 sent-cap projection did not match RTL sent_cap_state");
                assert (typed_state_projection.sent_object_id == M1_QUEUE_OBJECT_ID &&
                        typed_state_projection.sent_generation == consumer_fd_generation &&
                        typed_state_projection.sent_domain_id == M1_CONSUMER_DOMAIN_ID &&
                        typed_state_projection.sent_lineage_epoch == M1_LINEAGE_EPOCH &&
                        !typed_state_projection.sent_sealed &&
                        typed_state_projection.sent_rights == consumer_rights)
                    else $fatal(1, "M1 sent-cap projection did not match transferred consumer authority");
            end
            if (!typed_state_projection.minted_valid) begin
                assert (m1_cap_state_is_zero(minted_cap_state))
                    else $fatal(1, "M1 invalid minted-cap state retained authority bits");
                assert (m1_zero_cap_projection(
                    typed_state_projection.minted_object_id,
                    typed_state_projection.minted_generation,
                    typed_state_projection.minted_domain_id,
                    typed_state_projection.minted_lineage_epoch,
                    typed_state_projection.minted_sealed,
                    typed_state_projection.minted_rights
                )) else $fatal(1, "M1 invalid minted-cap projection carried authority fields");
            end else begin
                assert (m1_projection_matches_cap_state(
                    typed_state_projection.minted_object_id,
                    typed_state_projection.minted_generation,
                    typed_state_projection.minted_domain_id,
                    typed_state_projection.minted_lineage_epoch,
                    typed_state_projection.minted_sealed,
                    typed_state_projection.minted_rights,
                    minted_cap_state
                )) else $fatal(1, "M1 minted-cap projection did not match RTL minted_cap_state");
                assert (typed_state_projection.minted_object_id == M1_CREATED_OBJECT_ID &&
                        typed_state_projection.minted_generation == created_object_generation &&
                        typed_state_projection.minted_domain_id == M1_ROOT_DOMAIN_ID &&
                        typed_state_projection.minted_lineage_epoch == M1_LINEAGE_EPOCH &&
                        !typed_state_projection.minted_sealed &&
                        typed_state_projection.minted_rights == producer_rights)
                    else $fatal(1, "M1 minted-cap projection did not match root-created authority");
            end
            if (typed_commit_valid) begin
                assert (typed_state_projection.op == typed_commit.op &&
                        typed_state_projection.status == typed_commit.status)
                    else $fatal(1, "M1 typed state projection transition tag drifted from commit");
            end

            if (typed_state_projection.root_generation != previous_producer_fd_generation) begin
                assert (previous_rtl_state_projection == M1_STATE_BOOT ||
                        previous_rtl_state_projection == M1_STATE_CAP_REVOKE)
                    else $fatal(1, "M1 producer FDR generation changed outside boot or capRevoke authority path");
                if (previous_rtl_state_projection == M1_STATE_CAP_REVOKE) begin
                    assert (m1_ok_typed_commit(LNP64_M1_COMMIT_CAP_REVOKE))
                        else $fatal(1, "M1 producer FDR generation changed without an OK capRevoke commit");
                    assert (typed_state_projection.root_generation == previous_producer_fd_generation + 32'd1)
                        else $fatal(1, "M1 capRevoke did not advance producer FDR generation exactly once");
                end
                assert (typed_state_projection.root_generation == queue_generation)
                    else $fatal(1, "M1 producer FDR generation diverged from queue generation");
            end
            if (typed_state_projection.root_rights != previous_producer_rights) begin
                assert (previous_rtl_state_projection == M1_STATE_BOOT)
                    else $fatal(1, "M1 producer rights changed outside boot authority initialization");
                assert (typed_state_projection.root_rights == M1_ROOT_RIGHTS ||
                        typed_state_projection.root_rights == (M1_RIGHT_PUSH | M1_RIGHT_PULL))
                    else $fatal(1, "M1 producer rights were not root rights or explicit denied-path rights");
            end
            if (typed_state_projection.consumer_generation != previous_consumer_fd_generation) begin
                assert (previous_rtl_state_projection == M1_STATE_CAP_DUP)
                    else $fatal(1, "M1 consumer FDR generation changed outside capDup owner path");
                assert (m1_ok_typed_commit(LNP64_M1_COMMIT_CAP_DUP))
                    else $fatal(1, "M1 consumer FDR generation changed without an OK capDup commit");
                assert (typed_state_projection.consumer_generation == queue_generation)
                    else $fatal(1, "M1 consumer FDR generation diverged from live queue generation");
            end
            if (typed_state_projection.consumer_rights != previous_consumer_rights) begin
                assert (previous_rtl_state_projection == M1_STATE_CAP_DUP)
                    else $fatal(1, "M1 consumer rights changed outside capDup owner path");
                assert (m1_ok_typed_commit(LNP64_M1_COMMIT_CAP_DUP))
                    else $fatal(1, "M1 consumer rights changed without an OK capDup commit");
                assert (typed_state_projection.consumer_rights == M1_RIGHT_PULL)
                    else $fatal(1, "M1 consumer rights changed to something other than narrowed pull authority");
            end
            if (typed_state_projection.object_gen != previous_queue_generation) begin
                assert (previous_rtl_state_projection == M1_STATE_BOOT ||
                        previous_rtl_state_projection == M1_STATE_CAP_REVOKE)
                    else $fatal(1, "M1 object generation changed outside boot or capRevoke owner path");
                if (previous_rtl_state_projection == M1_STATE_CAP_REVOKE) begin
                    assert (m1_ok_typed_commit(LNP64_M1_COMMIT_CAP_REVOKE))
                        else $fatal(1, "M1 object generation changed without an OK capRevoke commit");
                    assert (typed_state_projection.object_gen == previous_queue_generation + 32'd1)
                        else $fatal(1, "M1 capRevoke did not advance object generation exactly once");
                end
            end
            if (typed_state_projection.sent_valid != previous_typed_state_projection.sent_valid) begin
                if (typed_state_projection.sent_valid) begin
                    assert (previous_rtl_state_projection == M1_STATE_CAP_SEND)
                        else $fatal(1, "M1 sent-cap validity set outside capSend owner path");
                    assert (m1_ok_typed_commit(LNP64_M1_COMMIT_CAP_SEND))
                        else $fatal(1, "M1 sent-cap validity set without an OK capSend commit");
                end else begin
                    assert (previous_rtl_state_projection == M1_STATE_CAP_RECV)
                        else $fatal(1, "M1 sent-cap validity cleared outside capRecv owner path");
                    assert (m1_ok_typed_commit(LNP64_M1_COMMIT_CAP_RECV))
                        else $fatal(1, "M1 sent-cap validity cleared without an OK capRecv commit");
                end
            end
            if (!m1_sent_projection_slots_match(
                typed_state_projection,
                previous_typed_state_projection
            )) begin
                if (typed_state_projection.sent_valid) begin
                    assert (previous_rtl_state_projection == M1_STATE_CAP_SEND)
                        else $fatal(1, "M1 sent-cap payload changed outside capSend owner path");
                    assert (m1_ok_typed_commit(LNP64_M1_COMMIT_CAP_SEND))
                        else $fatal(1, "M1 sent-cap payload changed without an OK capSend commit");
                    assert (m1_projection_cap_matches_commit(
                        typed_state_projection.sent_object_id,
                        typed_state_projection.sent_generation,
                        typed_state_projection.sent_domain_id,
                        typed_state_projection.sent_lineage_epoch,
                        typed_state_projection.sent_sealed,
                        typed_state_projection.sent_rights,
                        typed_commit
                    )) else $fatal(1, "M1 sent-cap payload did not match capSend commit");
                end else begin
                    assert (previous_rtl_state_projection == M1_STATE_CAP_RECV)
                        else $fatal(1, "M1 sent-cap payload cleared outside capRecv owner path");
                    assert (m1_ok_typed_commit(LNP64_M1_COMMIT_CAP_RECV))
                        else $fatal(1, "M1 sent-cap payload cleared without an OK capRecv commit");
                end
            end
            if (typed_state_projection.transfer_valid != previous_typed_state_projection.transfer_valid) begin
                assert (typed_state_projection.transfer_valid)
                    else $fatal(1, "M1 transfer-valid witness was cleared after publication");
                assert (previous_rtl_state_projection == M1_STATE_CAP_SEND)
                    else $fatal(1, "M1 transfer-valid witness set outside capSend owner path");
                assert (m1_ok_typed_commit(LNP64_M1_COMMIT_CAP_SEND))
                    else $fatal(1, "M1 transfer-valid witness set without an OK capSend commit");
            end
            if (typed_state_projection.minted_valid != previous_typed_state_projection.minted_valid) begin
                assert (typed_state_projection.minted_valid)
                    else $fatal(1, "M1 minted-cap validity was cleared after publication");
                assert (previous_rtl_state_projection == M1_STATE_OBJECT_CREATE)
                    else $fatal(1, "M1 minted-cap validity set outside objectCreate owner path");
                assert (m1_ok_typed_commit(LNP64_M1_COMMIT_OBJECT_CREATE))
                    else $fatal(1, "M1 minted-cap validity set without an OK objectCreate commit");
            end
            if (!m1_minted_projection_slots_match(
                typed_state_projection,
                previous_typed_state_projection
            )) begin
                assert (typed_state_projection.minted_valid)
                    else $fatal(1, "M1 minted-cap payload was cleared after publication");
                assert (previous_rtl_state_projection == M1_STATE_OBJECT_CREATE)
                    else $fatal(1, "M1 minted-cap payload changed outside objectCreate owner path");
                assert (m1_ok_typed_commit(LNP64_M1_COMMIT_OBJECT_CREATE))
                    else $fatal(1, "M1 minted-cap payload changed without an OK objectCreate commit");
                assert (m1_projection_cap_matches_commit(
                    typed_state_projection.minted_object_id,
                    typed_state_projection.minted_generation,
                    typed_state_projection.minted_domain_id,
                    typed_state_projection.minted_lineage_epoch,
                    typed_state_projection.minted_sealed,
                    typed_state_projection.minted_rights,
                    typed_commit
                )) else $fatal(1, "M1 minted-cap payload did not match objectCreate commit");
            end
            if (typed_state_projection.created_object_created != previous_typed_state_projection.created_object_created) begin
                assert (typed_state_projection.created_object_created)
                    else $fatal(1, "M1 created-object witness was cleared after publication");
                assert (previous_rtl_state_projection == M1_STATE_OBJECT_CREATE)
                    else $fatal(1, "M1 created-object witness set outside objectCreate owner path");
                assert (m1_ok_typed_commit(LNP64_M1_COMMIT_OBJECT_CREATE))
                    else $fatal(1, "M1 created-object witness set without an OK objectCreate commit");
            end

            assert (exactly_one_scheduler_location || !done)
                else $fatal(1, "M1 scheduler location invariant failed");
            if (typed_commit_valid) begin
                assert (typed_commit.op == expected_commit_op(commit_index))
                    else $fatal(1, "M1 typed commit sequence drifted");
                assert (typed_commit.status == expected_commit_status(typed_commit.op))
                    else $fatal(1, "M1 typed commit status did not match operation");
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
                if (typed_commit.status != LNP64_ERR_OK) begin
                    assert (m1_authority_projection_slots_match(
                        typed_state_projection,
                        previous_typed_state_projection
                    )) else $fatal(1, "M1 non-OK commit changed authority projection slots");
                end

                unique case (typed_commit.op)
                    LNP64_M1_COMMIT_CAP_DUP: begin
                        assert (m1_consumer_pull_authority(typed_commit))
                            else $fatal(1, "M1 consumer commit failed pull-authority predicate");
                        assert (m1_projection_cap_matches_commit(
                            typed_state_projection.consumer_object_id,
                            typed_state_projection.consumer_generation,
                            typed_state_projection.consumer_domain_id,
                            typed_state_projection.consumer_lineage_epoch,
                            typed_state_projection.consumer_sealed,
                            typed_state_projection.consumer_rights,
                            typed_commit
                        )) else $fatal(1, "M1 capDup commit did not match consumer post-state projection");
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
                        assert (typed_state_projection.sent_valid)
                            else $fatal(1, "M1 capSend did not publish a sent-cap projection");
                        assert (typed_state_projection.transfer_valid)
                            else $fatal(1, "M1 capSend did not publish transfer-valid witness");
                        assert (m1_projection_cap_matches_commit(
                            typed_state_projection.sent_object_id,
                            typed_state_projection.sent_generation,
                            typed_state_projection.sent_domain_id,
                            typed_state_projection.sent_lineage_epoch,
                            typed_state_projection.sent_sealed,
                            typed_state_projection.sent_rights,
                            typed_commit
                        )) else $fatal(1, "M1 capSend commit did not match sent-cap post-state projection");
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
                        assert (!typed_state_projection.sent_valid)
                            else $fatal(1, "M1 capRecv left the sent-cap projection valid");
                        assert (typed_state_projection.transfer_valid)
                            else $fatal(1, "M1 capRecv did not preserve transfer-valid witness");
                        assert (m1_projection_cap_matches_commit(
                            typed_state_projection.consumer_object_id,
                            typed_state_projection.consumer_generation,
                            typed_state_projection.consumer_domain_id,
                            typed_state_projection.consumer_lineage_epoch,
                            typed_state_projection.consumer_sealed,
                            typed_state_projection.consumer_rights,
                            typed_commit
                        )) else $fatal(1, "M1 capRecv commit did not match consumer post-state projection");
                        have_sent_cap <= 1'b0;
                    end
                    LNP64_M1_COMMIT_PULL: begin
                        assert (m1_consumer_pull_authority(typed_commit))
                            else $fatal(1, "M1 consumer commit failed pull-authority predicate");
                        assert (!typed_state_projection.wake_pending)
                            else $fatal(1, "M1 pull commit did not clear wake_pending in post-state projection");
                    end
                    LNP64_M1_COMMIT_PUSH: begin
                        assert (m1_root_live_authority(typed_commit))
                            else $fatal(1, "M1 push failed root live-authority predicate");
                        assert (typed_state_projection.wake_pending)
                            else $fatal(1, "M1 push commit did not set wake_pending in post-state projection");
                    end
                    LNP64_M1_COMMIT_REJECT_FULL: begin
                        assert (m1_root_queue_full_reject(typed_commit))
                            else $fatal(1, "M1 rejectFull failed root queue-full predicate");
                        assert (typed_state_projection.full_was_explicit)
                            else $fatal(1, "M1 rejectFull commit did not publish full_was_explicit");
                    end
                    LNP64_M1_COMMIT_OBJECT_CREATE: begin
                        assert (m1_root_object_create(typed_commit))
                            else $fatal(1, "M1 objectCreate failed root mint predicate");
                        assert (typed_state_projection.created_object_created &&
                                typed_state_projection.minted_valid)
                            else $fatal(1, "M1 objectCreate did not publish created/minted projections");
                        assert (m1_projection_cap_matches_commit(
                            typed_state_projection.minted_object_id,
                            typed_state_projection.minted_generation,
                            typed_state_projection.minted_domain_id,
                            typed_state_projection.minted_lineage_epoch,
                            typed_state_projection.minted_sealed,
                            typed_state_projection.minted_rights,
                            typed_commit
                        )) else $fatal(1, "M1 objectCreate commit did not match minted-cap post-state projection");
                    end
                    LNP64_M1_COMMIT_CAP_REVOKE: begin
                        assert (m1_root_revoke_commit(typed_commit))
                            else $fatal(1, "M1 revoke failed root revoke-commit predicate");
                        assert (typed_state_projection.object_gen == typed_commit.object_gen &&
                                typed_state_projection.root_generation == typed_commit.object_gen &&
                                typed_state_projection.has_revoked_generation &&
                                typed_state_projection.revoked_generation == typed_commit.fdr_gen)
                            else $fatal(1, "M1 capRevoke commit did not match revocation post-state projection");
                    end
                    LNP64_M1_COMMIT_REJECT_STALE: begin
                        assert (m1_consumer_stale_reject(typed_commit))
                            else $fatal(1, "M1 rejectStale failed consumer stale-reject predicate");
                        assert (typed_state_projection.stale_rejected)
                            else $fatal(1, "M1 rejectStale commit did not publish stale_rejected");
                    end
                    LNP64_M1_COMMIT_CAP_DUP_DENIED: begin
                        assert (m1_root_dup_denied(typed_commit))
                            else $fatal(1, "M1 denied capDup failed root denied predicate");
                        assert (typed_state_projection.failed_no_authority)
                            else $fatal(1, "M1 denied capDup did not publish failed_no_authority");
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
            previous_rtl_state_projection <= rtl_state_projection;
            previous_producer_fd_generation <= typed_state_projection.root_generation;
            previous_consumer_fd_generation <= typed_state_projection.consumer_generation;
            previous_queue_generation <= typed_state_projection.object_gen;
            previous_producer_rights <= typed_state_projection.root_rights;
            previous_consumer_rights <= typed_state_projection.consumer_rights;
            previous_typed_state_projection <= typed_state_projection;
        end
    end
endmodule

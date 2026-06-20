`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_fail_closed_engine #(
    parameter logic [15:0] ENGINE_ID = 16'd0,
    parameter logic [15:0] ERRNO_VALUE = LNP64_ERR_ENOTSUP,
    parameter logic [15:0] STATUS_VALUE = LNP64_STATUS_UNSUPPORTED
) (
    input  logic clk,
    input  logic reset_n,
    input  logic cmd_valid,
    output logic cmd_ready,
    input  lnp64_cmd_t cmd,
    output logic rsp_valid,
    input  logic rsp_ready,
    output lnp64_rsp_t rsp,
    output logic fault_valid,
    input  logic fault_ready,
    output lnp64_fault_t fault,
    output logic [31:0] accepted_counter,
    output logic [31:0] fault_counter
);
    logic have_rsp;
    logic have_fault;
    lnp64_rsp_t rsp_reg;
    lnp64_fault_t fault_reg;

    assign cmd_ready = reset_n && !have_rsp && !have_fault;
    assign rsp_valid = have_rsp;
    assign rsp = rsp_reg;
    assign fault_valid = have_fault;
    assign fault = fault_reg;

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            have_rsp <= 1'b0;
            have_fault <= 1'b0;
            rsp_reg <= '0;
            fault_reg <= '0;
            accepted_counter <= 32'd0;
            fault_counter <= 32'd0;
        end else begin
            if (cmd_valid && cmd_ready) begin
                have_rsp <= 1'b1;
                accepted_counter <= accepted_counter + 32'd1;
                rsp_reg.op_id <= cmd.op_id;
                rsp_reg.tile_id <= cmd.tile_id;
                rsp_reg.pid <= cmd.pid;
                rsp_reg.tid <= cmd.tid;
                rsp_reg.domain_id <= cmd.domain_id;
                rsp_reg.domain_gen <= cmd.domain_gen;
                rsp_reg.result_reg <= cmd.result_reg;
                rsp_reg.result_value <= 64'd0;
                rsp_reg.errno_value <= ERRNO_VALUE;
                rsp_reg.status <= STATUS_VALUE;
                rsp_reg.event_mask <= 64'd0;
                fault_reg.fault_id <= accepted_counter + 32'd1;
                fault_reg.tile_id <= cmd.tile_id;
                fault_reg.op_id <= cmd.op_id;
                fault_reg.pid <= cmd.pid;
                fault_reg.tid <= cmd.tid;
                fault_reg.domain_id <= cmd.domain_id;
                fault_reg.domain_gen <= cmd.domain_gen;
                fault_reg.fault_code <= ERRNO_VALUE;
                fault_reg.source <= ENGINE_ID;
                fault_reg.detail <= 64'd0;
                if (STATUS_VALUE == LNP64_STATUS_FAULT) begin
                    have_fault <= 1'b1;
                end
            end
            if (have_rsp && rsp_ready) begin
                have_rsp <= 1'b0;
            end
            if (have_fault && fault_ready) begin
                have_fault <= 1'b0;
                fault_counter <= fault_counter + 32'd1;
            end
        end
    end
endmodule

module lnp64_engine_router (
    input  logic clk,
    input  logic reset_n,
    input  logic cmd_valid,
    output logic cmd_ready,
    input  lnp64_cmd_t cmd,
    output logic rsp_valid,
    input  logic rsp_ready,
    output lnp64_rsp_t rsp,
    output logic cap_cmd_valid,
    input  logic cap_cmd_ready,
    output lnp64_cmd_t cap_cmd,
    input  logic cap_rsp_valid,
    output logic cap_rsp_ready,
    input  lnp64_rsp_t cap_rsp,
    output logic object_cmd_valid,
    input  logic object_cmd_ready,
    output lnp64_cmd_t object_cmd,
    input  logic object_rsp_valid,
    output logic object_rsp_ready,
    input  lnp64_rsp_t object_rsp,
    output logic domain_cmd_valid,
    input  logic domain_cmd_ready,
    output lnp64_cmd_t domain_cmd,
    input  logic domain_rsp_valid,
    output logic domain_rsp_ready,
    input  lnp64_rsp_t domain_rsp,
    output logic heap_cmd_valid,
    input  logic heap_cmd_ready,
    output lnp64_cmd_t heap_cmd,
    input  logic heap_rsp_valid,
    output logic heap_rsp_ready,
    input  lnp64_rsp_t heap_rsp,
    output logic vma_cmd_valid,
    input  logic vma_cmd_ready,
    output lnp64_cmd_t vma_cmd,
    input  logic vma_rsp_valid,
    output logic vma_rsp_ready,
    input  lnp64_rsp_t vma_rsp,
    output logic dma_cmd_valid,
    input  logic dma_cmd_ready,
    output lnp64_cmd_t dma_cmd,
    input  logic dma_rsp_valid,
    output logic dma_rsp_ready,
    input  lnp64_rsp_t dma_rsp,
    output logic fault_valid,
    input  logic fault_ready,
    output lnp64_fault_t fault,
    output logic [31:0] routed_counter
);
    logic route_cap;
    logic route_object;
    logic route_domain;
    logic route_heap;
    logic route_vma;
    logic route_dma;
    logic route_fault;
    logic route_default;
    logic fault_cmd_valid;
    logic fault_cmd_ready;
    logic fault_rsp_valid;
    logic fault_rsp_ready;
    lnp64_rsp_t fault_rsp;
    logic fault_fault_valid;
    lnp64_fault_t fault_fault;
    logic unsupported_cmd_valid;
    logic unsupported_cmd_ready;
    logic unsupported_rsp_valid;
    logic unsupported_rsp_ready;
    lnp64_rsp_t unsupported_rsp;
    logic unsupported_fault_valid;
    lnp64_fault_t unsupported_fault;
    lnp64_cmd_t fault_cmd;
    lnp64_cmd_t unsupported_cmd;
    logic [31:0] fault_accepts;
    logic [31:0] unsupported_accepts;
    logic [31:0] fault_faults;
    logic [31:0] unsupported_faults;

    assign route_cap = cmd.opcode == LNP64_OP_CAP_DUP ||
        cmd.opcode == LNP64_OP_CAP_SEND ||
        cmd.opcode == LNP64_OP_CAP_RECV ||
        cmd.opcode == LNP64_OP_CAP_REVOKE;
    assign route_object = cmd.opcode == LNP64_OP_OBJECT_CTL;
    assign route_domain = cmd.opcode == LNP64_OP_DOMAIN_CTL;
    assign route_heap = cmd.opcode == LNP64_OP_ALLOC ||
        cmd.opcode == LNP64_OP_ALLOC_EX ||
        cmd.opcode == LNP64_OP_ALLOC_SIZE ||
        cmd.opcode == LNP64_OP_FREE;
    assign route_vma = cmd.opcode == LNP64_OP_MMAP ||
        cmd.opcode == LNP64_OP_MPROTECT;
    assign route_dma = cmd.opcode == LNP64_OP_DMA_CTL;
    assign route_fault = cmd.opcode == LNP64_OP_FAULT_INJECT;
    assign route_default = !route_cap && !route_object && !route_domain &&
        !route_heap && !route_vma && !route_dma && !route_fault;

    assign cap_cmd_valid = cmd_valid && route_cap;
    assign object_cmd_valid = cmd_valid && route_object;
    assign domain_cmd_valid = cmd_valid && route_domain;
    assign heap_cmd_valid = cmd_valid && route_heap;
    assign vma_cmd_valid = cmd_valid && route_vma;
    assign dma_cmd_valid = cmd_valid && route_dma;
    assign fault_cmd_valid = cmd_valid && route_fault;
    assign unsupported_cmd_valid = cmd_valid && route_default;

    always_comb begin
        cap_cmd = cmd;
        cap_cmd.destination_engine = LNP64_ENGINE_CAP;
        object_cmd = cmd;
        object_cmd.destination_engine = LNP64_ENGINE_OBJECT;
        domain_cmd = cmd;
        domain_cmd.destination_engine = LNP64_ENGINE_DOMAIN;
        heap_cmd = cmd;
        heap_cmd.destination_engine = LNP64_ENGINE_HEAP;
        vma_cmd = cmd;
        vma_cmd.destination_engine = LNP64_ENGINE_VMA;
        dma_cmd = cmd;
        dma_cmd.destination_engine = LNP64_ENGINE_DMA;
        fault_cmd = cmd;
        fault_cmd.destination_engine = LNP64_ENGINE_FAULT;
        unsupported_cmd = cmd;
        unsupported_cmd.destination_engine = LNP64_ENGINE_UNSUPPORTED;
    end

    assign cmd_ready =
        route_cap ? cap_cmd_ready :
        route_object ? object_cmd_ready :
        route_domain ? domain_cmd_ready :
        route_heap ? heap_cmd_ready :
        route_vma ? vma_cmd_ready :
        route_dma ? dma_cmd_ready :
        route_fault ? fault_cmd_ready :
        unsupported_cmd_ready;

    assign rsp_valid = cap_rsp_valid || object_rsp_valid || domain_rsp_valid ||
        heap_rsp_valid || vma_rsp_valid || dma_rsp_valid || fault_rsp_valid ||
        unsupported_rsp_valid;
    assign rsp =
        cap_rsp_valid ? cap_rsp :
        object_rsp_valid ? object_rsp :
        domain_rsp_valid ? domain_rsp :
        heap_rsp_valid ? heap_rsp :
        vma_rsp_valid ? vma_rsp :
        dma_rsp_valid ? dma_rsp :
        fault_rsp_valid ? fault_rsp :
        unsupported_rsp;

    assign cap_rsp_ready = rsp_ready && cap_rsp_valid;
    assign object_rsp_ready = rsp_ready && !cap_rsp_valid && object_rsp_valid;
    assign domain_rsp_ready = rsp_ready && !cap_rsp_valid && !object_rsp_valid &&
        domain_rsp_valid;
    assign heap_rsp_ready = rsp_ready && !cap_rsp_valid && !object_rsp_valid &&
        !domain_rsp_valid && heap_rsp_valid;
    assign fault_rsp_ready = rsp_ready && !cap_rsp_valid && !object_rsp_valid &&
        !domain_rsp_valid && !heap_rsp_valid && !vma_rsp_valid && !dma_rsp_valid &&
        fault_rsp_valid;
    assign vma_rsp_ready = rsp_ready && !cap_rsp_valid && !object_rsp_valid &&
        !domain_rsp_valid && !heap_rsp_valid && vma_rsp_valid;
    assign dma_rsp_ready = rsp_ready && !cap_rsp_valid && !object_rsp_valid &&
        !domain_rsp_valid && !heap_rsp_valid && !vma_rsp_valid && dma_rsp_valid;
    assign unsupported_rsp_ready = rsp_ready && !cap_rsp_valid && !object_rsp_valid &&
        !domain_rsp_valid && !heap_rsp_valid && !vma_rsp_valid && !dma_rsp_valid &&
        !fault_rsp_valid && unsupported_rsp_valid;

    assign fault_valid = fault_fault_valid || unsupported_fault_valid;
    assign fault = fault_fault_valid ? fault_fault : unsupported_fault;

    lnp64_fail_closed_engine #(
        .ENGINE_ID(LNP64_ENGINE_FAULT),
        .ERRNO_VALUE(LNP64_ERR_EFAULT),
        .STATUS_VALUE(LNP64_STATUS_FAULT)
    ) fault_lane_i (
        .clk(clk),
        .reset_n(reset_n),
        .cmd_valid(fault_cmd_valid),
        .cmd_ready(fault_cmd_ready),
        .cmd(fault_cmd),
        .rsp_valid(fault_rsp_valid),
        .rsp_ready(fault_rsp_ready),
        .rsp(fault_rsp),
        .fault_valid(fault_fault_valid),
        .fault_ready(fault_ready),
        .fault(fault_fault),
        .accepted_counter(fault_accepts),
        .fault_counter(fault_faults)
    );

    lnp64_fail_closed_engine #(
        .ENGINE_ID(LNP64_ENGINE_UNSUPPORTED),
        .ERRNO_VALUE(LNP64_ERR_ENOTSUP),
        .STATUS_VALUE(LNP64_STATUS_UNSUPPORTED)
    ) unsupported_lane_i (
        .clk(clk),
        .reset_n(reset_n),
        .cmd_valid(unsupported_cmd_valid),
        .cmd_ready(unsupported_cmd_ready),
        .cmd(unsupported_cmd),
        .rsp_valid(unsupported_rsp_valid),
        .rsp_ready(unsupported_rsp_ready),
        .rsp(unsupported_rsp),
        .fault_valid(unsupported_fault_valid),
        .fault_ready(fault_ready && !fault_fault_valid),
        .fault(unsupported_fault),
        .accepted_counter(unsupported_accepts),
        .fault_counter(unsupported_faults)
    );

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            routed_counter <= 32'd0;
        end else if (cmd_valid && cmd_ready) begin
                routed_counter <= routed_counter + 32'd1;
        end
    end
endmodule

module lnp64_completion_router (
    input  logic clk,
    input  logic reset_n,
    input  logic rsp_valid,
    input  lnp64_rsp_t rsp,
    output lnp64_completion_t completion,
    output logic completion_valid
);
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            completion <= '0;
            completion_valid <= 1'b0;
        end else begin
            completion_valid <= rsp_valid;
            if (rsp_valid) begin
                completion.op_id <= rsp.op_id;
                completion.tile_id <= rsp.tile_id;
                completion.pid <= rsp.pid;
                completion.tid <= rsp.tid;
                completion.domain_id <= rsp.domain_id;
                completion.domain_gen <= rsp.domain_gen;
                completion.target <= 16'd1;
                completion.status <= rsp.status;
                completion.errno_value <= rsp.errno_value;
                completion.value <= rsp.result_value;
            end
        end
    end
endmodule

module lnp64_errno_writeback (
    input  logic clk,
    input  logic reset_n,
    input  logic rsp_valid,
    input  lnp64_rsp_t rsp,
    output logic [15:0] errno_value
);
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            errno_value <= LNP64_ERR_OK;
        end else if (rsp_valid && rsp.status != LNP64_STATUS_OK) begin
            errno_value <= rsp.errno_value;
        end
    end
endmodule

module lnp64_scheduler #(
    parameter int CORE_TILE_COUNT = 2
) (
    input  logic clk,
    input  logic reset_n,
    input  logic boot_valid,
    input  lnp64_thread_sched_t boot_context,
    input  logic [CORE_TILE_COUNT-1:0] submit_valid,
    input  lnp64_thread_sched_t submit_record [CORE_TILE_COUNT],
    input  logic [CORE_TILE_COUNT-1:0] park_submit_valid,
    input  lnp64_thread_sched_t park_submit_record [CORE_TILE_COUNT],
    input  logic wake_event_valid,
    input  lnp64_event_t wake_event,
    input  logic [CORE_TILE_COUNT-1:0] tile_idle,
    input  logic [CORE_TILE_COUNT-1:0] tile_running,
    input  logic [CORE_TILE_COUNT-1:0] tile_parked,
    input  logic [CORE_TILE_COUNT-1:0] tile_faulted,
    output logic [CORE_TILE_COUNT-1:0] issue_valid,
    output logic [CORE_TILE_COUNT*32-1:0] issue_tid_flat,
    output lnp64_thread_sched_t issue_record [CORE_TILE_COUNT],
    output logic wake_issue_valid,
    output logic exactly_one_location,
    output logic pid1_runnable,
    output logic pid1_parked,
    output logic no_duplicate_issue,
    output logic tile1_schedulable_idle,
    output logic tile_fault_isolated
);
    integer sched_i;
    integer sched_state_i;
    lnp64_thread_sched_t pid1_record;
    lnp64_thread_sched_t child_record;
    lnp64_thread_sched_t child_issue_record;
    logic child_runnable;
    logic child_issue_pending;
    logic child_issue_valid;
    int unsigned child_issue_tile;

    function automatic logic is_live_sched_record(input lnp64_thread_sched_t record);
        begin
            is_live_sched_record =
                record.pid != 32'd0 &&
                record.tid != 32'd0 &&
                record.domain_id != 32'd0 &&
                record.domain_gen != 32'd0 &&
                record.migration_generation != 32'd0 &&
                record.dispatch_eligible;
        end
    endfunction

    function automatic logic is_pid1_record(input lnp64_thread_sched_t record);
        begin
            is_pid1_record =
                is_live_sched_record(record) &&
                record.pid == 32'd1 &&
                record.tid == 32'd1;
        end
    endfunction

    function automatic logic is_child_record(input lnp64_thread_sched_t record);
        begin
            is_child_record =
                is_live_sched_record(record) &&
                record.pid == 32'd1 &&
                record.tid == 32'd2;
        end
    endfunction

    function automatic logic is_pid1_wake(input lnp64_event_t wake_record);
        begin
            is_pid1_wake =
                wake_record.pid == 32'd1 &&
                wake_record.tid == 32'd1 &&
                wake_record.domain_id == pid1_record.domain_id &&
                wake_record.domain_gen == pid1_record.domain_gen &&
                wake_record.status == LNP64_STATUS_EVENT;
        end
    endfunction

    always_comb begin
        issue_valid = '0;
        issue_tid_flat = '0;
        for (sched_i = 0; sched_i < CORE_TILE_COUNT; sched_i = sched_i + 1) begin
            issue_valid[sched_i] = 1'b0;
            issue_record[sched_i] = '0;
        end

        child_issue_pending = child_runnable;
        child_issue_record = child_record;
        child_issue_tile = child_record.tile_id;
        for (sched_i = 0; sched_i < CORE_TILE_COUNT; sched_i = sched_i + 1) begin
            if (submit_valid[sched_i] &&
                is_child_record(submit_record[sched_i]) &&
                submit_record[sched_i].tile_id == sched_i[31:0]) begin
                child_issue_pending = 1'b1;
                child_issue_record = submit_record[sched_i];
                child_issue_record.state = 16'd1;
                child_issue_record.active_location = submit_record[sched_i].tile_id;
                child_issue_tile = sched_i;
            end
        end
        child_issue_valid = child_issue_pending &&
            child_issue_tile < CORE_TILE_COUNT &&
            !tile_faulted[child_issue_tile];

        if (child_issue_valid) begin
            issue_valid[child_issue_tile] = 1'b1;
            issue_tid_flat[child_issue_tile*32 +: 32] = child_issue_record.tid;
            issue_record[child_issue_tile] = child_issue_record;
            issue_record[child_issue_tile].state = 16'd1;
            issue_record[child_issue_tile].tile_id = child_issue_tile[31:0];
            issue_record[child_issue_tile].active_location = child_issue_tile[31:0];
        end else if (pid1_runnable && !tile_faulted[0]) begin
            issue_valid[0] = 1'b1;
            issue_tid_flat[31:0] = 32'd1;
            issue_record[0] = pid1_record;
            issue_record[0].state = 16'd1;
            issue_record[0].tile_id = 32'd0;
            issue_record[0].active_location = 32'd0;
        end

        no_duplicate_issue = 1'b1;
        for (sched_i = 0; sched_i < CORE_TILE_COUNT; sched_i = sched_i + 1) begin
            if (issue_valid[sched_i]) begin
                for (int unsigned dup_i = sched_i + 1; dup_i < CORE_TILE_COUNT; dup_i = dup_i + 1) begin
                    if (issue_valid[dup_i] &&
                        issue_record[dup_i].pid == issue_record[sched_i].pid &&
                        issue_record[dup_i].tid == issue_record[sched_i].tid) begin
                        no_duplicate_issue = 1'b0;
                    end
                end
            end
        end
        tile1_schedulable_idle = (CORE_TILE_COUNT > 1) &&
            tile_idle[1] && !tile_running[1] && !tile_parked[1] && !tile_faulted[1];
        tile_fault_isolated = (CORE_TILE_COUNT < 2) || !tile_faulted[1] ||
            (!tile_faulted[0] && (pid1_runnable || pid1_parked) && is_pid1_record(pid1_record));
        wake_issue_valid = wake_event_valid && pid1_parked && is_pid1_wake(wake_event);
    end

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            pid1_runnable <= 1'b0;
            pid1_parked <= 1'b0;
            pid1_record <= '0;
            child_record <= '0;
            child_runnable <= 1'b0;
            exactly_one_location <= 1'b0;
        end else begin
            if (boot_valid) begin
                pid1_runnable <= 1'b1;
                pid1_parked <= 1'b0;
                child_runnable <= 1'b0;
                child_record <= '0;
                pid1_record <= boot_context;
                pid1_record.state <= 16'd1;
                pid1_record.tile_id <= 32'd0;
                pid1_record.active_location <= 32'd0;
            end

            for (sched_state_i = 0; sched_state_i < CORE_TILE_COUNT; sched_state_i = sched_state_i + 1) begin
                if (submit_valid[sched_state_i] &&
                    is_pid1_record(submit_record[sched_state_i]) &&
                    submit_record[sched_state_i].tile_id == sched_state_i[31:0]) begin
                    pid1_record <= submit_record[sched_state_i];
                    pid1_record.state <= 16'd1;
                    pid1_record.active_location <= submit_record[sched_state_i].tile_id;
                end
                if (submit_valid[sched_state_i] &&
                    is_child_record(submit_record[sched_state_i]) &&
                    submit_record[sched_state_i].tile_id == sched_state_i[31:0]) begin
                    child_runnable <= 1'b1;
                    child_record <= submit_record[sched_state_i];
                    child_record.state <= 16'd1;
                    child_record.active_location <= submit_record[sched_state_i].tile_id;
                end
                if (park_submit_valid[sched_state_i] &&
                    is_pid1_record(park_submit_record[sched_state_i]) &&
                    park_submit_record[sched_state_i].tile_id == sched_state_i[31:0]) begin
                    pid1_runnable <= 1'b0;
                    pid1_parked <= 1'b1;
                    pid1_record <= park_submit_record[sched_state_i];
                    pid1_record.state <= 16'd2;
                    pid1_record.active_location <= park_submit_record[sched_state_i].tile_id;
                end
            end

            if (wake_event_valid && pid1_parked && is_pid1_wake(wake_event)) begin
                pid1_runnable <= 1'b1;
                pid1_parked <= 1'b0;
                pid1_record.state <= 16'd1;
                pid1_record.active_location <= wake_event.tile_id;
            end
            if (child_issue_valid && child_runnable) begin
                child_runnable <= 1'b0;
            end
            exactly_one_location <= pid1_runnable ^ pid1_parked;
        end
    end

`ifndef SYNTHESIS
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
        end else begin
            if (pid1_runnable || pid1_parked) begin
                assert (is_pid1_record(pid1_record))
                    else $fatal(1, "SG-SCHED scheduler PID1 state missing typed metadata");
            end
            for (int unsigned assert_sched_i = 0; assert_sched_i < CORE_TILE_COUNT; assert_sched_i = assert_sched_i + 1) begin
                if (submit_valid[assert_sched_i]) begin
                    assert (is_live_sched_record(submit_record[assert_sched_i]))
                        else $fatal(1, "SG-SCHED scheduler submit record missing typed metadata");
                    assert (submit_record[assert_sched_i].tile_id == assert_sched_i[31:0] &&
                        submit_record[assert_sched_i].active_location == assert_sched_i[31:0])
                        else $fatal(1, "SG-SCHED scheduler submit record tile drift");
                end
                if (park_submit_valid[assert_sched_i] &&
                    is_pid1_record(park_submit_record[assert_sched_i])) begin
                    assert (park_submit_record[assert_sched_i].tile_id == assert_sched_i[31:0])
                        else $fatal(1, "SG-SCHED scheduler park record tile drift");
                end
                if (issue_valid[assert_sched_i]) begin
                    assert (is_pid1_record(issue_record[assert_sched_i]) ||
                        is_child_record(issue_record[assert_sched_i]))
                        else $fatal(1, "SG-SCHED scheduler issue record missing typed metadata");
                    assert (issue_record[assert_sched_i].tile_id == assert_sched_i[31:0] &&
                        issue_record[assert_sched_i].active_location == assert_sched_i[31:0])
                        else $fatal(1, "SG-SCHED scheduler issue record tile drift");
                    assert (issue_record[assert_sched_i].dispatch_eligible)
                        else $fatal(1, "SG-SCHED scheduler issued non-eligible record");
                end
            end
            if (wake_issue_valid) begin
                assert (pid1_parked && is_pid1_wake(wake_event))
                    else $fatal(1, "SG-WAKE scheduler issued wake without valid parked state");
            end
            if (child_runnable) begin
                assert (is_child_record(child_record))
                    else $fatal(1, "SG-SCHED scheduler child state missing typed metadata");
                assert (!pid1_runnable || child_record.tid != pid1_record.tid)
                    else $fatal(1, "SG-SCHED scheduler duplicated runnable TID");
            end
            assert (no_duplicate_issue)
                else $fatal(1, "SG-SCHED scheduler issued duplicate TID");
        end
    end
`endif
endmodule

module lnp64_event_router (
    input  logic clk,
    input  logic reset_n,
    input  logic synthetic_event,
    input  logic [31:0] source_tile_id,
    input  logic [31:0] target_tile_id,
    input  logic pid1_parked,
    output logic wake_valid,
    output logic event_valid,
    input  logic event_ready,
    output lnp64_event_t event_record,
    output logic [31:0] event_counter,
    output logic cross_tile_wake_valid,
    output logic [31:0] wake_counter
);
    logic synthetic_event_consumed;

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            wake_valid <= 1'b0;
            event_valid <= 1'b0;
            event_record <= '0;
            event_counter <= 32'd0;
            cross_tile_wake_valid <= 1'b0;
            wake_counter <= 32'd0;
            synthetic_event_consumed <= 1'b0;
        end else begin
            wake_valid <= 1'b0;
            cross_tile_wake_valid <= 1'b0;
            if (!synthetic_event) begin
                synthetic_event_consumed <= 1'b0;
            end
            if (synthetic_event && !synthetic_event_consumed && pid1_parked && !event_valid) begin
                event_counter <= event_counter + 32'd1;
                wake_counter <= wake_counter + 32'd1;
                wake_valid <= 1'b1;
                cross_tile_wake_valid <= source_tile_id != target_tile_id;
                synthetic_event_consumed <= 1'b1;
                event_valid <= 1'b1;
                event_record.event_id <= event_counter + 32'd1;
                event_record.tile_id <= target_tile_id;
                event_record.op_id <= 32'd0;
                event_record.pid <= 32'd1;
                event_record.tid <= 32'd1;
                event_record.domain_id <= 32'd1;
                event_record.domain_gen <= 32'd1;
                event_record.event_mask <= 64'h1;
                event_record.source <= LNP64_ENGINE_NONE;
                event_record.status <= LNP64_STATUS_EVENT;
            end
            if (event_valid && event_ready) begin
                event_valid <= 1'b0;
            end
        end
    end
endmodule

module lnp64_fault_telemetry (
    input  logic clk,
    input  logic reset_n,
    input  logic [31:0] tile_id,
    input  logic inject_fault,
    output logic fault_valid,
    input  logic fault_ready,
    output lnp64_fault_t fault,
    output logic [31:0] fault_counter
);
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            fault_valid <= 1'b0;
            fault <= '0;
            fault_counter <= 32'd0;
        end else begin
            if (inject_fault && !fault_valid) begin
                fault_counter <= fault_counter + 32'd1;
                fault_valid <= 1'b1;
                fault.fault_id <= fault_counter + 32'd1;
                fault.tile_id <= tile_id;
                fault.op_id <= 32'd0;
                fault.pid <= 32'd1;
                fault.tid <= 32'd1;
                fault.domain_id <= 32'd1;
                fault.domain_gen <= 32'd1;
                fault.fault_code <= LNP64_ERR_EFAULT;
                fault.source <= LNP64_ENGINE_FAULT;
                fault.detail <= 64'hfa01_7000;
            end
            if (fault_valid && fault_ready) begin
                fault_valid <= 1'b0;
            end
        end
    end
endmodule

module lnp64_watchdog (
    input  logic clk,
    input  logic reset_n,
    input  logic [31:0] tile_id,
    input  logic inject_stuck,
    output logic degraded,
    output logic fault_valid,
    input  logic fault_ready,
    output lnp64_fault_t fault
);
    logic [7:0] stuck_counter;

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            degraded <= 1'b0;
            fault_valid <= 1'b0;
            fault <= '0;
            stuck_counter <= 8'd0;
        end else begin
            if (inject_stuck && !degraded) begin
                stuck_counter <= stuck_counter + 8'd1;
                if (stuck_counter >= 8'd4) begin
                    degraded <= 1'b1;
                    fault_valid <= 1'b1;
                    fault.fault_id <= 32'hd06d_0001;
                    fault.tile_id <= tile_id;
                    fault.op_id <= 32'hffff_ffff;
                    fault.pid <= 32'd1;
                    fault.tid <= 32'd1;
                    fault.domain_id <= 32'd1;
                    fault.domain_gen <= 32'd1;
                    fault.fault_code <= LNP64_STATUS_DEGRADED;
                    fault.source <= LNP64_ENGINE_WATCHDOG;
                    fault.detail <= 64'hde6a_ded0;
                end
            end
            if (fault_valid && fault_ready) begin
                fault_valid <= 1'b0;
            end
        end
    end
endmodule

module lnp64_measurement_attestation (
    input  logic clk,
    input  logic reset_n,
    input  logic boot_valid,
    output lnp64_quote_t quote
);
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            quote <= '0;
        end else if (boot_valid) begin
            quote.quote_id <= 32'd1;
            quote.build_id <= LNP64_BUILD_ID;
            quote.feature_bits <= LNP64_S0_FEATURES;
            quote.boot_measurement <= 64'hb007_5000_0000_0001;
            quote.audit_root <= 64'ha0d1_7000_0000_0001;
            quote.proof_manifest_hash <= 64'hf0a1_5000_0000_0001;
        end
    end
endmodule

module lnp64_policy_engine (
    input  logic clk,
    input  logic reset_n,
    input  logic request_valid,
    output logic decision_allow
);
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            decision_allow <= 1'b0;
        end else begin
            decision_allow <= request_valid;
        end
    end
endmodule

module lnp64_typed_control_validator(
    input  logic clk,
    input  logic reset_n,
    output logic idle,
    output logic [31:0] telemetry_counter,
    output logic [31:0] fault_counter
);
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            idle <= 1'b0;
            telemetry_counter <= 32'd0;
            fault_counter <= 32'd0;
        end else begin
            idle <= 1'b1;
            telemetry_counter <= 32'd1;
        end
    end
endmodule

module lnp64_namespace_dispatch(
    input  logic clk,
    input  logic reset_n,
    output logic idle,
    output logic [31:0] telemetry_counter,
    output logic [31:0] fault_counter
);
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            idle <= 1'b0;
            telemetry_counter <= 32'd0;
            fault_counter <= 32'd0;
        end else begin
            idle <= 1'b1;
            telemetry_counter <= 32'd1;
        end
    end
endmodule

module lnp64_stream_frontend(
    input  logic clk,
    input  logic reset_n,
    output logic idle,
    output logic [31:0] telemetry_counter,
    output logic [31:0] fault_counter
);
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            idle <= 1'b0;
            telemetry_counter <= 32'd0;
            fault_counter <= 32'd0;
        end else begin
            idle <= 1'b1;
            telemetry_counter <= 32'd1;
        end
    end
endmodule

module lnp64_ddr_controller(
    input  logic clk,
    input  logic reset_n,
    output logic absent_or_idle,
    output logic [31:0] telemetry_counter,
    output logic [31:0] fault_counter
);
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            absent_or_idle <= 1'b0;
            telemetry_counter <= 32'd0;
            fault_counter <= 32'd0;
        end else begin
            absent_or_idle <= 1'b1;
            telemetry_counter <= 32'd1;
        end
    end
endmodule

module lnp64_sd_spi_flash(
    input  logic clk,
    input  logic reset_n,
    output logic absent_or_idle,
    output logic [31:0] telemetry_counter,
    output logic [31:0] fault_counter
);
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            absent_or_idle <= 1'b0;
            telemetry_counter <= 32'd0;
            fault_counter <= 32'd0;
        end else begin
            absent_or_idle <= 1'b1;
            telemetry_counter <= 32'd1;
        end
    end
endmodule

module lnp64_boot_image_storage(
    input  logic clk,
    input  logic reset_n,
    output logic idle,
    output logic [31:0] telemetry_counter,
    output logic [31:0] fault_counter
);
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            idle <= 1'b0;
            telemetry_counter <= 32'd0;
            fault_counter <= 32'd0;
        end else begin
            idle <= 1'b1;
            telemetry_counter <= 32'd1;
        end
    end
endmodule

module lnp64_cap_engine(
    input logic clk,
    input logic reset_n,
    input logic cmd_valid,
    output logic cmd_ready,
    input lnp64_cmd_t cmd,
    input logic object_cap_sync_valid,
    input logic [31:0] object_cap_sync_reader_fd,
    input logic [31:0] object_cap_sync_writer_fd,
    input logic object_cap_sync_single_valid,
    input logic [31:0] object_cap_sync_single_fd,
    input logic [2:0] object_cap_sync_single_kind,
    input logic [63:0] object_cap_sync_single_lineage,
    output logic rsp_valid,
    input logic rsp_ready,
    output lnp64_rsp_t rsp,
    output logic m1_commit_valid,
    output lnp64_m1_cap_commit_t m1_commit,
    output lnp64_m1_state_projection_t m1_pre_state_projection,
    output lnp64_m1_state_projection_t m1_state_projection,
    output logic [31:0] telemetry_counter,
    output logic [31:0] fault_counter
);
    logic have_rsp;
    lnp64_rsp_t rsp_reg;
    logic m1_commit_valid_reg;
    lnp64_m1_cap_commit_t m1_commit_reg;
    lnp64_m1_state_projection_t m1_pre_state_projection_reg;
    lnp64_m1_state_projection_t m1_state_projection_reg;
    logic fdr_valid [0:LNP64_FDR_SLOT_COUNT-1];
    logic fdr_revoked [0:LNP64_FDR_SLOT_COUNT-1];
    logic [63:0] fdr_generation [0:LNP64_FDR_SLOT_COUNT-1];
    logic [63:0] fdr_rights [0:LNP64_FDR_SLOT_COUNT-1];
    logic [63:0] fdr_object_id [0:LNP64_FDR_SLOT_COUNT-1];
    logic [63:0] fdr_lineage [0:LNP64_FDR_SLOT_COUNT-1];
    logic [31:0] fdr_domain_id [0:LNP64_FDR_SLOT_COUNT-1];
    logic [2:0] fdr_kind [0:LNP64_FDR_SLOT_COUNT-1];
    logic cap_queue_valid;
    logic [63:0] cap_queue_rights;
    logic [63:0] cap_queue_object_id;
    logic [63:0] cap_queue_lineage;
    logic [63:0] cap_queue_generation;
    logic [31:0] cap_queue_domain_id;
    logic cap_queue_revoked;

    assign cmd_ready = reset_n && !have_rsp;
    assign rsp_valid = have_rsp;
    assign rsp = rsp_reg;
    assign m1_commit_valid = m1_commit_valid_reg;
    assign m1_commit = m1_commit_reg;
    assign m1_pre_state_projection = m1_pre_state_projection_reg;
    assign m1_state_projection = m1_state_projection_reg;

    function automatic logic [31:0] default_cap_domain_id(input int unsigned fd);
        begin
            default_cap_domain_id = fd < 3 ? 32'd1 : 32'd2;
        end
    endfunction

    function automatic logic [31:0] cap_domain_id(input int unsigned fd);
        begin
            cap_domain_id = fd < LNP64_FDR_SLOT_COUNT ?
                fdr_domain_id[fd] : default_cap_domain_id(fd);
        end
    endfunction

    function automatic lnp64_m1_state_projection_t build_m1_state_projection(
        input logic [7:0] op,
        input logic [15:0] status,
        input int unsigned root_fd,
        input int unsigned consumer_fd
    );
        lnp64_m1_state_projection_t projection;
        begin
            projection = '0;
            projection.op = op;
            projection.status = status;
            if (root_fd < LNP64_FDR_SLOT_COUNT) begin
                projection.object_gen = fdr_generation[root_fd][31:0];
                projection.root_object_id = fdr_object_id[root_fd][31:0];
                projection.root_generation = fdr_generation[root_fd][31:0];
                projection.root_domain_id = cap_domain_id(root_fd);
                projection.root_lineage_epoch = fdr_lineage[root_fd][31:0];
                projection.root_sealed = 1'b0;
                projection.root_rights = (fdr_valid[root_fd] && !fdr_revoked[root_fd]) ?
                    fdr_rights[root_fd] : 64'd0;
                projection.has_revoked_generation = fdr_revoked[root_fd];
                projection.revoked_generation = fdr_revoked[root_fd] ?
                    fdr_generation[root_fd][31:0] : 32'd0;
            end
            if (consumer_fd < LNP64_FDR_SLOT_COUNT) begin
                projection.consumer_object_id = fdr_object_id[consumer_fd][31:0];
                projection.consumer_generation = fdr_generation[consumer_fd][31:0];
                projection.consumer_domain_id = cap_domain_id(consumer_fd);
                projection.consumer_lineage_epoch = fdr_lineage[consumer_fd][31:0];
                projection.consumer_sealed = 1'b0;
                projection.consumer_rights = (fdr_valid[consumer_fd] && !fdr_revoked[consumer_fd]) ?
                    fdr_rights[consumer_fd] : 64'd0;
            end
            projection.sent_valid = cap_queue_valid && !cap_queue_revoked;
            if (projection.sent_valid) begin
                projection.sent_object_id = cap_queue_object_id[31:0];
                projection.sent_generation = cap_queue_generation[31:0];
                projection.sent_domain_id = cap_queue_domain_id;
                projection.sent_lineage_epoch = cap_queue_lineage[31:0];
                projection.sent_sealed = 1'b0;
                projection.sent_rights = cap_queue_rights;
            end
            projection.transfer_valid = projection.sent_valid;
            projection.stale_rejected = status == LNP64_ERR_ESTALE;
            projection.revoked_rejected = status == LNP64_ERR_ESTALE;
            projection.failed_no_authority =
                status == LNP64_ERR_EPERM || status == LNP64_ERR_EBADF;
            projection.full_was_explicit = status == LNP64_ERR_EAGAIN;
            build_m1_state_projection = projection;
        end
    endfunction

`ifndef SYNTHESIS
    logic sampled_cap_queue_valid;
    logic [63:0] sampled_cap_queue_rights;
    logic [63:0] sampled_cap_queue_object_id;
    logic [63:0] sampled_cap_queue_lineage;
    logic [63:0] sampled_cap_queue_generation;
    logic [31:0] sampled_cap_queue_domain_id;
    logic sampled_cap_queue_revoked;

    function automatic logic live_fdr_projection_exists(
        input logic [31:0] object_id,
        input logic [31:0] generation,
        input logic [31:0] domain_id,
        input logic [31:0] lineage_epoch,
        input logic [63:0] rights
    );
        int unsigned slot;
        begin
            live_fdr_projection_exists = 1'b0;
            for (slot = 0; slot < LNP64_FDR_SLOT_COUNT; slot = slot + 1) begin
                if (fdr_valid[slot] && !fdr_revoked[slot] &&
                    fdr_object_id[slot][31:0] == object_id &&
                    fdr_generation[slot][31:0] == generation &&
                    fdr_domain_id[slot] == domain_id &&
                    fdr_lineage[slot][31:0] == lineage_epoch &&
                    fdr_rights[slot] == rights) begin
                    live_fdr_projection_exists = 1'b1;
                end
            end
        end
    endfunction

    function automatic logic cap_projection_is_zero(
        input logic [31:0] object_id,
        input logic [31:0] generation,
        input logic [31:0] domain_id,
        input logic [31:0] lineage_epoch,
        input logic sealed,
        input logic [63:0] rights
    );
        begin
            cap_projection_is_zero =
                object_id == 32'd0 &&
                generation == 32'd0 &&
                domain_id == 32'd0 &&
                lineage_epoch == 32'd0 &&
                !sealed &&
                rights == 64'd0;
        end
    endfunction

    function automatic logic fdr_projection_backed_by_state(
        input logic [31:0] object_id,
        input logic [31:0] generation,
        input logic [31:0] domain_id,
        input logic [31:0] lineage_epoch,
        input logic sealed,
        input logic [63:0] rights
    );
        int unsigned slot;
        begin
            fdr_projection_backed_by_state = cap_projection_is_zero(
                object_id,
                generation,
                domain_id,
                lineage_epoch,
                sealed,
                rights
            );
            for (slot = 0; slot < LNP64_FDR_SLOT_COUNT; slot = slot + 1) begin
                if (fdr_valid[slot] &&
                    fdr_object_id[slot][31:0] == object_id &&
                    fdr_generation[slot][31:0] == generation &&
                    fdr_domain_id[slot] == domain_id &&
                    fdr_lineage[slot][31:0] == lineage_epoch &&
                    !sealed &&
                    ((!fdr_revoked[slot] && fdr_rights[slot] == rights) ||
                     (fdr_revoked[slot] && rights == 64'd0))) begin
                    fdr_projection_backed_by_state = 1'b1;
                end
            end
        end
    endfunction

    function automatic logic projection_root_and_consumer_backed_by_fdr(
        input lnp64_m1_state_projection_t projection
    );
        begin
            projection_root_and_consumer_backed_by_fdr =
                fdr_projection_backed_by_state(
                    projection.root_object_id,
                    projection.root_generation,
                    projection.root_domain_id,
                    projection.root_lineage_epoch,
                    projection.root_sealed,
                    projection.root_rights
                ) &&
                fdr_projection_backed_by_state(
                    projection.consumer_object_id,
                    projection.consumer_generation,
                    projection.consumer_domain_id,
                    projection.consumer_lineage_epoch,
                    projection.consumer_sealed,
                    projection.consumer_rights
                );
        end
    endfunction

    function automatic logic sent_projection_backed_by_queue_state(
        input lnp64_m1_state_projection_t projection,
        input logic queue_valid,
        input logic queue_revoked,
        input logic [63:0] queue_object_id,
        input logic [63:0] queue_generation,
        input logic [31:0] queue_domain_id,
        input logic [63:0] queue_lineage,
        input logic [63:0] queue_rights
    );
        begin
            if (projection.sent_valid) begin
                sent_projection_backed_by_queue_state =
                    projection.transfer_valid &&
                    queue_valid && !queue_revoked &&
                    projection.sent_object_id == queue_object_id[31:0] &&
                    projection.sent_generation == queue_generation[31:0] &&
                    projection.sent_domain_id == queue_domain_id &&
                    projection.sent_lineage_epoch == queue_lineage[31:0] &&
                    !projection.sent_sealed &&
                    projection.sent_rights == queue_rights;
            end else begin
                sent_projection_backed_by_queue_state =
                    cap_projection_is_zero(
                        projection.sent_object_id,
                        projection.sent_generation,
                        projection.sent_domain_id,
                        projection.sent_lineage_epoch,
                        projection.sent_sealed,
                        projection.sent_rights
                    );
            end
        end
    endfunction

    function automatic logic sent_projection_backed_by_current_queue(
        input lnp64_m1_state_projection_t projection
    );
        begin
            sent_projection_backed_by_current_queue =
                sent_projection_backed_by_queue_state(
                    projection,
                    cap_queue_valid,
                    cap_queue_revoked,
                    cap_queue_object_id,
                    cap_queue_generation,
                    cap_queue_domain_id,
                    cap_queue_lineage,
                    cap_queue_rights
                );
        end
    endfunction

    function automatic logic sent_projection_backed_by_sampled_queue(
        input lnp64_m1_state_projection_t projection
    );
        begin
            sent_projection_backed_by_sampled_queue =
                sent_projection_backed_by_queue_state(
                    projection,
                    sampled_cap_queue_valid,
                    sampled_cap_queue_revoked,
                    sampled_cap_queue_object_id,
                    sampled_cap_queue_generation,
                    sampled_cap_queue_domain_id,
                    sampled_cap_queue_lineage,
                    sampled_cap_queue_rights
                );
        end
    endfunction

    function automatic logic minted_projection_is_zero(
        input lnp64_m1_state_projection_t projection
    );
        begin
            minted_projection_is_zero =
                !projection.minted_valid &&
                !projection.created_object_created &&
                projection.created_object_gen == 32'd0 &&
                cap_projection_is_zero(
                    projection.minted_object_id,
                    projection.minted_generation,
                    projection.minted_domain_id,
                    projection.minted_lineage_epoch,
                    projection.minted_sealed,
                    projection.minted_rights
                );
        end
    endfunction

    function automatic logic live_lineage_exists(input logic [31:0] lineage_epoch);
        int unsigned slot;
        begin
            live_lineage_exists = 1'b0;
            for (slot = 0; slot < LNP64_FDR_SLOT_COUNT; slot = slot + 1) begin
                if (fdr_valid[slot] && !fdr_revoked[slot] &&
                    fdr_lineage[slot][31:0] == lineage_epoch) begin
                    live_lineage_exists = 1'b1;
                end
            end
        end
    endfunction

    function automatic logic cap_queue_matches_sent_projection(
        input lnp64_m1_state_projection_t projection
    );
        begin
            cap_queue_matches_sent_projection =
                cap_queue_valid && !cap_queue_revoked &&
                projection.sent_valid &&
                projection.sent_object_id == cap_queue_object_id[31:0] &&
                projection.sent_generation == cap_queue_generation[31:0] &&
                projection.sent_domain_id == cap_queue_domain_id &&
                projection.sent_lineage_epoch == cap_queue_lineage[31:0] &&
                !projection.sent_sealed &&
                projection.sent_rights == cap_queue_rights;
        end
    endfunction

    function automatic logic consumer_projection_matches_commit(
        input lnp64_m1_state_projection_t projection,
        input lnp64_m1_cap_commit_t commit
    );
        begin
            consumer_projection_matches_commit =
                projection.consumer_object_id == commit.object_id &&
                projection.consumer_generation == commit.fdr_gen &&
                projection.consumer_domain_id == commit.domain_id &&
                projection.consumer_lineage_epoch == commit.lineage_epoch &&
                projection.consumer_sealed == commit.sealed &&
                projection.consumer_rights == commit.rights_mask;
        end
    endfunction

    function automatic logic sent_projection_matches_commit(
        input lnp64_m1_state_projection_t projection,
        input lnp64_m1_cap_commit_t commit
    );
        begin
            sent_projection_matches_commit =
                projection.sent_valid &&
                projection.sent_object_id == commit.object_id &&
                projection.sent_generation == commit.fdr_gen &&
                projection.sent_domain_id == commit.domain_id &&
                projection.sent_lineage_epoch == commit.lineage_epoch &&
                projection.sent_sealed == commit.sealed &&
                projection.sent_rights == commit.rights_mask;
        end
    endfunction

    function automatic logic revoked_root_projection_matches_commit(
        input lnp64_m1_state_projection_t projection,
        input lnp64_m1_cap_commit_t commit
    );
        begin
            revoked_root_projection_matches_commit =
                projection.root_object_id == commit.object_id &&
                projection.root_generation == commit.fdr_gen &&
                projection.root_domain_id == commit.domain_id &&
                projection.root_lineage_epoch == commit.lineage_epoch &&
                projection.has_revoked_generation;
        end
    endfunction

    function automatic logic authority_projection_slots_match(
        input lnp64_m1_state_projection_t left,
        input lnp64_m1_state_projection_t right
    );
        begin
            authority_projection_slots_match =
                left.object_gen == right.object_gen &&
                left.created_object_created == right.created_object_created &&
                left.created_object_gen == right.created_object_gen &&
                left.root_object_id == right.root_object_id &&
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
                left.minted_rights == right.minted_rights &&
                left.wake_pending == right.wake_pending &&
                left.transfer_valid == right.transfer_valid &&
                left.has_revoked_generation == right.has_revoked_generation &&
                left.revoked_generation == right.revoked_generation;
        end
    endfunction
`endif

    function automatic int unsigned cap_fd(input logic [63:0] value);
        logic [63:0] fd_bits;
        begin
            fd_bits = (value < 64'd256) ? value : (value & LNP64_FDR_TOKEN_INDEX_MASK);
            cap_fd = fd_bits[7:0];
        end
    endfunction

    function automatic logic cap_token_shape_valid(input logic [63:0] value);
        begin
            cap_token_shape_valid = (value & LNP64_FDR_TOKEN_MARKER) != 64'd0 &&
                value[7:0] < LNP64_FDR_SLOT_COUNT[7:0] &&
                ((value & ~LNP64_FDR_TOKEN_MARKER) >> 8) != 64'd0;
        end
    endfunction

    function automatic logic cap_generation_matches(
        input logic [63:0] value,
        input int unsigned fd
    );
        begin
            cap_generation_matches = fd < LNP64_FDR_SLOT_COUNT &&
                (((value & ~LNP64_FDR_TOKEN_MARKER) >> 8) == fdr_generation[fd]);
        end
    endfunction

    function automatic logic [63:0] cap_token(
        input int unsigned fd,
        input logic [63:0] generation
    );
        begin
            cap_token = LNP64_FDR_TOKEN_MARKER | (generation << 8) | {56'd0, fd[7:0]};
        end
    endfunction

    integer i;
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            have_rsp <= 1'b0;
            rsp_reg <= '0;
            m1_commit_valid_reg <= 1'b0;
            m1_commit_reg <= '0;
            m1_pre_state_projection_reg <= '0;
            m1_state_projection_reg <= '0;
            telemetry_counter <= 32'd0;
            fault_counter <= 32'd0;
            cap_queue_valid <= 1'b0;
            cap_queue_rights <= 64'd0;
            cap_queue_object_id <= 64'd0;
            cap_queue_lineage <= 64'd0;
            cap_queue_generation <= 64'd0;
            cap_queue_domain_id <= 32'd0;
            cap_queue_revoked <= 1'b0;
            for (i = 0; i < LNP64_FDR_SLOT_COUNT; i = i + 1) begin
                fdr_generation[i] <= 64'd1;
                fdr_valid[i] <= i < 3;
                fdr_revoked[i] <= 1'b0;
                fdr_rights[i] <= i < 3 ? LNP64_CAP_RIGHT_ALL : 64'd0;
                fdr_object_id[i] <= {32'd0, i[31:0]} + 64'd1;
                fdr_lineage[i] <= {32'd0, i[31:0]} + 64'd1;
                fdr_domain_id[i] <= default_cap_domain_id(i);
                fdr_kind[i] <= i < 3 ? LNP64_FDR_KIND_GENERIC : LNP64_FDR_KIND_CLOSED;
            end
        end else begin
            if (have_rsp && rsp_ready) begin
                have_rsp <= 1'b0;
                m1_commit_valid_reg <= 1'b0;
            end
            if (object_cap_sync_valid) begin
                if (object_cap_sync_reader_fd < LNP64_FDR_SLOT_COUNT) begin
                    fdr_valid[object_cap_sync_reader_fd] <= 1'b1;
                    fdr_revoked[object_cap_sync_reader_fd] <= 1'b0;
                    fdr_generation[object_cap_sync_reader_fd] <= fdr_generation[object_cap_sync_reader_fd] + 64'd1;
                    fdr_rights[object_cap_sync_reader_fd] <= LNP64_CAP_RIGHT_ALL;
                    fdr_object_id[object_cap_sync_reader_fd] <=
                        {32'd0, object_cap_sync_reader_fd} + 64'd1;
                    fdr_lineage[object_cap_sync_reader_fd] <= 64'd257 + {32'd0, object_cap_sync_reader_fd};
                    fdr_domain_id[object_cap_sync_reader_fd] <=
                        default_cap_domain_id(object_cap_sync_reader_fd);
                    fdr_kind[object_cap_sync_reader_fd] <= LNP64_FDR_KIND_PIPE_READER;
                end
                if (object_cap_sync_writer_fd < LNP64_FDR_SLOT_COUNT) begin
                    fdr_valid[object_cap_sync_writer_fd] <= 1'b1;
                    fdr_revoked[object_cap_sync_writer_fd] <= 1'b0;
                    fdr_generation[object_cap_sync_writer_fd] <= fdr_generation[object_cap_sync_writer_fd] + 64'd1;
                    fdr_rights[object_cap_sync_writer_fd] <= LNP64_CAP_RIGHT_ALL;
                    fdr_object_id[object_cap_sync_writer_fd] <=
                        {32'd0, object_cap_sync_writer_fd} + 64'd1;
                    fdr_lineage[object_cap_sync_writer_fd] <= 64'd257 + {32'd0, object_cap_sync_writer_fd};
                    fdr_domain_id[object_cap_sync_writer_fd] <=
                        default_cap_domain_id(object_cap_sync_writer_fd);
                    fdr_kind[object_cap_sync_writer_fd] <= LNP64_FDR_KIND_PIPE_WRITER;
                end
            end
            if (object_cap_sync_single_valid &&
                object_cap_sync_single_fd < LNP64_FDR_SLOT_COUNT) begin
                fdr_valid[object_cap_sync_single_fd] <= 1'b1;
                fdr_revoked[object_cap_sync_single_fd] <= 1'b0;
                fdr_generation[object_cap_sync_single_fd] <= fdr_generation[object_cap_sync_single_fd] + 64'd1;
                fdr_rights[object_cap_sync_single_fd] <= LNP64_CAP_RIGHT_ALL;
                fdr_object_id[object_cap_sync_single_fd] <= {32'd0, object_cap_sync_single_fd} + 64'd1;
                fdr_lineage[object_cap_sync_single_fd] <= object_cap_sync_single_lineage;
                fdr_domain_id[object_cap_sync_single_fd] <=
                    default_cap_domain_id(object_cap_sync_single_fd);
                fdr_kind[object_cap_sync_single_fd] <= object_cap_sync_single_kind;
            end
            if (cmd_valid && cmd_ready) begin : accept_cmd
                int unsigned src_fd;
                int unsigned dst_fd;
                int unsigned payload_fd;
                logic src_is_token;
                logic src_token_valid;
                logic src_generation_matches;
                logic src_live;
                logic src_stale;
                logic payload_live;
                logic [63:0] dup_rights;
                logic [63:0] recv_rights;
                logic [63:0] next_generation;
                logic [63:0] revoke_count;

                have_rsp <= 1'b1;
                m1_commit_valid_reg <=
                    cmd.opcode == LNP64_OP_CAP_DUP ||
                    cmd.opcode == LNP64_OP_CAP_SEND ||
                    cmd.opcode == LNP64_OP_CAP_RECV ||
                    cmd.opcode == LNP64_OP_CAP_REVOKE;
                telemetry_counter <= telemetry_counter + 32'd1;
                rsp_reg <= '0;
                m1_commit_reg <= '0;
                m1_pre_state_projection_reg <= '0;
                m1_state_projection_reg <= '0;
                unique case (cmd.opcode)
                    LNP64_OP_CAP_DUP: m1_commit_reg.op <= LNP64_M1_COMMIT_CAP_DUP;
                    LNP64_OP_CAP_SEND: m1_commit_reg.op <= LNP64_M1_COMMIT_CAP_SEND;
                    LNP64_OP_CAP_RECV: m1_commit_reg.op <= LNP64_M1_COMMIT_CAP_RECV;
                    LNP64_OP_CAP_REVOKE: m1_commit_reg.op <= LNP64_M1_COMMIT_CAP_REVOKE;
                    default: m1_commit_reg.op <= 8'd0;
                endcase
                m1_commit_reg.domain_id <= cmd.domain_id;
                m1_commit_reg.domain_gen <= cmd.domain_gen;
                m1_commit_reg.status <= LNP64_ERR_ENOTSUP;
                rsp_reg.op_id <= cmd.op_id;
                rsp_reg.tile_id <= cmd.tile_id;
                rsp_reg.pid <= cmd.pid;
                rsp_reg.tid <= cmd.tid;
                rsp_reg.domain_id <= cmd.domain_id;
                rsp_reg.domain_gen <= cmd.domain_gen;
                rsp_reg.result_reg <= cmd.result_reg;
                rsp_reg.result_value <= 64'hffff_ffff_ffff_ffff;
                rsp_reg.errno_value <= LNP64_ERR_ENOTSUP;
                rsp_reg.status <= LNP64_STATUS_UNSUPPORTED;

                if (cmd.opcode == LNP64_OP_CAP_DUP) begin
                    src_fd = cap_fd(cmd.arg0);
                    dst_fd = cmd.arg1 == 64'd0 ? 3 : cap_fd(cmd.arg1);
                    src_is_token = cmd.arg0 >= 64'd256;
                    src_token_valid = cap_token_shape_valid(cmd.arg0);
                    src_generation_matches = cap_generation_matches(cmd.arg0, src_fd);
                    src_live = src_fd < LNP64_FDR_SLOT_COUNT && fdr_valid[src_fd] &&
                        !fdr_revoked[src_fd] &&
                        (!src_is_token || (src_token_valid && src_generation_matches));
                    src_stale = src_is_token && src_token_valid &&
                        src_fd < LNP64_FDR_SLOT_COUNT &&
                        (!fdr_valid[src_fd] || fdr_revoked[src_fd] || !src_generation_matches);
                    dup_rights = (cmd.rights_mask == 64'd0 && src_fd < LNP64_FDR_SLOT_COUNT) ?
                        fdr_rights[src_fd] : cmd.rights_mask;
                    if (src_fd < LNP64_FDR_SLOT_COUNT) begin
                        m1_commit_reg.object_id <= fdr_object_id[src_fd][31:0];
                        m1_commit_reg.object_gen <= fdr_generation[src_fd][31:0];
                        m1_commit_reg.fdr_gen <= fdr_generation[src_fd][31:0];
                        m1_commit_reg.domain_id <= cap_domain_id(src_fd);
                        m1_commit_reg.rights_mask <= fdr_rights[src_fd];
                        m1_commit_reg.lineage_epoch <= fdr_lineage[src_fd][31:0];
                    end
                    if (cmd.flags & ~LNP64_CAP_DUP_FLAG_SEAL) begin
                        rsp_reg.errno_value <= LNP64_ERR_EINVAL;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                        m1_commit_reg.status <= LNP64_ERR_EINVAL;
                        m1_pre_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_DUP, LNP64_ERR_EINVAL, src_fd, dst_fd);
                        m1_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_DUP, LNP64_ERR_EINVAL, src_fd, dst_fd);
                    end else if (src_stale) begin
                        rsp_reg.errno_value <= LNP64_ERR_ESTALE;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                        m1_commit_reg.status <= LNP64_ERR_ESTALE;
                        m1_pre_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_DUP, LNP64_ERR_ESTALE, src_fd, dst_fd);
                        m1_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_DUP, LNP64_ERR_ESTALE, src_fd, dst_fd);
                    end else if (!src_live || dst_fd >= LNP64_FDR_SLOT_COUNT) begin
                        rsp_reg.errno_value <= LNP64_ERR_EBADF;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                        m1_commit_reg.status <= LNP64_ERR_EBADF;
                        m1_pre_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_DUP, LNP64_ERR_EBADF, src_fd, dst_fd);
                        m1_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_DUP, LNP64_ERR_EBADF, src_fd, dst_fd);
                    end else if ((fdr_rights[src_fd] & LNP64_CAP_RIGHT_DUP) == 64'd0 ||
                        ((dup_rights & ~fdr_rights[src_fd]) != 64'd0)) begin
                        rsp_reg.errno_value <= LNP64_ERR_EPERM;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                        m1_commit_reg.status <= LNP64_ERR_EPERM;
                        m1_pre_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_DUP, LNP64_ERR_EPERM, src_fd, dst_fd);
                        m1_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_DUP, LNP64_ERR_EPERM, src_fd, dst_fd);
                    end else begin
                        next_generation = fdr_generation[dst_fd] + 64'd1;
                        fdr_valid[dst_fd] <= 1'b1;
                        fdr_revoked[dst_fd] <= 1'b0;
                        fdr_generation[dst_fd] <= next_generation;
                        fdr_rights[dst_fd] <= dup_rights;
                        fdr_object_id[dst_fd] <= fdr_object_id[src_fd];
                        fdr_lineage[dst_fd] <= fdr_lineage[src_fd];
                        fdr_domain_id[dst_fd] <= cap_domain_id(dst_fd);
                        fdr_kind[dst_fd] <= fdr_kind[src_fd];
                        rsp_reg.result_value <= cap_token(dst_fd, next_generation);
                        rsp_reg.errno_value <= LNP64_ERR_OK;
                        rsp_reg.status <= LNP64_STATUS_OK;
                        m1_commit_reg.object_id <= fdr_object_id[src_fd][31:0];
                        m1_commit_reg.object_gen <= next_generation[31:0];
                        m1_commit_reg.fdr_gen <= next_generation[31:0];
                        m1_commit_reg.domain_id <= cap_domain_id(dst_fd);
                        m1_commit_reg.rights_mask <= dup_rights;
                        m1_commit_reg.lineage_epoch <= fdr_lineage[src_fd][31:0];
                        m1_commit_reg.sealed <= (cmd.flags & LNP64_CAP_DUP_FLAG_SEAL) != 64'd0;
                        m1_commit_reg.status <= LNP64_ERR_OK;
                        m1_pre_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_DUP, LNP64_ERR_OK, src_fd, dst_fd);
                        m1_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_DUP, LNP64_ERR_OK, src_fd, dst_fd);
                        m1_state_projection_reg.object_gen <= next_generation[31:0];
                        m1_state_projection_reg.consumer_object_id <= fdr_object_id[src_fd][31:0];
                        m1_state_projection_reg.consumer_generation <= next_generation[31:0];
                        m1_state_projection_reg.consumer_domain_id <= cap_domain_id(dst_fd);
                        m1_state_projection_reg.consumer_lineage_epoch <= fdr_lineage[src_fd][31:0];
                        m1_state_projection_reg.consumer_sealed <=
                            (cmd.flags & LNP64_CAP_DUP_FLAG_SEAL) != 64'd0;
                        m1_state_projection_reg.consumer_rights <= dup_rights;
                    end
                end else if (cmd.opcode == LNP64_OP_CAP_SEND) begin
                    src_fd = cap_fd(cmd.arg0);
                    payload_fd = cap_fd(cmd.arg1);
                    src_is_token = cmd.arg0 >= 64'd256;
                    src_token_valid = cap_token_shape_valid(cmd.arg0);
                    src_generation_matches = cap_generation_matches(cmd.arg0, src_fd);
                    src_live = src_fd < LNP64_FDR_SLOT_COUNT && fdr_valid[src_fd] &&
                        !fdr_revoked[src_fd] &&
                        (!src_is_token || (src_token_valid && src_generation_matches));
                    payload_live = payload_fd < LNP64_FDR_SLOT_COUNT &&
                        fdr_valid[payload_fd] && !fdr_revoked[payload_fd];
                    if (cmd.flags != 64'd0) begin
                        rsp_reg.errno_value <= LNP64_ERR_EINVAL;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                        m1_commit_reg.status <= LNP64_ERR_EINVAL;
                        m1_pre_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_SEND, LNP64_ERR_EINVAL, src_fd, payload_fd);
                        m1_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_SEND, LNP64_ERR_EINVAL, src_fd, payload_fd);
                    end else if (!src_live ||
                        fdr_kind[src_fd] != LNP64_FDR_KIND_PIPE_WRITER ||
                        ((fdr_rights[src_fd] &
                            (LNP64_CAP_RIGHT_TRANSFER | 64'd2)) !=
                            (LNP64_CAP_RIGHT_TRANSFER | 64'd2))) begin
                        rsp_reg.errno_value <= LNP64_ERR_EBADF;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                        m1_commit_reg.status <= LNP64_ERR_EBADF;
                        m1_pre_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_SEND, LNP64_ERR_EBADF, src_fd, payload_fd);
                        m1_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_SEND, LNP64_ERR_EBADF, src_fd, payload_fd);
                    end else if (!payload_live) begin
                        rsp_reg.errno_value <= LNP64_ERR_EBADF;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                        m1_commit_reg.status <= LNP64_ERR_EBADF;
                        m1_pre_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_SEND, LNP64_ERR_EBADF, src_fd, payload_fd);
                        m1_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_SEND, LNP64_ERR_EBADF, src_fd, payload_fd);
                    end else if ((fdr_rights[payload_fd] & LNP64_CAP_RIGHT_TRANSFER) == 64'd0) begin
                        rsp_reg.errno_value <= LNP64_ERR_EPERM;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                        m1_commit_reg.status <= LNP64_ERR_EPERM;
                        m1_pre_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_SEND, LNP64_ERR_EPERM, src_fd, payload_fd);
                        m1_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_SEND, LNP64_ERR_EPERM, src_fd, payload_fd);
                    end else if (cap_queue_valid) begin
                        rsp_reg.errno_value <= LNP64_ERR_EAGAIN;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                        m1_commit_reg.status <= LNP64_ERR_EAGAIN;
                        m1_pre_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_SEND, LNP64_ERR_EAGAIN, src_fd, payload_fd);
                        m1_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_SEND, LNP64_ERR_EAGAIN, src_fd, payload_fd);
                    end else begin
                        cap_queue_valid <= 1'b1;
                        cap_queue_rights <= fdr_rights[payload_fd];
                        cap_queue_object_id <= fdr_object_id[payload_fd];
                        cap_queue_lineage <= fdr_lineage[payload_fd];
                        cap_queue_generation <= fdr_generation[payload_fd];
                        cap_queue_domain_id <= cap_domain_id(payload_fd);
                        cap_queue_revoked <= fdr_revoked[payload_fd];
                        rsp_reg.result_value <= 64'd1;
                        rsp_reg.errno_value <= LNP64_ERR_OK;
                        rsp_reg.status <= LNP64_STATUS_OK;
                        m1_commit_reg.status <= LNP64_ERR_OK;
                        m1_pre_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_SEND, LNP64_ERR_OK, src_fd, payload_fd);
                        m1_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_SEND, LNP64_ERR_OK, src_fd, payload_fd);
                        m1_state_projection_reg.sent_valid <= 1'b1;
                        m1_state_projection_reg.sent_object_id <= fdr_object_id[payload_fd][31:0];
                        m1_state_projection_reg.sent_generation <= fdr_generation[payload_fd][31:0];
                        m1_state_projection_reg.sent_domain_id <= cap_domain_id(payload_fd);
                        m1_state_projection_reg.sent_lineage_epoch <= fdr_lineage[payload_fd][31:0];
                        m1_state_projection_reg.sent_sealed <= 1'b0;
                        m1_state_projection_reg.sent_rights <= fdr_rights[payload_fd];
                        m1_state_projection_reg.transfer_valid <= 1'b1;
                    end
                    if (payload_fd < LNP64_FDR_SLOT_COUNT) begin
                        m1_commit_reg.object_id <= fdr_object_id[payload_fd][31:0];
                        m1_commit_reg.object_gen <= fdr_generation[payload_fd][31:0];
                        m1_commit_reg.fdr_gen <= fdr_generation[payload_fd][31:0];
                        m1_commit_reg.domain_id <= cap_domain_id(payload_fd);
                        m1_commit_reg.rights_mask <= fdr_rights[payload_fd];
                        m1_commit_reg.lineage_epoch <= fdr_lineage[payload_fd][31:0];
                    end
                end else if (cmd.opcode == LNP64_OP_CAP_RECV) begin
                    src_fd = cap_fd(cmd.arg0);
                    dst_fd = cmd.arg1 == 64'd0 ? 3 : cap_fd(cmd.arg1);
                    src_is_token = cmd.arg0 >= 64'd256;
                    src_token_valid = cap_token_shape_valid(cmd.arg0);
                    src_generation_matches = cap_generation_matches(cmd.arg0, src_fd);
                    src_live = src_fd < LNP64_FDR_SLOT_COUNT && fdr_valid[src_fd] &&
                        !fdr_revoked[src_fd] &&
                        (!src_is_token || (src_token_valid && src_generation_matches));
                    recv_rights = cmd.rights_mask == 64'd0 ? cap_queue_rights : cmd.rights_mask;
                    if (cmd.flags != 64'd0) begin
                        rsp_reg.errno_value <= LNP64_ERR_EINVAL;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                        m1_commit_reg.status <= LNP64_ERR_EINVAL;
                        m1_pre_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_RECV, LNP64_ERR_EINVAL, src_fd, dst_fd);
                        m1_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_RECV, LNP64_ERR_EINVAL, src_fd, dst_fd);
                    end else if (!src_live ||
                        fdr_kind[src_fd] != LNP64_FDR_KIND_PIPE_READER ||
                        ((fdr_rights[src_fd] &
                            (LNP64_CAP_RIGHT_TRANSFER | 64'd1)) !=
                            (LNP64_CAP_RIGHT_TRANSFER | 64'd1))) begin
                        rsp_reg.errno_value <= LNP64_ERR_EBADF;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                        m1_commit_reg.status <= LNP64_ERR_EBADF;
                        m1_pre_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_RECV, LNP64_ERR_EBADF, src_fd, dst_fd);
                        m1_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_RECV, LNP64_ERR_EBADF, src_fd, dst_fd);
                    end else if (!cap_queue_valid) begin
                        rsp_reg.errno_value <= LNP64_ERR_EAGAIN;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                        m1_commit_reg.status <= LNP64_ERR_EAGAIN;
                        m1_pre_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_RECV, LNP64_ERR_EAGAIN, src_fd, dst_fd);
                        m1_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_RECV, LNP64_ERR_EAGAIN, src_fd, dst_fd);
                    end else if (cap_queue_revoked) begin
                        rsp_reg.errno_value <= LNP64_ERR_ESTALE;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                        m1_commit_reg.status <= LNP64_ERR_ESTALE;
                        m1_pre_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_RECV, LNP64_ERR_ESTALE, src_fd, dst_fd);
                        m1_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_RECV, LNP64_ERR_ESTALE, src_fd, dst_fd);
                    end else if ((recv_rights & ~cap_queue_rights) != 64'd0) begin
                        rsp_reg.errno_value <= LNP64_ERR_EPERM;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                        m1_commit_reg.status <= LNP64_ERR_EPERM;
                        m1_pre_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_RECV, LNP64_ERR_EPERM, src_fd, dst_fd);
                        m1_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_RECV, LNP64_ERR_EPERM, src_fd, dst_fd);
                    end else if (dst_fd >= LNP64_FDR_SLOT_COUNT) begin
                        rsp_reg.errno_value <= LNP64_ERR_EBADF;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                        m1_commit_reg.status <= LNP64_ERR_EBADF;
                        m1_pre_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_RECV, LNP64_ERR_EBADF, src_fd, dst_fd);
                        m1_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_RECV, LNP64_ERR_EBADF, src_fd, dst_fd);
                    end else begin
                        next_generation = fdr_generation[dst_fd] + 64'd1;
                        cap_queue_valid <= 1'b0;
                        cap_queue_object_id <= 64'd0;
                        cap_queue_generation <= 64'd0;
                        cap_queue_domain_id <= 32'd0;
                        fdr_valid[dst_fd] <= 1'b1;
                        fdr_revoked[dst_fd] <= 1'b0;
                        fdr_generation[dst_fd] <= next_generation;
                        fdr_rights[dst_fd] <= recv_rights;
                        fdr_object_id[dst_fd] <= cap_queue_object_id;
                        fdr_lineage[dst_fd] <= cap_queue_lineage;
                        fdr_domain_id[dst_fd] <= cap_queue_domain_id;
                        fdr_kind[dst_fd] <= LNP64_FDR_KIND_GENERIC;
                        rsp_reg.result_value <= cap_token(dst_fd, next_generation);
                        rsp_reg.errno_value <= LNP64_ERR_OK;
                        rsp_reg.status <= LNP64_STATUS_OK;
                        m1_commit_reg.object_gen <= next_generation[31:0];
                        m1_commit_reg.fdr_gen <= next_generation[31:0];
                        m1_commit_reg.domain_id <= cap_queue_domain_id;
                        m1_commit_reg.status <= LNP64_ERR_OK;
                        m1_pre_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_RECV, LNP64_ERR_OK, src_fd, dst_fd);
                        m1_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_RECV, LNP64_ERR_OK, src_fd, dst_fd);
                        m1_state_projection_reg.object_gen <= next_generation[31:0];
                        m1_state_projection_reg.consumer_object_id <= cap_queue_object_id[31:0];
                        m1_state_projection_reg.consumer_generation <= next_generation[31:0];
                        m1_state_projection_reg.consumer_domain_id <= cap_queue_domain_id;
                        m1_state_projection_reg.consumer_lineage_epoch <= cap_queue_lineage[31:0];
                        m1_state_projection_reg.consumer_sealed <= 1'b0;
                        m1_state_projection_reg.consumer_rights <= recv_rights;
                        m1_state_projection_reg.sent_valid <= 1'b0;
                        m1_state_projection_reg.sent_object_id <= 32'd0;
                        m1_state_projection_reg.sent_generation <= 32'd0;
                        m1_state_projection_reg.sent_domain_id <= 32'd0;
                        m1_state_projection_reg.sent_lineage_epoch <= 32'd0;
                        m1_state_projection_reg.sent_sealed <= 1'b0;
                        m1_state_projection_reg.sent_rights <= 64'd0;
                        m1_state_projection_reg.transfer_valid <= 1'b1;
                    end
                    if (cap_queue_valid) begin
                        m1_commit_reg.object_id <= cap_queue_object_id[31:0];
                        m1_commit_reg.domain_id <= cap_queue_domain_id;
                        m1_commit_reg.rights_mask <= recv_rights;
                        m1_commit_reg.lineage_epoch <= cap_queue_lineage[31:0];
                    end
                end else if (cmd.opcode == LNP64_OP_CAP_REVOKE) begin
                    src_fd = cap_fd(cmd.arg0);
                    src_is_token = cmd.arg0 >= 64'd256;
                    src_token_valid = cap_token_shape_valid(cmd.arg0);
                    src_generation_matches = cap_generation_matches(cmd.arg0, src_fd);
                    src_live = src_fd < LNP64_FDR_SLOT_COUNT && fdr_valid[src_fd] &&
                        !fdr_revoked[src_fd] &&
                        (!src_is_token || (src_token_valid && src_generation_matches));
                    src_stale = src_is_token && src_token_valid &&
                        src_fd < LNP64_FDR_SLOT_COUNT &&
                        (!fdr_valid[src_fd] || fdr_revoked[src_fd] || !src_generation_matches);
                    revoke_count = 64'd0;
                    if (src_fd < LNP64_FDR_SLOT_COUNT) begin
                        for (i = 0; i < LNP64_FDR_SLOT_COUNT; i = i + 1) begin
                            if (fdr_valid[i] && !fdr_revoked[i] &&
                                fdr_lineage[i] == fdr_lineage[src_fd]) begin
                                revoke_count = revoke_count + 64'd1;
                            end
                        end
                        if (cap_queue_valid && !cap_queue_revoked &&
                            cap_queue_lineage == fdr_lineage[src_fd]) begin
                            revoke_count = revoke_count + 64'd1;
                        end
                    end
                    if (src_fd < LNP64_FDR_SLOT_COUNT) begin
                        m1_commit_reg.object_id <= fdr_object_id[src_fd][31:0];
                        m1_commit_reg.object_gen <= fdr_generation[src_fd][31:0];
                        m1_commit_reg.fdr_gen <= fdr_generation[src_fd][31:0];
                        m1_commit_reg.domain_id <= cap_domain_id(src_fd);
                        m1_commit_reg.rights_mask <= fdr_rights[src_fd];
                        m1_commit_reg.lineage_epoch <= fdr_lineage[src_fd][31:0];
                    end
                    if (src_stale) begin
                        rsp_reg.errno_value <= LNP64_ERR_ESTALE;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                        m1_commit_reg.status <= LNP64_ERR_ESTALE;
                        m1_pre_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_REVOKE, LNP64_ERR_ESTALE, src_fd, src_fd);
                        m1_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_REVOKE, LNP64_ERR_ESTALE, src_fd, src_fd);
                    end else if (src_live && ((fdr_rights[src_fd] & LNP64_CAP_RIGHT_REVOKE) != 64'd0)) begin
                        next_generation = fdr_generation[src_fd] + 64'd1;
                        for (i = 0; i < LNP64_FDR_SLOT_COUNT; i = i + 1) begin
                            if (fdr_valid[i] && !fdr_revoked[i] &&
                                fdr_lineage[i] == fdr_lineage[src_fd]) begin
                                fdr_revoked[i] <= 1'b1;
                                fdr_generation[i] <= fdr_generation[i] + 64'd1;
                            end
                        end
                        if (cap_queue_valid && !cap_queue_revoked &&
                            cap_queue_lineage == fdr_lineage[src_fd]) begin
                            cap_queue_revoked <= 1'b1;
                        end
                        rsp_reg.result_value <= revoke_count;
                        rsp_reg.errno_value <= LNP64_ERR_OK;
                        rsp_reg.status <= LNP64_STATUS_OK;
                        m1_commit_reg.object_gen <= next_generation[31:0];
                        m1_commit_reg.fdr_gen <= next_generation[31:0];
                        m1_commit_reg.status <= LNP64_ERR_OK;
                        m1_pre_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_REVOKE, LNP64_ERR_OK, src_fd, src_fd);
                        m1_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_REVOKE, LNP64_ERR_OK, src_fd, src_fd);
                        m1_state_projection_reg.object_gen <= next_generation[31:0];
                        m1_state_projection_reg.root_generation <= next_generation[31:0];
                        m1_state_projection_reg.root_rights <= 64'd0;
                        m1_state_projection_reg.has_revoked_generation <= 1'b1;
                        m1_state_projection_reg.revoked_generation <= next_generation[31:0];
                        m1_state_projection_reg.consumer_generation <= next_generation[31:0];
                        m1_state_projection_reg.consumer_rights <= 64'd0;
                        if (cap_queue_valid && !cap_queue_revoked &&
                            cap_queue_lineage == fdr_lineage[src_fd]) begin
                            m1_state_projection_reg.sent_valid <= 1'b0;
                            m1_state_projection_reg.sent_object_id <= 32'd0;
                            m1_state_projection_reg.sent_generation <= 32'd0;
                            m1_state_projection_reg.sent_domain_id <= 32'd0;
                            m1_state_projection_reg.sent_lineage_epoch <= 32'd0;
                            m1_state_projection_reg.sent_sealed <= 1'b0;
                            m1_state_projection_reg.sent_rights <= 64'd0;
                            m1_state_projection_reg.transfer_valid <= 1'b1;
                        end
                    end else if (src_fd < LNP64_FDR_SLOT_COUNT && fdr_valid[src_fd] &&
                        !fdr_revoked[src_fd] &&
                        ((fdr_rights[src_fd] & LNP64_CAP_RIGHT_REVOKE) == 64'd0)) begin
                        rsp_reg.errno_value <= LNP64_ERR_EPERM;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                        m1_commit_reg.status <= LNP64_ERR_EPERM;
                        m1_pre_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_REVOKE, LNP64_ERR_EPERM, src_fd, src_fd);
                        m1_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_REVOKE, LNP64_ERR_EPERM, src_fd, src_fd);
                    end else begin
                        rsp_reg.errno_value <= LNP64_ERR_EBADF;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                        m1_commit_reg.status <= LNP64_ERR_EBADF;
                        m1_pre_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_REVOKE, LNP64_ERR_EBADF, src_fd, src_fd);
                        m1_state_projection_reg <= build_m1_state_projection(
                            LNP64_M1_COMMIT_CAP_REVOKE, LNP64_ERR_EBADF, src_fd, src_fd);
                    end
                end
            end
        end
    end

`ifndef SYNTHESIS
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            sampled_cap_queue_valid <= 1'b0;
            sampled_cap_queue_rights <= 64'd0;
            sampled_cap_queue_object_id <= 64'd0;
            sampled_cap_queue_lineage <= 64'd0;
            sampled_cap_queue_generation <= 64'd0;
            sampled_cap_queue_domain_id <= 32'd0;
            sampled_cap_queue_revoked <= 1'b0;
        end else begin
            sampled_cap_queue_valid <= cap_queue_valid;
            sampled_cap_queue_rights <= cap_queue_rights;
            sampled_cap_queue_object_id <= cap_queue_object_id;
            sampled_cap_queue_lineage <= cap_queue_lineage;
            sampled_cap_queue_generation <= cap_queue_generation;
            sampled_cap_queue_domain_id <= cap_queue_domain_id;
            sampled_cap_queue_revoked <= cap_queue_revoked;
        end
    end

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
        end else if (m1_commit_valid_reg && have_rsp) begin
            assert (projection_root_and_consumer_backed_by_fdr(m1_pre_state_projection_reg))
                else $fatal(1, "SG-AUTH cap-engine M1 pre-state projection was not backed by FDR state");
            assert (projection_root_and_consumer_backed_by_fdr(m1_state_projection_reg))
                else $fatal(1, "SG-AUTH cap-engine M1 post-state projection was not backed by FDR state");
            assert (sent_projection_backed_by_sampled_queue(m1_pre_state_projection_reg))
                else $fatal(1, "SG-AUTH cap-engine M1 pre-state sent projection was not queue-backed or zero");
            assert (sent_projection_backed_by_current_queue(m1_state_projection_reg))
                else $fatal(1, "SG-AUTH cap-engine M1 post-state sent projection was not queue-backed or zero");
            assert (minted_projection_is_zero(m1_pre_state_projection_reg))
                else $fatal(1, "SG-AUTH cap-engine M1 pre-state minted projection carried unowned authority");
            assert (minted_projection_is_zero(m1_state_projection_reg))
                else $fatal(1, "SG-AUTH cap-engine M1 post-state minted projection carried unowned authority");
            if (m1_commit_reg.status == LNP64_ERR_OK) begin
                unique case (m1_commit_reg.op)
                    LNP64_M1_COMMIT_CAP_DUP,
                    LNP64_M1_COMMIT_CAP_RECV: begin
                        assert (consumer_projection_matches_commit(
                            m1_state_projection_reg,
                            m1_commit_reg
                        )) else $fatal(1, "SG-AUTH cap-engine M1 consumer projection drifted from commit");
                        assert (live_fdr_projection_exists(
                            m1_state_projection_reg.consumer_object_id,
                            m1_state_projection_reg.consumer_generation,
                            m1_state_projection_reg.consumer_domain_id,
                            m1_state_projection_reg.consumer_lineage_epoch,
                            m1_state_projection_reg.consumer_rights
                        )) else $fatal(1, "SG-AUTH cap-engine M1 consumer projection was not backed by live FDR state");
                    end
                    LNP64_M1_COMMIT_CAP_SEND: begin
                        assert (sent_projection_matches_commit(
                            m1_state_projection_reg,
                            m1_commit_reg
                        )) else $fatal(1, "SG-AUTH cap-engine M1 sent projection drifted from commit");
                        assert (cap_queue_matches_sent_projection(m1_state_projection_reg))
                            else $fatal(1, "SG-AUTH cap-engine M1 sent projection was not backed by transfer queue state");
                    end
                    LNP64_M1_COMMIT_CAP_REVOKE: begin
                        assert (revoked_root_projection_matches_commit(
                            m1_state_projection_reg,
                            m1_commit_reg
                        )) else $fatal(1, "SG-AUTH cap-engine M1 revoke projection drifted from commit");
                        assert (!live_lineage_exists(m1_commit_reg.lineage_epoch))
                            else $fatal(1, "SG-AUTH cap-engine M1 revoke left live FDR authority in revoked lineage");
                    end
                    default: begin
                    end
                endcase
            end else begin
                assert (authority_projection_slots_match(
                    m1_pre_state_projection_reg,
                    m1_state_projection_reg
                )) else $fatal(1, "SG-AUTH cap-engine M1 non-OK commit changed authority projection");
            end
        end
    end
`endif
endmodule

module lnp64_domain_engine(
    input logic clk,
    input logic reset_n,
    input logic cmd_valid,
    output logic cmd_ready,
    input lnp64_cmd_t cmd,
    output logic rsp_valid,
    input logic rsp_ready,
    output lnp64_rsp_t rsp,
    output logic [31:0] telemetry_counter,
    output logic [31:0] fault_counter
);
    localparam int unsigned DOMAIN_SLOT_COUNT = 8;
    localparam logic [63:0] DOMAIN_OP_CREATE = 64'd1;
    localparam logic [63:0] DOMAIN_OP_CONFIGURE = 64'd2;
    localparam logic [63:0] DOMAIN_OP_QUERY = 64'd3;
    localparam logic [63:0] DOMAIN_OP_FREEZE = 64'd4;
    localparam logic [63:0] DOMAIN_OP_RESUME = 64'd5;
    localparam logic [63:0] DOMAIN_OP_DESTROY = 64'd6;
    localparam logic [63:0] DOMAIN_OP_ATTACH_SELF = 64'd7;
    localparam logic [63:0] DOMAIN_ROOT_ID = 64'd1;
    localparam logic [63:0] DOMAIN_QUERY_SIZE = 64'd200;

    logic have_rsp;
    lnp64_rsp_t rsp_reg;
    logic [63:0] domain_next_id;
    logic domain_valid [0:DOMAIN_SLOT_COUNT-1];
    logic domain_destroyed [0:DOMAIN_SLOT_COUNT-1];
    logic [63:0] domain_generation [0:DOMAIN_SLOT_COUNT-1];
    logic [63:0] domain_parent [0:DOMAIN_SLOT_COUNT-1];
    logic [63:0] domain_profile [0:DOMAIN_SLOT_COUNT-1];
    logic [63:0] domain_cpu_limit [0:DOMAIN_SLOT_COUNT-1];
    logic [63:0] domain_memory_limit [0:DOMAIN_SLOT_COUNT-1];
    logic [63:0] domain_pids_limit [0:DOMAIN_SLOT_COUNT-1];
    logic [63:0] domain_fdrs_limit [0:DOMAIN_SLOT_COUNT-1];
    logic [63:0] domain_cap_mask [0:DOMAIN_SLOT_COUNT-1];
    logic [63:0] domain_upcall_mask [0:DOMAIN_SLOT_COUNT-1];
    logic domain_frozen [0:DOMAIN_SLOT_COUNT-1];

    assign cmd_ready = reset_n && !have_rsp;
    assign rsp_valid = have_rsp;
    assign rsp = rsp_reg;

    integer i;
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            have_rsp <= 1'b0;
            rsp_reg <= '0;
            telemetry_counter <= 32'd0;
            fault_counter <= 32'd0;
            domain_next_id <= 64'd2;
            for (i = 0; i < DOMAIN_SLOT_COUNT; i = i + 1) begin
                domain_valid[i] <= i == 0;
                domain_destroyed[i] <= 1'b0;
                domain_generation[i] <= 64'd1;
                domain_parent[i] <= i == 0 ? 64'd0 : DOMAIN_ROOT_ID;
                domain_profile[i] <= 64'd0;
                domain_cpu_limit[i] <= i == 0 ? 64'hffff_ffff_ffff_ffff : 64'd0;
                domain_memory_limit[i] <= i == 0 ? 64'hffff_ffff_ffff_ffff : 64'd0;
                domain_pids_limit[i] <= i == 0 ? 64'hffff_ffff_ffff_ffff : 64'd0;
                domain_fdrs_limit[i] <= i == 0 ? 64'hffff_ffff_ffff_ffff : 64'd0;
                domain_cap_mask[i] <= i == 0 ? 64'hffff_ffff_ffff_ffff : 64'd0;
                domain_upcall_mask[i] <= i == 0 ? 64'hffff_ffff_ffff_ffff : 64'd0;
                domain_frozen[i] <= 1'b0;
            end
        end else begin
            if (have_rsp && rsp_ready) begin
                have_rsp <= 1'b0;
            end
            if (cmd_valid && cmd_ready) begin : accept_cmd
                logic [63:0] op;
                logic [63:0] ref_id;
                logic [63:0] generation_req;
                logic [63:0] profile_req;
                logic [63:0] cpu_req;
                logic [63:0] memory_req;
                logic [63:0] pids_req;
                logic [63:0] fdrs_req;
                logic [63:0] caps_req;
                logic [63:0] upcalls_req;
                int unsigned ref_slot;
                int unsigned create_slot;
                logic ref_in_range;
                logic ref_live;
                logic create_in_range;
                logic limits_narrow;

                op = cmd.arg0;
                ref_id = cmd.arg1 == 64'd0 ? DOMAIN_ROOT_ID : cmd.arg1;
                generation_req = cmd.arg2;
                profile_req = cmd.arg3;
                cpu_req = cmd.arg_block_ptr;
                memory_req = cmd.arg_block_len;
                pids_req = {48'd0, cmd.cancel_class};
                fdrs_req = {48'd0, cmd.completion_target};
                caps_req = cmd.rights_mask;
                upcalls_req = cmd.flags;
                ref_slot = (ref_id > 64'd0 && (ref_id - 64'd1) < DOMAIN_SLOT_COUNT) ?
                    int'(ref_id - 64'd1) : 0;
                create_slot = (domain_next_id > 64'd0 &&
                    (domain_next_id - 64'd1) < DOMAIN_SLOT_COUNT) ?
                    int'(domain_next_id - 64'd1) : 0;
                ref_in_range = ref_id > 64'd0 && (ref_id - 64'd1) < DOMAIN_SLOT_COUNT;
                ref_live = ref_in_range && domain_valid[ref_slot] &&
                    !domain_destroyed[ref_slot] &&
                    (generation_req == 64'd0 || generation_req == domain_generation[ref_slot]);
                create_in_range = domain_next_id > 64'd0 && create_slot < DOMAIN_SLOT_COUNT;
                limits_narrow =
                    (cpu_req == 64'd0 || cpu_req <= domain_cpu_limit[ref_slot]) &&
                    (memory_req == 64'd0 || memory_req <= domain_memory_limit[ref_slot]) &&
                    (pids_req == 64'd0 || pids_req <= domain_pids_limit[ref_slot]) &&
                    (fdrs_req == 64'd0 || fdrs_req <= domain_fdrs_limit[ref_slot]) &&
                    (caps_req == 64'd0 || ((caps_req & ~domain_cap_mask[ref_slot]) == 64'd0)) &&
                    (upcalls_req == 64'd0 || ((upcalls_req & ~domain_upcall_mask[ref_slot]) == 64'd0));

                have_rsp <= 1'b1;
                telemetry_counter <= telemetry_counter + 32'd1;
                rsp_reg <= '0;
                rsp_reg.op_id <= cmd.op_id;
                rsp_reg.tile_id <= cmd.tile_id;
                rsp_reg.pid <= cmd.pid;
                rsp_reg.tid <= cmd.tid;
                rsp_reg.domain_id <= cmd.domain_id;
                rsp_reg.domain_gen <= cmd.domain_gen;
                rsp_reg.result_reg <= cmd.result_reg;
                rsp_reg.result_value <= 64'hffff_ffff_ffff_ffff;
                rsp_reg.errno_value <= LNP64_ERR_EINVAL;
                rsp_reg.status <= LNP64_STATUS_ERROR;

                if (op == DOMAIN_OP_CREATE && ref_live && create_in_range &&
                    !domain_valid[create_slot] && limits_narrow) begin
                    domain_valid[create_slot] <= 1'b1;
                    domain_destroyed[create_slot] <= 1'b0;
                    domain_generation[create_slot] <= 64'd1;
                    domain_parent[create_slot] <= ref_id;
                    domain_profile[create_slot] <= profile_req;
                    domain_cpu_limit[create_slot] <= cpu_req == 64'd0 ?
                        domain_cpu_limit[ref_slot] : cpu_req;
                    domain_memory_limit[create_slot] <= memory_req == 64'd0 ?
                        domain_memory_limit[ref_slot] : memory_req;
                    domain_pids_limit[create_slot] <= pids_req == 64'd0 ?
                        domain_pids_limit[ref_slot] : pids_req;
                    domain_fdrs_limit[create_slot] <= fdrs_req == 64'd0 ?
                        domain_fdrs_limit[ref_slot] : fdrs_req;
                    domain_cap_mask[create_slot] <= caps_req == 64'd0 ?
                        domain_cap_mask[ref_slot] : caps_req;
                    domain_upcall_mask[create_slot] <= upcalls_req == 64'd0 ?
                        domain_upcall_mask[ref_slot] : upcalls_req;
                    domain_frozen[create_slot] <= 1'b0;
                    rsp_reg.result_value <= domain_next_id;
                    rsp_reg.errno_value <= LNP64_ERR_OK;
                    rsp_reg.status <= LNP64_STATUS_OK;
                    domain_next_id <= domain_next_id + 64'd1;
                end else if (op == DOMAIN_OP_QUERY && ref_live) begin
                    rsp_reg.result_value <= DOMAIN_QUERY_SIZE;
                    rsp_reg.errno_value <= LNP64_ERR_OK;
                    rsp_reg.status <= LNP64_STATUS_OK;
                end else if (op == DOMAIN_OP_CONFIGURE && ref_live &&
                    !domain_frozen[ref_slot] && limits_narrow) begin
                    if (profile_req != 64'd0) begin
                        domain_profile[ref_slot] <= profile_req;
                    end
                    if (cpu_req != 64'd0) begin
                        domain_cpu_limit[ref_slot] <= cpu_req;
                    end
                    if (memory_req != 64'd0) begin
                        domain_memory_limit[ref_slot] <= memory_req;
                    end
                    if (pids_req != 64'd0) begin
                        domain_pids_limit[ref_slot] <= pids_req;
                    end
                    if (fdrs_req != 64'd0) begin
                        domain_fdrs_limit[ref_slot] <= fdrs_req;
                    end
                    if (caps_req != 64'd0) begin
                        domain_cap_mask[ref_slot] <= caps_req;
                        for (i = 0; i < DOMAIN_SLOT_COUNT; i = i + 1) begin
                            if (domain_valid[i] && domain_parent[i] == ref_id) begin
                                domain_cap_mask[i] <= domain_cap_mask[i] & caps_req;
                            end
                        end
                    end
                    if (upcalls_req != 64'd0) begin
                        domain_upcall_mask[ref_slot] <= upcalls_req;
                        for (i = 0; i < DOMAIN_SLOT_COUNT; i = i + 1) begin
                            if (domain_valid[i] && domain_parent[i] == ref_id) begin
                                domain_upcall_mask[i] <= domain_upcall_mask[i] & upcalls_req;
                            end
                        end
                    end
                    rsp_reg.result_value <= 64'd0;
                    rsp_reg.errno_value <= LNP64_ERR_OK;
                    rsp_reg.status <= LNP64_STATUS_OK;
                end else if (op == DOMAIN_OP_FREEZE && ref_live) begin
                    domain_frozen[ref_slot] <= 1'b1;
                    for (i = 0; i < DOMAIN_SLOT_COUNT; i = i + 1) begin
                        if (domain_valid[i] && domain_parent[i] == ref_id) begin
                            domain_frozen[i] <= 1'b1;
                        end
                    end
                    rsp_reg.result_value <= 64'd0;
                    rsp_reg.errno_value <= LNP64_ERR_OK;
                    rsp_reg.status <= LNP64_STATUS_OK;
                end else if (op == DOMAIN_OP_RESUME && ref_live) begin
                    domain_frozen[ref_slot] <= 1'b0;
                    for (i = 0; i < DOMAIN_SLOT_COUNT; i = i + 1) begin
                        if (domain_valid[i] && domain_parent[i] == ref_id) begin
                            domain_frozen[i] <= 1'b0;
                        end
                    end
                    rsp_reg.result_value <= 64'd0;
                    rsp_reg.errno_value <= LNP64_ERR_OK;
                    rsp_reg.status <= LNP64_STATUS_OK;
                end else if (op == DOMAIN_OP_DESTROY && ref_live && ref_id != DOMAIN_ROOT_ID) begin
                    domain_destroyed[ref_slot] <= 1'b1;
                    domain_generation[ref_slot] <= domain_generation[ref_slot] + 64'd1;
                    rsp_reg.result_value <= 64'd0;
                    rsp_reg.errno_value <= LNP64_ERR_OK;
                    rsp_reg.status <= LNP64_STATUS_OK;
                end else if (op == DOMAIN_OP_ATTACH_SELF && ref_live && !domain_frozen[ref_slot]) begin
                    rsp_reg.result_value <= 64'd0;
                    rsp_reg.errno_value <= LNP64_ERR_OK;
                    rsp_reg.status <= LNP64_STATUS_OK;
                end else if (op == DOMAIN_OP_CREATE) begin
                    rsp_reg.errno_value <= LNP64_ERR_EPERM;
                end else if (op == DOMAIN_OP_QUERY || op == DOMAIN_OP_DESTROY) begin
                    rsp_reg.errno_value <= LNP64_ERR_ESTALE;
                end else if (op == DOMAIN_OP_CONFIGURE || op == DOMAIN_OP_FREEZE ||
                    op == DOMAIN_OP_RESUME || op == DOMAIN_OP_ATTACH_SELF) begin
                    rsp_reg.errno_value <= LNP64_ERR_EPERM;
                end
            end
        end
    end
endmodule

module lnp64_object_engine(
    input logic clk,
    input logic reset_n,
    input logic cmd_valid,
    output logic cmd_ready,
    input lnp64_cmd_t cmd,
    output logic rsp_valid,
    input logic rsp_ready,
    output lnp64_rsp_t rsp,
    output logic cap_sync_valid,
    output logic [31:0] cap_sync_reader_fd,
    output logic [31:0] cap_sync_writer_fd,
    output logic cap_sync_single_valid,
    output logic [31:0] cap_sync_single_fd,
    output logic [2:0] cap_sync_single_kind,
    output logic [63:0] cap_sync_single_lineage,
    output logic [31:0] telemetry_counter,
    output logic [31:0] fault_counter
);
    localparam logic [63:0] OBJECT_LINEAGE_CALL_GATE_BASE = 64'd1281;
    localparam logic [63:0] OBJECT_LINEAGE_MEMORY_OBJECT = 64'd513;
    localparam logic [63:0] OBJECT_LINEAGE_DMA_BUFFER = 64'd1281;
    localparam logic [63:0] CALL_MODE_SYNC = 64'd0;
    localparam logic [63:0] CALL_MODE_ASYNC = 64'd1;
    localparam logic [63:0] CALL_MODE_HANDOFF = 64'd2;
    localparam int unsigned OBJECT_MEMORY_DEFAULT_FD = 5;
    localparam int unsigned OBJECT_DMA_BUFFER_DEFAULT_FD = 3;

    logic have_rsp;
    lnp64_rsp_t rsp_reg;
    logic fdr_valid [0:LNP64_FDR_SLOT_COUNT-1];
    int unsigned pipe_alloc_reader_fd;
    int unsigned pipe_alloc_writer_fd;
    logic pipe_alloc_available;

    assign cmd_ready = reset_n && !have_rsp;
    assign rsp_valid = have_rsp;
    assign rsp = rsp_reg;

    integer scan_i;
    always_comb begin
        pipe_alloc_reader_fd = 3;
        pipe_alloc_writer_fd = 4;
        pipe_alloc_available = 1'b0;
        for (scan_i = 3; scan_i + 1 < LNP64_FDR_SLOT_COUNT; scan_i = scan_i + 2) begin
            if (!pipe_alloc_available && !fdr_valid[scan_i] && !fdr_valid[scan_i + 1]) begin
                pipe_alloc_reader_fd = scan_i;
                pipe_alloc_writer_fd = scan_i + 1;
                pipe_alloc_available = 1'b1;
            end
        end
    end

    integer i;
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            have_rsp <= 1'b0;
            rsp_reg <= '0;
            telemetry_counter <= 32'd0;
            fault_counter <= 32'd0;
            cap_sync_valid <= 1'b0;
            cap_sync_reader_fd <= 32'd0;
            cap_sync_writer_fd <= 32'd0;
            cap_sync_single_valid <= 1'b0;
            cap_sync_single_fd <= 32'd0;
            cap_sync_single_kind <= LNP64_FDR_KIND_CLOSED;
            cap_sync_single_lineage <= 64'd0;
            for (i = 0; i < LNP64_FDR_SLOT_COUNT; i = i + 1) begin
                fdr_valid[i] <= i < 3;
            end
        end else begin
            cap_sync_valid <= 1'b0;
            cap_sync_single_valid <= 1'b0;
            if (have_rsp && rsp_ready) begin
                have_rsp <= 1'b0;
            end
            if (cmd_valid && cmd_ready) begin : accept_cmd
                int unsigned reader_fd;
                int unsigned writer_fd;
                int unsigned single_fd;
                logic explicit_pair_valid;
                logic auto_pair_valid;

                have_rsp <= 1'b1;
                telemetry_counter <= telemetry_counter + 32'd1;
                rsp_reg <= '0;
                rsp_reg.op_id <= cmd.op_id;
                rsp_reg.tile_id <= cmd.tile_id;
                rsp_reg.pid <= cmd.pid;
                rsp_reg.tid <= cmd.tid;
                rsp_reg.domain_id <= cmd.domain_id;
                rsp_reg.domain_gen <= cmd.domain_gen;
                rsp_reg.result_reg <= cmd.result_reg;
                rsp_reg.result_value <= 64'hffff_ffff_ffff_ffff;
                rsp_reg.errno_value <= LNP64_ERR_EINVAL;
                rsp_reg.status <= LNP64_STATUS_ERROR;

                if (cmd.opcode == LNP64_OP_OBJECT_CTL &&
                    cmd.arg0 == LNP64_OBJECT_OP_CREATE &&
                    cmd.arg1 == LNP64_OBJECT_KIND_QUEUE &&
                    cmd.arg2 == LNP64_OBJECT_PROFILE_PIPE) begin
                    reader_fd = cmd.arg3 == 64'd0 ? pipe_alloc_reader_fd : cmd.arg3[31:0];
                    writer_fd = cmd.arg3 == 64'd0 ? pipe_alloc_writer_fd : cmd.rights_mask[31:0];
                    auto_pair_valid = cmd.arg3 == 64'd0 && cmd.rights_mask == 64'd0 &&
                        pipe_alloc_available;
                    explicit_pair_valid = cmd.arg3 != 64'd0 &&
                        cmd.arg3 < LNP64_FDR_SLOT_COUNT &&
                        cmd.rights_mask == cmd.arg3 + 64'd1 &&
                        cmd.rights_mask < LNP64_FDR_SLOT_COUNT &&
                        !fdr_valid[cmd.arg3[31:0]] &&
                        !fdr_valid[cmd.rights_mask[31:0]];
                    if (auto_pair_valid || explicit_pair_valid) begin
                        fdr_valid[reader_fd] <= 1'b1;
                        fdr_valid[writer_fd] <= 1'b1;
                        rsp_reg.result_value <= 64'd0;
                        rsp_reg.errno_value <= LNP64_ERR_OK;
                        rsp_reg.status <= LNP64_STATUS_OK;
                        rsp_reg.event_mask <= {writer_fd[31:0], reader_fd[31:0]};
                        cap_sync_valid <= 1'b1;
                        cap_sync_reader_fd <= reader_fd[31:0];
                        cap_sync_writer_fd <= writer_fd[31:0];
                    end else begin
                        rsp_reg.errno_value <= LNP64_ERR_EINVAL;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                    end
                end else if (cmd.opcode == LNP64_OP_OBJECT_CTL &&
                    cmd.arg0 == LNP64_OBJECT_OP_CREATE &&
                    cmd.arg1 == LNP64_OBJECT_KIND_QUEUE &&
                    cmd.arg2 == LNP64_OBJECT_PROFILE_CALL_GATE) begin
                    single_fd = cmd.arg3 == 64'd0 ? 3 : cmd.arg3[31:0];
                    if (single_fd < LNP64_FDR_SLOT_COUNT &&
                        (cmd.arg_block_len == CALL_MODE_SYNC ||
                         cmd.arg_block_len == CALL_MODE_ASYNC ||
                         cmd.arg_block_len == CALL_MODE_HANDOFF) &&
                        cmd.cancel_class == 16'd0 &&
                        (cmd.arg_block_len != CALL_MODE_ASYNC ||
                            (cmd.rights_mask < LNP64_FDR_SLOT_COUNT &&
                             fdr_valid[cmd.rights_mask[31:0]]))) begin
                        fdr_valid[single_fd] <= 1'b1;
                        rsp_reg.result_value <= {32'd0, single_fd[31:0]};
                        rsp_reg.errno_value <= LNP64_ERR_OK;
                        rsp_reg.status <= LNP64_STATUS_OK;
                        cap_sync_single_valid <= 1'b1;
                        cap_sync_single_fd <= single_fd[31:0];
                        cap_sync_single_kind <= LNP64_FDR_KIND_CALL_GATE;
                        cap_sync_single_lineage <= OBJECT_LINEAGE_CALL_GATE_BASE + {32'd0, single_fd[31:0]};
                    end else begin
                        rsp_reg.errno_value <= LNP64_ERR_EINVAL;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                    end
                end else if (cmd.opcode == LNP64_OP_OBJECT_CTL &&
                    cmd.arg0 == LNP64_OBJECT_OP_CREATE &&
                    cmd.arg1 == LNP64_OBJECT_KIND_COUNTER &&
                    (cmd.arg2 == 64'd0 || cmd.arg2 == 64'd1)) begin
                    single_fd = cmd.arg3 == 64'd0 ? 3 : cmd.arg3[31:0];
                    if (single_fd < LNP64_FDR_SLOT_COUNT) begin
                        fdr_valid[single_fd] <= 1'b1;
                        rsp_reg.result_value <= {32'd0, single_fd[31:0]};
                        rsp_reg.errno_value <= LNP64_ERR_OK;
                        rsp_reg.status <= LNP64_STATUS_OK;
                        cap_sync_single_valid <= 1'b1;
                        cap_sync_single_fd <= single_fd[31:0];
                        cap_sync_single_kind <= LNP64_FDR_KIND_GENERIC;
                        cap_sync_single_lineage <= 64'd769;
                    end else begin
                        rsp_reg.errno_value <= LNP64_ERR_EINVAL;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                    end
                end else if (cmd.opcode == LNP64_OP_OBJECT_CTL &&
                    cmd.arg0 == LNP64_OBJECT_OP_CREATE &&
                    cmd.arg1 == LNP64_OBJECT_KIND_TIMER) begin
                    single_fd = cmd.arg3 == 64'd0 ? 3 : cmd.arg3[31:0];
                    if (single_fd < LNP64_FDR_SLOT_COUNT) begin
                        fdr_valid[single_fd] <= 1'b1;
                        rsp_reg.result_value <= {32'd0, single_fd[31:0]};
                        rsp_reg.errno_value <= LNP64_ERR_OK;
                        rsp_reg.status <= LNP64_STATUS_OK;
                        cap_sync_single_valid <= 1'b1;
                        cap_sync_single_fd <= single_fd[31:0];
                        cap_sync_single_kind <= LNP64_FDR_KIND_GENERIC;
                        cap_sync_single_lineage <= 64'd1025;
                    end else begin
                        rsp_reg.errno_value <= LNP64_ERR_EINVAL;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                    end
                end else if (cmd.opcode == LNP64_OP_OBJECT_CTL &&
                    cmd.arg0 == LNP64_OBJECT_OP_CREATE &&
                    cmd.arg1 == LNP64_OBJECT_KIND_MEMORY_OBJECT) begin
                    single_fd = cmd.arg3 == 64'd0 ? OBJECT_MEMORY_DEFAULT_FD : cmd.arg3[31:0];
                    if (cmd.flags != 64'd0 && single_fd < LNP64_FDR_SLOT_COUNT) begin
                        fdr_valid[single_fd] <= 1'b1;
                        rsp_reg.result_value <= {32'd0, single_fd[31:0]};
                        rsp_reg.errno_value <= LNP64_ERR_OK;
                        rsp_reg.status <= LNP64_STATUS_OK;
                        cap_sync_single_valid <= 1'b1;
                        cap_sync_single_fd <= single_fd[31:0];
                        cap_sync_single_kind <= LNP64_FDR_KIND_GENERIC;
                        cap_sync_single_lineage <= OBJECT_LINEAGE_MEMORY_OBJECT;
                    end else begin
                        rsp_reg.errno_value <= LNP64_ERR_EINVAL;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                    end
                end else if (cmd.opcode == LNP64_OP_OBJECT_CTL &&
                    cmd.arg0 == LNP64_OBJECT_OP_CREATE &&
                    cmd.arg1 == LNP64_OBJECT_KIND_DMA_BUFFER) begin
                    single_fd = cmd.arg3 == 64'd0 ? OBJECT_DMA_BUFFER_DEFAULT_FD : cmd.arg3[31:0];
                    if (cmd.arg_block_len == 64'd0) begin
                        rsp_reg.errno_value <= LNP64_ERR_EINVAL;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                    end else if (cmd.cancel_class != 16'd0 &&
                        single_fd == OBJECT_DMA_BUFFER_DEFAULT_FD) begin
                        fdr_valid[single_fd] <= 1'b1;
                        rsp_reg.result_value <= {32'd0, single_fd[31:0]};
                        rsp_reg.errno_value <= LNP64_ERR_OK;
                        rsp_reg.status <= LNP64_STATUS_OK;
                        cap_sync_single_valid <= 1'b1;
                        cap_sync_single_fd <= single_fd[31:0];
                        cap_sync_single_kind <= LNP64_FDR_KIND_GENERIC;
                        cap_sync_single_lineage <= OBJECT_LINEAGE_DMA_BUFFER;
                    end else begin
                        rsp_reg.errno_value <= LNP64_ERR_EFAULT;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                    end
                end
            end
        end
    end
endmodule

module lnp64_gate_engine(input logic clk, input logic reset_n, input logic cmd_valid, output logic cmd_ready, input lnp64_cmd_t cmd, output logic rsp_valid, input logic rsp_ready, output lnp64_rsp_t rsp, output logic [31:0] telemetry_counter, output logic [31:0] fault_counter);
    logic unused_fault_valid; lnp64_fault_t unused_fault;
    lnp64_fail_closed_engine #(.ENGINE_ID(16'd13), .ERRNO_VALUE(LNP64_ERR_ENOTSUP), .STATUS_VALUE(LNP64_STATUS_UNSUPPORTED)) shell(.*,.fault_valid(unused_fault_valid),.fault_ready(1'b1),.fault(unused_fault),.accepted_counter(telemetry_counter),.fault_counter(fault_counter));
endmodule

module lnp64_process_engine(input logic clk, input logic reset_n, input logic cmd_valid, output logic cmd_ready, input lnp64_cmd_t cmd, output logic rsp_valid, input logic rsp_ready, output lnp64_rsp_t rsp, output logic [31:0] telemetry_counter, output logic [31:0] fault_counter);
    logic unused_fault_valid; lnp64_fault_t unused_fault;
    lnp64_fail_closed_engine #(.ENGINE_ID(16'd14), .ERRNO_VALUE(LNP64_ERR_ENOTSUP), .STATUS_VALUE(LNP64_STATUS_UNSUPPORTED)) shell(.*,.fault_valid(unused_fault_valid),.fault_ready(1'b1),.fault(unused_fault),.accepted_counter(telemetry_counter),.fault_counter(fault_counter));
endmodule

module lnp64_vma_engine(
    input logic clk,
    input logic reset_n,
    input logic cmd_valid,
    output logic cmd_ready,
    input lnp64_cmd_t cmd,
    output logic rsp_valid,
    input logic rsp_ready,
    output lnp64_rsp_t rsp,
    output logic [31:0] telemetry_counter,
    output logic [31:0] fault_counter
);
    localparam logic [63:0] MMAP_ARCH_BASE = 64'h0000_0000_0020_e000;

    logic have_rsp;
    lnp64_rsp_t rsp_reg;
    logic [63:0] mmap_next;
    logic [63:0] vma_start [0:3];
    logic [63:0] vma_len [0:3];
    logic [63:0] vma_prot [0:3];
    logic vma_valid [0:3];
    logic [1:0] vma_next_slot;

    assign cmd_ready = reset_n && !have_rsp;
    assign rsp_valid = have_rsp;
    assign rsp = rsp_reg;

    function automatic logic [63:0] align_up_u64(input logic [63:0] value, input logic [63:0] align);
        logic [63:0] mask;
        begin
            if (align <= 64'd1) begin
                align_up_u64 = value;
            end else begin
                mask = align - 64'd1;
                align_up_u64 = (value + mask) & ~mask;
            end
        end
    endfunction

    integer i;
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            have_rsp <= 1'b0;
            rsp_reg <= '0;
            telemetry_counter <= 32'd0;
            fault_counter <= 32'd0;
            mmap_next <= MMAP_ARCH_BASE;
            vma_next_slot <= 2'd0;
            for (i = 0; i < 4; i = i + 1) begin
                vma_start[i] <= 64'd0;
                vma_len[i] <= 64'd0;
                vma_prot[i] <= 64'd0;
                vma_valid[i] <= 1'b0;
            end
        end else begin
            if (have_rsp && rsp_ready) begin
                have_rsp <= 1'b0;
            end
            if (cmd_valid && cmd_ready) begin : accept_cmd
                logic [63:0] map_addr;
                logic range_hit;
                int unsigned hit_slot;

                have_rsp <= 1'b1;
                telemetry_counter <= telemetry_counter + 32'd1;
                rsp_reg <= '0;
                rsp_reg.op_id <= cmd.op_id;
                rsp_reg.tile_id <= cmd.tile_id;
                rsp_reg.pid <= cmd.pid;
                rsp_reg.tid <= cmd.tid;
                rsp_reg.domain_id <= cmd.domain_id;
                rsp_reg.domain_gen <= cmd.domain_gen;
                rsp_reg.result_reg <= cmd.result_reg;
                rsp_reg.result_value <= 64'hffff_ffff_ffff_ffff;
                rsp_reg.errno_value <= LNP64_ERR_OK;
                rsp_reg.status <= LNP64_STATUS_OK;

                if (cmd.opcode == LNP64_OP_MMAP) begin
                    map_addr = cmd.arg0 != 64'd0 ? cmd.arg0 : align_up_u64(mmap_next, 64'd4096);
                    if (cmd.arg1 == 64'd0 || (cmd.arg2 & ~64'd7) != 64'd0) begin
                        rsp_reg.errno_value <= LNP64_ERR_EINVAL;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                    end else if ((cmd.arg2 & 64'd6) == 64'd6) begin
                        rsp_reg.errno_value <= LNP64_ERR_EPERM;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                    end else if (cmd.arg3 != 64'd0 || cmd.flags != 64'd0) begin
                        rsp_reg.errno_value <= LNP64_ERR_EBADF;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                    end else begin
                        vma_start[vma_next_slot] <= map_addr;
                        vma_len[vma_next_slot] <= cmd.arg1;
                        vma_prot[vma_next_slot] <= cmd.arg2;
                        vma_valid[vma_next_slot] <= 1'b1;
                        vma_next_slot <= vma_next_slot + 2'd1;
                        mmap_next <= map_addr + cmd.arg1;
                        rsp_reg.result_value <= map_addr;
                    end
                end else if (cmd.opcode == LNP64_OP_MPROTECT) begin
                    range_hit = 1'b0;
                    hit_slot = 0;
                    for (i = 0; i < 4; i = i + 1) begin
                        if (!range_hit && vma_valid[i] &&
                            cmd.arg0 >= vma_start[i] &&
                            (cmd.arg0 + cmd.arg1) <= (vma_start[i] + vma_len[i])) begin
                            range_hit = 1'b1;
                            hit_slot = i;
                        end
                    end
                    if (cmd.arg1 == 64'd0 || (cmd.arg2 & ~64'd7) != 64'd0) begin
                        rsp_reg.errno_value <= LNP64_ERR_EINVAL;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                    end else if ((cmd.arg2 & 64'd6) == 64'd6) begin
                        rsp_reg.errno_value <= LNP64_ERR_EPERM;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                    end else if (!range_hit) begin
                        rsp_reg.errno_value <= LNP64_ERR_EINVAL;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                    end else begin
                        vma_prot[hit_slot] <= cmd.arg2;
                        rsp_reg.result_value <= 64'd0;
                    end
                end else begin
                    rsp_reg.errno_value <= LNP64_ERR_ENOTSUP;
                    rsp_reg.status <= LNP64_STATUS_UNSUPPORTED;
                end
            end
        end
    end
endmodule

module lnp64_page_allocator(input logic clk, input logic reset_n, output logic idle, output logic [31:0] telemetry_counter, output logic [31:0] fault_counter);
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin idle <= 1'b0; telemetry_counter <= 32'd0; fault_counter <= 32'd0; end
        else begin idle <= 1'b1; telemetry_counter <= 32'd1; end
    end
endmodule

module lnp64_memory_fabric(input logic clk, input logic reset_n, output logic coherence_event_path_live, output logic raw_physical_address_visible, output logic [31:0] telemetry_counter, output logic [31:0] fault_counter);
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin coherence_event_path_live <= 1'b0; raw_physical_address_visible <= 1'b0; telemetry_counter <= 32'd0; fault_counter <= 32'd0; end
        else begin coherence_event_path_live <= 1'b1; raw_physical_address_visible <= 1'b0; telemetry_counter <= 32'd1; end
    end
endmodule

module lnp64_metadata_broker(input logic clk, input logic reset_n, output logic idle, output logic [31:0] telemetry_counter, output logic [31:0] fault_counter);
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin idle <= 1'b0; telemetry_counter <= 32'd0; fault_counter <= 32'd0; end
        else begin idle <= 1'b1; telemetry_counter <= 32'd1; end
    end
endmodule

module lnp64_dma_fabric(
    input logic clk,
    input logic reset_n,
    input logic cmd_valid,
    output logic cmd_ready,
    input lnp64_cmd_t cmd,
    output logic rsp_valid,
    input logic rsp_ready,
    output lnp64_rsp_t rsp,
    output logic visibility_event_path_live,
    output logic raw_dma_authority_visible,
    output logic [31:0] telemetry_counter,
    output logic [31:0] fault_counter
);
    logic have_rsp;
    lnp64_rsp_t rsp_reg;

    assign cmd_ready = reset_n && !have_rsp;
    assign rsp_valid = have_rsp;
    assign rsp = rsp_reg;

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            have_rsp <= 1'b0;
            rsp_reg <= '0;
            visibility_event_path_live <= 1'b0;
            raw_dma_authority_visible <= 1'b0;
            telemetry_counter <= 32'd0;
            fault_counter <= 32'd0;
        end else begin
            visibility_event_path_live <= 1'b1;
            raw_dma_authority_visible <= 1'b0;
            if (have_rsp && rsp_ready) begin
                have_rsp <= 1'b0;
            end
            if (cmd_valid && cmd_ready) begin
                have_rsp <= 1'b1;
                telemetry_counter <= telemetry_counter + 32'd1;
                rsp_reg <= '0;
                rsp_reg.op_id <= cmd.op_id;
                rsp_reg.tile_id <= cmd.tile_id;
                rsp_reg.pid <= cmd.pid;
                rsp_reg.tid <= cmd.tid;
                rsp_reg.domain_id <= cmd.domain_id;
                rsp_reg.domain_gen <= cmd.domain_gen;
                rsp_reg.result_reg <= cmd.result_reg;
                rsp_reg.result_value <= 64'hffff_ffff_ffff_ffff;
                rsp_reg.errno_value <= LNP64_ERR_OK;
                rsp_reg.status <= LNP64_STATUS_OK;

                if (cmd.opcode != LNP64_OP_DMA_CTL) begin
                    rsp_reg.errno_value <= LNP64_ERR_ENOTSUP;
                    rsp_reg.status <= LNP64_STATUS_UNSUPPORTED;
                end else if (cmd.arg_block_ptr != 64'd0 && !cmd.flags[1]) begin
                    rsp_reg.errno_value <= LNP64_ERR_EBADF;
                    rsp_reg.status <= LNP64_STATUS_ERROR;
                end else if (cmd.arg_block_ptr != 64'd0 && cmd.flags[2]) begin
                    rsp_reg.errno_value <= LNP64_ERR_ESTALE;
                    rsp_reg.status <= LNP64_STATUS_ERROR;
                end else if (cmd.arg_block_ptr != 64'd0 && !cmd.flags[3]) begin
                    rsp_reg.errno_value <= LNP64_ERR_EFAULT;
                    rsp_reg.status <= LNP64_STATUS_ERROR;
                end else if (cmd.arg0 == 64'd2) begin
                    if (cmd.arg_block_ptr == 64'd0 && !cmd.flags[4]) begin
                        rsp_reg.errno_value <= LNP64_ERR_EFAULT;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                    end else if (cmd.arg3 == 64'd0) begin
                        rsp_reg.result_value <= 64'd0;
                    end else if (cmd.arg3 == 64'd1) begin
                        rsp_reg.result_value <= 64'd1;
                    end else begin
                        rsp_reg.errno_value <= LNP64_ERR_ENOTSUP;
                        rsp_reg.status <= LNP64_STATUS_UNSUPPORTED;
                    end
                end else if (cmd.arg0 == 64'd1) begin
                    if (cmd.arg_block_ptr == 64'd0 && (!cmd.flags[4] || !cmd.flags[5])) begin
                        rsp_reg.errno_value <= LNP64_ERR_EFAULT;
                        rsp_reg.status <= LNP64_STATUS_ERROR;
                    end else if (cmd.arg3 == 64'd0) begin
                        rsp_reg.result_value <= 64'd0;
                    end else if (cmd.arg3 == 64'd8) begin
                        rsp_reg.result_value <= 64'd8;
                    end else begin
                        rsp_reg.errno_value <= LNP64_ERR_ENOTSUP;
                        rsp_reg.status <= LNP64_STATUS_UNSUPPORTED;
                    end
                end else begin
                    rsp_reg.errno_value <= LNP64_ERR_EINVAL;
                    rsp_reg.status <= LNP64_STATUS_ERROR;
                end
            end
        end
    end
endmodule

module lnp64_service_boundary(input logic clk, input logic reset_n, input logic cmd_valid, output logic cmd_ready, input lnp64_cmd_t cmd, output logic rsp_valid, input logic rsp_ready, output lnp64_rsp_t rsp, output logic [31:0] telemetry_counter, output logic [31:0] fault_counter);
    logic unused_fault_valid; lnp64_fault_t unused_fault;
    lnp64_fail_closed_engine #(.ENGINE_ID(16'd16), .ERRNO_VALUE(LNP64_ERR_ENOTSUP), .STATUS_VALUE(LNP64_STATUS_UNSUPPORTED)) shell(.*,.fault_valid(unused_fault_valid),.fault_ready(1'b1),.fault(unused_fault),.accepted_counter(telemetry_counter),.fault_counter(fault_counter));
endmodule

module lnp64_futex_atomic(input logic clk, input logic reset_n, output logic idle, output logic [31:0] telemetry_counter, output logic [31:0] fault_counter);
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin idle <= 1'b0; telemetry_counter <= 32'd0; fault_counter <= 32'd0; end
        else begin idle <= 1'b1; telemetry_counter <= 32'd1; end
    end
endmodule

module lnp64_heap_engine(
    input logic clk,
    input logic reset_n,
    input logic cmd_valid,
    output logic cmd_ready,
    input lnp64_cmd_t cmd,
    output logic rsp_valid,
    input logic rsp_ready,
    output lnp64_rsp_t rsp,
    output logic [31:0] telemetry_counter,
    output logic [31:0] fault_counter
);
    localparam logic [63:0] HEAP_ARCH_BASE = 64'h0000_0000_0010_f000;

    logic have_rsp;
    lnp64_rsp_t rsp_reg;
    logic [63:0] heap_next;
    logic [63:0] heap_alloc_ptr [0:3];
    logic [63:0] heap_alloc_size [0:3];
    logic heap_alloc_valid [0:3];
    logic [1:0] heap_alloc_next_slot;

    assign cmd_ready = reset_n && !have_rsp;
    assign rsp_valid = have_rsp;
    assign rsp = rsp_reg;

    function automatic logic [63:0] align_up_u64(input logic [63:0] value, input logic [63:0] align);
        logic [63:0] mask;
        begin
            if (align <= 64'd1) begin
                align_up_u64 = value;
            end else begin
                mask = align - 64'd1;
                align_up_u64 = (value + mask) & ~mask;
            end
        end
    endfunction

    function automatic logic [63:0] alloc_len_u64(input logic [63:0] len);
        begin
            alloc_len_u64 = len == 64'd0 ? 64'd1 : len;
        end
    endfunction

    function automatic logic [63:0] alloc_align_u64(input logic [63:0] requested);
        logic [63:0] clamped;
        logic [63:0] rounded;
        begin
            if (requested < 64'd1) begin
                clamped = 64'd1;
            end else if (requested > 64'd4096) begin
                clamped = 64'd4096;
            end else begin
                clamped = requested;
            end
            rounded = 64'd1;
            for (int align_bit = 0; align_bit < 12; align_bit = align_bit + 1) begin
                if (rounded < clamped) begin
                    rounded = rounded << 1;
                end
            end
            alloc_align_u64 = rounded;
        end
    endfunction

    integer i;
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            have_rsp <= 1'b0;
            rsp_reg <= '0;
            telemetry_counter <= 32'd0;
            fault_counter <= 32'd0;
            heap_next <= HEAP_ARCH_BASE;
            heap_alloc_next_slot <= 2'd0;
            for (i = 0; i < 4; i = i + 1) begin
                heap_alloc_ptr[i] <= 64'd0;
                heap_alloc_size[i] <= 64'd0;
                heap_alloc_valid[i] <= 1'b0;
            end
        end else begin
            if (have_rsp && rsp_ready) begin
                have_rsp <= 1'b0;
            end
            if (cmd_valid && cmd_ready) begin : accept_cmd
                logic [63:0] alloc_addr;
                logic [63:0] alloc_len;
                logic [63:0] alloc_align;

                have_rsp <= 1'b1;
                telemetry_counter <= telemetry_counter + 32'd1;
                rsp_reg <= '0;
                rsp_reg.op_id <= cmd.op_id;
                rsp_reg.tile_id <= cmd.tile_id;
                rsp_reg.pid <= cmd.pid;
                rsp_reg.tid <= cmd.tid;
                rsp_reg.domain_id <= cmd.domain_id;
                rsp_reg.domain_gen <= cmd.domain_gen;
                rsp_reg.result_reg <= cmd.result_reg;
                rsp_reg.result_value <= 64'd0;
                rsp_reg.errno_value <= LNP64_ERR_OK;
                rsp_reg.status <= LNP64_STATUS_OK;

                if (cmd.opcode == LNP64_OP_ALLOC) begin
                    alloc_len = alloc_len_u64(cmd.arg0);
                    alloc_addr = align_up_u64(heap_next, 64'd64);
                    heap_alloc_ptr[heap_alloc_next_slot] <= alloc_addr;
                    heap_alloc_size[heap_alloc_next_slot] <= alloc_len;
                    heap_alloc_valid[heap_alloc_next_slot] <= 1'b1;
                    heap_alloc_next_slot <= heap_alloc_next_slot + 2'd1;
                    heap_next <= alloc_addr + alloc_len;
                    rsp_reg.result_value <= alloc_addr;
                end else if (cmd.opcode == LNP64_OP_ALLOC_EX) begin
                    alloc_len = alloc_len_u64(cmd.arg0);
                    alloc_align = alloc_align_u64(cmd.arg1);
                    alloc_addr = align_up_u64(heap_next + 64'd4096, alloc_align);
                    heap_alloc_ptr[heap_alloc_next_slot] <= alloc_addr;
                    heap_alloc_size[heap_alloc_next_slot] <= alloc_len;
                    heap_alloc_valid[heap_alloc_next_slot] <= 1'b1;
                    heap_alloc_next_slot <= heap_alloc_next_slot + 2'd1;
                    heap_next <= alloc_addr + alloc_len + 64'd4096;
                    rsp_reg.result_value <= alloc_addr;
                end else if (cmd.opcode == LNP64_OP_ALLOC_SIZE) begin
                    for (i = 0; i < 4; i = i + 1) begin
                        if (heap_alloc_valid[i] && heap_alloc_ptr[i] == cmd.arg0) begin
                            rsp_reg.result_value <= heap_alloc_size[i];
                        end
                    end
                end else if (cmd.opcode == LNP64_OP_FREE) begin
                    for (i = 0; i < 4; i = i + 1) begin
                        if (heap_alloc_valid[i] && heap_alloc_ptr[i] == cmd.arg0) begin
                            heap_alloc_valid[i] <= 1'b0;
                        end
                    end
                end else begin
                    rsp_reg.errno_value <= LNP64_ERR_ENOTSUP;
                    rsp_reg.status <= LNP64_STATUS_UNSUPPORTED;
                end
            end
        end
    end
endmodule

module lnp64_classifier_servicelet(input logic clk, input logic reset_n, input logic cmd_valid, output logic cmd_ready, input lnp64_cmd_t cmd, output logic rsp_valid, input logic rsp_ready, output lnp64_rsp_t rsp, output logic [31:0] telemetry_counter, output logic [31:0] fault_counter);
    logic unused_fault_valid; lnp64_fault_t unused_fault;
    lnp64_fail_closed_engine #(.ENGINE_ID(16'd18), .ERRNO_VALUE(LNP64_ERR_ENOTSUP), .STATUS_VALUE(LNP64_STATUS_UNSUPPORTED)) shell(.*,.fault_valid(unused_fault_valid),.fault_ready(1'b1),.fault(unused_fault),.accepted_counter(telemetry_counter),.fault_counter(fault_counter));
endmodule

module lnp64_entropy_env(input logic clk, input logic reset_n, output logic [63:0] feature_bits, output logic [31:0] limit_threads);
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin feature_bits <= 64'd0; limit_threads <= 32'd0; end
        else begin feature_bits <= LNP64_S0_FEATURES; limit_threads <= 32'd1; end
    end
endmodule

module lnp64_uart(input logic clk, input logic reset_n, input logic boot_valid, output logic uart_valid, output logic [7:0] uart_byte);
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin uart_valid <= 1'b0; uart_byte <= 8'd0; end
        else begin uart_valid <= boot_valid; if (boot_valid) uart_byte <= 8'h53; end
    end
endmodule

module lnp64_storage_stub(input logic clk, input logic reset_n, output logic raw_device_authority_visible, output logic [31:0] telemetry_counter, output logic [31:0] fault_counter);
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin raw_device_authority_visible <= 1'b0; telemetry_counter <= 32'd0; fault_counter <= 32'd0; end
        else begin raw_device_authority_visible <= 1'b0; telemetry_counter <= 32'd1; end
    end
endmodule

module lnp64_eth_stub(input logic clk, input logic reset_n, output logic raw_interrupt_visible, output logic [31:0] telemetry_counter, output logic [31:0] fault_counter);
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin raw_interrupt_visible <= 1'b0; telemetry_counter <= 32'd0; fault_counter <= 32'd0; end
        else begin raw_interrupt_visible <= 1'b0; telemetry_counter <= 32'd1; end
    end
endmodule

module lnp64_pcie_stub(input logic clk, input logic reset_n, output logic raw_dma_authority_visible, output logic raw_interrupt_visible, output logic [31:0] telemetry_counter, output logic [31:0] fault_counter);
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin raw_dma_authority_visible <= 1'b0; raw_interrupt_visible <= 1'b0; telemetry_counter <= 32'd0; fault_counter <= 32'd0; end
        else begin raw_dma_authority_visible <= 1'b0; raw_interrupt_visible <= 1'b0; telemetry_counter <= 32'd1; end
    end
endmodule

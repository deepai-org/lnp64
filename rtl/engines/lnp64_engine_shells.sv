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
    lnp64_rsp_t rsp_reg;
    lnp64_fault_t fault_reg;

    assign cmd_ready = reset_n && !have_rsp;
    assign rsp_valid = have_rsp;
    assign rsp = rsp_reg;
    assign fault_valid = 1'b0;
    assign fault = fault_reg;

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            have_rsp <= 1'b0;
            rsp_reg <= '0;
            fault_reg <= '0;
            accepted_counter <= 32'd0;
            fault_counter <= 32'd0;
        end else begin
            if (cmd_valid && cmd_ready) begin
                have_rsp <= 1'b1;
                accepted_counter <= accepted_counter + 32'd1;
                rsp_reg.op_id <= cmd.op_id;
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
                fault_reg.op_id <= cmd.op_id;
                fault_reg.pid <= cmd.pid;
                fault_reg.tid <= cmd.tid;
                fault_reg.domain_id <= cmd.domain_id;
                fault_reg.domain_gen <= cmd.domain_gen;
                fault_reg.fault_code <= ERRNO_VALUE;
                fault_reg.source <= ENGINE_ID;
                fault_reg.detail <= 64'd0;
            end
            if (have_rsp && rsp_ready) begin
                have_rsp <= 1'b0;
            end
            if (fault_valid && fault_ready) begin
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
    output logic [31:0] routed_counter
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
            routed_counter <= 32'd0;
        end else begin
            if (cmd_valid && cmd_ready) begin
                routed_counter <= routed_counter + 32'd1;
                have_rsp <= 1'b1;
                rsp_reg.op_id <= cmd.op_id;
                rsp_reg.pid <= cmd.pid;
                rsp_reg.tid <= cmd.tid;
                rsp_reg.domain_id <= cmd.domain_id;
                rsp_reg.domain_gen <= cmd.domain_gen;
                rsp_reg.result_reg <= cmd.result_reg;
                rsp_reg.result_value <= 64'd0;
                rsp_reg.event_mask <= 64'd0;
                if (cmd.opcode == LNP64_OP_OBJECT_CTL) begin
                    rsp_reg.errno_value <= LNP64_ERR_EPERM;
                    rsp_reg.status <= LNP64_STATUS_ERROR;
                end else begin
                    rsp_reg.errno_value <= LNP64_ERR_ENOTSUP;
                    rsp_reg.status <= LNP64_STATUS_UNSUPPORTED;
                end
            end
            if (have_rsp && rsp_ready) begin
                have_rsp <= 1'b0;
            end
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

module lnp64_scheduler (
    input  logic clk,
    input  logic reset_n,
    input  logic boot_valid,
    input  logic park_pid1,
    input  logic wake_pid1,
    output logic exactly_one_location,
    output logic pid1_runnable,
    output logic pid1_parked
);
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            pid1_runnable <= 1'b0;
            pid1_parked <= 1'b0;
            exactly_one_location <= 1'b0;
        end else begin
            if (boot_valid) begin
                pid1_runnable <= 1'b1;
                pid1_parked <= 1'b0;
            end
            if (park_pid1) begin
                pid1_runnable <= 1'b0;
                pid1_parked <= 1'b1;
            end
            if (wake_pid1) begin
                pid1_runnable <= 1'b1;
                pid1_parked <= 1'b0;
            end
            exactly_one_location <= pid1_runnable ^ pid1_parked;
        end
    end
endmodule

module lnp64_event_router (
    input  logic clk,
    input  logic reset_n,
    input  logic synthetic_event,
    input  logic pid1_parked,
    output logic wake_valid,
    output logic event_valid,
    input  logic event_ready,
    output lnp64_event_t event_record,
    output logic [31:0] event_counter
);
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            wake_valid <= 1'b0;
            event_valid <= 1'b0;
            event_record <= '0;
            event_counter <= 32'd0;
        end else begin
            wake_valid <= 1'b0;
            if (synthetic_event && pid1_parked && !event_valid) begin
                event_counter <= event_counter + 32'd1;
                wake_valid <= 1'b1;
                event_valid <= 1'b1;
                event_record.event_id <= event_counter + 32'd1;
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

module lnp64_cap_engine(input logic clk, input logic reset_n, input logic cmd_valid, output logic cmd_ready, input lnp64_cmd_t cmd, output logic rsp_valid, input logic rsp_ready, output lnp64_rsp_t rsp, output logic [31:0] telemetry_counter, output logic [31:0] fault_counter);
    logic unused_fault_valid; lnp64_fault_t unused_fault;
    lnp64_fail_closed_engine #(.ENGINE_ID(16'd10), .ERRNO_VALUE(LNP64_ERR_EBADF), .STATUS_VALUE(LNP64_STATUS_ERROR)) shell(.*,.fault_valid(unused_fault_valid),.fault_ready(1'b1),.fault(unused_fault),.accepted_counter(telemetry_counter),.fault_counter(fault_counter));
endmodule

module lnp64_domain_engine(input logic clk, input logic reset_n, input logic cmd_valid, output logic cmd_ready, input lnp64_cmd_t cmd, output logic rsp_valid, input logic rsp_ready, output lnp64_rsp_t rsp, output logic [31:0] telemetry_counter, output logic [31:0] fault_counter);
    logic unused_fault_valid; lnp64_fault_t unused_fault;
    lnp64_fail_closed_engine #(.ENGINE_ID(16'd11), .ERRNO_VALUE(LNP64_ERR_EPERM), .STATUS_VALUE(LNP64_STATUS_ERROR)) shell(.*,.fault_valid(unused_fault_valid),.fault_ready(1'b1),.fault(unused_fault),.accepted_counter(telemetry_counter),.fault_counter(fault_counter));
endmodule

module lnp64_object_engine(input logic clk, input logic reset_n, input logic cmd_valid, output logic cmd_ready, input lnp64_cmd_t cmd, output logic rsp_valid, input logic rsp_ready, output lnp64_rsp_t rsp, output logic [31:0] telemetry_counter, output logic [31:0] fault_counter);
    logic unused_fault_valid; lnp64_fault_t unused_fault;
    lnp64_fail_closed_engine #(.ENGINE_ID(16'd12), .ERRNO_VALUE(LNP64_ERR_EPERM), .STATUS_VALUE(LNP64_STATUS_ERROR)) shell(.*,.fault_valid(unused_fault_valid),.fault_ready(1'b1),.fault(unused_fault),.accepted_counter(telemetry_counter),.fault_counter(fault_counter));
endmodule

module lnp64_gate_engine(input logic clk, input logic reset_n, input logic cmd_valid, output logic cmd_ready, input lnp64_cmd_t cmd, output logic rsp_valid, input logic rsp_ready, output lnp64_rsp_t rsp, output logic [31:0] telemetry_counter, output logic [31:0] fault_counter);
    logic unused_fault_valid; lnp64_fault_t unused_fault;
    lnp64_fail_closed_engine #(.ENGINE_ID(16'd13), .ERRNO_VALUE(LNP64_ERR_ENOTSUP), .STATUS_VALUE(LNP64_STATUS_UNSUPPORTED)) shell(.*,.fault_valid(unused_fault_valid),.fault_ready(1'b1),.fault(unused_fault),.accepted_counter(telemetry_counter),.fault_counter(fault_counter));
endmodule

module lnp64_process_engine(input logic clk, input logic reset_n, input logic cmd_valid, output logic cmd_ready, input lnp64_cmd_t cmd, output logic rsp_valid, input logic rsp_ready, output lnp64_rsp_t rsp, output logic [31:0] telemetry_counter, output logic [31:0] fault_counter);
    logic unused_fault_valid; lnp64_fault_t unused_fault;
    lnp64_fail_closed_engine #(.ENGINE_ID(16'd14), .ERRNO_VALUE(LNP64_ERR_ENOTSUP), .STATUS_VALUE(LNP64_STATUS_UNSUPPORTED)) shell(.*,.fault_valid(unused_fault_valid),.fault_ready(1'b1),.fault(unused_fault),.accepted_counter(telemetry_counter),.fault_counter(fault_counter));
endmodule

module lnp64_vma_engine(input logic clk, input logic reset_n, input logic cmd_valid, output logic cmd_ready, input lnp64_cmd_t cmd, output logic rsp_valid, input logic rsp_ready, output lnp64_rsp_t rsp, output logic [31:0] telemetry_counter, output logic [31:0] fault_counter);
    logic unused_fault_valid; lnp64_fault_t unused_fault;
    lnp64_fail_closed_engine #(.ENGINE_ID(16'd15), .ERRNO_VALUE(LNP64_ERR_ENOTSUP), .STATUS_VALUE(LNP64_STATUS_UNSUPPORTED)) shell(.*,.fault_valid(unused_fault_valid),.fault_ready(1'b1),.fault(unused_fault),.accepted_counter(telemetry_counter),.fault_counter(fault_counter));
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

module lnp64_dma_fabric(input logic clk, input logic reset_n, output logic visibility_event_path_live, output logic raw_dma_authority_visible, output logic [31:0] telemetry_counter, output logic [31:0] fault_counter);
    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin visibility_event_path_live <= 1'b0; raw_dma_authority_visible <= 1'b0; telemetry_counter <= 32'd0; fault_counter <= 32'd0; end
        else begin visibility_event_path_live <= 1'b1; raw_dma_authority_visible <= 1'b0; telemetry_counter <= 32'd1; end
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

module lnp64_heap_engine(input logic clk, input logic reset_n, input logic cmd_valid, output logic cmd_ready, input lnp64_cmd_t cmd, output logic rsp_valid, input logic rsp_ready, output lnp64_rsp_t rsp, output logic [31:0] telemetry_counter, output logic [31:0] fault_counter);
    logic unused_fault_valid; lnp64_fault_t unused_fault;
    lnp64_fail_closed_engine #(.ENGINE_ID(16'd17), .ERRNO_VALUE(LNP64_ERR_ENOTSUP), .STATUS_VALUE(LNP64_STATUS_UNSUPPORTED)) shell(.*,.fault_valid(unused_fault_valid),.fault_ready(1'b1),.fault(unused_fault),.accepted_counter(telemetry_counter),.fault_counter(fault_counter));
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

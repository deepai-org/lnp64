`timescale 1ns/1ps

// "Does what it says" formal harness for the lnp64_fail_closed_engine shell.
//
// The fail-closed engine is the shell LNP64 routes unsupported/denied commands
// to. Its contract: every response it emits carries the configured errno/status
// and NEVER a success or a non-zero result value -- it cannot grant authority.
// Here cmd/rsp_ready/fault_ready are free model-checker inputs, so SymbiYosys
// proves the contract holds for every command stream, on the real RTL.
import lnp64_pkg::*;

module lnp64_fail_closed_formal (
    input logic clk,
    input logic reset_n,
    input logic cmd_valid,
    input lnp64_cmd_t cmd,
    input logic rsp_ready,
    input logic fault_ready
);
    localparam logic [15:0] ERRNO_VALUE = LNP64_ERR_ENOTSUP;
    localparam logic [15:0] STATUS_VALUE = LNP64_STATUS_UNSUPPORTED;

    logic cmd_ready;
    logic rsp_valid;
    lnp64_rsp_t rsp;
    logic fault_valid;
    lnp64_fault_t fault;
    logic [31:0] accepted_counter;
    logic [31:0] fault_counter;

    lnp64_fail_closed_engine #(
        .ENGINE_ID(16'd1),
        .ERRNO_VALUE(ERRNO_VALUE),
        .STATUS_VALUE(STATUS_VALUE)
    ) dut (
        .clk(clk),
        .reset_n(reset_n),
        .cmd_valid(cmd_valid),
        .cmd_ready(cmd_ready),
        .cmd(cmd),
        .rsp_valid(rsp_valid),
        .rsp_ready(rsp_ready),
        .rsp(rsp),
        .fault_valid(fault_valid),
        .fault_ready(fault_ready),
        .fault(fault),
        .accepted_counter(accepted_counter),
        .fault_counter(fault_counter)
    );

    // Power-on reset discipline.
    reg init_done = 1'b0;
    always @(posedge clk) init_done <= 1'b1;
    always @(posedge clk) begin
        if (!init_done) assume (!reset_n);
    end

    always @(posedge clk) begin
        if (reset_n && rsp_valid) begin
            // The fail-closed engine never returns a success: errno and status
            // are exactly the configured fail-closed values.
            a_errno_is_failclosed:
                assert (rsp.errno_value == ERRNO_VALUE);
            a_status_is_failclosed:
                assert (rsp.status == STATUS_VALUE);

            // It never returns a non-zero result value -- no authority/data is
            // handed back through this shell.
            a_no_result_value:
                assert (rsp.result_value == 64'd0);
        end
    end
endmodule

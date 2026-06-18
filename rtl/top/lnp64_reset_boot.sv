`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_clock_reset (
    input  logic clk,
    input  logic reset_n,
    output logic logic_reset_n
);
    logic reset_sync;

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            reset_sync <= 1'b0;
            logic_reset_n <= 1'b0;
        end else begin
            reset_sync <= 1'b1;
            logic_reset_n <= reset_sync;
        end
    end
endmodule

module lnp64_reset_boot (
    input  logic clk,
    input  logic reset_n,
    input  logic force_boot_fault,
    output logic boot_valid,
    output logic release_core,
    output logic boot_fault_valid,
    output lnp64_fault_t boot_fault,
    output lnp64_domain_t root_domain,
    output lnp64_cap_t root_fdr,
    output logic [31:0] pid1,
    output logic [31:0] tid1
);
    typedef enum logic [2:0] {
        BOOT_RESET,
        BOOT_FEATURES,
        BOOT_ROOT_DOMAIN,
        BOOT_PID1,
        BOOT_SCHEDULER,
        BOOT_RELEASE,
        BOOT_FAULT
    } boot_state_e;

    boot_state_e state;

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            state <= BOOT_RESET;
            boot_valid <= 1'b0;
            release_core <= 1'b0;
            boot_fault_valid <= 1'b0;
            boot_fault <= '0;
            root_domain <= '0;
            root_fdr <= '0;
            pid1 <= 32'd0;
            tid1 <= 32'd0;
        end else begin
            boot_valid <= 1'b0;
            release_core <= 1'b0;
            unique case (state)
                BOOT_RESET: begin
                    state <= force_boot_fault ? BOOT_FAULT : BOOT_FEATURES;
                end
                BOOT_FEATURES: begin
                    state <= BOOT_ROOT_DOMAIN;
                end
                BOOT_ROOT_DOMAIN: begin
                    root_domain.domain_id <= 32'd1;
                    root_domain.domain_gen <= 32'd1;
                    root_domain.parent_domain_id <= 32'd0;
                    root_domain.parent_domain_gen <= 32'd0;
                    root_domain.budget_limit <= 64'd1024;
                    root_domain.budget_used <= 64'd0;
                    root_domain.lifecycle_state <= 16'd1;
                    root_domain.assurance_profile <= 16'd0;
                    root_domain.label_id <= 32'd0;
                    root_fdr.object_id <= 32'd1;
                    root_fdr.object_gen <= 32'd1;
                    root_fdr.fdr_gen <= 32'd1;
                    root_fdr.domain_id <= 32'd1;
                    root_fdr.domain_gen <= 32'd1;
                    root_fdr.rights_mask <= 64'h0000_0000_0000_ffff;
                    root_fdr.lineage_epoch <= 32'd1;
                    root_fdr.sealed <= 1'b0;
                    root_fdr.narrowable <= 1'b1;
                    state <= BOOT_PID1;
                end
                BOOT_PID1: begin
                    pid1 <= 32'd1;
                    tid1 <= 32'd1;
                    state <= BOOT_SCHEDULER;
                end
                BOOT_SCHEDULER: begin
                    boot_valid <= 1'b1;
                    state <= BOOT_RELEASE;
                end
                BOOT_RELEASE: begin
                    release_core <= 1'b1;
                end
                BOOT_FAULT: begin
                    boot_fault_valid <= 1'b1;
                    boot_fault.fault_id <= 32'hb007_f001;
                    boot_fault.op_id <= 32'd0;
                    boot_fault.pid <= 32'd0;
                    boot_fault.tid <= 32'd0;
                    boot_fault.domain_id <= 32'd0;
                    boot_fault.domain_gen <= 32'd0;
                    boot_fault.fault_code <= LNP64_ERR_EFAULT;
                    boot_fault.source <= 16'hb007;
                    boot_fault.detail <= 64'hb007_fa11;
                end
                default: state <= BOOT_RESET;
            endcase
        end
    end
endmodule

`timescale 1ns/1ps

module lnp64_m12_assertions (
    input logic clk,
    input logic reset_n,
    input logic done,
    input logic boot_image_visible,
    input logic block_object_authorized,
    input logic block_write_completed,
    input logic storage_barrier_issued,
    input logic storage_barrier_quiescent,
    input logic stale_object_rejected,
    input logic cross_domain_rejected,
    input logic media_fault_terminal,
    input logic no_raw_device_authority,
    input logic counts_exact
);
    always_ff @(posedge clk) begin
        if (reset_n && done) begin
            assert (boot_image_visible);
            assert (block_object_authorized);
            assert (block_write_completed);
            assert (storage_barrier_issued);
            assert (storage_barrier_quiescent);
            assert (stale_object_rejected);
            assert (cross_domain_rejected);
            assert (media_fault_terminal);
            assert (no_raw_device_authority);
            assert (counts_exact);
        end
    end
endmodule

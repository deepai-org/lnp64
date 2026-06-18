`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_s0_fpga_top (
    input  logic clk,
    input  logic reset_n,
    output logic uart_tx,
    output logic [5:0] status_led
);
    logic [7:0] bringup_counter;
    logic event_inject;
    logic fault_inject;
    logic watchdog_inject;

    logic boot_stable;
    logic pid1_exactly_one_location;
    logic pid1_completed;
    logic [31:0] retired_count;
    logic env_get_ok;
    logic sram_ldst_ok;
    logic unsupported_failed_closed;
    logic stub_failed_closed;
    logic uart_seen;
    logic [7:0] uart_byte_seen;
    logic event_woke_thread;
    logic structured_fault_seen;
    logic watchdog_degraded_seen;
    logic no_raw_authority_visible;
    logic coherence_paths_live;
    logic multicore_no_duplicate_tid;
    logic tile_reset_stable_all;
    logic tile1_observable_idle;
    logic cross_tile_wake_one;
    logic tile_fault_isolated;
    logic [31:0] topology_tile_count_seen;
    logic [63:0] topology_enabled_tile_mask_seen;
    logic [31:0] topology_coherence_domain_seen;
    logic [31:0] topology_active_window_base_seen;
    logic [31:0] topology_active_window_count_seen;
    logic uart_seen_q;
    logic uart_send;

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            bringup_counter <= 8'd0;
            uart_seen_q <= 1'b0;
        end else if (bringup_counter != 8'hff) begin
            bringup_counter <= bringup_counter + 8'd1;
            uart_seen_q <= uart_seen;
        end else begin
            uart_seen_q <= uart_seen;
        end
    end

    assign event_inject = bringup_counter >= 8'd80 && bringup_counter < 8'd104;
    assign fault_inject = bringup_counter == 8'd112;
    assign watchdog_inject = bringup_counter >= 8'd144 && bringup_counter < 8'd152;
    assign uart_send = uart_seen && !uart_seen_q;

    lnp64_top dut(
        .clk(clk),
        .reset_n(reset_n),
        .force_boot_fault(1'b0),
        .sim_event_inject(event_inject),
        .sim_fault_inject(fault_inject),
        .sim_watchdog_inject(watchdog_inject),
        .boot_stable(boot_stable),
        .pid1_exactly_one_location(pid1_exactly_one_location),
        .pid1_completed(pid1_completed),
        .retired_count(retired_count),
        .env_get_ok(env_get_ok),
        .sram_ldst_ok(sram_ldst_ok),
        .unsupported_failed_closed(unsupported_failed_closed),
        .stub_failed_closed(stub_failed_closed),
        .uart_seen(uart_seen),
        .uart_byte_seen(uart_byte_seen),
        .event_woke_thread(event_woke_thread),
        .structured_fault_seen(structured_fault_seen),
        .watchdog_degraded_seen(watchdog_degraded_seen),
        .no_raw_authority_visible(no_raw_authority_visible),
        .coherence_paths_live(coherence_paths_live),
        .multicore_no_duplicate_tid(multicore_no_duplicate_tid),
        .tile_reset_stable_all(tile_reset_stable_all),
        .tile1_observable_idle(tile1_observable_idle),
        .cross_tile_wake_one(cross_tile_wake_one),
        .tile_fault_isolated(tile_fault_isolated),
        .topology_tile_count_seen(topology_tile_count_seen),
        .topology_enabled_tile_mask_seen(topology_enabled_tile_mask_seen),
        .topology_coherence_domain_seen(topology_coherence_domain_seen),
        .topology_active_window_base_seen(topology_active_window_base_seen),
        .topology_active_window_count_seen(topology_active_window_count_seen)
    );

    lnp64_s0_uart_tx uart_tx_i(
        .clk(clk),
        .reset_n(reset_n),
        .send(uart_send),
        .data(uart_byte_seen),
        .tx(uart_tx),
        .busy()
    );

    assign status_led = {
        watchdog_degraded_seen,
        structured_fault_seen,
        event_woke_thread,
        env_get_ok && sram_ldst_ok,
        pid1_completed && pid1_exactly_one_location,
        boot_stable && no_raw_authority_visible && coherence_paths_live
    };
endmodule

module lnp64_s0_uart_tx #(
    parameter int CLK_HZ = 12000000,
    parameter int BAUD = 115200
) (
    input  logic clk,
    input  logic reset_n,
    input  logic send,
    input  logic [7:0] data,
    output logic tx,
    output logic busy
);
    localparam int BAUD_DIV_RAW = CLK_HZ / BAUD;
    localparam int BAUD_DIV = (BAUD_DIV_RAW < 1) ? 1 : BAUD_DIV_RAW;
    localparam logic [15:0] BAUD_DIV_COUNT = BAUD_DIV;

    logic [15:0] baud_count;
    logic [9:0] shifter;
    logic [3:0] bit_count;

    always_ff @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            tx <= 1'b1;
            busy <= 1'b0;
            baud_count <= 16'd0;
            shifter <= 10'h3ff;
            bit_count <= 4'd0;
        end else if (!busy) begin
            tx <= 1'b1;
            if (send) begin
                busy <= 1'b1;
                shifter <= {1'b1, data, 1'b0};
                bit_count <= 4'd10;
                baud_count <= BAUD_DIV_COUNT - 16'd1;
                tx <= 1'b0;
            end
        end else if (baud_count != 16'd0) begin
            baud_count <= baud_count - 16'd1;
        end else begin
            shifter <= {1'b1, shifter[9:1]};
            bit_count <= bit_count - 4'd1;
            baud_count <= BAUD_DIV_COUNT - 16'd1;
            if (bit_count == 4'd1) begin
                busy <= 1'b0;
                tx <= 1'b1;
            end else begin
                tx <= shifter[1];
            end
        end
    end
endmodule

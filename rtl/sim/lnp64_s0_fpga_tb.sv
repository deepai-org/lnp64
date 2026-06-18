`timescale 1ns/1ps

module lnp64_s0_fpga_tb;
    localparam int UART_BIT_CYCLES = 104;

    logic clk;
    logic reset_n;
    logic uart_tx;
    logic [5:0] status_led;
    logic [7:0] uart_byte;
    logic stop_bit;
    int i;

    lnp64_s0_fpga_top dut(
        .clk(clk),
        .reset_n(reset_n),
        .uart_tx(uart_tx),
        .status_led(status_led)
    );

    always #5 clk = ~clk;

    task automatic require(input logic condition, input string message);
        if (!condition) begin
            $fatal(1, "%s", message);
        end
    endtask

    initial begin
        clk = 1'b0;
        reset_n = 1'b0;
        uart_byte = 8'd0;
        stop_bit = 1'b0;

        repeat (8) @(posedge clk);
        reset_n = 1'b1;

        wait (uart_tx === 1'b0);
        repeat (UART_BIT_CYCLES / 2) @(posedge clk);
        require(uart_tx === 1'b0, "S0 FPGA UART start bit was not stable");

        repeat (UART_BIT_CYCLES) @(posedge clk);
        for (i = 0; i < 8; i = i + 1) begin
            uart_byte[i] = uart_tx;
            repeat (UART_BIT_CYCLES) @(posedge clk);
        end
        stop_bit = uart_tx;

        require(stop_bit === 1'b1, "S0 FPGA UART stop bit was not high");
        require(uart_byte == 8'h53, "S0 FPGA UART boot/status byte was not 0x53");

        repeat (256) @(posedge clk);
        require(status_led == 6'b111111, "S0 FPGA status LEDs did not reach all bring-up predicates");

        $display("LNP64-RTL-S0-FPGA PASS uart=0x%02h leds=0b%06b", uart_byte, status_led);
        $finish;
    end
endmodule

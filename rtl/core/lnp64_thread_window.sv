`timescale 1ns/1ps

module lnp64_thread_window (
    input  logic active_slot,
    input  logic [1:0] context_ready,
    output logic next_slot,
    output logic [31:0] active_tid
);
    always_comb begin
        active_tid = {31'd0, active_slot} + 32'd1;
        if (active_slot == 1'b0 && context_ready[1]) begin
            next_slot = 1'b1;
        end else if (active_slot == 1'b1 && context_ready[0]) begin
            next_slot = 1'b0;
        end else begin
            next_slot = active_slot;
        end
    end
endmodule

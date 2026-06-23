`timescale 1ns/1ps

// M16 endpoint invariant assertions (EP-F): bounded latency, fail-closed,
// cap-safety, and framing — checked at slice completion, mirroring M15.
module lnp64_m16_assertions (
    input logic clk,
    input logic reset_n,
    input logic done,
    input logic bounded_depth_le_capacity,
    input logic drain_bounded_by_capacity,
    input logic full_fails_closed,
    input logic empty_fails_closed,
    input logic oversize_fails_closed,
    input logic no_block_except_wait,
    input logic caps_resolve_sender_only,
    input logic caps_reject_out_of_range,
    input logic install_no_amplify,
    input logic framing_one_send_one_recv,
    input logic notify_raises_register_edge,
    input logic counts_exact
);
    always_ff @(posedge clk) begin
        if (reset_n && done) begin
            assert (bounded_depth_le_capacity)
                else $fatal(1, "M16 endpoint depth exceeded capacity");
            assert (drain_bounded_by_capacity)
                else $fatal(1, "M16 drain was not bounded by capacity (WCET)");
            assert (full_fails_closed)
                else $fatal(1, "M16 send on full did not fail closed (EAGAIN)");
            assert (empty_fails_closed)
                else $fatal(1, "M16 recv on empty did not fail closed (EAGAIN)");
            assert (oversize_fails_closed)
                else $fatal(1, "M16 oversize send did not fail closed (EMSGSIZE)");
            assert (no_block_except_wait)
                else $fatal(1, "M16 a non-wait op blocked");
            assert (caps_resolve_sender_only)
                else $fatal(1, "M16 cap did not resolve against the sender table");
            assert (caps_reject_out_of_range)
                else $fatal(1, "M16 out-of-range cap handle was not rejected");
            assert (install_no_amplify)
                else $fatal(1, "M16 cap install amplified rights");
            assert (framing_one_send_one_recv)
                else $fatal(1, "M16 framing (one send = one message = one recv) broke");
            assert (notify_raises_register_edge)
                else $fatal(1, "M16 notify did not raise the register edge");
            assert (counts_exact)
                else $fatal(1, "M16 failure/event counts were not exact");
        end
    end
endmodule

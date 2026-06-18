`timescale 1ns/1ps

module lnp64_m2_assertions (
    input logic clk,
    input logic reset_n,
    input logic done,
    input logic continuation_unique,
    input logic sync_roundtrip_ok,
    input logic async_delivery_ok,
    input logic handoff_delivery_ok,
    input logic stale_continuation_rejected,
    input logic fault_delivery_gate_ok,
    input logic signal_compatibility_ok
);
    always_ff @(posedge clk) begin
        if (reset_n && done) begin
            assert (continuation_unique)
                else $fatal(1, "M2 continuation uniqueness invariant failed");
            assert (sync_roundtrip_ok)
                else $fatal(1, "M2 sync gate roundtrip failed");
            assert (async_delivery_ok)
                else $fatal(1, "M2 async gate delivery failed");
            assert (handoff_delivery_ok)
                else $fatal(1, "M2 handoff gate delivery failed");
            assert (stale_continuation_rejected)
                else $fatal(1, "M2 stale continuation was not rejected");
            assert (fault_delivery_gate_ok)
                else $fatal(1, "M2 fault delivery gate failed");
            assert (signal_compatibility_ok)
                else $fatal(1, "M2 signal compatibility path failed");
        end
    end
endmodule

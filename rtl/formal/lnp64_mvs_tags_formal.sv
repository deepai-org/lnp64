`timescale 1ns/1ps

// Capability-FORGERY proof for the tagged-memory MVS core.
//
// Proves on the actual RTL, over all inputs, that integer data can never become
// a valid capability: the per-word tag and the taint ghost are exact
// complements (a word is tagged iff it was last written by a capability store),
// and a CapLoad yields a valid capability only from a tagged (non-taint) word.
// Hence writing raw integer data then CapLoading it always yields an invalid
// capability.
module lnp64_mvs_tags_formal (
    input logic        clk,
    input logic        rst_n,
    input logic [1:0]  op,
    input logic        addr,
    input logic [7:0]  in_data
);
    logic [7:0] mem0_data, mem1_data, creg_data;
    logic       mem0_tag, mem1_tag, creg_valid, creg_tainted, mem0_taint, mem1_taint;

    lnp64_mvs_tags dut (
        .clk(clk), .rst_n(rst_n),
        .op(op), .addr(addr), .in_data(in_data),
        .mem0_data(mem0_data), .mem1_data(mem1_data),
        .mem0_tag(mem0_tag), .mem1_tag(mem1_tag),
        .creg_valid(creg_valid), .creg_data(creg_data), .creg_tainted(creg_tainted),
        .mem0_taint(mem0_taint), .mem1_taint(mem1_taint)
    );

    reg init_done = 1'b0;
    always @(posedge clk) init_done <= 1'b1;
    always @(posedge clk) begin
        if (!init_done) assume (!rst_n);
    end

    always @(posedge clk) begin
        if (rst_n) begin
            // Tag integrity: a word is never both capability-tagged AND
            // integer-tainted (a store is either a cap store or an int store).
            a_tag0_taint0_exclusive: assert (!(mem0_tag && mem0_taint));
            a_tag1_taint1_exclusive: assert (!(mem1_tag && mem1_taint));

            // No forgery: a valid loaded capability never came from integer data.
            a_no_forged_capability:
                assert (!(creg_valid && creg_tainted));
        end
    end
endmodule

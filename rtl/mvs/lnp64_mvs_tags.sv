`timescale 1ns/1ps

// LNP64 MVS tagged-memory core (closes the capability-forgery gap).
//
// Capabilities are memory-resident, not just register-held. Each memory word
// carries a 1-bit hardware tag. The rules that make capabilities unforgeable:
//   * an integer store writes data and CLEARS the word's tag (data is not a cap);
//   * a capability store writes a capability and SETS the word's tag;
//   * a CapLoad yields a VALID capability register only if the source word's tag
//     is set -- otherwise it yields an invalid (null) capability.
// Therefore an attacker who writes raw integer data to RAM and then CapLoads it
// gets an invalid capability: integer data can never become authority.
//
// Two words; op selects the action (free adversarial inputs).
module lnp64_mvs_tags (
    input  logic        clk,
    input  logic        rst_n,

    input  logic [1:0]  op,        // 0=nop, 1=int_store, 2=cap_store, 3=cap_load
    input  logic        addr,      // word 0 or 1
    input  logic [7:0]  in_data,

    output logic [7:0]  mem0_data,
    output logic [7:0]  mem1_data,
    output logic        mem0_tag,
    output logic        mem1_tag,
    output logic        creg_valid,   // loaded capability register valid bit
    output logic [7:0]  creg_data,
    output logic        creg_tainted, // ghost: loaded from integer-written data?
    output logic        mem0_taint,   // ghost: word 0 currently holds integer data
    output logic        mem1_taint
);
    localparam logic [1:0] OP_NOP   = 2'd0;
    localparam logic [1:0] OP_ISTORE = 2'd1;
    localparam logic [1:0] OP_CSTORE = 2'd2;
    localparam logic [1:0] OP_CLOAD  = 2'd3;

    logic [7:0] data0, data1;
    logic       tag0, tag1;
    // Ghost taint bit: 1 if the word's current data came from an integer store.
    logic       taint0, taint1;

    assign mem0_data = data0;
    assign mem1_data = data1;
    assign mem0_tag  = tag0;
    assign mem1_tag  = tag1;
    assign mem0_taint = taint0;
    assign mem1_taint = taint1;

    logic sel_tag, sel_taint;
    logic [7:0] sel_data;
    assign sel_tag   = addr ? tag1   : tag0;
    assign sel_taint = addr ? taint1 : taint0;
    assign sel_data  = addr ? data1  : data0;

    logic cvalid;
    logic [7:0] cdata;
    logic ctaint;
    assign creg_valid   = cvalid;
    assign creg_data    = cdata;
    assign creg_tainted = ctaint;

    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            data0 <= 8'd0; data1 <= 8'd0;
            tag0  <= 1'b0; tag1  <= 1'b0;
            taint0 <= 1'b0; taint1 <= 1'b0;
            cvalid <= 1'b0; cdata <= 8'd0; ctaint <= 1'b0;
        end else begin
            unique case (op)
                OP_ISTORE: begin
                    if (addr) begin data1 <= in_data; tag1 <= 1'b0; taint1 <= 1'b1; end
                    else      begin data0 <= in_data; tag0 <= 1'b0; taint0 <= 1'b1; end
                end
                OP_CSTORE: begin
                    if (addr) begin data1 <= in_data; tag1 <= 1'b1; taint1 <= 1'b0; end
                    else      begin data0 <= in_data; tag0 <= 1'b1; taint0 <= 1'b0; end
                end
                OP_CLOAD: begin
                    // A loaded capability is valid only if the source word is tagged.
                    cvalid <= sel_tag;
                    cdata  <= sel_data;
                    ctaint <= sel_taint;
                end
                default: ; // nop
            endcase
        end
    end
endmodule

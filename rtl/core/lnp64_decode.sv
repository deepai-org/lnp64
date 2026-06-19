`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_decode (
    input  logic [31:0] instr,
    output lnp64_decode_t dec
);
    logic [7:0] raw_opcode;

    always_comb begin
        raw_opcode = instr[31:24];
        dec.opcode = {8'd0, raw_opcode};
        dec.profile = 16'd0;
        dec.rd = {3'd0, instr[23:19]};
        dec.rs1 = {3'd0, instr[18:14]};
        dec.rs2 = {3'd0, instr[13:9]};
        dec.imm = {{18{instr[13]}}, instr[13:0]};

        unique case (raw_opcode)
            8'h00: begin
                dec.opcode = LNP64_OP_NOP;
            end
            8'h01: begin
                dec.opcode = LNP64_OP_LI32;
                dec.imm = {{16{instr[15]}}, instr[15:0]};
            end
            8'h06: begin
                dec.opcode = LNP64_OP_YIELD;
            end
            8'h10: begin
                dec.opcode = LNP64_OP_ADD;
            end
            8'h20: begin
                dec.opcode = LNP64_OP_JMP;
                dec.imm = {{8{instr[23]}}, instr[23:0]};
            end
            8'h30: begin
                dec.opcode = LNP64_OP_LD;
            end
            8'h33: begin
                dec.opcode = LNP64_OP_ST;
            end
            8'h38: begin
                dec.opcode = LNP64_OP_GET_ERRNO;
            end
            8'h39: begin
                dec.opcode = LNP64_OP_SET_ERRNO;
            end
            8'h3a: begin
                dec.opcode = LNP64_OP_EXIT;
            end
            8'h4b: begin
                dec.opcode = LNP64_OP_OBJECT_CTL;
            end
            8'h56: begin
                dec.opcode = LNP64_OP_ENV_GET;
            end
            8'hff: begin
                dec.opcode = LNP64_OP_UNSUPPORTED;
            end
            default: begin
                dec.opcode = {8'd0, raw_opcode};
            end
        endcase

        dec.supported =
            dec.opcode == LNP64_OP_NOP ||
            dec.opcode == LNP64_OP_LI32 ||
            dec.opcode == LNP64_OP_ADD ||
            dec.opcode == LNP64_OP_JMP ||
            dec.opcode == LNP64_OP_LD ||
            dec.opcode == LNP64_OP_ST ||
            dec.opcode == LNP64_OP_YIELD ||
            dec.opcode == LNP64_OP_ENV_GET ||
            dec.opcode == LNP64_OP_GET_ERRNO ||
            dec.opcode == LNP64_OP_SET_ERRNO ||
            dec.opcode == LNP64_OP_EXIT ||
            dec.opcode == LNP64_OP_OBJECT_CTL ||
            dec.opcode == LNP64_OP_FAULT_INJECT;
    end
endmodule

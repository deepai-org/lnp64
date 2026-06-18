`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_decode (
    input  logic [31:0] instr,
    output lnp64_decode_t dec
);
    always_comb begin
        dec.opcode = {8'd0, instr[31:24]};
        dec.profile = 16'd0;
        dec.rd = instr[23:16];
        dec.rs1 = instr[15:8];
        dec.rs2 = instr[7:0];
        dec.imm = {{24{instr[7]}}, instr[7:0]};
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
            dec.opcode == LNP64_OP_OBJECT_CTL ||
            dec.opcode == LNP64_OP_FAULT_INJECT;
    end
endmodule

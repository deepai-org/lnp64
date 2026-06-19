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
        dec.rs3 = {3'd0, instr[8:4]};
        dec.imm = {{18{instr[13]}}, instr[13:0]};

        unique case (raw_opcode)
            8'h00: begin
                dec.opcode = LNP64_OP_NOP;
            end
            8'h01: begin
                dec.opcode = LNP64_OP_LI32;
                dec.imm = {{16{instr[15]}}, instr[15:0]};
            end
            8'h04: begin
                dec.opcode = LNP64_OP_LI32_LITERAL;
            end
            8'h02: begin
                dec.opcode = LNP64_OP_MOV;
            end
            8'h03: begin
                dec.opcode = LNP64_OP_LA_LITERAL;
            end
            8'h06: begin
                dec.opcode = LNP64_OP_YIELD;
            end
            8'h10: begin
                dec.opcode = LNP64_OP_ADD;
            end
            8'h11: begin
                dec.opcode = LNP64_OP_SUB;
            end
            8'h12: begin
                dec.opcode = LNP64_OP_MUL;
            end
            8'h13: begin
                dec.opcode = LNP64_OP_DIV;
            end
            8'h14: begin
                dec.opcode = LNP64_OP_AND;
            end
            8'h15: begin
                dec.opcode = LNP64_OP_OR;
            end
            8'h16: begin
                dec.opcode = LNP64_OP_XOR;
            end
            8'h17: begin
                dec.opcode = LNP64_OP_NOT;
            end
            8'h18: begin
                dec.opcode = LNP64_OP_LSL;
            end
            8'h19: begin
                dec.opcode = LNP64_OP_LSR;
            end
            8'h1a: begin
                dec.opcode = LNP64_OP_ASR;
            end
            8'h1b: begin
                dec.opcode = LNP64_OP_CMP;
            end
            8'h1c: begin
                dec.opcode = LNP64_OP_CMPU;
            end
            8'h1f: begin
                dec.opcode = LNP64_OP_RET;
            end
            8'h20: begin
                dec.opcode = LNP64_OP_JMP;
                dec.imm = {{8{instr[23]}}, instr[23:0]};
            end
            8'h21: begin
                dec.opcode = LNP64_OP_BRANCH_EQ;
                dec.imm = {{8{instr[23]}}, instr[23:0]};
            end
            8'h22: begin
                dec.opcode = LNP64_OP_BRANCH_NE;
                dec.imm = {{8{instr[23]}}, instr[23:0]};
            end
            8'h23: begin
                dec.opcode = LNP64_OP_BRANCH_LT;
                dec.imm = {{8{instr[23]}}, instr[23:0]};
            end
            8'h24: begin
                dec.opcode = LNP64_OP_BRANCH_GT;
                dec.imm = {{8{instr[23]}}, instr[23:0]};
            end
            8'h25: begin
                dec.opcode = LNP64_OP_BRANCH_LE;
                dec.imm = {{8{instr[23]}}, instr[23:0]};
            end
            8'h26: begin
                dec.opcode = LNP64_OP_BRANCH_GE;
                dec.imm = {{8{instr[23]}}, instr[23:0]};
            end
            8'h27: begin
                dec.opcode = LNP64_OP_CALL;
                dec.imm = {{8{instr[23]}}, instr[23:0]};
            end
            8'h28: begin
                dec.opcode = LNP64_OP_CALL_REG;
            end
            8'h29: begin
                dec.opcode = LNP64_OP_LR_GET;
            end
            8'h2a: begin
                dec.opcode = LNP64_OP_LR_SET;
            end
            8'h2b: begin
                dec.opcode = LNP64_OP_PULL;
            end
            8'h2c: begin
                dec.opcode = LNP64_OP_PUSH;
            end
            8'h30: begin
                dec.opcode = LNP64_OP_LD;
            end
            8'h31: begin
                dec.opcode = LNP64_OP_LD_W;
            end
            8'h32: begin
                dec.opcode = LNP64_OP_LD_B;
            end
            8'h33: begin
                dec.opcode = LNP64_OP_ST;
            end
            8'h34: begin
                dec.opcode = LNP64_OP_ST_W;
            end
            8'h35: begin
                dec.opcode = LNP64_OP_ST_B;
            end
            8'h36: begin
                dec.opcode = LNP64_OP_LD_H;
            end
            8'h37: begin
                dec.opcode = LNP64_OP_ST_H;
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
            8'h3b: begin
                dec.opcode = LNP64_OP_PULL;
            end
            8'h3c: begin
                dec.opcode = LNP64_OP_PUSH;
            end
            8'h3d: begin
                dec.opcode = LNP64_OP_CSET_EQ;
            end
            8'h3e: begin
                dec.opcode = LNP64_OP_CSET_NE;
            end
            8'h3f: begin
                dec.opcode = LNP64_OP_CSET_LT;
            end
            8'h40: begin
                dec.opcode = LNP64_OP_CSET_GT;
            end
            8'h41: begin
                dec.opcode = LNP64_OP_CSET_LE;
            end
            8'h42: begin
                dec.opcode = LNP64_OP_CSET_GE;
            end
            8'h43: begin
                dec.opcode = LNP64_OP_CSET_ULT;
            end
            8'h44: begin
                dec.opcode = LNP64_OP_CSET_UGT;
            end
            8'h45: begin
                dec.opcode = LNP64_OP_CSET_ULE;
            end
            8'h46: begin
                dec.opcode = LNP64_OP_CSET_UGE;
            end
            8'h4b: begin
                dec.opcode = LNP64_OP_OBJECT_CTL;
            end
            8'h50: begin
                dec.opcode = LNP64_OP_CAP_DUP;
            end
            8'h51: begin
                dec.opcode = LNP64_OP_CAP_SEND;
            end
            8'h52: begin
                dec.opcode = LNP64_OP_CAP_RECV;
            end
            8'h53: begin
                dec.opcode = LNP64_OP_CAP_REVOKE;
            end
            8'h56: begin
                dec.opcode = LNP64_OP_ENV_GET;
            end
            8'h57: begin
                dec.opcode = LNP64_OP_WRITE_FD;
            end
            8'h5b: begin
                dec.opcode = LNP64_OP_DMA_CTL;
            end
            8'h47: begin
                dec.opcode = LNP64_OP_ALLOC;
            end
            8'h48: begin
                dec.opcode = LNP64_OP_ALLOC_SIZE;
            end
            8'h49: begin
                dec.opcode = LNP64_OP_FREE;
            end
            8'h4a: begin
                dec.opcode = LNP64_OP_ALLOC_EX;
            end
            8'hc5: begin
                dec.opcode = LNP64_OP_AMO_SWAP;
            end
            8'hc6: begin
                dec.opcode = LNP64_OP_AMO_ADD;
            end
            8'hc7: begin
                dec.opcode = LNP64_OP_AMO_AND;
            end
            8'hc8: begin
                dec.opcode = LNP64_OP_AMO_OR;
            end
            8'hc9: begin
                dec.opcode = LNP64_OP_LOCK_CMPXCHG;
            end
            8'hca: begin
                dec.opcode = LNP64_OP_AMO_XOR;
            end
            8'hcd: begin
                dec.opcode = LNP64_OP_FENCE;
            end
            8'hce: begin
                dec.opcode = LNP64_OP_ISYNC;
            end
            8'hd0: begin
                dec.opcode = LNP64_OP_AUIPC_LITERAL;
            end
            8'ha7: begin
                dec.opcode = LNP64_OP_UDIV;
            end
            8'ha8: begin
                dec.opcode = LNP64_OP_SREM;
            end
            8'ha9: begin
                dec.opcode = LNP64_OP_UREM;
            end
            8'haa: begin
                dec.opcode = LNP64_OP_MULH;
            end
            8'hab: begin
                dec.opcode = LNP64_OP_MULHU;
            end
            8'hac: begin
                dec.opcode = LNP64_OP_MULHSU;
            end
            8'hff: begin
                dec.opcode = LNP64_OP_UNSUPPORTED;
            end
            8'ha0: begin
                dec.opcode = LNP64_OP_ADDI;
            end
            8'ha1: begin
                dec.opcode = LNP64_OP_ANDI;
            end
            8'ha2: begin
                dec.opcode = LNP64_OP_ORI;
            end
            8'ha3: begin
                dec.opcode = LNP64_OP_XORI;
            end
            8'ha4: begin
                dec.opcode = LNP64_OP_LSLI;
            end
            8'ha5: begin
                dec.opcode = LNP64_OP_LSRI;
            end
            8'ha6: begin
                dec.opcode = LNP64_OP_ASRI;
            end
            8'had: begin
                dec.opcode = LNP64_OP_SEXT_B;
            end
            8'hae: begin
                dec.opcode = LNP64_OP_SEXT_H;
            end
            8'haf: begin
                dec.opcode = LNP64_OP_SEXT_W;
            end
            8'hb0: begin
                dec.opcode = LNP64_OP_ZEXT_B;
            end
            8'hb1: begin
                dec.opcode = LNP64_OP_ZEXT_H;
            end
            8'hb2: begin
                dec.opcode = LNP64_OP_ZEXT_W;
            end
            8'hb3: begin
                dec.opcode = LNP64_OP_CLZ;
            end
            8'hb4: begin
                dec.opcode = LNP64_OP_CTZ;
            end
            8'hb5: begin
                dec.opcode = LNP64_OP_POPCNT;
            end
            8'hb6: begin
                dec.opcode = LNP64_OP_ROL;
            end
            8'hb7: begin
                dec.opcode = LNP64_OP_ROR;
            end
            8'hb8: begin
                dec.opcode = LNP64_OP_BSWAP16;
            end
            8'hb9: begin
                dec.opcode = LNP64_OP_BSWAP32;
            end
            8'hba: begin
                dec.opcode = LNP64_OP_BSWAP64;
            end
            8'hbb: begin
                dec.opcode = LNP64_OP_CSEL_EQ;
            end
            8'hbc: begin
                dec.opcode = LNP64_OP_CSEL_NE;
            end
            8'hbd: begin
                dec.opcode = LNP64_OP_CSEL_LT;
            end
            8'hbe: begin
                dec.opcode = LNP64_OP_CSEL_GT;
            end
            8'hbf: begin
                dec.opcode = LNP64_OP_CSEL_LE;
            end
            8'hc0: begin
                dec.opcode = LNP64_OP_CSEL_GE;
            end
            8'hc1: begin
                dec.opcode = LNP64_OP_CSEL_ULT;
            end
            8'hc2: begin
                dec.opcode = LNP64_OP_CSEL_UGT;
            end
            8'hc3: begin
                dec.opcode = LNP64_OP_CSEL_ULE;
            end
            8'hc4: begin
                dec.opcode = LNP64_OP_CSEL_UGE;
            end
            default: begin
                dec.opcode = {8'd0, raw_opcode};
            end
        endcase

        dec.supported =
            dec.opcode == LNP64_OP_NOP ||
            dec.opcode == LNP64_OP_LI32 ||
            dec.opcode == LNP64_OP_LI32_LITERAL ||
            dec.opcode == LNP64_OP_LA_LITERAL ||
            dec.opcode == LNP64_OP_MOV ||
            dec.opcode == LNP64_OP_ADD ||
            dec.opcode == LNP64_OP_SUB ||
            dec.opcode == LNP64_OP_MUL ||
            dec.opcode == LNP64_OP_DIV ||
            dec.opcode == LNP64_OP_AND ||
            dec.opcode == LNP64_OP_OR ||
            dec.opcode == LNP64_OP_XOR ||
            dec.opcode == LNP64_OP_NOT ||
            dec.opcode == LNP64_OP_LSL ||
            dec.opcode == LNP64_OP_LSR ||
            dec.opcode == LNP64_OP_ASR ||
            dec.opcode == LNP64_OP_ADDI ||
            dec.opcode == LNP64_OP_ANDI ||
            dec.opcode == LNP64_OP_ORI ||
            dec.opcode == LNP64_OP_XORI ||
            dec.opcode == LNP64_OP_LSLI ||
            dec.opcode == LNP64_OP_LSRI ||
            dec.opcode == LNP64_OP_ASRI ||
            dec.opcode == LNP64_OP_SEXT_B ||
            dec.opcode == LNP64_OP_SEXT_H ||
            dec.opcode == LNP64_OP_SEXT_W ||
            dec.opcode == LNP64_OP_ZEXT_B ||
            dec.opcode == LNP64_OP_ZEXT_H ||
            dec.opcode == LNP64_OP_ZEXT_W ||
            dec.opcode == LNP64_OP_CLZ ||
            dec.opcode == LNP64_OP_CTZ ||
            dec.opcode == LNP64_OP_POPCNT ||
            dec.opcode == LNP64_OP_ROL ||
            dec.opcode == LNP64_OP_ROR ||
            dec.opcode == LNP64_OP_BSWAP16 ||
            dec.opcode == LNP64_OP_BSWAP32 ||
            dec.opcode == LNP64_OP_BSWAP64 ||
            dec.opcode == LNP64_OP_CMP ||
            dec.opcode == LNP64_OP_CMPU ||
            dec.opcode == LNP64_OP_CSEL_EQ ||
            dec.opcode == LNP64_OP_CSEL_NE ||
            dec.opcode == LNP64_OP_CSEL_LT ||
            dec.opcode == LNP64_OP_CSEL_GT ||
            dec.opcode == LNP64_OP_CSEL_LE ||
            dec.opcode == LNP64_OP_CSEL_GE ||
            dec.opcode == LNP64_OP_CSEL_ULT ||
            dec.opcode == LNP64_OP_CSEL_UGT ||
            dec.opcode == LNP64_OP_CSEL_ULE ||
            dec.opcode == LNP64_OP_CSEL_UGE ||
            dec.opcode == LNP64_OP_CSET_EQ ||
            dec.opcode == LNP64_OP_CSET_NE ||
            dec.opcode == LNP64_OP_CSET_LT ||
            dec.opcode == LNP64_OP_CSET_GT ||
            dec.opcode == LNP64_OP_CSET_LE ||
            dec.opcode == LNP64_OP_CSET_GE ||
            dec.opcode == LNP64_OP_CSET_ULT ||
            dec.opcode == LNP64_OP_CSET_UGT ||
            dec.opcode == LNP64_OP_CSET_ULE ||
            dec.opcode == LNP64_OP_CSET_UGE ||
            dec.opcode == LNP64_OP_JMP ||
            dec.opcode == LNP64_OP_BRANCH_EQ ||
            dec.opcode == LNP64_OP_BRANCH_NE ||
            dec.opcode == LNP64_OP_BRANCH_LT ||
            dec.opcode == LNP64_OP_BRANCH_GT ||
            dec.opcode == LNP64_OP_BRANCH_LE ||
            dec.opcode == LNP64_OP_BRANCH_GE ||
            dec.opcode == LNP64_OP_CALL ||
            dec.opcode == LNP64_OP_CALL_REG ||
            dec.opcode == LNP64_OP_LR_GET ||
            dec.opcode == LNP64_OP_LR_SET ||
            dec.opcode == LNP64_OP_RET ||
            dec.opcode == LNP64_OP_AUIPC_LITERAL ||
            dec.opcode == LNP64_OP_FENCE ||
            dec.opcode == LNP64_OP_ISYNC ||
            dec.opcode == LNP64_OP_AMO_SWAP ||
            dec.opcode == LNP64_OP_AMO_ADD ||
            dec.opcode == LNP64_OP_AMO_AND ||
            dec.opcode == LNP64_OP_AMO_OR ||
            dec.opcode == LNP64_OP_AMO_XOR ||
            dec.opcode == LNP64_OP_LOCK_CMPXCHG ||
            dec.opcode == LNP64_OP_LD ||
            dec.opcode == LNP64_OP_LD_W ||
            dec.opcode == LNP64_OP_LD_H ||
            dec.opcode == LNP64_OP_LD_B ||
            dec.opcode == LNP64_OP_ST ||
            dec.opcode == LNP64_OP_ST_W ||
            dec.opcode == LNP64_OP_ST_H ||
            dec.opcode == LNP64_OP_ST_B ||
            dec.opcode == LNP64_OP_YIELD ||
            dec.opcode == LNP64_OP_DMA_CTL ||
            dec.opcode == LNP64_OP_ENV_GET ||
            dec.opcode == LNP64_OP_WRITE_FD ||
            dec.opcode == LNP64_OP_PUSH ||
            dec.opcode == LNP64_OP_PULL ||
            dec.opcode == LNP64_OP_ALLOC ||
            dec.opcode == LNP64_OP_ALLOC_SIZE ||
            dec.opcode == LNP64_OP_FREE ||
            dec.opcode == LNP64_OP_ALLOC_EX ||
            dec.opcode == LNP64_OP_GET_ERRNO ||
            dec.opcode == LNP64_OP_SET_ERRNO ||
            dec.opcode == LNP64_OP_UDIV ||
            dec.opcode == LNP64_OP_SREM ||
            dec.opcode == LNP64_OP_UREM ||
            dec.opcode == LNP64_OP_MULH ||
            dec.opcode == LNP64_OP_MULHU ||
            dec.opcode == LNP64_OP_MULHSU ||
            dec.opcode == LNP64_OP_EXIT ||
            dec.opcode == LNP64_OP_OBJECT_CTL ||
            dec.opcode == LNP64_OP_CAP_DUP ||
            dec.opcode == LNP64_OP_CAP_SEND ||
            dec.opcode == LNP64_OP_CAP_RECV ||
            dec.opcode == LNP64_OP_CAP_REVOKE ||
            dec.opcode == LNP64_OP_FAULT_INJECT;
    end
endmodule

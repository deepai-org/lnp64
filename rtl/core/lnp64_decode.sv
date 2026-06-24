`timescale 1ns/1ps

import lnp64_pkg::*;

// ISA v2 decode: every instruction is a single 64-bit word. The opcode is the
// high byte [63:56]; register slots are fixed (rd[55:51], rs1[50:46],
// rs2[45:41], rs3[40:36], rs4[35:31], rs5[30:26]); the 32-bit immediate sits
// immediately below the lowest register slot the format uses (I:[45:14],
// S/B:[40:9], U/J:[50:19]) and is sign-extended to 32 bits here.
module lnp64_decode (
    input  logic [63:0] instr,
    output lnp64_decode_t dec
);
    logic [7:0]  raw_opcode;
    logic [31:0] imm_i;   // I-type immediate  [45:14]
    logic [31:0] imm_sb;  // S/B-type immediate [40:9]
    logic [31:0] imm_uj;  // U/J-type immediate [50:19]

    always_comb begin
        raw_opcode = instr[63:56];
        imm_i  = instr[45:14];
        imm_sb = instr[40:9];
        imm_uj = instr[50:19];

        dec.opcode = {8'd0, raw_opcode};
        dec.profile = 16'd0;
        dec.rd  = {3'd0, instr[55:51]};
        dec.rs1 = {3'd0, instr[50:46]};
        dec.rs2 = {3'd0, instr[45:41]};
        dec.rs3 = {3'd0, instr[40:36]};
        dec.rs4 = {3'd0, instr[35:31]};
        dec.rs5 = {3'd0, instr[30:26]};
        // Default immediate is the I-type slot; overridden per format below.
        dec.imm = imm_i;

        unique case (raw_opcode)
            // --- system / misc ---
             8'h00: begin dec.opcode = LNP64_OP_NOP; end
             8'h02: begin dec.opcode = LNP64_OP_MOV; end
             8'h06: begin dec.opcode = LNP64_OP_YIELD; end
             8'h07: begin dec.opcode = LNP64_OP_SLEEP; end

            // --- constant materialization (I-type) ---
            8'h04: begin dec.opcode = LNP64_OP_LIU; end   // liu rd, rs1, imm32

            // --- integer ALU (R-type) ---
             8'h10: begin dec.opcode = LNP64_OP_ADD; end
             8'h11: begin dec.opcode = LNP64_OP_SUB; end
             8'h12: begin dec.opcode = LNP64_OP_MUL; end
             8'h13: begin dec.opcode = LNP64_OP_DIV; end
             8'ha7: begin dec.opcode = LNP64_OP_UDIV; end
             8'ha8: begin dec.opcode = LNP64_OP_SREM; end
             8'ha9: begin dec.opcode = LNP64_OP_UREM; end
             8'haa: begin dec.opcode = LNP64_OP_MULH; end
             8'hab: begin dec.opcode = LNP64_OP_MULHU; end
             8'hac: begin dec.opcode = LNP64_OP_MULHSU; end
             8'h14: begin dec.opcode = LNP64_OP_AND; end
             8'h15: begin dec.opcode = LNP64_OP_OR; end
             8'h16: begin dec.opcode = LNP64_OP_XOR; end
             8'h18: begin dec.opcode = LNP64_OP_LSL; end
             8'h19: begin dec.opcode = LNP64_OP_LSR; end
             8'h1a: begin dec.opcode = LNP64_OP_ASR; end
             8'hb6: begin dec.opcode = LNP64_OP_ROL; end
             8'hb7: begin dec.opcode = LNP64_OP_ROR; end
            8'h1b: begin dec.opcode = LNP64_OP_SLT; end   // set-less-than (was v1 cmp)
            8'h1c: begin dec.opcode = LNP64_OP_SLTU; end  // (was v1 cmpu)

            // --- unary (R-type) ---
             8'h17: begin dec.opcode = LNP64_OP_NOT; end
             8'had: begin dec.opcode = LNP64_OP_SEXT_B; end
             8'hae: begin dec.opcode = LNP64_OP_SEXT_H; end
             8'haf: begin dec.opcode = LNP64_OP_SEXT_W; end
             8'hb0: begin dec.opcode = LNP64_OP_ZEXT_B; end
             8'hb1: begin dec.opcode = LNP64_OP_ZEXT_H; end
             8'hb2: begin dec.opcode = LNP64_OP_ZEXT_W; end
             8'hb3: begin dec.opcode = LNP64_OP_CLZ; end
             8'hb4: begin dec.opcode = LNP64_OP_CTZ; end
             8'hb5: begin dec.opcode = LNP64_OP_POPCNT; end
             8'hb8: begin dec.opcode = LNP64_OP_BSWAP16; end
             8'hb9: begin dec.opcode = LNP64_OP_BSWAP32; end
             8'hba: begin dec.opcode = LNP64_OP_BSWAP64; end

            // --- register-immediate (I-type) ---
             8'ha0: begin dec.opcode = LNP64_OP_ADDI; end
             8'ha1: begin dec.opcode = LNP64_OP_ANDI; end
             8'ha2: begin dec.opcode = LNP64_OP_ORI; end
             8'ha3: begin dec.opcode = LNP64_OP_XORI; end
             8'ha4: begin dec.opcode = LNP64_OP_LSLI; end
             8'ha5: begin dec.opcode = LNP64_OP_LSRI; end
             8'ha6: begin dec.opcode = LNP64_OP_ASRI; end
             8'h1d: begin dec.opcode = LNP64_OP_SLTI; end
             8'h1e: begin dec.opcode = LNP64_OP_SLTIU; end

            // --- loads (I-type) ---
            8'h30: begin dec.opcode = LNP64_OP_LD; end     // ld   64
            8'h31: begin dec.opcode = LNP64_OP_LD_W; end   // lwu  zext32
            8'h32: begin dec.opcode = LNP64_OP_LD_B; end   // lbu  zext8
            8'h36: begin dec.opcode = LNP64_OP_LD_H; end   // lhu  zext16
            8'h05: begin dec.opcode = LNP64_OP_LW; end     // lw   sext32
            8'h08: begin dec.opcode = LNP64_OP_SEXT_B; end // lb   sext8  (load+sext path)
            8'h09: begin dec.opcode = LNP64_OP_SEXT_H; end // lh   sext16

            // --- stores (S-type) ---
            8'h33: begin dec.opcode = LNP64_OP_ST;   dec.imm = imm_sb; end
            8'h34: begin dec.opcode = LNP64_OP_ST_W; dec.imm = imm_sb; end
            8'h35: begin dec.opcode = LNP64_OP_ST_B; dec.imm = imm_sb; end
            8'h37: begin dec.opcode = LNP64_OP_ST_H; dec.imm = imm_sb; end

            // --- atomics LR/SC ---
             8'hc5: begin dec.opcode = LNP64_OP_LR_D; end
             8'hc6: begin dec.opcode = LNP64_OP_SC_D; end
             8'hcd: begin dec.opcode = LNP64_OP_FENCE; end
             8'hce: begin dec.opcode = LNP64_OP_ISYNC; end

            // --- control transfer ---
            8'h20: begin dec.opcode = LNP64_OP_JMP;        dec.imm = imm_uj; end
            8'h27: begin dec.opcode = LNP64_OP_JAL;        dec.imm = imm_uj; end
            8'h28: begin dec.opcode = LNP64_OP_JALR;       dec.imm = imm_i;  end
            8'h21: begin dec.opcode = LNP64_OP_BRANCH_EQ;  dec.imm = imm_sb; end
            8'h22: begin dec.opcode = LNP64_OP_BRANCH_NE;  dec.imm = imm_sb; end
            8'h23: begin dec.opcode = LNP64_OP_BRANCH_LT;  dec.imm = imm_sb; end
            8'h24: begin dec.opcode = LNP64_OP_BRANCH_GE;  dec.imm = imm_sb; end
            8'h25: begin dec.opcode = LNP64_OP_BRANCH_LTU; dec.imm = imm_sb; end
            8'h26: begin dec.opcode = LNP64_OP_BRANCH_GEU; dec.imm = imm_sb; end

            // --- fused compare-and-select: rd = (rs1 <cc> rs2) ? rs3 : rs4 ---
            8'h40: begin dec.opcode = LNP64_OP_SEL_EQ;  end
            8'h41: begin dec.opcode = LNP64_OP_SEL_NE;  end
            8'h42: begin dec.opcode = LNP64_OP_SEL_LT;  end
            8'h43: begin dec.opcode = LNP64_OP_SEL_GE;  end
            8'h44: begin dec.opcode = LNP64_OP_SEL_LTU; end
            8'h45: begin dec.opcode = LNP64_OP_SEL_GEU; end

            // --- constants / PC-relative ---
            8'hd0: begin dec.opcode = LNP64_OP_AUIPC; dec.imm = imm_uj; end

            // --- PCR ---
             8'h54: begin dec.opcode = LNP64_OP_GET_PCR; end
             8'h55: begin dec.opcode = LNP64_OP_SET_PCR; end

            // --- system / capability / FDR / path primitives (single word) ---
             8'h2b: begin dec.opcode = LNP64_OP_PULL; end
             8'h2c: begin dec.opcode = LNP64_OP_PUSH; end
             8'h2d: begin dec.opcode = LNP64_OP_READ_FD; end
             8'h2e: begin dec.opcode = LNP64_OP_AWAIT; end
             8'h2f: begin dec.opcode = LNP64_OP_GATE_CALL; end
             8'h38: begin dec.opcode = LNP64_OP_GET_ERRNO; end
             8'h39: begin dec.opcode = LNP64_OP_SET_ERRNO; end
             8'h3a: begin dec.opcode = LNP64_OP_EXIT; end
             // 0x3b/0x3c (ReadFdDyn/WriteFdDyn twins) retired in F1-step-2 — the
             // unified recv/send verbs (0x84/0x83) cover byte-fd transfer.
             8'h47: begin dec.opcode = LNP64_OP_ALLOC; end
             8'h48: begin dec.opcode = LNP64_OP_ALLOC_SIZE; end
             8'h49: begin dec.opcode = LNP64_OP_FREE; end
             8'h4a: begin dec.opcode = LNP64_OP_ALLOC_EX; end
             8'h4b: begin dec.opcode = LNP64_OP_OBJECT_CTL; end
             8'h4c: begin dec.opcode = LNP64_OP_DOMAIN_CTL; end
             8'h4d: begin dec.opcode = LNP64_OP_AWAIT; end
             8'h4e: begin dec.opcode = LNP64_OP_GATE_CALL; end
             8'h4f: begin dec.opcode = LNP64_OP_GATE_RETURN; end
             8'h50: begin dec.opcode = LNP64_OP_CAP_DUP; end
             8'h51: begin dec.opcode = LNP64_OP_CAP_SEND; end
             8'h52: begin dec.opcode = LNP64_OP_CAP_RECV; end
             8'h53: begin dec.opcode = LNP64_OP_CAP_REVOKE; end
             8'h56: begin dec.opcode = LNP64_OP_ENV_GET; end
             8'h57: begin dec.opcode = LNP64_OP_WRITE_FD; end
             8'h59: begin dec.opcode = LNP64_OP_CLONE; end
             8'h5a: begin dec.opcode = LNP64_OP_JOIN; end
             8'h5b: begin dec.opcode = LNP64_OP_DMA_CTL; end
             8'h62: begin dec.opcode = LNP64_OP_SIGACTION; end
             8'h64: begin dec.opcode = LNP64_OP_KILL; end
             8'h65: begin dec.opcode = LNP64_OP_SIGRET; end
             8'h6a: begin dec.opcode = LNP64_OP_MMAP; end
             8'h6c: begin dec.opcode = LNP64_OP_MPROTECT; end
             8'h6d: begin dec.opcode = LNP64_OP_OPEN_FD; end
             8'h6e: begin dec.opcode = LNP64_OP_FD_CLOSE; end
             8'h6f: begin dec.opcode = LNP64_OP_WAITABLE_PROBE; end
             // 0x70/0x72 (dynamic waitable_probe/await_ex twins) retired in F1.
             8'h71: begin dec.opcode = LNP64_OP_AWAIT_EX; end
             8'h7d: begin dec.opcode = LNP64_OP_FORK; end
             8'h7f: begin dec.opcode = LNP64_OP_EXEC; end
             8'h80: begin dec.opcode = LNP64_OP_INB; end
             8'h81: begin dec.opcode = LNP64_OP_OUTB; end
             8'h82: begin dec.opcode = LNP64_OP_LOAD_UCODE; end
             // EP-I-lite: byte-fd IPC verbs delegate to the WRITE_FD/READ_FD
             // microcode; lnp64_core_tile.sv detects raw 0x83/0x84 and re-sources
             // operands (handle in rs1, msg descriptor in rs2). send=0x83 is the
             // write path, recv=0x84 the read path. (wait=0x86/endpoint=0x88 land
             // with the M16 endpoint engine in EP-I-full.)
             8'h83: begin dec.opcode = LNP64_OP_WRITE_FD; end
             8'h84: begin dec.opcode = LNP64_OP_READ_FD; end
             8'hcb: begin dec.opcode = LNP64_OP_FUTEX_WAIT; end
             8'hcc: begin dec.opcode = LNP64_OP_FUTEX_WAKE; end

             8'hff: begin dec.opcode = LNP64_OP_UNSUPPORTED; end
            default: dec.opcode = {8'd0, raw_opcode};
        endcase

        dec.supported =
            dec.opcode == LNP64_OP_NOP ||
            dec.opcode == LNP64_OP_MOV ||
            dec.opcode == LNP64_OP_LIU ||
            dec.opcode == LNP64_OP_ADD ||
            dec.opcode == LNP64_OP_SUB ||
            dec.opcode == LNP64_OP_MUL ||
            dec.opcode == LNP64_OP_DIV ||
            dec.opcode == LNP64_OP_UDIV ||
            dec.opcode == LNP64_OP_SREM ||
            dec.opcode == LNP64_OP_UREM ||
            dec.opcode == LNP64_OP_MULH ||
            dec.opcode == LNP64_OP_MULHU ||
            dec.opcode == LNP64_OP_MULHSU ||
            dec.opcode == LNP64_OP_AND ||
            dec.opcode == LNP64_OP_OR ||
            dec.opcode == LNP64_OP_XOR ||
            dec.opcode == LNP64_OP_NOT ||
            dec.opcode == LNP64_OP_LSL ||
            dec.opcode == LNP64_OP_LSR ||
            dec.opcode == LNP64_OP_ASR ||
            dec.opcode == LNP64_OP_ROL ||
            dec.opcode == LNP64_OP_ROR ||
            dec.opcode == LNP64_OP_SLT ||
            dec.opcode == LNP64_OP_SLTU ||
            dec.opcode == LNP64_OP_SLTI ||
            dec.opcode == LNP64_OP_SLTIU ||
            dec.opcode == LNP64_OP_SEL_EQ ||
            dec.opcode == LNP64_OP_SEL_NE ||
            dec.opcode == LNP64_OP_SEL_LT ||
            dec.opcode == LNP64_OP_SEL_GE ||
            dec.opcode == LNP64_OP_SEL_LTU ||
            dec.opcode == LNP64_OP_SEL_GEU ||
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
            dec.opcode == LNP64_OP_BSWAP16 ||
            dec.opcode == LNP64_OP_BSWAP32 ||
            dec.opcode == LNP64_OP_BSWAP64 ||
            dec.opcode == LNP64_OP_JMP ||
            dec.opcode == LNP64_OP_JAL ||
            dec.opcode == LNP64_OP_JALR ||
            dec.opcode == LNP64_OP_BRANCH_EQ ||
            dec.opcode == LNP64_OP_BRANCH_NE ||
            dec.opcode == LNP64_OP_BRANCH_LT ||
            dec.opcode == LNP64_OP_BRANCH_GE ||
            dec.opcode == LNP64_OP_BRANCH_LTU ||
            dec.opcode == LNP64_OP_BRANCH_GEU ||
            dec.opcode == LNP64_OP_AUIPC ||
            dec.opcode == LNP64_OP_FENCE ||
            dec.opcode == LNP64_OP_ISYNC ||
            dec.opcode == LNP64_OP_GATE_CALL ||
            dec.opcode == LNP64_OP_GATE_RETURN ||
            dec.opcode == LNP64_OP_LR_D ||
            dec.opcode == LNP64_OP_SC_D ||
            dec.opcode == LNP64_OP_LW ||
            dec.opcode == LNP64_OP_LD ||
            dec.opcode == LNP64_OP_LD_W ||
            dec.opcode == LNP64_OP_LD_H ||
            dec.opcode == LNP64_OP_LD_B ||
            dec.opcode == LNP64_OP_ST ||
            dec.opcode == LNP64_OP_ST_W ||
            dec.opcode == LNP64_OP_ST_H ||
            dec.opcode == LNP64_OP_ST_B ||
            dec.opcode == LNP64_OP_YIELD ||
            dec.opcode == LNP64_OP_SLEEP ||
            dec.opcode == LNP64_OP_MMAP ||
            dec.opcode == LNP64_OP_MPROTECT ||
            dec.opcode == LNP64_OP_SIGACTION ||
            dec.opcode == LNP64_OP_KILL ||
            dec.opcode == LNP64_OP_SIGRET ||
            dec.opcode == LNP64_OP_INB ||
            dec.opcode == LNP64_OP_OUTB ||
            dec.opcode == LNP64_OP_LOAD_UCODE ||
            dec.opcode == LNP64_OP_OPEN_FD ||
            dec.opcode == LNP64_OP_FD_CLOSE ||
            dec.opcode == LNP64_OP_WAITABLE_PROBE ||
            dec.opcode == LNP64_OP_AWAIT_EX ||
            dec.opcode == LNP64_OP_GET_PCR ||
            dec.opcode == LNP64_OP_SET_PCR ||
            dec.opcode == LNP64_OP_CLONE ||
            dec.opcode == LNP64_OP_JOIN ||
            dec.opcode == LNP64_OP_FUTEX_WAIT ||
            dec.opcode == LNP64_OP_FUTEX_WAKE ||
            dec.opcode == LNP64_OP_FORK ||
            dec.opcode == LNP64_OP_EXEC ||
            dec.opcode == LNP64_OP_DMA_CTL ||
            dec.opcode == LNP64_OP_ENV_GET ||
            dec.opcode == LNP64_OP_READ_FD ||
            dec.opcode == LNP64_OP_WRITE_FD ||
            dec.opcode == LNP64_OP_AWAIT ||
            dec.opcode == LNP64_OP_PUSH ||
            dec.opcode == LNP64_OP_PULL ||
            dec.opcode == LNP64_OP_ALLOC ||
            dec.opcode == LNP64_OP_ALLOC_SIZE ||
            dec.opcode == LNP64_OP_FREE ||
            dec.opcode == LNP64_OP_ALLOC_EX ||
            dec.opcode == LNP64_OP_GET_ERRNO ||
            dec.opcode == LNP64_OP_SET_ERRNO ||
            dec.opcode == LNP64_OP_EXIT ||
            dec.opcode == LNP64_OP_OBJECT_CTL ||
            dec.opcode == LNP64_OP_DOMAIN_CTL ||
            dec.opcode == LNP64_OP_CAP_DUP ||
            dec.opcode == LNP64_OP_CAP_SEND ||
            dec.opcode == LNP64_OP_CAP_RECV ||
            dec.opcode == LNP64_OP_CAP_REVOKE ||
            dec.opcode == LNP64_OP_FAULT_INJECT;
    end
endmodule

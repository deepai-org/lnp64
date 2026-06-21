//===-- LNP64InstrInfo.td - LNP64 v2 instruction definitions -------------===//
//
// v2 ISA: fixed 64-bit instructions, one major opcode in [63:56].
//   rd[55:51] rs1[50:46] rs2[45:41] rs3[40:36] rs4[35:31] rs5[30:26]
//   I-type imm32 [45:14]; S/B-type imm32 [40:9]; U/J-type imm32 [50:19].
//   Branch/jump target = pc + (sext32(imm) << 3).
//
// The hand-written MCCodeEmitter / AsmParser / Disassembler / InstPrinter are
// the source of bytes for the MC layer (TableGen-generated AsmMatcher/encoder
// is deferred). The bits<64> Inst fields below document the layout.
//===----------------------------------------------------------------------===//

def simm32 : Operand<i64>;
def brtarget : Operand<OtherVT>;
def calltarget : Operand<i64>;

def simm32_imm : PatLeaf<(imm), [{
  return isInt<32>(N->getSExtValue());
}]>;
def simm32_i64_imm : PatLeaf<(i64 imm), [{
  return isInt<32>(N->getSExtValue());
}]>;
def wide64_imm : PatLeaf<(imm), [{
  return !isInt<32>(N->getSExtValue());
}]>;
def all_ones_imm : PatLeaf<(imm), [{
  return N->isAllOnes();
}]>;
def u8_mask_i64_imm : PatLeaf<(i64 imm), [{
  return N->getZExtValue() == 0xff;
}]>;
def u16_mask_i64_imm : PatLeaf<(i64 imm), [{
  return N->getZExtValue() == 0xffff;
}]>;
def u32_mask_i64_imm : PatLeaf<(i64 imm), [{
  return N->getZExtValue() == 0xffffffffULL;
}]>;

def LNP64retflag : SDNode<"LNP64ISD::RET_FLAG", SDTNone,
                          [SDNPHasChain, SDNPOptInGlue, SDNPVariadic]>;
def SDT_LNP64Call : SDTypeProfile<0, -1, []>;
def LNP64call : SDNode<"LNP64ISD::CALL", SDT_LNP64Call,
                       [SDNPHasChain, SDNPOptInGlue, SDNPOutGlue,
                        SDNPVariadic]>;
def SDT_LNP64CallSeqStart : SDCallSeqStart<[SDTCisVT<0, i64>,
                                             SDTCisVT<1, i64>]>;
def SDT_LNP64CallSeqEnd : SDCallSeqEnd<[SDTCisVT<0, i64>, SDTCisVT<1, i64>]>;
def LNP64callseq_start : SDNode<"ISD::CALLSEQ_START", SDT_LNP64CallSeqStart,
                                [SDNPHasChain, SDNPOutGlue]>;
def LNP64callseq_end : SDNode<"ISD::CALLSEQ_END", SDT_LNP64CallSeqEnd,
                              [SDNPHasChain, SDNPOptInGlue, SDNPOutGlue]>;

def SDT_LNP64Native1 : SDTypeProfile<1, 1, [
  SDTCisVT<0, i64>, SDTCisVT<1, i64>
]>;
def SDT_LNP64Native3 : SDTypeProfile<1, 3, [
  SDTCisVT<0, i64>, SDTCisVT<1, i64>, SDTCisVT<2, i64>, SDTCisVT<3, i64>
]>;
def LNP64await : SDNode<"LNP64ISD::AWAIT", SDT_LNP64Native3, [SDNPHasChain]>;
def LNP64domainctl : SDNode<"LNP64ISD::DOMAIN_CTL", SDT_LNP64Native1,
                            [SDNPHasChain]>;
def LNP64gatecall : SDNode<"LNP64ISD::GATE_CALL", SDT_LNP64Native3,
                           [SDNPHasChain]>;
def LNP64gatereturn : SDNode<"LNP64ISD::GATE_RETURN", SDT_LNP64Native3,
                             [SDNPHasChain]>;
def LNP64objectctl : SDNode<"LNP64ISD::OBJECT_CTL", SDT_LNP64Native1,
                            [SDNPHasChain]>;
def LNP64pull : SDNode<"LNP64ISD::PULL", SDT_LNP64Native3, [SDNPHasChain]>;
def LNP64push : SDNode<"LNP64ISD::PUSH", SDT_LNP64Native3, [SDNPHasChain]>;
def SDT_LNP64Wrapper : SDTypeProfile<1, 1, [
  SDTCisSameAs<0, 1>, SDTCisPtrTy<0>
]>;
def LNP64wrapper : SDNode<"LNP64ISD::WRAPPER", SDT_LNP64Wrapper>;
def SDT_LNP64SelectCC : SDTypeProfile<1, 5, [
  SDTCisSameAs<0, 1>, SDTCisSameAs<1, 2>, SDTCisVT<3, i64>,
  SDTCisSameAs<0, 4>, SDTCisSameAs<0, 5>
]>;
def LNP64selectcc : SDNode<"LNP64ISD::SELECT_CC", SDT_LNP64SelectCC>;

class LNP64Inst<dag Outs, dag Ins, string Mnemonic, string Operands>
    : Instruction {
  let Namespace = "LNP64";
  let Size = 8;
  let OutOperandList = Outs;
  let InOperandList = Ins;
  let AsmString = !strconcat(Mnemonic, Operands);
}

class LNP64NoOperand<string Mnemonic>
    : LNP64Inst<(outs), (ins), Mnemonic, "">;

class LNP64RR<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$rs), Mnemonic, "\t$rd, $rs">;

class LNP64RRR<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$rs1, GPR:$rs2), Mnemonic,
                "\t$rd, $rs1, $rs2">;

// I-type register-immediate. The immediate is now a full signed-32.
class LNP64RRI<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$rs, simm32:$imm), Mnemonic,
                "\t$rd, $rs, $imm">;

class LNP64Branch<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$rs1, GPR:$rs2, brtarget:$target), Mnemonic,
                "\t$rs1, $rs2, $target"> {
  let isBranch = 1;
  let isTerminator = 1;
}

// I-type load: rd, imm32(base).
class LNP64MemLoad<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$base, simm32:$offset), Mnemonic,
                "\t$rd, $offset($base)">;

class LNP64FrameAddrPseudo<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$base, simm32:$offset), Mnemonic,
                "\t$rd, $offset($base)"> {
  let isPseudo = 1;
}

// S-type store: rs(src), imm32(base).
class LNP64MemStore<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$rs, GPR:$base, simm32:$offset), Mnemonic,
                "\t$rs, $offset($base)">;

// Native multi-register forms (syscall/capability/path primitives). These keep
// their v1 semantics, re-encoded as one 64-bit R-type word.
class LNP64Native4<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$cap, GPR:$arg0, GPR:$arg1),
                Mnemonic, "\t$rd, $cap, $arg0, $arg1">;
class LNP64Native2<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$arg), Mnemonic, "\t$rd, $arg">;
class LNP64Status5<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$arg0, GPR:$arg1, GPR:$arg2, GPR:$arg3,
                         GPR:$arg4),
                Mnemonic, "\t$arg0, $arg1, $arg2, $arg3, $arg4">;
class LNP64Status4<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$arg0, GPR:$arg1, GPR:$arg2, GPR:$arg3),
                Mnemonic, "\t$arg0, $arg1, $arg2, $arg3">;
class LNP64Status3<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$arg0, GPR:$arg1, GPR:$arg2),
                Mnemonic, "\t$arg0, $arg1, $arg2">;
class LNP64Status2<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$arg0, GPR:$arg1), Mnemonic,
                "\t$arg0, $arg1">;
class LNP64Status1<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$arg0), Mnemonic, "\t$arg0">;

class LNP64RuntimeGet<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins), Mnemonic, "\t$rd">;
class LNP64RuntimeSet<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$rs), Mnemonic, "\t$rs">;
class LNP64PcrGet<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins PCR:$pcr), Mnemonic, "\t$rd, $pcr">;
class LNP64PcrSet<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins PCR:$pcr, GPR:$rs), Mnemonic,
                "\t$rd, $pcr, $rs">;
class LNP64Signal2<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$rs1, GPR:$rs2), Mnemonic, "\t$rs1, $rs2">;
class LNP64Signal1<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$rs), Mnemonic, "\t$rs">;

//===----------------------------------------------------------------------===//
// Constants and moves.
//===----------------------------------------------------------------------===//

def NOP : LNP64NoOperand<"nop">;
def YIELD : LNP64NoOperand<"yield">;

// li rd, imm32 is an alias for addi rd, r0, imm32; LLVM materializes it via the
// addi path. We keep an LI form for the asm spelling "li rd, imm".
def LI : LNP64Inst<(outs GPR:$rd), (ins simm32:$imm), "li", "\t$rd, $imm"> {
  let Pattern = [(set GPR:$rd, simm32_imm:$imm)];
}
def MOV : LNP64RR<"mov">;
def AUIPC : LNP64Inst<(outs GPR:$rd), (ins i64imm:$target), "auipc",
                      "\t$rd, $target">;
def LIU : LNP64RRI<"liu">;
def : Pat<(i64 (LNP64wrapper tglobaladdr:$target)), (AUIPC tglobaladdr:$target)>;

// 64-bit constants: li lo32 ; liu rd, rd, hi32.
def PseudoLI64
    : LNP64Inst<(outs GPR:$rd), (ins i64imm:$imm), "pseudo_li64",
                "\t$rd, $imm"> {
  let isPseudo = 1;
  let usesCustomInserter = 1;
}
def : Pat<(i64 wide64_imm:$imm), (PseudoLI64 wide64_imm:$imm)>;

// SELECT_CC pseudo: compare lhs/rhs under cc and pick trueV/falseV. Expanded by
// the custom inserter into a compare-and-branch diamond.
def PseudoSELECT_CC
    : LNP64Inst<(outs GPR:$dst),
                (ins GPR:$lhs, GPR:$rhs, i64imm:$cc, GPR:$truev, GPR:$falsev),
                "pseudo_select_cc", "\t$dst, $lhs, $rhs, $cc, $truev, $falsev"> {
  let isPseudo = 1;
  let usesCustomInserter = 1;
  let Pattern = [(set GPR:$dst,
                  (LNP64selectcc GPR:$lhs, GPR:$rhs, imm:$cc, GPR:$truev,
                   GPR:$falsev))];
}

//===----------------------------------------------------------------------===//
// Integer ALU (R-type).
//===----------------------------------------------------------------------===//

def ADD : LNP64RRR<"add"> { let Pattern = [(set GPR:$rd, (add GPR:$rs1, GPR:$rs2))]; }
def SUB : LNP64RRR<"sub"> { let Pattern = [(set GPR:$rd, (sub GPR:$rs1, GPR:$rs2))]; }
def MUL : LNP64RRR<"mul"> { let Pattern = [(set GPR:$rd, (mul GPR:$rs1, GPR:$rs2))]; }
def DIV : LNP64RRR<"div"> { let Pattern = [(set GPR:$rd, (sdiv GPR:$rs1, GPR:$rs2))]; }
def UDIV : LNP64RRR<"udiv"> { let Pattern = [(set GPR:$rd, (udiv GPR:$rs1, GPR:$rs2))]; }
def SREM : LNP64RRR<"srem"> { let Pattern = [(set GPR:$rd, (srem GPR:$rs1, GPR:$rs2))]; }
def UREM : LNP64RRR<"urem"> { let Pattern = [(set GPR:$rd, (urem GPR:$rs1, GPR:$rs2))]; }
def MULH : LNP64RRR<"mulh"> { let Pattern = [(set GPR:$rd, (mulhs GPR:$rs1, GPR:$rs2))]; }
def MULHU : LNP64RRR<"mulhu"> { let Pattern = [(set GPR:$rd, (mulhu GPR:$rs1, GPR:$rs2))]; }
def MULHSU : LNP64RRR<"mulhsu">;
def AND : LNP64RRR<"and"> { let Pattern = [(set GPR:$rd, (and GPR:$rs1, GPR:$rs2))]; }
def OR : LNP64RRR<"or"> { let Pattern = [(set GPR:$rd, (or GPR:$rs1, GPR:$rs2))]; }
def XOR : LNP64RRR<"xor"> { let Pattern = [(set GPR:$rd, (xor GPR:$rs1, GPR:$rs2))]; }
def SLL : LNP64RRR<"sll"> { let Pattern = [(set GPR:$rd, (shl GPR:$rs1, GPR:$rs2))]; }
def SRL : LNP64RRR<"srl"> { let Pattern = [(set GPR:$rd, (srl GPR:$rs1, GPR:$rs2))]; }
def SRA : LNP64RRR<"sra"> { let Pattern = [(set GPR:$rd, (sra GPR:$rs1, GPR:$rs2))]; }
def ROL : LNP64RRR<"rol"> { let Pattern = [(set GPR:$rd, (rotl GPR:$rs1, GPR:$rs2))]; }
def ROR : LNP64RRR<"ror"> { let Pattern = [(set GPR:$rd, (rotr GPR:$rs1, GPR:$rs2))]; }
def SLT : LNP64RRR<"slt"> { let Pattern = [(set GPR:$rd, (setcc GPR:$rs1, GPR:$rs2, SETLT))]; }
def SLTU : LNP64RRR<"sltu"> { let Pattern = [(set GPR:$rd, (setcc GPR:$rs1, GPR:$rs2, SETULT))]; }

//===----------------------------------------------------------------------===//
// Unary (R-type).
//===----------------------------------------------------------------------===//

def NOT : LNP64RR<"not"> { let Pattern = [(set GPR:$rd, (xor GPR:$rs, all_ones_imm))]; }
def SEXT_B : LNP64RR<"sext.b"> { let Pattern = [(set GPR:$rd, (sext_inreg GPR:$rs, i8))]; }
def SEXT_H : LNP64RR<"sext.h"> { let Pattern = [(set GPR:$rd, (sext_inreg GPR:$rs, i16))]; }
def SEXT_W : LNP64RR<"sext.w"> { let Pattern = [(set GPR:$rd, (sext_inreg GPR:$rs, i32))]; }
let AddedComplexity = 1 in {
def ZEXT_B : LNP64RR<"zext.b"> { let Pattern = [(set GPR:$rd, (and GPR:$rs, u8_mask_i64_imm))]; }
def ZEXT_H : LNP64RR<"zext.h"> { let Pattern = [(set GPR:$rd, (and GPR:$rs, u16_mask_i64_imm))]; }
def ZEXT_W : LNP64RR<"zext.w"> { let Pattern = [(set GPR:$rd, (and GPR:$rs, u32_mask_i64_imm))]; }
}
def CLZ : LNP64RR<"clz"> { let Pattern = [(set GPR:$rd, (ctlz GPR:$rs))]; }
def : Pat<(i64 (ctlz_zero_undef GPR:$rs)), (CLZ GPR:$rs)>;
def CTZ : LNP64RR<"ctz"> { let Pattern = [(set GPR:$rd, (cttz GPR:$rs))]; }
def : Pat<(i64 (cttz_zero_undef GPR:$rs)), (CTZ GPR:$rs)>;
def POPCNT : LNP64RR<"popcnt"> { let Pattern = [(set GPR:$rd, (ctpop GPR:$rs))]; }
def BSWAP16 : LNP64RR<"bswap16">;
def BSWAP32 : LNP64RR<"bswap32">;
def BSWAP64 : LNP64RR<"bswap64"> { let Pattern = [(set GPR:$rd, (bswap GPR:$rs))]; }

//===----------------------------------------------------------------------===//
// Register-immediate (I-type).
//===----------------------------------------------------------------------===//

def ADDI : LNP64RRI<"addi"> { let Pattern = [(set GPR:$rd, (add GPR:$rs, simm32_i64_imm:$imm))]; }
def ANDI : LNP64RRI<"andi"> { let Pattern = [(set GPR:$rd, (and GPR:$rs, simm32_i64_imm:$imm))]; }
def ORI : LNP64RRI<"ori"> { let Pattern = [(set GPR:$rd, (or GPR:$rs, simm32_i64_imm:$imm))]; }
def XORI : LNP64RRI<"xori"> { let Pattern = [(set GPR:$rd, (xor GPR:$rs, simm32_i64_imm:$imm))]; }
def SLLI : LNP64RRI<"slli"> { let Pattern = [(set GPR:$rd, (shl GPR:$rs, simm32_i64_imm:$imm))]; }
def SRLI : LNP64RRI<"srli"> { let Pattern = [(set GPR:$rd, (srl GPR:$rs, simm32_i64_imm:$imm))]; }
def SRAI : LNP64RRI<"srai"> { let Pattern = [(set GPR:$rd, (sra GPR:$rs, simm32_i64_imm:$imm))]; }
def SLTI : LNP64RRI<"slti"> { let Pattern = [(set GPR:$rd, (setcc GPR:$rs, simm32_i64_imm:$imm, SETLT))]; }
def SLTIU : LNP64RRI<"sltiu"> { let Pattern = [(set GPR:$rd, (setcc GPR:$rs, simm32_i64_imm:$imm, SETULT))]; }

//===----------------------------------------------------------------------===//
// setcc / select_cc legalization using SLT idioms.
//===----------------------------------------------------------------------===//

// seteq a,b  -> sltiu (xor a,b), 1
def : Pat<(i64 (setcc GPR:$a, GPR:$b, SETEQ)),
          (SLTIU (XOR GPR:$a, GPR:$b), 1)>;
// setne a,b  -> sltu r0, (xor a,b)
def : Pat<(i64 (setcc GPR:$a, GPR:$b, SETNE)),
          (SLTU R0, (XOR GPR:$a, GPR:$b))>;
// setgt a,b  -> slt b,a
def : Pat<(i64 (setcc GPR:$a, GPR:$b, SETGT)), (SLT GPR:$b, GPR:$a)>;
def : Pat<(i64 (setcc GPR:$a, GPR:$b, SETUGT)), (SLTU GPR:$b, GPR:$a)>;
// setle a,b  -> xori (slt b,a), 1
def : Pat<(i64 (setcc GPR:$a, GPR:$b, SETLE)), (XORI (SLT GPR:$b, GPR:$a), 1)>;
def : Pat<(i64 (setcc GPR:$a, GPR:$b, SETULE)), (XORI (SLTU GPR:$b, GPR:$a), 1)>;
// setge a,b  -> xori (slt a,b), 1
def : Pat<(i64 (setcc GPR:$a, GPR:$b, SETGE)), (XORI (SLT GPR:$a, GPR:$b), 1)>;
def : Pat<(i64 (setcc GPR:$a, GPR:$b, SETUGE)), (XORI (SLTU GPR:$a, GPR:$b), 1)>;

//===----------------------------------------------------------------------===//
// Control transfer.
//===----------------------------------------------------------------------===//

let isBranch = 1, isTerminator = 1, isBarrier = 1 in
def JMP : LNP64Inst<(outs), (ins brtarget:$target), "jmp", "\t$target"> {
  let Pattern = [(br bb:$target)];
}

def BEQ : LNP64Branch<"beq">;
def BNE : LNP64Branch<"bne">;
def BLT : LNP64Branch<"blt">;
def BGE : LNP64Branch<"bge">;
def BLTU : LNP64Branch<"bltu">;
def BGEU : LNP64Branch<"bgeu">;

def : Pat<(brcc SETEQ, GPR:$a, GPR:$b, bb:$t), (BEQ GPR:$a, GPR:$b, bb:$t)>;
def : Pat<(brcc SETNE, GPR:$a, GPR:$b, bb:$t), (BNE GPR:$a, GPR:$b, bb:$t)>;
def : Pat<(brcc SETLT, GPR:$a, GPR:$b, bb:$t), (BLT GPR:$a, GPR:$b, bb:$t)>;
def : Pat<(brcc SETGT, GPR:$a, GPR:$b, bb:$t), (BLT GPR:$b, GPR:$a, bb:$t)>;
def : Pat<(brcc SETLE, GPR:$a, GPR:$b, bb:$t), (BGE GPR:$b, GPR:$a, bb:$t)>;
def : Pat<(brcc SETGE, GPR:$a, GPR:$b, bb:$t), (BGE GPR:$a, GPR:$b, bb:$t)>;
def : Pat<(brcc SETULT, GPR:$a, GPR:$b, bb:$t), (BLTU GPR:$a, GPR:$b, bb:$t)>;
def : Pat<(brcc SETUGT, GPR:$a, GPR:$b, bb:$t), (BLTU GPR:$b, GPR:$a, bb:$t)>;
def : Pat<(brcc SETULE, GPR:$a, GPR:$b, bb:$t), (BGEU GPR:$b, GPR:$a, bb:$t)>;
def : Pat<(brcc SETUGE, GPR:$a, GPR:$b, bb:$t), (BGEU GPR:$a, GPR:$b, bb:$t)>;

// JAL: J-type. rd = pc+8; pc += off. call sym -> jal r1, sym.
let isCall = 1,
    Defs = [R1, R2, R3, R4, R5, R6, R7, R8, R9, R10, R11, R12, R13, R14,
            R15, R16, R17, R18, R19, R20, R21, R22, R23, R24, R25, R26, R27,
            R28, R29] in {
def JAL : LNP64Inst<(outs GPR:$rd), (ins calltarget:$target), "jal",
                    "\t$rd, $target">;
def JALR : LNP64Inst<(outs GPR:$rd), (ins GPR:$rs1, simm32:$imm), "jalr",
                     "\t$rd, $rs1, $imm">;
// Codegen call pseudos: rd is implicitly r1 (ra). Emitter expands them to
// jal r1, target / jalr r1, target, 0.
def PseudoCALL
    : LNP64Inst<(outs), (ins calltarget:$target), "jal", "\tr1, $target">;
def PseudoCALLIndirect
    : LNP64Inst<(outs), (ins GPR:$target), "jalr", "\tr1, $target, 0">;
}
def : Pat<(LNP64call tglobaladdr:$target), (PseudoCALL tglobaladdr:$target)>;
def : Pat<(LNP64call texternalsym:$target), (PseudoCALL texternalsym:$target)>;
def : Pat<(LNP64call GPR:$target), (PseudoCALLIndirect GPR:$target)>;

let Defs = [R31], Uses = [R31] in {
def ADJCALLSTACKDOWN
    : LNP64Inst<(outs), (ins i64imm:$amt1, i64imm:$amt2),
                "#ADJCALLSTACKDOWN", "\t$amt1, $amt2"> {
  let isPseudo = 1;
  let Pattern = [(LNP64callseq_start timm:$amt1, timm:$amt2)];
}
def ADJCALLSTACKUP
    : LNP64Inst<(outs), (ins i64imm:$amt1, i64imm:$amt2),
                "#ADJCALLSTACKUP", "\t$amt1, $amt2"> {
  let isPseudo = 1;
  let Pattern = [(LNP64callseq_end timm:$amt1, timm:$amt2)];
}
}

// ret = jalr r0, r1, 0. Lowered through a pseudo so the asm spelling is "ret".
let isReturn = 1, isTerminator = 1, isBarrier = 1, Uses = [R1] in
def RET : LNP64NoOperand<"ret"> {
  let Pattern = [(LNP64retflag)];
}

//===----------------------------------------------------------------------===//
// Loads / stores.
//===----------------------------------------------------------------------===//

def LD : LNP64MemLoad<"ld">;
def LWU : LNP64MemLoad<"lwu">;
def LBU : LNP64MemLoad<"lbu">;
def LHU : LNP64MemLoad<"lhu">;
def LW : LNP64MemLoad<"lw">;
def LB : LNP64MemLoad<"lb">;
def LH : LNP64MemLoad<"lh">;
def PseudoFRAMEADDR : LNP64FrameAddrPseudo<"pseudo_frameaddr">;
def SD : LNP64MemStore<"sd">;
def SW : LNP64MemStore<"sw">;
def SB : LNP64MemStore<"sb">;
def SH : LNP64MemStore<"sh">;

def : Pat<(i64 (load (add GPR:$base, simm32_imm:$offset))), (LD GPR:$base, simm32_imm:$offset)>;
def : Pat<(i64 (load GPR:$base)), (LD GPR:$base, 0)>;
def : Pat<(i64 (zextloadi32 (add GPR:$base, simm32_imm:$offset))), (LWU GPR:$base, simm32_imm:$offset)>;
def : Pat<(i64 (zextloadi32 GPR:$base)), (LWU GPR:$base, 0)>;
def : Pat<(i64 (extloadi32 (add GPR:$base, simm32_imm:$offset))), (LWU GPR:$base, simm32_imm:$offset)>;
def : Pat<(i64 (extloadi32 GPR:$base)), (LWU GPR:$base, 0)>;
def : Pat<(i64 (sextloadi32 (add GPR:$base, simm32_imm:$offset))), (LW GPR:$base, simm32_imm:$offset)>;
def : Pat<(i64 (sextloadi32 GPR:$base)), (LW GPR:$base, 0)>;
def : Pat<(i64 (zextloadi16 (add GPR:$base, simm32_imm:$offset))), (LHU GPR:$base, simm32_imm:$offset)>;
def : Pat<(i64 (zextloadi16 GPR:$base)), (LHU GPR:$base, 0)>;
def : Pat<(i64 (extloadi16 (add GPR:$base, simm32_imm:$offset))), (LHU GPR:$base, simm32_imm:$offset)>;
def : Pat<(i64 (extloadi16 GPR:$base)), (LHU GPR:$base, 0)>;
def : Pat<(i64 (sextloadi16 (add GPR:$base, simm32_imm:$offset))), (LH GPR:$base, simm32_imm:$offset)>;
def : Pat<(i64 (sextloadi16 GPR:$base)), (LH GPR:$base, 0)>;
def : Pat<(i64 (zextloadi8 (add GPR:$base, simm32_imm:$offset))), (LBU GPR:$base, simm32_imm:$offset)>;
def : Pat<(i64 (zextloadi8 GPR:$base)), (LBU GPR:$base, 0)>;
def : Pat<(i64 (extloadi8 (add GPR:$base, simm32_imm:$offset))), (LBU GPR:$base, simm32_imm:$offset)>;
def : Pat<(i64 (extloadi8 GPR:$base)), (LBU GPR:$base, 0)>;
def : Pat<(i64 (zextloadi1 (add GPR:$base, simm32_imm:$offset))), (LBU GPR:$base, simm32_imm:$offset)>;
def : Pat<(i64 (zextloadi1 GPR:$base)), (LBU GPR:$base, 0)>;
def : Pat<(i64 (extloadi1 (add GPR:$base, simm32_imm:$offset))), (LBU GPR:$base, simm32_imm:$offset)>;
def : Pat<(i64 (extloadi1 GPR:$base)), (LBU GPR:$base, 0)>;
def : Pat<(i64 (sextloadi8 (add GPR:$base, simm32_imm:$offset))), (LB GPR:$base, simm32_imm:$offset)>;
def : Pat<(i64 (sextloadi8 GPR:$base)), (LB GPR:$base, 0)>;
def : Pat<(store GPR:$rs, (add GPR:$base, simm32_imm:$offset)), (SD GPR:$rs, GPR:$base, simm32_imm:$offset)>;
def : Pat<(store GPR:$rs, GPR:$base), (SD GPR:$rs, GPR:$base, 0)>;
def : Pat<(truncstorei32 GPR:$rs, (add GPR:$base, simm32_imm:$offset)), (SW GPR:$rs, GPR:$base, simm32_imm:$offset)>;
def : Pat<(truncstorei32 GPR:$rs, GPR:$base), (SW GPR:$rs, GPR:$base, 0)>;
def : Pat<(truncstorei16 GPR:$rs, (add GPR:$base, simm32_imm:$offset)), (SH GPR:$rs, GPR:$base, simm32_imm:$offset)>;
def : Pat<(truncstorei16 GPR:$rs, GPR:$base), (SH GPR:$rs, GPR:$base, 0)>;
def : Pat<(truncstorei8 GPR:$rs, (add GPR:$base, simm32_imm:$offset)), (SB GPR:$rs, GPR:$base, simm32_imm:$offset)>;
def : Pat<(truncstorei8 GPR:$rs, GPR:$base), (SB GPR:$rs, GPR:$base, 0)>;
def : Pat<(i64 (atomic_load_64 GPR:$base)), (LD GPR:$base, 0)>;
def : Pat<(atomic_store_64 GPR:$base, GPR:$rs), (SD GPR:$rs, GPR:$base, 0)>;

//===----------------------------------------------------------------------===//
// Atomics (LR/SC) and fences.
//===----------------------------------------------------------------------===//

def LR_D : LNP64Inst<(outs GPR:$rd), (ins GPR:$rs1), "lr.d", "\t$rd, ($rs1)">;
def SC_D : LNP64Inst<(outs GPR:$rd), (ins GPR:$rs2, GPR:$rs1), "sc.d",
                     "\t$rd, $rs2, ($rs1)">;
def FENCE : LNP64NoOperand<"fence">;
def ISYNC : LNP64RRR<"isync">;
def FUTEX_WAIT : LNP64Signal2<"futex_wait">;
def FUTEX_WAKE : LNP64Signal2<"futex_wake">;

//===----------------------------------------------------------------------===//
// PCR access.
//===----------------------------------------------------------------------===//

def GET_PCR : LNP64PcrGet<"get_pcr">;
def SET_PCR : LNP64PcrSet<"set_pcr">;

//===----------------------------------------------------------------------===//
// System / capability / FDR / path primitives (semantics = v1, re-encoded).
//===----------------------------------------------------------------------===//

def ERRNO_GET : LNP64RuntimeGet<"errno_get">;
def ERRNO_SET : LNP64RuntimeSet<"errno_set">;
def FORK : LNP64RuntimeGet<"fork">;
def WAIT_PID : LNP64RR<"wait_pid">;
def EXEC : LNP64RRR<"exec">;
let isTerminator = 1, isBarrier = 1 in
def EXIT : LNP64RuntimeSet<"exit">;

def ALLOC : LNP64RR<"alloc">;
def ALLOC_EX : LNP64RRR<"alloc_ex">;
def ALLOC_SIZE : LNP64RR<"alloc_size">;
def FREE : LNP64RuntimeSet<"free">;

def OPEN_AT : LNP64Native4<"open_at">;
def CLONE_SPAWN : LNP64RRR<"clone.spawn">;
def THREAD_JOIN : LNP64RRR<"thread_join">;
def OPEN_DIR_DYN : LNP64Status3<"open_dir_dyn">;
def MKDIR_PATH_AT : LNP64Status3<"mkdir_path_at">;
def UNLINK_PATH_AT : LNP64Status3<"unlink_path_at">;
def RENAME_PATH_AT : LNP64Status4<"rename_path_at">;
def LINK_PATH_AT : LNP64Status5<"link_path_at">;
def SYMLINK_PATH_AT : LNP64Status3<"symlink_path_at">;
def READLINK_PATH_AT : LNP64Status4<"readlink_path_at">;
def CHDIR_PATH : LNP64Status1<"chdir_path">;
def GETCWD_PATH : LNP64Status2<"getcwd_path">;
def READDIR_FD_DYN : LNP64Status2<"readdir_fd_dyn">;
def CHMOD_PATH_AT : LNP64Status4<"chmod_path_at">;
def CHOWN_PATH_AT : LNP64Status5<"chown_path_at">;
def STAT_PATH_AT : LNP64Status4<"stat_path_at">;
def STAT_FD_DYN : LNP64Status2<"stat_fd_dyn">;
def UTIME_PATH_AT : LNP64Status4<"utime_path_at">;
def UTIME_FD_DYN : LNP64Status2<"utime_fd_dyn">;
def FCNTL_FD_DYN : LNP64Status3<"fcntl_fd_dyn">;
def FD_SEEK_DYN : LNP64Status3<"fd_seek_dyn">;
def PULL : LNP64Native4<"pull"> {
  let Pattern = [(set GPR:$rd, (LNP64pull GPR:$cap, GPR:$arg0, GPR:$arg1))];
}
def PUSH : LNP64Native4<"push"> {
  let Pattern = [(set GPR:$rd, (LNP64push GPR:$cap, GPR:$arg0, GPR:$arg1))];
}
def AWAIT : LNP64Native4<"await"> {
  let Pattern = [(set GPR:$rd, (LNP64await GPR:$cap, GPR:$arg0, GPR:$arg1))];
}
def MMAP : LNP64Inst<(outs GPR:$rd),
                     (ins GPR:$addr, GPR:$len, GPR:$prot, GPR:$fd, GPR:$off),
                     "mmap", "\t$rd, $addr, $len, $prot, $fd, $off">;
def MUNMAP : LNP64Native2<"munmap">;
def MPROTECT : LNP64Inst<(outs GPR:$rd), (ins GPR:$addr, GPR:$len, GPR:$prot),
                         "mprotect", "\t$rd, $addr, $len, $prot">;
def SIGACTION : LNP64Signal2<"sigaction">;
def SIGMASK_SET : LNP64Signal1<"sigmask_set">;
def LNP64_KILL : LNP64Signal2<"kill">;
def SIGRET : LNP64NoOperand<"sigret">;
def ALARM : LNP64Native2<"alarm">;
def ENV_GET : LNP64Inst<(outs GPR:$rd), (ins GPR:$key, GPR:$index, GPR:$len),
                        "env_get", "\t$rd, $key, $index, $len">;
def OBJECT_CTL : LNP64Native2<"object_ctl"> {
  let Pattern = [(set GPR:$rd, (LNP64objectctl GPR:$arg))];
}
def DOMAIN_CTL : LNP64Native2<"domain_ctl"> {
  let Pattern = [(set GPR:$rd, (LNP64domainctl GPR:$arg))];
}
def CAP_SEND : LNP64Native2<"cap_send">;
def CAP_RECV : LNP64Native2<"cap_recv">;
def CAP_DUP : LNP64Native2<"cap_dup">;
def CAP_REVOKE : LNP64Native2<"cap_revoke">;
def GATE_CALL : LNP64Native4<"gate_call"> {
  let Pattern = [(set GPR:$rd, (LNP64gatecall GPR:$cap, GPR:$arg0, GPR:$arg1))];
}
def GATE_RETURN : LNP64Native4<"gate_return"> {
  let Pattern = [(set GPR:$rd, (LNP64gatereturn GPR:$cap, GPR:$arg0, GPR:$arg1))];
}

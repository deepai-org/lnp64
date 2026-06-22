//===-- LNP64InstrInfo.td - LNP64 v2 instruction definitions -------------===//
//
// v2 ISA: fixed 64-bit instructions, one major opcode in [63:56].
//   rd[55:51] rs1[50:46] rs2[45:41] rs3[40:36] rs4[35:31] rs5[30:26]
//   I-type imm32 [45:14]; S/B-type imm32 [40:9]; U/J-type imm32 [50:19].
//   Branch/jump target = pc + (sext32(imm) << 3).
//
// The ENCODER, DISASSEMBLER and INSTPRINTER are all TableGen-generated from the
// `field bits<64> Inst` and AsmString on each instruction (opcode in Inst{63-56},
// operands bound to their bit fields by the format base classes below). mov/li/
// ret are InstAliases of the canonical addi/jalr encodings, so the generated
// disassembler+printer round-trip them through one decodable instruction per
// opcode. Only the AsmParser remains hand-written (custom off(base) lexing).
//===----------------------------------------------------------------------===//

// AsmOperandClasses for the generated AsmMatcher. Registers (GPR/PCR) get their
// operand classes auto-generated from the RegisterClass; these cover the
// immediate and pc-relative-target operands.
def SImm32AsmOperand : AsmOperandClass {
  let Name = "SImm32";
  let RenderMethod = "addImmOperands";
  let PredicateMethod = "isSImm32";
}
// Branch/jump/call/auipc targets accept a concrete offset or a symbol ref
// (the encoder emits the appropriate fixup).
def TargetAsmOperand : AsmOperandClass {
  let Name = "Target";
  let RenderMethod = "addImmOperands";
  let PredicateMethod = "isImm";
}

def simm32 : Operand<i64> {
  let DecoderMethod = "decodeSImm32";
  let ParserMatchClass = SImm32AsmOperand;
}
def brtarget : Operand<OtherVT> {
  let EncoderMethod = "getBranchTargetOpValue";
  let DecoderMethod = "decodeShiftedTarget";   // field holds offset>>3 (signed)
  let ParserMatchClass = TargetAsmOperand;
}
def jmptarget : Operand<OtherVT> {
  let EncoderMethod = "getJumpTargetOpValue";
  let DecoderMethod = "decodeShiftedTarget";
  let ParserMatchClass = TargetAsmOperand;
}
def calltarget : Operand<i64> {
  let EncoderMethod = "getJumpTargetOpValue";
  let DecoderMethod = "decodeShiftedTarget";
  let ParserMatchClass = TargetAsmOperand;
}
def auipctarget : Operand<i64> {
  let EncoderMethod = "getAUIPCTargetOpValue";
  let DecoderMethod = "decodeSImm32";          // signed 32-bit, no shift
  let ParserMatchClass = TargetAsmOperand;
}

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

class LNP64Inst<dag Outs, dag Ins, string Mnemonic, string Operands>
    : Instruction {
  let Namespace = "LNP64";
  let Size = 8;
  let OutOperandList = Outs;
  let InOperandList = Ins;
  let AsmString = !strconcat(Mnemonic, Operands);

  // 8-bit major opcode in Inst{63-56}. Subclasses set this.
  bits<8> Opcode = 0;
  field bits<64> Inst;
  let Inst{63-56} = Opcode;
  // Reserved low bits default to 0; format classes overlay register/imm slices.
  // Required by the -gen-disassembler backend: no soft-fail bits in this ISA.
  field bits<64> SoftFail = 0;
}

// Register field shifts (must match the v2 ISA layout):
//   rd[55:51] rs1[50:46] rs2[45:41] rs3[40:36] rs4[35:31] rs5[30:26]
//   I-type imm32 [45:14]; S/B-type imm32 [40:9]; U/J-type imm32 [50:19].

class LNP64NoOperand<string Mnemonic>
    : LNP64Inst<(outs), (ins), Mnemonic, "">;

class LNP64RR<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$rs), Mnemonic, "\t$rd, $rs"> {
  bits<5> rd;
  bits<5> rs;
  let Inst{55-51} = rd;
  let Inst{50-46} = rs;
}

class LNP64RRR<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$rs1, GPR:$rs2), Mnemonic,
                "\t$rd, $rs1, $rs2"> {
  bits<5> rd;
  bits<5> rs1;
  bits<5> rs2;
  let Inst{55-51} = rd;
  let Inst{50-46} = rs1;
  let Inst{45-41} = rs2;
}

// I-type register-immediate. The immediate is now a full signed-32.
class LNP64RRI<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$rs, simm32:$imm), Mnemonic,
                "\t$rd, $rs, $imm"> {
  bits<5> rd;
  bits<5> rs;
  bits<32> imm;
  let Inst{55-51} = rd;
  let Inst{50-46} = rs;
  let Inst{45-14} = imm;
}

class LNP64Branch<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$rs1, GPR:$rs2, brtarget:$target), Mnemonic,
                "\t$rs1, $rs2, $target"> {
  bits<5> rs1;
  bits<5> rs2;
  bits<32> target;
  let Inst{50-46} = rs1;
  let Inst{45-41} = rs2;
  let Inst{40-9} = target;
  let isBranch = 1;
  let isTerminator = 1;
}

// I-type load: rd, imm32(base).
class LNP64MemLoad<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$base, simm32:$offset), Mnemonic,
                "\t$rd, ${offset}(${base})"> {
  bits<5> rd;
  bits<5> base;
  bits<32> offset;
  let Inst{55-51} = rd;
  let Inst{50-46} = base;
  let Inst{45-14} = offset;
}

// S-type store: rs(src), imm32(base). base->rs1 slot, src->rs2 slot.
class LNP64MemStore<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$rs, GPR:$base, simm32:$offset), Mnemonic,
                "\t$rs, ${offset}(${base})"> {
  bits<5> rs;
  bits<5> base;
  bits<32> offset;
  let Inst{50-46} = base;
  let Inst{45-41} = rs;
  let Inst{40-9} = offset;
}

// Native multi-register forms (syscall/capability/path primitives). These keep
// their v1 semantics, re-encoded as one 64-bit R-type word.
class LNP64Native4<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$cap, GPR:$arg0, GPR:$arg1),
                Mnemonic, "\t$rd, $cap, $arg0, $arg1"> {
  bits<5> rd;
  bits<5> cap;
  bits<5> arg0;
  bits<5> arg1;
  let Inst{55-51} = rd;
  let Inst{50-46} = cap;
  let Inst{45-41} = arg0;
  let Inst{40-36} = arg1;
}
class LNP64Native2<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$arg), Mnemonic, "\t$rd, $arg"> {
  bits<5> rd;
  bits<5> arg;
  let Inst{55-51} = rd;
  let Inst{50-46} = arg;
}
class LNP64Status5<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$arg0, GPR:$arg1, GPR:$arg2, GPR:$arg3,
                         GPR:$arg4),
                Mnemonic, "\t$arg0, $arg1, $arg2, $arg3, $arg4"> {
  bits<5> arg0;
  bits<5> arg1;
  bits<5> arg2;
  bits<5> arg3;
  bits<5> arg4;
  let Inst{55-51} = arg0;
  let Inst{50-46} = arg1;
  let Inst{45-41} = arg2;
  let Inst{40-36} = arg3;
  let Inst{35-31} = arg4;
}
class LNP64Status4<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$arg0, GPR:$arg1, GPR:$arg2, GPR:$arg3),
                Mnemonic, "\t$arg0, $arg1, $arg2, $arg3"> {
  bits<5> arg0;
  bits<5> arg1;
  bits<5> arg2;
  bits<5> arg3;
  let Inst{55-51} = arg0;
  let Inst{50-46} = arg1;
  let Inst{45-41} = arg2;
  let Inst{40-36} = arg3;
}
class LNP64Status3<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$arg0, GPR:$arg1, GPR:$arg2),
                Mnemonic, "\t$arg0, $arg1, $arg2"> {
  bits<5> arg0;
  bits<5> arg1;
  bits<5> arg2;
  let Inst{55-51} = arg0;
  let Inst{50-46} = arg1;
  let Inst{45-41} = arg2;
}
class LNP64Status2<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$arg0, GPR:$arg1), Mnemonic,
                "\t$arg0, $arg1"> {
  bits<5> arg0;
  bits<5> arg1;
  let Inst{55-51} = arg0;
  let Inst{50-46} = arg1;
}
class LNP64Status1<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$arg0), Mnemonic, "\t$arg0"> {
  bits<5> arg0;
  let Inst{55-51} = arg0;
}

class LNP64RuntimeGet<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins), Mnemonic, "\t$rd"> {
  bits<5> rd;
  let Inst{55-51} = rd;
}
class LNP64RuntimeSet<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$rs), Mnemonic, "\t$rs"> {
  bits<5> rs;
  let Inst{55-51} = rs;
}
class LNP64PcrGet<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins PCR:$pcr), Mnemonic, "\t$rd, $pcr"> {
  bits<5> rd;
  bits<5> pcr;
  let Inst{55-51} = rd;
  let Inst{50-46} = pcr;
}
class LNP64PcrSet<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins PCR:$pcr, GPR:$rs), Mnemonic,
                "\t$rd, $pcr, $rs"> {
  bits<5> rd;
  bits<5> pcr;
  bits<5> rs;
  let Inst{55-51} = rd;
  let Inst{50-46} = pcr;
  let Inst{45-41} = rs;
}
class LNP64Signal2<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$rs1, GPR:$rs2), Mnemonic, "\t$rs1, $rs2"> {
  bits<5> rs1;
  bits<5> rs2;
  let Inst{55-51} = rs1;
  let Inst{50-46} = rs2;
}
class LNP64Signal1<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$rs), Mnemonic, "\t$rs"> {
  bits<5> rs;
  let Inst{55-51} = rs;
}

//===----------------------------------------------------------------------===//
// Constants and moves.
//===----------------------------------------------------------------------===//

def NOP : LNP64NoOperand<"nop"> { let Opcode = 0x00; }
def YIELD : LNP64NoOperand<"yield"> { let Opcode = 0x06; }

// li rd, imm32 and mov rd, rs are codegen-internal spellings of the addi (0xa0)
// encoding (li = addi rd, r0, imm; mov = addi rd, rs, 0). They are NOT distinct
// decodable instructions -- addi is the one canonical form at 0xa0. Marking
// them isCodeGenOnly keeps them selectable/rematerializable by codegen while
// excluding them from the (generated) disassembler and asm-matcher, so each
// binary opcode decodes to exactly one instruction. The asm spellings "li"/
// "mov" are provided as InstAliases of addi (see below).
def LI : LNP64Inst<(outs GPR:$rd), (ins simm32:$imm), "li", "\t$rd, $imm"> {
  bits<5> rd;
  bits<32> imm;
  let Opcode = 0xa0;        // addi rd, r0, imm
  let Inst{55-51} = rd;
  let Inst{45-14} = imm;
  let Pattern = [(set GPR:$rd, simm32_imm:$imm)];
  // Constant with no register inputs: rematerialize instead of spill/reload.
  let isReMaterializable = 1;
  let isAsCheapAsAMove = 1;
  let isCodeGenOnly = 1;
}
def MOV : LNP64RR<"mov"> { let Opcode = 0xa0; let isCodeGenOnly = 1; }  // addi rd, rs, 0
def AUIPC : LNP64Inst<(outs GPR:$rd), (ins auipctarget:$target), "auipc",
                      "\t$rd, $target"> {
  bits<5> rd;
  bits<32> target;
  let Opcode = 0xd0;
  let Inst{55-51} = rd;
  let Inst{50-19} = target;  // U-type imm32 [50:19]
  // PC-relative address with no register inputs: rematerializable.
  let isReMaterializable = 1;
  let isAsCheapAsAMove = 1;
}
def LIU : LNP64RRI<"liu"> { let Opcode = 0x04; }
def : Pat<(i64 (LNP64wrapper tglobaladdr:$target)), (AUIPC tglobaladdr:$target)>;

// 64-bit constants: li lo32 ; liu rd, rd, hi32.
def PseudoLI64
    : LNP64Inst<(outs GPR:$rd), (ins i64imm:$imm), "pseudo_li64",
                "\t$rd, $imm"> {
  let isPseudo = 1;
  let usesCustomInserter = 1;
}
def : Pat<(i64 wide64_imm:$imm), (PseudoLI64 wide64_imm:$imm)>;

//===----------------------------------------------------------------------===//
// Fused compare-and-select: rd = (ra <cc> rb) ? rt : rf. A branch-free Class-A
// datapath mux, one opcode per condition (mirrors the branch family beq..bgeu).
// This is the hardware target for ISD::SELECT_CC; plain ISD::SELECT lowers to
// `selectcc cond, 0, t, f, NE` (LowerSELECT) and selects to sel.ne.
//===----------------------------------------------------------------------===//
class LNP64Select<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$ra, GPR:$rb, GPR:$rt, GPR:$rf),
                Mnemonic, "\t$rd, $ra, $rb, $rt, $rf"> {
  bits<5> rd;
  bits<5> ra;
  bits<5> rb;
  bits<5> rt;
  bits<5> rf;
  let Inst{55-51} = rd;
  let Inst{50-46} = ra;
  let Inst{45-41} = rb;
  let Inst{40-36} = rt;
  let Inst{35-31} = rf;
}
def SEL_EQ  : LNP64Select<"sel.eq">  { let Opcode = 0x40; }
def SEL_NE  : LNP64Select<"sel.ne">  { let Opcode = 0x41; }
def SEL_LT  : LNP64Select<"sel.lt">  { let Opcode = 0x42; }
def SEL_GE  : LNP64Select<"sel.ge">  { let Opcode = 0x43; }
def SEL_LTU : LNP64Select<"sel.ltu"> { let Opcode = 0x44; }
def SEL_GEU : LNP64Select<"sel.geu"> { let Opcode = 0x45; }

class SelPat<CondCode cc, LNP64Select inst>
    : Pat<(selectcc GPR:$a, GPR:$b, GPR:$t, GPR:$f, cc),
          (inst GPR:$a, GPR:$b, GPR:$t, GPR:$f)>;
def : SelPat<SETEQ,  SEL_EQ>;
def : SelPat<SETNE,  SEL_NE>;
def : SelPat<SETLT,  SEL_LT>;
def : SelPat<SETGE,  SEL_GE>;
def : SelPat<SETULT, SEL_LTU>;
def : SelPat<SETUGE, SEL_GEU>;
// Conditions without a native opcode: swap the compared operands.
class SelPatSwap<CondCode cc, LNP64Select inst>
    : Pat<(selectcc GPR:$a, GPR:$b, GPR:$t, GPR:$f, cc),
          (inst GPR:$b, GPR:$a, GPR:$t, GPR:$f)>;
def : SelPatSwap<SETGT,  SEL_LT>;
def : SelPatSwap<SETLE,  SEL_GE>;
def : SelPatSwap<SETUGT, SEL_LTU>;
def : SelPatSwap<SETULE, SEL_GEU>;

//===----------------------------------------------------------------------===//
// Integer ALU (R-type).
//===----------------------------------------------------------------------===//

def ADD : LNP64RRR<"add"> { let Opcode = 0x10; let Pattern = [(set GPR:$rd, (add GPR:$rs1, GPR:$rs2))]; }
def SUB : LNP64RRR<"sub"> { let Opcode = 0x11; let Pattern = [(set GPR:$rd, (sub GPR:$rs1, GPR:$rs2))]; }
def MUL : LNP64RRR<"mul"> { let Opcode = 0x12; let Pattern = [(set GPR:$rd, (mul GPR:$rs1, GPR:$rs2))]; }
def DIV : LNP64RRR<"div"> { let Opcode = 0x13; let Pattern = [(set GPR:$rd, (sdiv GPR:$rs1, GPR:$rs2))]; }
def UDIV : LNP64RRR<"udiv"> { let Opcode = 0xa7; let Pattern = [(set GPR:$rd, (udiv GPR:$rs1, GPR:$rs2))]; }
def SREM : LNP64RRR<"srem"> { let Opcode = 0xa8; let Pattern = [(set GPR:$rd, (srem GPR:$rs1, GPR:$rs2))]; }
def UREM : LNP64RRR<"urem"> { let Opcode = 0xa9; let Pattern = [(set GPR:$rd, (urem GPR:$rs1, GPR:$rs2))]; }
def MULH : LNP64RRR<"mulh"> { let Opcode = 0xaa; let Pattern = [(set GPR:$rd, (mulhs GPR:$rs1, GPR:$rs2))]; }
def MULHU : LNP64RRR<"mulhu"> { let Opcode = 0xab; let Pattern = [(set GPR:$rd, (mulhu GPR:$rs1, GPR:$rs2))]; }
def MULHSU : LNP64RRR<"mulhsu"> { let Opcode = 0xac; }
def AND : LNP64RRR<"and"> { let Opcode = 0x14; let Pattern = [(set GPR:$rd, (and GPR:$rs1, GPR:$rs2))]; }
def OR : LNP64RRR<"or"> { let Opcode = 0x15; let Pattern = [(set GPR:$rd, (or GPR:$rs1, GPR:$rs2))]; }
def XOR : LNP64RRR<"xor"> { let Opcode = 0x16; let Pattern = [(set GPR:$rd, (xor GPR:$rs1, GPR:$rs2))]; }
def SLL : LNP64RRR<"sll"> { let Opcode = 0x18; let Pattern = [(set GPR:$rd, (shl GPR:$rs1, GPR:$rs2))]; }
def SRL : LNP64RRR<"srl"> { let Opcode = 0x19; let Pattern = [(set GPR:$rd, (srl GPR:$rs1, GPR:$rs2))]; }
def SRA : LNP64RRR<"sra"> { let Opcode = 0x1a; let Pattern = [(set GPR:$rd, (sra GPR:$rs1, GPR:$rs2))]; }
def ROL : LNP64RRR<"rol"> { let Opcode = 0xb6; let Pattern = [(set GPR:$rd, (rotl GPR:$rs1, GPR:$rs2))]; }
def ROR : LNP64RRR<"ror"> { let Opcode = 0xb7; let Pattern = [(set GPR:$rd, (rotr GPR:$rs1, GPR:$rs2))]; }
def SLT : LNP64RRR<"slt"> { let Opcode = 0x1b; let Pattern = [(set GPR:$rd, (setcc GPR:$rs1, GPR:$rs2, SETLT))]; }
def SLTU : LNP64RRR<"sltu"> { let Opcode = 0x1c; let Pattern = [(set GPR:$rd, (setcc GPR:$rs1, GPR:$rs2, SETULT))]; }

//===----------------------------------------------------------------------===//
// Unary (R-type).
//===----------------------------------------------------------------------===//

def NOT : LNP64RR<"not"> { let Opcode = 0x17; let Pattern = [(set GPR:$rd, (xor GPR:$rs, all_ones_imm))]; }
def SEXT_B : LNP64RR<"sext.b"> { let Opcode = 0xad; let Pattern = [(set GPR:$rd, (sext_inreg GPR:$rs, i8))]; }
def SEXT_H : LNP64RR<"sext.h"> { let Opcode = 0xae; let Pattern = [(set GPR:$rd, (sext_inreg GPR:$rs, i16))]; }
def SEXT_W : LNP64RR<"sext.w"> { let Opcode = 0xaf; let Pattern = [(set GPR:$rd, (sext_inreg GPR:$rs, i32))]; }
let AddedComplexity = 1 in {
def ZEXT_B : LNP64RR<"zext.b"> { let Opcode = 0xb0; let Pattern = [(set GPR:$rd, (and GPR:$rs, u8_mask_i64_imm))]; }
def ZEXT_H : LNP64RR<"zext.h"> { let Opcode = 0xb1; let Pattern = [(set GPR:$rd, (and GPR:$rs, u16_mask_i64_imm))]; }
def ZEXT_W : LNP64RR<"zext.w"> { let Opcode = 0xb2; let Pattern = [(set GPR:$rd, (and GPR:$rs, u32_mask_i64_imm))]; }
}
def CLZ : LNP64RR<"clz"> { let Opcode = 0xb3; let Pattern = [(set GPR:$rd, (ctlz GPR:$rs))]; }
def : Pat<(i64 (ctlz_zero_undef GPR:$rs)), (CLZ GPR:$rs)>;
def CTZ : LNP64RR<"ctz"> { let Opcode = 0xb4; let Pattern = [(set GPR:$rd, (cttz GPR:$rs))]; }
def : Pat<(i64 (cttz_zero_undef GPR:$rs)), (CTZ GPR:$rs)>;
def POPCNT : LNP64RR<"popcnt"> { let Opcode = 0xb5; let Pattern = [(set GPR:$rd, (ctpop GPR:$rs))]; }
def BSWAP16 : LNP64RR<"bswap16"> { let Opcode = 0xb8; }
def BSWAP32 : LNP64RR<"bswap32"> { let Opcode = 0xb9; }
def BSWAP64 : LNP64RR<"bswap64"> { let Opcode = 0xba; let Pattern = [(set GPR:$rd, (bswap GPR:$rs))]; }

//===----------------------------------------------------------------------===//
// Register-immediate (I-type).
//===----------------------------------------------------------------------===//

def ADDI : LNP64RRI<"addi"> { let Opcode = 0xa0; let Pattern = [(set GPR:$rd, (add GPR:$rs, simm32_i64_imm:$imm))]; }
def ANDI : LNP64RRI<"andi"> { let Opcode = 0xa1; let Pattern = [(set GPR:$rd, (and GPR:$rs, simm32_i64_imm:$imm))]; }
def ORI : LNP64RRI<"ori"> { let Opcode = 0xa2; let Pattern = [(set GPR:$rd, (or GPR:$rs, simm32_i64_imm:$imm))]; }
def XORI : LNP64RRI<"xori"> { let Opcode = 0xa3; let Pattern = [(set GPR:$rd, (xor GPR:$rs, simm32_i64_imm:$imm))]; }
def SLLI : LNP64RRI<"slli"> { let Opcode = 0xa4; let Pattern = [(set GPR:$rd, (shl GPR:$rs, simm32_i64_imm:$imm))]; }
def SRLI : LNP64RRI<"srli"> { let Opcode = 0xa5; let Pattern = [(set GPR:$rd, (srl GPR:$rs, simm32_i64_imm:$imm))]; }
def SRAI : LNP64RRI<"srai"> { let Opcode = 0xa6; let Pattern = [(set GPR:$rd, (sra GPR:$rs, simm32_i64_imm:$imm))]; }
def SLTI : LNP64RRI<"slti"> { let Opcode = 0x1d; let Pattern = [(set GPR:$rd, (setcc GPR:$rs, simm32_i64_imm:$imm, SETLT))]; }
def SLTIU : LNP64RRI<"sltiu"> { let Opcode = 0x1e; let Pattern = [(set GPR:$rd, (setcc GPR:$rs, simm32_i64_imm:$imm, SETULT))]; }

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
def JMP : LNP64Inst<(outs), (ins jmptarget:$target), "jmp", "\t$target"> {
  bits<32> target;
  let Opcode = 0x20;
  let Inst{50-19} = target;  // U/J-type imm32 [50:19]
  let Pattern = [(br bb:$target)];
}

def BEQ : LNP64Branch<"beq"> { let Opcode = 0x21; }
def BNE : LNP64Branch<"bne"> { let Opcode = 0x22; }
def BLT : LNP64Branch<"blt"> { let Opcode = 0x23; }
def BGE : LNP64Branch<"bge"> { let Opcode = 0x24; }
def BLTU : LNP64Branch<"bltu"> { let Opcode = 0x25; }
def BGEU : LNP64Branch<"bgeu"> { let Opcode = 0x26; }

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
// A call clobbers the caller-saved GPRs only: r1 (ra) and the temporaries
// r2..r17, r28, r29, r30. The callee-saved set s0..s9 = r18..r27 is preserved
// and must NOT appear here (listing it would force PEI to save all of r18..r27
// on every call and prevent the allocator from keeping cross-call values in
// s-registers). The precise clobber set is also carried by the call-preserved
// regmask (getCallPreservedMask / CSR_LNP64) attached in LowerCall.
let isCall = 1,
    Defs = [R1, R2, R3, R4, R5, R6, R7, R8, R9, R10, R11, R12, R13, R14,
            R15, R16, R17, R28, R29, R30] in {
def JAL : LNP64Inst<(outs GPR:$rd), (ins calltarget:$target), "jal",
                    "\t$rd, $target"> {
  bits<5> rd;
  bits<32> target;
  let Opcode = 0x27;
  let Inst{55-51} = rd;
  let Inst{50-19} = target;  // U/J-type imm32 [50:19]
}
def JALR : LNP64Inst<(outs GPR:$rd), (ins GPR:$rs1, simm32:$imm), "jalr",
                     "\t$rd, $rs1, $imm"> {
  bits<5> rd;
  bits<5> rs1;
  bits<32> imm;
  let Opcode = 0x28;
  let Inst{55-51} = rd;
  let Inst{50-46} = rs1;
  let Inst{45-14} = imm;  // I-type imm32 [45:14]
}
// Codegen call pseudos: rd is implicitly r1 (ra). Emitter expands them to
// jal r1, target / jalr r1, target, 0.
def PseudoCALL
    : LNP64Inst<(outs), (ins calltarget:$target), "jal", "\tr1, $target"> {
  bits<32> target;
  let Opcode = 0x27;
  let Inst{55-51} = 1;       // rd = r1 (ra)
  let Inst{50-19} = target;
  let isCodeGenOnly = 1;     // codegen call form of jal r1, target
}
def PseudoCALLIndirect
    : LNP64Inst<(outs), (ins GPR:$target), "jalr", "\tr1, $target, 0"> {
  bits<5> target;
  let Opcode = 0x28;
  let Inst{55-51} = 1;       // rd = r1 (ra)
  let isCodeGenOnly = 1;     // codegen call form of jalr r1, rs, 0
  let Inst{50-46} = target;  // rs1 = base; imm = 0
}
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

// ret = jalr r0, r1, 0. Codegen-internal return form of jalr (one canonical
// decodable instruction at 0x28). The asm spelling "ret" is an InstAlias of
// jalr (see below).
let isReturn = 1, isTerminator = 1, isBarrier = 1, Uses = [R1] in
def RET : LNP64NoOperand<"ret"> {
  let Opcode = 0x28;       // jalr r0, r1, 0
  let Inst{50-46} = 1;     // rs1 = r1
  let Pattern = [(LNP64retflag)];
  let isCodeGenOnly = 1;
}

//===----------------------------------------------------------------------===//
// Loads / stores.
//===----------------------------------------------------------------------===//

def LD : LNP64MemLoad<"ld"> { let Opcode = 0x30; }
def LWU : LNP64MemLoad<"lwu"> { let Opcode = 0x31; }
def LBU : LNP64MemLoad<"lbu"> { let Opcode = 0x32; }
def LHU : LNP64MemLoad<"lhu"> { let Opcode = 0x36; }
def LW : LNP64MemLoad<"lw"> { let Opcode = 0x05; }
def LB : LNP64MemLoad<"lb"> { let Opcode = 0x08; }
def LH : LNP64MemLoad<"lh"> { let Opcode = 0x09; }
def SD : LNP64MemStore<"sd"> { let Opcode = 0x33; }
def SW : LNP64MemStore<"sw"> { let Opcode = 0x34; }
def SB : LNP64MemStore<"sb"> { let Opcode = 0x35; }
def SH : LNP64MemStore<"sh"> { let Opcode = 0x37; }

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

def LR_D : LNP64Inst<(outs GPR:$rd), (ins GPR:$rs1), "lr.d", "\t$rd, (${rs1})"> {
  bits<5> rd;
  bits<5> rs1;
  let Opcode = 0xc5;
  let Inst{55-51} = rd;
  let Inst{50-46} = rs1;
}
def SC_D : LNP64Inst<(outs GPR:$rd), (ins GPR:$rs2, GPR:$rs1), "sc.d",
                     "\t$rd, $rs2, (${rs1})"> {
  // sc.d rd, rs2, (rs1): rd in rd slot, rs1(base) in rs1 slot, rs2(src) in rs2.
  bits<5> rd;
  bits<5> rs2;
  bits<5> rs1;
  let Opcode = 0xc6;
  let Inst{55-51} = rd;
  let Inst{50-46} = rs1;
  let Inst{45-41} = rs2;
}
def FENCE : LNP64NoOperand<"fence"> { let Opcode = 0xcd; }
def ISYNC : LNP64RRR<"isync"> { let Opcode = 0xce; }
def FUTEX_WAIT : LNP64Signal2<"futex_wait"> { let Opcode = 0xcb; }
def FUTEX_WAKE : LNP64Signal2<"futex_wake"> { let Opcode = 0xcc; }

//===----------------------------------------------------------------------===//
// PCR access.
//===----------------------------------------------------------------------===//

def GET_PCR : LNP64PcrGet<"get_pcr"> { let Opcode = 0x54; }
def SET_PCR : LNP64PcrSet<"set_pcr"> { let Opcode = 0x55; }

//===----------------------------------------------------------------------===//
// System / capability / FDR / path primitives (semantics = v1, re-encoded).
//===----------------------------------------------------------------------===//

def ERRNO_GET : LNP64RuntimeGet<"errno_get"> { let Opcode = 0x38; }
def ERRNO_SET : LNP64RuntimeSet<"errno_set"> { let Opcode = 0x39; }
def FORK : LNP64RuntimeGet<"fork"> { let Opcode = 0x7d; }
def WAIT_PID : LNP64RR<"wait_pid"> { let Opcode = 0x7e; }
def EXEC : LNP64RRR<"exec"> { let Opcode = 0x7f; }
let isTerminator = 1, isBarrier = 1 in
def EXIT : LNP64RuntimeSet<"exit"> { let Opcode = 0x3a; }

def ALLOC : LNP64RR<"alloc"> { let Opcode = 0x47; }
def ALLOC_EX : LNP64RRR<"alloc_ex"> { let Opcode = 0x4a; }
def ALLOC_SIZE : LNP64RR<"alloc_size"> { let Opcode = 0x48; }
def FREE : LNP64RuntimeSet<"free"> { let Opcode = 0x49; }

def OPEN_AT : LNP64Native4<"open_at"> { let Opcode = 0x58; }
def CLONE_SPAWN : LNP64RRR<"clone.spawn"> { let Opcode = 0x59; }
def THREAD_JOIN : LNP64RRR<"thread_join"> { let Opcode = 0x5a; }
def OPEN_DIR_DYN : LNP64Status3<"open_dir_dyn"> { let Opcode = 0x73; }
def MKDIR_PATH_AT : LNP64Status3<"mkdir_path_at"> { let Opcode = 0x74; }
def UNLINK_PATH_AT : LNP64Status3<"unlink_path_at"> { let Opcode = 0x6b; }
def RENAME_PATH_AT : LNP64Status4<"rename_path_at"> { let Opcode = 0x75; }
def LINK_PATH_AT : LNP64Status5<"link_path_at"> { let Opcode = 0x76; }
def SYMLINK_PATH_AT : LNP64Status3<"symlink_path_at"> { let Opcode = 0x77; }
def READLINK_PATH_AT : LNP64Status4<"readlink_path_at"> { let Opcode = 0x78; }
def CHDIR_PATH : LNP64Status1<"chdir_path"> { let Opcode = 0x79; }
def GETCWD_PATH : LNP64Status2<"getcwd_path"> { let Opcode = 0x7a; }
def READDIR_FD_DYN : LNP64Status2<"readdir_fd_dyn"> { let Opcode = 0xcf; }
def CHMOD_PATH_AT : LNP64Status4<"chmod_path_at"> { let Opcode = 0x7b; }
def CHOWN_PATH_AT : LNP64Status5<"chown_path_at"> { let Opcode = 0x7c; }
def STAT_PATH_AT : LNP64Status4<"stat_path_at"> { let Opcode = 0x5c; }
def STAT_FD_DYN : LNP64Status2<"stat_fd_dyn"> { let Opcode = 0x5d; }
def UTIME_PATH_AT : LNP64Status4<"utime_path_at"> { let Opcode = 0x5e; }
def UTIME_FD_DYN : LNP64Status2<"utime_fd_dyn"> { let Opcode = 0x5f; }
def FCNTL_FD_DYN : LNP64Status3<"fcntl_fd_dyn"> { let Opcode = 0x67; }
def FD_SEEK_DYN : LNP64Status3<"fd_seek_dyn"> { let Opcode = 0x69; }
def PULL : LNP64Native4<"pull"> {
  let Opcode = 0x2b;
  let Pattern = [(set GPR:$rd, (LNP64pull GPR:$cap, GPR:$arg0, GPR:$arg1))];
}
def PUSH : LNP64Native4<"push"> {
  let Opcode = 0x2c;
  let Pattern = [(set GPR:$rd, (LNP64push GPR:$cap, GPR:$arg0, GPR:$arg1))];
}
def AWAIT : LNP64Native4<"await"> {
  let Opcode = 0x4d;
  let Pattern = [(set GPR:$rd, (LNP64await GPR:$cap, GPR:$arg0, GPR:$arg1))];
}
def MMAP : LNP64Inst<(outs GPR:$rd),
                     (ins GPR:$addr, GPR:$len, GPR:$prot, GPR:$fd, GPR:$off),
                     "mmap", "\t$rd, $addr, $len, $prot, $fd, $off"> {
  bits<5> rd; bits<5> addr; bits<5> len; bits<5> prot; bits<5> fd; bits<5> off;
  let Opcode = 0x6a;
  let Inst{55-51} = rd;
  let Inst{50-46} = addr;
  let Inst{45-41} = len;
  let Inst{40-36} = prot;
  let Inst{35-31} = fd;
  let Inst{30-26} = off;
}
def MUNMAP : LNP64Native2<"munmap"> { let Opcode = 0x61; }
def MPROTECT : LNP64Inst<(outs GPR:$rd), (ins GPR:$addr, GPR:$len, GPR:$prot),
                         "mprotect", "\t$rd, $addr, $len, $prot"> {
  bits<5> rd; bits<5> addr; bits<5> len; bits<5> prot;
  let Opcode = 0x6c;
  let Inst{55-51} = rd;
  let Inst{50-46} = addr;
  let Inst{45-41} = len;
  let Inst{40-36} = prot;
}
def MMAP_BOOTSTRAP : LNP64Inst<(outs GPR:$rd),
                               (ins GPR:$addr, GPR:$len, GPR:$prot),
                               "mmap_bootstrap", "\t$rd, $addr, $len, $prot"> {
  bits<5> rd; bits<5> addr; bits<5> len; bits<5> prot;
  let Opcode = 0x60;
  let Inst{55-51} = rd;
  let Inst{50-46} = addr;
  let Inst{45-41} = len;
  let Inst{40-36} = prot;
}
def MPROTECT_BOOTSTRAP : LNP64Inst<(outs GPR:$rd),
                                   (ins GPR:$addr, GPR:$len, GPR:$prot),
                                   "mprotect_bootstrap",
                                   "\t$rd, $addr, $len, $prot"> {
  bits<5> rd; bits<5> addr; bits<5> len; bits<5> prot;
  let Opcode = 0x66;
  let Inst{55-51} = rd;
  let Inst{50-46} = addr;
  let Inst{45-41} = len;
  let Inst{40-36} = prot;
}
def SIGACTION : LNP64Signal2<"sigaction"> { let Opcode = 0x62; }
def SIGMASK_SET : LNP64Signal1<"sigmask_set"> { let Opcode = 0x63; }
def LNP64_KILL : LNP64Signal2<"kill"> { let Opcode = 0x64; }
def SIGRET : LNP64NoOperand<"sigret"> { let Opcode = 0x65; }
def ALARM : LNP64Native2<"alarm"> { let Opcode = 0x68; }
def ENV_GET : LNP64Inst<(outs GPR:$rd), (ins GPR:$key, GPR:$index, GPR:$len),
                        "env_get", "\t$rd, $key, $index, $len"> {
  bits<5> rd; bits<5> key; bits<5> index; bits<5> len;
  let Opcode = 0x56;
  let Inst{55-51} = rd;
  let Inst{50-46} = key;
  let Inst{45-41} = index;
  let Inst{40-36} = len;
}
def OBJECT_CTL : LNP64Native2<"object_ctl"> {
  let Opcode = 0x4b;
  let Pattern = [(set GPR:$rd, (LNP64objectctl GPR:$arg))];
}
def DOMAIN_CTL : LNP64Native2<"domain_ctl"> {
  let Opcode = 0x4c;
  let Pattern = [(set GPR:$rd, (LNP64domainctl GPR:$arg))];
}
def CAP_SEND : LNP64Native2<"cap_send"> { let Opcode = 0x51; }
def CAP_RECV : LNP64Native2<"cap_recv"> { let Opcode = 0x52; }
def CAP_DUP : LNP64Native2<"cap_dup"> { let Opcode = 0x50; }
def CAP_REVOKE : LNP64Native2<"cap_revoke"> { let Opcode = 0x53; }
def GATE_CALL : LNP64Native4<"gate_call"> {
  let Opcode = 0x4e;
  let Pattern = [(set GPR:$rd, (LNP64gatecall GPR:$cap, GPR:$arg0, GPR:$arg1))];
}
def GATE_RETURN : LNP64Native4<"gate_return"> {
  let Opcode = 0x4f;
  let Pattern = [(set GPR:$rd, (LNP64gatereturn GPR:$cap, GPR:$arg0, GPR:$arg1))];
}

//===----------------------------------------------------------------------===//
// Assembler aliases for the canonical encodings.
//
// mov/li/ret are the addi (0xa0) and jalr (0x28) encodings with fixed operands.
// The codegen-internal LI/MOV/RET defs (isCodeGenOnly) carry these for
// instruction selection; these InstAliases give the (generated) assembler and
// disassembler the human spellings, so `addi rd,rs,0` prints as `mov rd, rs`,
// `addi rd,r0,imm` as `li rd, imm`, and `jalr r0,r1,0` as `ret`.
//===----------------------------------------------------------------------===//
def : InstAlias<"mov $rd, $rs", (ADDI GPR:$rd, GPR:$rs, 0)>;
def : InstAlias<"li $rd, $imm",  (ADDI GPR:$rd, R0, simm32:$imm)>;
def : InstAlias<"ret",           (JALR R0, R1, 0)>;

// Ordering-annotated fence spellings are all the single v2 fence (parser-only:
// Emit=0 keeps the printer/disassembler emitting plain "fence").
def : InstAlias<"fence.acq",     (FENCE), 0>;
def : InstAlias<"fence.rel",     (FENCE), 0>;
def : InstAlias<"fence.acq_rel", (FENCE), 0>;
def : InstAlias<"fence.sc",      (FENCE), 0>;

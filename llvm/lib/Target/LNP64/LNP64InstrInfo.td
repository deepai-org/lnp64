def simm16 : Operand<i64>;
def simm14 : Operand<i64>;
def simm26 : Operand<i64>;
def brtarget : Operand<OtherVT>;
def simm16_imm : PatLeaf<(imm), [{
  return isInt<16>(N->getSExtValue());
}]>;
def simm14_imm : PatLeaf<(imm), [{
  return isInt<14>(N->getSExtValue());
}]>;
def uimm32_imm : PatLeaf<(imm), [{
  return !isInt<16>(N->getSExtValue()) && isUInt<32>(N->getZExtValue());
}]>;
def sneg32_imm : PatLeaf<(imm), [{
  int64_t Value = N->getSExtValue();
  return Value < -32768 && isInt<32>(Value);
}]>;
def all_ones_imm : PatLeaf<(imm), [{
  return N->isAllOnes();
}]>;

def LNP64retflag : SDNode<"LNP64ISD::RET_FLAG", SDTNone,
                          [SDNPHasChain, SDNPOptInGlue, SDNPVariadic]>;
def SDT_LNP64Call : SDTypeProfile<0, -1, []>;
def LNP64call : SDNode<"LNP64ISD::CALL", SDT_LNP64Call,
                       [SDNPHasChain, SDNPOptInGlue, SDNPOutGlue,
                        SDNPVariadic]>;
def SDT_LNP64BrCC : SDTypeProfile<0, 3, [
  SDTCisVT<0, i64>, SDTCisVT<1, i64>, SDTCisVT<2, OtherVT>
]>;
def LNP64breq : SDNode<"LNP64ISD::BR_EQ", SDT_LNP64BrCC, [SDNPHasChain]>;
def LNP64brge : SDNode<"LNP64ISD::BR_GE", SDT_LNP64BrCC, [SDNPHasChain]>;
def LNP64brgt : SDNode<"LNP64ISD::BR_GT", SDT_LNP64BrCC, [SDNPHasChain]>;
def LNP64brle : SDNode<"LNP64ISD::BR_LE", SDT_LNP64BrCC, [SDNPHasChain]>;
def LNP64brlt : SDNode<"LNP64ISD::BR_LT", SDT_LNP64BrCC, [SDNPHasChain]>;
def LNP64brne : SDNode<"LNP64ISD::BR_NE", SDT_LNP64BrCC, [SDNPHasChain]>;
def LNP64bruge : SDNode<"LNP64ISD::BR_UGE", SDT_LNP64BrCC, [SDNPHasChain]>;
def LNP64brugt : SDNode<"LNP64ISD::BR_UGT", SDT_LNP64BrCC, [SDNPHasChain]>;
def LNP64brule : SDNode<"LNP64ISD::BR_ULE", SDT_LNP64BrCC, [SDNPHasChain]>;
def LNP64brult : SDNode<"LNP64ISD::BR_ULT", SDT_LNP64BrCC, [SDNPHasChain]>;
def SDT_LNP64Native1 : SDTypeProfile<1, 1, [
  SDTCisVT<0, i64>, SDTCisVT<1, i64>
]>;
def SDT_LNP64Native3 : SDTypeProfile<1, 3, [
  SDTCisVT<0, i64>, SDTCisVT<1, i64>, SDTCisVT<2, i64>, SDTCisVT<3, i64>
]>;
def LNP64domainctl : SDNode<"LNP64ISD::DOMAIN_CTL", SDT_LNP64Native1,
                            [SDNPHasChain]>;
def LNP64gatecall : SDNode<"LNP64ISD::GATE_CALL", SDT_LNP64Native3,
                           [SDNPHasChain]>;
def LNP64objectctl : SDNode<"LNP64ISD::OBJECT_CTL", SDT_LNP64Native1,
                            [SDNPHasChain]>;
def LNP64pull : SDNode<"LNP64ISD::PULL", SDT_LNP64Native3,
                       [SDNPHasChain]>;
def LNP64push : SDNode<"LNP64ISD::PUSH", SDT_LNP64Native3,
                       [SDNPHasChain]>;
def SDT_LNP64Wrapper : SDTypeProfile<1, 1, [
  SDTCisSameAs<0, 1>, SDTCisPtrTy<0>
]>;
def LNP64wrapper : SDNode<"LNP64ISD::WRAPPER", SDT_LNP64Wrapper>;

class LNP64Inst<dag Outs, dag Ins, string Mnemonic, string Operands>
    : Instruction {
  let Namespace = "LNP64";
  let Size = 4;
  let OutOperandList = Outs;
  let InOperandList = Ins;
  let AsmString = !strconcat(Mnemonic, Operands);
}

class LNP64NoOperand<string Mnemonic>
    : LNP64Inst<(outs), (ins), Mnemonic, "">;

class LNP64RI<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins simm16:$imm), Mnemonic,
                "\t$rd, $imm">;

class LNP64ConstPseudo<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins i64imm:$imm), Mnemonic,
                "\t$rd, $imm"> {
  let isPseudo = 1;
  let usesCustomInserter = 1;
}

class LNP64RR<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$rs), Mnemonic, "\t$rd, $rs">;

class LNP64RRR<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$rs1, GPR:$rs2), Mnemonic,
                "\t$rd, $rs1, $rs2">;

class LNP64Cmp<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$rs1, GPR:$rs2), Mnemonic,
                "\t$rs1, $rs2">;

class LNP64Cset<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins), Mnemonic, "\t$rd"> {
  let Uses = [FLAGS];
}

class LNP64SetCCPseudo<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$lhs, GPR:$rhs), Mnemonic,
                "\t$rd, $lhs, $rhs"> {
  let isPseudo = 1;
  let usesCustomInserter = 1;
}

class LNP64SetCCImmPseudo<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$lhs, simm16:$rhs), Mnemonic,
                "\t$rd, $lhs, $rhs"> {
  let isPseudo = 1;
  let usesCustomInserter = 1;
}

class LNP64Branch<string Mnemonic>
    : LNP64Inst<(outs), (ins brtarget:$target), Mnemonic, "\t$target">;

class LNP64CondBranchPseudo<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$lhs, GPR:$rhs, brtarget:$target),
                Mnemonic, "\t$lhs, $rhs, $target"> {
  let isPseudo = 1;
  let isBranch = 1;
  let isTerminator = 1;
  let Uses = [FLAGS];
  let usesCustomInserter = 1;
}

class LNP64MemLoad<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$base, simm14:$offset), Mnemonic,
                "\t$rd, $offset($base)">;

class LNP64SignedLoadPseudo<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$base, simm14:$offset), Mnemonic,
                "\t$rd, $offset($base)"> {
  let isPseudo = 1;
  let usesCustomInserter = 1;
}

class LNP64FrameAddrPseudo<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$base, simm14:$offset), Mnemonic,
                "\t$rd, $offset($base)"> {
  let isPseudo = 1;
}

class LNP64MemStore<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$rs, GPR:$base, simm14:$offset), Mnemonic,
                "\t$rs, $offset($base)">;

class LNP64Native4<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$cap, GPR:$arg0, GPR:$arg1),
                Mnemonic, "\t$rd, $cap, $arg0, $arg1">;

class LNP64Native2<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$arg), Mnemonic, "\t$rd, $arg">;

class LNP64RuntimeGet<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins), Mnemonic, "\t$rd">;

class LNP64RuntimeSet<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$rs), Mnemonic, "\t$rs">;

def NOP : LNP64NoOperand<"nop">;
def LI : LNP64RI<"li"> {
  let Pattern = [(set GPR:$rd, simm16_imm:$imm)];
}
def MOV : LNP64RR<"mov">;
def LA : LNP64Inst<(outs GPR:$rd), (ins i64imm:$target), "la",
                   "\t$rd, $target"> {
  let Size = 8;
}
def LI32 : LNP64Inst<(outs GPR:$rd), (ins i64imm:$imm), "li32",
                     "\t$rd, $imm"> {
  let Size = 8;
  let Pattern = [(set GPR:$rd, uimm32_imm:$imm)];
}
def PseudoLINeg32 : LNP64ConstPseudo<"pseudo_li.neg32">;
def : Pat<(i64 sneg32_imm:$imm), (PseudoLINeg32 sneg32_imm:$imm)>;
def : Pat<(i64 (LNP64wrapper tglobaladdr:$target)), (LA tglobaladdr:$target)>;

def ADD : LNP64RRR<"add"> {
  let Pattern = [(set GPR:$rd, (add GPR:$rs1, GPR:$rs2))];
}
def SUB : LNP64RRR<"sub"> {
  let Pattern = [(set GPR:$rd, (sub GPR:$rs1, GPR:$rs2))];
}
def MUL : LNP64RRR<"mul"> {
  let Pattern = [(set GPR:$rd, (mul GPR:$rs1, GPR:$rs2))];
}
def DIV : LNP64RRR<"div"> {
  let Pattern = [(set GPR:$rd, (sdiv GPR:$rs1, GPR:$rs2))];
}
def AND : LNP64RRR<"and"> {
  let Pattern = [(set GPR:$rd, (and GPR:$rs1, GPR:$rs2))];
}
def OR : LNP64RRR<"or"> {
  let Pattern = [(set GPR:$rd, (or GPR:$rs1, GPR:$rs2))];
}
def XOR : LNP64RRR<"xor"> {
  let Pattern = [(set GPR:$rd, (xor GPR:$rs1, GPR:$rs2))];
}
def NOT : LNP64RR<"not"> {
  let Pattern = [(set GPR:$rd, (xor GPR:$rs, all_ones_imm))];
}
def LSL : LNP64RRR<"lsl"> {
  let Pattern = [(set GPR:$rd, (shl GPR:$rs1, GPR:$rs2))];
}
def LSR : LNP64RRR<"lsr"> {
  let Pattern = [(set GPR:$rd, (srl GPR:$rs1, GPR:$rs2))];
}
def ASR : LNP64RRR<"asr"> {
  let Pattern = [(set GPR:$rd, (sra GPR:$rs1, GPR:$rs2))];
}
let isCompare = 1, Defs = [FLAGS] in
def CMP : LNP64Cmp<"cmp">;
def CSET_EQ : LNP64Cset<"cset.eq">;
def CSET_NE : LNP64Cset<"cset.ne">;
def CSET_LT : LNP64Cset<"cset.lt">;
def CSET_GT : LNP64Cset<"cset.gt">;
def CSET_LE : LNP64Cset<"cset.le">;
def CSET_GE : LNP64Cset<"cset.ge">;
def CSET_ULT : LNP64Cset<"cset.ult">;
def CSET_UGT : LNP64Cset<"cset.ugt">;
def CSET_ULE : LNP64Cset<"cset.ule">;
def CSET_UGE : LNP64Cset<"cset.uge">;
def PseudoCSETEQ : LNP64SetCCPseudo<"pseudo_cseteq">;
def PseudoCSETNE : LNP64SetCCPseudo<"pseudo_csetne">;
def PseudoCSETLT : LNP64SetCCPseudo<"pseudo_csetlt">;
def PseudoCSETGT : LNP64SetCCPseudo<"pseudo_csetgt">;
def PseudoCSETLE : LNP64SetCCPseudo<"pseudo_csetle">;
def PseudoCSETGE : LNP64SetCCPseudo<"pseudo_csetge">;
def PseudoCSETULT : LNP64SetCCPseudo<"pseudo_csetult">;
def PseudoCSETUGT : LNP64SetCCPseudo<"pseudo_csetugt">;
def PseudoCSETULE : LNP64SetCCPseudo<"pseudo_csetule">;
def PseudoCSETUGE : LNP64SetCCPseudo<"pseudo_csetuge">;
def PseudoCSETEQI : LNP64SetCCImmPseudo<"pseudo_cseteqi">;
def PseudoCSETNEI : LNP64SetCCImmPseudo<"pseudo_csetnei">;
def PseudoCSETLTI : LNP64SetCCImmPseudo<"pseudo_csetlti">;
def PseudoCSETGTI : LNP64SetCCImmPseudo<"pseudo_csetgti">;
def PseudoCSETLEI : LNP64SetCCImmPseudo<"pseudo_csetlei">;
def PseudoCSETGEI : LNP64SetCCImmPseudo<"pseudo_csetgei">;
def PseudoCSETULTI : LNP64SetCCImmPseudo<"pseudo_csetulti">;
def PseudoCSETUGTI : LNP64SetCCImmPseudo<"pseudo_csetugti">;
def PseudoCSETULEI : LNP64SetCCImmPseudo<"pseudo_csetulei">;
def PseudoCSETUGEI : LNP64SetCCImmPseudo<"pseudo_csetugei">;
def : Pat<(i64 (seteq GPR:$lhs, GPR:$rhs)),
          (PseudoCSETEQ GPR:$lhs, GPR:$rhs)>;
def : Pat<(i64 (setne GPR:$lhs, GPR:$rhs)),
          (PseudoCSETNE GPR:$lhs, GPR:$rhs)>;
def : Pat<(i64 (setlt GPR:$lhs, GPR:$rhs)),
          (PseudoCSETLT GPR:$lhs, GPR:$rhs)>;
def : Pat<(i64 (setgt GPR:$lhs, GPR:$rhs)),
          (PseudoCSETGT GPR:$lhs, GPR:$rhs)>;
def : Pat<(i64 (setle GPR:$lhs, GPR:$rhs)),
          (PseudoCSETLE GPR:$lhs, GPR:$rhs)>;
def : Pat<(i64 (setge GPR:$lhs, GPR:$rhs)),
          (PseudoCSETGE GPR:$lhs, GPR:$rhs)>;
def : Pat<(i64 (setult GPR:$lhs, GPR:$rhs)),
          (PseudoCSETULT GPR:$lhs, GPR:$rhs)>;
def : Pat<(i64 (setugt GPR:$lhs, GPR:$rhs)),
          (PseudoCSETUGT GPR:$lhs, GPR:$rhs)>;
def : Pat<(i64 (setule GPR:$lhs, GPR:$rhs)),
          (PseudoCSETULE GPR:$lhs, GPR:$rhs)>;
def : Pat<(i64 (setuge GPR:$lhs, GPR:$rhs)),
          (PseudoCSETUGE GPR:$lhs, GPR:$rhs)>;
def : Pat<(i64 (seteq GPR:$lhs, simm16_imm:$rhs)),
          (PseudoCSETEQI GPR:$lhs, simm16_imm:$rhs)>;
def : Pat<(i64 (setne GPR:$lhs, simm16_imm:$rhs)),
          (PseudoCSETNEI GPR:$lhs, simm16_imm:$rhs)>;
def : Pat<(i64 (setlt GPR:$lhs, simm16_imm:$rhs)),
          (PseudoCSETLTI GPR:$lhs, simm16_imm:$rhs)>;
def : Pat<(i64 (setgt GPR:$lhs, simm16_imm:$rhs)),
          (PseudoCSETGTI GPR:$lhs, simm16_imm:$rhs)>;
def : Pat<(i64 (setle GPR:$lhs, simm16_imm:$rhs)),
          (PseudoCSETLEI GPR:$lhs, simm16_imm:$rhs)>;
def : Pat<(i64 (setge GPR:$lhs, simm16_imm:$rhs)),
          (PseudoCSETGEI GPR:$lhs, simm16_imm:$rhs)>;
def : Pat<(i64 (setult GPR:$lhs, simm16_imm:$rhs)),
          (PseudoCSETULTI GPR:$lhs, simm16_imm:$rhs)>;
def : Pat<(i64 (setugt GPR:$lhs, simm16_imm:$rhs)),
          (PseudoCSETUGTI GPR:$lhs, simm16_imm:$rhs)>;
def : Pat<(i64 (setule GPR:$lhs, simm16_imm:$rhs)),
          (PseudoCSETULEI GPR:$lhs, simm16_imm:$rhs)>;
def : Pat<(i64 (setuge GPR:$lhs, simm16_imm:$rhs)),
          (PseudoCSETUGEI GPR:$lhs, simm16_imm:$rhs)>;
let isCompare = 1, Defs = [FLAGS] in
def CMPU : LNP64Cmp<"cmpu">;

let isBranch = 1, isTerminator = 1, isBarrier = 1 in
def JMP : LNP64Branch<"jmp"> {
  let Pattern = [(br bb:$target)];
}
let isBranch = 1, isTerminator = 1, Uses = [FLAGS] in {
def BEQ : LNP64Branch<"beq">;
def BNE : LNP64Branch<"bne">;
def BLT : LNP64Branch<"blt">;
def BGT : LNP64Branch<"bgt">;
def BLE : LNP64Branch<"ble">;
def BGE : LNP64Branch<"bge">;
}
def PseudoBEQ : LNP64CondBranchPseudo<"pseudo_beq">;
def PseudoBNE : LNP64CondBranchPseudo<"pseudo_bne">;
def PseudoBLT : LNP64CondBranchPseudo<"pseudo_blt">;
def PseudoBGT : LNP64CondBranchPseudo<"pseudo_bgt">;
def PseudoBLE : LNP64CondBranchPseudo<"pseudo_ble">;
def PseudoBGE : LNP64CondBranchPseudo<"pseudo_bge">;
def PseudoBUGT : LNP64CondBranchPseudo<"pseudo_bugt">;
def PseudoBULE : LNP64CondBranchPseudo<"pseudo_bule">;
def PseudoBULT : LNP64CondBranchPseudo<"pseudo_bult">;
def PseudoBUGE : LNP64CondBranchPseudo<"pseudo_buge">;
def : Pat<(LNP64breq GPR:$lhs, GPR:$rhs, bb:$target),
          (PseudoBEQ GPR:$lhs, GPR:$rhs, bb:$target)>;
def : Pat<(LNP64brne GPR:$lhs, GPR:$rhs, bb:$target),
          (PseudoBNE GPR:$lhs, GPR:$rhs, bb:$target)>;
def : Pat<(LNP64brlt GPR:$lhs, GPR:$rhs, bb:$target),
          (PseudoBLT GPR:$lhs, GPR:$rhs, bb:$target)>;
def : Pat<(LNP64brgt GPR:$lhs, GPR:$rhs, bb:$target),
          (PseudoBGT GPR:$lhs, GPR:$rhs, bb:$target)>;
def : Pat<(LNP64brle GPR:$lhs, GPR:$rhs, bb:$target),
          (PseudoBLE GPR:$lhs, GPR:$rhs, bb:$target)>;
def : Pat<(LNP64brge GPR:$lhs, GPR:$rhs, bb:$target),
          (PseudoBGE GPR:$lhs, GPR:$rhs, bb:$target)>;
def : Pat<(LNP64brult GPR:$lhs, GPR:$rhs, bb:$target),
          (PseudoBULT GPR:$lhs, GPR:$rhs, bb:$target)>;
def : Pat<(LNP64brugt GPR:$lhs, GPR:$rhs, bb:$target),
          (PseudoBUGT GPR:$lhs, GPR:$rhs, bb:$target)>;
def : Pat<(LNP64brule GPR:$lhs, GPR:$rhs, bb:$target),
          (PseudoBULE GPR:$lhs, GPR:$rhs, bb:$target)>;
def : Pat<(LNP64bruge GPR:$lhs, GPR:$rhs, bb:$target),
          (PseudoBUGE GPR:$lhs, GPR:$rhs, bb:$target)>;
let isCall = 1, Defs = [LR] in
def CALL : LNP64Branch<"call">;
def : Pat<(LNP64call tglobaladdr:$target), (CALL tglobaladdr:$target)>;
def : Pat<(LNP64call texternalsym:$target), (CALL texternalsym:$target)>;
let isCall = 1, Defs = [LR] in
def CALL_REG : LNP64Inst<(outs), (ins GPR:$target), "call_reg", "\t$target">;
def : Pat<(LNP64call GPR:$target), (CALL_REG GPR:$target)>;
let isReturn = 1, isTerminator = 1, isBarrier = 1, Uses = [LR] in
def RET : LNP64NoOperand<"ret"> {
  let Pattern = [(LNP64retflag)];
}
def ERRNO_GET : LNP64RuntimeGet<"errno_get">;
def ERRNO_SET : LNP64RuntimeSet<"errno_set">;
let isTerminator = 1, isBarrier = 1 in
def EXIT : LNP64RuntimeSet<"exit">;

def LD : LNP64MemLoad<"ld">;
def LD_W : LNP64MemLoad<"ld.w">;
def LD_H : LNP64MemLoad<"ld.h">;
def LD_B : LNP64MemLoad<"ld.b">;
def PseudoLD_SW : LNP64SignedLoadPseudo<"pseudo_ld.sw">;
def PseudoLD_SH : LNP64SignedLoadPseudo<"pseudo_ld.sh">;
def PseudoLD_SB : LNP64SignedLoadPseudo<"pseudo_ld.sb">;
def PseudoFRAMEADDR : LNP64FrameAddrPseudo<"pseudo_frameaddr">;
def ST : LNP64MemStore<"st">;
def ST_W : LNP64MemStore<"st.w">;
def ST_H : LNP64MemStore<"st.h">;
def ST_B : LNP64MemStore<"st.b">;

def : Pat<(i64 (load (add GPR:$base, simm14_imm:$offset))),
          (LD GPR:$base, simm14_imm:$offset)>;
def : Pat<(i64 (load GPR:$base)), (LD GPR:$base, 0)>;
def : Pat<(i64 (zextloadi32 (add GPR:$base, simm14_imm:$offset))),
          (LD_W GPR:$base, simm14_imm:$offset)>;
def : Pat<(i64 (zextloadi32 GPR:$base)), (LD_W GPR:$base, 0)>;
def : Pat<(i64 (extloadi32 (add GPR:$base, simm14_imm:$offset))),
          (LD_W GPR:$base, simm14_imm:$offset)>;
def : Pat<(i64 (extloadi32 GPR:$base)), (LD_W GPR:$base, 0)>;
def : Pat<(i64 (sextloadi32 (add GPR:$base, simm14_imm:$offset))),
          (PseudoLD_SW GPR:$base, simm14_imm:$offset)>;
def : Pat<(i64 (sextloadi32 GPR:$base)), (PseudoLD_SW GPR:$base, 0)>;
def : Pat<(i64 (zextloadi16 (add GPR:$base, simm14_imm:$offset))),
          (LD_H GPR:$base, simm14_imm:$offset)>;
def : Pat<(i64 (zextloadi16 GPR:$base)), (LD_H GPR:$base, 0)>;
def : Pat<(i64 (extloadi16 (add GPR:$base, simm14_imm:$offset))),
          (LD_H GPR:$base, simm14_imm:$offset)>;
def : Pat<(i64 (extloadi16 GPR:$base)), (LD_H GPR:$base, 0)>;
def : Pat<(i64 (sextloadi16 (add GPR:$base, simm14_imm:$offset))),
          (PseudoLD_SH GPR:$base, simm14_imm:$offset)>;
def : Pat<(i64 (sextloadi16 GPR:$base)), (PseudoLD_SH GPR:$base, 0)>;
def : Pat<(i64 (zextloadi8 (add GPR:$base, simm14_imm:$offset))),
          (LD_B GPR:$base, simm14_imm:$offset)>;
def : Pat<(i64 (zextloadi8 GPR:$base)), (LD_B GPR:$base, 0)>;
def : Pat<(i64 (extloadi8 (add GPR:$base, simm14_imm:$offset))),
          (LD_B GPR:$base, simm14_imm:$offset)>;
def : Pat<(i64 (extloadi8 GPR:$base)), (LD_B GPR:$base, 0)>;
def : Pat<(i64 (sextloadi8 (add GPR:$base, simm14_imm:$offset))),
          (PseudoLD_SB GPR:$base, simm14_imm:$offset)>;
def : Pat<(i64 (sextloadi8 GPR:$base)), (PseudoLD_SB GPR:$base, 0)>;
def : Pat<(store GPR:$rs, (add GPR:$base, simm14_imm:$offset)),
          (ST GPR:$rs, GPR:$base, simm14_imm:$offset)>;
def : Pat<(store GPR:$rs, GPR:$base), (ST GPR:$rs, GPR:$base, 0)>;
def : Pat<(truncstorei32 GPR:$rs, (add GPR:$base, simm14_imm:$offset)),
          (ST_W GPR:$rs, GPR:$base, simm14_imm:$offset)>;
def : Pat<(truncstorei32 GPR:$rs, GPR:$base), (ST_W GPR:$rs, GPR:$base, 0)>;
def : Pat<(truncstorei16 GPR:$rs, (add GPR:$base, simm14_imm:$offset)),
          (ST_H GPR:$rs, GPR:$base, simm14_imm:$offset)>;
def : Pat<(truncstorei16 GPR:$rs, GPR:$base), (ST_H GPR:$rs, GPR:$base, 0)>;
def : Pat<(truncstorei8 GPR:$rs, (add GPR:$base, simm14_imm:$offset)),
          (ST_B GPR:$rs, GPR:$base, simm14_imm:$offset)>;
def : Pat<(truncstorei8 GPR:$rs, GPR:$base), (ST_B GPR:$rs, GPR:$base, 0)>;
def FENCE : LNP64NoOperand<"fence">;
def ISYNC : LNP64Inst<(outs), (ins GPR:$rd, GPR:$rs1, GPR:$rs2), "isync",
                      "\t$rd, $rs1, $rs2">;

def LOCK_CMPXCHG
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$addr, GPR:$old, GPR:$new), "lock.cmpxchg",
                "\t$rd, $addr, $old, $new">;
def FUTEX_WAIT : LNP64Inst<(outs GPR:$rd), (ins GPR:$addr, GPR:$expected),
                           "futex_wait", "\t$rd, $addr, $expected">;
def FUTEX_WAKE : LNP64Inst<(outs GPR:$rd), (ins GPR:$addr, GPR:$count),
                           "futex_wake", "\t$rd, $addr, $count">;

def PULL : LNP64Native4<"pull"> {
  let Pattern = [(set GPR:$rd, (LNP64pull GPR:$cap, GPR:$arg0, GPR:$arg1))];
}
def PUSH : LNP64Native4<"push"> {
  let Pattern = [(set GPR:$rd, (LNP64push GPR:$cap, GPR:$arg0, GPR:$arg1))];
}
def AWAIT : LNP64Inst<(outs GPR:$rd), (ins FDR:$cap, GPR:$mask), "await",
                      "\t$rd, $cap, $mask">;
def MMAP : LNP64Inst<(outs GPR:$rd), (ins GPR:$addr, GPR:$len, GPR:$prot),
                     "mmap", "\t$rd, $addr, $len, $prot">;
def MUNMAP : LNP64Native2<"munmap">;
def MPROTECT : LNP64Inst<(outs GPR:$rd), (ins GPR:$addr, GPR:$len, GPR:$prot),
                         "mprotect", "\t$rd, $addr, $len, $prot">;
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
  let Pattern = [(set GPR:$rd,
                  (LNP64gatecall GPR:$cap, GPR:$arg0, GPR:$arg1))];
}
def GATE_RETURN : LNP64Inst<(outs), (ins GPR:$status, GPR:$arg0, GPR:$arg1),
                            "gate_return", "\t$status, $arg0, $arg1">;

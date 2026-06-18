def simm16 : Operand<i64>;
def simm14 : Operand<i64>;
def simm26 : Operand<i64>;
def simm16_imm : PatLeaf<(imm), [{
  return isInt<16>(N->getSExtValue());
}]>;
def simm14_imm : PatLeaf<(imm), [{
  return isInt<14>(N->getSExtValue());
}]>;

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

class LNP64RR<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$rs), Mnemonic, "\t$rd, $rs">;

class LNP64RRR<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$rs1, GPR:$rs2), Mnemonic,
                "\t$rd, $rs1, $rs2">;

class LNP64Cmp<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$rs1, GPR:$rs2), Mnemonic,
                "\t$rs1, $rs2">;

class LNP64Branch<string Mnemonic>
    : LNP64Inst<(outs), (ins simm26:$target), Mnemonic, "\t$target">;

class LNP64MemLoad<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$base, simm14:$offset), Mnemonic,
                "\t$rd, $offset($base)">;

class LNP64MemStore<string Mnemonic>
    : LNP64Inst<(outs), (ins GPR:$rs, GPR:$base, simm14:$offset), Mnemonic,
                "\t$rs, $offset($base)">;

class LNP64Native4<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins FDR:$cap, GPR:$arg0, GPR:$arg1),
                Mnemonic, "\t$rd, $cap, $arg0, $arg1">;

class LNP64Native2<string Mnemonic>
    : LNP64Inst<(outs GPR:$rd), (ins GPR:$arg), Mnemonic, "\t$rd, $arg">;

def NOP : LNP64NoOperand<"nop">;
def LI : LNP64RI<"li"> {
  let Pattern = [(set GPR:$rd, simm16_imm:$imm)];
}
def MOV : LNP64RR<"mov">;

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
def NOT : LNP64RR<"not">;
def LSL : LNP64RRR<"lsl"> {
  let Pattern = [(set GPR:$rd, (shl GPR:$rs1, GPR:$rs2))];
}
def LSR : LNP64RRR<"lsr"> {
  let Pattern = [(set GPR:$rd, (srl GPR:$rs1, GPR:$rs2))];
}
def ASR : LNP64RRR<"asr"> {
  let Pattern = [(set GPR:$rd, (sra GPR:$rs1, GPR:$rs2))];
}
let isCompare = 1 in
def CMP : LNP64Cmp<"cmp">;

let isBranch = 1, isTerminator = 1, isBarrier = 1 in
def JMP : LNP64Branch<"jmp">;
let isBranch = 1, isTerminator = 1 in {
def BEQ : LNP64Branch<"beq">;
def BNE : LNP64Branch<"bne">;
def BLT : LNP64Branch<"blt">;
def BGT : LNP64Branch<"bgt">;
def BLE : LNP64Branch<"ble">;
def BGE : LNP64Branch<"bge">;
}
let isCall = 1, Defs = [LR] in
def CALL : LNP64Branch<"call">;
let isCall = 1, Defs = [LR] in
def CALL_REG : LNP64Inst<(outs), (ins GPR:$target), "call_reg", "\t$target">;
let isReturn = 1, isTerminator = 1, isBarrier = 1, Uses = [LR] in
def RET : LNP64NoOperand<"ret">;

def LD : LNP64MemLoad<"ld">;
def LD_W : LNP64MemLoad<"ld.w">;
def LD_H : LNP64MemLoad<"ld.h">;
def LD_B : LNP64MemLoad<"ld.b">;
def ST : LNP64MemStore<"st">;
def ST_W : LNP64MemStore<"st.w">;
def ST_H : LNP64MemStore<"st.h">;
def ST_B : LNP64MemStore<"st.b">;

def : Pat<(i64 (load (add GPR:$base, simm14_imm:$offset))),
          (LD GPR:$base, simm14_imm:$offset)>;
def : Pat<(i64 (load GPR:$base)), (LD GPR:$base, 0)>;
def : Pat<(store GPR:$rs, (add GPR:$base, simm14_imm:$offset)),
          (ST GPR:$rs, GPR:$base, simm14_imm:$offset)>;
def : Pat<(store GPR:$rs, GPR:$base), (ST GPR:$rs, GPR:$base, 0)>;
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

def PULL : LNP64Native4<"pull">;
def PUSH : LNP64Native4<"push">;
def AWAIT : LNP64Inst<(outs GPR:$rd), (ins FDR:$cap, GPR:$mask), "await",
                      "\t$rd, $cap, $mask">;
def MMAP : LNP64Inst<(outs GPR:$rd), (ins GPR:$addr, GPR:$len, GPR:$prot),
                     "mmap", "\t$rd, $addr, $len, $prot">;
def MUNMAP : LNP64Native2<"munmap">;
def MPROTECT : LNP64Inst<(outs GPR:$rd), (ins GPR:$addr, GPR:$len, GPR:$prot),
                         "mprotect", "\t$rd, $addr, $len, $prot">;
def OBJECT_CTL : LNP64Native2<"object_ctl">;
def DOMAIN_CTL : LNP64Native2<"domain_ctl">;
def CAP_SEND : LNP64Native2<"cap_send">;
def CAP_RECV : LNP64Native2<"cap_recv">;
def CAP_DUP : LNP64Native2<"cap_dup">;
def CAP_REVOKE : LNP64Native2<"cap_revoke">;
def GATE_CALL : LNP64Native4<"gate_call">;
def GATE_RETURN : LNP64Inst<(outs), (ins GPR:$status, GPR:$arg0, GPR:$arg1),
                            "gate_return", "\t$status, $arg0, $arg1">;

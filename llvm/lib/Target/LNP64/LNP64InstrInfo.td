class LNP64Inst<string Mnemonic> : Instruction {
  let Namespace = "LNP64";
  let Size = 4;
  let OutOperandList = (outs);
  let InOperandList = (ins);
  let AsmString = Mnemonic;
}

def NOP : LNP64Inst<"nop">;
def LI : LNP64Inst<"li">;
def MOV : LNP64Inst<"mov">;

def ADD : LNP64Inst<"add">;
def SUB : LNP64Inst<"sub">;
def MUL : LNP64Inst<"mul">;
def DIV : LNP64Inst<"div">;
def AND : LNP64Inst<"and">;
def OR : LNP64Inst<"or">;
def XOR : LNP64Inst<"xor">;
def NOT : LNP64Inst<"not">;
def LSL : LNP64Inst<"lsl">;
def LSR : LNP64Inst<"lsr">;
def ASR : LNP64Inst<"asr">;
def CMP : LNP64Inst<"cmp">;

def JMP : LNP64Inst<"jmp">;
def BEQ : LNP64Inst<"beq">;
def BNE : LNP64Inst<"bne">;
def BLT : LNP64Inst<"blt">;
def BGT : LNP64Inst<"bgt">;
def BLE : LNP64Inst<"ble">;
def BGE : LNP64Inst<"bge">;
def CALL : LNP64Inst<"call">;
def CALL_REG : LNP64Inst<"call_reg">;
def RET : LNP64Inst<"ret">;

def LD : LNP64Inst<"ld">;
def LD_W : LNP64Inst<"ld.w">;
def LD_B : LNP64Inst<"ld.b">;
def ST : LNP64Inst<"st">;
def ST_W : LNP64Inst<"st.w">;
def ST_B : LNP64Inst<"st.b">;
def FENCE : LNP64Inst<"fence">;
def ISYNC : LNP64Inst<"isync">;

def LOCK_CMPXCHG : LNP64Inst<"lock.cmpxchg">;
def FUTEX_WAIT : LNP64Inst<"futex_wait">;
def FUTEX_WAKE : LNP64Inst<"futex_wake">;

def PULL : LNP64Inst<"pull">;
def PUSH : LNP64Inst<"push">;
def AWAIT : LNP64Inst<"await">;
def MMAP : LNP64Inst<"mmap">;
def MUNMAP : LNP64Inst<"munmap">;
def MPROTECT : LNP64Inst<"mprotect">;
def OBJECT_CTL : LNP64Inst<"object_ctl">;
def DOMAIN_CTL : LNP64Inst<"domain_ctl">;
def CAP_SEND : LNP64Inst<"cap_send">;
def CAP_RECV : LNP64Inst<"cap_recv">;
def CAP_DUP : LNP64Inst<"cap_dup">;
def CAP_REVOKE : LNP64Inst<"cap_revoke">;
def GATE_CALL : LNP64Inst<"gate_call">;
def GATE_RETURN : LNP64Inst<"gate_return">;

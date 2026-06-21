// v2 integer ABI: r1 is the return address (ra) and is call-clobbered. It is
// no longer an argument/return register. Arguments come in r2..r9; the integer
// result returns in r2.
def CC_LNP64 : CallingConv<[
  CCIfType<[i1, i8, i16, i32, i64, iPTR],
           CCAssignToReg<[R2, R3, R4, R5, R6, R7, R8, R9]>>,
  CCAssignToStack<8, 16>
]>;

def RetCC_LNP64 : CallingConv<[
  CCIfType<[i1, i8, i16, i32, i64, iPTR], CCAssignToReg<[R2]>>,
  CCAssignToStack<8, 16>
]>;

// v2 callee-saved (preserved) GPR set: s0..s9 = r18..r27. A callee that uses
// any of these must save/restore it; the register allocator may keep
// cross-call values here instead of spilling to the stack. r0 (zero),
// r1 (ra), r30 (backend scratch) and r31 (sp) are not part of this set; every
// other GPR (r2..r17, r28, r29) is call-clobbered (caller-saved).
def CSR_LNP64 : CalleeSavedRegs<(add R18, R19, R20, R21, R22, R23, R24, R25,
                                     R26, R27)>;

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

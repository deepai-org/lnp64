def CC_LNP64 : CallingConv<[
  CCIfType<[i1, i8, i16, i32, i64, ptr], CCAssignToReg<[R1, R2, R3, R4, R5, R6]>>,
  CCAssignToStack<8, 16>
]>;

def RetCC_LNP64 : CallingConv<[
  CCIfType<[i1, i8, i16, i32, i64, ptr], CCAssignToReg<[R1]>>,
  CCAssignToStack<8, 16>
]>;

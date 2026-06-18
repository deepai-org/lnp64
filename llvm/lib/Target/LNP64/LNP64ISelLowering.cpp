#include "LNP64ISelLowering.h"
#include "LNP64Subtarget.h"
#include "llvm/CodeGen/MachineFunction.h"

using namespace llvm;

LNP64TargetLowering::LNP64TargetLowering(const TargetMachine &TM,
                                         const LNP64Subtarget &)
    : TargetLowering(TM) {
  addRegisterClass(MVT::i64, &LNP64::GPRRegClass);
  setStackPointerRegisterToSaveRestore(LNP64::R31);
  setBooleanContents(ZeroOrOneBooleanContent);
}

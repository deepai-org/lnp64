#include "LNP64ISelLowering.h"
#include "LNP64Subtarget.h"
#include "llvm/CodeGen/MachineFunction.h"

using namespace llvm;

LNP64TargetLowering::LNP64TargetLowering(const TargetMachine &TM,
                                         const LNP64Subtarget &STI)
    : TargetLowering(TM) {
  addRegisterClass(MVT::i64, &LNP64::GPRRegClass);
  setStackPointerRegisterToSaveRestore(LNP64::R31);
  setBooleanContents(ZeroOrOneBooleanContent);

  for (unsigned Opcode : {ISD::ADD, ISD::SUB, ISD::MUL, ISD::SDIV, ISD::AND,
                          ISD::OR, ISD::XOR, ISD::SHL, ISD::SRL, ISD::SRA})
    setOperationAction(Opcode, MVT::i64, Legal);

  setOperationAction(ISD::UDIV, MVT::i64, Expand);
  setOperationAction(ISD::UREM, MVT::i64, Expand);
  setOperationAction(ISD::SREM, MVT::i64, Expand);
  computeRegisterProperties(STI.getRegisterInfo());
}

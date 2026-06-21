#ifndef LLVM_LIB_TARGET_LNP64_LNP64REGISTERINFO_H
#define LLVM_LIB_TARGET_LNP64_LNP64REGISTERINFO_H

#include "llvm/CodeGen/TargetRegisterInfo.h"

#define GET_REGINFO_HEADER
#include "LNP64GenRegisterInfo.inc"

namespace llvm {

class LNP64RegisterInfo : public LNP64GenRegisterInfo {
public:
  LNP64RegisterInfo();

  BitVector getReservedRegs(const MachineFunction &MF) const override;
  const MCPhysReg *getCalleeSavedRegs(const MachineFunction *MF) const override;
  const uint32_t *getCallPreservedMask(const MachineFunction &MF,
                                       CallingConv::ID CC) const override;
  Register getFrameRegister(const MachineFunction &MF) const override;
  void eliminateFrameIndex(MachineBasicBlock::iterator MI, int SPAdj,
                           unsigned FIOperandNum,
                           RegScavenger *RS = nullptr) const override;
};

} // end namespace llvm

#endif

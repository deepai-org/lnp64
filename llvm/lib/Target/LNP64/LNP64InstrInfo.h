#ifndef LLVM_LIB_TARGET_LNP64_LNP64INSTRINFO_H
#define LLVM_LIB_TARGET_LNP64_LNP64INSTRINFO_H

#include "LNP64RegisterInfo.h"

#define GET_INSTRINFO_HEADER
#include "LNP64GenInstrInfo.inc"

namespace llvm {

class LNP64InstrInfo : public LNP64GenInstrInfo {
  LNP64RegisterInfo RI;

public:
  LNP64InstrInfo();

  const LNP64RegisterInfo &getRegisterInfo() const { return RI; }
  void copyPhysReg(MachineBasicBlock &MBB, MachineBasicBlock::iterator I,
                   const DebugLoc &DL, MCRegister DestReg, MCRegister SrcReg,
                   bool KillSrc) const override;
};

} // end namespace llvm

#endif

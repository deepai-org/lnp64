#ifndef LLVM_LIB_TARGET_LNP64_LNP64FRAMELOWERING_H
#define LLVM_LIB_TARGET_LNP64_LNP64FRAMELOWERING_H

#include "llvm/CodeGen/TargetFrameLowering.h"

namespace llvm {

class LNP64FrameLowering : public TargetFrameLowering {
public:
  LNP64FrameLowering();

  void emitPrologue(MachineFunction &MF, MachineBasicBlock &MBB) const override;
  void emitEpilogue(MachineFunction &MF, MachineBasicBlock &MBB) const override;
};

} // end namespace llvm

#endif

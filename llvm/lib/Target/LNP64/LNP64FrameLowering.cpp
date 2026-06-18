#include "LNP64FrameLowering.h"
#include "llvm/CodeGen/MachineFrameInfo.h"
#include "llvm/CodeGen/MachineFunction.h"
#include "llvm/Support/ErrorHandling.h"

using namespace llvm;

LNP64FrameLowering::LNP64FrameLowering()
    : TargetFrameLowering(StackGrowsDown, Align(16), /*LocalAreaOffset=*/8) {}

void LNP64FrameLowering::emitPrologue(MachineFunction &MF,
                                      MachineBasicBlock &) const {
  if (MF.getFrameInfo().getStackSize() == 0)
    return;
  llvm_unreachable("LNP64 nonzero stack adjustment is not implemented yet");
}

void LNP64FrameLowering::emitEpilogue(MachineFunction &MF,
                                      MachineBasicBlock &) const {
  if (MF.getFrameInfo().getStackSize() == 0)
    return;
  llvm_unreachable("LNP64 nonzero stack adjustment is not implemented yet");
}

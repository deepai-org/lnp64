#include "LNP64FrameLowering.h"
#include "llvm/Support/ErrorHandling.h"

using namespace llvm;

LNP64FrameLowering::LNP64FrameLowering()
    : TargetFrameLowering(StackGrowsDown, Align(16), /*LocalAreaOffset=*/8) {}

void LNP64FrameLowering::emitPrologue(MachineFunction &,
                                      MachineBasicBlock &) const {
  llvm_unreachable("LNP64 frame lowering is scaffolded but not implemented yet");
}

void LNP64FrameLowering::emitEpilogue(MachineFunction &,
                                      MachineBasicBlock &) const {
  llvm_unreachable("LNP64 frame lowering is scaffolded but not implemented yet");
}

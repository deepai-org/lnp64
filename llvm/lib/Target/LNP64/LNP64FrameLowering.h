#ifndef LLVM_LIB_TARGET_LNP64_LNP64FRAMELOWERING_H
#define LLVM_LIB_TARGET_LNP64_LNP64FRAMELOWERING_H

#include "llvm/CodeGen/TargetFrameLowering.h"

namespace llvm {

// Bytes the prologue reserves on top of the PEI frame for the r1 (ra) spill
// slot when a function makes calls -- one 64-bit SD/LD slot. Shared by frame
// lowering and eliminateFrameIndex (fixed-object offsets) so the two stay in
// lockstep.
inline constexpr uint64_t LNP64RASaveSlotBytes = 8;

class LNP64FrameLowering : public TargetFrameLowering {
public:
  LNP64FrameLowering();

  void emitPrologue(MachineFunction &MF, MachineBasicBlock &MBB) const override;
  void emitEpilogue(MachineFunction &MF, MachineBasicBlock &MBB) const override;
  MachineBasicBlock::iterator
  eliminateCallFramePseudoInstr(MachineFunction &MF, MachineBasicBlock &MBB,
                                MachineBasicBlock::iterator I) const override;
  bool hasFP(const MachineFunction &MF) const override;
};

} // end namespace llvm

#endif

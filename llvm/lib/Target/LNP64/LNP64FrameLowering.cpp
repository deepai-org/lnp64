#include "LNP64FrameLowering.h"
#include "LNP64.h"
#include "LNP64InstrInfo.h"
#include "llvm/CodeGen/MachineFrameInfo.h"
#include "llvm/CodeGen/MachineFunction.h"
#include "llvm/CodeGen/MachineInstrBuilder.h"
#include "llvm/CodeGen/TargetInstrInfo.h"
#include "llvm/CodeGen/TargetSubtargetInfo.h"
#include "llvm/Support/ErrorHandling.h"
#include "llvm/Support/MathExtras.h"

using namespace llvm;

static uint64_t getLRSaveSize(const MachineFunction &MF) {
  return MF.getFrameInfo().hasCalls() ? 8 : 0;
}

static void emitSPAdjust(MachineFunction &MF, MachineBasicBlock &MBB,
                         MachineBasicBlock::iterator I, const DebugLoc &DL,
                         int64_t Amount) {
  if (Amount == 0)
    return;
  if (!isInt<16>(Amount))
    llvm_unreachable("LNP64 stack adjustment exceeds signed-16 LI range");

  const TargetInstrInfo &TII = *MF.getSubtarget().getInstrInfo();
  int64_t Magnitude = Amount < 0 ? -Amount : Amount;
  BuildMI(MBB, I, DL, TII.get(LNP64::LI), LNP64::R30).addImm(Magnitude);
  BuildMI(MBB, I, DL, TII.get(Amount < 0 ? LNP64::SUB : LNP64::ADD),
          LNP64::R31)
      .addReg(LNP64::R31)
      .addReg(LNP64::R30);
}

LNP64FrameLowering::LNP64FrameLowering()
    : TargetFrameLowering(StackGrowsDown, Align(16), /*LocalAreaOffset=*/0) {}

void LNP64FrameLowering::emitPrologue(MachineFunction &MF,
                                      MachineBasicBlock &MBB) const {
  const uint64_t LRSaveSize = getLRSaveSize(MF);
  uint64_t StackSize = MF.getFrameInfo().getStackSize() + LRSaveSize;
  MachineBasicBlock::iterator I = MBB.begin();
  emitSPAdjust(MF, MBB, I, DebugLoc(), -int64_t(StackSize));

  if (LRSaveSize != 0) {
    const TargetInstrInfo &TII = *MF.getSubtarget().getInstrInfo();
    BuildMI(MBB, I, DebugLoc(), TII.get(LNP64::LR_GET), LNP64::R30);
    BuildMI(MBB, I, DebugLoc(), TII.get(LNP64::ST))
        .addReg(LNP64::R30)
        .addReg(LNP64::R31)
        .addImm(0);
  }
}

void LNP64FrameLowering::emitEpilogue(MachineFunction &MF,
                                      MachineBasicBlock &MBB) const {
  const uint64_t LRSaveSize = getLRSaveSize(MF);
  uint64_t StackSize = MF.getFrameInfo().getStackSize() + LRSaveSize;
  MachineBasicBlock::iterator I = MBB.getFirstTerminator();

  if (LRSaveSize != 0) {
    const TargetInstrInfo &TII = *MF.getSubtarget().getInstrInfo();
    BuildMI(MBB, I, DebugLoc(), TII.get(LNP64::LD), LNP64::R30)
        .addReg(LNP64::R31)
        .addImm(0);
    BuildMI(MBB, I, DebugLoc(), TII.get(LNP64::LR_SET)).addReg(LNP64::R30);
  }
  emitSPAdjust(MF, MBB, I, DebugLoc(), int64_t(StackSize));
}

MachineBasicBlock::iterator LNP64FrameLowering::eliminateCallFramePseudoInstr(
    MachineFunction &, MachineBasicBlock &MBB,
    MachineBasicBlock::iterator I) const {
  return MBB.erase(I);
}

bool LNP64FrameLowering::hasFP(const MachineFunction &) const { return false; }

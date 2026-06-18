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
    : TargetFrameLowering(StackGrowsDown, Align(16), /*LocalAreaOffset=*/8) {}

void LNP64FrameLowering::emitPrologue(MachineFunction &MF,
                                      MachineBasicBlock &MBB) const {
  uint64_t StackSize = MF.getFrameInfo().getStackSize();
  emitSPAdjust(MF, MBB, MBB.begin(), DebugLoc(), -int64_t(StackSize));
}

void LNP64FrameLowering::emitEpilogue(MachineFunction &MF,
                                      MachineBasicBlock &MBB) const {
  uint64_t StackSize = MF.getFrameInfo().getStackSize();
  emitSPAdjust(MF, MBB, MBB.getFirstTerminator(), DebugLoc(),
               int64_t(StackSize));
}

bool LNP64FrameLowering::hasFP(const MachineFunction &) const { return true; }

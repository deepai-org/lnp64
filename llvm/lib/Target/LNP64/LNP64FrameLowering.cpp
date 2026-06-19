#include "LNP64FrameLowering.h"
#include "LNP64.h"
#include "LNP64InstrInfo.h"
#include "llvm/CodeGen/MachineFrameInfo.h"
#include "llvm/CodeGen/MachineFunction.h"
#include "llvm/CodeGen/MachineInstrBuilder.h"
#include "llvm/CodeGen/TargetInstrInfo.h"
#include "llvm/CodeGen/TargetSubtargetInfo.h"
#include "llvm/CodeGen/TargetOpcodes.h"
#include "llvm/MC/MCDwarf.h"
#include "llvm/Support/ErrorHandling.h"
#include "llvm/Support/MathExtras.h"

using namespace llvm;

static uint64_t getLRSaveSize(const MachineFunction &MF) {
  return MF.getFrameInfo().hasCalls() ? 8 : 0;
}

static uint64_t getLRSaveOffset(const MachineFunction &MF) {
  return MF.getFrameInfo().hasCalls() ? MF.getFrameInfo().getMaxCallFrameSize()
                                     : 0;
}

static void emitCFI(MachineFunction &MF, MachineBasicBlock &MBB,
                    MachineBasicBlock::iterator I, const DebugLoc &DL,
                    const MCCFIInstruction &CFI) {
  const TargetInstrInfo &TII = *MF.getSubtarget().getInstrInfo();
  unsigned CFIIndex = MF.addFrameInst(CFI);
  BuildMI(MBB, I, DL, TII.get(TargetOpcode::CFI_INSTRUCTION))
      .addCFIIndex(CFIIndex);
}

static void emitSPAdjust(MachineFunction &MF, MachineBasicBlock &MBB,
                         MachineBasicBlock::iterator I, const DebugLoc &DL,
                         int64_t Amount) {
  if (Amount == 0)
    return;

  const TargetInstrInfo &TII = *MF.getSubtarget().getInstrInfo();
  uint64_t Magnitude =
      Amount < 0 ? uint64_t(-(Amount + 1)) + 1 : uint64_t(Amount);
  if (isInt<16>(int64_t(Magnitude))) {
    BuildMI(MBB, I, DL, TII.get(LNP64::LI), LNP64::R30).addImm(Magnitude);
  } else {
    if (!isUInt<32>(Magnitude))
      llvm_unreachable("LNP64 stack adjustment exceeds 32-bit materialization");
    BuildMI(MBB, I, DL, TII.get(LNP64::LI32), LNP64::R30).addImm(Magnitude);
  }
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
  if (StackSize != 0)
    emitCFI(MF, MBB, I, DebugLoc(),
            MCCFIInstruction::cfiDefCfaOffset(nullptr, StackSize));

  if (LRSaveSize != 0) {
    const TargetInstrInfo &TII = *MF.getSubtarget().getInstrInfo();
    const int64_t LRSaveOffsetFromCFA =
        int64_t(getLRSaveOffset(MF)) - int64_t(StackSize);
    BuildMI(MBB, I, DebugLoc(), TII.get(LNP64::LR_GET), LNP64::R30);
    BuildMI(MBB, I, DebugLoc(), TII.get(LNP64::ST))
        .addReg(LNP64::R30)
        .addReg(LNP64::R31)
        .addImm(getLRSaveOffset(MF));
    emitCFI(MF, MBB, I, DebugLoc(),
            MCCFIInstruction::createOffset(nullptr, /*LR=*/32,
                                           LRSaveOffsetFromCFA));
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
        .addImm(getLRSaveOffset(MF));
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

#include "LNP64RegisterInfo.h"
#include "LNP64FrameLowering.h"
#include "MCTargetDesc/LNP64MCTargetDesc.h"
#include "llvm/CodeGen/MachineFrameInfo.h"
#include "llvm/CodeGen/MachineInstr.h"
#include "llvm/CodeGen/MachineInstrBuilder.h"
#include "llvm/CodeGen/MachineFunction.h"
#include "llvm/CodeGen/TargetInstrInfo.h"
#include "llvm/CodeGen/TargetSubtargetInfo.h"
#include "llvm/Support/ErrorHandling.h"
#include "llvm/Support/MathExtras.h"

using namespace llvm;

#define GET_REGINFO_TARGET_DESC
#include "LNP64GenRegisterInfo.inc"

static uint64_t getLRSaveSize(const MachineFunction &MF) {
  return MF.getFrameInfo().hasCalls() ? 8 : 0;
}

LNP64RegisterInfo::LNP64RegisterInfo() : LNP64GenRegisterInfo(LNP64::LR) {}

BitVector LNP64RegisterInfo::getReservedRegs(const MachineFunction &) const {
  BitVector Reserved(getNumRegs());
  Reserved.set(LNP64::R0);
  Reserved.set(LNP64::R30);
  Reserved.set(LNP64::R31);
  Reserved.set(LNP64::LR);
  Reserved.set(LNP64::TP);
  return Reserved;
}

const MCPhysReg *
LNP64RegisterInfo::getCalleeSavedRegs(const MachineFunction *) const {
  static const MCPhysReg NoCalleeSaved[] = {0};
  return NoCalleeSaved;
}

Register LNP64RegisterInfo::getFrameRegister(const MachineFunction &) const {
  return LNP64::R31;
}

void LNP64RegisterInfo::eliminateFrameIndex(MachineBasicBlock::iterator II,
                                            int, unsigned FIOperandNum,
                                            RegScavenger *) const {
  MachineInstr &MI = *II;
  MachineFunction &MF = *MI.getParent()->getParent();
  const MachineFrameInfo &MFI = MF.getFrameInfo();
  int FrameIndex = MI.getOperand(FIOperandNum).getIndex();
  int64_t Offset =
      MFI.getObjectOffset(FrameIndex) + MFI.getStackSize() + getLRSaveSize(MF);
  if (FIOperandNum + 1 < MI.getNumOperands() &&
      MI.getOperand(FIOperandNum + 1).isImm())
    Offset += MI.getOperand(FIOperandNum + 1).getImm();

  const TargetInstrInfo &TII = *MF.getSubtarget().getInstrInfo();
  MachineBasicBlock &MBB = *MI.getParent();
  DebugLoc DL = MI.getDebugLoc();

  if (MI.getOpcode() == LNP64::PseudoFRAMEADDR) {
    Register Dst = MI.getOperand(0).getReg();
    if (Offset == 0) {
      BuildMI(MBB, MI, DL, TII.get(LNP64::MOV), Dst).addReg(LNP64::R31);
    } else if (isInt<16>(Offset)) {
      BuildMI(MBB, MI, DL, TII.get(LNP64::LI), LNP64::R30).addImm(Offset);
      BuildMI(MBB, MI, DL, TII.get(LNP64::ADD), Dst)
          .addReg(LNP64::R31)
          .addReg(LNP64::R30);
    } else {
      BuildMI(MBB, MI, DL, TII.get(LNP64::LI32), LNP64::R30).addImm(Offset);
      BuildMI(MBB, MI, DL, TII.get(LNP64::ADD), Dst)
          .addReg(LNP64::R31)
          .addReg(LNP64::R30);
    }
    MI.eraseFromParent();
    return;
  }

  // ST/LD instructions use a 14-bit signed offset field.  When the frame
  // index resolves to an offset outside [-8192, 8191], we cannot encode it
  // directly.  Materialise the full address in R30 (the reserved scratch
  // register) and use R30+0 as the effective address instead.
  if (!isInt<14>(Offset)) {
    if (isInt<16>(Offset)) {
      BuildMI(MBB, II, DL, TII.get(LNP64::LI), LNP64::R30).addImm(Offset);
    } else {
      BuildMI(MBB, II, DL, TII.get(LNP64::LI32), LNP64::R30).addImm(Offset);
    }
    BuildMI(MBB, II, DL, TII.get(LNP64::ADD), LNP64::R30)
        .addReg(LNP64::R30)
        .addReg(LNP64::R31);
    MI.getOperand(FIOperandNum).ChangeToRegister(LNP64::R30, false);
    if (FIOperandNum + 1 < MI.getNumOperands() &&
        MI.getOperand(FIOperandNum + 1).isImm())
      MI.getOperand(FIOperandNum + 1).ChangeToImmediate(0);
    return;
  }

  MI.getOperand(FIOperandNum).ChangeToRegister(LNP64::R31, false);
  if (FIOperandNum + 1 < MI.getNumOperands() &&
      MI.getOperand(FIOperandNum + 1).isImm())
    MI.getOperand(FIOperandNum + 1).ChangeToImmediate(Offset);
}

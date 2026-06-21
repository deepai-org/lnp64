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

// In v2 the return address is a normal callee-saved GPR (r1), spilled in the
// prologue like any other; there is no separate save slot beyond the stack
// frame the spill code allocates.
LNP64RegisterInfo::LNP64RegisterInfo() : LNP64GenRegisterInfo(LNP64::R1) {}

BitVector LNP64RegisterInfo::getReservedRegs(const MachineFunction &) const {
  BitVector Reserved(getNumRegs());
  Reserved.set(LNP64::R0);  // hardwired zero
  Reserved.set(LNP64::R31); // stack pointer
  // r30 is reclaimed in v2 (general allocatable). r1 (ra) stays allocatable.
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
  int64_t Offset = MFI.getObjectOffset(FrameIndex) + MFI.getStackSize();
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
    } else {
      // Load/store displacements are 32-bit signed in v2, so LI always fits.
      BuildMI(MBB, MI, DL, TII.get(LNP64::LI), LNP64::R30).addImm(Offset);
      BuildMI(MBB, MI, DL, TII.get(LNP64::ADD), Dst)
          .addReg(LNP64::R31)
          .addReg(LNP64::R30);
    }
    MI.eraseFromParent();
    return;
  }

  // v2 load/store offsets are 32-bit signed; frame offsets never overflow in
  // practice, so the v1 large-offset r30 scratch-address path is deleted.
  MI.getOperand(FIOperandNum).ChangeToRegister(LNP64::R31, false);
  if (FIOperandNum + 1 < MI.getNumOperands() &&
      MI.getOperand(FIOperandNum + 1).isImm())
    MI.getOperand(FIOperandNum + 1).ChangeToImmediate(Offset);
}

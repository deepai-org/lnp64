#include "LNP64InstrInfo.h"
#include "LNP64.h"
#include "llvm/CodeGen/MachineInstrBuilder.h"
#include "llvm/Support/ErrorHandling.h"

using namespace llvm;

#define GET_INSTRINFO_CTOR_DTOR
#include "LNP64GenInstrInfo.inc"

LNP64InstrInfo::LNP64InstrInfo() : LNP64GenInstrInfo(LNP64::RET) {}

void LNP64InstrInfo::copyPhysReg(MachineBasicBlock &MBB,
                                 MachineBasicBlock::iterator I,
                                 const DebugLoc &DL, MCRegister DestReg,
                                 MCRegister SrcReg, bool KillSrc) const {
  if (!LNP64::GPRRegClass.contains(DestReg, SrcReg))
    llvm_unreachable("LNP64 only supports GPR register copies today");
  BuildMI(MBB, I, DL, get(LNP64::MOV), DestReg)
      .addReg(SrcReg, getKillRegState(KillSrc));
}

void LNP64InstrInfo::storeRegToStackSlot(
    MachineBasicBlock &MBB, MachineBasicBlock::iterator I, Register SrcReg,
    bool IsKill, int FrameIndex, const TargetRegisterClass *RC,
    const TargetRegisterInfo *) const {
  if (RC != &LNP64::GPRRegClass)
    llvm_unreachable("LNP64 only supports GPR stack spills today");
  DebugLoc DL;
  if (I != MBB.end())
    DL = I->getDebugLoc();
  BuildMI(MBB, I, DL, get(LNP64::ST))
      .addReg(SrcReg, getKillRegState(IsKill))
      .addFrameIndex(FrameIndex)
      .addImm(0);
}

void LNP64InstrInfo::loadRegFromStackSlot(
    MachineBasicBlock &MBB, MachineBasicBlock::iterator I, Register DestReg,
    int FrameIndex, const TargetRegisterClass *RC,
    const TargetRegisterInfo *) const {
  if (RC != &LNP64::GPRRegClass)
    llvm_unreachable("LNP64 only supports GPR stack reloads today");
  DebugLoc DL;
  if (I != MBB.end())
    DL = I->getDebugLoc();
  BuildMI(MBB, I, DL, get(LNP64::LD), DestReg)
      .addFrameIndex(FrameIndex)
      .addImm(0);
}

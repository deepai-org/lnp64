#include "LNP64RegisterInfo.h"
#include "llvm/CodeGen/MachineFunction.h"

using namespace llvm;

#define GET_REGINFO_TARGET_DESC
#include "LNP64GenRegisterInfo.inc"

LNP64RegisterInfo::LNP64RegisterInfo() : LNP64GenRegisterInfo(LNP64::LR) {}

BitVector LNP64RegisterInfo::getReservedRegs(const MachineFunction &) const {
  BitVector Reserved(getNumRegs());
  Reserved.set(LNP64::R0);
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

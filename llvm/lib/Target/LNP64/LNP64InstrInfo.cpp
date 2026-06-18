#include "LNP64InstrInfo.h"
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

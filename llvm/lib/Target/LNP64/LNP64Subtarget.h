#ifndef LLVM_LIB_TARGET_LNP64_LNP64SUBTARGET_H
#define LLVM_LIB_TARGET_LNP64_LNP64SUBTARGET_H

#include "LNP64FrameLowering.h"
#include "LNP64ISelLowering.h"
#include "LNP64InstrInfo.h"
#include "llvm/CodeGen/TargetSubtargetInfo.h"

#define GET_SUBTARGETINFO_HEADER
#include "LNP64GenSubtargetInfo.inc"

namespace llvm {

class LNP64Subtarget : public LNP64GenSubtargetInfo {
  LNP64FrameLowering FrameLowering;
  LNP64InstrInfo InstrInfo;
  LNP64TargetLowering TLInfo;

public:
  LNP64Subtarget(const Triple &TT, StringRef CPU, StringRef FS,
                 const TargetMachine &TM);

  const LNP64FrameLowering *getFrameLowering() const override {
    return &FrameLowering;
  }
  const LNP64InstrInfo *getInstrInfo() const override { return &InstrInfo; }
  const LNP64RegisterInfo *getRegisterInfo() const override {
    return &InstrInfo.getRegisterInfo();
  }
  const LNP64TargetLowering *getTargetLowering() const override {
    return &TLInfo;
  }
};

} // end namespace llvm

#endif

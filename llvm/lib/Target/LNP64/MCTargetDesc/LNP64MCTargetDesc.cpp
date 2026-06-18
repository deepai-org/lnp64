#include "LNP64MCTargetDesc.h"
#include "llvm/MC/MCAsmInfo.h"
#include "llvm/MC/MCInstrInfo.h"
#include "llvm/MC/MCRegisterInfo.h"
#include "llvm/MC/MCSubtargetInfo.h"
#include "llvm/MC/TargetRegistry.h"

using namespace llvm;

#define GET_INSTRINFO_MC_DESC
#include "LNP64GenInstrInfo.inc"

#define GET_REGINFO_MC_DESC
#include "LNP64GenRegisterInfo.inc"

#define GET_SUBTARGETINFO_MC_DESC
#include "LNP64GenSubtargetInfo.inc"

static MCInstrInfo *createLNP64MCInstrInfo() {
  MCInstrInfo *X = new MCInstrInfo();
  InitLNP64MCInstrInfo(X);
  return X;
}

static MCRegisterInfo *createLNP64MCRegisterInfo(const Triple &) {
  MCRegisterInfo *X = new MCRegisterInfo();
  InitLNP64MCRegisterInfo(X, LNP64::LR);
  return X;
}

static MCSubtargetInfo *createLNP64MCSubtargetInfo(const Triple &TT,
                                                  StringRef CPU,
                                                  StringRef FS) {
  if (CPU.empty())
    CPU = "generic-lnp64";
  return createLNP64MCSubtargetInfoImpl(TT, CPU, /*TuneCPU=*/CPU, FS);
}

extern "C" LLVM_EXTERNAL_VISIBILITY void LLVMInitializeLNP64TargetMC() {
  Target &T = getTheLNP64Target();
  TargetRegistry::RegisterMCInstrInfo(T, createLNP64MCInstrInfo);
  TargetRegistry::RegisterMCRegInfo(T, createLNP64MCRegisterInfo);
  TargetRegistry::RegisterMCSubtargetInfo(T, createLNP64MCSubtargetInfo);
  TargetRegistry::RegisterMCCodeEmitter(T, createLNP64MCCodeEmitter);
}

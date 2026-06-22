#include "LNP64Subtarget.h"

using namespace llvm;

#define GET_SUBTARGETINFO_TARGET_DESC
#define GET_SUBTARGETINFO_CTOR
#include "LNP64GenSubtargetInfo.inc"

static StringRef cpuOrDefault(StringRef CPU) {
  return CPU.empty() ? StringRef("generic-lnp64") : CPU;
}

LNP64Subtarget::LNP64Subtarget(const Triple &TT, StringRef CPU, StringRef FS,
                               const TargetMachine &TM)
    : LNP64GenSubtargetInfo(TT, cpuOrDefault(CPU), cpuOrDefault(CPU), FS),
      TLInfo(TM, *this) {
  ParseSubtargetFeatures(cpuOrDefault(CPU), cpuOrDefault(CPU), FS);
}

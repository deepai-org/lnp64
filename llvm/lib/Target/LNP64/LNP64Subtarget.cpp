#include "LNP64Subtarget.h"

using namespace llvm;

#define GET_SUBTARGETINFO_TARGET_DESC
#define GET_SUBTARGETINFO_CTOR
#include "LNP64GenSubtargetInfo.inc"

LNP64Subtarget::LNP64Subtarget(const Triple &TT, StringRef CPU, StringRef FS,
                               const TargetMachine &TM)
    : LNP64GenSubtargetInfo(TT, CPU.empty() ? "generic-lnp64" : CPU,
                            CPU.empty() ? "generic-lnp64" : CPU, FS),
      TLInfo(TM, *this) {
  StringRef CPUName = CPU.empty() ? "generic-lnp64" : CPU;
  ParseSubtargetFeatures(CPUName, CPUName, FS);
}

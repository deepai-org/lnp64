#ifndef LLVM_LIB_TARGET_LNP64_LNP64INSTRINFO_H
#define LLVM_LIB_TARGET_LNP64_LNP64INSTRINFO_H

#include "LNP64RegisterInfo.h"

#define GET_INSTRINFO_HEADER
#include "LNP64GenInstrInfo.inc"

namespace llvm {

class LNP64InstrInfo : public LNP64GenInstrInfo {
  LNP64RegisterInfo RI;

public:
  LNP64InstrInfo();

  const LNP64RegisterInfo &getRegisterInfo() const { return RI; }
};

} // end namespace llvm

#endif

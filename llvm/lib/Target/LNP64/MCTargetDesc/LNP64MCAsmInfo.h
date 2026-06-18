#ifndef LLVM_LIB_TARGET_LNP64_MCTARGETDESC_LNP64MCASMINFO_H
#define LLVM_LIB_TARGET_LNP64_MCTARGETDESC_LNP64MCASMINFO_H

#include "llvm/MC/MCAsmInfoELF.h"

namespace llvm {

class Triple;

class LNP64MCAsmInfo : public MCAsmInfoELF {
  void anchor() override;

public:
  explicit LNP64MCAsmInfo(const Triple &TT, const MCTargetOptions &Options);
};

} // end namespace llvm

#endif

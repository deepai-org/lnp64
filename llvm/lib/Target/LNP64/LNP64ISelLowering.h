#ifndef LLVM_LIB_TARGET_LNP64_LNP64ISELLOWERING_H
#define LLVM_LIB_TARGET_LNP64_LNP64ISELLOWERING_H

#include "llvm/CodeGen/TargetLowering.h"

namespace llvm {

class LNP64Subtarget;

class LNP64TargetLowering : public TargetLowering {
public:
  explicit LNP64TargetLowering(const TargetMachine &TM,
                               const LNP64Subtarget &STI);
};

} // end namespace llvm

#endif

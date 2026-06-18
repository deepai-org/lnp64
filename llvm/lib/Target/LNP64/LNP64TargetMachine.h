#ifndef LLVM_LIB_TARGET_LNP64_LNP64TARGETMACHINE_H
#define LLVM_LIB_TARGET_LNP64_LNP64TARGETMACHINE_H

#include "LNP64Subtarget.h"
#include "llvm/ADT/Optional.h"
#include "llvm/Target/TargetMachine.h"

namespace llvm {

class LNP64TargetMachine : public LLVMTargetMachine {
  LNP64Subtarget Subtarget;

public:
  LNP64TargetMachine(const Target &T, const Triple &TT, StringRef CPU,
                     StringRef FS, const TargetOptions &Options,
                     Optional<Reloc::Model> RM,
                     Optional<CodeModel::Model> CM, CodeGenOpt::Level OL,
                     bool JIT);

  const LNP64Subtarget *getSubtargetImpl(const Function &) const override {
    return &Subtarget;
  }
};

} // end namespace llvm

#endif

#ifndef LLVM_LIB_TARGET_LNP64_LNP64TARGETMACHINE_H
#define LLVM_LIB_TARGET_LNP64_LNP64TARGETMACHINE_H

#include "LNP64Subtarget.h"
#include "llvm/ADT/Optional.h"
#include "llvm/Target/TargetMachine.h"
#include <memory>

namespace llvm {

class PassManagerBase;
class TargetLoweringObjectFile;
class TargetPassConfig;

class LNP64TargetMachine : public LLVMTargetMachine {
  std::unique_ptr<TargetLoweringObjectFile> TLOF;
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

  TargetLoweringObjectFile *getObjFileLowering() const override {
    return TLOF.get();
  }

  TargetPassConfig *createPassConfig(PassManagerBase &PM) override;
};

} // end namespace llvm

#endif

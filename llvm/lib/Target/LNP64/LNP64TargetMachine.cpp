#include "LNP64TargetMachine.h"
#include "LNP64.h"
#include "llvm/CodeGen/TargetPassConfig.h"
#include "llvm/CodeGen/TargetLoweringObjectFileImpl.h"
#include "llvm/MC/TargetRegistry.h"
#include "llvm/Support/Compiler.h"

using namespace llvm;

extern "C" void LLVMInitializeLNP64AsmPrinter();

namespace {

class LNP64PassConfig final : public TargetPassConfig {
public:
  LNP64PassConfig(LNP64TargetMachine &TM, PassManagerBase &PM)
      : TargetPassConfig(TM, PM) {}

  LNP64TargetMachine &getLNP64TargetMachine() const {
    return getTM<LNP64TargetMachine>();
  }

  bool addInstSelector() override;
};

} // end anonymous namespace

extern "C" LLVM_EXTERNAL_VISIBILITY void LLVMInitializeLNP64Target() {
  RegisterTargetMachine<LNP64TargetMachine> X(getTheLNP64Target());
  LLVMInitializeLNP64AsmPrinter();
}

static Reloc::Model getEffectiveRelocModel(Optional<Reloc::Model> RM) {
  return RM.getValueOr(Reloc::PIC_);
}

LNP64TargetMachine::LNP64TargetMachine(
    const Target &T, const Triple &TT, StringRef CPU, StringRef FS,
    const TargetOptions &Options, Optional<Reloc::Model> RM,
    Optional<CodeModel::Model> CM, CodeGenOpt::Level OL, bool)
    : LLVMTargetMachine(T, "e-m:e-p:64:64-i64:64-n64-S128", TT, CPU, FS,
                        Options, getEffectiveRelocModel(RM),
                        getEffectiveCodeModel(CM, CodeModel::Small), OL),
      TLOF(std::make_unique<TargetLoweringObjectFileELF>()),
      Subtarget(TT, CPU, FS, *this) {
  initAsmInfo();
}

TargetPassConfig *LNP64TargetMachine::createPassConfig(PassManagerBase &PM) {
  return new LNP64PassConfig(*this, PM);
}

bool LNP64PassConfig::addInstSelector() {
  addPass(createLNP64ISelDag(getLNP64TargetMachine(), getOptLevel()));
  return false;
}

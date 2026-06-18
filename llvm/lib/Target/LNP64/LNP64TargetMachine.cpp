#include "LNP64TargetMachine.h"
#include "LNP64.h"
#include "llvm/CodeGen/TargetPassConfig.h"
#include "llvm/MC/TargetRegistry.h"
#include "llvm/Support/Compiler.h"

using namespace llvm;

extern "C" LLVM_EXTERNAL_VISIBILITY void LLVMInitializeLNP64Target() {
  RegisterTargetMachine<LNP64TargetMachine> X(getTheLNP64Target());
}

LNP64TargetMachine::LNP64TargetMachine(
    const Target &T, const Triple &TT, StringRef CPU, StringRef FS,
    const TargetOptions &Options, std::optional<Reloc::Model> RM,
    std::optional<CodeModel::Model> CM, CodeGenOpt::Level OL, bool)
    : LLVMTargetMachine(T, "e-m:e-p:64:64-i64:64-n64-S128", TT, CPU, FS,
                        Options, RM, CM, OL),
      Subtarget(TT, CPU, FS, *this) {}

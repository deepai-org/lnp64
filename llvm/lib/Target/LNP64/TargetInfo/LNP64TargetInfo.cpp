#include "llvm/MC/TargetRegistry.h"
#include "llvm/Support/Compiler.h"

using namespace llvm;

Target &llvm::getTheLNP64Target() {
  static Target TheLNP64Target;
  return TheLNP64Target;
}

extern "C" LLVM_EXTERNAL_VISIBILITY void LLVMInitializeLNP64TargetInfo() {
  RegisterTarget<Triple::lnp64> X(getTheLNP64Target(), "lnp64", "LNP64",
                                  "LNP64");
}

#ifndef LLVM_LIB_TARGET_LNP64_LNP64_H
#define LLVM_LIB_TARGET_LNP64_LNP64_H

#include "MCTargetDesc/LNP64MCTargetDesc.h"
#include "llvm/Target/TargetMachine.h"

namespace llvm {

class FunctionPass;
class LNP64TargetMachine;

FunctionPass *createLNP64ISelDag(LNP64TargetMachine &TM,
                                 CodeGenOpt::Level OptLevel);

} // end namespace llvm

#endif

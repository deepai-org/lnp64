#ifndef LLVM_LIB_TARGET_LNP64_MCTARGETDESC_LNP64MCTARGETDESC_H
#define LLVM_LIB_TARGET_LNP64_MCTARGETDESC_LNP64MCTARGETDESC_H

#include "llvm/MC/MCInstrInfo.h"
#include "llvm/MC/MCRegisterInfo.h"
#include "llvm/Support/DataTypes.h"

namespace llvm {

class MCCodeEmitter;
class MCContext;
class MCInstrInfo;
class MCRegisterInfo;
class MCSubtargetInfo;
class MCTargetOptions;
class Target;

Target &getTheLNP64Target();

MCCodeEmitter *createLNP64MCCodeEmitter(const MCInstrInfo &MCII,
                                        MCContext &Ctx);

} // end namespace llvm

#define GET_REGINFO_ENUM
#include "LNP64GenRegisterInfo.inc"

#define GET_INSTRINFO_ENUM
#include "LNP64GenInstrInfo.inc"

#endif

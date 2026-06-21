#ifndef LLVM_LIB_TARGET_LNP64_MCTARGETDESC_LNP64MCTARGETDESC_H
#define LLVM_LIB_TARGET_LNP64_MCTARGETDESC_LNP64MCTARGETDESC_H

#include "llvm/MC/MCInstrInfo.h"
#include "llvm/MC/MCFixup.h"
#include "llvm/MC/MCRegisterInfo.h"
#include "llvm/Support/DataTypes.h"

namespace llvm {

class MCCodeEmitter;
class MCContext;
class MCInstrInfo;
class MCRegisterInfo;
class MCAsmBackend;
class MCSubtargetInfo;
class MCTargetOptions;
class Target;

Target &getTheLNP64Target();

namespace LNP64 {
enum Fixups {
  // B-type compare-and-branch: imm32 field at bit 9, PC-relative, the stored
  // value is (S - PC) >> 3 (instruction-count offset).
  fixup_lnp64_branch = FirstTargetFixupKind,
  // J-type jump/jal: imm32 field at bit 19, PC-relative, value (S - PC) >> 3.
  fixup_lnp64_jump,
  // U-type AUIPC: imm32 field at bit 19, PC-relative, byte granularity.
  fixup_lnp64_auipc,
  // I-type JALR low / call-target absolute helpers (byte granularity).
  fixup_lnp64_abs32,
  fixup_lnp64_pcrel32,
  fixup_lnp64_tls_tprel_slot64,
  LastTargetFixupKind,
  NumTargetFixupKinds = LastTargetFixupKind - FirstTargetFixupKind
};
} // end namespace LNP64

MCCodeEmitter *createLNP64MCCodeEmitter(const MCInstrInfo &MCII,
                                        const MCRegisterInfo &MRI,
                                        MCContext &Ctx);
MCAsmBackend *createLNP64AsmBackend(const Target &T,
                                    const MCSubtargetInfo &STI,
                                    const MCRegisterInfo &MRI,
                                    const MCTargetOptions &Options);

} // end namespace llvm

#define GET_REGINFO_ENUM
#include "LNP64GenRegisterInfo.inc"

#define GET_INSTRINFO_ENUM
#include "LNP64GenInstrInfo.inc"

#endif

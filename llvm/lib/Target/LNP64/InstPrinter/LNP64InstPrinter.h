#ifndef LLVM_LIB_TARGET_LNP64_INSTPRINTER_LNP64INSTPRINTER_H
#define LLVM_LIB_TARGET_LNP64_INSTPRINTER_LNP64INSTPRINTER_H

#include "llvm/MC/MCInstPrinter.h"

namespace llvm {

class LNP64InstPrinter : public MCInstPrinter {
public:
  LNP64InstPrinter(const MCAsmInfo &MAI, const MCInstrInfo &MII,
                   const MCRegisterInfo &MRI)
      : MCInstPrinter(MAI, MII, MRI) {}

  void printRegName(raw_ostream &OS, MCRegister Reg) const override;
  void printInst(const MCInst *MI, uint64_t Address, StringRef Annot,
                 const MCSubtargetInfo &STI, raw_ostream &OS) override;

private:
  void printOperand(const MCOperand &Operand, raw_ostream &OS) const;
  void printMemOperand(const MCInst *MI, unsigned RegOp, unsigned BaseOp,
                       unsigned OffsetOp, raw_ostream &OS) const;
};

MCInstPrinter *createLNP64MCInstPrinter(const Triple &T, unsigned SyntaxVariant,
                                        const MCAsmInfo &MAI,
                                        const MCInstrInfo &MII,
                                        const MCRegisterInfo &MRI);

} // end namespace llvm

#endif

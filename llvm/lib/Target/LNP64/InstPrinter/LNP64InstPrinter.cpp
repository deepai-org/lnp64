//===-- LNP64InstPrinter.cpp - v2 instruction printer --------------------===//
//
// Thin wrapper over the TableGen-generated AsmWriter (LNP64GenAsmWriter.inc).
// Mnemonics, operand order, and the mov/li/ret InstAliases all come from the
// .td AsmStrings; this file only provides operand/register printing hooks.
//===----------------------------------------------------------------------===//

#include "LNP64InstPrinter.h"
#include "MCTargetDesc/LNP64MCTargetDesc.h"
#include "llvm/MC/MCAsmInfo.h"
#include "llvm/MC/MCExpr.h"
#include "llvm/MC/MCInst.h"
#include "llvm/MC/MCInstrInfo.h"
#include "llvm/MC/MCRegisterInfo.h"
#include "llvm/MC/MCSubtargetInfo.h"
#include "llvm/Support/raw_ostream.h"

using namespace llvm;

#define PRINT_ALIAS_INSTR
#include "LNP64GenAsmWriter.inc"

void LNP64InstPrinter::printRegName(raw_ostream &OS, unsigned Reg) const {
  OS << getRegisterName(Reg);
}

void LNP64InstPrinter::printOperand(const MCInst *MI, unsigned OpNo,
                                    raw_ostream &O) {
  const MCOperand &Op = MI->getOperand(OpNo);
  if (Op.isReg()) {
    O << getRegisterName(Op.getReg());
    return;
  }
  if (Op.isImm()) {
    O << Op.getImm();
    return;
  }
  Op.getExpr()->print(O, &MAI);
}

void LNP64InstPrinter::printInst(const MCInst *MI, uint64_t Address,
                                 StringRef Annot, const MCSubtargetInfo &,
                                 raw_ostream &O) {
  if (!printAliasInstr(MI, Address, O))
    printInstruction(MI, Address, O);
  printAnnotation(O, Annot);
}

MCInstPrinter *llvm::createLNP64MCInstPrinter(const Triple &, unsigned,
                                              const MCAsmInfo &MAI,
                                              const MCInstrInfo &MII,
                                              const MCRegisterInfo &MRI) {
  return new LNP64InstPrinter(MAI, MII, MRI);
}

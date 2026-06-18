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

static const char *getLNP64Mnemonic(unsigned Opcode) {
  switch (Opcode) {
  case LNP64::ADD:
    return "add";
  case LNP64::SUB:
    return "sub";
  case LNP64::MUL:
    return "mul";
  case LNP64::DIV:
    return "div";
  case LNP64::AND:
    return "and";
  case LNP64::OR:
    return "or";
  case LNP64::XOR:
    return "xor";
  case LNP64::LSL:
    return "lsl";
  case LNP64::LSR:
    return "lsr";
  case LNP64::ASR:
    return "asr";
  case LNP64::NOT:
    return "not";
  case LNP64::CMP:
    return "cmp";
  case LNP64::JMP:
    return "jmp";
  case LNP64::BEQ:
    return "beq";
  case LNP64::BNE:
    return "bne";
  case LNP64::BLT:
    return "blt";
  case LNP64::BGT:
    return "bgt";
  case LNP64::BLE:
    return "ble";
  case LNP64::BGE:
    return "bge";
  case LNP64::CALL:
    return "call";
  case LNP64::ERRNO_GET:
    return "errno_get";
  case LNP64::ERRNO_SET:
    return "errno_set";
  case LNP64::EXIT:
    return "exit";
  case LNP64::LD:
    return "ld";
  case LNP64::LD_W:
    return "ld.w";
  case LNP64::LD_H:
    return "ld.h";
  case LNP64::LD_B:
    return "ld.b";
  case LNP64::ST:
    return "st";
  case LNP64::ST_W:
    return "st.w";
  case LNP64::ST_H:
    return "st.h";
  case LNP64::ST_B:
    return "st.b";
  case LNP64::LA:
    return "la";
  default:
    return "";
  }
}

std::pair<const char *, uint64_t>
LNP64InstPrinter::getMnemonic(const MCInst *MI) {
  return std::make_pair(getLNP64Mnemonic(MI->getOpcode()), 0);
}

void LNP64InstPrinter::printRegName(raw_ostream &OS, unsigned Reg) const {
  if (Reg >= LNP64::R0 && Reg <= LNP64::R31) {
    OS << "r" << unsigned(Reg - LNP64::R0);
    return;
  }
  if (Reg == LNP64::LR) {
    OS << "lr";
    return;
  }
  if (Reg == LNP64::TP) {
    OS << "tp";
    return;
  }
  OS << MRI.getName(Reg);
}

void LNP64InstPrinter::printOperand(const MCOperand &Operand,
                                    raw_ostream &OS) const {
  if (Operand.isReg()) {
    printRegName(OS, Operand.getReg());
    return;
  }
  if (Operand.isImm()) {
    OS << Operand.getImm();
    return;
  }
  Operand.getExpr()->print(OS, &MAI);
}

void LNP64InstPrinter::printMemOperand(const MCInst *MI, unsigned RegOp,
                                       unsigned BaseOp, unsigned OffsetOp,
                                       raw_ostream &OS) const {
  printOperand(MI->getOperand(RegOp), OS);
  OS << ", ";
  printOperand(MI->getOperand(OffsetOp), OS);
  OS << '(';
  printOperand(MI->getOperand(BaseOp), OS);
  OS << ')';
}

void LNP64InstPrinter::printInst(const MCInst *MI, uint64_t, StringRef Annot,
                                 const MCSubtargetInfo &, raw_ostream &OS) {
  switch (MI->getOpcode()) {
  case LNP64::NOP:
    OS << "nop";
    break;
  case LNP64::RET:
    OS << "ret";
    break;
  case LNP64::LI:
    OS << "li ";
    printOperand(MI->getOperand(0), OS);
    OS << ", ";
    printOperand(MI->getOperand(1), OS);
    break;
  case LNP64::MOV:
    OS << "mov ";
    printOperand(MI->getOperand(0), OS);
    OS << ", ";
    printOperand(MI->getOperand(1), OS);
    break;
  case LNP64::LA:
    OS << "la ";
    printOperand(MI->getOperand(0), OS);
    OS << ", ";
    printOperand(MI->getOperand(1), OS);
    break;
  case LNP64::ADD:
  case LNP64::SUB:
  case LNP64::MUL:
  case LNP64::DIV:
  case LNP64::AND:
  case LNP64::OR:
  case LNP64::XOR:
  case LNP64::LSL:
  case LNP64::LSR:
  case LNP64::ASR:
    OS << getLNP64Mnemonic(MI->getOpcode()) << ' ';
    printOperand(MI->getOperand(0), OS);
    OS << ", ";
    printOperand(MI->getOperand(1), OS);
    OS << ", ";
    printOperand(MI->getOperand(2), OS);
    break;
  case LNP64::NOT:
  case LNP64::CMP:
    OS << getLNP64Mnemonic(MI->getOpcode()) << ' ';
    printOperand(MI->getOperand(0), OS);
    OS << ", ";
    printOperand(MI->getOperand(1), OS);
    break;
  case LNP64::JMP:
  case LNP64::BEQ:
  case LNP64::BNE:
  case LNP64::BLT:
  case LNP64::BGT:
  case LNP64::BLE:
  case LNP64::BGE:
  case LNP64::CALL:
    OS << getLNP64Mnemonic(MI->getOpcode()) << ' ';
    printOperand(MI->getOperand(0), OS);
    break;
  case LNP64::CALL_REG:
    OS << "call_reg ";
    printOperand(MI->getOperand(0), OS);
    break;
  case LNP64::ERRNO_GET:
  case LNP64::ERRNO_SET:
  case LNP64::EXIT:
    OS << getLNP64Mnemonic(MI->getOpcode()) << ' ';
    printOperand(MI->getOperand(0), OS);
    break;
  case LNP64::LD:
  case LNP64::LD_W:
  case LNP64::LD_H:
  case LNP64::LD_B:
  case LNP64::ST:
  case LNP64::ST_W:
  case LNP64::ST_H:
  case LNP64::ST_B:
    OS << getLNP64Mnemonic(MI->getOpcode()) << ' ';
    printMemOperand(MI, 0, 1, 2, OS);
    break;
  default:
    OS << "<unknown lnp64 opcode " << MI->getOpcode() << ">";
    break;
  }
  printAnnotation(OS, Annot);
}

MCInstPrinter *llvm::createLNP64MCInstPrinter(const Triple &, unsigned,
                                              const MCAsmInfo &MAI,
                                              const MCInstrInfo &MII,
                                              const MCRegisterInfo &MRI) {
  return new LNP64InstPrinter(MAI, MII, MRI);
}

#include "LNP64MCTargetDesc.h"
#include "llvm/MC/MCCodeEmitter.h"
#include "llvm/MC/MCFixup.h"
#include "llvm/MC/MCInst.h"
#include "llvm/Support/ErrorHandling.h"
#include "llvm/Support/MathExtras.h"
#include "llvm/Support/raw_ostream.h"

using namespace llvm;

namespace {

static void emitLE32(uint32_t Word, raw_ostream &OS) {
  char Bytes[4] = {static_cast<char>(Word), static_cast<char>(Word >> 8),
                   static_cast<char>(Word >> 16),
                   static_cast<char>(Word >> 24)};
  OS.write(Bytes, sizeof(Bytes));
}

static uint32_t encodeFixed32NoOperand(uint8_t Opcode) {
  return uint32_t(Opcode) << 24;
}

static uint32_t encodeFixed32RI(uint8_t Opcode, unsigned Rd, int64_t Imm) {
  return (uint32_t(Opcode) << 24) | ((Rd & 0x1f) << 19) |
         (uint32_t(Imm) & 0xffff);
}

static uint32_t encodeFixed32R(uint8_t Opcode, unsigned Rd) {
  return (uint32_t(Opcode) << 24) | ((Rd & 0x1f) << 19);
}

static uint32_t encodeFixed32RR(uint8_t Opcode, unsigned Rd, unsigned Rs) {
  return (uint32_t(Opcode) << 24) | ((Rd & 0x1f) << 19) |
         ((Rs & 0x1f) << 14);
}

static uint32_t encodeFixed32RRR(uint8_t Opcode, unsigned Rd, unsigned Rs1,
                                 unsigned Rs2) {
  return (uint32_t(Opcode) << 24) | ((Rd & 0x1f) << 19) |
         ((Rs1 & 0x1f) << 14) | ((Rs2 & 0x1f) << 9);
}

static uint32_t encodeFixed32Mem(uint8_t Opcode, unsigned Reg, unsigned Base,
                                 int64_t Offset) {
  if (!isInt<14>(Offset))
    llvm_unreachable("expected signed-14 LNP64 memory offset");
  return (uint32_t(Opcode) << 24) | ((Reg & 0x1f) << 19) |
         ((Base & 0x1f) << 14) | (uint32_t(Offset) & 0x3fff);
}

static uint32_t encodeFixed32Branch(uint8_t Opcode, int64_t Target) {
  if (Target % 4 != 0)
    llvm_unreachable("expected instruction-aligned LNP64 branch target");
  int64_t Scaled = Target / 4;
  return (uint32_t(Opcode) << 24) | (uint32_t(Scaled) & 0x00ffffff);
}

static uint32_t encodeFixed32Reg(uint8_t Opcode, unsigned Reg) {
  return (uint32_t(Opcode) << 24) | ((Reg & 0x1f) << 19);
}

static uint32_t encodeFixed32BranchOperand(uint8_t Opcode,
                                           const MCOperand &Operand,
                                           SmallVectorImpl<MCFixup> &Fixups) {
  if (Operand.isImm())
    return encodeFixed32Branch(Opcode, Operand.getImm());
  if (Operand.isExpr()) {
    Fixups.push_back(MCFixup::create(
        0, Operand.getExpr(), MCFixupKind(LNP64::fixup_lnp64_branch26)));
    return uint32_t(Opcode) << 24;
  }
  llvm_unreachable("expected immediate or expression branch operand");
}

static void emitFixed32AddressOperand(const MCOperand &Operand, raw_ostream &OS,
                                      SmallVectorImpl<MCFixup> &Fixups) {
  if (Operand.isImm()) {
    emitLE32(static_cast<uint32_t>(Operand.getImm()), OS);
    return;
  }
  if (Operand.isExpr()) {
    Fixups.push_back(MCFixup::create(
        4, Operand.getExpr(), MCFixupKind(LNP64::fixup_lnp64_abs32)));
    emitLE32(0, OS);
    return;
  }
  llvm_unreachable("expected immediate or expression address operand");
}

static unsigned getGPRNo(const MCOperand &Operand) {
  unsigned Reg = Operand.getReg();
  if (Reg < LNP64::R0 || Reg > LNP64::R31)
    llvm_unreachable("expected LNP64 GPR operand");
  return Reg - LNP64::R0;
}

class LNP64MCCodeEmitter final : public MCCodeEmitter {
public:
  void encodeInstruction(const MCInst &MI, raw_ostream &OS,
                         SmallVectorImpl<MCFixup> &Fixups,
                         const MCSubtargetInfo &) const override {
    switch (MI.getOpcode()) {
    case LNP64::NOP:
      emitLE32(encodeFixed32NoOperand(0x00), OS);
      return;
    case LNP64::RET:
      emitLE32(encodeFixed32NoOperand(0x1f), OS);
      return;
    case LNP64::LI:
      emitLE32(encodeFixed32RI(0x01, getGPRNo(MI.getOperand(0)),
                               MI.getOperand(1).getImm()),
               OS);
      return;
    case LNP64::LA:
      emitLE32(encodeFixed32R(0x03, getGPRNo(MI.getOperand(0))), OS);
      emitFixed32AddressOperand(MI.getOperand(1), OS, Fixups);
      return;
    case LNP64::MOV:
      emitLE32(encodeFixed32RR(0x02, getGPRNo(MI.getOperand(0)),
                               getGPRNo(MI.getOperand(1))),
               OS);
      return;
    case LNP64::ADD:
      emitLE32(encodeFixed32RRR(0x10, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::SUB:
      emitLE32(encodeFixed32RRR(0x11, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::MUL:
      emitLE32(encodeFixed32RRR(0x12, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::DIV:
      emitLE32(encodeFixed32RRR(0x13, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::AND:
      emitLE32(encodeFixed32RRR(0x14, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::OR:
      emitLE32(encodeFixed32RRR(0x15, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::XOR:
      emitLE32(encodeFixed32RRR(0x16, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::NOT:
      emitLE32(encodeFixed32RR(0x17, getGPRNo(MI.getOperand(0)),
                               getGPRNo(MI.getOperand(1))),
               OS);
      return;
    case LNP64::LSL:
      emitLE32(encodeFixed32RRR(0x18, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::LSR:
      emitLE32(encodeFixed32RRR(0x19, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::ASR:
      emitLE32(encodeFixed32RRR(0x1a, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::CMP:
      emitLE32(encodeFixed32RR(0x1b, getGPRNo(MI.getOperand(0)),
                               getGPRNo(MI.getOperand(1))),
               OS);
      return;
    case LNP64::JMP:
      emitLE32(encodeFixed32BranchOperand(0x20, MI.getOperand(0), Fixups), OS);
      return;
    case LNP64::BEQ:
      emitLE32(encodeFixed32BranchOperand(0x21, MI.getOperand(0), Fixups), OS);
      return;
    case LNP64::BNE:
      emitLE32(encodeFixed32BranchOperand(0x22, MI.getOperand(0), Fixups), OS);
      return;
    case LNP64::BLT:
      emitLE32(encodeFixed32BranchOperand(0x23, MI.getOperand(0), Fixups), OS);
      return;
    case LNP64::BGT:
      emitLE32(encodeFixed32BranchOperand(0x24, MI.getOperand(0), Fixups), OS);
      return;
    case LNP64::BLE:
      emitLE32(encodeFixed32BranchOperand(0x25, MI.getOperand(0), Fixups), OS);
      return;
    case LNP64::BGE:
      emitLE32(encodeFixed32BranchOperand(0x26, MI.getOperand(0), Fixups), OS);
      return;
    case LNP64::CALL:
      emitLE32(encodeFixed32BranchOperand(0x27, MI.getOperand(0), Fixups), OS);
      return;
    case LNP64::CALL_REG:
      emitLE32(encodeFixed32Reg(0x28, getGPRNo(MI.getOperand(0))), OS);
      return;
    case LNP64::ERRNO_GET:
      emitLE32(encodeFixed32Reg(0x38, getGPRNo(MI.getOperand(0))), OS);
      return;
    case LNP64::ERRNO_SET:
      emitLE32(encodeFixed32Reg(0x39, getGPRNo(MI.getOperand(0))), OS);
      return;
    case LNP64::EXIT:
      emitLE32(encodeFixed32Reg(0x3a, getGPRNo(MI.getOperand(0))), OS);
      return;
    case LNP64::LD:
      emitLE32(encodeFixed32Mem(0x30, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                MI.getOperand(2).getImm()),
               OS);
      return;
    case LNP64::LD_W:
      emitLE32(encodeFixed32Mem(0x31, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                MI.getOperand(2).getImm()),
               OS);
      return;
    case LNP64::LD_B:
      emitLE32(encodeFixed32Mem(0x32, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                MI.getOperand(2).getImm()),
               OS);
      return;
    case LNP64::ST:
      emitLE32(encodeFixed32Mem(0x33, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                MI.getOperand(2).getImm()),
               OS);
      return;
    case LNP64::ST_W:
      emitLE32(encodeFixed32Mem(0x34, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                MI.getOperand(2).getImm()),
               OS);
      return;
    case LNP64::ST_B:
      emitLE32(encodeFixed32Mem(0x35, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                MI.getOperand(2).getImm()),
               OS);
      return;
    case LNP64::LD_H:
      emitLE32(encodeFixed32Mem(0x36, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                MI.getOperand(2).getImm()),
               OS);
      return;
    case LNP64::ST_H:
      emitLE32(encodeFixed32Mem(0x37, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                MI.getOperand(2).getImm()),
               OS);
      return;
    default:
      llvm_unreachable("LNP64 MC encoding for this opcode is not implemented yet");
    }
  }
};

} // end anonymous namespace

MCCodeEmitter *llvm::createLNP64MCCodeEmitter(const MCInstrInfo &,
                                             const MCRegisterInfo &,
                                             MCContext &) {
  return new LNP64MCCodeEmitter();
}

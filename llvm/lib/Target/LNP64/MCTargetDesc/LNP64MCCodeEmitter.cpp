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

static uint32_t encodeFixed32RRI(uint8_t Opcode, unsigned Rd, unsigned Rs,
                                 int64_t Imm) {
  if (!isInt<14>(Imm))
    llvm_unreachable("expected signed-14 LNP64 immediate");
  return (uint32_t(Opcode) << 24) | ((Rd & 0x1f) << 19) |
         ((Rs & 0x1f) << 14) | (uint32_t(Imm) & 0x3fff);
}

static uint32_t encodeFixed32Mem(uint8_t Opcode, unsigned Reg, unsigned Base,
                                 int64_t Offset) {
  if (!isInt<14>(Offset))
    llvm_unreachable("expected signed-14 LNP64 memory offset");
  return (uint32_t(Opcode) << 24) | ((Reg & 0x1f) << 19) |
         ((Base & 0x1f) << 14) | (uint32_t(Offset) & 0x3fff);
}

static uint32_t encodeFixed32Native4(uint8_t Opcode, unsigned Rd,
                                     unsigned Cap, unsigned Arg0,
                                     unsigned Arg1) {
  return (uint32_t(Opcode) << 24) | ((Rd & 0x1f) << 19) |
         ((Cap & 0x1f) << 14) | ((Arg0 & 0x1f) << 9) |
         ((Arg1 & 0x1f) << 4);
}

static uint32_t encodeFixed32RRRR(uint8_t Opcode, unsigned Rd, unsigned Rs1,
                                  unsigned Rs2, unsigned Rs3) {
  return (uint32_t(Opcode) << 24) | ((Rd & 0x1f) << 19) |
         ((Rs1 & 0x1f) << 14) | ((Rs2 & 0x1f) << 9) |
         ((Rs3 & 0x1f) << 4);
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
    case LNP64::LI32:
      emitLE32(encodeFixed32R(0x04, getGPRNo(MI.getOperand(0))), OS);
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
    case LNP64::ADDI:
      emitLE32(encodeFixed32RRI(0xa0, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                MI.getOperand(2).getImm()),
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
    case LNP64::UDIV:
      emitLE32(encodeFixed32RRR(0xa7, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::SREM:
      emitLE32(encodeFixed32RRR(0xa8, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::UREM:
      emitLE32(encodeFixed32RRR(0xa9, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::MULH:
      emitLE32(encodeFixed32RRR(0xaa, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::MULHU:
      emitLE32(encodeFixed32RRR(0xab, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::MULHSU:
      emitLE32(encodeFixed32RRR(0xac, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::AMO_SWAP:
      emitLE32(encodeFixed32RRR(0xc5, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::AMO_ADD:
      emitLE32(encodeFixed32RRR(0xc6, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::AMO_AND:
      emitLE32(encodeFixed32RRR(0xc7, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::AMO_OR:
      emitLE32(encodeFixed32RRR(0xc8, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::LOCK_CMPXCHG:
      emitLE32(encodeFixed32RRRR(0xc9, getGPRNo(MI.getOperand(0)),
                                 getGPRNo(MI.getOperand(1)),
                                 getGPRNo(MI.getOperand(2)),
                                 getGPRNo(MI.getOperand(3))),
               OS);
      return;
    case LNP64::AND:
      emitLE32(encodeFixed32RRR(0x14, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::ANDI:
      emitLE32(encodeFixed32RRI(0xa1, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                MI.getOperand(2).getImm()),
               OS);
      return;
    case LNP64::OR:
      emitLE32(encodeFixed32RRR(0x15, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::ORI:
      emitLE32(encodeFixed32RRI(0xa2, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                MI.getOperand(2).getImm()),
               OS);
      return;
    case LNP64::XOR:
      emitLE32(encodeFixed32RRR(0x16, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::XORI:
      emitLE32(encodeFixed32RRI(0xa3, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                MI.getOperand(2).getImm()),
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
    case LNP64::LSLI:
      emitLE32(encodeFixed32RRI(0xa4, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                MI.getOperand(2).getImm()),
               OS);
      return;
    case LNP64::LSR:
      emitLE32(encodeFixed32RRR(0x19, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::LSRI:
      emitLE32(encodeFixed32RRI(0xa5, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                MI.getOperand(2).getImm()),
               OS);
      return;
    case LNP64::ASR:
      emitLE32(encodeFixed32RRR(0x1a, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::ASRI:
      emitLE32(encodeFixed32RRI(0xa6, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                MI.getOperand(2).getImm()),
               OS);
      return;
    case LNP64::SEXT_B:
      emitLE32(encodeFixed32RR(0xad, getGPRNo(MI.getOperand(0)),
                               getGPRNo(MI.getOperand(1))),
               OS);
      return;
    case LNP64::SEXT_H:
      emitLE32(encodeFixed32RR(0xae, getGPRNo(MI.getOperand(0)),
                               getGPRNo(MI.getOperand(1))),
               OS);
      return;
    case LNP64::SEXT_W:
      emitLE32(encodeFixed32RR(0xaf, getGPRNo(MI.getOperand(0)),
                               getGPRNo(MI.getOperand(1))),
               OS);
      return;
    case LNP64::ZEXT_B:
      emitLE32(encodeFixed32RR(0xb0, getGPRNo(MI.getOperand(0)),
                               getGPRNo(MI.getOperand(1))),
               OS);
      return;
    case LNP64::ZEXT_H:
      emitLE32(encodeFixed32RR(0xb1, getGPRNo(MI.getOperand(0)),
                               getGPRNo(MI.getOperand(1))),
               OS);
      return;
    case LNP64::ZEXT_W:
      emitLE32(encodeFixed32RR(0xb2, getGPRNo(MI.getOperand(0)),
                               getGPRNo(MI.getOperand(1))),
               OS);
      return;
    case LNP64::CLZ:
      emitLE32(encodeFixed32RR(0xb3, getGPRNo(MI.getOperand(0)),
                               getGPRNo(MI.getOperand(1))),
               OS);
      return;
    case LNP64::CTZ:
      emitLE32(encodeFixed32RR(0xb4, getGPRNo(MI.getOperand(0)),
                               getGPRNo(MI.getOperand(1))),
               OS);
      return;
    case LNP64::POPCNT:
      emitLE32(encodeFixed32RR(0xb5, getGPRNo(MI.getOperand(0)),
                               getGPRNo(MI.getOperand(1))),
               OS);
      return;
    case LNP64::ROL:
      emitLE32(encodeFixed32RRR(0xb6, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::ROR:
      emitLE32(encodeFixed32RRR(0xb7, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::BSWAP16:
      emitLE32(encodeFixed32RR(0xb8, getGPRNo(MI.getOperand(0)),
                               getGPRNo(MI.getOperand(1))),
               OS);
      return;
    case LNP64::BSWAP32:
      emitLE32(encodeFixed32RR(0xb9, getGPRNo(MI.getOperand(0)),
                               getGPRNo(MI.getOperand(1))),
               OS);
      return;
    case LNP64::BSWAP64:
      emitLE32(encodeFixed32RR(0xba, getGPRNo(MI.getOperand(0)),
                               getGPRNo(MI.getOperand(1))),
               OS);
      return;
    case LNP64::CSEL_EQ:
      emitLE32(encodeFixed32RRR(0xbb, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::CSEL_NE:
      emitLE32(encodeFixed32RRR(0xbc, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::CSEL_LT:
      emitLE32(encodeFixed32RRR(0xbd, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::CSEL_GT:
      emitLE32(encodeFixed32RRR(0xbe, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::CSEL_LE:
      emitLE32(encodeFixed32RRR(0xbf, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::CSEL_GE:
      emitLE32(encodeFixed32RRR(0xc0, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::CSEL_ULT:
      emitLE32(encodeFixed32RRR(0xc1, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::CSEL_UGT:
      emitLE32(encodeFixed32RRR(0xc2, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::CSEL_ULE:
      emitLE32(encodeFixed32RRR(0xc3, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::CSEL_UGE:
      emitLE32(encodeFixed32RRR(0xc4, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::CMP:
      emitLE32(encodeFixed32RR(0x1b, getGPRNo(MI.getOperand(0)),
                               getGPRNo(MI.getOperand(1))),
               OS);
      return;
    case LNP64::CMPU:
      emitLE32(encodeFixed32RR(0x1c, getGPRNo(MI.getOperand(0)),
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
    case LNP64::PULL:
      emitLE32(encodeFixed32Native4(0x3b, getGPRNo(MI.getOperand(0)),
                                    getGPRNo(MI.getOperand(1)),
                                    getGPRNo(MI.getOperand(2)),
                                    getGPRNo(MI.getOperand(3))),
               OS);
      return;
    case LNP64::PUSH:
      emitLE32(encodeFixed32Native4(0x3c, getGPRNo(MI.getOperand(0)),
                                    getGPRNo(MI.getOperand(1)),
                                    getGPRNo(MI.getOperand(2)),
                                    getGPRNo(MI.getOperand(3))),
               OS);
      return;
    case LNP64::AWAIT:
      emitLE32(encodeFixed32Native4(0x4d, getGPRNo(MI.getOperand(0)),
                                    getGPRNo(MI.getOperand(1)),
                                    getGPRNo(MI.getOperand(2)),
                                    getGPRNo(MI.getOperand(3))),
               OS);
      return;
    case LNP64::GATE_CALL:
      emitLE32(encodeFixed32Native4(0x4e, getGPRNo(MI.getOperand(0)),
                                    getGPRNo(MI.getOperand(1)),
                                    getGPRNo(MI.getOperand(2)),
                                    getGPRNo(MI.getOperand(3))),
               OS);
      return;
    case LNP64::GATE_RETURN:
      emitLE32(encodeFixed32Native4(0x4f, getGPRNo(MI.getOperand(0)),
                                    getGPRNo(MI.getOperand(1)),
                                    getGPRNo(MI.getOperand(2)),
                                    getGPRNo(MI.getOperand(3))),
               OS);
      return;
    case LNP64::CSET_EQ:
      emitLE32(encodeFixed32Reg(0x3d, getGPRNo(MI.getOperand(0))), OS);
      return;
    case LNP64::CSET_NE:
      emitLE32(encodeFixed32Reg(0x3e, getGPRNo(MI.getOperand(0))), OS);
      return;
    case LNP64::CSET_LT:
      emitLE32(encodeFixed32Reg(0x3f, getGPRNo(MI.getOperand(0))), OS);
      return;
    case LNP64::CSET_GT:
      emitLE32(encodeFixed32Reg(0x40, getGPRNo(MI.getOperand(0))), OS);
      return;
    case LNP64::CSET_LE:
      emitLE32(encodeFixed32Reg(0x41, getGPRNo(MI.getOperand(0))), OS);
      return;
    case LNP64::CSET_GE:
      emitLE32(encodeFixed32Reg(0x42, getGPRNo(MI.getOperand(0))), OS);
      return;
    case LNP64::CSET_ULT:
      emitLE32(encodeFixed32Reg(0x43, getGPRNo(MI.getOperand(0))), OS);
      return;
    case LNP64::CSET_UGT:
      emitLE32(encodeFixed32Reg(0x44, getGPRNo(MI.getOperand(0))), OS);
      return;
    case LNP64::CSET_ULE:
      emitLE32(encodeFixed32Reg(0x45, getGPRNo(MI.getOperand(0))), OS);
      return;
    case LNP64::CSET_UGE:
      emitLE32(encodeFixed32Reg(0x46, getGPRNo(MI.getOperand(0))), OS);
      return;
    case LNP64::ALLOC:
      emitLE32(encodeFixed32RR(0x47, getGPRNo(MI.getOperand(0)),
                               getGPRNo(MI.getOperand(1))),
               OS);
      return;
    case LNP64::ALLOC_SIZE:
      emitLE32(encodeFixed32RR(0x48, getGPRNo(MI.getOperand(0)),
                               getGPRNo(MI.getOperand(1))),
               OS);
      return;
    case LNP64::FREE:
      emitLE32(encodeFixed32Reg(0x49, getGPRNo(MI.getOperand(0))), OS);
      return;
    case LNP64::ALLOC_EX:
      emitLE32(encodeFixed32RRR(0x4a, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               OS);
      return;
    case LNP64::OBJECT_CTL:
      emitLE32(encodeFixed32RR(0x4b, getGPRNo(MI.getOperand(0)),
                               getGPRNo(MI.getOperand(1))),
               OS);
      return;
    case LNP64::DOMAIN_CTL:
      emitLE32(encodeFixed32RR(0x4c, getGPRNo(MI.getOperand(0)),
                               getGPRNo(MI.getOperand(1))),
               OS);
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

#include "MCTargetDesc/LNP64MCTargetDesc.h"
#include "llvm/MC/MCDisassembler/MCDisassembler.h"
#include "llvm/MC/MCFixedLenDisassembler.h"
#include "llvm/MC/MCInst.h"
#include "llvm/MC/TargetRegistry.h"
#include "llvm/Support/MathExtras.h"

using namespace llvm;

namespace {

static unsigned getGPR(unsigned Enc) {
  if (Enc > 31)
    return 0;
  return LNP64::R0 + Enc;
}

static uint32_t readLE32(ArrayRef<uint8_t> Bytes) {
  return uint32_t(Bytes[0]) | (uint32_t(Bytes[1]) << 8) |
         (uint32_t(Bytes[2]) << 16) | (uint32_t(Bytes[3]) << 24);
}

static uint32_t readLE32At(ArrayRef<uint8_t> Bytes, unsigned Offset) {
  return uint32_t(Bytes[Offset]) | (uint32_t(Bytes[Offset + 1]) << 8) |
         (uint32_t(Bytes[Offset + 2]) << 16) |
         (uint32_t(Bytes[Offset + 3]) << 24);
}

static void addReg(MCInst &Instr, unsigned Enc) {
  Instr.addOperand(MCOperand::createReg(getGPR(Enc)));
}

static void addImm(MCInst &Instr, int64_t Imm) {
  Instr.addOperand(MCOperand::createImm(Imm));
}

static int64_t decodeBranchTarget(uint32_t Word) {
  return SignExtend64<24>(Word & 0x00ffffff) * 4;
}

class LNP64Disassembler : public MCDisassembler {
public:
  LNP64Disassembler(const MCSubtargetInfo &STI, MCContext &Ctx)
      : MCDisassembler(STI, Ctx) {}

  DecodeStatus getInstruction(MCInst &Instr, uint64_t &Size,
                              ArrayRef<uint8_t> Bytes, uint64_t,
                              raw_ostream &) const override {
    if (Bytes.size() < 4) {
      Size = 0;
      return MCDisassembler::Fail;
    }

    uint32_t Word = readLE32(Bytes);
    uint8_t Opcode = Word >> 24;
    unsigned A = (Word >> 19) & 0x1f;
    unsigned B = (Word >> 14) & 0x1f;
    unsigned C = (Word >> 9) & 0x1f;

    Size = 4;
    switch (Opcode) {
    case 0x00:
      Instr.setOpcode(LNP64::NOP);
      return MCDisassembler::Success;
    case 0x01:
      Instr.setOpcode(LNP64::LI);
      addReg(Instr, A);
      addImm(Instr, SignExtend64<14>(Word & 0x3fff));
      return MCDisassembler::Success;
    case 0x02:
      Instr.setOpcode(LNP64::MOV);
      addReg(Instr, A);
      addReg(Instr, B);
      return MCDisassembler::Success;
    case 0x03:
      if (Bytes.size() < 8) {
        Size = 0;
        return MCDisassembler::Fail;
      }
      Size = 8;
      Instr.setOpcode(LNP64::LA);
      addReg(Instr, A);
      addImm(Instr, readLE32At(Bytes, 4));
      return MCDisassembler::Success;
    case 0x04:
      if (Bytes.size() < 8) {
        Size = 0;
        return MCDisassembler::Fail;
      }
      Size = 8;
      Instr.setOpcode(LNP64::LI32);
      addReg(Instr, A);
      addImm(Instr, readLE32At(Bytes, 4));
      return MCDisassembler::Success;
    case 0x10:
      Instr.setOpcode(LNP64::ADD);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0x11:
      Instr.setOpcode(LNP64::SUB);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0x12:
      Instr.setOpcode(LNP64::MUL);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0x13:
      Instr.setOpcode(LNP64::DIV);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0x14:
      Instr.setOpcode(LNP64::AND);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0x15:
      Instr.setOpcode(LNP64::OR);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0x16:
      Instr.setOpcode(LNP64::XOR);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0x17:
      Instr.setOpcode(LNP64::NOT);
      addReg(Instr, A);
      addReg(Instr, B);
      return MCDisassembler::Success;
    case 0x18:
      Instr.setOpcode(LNP64::LSL);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0x19:
      Instr.setOpcode(LNP64::LSR);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0x1a:
      Instr.setOpcode(LNP64::ASR);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0x1b:
      Instr.setOpcode(LNP64::CMP);
      addReg(Instr, A);
      addReg(Instr, B);
      return MCDisassembler::Success;
    case 0x1c:
      Instr.setOpcode(LNP64::CMPU);
      addReg(Instr, A);
      addReg(Instr, B);
      return MCDisassembler::Success;
    case 0x1f:
      Instr.setOpcode(LNP64::RET);
      return MCDisassembler::Success;
    case 0x20:
      Instr.setOpcode(LNP64::JMP);
      addImm(Instr, decodeBranchTarget(Word));
      return MCDisassembler::Success;
    case 0x21:
      Instr.setOpcode(LNP64::BEQ);
      addImm(Instr, decodeBranchTarget(Word));
      return MCDisassembler::Success;
    case 0x22:
      Instr.setOpcode(LNP64::BNE);
      addImm(Instr, decodeBranchTarget(Word));
      return MCDisassembler::Success;
    case 0x23:
      Instr.setOpcode(LNP64::BLT);
      addImm(Instr, decodeBranchTarget(Word));
      return MCDisassembler::Success;
    case 0x24:
      Instr.setOpcode(LNP64::BGT);
      addImm(Instr, decodeBranchTarget(Word));
      return MCDisassembler::Success;
    case 0x25:
      Instr.setOpcode(LNP64::BLE);
      addImm(Instr, decodeBranchTarget(Word));
      return MCDisassembler::Success;
    case 0x26:
      Instr.setOpcode(LNP64::BGE);
      addImm(Instr, decodeBranchTarget(Word));
      return MCDisassembler::Success;
    case 0x27:
      Instr.setOpcode(LNP64::CALL);
      addImm(Instr, decodeBranchTarget(Word));
      return MCDisassembler::Success;
    case 0x28:
      Instr.setOpcode(LNP64::CALL_REG);
      addReg(Instr, A);
      return MCDisassembler::Success;
    case 0x29:
      Instr.setOpcode(LNP64::LR_GET);
      addReg(Instr, A);
      return MCDisassembler::Success;
    case 0x2a:
      Instr.setOpcode(LNP64::LR_SET);
      addReg(Instr, A);
      return MCDisassembler::Success;
    case 0xa0:
      Instr.setOpcode(LNP64::ADDI);
      addReg(Instr, A);
      addReg(Instr, B);
      addImm(Instr, SignExtend64<14>(Word & 0x3fff));
      return MCDisassembler::Success;
    case 0xa1:
      Instr.setOpcode(LNP64::ANDI);
      addReg(Instr, A);
      addReg(Instr, B);
      addImm(Instr, SignExtend64<14>(Word & 0x3fff));
      return MCDisassembler::Success;
    case 0xa2:
      Instr.setOpcode(LNP64::ORI);
      addReg(Instr, A);
      addReg(Instr, B);
      addImm(Instr, SignExtend64<14>(Word & 0x3fff));
      return MCDisassembler::Success;
    case 0xa3:
      Instr.setOpcode(LNP64::XORI);
      addReg(Instr, A);
      addReg(Instr, B);
      addImm(Instr, SignExtend64<14>(Word & 0x3fff));
      return MCDisassembler::Success;
    case 0xa4:
      Instr.setOpcode(LNP64::LSLI);
      addReg(Instr, A);
      addReg(Instr, B);
      addImm(Instr, SignExtend64<14>(Word & 0x3fff));
      return MCDisassembler::Success;
    case 0xa5:
      Instr.setOpcode(LNP64::LSRI);
      addReg(Instr, A);
      addReg(Instr, B);
      addImm(Instr, SignExtend64<14>(Word & 0x3fff));
      return MCDisassembler::Success;
    case 0xa6:
      Instr.setOpcode(LNP64::ASRI);
      addReg(Instr, A);
      addReg(Instr, B);
      addImm(Instr, SignExtend64<14>(Word & 0x3fff));
      return MCDisassembler::Success;
    case 0xa7:
      Instr.setOpcode(LNP64::UDIV);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0xa8:
      Instr.setOpcode(LNP64::SREM);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0xa9:
      Instr.setOpcode(LNP64::UREM);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0xaa:
      Instr.setOpcode(LNP64::MULH);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0xab:
      Instr.setOpcode(LNP64::MULHU);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0xac:
      Instr.setOpcode(LNP64::MULHSU);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0xc5:
      Instr.setOpcode(LNP64::AMO_SWAP);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0xc6:
      Instr.setOpcode(LNP64::AMO_ADD);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0xc7:
      Instr.setOpcode(LNP64::AMO_AND);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0xc8:
      Instr.setOpcode(LNP64::AMO_OR);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0xca:
      Instr.setOpcode(LNP64::AMO_XOR);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0xc9:
      Instr.setOpcode(LNP64::LOCK_CMPXCHG);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      addReg(Instr, (Word >> 4) & 0x1f);
      return MCDisassembler::Success;
    case 0xcb:
      Instr.setOpcode(LNP64::FUTEX_WAIT);
      addReg(Instr, A);
      addReg(Instr, B);
      return MCDisassembler::Success;
    case 0xcc:
      Instr.setOpcode(LNP64::FUTEX_WAKE);
      addReg(Instr, A);
      addReg(Instr, B);
      return MCDisassembler::Success;
    case 0xad:
      Instr.setOpcode(LNP64::SEXT_B);
      addReg(Instr, A);
      addReg(Instr, B);
      return MCDisassembler::Success;
    case 0xae:
      Instr.setOpcode(LNP64::SEXT_H);
      addReg(Instr, A);
      addReg(Instr, B);
      return MCDisassembler::Success;
    case 0xaf:
      Instr.setOpcode(LNP64::SEXT_W);
      addReg(Instr, A);
      addReg(Instr, B);
      return MCDisassembler::Success;
    case 0xb0:
      Instr.setOpcode(LNP64::ZEXT_B);
      addReg(Instr, A);
      addReg(Instr, B);
      return MCDisassembler::Success;
    case 0xb1:
      Instr.setOpcode(LNP64::ZEXT_H);
      addReg(Instr, A);
      addReg(Instr, B);
      return MCDisassembler::Success;
    case 0xb2:
      Instr.setOpcode(LNP64::ZEXT_W);
      addReg(Instr, A);
      addReg(Instr, B);
      return MCDisassembler::Success;
    case 0xb3:
      Instr.setOpcode(LNP64::CLZ);
      addReg(Instr, A);
      addReg(Instr, B);
      return MCDisassembler::Success;
    case 0xb4:
      Instr.setOpcode(LNP64::CTZ);
      addReg(Instr, A);
      addReg(Instr, B);
      return MCDisassembler::Success;
    case 0xb5:
      Instr.setOpcode(LNP64::POPCNT);
      addReg(Instr, A);
      addReg(Instr, B);
      return MCDisassembler::Success;
    case 0xb6:
      Instr.setOpcode(LNP64::ROL);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0xb7:
      Instr.setOpcode(LNP64::ROR);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0xb8:
      Instr.setOpcode(LNP64::BSWAP16);
      addReg(Instr, A);
      addReg(Instr, B);
      return MCDisassembler::Success;
    case 0xb9:
      Instr.setOpcode(LNP64::BSWAP32);
      addReg(Instr, A);
      addReg(Instr, B);
      return MCDisassembler::Success;
    case 0xba:
      Instr.setOpcode(LNP64::BSWAP64);
      addReg(Instr, A);
      addReg(Instr, B);
      return MCDisassembler::Success;
    case 0xbb:
      Instr.setOpcode(LNP64::CSEL_EQ);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0xbc:
      Instr.setOpcode(LNP64::CSEL_NE);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0xbd:
      Instr.setOpcode(LNP64::CSEL_LT);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0xbe:
      Instr.setOpcode(LNP64::CSEL_GT);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0xbf:
      Instr.setOpcode(LNP64::CSEL_LE);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0xc0:
      Instr.setOpcode(LNP64::CSEL_GE);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0xc1:
      Instr.setOpcode(LNP64::CSEL_ULT);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0xc2:
      Instr.setOpcode(LNP64::CSEL_UGT);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0xc3:
      Instr.setOpcode(LNP64::CSEL_ULE);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0xc4:
      Instr.setOpcode(LNP64::CSEL_UGE);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0x38:
      Instr.setOpcode(LNP64::ERRNO_GET);
      addReg(Instr, A);
      return MCDisassembler::Success;
    case 0x39:
      Instr.setOpcode(LNP64::ERRNO_SET);
      addReg(Instr, A);
      return MCDisassembler::Success;
    case 0x3a:
      Instr.setOpcode(LNP64::EXIT);
      addReg(Instr, A);
      return MCDisassembler::Success;
    case 0x3b:
      Instr.setOpcode(LNP64::PULL);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      addReg(Instr, (Word >> 4) & 0x1f);
      return MCDisassembler::Success;
    case 0x3c:
      Instr.setOpcode(LNP64::PUSH);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      addReg(Instr, (Word >> 4) & 0x1f);
      return MCDisassembler::Success;
    case 0x3d:
      Instr.setOpcode(LNP64::CSET_EQ);
      addReg(Instr, A);
      return MCDisassembler::Success;
    case 0x3e:
      Instr.setOpcode(LNP64::CSET_NE);
      addReg(Instr, A);
      return MCDisassembler::Success;
    case 0x3f:
      Instr.setOpcode(LNP64::CSET_LT);
      addReg(Instr, A);
      return MCDisassembler::Success;
    case 0x40:
      Instr.setOpcode(LNP64::CSET_GT);
      addReg(Instr, A);
      return MCDisassembler::Success;
    case 0x41:
      Instr.setOpcode(LNP64::CSET_LE);
      addReg(Instr, A);
      return MCDisassembler::Success;
    case 0x42:
      Instr.setOpcode(LNP64::CSET_GE);
      addReg(Instr, A);
      return MCDisassembler::Success;
    case 0x43:
      Instr.setOpcode(LNP64::CSET_ULT);
      addReg(Instr, A);
      return MCDisassembler::Success;
    case 0x44:
      Instr.setOpcode(LNP64::CSET_UGT);
      addReg(Instr, A);
      return MCDisassembler::Success;
    case 0x45:
      Instr.setOpcode(LNP64::CSET_ULE);
      addReg(Instr, A);
      return MCDisassembler::Success;
    case 0x46:
      Instr.setOpcode(LNP64::CSET_UGE);
      addReg(Instr, A);
      return MCDisassembler::Success;
    case 0x47:
      Instr.setOpcode(LNP64::ALLOC);
      addReg(Instr, A);
      addReg(Instr, B);
      return MCDisassembler::Success;
    case 0x48:
      Instr.setOpcode(LNP64::ALLOC_SIZE);
      addReg(Instr, A);
      addReg(Instr, B);
      return MCDisassembler::Success;
    case 0x49:
      Instr.setOpcode(LNP64::FREE);
      addReg(Instr, A);
      return MCDisassembler::Success;
    case 0x4a:
      Instr.setOpcode(LNP64::ALLOC_EX);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      return MCDisassembler::Success;
    case 0x4b:
      Instr.setOpcode(LNP64::OBJECT_CTL);
      addReg(Instr, A);
      addReg(Instr, B);
      return MCDisassembler::Success;
    case 0x4c:
      Instr.setOpcode(LNP64::DOMAIN_CTL);
      addReg(Instr, A);
      addReg(Instr, B);
      return MCDisassembler::Success;
    case 0x4d:
      Instr.setOpcode(LNP64::AWAIT);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      addReg(Instr, (Word >> 4) & 0x1f);
      return MCDisassembler::Success;
    case 0x4e:
      Instr.setOpcode(LNP64::GATE_CALL);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      addReg(Instr, (Word >> 4) & 0x1f);
      return MCDisassembler::Success;
    case 0x4f:
      Instr.setOpcode(LNP64::GATE_RETURN);
      addReg(Instr, A);
      addReg(Instr, B);
      addReg(Instr, C);
      addReg(Instr, (Word >> 4) & 0x1f);
      return MCDisassembler::Success;
    case 0x30:
      Instr.setOpcode(LNP64::LD);
      addReg(Instr, A);
      addReg(Instr, B);
      addImm(Instr, SignExtend64<14>(Word & 0x3fff));
      return MCDisassembler::Success;
    case 0x31:
      Instr.setOpcode(LNP64::LD_W);
      addReg(Instr, A);
      addReg(Instr, B);
      addImm(Instr, SignExtend64<14>(Word & 0x3fff));
      return MCDisassembler::Success;
    case 0x32:
      Instr.setOpcode(LNP64::LD_B);
      addReg(Instr, A);
      addReg(Instr, B);
      addImm(Instr, SignExtend64<14>(Word & 0x3fff));
      return MCDisassembler::Success;
    case 0x33:
      Instr.setOpcode(LNP64::ST);
      addReg(Instr, A);
      addReg(Instr, B);
      addImm(Instr, SignExtend64<14>(Word & 0x3fff));
      return MCDisassembler::Success;
    case 0x34:
      Instr.setOpcode(LNP64::ST_W);
      addReg(Instr, A);
      addReg(Instr, B);
      addImm(Instr, SignExtend64<14>(Word & 0x3fff));
      return MCDisassembler::Success;
    case 0x35:
      Instr.setOpcode(LNP64::ST_B);
      addReg(Instr, A);
      addReg(Instr, B);
      addImm(Instr, SignExtend64<14>(Word & 0x3fff));
      return MCDisassembler::Success;
    case 0x36:
      Instr.setOpcode(LNP64::LD_H);
      addReg(Instr, A);
      addReg(Instr, B);
      addImm(Instr, SignExtend64<14>(Word & 0x3fff));
      return MCDisassembler::Success;
    case 0x37:
      Instr.setOpcode(LNP64::ST_H);
      addReg(Instr, A);
      addReg(Instr, B);
      addImm(Instr, SignExtend64<14>(Word & 0x3fff));
      return MCDisassembler::Success;
    default:
      return MCDisassembler::Fail;
    }
  }
};

} // end anonymous namespace

static MCDisassembler *createLNP64Disassembler(const Target &,
                                               const MCSubtargetInfo &STI,
                                               MCContext &Ctx) {
  return new LNP64Disassembler(STI, Ctx);
}

extern "C" LLVM_EXTERNAL_VISIBILITY void LLVMInitializeLNP64Disassembler() {
  TargetRegistry::RegisterMCDisassembler(getTheLNP64Target(),
                                         createLNP64Disassembler);
}

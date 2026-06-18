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
      addImm(Instr, SignExtend64<16>(Word & 0xffff));
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

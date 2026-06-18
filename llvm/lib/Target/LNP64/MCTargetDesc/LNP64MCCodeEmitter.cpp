#include "LNP64MCTargetDesc.h"
#include "llvm/MC/MCCodeEmitter.h"
#include "llvm/MC/MCFixup.h"
#include "llvm/MC/MCInst.h"
#include "llvm/Support/ErrorHandling.h"

using namespace llvm;

namespace {

static void emitLE32(uint32_t Word, SmallVectorImpl<char> &CB) {
  CB.push_back(static_cast<char>(Word));
  CB.push_back(static_cast<char>(Word >> 8));
  CB.push_back(static_cast<char>(Word >> 16));
  CB.push_back(static_cast<char>(Word >> 24));
}

static uint32_t encodeFixed32NoOperand(uint8_t Opcode) {
  return uint32_t(Opcode) << 24;
}

static uint32_t encodeFixed32RI(uint8_t Opcode, unsigned Rd, int64_t Imm) {
  return (uint32_t(Opcode) << 24) | ((Rd & 0x1f) << 19) |
         (uint32_t(Imm) & 0xffff);
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
  return (uint32_t(Opcode) << 24) | ((Reg & 0x1f) << 19) |
         ((Base & 0x1f) << 14) | (uint32_t(Offset) & 0x3fff);
}

static unsigned getGPRNo(const MCOperand &Operand) {
  unsigned Reg = Operand.getReg();
  if (Reg < LNP64::R0 || Reg > LNP64::R31)
    llvm_unreachable("expected LNP64 GPR operand");
  return Reg - LNP64::R0;
}

class LNP64MCCodeEmitter final : public MCCodeEmitter {
public:
  void encodeInstruction(const MCInst &MI, SmallVectorImpl<char> &CB,
                         SmallVectorImpl<MCFixup> &,
                         const MCSubtargetInfo &) const override {
    switch (MI.getOpcode()) {
    case LNP64::NOP:
      emitLE32(encodeFixed32NoOperand(0x00), CB);
      return;
    case LNP64::RET:
      emitLE32(encodeFixed32NoOperand(0x1f), CB);
      return;
    case LNP64::LI:
      emitLE32(encodeFixed32RI(0x01, getGPRNo(MI.getOperand(0)),
                               MI.getOperand(1).getImm()),
               CB);
      return;
    case LNP64::MOV:
      emitLE32(encodeFixed32RR(0x02, getGPRNo(MI.getOperand(0)),
                               getGPRNo(MI.getOperand(1))),
               CB);
      return;
    case LNP64::ADD:
      emitLE32(encodeFixed32RRR(0x10, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               CB);
      return;
    case LNP64::SUB:
      emitLE32(encodeFixed32RRR(0x11, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               CB);
      return;
    case LNP64::MUL:
      emitLE32(encodeFixed32RRR(0x12, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               CB);
      return;
    case LNP64::DIV:
      emitLE32(encodeFixed32RRR(0x13, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               CB);
      return;
    case LNP64::AND:
      emitLE32(encodeFixed32RRR(0x14, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               CB);
      return;
    case LNP64::OR:
      emitLE32(encodeFixed32RRR(0x15, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               CB);
      return;
    case LNP64::XOR:
      emitLE32(encodeFixed32RRR(0x16, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               CB);
      return;
    case LNP64::NOT:
      emitLE32(encodeFixed32RR(0x17, getGPRNo(MI.getOperand(0)),
                               getGPRNo(MI.getOperand(1))),
               CB);
      return;
    case LNP64::LSL:
      emitLE32(encodeFixed32RRR(0x18, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               CB);
      return;
    case LNP64::LSR:
      emitLE32(encodeFixed32RRR(0x19, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               CB);
      return;
    case LNP64::ASR:
      emitLE32(encodeFixed32RRR(0x1a, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                getGPRNo(MI.getOperand(2))),
               CB);
      return;
    case LNP64::CMP:
      emitLE32(encodeFixed32RR(0x1b, getGPRNo(MI.getOperand(0)),
                               getGPRNo(MI.getOperand(1))),
               CB);
      return;
    case LNP64::LD:
      emitLE32(encodeFixed32Mem(0x30, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                MI.getOperand(2).getImm()),
               CB);
      return;
    case LNP64::LD_W:
      emitLE32(encodeFixed32Mem(0x31, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                MI.getOperand(2).getImm()),
               CB);
      return;
    case LNP64::LD_B:
      emitLE32(encodeFixed32Mem(0x32, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                MI.getOperand(2).getImm()),
               CB);
      return;
    case LNP64::ST:
      emitLE32(encodeFixed32Mem(0x33, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                MI.getOperand(2).getImm()),
               CB);
      return;
    case LNP64::ST_W:
      emitLE32(encodeFixed32Mem(0x34, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                MI.getOperand(2).getImm()),
               CB);
      return;
    case LNP64::ST_B:
      emitLE32(encodeFixed32Mem(0x35, getGPRNo(MI.getOperand(0)),
                                getGPRNo(MI.getOperand(1)),
                                MI.getOperand(2).getImm()),
               CB);
      return;
    default:
      llvm_unreachable("LNP64 MC encoding for this opcode is not implemented yet");
    }
  }
};

} // end anonymous namespace

MCCodeEmitter *llvm::createLNP64MCCodeEmitter(const MCInstrInfo &,
                                             MCContext &) {
  return new LNP64MCCodeEmitter();
}

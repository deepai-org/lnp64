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

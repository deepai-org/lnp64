#include "MCTargetDesc/LNP64MCTargetDesc.h"
#include "llvm/MC/MCDisassembler/MCDisassembler.h"
#include "llvm/MC/MCFixedLenDisassembler.h"
#include "llvm/MC/MCInst.h"
#include "llvm/MC/TargetRegistry.h"
#include "llvm/Support/MemoryObject.h"

using namespace llvm;

namespace {

class LNP64Disassembler : public MCDisassembler {
public:
  LNP64Disassembler(const MCSubtargetInfo &STI, MCContext &Ctx)
      : MCDisassembler(STI, Ctx) {}

  DecodeStatus getInstruction(MCInst &, uint64_t &Size, ArrayRef<uint8_t>,
                              uint64_t, raw_ostream &) const override {
    Size = 0;
    return MCDisassembler::Fail;
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

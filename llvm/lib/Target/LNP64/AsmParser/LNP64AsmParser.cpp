#include "MCTargetDesc/LNP64MCTargetDesc.h"
#include "llvm/MC/MCContext.h"
#include "llvm/MC/MCInst.h"
#include "llvm/MC/MCParser/MCAsmParser.h"
#include "llvm/MC/MCParser/MCParsedAsmOperand.h"
#include "llvm/MC/MCParser/MCTargetAsmParser.h"
#include "llvm/MC/MCStreamer.h"
#include "llvm/MC/TargetRegistry.h"
#include "llvm/Support/SMLoc.h"

using namespace llvm;

namespace {

class LNP64AsmParser : public MCTargetAsmParser {
public:
  LNP64AsmParser(const MCSubtargetInfo &STI, MCAsmParser &Parser,
                 const MCInstrInfo &MII, const MCTargetOptions &Options)
      : MCTargetAsmParser(Options, STI, MII) {
    setAvailableFeatures(ComputeAvailableFeatures(STI.getFeatureBits()));
  }

  bool ParseRegister(unsigned &, SMLoc &, SMLoc &) override {
    return true;
  }

  bool ParseInstruction(ParseInstructionInfo &, StringRef, SMLoc NameLoc,
                        OperandVector &) override {
    return Error(NameLoc, "LNP64 assembly parser is scaffolded");
  }

  bool MatchAndEmitInstruction(SMLoc IDLoc, unsigned &, OperandVector &,
                               MCStreamer &, uint64_t &, bool) override {
    return Error(IDLoc, "LNP64 instruction matching is not implemented yet");
  }
};

} // end anonymous namespace

extern "C" LLVM_EXTERNAL_VISIBILITY void LLVMInitializeLNP64AsmParser() {
  RegisterMCAsmParser<LNP64AsmParser> X(getTheLNP64Target());
}

#include "LNP64MCTargetDesc.h"
#include "llvm/MC/MCCodeEmitter.h"
#include "llvm/MC/MCFixup.h"
#include "llvm/MC/MCInst.h"
#include "llvm/Support/ErrorHandling.h"

using namespace llvm;

namespace {

class LNP64MCCodeEmitter final : public MCCodeEmitter {
public:
  void encodeInstruction(const MCInst &, SmallVectorImpl<char> &,
                         SmallVectorImpl<MCFixup> &,
                         const MCSubtargetInfo &) const override {
    llvm_unreachable("LNP64 MC encoding is scaffolded but not implemented yet");
  }
};

} // end anonymous namespace

MCCodeEmitter *llvm::createLNP64MCCodeEmitter(const MCInstrInfo &,
                                             MCContext &) {
  return new LNP64MCCodeEmitter();
}

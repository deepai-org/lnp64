#include "LNP64.h"
#include "LNP64TargetMachine.h"
#include "llvm/CodeGen/AsmPrinter.h"
#include "llvm/CodeGen/MachineBasicBlock.h"
#include "llvm/CodeGen/MachineInstr.h"
#include "llvm/CodeGen/MachineOperand.h"
#include "llvm/MC/MCExpr.h"
#include "llvm/MC/MCInst.h"
#include "llvm/MC/MCStreamer.h"
#include "llvm/MC/MCSymbol.h"
#include "llvm/MC/TargetRegistry.h"
#include "llvm/Support/Compiler.h"
#include "llvm/Support/ErrorHandling.h"
#include <memory>

using namespace llvm;

namespace {

class LNP64AsmPrinter final : public AsmPrinter {
public:
  LNP64AsmPrinter(TargetMachine &TM, std::unique_ptr<MCStreamer> Streamer)
      : AsmPrinter(TM, std::move(Streamer)) {}

  StringRef getPassName() const override { return "LNP64 Assembly Printer"; }

  void emitInstruction(const MachineInstr *MI) override;

private:
  bool lowerOperand(const MachineOperand &MO, MCOperand &Out) const;
};

static const MCExpr *lowerSymbolOperand(MCContext &Context,
                                        const MachineOperand &MO,
                                        MCSymbol *Symbol) {
  const MCExpr *Expr = MCSymbolRefExpr::create(Symbol, Context);
  if (MO.getOffset() == 0)
    return Expr;

  return MCBinaryExpr::createAdd(
      Expr, MCConstantExpr::create(MO.getOffset(), Context), Context);
}

bool LNP64AsmPrinter::lowerOperand(const MachineOperand &MO,
                                   MCOperand &Out) const {
  switch (MO.getType()) {
  case MachineOperand::MO_Register:
    if (MO.isImplicit())
      return false;
    Out = MCOperand::createReg(MO.getReg());
    return true;
  case MachineOperand::MO_Immediate:
    Out = MCOperand::createImm(MO.getImm());
    return true;
  case MachineOperand::MO_MachineBasicBlock:
    Out = MCOperand::createExpr(
        MCSymbolRefExpr::create(MO.getMBB()->getSymbol(), OutContext));
    return true;
  case MachineOperand::MO_GlobalAddress:
    Out = MCOperand::createExpr(
        lowerSymbolOperand(OutContext, MO, getSymbol(MO.getGlobal())));
    return true;
  case MachineOperand::MO_ExternalSymbol:
    Out = MCOperand::createExpr(lowerSymbolOperand(
        OutContext, MO, GetExternalSymbolSymbol(MO.getSymbolName())));
    return true;
  case MachineOperand::MO_RegisterMask:
    return false;
  default:
    llvm_unreachable("unsupported LNP64 MachineOperand kind for MC lowering");
  }
}

void LNP64AsmPrinter::emitInstruction(const MachineInstr *MI) {
  if (MI->isDebugInstr())
    return;

  MCInst Inst;
  Inst.setOpcode(MI->getOpcode());
  for (const MachineOperand &MO : MI->operands()) {
    MCOperand Lowered;
    if (lowerOperand(MO, Lowered))
      Inst.addOperand(Lowered);
  }
  EmitToStreamer(*OutStreamer, Inst);
}

} // end anonymous namespace

extern "C" LLVM_EXTERNAL_VISIBILITY void LLVMInitializeLNP64AsmPrinter() {
  RegisterAsmPrinter<LNP64AsmPrinter> X(getTheLNP64Target());
}

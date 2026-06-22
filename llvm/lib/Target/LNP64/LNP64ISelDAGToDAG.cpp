#include "LNP64.h"
#include "LNP64TargetMachine.h"
#include "llvm/CodeGen/SelectionDAGISel.h"
#include "llvm/CodeGen/SelectionDAGNodes.h"
#include "llvm/Support/Compiler.h"
#include "llvm/Support/Debug.h"

using namespace llvm;

#define DEBUG_TYPE "lnp64-isel"

namespace {

class LNP64DAGToDAGISel final : public SelectionDAGISel {
public:
  LNP64DAGToDAGISel(LNP64TargetMachine &TM, CodeGenOpt::Level OptLevel)
      : SelectionDAGISel(TM, OptLevel) {}

  StringRef getPassName() const override {
    return "LNP64 DAG->DAG Pattern Instruction Selection";
  }

  void Select(SDNode *Node) override;
  bool SelectFrameIndexValue(SDNode *Node);
  // ComplexPattern: match a frame-index address (bare `frameindex` or
  // `frameindex + const`) into (base = TargetFrameIndex, offset). Lets the
  // load/store TableGen patterns fold a constant displacement into the
  // instruction immediate -- the generic eliminateFrameIndex path then resolves
  // base -> r31 and the offset. Replaces the hand-written per-opcode
  // load/store frame-index selectors.
  bool SelectFrameAddr(SDValue Addr, SDValue &Base, SDValue &Offset);

#include "LNP64GenDAGISel.inc"
};

} // end anonymous namespace

bool LNP64DAGToDAGISel::SelectFrameAddr(SDValue Addr, SDValue &Base,
                                        SDValue &Offset) {
  SDLoc DL(Addr);
  if (auto *FI = dyn_cast<FrameIndexSDNode>(Addr)) {
    Base = CurDAG->getTargetFrameIndex(FI->getIndex(), MVT::i64);
    Offset = CurDAG->getTargetConstant(0, DL, MVT::i64);
    return true;
  }
  if (Addr.getOpcode() == ISD::ADD)
    if (auto *FI = dyn_cast<FrameIndexSDNode>(Addr.getOperand(0)))
      if (auto *C = dyn_cast<ConstantSDNode>(Addr.getOperand(1)))
        if (isInt<32>(C->getSExtValue())) {
          Base = CurDAG->getTargetFrameIndex(FI->getIndex(), MVT::i64);
          Offset = CurDAG->getTargetConstant(C->getSExtValue(), DL, MVT::i64);
          return true;
        }
  return false;
}

bool LNP64DAGToDAGISel::SelectFrameIndexValue(SDNode *Node) {
  auto *FI = dyn_cast<FrameIndexSDNode>(Node);
  if (!FI)
    return false;

  SDLoc DL(Node);
  SDValue Base = CurDAG->getTargetFrameIndex(FI->getIndex(), MVT::i64);
  SDValue Offset = CurDAG->getTargetConstant(0, DL, MVT::i64);
  // Select a bare frame-index value to `addi rd, <fi>, 0`. The frame-index
  // operand is resolved by the generic eliminateFrameIndex path (rs1 -> r31,
  // imm -> resolved offset), so no dedicated pseudo is needed. Use SelectNodeTo
  // (in-place mutation) rather than getMachineNode + ReplaceNode: the latter
  // left dead, self-referential nodes (`%x = OP %x, 0`) for address-of values
  // that turned out unused -- those have a *register* base, so PEI's isFI()
  // check skipped them and they reached the encoder unexpanded.
  CurDAG->SelectNodeTo(Node, LNP64::ADDI, MVT::i64, Base, Offset);
  return true;
}

void LNP64DAGToDAGISel::Select(SDNode *Node) {
  if (Node->isMachineOpcode()) {
    Node->setNodeId(-1);
    return;
  }

  // Frame-index loads/stores are matched declaratively via the FrameAddr
  // ComplexPattern (see LNP64InstrInfo.td); only a bare frame-index *value*
  // (address-of) needs the in-place ADDI selection here.
  if (SelectFrameIndexValue(Node))
    return;

  SelectCode(Node);
}

FunctionPass *llvm::createLNP64ISelDag(LNP64TargetMachine &TM,
                                       CodeGenOpt::Level OptLevel) {
  return new LNP64DAGToDAGISel(TM, OptLevel);
}

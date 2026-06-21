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
  bool SelectFrameIndexLoad(SDNode *Node);
  bool SelectFrameIndexStore(SDNode *Node);

#include "LNP64GenDAGISel.inc"
};

} // end anonymous namespace

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

bool LNP64DAGToDAGISel::SelectFrameIndexLoad(SDNode *Node) {
  auto *Load = dyn_cast<LoadSDNode>(Node);
  if (!Load)
    return false;

  auto *FI = dyn_cast<FrameIndexSDNode>(Load->getBasePtr());
  if (!FI)
    return false;

  unsigned Opcode;
  EVT MemVT = Load->getMemoryVT();
  if (MemVT == MVT::i64 && Load->getExtensionType() == ISD::NON_EXTLOAD)
    Opcode = LNP64::LD;
  else if (MemVT == MVT::i32 && Load->getExtensionType() == ISD::SEXTLOAD)
    Opcode = LNP64::LW;
  else if (MemVT == MVT::i16 && Load->getExtensionType() == ISD::SEXTLOAD)
    Opcode = LNP64::LH;
  else if (MemVT == MVT::i8 && Load->getExtensionType() == ISD::SEXTLOAD)
    Opcode = LNP64::LB;
  else if (MemVT == MVT::i32 &&
           (Load->getExtensionType() == ISD::ZEXTLOAD ||
            Load->getExtensionType() == ISD::EXTLOAD))
    Opcode = LNP64::LWU;
  else if (MemVT == MVT::i16 &&
           (Load->getExtensionType() == ISD::ZEXTLOAD ||
            Load->getExtensionType() == ISD::EXTLOAD))
    Opcode = LNP64::LHU;
  else if ((MemVT == MVT::i1 || MemVT == MVT::i8) &&
           (Load->getExtensionType() == ISD::ZEXTLOAD ||
            Load->getExtensionType() == ISD::EXTLOAD))
    Opcode = LNP64::LBU;
  else
    return false;

  SDLoc DL(Node);
  SDValue Base = CurDAG->getTargetFrameIndex(FI->getIndex(), MVT::i64);
  SDValue Offset = CurDAG->getTargetConstant(0, DL, MVT::i64);
  SDValue Chain = Load->getChain();
  SDNode *Selected =
      CurDAG->getMachineNode(Opcode, DL, CurDAG->getVTList(MVT::i64, MVT::Other),
                             {Base, Offset, Chain});
  ReplaceNode(Node, Selected);
  return true;
}

bool LNP64DAGToDAGISel::SelectFrameIndexStore(SDNode *Node) {
  auto *Store = dyn_cast<StoreSDNode>(Node);
  if (!Store)
    return false;

  auto *FI = dyn_cast<FrameIndexSDNode>(Store->getBasePtr());
  if (!FI)
    return false;

  unsigned Opcode;
  EVT MemVT = Store->getMemoryVT();
  if (MemVT == MVT::i64 && !Store->isTruncatingStore())
    Opcode = LNP64::SD;
  else if (MemVT == MVT::i32 && Store->isTruncatingStore())
    Opcode = LNP64::SW;
  else if (MemVT == MVT::i16 && Store->isTruncatingStore())
    Opcode = LNP64::SH;
  else if ((MemVT == MVT::i1 || MemVT == MVT::i8) && Store->isTruncatingStore())
    Opcode = LNP64::SB;
  else
    return false;

  SDLoc DL(Node);
  SDValue Base = CurDAG->getTargetFrameIndex(FI->getIndex(), MVT::i64);
  SDValue Offset = CurDAG->getTargetConstant(0, DL, MVT::i64);
  SDValue Value = Store->getValue();
  SDValue Chain = Store->getChain();
  SDNode *Selected = CurDAG->getMachineNode(Opcode, DL, MVT::Other,
                                            {Value, Base, Offset, Chain});
  ReplaceNode(Node, Selected);
  return true;
}

void LNP64DAGToDAGISel::Select(SDNode *Node) {
  if (Node->isMachineOpcode()) {
    Node->setNodeId(-1);
    return;
  }

  if (SelectFrameIndexLoad(Node) || SelectFrameIndexStore(Node) ||
      SelectFrameIndexValue(Node))
    return;

  SelectCode(Node);
}

FunctionPass *llvm::createLNP64ISelDag(LNP64TargetMachine &TM,
                                       CodeGenOpt::Level OptLevel) {
  return new LNP64DAGToDAGISel(TM, OptLevel);
}

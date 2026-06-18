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
  SDNode *Selected =
      CurDAG->getMachineNode(LNP64::PseudoFRAMEADDR, DL, MVT::i64,
                             {Base, Offset});
  ReplaceNode(Node, Selected);
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
    Opcode = LNP64::PseudoLD_SW;
  else if (MemVT == MVT::i16 && Load->getExtensionType() == ISD::SEXTLOAD)
    Opcode = LNP64::PseudoLD_SH;
  else if (MemVT == MVT::i8 && Load->getExtensionType() == ISD::SEXTLOAD)
    Opcode = LNP64::PseudoLD_SB;
  else if (MemVT == MVT::i32 &&
           (Load->getExtensionType() == ISD::ZEXTLOAD ||
            Load->getExtensionType() == ISD::EXTLOAD))
    Opcode = LNP64::LD_W;
  else if (MemVT == MVT::i16 &&
           (Load->getExtensionType() == ISD::ZEXTLOAD ||
            Load->getExtensionType() == ISD::EXTLOAD))
    Opcode = LNP64::LD_H;
  else if (MemVT == MVT::i8 &&
           (Load->getExtensionType() == ISD::ZEXTLOAD ||
            Load->getExtensionType() == ISD::EXTLOAD))
    Opcode = LNP64::LD_B;
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
    Opcode = LNP64::ST;
  else if (MemVT == MVT::i32 && Store->isTruncatingStore())
    Opcode = LNP64::ST_W;
  else if (MemVT == MVT::i16 && Store->isTruncatingStore())
    Opcode = LNP64::ST_H;
  else if (MemVT == MVT::i8 && Store->isTruncatingStore())
    Opcode = LNP64::ST_B;
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

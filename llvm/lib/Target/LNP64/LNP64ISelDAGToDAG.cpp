#include "LNP64.h"
#include "LNP64TargetMachine.h"
#include "llvm/CodeGen/SelectionDAGISel.h"
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

#include "LNP64GenDAGISel.inc"
};

} // end anonymous namespace

void LNP64DAGToDAGISel::Select(SDNode *Node) {
  if (Node->isMachineOpcode()) {
    Node->setNodeId(-1);
    return;
  }

  SelectCode(Node);
}

FunctionPass *llvm::createLNP64ISelDag(LNP64TargetMachine &TM,
                                       CodeGenOpt::Level OptLevel) {
  return new LNP64DAGToDAGISel(TM, OptLevel);
}

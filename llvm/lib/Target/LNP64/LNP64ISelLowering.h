#ifndef LLVM_LIB_TARGET_LNP64_LNP64ISELLOWERING_H
#define LLVM_LIB_TARGET_LNP64_LNP64ISELLOWERING_H

#include "llvm/CodeGen/TargetLowering.h"

namespace llvm {

class MachineBasicBlock;
class MachineInstr;
class LNP64Subtarget;

namespace LNP64ISD {
enum NodeType : unsigned {
  FIRST_NUMBER = ISD::BUILTIN_OP_END,
  CALL,
  AWAIT,
  DOMAIN_CTL,
  GATE_CALL,
  GATE_RETURN,
  OBJECT_CTL,
  PULL,
  PUSH,
  WRAPPER,
  SELECT_CC,
  RET_FLAG,
};
}

class LNP64TargetLowering : public TargetLowering {
public:
  explicit LNP64TargetLowering(const TargetMachine &TM,
                               const LNP64Subtarget &STI);

  const char *getTargetNodeName(unsigned Opcode) const override;
  AtomicExpansionKind
  shouldExpandAtomicRMWInIR(AtomicRMWInst *RMW) const override;
  AtomicExpansionKind
  shouldExpandAtomicCmpXchgInIR(AtomicCmpXchgInst *CI) const override;
  ConstraintType getConstraintType(StringRef Constraint) const override;
  std::pair<unsigned, const TargetRegisterClass *>
  getRegForInlineAsmConstraint(const TargetRegisterInfo *TRI,
                               StringRef Constraint, MVT VT) const override;
  SDValue LowerOperation(SDValue Op, SelectionDAG &DAG) const override;
  void ReplaceNodeResults(SDNode *N, SmallVectorImpl<SDValue> &Results,
                          SelectionDAG &DAG) const override;
  MachineBasicBlock *
  EmitInstrWithCustomInserter(MachineInstr &MI,
                              MachineBasicBlock *BB) const override;

  SDValue LowerFormalArguments(SDValue Chain, CallingConv::ID CallConv,
                               bool IsVarArg,
                               const SmallVectorImpl<ISD::InputArg> &Ins,
                               const SDLoc &DL, SelectionDAG &DAG,
                               SmallVectorImpl<SDValue> &InVals) const override;

  SDValue LowerReturn(SDValue Chain, CallingConv::ID CallConv, bool IsVarArg,
                      const SmallVectorImpl<ISD::OutputArg> &Outs,
                      const SmallVectorImpl<SDValue> &OutVals, const SDLoc &DL,
                      SelectionDAG &DAG) const override;

  SDValue LowerCall(CallLoweringInfo &CLI,
                    SmallVectorImpl<SDValue> &InVals) const override;
};

} // end namespace llvm

#endif

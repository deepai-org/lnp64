#include "LNP64ISelLowering.h"
#include "LNP64Subtarget.h"
#include "llvm/CodeGen/CallingConvLower.h"
#include "llvm/CodeGen/MachineFunction.h"
#include "llvm/CodeGen/SelectionDAG.h"
#include "llvm/Support/ErrorHandling.h"

using namespace llvm;

#include "LNP64GenCallingConv.inc"

LNP64TargetLowering::LNP64TargetLowering(const TargetMachine &TM,
                                         const LNP64Subtarget &STI)
    : TargetLowering(TM) {
  addRegisterClass(MVT::i64, &LNP64::GPRRegClass);
  setStackPointerRegisterToSaveRestore(LNP64::R31);
  setBooleanContents(ZeroOrOneBooleanContent);

  for (unsigned Opcode : {ISD::ADD, ISD::SUB, ISD::MUL, ISD::SDIV, ISD::AND,
                          ISD::OR, ISD::XOR, ISD::SHL, ISD::SRL, ISD::SRA})
    setOperationAction(Opcode, MVT::i64, Legal);

  setOperationAction(ISD::UDIV, MVT::i64, Expand);
  setOperationAction(ISD::UREM, MVT::i64, Expand);
  setOperationAction(ISD::SREM, MVT::i64, Expand);
  computeRegisterProperties(STI.getRegisterInfo());
}

const char *LNP64TargetLowering::getTargetNodeName(unsigned Opcode) const {
  switch (Opcode) {
  case LNP64ISD::RET_FLAG:
    return "LNP64ISD::RET_FLAG";
  default:
    return nullptr;
  }
}

SDValue LNP64TargetLowering::LowerFormalArguments(
    SDValue Chain, CallingConv::ID CallConv, bool IsVarArg,
    const SmallVectorImpl<ISD::InputArg> &Ins, const SDLoc &DL,
    SelectionDAG &DAG, SmallVectorImpl<SDValue> &InVals) const {
  if (IsVarArg)
    llvm_unreachable("LNP64 varargs lowering is not implemented yet");

  MachineFunction &MF = DAG.getMachineFunction();
  SmallVector<CCValAssign, 8> ArgLocs;
  CCState CCInfo(CallConv, IsVarArg, MF, ArgLocs, *DAG.getContext());
  CCInfo.AnalyzeFormalArguments(Ins, CC_LNP64);

  for (CCValAssign &VA : ArgLocs) {
    if (!VA.isRegLoc())
      llvm_unreachable("LNP64 stack argument lowering is not implemented yet");

    Register VReg = MF.addLiveIn(VA.getLocReg(), &LNP64::GPRRegClass);
    SDValue Arg = DAG.getCopyFromReg(Chain, DL, VReg, VA.getLocVT());
    InVals.push_back(Arg);
  }

  return Chain;
}

SDValue LNP64TargetLowering::LowerReturn(
    SDValue Chain, CallingConv::ID CallConv, bool IsVarArg,
    const SmallVectorImpl<ISD::OutputArg> &Outs,
    const SmallVectorImpl<SDValue> &OutVals, const SDLoc &DL,
    SelectionDAG &DAG) const {
  if (IsVarArg)
    llvm_unreachable("LNP64 varargs return lowering is not implemented yet");

  MachineFunction &MF = DAG.getMachineFunction();
  SmallVector<CCValAssign, 4> RetLocs;
  CCState CCInfo(CallConv, IsVarArg, MF, RetLocs, *DAG.getContext());
  CCInfo.AnalyzeReturn(Outs, RetCC_LNP64);

  SDValue Glue;
  SmallVector<SDValue, 4> RetOps(1, Chain);
  for (unsigned I = 0, E = RetLocs.size(); I != E; ++I) {
    CCValAssign &VA = RetLocs[I];
    if (!VA.isRegLoc())
      llvm_unreachable("LNP64 stack return lowering is not implemented yet");

    Chain = DAG.getCopyToReg(Chain, DL, VA.getLocReg(), OutVals[I], Glue);
    Glue = Chain.getValue(1);
    RetOps[0] = Chain;
    RetOps.push_back(DAG.getRegister(VA.getLocReg(), VA.getLocVT()));
  }

  if (Glue)
    RetOps.push_back(Glue);
  return DAG.getNode(LNP64ISD::RET_FLAG, DL, MVT::Other, RetOps);
}

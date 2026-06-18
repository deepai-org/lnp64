#include "LNP64ISelLowering.h"
#include "LNP64.h"
#include "LNP64InstrInfo.h"
#include "LNP64Subtarget.h"
#include "llvm/CodeGen/CallingConvLower.h"
#include "llvm/CodeGen/MachineBasicBlock.h"
#include "llvm/CodeGen/MachineFunction.h"
#include "llvm/CodeGen/MachineInstrBuilder.h"
#include "llvm/CodeGen/SelectionDAG.h"
#include "llvm/Support/ErrorHandling.h"

using namespace llvm;

#include "LNP64GenCallingConv.inc"

static StringRef getDirectCalleeName(SDValue Callee) {
  if (GlobalAddressSDNode *G = dyn_cast<GlobalAddressSDNode>(Callee))
    return G->getGlobal()->getName();
  if (ExternalSymbolSDNode *S = dyn_cast<ExternalSymbolSDNode>(Callee))
    return S->getSymbol();
  return StringRef();
}

static unsigned getLNP64BranchOpcode(ISD::CondCode CC) {
  switch (CC) {
  case ISD::SETEQ:
    return LNP64ISD::BR_EQ;
  case ISD::SETNE:
    return LNP64ISD::BR_NE;
  case ISD::SETLT:
    return LNP64ISD::BR_LT;
  case ISD::SETGT:
    return LNP64ISD::BR_GT;
  case ISD::SETLE:
    return LNP64ISD::BR_LE;
  case ISD::SETGE:
    return LNP64ISD::BR_GE;
  default:
    llvm_unreachable(
        "LNP64 conditional branch lowering only supports signed comparisons today");
  }
}

static unsigned getLNP64BranchInstr(unsigned Opcode) {
  switch (Opcode) {
  case LNP64::PseudoBEQ:
    return LNP64::BEQ;
  case LNP64::PseudoBNE:
    return LNP64::BNE;
  case LNP64::PseudoBLT:
    return LNP64::BLT;
  case LNP64::PseudoBGT:
    return LNP64::BGT;
  case LNP64::PseudoBLE:
    return LNP64::BLE;
  case LNP64::PseudoBGE:
    return LNP64::BGE;
  default:
    llvm_unreachable("expected LNP64 conditional branch pseudo");
  }
}

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
  setOperationAction(ISD::BR_CC, MVT::i64, Custom);
  for (MVT MemVT : {MVT::i8, MVT::i16, MVT::i32}) {
    setLoadExtAction(ISD::ZEXTLOAD, MVT::i64, MemVT, Legal);
    setTruncStoreAction(MVT::i64, MemVT, Legal);
  }
  computeRegisterProperties(STI.getRegisterInfo());
}

const char *LNP64TargetLowering::getTargetNodeName(unsigned Opcode) const {
  switch (Opcode) {
  case LNP64ISD::BR_EQ:
    return "LNP64ISD::BR_EQ";
  case LNP64ISD::BR_GE:
    return "LNP64ISD::BR_GE";
  case LNP64ISD::BR_GT:
    return "LNP64ISD::BR_GT";
  case LNP64ISD::BR_LE:
    return "LNP64ISD::BR_LE";
  case LNP64ISD::BR_LT:
    return "LNP64ISD::BR_LT";
  case LNP64ISD::BR_NE:
    return "LNP64ISD::BR_NE";
  case LNP64ISD::CALL:
    return "LNP64ISD::CALL";
  case LNP64ISD::DOMAIN_CTL:
    return "LNP64ISD::DOMAIN_CTL";
  case LNP64ISD::GATE_CALL:
    return "LNP64ISD::GATE_CALL";
  case LNP64ISD::OBJECT_CTL:
    return "LNP64ISD::OBJECT_CTL";
  case LNP64ISD::PULL:
    return "LNP64ISD::PULL";
  case LNP64ISD::PUSH:
    return "LNP64ISD::PUSH";
  case LNP64ISD::RET_FLAG:
    return "LNP64ISD::RET_FLAG";
  default:
    return nullptr;
  }
}

SDValue LNP64TargetLowering::LowerOperation(SDValue Op,
                                            SelectionDAG &DAG) const {
  switch (Op.getOpcode()) {
  case ISD::BR_CC: {
    SDValue Chain = Op.getOperand(0);
    auto *CC = cast<CondCodeSDNode>(Op.getOperand(1));
    SDValue LHS = Op.getOperand(2);
    SDValue RHS = Op.getOperand(3);
    SDValue Target = Op.getOperand(4);
    return DAG.getNode(getLNP64BranchOpcode(CC->get()), SDLoc(Op), MVT::Other,
                       {Chain, LHS, RHS, Target});
  }
  default:
    llvm_unreachable("unsupported LNP64 custom lowering opcode");
  }
}

MachineBasicBlock *LNP64TargetLowering::EmitInstrWithCustomInserter(
    MachineInstr &MI, MachineBasicBlock *BB) const {
  const TargetInstrInfo &TII = *BB->getParent()->getSubtarget().getInstrInfo();
  DebugLoc DL = MI.getDebugLoc();
  unsigned BranchOpcode = getLNP64BranchInstr(MI.getOpcode());

  BuildMI(*BB, MI, DL, TII.get(LNP64::CMP))
      .add(MI.getOperand(0))
      .add(MI.getOperand(1));
  BuildMI(*BB, MI, DL, TII.get(BranchOpcode)).add(MI.getOperand(2));
  MI.eraseFromParent();
  return BB;
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

SDValue
LNP64TargetLowering::LowerCall(CallLoweringInfo &CLI,
                               SmallVectorImpl<SDValue> &InVals) const {
  SelectionDAG &DAG = CLI.DAG;
  SDLoc DL = CLI.DL;
  SDValue Chain = CLI.Chain;
  SDValue Callee = CLI.Callee;

  if (CLI.IsVarArg)
    llvm_unreachable("LNP64 varargs call lowering is not implemented yet");

  StringRef CalleeName = getDirectCalleeName(Callee);
  if (CalleeName == "__lnp_call" || CalleeName == "__lnp_pull" ||
      CalleeName == "__lnp_push") {
    if (CLI.OutVals.size() != 3 || CLI.Ins.empty())
      llvm_unreachable(
          "LNP64 native shim lowering expects three arguments and a result");
    SDVTList NodeTys = DAG.getVTList(MVT::i64, MVT::Other);
    SmallVector<SDValue, 4> Ops = {Chain, CLI.OutVals[0], CLI.OutVals[1],
                                   CLI.OutVals[2]};
    unsigned Opcode = CalleeName == "__lnp_call"    ? LNP64ISD::GATE_CALL
                      : CalleeName == "__lnp_pull" ? LNP64ISD::PULL
                                                   : LNP64ISD::PUSH;
    SDValue NativeShim = DAG.getNode(Opcode, DL, NodeTys, Ops);
    InVals.push_back(NativeShim);
    return NativeShim.getValue(1);
  }
  if (CalleeName == "__lnp_domain_ctl" || CalleeName == "__lnp_object_ctl") {
    if (CLI.OutVals.size() != 1 || CLI.Ins.empty())
      llvm_unreachable(
          "LNP64 native control lowering expects one argument and a result");
    SDVTList NodeTys = DAG.getVTList(MVT::i64, MVT::Other);
    SmallVector<SDValue, 2> Ops = {Chain, CLI.OutVals[0]};
    unsigned Opcode = CalleeName == "__lnp_domain_ctl" ? LNP64ISD::DOMAIN_CTL
                                                       : LNP64ISD::OBJECT_CTL;
    SDValue NativeCtl = DAG.getNode(Opcode, DL, NodeTys, Ops);
    InVals.push_back(NativeCtl);
    return NativeCtl.getValue(1);
  }

  MachineFunction &MF = DAG.getMachineFunction();
  SmallVector<CCValAssign, 8> ArgLocs;
  CCState ArgCCInfo(CLI.CallConv, CLI.IsVarArg, MF, ArgLocs,
                    *DAG.getContext());
  ArgCCInfo.AnalyzeCallOperands(CLI.Outs, CC_LNP64);

  SDValue Glue;
  SmallVector<std::pair<unsigned, SDValue>, 8> RegsToPass;
  for (unsigned I = 0, E = ArgLocs.size(); I != E; ++I) {
    CCValAssign &VA = ArgLocs[I];
    if (!VA.isRegLoc())
      llvm_unreachable("LNP64 stack call arguments are not implemented yet");
    RegsToPass.push_back(std::make_pair(VA.getLocReg(), CLI.OutVals[I]));
  }

  for (auto &RegAndValue : RegsToPass) {
    Chain = DAG.getCopyToReg(Chain, DL, RegAndValue.first, RegAndValue.second,
                             Glue);
    Glue = Chain.getValue(1);
  }

  if (GlobalAddressSDNode *G = dyn_cast<GlobalAddressSDNode>(Callee))
    Callee = DAG.getTargetGlobalAddress(G->getGlobal(), DL, MVT::i64);
  else if (ExternalSymbolSDNode *S = dyn_cast<ExternalSymbolSDNode>(Callee))
    Callee = DAG.getTargetExternalSymbol(S->getSymbol(), MVT::i64);
  else if (Callee.getValueType() != MVT::i64)
    llvm_unreachable("LNP64 indirect call callee must lower to an i64 register");

  SmallVector<SDValue, 12> Ops;
  Ops.push_back(Chain);
  Ops.push_back(Callee);
  for (auto &RegAndValue : RegsToPass)
    Ops.push_back(
        DAG.getRegister(RegAndValue.first, RegAndValue.second.getValueType()));
  if (Glue)
    Ops.push_back(Glue);

  SDVTList NodeTys = DAG.getVTList(MVT::Other, MVT::Glue);
  Chain = DAG.getNode(LNP64ISD::CALL, DL, NodeTys, Ops);
  Glue = Chain.getValue(1);

  SmallVector<CCValAssign, 4> RVLocs;
  CCState RetCCInfo(CLI.CallConv, CLI.IsVarArg, MF, RVLocs,
                    *DAG.getContext());
  RetCCInfo.AnalyzeCallResult(CLI.Ins, RetCC_LNP64);
  for (CCValAssign &VA : RVLocs) {
    if (!VA.isRegLoc())
      llvm_unreachable("LNP64 stack call results are not implemented yet");
    SDValue RetValue = DAG.getCopyFromReg(Chain, DL, VA.getLocReg(),
                                          VA.getLocVT(), Glue);
    Chain = RetValue.getValue(1);
    Glue = RetValue.getValue(2);
    InVals.push_back(RetValue);
  }

  return Chain;
}

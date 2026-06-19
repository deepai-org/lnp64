#include "LNP64ISelLowering.h"
#include "LNP64.h"
#include "LNP64InstrInfo.h"
#include "LNP64Subtarget.h"
#include "llvm/CodeGen/CallingConvLower.h"
#include "llvm/CodeGen/MachineBasicBlock.h"
#include "llvm/CodeGen/MachineFunction.h"
#include "llvm/CodeGen/MachineInstrBuilder.h"
#include "llvm/CodeGen/MachineRegisterInfo.h"
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
  case ISD::SETULT:
    return LNP64ISD::BR_ULT;
  case ISD::SETUGT:
    return LNP64ISD::BR_UGT;
  case ISD::SETULE:
    return LNP64ISD::BR_ULE;
  case ISD::SETUGE:
    return LNP64ISD::BR_UGE;
  default:
    llvm_unreachable(
        "LNP64 conditional branch lowering only supports integer comparisons today");
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
  case LNP64::PseudoBUGE:
    return LNP64::BGE;
  case LNP64::PseudoBULT:
    return LNP64::BLT;
  case LNP64::PseudoBUGT:
    return LNP64::BGT;
  case LNP64::PseudoBULE:
    return LNP64::BLE;
  default:
    llvm_unreachable("expected LNP64 conditional branch pseudo");
  }
}

static bool isLNP64UnsignedBranchPseudo(unsigned Opcode) {
  switch (Opcode) {
  case LNP64::PseudoBULT:
  case LNP64::PseudoBUGT:
  case LNP64::PseudoBULE:
  case LNP64::PseudoBUGE:
    return true;
  default:
    return false;
  }
}

static unsigned getLNP64CSetInstr(unsigned Opcode) {
  switch (Opcode) {
  case LNP64::PseudoCSETEQ:
  case LNP64::PseudoCSETEQI:
    return LNP64::CSET_EQ;
  case LNP64::PseudoCSETNE:
  case LNP64::PseudoCSETNEI:
    return LNP64::CSET_NE;
  case LNP64::PseudoCSETLT:
  case LNP64::PseudoCSETLTI:
    return LNP64::CSET_LT;
  case LNP64::PseudoCSETGT:
  case LNP64::PseudoCSETGTI:
    return LNP64::CSET_GT;
  case LNP64::PseudoCSETLE:
  case LNP64::PseudoCSETLEI:
    return LNP64::CSET_LE;
  case LNP64::PseudoCSETGE:
  case LNP64::PseudoCSETGEI:
    return LNP64::CSET_GE;
  case LNP64::PseudoCSETULT:
  case LNP64::PseudoCSETULTI:
    return LNP64::CSET_ULT;
  case LNP64::PseudoCSETUGT:
  case LNP64::PseudoCSETUGTI:
    return LNP64::CSET_UGT;
  case LNP64::PseudoCSETULE:
  case LNP64::PseudoCSETULEI:
    return LNP64::CSET_ULE;
  case LNP64::PseudoCSETUGE:
  case LNP64::PseudoCSETUGEI:
    return LNP64::CSET_UGE;
  default:
    llvm_unreachable("expected LNP64 setcc pseudo");
  }
}

static unsigned getLNP64CSelectInstr(unsigned Opcode) {
  switch (Opcode) {
  case LNP64::PseudoCSELEQ:
    return LNP64::CSEL_EQ;
  case LNP64::PseudoCSELNE:
    return LNP64::CSEL_NE;
  case LNP64::PseudoCSELLT:
    return LNP64::CSEL_LT;
  case LNP64::PseudoCSELGT:
    return LNP64::CSEL_GT;
  case LNP64::PseudoCSELLE:
    return LNP64::CSEL_LE;
  case LNP64::PseudoCSELGE:
    return LNP64::CSEL_GE;
  case LNP64::PseudoCSELULT:
    return LNP64::CSEL_ULT;
  case LNP64::PseudoCSELUGT:
    return LNP64::CSEL_UGT;
  case LNP64::PseudoCSELULE:
    return LNP64::CSEL_ULE;
  case LNP64::PseudoCSELUGE:
    return LNP64::CSEL_UGE;
  default:
    llvm_unreachable("expected LNP64 selectcc pseudo");
  }
}

static bool isLNP64SetCCImmPseudo(unsigned Opcode) {
  switch (Opcode) {
  case LNP64::PseudoCSETEQI:
  case LNP64::PseudoCSETNEI:
  case LNP64::PseudoCSETLTI:
  case LNP64::PseudoCSETGTI:
  case LNP64::PseudoCSETLEI:
  case LNP64::PseudoCSETGEI:
  case LNP64::PseudoCSETULTI:
  case LNP64::PseudoCSETUGTI:
  case LNP64::PseudoCSETULEI:
  case LNP64::PseudoCSETUGEI:
    return true;
  default:
    return false;
  }
}

static bool isLNP64SetCCPseudo(unsigned Opcode) {
  switch (Opcode) {
  case LNP64::PseudoCSETEQ:
  case LNP64::PseudoCSETNE:
  case LNP64::PseudoCSETLT:
  case LNP64::PseudoCSETGT:
  case LNP64::PseudoCSETLE:
  case LNP64::PseudoCSETGE:
  case LNP64::PseudoCSETEQI:
  case LNP64::PseudoCSETNEI:
  case LNP64::PseudoCSETLTI:
  case LNP64::PseudoCSETGTI:
  case LNP64::PseudoCSETLEI:
  case LNP64::PseudoCSETGEI:
  case LNP64::PseudoCSETULT:
  case LNP64::PseudoCSETUGT:
  case LNP64::PseudoCSETULE:
  case LNP64::PseudoCSETUGE:
  case LNP64::PseudoCSETULTI:
  case LNP64::PseudoCSETUGTI:
  case LNP64::PseudoCSETULEI:
  case LNP64::PseudoCSETUGEI:
    return true;
  default:
    return false;
  }
}

static bool isLNP64SelectCCPseudo(unsigned Opcode) {
  switch (Opcode) {
  case LNP64::PseudoCSELEQ:
  case LNP64::PseudoCSELNE:
  case LNP64::PseudoCSELLT:
  case LNP64::PseudoCSELGT:
  case LNP64::PseudoCSELLE:
  case LNP64::PseudoCSELGE:
  case LNP64::PseudoCSELULT:
  case LNP64::PseudoCSELUGT:
  case LNP64::PseudoCSELULE:
  case LNP64::PseudoCSELUGE:
    return true;
  default:
    return false;
  }
}

static bool isLNP64UnsignedSetCCPseudo(unsigned Opcode) {
  switch (Opcode) {
  case LNP64::PseudoCSETULT:
  case LNP64::PseudoCSETUGT:
  case LNP64::PseudoCSETULE:
  case LNP64::PseudoCSETUGE:
  case LNP64::PseudoCSETULTI:
  case LNP64::PseudoCSETUGTI:
  case LNP64::PseudoCSETULEI:
  case LNP64::PseudoCSETUGEI:
    return true;
  default:
    return false;
  }
}

static bool isLNP64UnsignedSelectCCPseudo(unsigned Opcode) {
  switch (Opcode) {
  case LNP64::PseudoCSELULT:
  case LNP64::PseudoCSELUGT:
  case LNP64::PseudoCSELULE:
  case LNP64::PseudoCSELUGE:
    return true;
  default:
    return false;
  }
}

static bool isLNP64SignedLoadPseudo(unsigned Opcode) {
  switch (Opcode) {
  case LNP64::PseudoLD_SB:
  case LNP64::PseudoLD_SH:
  case LNP64::PseudoLD_SW:
    return true;
  default:
    return false;
  }
}

static unsigned getLNP64SignedLoadInstr(unsigned Opcode) {
  switch (Opcode) {
  case LNP64::PseudoLD_SB:
    return LNP64::LD_B;
  case LNP64::PseudoLD_SH:
    return LNP64::LD_H;
  case LNP64::PseudoLD_SW:
    return LNP64::LD_W;
  default:
    llvm_unreachable("expected LNP64 signed load pseudo");
  }
}

static unsigned getLNP64SignExtendInstr(unsigned Opcode) {
  switch (Opcode) {
  case LNP64::PseudoLD_SB:
    return LNP64::SEXT_B;
  case LNP64::PseudoLD_SH:
    return LNP64::SEXT_H;
  case LNP64::PseudoLD_SW:
    return LNP64::SEXT_W;
  default:
    llvm_unreachable("expected LNP64 signed load pseudo");
  }
}

LNP64TargetLowering::LNP64TargetLowering(const TargetMachine &TM,
                                         const LNP64Subtarget &STI)
    : TargetLowering(TM) {
  addRegisterClass(MVT::i64, &LNP64::GPRRegClass);
  setStackPointerRegisterToSaveRestore(LNP64::R31);
  setBooleanContents(ZeroOrOneBooleanContent);

  for (unsigned Opcode : {ISD::ADD, ISD::SUB, ISD::MUL, ISD::SDIV, ISD::UDIV,
                          ISD::SREM, ISD::UREM, ISD::AND, ISD::OR, ISD::XOR,
                          ISD::SHL, ISD::SRL, ISD::SRA, ISD::MULHS,
                          ISD::MULHU, ISD::CTLZ, ISD::CTLZ_ZERO_UNDEF, ISD::CTTZ,
                          ISD::CTTZ_ZERO_UNDEF, ISD::CTPOP, ISD::ROTL,
                          ISD::ROTR, ISD::BSWAP})
    setOperationAction(Opcode, MVT::i64, Legal);

  for (unsigned Opcode : {ISD::ATOMIC_SWAP, ISD::ATOMIC_LOAD_ADD,
                          ISD::ATOMIC_LOAD_AND, ISD::ATOMIC_LOAD_OR,
                          ISD::ATOMIC_CMP_SWAP})
    setOperationAction(Opcode, MVT::i64, Legal);

  setOperationAction(ISD::ATOMIC_LOAD, MVT::i64, Legal);
  setOperationAction(ISD::ATOMIC_STORE, MVT::i64, Legal);

  setOperationAction(ISD::GlobalAddress, MVT::i64, Custom);
  setOperationAction(ISD::BR_CC, MVT::i64, Custom);
  for (MVT MemVT : {MVT::i8, MVT::i16, MVT::i32}) {
    setLoadExtAction(ISD::ZEXTLOAD, MVT::i64, MemVT, Legal);
    setLoadExtAction(ISD::SEXTLOAD, MVT::i64, MemVT, Legal);
    setLoadExtAction(ISD::EXTLOAD, MVT::i64, MemVT, Legal);
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
  case LNP64ISD::BR_UGE:
    return "LNP64ISD::BR_UGE";
  case LNP64ISD::BR_UGT:
    return "LNP64ISD::BR_UGT";
  case LNP64ISD::BR_ULE:
    return "LNP64ISD::BR_ULE";
  case LNP64ISD::BR_ULT:
    return "LNP64ISD::BR_ULT";
  case LNP64ISD::CALL:
    return "LNP64ISD::CALL";
  case LNP64ISD::AWAIT:
    return "LNP64ISD::AWAIT";
  case LNP64ISD::DOMAIN_CTL:
    return "LNP64ISD::DOMAIN_CTL";
  case LNP64ISD::GATE_CALL:
    return "LNP64ISD::GATE_CALL";
  case LNP64ISD::GATE_RETURN:
    return "LNP64ISD::GATE_RETURN";
  case LNP64ISD::OBJECT_CTL:
    return "LNP64ISD::OBJECT_CTL";
  case LNP64ISD::PULL:
    return "LNP64ISD::PULL";
  case LNP64ISD::PUSH:
    return "LNP64ISD::PUSH";
  case LNP64ISD::WRAPPER:
    return "LNP64ISD::WRAPPER";
  case LNP64ISD::RET_FLAG:
    return "LNP64ISD::RET_FLAG";
  default:
    return nullptr;
  }
}

TargetLowering::ConstraintType
LNP64TargetLowering::getConstraintType(StringRef Constraint) const {
  if (Constraint.size() == 1) {
    switch (Constraint[0]) {
    case 'r':
      return C_RegisterClass;
    case 'm':
      return C_Memory;
    case 'i':
      return C_Immediate;
    default:
      break;
    }
  }
  return TargetLowering::getConstraintType(Constraint);
}

std::pair<unsigned, const TargetRegisterClass *>
LNP64TargetLowering::getRegForInlineAsmConstraint(
    const TargetRegisterInfo *TRI, StringRef Constraint, MVT VT) const {
  if (Constraint == "r")
    return std::make_pair(0U, &LNP64::GPRRegClass);
  return TargetLowering::getRegForInlineAsmConstraint(TRI, Constraint, VT);
}

SDValue LNP64TargetLowering::LowerOperation(SDValue Op,
                                            SelectionDAG &DAG) const {
  switch (Op.getOpcode()) {
  case ISD::GlobalAddress: {
    auto *G = cast<GlobalAddressSDNode>(Op);
    SDLoc DL(Op);
    SDValue Target = DAG.getTargetGlobalAddress(
        G->getGlobal(), DL, getPointerTy(DAG.getDataLayout()), G->getOffset());
    return DAG.getNode(LNP64ISD::WRAPPER, DL,
                       getPointerTy(DAG.getDataLayout()), Target);
  }
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

  if (MI.getOpcode() == LNP64::PseudoLINeg32) {
    MachineFunction *MF = BB->getParent();
    MachineRegisterInfo &MRI = MF->getRegInfo();
    Register Magnitude = MRI.createVirtualRegister(&LNP64::GPRRegClass);
    int64_t Value = MI.getOperand(1).getImm();
    BuildMI(*BB, MI, DL, TII.get(LNP64::LI32), Magnitude).addImm(-Value);
    BuildMI(*BB, MI, DL, TII.get(LNP64::SUB), MI.getOperand(0).getReg())
        .addReg(LNP64::R0)
        .addReg(Magnitude);
    MI.eraseFromParent();
    return BB;
  }

  if (isLNP64SignedLoadPseudo(MI.getOpcode())) {
    MachineFunction *MF = BB->getParent();
    MachineRegisterInfo &MRI = MF->getRegInfo();
    Register Loaded = MRI.createVirtualRegister(&LNP64::GPRRegClass);

    BuildMI(*BB, MI, DL, TII.get(getLNP64SignedLoadInstr(MI.getOpcode())),
            Loaded)
        .add(MI.getOperand(1))
        .add(MI.getOperand(2));
    BuildMI(*BB, MI, DL, TII.get(getLNP64SignExtendInstr(MI.getOpcode())),
            MI.getOperand(0).getReg())
        .addReg(Loaded);
    MI.eraseFromParent();
    return BB;
  }

  if (isLNP64SetCCPseudo(MI.getOpcode())) {
    unsigned CmpOpcode =
        isLNP64UnsignedSetCCPseudo(MI.getOpcode()) ? LNP64::CMPU : LNP64::CMP;
    MachineFunction *MF = BB->getParent();
    MachineRegisterInfo &MRI = MF->getRegInfo();
    if (isLNP64SetCCImmPseudo(MI.getOpcode())) {
      Register RHS = MRI.createVirtualRegister(&LNP64::GPRRegClass);
      BuildMI(*BB, MI, DL, TII.get(LNP64::LI), RHS)
          .addImm(MI.getOperand(2).getImm());
      BuildMI(*BB, MI, DL, TII.get(CmpOpcode))
          .add(MI.getOperand(1))
          .addReg(RHS);
    } else {
      BuildMI(*BB, MI, DL, TII.get(CmpOpcode))
          .add(MI.getOperand(1))
          .add(MI.getOperand(2));
    }
    BuildMI(*BB, MI, DL, TII.get(getLNP64CSetInstr(MI.getOpcode())),
            MI.getOperand(0).getReg());
    MI.eraseFromParent();
    return BB;
  }

  if (isLNP64SelectCCPseudo(MI.getOpcode())) {
    unsigned CmpOpcode =
        isLNP64UnsignedSelectCCPseudo(MI.getOpcode()) ? LNP64::CMPU : LNP64::CMP;
    BuildMI(*BB, MI, DL, TII.get(CmpOpcode))
        .add(MI.getOperand(1))
        .add(MI.getOperand(2));
    BuildMI(*BB, MI, DL, TII.get(getLNP64CSelectInstr(MI.getOpcode())),
            MI.getOperand(0).getReg())
        .add(MI.getOperand(3))
        .add(MI.getOperand(4));
    MI.eraseFromParent();
    return BB;
  }

  unsigned BranchOpcode = getLNP64BranchInstr(MI.getOpcode());
  unsigned CmpOpcode =
      isLNP64UnsignedBranchPseudo(MI.getOpcode()) ? LNP64::CMPU : LNP64::CMP;

  BuildMI(*BB, MI, DL, TII.get(CmpOpcode))
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
    report_fatal_error("LNP64 varargs lowering is not implemented yet");

  MachineFunction &MF = DAG.getMachineFunction();
  SmallVector<CCValAssign, 8> ArgLocs;
  CCState CCInfo(CallConv, IsVarArg, MF, ArgLocs, *DAG.getContext());
  CCInfo.AnalyzeFormalArguments(Ins, CC_LNP64);

  for (CCValAssign &VA : ArgLocs) {
    if (!VA.isRegLoc())
      report_fatal_error(
          "LNP64 stack formal arguments are not implemented yet");

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
    report_fatal_error("LNP64 varargs return lowering is not implemented yet");

  MachineFunction &MF = DAG.getMachineFunction();
  SmallVector<CCValAssign, 4> RetLocs;
  CCState CCInfo(CallConv, IsVarArg, MF, RetLocs, *DAG.getContext());
  CCInfo.AnalyzeReturn(Outs, RetCC_LNP64);

  SDValue Glue;
  SmallVector<SDValue, 4> RetOps(1, Chain);
  for (unsigned I = 0, E = RetLocs.size(); I != E; ++I) {
    CCValAssign &VA = RetLocs[I];
    if (!VA.isRegLoc())
      report_fatal_error("LNP64 stack return lowering is not implemented yet");

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
    report_fatal_error("LNP64 varargs call lowering is not implemented yet");

  StringRef CalleeName = getDirectCalleeName(Callee);
  if (CalleeName == "__lnp_await" || CalleeName == "__lnp_call" ||
      CalleeName == "__lnp_gate_return" || CalleeName == "__lnp_pull" ||
      CalleeName == "__lnp_push") {
    if (CLI.OutVals.size() != 3 || CLI.Ins.empty())
      llvm_unreachable(
          "LNP64 native shim lowering expects three arguments and a result");
    SDVTList NodeTys = DAG.getVTList(MVT::i64, MVT::Other);
    SmallVector<SDValue, 4> Ops = {Chain, CLI.OutVals[0], CLI.OutVals[1],
                                   CLI.OutVals[2]};
    unsigned Opcode = CalleeName == "__lnp_await"   ? LNP64ISD::AWAIT
                      : CalleeName == "__lnp_call"  ? LNP64ISD::GATE_CALL
                      : CalleeName == "__lnp_gate_return"
                          ? LNP64ISD::GATE_RETURN
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
      report_fatal_error(
          "LNP64 stack call arguments are not implemented yet");
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
      report_fatal_error("LNP64 stack call results are not implemented yet");
    SDValue RetValue = DAG.getCopyFromReg(Chain, DL, VA.getLocReg(),
                                          VA.getLocVT(), Glue);
    Chain = RetValue.getValue(1);
    Glue = RetValue.getValue(2);
    InVals.push_back(RetValue);
  }

  return Chain;
}

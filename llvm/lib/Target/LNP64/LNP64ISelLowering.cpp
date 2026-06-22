#include "LNP64ISelLowering.h"
#include "LNP64.h"
#include "LNP64InstrInfo.h"
#include "LNP64Subtarget.h"
#include "llvm/CodeGen/CallingConvLower.h"
#include "llvm/CodeGen/MachineBasicBlock.h"
#include "llvm/CodeGen/MachineFrameInfo.h"
#include "llvm/CodeGen/MachineFunction.h"
#include "llvm/CodeGen/MachineInstrBuilder.h"
#include "llvm/CodeGen/MachineRegisterInfo.h"
#include "llvm/CodeGen/SelectionDAG.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/InlineAsm.h"
#include "llvm/Support/ErrorHandling.h"
#include "llvm/Support/MathExtras.h"

using namespace llvm;

#include "LNP64GenCallingConv.inc"

static StringRef getDirectCalleeName(SDValue Callee) {
  if (GlobalAddressSDNode *G = dyn_cast<GlobalAddressSDNode>(Callee))
    return G->getGlobal()->getName();
  if (ExternalSymbolSDNode *S = dyn_cast<ExternalSymbolSDNode>(Callee))
    return S->getSymbol();
  return StringRef();
}

static SDValue adjustArgToLocVT(SelectionDAG &DAG, const SDLoc &DL,
                                const CCValAssign &VA, SDValue Arg) {
  switch (VA.getLocInfo()) {
  case CCValAssign::Full:
    return Arg;
  case CCValAssign::SExt:
    return DAG.getNode(ISD::SIGN_EXTEND, DL, VA.getLocVT(), Arg);
  case CCValAssign::ZExt:
    return DAG.getNode(ISD::ZERO_EXTEND, DL, VA.getLocVT(), Arg);
  case CCValAssign::AExt:
    return DAG.getNode(ISD::ANY_EXTEND, DL, VA.getLocVT(), Arg);
  case CCValAssign::BCvt:
    return DAG.getNode(ISD::BITCAST, DL, VA.getLocVT(), Arg);
  default:
    llvm_unreachable("unsupported LNP64 call argument location info");
  }
}

static SDValue adjustArgToValVT(SelectionDAG &DAG, const SDLoc &DL,
                                const CCValAssign &VA, SDValue Arg) {
  if (VA.getLocInfo() == CCValAssign::SExt)
    Arg = DAG.getNode(ISD::AssertSext, DL, VA.getLocVT(), Arg,
                      DAG.getValueType(VA.getValVT()));
  else if (VA.getLocInfo() == CCValAssign::ZExt)
    Arg = DAG.getNode(ISD::AssertZext, DL, VA.getLocVT(), Arg,
                      DAG.getValueType(VA.getValVT()));

  if (VA.getLocInfo() != CCValAssign::Full)
    Arg = DAG.getNode(ISD::TRUNCATE, DL, VA.getValVT(), Arg);
  return Arg;
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
                          ISD::ROTR, ISD::BSWAP, ISD::SETCC, ISD::BR_CC})
    setOperationAction(Opcode, MVT::i64, Legal);

  // v2 has honest hardware LR/SC. Let the generic AtomicExpand pass turn every
  // atomicrmw / cmpxchg into an LR/SC retry loop (RISC-V model). Word-sized
  // atomic load/store are plain ordered memory ops.
  setMaxAtomicSizeInBitsSupported(64);
  setMinCmpXchgSizeInBits(32);
  setOperationAction(ISD::ATOMIC_LOAD, MVT::i64, Legal);
  setOperationAction(ISD::ATOMIC_STORE, MVT::i64, Legal);
  for (MVT VT : {MVT::i8, MVT::i16, MVT::i32}) {
    setOperationAction(ISD::ATOMIC_LOAD, VT, Custom);
    setOperationAction(ISD::ATOMIC_STORE, VT, Custom);
  }

  setOperationAction(ISD::DYNAMIC_STACKALLOC, MVT::i64, Custom);
  setOperationAction(ISD::STACKSAVE,         MVT::Other, Custom);
  setOperationAction(ISD::STACKRESTORE,      MVT::Other, Custom);
  setOperationAction(ISD::GlobalAddress, MVT::i64, Custom);
  setOperationAction(ISD::BRCOND, MVT::Other, Custom);
  setOperationAction(ISD::SELECT, MVT::i64, Custom);
  setOperationAction(ISD::SELECT_CC, MVT::i64, Expand);
  setOperationAction(ISD::VASTART, MVT::Other, Custom);
  setOperationAction(ISD::VAARG, MVT::Other, Expand);
  setOperationAction(ISD::VACOPY, MVT::Other, Expand);
  setOperationAction(ISD::VAEND, MVT::Other, Expand);
  for (MVT MemVT : {MVT::i1, MVT::i8, MVT::i16, MVT::i32}) {
    setLoadExtAction(ISD::ZEXTLOAD, MVT::i64, MemVT, Legal);
    setLoadExtAction(ISD::SEXTLOAD, MVT::i64, MemVT, Legal);
    setLoadExtAction(ISD::EXTLOAD, MVT::i64, MemVT, Legal);
  }
  for (MVT MemVT : {MVT::i8, MVT::i16, MVT::i32})
    setTruncStoreAction(MVT::i64, MemVT, Legal);
  computeRegisterProperties(STI.getRegisterInfo());
}

const char *LNP64TargetLowering::getTargetNodeName(unsigned Opcode) const {
  switch (Opcode) {
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
  case LNP64ISD::SELECT_CC:
    return "LNP64ISD::SELECT_CC";
  case LNP64ISD::RET_FLAG:
    return "LNP64ISD::RET_FLAG";
  default:
    return nullptr;
  }
}

// v2 atomics: expand atomicrmw / cmpxchg into LR/SC loops.
TargetLowering::AtomicExpansionKind
LNP64TargetLowering::shouldExpandAtomicRMWInIR(AtomicRMWInst *) const {
  return AtomicExpansionKind::LLSC;
}

TargetLowering::AtomicExpansionKind
LNP64TargetLowering::shouldExpandAtomicCmpXchgInIR(AtomicCmpXchgInst *) const {
  return AtomicExpansionKind::LLSC;
}

// LL/SC emit hooks for the generic AtomicExpand pass (RISC-V LLSC model).
// The hardware exposes LR.D / SC.D; the cleanest mechanism for this backend
// (which has no target Intrinsics.td infrastructure) is to emit a single
// lr.d / sc.d via inline asm. AtomicExpandPass wraps these in the retry loop.
Value *LNP64TargetLowering::emitLoadLinked(IRBuilderBase &Builder, Type *ValueTy,
                                           Value *Addr,
                                           AtomicOrdering Ord) const {
  Type *Int64Ty = Builder.getInt64Ty();
  // Addr is a pointer; lr.d takes the address in a GPR and loads 64 bits.
  FunctionType *FTy = FunctionType::get(Int64Ty, {Addr->getType()},
                                        /*isVarArg=*/false);
  InlineAsm *IA = InlineAsm::get(FTy, "lr.d $0, ($1)", "=&r,r,~{memory}",
                                 /*hasSideEffects=*/true);
  Value *Loaded = Builder.CreateCall(IA, {Addr});
  if (ValueTy->getPrimitiveSizeInBits() < 64)
    Loaded = Builder.CreateTrunc(Loaded, ValueTy);
  return Loaded;
}

Value *LNP64TargetLowering::emitStoreConditional(IRBuilderBase &Builder,
                                                 Value *Val, Value *Addr,
                                                 AtomicOrdering Ord) const {
  Type *Int64Ty = Builder.getInt64Ty();
  if (Val->getType()->getPrimitiveSizeInBits() < 64)
    Val = Builder.CreateZExt(Val, Int64Ty);
  // sc.d $0, $2, ($1): $0 = status (0 = success, 1 = fail), $1 = addr, $2 = val.
  // AtomicExpandPass compares the result against an i32 zero, so return i32.
  Type *Int32Ty = Builder.getInt32Ty();
  FunctionType *FTy =
      FunctionType::get(Int32Ty, {Addr->getType(), Int64Ty},
                        /*isVarArg=*/false);
  InlineAsm *IA = InlineAsm::get(FTy, "sc.d $0, $2, ($1)", "=&r,r,r,~{memory}",
                                 /*hasSideEffects=*/true);
  // AtomicExpandPass expects a non-zero value to mean failure; sc.d already
  // returns 1 on failure / 0 on success, which is exactly that convention.
  return Builder.CreateCall(IA, {Addr, Val});
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
  case ISD::STACKSAVE: {
    SDLoc DL(Op);
    SDValue Chain = Op.getOperand(0);
    SDValue SP = DAG.getCopyFromReg(Chain, DL, LNP64::R31, MVT::i64);
    return DAG.getMergeValues({SP, SP.getValue(1)}, DL);
  }
  case ISD::STACKRESTORE: {
    SDLoc DL(Op);
    SDValue Chain = Op.getOperand(0);
    SDValue SP    = Op.getOperand(1);
    return DAG.getCopyToReg(Chain, DL, LNP64::R31, SP);
  }
  case ISD::DYNAMIC_STACKALLOC: {
    SDLoc DL(Op);
    SDValue Chain = Op.getOperand(0);
    SDValue Size  = Op.getOperand(1);
    SDValue Align = Op.getOperand(2);
    unsigned AlignVal = cast<ConstantSDNode>(Align)->getZExtValue();
    if (AlignVal < 8) AlignVal = 8; // LNP64 min stack alignment
    SDValue AlignM1 = DAG.getConstant(AlignVal - 1, DL, MVT::i64);
    SDValue Rounded = DAG.getNode(ISD::AND, DL, MVT::i64,
                        DAG.getNode(ISD::ADD, DL, MVT::i64, Size, AlignM1),
                        DAG.getNOT(DL, AlignM1, MVT::i64));
    SDValue SP = DAG.getCopyFromReg(Chain, DL, LNP64::R31, MVT::i64);
    SDValue NewSP = DAG.getNode(ISD::SUB, DL, MVT::i64, SP, Rounded);
    Chain = DAG.getCopyToReg(SP.getValue(1), DL, LNP64::R31, NewSP);
    return DAG.getMergeValues({NewSP, Chain}, DL);
  }
  case ISD::GlobalAddress: {
    auto *G = cast<GlobalAddressSDNode>(Op);
    SDLoc DL(Op);
    SDValue Target = DAG.getTargetGlobalAddress(
        G->getGlobal(), DL, getPointerTy(DAG.getDataLayout()), G->getOffset());
    return DAG.getNode(LNP64ISD::WRAPPER, DL,
                       getPointerTy(DAG.getDataLayout()), Target);
  }
  case ISD::SELECT: {
    // Lower to an LNP64ISD::SELECT_CC against zero, expanded by the custom
    // inserter into a branch diamond. Operands: (cond, true, false).
    SDLoc DL(Op);
    SDValue Cond = Op.getOperand(0);
    SDValue TrueV = Op.getOperand(1);
    SDValue FalseV = Op.getOperand(2);
    if (Cond.getValueType() != MVT::i64)
      Cond = DAG.getNode(ISD::ZERO_EXTEND, DL, MVT::i64, Cond);
    SDValue Zero = DAG.getConstant(0, DL, MVT::i64);
    SDValue CC = DAG.getConstant(ISD::SETNE, DL, MVT::i64);
    SDValue Ops[] = {Cond, Zero, CC, TrueV, FalseV};
    return DAG.getNode(LNP64ISD::SELECT_CC, DL, MVT::i64, Ops);
  }
  case ISD::BRCOND: {
    SDLoc DL(Op);
    SDValue Chain = Op.getOperand(0);
    SDValue Cond = Op.getOperand(1);
    SDValue Target = Op.getOperand(2);
    if (Cond.getValueType() != MVT::i64)
      Cond = DAG.getNode(ISD::ZERO_EXTEND, DL, MVT::i64, Cond);
    SDValue Zero = DAG.getConstant(0, DL, MVT::i64);
    return DAG.getNode(ISD::BR_CC, DL, MVT::Other,
                       {Chain, DAG.getCondCode(ISD::SETNE), Cond, Zero, Target});
  }
  case ISD::ATOMIC_LOAD: {
    auto *AN = cast<AtomicSDNode>(Op);
    SDLoc DL(Op);
    EVT MemVT = AN->getMemoryVT();
    return DAG.getExtLoad(ISD::ZEXTLOAD, DL, MVT::i64, AN->getChain(),
                          AN->getBasePtr(), AN->getPointerInfo(), MemVT,
                          AN->getOriginalAlign(),
                          AN->getMemOperand()->getFlags());
  }
  case ISD::ATOMIC_STORE: {
    auto *AN = cast<AtomicSDNode>(Op);
    SDLoc DL(Op);
    SDValue Chain = AN->getChain();
    SDValue Ptr   = AN->getBasePtr();
    SDValue Val   = AN->getVal();
    EVT MemVT = AN->getMemoryVT();
    MachineMemOperand *MMO = AN->getMemOperand();
    if (Val.getValueType() != MemVT)
      Val = DAG.getNode(ISD::TRUNCATE, DL, MemVT, Val);
    return DAG.getStore(Chain, DL, Val, Ptr, MMO);
  }
  case ISD::VASTART: {
    SDLoc DL(Op);
    SDValue Chain = Op.getOperand(0);
    EVT PtrVT = getPointerTy(DAG.getDataLayout());
    MachineFrameInfo &MFI = DAG.getMachineFunction().getFrameInfo();
    int FI = MFI.CreateFixedObject(8, 0, /*IsImmutable=*/true);
    SDValue VarArgsPtr = DAG.getFrameIndex(FI, PtrVT);
    const Value *SV = cast<SrcValueSDNode>(Op.getOperand(2))->getValue();
    return DAG.getStore(Chain, DL, VarArgsPtr, Op.getOperand(1),
                        MachinePointerInfo(SV));
  }
  default:
    llvm_unreachable("unsupported LNP64 custom lowering opcode");
  }
}

void LNP64TargetLowering::ReplaceNodeResults(SDNode *N,
                                             SmallVectorImpl<SDValue> &Results,
                                             SelectionDAG &DAG) const {
  SDLoc DL(N);
  switch (N->getOpcode()) {
  case ISD::ATOMIC_LOAD: {
    auto *AN = cast<AtomicSDNode>(N);
    EVT MemVT = AN->getMemoryVT();
    SDValue Load = DAG.getExtLoad(
        ISD::ZEXTLOAD, DL, MVT::i64, AN->getChain(), AN->getBasePtr(),
        AN->getPointerInfo(), MemVT, AN->getOriginalAlign(),
        AN->getMemOperand()->getFlags());
    Results.push_back(DAG.getNode(ISD::TRUNCATE, DL, N->getValueType(0),
                                  Load.getValue(0)));
    Results.push_back(Load.getValue(1)); // chain
    return;
  }
  default:
    break;
  }
}

// Map an integer condition code + (lhs,rhs) into a compare-and-branch opcode
// that LNP64 implements natively, swapping operands where needed.
static unsigned getBranchForCC(ISD::CondCode CC, bool &SwapOps) {
  SwapOps = false;
  switch (CC) {
  case ISD::SETEQ:  return LNP64::BEQ;
  case ISD::SETNE:  return LNP64::BNE;
  case ISD::SETLT:  return LNP64::BLT;
  case ISD::SETGE:  return LNP64::BGE;
  case ISD::SETULT: return LNP64::BLTU;
  case ISD::SETUGE: return LNP64::BGEU;
  case ISD::SETGT:  SwapOps = true; return LNP64::BLT;
  case ISD::SETLE:  SwapOps = true; return LNP64::BGE;
  case ISD::SETUGT: SwapOps = true; return LNP64::BLTU;
  case ISD::SETULE: SwapOps = true; return LNP64::BGEU;
  default:
    llvm_unreachable("LNP64 select only supports integer comparisons");
  }
}

MachineBasicBlock *LNP64TargetLowering::EmitInstrWithCustomInserter(
    MachineInstr &MI, MachineBasicBlock *BB) const {
  const TargetInstrInfo &TII = *BB->getParent()->getSubtarget().getInstrInfo();
  DebugLoc DL = MI.getDebugLoc();

  if (MI.getOpcode() == LNP64::PseudoLI64) {
    MachineFunction *MF = BB->getParent();
    MachineRegisterInfo &MRI = MF->getRegInfo();
    uint64_t Value = static_cast<uint64_t>(MI.getOperand(1).getImm());
    uint32_t Hi = static_cast<uint32_t>(Value >> 32);
    uint32_t Lo = static_cast<uint32_t>(Value);
    Register LoReg = MRI.createVirtualRegister(&LNP64::GPRRegClass);
    BuildMI(*BB, MI, DL, TII.get(LNP64::LI), LoReg).addImm(int32_t(Lo));
    BuildMI(*BB, MI, DL, TII.get(LNP64::LIU), MI.getOperand(0).getReg())
        .addReg(LoReg)
        .addImm(int32_t(Hi));
    MI.eraseFromParent();
    return BB;
  }

  if (MI.getOpcode() == LNP64::PseudoSELECT_CC) {
    // operands: dst, lhs, rhs, cc(imm), trueV, falseV
    MachineFunction *MF = BB->getParent();
    const BasicBlock *LLVM_BB = BB->getBasicBlock();
    MachineFunction::iterator It = ++BB->getIterator();

    MachineBasicBlock *HeadMBB = BB;
    MachineBasicBlock *IfFalseMBB = MF->CreateMachineBasicBlock(LLVM_BB);
    MachineBasicBlock *TailMBB = MF->CreateMachineBasicBlock(LLVM_BB);
    MF->insert(It, IfFalseMBB);
    MF->insert(It, TailMBB);
    TailMBB->splice(TailMBB->begin(), HeadMBB,
                    std::next(MachineBasicBlock::iterator(MI)), HeadMBB->end());
    TailMBB->transferSuccessorsAndUpdatePHIs(HeadMBB);
    HeadMBB->addSuccessor(IfFalseMBB);
    HeadMBB->addSuccessor(TailMBB);

    ISD::CondCode CC = (ISD::CondCode)MI.getOperand(3).getImm();
    bool SwapOps = false;
    unsigned BrOpc = getBranchForCC(CC, SwapOps);
    Register LHS = MI.getOperand(1).getReg();
    Register RHS = MI.getOperand(2).getReg();
    // Branch to TailMBB (taking trueV) when the condition holds.
    BuildMI(HeadMBB, DL, TII.get(BrOpc))
        .addReg(SwapOps ? RHS : LHS)
        .addReg(SwapOps ? LHS : RHS)
        .addMBB(TailMBB);

    IfFalseMBB->addSuccessor(TailMBB);

    BuildMI(*TailMBB, TailMBB->begin(), DL, TII.get(LNP64::PHI),
            MI.getOperand(0).getReg())
        .addReg(MI.getOperand(4).getReg())
        .addMBB(HeadMBB)
        .addReg(MI.getOperand(5).getReg())
        .addMBB(IfFalseMBB);
    MI.eraseFromParent();
    return TailMBB;
  }

  llvm_unreachable("unexpected LNP64 custom inserter pseudo");
}

SDValue LNP64TargetLowering::LowerFormalArguments(
    SDValue Chain, CallingConv::ID CallConv, bool IsVarArg,
    const SmallVectorImpl<ISD::InputArg> &Ins, const SDLoc &DL,
    SelectionDAG &DAG, SmallVectorImpl<SDValue> &InVals) const {
  MachineFunction &MF = DAG.getMachineFunction();
  SmallVector<CCValAssign, 8> ArgLocs;
  CCState CCInfo(CallConv, IsVarArg, MF, ArgLocs, *DAG.getContext());
  CCInfo.AnalyzeFormalArguments(Ins, CC_LNP64);
  EVT PtrVT = getPointerTy(DAG.getDataLayout());

  for (CCValAssign &VA : ArgLocs) {
    SDValue Arg;
    if (VA.isRegLoc()) {
      Register VReg = MF.addLiveIn(VA.getLocReg(), &LNP64::GPRRegClass);
      Arg = DAG.getCopyFromReg(Chain, DL, VReg, VA.getLocVT());
    } else {
      unsigned ObjSize = VA.getLocVT().getStoreSize();
      int FI = MF.getFrameInfo().CreateFixedObject(
          ObjSize, VA.getLocMemOffset(), /*IsImmutable=*/true);
      SDValue FIPtr = DAG.getFrameIndex(FI, PtrVT);
      Arg = DAG.getLoad(
          VA.getLocVT(), DL, Chain, FIPtr,
          MachinePointerInfo::getFixedStack(DAG.getMachineFunction(), FI));
    }
    InVals.push_back(adjustArgToValVT(DAG, DL, VA, Arg));
  }

  return Chain;
}

SDValue LNP64TargetLowering::LowerReturn(
    SDValue Chain, CallingConv::ID CallConv, bool IsVarArg,
    const SmallVectorImpl<ISD::OutputArg> &Outs,
    const SmallVectorImpl<SDValue> &OutVals, const SDLoc &DL,
    SelectionDAG &DAG) const {
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
  unsigned VarArgStackBytes = 0;
  if (CLI.IsVarArg) {
    for (unsigned I = 0, E = CLI.Outs.size(); I != E; ++I) {
      if (!CLI.Outs[I].IsFixed) {
        EVT ArgVT = CLI.OutVals[I].getValueType();
        VarArgStackBytes += alignTo(ArgVT.getStoreSize(), Align(8));
      }
    }
  }
  unsigned NumBytes =
      alignTo(std::max(ArgCCInfo.getNextStackOffset(), VarArgStackBytes),
              Align(16));
  Chain = DAG.getCALLSEQ_START(Chain, NumBytes, 0, DL);

  SDValue Glue;
  SmallVector<std::pair<unsigned, SDValue>, 8> RegsToPass;
  SmallVector<SDValue, 8> MemOpChains;
  SDValue StackPtr;
  EVT PtrVT = getPointerTy(DAG.getDataLayout());
  unsigned VarArgStackOffset = 0;
  for (unsigned I = 0, E = ArgLocs.size(); I != E; ++I) {
    CCValAssign &VA = ArgLocs[I];
    SDValue Arg = adjustArgToLocVT(DAG, DL, VA, CLI.OutVals[I]);
    if (CLI.IsVarArg && !CLI.Outs[I].IsFixed) {
      if (!StackPtr)
        StackPtr = DAG.getCopyFromReg(Chain, DL, LNP64::R31, PtrVT);
      SDValue PtrOff =
          DAG.getNode(ISD::ADD, DL, PtrVT, StackPtr,
                      DAG.getIntPtrConstant(VarArgStackOffset, DL));
      MemOpChains.push_back(
          DAG.getStore(Chain, DL, Arg, PtrOff, MachinePointerInfo()));
      VarArgStackOffset += alignTo(Arg.getValueType().getStoreSize(), Align(8));
      continue;
    }
    if (VA.isRegLoc()) {
      RegsToPass.push_back(std::make_pair(VA.getLocReg(), Arg));
    } else {
      if (!StackPtr)
        StackPtr = DAG.getCopyFromReg(Chain, DL, LNP64::R31, PtrVT);
      SDValue PtrOff =
          DAG.getNode(ISD::ADD, DL, PtrVT, StackPtr,
                      DAG.getIntPtrConstant(VA.getLocMemOffset(), DL));
      MemOpChains.push_back(
          DAG.getStore(Chain, DL, Arg, PtrOff, MachinePointerInfo()));
    }
  }

  if (!MemOpChains.empty())
    Chain = DAG.getNode(ISD::TokenFactor, DL, MVT::Other, MemOpChains);

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

  // Attach the call-preserved regmask (callee-saved set s0..s9 = r18..r27) so
  // the register allocator knows exactly which registers the call clobbers and
  // can keep cross-call values live in the preserved s-registers.
  const LNP64RegisterInfo *TRI =
      MF.getSubtarget<LNP64Subtarget>().getRegisterInfo();
  const uint32_t *Mask = TRI->getCallPreservedMask(MF, CLI.CallConv);
  assert(Mask && "missing call-preserved regmask");
  Ops.push_back(DAG.getRegisterMask(Mask));

  if (Glue)
    Ops.push_back(Glue);

  SDVTList NodeTys = DAG.getVTList(MVT::Other, MVT::Glue);
  Chain = DAG.getNode(LNP64ISD::CALL, DL, NodeTys, Ops);
  Glue = Chain.getValue(1);
  Chain = DAG.getCALLSEQ_END(
      Chain, DAG.getIntPtrConstant(NumBytes, DL, true),
      DAG.getIntPtrConstant(0, DL, true), Glue, DL);
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

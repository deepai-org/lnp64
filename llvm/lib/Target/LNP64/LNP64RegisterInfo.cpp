#include "LNP64RegisterInfo.h"
#include "LNP64FrameLowering.h"
#include "MCTargetDesc/LNP64MCTargetDesc.h"
#include "llvm/CodeGen/MachineFrameInfo.h"
#include "llvm/CodeGen/MachineInstr.h"
#include "llvm/CodeGen/MachineInstrBuilder.h"
#include "llvm/CodeGen/MachineFunction.h"
#include "llvm/CodeGen/TargetInstrInfo.h"
#include "llvm/CodeGen/TargetSubtargetInfo.h"
#include "llvm/Support/ErrorHandling.h"
#include "llvm/Support/MathExtras.h"

using namespace llvm;

#define GET_REGINFO_TARGET_DESC
#include "LNP64GenRegisterInfo.inc"

// In v2 the return address is a normal callee-saved GPR (r1), spilled in the
// prologue like any other; there is no separate save slot beyond the stack
// frame the spill code allocates.
LNP64RegisterInfo::LNP64RegisterInfo() : LNP64GenRegisterInfo(LNP64::R1) {}

BitVector LNP64RegisterInfo::getReservedRegs(const MachineFunction &) const {
  BitVector Reserved(getNumRegs());
  Reserved.set(LNP64::R0);  // hardwired zero
  Reserved.set(LNP64::R1);  // ra -- dedicated return-address link register
  Reserved.set(LNP64::R31); // stack pointer
  // r1 is a dedicated link register, NOT a general allocatable temp: it holds
  // the return address live-in (placed by the caller's jal) and is read by
  // `ret` (= jalr r0, r1, 0). If it were allocatable, the register allocator
  // would reuse it as a scratch in leaf functions (which do not save it) and
  // clobber the return address before `ret`. r30 is a normal allocatable
  // caller-saved GPR: prologue/epilogue SP adjustment and frame-address
  // computation use ADDI's 32-bit immediate directly, so no scratch register
  // is reserved.
  return Reserved;
}

const MCPhysReg *
LNP64RegisterInfo::getCalleeSavedRegs(const MachineFunction *) const {
  // v2 ABI callee-saved set s0..s9 = r18..r27. The generic
  // PrologueEpilogueInserter spills/restores whichever of these a function
  // actually clobbers. r1 (ra) is handled by the bespoke prologue spill in
  // LNP64FrameLowering and is intentionally NOT listed here.
  static const MCPhysReg CalleeSaved[] = {
      LNP64::R18, LNP64::R19, LNP64::R20, LNP64::R21, LNP64::R22,
      LNP64::R23, LNP64::R24, LNP64::R25, LNP64::R26, LNP64::R27, 0};
  return CalleeSaved;
}

const uint32_t *
LNP64RegisterInfo::getCallPreservedMask(const MachineFunction &,
                                        CallingConv::ID) const {
  // The TableGen CSR_LNP64 def generates this regmask: bits set = preserved
  // across a call (r18..r27). Attaching it to call instructions tells the
  // register allocator that everything else is clobbered, so it can keep
  // cross-call values in s-registers instead of spilling them.
  return CSR_LNP64_RegMask;
}

Register LNP64RegisterInfo::getFrameRegister(const MachineFunction &) const {
  return LNP64::R31;
}

void LNP64RegisterInfo::eliminateFrameIndex(MachineBasicBlock::iterator II,
                                            int, unsigned FIOperandNum,
                                            RegScavenger *) const {
  MachineInstr &MI = *II;
  MachineFunction &MF = *MI.getParent()->getParent();
  const MachineFrameInfo &MFI = MF.getFrameInfo();
  int FrameIndex = MI.getOperand(FIOperandNum).getIndex();
  // Fixed frame objects (FI < 0) represent the caller's incoming frame area
  // and are offset from the incoming SP. The prologue reserves
  // getStackSize() + RASaveSize bytes below incoming SP, but
  // getStackSize() alone is reported by MFI. Fixed objects therefore need
  // the extra RASaveSize added so they land at incoming_SP + object_offset
  // rather than incoming_SP - RASaveSize + object_offset.
  uint64_t RASaveSize =
      MF.getFrameInfo().hasCalls() ? LNP64RASaveSlotBytes : 0;
  int64_t BaseSize = int64_t(MFI.getStackSize()) + (FrameIndex < 0 ? int64_t(RASaveSize) : 0);
  int64_t Offset = MFI.getObjectOffset(FrameIndex) + BaseSize;
  if (FIOperandNum + 1 < MI.getNumOperands() &&
      MI.getOperand(FIOperandNum + 1).isImm())
    Offset += MI.getOperand(FIOperandNum + 1).getImm();

  // Frame-index operands are resolved uniformly: the base operand becomes r31
  // (sp) and the following immediate becomes the resolved offset. This covers
  // loads/stores (LD/SD/...) and the `addi rd, <fi>, 0` frame-address form
  // emitted by SelectFrameIndexValue -- ADDI's 32-bit immediate computes the
  // frame address directly, so no scratch register and no dedicated pseudo are
  // needed. v2 load/store offsets are 32-bit signed; frame offsets never
  // overflow in practice, so the v1 large-offset r30 scratch-address path is
  // deleted.
  MI.getOperand(FIOperandNum).ChangeToRegister(LNP64::R31, false);
  if (FIOperandNum + 1 < MI.getNumOperands() &&
      MI.getOperand(FIOperandNum + 1).isImm())
    MI.getOperand(FIOperandNum + 1).ChangeToImmediate(Offset);
}

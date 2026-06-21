#include "LNP64FrameLowering.h"
#include "LNP64.h"
#include "LNP64InstrInfo.h"
#include "llvm/CodeGen/MachineFrameInfo.h"
#include "llvm/CodeGen/MachineFunction.h"
#include "llvm/CodeGen/MachineInstrBuilder.h"
#include "llvm/CodeGen/TargetInstrInfo.h"
#include "llvm/CodeGen/TargetSubtargetInfo.h"
#include "llvm/CodeGen/TargetOpcodes.h"
#include "llvm/MC/MCDwarf.h"
#include "llvm/Support/ErrorHandling.h"
#include "llvm/Support/MathExtras.h"

using namespace llvm;

static constexpr unsigned LNP64DwarfSP = 31;
// v2: the return address lives in r1 (no separate LR register).
static constexpr unsigned LNP64DwarfRA = 1;

// In v2 r1 (ra) is a dedicated link register, saved here in the prologue when
// the function makes calls. The s0..s9 (r18..r27) callee-saved GPRs are NOT
// handled here: the generic PrologueEpilogueInserter spills/restores whichever
// of them a function clobbers via storeRegToStackSlot/loadRegFromStackSlot
// (their frame objects live inside [0, getStackSize())). The ra slot lives in
// the reserved RASaveSize bytes ON TOP at offset getStackSize(), so it never
// aliases the PEI-laid-out CSR save area, locals, or spills.
static uint64_t getRASaveSize(const MachineFunction &MF) {
  return MF.getFrameInfo().hasCalls() ? 8 : 0;
}

static uint64_t getRASaveOffset(const MachineFunction &MF) {
  // The prologue reserves RASaveSize (8) bytes ON TOP of the PEI frame
  // (StackSize = getStackSize() + RASaveSize). PEI lays out all locals,
  // spills, and the reserved outgoing-call-frame in [0, getStackSize()) (SP-
  // relative), so the RA slot must sit in the reserved top bytes at offset
  // getStackSize() -- not getMaxCallFrameSize(), which is 0 for leaf-arg calls
  // and aliases the first local (eliminateFrameIndex resolves objects with the
  // same getStackSize() base).
  return MF.getFrameInfo().hasCalls() ? MF.getFrameInfo().getStackSize() : 0;
}

static void emitCFI(MachineFunction &MF, MachineBasicBlock &MBB,
                    MachineBasicBlock::iterator I, const DebugLoc &DL,
                    const MCCFIInstruction &CFI) {
  const TargetInstrInfo &TII = *MF.getSubtarget().getInstrInfo();
  unsigned CFIIndex = MF.addFrameInst(CFI);
  BuildMI(MBB, I, DL, TII.get(TargetOpcode::CFI_INSTRUCTION))
      .addCFIIndex(CFIIndex);
}

static void emitSPAdjust(MachineFunction &MF, MachineBasicBlock &MBB,
                         MachineBasicBlock::iterator I, const DebugLoc &DL,
                         int64_t Amount) {
  if (Amount == 0)
    return;

  const TargetInstrInfo &TII = *MF.getSubtarget().getInstrInfo();
  // ADDI carries a full signed-32 immediate in one v2 word, so the stack
  // pointer is adjusted in a single instruction -- no materialize-into-scratch
  // and no reserved scratch register needed (negative Amount subtracts).
  if (!isInt<32>(Amount))
    llvm_unreachable("LNP64 stack adjustment exceeds 32-bit immediate");
  BuildMI(MBB, I, DL, TII.get(LNP64::ADDI), LNP64::R31)
      .addReg(LNP64::R31)
      .addImm(Amount);
}

LNP64FrameLowering::LNP64FrameLowering()
    : TargetFrameLowering(StackGrowsDown, Align(16), /*LocalAreaOffset=*/0) {}

void LNP64FrameLowering::emitPrologue(MachineFunction &MF,
                                      MachineBasicBlock &MBB) const {
  const uint64_t RASaveSize = getRASaveSize(MF);
  uint64_t StackSize = MF.getFrameInfo().getStackSize() + RASaveSize;
  MachineBasicBlock::iterator I = MBB.begin();
  emitSPAdjust(MF, MBB, I, DebugLoc(), -int64_t(StackSize));
  if (StackSize != 0)
    emitCFI(MF, MBB, I, DebugLoc(),
            MCCFIInstruction::cfiDefCfa(nullptr, LNP64DwarfSP, StackSize));

  if (RASaveSize != 0) {
    const TargetInstrInfo &TII = *MF.getSubtarget().getInstrInfo();
    const int64_t RASaveOffsetFromCFA =
        int64_t(getRASaveOffset(MF)) - int64_t(StackSize);
    // Spill r1 (ra) like any callee-saved GPR.
    BuildMI(MBB, I, DebugLoc(), TII.get(LNP64::SD))
        .addReg(LNP64::R1)
        .addReg(LNP64::R31)
        .addImm(getRASaveOffset(MF));
    emitCFI(MF, MBB, I, DebugLoc(),
            MCCFIInstruction::createOffset(nullptr, LNP64DwarfRA,
                                           RASaveOffsetFromCFA));
  }
}

void LNP64FrameLowering::emitEpilogue(MachineFunction &MF,
                                      MachineBasicBlock &MBB) const {
  const uint64_t RASaveSize = getRASaveSize(MF);
  uint64_t StackSize = MF.getFrameInfo().getStackSize() + RASaveSize;
  MachineBasicBlock::iterator I = MBB.getFirstTerminator();

  if (RASaveSize != 0) {
    const TargetInstrInfo &TII = *MF.getSubtarget().getInstrInfo();
    BuildMI(MBB, I, DebugLoc(), TII.get(LNP64::LD), LNP64::R1)
        .addReg(LNP64::R31)
        .addImm(getRASaveOffset(MF));
  }
  emitSPAdjust(MF, MBB, I, DebugLoc(), int64_t(StackSize));
}

MachineBasicBlock::iterator LNP64FrameLowering::eliminateCallFramePseudoInstr(
    MachineFunction &, MachineBasicBlock &MBB,
    MachineBasicBlock::iterator I) const {
  return MBB.erase(I);
}

bool LNP64FrameLowering::hasFP(const MachineFunction &) const { return false; }

//===-- LNP64Disassembler.cpp - v2 64-bit decoder ------------------------===//
//
// Thin wrapper over the TableGen-generated fixed-length decoder
// (LNP64GenDisassemblerTables.inc), the verified inverse of the generated
// encoder. One 8-byte little-endian word per instruction:
//   opcode[63:56] rd[55:51] rs1[50:46] rs2[45:41] rs3[40:36] rs4[35:31]
//   rs5[30:26]; I-type imm32 [45:14]; S/B-type imm32 [40:9]; U/J [50:19];
//   branch/jump target = sext32(field) << 3. This file only provides the
//   register-class and immediate decode hooks the generated table calls.
//===----------------------------------------------------------------------===//

#include "MCTargetDesc/LNP64MCTargetDesc.h"
#include "llvm/MC/MCDisassembler/MCDisassembler.h"
#include "llvm/MC/MCFixedLenDisassembler.h"
#include "llvm/MC/MCInst.h"
#include "llvm/MC/MCSubtargetInfo.h"
#include "llvm/MC/TargetRegistry.h"
#include "llvm/Support/MathExtras.h"

using namespace llvm;

using DecodeStatus = MCDisassembler::DecodeStatus;

namespace {
class LNP64Disassembler : public MCDisassembler {
public:
  LNP64Disassembler(const MCSubtargetInfo &STI, MCContext &Ctx)
      : MCDisassembler(STI, Ctx) {}

  DecodeStatus getInstruction(MCInst &MI, uint64_t &Size,
                              ArrayRef<uint8_t> Bytes, uint64_t Address,
                              raw_ostream &) const override;
};
} // end anonymous namespace

// Register-class decoders. HWEncoding is the dense index within each class, so
// the register is the class's first enum value plus the field value.
static const unsigned PCRTable[] = {
    LNP64::PID,        LNP64::PPID,       LNP64::TID,
    LNP64::TP,         LNP64::UID,        LNP64::GID,
    LNP64::SIGMASK,    LNP64::SIGPENDING, LNP64::REALTIME_SEC,
    LNP64::REALTIME_NSEC, LNP64::CRED_PROFILE, LNP64::CRED_HANDLE};

// Only GPR and PCR appear as instruction operands today (capabilities are
// passed in GPRs); FDR/FPR/VR register classes have no instruction operands, so
// the generated decoder never references their decode hooks.
static DecodeStatus DecodeGPRRegisterClass(MCInst &Inst, uint64_t RegNo,
                                           uint64_t, const void *) {
  if (RegNo > 31)
    return MCDisassembler::Fail;
  Inst.addOperand(MCOperand::createReg(LNP64::R0 + RegNo));
  return MCDisassembler::Success;
}

static DecodeStatus DecodePCRRegisterClass(MCInst &Inst, uint64_t RegNo,
                                           uint64_t, const void *) {
  if (RegNo >= sizeof(PCRTable) / sizeof(PCRTable[0]))
    return MCDisassembler::Fail;
  Inst.addOperand(MCOperand::createReg(PCRTable[RegNo]));
  return MCDisassembler::Success;
}

static DecodeStatus decodeSImm32(MCInst &Inst, uint64_t Imm, uint64_t,
                                 const void *) {
  Inst.addOperand(MCOperand::createImm(SignExtend64<32>(Imm)));
  return MCDisassembler::Success;
}

// Branch/jump targets store the signed byte offset >> 3.
static DecodeStatus decodeShiftedTarget(MCInst &Inst, uint64_t Imm, uint64_t,
                                        const void *) {
  Inst.addOperand(MCOperand::createImm(SignExtend64<32>(Imm) << 3));
  return MCDisassembler::Success;
}

#include "LNP64GenDisassemblerTables.inc"

DecodeStatus LNP64Disassembler::getInstruction(MCInst &MI, uint64_t &Size,
                                               ArrayRef<uint8_t> Bytes,
                                               uint64_t Address,
                                               raw_ostream &) const {
  if (Bytes.size() < 8) {
    Size = 0;
    return MCDisassembler::Fail;
  }
  Size = 8;
  uint64_t W = 0;
  for (unsigned I = 0; I < 8; ++I)
    W |= uint64_t(Bytes[I]) << (8 * I);
  return decodeInstruction(DecoderTable64, MI, W, Address, this, STI);
}

static MCDisassembler *createLNP64Disassembler(const Target &,
                                               const MCSubtargetInfo &STI,
                                               MCContext &Ctx) {
  return new LNP64Disassembler(STI, Ctx);
}

extern "C" LLVM_EXTERNAL_VISIBILITY void LLVMInitializeLNP64Disassembler() {
  TargetRegistry::RegisterMCDisassembler(getTheLNP64Target(),
                                         createLNP64Disassembler);
}

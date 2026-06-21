//===-- LNP64Disassembler.cpp - v2 64-bit decoder ------------------------===//
//
// Hand-written v2 decoder: one 8-byte little-endian word per instruction.
//   opcode[63:56] rd[55:51] rs1[50:46] rs2[45:41] rs3[40:36] rs4[35:31]
//   rs5[30:26]; I-type imm32 [45:14]; S/B-type imm32 [40:9]; U/J [50:19].
//===----------------------------------------------------------------------===//

#include "MCTargetDesc/LNP64MCTargetDesc.h"
#include "llvm/MC/MCDisassembler/MCDisassembler.h"
#include "llvm/MC/MCInst.h"
#include "llvm/MC/TargetRegistry.h"
#include "llvm/Support/MathExtras.h"

using namespace llvm;

namespace {

static unsigned getGPR(unsigned Enc) {
  if (Enc > 31)
    return 0;
  return LNP64::R0 + Enc;
}

static unsigned getPCR(unsigned Enc) {
  switch (Enc) {
  case 0: return LNP64::PID;
  case 1: return LNP64::PPID;
  case 2: return LNP64::TID;
  case 3: return LNP64::TP;
  case 4: return LNP64::UID;
  case 5: return LNP64::GID;
  case 6: return LNP64::SIGMASK;
  case 7: return LNP64::SIGPENDING;
  case 8: return LNP64::REALTIME_SEC;
  case 9: return LNP64::REALTIME_NSEC;
  case 10: return LNP64::CRED_PROFILE;
  case 11: return LNP64::CRED_HANDLE;
  default: return 0;
  }
}

static uint64_t readLE64(ArrayRef<uint8_t> Bytes) {
  uint64_t V = 0;
  for (unsigned I = 0; I < 8; ++I)
    V |= uint64_t(Bytes[I]) << (8 * I);
  return V;
}

class LNP64Disassembler : public MCDisassembler {
public:
  LNP64Disassembler(const MCSubtargetInfo &STI, MCContext &Ctx)
      : MCDisassembler(STI, Ctx) {}

  DecodeStatus getInstruction(MCInst &MI, uint64_t &Size,
                              ArrayRef<uint8_t> Bytes, uint64_t,
                              raw_ostream &) const override {
    if (Bytes.size() < 8) {
      Size = 0;
      return MCDisassembler::Fail;
    }
    Size = 8;
    const DecodeStatus Success = MCDisassembler::Success;

    uint64_t W = readLE64(Bytes);
    uint8_t Opcode = W >> 56;
    unsigned RD = (W >> 51) & 0x1f;
    unsigned RS1 = (W >> 46) & 0x1f;
    unsigned RS2 = (W >> 41) & 0x1f;
    unsigned RS3 = (W >> 36) & 0x1f;
    unsigned RS4 = (W >> 31) & 0x1f;
    unsigned RS5 = (W >> 26) & 0x1f;
    int64_t ImmI = SignExtend64<32>((W >> 14) & 0xffffffffULL);
    int64_t ImmS = SignExtend64<32>((W >> 9) & 0xffffffffULL);
    int64_t ImmU = SignExtend64<32>((W >> 19) & 0xffffffffULL);

    auto R = [&](unsigned Enc) { MI.addOperand(MCOperand::createReg(getGPR(Enc))); };
    auto Imm = [&](int64_t V) { MI.addOperand(MCOperand::createImm(V)); };
    auto Pcr = [&](unsigned Enc) {
      unsigned Reg = getPCR(Enc);
      if (Reg) MI.addOperand(MCOperand::createReg(Reg));
    };

    switch (Opcode) {
    case 0x00: MI.setOpcode(LNP64::NOP); return Success;
    case 0x06: MI.setOpcode(LNP64::YIELD); return Success;
    case 0xcd: MI.setOpcode(LNP64::FENCE); return Success;
    case 0x65: MI.setOpcode(LNP64::SIGRET); return Success;

    case 0x04: MI.setOpcode(LNP64::LIU); R(RD); R(RS1); Imm(ImmI); return Success;
    case 0xd0: MI.setOpcode(LNP64::AUIPC); R(RD); Imm(ImmU); return Success;

    case 0x10: MI.setOpcode(LNP64::ADD); R(RD); R(RS1); R(RS2); return Success;
    case 0x11: MI.setOpcode(LNP64::SUB); R(RD); R(RS1); R(RS2); return Success;
    case 0x12: MI.setOpcode(LNP64::MUL); R(RD); R(RS1); R(RS2); return Success;
    case 0x13: MI.setOpcode(LNP64::DIV); R(RD); R(RS1); R(RS2); return Success;
    case 0x14: MI.setOpcode(LNP64::AND); R(RD); R(RS1); R(RS2); return Success;
    case 0x15: MI.setOpcode(LNP64::OR); R(RD); R(RS1); R(RS2); return Success;
    case 0x16: MI.setOpcode(LNP64::XOR); R(RD); R(RS1); R(RS2); return Success;
    case 0x18: MI.setOpcode(LNP64::SLL); R(RD); R(RS1); R(RS2); return Success;
    case 0x19: MI.setOpcode(LNP64::SRL); R(RD); R(RS1); R(RS2); return Success;
    case 0x1a: MI.setOpcode(LNP64::SRA); R(RD); R(RS1); R(RS2); return Success;
    case 0x1b: MI.setOpcode(LNP64::SLT); R(RD); R(RS1); R(RS2); return Success;
    case 0x1c: MI.setOpcode(LNP64::SLTU); R(RD); R(RS1); R(RS2); return Success;
    case 0xa7: MI.setOpcode(LNP64::UDIV); R(RD); R(RS1); R(RS2); return Success;
    case 0xa8: MI.setOpcode(LNP64::SREM); R(RD); R(RS1); R(RS2); return Success;
    case 0xa9: MI.setOpcode(LNP64::UREM); R(RD); R(RS1); R(RS2); return Success;
    case 0xaa: MI.setOpcode(LNP64::MULH); R(RD); R(RS1); R(RS2); return Success;
    case 0xab: MI.setOpcode(LNP64::MULHU); R(RD); R(RS1); R(RS2); return Success;
    case 0xac: MI.setOpcode(LNP64::MULHSU); R(RD); R(RS1); R(RS2); return Success;
    case 0xb6: MI.setOpcode(LNP64::ROL); R(RD); R(RS1); R(RS2); return Success;
    case 0xb7: MI.setOpcode(LNP64::ROR); R(RD); R(RS1); R(RS2); return Success;

    case 0x17: MI.setOpcode(LNP64::NOT); R(RD); R(RS1); return Success;
    case 0xad: MI.setOpcode(LNP64::SEXT_B); R(RD); R(RS1); return Success;
    case 0xae: MI.setOpcode(LNP64::SEXT_H); R(RD); R(RS1); return Success;
    case 0xaf: MI.setOpcode(LNP64::SEXT_W); R(RD); R(RS1); return Success;
    case 0xb0: MI.setOpcode(LNP64::ZEXT_B); R(RD); R(RS1); return Success;
    case 0xb1: MI.setOpcode(LNP64::ZEXT_H); R(RD); R(RS1); return Success;
    case 0xb2: MI.setOpcode(LNP64::ZEXT_W); R(RD); R(RS1); return Success;
    case 0xb3: MI.setOpcode(LNP64::CLZ); R(RD); R(RS1); return Success;
    case 0xb4: MI.setOpcode(LNP64::CTZ); R(RD); R(RS1); return Success;
    case 0xb5: MI.setOpcode(LNP64::POPCNT); R(RD); R(RS1); return Success;
    case 0xb8: MI.setOpcode(LNP64::BSWAP16); R(RD); R(RS1); return Success;
    case 0xb9: MI.setOpcode(LNP64::BSWAP32); R(RD); R(RS1); return Success;
    case 0xba: MI.setOpcode(LNP64::BSWAP64); R(RD); R(RS1); return Success;

    case 0xa0: {
      // addi; recognize the li / mov aliases.
      if (RS1 == 0) { MI.setOpcode(LNP64::LI); R(RD); Imm(ImmI); return Success; }
      if (ImmI == 0) { MI.setOpcode(LNP64::MOV); R(RD); R(RS1); return Success; }
      MI.setOpcode(LNP64::ADDI); R(RD); R(RS1); Imm(ImmI); return Success;
    }
    case 0xa1: MI.setOpcode(LNP64::ANDI); R(RD); R(RS1); Imm(ImmI); return Success;
    case 0xa2: MI.setOpcode(LNP64::ORI); R(RD); R(RS1); Imm(ImmI); return Success;
    case 0xa3: MI.setOpcode(LNP64::XORI); R(RD); R(RS1); Imm(ImmI); return Success;
    case 0xa4: MI.setOpcode(LNP64::SLLI); R(RD); R(RS1); Imm(ImmI); return Success;
    case 0xa5: MI.setOpcode(LNP64::SRLI); R(RD); R(RS1); Imm(ImmI); return Success;
    case 0xa6: MI.setOpcode(LNP64::SRAI); R(RD); R(RS1); Imm(ImmI); return Success;
    case 0x1d: MI.setOpcode(LNP64::SLTI); R(RD); R(RS1); Imm(ImmI); return Success;
    case 0x1e: MI.setOpcode(LNP64::SLTIU); R(RD); R(RS1); Imm(ImmI); return Success;

    case 0x20: MI.setOpcode(LNP64::JMP); Imm(ImmU << 3); return Success;
    case 0x27: MI.setOpcode(LNP64::JAL); R(RD); Imm(ImmU << 3); return Success;
    case 0x28:
      if (RD == 0 && RS1 == 1 && ImmI == 0) { MI.setOpcode(LNP64::RET); return Success; }
      MI.setOpcode(LNP64::JALR); R(RD); R(RS1); Imm(ImmI); return Success;
    case 0x21: MI.setOpcode(LNP64::BEQ); R(RS1); R(RS2); Imm(ImmS << 3); return Success;
    case 0x22: MI.setOpcode(LNP64::BNE); R(RS1); R(RS2); Imm(ImmS << 3); return Success;
    case 0x23: MI.setOpcode(LNP64::BLT); R(RS1); R(RS2); Imm(ImmS << 3); return Success;
    case 0x24: MI.setOpcode(LNP64::BGE); R(RS1); R(RS2); Imm(ImmS << 3); return Success;
    case 0x25: MI.setOpcode(LNP64::BLTU); R(RS1); R(RS2); Imm(ImmS << 3); return Success;
    case 0x26: MI.setOpcode(LNP64::BGEU); R(RS1); R(RS2); Imm(ImmS << 3); return Success;

    case 0x30: MI.setOpcode(LNP64::LD); R(RD); R(RS1); Imm(ImmI); return Success;
    case 0x31: MI.setOpcode(LNP64::LWU); R(RD); R(RS1); Imm(ImmI); return Success;
    case 0x32: MI.setOpcode(LNP64::LBU); R(RD); R(RS1); Imm(ImmI); return Success;
    case 0x36: MI.setOpcode(LNP64::LHU); R(RD); R(RS1); Imm(ImmI); return Success;
    case 0x05: MI.setOpcode(LNP64::LW); R(RD); R(RS1); Imm(ImmI); return Success;
    case 0x08: MI.setOpcode(LNP64::LB); R(RD); R(RS1); Imm(ImmI); return Success;
    case 0x09: MI.setOpcode(LNP64::LH); R(RD); R(RS1); Imm(ImmI); return Success;
    case 0x33: MI.setOpcode(LNP64::SD); R(RS2); R(RS1); Imm(ImmS); return Success;
    case 0x34: MI.setOpcode(LNP64::SW); R(RS2); R(RS1); Imm(ImmS); return Success;
    case 0x35: MI.setOpcode(LNP64::SB); R(RS2); R(RS1); Imm(ImmS); return Success;
    case 0x37: MI.setOpcode(LNP64::SH); R(RS2); R(RS1); Imm(ImmS); return Success;

    case 0xc5: MI.setOpcode(LNP64::LR_D); R(RD); R(RS1); return Success;
    case 0xc6: MI.setOpcode(LNP64::SC_D); R(RD); R(RS2); R(RS1); return Success;
    case 0xce: MI.setOpcode(LNP64::ISYNC); R(RD); R(RS1); R(RS2); return Success;
    case 0xcb: MI.setOpcode(LNP64::FUTEX_WAIT); R(RD); R(RS1); return Success;
    case 0xcc: MI.setOpcode(LNP64::FUTEX_WAKE); R(RD); R(RS1); return Success;

    case 0x54: MI.setOpcode(LNP64::GET_PCR); R(RD); Pcr(RS1); return Success;
    case 0x55: MI.setOpcode(LNP64::SET_PCR); R(RD); Pcr(RS1); R(RS2); return Success;

    case 0x38: MI.setOpcode(LNP64::ERRNO_GET); R(RD); return Success;
    case 0x39: MI.setOpcode(LNP64::ERRNO_SET); R(RD); return Success;
    case 0x3a: MI.setOpcode(LNP64::EXIT); R(RD); return Success;
    case 0x7d: MI.setOpcode(LNP64::FORK); R(RD); return Success;
    case 0x7e: MI.setOpcode(LNP64::WAIT_PID); R(RD); R(RS1); return Success;
    case 0x7f: MI.setOpcode(LNP64::EXEC); R(RD); R(RS1); R(RS2); return Success;
    case 0x47: MI.setOpcode(LNP64::ALLOC); R(RD); R(RS1); return Success;
    case 0x48: MI.setOpcode(LNP64::ALLOC_SIZE); R(RD); R(RS1); return Success;
    case 0x49: MI.setOpcode(LNP64::FREE); R(RD); return Success;
    case 0x4a: MI.setOpcode(LNP64::ALLOC_EX); R(RD); R(RS1); R(RS2); return Success;
    case 0x4b: MI.setOpcode(LNP64::OBJECT_CTL); R(RD); R(RS1); return Success;
    case 0x4c: MI.setOpcode(LNP64::DOMAIN_CTL); R(RD); R(RS1); return Success;
    case 0x4d: MI.setOpcode(LNP64::AWAIT); R(RD); R(RS1); R(RS2); R(RS3); return Success;
    case 0x4e: MI.setOpcode(LNP64::GATE_CALL); R(RD); R(RS1); R(RS2); R(RS3); return Success;
    case 0x4f: MI.setOpcode(LNP64::GATE_RETURN); R(RD); R(RS1); R(RS2); R(RS3); return Success;
    case 0x50: MI.setOpcode(LNP64::CAP_DUP); R(RD); R(RS1); return Success;
    case 0x51: MI.setOpcode(LNP64::CAP_SEND); R(RD); R(RS1); return Success;
    case 0x52: MI.setOpcode(LNP64::CAP_RECV); R(RD); R(RS1); return Success;
    case 0x53: MI.setOpcode(LNP64::CAP_REVOKE); R(RD); R(RS1); return Success;
    case 0x56: MI.setOpcode(LNP64::ENV_GET); R(RD); R(RS1); R(RS2); R(RS3); return Success;
    case 0x58: MI.setOpcode(LNP64::OPEN_AT); R(RD); R(RS1); R(RS2); R(RS3); return Success;
    case 0x59: MI.setOpcode(LNP64::CLONE_SPAWN); R(RD); R(RS1); R(RS2); return Success;
    case 0x5a: MI.setOpcode(LNP64::THREAD_JOIN); R(RD); R(RS1); R(RS2); return Success;
    case 0x5c: MI.setOpcode(LNP64::STAT_PATH_AT); R(RD); R(RS1); R(RS2); R(RS3); return Success;
    case 0x5d: MI.setOpcode(LNP64::STAT_FD_DYN); R(RD); R(RS1); return Success;
    case 0x5e: MI.setOpcode(LNP64::UTIME_PATH_AT); R(RD); R(RS1); R(RS2); R(RS3); return Success;
    case 0x5f: MI.setOpcode(LNP64::UTIME_FD_DYN); R(RD); R(RS1); return Success;
    case 0x6a:
      MI.setOpcode(LNP64::MMAP); R(RD); R(RS1); R(RS2); R(RS3); R(RS4); R(RS5);
      return Success;
    case 0x61: MI.setOpcode(LNP64::MUNMAP); R(RD); R(RS1); return Success;
    case 0x6c: MI.setOpcode(LNP64::MPROTECT); R(RD); R(RS1); R(RS2); R(RS3); return Success;
    case 0x60: MI.setOpcode(LNP64::MMAP_BOOTSTRAP); R(RD); R(RS1); R(RS2); R(RS3); return Success;
    case 0x66: MI.setOpcode(LNP64::MPROTECT_BOOTSTRAP); R(RD); R(RS1); R(RS2); R(RS3); return Success;
    case 0x62: MI.setOpcode(LNP64::SIGACTION); R(RD); R(RS1); return Success;
    case 0x63: MI.setOpcode(LNP64::SIGMASK_SET); R(RD); return Success;
    case 0x64: MI.setOpcode(LNP64::LNP64_KILL); R(RD); R(RS1); return Success;
    case 0x67: MI.setOpcode(LNP64::FCNTL_FD_DYN); R(RD); R(RS1); R(RS2); return Success;
    case 0x68: MI.setOpcode(LNP64::ALARM); R(RD); R(RS1); return Success;
    case 0x69: MI.setOpcode(LNP64::FD_SEEK_DYN); R(RD); R(RS1); R(RS2); return Success;
    case 0x6b: MI.setOpcode(LNP64::UNLINK_PATH_AT); R(RD); R(RS1); R(RS2); return Success;
    case 0x73: MI.setOpcode(LNP64::OPEN_DIR_DYN); R(RD); R(RS1); R(RS2); return Success;
    case 0x74: MI.setOpcode(LNP64::MKDIR_PATH_AT); R(RD); R(RS1); R(RS2); return Success;
    case 0x75: MI.setOpcode(LNP64::RENAME_PATH_AT); R(RD); R(RS1); R(RS2); R(RS3); return Success;
    case 0x76:
      MI.setOpcode(LNP64::LINK_PATH_AT); R(RD); R(RS1); R(RS2); R(RS3); R(RS4);
      return Success;
    case 0x77: MI.setOpcode(LNP64::SYMLINK_PATH_AT); R(RD); R(RS1); R(RS2); return Success;
    case 0x78: MI.setOpcode(LNP64::READLINK_PATH_AT); R(RD); R(RS1); R(RS2); R(RS3); return Success;
    case 0x79: MI.setOpcode(LNP64::CHDIR_PATH); R(RD); return Success;
    case 0x7a: MI.setOpcode(LNP64::GETCWD_PATH); R(RD); R(RS1); return Success;
    case 0x7b: MI.setOpcode(LNP64::CHMOD_PATH_AT); R(RD); R(RS1); R(RS2); R(RS3); return Success;
    case 0x7c:
      MI.setOpcode(LNP64::CHOWN_PATH_AT); R(RD); R(RS1); R(RS2); R(RS3); R(RS4);
      return Success;
    case 0xcf: MI.setOpcode(LNP64::READDIR_FD_DYN); R(RD); R(RS1); return Success;
    case 0x2b: MI.setOpcode(LNP64::PULL); R(RD); R(RS1); R(RS2); R(RS3); return Success;
    case 0x2c: MI.setOpcode(LNP64::PUSH); R(RD); R(RS1); R(RS2); R(RS3); return Success;

    default:
      return MCDisassembler::Fail;
    }
  }
};

} // end anonymous namespace

static MCDisassembler *createLNP64Disassembler(const Target &,
                                               const MCSubtargetInfo &STI,
                                               MCContext &Ctx) {
  return new LNP64Disassembler(STI, Ctx);
}

extern "C" LLVM_EXTERNAL_VISIBILITY void LLVMInitializeLNP64Disassembler() {
  TargetRegistry::RegisterMCDisassembler(getTheLNP64Target(),
                                         createLNP64Disassembler);
}

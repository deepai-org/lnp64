//===-- LNP64MCCodeEmitter.cpp - v2 64-bit encoder -----------------------===//
//
// Hand-written v2 encoder. Every instruction is one 8-byte little-endian word:
//   opcode[63:56] rd[55:51] rs1[50:46] rs2[45:41] rs3[40:36] rs4[35:31]
//   rs5[30:26]; I-type imm32 [45:14]; S/B-type imm32 [40:9]; U/J-type [50:19].
//
//===----------------------------------------------------------------------===//

#include "LNP64MCTargetDesc.h"
#include "llvm/MC/MCCodeEmitter.h"
#include "llvm/MC/MCFixup.h"
#include "llvm/MC/MCInst.h"
#include "llvm/Support/ErrorHandling.h"
#include "llvm/Support/MathExtras.h"
#include "llvm/Support/raw_ostream.h"

using namespace llvm;

namespace {

static void emitLE64(uint64_t Word, raw_ostream &OS) {
  char Bytes[8];
  for (unsigned I = 0; I < 8; ++I)
    Bytes[I] = static_cast<char>(Word >> (8 * I));
  OS.write(Bytes, sizeof(Bytes));
}

// Field shift positions in the 64-bit word.
static constexpr unsigned SH_OP = 56;
static constexpr unsigned SH_RD = 51;
static constexpr unsigned SH_RS1 = 46;
static constexpr unsigned SH_RS2 = 41;
static constexpr unsigned SH_RS3 = 36;
static constexpr unsigned SH_RS4 = 31;
static constexpr unsigned SH_RS5 = 26;
static constexpr unsigned SH_IMM_I = 14; // I-type imm32 [45:14]
static constexpr unsigned SH_IMM_S = 9;  // S/B-type imm32 [40:9]
static constexpr unsigned SH_IMM_U = 19; // U/J-type imm32 [50:19]

static uint64_t op(uint8_t Opcode) { return uint64_t(Opcode) << SH_OP; }
static uint64_t reg(unsigned Shift, unsigned R) {
  return uint64_t(R & 0x1f) << Shift;
}
static uint64_t imm32At(unsigned Shift, int64_t Imm) {
  return (uint64_t(uint32_t(Imm))) << Shift;
}

static unsigned getGPRNo(const MCOperand &Operand) {
  unsigned Reg = Operand.getReg();
  if (Reg < LNP64::R0 || Reg > LNP64::R31)
    llvm_unreachable("expected LNP64 GPR operand");
  return Reg - LNP64::R0;
}

static unsigned getPCRNo(const MCOperand &Operand) {
  switch (Operand.getReg()) {
  case LNP64::PID: return 0;
  case LNP64::PPID: return 1;
  case LNP64::TID: return 2;
  case LNP64::TP: return 3;
  case LNP64::UID: return 4;
  case LNP64::GID: return 5;
  case LNP64::SIGMASK: return 6;
  case LNP64::SIGPENDING: return 7;
  case LNP64::REALTIME_SEC: return 8;
  case LNP64::REALTIME_NSEC: return 9;
  case LNP64::CRED_PROFILE: return 10;
  case LNP64::CRED_HANDLE: return 11;
  default:
    llvm_unreachable("expected LNP64 PCR operand");
  }
}

// Encode an R-type word with up to 5 register slots after rd.
static uint64_t rType(uint8_t Opcode, const MCInst &MI, unsigned NumRegs) {
  static const unsigned Shifts[6] = {SH_RD, SH_RS1, SH_RS2, SH_RS3, SH_RS4,
                                     SH_RS5};
  uint64_t W = op(Opcode);
  for (unsigned I = 0; I < NumRegs; ++I)
    W |= reg(Shifts[I], getGPRNo(MI.getOperand(I)));
  return W;
}

// I-type: rd, rs1, imm32.
static uint64_t iType(uint8_t Opcode, unsigned Rd, unsigned Rs1, int64_t Imm) {
  return op(Opcode) | reg(SH_RD, Rd) | reg(SH_RS1, Rs1) |
         imm32At(SH_IMM_I, Imm);
}

// S-type: rs1(base), rs2(src), imm32. rd slot is zero.
static uint64_t sType(uint8_t Opcode, unsigned Base, unsigned Src, int64_t Imm) {
  return op(Opcode) | reg(SH_RS1, Base) | reg(SH_RS2, Src) |
         imm32At(SH_IMM_S, Imm);
}

class LNP64MCCodeEmitter final : public MCCodeEmitter {
public:
  void encodeInstruction(const MCInst &MI, raw_ostream &OS,
                         SmallVectorImpl<MCFixup> &Fixups,
                         const MCSubtargetInfo &) const override {
    uint64_t W = 0;
    switch (MI.getOpcode()) {
    // No-operand.
    case LNP64::NOP: W = op(0x00); break;
    case LNP64::YIELD: W = op(0x06); break;
    case LNP64::FENCE: W = op(0xcd); break;
    case LNP64::SIGRET: W = op(0x65); break;
    case LNP64::RET:
      // ret = jalr r0, r1, 0.
      W = iType(0x28, 0, 1, 0);
      break;

    // Constants / moves.
    case LNP64::LI:
      // li rd, imm == addi rd, r0, imm.
      W = iType(0xa0, getGPRNo(MI.getOperand(0)), 0,
                MI.getOperand(1).getImm());
      break;
    case LNP64::MOV:
      // mov rd, rs == addi rd, rs, 0.
      W = iType(0xa0, getGPRNo(MI.getOperand(0)),
                getGPRNo(MI.getOperand(1)), 0);
      break;
    case LNP64::LIU:
      W = iType(0x04, getGPRNo(MI.getOperand(0)),
                getGPRNo(MI.getOperand(1)), MI.getOperand(2).getImm());
      break;
    case LNP64::AUIPC: {
      unsigned Rd = getGPRNo(MI.getOperand(0));
      W = op(0xd0) | reg(SH_RD, Rd);
      const MCOperand &Tgt = MI.getOperand(1);
      if (Tgt.isImm())
        W |= imm32At(SH_IMM_U, Tgt.getImm());
      else if (Tgt.isExpr())
        Fixups.push_back(MCFixup::create(
            0, Tgt.getExpr(), MCFixupKind(LNP64::fixup_lnp64_auipc)));
      break;
    }

    // Integer ALU R-type.
    case LNP64::ADD: W = rType(0x10, MI, 3); break;
    case LNP64::SUB: W = rType(0x11, MI, 3); break;
    case LNP64::MUL: W = rType(0x12, MI, 3); break;
    case LNP64::DIV: W = rType(0x13, MI, 3); break;
    case LNP64::AND: W = rType(0x14, MI, 3); break;
    case LNP64::OR: W = rType(0x15, MI, 3); break;
    case LNP64::XOR: W = rType(0x16, MI, 3); break;
    case LNP64::SLL: W = rType(0x18, MI, 3); break;
    case LNP64::SRL: W = rType(0x19, MI, 3); break;
    case LNP64::SRA: W = rType(0x1a, MI, 3); break;
    case LNP64::SLT: W = rType(0x1b, MI, 3); break;
    case LNP64::SLTU: W = rType(0x1c, MI, 3); break;
    case LNP64::UDIV: W = rType(0xa7, MI, 3); break;
    case LNP64::SREM: W = rType(0xa8, MI, 3); break;
    case LNP64::UREM: W = rType(0xa9, MI, 3); break;
    case LNP64::MULH: W = rType(0xaa, MI, 3); break;
    case LNP64::MULHU: W = rType(0xab, MI, 3); break;
    case LNP64::MULHSU: W = rType(0xac, MI, 3); break;
    case LNP64::ROL: W = rType(0xb6, MI, 3); break;
    case LNP64::ROR: W = rType(0xb7, MI, 3); break;

    // Unary R-type.
    case LNP64::NOT: W = rType(0x17, MI, 2); break;
    case LNP64::SEXT_B: W = rType(0xad, MI, 2); break;
    case LNP64::SEXT_H: W = rType(0xae, MI, 2); break;
    case LNP64::SEXT_W: W = rType(0xaf, MI, 2); break;
    case LNP64::ZEXT_B: W = rType(0xb0, MI, 2); break;
    case LNP64::ZEXT_H: W = rType(0xb1, MI, 2); break;
    case LNP64::ZEXT_W: W = rType(0xb2, MI, 2); break;
    case LNP64::CLZ: W = rType(0xb3, MI, 2); break;
    case LNP64::CTZ: W = rType(0xb4, MI, 2); break;
    case LNP64::POPCNT: W = rType(0xb5, MI, 2); break;
    case LNP64::BSWAP16: W = rType(0xb8, MI, 2); break;
    case LNP64::BSWAP32: W = rType(0xb9, MI, 2); break;
    case LNP64::BSWAP64: W = rType(0xba, MI, 2); break;

    // Register-immediate I-type.
    case LNP64::ADDI:
      W = iType(0xa0, getGPRNo(MI.getOperand(0)), getGPRNo(MI.getOperand(1)),
                MI.getOperand(2).getImm());
      break;
    case LNP64::ANDI:
      W = iType(0xa1, getGPRNo(MI.getOperand(0)), getGPRNo(MI.getOperand(1)),
                MI.getOperand(2).getImm());
      break;
    case LNP64::ORI:
      W = iType(0xa2, getGPRNo(MI.getOperand(0)), getGPRNo(MI.getOperand(1)),
                MI.getOperand(2).getImm());
      break;
    case LNP64::XORI:
      W = iType(0xa3, getGPRNo(MI.getOperand(0)), getGPRNo(MI.getOperand(1)),
                MI.getOperand(2).getImm());
      break;
    case LNP64::SLLI:
      W = iType(0xa4, getGPRNo(MI.getOperand(0)), getGPRNo(MI.getOperand(1)),
                MI.getOperand(2).getImm());
      break;
    case LNP64::SRLI:
      W = iType(0xa5, getGPRNo(MI.getOperand(0)), getGPRNo(MI.getOperand(1)),
                MI.getOperand(2).getImm());
      break;
    case LNP64::SRAI:
      W = iType(0xa6, getGPRNo(MI.getOperand(0)), getGPRNo(MI.getOperand(1)),
                MI.getOperand(2).getImm());
      break;
    case LNP64::SLTI:
      W = iType(0x1d, getGPRNo(MI.getOperand(0)), getGPRNo(MI.getOperand(1)),
                MI.getOperand(2).getImm());
      break;
    case LNP64::SLTIU:
      W = iType(0x1e, getGPRNo(MI.getOperand(0)), getGPRNo(MI.getOperand(1)),
                MI.getOperand(2).getImm());
      break;
    case LNP64::JALR:
      W = iType(0x28, getGPRNo(MI.getOperand(0)), getGPRNo(MI.getOperand(1)),
                MI.getOperand(2).getImm());
      break;

    // Control transfer.
    case LNP64::JMP:
      W = emitJump(0x20, 0, MI.getOperand(0), Fixups);
      break;
    case LNP64::JAL:
      W = emitJump(0x27, getGPRNo(MI.getOperand(0)), MI.getOperand(1), Fixups);
      break;
    case LNP64::PseudoCALL:
      // jal r1, target.
      W = emitJump(0x27, 1, MI.getOperand(0), Fixups);
      break;
    case LNP64::PseudoCALLIndirect:
      // jalr r1, target, 0.
      W = iType(0x28, 1, getGPRNo(MI.getOperand(0)), 0);
      break;
    case LNP64::BEQ: W = emitBranch(0x21, MI, Fixups); break;
    case LNP64::BNE: W = emitBranch(0x22, MI, Fixups); break;
    case LNP64::BLT: W = emitBranch(0x23, MI, Fixups); break;
    case LNP64::BGE: W = emitBranch(0x24, MI, Fixups); break;
    case LNP64::BLTU: W = emitBranch(0x25, MI, Fixups); break;
    case LNP64::BGEU: W = emitBranch(0x26, MI, Fixups); break;

    // Loads (I-type) / stores (S-type).
    case LNP64::LD:
      W = iType(0x30, getGPRNo(MI.getOperand(0)), getGPRNo(MI.getOperand(1)),
                MI.getOperand(2).getImm());
      break;
    case LNP64::LWU:
      W = iType(0x31, getGPRNo(MI.getOperand(0)), getGPRNo(MI.getOperand(1)),
                MI.getOperand(2).getImm());
      break;
    case LNP64::LBU:
      W = iType(0x32, getGPRNo(MI.getOperand(0)), getGPRNo(MI.getOperand(1)),
                MI.getOperand(2).getImm());
      break;
    case LNP64::LHU:
      W = iType(0x36, getGPRNo(MI.getOperand(0)), getGPRNo(MI.getOperand(1)),
                MI.getOperand(2).getImm());
      break;
    case LNP64::LW:
      W = iType(0x05, getGPRNo(MI.getOperand(0)), getGPRNo(MI.getOperand(1)),
                MI.getOperand(2).getImm());
      break;
    case LNP64::LB:
      W = iType(0x08, getGPRNo(MI.getOperand(0)), getGPRNo(MI.getOperand(1)),
                MI.getOperand(2).getImm());
      break;
    case LNP64::LH:
      W = iType(0x09, getGPRNo(MI.getOperand(0)), getGPRNo(MI.getOperand(1)),
                MI.getOperand(2).getImm());
      break;
    case LNP64::SD:
      W = sType(0x33, getGPRNo(MI.getOperand(1)), getGPRNo(MI.getOperand(0)),
                MI.getOperand(2).getImm());
      break;
    case LNP64::SW:
      W = sType(0x34, getGPRNo(MI.getOperand(1)), getGPRNo(MI.getOperand(0)),
                MI.getOperand(2).getImm());
      break;
    case LNP64::SB:
      W = sType(0x35, getGPRNo(MI.getOperand(1)), getGPRNo(MI.getOperand(0)),
                MI.getOperand(2).getImm());
      break;
    case LNP64::SH:
      W = sType(0x37, getGPRNo(MI.getOperand(1)), getGPRNo(MI.getOperand(0)),
                MI.getOperand(2).getImm());
      break;

    // Atomics.
    case LNP64::LR_D: W = rType(0xc5, MI, 2); break;
    case LNP64::SC_D:
      // sc.d rd, rs2, (rs1): rd in rd slot, rs2 src, rs1 base.
      W = op(0xc6) | reg(SH_RD, getGPRNo(MI.getOperand(0))) |
          reg(SH_RS1, getGPRNo(MI.getOperand(2))) |
          reg(SH_RS2, getGPRNo(MI.getOperand(1)));
      break;
    case LNP64::ISYNC: W = rType(0xce, MI, 3); break;
    case LNP64::FUTEX_WAIT: W = rType(0xcb, MI, 2); break;
    case LNP64::FUTEX_WAKE: W = rType(0xcc, MI, 2); break;

    // PCR.
    case LNP64::GET_PCR:
      W = op(0x54) | reg(SH_RD, getGPRNo(MI.getOperand(0))) |
          reg(SH_RS1, getPCRNo(MI.getOperand(1)));
      break;
    case LNP64::SET_PCR:
      W = op(0x55) | reg(SH_RD, getGPRNo(MI.getOperand(0))) |
          reg(SH_RS1, getPCRNo(MI.getOperand(1))) |
          reg(SH_RS2, getGPRNo(MI.getOperand(2)));
      break;

    // System / capability / FDR / path primitives.
    case LNP64::ERRNO_GET: W = rType(0x38, MI, 1); break;
    case LNP64::ERRNO_SET: W = rType(0x39, MI, 1); break;
    case LNP64::EXIT: W = rType(0x3a, MI, 1); break;
    case LNP64::FORK: W = rType(0x7d, MI, 1); break;
    case LNP64::WAIT_PID: W = rType(0x7e, MI, 2); break;
    case LNP64::EXEC: W = rType(0x7f, MI, 3); break;
    case LNP64::ALLOC: W = rType(0x47, MI, 2); break;
    case LNP64::ALLOC_SIZE: W = rType(0x48, MI, 2); break;
    case LNP64::FREE: W = rType(0x49, MI, 1); break;
    case LNP64::ALLOC_EX: W = rType(0x4a, MI, 3); break;
    case LNP64::OBJECT_CTL: W = rType(0x4b, MI, 2); break;
    case LNP64::DOMAIN_CTL: W = rType(0x4c, MI, 2); break;
    case LNP64::AWAIT: W = rType(0x4d, MI, 4); break;
    case LNP64::GATE_CALL: W = rType(0x4e, MI, 4); break;
    case LNP64::GATE_RETURN: W = rType(0x4f, MI, 4); break;
    case LNP64::CAP_DUP: W = rType(0x50, MI, 2); break;
    case LNP64::CAP_SEND: W = rType(0x51, MI, 2); break;
    case LNP64::CAP_RECV: W = rType(0x52, MI, 2); break;
    case LNP64::CAP_REVOKE: W = rType(0x53, MI, 2); break;
    case LNP64::ENV_GET: W = rType(0x56, MI, 4); break;
    case LNP64::OPEN_AT: W = rType(0x58, MI, 4); break;
    case LNP64::CLONE_SPAWN: W = rType(0x59, MI, 3); break;
    case LNP64::THREAD_JOIN: W = rType(0x5a, MI, 3); break;
    case LNP64::STAT_PATH_AT: W = rType(0x5c, MI, 4); break;
    case LNP64::STAT_FD_DYN: W = rType(0x5d, MI, 2); break;
    case LNP64::UTIME_PATH_AT: W = rType(0x5e, MI, 4); break;
    case LNP64::UTIME_FD_DYN: W = rType(0x5f, MI, 2); break;
    case LNP64::MMAP:
      // mmap rd, addr, len, prot, fd, off: addr/len/prot in rs1-3, fd->rs4,
      // off->rs5 (single word; no trailing word).
      W = rType(0x6a, MI, 6);
      break;
    case LNP64::MUNMAP: W = rType(0x61, MI, 2); break;
    case LNP64::MPROTECT: W = rType(0x6c, MI, 4); break;
    case LNP64::MMAP_BOOTSTRAP: W = rType(0x60, MI, 4); break;
    case LNP64::MPROTECT_BOOTSTRAP: W = rType(0x66, MI, 4); break;
    case LNP64::SIGACTION: W = rType(0x62, MI, 2); break;
    case LNP64::SIGMASK_SET: W = rType(0x63, MI, 1); break;
    case LNP64::LNP64_KILL: W = rType(0x64, MI, 2); break;
    case LNP64::FCNTL_FD_DYN: W = rType(0x67, MI, 3); break;
    case LNP64::ALARM: W = rType(0x68, MI, 2); break;
    case LNP64::FD_SEEK_DYN: W = rType(0x69, MI, 3); break;
    case LNP64::UNLINK_PATH_AT: W = rType(0x6b, MI, 3); break;
    case LNP64::OPEN_DIR_DYN: W = rType(0x73, MI, 3); break;
    case LNP64::MKDIR_PATH_AT: W = rType(0x74, MI, 3); break;
    case LNP64::RENAME_PATH_AT: W = rType(0x75, MI, 4); break;
    case LNP64::LINK_PATH_AT: W = rType(0x76, MI, 5); break;
    case LNP64::SYMLINK_PATH_AT: W = rType(0x77, MI, 3); break;
    case LNP64::READLINK_PATH_AT: W = rType(0x78, MI, 4); break;
    case LNP64::CHDIR_PATH: W = rType(0x79, MI, 1); break;
    case LNP64::GETCWD_PATH: W = rType(0x7a, MI, 2); break;
    case LNP64::CHMOD_PATH_AT: W = rType(0x7b, MI, 4); break;
    case LNP64::CHOWN_PATH_AT: W = rType(0x7c, MI, 5); break;
    case LNP64::READDIR_FD_DYN: W = rType(0xcf, MI, 2); break;
    case LNP64::PULL: W = rType(0x2b, MI, 4); break;
    case LNP64::PUSH: W = rType(0x2c, MI, 4); break;

    default:
      llvm_unreachable("LNP64 MC encoding for this opcode is not implemented");
    }
    emitLE64(W, OS);
  }

private:
  // B-type compare-and-branch: rs1, rs2, target.
  static uint64_t emitBranch(uint8_t Opcode, const MCInst &MI,
                             SmallVectorImpl<MCFixup> &Fixups) {
    uint64_t W = op(Opcode) | reg(SH_RS1, getGPRNo(MI.getOperand(0))) |
                 reg(SH_RS2, getGPRNo(MI.getOperand(1)));
    const MCOperand &Tgt = MI.getOperand(2);
    if (Tgt.isImm()) {
      int64_t Off = Tgt.getImm();
      W |= imm32At(SH_IMM_S, Off >> 3);
    } else if (Tgt.isExpr()) {
      Fixups.push_back(MCFixup::create(
          0, Tgt.getExpr(), MCFixupKind(LNP64::fixup_lnp64_branch)));
    }
    return W;
  }

  // J-type jump/jal: rd, target.
  static uint64_t emitJump(uint8_t Opcode, unsigned Rd, const MCOperand &Tgt,
                           SmallVectorImpl<MCFixup> &Fixups) {
    uint64_t W = op(Opcode) | reg(SH_RD, Rd);
    if (Tgt.isImm())
      W |= imm32At(SH_IMM_U, Tgt.getImm() >> 3);
    else if (Tgt.isExpr())
      Fixups.push_back(MCFixup::create(
          0, Tgt.getExpr(), MCFixupKind(LNP64::fixup_lnp64_jump)));
    return W;
  }
};

} // end anonymous namespace

MCCodeEmitter *llvm::createLNP64MCCodeEmitter(const MCInstrInfo &,
                                             const MCRegisterInfo &,
                                             MCContext &) {
  return new LNP64MCCodeEmitter();
}

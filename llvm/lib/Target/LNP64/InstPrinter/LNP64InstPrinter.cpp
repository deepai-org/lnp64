//===-- LNP64InstPrinter.cpp - v2 instruction printer --------------------===//

#include "LNP64InstPrinter.h"
#include "MCTargetDesc/LNP64MCTargetDesc.h"
#include "llvm/MC/MCAsmInfo.h"
#include "llvm/MC/MCExpr.h"
#include "llvm/MC/MCInst.h"
#include "llvm/MC/MCInstrInfo.h"
#include "llvm/MC/MCRegisterInfo.h"
#include "llvm/MC/MCSubtargetInfo.h"
#include "llvm/Support/raw_ostream.h"

using namespace llvm;

static const char *getLNP64Mnemonic(unsigned Opcode) {
  switch (Opcode) {
  case LNP64::ADD: return "add";
  case LNP64::ADDI: return "addi";
  case LNP64::SUB: return "sub";
  case LNP64::MUL: return "mul";
  case LNP64::MULH: return "mulh";
  case LNP64::MULHU: return "mulhu";
  case LNP64::MULHSU: return "mulhsu";
  case LNP64::DIV: return "div";
  case LNP64::UDIV: return "udiv";
  case LNP64::SREM: return "srem";
  case LNP64::UREM: return "urem";
  case LNP64::AND: return "and";
  case LNP64::ANDI: return "andi";
  case LNP64::OR: return "or";
  case LNP64::ORI: return "ori";
  case LNP64::XOR: return "xor";
  case LNP64::XORI: return "xori";
  case LNP64::SLL: return "sll";
  case LNP64::SLLI: return "slli";
  case LNP64::SRL: return "srl";
  case LNP64::SRLI: return "srli";
  case LNP64::SRA: return "sra";
  case LNP64::SRAI: return "srai";
  case LNP64::SLT: return "slt";
  case LNP64::SLTU: return "sltu";
  case LNP64::SLTI: return "slti";
  case LNP64::SLTIU: return "sltiu";
  case LNP64::NOT: return "not";
  case LNP64::SEXT_B: return "sext.b";
  case LNP64::SEXT_H: return "sext.h";
  case LNP64::SEXT_W: return "sext.w";
  case LNP64::ZEXT_B: return "zext.b";
  case LNP64::ZEXT_H: return "zext.h";
  case LNP64::ZEXT_W: return "zext.w";
  case LNP64::CLZ: return "clz";
  case LNP64::CTZ: return "ctz";
  case LNP64::POPCNT: return "popcnt";
  case LNP64::ROL: return "rol";
  case LNP64::ROR: return "ror";
  case LNP64::BSWAP16: return "bswap16";
  case LNP64::BSWAP32: return "bswap32";
  case LNP64::BSWAP64: return "bswap64";
  case LNP64::JMP: return "jmp";
  case LNP64::JAL: return "jal";
  case LNP64::JALR: return "jalr";
  case LNP64::BEQ: return "beq";
  case LNP64::BNE: return "bne";
  case LNP64::BLT: return "blt";
  case LNP64::BGE: return "bge";
  case LNP64::BLTU: return "bltu";
  case LNP64::BGEU: return "bgeu";
  case LNP64::LR_D: return "lr.d";
  case LNP64::SC_D: return "sc.d";
  case LNP64::FENCE: return "fence";
  case LNP64::ISYNC: return "isync";
  case LNP64::FUTEX_WAIT: return "futex_wait";
  case LNP64::FUTEX_WAKE: return "futex_wake";
  case LNP64::ERRNO_GET: return "errno_get";
  case LNP64::ERRNO_SET: return "errno_set";
  case LNP64::FORK: return "fork";
  case LNP64::WAIT_PID: return "wait_pid";
  case LNP64::EXEC: return "exec";
  case LNP64::EXIT: return "exit";
  case LNP64::ALLOC: return "alloc";
  case LNP64::ALLOC_EX: return "alloc_ex";
  case LNP64::ALLOC_SIZE: return "alloc_size";
  case LNP64::FREE: return "free";
  case LNP64::MMAP: return "mmap";
  case LNP64::MUNMAP: return "munmap";
  case LNP64::MPROTECT: return "mprotect";
  case LNP64::GET_PCR: return "get_pcr";
  case LNP64::SET_PCR: return "set_pcr";
  case LNP64::SIGACTION: return "sigaction";
  case LNP64::SIGMASK_SET: return "sigmask_set";
  case LNP64::LNP64_KILL: return "kill";
  case LNP64::SIGRET: return "sigret";
  case LNP64::ALARM: return "alarm";
  case LNP64::ENV_GET: return "env_get";
  case LNP64::OPEN_AT: return "open_at";
  case LNP64::CLONE_SPAWN: return "clone.spawn";
  case LNP64::THREAD_JOIN: return "thread_join";
  case LNP64::OPEN_DIR_DYN: return "open_dir_dyn";
  case LNP64::MKDIR_PATH_AT: return "mkdir_path_at";
  case LNP64::UNLINK_PATH_AT: return "unlink_path_at";
  case LNP64::RENAME_PATH_AT: return "rename_path_at";
  case LNP64::LINK_PATH_AT: return "link_path_at";
  case LNP64::SYMLINK_PATH_AT: return "symlink_path_at";
  case LNP64::READLINK_PATH_AT: return "readlink_path_at";
  case LNP64::CHDIR_PATH: return "chdir_path";
  case LNP64::GETCWD_PATH: return "getcwd_path";
  case LNP64::READDIR_FD_DYN: return "readdir_fd_dyn";
  case LNP64::CHMOD_PATH_AT: return "chmod_path_at";
  case LNP64::CHOWN_PATH_AT: return "chown_path_at";
  case LNP64::STAT_PATH_AT: return "stat_path_at";
  case LNP64::STAT_FD_DYN: return "stat_fd_dyn";
  case LNP64::UTIME_PATH_AT: return "utime_path_at";
  case LNP64::UTIME_FD_DYN: return "utime_fd_dyn";
  case LNP64::FCNTL_FD_DYN: return "fcntl_fd_dyn";
  case LNP64::FD_SEEK_DYN: return "fd_seek_dyn";
  case LNP64::OBJECT_CTL: return "object_ctl";
  case LNP64::DOMAIN_CTL: return "domain_ctl";
  case LNP64::CAP_SEND: return "cap_send";
  case LNP64::CAP_RECV: return "cap_recv";
  case LNP64::CAP_DUP: return "cap_dup";
  case LNP64::CAP_REVOKE: return "cap_revoke";
  case LNP64::AWAIT: return "await";
  case LNP64::GATE_CALL: return "gate_call";
  case LNP64::GATE_RETURN: return "gate_return";
  case LNP64::PULL: return "pull";
  case LNP64::PUSH: return "push";
  case LNP64::LD: return "ld";
  case LNP64::LWU: return "lwu";
  case LNP64::LHU: return "lhu";
  case LNP64::LBU: return "lbu";
  case LNP64::LW: return "lw";
  case LNP64::LH: return "lh";
  case LNP64::LB: return "lb";
  case LNP64::SD: return "sd";
  case LNP64::SW: return "sw";
  case LNP64::SH: return "sh";
  case LNP64::SB: return "sb";
  case LNP64::LI: return "li";
  case LNP64::LIU: return "liu";
  case LNP64::MOV: return "mov";
  case LNP64::AUIPC: return "auipc";
  default: return "";
  }
}

std::pair<const char *, uint64_t>
LNP64InstPrinter::getMnemonic(const MCInst *MI) {
  return std::make_pair(getLNP64Mnemonic(MI->getOpcode()), 0);
}

void LNP64InstPrinter::printRegName(raw_ostream &OS, unsigned Reg) const {
  if (Reg >= LNP64::R0 && Reg <= LNP64::R31) {
    OS << "r" << unsigned(Reg - LNP64::R0);
    return;
  }
  switch (Reg) {
  case LNP64::PID: OS << "PID"; return;
  case LNP64::PPID: OS << "PPID"; return;
  case LNP64::TID: OS << "TID"; return;
  case LNP64::TP: OS << "TP"; return;
  case LNP64::UID: OS << "UID"; return;
  case LNP64::GID: OS << "GID"; return;
  case LNP64::SIGMASK: OS << "SIGMASK"; return;
  case LNP64::SIGPENDING: OS << "SIGPENDING"; return;
  case LNP64::REALTIME_SEC: OS << "REALTIME_SEC"; return;
  case LNP64::REALTIME_NSEC: OS << "REALTIME_NSEC"; return;
  case LNP64::CRED_PROFILE: OS << "CRED_PROFILE"; return;
  case LNP64::CRED_HANDLE: OS << "CRED_HANDLE"; return;
  default: break;
  }
  OS << MRI.getName(Reg);
}

void LNP64InstPrinter::printOperand(const MCOperand &Operand,
                                    raw_ostream &OS) const {
  if (Operand.isReg()) {
    printRegName(OS, Operand.getReg());
    return;
  }
  if (Operand.isImm()) {
    OS << Operand.getImm();
    return;
  }
  Operand.getExpr()->print(OS, &MAI);
}

void LNP64InstPrinter::printMemOperand(const MCInst *MI, unsigned RegOp,
                                       unsigned BaseOp, unsigned OffsetOp,
                                       raw_ostream &OS) const {
  printOperand(MI->getOperand(RegOp), OS);
  OS << ", ";
  printOperand(MI->getOperand(OffsetOp), OS);
  OS << '(';
  printOperand(MI->getOperand(BaseOp), OS);
  OS << ')';
}

void LNP64InstPrinter::printInst(const MCInst *MI, uint64_t, StringRef Annot,
                                 const MCSubtargetInfo &, raw_ostream &OS) {
  unsigned Op = MI->getOpcode();
  switch (Op) {
  case LNP64::NOP: OS << "nop"; break;
  case LNP64::YIELD: OS << "yield"; break;
  case LNP64::FENCE: OS << "fence"; break;
  case LNP64::RET: OS << "ret"; break;
  case LNP64::SIGRET: OS << "sigret"; break;

  case LNP64::LR_D:
    OS << "lr.d ";
    printOperand(MI->getOperand(0), OS);
    OS << ", (";
    printOperand(MI->getOperand(1), OS);
    OS << ')';
    break;
  case LNP64::SC_D:
    OS << "sc.d ";
    printOperand(MI->getOperand(0), OS);
    OS << ", ";
    printOperand(MI->getOperand(1), OS);
    OS << ", (";
    printOperand(MI->getOperand(2), OS);
    OS << ')';
    break;
  case LNP64::JALR:
    OS << "jalr ";
    printOperand(MI->getOperand(0), OS);
    OS << ", ";
    printOperand(MI->getOperand(1), OS);
    OS << ", ";
    printOperand(MI->getOperand(2), OS);
    break;

  case LNP64::LD:
  case LNP64::LWU:
  case LNP64::LHU:
  case LNP64::LBU:
  case LNP64::LW:
  case LNP64::LH:
  case LNP64::LB:
  case LNP64::SD:
  case LNP64::SW:
  case LNP64::SH:
  case LNP64::SB:
    OS << getLNP64Mnemonic(Op) << ' ';
    printMemOperand(MI, 0, 1, 2, OS);
    break;

  default: {
    const char *Mn = getLNP64Mnemonic(Op);
    if (!*Mn) {
      OS << "<unknown lnp64 opcode " << Op << ">";
      break;
    }
    OS << Mn;
    unsigned N = MI->getNumOperands();
    for (unsigned I = 0; I < N; ++I) {
      OS << (I == 0 ? " " : ", ");
      printOperand(MI->getOperand(I), OS);
    }
    break;
  }
  }
  printAnnotation(OS, Annot);
}

MCInstPrinter *llvm::createLNP64MCInstPrinter(const Triple &, unsigned,
                                              const MCAsmInfo &MAI,
                                              const MCInstrInfo &MII,
                                              const MCRegisterInfo &MRI) {
  return new LNP64InstPrinter(MAI, MII, MRI);
}

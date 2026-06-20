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
  case LNP64::ADD:
    return "add";
  case LNP64::ADDI:
    return "addi";
  case LNP64::SUB:
    return "sub";
  case LNP64::MUL:
    return "mul";
  case LNP64::MULH:
    return "mulh";
  case LNP64::MULHU:
    return "mulhu";
  case LNP64::MULHSU:
    return "mulhsu";
  case LNP64::AMO_SWAP:
    return "amo.swap";
  case LNP64::AMO_ADD:
    return "amo.add";
  case LNP64::AMO_AND:
    return "amo.and";
  case LNP64::AMO_OR:
    return "amo.or";
  case LNP64::AMO_XOR:
    return "amo.xor";
  case LNP64::LOCK_CMPXCHG:
    return "lock.cmpxchg";
  case LNP64::FENCE:
    return "fence";
  case LNP64::ISYNC:
    return "isync";
  case LNP64::FUTEX_WAIT:
    return "futex_wait";
  case LNP64::FUTEX_WAKE:
    return "futex_wake";
  case LNP64::DIV:
    return "div";
  case LNP64::UDIV:
    return "udiv";
  case LNP64::SREM:
    return "srem";
  case LNP64::UREM:
    return "urem";
  case LNP64::AND:
    return "and";
  case LNP64::ANDI:
    return "andi";
  case LNP64::OR:
    return "or";
  case LNP64::ORI:
    return "ori";
  case LNP64::XOR:
    return "xor";
  case LNP64::XORI:
    return "xori";
  case LNP64::LSL:
    return "lsl";
  case LNP64::LSLI:
    return "lsli";
  case LNP64::LSR:
    return "lsr";
  case LNP64::LSRI:
    return "lsri";
  case LNP64::ASR:
    return "asr";
  case LNP64::ASRI:
    return "asri";
  case LNP64::NOT:
    return "not";
  case LNP64::SEXT_B:
    return "sext.b";
  case LNP64::SEXT_H:
    return "sext.h";
  case LNP64::SEXT_W:
    return "sext.w";
  case LNP64::ZEXT_B:
    return "zext.b";
  case LNP64::ZEXT_H:
    return "zext.h";
  case LNP64::ZEXT_W:
    return "zext.w";
  case LNP64::CLZ:
    return "clz";
  case LNP64::CTZ:
    return "ctz";
  case LNP64::POPCNT:
    return "popcnt";
  case LNP64::ROL:
    return "rol";
  case LNP64::ROR:
    return "ror";
  case LNP64::BSWAP16:
    return "bswap16";
  case LNP64::BSWAP32:
    return "bswap32";
  case LNP64::BSWAP64:
    return "bswap64";
  case LNP64::CMP:
    return "cmp";
  case LNP64::CMPU:
    return "cmpu";
  case LNP64::CSET_EQ:
    return "cset.eq";
  case LNP64::CSET_NE:
    return "cset.ne";
  case LNP64::CSET_LT:
    return "cset.lt";
  case LNP64::CSET_GT:
    return "cset.gt";
  case LNP64::CSET_LE:
    return "cset.le";
  case LNP64::CSET_GE:
    return "cset.ge";
  case LNP64::CSET_ULT:
    return "cset.ult";
  case LNP64::CSET_UGT:
    return "cset.ugt";
  case LNP64::CSET_ULE:
    return "cset.ule";
  case LNP64::CSET_UGE:
    return "cset.uge";
  case LNP64::CSEL_EQ:
    return "csel.eq";
  case LNP64::CSEL_NE:
    return "csel.ne";
  case LNP64::CSEL_LT:
    return "csel.lt";
  case LNP64::CSEL_GT:
    return "csel.gt";
  case LNP64::CSEL_LE:
    return "csel.le";
  case LNP64::CSEL_GE:
    return "csel.ge";
  case LNP64::CSEL_ULT:
    return "csel.ult";
  case LNP64::CSEL_UGT:
    return "csel.ugt";
  case LNP64::CSEL_ULE:
    return "csel.ule";
  case LNP64::CSEL_UGE:
    return "csel.uge";
  case LNP64::JMP:
    return "jmp";
  case LNP64::BEQ:
    return "beq";
  case LNP64::BNE:
    return "bne";
  case LNP64::BLT:
    return "blt";
  case LNP64::BGT:
    return "bgt";
  case LNP64::BLE:
    return "ble";
  case LNP64::BGE:
    return "bge";
  case LNP64::CALL:
    return "call";
  case LNP64::LR_GET:
    return "lr_get";
  case LNP64::LR_SET:
    return "lr_set";
  case LNP64::ERRNO_GET:
    return "errno_get";
  case LNP64::ERRNO_SET:
    return "errno_set";
  case LNP64::FORK:
    return "fork";
  case LNP64::WAIT_PID:
    return "wait_pid";
  case LNP64::EXEC:
    return "exec";
  case LNP64::EXIT:
    return "exit";
  case LNP64::ALLOC:
    return "alloc";
  case LNP64::ALLOC_EX:
    return "alloc_ex";
  case LNP64::ALLOC_SIZE:
    return "alloc_size";
  case LNP64::FREE:
    return "free";
  case LNP64::MMAP:
    return "mmap";
  case LNP64::MUNMAP:
    return "munmap";
  case LNP64::MPROTECT:
    return "mprotect";
  case LNP64::GET_PCR:
    return "get_pcr";
  case LNP64::SET_PCR:
    return "set_pcr";
  case LNP64::SIGACTION:
    return "sigaction";
  case LNP64::SIGMASK_SET:
    return "sigmask_set";
  case LNP64::LNP64_KILL:
    return "kill";
  case LNP64::SIGRET:
    return "sigret";
  case LNP64::ALARM:
    return "alarm";
  case LNP64::ENV_GET:
    return "env_get";
  case LNP64::OPEN_AT:
    return "open_at";
  case LNP64::CLONE_SPAWN:
    return "clone.spawn";
  case LNP64::THREAD_JOIN:
    return "thread_join";
  case LNP64::OPEN_DIR_DYN:
    return "open_dir_dyn";
  case LNP64::MKDIR_PATH_AT:
    return "mkdir_path_at";
  case LNP64::UNLINK_PATH_AT:
    return "unlink_path_at";
  case LNP64::RENAME_PATH_AT:
    return "rename_path_at";
  case LNP64::LINK_PATH_AT:
    return "link_path_at";
  case LNP64::SYMLINK_PATH_AT:
    return "symlink_path_at";
  case LNP64::READLINK_PATH_AT:
    return "readlink_path_at";
  case LNP64::CHDIR_PATH:
    return "chdir_path";
  case LNP64::GETCWD_PATH:
    return "getcwd_path";
  case LNP64::READDIR_FD_DYN:
    return "readdir_fd_dyn";
  case LNP64::CHMOD_PATH_AT:
    return "chmod_path_at";
  case LNP64::CHOWN_PATH_AT:
    return "chown_path_at";
  case LNP64::STAT_PATH_AT:
    return "stat_path_at";
  case LNP64::STAT_FD_DYN:
    return "stat_fd_dyn";
  case LNP64::UTIME_PATH_AT:
    return "utime_path_at";
  case LNP64::UTIME_FD_DYN:
    return "utime_fd_dyn";
  case LNP64::FCNTL_FD_DYN:
    return "fcntl_fd_dyn";
  case LNP64::FD_SEEK_DYN:
    return "fd_seek_dyn";
  case LNP64::OBJECT_CTL:
    return "object_ctl";
  case LNP64::DOMAIN_CTL:
    return "domain_ctl";
  case LNP64::CAP_SEND:
    return "cap_send";
  case LNP64::CAP_RECV:
    return "cap_recv";
  case LNP64::CAP_DUP:
    return "cap_dup";
  case LNP64::CAP_REVOKE:
    return "cap_revoke";
  case LNP64::AWAIT:
    return "await";
  case LNP64::GATE_CALL:
    return "gate_call";
  case LNP64::GATE_RETURN:
    return "gate_return";
  case LNP64::PULL:
    return "pull";
  case LNP64::PUSH:
    return "push";
  case LNP64::LD:
    return "ld";
  case LNP64::LD_W:
    return "ld.w";
  case LNP64::LD_H:
    return "ld.h";
  case LNP64::LD_B:
    return "ld.b";
  case LNP64::ST:
    return "st";
  case LNP64::ST_W:
    return "st.w";
  case LNP64::ST_H:
    return "st.h";
  case LNP64::ST_B:
    return "st.b";
  case LNP64::LA:
    return "la";
  case LNP64::AUIPC:
    return "auipc";
  case LNP64::LI32:
    return "li32";
  default:
    return "";
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
  if (Reg == LNP64::LR) {
    OS << "lr";
    return;
  }
  if (Reg == LNP64::PID) {
    OS << "PID";
    return;
  }
  if (Reg == LNP64::PPID) {
    OS << "PPID";
    return;
  }
  if (Reg == LNP64::TID) {
    OS << "TID";
    return;
  }
  if (Reg == LNP64::TP) {
    OS << "TP";
    return;
  }
  if (Reg == LNP64::UID) {
    OS << "UID";
    return;
  }
  if (Reg == LNP64::GID) {
    OS << "GID";
    return;
  }
  if (Reg == LNP64::SIGMASK) {
    OS << "SIGMASK";
    return;
  }
  if (Reg == LNP64::SIGPENDING) {
    OS << "SIGPENDING";
    return;
  }
  if (Reg == LNP64::REALTIME_SEC) {
    OS << "REALTIME_SEC";
    return;
  }
  if (Reg == LNP64::REALTIME_NSEC) {
    OS << "REALTIME_NSEC";
    return;
  }
  if (Reg == LNP64::CRED_PROFILE) {
    OS << "CRED_PROFILE";
    return;
  }
  if (Reg == LNP64::CRED_HANDLE) {
    OS << "CRED_HANDLE";
    return;
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
  switch (MI->getOpcode()) {
  case LNP64::NOP:
    OS << "nop";
    break;
  case LNP64::YIELD:
    OS << "yield";
    break;
  case LNP64::FENCE:
    OS << "fence";
    break;
  case LNP64::RET:
    OS << "ret";
    break;
  case LNP64::SIGRET:
    OS << "sigret";
    break;
  case LNP64::LI:
    OS << "li ";
    printOperand(MI->getOperand(0), OS);
    OS << ", ";
    printOperand(MI->getOperand(1), OS);
    break;
  case LNP64::MOV:
    OS << "mov ";
    printOperand(MI->getOperand(0), OS);
    OS << ", ";
    printOperand(MI->getOperand(1), OS);
    break;
  case LNP64::LA:
  case LNP64::AUIPC:
  case LNP64::LI32:
    OS << getLNP64Mnemonic(MI->getOpcode()) << ' ';
    printOperand(MI->getOperand(0), OS);
    OS << ", ";
    printOperand(MI->getOperand(1), OS);
    break;
  case LNP64::ADD:
  case LNP64::ADDI:
  case LNP64::SUB:
  case LNP64::MUL:
  case LNP64::MULH:
  case LNP64::MULHU:
  case LNP64::MULHSU:
  case LNP64::AMO_SWAP:
  case LNP64::AMO_ADD:
  case LNP64::AMO_AND:
  case LNP64::AMO_OR:
  case LNP64::AMO_XOR:
  case LNP64::ISYNC:
  case LNP64::DIV:
  case LNP64::UDIV:
  case LNP64::SREM:
  case LNP64::UREM:
  case LNP64::AND:
  case LNP64::ANDI:
  case LNP64::OR:
  case LNP64::ORI:
  case LNP64::XOR:
  case LNP64::XORI:
  case LNP64::LSL:
  case LNP64::LSLI:
  case LNP64::LSR:
  case LNP64::LSRI:
  case LNP64::ASR:
  case LNP64::ASRI:
  case LNP64::ROL:
  case LNP64::ROR:
  case LNP64::CSEL_EQ:
  case LNP64::CSEL_NE:
  case LNP64::CSEL_LT:
  case LNP64::CSEL_GT:
  case LNP64::CSEL_LE:
  case LNP64::CSEL_GE:
  case LNP64::CSEL_ULT:
  case LNP64::CSEL_UGT:
  case LNP64::CSEL_ULE:
  case LNP64::CSEL_UGE:
  case LNP64::CLONE_SPAWN:
  case LNP64::THREAD_JOIN:
  case LNP64::EXEC:
    OS << getLNP64Mnemonic(MI->getOpcode()) << ' ';
    printOperand(MI->getOperand(0), OS);
    OS << ", ";
    printOperand(MI->getOperand(1), OS);
    OS << ", ";
    printOperand(MI->getOperand(2), OS);
    break;
  case LNP64::NOT:
  case LNP64::CMP:
  case LNP64::CMPU:
  case LNP64::FUTEX_WAIT:
  case LNP64::FUTEX_WAKE:
  case LNP64::WAIT_PID:
  case LNP64::STAT_FD_DYN:
  case LNP64::UTIME_FD_DYN:
  case LNP64::SIGACTION:
  case LNP64::LNP64_KILL:
    OS << getLNP64Mnemonic(MI->getOpcode()) << ' ';
    printOperand(MI->getOperand(0), OS);
    OS << ", ";
    printOperand(MI->getOperand(1), OS);
    break;
  case LNP64::JMP:
  case LNP64::BEQ:
  case LNP64::BNE:
  case LNP64::BLT:
  case LNP64::BGT:
  case LNP64::BLE:
  case LNP64::BGE:
  case LNP64::CALL:
    OS << getLNP64Mnemonic(MI->getOpcode()) << ' ';
    printOperand(MI->getOperand(0), OS);
    break;
  case LNP64::CALL_REG:
    OS << "call_reg ";
    printOperand(MI->getOperand(0), OS);
    break;
  case LNP64::LR_GET:
  case LNP64::LR_SET:
    OS << getLNP64Mnemonic(MI->getOpcode()) << ' ';
    printOperand(MI->getOperand(0), OS);
    break;
  case LNP64::ERRNO_GET:
  case LNP64::ERRNO_SET:
  case LNP64::FORK:
  case LNP64::EXIT:
  case LNP64::FREE:
  case LNP64::SIGMASK_SET:
  case LNP64::CHDIR_PATH:
  case LNP64::CSET_EQ:
  case LNP64::CSET_NE:
  case LNP64::CSET_LT:
  case LNP64::CSET_GT:
  case LNP64::CSET_LE:
  case LNP64::CSET_GE:
  case LNP64::CSET_ULT:
  case LNP64::CSET_UGT:
  case LNP64::CSET_ULE:
  case LNP64::CSET_UGE:
    OS << getLNP64Mnemonic(MI->getOpcode()) << ' ';
    printOperand(MI->getOperand(0), OS);
    break;
  case LNP64::ALLOC:
  case LNP64::ALLOC_SIZE:
  case LNP64::MUNMAP:
  case LNP64::ALARM:
  case LNP64::GETCWD_PATH:
  case LNP64::READDIR_FD_DYN:
  case LNP64::GET_PCR:
  case LNP64::OBJECT_CTL:
  case LNP64::DOMAIN_CTL:
  case LNP64::CAP_SEND:
  case LNP64::CAP_RECV:
  case LNP64::CAP_DUP:
  case LNP64::CAP_REVOKE:
  case LNP64::SEXT_B:
  case LNP64::SEXT_H:
  case LNP64::SEXT_W:
  case LNP64::ZEXT_B:
  case LNP64::ZEXT_H:
  case LNP64::ZEXT_W:
  case LNP64::CLZ:
  case LNP64::CTZ:
  case LNP64::POPCNT:
  case LNP64::BSWAP16:
  case LNP64::BSWAP32:
  case LNP64::BSWAP64:
    OS << getLNP64Mnemonic(MI->getOpcode()) << ' ';
    printOperand(MI->getOperand(0), OS);
    OS << ", ";
    printOperand(MI->getOperand(1), OS);
    break;
  case LNP64::SET_PCR:
    OS << getLNP64Mnemonic(MI->getOpcode()) << ' ';
    printOperand(MI->getOperand(0), OS);
    OS << ", ";
    printOperand(MI->getOperand(1), OS);
    OS << ", ";
    printOperand(MI->getOperand(2), OS);
    break;
  case LNP64::ALLOC_EX:
  case LNP64::FCNTL_FD_DYN:
  case LNP64::FD_SEEK_DYN:
  case LNP64::OPEN_DIR_DYN:
  case LNP64::MKDIR_PATH_AT:
  case LNP64::UNLINK_PATH_AT:
  case LNP64::SYMLINK_PATH_AT:
    OS << getLNP64Mnemonic(MI->getOpcode()) << ' ';
    printOperand(MI->getOperand(0), OS);
    OS << ", ";
    printOperand(MI->getOperand(1), OS);
    OS << ", ";
    printOperand(MI->getOperand(2), OS);
    break;
  case LNP64::AWAIT:
  case LNP64::GATE_CALL:
  case LNP64::GATE_RETURN:
  case LNP64::OPEN_AT:
  case LNP64::PULL:
  case LNP64::PUSH:
  case LNP64::MMAP:
  case LNP64::MPROTECT:
  case LNP64::ENV_GET:
  case LNP64::LOCK_CMPXCHG:
  case LNP64::STAT_PATH_AT:
  case LNP64::UTIME_PATH_AT:
  case LNP64::RENAME_PATH_AT:
  case LNP64::READLINK_PATH_AT:
  case LNP64::CHMOD_PATH_AT:
    OS << getLNP64Mnemonic(MI->getOpcode()) << ' ';
    printOperand(MI->getOperand(0), OS);
    OS << ", ";
    printOperand(MI->getOperand(1), OS);
    OS << ", ";
    printOperand(MI->getOperand(2), OS);
    OS << ", ";
    printOperand(MI->getOperand(3), OS);
    break;
  case LNP64::LINK_PATH_AT:
  case LNP64::CHOWN_PATH_AT:
    OS << getLNP64Mnemonic(MI->getOpcode()) << ' ';
    printOperand(MI->getOperand(0), OS);
    OS << ", ";
    printOperand(MI->getOperand(1), OS);
    OS << ", ";
    printOperand(MI->getOperand(2), OS);
    OS << ", ";
    printOperand(MI->getOperand(3), OS);
    OS << ", ";
    printOperand(MI->getOperand(4), OS);
    break;
  case LNP64::LD:
  case LNP64::LD_W:
  case LNP64::LD_H:
  case LNP64::LD_B:
  case LNP64::ST:
  case LNP64::ST_W:
  case LNP64::ST_H:
  case LNP64::ST_B:
    OS << getLNP64Mnemonic(MI->getOpcode()) << ' ';
    printMemOperand(MI, 0, 1, 2, OS);
    break;
  default:
    OS << "<unknown lnp64 opcode " << MI->getOpcode() << ">";
    break;
  }
  printAnnotation(OS, Annot);
}

MCInstPrinter *llvm::createLNP64MCInstPrinter(const Triple &, unsigned,
                                              const MCAsmInfo &MAI,
                                              const MCInstrInfo &MII,
                                              const MCRegisterInfo &MRI) {
  return new LNP64InstPrinter(MAI, MII, MRI);
}

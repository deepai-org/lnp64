#include "LNP64MCTargetDesc.h"
#include "llvm/BinaryFormat/ELF.h"
#include "llvm/MC/MCAsmBackend.h"
#include "llvm/MC/MCAssembler.h"
#include "llvm/MC/MCELFObjectWriter.h"
#include "llvm/MC/MCFixupKindInfo.h"
#include "llvm/MC/MCInst.h"
#include "llvm/MC/MCObjectWriter.h"
#include "llvm/MC/MCSubtargetInfo.h"
#include "llvm/Support/Endian.h"
#include "llvm/Support/raw_ostream.h"

using namespace llvm;

namespace {

// These numbers MUST match the R_LNP64_* enum in lld/ELF/Arch/LNP64.cpp.
// 4/5/6 there are dynamic relocs (GOT64/GLOB_DAT/RELATIVE); the code fixups
// the emitter produces are 13 (AUIPC), 14 (BRANCH), 15 (JUMP), with the TLS
// slot at 12.
enum : unsigned {
  R_LNP64_ABS64 = 1,
  R_LNP64_ABS32 = 2,
  R_LNP64_PC32 = 3,
  R_LNP64_TLS_TPREL_SLOT64 = 12,
  R_LNP64_AUIPC = 13,    // U-type, (S-PC), field at bit 19
  R_LNP64_BRANCH = 14,   // B-type, (S-PC)>>3, field at bit 9
  R_LNP64_JUMP = 15,     // J-type, (S-PC)>>3, field at bit 19
};

class LNP64ELFObjectWriter final : public MCELFObjectTargetWriter {
public:
  LNP64ELFObjectWriter()
      : MCELFObjectTargetWriter(/*Is64Bit=*/true, ELF::ELFOSABI_NONE,
                                /*EMachine=*/0x6c64,
                                /*HasRelocationAddend=*/true) {}

  unsigned getRelocType(MCContext &, const MCValue &, const MCFixup &Fixup,
                        bool IsPCRel) const override {
    switch (Fixup.getKind()) {
    case FK_Data_8:
      return R_LNP64_ABS64;
    case FK_Data_4:
      return IsPCRel ? R_LNP64_PC32 : R_LNP64_ABS32;
    default:
      break;
    }

    switch (static_cast<unsigned>(Fixup.getKind())) {
    case LNP64::fixup_lnp64_branch:
      return R_LNP64_BRANCH;
    case LNP64::fixup_lnp64_jump:
      return R_LNP64_JUMP;
    case LNP64::fixup_lnp64_auipc:
      return R_LNP64_AUIPC;
    case LNP64::fixup_lnp64_abs32:
      return R_LNP64_ABS32;
    case LNP64::fixup_lnp64_pcrel32:
      return R_LNP64_PC32;
    case LNP64::fixup_lnp64_tls_tprel_slot64:
      return R_LNP64_TLS_TPREL_SLOT64;
    default:
      llvm_unreachable("unknown LNP64 fixup kind");
    }
  }
};

class LNP64AsmBackend final : public MCAsmBackend {
public:
  LNP64AsmBackend() : MCAsmBackend(support::endianness::little) {}

  std::unique_ptr<MCObjectTargetWriter>
  createObjectTargetWriter() const override {
    return std::make_unique<LNP64ELFObjectWriter>();
  }

  unsigned getNumFixupKinds() const override {
    return LNP64::NumTargetFixupKinds;
  }

  const MCFixupKindInfo &getFixupKindInfo(MCFixupKind Kind) const override {
    // All v2 fixups patch a 32-bit immediate field inside the 64-bit word; the
    // field bit-offset within the word is given as TargetOffset.
    static const MCFixupKindInfo Infos[LNP64::NumTargetFixupKinds] = {
        {"fixup_lnp64_branch", 9, 32, MCFixupKindInfo::FKF_IsPCRel},
        {"fixup_lnp64_jump", 19, 32, MCFixupKindInfo::FKF_IsPCRel},
        {"fixup_lnp64_auipc", 19, 32, MCFixupKindInfo::FKF_IsPCRel},
        {"fixup_lnp64_abs32", 0, 32, 0},
        {"fixup_lnp64_pcrel32", 0, 32, MCFixupKindInfo::FKF_IsPCRel},
        {"fixup_lnp64_tls_tprel_slot64", 0, 64, 0},
    };

    if (Kind < FirstTargetFixupKind)
      return MCAsmBackend::getFixupKindInfo(Kind);
    return Infos[static_cast<unsigned>(Kind) - FirstTargetFixupKind];
  }

  bool mayNeedRelaxation(const MCInst &, const MCSubtargetInfo &) const override {
    return false;
  }

  bool fixupNeedsRelaxation(const MCFixup &, uint64_t, const MCRelaxableFragment *,
                            const MCAsmLayout &) const override {
    return false;
  }

  void relaxInstruction(MCInst &, const MCSubtargetInfo &) const override {}

  bool shouldForceRelocation(const MCAssembler &, const MCFixup &Fixup,
                             const MCValue &) override {
    switch (static_cast<unsigned>(Fixup.getKind())) {
    case LNP64::fixup_lnp64_branch:
    case LNP64::fixup_lnp64_jump:
    case LNP64::fixup_lnp64_auipc:
    case LNP64::fixup_lnp64_abs32:
    case LNP64::fixup_lnp64_pcrel32:
    case LNP64::fixup_lnp64_tls_tprel_slot64:
      return true;
    default:
      return false;
    }
  }

  void applyFixup(const MCAssembler &, const MCFixup &Fixup, const MCValue &,
                  MutableArrayRef<char> Data, uint64_t Value, bool,
                  const MCSubtargetInfo *) const override {
    unsigned Offset = Fixup.getOffset();
    if (Offset + 8 > Data.size())
      return;

    uint64_t Word = read64(Data, Offset);
    switch (static_cast<unsigned>(Fixup.getKind())) {
    case LNP64::fixup_lnp64_branch: {
      uint32_t Field = uint32_t(int64_t(Value) >> 3);
      Word |= (uint64_t(Field) << 9);
      break;
    }
    case LNP64::fixup_lnp64_jump: {
      uint32_t Field = uint32_t(int64_t(Value) >> 3);
      Word |= (uint64_t(Field) << 19);
      break;
    }
    case LNP64::fixup_lnp64_auipc: {
      uint32_t Field = uint32_t(Value);
      Word |= (uint64_t(Field) << 19);
      break;
    }
    default:
      return;
    }
    write64(Data, Offset, Word);
  }

  bool writeNopData(raw_ostream &OS, uint64_t Count,
                    const MCSubtargetInfo *) const override {
    if (Count % 8 != 0)
      return false;
    for (uint64_t I = 0; I != Count; ++I)
      OS << '\0';
    return true;
  }

private:
  static uint64_t read64(MutableArrayRef<char> Data, unsigned Offset) {
    uint64_t V = 0;
    for (unsigned I = 0; I < 8; ++I)
      V |= uint64_t(uint8_t(Data[Offset + I])) << (8 * I);
    return V;
  }

  static void write64(MutableArrayRef<char> Data, unsigned Offset,
                      uint64_t Value) {
    for (unsigned I = 0; I < 8; ++I)
      Data[Offset + I] = char(Value >> (8 * I));
  }
};

} // end anonymous namespace

MCAsmBackend *llvm::createLNP64AsmBackend(const Target &,
                                          const MCSubtargetInfo &,
                                          const MCRegisterInfo &,
                                          const MCTargetOptions &) {
  return new LNP64AsmBackend();
}

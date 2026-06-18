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

enum : unsigned {
  R_LNP64_ABS64 = 1,
  R_LNP64_ABS32 = 2,
  R_LNP64_PC32 = 3,
  R_LNP64_BRANCH26 = 4,
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
    case LNP64::fixup_lnp64_abs32:
      return R_LNP64_ABS32;
    case LNP64::fixup_lnp64_pcrel32:
      return R_LNP64_PC32;
    case LNP64::fixup_lnp64_branch26:
      return R_LNP64_BRANCH26;
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
    static const MCFixupKindInfo Infos[LNP64::NumTargetFixupKinds] = {
        {"fixup_lnp64_abs32", 0, 32, 0},
        {"fixup_lnp64_pcrel32", 0, 32, MCFixupKindInfo::FKF_IsPCRel},
        {"fixup_lnp64_branch26", 0, 24, MCFixupKindInfo::FKF_IsPCRel},
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
    return Fixup.getKind() == MCFixupKind(LNP64::fixup_lnp64_abs32) ||
           Fixup.getKind() == MCFixupKind(LNP64::fixup_lnp64_branch26) ||
           Fixup.getKind() == MCFixupKind(LNP64::fixup_lnp64_pcrel32);
  }

  void applyFixup(const MCAssembler &, const MCFixup &Fixup, const MCValue &,
                  MutableArrayRef<char> Data, uint64_t Value, bool,
                  const MCSubtargetInfo *) const override {
    unsigned Offset = Fixup.getOffset();
    if (Offset + 4 > Data.size())
      return;

    switch (static_cast<unsigned>(Fixup.getKind())) {
    case LNP64::fixup_lnp64_branch26:
      write32(Data, Offset,
              (read32(Data, Offset) & 0xff000000) |
                  (static_cast<uint32_t>(Value / 4) & 0x00ffffff));
      return;
    case LNP64::fixup_lnp64_abs32:
      write32(Data, Offset, static_cast<uint32_t>(Value));
      return;
    case LNP64::fixup_lnp64_pcrel32:
      write32(Data, Offset, static_cast<uint32_t>(Value));
      return;
    default:
      return;
    }
  }

  bool writeNopData(raw_ostream &OS, uint64_t Count,
                    const MCSubtargetInfo *) const override {
    if (Count % 4 != 0)
      return false;
    for (uint64_t I = 0; I != Count; ++I)
      OS << '\0';
    return true;
  }

private:
  static uint32_t read32(MutableArrayRef<char> Data, unsigned Offset) {
    return uint8_t(Data[Offset]) | (uint32_t(uint8_t(Data[Offset + 1])) << 8) |
           (uint32_t(uint8_t(Data[Offset + 2])) << 16) |
           (uint32_t(uint8_t(Data[Offset + 3])) << 24);
  }

  static void write32(MutableArrayRef<char> Data, unsigned Offset,
                      uint32_t Value) {
    Data[Offset] = char(Value);
    Data[Offset + 1] = char(Value >> 8);
    Data[Offset + 2] = char(Value >> 16);
    Data[Offset + 3] = char(Value >> 24);
  }
};

} // end anonymous namespace

MCAsmBackend *llvm::createLNP64AsmBackend(const Target &,
                                          const MCSubtargetInfo &,
                                          const MCRegisterInfo &,
                                          const MCTargetOptions &) {
  return new LNP64AsmBackend();
}

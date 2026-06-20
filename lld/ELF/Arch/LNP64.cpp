#include "InputFiles.h"
#include "Symbols.h"
#include "SyntheticSections.h"
#include "Target.h"
#include "llvm/BinaryFormat/ELF.h"
#include "llvm/Support/Endian.h"

using namespace llvm;
using namespace llvm::ELF;
using namespace llvm::support::endian;
using namespace lld;
using namespace lld::elf;

namespace {

enum : uint32_t {
  R_LNP64_NONE = 0,
  R_LNP64_ABS64 = 1,
  R_LNP64_ABS32 = 2,
  R_LNP64_PC32 = 3,
  R_LNP64_BRANCH26 = 4,
  R_LNP64_GOT64 = 5,
  R_LNP64_GLOB_DAT = 6,
  R_LNP64_RELATIVE = 7,
  R_LNP64_TLS_TPREL64 = 8,
  R_LNP64_TLS_DTPREL64 = 9,
  R_LNP64_FDR_DESC64 = 10,
  R_LNP64_CAP_DESC64 = 11,
  R_LNP64_CALLGATE64 = 12,
  R_LNP64_PCREL_HI20 = 13,
  R_LNP64_PCREL_LO12_I = 14,
  R_LNP64_PCREL_LO12_LD = 15,
  R_LNP64_TLS_TPREL_SLOT64 = 16,
};

static bool isInt(int64_t Value, unsigned Bits) {
  int64_t Min = -(int64_t(1) << (Bits - 1));
  int64_t Max = (int64_t(1) << (Bits - 1)) - 1;
  return Value >= Min && Value <= Max;
}

static void relocateBranch26(uint8_t *Loc, uint64_t Val) {
  int64_t Delta = static_cast<int64_t>(Val);
  if (Delta % 4 != 0) {
    error(getErrorLocation(Loc) + "R_LNP64_BRANCH26 target is not aligned");
    return;
  }

  int64_t Scaled = Delta / 4;
  if (!isInt(Scaled, 24)) {
    error(getErrorLocation(Loc) + "R_LNP64_BRANCH26 out of range");
    return;
  }

  uint32_t Word = read32le(Loc);
  write32le(Loc, (Word & 0xff000000) |
                     (static_cast<uint32_t>(Scaled) & 0x00ffffff));
}

class LNP64 final : public TargetInfo {
public:
  LNP64();
  RelExpr getRelExpr(RelType Type, const Symbol &S,
                     const uint8_t *Loc) const override;
  void relocate(uint8_t *Loc, const Relocation &Rel,
                uint64_t Val) const override;
};

} // end anonymous namespace

LNP64::LNP64() {
  copyRel = R_LNP64_NONE;
  relativeRel = R_LNP64_RELATIVE;
  symbolicRel = R_LNP64_ABS64;
  gotRel = R_LNP64_GLOB_DAT;
  gotEntrySize = 8;
  pltEntrySize = 0;
  defaultMaxPageSize = 4096;
}

RelExpr LNP64::getRelExpr(RelType Type, const Symbol &,
                          const uint8_t *) const {
  switch (Type) {
  case R_LNP64_NONE:
    return R_NONE;
  case R_LNP64_ABS64:
  case R_LNP64_ABS32:
  case R_LNP64_GLOB_DAT:
  case R_LNP64_FDR_DESC64:
  case R_LNP64_CAP_DESC64:
  case R_LNP64_CALLGATE64:
    return R_ABS;
  case R_LNP64_RELATIVE:
    return R_ABS;
  case R_LNP64_PC32:
  case R_LNP64_BRANCH26:
  case R_LNP64_PCREL_HI20:
  case R_LNP64_PCREL_LO12_I:
  case R_LNP64_PCREL_LO12_LD:
    return R_PC;
  case R_LNP64_GOT64:
    return R_GOT;
  case R_LNP64_TLS_TPREL64:
  case R_LNP64_TLS_TPREL_SLOT64:
    return R_TPREL;
  case R_LNP64_TLS_DTPREL64:
    return R_DTPREL;
  default:
    return R_NONE;
  }
}

void LNP64::relocate(uint8_t *Loc, const Relocation &Rel, uint64_t Val) const {
  switch (Rel.type) {
  case R_LNP64_NONE:
    return;
  case R_LNP64_ABS64:
  case R_LNP64_GLOB_DAT:
  case R_LNP64_RELATIVE:
  case R_LNP64_TLS_TPREL64:
  case R_LNP64_TLS_TPREL_SLOT64:
  case R_LNP64_TLS_DTPREL64:
  case R_LNP64_FDR_DESC64:
  case R_LNP64_CAP_DESC64:
  case R_LNP64_CALLGATE64:
    write64le(Loc, Val);
    return;
  case R_LNP64_ABS32:
  case R_LNP64_PC32:
    write32le(Loc, Val);
    return;
  case R_LNP64_BRANCH26:
    relocateBranch26(Loc, Val);
    return;
  case R_LNP64_PCREL_HI20:
  case R_LNP64_PCREL_LO12_I:
  case R_LNP64_PCREL_LO12_LD:
    error(getErrorLocation(Loc) +
          "split PC-relative LNP64 relocations are not implemented yet");
    return;
  default:
    error(getErrorLocation(Loc) + "unknown LNP64 relocation");
    return;
  }
}

TargetInfo *elf::getLNP64TargetInfo() {
  static LNP64 Target;
  return &Target;
}

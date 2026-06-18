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
  CopyRel = R_LNP64_NONE;
  RelativeRel = R_LNP64_RELATIVE;
  SymbolicRel = R_LNP64_ABS64;
  GotRel = R_LNP64_GLOB_DAT;
  GotEntrySize = 8;
  PltEntrySize = 0;
  DefaultMaxPageSize = 4096;
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
    return R_RELATIVE;
  case R_LNP64_PC32:
  case R_LNP64_BRANCH26:
    return R_PC;
  case R_LNP64_GOT64:
    return R_GOT;
  case R_LNP64_TLS_TPREL64:
  case R_LNP64_TLS_DTPREL64:
    return R_TLS;
  default:
    return R_INVALID;
  }
}

void LNP64::relocate(uint8_t *Loc, const Relocation &Rel, uint64_t Val) const {
  switch (Rel.Type) {
  case R_LNP64_NONE:
    return;
  case R_LNP64_ABS64:
  case R_LNP64_GLOB_DAT:
  case R_LNP64_RELATIVE:
  case R_LNP64_TLS_TPREL64:
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
  default:
    error(getErrorLocation(Loc) + "unknown LNP64 relocation");
    return;
  }
}

TargetInfo *elf::getLNP64TargetInfo() {
  static LNP64 Target;
  return &Target;
}

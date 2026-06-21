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

// LNP64 ISA v2 relocation numbers. These match
// toolchain/lnp64_relocations.manifest and object_format.md exactly.
//
// Numbers 0-12 are the data / symbol / descriptor / TLS relocations the loader
// and lld resolve. Numbers 13-15 are the MC backend code fixups emitted by the
// LLVM LNP64 code emitter: AUIPC (PC-relative high part, byte granular) and the
// instruction-count BRANCH / JUMP offsets for the fixed 64-bit instruction word.
enum : uint32_t {
  R_LNP64_NONE = 0,
  R_LNP64_ABS64 = 1,
  R_LNP64_ABS32 = 2,
  R_LNP64_PC32 = 3,
  R_LNP64_GOT64 = 4,
  R_LNP64_GLOB_DAT = 5,
  R_LNP64_RELATIVE = 6,
  R_LNP64_TLS_TPREL64 = 7,
  R_LNP64_TLS_DTPREL64 = 8,
  R_LNP64_FDR_DESC64 = 9,
  R_LNP64_CAP_DESC64 = 10,
  R_LNP64_CALLGATE64 = 11,
  R_LNP64_TLS_TPREL_SLOT64 = 12,
  R_LNP64_AUIPC = 13,
  R_LNP64_BRANCH = 14,
  R_LNP64_JUMP = 15,
};

static bool isInt(int64_t Value, unsigned Bits) {
  int64_t Min = -(int64_t(1) << (Bits - 1));
  int64_t Max = (int64_t(1) << (Bits - 1)) - 1;
  return Value >= Min && Value <= Max;
}

// LNP64 v2 instructions are fixed 64-bit words, 8-byte aligned. The 32-bit
// immediate fields used by code relocations live at a fixed bit offset inside
// that word:
//   B-type (branch): imm32 at bit 9   (just above rs2[45:41]? no -- imm[40:9])
//   J-type (jump):   imm32 at bit 19  (imm[50:19])
//   U-type (auipc):  imm32 at bit 19  (imm[50:19])
// We read the 64-bit little-endian word, OR the field into place, and write it
// back. The fixup-emitted instruction word has the field zeroed.
static uint64_t read64(const uint8_t *Loc) { return read64le(Loc); }
static void patchField(uint8_t *Loc, unsigned Shift, uint32_t Field) {
  uint64_t Word = read64(Loc);
  Word |= (uint64_t(Field) << Shift);
  write64le(Loc, Word);
}

// Write a sign-extended instruction-count displacement into a 32-bit immediate
// field at the given bit offset. The architectural value is (S + A - P) >> 3.
static void relocateInstCount(uint8_t *Loc, uint64_t Val, unsigned Shift,
                              const char *Name) {
  int64_t Delta = static_cast<int64_t>(Val);
  if (Delta % 8 != 0) {
    error(getErrorLocation(Loc) + Twine(Name) + " target is not 8-byte aligned");
    return;
  }
  int64_t Words = Delta >> 3;
  if (!isInt(Words, 32)) {
    error(getErrorLocation(Loc) + Twine(Name) + " out of range");
    return;
  }
  patchField(Loc, Shift, static_cast<uint32_t>(Words));
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
  case R_LNP64_RELATIVE:
    return R_ABS;
  case R_LNP64_PC32:
  case R_LNP64_AUIPC:
  case R_LNP64_BRANCH:
  case R_LNP64_JUMP:
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
    // 64-bit data / descriptor / TLS slots: raw 64-bit value (S + A, B + A,
    // or a descriptor index + addend computed by lld's R_ABS/R_TPREL path).
    write64le(Loc, Val);
    return;
  case R_LNP64_ABS32:
    // low 32 bits of S + A.
    write32le(Loc, Val);
    return;
  case R_LNP64_PC32:
    // S + A - P, 32-bit pc-relative data word, raw bytes.
    write32le(Loc, Val);
    return;
  case R_LNP64_AUIPC:
    // U-type AUIPC high part: rd = PC + sext32(S + A - P). The byte delta
    // (S + A - P) goes into the U-type imm32 field at bit 19 [50:19].
    // lld supplies Val = S + A - P for R_PC.
    if (!isInt(static_cast<int64_t>(Val), 32)) {
      error(getErrorLocation(Loc) + "R_LNP64_AUIPC out of range");
      return;
    }
    patchField(Loc, 19, static_cast<uint32_t>(static_cast<int64_t>(Val)));
    return;
  case R_LNP64_BRANCH:
    // B-type instruction-count displacement (S + A - P) >> 3 into imm32 at
    // bit 9 [40:9].
    relocateInstCount(Loc, Val, 9, "R_LNP64_BRANCH");
    return;
  case R_LNP64_JUMP:
    // J-type instruction-count displacement (S + A - P) >> 3 into imm32 at
    // bit 19 [50:19].
    relocateInstCount(Loc, Val, 19, "R_LNP64_JUMP");
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

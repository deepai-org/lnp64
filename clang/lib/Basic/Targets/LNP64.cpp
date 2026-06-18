#include "LNP64.h"
#include "clang/Basic/MacroBuilder.h"
#include "llvm/ADT/StringSwitch.h"

using namespace clang;
using namespace clang::targets;

static const char *const GCCRegNames[] = {
    "r0",  "r1",  "r2",  "r3",  "r4",  "r5",  "r6",  "r7",
    "r8",  "r9",  "r10", "r11", "r12", "r13", "r14", "r15",
    "r16", "r17", "r18", "r19", "r20", "r21", "r22", "r23",
    "r24", "r25", "r26", "r27", "r28", "r29", "r30", "r31",
    "fd0", "fd255", "LR",  "TP",  "PID", "TID"};

LNP64TargetInfo::LNP64TargetInfo(const llvm::Triple &Triple,
                                 const TargetOptions &Opts)
    : TargetInfo(Triple) {
  BigEndian = false;
  TLSSupported = true;
  LongWidth = LongAlign = PointerWidth = PointerAlign = 64;
  IntWidth = IntAlign = 32;
  LongLongWidth = LongLongAlign = 64;
  MaxAtomicPromoteWidth = MaxAtomicInlineWidth = 64;
  SizeType = UnsignedLong;
  PtrDiffType = SignedLong;
  IntPtrType = SignedLong;
  resetDataLayout("e-m:e-p:64:64-i64:64-n64-S128");
}

void LNP64TargetInfo::getTargetDefines(const LangOptions &,
                                       MacroBuilder &Builder) const {
  Builder.defineMacro("__LNP64__");
  Builder.defineMacro("__lnp64__");
  Builder.defineMacro("__ELF__");
}

bool LNP64TargetInfo::isValidCPUName(StringRef Name) const {
  return Name == "generic-lnp64";
}

void LNP64TargetInfo::fillValidCPUList(
    SmallVectorImpl<StringRef> &Values) const {
  Values.emplace_back("generic-lnp64");
}

bool LNP64TargetInfo::setCPU(const std::string &Name) {
  return isValidCPUName(Name);
}

bool LNP64TargetInfo::hasFeature(StringRef Feature) const {
  return llvm::StringSwitch<bool>(Feature).Case("lnp64", true).Default(false);
}

ArrayRef<const char *> LNP64TargetInfo::getGCCRegNames() const {
  return llvm::ArrayRef<const char *>(GCCRegNames);
}

ArrayRef<TargetInfo::GCCRegAlias> LNP64TargetInfo::getGCCRegAliases() const {
  return {};
}

bool LNP64TargetInfo::validateAsmConstraint(
    const char *&Name, TargetInfo::ConstraintInfo &Info) const {
  switch (*Name) {
  case 'r':
    Info.setAllowsRegister();
    return true;
  case 'f':
    Info.setAllowsRegister();
    return true;
  case 'd':
    Info.setAllowsRegister();
    return true;
  case 'v':
    Info.setAllowsRegister();
    return true;
  case 'p':
    Info.setAllowsRegister();
    return true;
  case 'm':
    Info.setAllowsMemory();
    return true;
  case 'i':
    Info.setRequiresImmediate();
    return true;
  default:
    return false;
  }
}

const char *LNP64TargetInfo::getClobbers() const { return ""; }

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
    "f0",  "f1",  "f2",  "f3",  "f4",  "f5",  "f6",  "f7",
    "f8",  "f9",  "f10", "f11", "f12", "f13", "f14", "f15",
    "f16", "f17", "f18", "f19", "f20", "f21", "f22", "f23",
    "f24", "f25", "f26", "f27", "f28", "f29", "f30", "f31",
    // ISA v2 control/credential register names. The v1 architectural link
    // register "LR" and FLAGS are removed: in v2 the return address lives in
    // r1 (ra) and there is no FLAGS register. r30 is a reclaimed GPR and r31
    // is the stack pointer; both are covered by the r0-r31 GPR names above.
    "TP",  "PID", "PPID", "TID", "UID", "GID", "SIGMASK",
    "SIGPENDING", "REALTIME_SEC", "REALTIME_NSEC", "CRED_PROFILE",
    "CRED_HANDLE"};

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
  // ISA version: v2 is the current architecture (fixed 64-bit instructions,
  // r1=ra/r2-r9 args/r2 return/r31=sp ABI, no LR/FLAGS registers).
  Builder.defineMacro("__LNP64_ISA_VERSION__", "2");
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
  // ISA v2 ABI register-name aliases for inline asm. r0=zero, r1=ra (return
  // address; replaces the removed v1 LR register), r2=ret/first-arg, r31=sp.
  static const TargetInfo::GCCRegAlias Aliases[] = {
      {{"zero"}, "r0"},
      {{"ra"}, "r1"},
      {{"sp"}, "r31"},
  };
  return llvm::makeArrayRef(Aliases);
}

bool LNP64TargetInfo::validateAsmConstraint(
    const char *&Name, TargetInfo::ConstraintInfo &Info) const {
  switch (*Name) {
  case 'r':
    Info.setAllowsRegister();
    return true;
  case 'd': // floating-point register (FPR, reserved for future hardware FP)
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

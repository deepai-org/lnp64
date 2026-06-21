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
    "fd0", "fd1", "fd2", "fd3", "fd4", "fd5", "fd6", "fd7",
    "fd8", "fd9", "fd10", "fd11", "fd12", "fd13", "fd14", "fd15",
    "fd16", "fd17", "fd18", "fd19", "fd20", "fd21", "fd22", "fd23",
    "fd24", "fd25", "fd26", "fd27", "fd28", "fd29", "fd30", "fd31",
    "fd32", "fd33", "fd34", "fd35", "fd36", "fd37", "fd38", "fd39",
    "fd40", "fd41", "fd42", "fd43", "fd44", "fd45", "fd46", "fd47",
    "fd48", "fd49", "fd50", "fd51", "fd52", "fd53", "fd54", "fd55",
    "fd56", "fd57", "fd58", "fd59", "fd60", "fd61", "fd62", "fd63",
    "fd64", "fd65", "fd66", "fd67", "fd68", "fd69", "fd70", "fd71",
    "fd72", "fd73", "fd74", "fd75", "fd76", "fd77", "fd78", "fd79",
    "fd80", "fd81", "fd82", "fd83", "fd84", "fd85", "fd86", "fd87",
    "fd88", "fd89", "fd90", "fd91", "fd92", "fd93", "fd94", "fd95",
    "fd96", "fd97", "fd98", "fd99", "fd100", "fd101", "fd102", "fd103",
    "fd104", "fd105", "fd106", "fd107", "fd108", "fd109", "fd110", "fd111",
    "fd112", "fd113", "fd114", "fd115", "fd116", "fd117", "fd118", "fd119",
    "fd120", "fd121", "fd122", "fd123", "fd124", "fd125", "fd126", "fd127",
    "fd128", "fd129", "fd130", "fd131", "fd132", "fd133", "fd134", "fd135",
    "fd136", "fd137", "fd138", "fd139", "fd140", "fd141", "fd142", "fd143",
    "fd144", "fd145", "fd146", "fd147", "fd148", "fd149", "fd150", "fd151",
    "fd152", "fd153", "fd154", "fd155", "fd156", "fd157", "fd158", "fd159",
    "fd160", "fd161", "fd162", "fd163", "fd164", "fd165", "fd166", "fd167",
    "fd168", "fd169", "fd170", "fd171", "fd172", "fd173", "fd174", "fd175",
    "fd176", "fd177", "fd178", "fd179", "fd180", "fd181", "fd182", "fd183",
    "fd184", "fd185", "fd186", "fd187", "fd188", "fd189", "fd190", "fd191",
    "fd192", "fd193", "fd194", "fd195", "fd196", "fd197", "fd198", "fd199",
    "fd200", "fd201", "fd202", "fd203", "fd204", "fd205", "fd206", "fd207",
    "fd208", "fd209", "fd210", "fd211", "fd212", "fd213", "fd214", "fd215",
    "fd216", "fd217", "fd218", "fd219", "fd220", "fd221", "fd222", "fd223",
    "fd224", "fd225", "fd226", "fd227", "fd228", "fd229", "fd230", "fd231",
    "fd232", "fd233", "fd234", "fd235", "fd236", "fd237", "fd238", "fd239",
    "fd240", "fd241", "fd242", "fd243", "fd244", "fd245", "fd246", "fd247",
    "fd248", "fd249", "fd250", "fd251", "fd252", "fd253", "fd254", "fd255",
    "f0",  "f1",  "f2",  "f3",  "f4",  "f5",  "f6",  "f7",
    "f8",  "f9",  "f10", "f11", "f12", "f13", "f14", "f15",
    "f16", "f17", "f18", "f19", "f20", "f21", "f22", "f23",
    "f24", "f25", "f26", "f27", "f28", "f29", "f30", "f31",
    "v0",  "v1",  "v2",  "v3",  "v4",  "v5",  "v6",  "v7",
    "v8",  "v9",  "v10", "v11", "v12", "v13", "v14", "v15",
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

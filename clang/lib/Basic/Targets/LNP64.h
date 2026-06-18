#ifndef LLVM_CLANG_LIB_BASIC_TARGETS_LNP64_H
#define LLVM_CLANG_LIB_BASIC_TARGETS_LNP64_H

#include "clang/Basic/TargetInfo.h"
#include "clang/Basic/TargetOptions.h"
#include "llvm/ADT/ArrayRef.h"
#include "llvm/Support/Compiler.h"
#include "llvm/ADT/Triple.h"

namespace clang {
namespace targets {

class LLVM_LIBRARY_VISIBILITY LNP64TargetInfo : public TargetInfo {
public:
  LNP64TargetInfo(const llvm::Triple &Triple, const TargetOptions &Opts);

  void getTargetDefines(const LangOptions &Opts,
                        MacroBuilder &Builder) const override;
  bool isValidCPUName(StringRef Name) const override;
  void fillValidCPUList(SmallVectorImpl<StringRef> &Values) const override;
  bool setCPU(const std::string &Name) override;
  bool hasFeature(StringRef Feature) const override;
  ArrayRef<const char *> getGCCRegNames() const override;
  ArrayRef<TargetInfo::GCCRegAlias> getGCCRegAliases() const override;
  bool validateAsmConstraint(const char *&Name,
                             TargetInfo::ConstraintInfo &Info) const override;
  const char *getClobbers() const override;

  BuiltinVaListKind getBuiltinVaListKind() const override {
    return VoidPtrBuiltinVaList;
  }
  ArrayRef<Builtin::Info> getTargetBuiltins() const override { return None; }
};

} // end namespace targets
} // end namespace clang

#endif

#ifndef LLVM_CLANG_LIB_BASIC_TARGETS_LNP64_H
#define LLVM_CLANG_LIB_BASIC_TARGETS_LNP64_H

#include "clang/Basic/TargetInfo.h"
#include "llvm/ADT/ArrayRef.h"
#include "llvm/Support/Compiler.h"

namespace clang {
namespace targets {

class LLVM_LIBRARY_VISIBILITY LNP64TargetInfo : public TargetInfo {
public:
  LNP64TargetInfo(const llvm::Triple &Triple, const TargetOptions &Opts);

  void getTargetDefines(const LangOptions &Opts,
                        MacroBuilder &Builder) const override;
  ArrayRef<const char *> getGCCRegNames() const override;
  ArrayRef<TargetInfo::GCCRegAlias> getGCCRegAliases() const override;
  bool validateAsmConstraint(const char *&Name,
                             TargetInfo::ConstraintInfo &Info) const override;
  StringRef getClobbers() const override;

  BuiltinVaListKind getBuiltinVaListKind() const override {
    return VoidPtrBuiltinVaList;
  }
};

} // end namespace targets
} // end namespace clang

#endif

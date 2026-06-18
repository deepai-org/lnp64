#include "clang/Driver/Driver.h"
#include "clang/Driver/Options.h"
#include "llvm/ADT/StringRef.h"
#include "llvm/Option/ArgList.h"
#include <vector>

using namespace clang::driver;
using namespace clang::driver::tools;
using namespace llvm::opt;

namespace clang {
namespace driver {
namespace tools {
namespace lnp64 {

StringRef getLNP64TargetCPU(const ArgList &) { return "generic-lnp64"; }

void getLNP64TargetFeatures(const Driver &, const llvm::Triple &,
                            const ArgList &, std::vector<StringRef> &) {}

void addLNP64TargetArgs(const ArgList &, ArgStringList &CmdArgs) {
  CmdArgs.push_back("-ffreestanding");
  CmdArgs.push_back("-fno-pic");
  CmdArgs.push_back("-I");
  CmdArgs.push_back("toolchain");
}

const char *getLNP64Crt0() { return "toolchain/crt0_lnp64.s"; }
const char *getLNP64Emulation() { return "elf64lnp64"; }
const char *getLNP64LinkerScript() { return "toolchain/lnp64_static.ld"; }

} // end namespace lnp64
} // end namespace tools
} // end namespace driver
} // end namespace clang

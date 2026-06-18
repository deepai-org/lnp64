#include "LNP64MCAsmInfo.h"
#include "llvm/ADT/Triple.h"

using namespace llvm;

void LNP64MCAsmInfo::anchor() {}

LNP64MCAsmInfo::LNP64MCAsmInfo(const Triple &, const MCTargetOptions &) {
  CodePointerSize = 8;
  CalleeSaveStackSlotSize = 8;
  IsLittleEndian = true;
  PrivateGlobalPrefix = ".L";
  WeakRefDirective = "\t.weak\t";
  ExceptionsType = ExceptionHandling::DwarfCFI;
  UsesELFSectionDirectiveForBSS = true;
  SupportsDebugInformation = true;
  MinInstAlignment = 4;
}

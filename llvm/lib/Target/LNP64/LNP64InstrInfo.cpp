#include "LNP64InstrInfo.h"

using namespace llvm;

#define GET_INSTRINFO_CTOR_DTOR
#include "LNP64GenInstrInfo.inc"

LNP64InstrInfo::LNP64InstrInfo() : LNP64GenInstrInfo(LNP64::RET) {}

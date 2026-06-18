include "llvm/Target/Target.td"

include "LNP64RegisterInfo.td"
include "LNP64CallingConv.td"
include "LNP64InstrInfo.td"

def LNP64InstrInfo : InstrInfo;

def LNP64 : Target {
  let InstructionSet = LNP64InstrInfo;
}

def : ProcessorModel<"generic-lnp64", NoSchedModel, []>;

set(LLVM_TARGET_DEFINITIONS LNP64.td)

tablegen(LLVM LNP64GenRegisterInfo.inc -gen-register-info)
tablegen(LLVM LNP64GenInstrInfo.inc -gen-instr-info)
tablegen(LLVM LNP64GenCallingConv.inc -gen-callingconv)
tablegen(LLVM LNP64GenSubtargetInfo.inc -gen-subtarget)

add_public_tablegen_target(LNP64CommonTableGen)

add_llvm_target(LNP64CodeGen
  LINK_COMPONENTS
  CodeGen
  Core
  MC
  Support
  Target
  LNP64Desc
  LNP64Info
)

add_subdirectory(TargetInfo)
add_subdirectory(MCTargetDesc)

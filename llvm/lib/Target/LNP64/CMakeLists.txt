add_llvm_component_group(LNP64)

set(LLVM_TARGET_DEFINITIONS LNP64.td)

tablegen(LLVM LNP64GenRegisterInfo.inc -gen-register-info)
tablegen(LLVM LNP64GenInstrInfo.inc -gen-instr-info)
tablegen(LLVM LNP64GenCallingConv.inc -gen-callingconv)
tablegen(LLVM LNP64GenSubtargetInfo.inc -gen-subtarget)
tablegen(LLVM LNP64GenDAGISel.inc -gen-dag-isel)

add_public_tablegen_target(LNP64CommonTableGen)

add_llvm_target(LNP64CodeGen
  LNP64AsmPrinter.cpp
  LNP64FrameLowering.cpp
  LNP64ISelLowering.cpp
  LNP64ISelDAGToDAG.cpp
  LNP64InstrInfo.cpp
  LNP64RegisterInfo.cpp
  LNP64Subtarget.cpp
  LNP64TargetMachine.cpp

  LINK_COMPONENTS
  AsmPrinter
  CodeGen
  Core
  MC
  SelectionDAG
  Support
  Target
  LNP64Desc
  LNP64Info

  ADD_TO_COMPONENT
  LNP64
)

add_subdirectory(TargetInfo)
add_subdirectory(MCTargetDesc)
add_subdirectory(InstPrinter)
add_subdirectory(AsmParser)
add_subdirectory(Disassembler)

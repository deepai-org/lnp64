add_llvm_component_library(LLVMLNP64Disassembler
  LNP64Disassembler.cpp

  LINK_COMPONENTS
  MC
  MCDisassembler
  Support
  LNP64Desc
  LNP64Info

  ADD_TO_COMPONENT
  LNP64
)

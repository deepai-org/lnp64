add_llvm_component_library(LLVMLNP64AsmParser
  LNP64AsmParser.cpp

  LINK_COMPONENTS
  MC
  MCParser
  Support
  LNP64Desc
  LNP64Info

  ADD_TO_COMPONENT
  LNP64
)

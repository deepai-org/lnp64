add_llvm_component_library(LLVMLNP64InstPrinter
  LNP64InstPrinter.cpp

  LINK_COMPONENTS
  MC
  Support
  LNP64Info

  ADD_TO_COMPONENT
  LNP64
)

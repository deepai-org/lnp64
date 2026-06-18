add_llvm_component_library(LLVMLNP64Info
  LNP64TargetInfo.cpp

  LINK_COMPONENTS
  MC
  Support

  ADD_TO_COMPONENT
  LNP64
)

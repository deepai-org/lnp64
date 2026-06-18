add_llvm_component_library(LLVMLNP64Desc
  LNP64MCTargetDesc.cpp
  LNP64MCCodeEmitter.cpp

  LINK_COMPONENTS
  MC
  Support
  LNP64Info

  ADD_TO_COMPONENT
  LNP64
)

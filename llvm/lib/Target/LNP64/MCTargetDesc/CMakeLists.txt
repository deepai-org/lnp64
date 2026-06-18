add_llvm_component_library(LLVMLNP64Desc
  LNP64MCAsmInfo.cpp
  LNP64MCTargetDesc.cpp
  LNP64MCCodeEmitter.cpp
  LNP64MCAsmBackend.cpp

  LINK_COMPONENTS
  MC
  Support
  LNP64InstPrinter
  LNP64Info

  ADD_TO_COMPONENT
  LNP64
)

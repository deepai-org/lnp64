; NOTE: Scaffolded LNP64 backend acceptance test. This becomes executable once
; the in-tree target is registered with llc and integer lowering is implemented.
; RUN: llc -mtriple=lnp64-unknown-none -verify-machineinstrs < %s | FileCheck %s
; XFAIL: *

@msg = private unnamed_addr constant [6 x i8] c"hello\00", align 1

declare i64 @__lnp_push(i64, ptr, i64)

define i64 @main() {
entry:
  %n = call i64 @__lnp_push(i64 1, ptr @msg, i64 5)
  ret i64 %n
}

; CHECK-LABEL: main:
; CHECK: li
; CHECK: push
; CHECK: ret

; NOTE: Scaffolded LNP64 backend acceptance test. This becomes executable once
; the in-tree target is registered with llc and integer lowering is implemented.
; RUN: llc -mtriple=lnp64-unknown-none -verify-machineinstrs < %s | FileCheck %s
; XFAIL: *

@msg = private unnamed_addr constant [6 x i8] c"hello\00", align 1

declare i64 @__lnp_push(i64, ptr, i64)
declare i64 @callee(i64)

define i64 @main() {
entry:
  %n = call i64 @__lnp_push(i64 1, ptr @msg, i64 5)
  ret i64 %n
}

define i64 @arith(i64 %a, i64 %b) {
entry:
  %sum = add i64 %a, %b
  %biased = add i64 %sum, 7
  %masked = and i64 %biased, %a
  %shifted = shl i64 %masked, %b
  ret i64 %shifted
}

define i64 @jump(i64 %x) {
entry:
  br label %exit

cold:
  ret i64 0

exit:
  ret i64 %x
}

define i64 @call_direct(i64 %x) {
entry:
  %y = call i64 @callee(i64 %x)
  ret i64 %y
}

; CHECK-LABEL: main:
; CHECK: li
; CHECK: push
; CHECK: ret
; CHECK-LABEL: arith:
; CHECK: li
; CHECK: add
; CHECK: and
; CHECK: lsl
; CHECK: ret
; CHECK-LABEL: jump:
; CHECK: jmp
; CHECK: ret
; CHECK-LABEL: call_direct:
; CHECK: call callee
; CHECK: ret

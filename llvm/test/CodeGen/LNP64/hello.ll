; NOTE: Scaffolded LNP64 backend acceptance test. This becomes executable once
; the in-tree target is registered with llc and integer lowering is implemented.
; RUN: llc -mtriple=lnp64-unknown-none -verify-machineinstrs < %s | FileCheck %s
; XFAIL: *

@msg = private unnamed_addr constant [6 x i8] c"hello\00", align 1

declare i64 @__lnp_call(i64, i64, i64)
declare i64 @__lnp_domain_ctl(i64)
declare i64 @__lnp_object_ctl(i64)
declare i64 @__lnp_pull(i64, ptr, i64)
declare i64 @__lnp_push(i64, ptr, i64)
declare i64 @callee(i64)

define i64 @main() {
entry:
  %n = call i64 @__lnp_push(i64 1, ptr @msg, i64 5)
  ret i64 %n
}

define i64 @read_stream(ptr %p) {
entry:
  %n = call i64 @__lnp_pull(i64 0, ptr %p, i64 32)
  ret i64 %n
}

define i64 @gate(i64 %cap, i64 %a, i64 %b) {
entry:
  %r = call i64 @__lnp_call(i64 %cap, i64 %a, i64 %b)
  ret i64 %r
}

define i64 @control(i64 %record) {
entry:
  %d = call i64 @__lnp_domain_ctl(i64 %record)
  %o = call i64 @__lnp_object_ctl(i64 %record)
  %sum = add i64 %d, %o
  ret i64 %sum
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

define i64 @memory(ptr %p, i64 %v) {
entry:
  %p2 = getelementptr i8, ptr %p, i64 2
  %p4 = getelementptr i8, ptr %p, i64 4
  %b = load i8, ptr %p, align 1
  %h = load i16, ptr %p2, align 2
  %w = load i32, ptr %p4, align 4
  %tb = trunc i64 %v to i8
  %th = trunc i64 %v to i16
  %tw = trunc i64 %v to i32
  store i8 %tb, ptr %p, align 1
  store i16 %th, ptr %p2, align 2
  store i32 %tw, ptr %p4, align 4
  %bz = zext i8 %b to i64
  %hz = zext i16 %h to i64
  %wz = zext i32 %w to i64
  %sum0 = add i64 %bz, %hz
  %sum1 = add i64 %sum0, %wz
  ret i64 %sum1
}

; CHECK-LABEL: main:
; CHECK: li
; CHECK: push
; CHECK: ret
; CHECK-LABEL: read_stream:
; CHECK: pull
; CHECK: ret
; CHECK-LABEL: gate:
; CHECK: gate_call
; CHECK: ret
; CHECK-LABEL: control:
; CHECK: domain_ctl
; CHECK: object_ctl
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
; CHECK-LABEL: memory:
; CHECK: ld.b
; CHECK: ld.h
; CHECK: ld.w
; CHECK: st.b
; CHECK: st.h
; CHECK: st.w
; CHECK: ret

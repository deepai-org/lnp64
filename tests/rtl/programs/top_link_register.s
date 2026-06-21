.text
  ; v2: the link register is gone; the return address lives in r1.
  ; This exercises nested CALL/RET with explicit r1 (link) save/restore.
  CALL setup
  ADD r1, r2, r0          ; result returned in r2 -> exit code
  EXIT r1

setup:
  ; save our return address (link in r1) across a nested call
  ADD r4, r1, r0          ; LR_GET: read link -> r4 (callee-saved by us)
  CALL leaf
  ADD r1, r4, r0          ; LR_SET: restore link <- r4 before RET
  ADD r2, r3, r0          ; propagate leaf result (0) to caller in r2
  RET

leaf:
  LI r3, 0
  RET

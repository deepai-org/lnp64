.text
  LI r20, 1

pid_is_init:
  GET_PCR r1, PID
  CMP r1, r20
  BNE bad

tid_is_initial_thread:
  GET_PCR r2, TID
  CMP r2, r20
  BNE bad

tls_base_alias_is_thread_pointer:
  GET_PCR r3, TLS_BASE
  CMP r3, r0
  BNE bad

thread_pointer_matches_tls_base:
  GET_PCR r4, TP
  CMP r4, r3
  BNE bad

root_uid_gid_are_zero:
  GET_PCR r5, UID
  CMP r5, r0
  BNE bad
  GET_PCR r6, GID
  CMP r6, r0
  BNE bad

done:
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

.text
  LI r20, 1

pid_is_init:
  GET_PCR r1, PID
  BNE r1, r20, bad

tid_is_initial_thread:
  GET_PCR r2, TID
  BNE r2, r20, bad

tls_base_alias_is_thread_pointer:
  GET_PCR r3, TLS_BASE
  BNE r3, r0, bad

thread_pointer_matches_tls_base:
  GET_PCR r4, TP
  BNE r4, r3, bad

root_uid_gid_are_zero:
  GET_PCR r5, UID
  BNE r5, r0, bad
  GET_PCR r6, GID
  BNE r6, r0, bad

credential_selectors_are_valid:
  GET_PCR r9, CRED_PROFILE
  BNE r9, r0, bad
  GET_PCR r10, CRED_HANDLE
  BNE r10, r0, bad

done:
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

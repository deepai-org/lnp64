.data
exec_path: .string "demos/exec_target.s"
argv: .quad exec_path
      .quad 0
envp: .quad 0

.text
  LI r1, exec_path
  LI r2, argv
  LI r3, envp
  EXEC r1, r2, r3
  LI r4, 99
  EXIT r4

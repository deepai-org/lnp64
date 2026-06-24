.text
# EP-I-lite verb-over-pipe cosim: exercises the unified send/recv verbs over a
# byte-fd (pipe) backing. send (0x83) and recv (0x84) decode to the WRITE_FD/
# READ_FD datapath in the RTL with operand-sourcing as the only fork: the
# endpoint handle is in rs1 and a pointer to the frozen msg descriptor
# ([0]=bytes_ptr, [8]=bytes_len, [16]=caps_ptr, [24]=caps_len) is in rs2. The
# byte-fd ABI returns the transfer count in r2, so results land in r2.
#
# F1-step-2 migrates top_pipe_push_pull onto this exact shape (one test, two
# jobs): this is the verb-over-pipe coverage, not a throwaway.
  LI r29, -1
  LI r10, 0

create_pipe_queue:
  LI r1, 1
  ST [r10, 0], r1
  LI r1, 2
  ST [r10, 8], r1
  LI r1, 1
  ST [r10, 16], r1
  LI r1, 3
  ST [r10, 24], r1
  LI r1, 4
  ST [r10, 32], r1
  ST [r10, 40], r0
  OBJECT_CTL r11, r10
  BNE r11, r0, bad

send_byte:
  LI r12, 80              # bytes buffer
  LI r1, 81
  ST.B [r12, 0], r1       # buf[0] = 81
  LI r20, 96              # send msg descriptor
  ST [r20, 0], r12        # bytes_ptr = 80
  LI r1, 1
  ST [r20, 8], r1         # bytes_len = 1
  ST [r20, 16], r0        # caps_ptr = 0
  ST [r20, 24], r0        # caps_len = 0
  LI r5, 4               # ep handle = pipe writer fd 4
  SEND r2, r5, r20       # r2 = bytes written
  LI r13, 1
  BNE r2, r13, bad

recv_byte:
  LI r15, 88              # recv buffer
  LI r21, 128             # recv msg descriptor
  ST [r21, 0], r15        # bytes_ptr = 88
  LI r1, 1
  ST [r21, 8], r1         # bytes_len (capacity) = 1
  ST [r21, 16], r0        # caps_ptr = 0
  ST [r21, 24], r0        # caps_len = 0
  LI r6, 3               # ep handle = pipe reader fd 3
  RECV r2, r6, r21       # r2 = bytes read
  BNE r2, r13, bad
  LD.B r17, [r15, 0]
  LI r18, 81
  BNE r17, r18, bad
  EXIT r0

bad:
  LI r1, 1
  EXIT r1

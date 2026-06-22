class LNP64Reg<bits<16> Enc, string Name> : Register<Name> {
  let HWEncoding = Enc;
  let Namespace = "LNP64";
}

class LNP64GPR<bits<16> Enc, string Name> : LNP64Reg<Enc, Name>;
class LNP64FPR<bits<16> Enc, string Name> : LNP64Reg<Enc, Name>;
class LNP64PCR<bits<16> Enc, string Name> : LNP64Reg<Enc, Name>;

foreach I = 0-31 in {
  def R#I : LNP64GPR<I, "r"#I>;
}

// Floating-point register file (reserved for a future hardware-FP extension;
// v1 is integer-only with soft-float, so no instruction binds FPR yet).
foreach I = 0-31 in {
  def F#I : LNP64FPR<I, "f"#I>;
}

def PID : LNP64PCR<0, "PID">;
def PPID : LNP64PCR<1, "PPID">;
def TID : LNP64PCR<2, "TID">;
def TP : LNP64PCR<3, "TP">;
def UID : LNP64PCR<4, "UID">;
def GID : LNP64PCR<5, "GID">;
def SIGMASK : LNP64PCR<6, "SIGMASK">;
def SIGPENDING : LNP64PCR<7, "SIGPENDING">;
def REALTIME_SEC : LNP64PCR<8, "REALTIME_SEC">;
def REALTIME_NSEC : LNP64PCR<9, "REALTIME_NSEC">;
def CRED_PROFILE : LNP64PCR<10, "CRED_PROFILE">;
def CRED_HANDLE : LNP64PCR<11, "CRED_HANDLE">;

// v2: one uniform GPR file. r1 is the return address (ra); the v1 LR and FLAGS
// SPECIAL registers are gone. Only r0 (zero), r1 (ra) and r31 (sp) stay out of
// allocation; r30 is a normal allocatable caller-saved GPR (frame lowering uses
// ADDI's 32-bit immediate directly, so no scratch register is reserved). The
// callee-saved set s0..s9 = r18..r27 is allocatable (callee-saved-ness is
// expressed via getCalleeSavedRegs/CSR_LNP64, not by excluding from allocation).
def GPR : RegisterClass<"LNP64", [i64], 64,
  (add R0, R1, R2, R3, R4, R5, R6, R7,
       R8, R9, R10, R11, R12, R13, R14, R15,
       R16, R17, R18, R19, R20, R21, R22, R23,
       R24, R25, R26, R27, R28, R29, R30, R31)> {
  // Exclude r0 (zero), r1 (ra) and r31 (sp) from allocation.
  let AltOrders = [(sub GPR, R0, R1, R31)];
  let AltOrderSelect = [{
    return 1;
  }];
}

// Reserved for a future hardware-FP extension; not yet bound to any value type
// or instruction (v1 is integer-only / soft-float). FDR capabilities are NOT a
// program-allocatable register file -- they are a hardware/Capability-Engine
// table named by generation-tagged integer handles carried in GPRs -- so there
// is deliberately no FDR register class; likewise no vector class in v1.
def FPR : RegisterClass<"LNP64", [f64], 64, (sequence "F%u", 0, 31)>;
def PCR : RegisterClass<"LNP64", [i64], 64,
  (add PID, PPID, TID, TP, UID, GID, SIGMASK, SIGPENDING,
       REALTIME_SEC, REALTIME_NSEC, CRED_PROFILE, CRED_HANDLE)>;

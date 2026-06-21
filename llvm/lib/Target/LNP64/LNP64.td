include "llvm/Target/Target.td"

include "LNP64RegisterInfo.td"
include "LNP64CallingConv.td"
include "LNP64InstrInfo.td"

def LNP64InstrInfo : InstrInfo;

def LNP64 : Target {
  let InstructionSet = LNP64InstrInfo;
}

// LNP64 is an in-order, single-issue, non-speculative machine with uniform
// instruction timing (every instruction is ~1 cycle in the core pipeline; only
// memory latency varies, behind the cache, which the compiler cannot predict).
// Model it as exactly that so the scheduler's cost model matches the silicon.
def LNP64Model : SchedMachineModel {
  let MicroOpBufferSize = 0;   // in-order (no reorder buffer)
  let IssueWidth = 1;          // single-issue
  let LoadLatency = 1;         // uniform static model (cache hit/miss not modeled)
  let CompleteModel = 0;       // per-instruction SchedRW not enumerated
}
def : ProcessorModel<"generic-lnp64", LNP64Model, []>;

package lnp64_pkg;
    localparam int LNP64_ID_W = 32;
    localparam int LNP64_OP_W = 16;
    localparam int LNP64_REG_W = 8;
    localparam int LNP64_WORD_W = 64;

    localparam logic [31:0] LNP64_BUILD_ID = 32'h0000_0050;

    localparam logic [63:0] LNP64_FEATURE_CORE_TILE        = 64'h0000_0000_0000_0001;
    localparam logic [63:0] LNP64_FEATURE_DECODE           = 64'h0000_0000_0000_0002;
    localparam logic [63:0] LNP64_FEATURE_ENV_GET          = 64'h0000_0000_0000_0004;
    localparam logic [63:0] LNP64_FEATURE_SCHEDULER_STUB   = 64'h0000_0000_0000_0008;
    localparam logic [63:0] LNP64_FEATURE_EVENT_STUB       = 64'h0000_0000_0000_0010;
    localparam logic [63:0] LNP64_FEATURE_CAP_STUB         = 64'h0000_0000_0000_0020;
    localparam logic [63:0] LNP64_FEATURE_DOMAIN_STUB      = 64'h0000_0000_0000_0040;
    localparam logic [63:0] LNP64_FEATURE_RAS_STUB         = 64'h0000_0000_0000_0080;
    localparam logic [63:0] LNP64_FEATURE_UART_STUB        = 64'h0000_0000_0000_0100;
    localparam logic [63:0] LNP64_FEATURE_VMA_ABSENT       = 64'h0000_0000_0000_0200;
    localparam logic [63:0] LNP64_FEATURE_DMA_ABSENT       = 64'h0000_0000_0000_0400;
    localparam logic [63:0] LNP64_FEATURE_STORAGE_STUB     = 64'h0000_0000_0000_0800;
    localparam logic [63:0] LNP64_FEATURE_ETH_STUB         = 64'h0000_0000_0000_1000;
    localparam logic [63:0] LNP64_FEATURE_PCIE_STUB        = 64'h0000_0000_0000_2000;

    localparam logic [63:0] LNP64_S0_FEATURES =
        LNP64_FEATURE_CORE_TILE |
        LNP64_FEATURE_DECODE |
        LNP64_FEATURE_ENV_GET |
        LNP64_FEATURE_SCHEDULER_STUB |
        LNP64_FEATURE_EVENT_STUB |
        LNP64_FEATURE_CAP_STUB |
        LNP64_FEATURE_DOMAIN_STUB |
        LNP64_FEATURE_RAS_STUB |
        LNP64_FEATURE_UART_STUB |
        LNP64_FEATURE_VMA_ABSENT |
        LNP64_FEATURE_DMA_ABSENT |
        LNP64_FEATURE_STORAGE_STUB |
        LNP64_FEATURE_ETH_STUB |
        LNP64_FEATURE_PCIE_STUB;

    typedef enum logic [15:0] {
        LNP64_OP_NOP          = 16'h0000,
        LNP64_OP_LI32         = 16'h0001,
        LNP64_OP_ADD          = 16'h0002,
        LNP64_OP_JMP          = 16'h0003,
        LNP64_OP_LD           = 16'h0004,
        LNP64_OP_ST           = 16'h0005,
        LNP64_OP_YIELD        = 16'h0006,
        LNP64_OP_ENV_GET      = 16'h0007,
        LNP64_OP_GET_ERRNO    = 16'h0008,
        LNP64_OP_SET_ERRNO    = 16'h0009,
        LNP64_OP_OBJECT_CTL   = 16'h000a,
        LNP64_OP_FAULT_INJECT = 16'h000b,
        LNP64_OP_PUSH         = 16'h000c,
        LNP64_OP_PULL         = 16'h000d,
        LNP64_OP_AWAIT        = 16'h000e,
        LNP64_OP_CAP_DUP      = 16'h000f,
        LNP64_OP_UNSUPPORTED  = 16'h00ff
    } lnp64_opcode_e;

    typedef enum logic [15:0] {
        LNP64_STATUS_OK          = 16'h0000,
        LNP64_STATUS_ERROR       = 16'h0001,
        LNP64_STATUS_EVENT       = 16'h0002,
        LNP64_STATUS_FAULT       = 16'h0003,
        LNP64_STATUS_DEGRADED    = 16'h0004,
        LNP64_STATUS_UNSUPPORTED = 16'h0005
    } lnp64_status_e;

    typedef enum logic [15:0] {
        LNP64_ERR_OK       = 16'd0,
        LNP64_ERR_EPERM    = 16'd1,
        LNP64_ERR_EBADF    = 16'd9,
        LNP64_ERR_EACCES   = 16'd13,
        LNP64_ERR_EFAULT   = 16'd14,
        LNP64_ERR_EAGAIN   = 16'd11,
        LNP64_ERR_EINVAL   = 16'd22,
        LNP64_ERR_ENOTSUP  = 16'd95,
        LNP64_ERR_EOVERFLOW= 16'd75,
        LNP64_ERR_EREVOKED = 16'd122
    } lnp64_errno_e;

    typedef enum logic [15:0] {
        LNP64_ENGINE_NONE       = 16'd0,
        LNP64_ENGINE_OBJECT     = 16'd1,
        LNP64_ENGINE_FAULT      = 16'd2,
        LNP64_ENGINE_WATCHDOG   = 16'd3,
        LNP64_ENGINE_UNSUPPORTED= 16'd255
    } lnp64_engine_e;

    typedef struct packed {
        logic [31:0] op_id;
        logic [15:0] opcode;
        logic [15:0] profile;
        logic [31:0] pid;
        logic [31:0] tid;
        logic [31:0] domain_id;
        logic [31:0] domain_gen;
        logic [31:0] credential_snapshot_id;
        logic [7:0]  result_reg;
        logic [63:0] rights_mask;
        logic [63:0] flags;
        logic [63:0] arg0;
        logic [63:0] arg1;
        logic [63:0] arg2;
        logic [63:0] arg3;
        logic [63:0] arg_block_ptr;
        logic [63:0] arg_block_len;
        logic [15:0] cancel_class;
        logic [15:0] completion_target;
    } lnp64_cmd_t;

    typedef struct packed {
        logic [31:0] op_id;
        logic [31:0] pid;
        logic [31:0] tid;
        logic [31:0] domain_id;
        logic [31:0] domain_gen;
        logic [7:0]  result_reg;
        logic [63:0] result_value;
        logic [15:0] errno_value;
        logic [15:0] status;
        logic [63:0] event_mask;
    } lnp64_rsp_t;

    typedef struct packed {
        logic [31:0] op_id;
        logic [31:0] pid;
        logic [31:0] tid;
        logic [31:0] domain_id;
        logic [31:0] domain_gen;
        logic [15:0] target;
        logic [15:0] status;
        logic [15:0] errno_value;
        logic [63:0] value;
    } lnp64_completion_t;

    typedef struct packed {
        logic [31:0] event_id;
        logic [31:0] op_id;
        logic [31:0] pid;
        logic [31:0] tid;
        logic [31:0] domain_id;
        logic [31:0] domain_gen;
        logic [63:0] event_mask;
        logic [15:0] source;
        logic [15:0] status;
    } lnp64_event_t;

    typedef struct packed {
        logic [31:0] fault_id;
        logic [31:0] op_id;
        logic [31:0] pid;
        logic [31:0] tid;
        logic [31:0] domain_id;
        logic [31:0] domain_gen;
        logic [15:0] fault_code;
        logic [15:0] source;
        logic [63:0] detail;
    } lnp64_fault_t;

    typedef struct packed {
        logic [31:0] object_id;
        logic [31:0] object_gen;
        logic [31:0] fdr_gen;
        logic [31:0] domain_id;
        logic [31:0] domain_gen;
        logic [63:0] rights_mask;
        logic [31:0] lineage_epoch;
        logic        sealed;
        logic        narrowable;
    } lnp64_cap_t;

    typedef struct packed {
        logic [31:0] object_id;
        logic [31:0] object_gen;
        logic [15:0] profile;
        logic [63:0] length;
        logic [63:0] bounds_base;
    } lnp64_object_ref_t;

    typedef struct packed {
        logic [15:0] version;
        logic [15:0] profile;
        logic [31:0] byte_len;
        logic [31:0] selector;
        logic [31:0] service_generation;
        logic [63:0] payload_ptr;
    } lnp64_control_envelope_t;

    typedef struct packed {
        logic [31:0] domain_id;
        logic [31:0] domain_gen;
        logic [31:0] parent_domain_id;
        logic [31:0] parent_domain_gen;
        logic [63:0] budget_limit;
        logic [63:0] budget_used;
        logic [15:0] lifecycle_state;
        logic [15:0] assurance_profile;
        logic [31:0] label_id;
    } lnp64_domain_t;

    typedef struct packed {
        logic [31:0] pid;
        logic [31:0] tid;
        logic [31:0] domain_id;
        logic [31:0] domain_gen;
        logic [15:0] state;
        logic [15:0] latency_class;
        logic [31:0] wait_generation;
        logic [31:0] active_location;
    } lnp64_thread_sched_t;

    typedef struct packed {
        logic [15:0] opcode;
        logic [15:0] profile;
        logic [7:0] rd;
        logic [7:0] rs1;
        logic [7:0] rs2;
        logic [31:0] imm;
        logic supported;
    } lnp64_decode_t;

    typedef struct packed {
        logic [15:0] isa_version;
        logic [15:0] profile;
        logic [15:0] opcode;
        logic [63:0] feature_bits;
        logic supported;
    } lnp64_feature_t;

    typedef struct packed {
        logic [31:0] op_id;
        logic [15:0] errno_value;
        logic [15:0] status;
        logic [15:0] cancel_class;
        logic [31:0] revoke_epoch;
    } lnp64_error_cancel_t;

    typedef struct packed {
        logic [31:0] namespace_id;
        logic [31:0] namespace_generation;
        logic [31:0] selector;
        logic [31:0] service_generation;
        logic [63:0] name_hash;
    } lnp64_namespace_selector_t;

    typedef struct packed {
        logic [31:0] proposal_id;
        logic [31:0] object_id;
        logic [31:0] object_generation;
        logic [31:0] fdr_generation;
        logic [31:0] domain_id;
        logic [31:0] domain_generation;
        logic [63:0] rights_mask;
    } lnp64_returned_capability_t;

    typedef struct packed {
        logic [31:0] snapshot_id;
        logic [31:0] pid;
        logic [31:0] tid;
        logic [31:0] domain_id;
        logic [31:0] domain_generation;
        logic [63:0] policy_mask;
        logic [31:0] label_id;
    } lnp64_policy_decision_t;

    typedef struct packed {
        logic [31:0] op_id;
        logic [31:0] pid;
        logic [31:0] tid;
        logic [31:0] pc;
        logic [15:0] action;
        logic [15:0] latency_class;
        logic [63:0] wait_source;
    } lnp64_retire_submit_t;

    typedef struct packed {
        logic [31:0] wait_id;
        logic [31:0] op_id;
        logic [31:0] pid;
        logic [31:0] tid;
        logic [31:0] domain_id;
        logic [31:0] domain_gen;
        logic [15:0] wait_kind;
        logic [63:0] source_id;
        logic [63:0] timeout_cycles;
    } lnp64_waitable_t;

    typedef struct packed {
        logic [31:0] continuation_id;
        logic [31:0] caller_pid;
        logic [31:0] caller_tid;
        logic [31:0] callee_pid;
        logic [31:0] callee_tid;
        logic [31:0] domain_id;
        logic [31:0] domain_gen;
        logic [31:0] generation;
        logic [15:0] mode;
    } lnp64_gate_continuation_t;

    typedef struct packed {
        logic [31:0] process_id;
        logic [31:0] process_generation;
        logic [31:0] parent_pid;
        logic [31:0] domain_id;
        logic [31:0] domain_generation;
        logic [63:0] exec_plan_ptr;
        logic [63:0] exec_plan_len;
        logic [15:0] lifecycle_state;
    } lnp64_process_lifecycle_t;

    typedef struct packed {
        logic [31:0] vma_id;
        logic [31:0] vma_gen;
        logic [31:0] domain_id;
        logic [31:0] domain_gen;
        logic [63:0] virt_base;
        logic [63:0] length;
        logic [63:0] permissions;
    } lnp64_vma_req_t;

    typedef struct packed {
        logic [31:0] invalidate_id;
        logic [31:0] domain_id;
        logic [31:0] domain_generation;
        logic [63:0] virtual_base;
        logic [63:0] byte_len;
        logic [15:0] scope;
    } lnp64_tlb_cache_invalidate_t;

    typedef struct packed {
        logic [31:0] txn_id;
        logic [31:0] domain_id;
        logic [31:0] domain_generation;
        logic [63:0] address;
        logic [63:0] byte_len;
        logic [15:0] memory_type;
        logic [15:0] ordering;
    } lnp64_coherence_txn_t;

    typedef struct packed {
        logic [31:0] allocation_id;
        logic [31:0] pid;
        logic [31:0] tid;
        logic [31:0] domain_id;
        logic [31:0] domain_generation;
        logic [63:0] size;
        logic [63:0] alignment;
        logic [15:0] heap_profile;
    } lnp64_heap_alloc_t;

    typedef struct packed {
        logic [31:0] futex_id;
        logic [31:0] pid;
        logic [31:0] tid;
        logic [31:0] domain_id;
        logic [31:0] domain_generation;
        logic [63:0] address_token;
        logic [63:0] expected_value;
        logic [63:0] timeout_cycles;
    } lnp64_futex_wait_t;

    typedef struct packed {
        logic [31:0] dma_id;
        logic [31:0] op_id;
        logic [31:0] domain_id;
        logic [31:0] domain_gen;
        logic [63:0] src_cap;
        logic [63:0] dst_cap;
        logic [63:0] byte_len;
        logic [15:0] latency_class;
    } lnp64_dma_req_t;

    typedef struct packed {
        logic [31:0] barrier_id;
        logic [31:0] object_id;
        logic [31:0] object_generation;
        logic [31:0] domain_id;
        logic [31:0] domain_generation;
        logic [15:0] barrier_kind;
    } lnp64_storage_barrier_t;

    typedef struct packed {
        logic [31:0] service_id;
        logic [31:0] service_generation;
        logic [31:0] op_id;
        logic [31:0] pid;
        logic [31:0] tid;
        logic [31:0] domain_id;
        logic [31:0] domain_generation;
        logic [63:0] payload_ptr;
        logic [63:0] payload_len;
    } lnp64_service_txn_t;

    typedef struct packed {
        logic [31:0] action_id;
        logic [31:0] table_id;
        logic [31:0] domain_id;
        logic [31:0] domain_generation;
        logic [15:0] action_kind;
        logic [63:0] output_queue;
        logic [63:0] mark;
    } lnp64_classifier_action_t;

    typedef struct packed {
        logic [31:0] reset_id;
        logic [31:0] op_id;
        logic [31:0] domain_id;
        logic [31:0] domain_generation;
        logic [15:0] reset_kind;
        logic [15:0] degraded_state;
        logic [63:0] reason_code;
    } lnp64_watchdog_reset_t;

    typedef struct packed {
        logic [31:0] trace_id;
        logic [31:0] domain_id;
        logic [31:0] domain_gen;
        logic [15:0] source;
        logic [15:0] severity;
        logic [63:0] counter_value;
        logic [63:0] payload_hash;
    } lnp64_trace_t;

    typedef struct packed {
        logic [31:0] quote_id;
        logic [31:0] build_id;
        logic [63:0] feature_bits;
        logic [63:0] boot_measurement;
        logic [63:0] audit_root;
        logic [63:0] proof_manifest_hash;
    } lnp64_quote_t;

    typedef struct packed {
        logic [31:0] boot_id;
        logic [31:0] build_id;
        logic [63:0] feature_bits;
        logic [63:0] manifest_hash;
        logic [63:0] image_hash;
        logic [63:0] measurement_root;
    } lnp64_boot_metadata_t;

    function automatic lnp64_rsp_t lnp64_error_rsp(
        input lnp64_cmd_t cmd,
        input logic [15:0] errno_value,
        input logic [15:0] status
    );
        lnp64_rsp_t rsp;
        rsp.op_id = cmd.op_id;
        rsp.pid = cmd.pid;
        rsp.tid = cmd.tid;
        rsp.domain_id = cmd.domain_id;
        rsp.domain_gen = cmd.domain_gen;
        rsp.result_reg = cmd.result_reg;
        rsp.result_value = 64'd0;
        rsp.errno_value = errno_value;
        rsp.status = status;
        rsp.event_mask = 64'd0;
        return rsp;
    endfunction
endpackage

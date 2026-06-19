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
    localparam logic [63:0] LNP64_FEATURE_HEAP_STUB        = 64'h0000_0000_0000_0800;
    localparam logic [63:0] LNP64_FEATURE_FUTEX_STUB       = 64'h0000_0000_0000_1000;
    localparam logic [63:0] LNP64_FEATURE_CLASSIFIER_STUB  = 64'h0000_0000_0000_2000;
    localparam logic [63:0] LNP64_FEATURE_STORAGE_STUB     = 64'h0000_0000_0000_4000;
    localparam logic [63:0] LNP64_FEATURE_ETH_STUB         = 64'h0000_0000_0000_8000;
    localparam logic [63:0] LNP64_FEATURE_PCIE_STUB        = 64'h0000_0000_0001_0000;

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
        LNP64_FEATURE_HEAP_STUB |
        LNP64_FEATURE_FUTEX_STUB |
        LNP64_FEATURE_CLASSIFIER_STUB |
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
        LNP64_OP_GATE_CALL    = 16'h0010,
        LNP64_OP_GATE_RETURN  = 16'h0011,
        LNP64_OP_CLONE        = 16'h0012,
        LNP64_OP_EXIT         = 16'h0013,
        LNP64_OP_JOIN         = 16'h0014,
        LNP64_OP_EXEC         = 16'h0015,
        LNP64_OP_MMAP         = 16'h0016,
        LNP64_OP_MUNMAP       = 16'h0017,
        LNP64_OP_MPROTECT     = 16'h0018,
        LNP64_OP_DMA_CTL      = 16'h0019,
        LNP64_OP_OPEN_AT      = 16'h001a,
        LNP64_OP_NS_CTL       = 16'h001b,
        LNP64_OP_SERVICE_REPLY= 16'h001c,
        LNP64_OP_LOCK_CMPXCHG = 16'h001d,
        LNP64_OP_FUTEX_WAIT   = 16'h001e,
        LNP64_OP_FUTEX_WAKE   = 16'h001f,
        LNP64_OP_ALLOC        = 16'h0020,
        LNP64_OP_FREE         = 16'h0021,
        LNP64_OP_ALLOC_SIZE   = 16'h0022,
        LNP64_OP_CLASSIFY     = 16'h0023,
        LNP64_OP_SERVICELET_CTL=16'h0024,
        LNP64_OP_CMP          = 16'h0025,
        LNP64_OP_BRANCH_EQ    = 16'h0026,
        LNP64_OP_BRANCH_NE    = 16'h0027,
        LNP64_OP_BRANCH_LT    = 16'h0028,
        LNP64_OP_BRANCH_GT    = 16'h0029,
        LNP64_OP_BRANCH_LE    = 16'h002a,
        LNP64_OP_BRANCH_GE    = 16'h002b,
        LNP64_OP_MUL          = 16'h002c,
        LNP64_OP_SUB          = 16'h002d,
        LNP64_OP_AND          = 16'h002e,
        LNP64_OP_OR           = 16'h002f,
        LNP64_OP_XOR          = 16'h0030,
        LNP64_OP_LSL          = 16'h0031,
        LNP64_OP_LSR          = 16'h0032,
        LNP64_OP_UDIV         = 16'h0033,
        LNP64_OP_UREM         = 16'h0034,
        LNP64_OP_NOT          = 16'h0035,
        LNP64_OP_LI32_LITERAL = 16'h0036,
        LNP64_OP_MOV          = 16'h0037,
        LNP64_OP_CALL         = 16'h0038,
        LNP64_OP_RET          = 16'h0039,
        LNP64_OP_LD_B         = 16'h003a,
        LNP64_OP_ST_B         = 16'h003b,
        LNP64_OP_DIV          = 16'h003c,
        LNP64_OP_SREM         = 16'h003d,
        LNP64_OP_ASR          = 16'h003e,
        LNP64_OP_ADDI         = 16'h003f,
        LNP64_OP_ANDI         = 16'h0040,
        LNP64_OP_ORI          = 16'h0041,
        LNP64_OP_XORI         = 16'h0042,
        LNP64_OP_LSLI         = 16'h0043,
        LNP64_OP_LSRI         = 16'h0044,
        LNP64_OP_ASRI         = 16'h0045,
        LNP64_OP_SEXT_B       = 16'h0046,
        LNP64_OP_SEXT_H       = 16'h0047,
        LNP64_OP_SEXT_W       = 16'h0048,
        LNP64_OP_ZEXT_B       = 16'h0049,
        LNP64_OP_ZEXT_H       = 16'h004a,
        LNP64_OP_ZEXT_W       = 16'h004b,
        LNP64_OP_UNSUPPORTED  = 16'h00ff
    } lnp64_opcode_e;

    typedef enum logic [7:0] {
        LNP64_M1_COMMIT_CAP_DUP      = 8'd1,
        LNP64_M1_COMMIT_CAP_SEND     = 8'd2,
        LNP64_M1_COMMIT_CAP_RECV     = 8'd3,
        LNP64_M1_COMMIT_CAP_REVOKE   = 8'd4,
        LNP64_M1_COMMIT_REJECT_STALE = 8'd5,
        LNP64_M1_COMMIT_PUSH         = 8'd6,
        LNP64_M1_COMMIT_PULL         = 8'd7,
        LNP64_M1_COMMIT_REJECT_FULL  = 8'd8,
        LNP64_M1_COMMIT_CAP_DUP_DENIED = 8'd9,
        LNP64_M1_COMMIT_OBJECT_CREATE = 8'd10
    } lnp64_m1_commit_op_e;

    typedef enum logic [7:0] {
        LNP64_M7_COMMIT_CMPXCHG_SUCCESS      = 8'd1,
        LNP64_M7_COMMIT_CMPXCHG_FAIL         = 8'd2,
        LNP64_M7_COMMIT_FUTEX_WAIT           = 8'd3,
        LNP64_M7_COMMIT_FUTEX_WAKE           = 8'd4,
        LNP64_M7_COMMIT_TIMER_WAIT           = 8'd5,
        LNP64_M7_COMMIT_TIMER_EXPIRE         = 8'd6,
        LNP64_M7_COMMIT_CONSUME_WAKE         = 8'd7,
        LNP64_M7_COMMIT_REJECT_STALE_ADDRESS = 8'd8
    } lnp64_m7_commit_op_e;

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
        LNP64_ERR_EIO      = 16'd5,
        LNP64_ERR_EBADF    = 16'd9,
        LNP64_ERR_ECHILD   = 16'd10,
        LNP64_ERR_EACCES   = 16'd13,
        LNP64_ERR_EFAULT   = 16'd14,
        LNP64_ERR_EAGAIN   = 16'd11,
        LNP64_ERR_EINVAL   = 16'd22,
        LNP64_ERR_ENOTSUP  = 16'd95,
        LNP64_ERR_EOVERFLOW= 16'd75,
        LNP64_ERR_EREVOKED = 16'd122,
        LNP64_ERR_ECANCELED= 16'd125
    } lnp64_errno_e;

    typedef enum logic [15:0] {
        LNP64_ENGINE_NONE       = 16'd0,
        LNP64_ENGINE_OBJECT     = 16'd1,
        LNP64_ENGINE_FAULT      = 16'd2,
        LNP64_ENGINE_WATCHDOG   = 16'd3,
        LNP64_ENGINE_UNSUPPORTED= 16'd255
    } lnp64_engine_e;

    typedef enum logic [15:0] {
        LNP64_LIFECYCLE_PURE_LOCAL        = 16'd0,
        LNP64_LIFECYCLE_PIPELINE_QUEUE    = 16'd1,
        LNP64_LIFECYCLE_OWNER_ENGINE      = 16'd2,
        LNP64_LIFECYCLE_LONG_OWNER_ENGINE = 16'd3,
        LNP64_LIFECYCLE_EXTERNAL_IP       = 16'd4
    } lnp64_lifecycle_profile_e;

    typedef enum logic [15:0] {
        LNP64_LSTATE_RESET      = 16'd0,
        LNP64_LSTATE_READY      = 16'd1,
        LNP64_LSTATE_EMPTY      = 16'd2,
        LNP64_LSTATE_FULL       = 16'd3,
        LNP64_LSTATE_PREPARE    = 16'd4,
        LNP64_LSTATE_COMMIT     = 16'd5,
        LNP64_LSTATE_COMPLETE   = 16'd6,
        LNP64_LSTATE_ABORT      = 16'd7,
        LNP64_LSTATE_POISONED   = 16'd8,
        LNP64_LSTATE_DEGRADED   = 16'd9,
        LNP64_LSTATE_LINK_DOWN  = 16'd10,
        LNP64_LSTATE_TRAINING   = 16'd11,
        LNP64_LSTATE_ERROR      = 16'd12
    } lnp64_lifecycle_state_e;

    typedef struct packed {
        logic [31:0] op_id;
        logic [31:0] tile_id;
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
        logic [31:0] tile_id;
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
        logic [31:0] tile_id;
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
        logic [31:0] tile_id;
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
        logic [31:0] tile_id;
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
        logic [15:0] engine_id;
        logic [15:0] profile;
        logic [15:0] state;
        logic [31:0] owner_shard_id;
        logic [31:0] generation;
        logic [15:0] fault_policy;
    } lnp64_engine_lifecycle_t;

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
        logic [7:0]  op;
        logic [31:0] object_id;
        logic [31:0] object_gen;
        logic [31:0] fdr_gen;
        logic [31:0] domain_id;
        logic [31:0] domain_gen;
        logic [63:0] rights_mask;
        logic [31:0] lineage_epoch;
        logic        sealed;
        logic [15:0] status;
    } lnp64_m1_cap_commit_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] object_gen;
        logic        created_object_created;
        logic [31:0] created_object_gen;
        logic [31:0] root_object_id;
        logic [31:0] root_generation;
        logic [31:0] root_domain_id;
        logic [31:0] root_lineage_epoch;
        logic        root_sealed;
        logic [63:0] root_rights;
        logic [31:0] consumer_object_id;
        logic [31:0] consumer_generation;
        logic [31:0] consumer_domain_id;
        logic [31:0] consumer_lineage_epoch;
        logic        consumer_sealed;
        logic [63:0] consumer_rights;
        logic        sent_valid;
        logic [31:0] sent_object_id;
        logic [31:0] sent_generation;
        logic [31:0] sent_domain_id;
        logic [31:0] sent_lineage_epoch;
        logic        sent_sealed;
        logic [63:0] sent_rights;
        logic        minted_valid;
        logic [31:0] minted_object_id;
        logic [31:0] minted_generation;
        logic [31:0] minted_domain_id;
        logic [31:0] minted_lineage_epoch;
        logic        minted_sealed;
        logic [63:0] minted_rights;
        logic        wake_pending;
        logic        transfer_valid;
        logic        stale_rejected;
        logic        revoked_rejected;
        logic        failed_no_authority;
        logic        full_was_explicit;
        logic        has_revoked_generation;
        logic [31:0] revoked_generation;
    } lnp64_m1_state_projection_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] tid;
        logic [15:0] before_location;
        logic [15:0] after_location;
        logic [31:0] wait_generation;
        logic [31:0] address_generation;
    } lnp64_m7_sched_commit_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] tid;
        logic [15:0] location;
        logic [31:0] wait_generation;
        logic [31:0] atomic_word;
        logic [31:0] atomic_count;
        logic        cmpxchg_failure_explicit;
        logic [31:0] address_generation;
        logic [31:0] stale_address_generation;
        logic [31:0] domain_budget;
        logic [31:0] wait_cost;
        logic        wake_pending;
        logic        futex_wake_delivered;
        logic        timer_wake_delivered;
        logic        stale_address_rejected;
    } lnp64_m7_state_projection_t;

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
        logic [31:0] tile_id;
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
        logic [31:0] snapshot_id;
        logic [31:0] pid;
        logic [31:0] tid;
        logic [31:0] domain_id;
        logic [31:0] domain_generation;
        logic [31:0] credential_generation;
        logic [63:0] delegated_fdr_root;
        logic [63:0] policy_mask;
        logic [31:0] label_id;
    } lnp64_credential_snapshot_t;

    typedef struct packed {
        logic [31:0] op_id;
        logic [31:0] pid;
        logic [31:0] tid;
        logic [31:0] tile_id;
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
        logic [31:0] tile_id;
        logic [31:0] domain_id;
        logic [31:0] domain_generation;
        logic [63:0] virtual_base;
        logic [63:0] byte_len;
        logic [15:0] scope;
    } lnp64_tlb_cache_invalidate_t;

    typedef struct packed {
        logic [31:0] txn_id;
        logic [31:0] tile_id;
        logic [31:0] domain_id;
        logic [31:0] domain_generation;
        logic [63:0] address;
        logic [63:0] byte_len;
        logic [15:0] memory_type;
        logic [15:0] ordering;
    } lnp64_coherence_txn_t;

    typedef struct packed {
        logic [31:0] line_id;
        logic [31:0] line_generation;
        logic [31:0] domain_id;
        logic [31:0] domain_generation;
        logic [63:0] byte_address;
        logic [63:0] byte_len;
        logic [63:0] data_value;
        logic [15:0] latency_class;
    } lnp64_ddr_line_t;

    typedef struct packed {
        logic [31:0] entry_id;
        logic [31:0] line_id;
        logic [31:0] line_generation;
        logic [31:0] domain_id;
        logic [31:0] domain_generation;
        logic [31:0] metadata_epoch;
        logic [63:0] rights_mask;
        logic [15:0] integrity_state;
    } lnp64_metadata_entry_t;

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
        logic [31:0] requester_id;
        logic [31:0] bar_id;
        logic [31:0] bar_generation;
        logic [31:0] domain_id;
        logic [31:0] domain_generation;
        logic [63:0] bar_base_token;
        logic [63:0] bar_length;
        logic [63:0] rights_mask;
        logic [15:0] msi_vector;
        logic [15:0] device_state;
    } lnp64_pcie_device_t;

    typedef struct packed {
        logic [31:0] context_id;
        logic [31:0] requester_id;
        logic [31:0] domain_id;
        logic [31:0] domain_generation;
        logic [31:0] bar_id;
        logic [31:0] bar_generation;
        logic [63:0] dma_window_token;
        logic [63:0] byte_len;
        logic [15:0] permission;
    } lnp64_iommu_mapping_t;

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
        logic [31:0] tile_id;
        logic [31:0] op_id;
        logic [31:0] domain_id;
        logic [31:0] domain_generation;
        logic [15:0] reset_kind;
        logic [15:0] degraded_state;
        logic [63:0] reason_code;
    } lnp64_watchdog_reset_t;

    typedef struct packed {
        logic [31:0] trace_id;
        logic [31:0] tile_id;
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
        rsp.tile_id = cmd.tile_id;
        rsp.pid = cmd.pid;
        rsp.tid = cmd.tid;
        rsp.domain_id = cmd.domain_id;
        rsp.domain_gen = cmd.domain_gen;
        rsp.result_reg = cmd.result_reg;
        rsp.result_value = 64'd0;
        rsp.errno_value = errno_value;
        rsp.status = status;
        rsp.event_mask = 64'd0;
        lnp64_error_rsp = rsp;
    endfunction
endpackage

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

    localparam int LNP64_FDR_SLOT_COUNT = 10;
    localparam logic [63:0] LNP64_FDR_TOKEN_MARKER = 64'h4000_0000_0000_0000;
    localparam logic [63:0] LNP64_FDR_TOKEN_INDEX_MASK = 64'h0000_0000_0000_00ff;
    localparam logic [63:0] LNP64_CAP_DUP_FLAG_SEAL = 64'd1;
    localparam logic [63:0] LNP64_CAP_RIGHT_ALL = 64'h0000_0000_0000_01ff;
    localparam logic [63:0] LNP64_CAP_RIGHT_CALL = 64'h0000_0000_0000_0020;
    localparam logic [63:0] LNP64_CAP_RIGHT_DUP = 64'h0000_0000_0000_0040;
    localparam logic [63:0] LNP64_CAP_RIGHT_REVOKE = 64'h0000_0000_0000_0080;
    localparam logic [63:0] LNP64_CAP_RIGHT_TRANSFER = 64'h0000_0000_0000_0100;
    localparam logic [15:0] LNP64_ERR_ESTALE = 16'd116;
    localparam logic [63:0] LNP64_OBJECT_OP_CREATE = 64'd1;
    localparam logic [63:0] LNP64_OBJECT_KIND_COUNTER = 64'd1;
    localparam logic [63:0] LNP64_OBJECT_KIND_QUEUE = 64'd2;
    localparam logic [63:0] LNP64_OBJECT_KIND_MEMORY_OBJECT = 64'd3;
    localparam logic [63:0] LNP64_OBJECT_KIND_DMA_BUFFER = 64'd4;
    localparam logic [63:0] LNP64_OBJECT_KIND_TIMER = 64'd6;
    localparam logic [63:0] LNP64_OBJECT_PROFILE_PIPE = 64'd1;
    localparam logic [63:0] LNP64_OBJECT_PROFILE_CALL_GATE = 64'd4;

    typedef enum logic [2:0] {
        LNP64_FDR_KIND_CLOSED = 3'd0,
        LNP64_FDR_KIND_GENERIC = 3'd1,
        LNP64_FDR_KIND_PIPE_READER = 3'd2,
        LNP64_FDR_KIND_PIPE_WRITER = 3'd3,
        LNP64_FDR_KIND_CALL_GATE = 3'd4
    } lnp64_fdr_kind_e;

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
        LNP64_OP_READ_FD      = 16'h0081,
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
        LNP64_OP_CLZ          = 16'h004c,
        LNP64_OP_CTZ          = 16'h004d,
        LNP64_OP_POPCNT       = 16'h004e,
        LNP64_OP_ROL          = 16'h004f,
        LNP64_OP_ROR          = 16'h0050,
        LNP64_OP_BSWAP16      = 16'h0051,
        LNP64_OP_BSWAP32      = 16'h0052,
        LNP64_OP_BSWAP64      = 16'h0053,
        LNP64_OP_MULH         = 16'h005f,
        LNP64_OP_MULHU        = 16'h0060,
        LNP64_OP_MULHSU       = 16'h0061,
        LNP64_OP_AUIPC_LITERAL= 16'h0062,
        LNP64_OP_FENCE        = 16'h0063,
        LNP64_OP_LD_W         = 16'h0064,
        LNP64_OP_ST_W         = 16'h0065,
        LNP64_OP_LD_H         = 16'h0066,
        LNP64_OP_ST_H         = 16'h0067,
        LNP64_OP_WRITE_FD     = 16'h0072,
        LNP64_OP_ALLOC_EX     = 16'h0073,
        LNP64_OP_ISYNC        = 16'h0074,
        LNP64_OP_CAP_REVOKE   = 16'h0075,
        LNP64_OP_AMO_SWAP     = 16'h0076,
        LNP64_OP_AMO_ADD      = 16'h0077,
        LNP64_OP_AMO_AND      = 16'h0078,
        LNP64_OP_AMO_OR       = 16'h0079,
        LNP64_OP_AMO_XOR      = 16'h007a,
        LNP64_OP_CALL_REG     = 16'h007b,
        LNP64_OP_LR_GET       = 16'h007c,
        LNP64_OP_LR_SET       = 16'h007d,
        LNP64_OP_LA_LITERAL   = 16'h007e,
        LNP64_OP_CAP_SEND     = 16'h007f,
        LNP64_OP_CAP_RECV     = 16'h0080,
        LNP64_OP_SLEEP        = 16'h0082,
        LNP64_OP_DOMAIN_CTL   = 16'h0083,
        LNP64_OP_OPEN_FD      = 16'h0084,
        LNP64_OP_FD_CLOSE     = 16'h0085,
        LNP64_OP_WAITABLE_PROBE = 16'h0086,
        LNP64_OP_AWAIT_EX     = 16'h0087,
        LNP64_OP_GET_PCR      = 16'h0088,
        LNP64_OP_SET_PCR      = 16'h0089,
        LNP64_OP_SIGACTION    = 16'h008a,
        LNP64_OP_KILL         = 16'h008b,
        LNP64_OP_SIGRET       = 16'h008c,
        LNP64_OP_INB          = 16'h008d,
        LNP64_OP_OUTB         = 16'h008e,
        LNP64_OP_LOAD_UCODE   = 16'h008f,
        LNP64_OP_FORK         = 16'h0090,
        // --- ISA v2 additions (decode emits these; legacy ids above kept so
        // the as-yet-unmigrated execute paths in lnp64_core_tile.sv still
        // compile). ---
        LNP64_OP_SLT          = 16'h0091,
        LNP64_OP_SLTU         = 16'h0092,
        LNP64_OP_SLTI         = 16'h0093,
        LNP64_OP_SLTIU        = 16'h0094,
        LNP64_OP_LIU          = 16'h0095,
        LNP64_OP_JAL          = 16'h0096,
        LNP64_OP_JALR         = 16'h0097,
        LNP64_OP_LR_D         = 16'h0098,
        LNP64_OP_SC_D         = 16'h0099,
        LNP64_OP_BRANCH_LTU   = 16'h009a,
        LNP64_OP_BRANCH_GEU   = 16'h009b,
        LNP64_OP_AUIPC        = 16'h009c,
        LNP64_OP_LW           = 16'h009d,
        // Fused compare-and-select (rd = (ra <cc> rb) ? rt : rf), one per
        // condition, mirroring the branch family.
        LNP64_OP_SEL_EQ       = 16'h009e,
        LNP64_OP_SEL_NE       = 16'h009f,
        LNP64_OP_SEL_LT       = 16'h00a0,
        LNP64_OP_SEL_GE       = 16'h00a1,
        LNP64_OP_SEL_LTU      = 16'h00a2,
        LNP64_OP_SEL_GEU      = 16'h00a3,
        // EP-I-full: the unified `wait` verb (waitset poll). Distinct microcode
        // from waitable_probe — it reads a {entries_ptr,count} waitset, writes
        // per-entry revents back to memory, and returns the ready count.
        LNP64_OP_WAIT         = 16'h00a4,
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

    typedef enum logic [7:0] {
        LNP64_M4_COMMIT_MMAP           = 8'd1,
        LNP64_M4_COMMIT_LOAD           = 8'd2,
        LNP64_M4_COMMIT_STORE_DENIED   = 8'd3,
        LNP64_M4_COMMIT_EXEC_FAULT     = 8'd4,
        LNP64_M4_COMMIT_GUARD_FAULT    = 8'd5,
        LNP64_M4_COMMIT_STALE_REJECT   = 8'd6,
        LNP64_M4_COMMIT_TLB_INVALIDATE = 8'd7
    } lnp64_m4_vma_op_e;

    typedef enum logic [7:0] {
        LNP64_M5_COMMIT_PIN              = 8'd1,
        LNP64_M5_COMMIT_COPY             = 8'd2,
        LNP64_M5_COMMIT_FILL             = 8'd3,
        LNP64_M5_COMMIT_UNPIN            = 8'd4,
        LNP64_M5_COMMIT_PERMISSION_FAULT = 8'd5,
        LNP64_M5_COMMIT_REVOKED_SUBMIT   = 8'd6,
        LNP64_M5_COMMIT_DOMAIN_ISOLATION = 8'd7,
        LNP64_M5_COMMIT_COHERENCE_FLUSH  = 8'd8
    } lnp64_m5_dma_op_e;

    typedef enum logic [7:0] {
        LNP64_M2_COMMIT_SYNC_CALL      = 8'd1,
        LNP64_M2_COMMIT_SYNC_RETURN    = 8'd2,
        LNP64_M2_COMMIT_ASYNC_CALL     = 8'd3,
        LNP64_M2_COMMIT_HANDOFF_CALL   = 8'd4,
        LNP64_M2_COMMIT_STALE_RETURN   = 8'd5,
        LNP64_M2_COMMIT_FAULT_DELIVERY = 8'd6,
        LNP64_M2_COMMIT_SIGNAL_COMPAT  = 8'd7
    } lnp64_m2_gate_op_e;

    typedef enum logic [7:0] {
        LNP64_M11_COMMIT_METADATA_ALLOC = 8'd1,
        LNP64_M11_COMMIT_DDR_WRITE      = 8'd2,
        LNP64_M11_COMMIT_DDR_READ       = 8'd3,
        LNP64_M11_COMMIT_STALE_SUBMIT   = 8'd4,
        LNP64_M11_COMMIT_CROSS_DOMAIN   = 8'd5,
        LNP64_M11_COMMIT_ECC_SCRUB      = 8'd6,
        LNP64_M11_COMMIT_BARRIER        = 8'd7
    } lnp64_m11_ddr_op_e;

    typedef enum logic [7:0] {
        LNP64_M12_COMMIT_BOOT_IMAGE     = 8'd1,
        LNP64_M12_COMMIT_BLOCK_WRITE    = 8'd2,
        LNP64_M12_COMMIT_BARRIER        = 8'd3,
        LNP64_M12_COMMIT_STALE_OBJECT   = 8'd4,
        LNP64_M12_COMMIT_CROSS_DOMAIN   = 8'd5,
        LNP64_M12_COMMIT_MEDIA_FAULT    = 8'd6,
        LNP64_M12_COMMIT_RAW_AUTHORITY  = 8'd7
    } lnp64_m12_storage_op_e;

    typedef enum logic [7:0] {
        LNP64_M13_COMMIT_ENUMERATE        = 8'd1,
        LNP64_M13_COMMIT_IOMMU_DMA        = 8'd2,
        LNP64_M13_COMMIT_MSI              = 8'd3,
        LNP64_M13_COMMIT_BUS_MASTER       = 8'd4,
        LNP64_M13_COMMIT_STALE_BAR        = 8'd5,
        LNP64_M13_COMMIT_MALFORMED_CONFIG = 8'd6,
        LNP64_M13_COMMIT_RAW_AUTHORITY    = 8'd7
    } lnp64_m13_pcie_op_e;

    typedef enum logic [7:0] {
        LNP64_M15_COMMIT_COUNTER        = 8'd1,
        LNP64_M15_COMMIT_QUEUE_PUSH     = 8'd2,
        LNP64_M15_COMMIT_QUEUE_OVERFLOW = 8'd3,
        LNP64_M15_COMMIT_EVENT_EMIT     = 8'd4,
        LNP64_M15_COMMIT_STALE_EVENT    = 8'd5,
        LNP64_M15_COMMIT_GATE_PROFILE   = 8'd6
    } lnp64_m15_object_op_e;

    // M16 unified-endpoint typed-trace ops (send/recv/wait/create over a
    // backing-typed endpoint; the "ring" is a Memory-backed endpoint).
    typedef enum logic [7:0] {
        LNP64_M16_COMMIT_CREATE      = 8'd1,
        LNP64_M16_COMMIT_SEND        = 8'd2,
        LNP64_M16_COMMIT_RECV        = 8'd3,
        LNP64_M16_COMMIT_WAIT        = 8'd4,
        LNP64_M16_COMMIT_SEND_FULL   = 8'd5,
        LNP64_M16_COMMIT_RECV_EMPTY  = 8'd6,
        LNP64_M16_COMMIT_OVERSIZE    = 8'd7,
        LNP64_M16_COMMIT_NOTIFY      = 8'd8,
        LNP64_M16_COMMIT_CAP_SEND    = 8'd9,
        LNP64_M16_COMMIT_CAP_REJECT  = 8'd10
    } lnp64_m16_endpoint_op_e;

    typedef enum logic [7:0] {
        LNP64_M16_BACKING_THREAD   = 8'd1,
        LNP64_M16_BACKING_MEMORY   = 8'd2,
        LNP64_M16_BACKING_REGISTER = 8'd3
    } lnp64_m16_backing_e;

    typedef enum logic [7:0] {
        LNP64_M10_COMMIT_BOOT_MEASURE  = 8'd1,
        LNP64_M10_COMMIT_ECC_CORRECT   = 8'd2,
        LNP64_M10_COMMIT_PARITY_POISON = 8'd3,
        LNP64_M10_COMMIT_WATCHDOG      = 8'd4,
        LNP64_M10_COMMIT_TELEMETRY_READ = 8'd5,
        LNP64_M10_COMMIT_TRACE_RING    = 8'd6,
        LNP64_M10_COMMIT_QUOTE         = 8'd7,
        LNP64_M10_COMMIT_AUDIT_MLS     = 8'd8
    } lnp64_m10_ras_op_e;

    typedef enum logic [7:0] {
        LNP64_M9_COMMIT_VERIFY_ACCEPT    = 8'd1,
        LNP64_M9_COMMIT_VERIFY_REJECT    = 8'd2,
        LNP64_M9_COMMIT_PACKET_STEER     = 8'd3,
        LNP64_M9_COMMIT_IPC_STEER        = 8'd4,
        LNP64_M9_COMMIT_ACTION_EMIT      = 8'd5,
        LNP64_M9_COMMIT_BUDGET_EXHAUST   = 8'd6,
        LNP64_M9_COMMIT_STALE_ATTACHMENT = 8'd7
    } lnp64_m9_classifier_op_e;

    typedef enum logic [7:0] {
        LNP64_M8_COMMIT_ALLOC             = 8'd1,
        LNP64_M8_COMMIT_ALLOC_SIZE        = 8'd2,
        LNP64_M8_COMMIT_FREE              = 8'd3,
        LNP64_M8_COMMIT_REUSE             = 8'd4,
        LNP64_M8_COMMIT_DOUBLE_FREE       = 8'd5,
        LNP64_M8_COMMIT_STALE_FREE        = 8'd6,
        LNP64_M8_COMMIT_CROSS_THREAD_FREE = 8'd7,
        LNP64_M8_COMMIT_GUARD_FAULT       = 8'd8
    } lnp64_m8_heap_op_e;

    typedef enum logic [7:0] {
        LNP64_M6_COMMIT_ENVELOPE         = 8'd1,
        LNP64_M6_COMMIT_NS_DISPATCH      = 8'd2,
        LNP64_M6_COMMIT_SERVICE_REQUEST  = 8'd3,
        LNP64_M6_COMMIT_CAP_RETURN       = 8'd4,
        LNP64_M6_COMMIT_SERVICE_CANCEL   = 8'd5,
        LNP64_M6_COMMIT_STALE_SERVICE    = 8'd6,
        LNP64_M6_COMMIT_CRASH_COMPLETION = 8'd7
    } lnp64_m6_service_op_e;

    typedef enum logic [7:0] {
        LNP64_M3_COMMIT_CLONE         = 8'd1,
        LNP64_M3_COMMIT_CHILD_EXIT    = 8'd2,
        LNP64_M3_COMMIT_PARENT_JOIN   = 8'd3,
        LNP64_M3_COMMIT_EXEC_BARRIER  = 8'd4,
        LNP64_M3_COMMIT_STALE_JOIN    = 8'd5,
        LNP64_M3_COMMIT_EXEC_CANCEL   = 8'd6
    } lnp64_m3_process_op_e;

    typedef enum logic [7:0] {
        LNP64_M14_COMMIT_DELEGATE      = 8'd1,
        LNP64_M14_COMMIT_CREATE_CHILD  = 8'd2,
        LNP64_M14_COMMIT_EXCESS_BUDGET = 8'd3,
        LNP64_M14_COMMIT_FREEZE        = 8'd4,
        LNP64_M14_COMMIT_RESUME        = 8'd5,
        LNP64_M14_COMMIT_USAGE         = 8'd6,
        LNP64_M14_COMMIT_DESTROY       = 8'd7,
        LNP64_M14_COMMIT_POLICY        = 8'd8
    } lnp64_m14_domain_op_e;

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
        LNP64_ERR_ENOMEM   = 16'd12,
        LNP64_ERR_EACCES   = 16'd13,
        LNP64_ERR_EFAULT   = 16'd14,
        LNP64_ERR_EAGAIN   = 16'd11,
        LNP64_ERR_EINVAL   = 16'd22,
        LNP64_ERR_ENOTSUP  = 16'd95,
        LNP64_ERR_EOVERFLOW= 16'd75,
        LNP64_ERR_EMSGSIZE = 16'd90,
        LNP64_ERR_EREVOKED = 16'd122,
        LNP64_ERR_ECANCELED= 16'd125
    } lnp64_errno_e;

    typedef enum logic [15:0] {
        LNP64_ENGINE_NONE       = 16'd0,
        LNP64_ENGINE_OBJECT     = 16'd1,
        LNP64_ENGINE_FAULT      = 16'd2,
        LNP64_ENGINE_WATCHDOG   = 16'd3,
        LNP64_ENGINE_CORE       = 16'd4,
        LNP64_ENGINE_DOMAIN     = 16'd5,
        LNP64_ENGINE_HEAP       = 16'd6,
        LNP64_ENGINE_VMA        = 16'd7,
        LNP64_ENGINE_DMA        = 16'd8,
        LNP64_ENGINE_ROUTER     = 16'd9,
        LNP64_ENGINE_CAP        = 16'd10,
        LNP64_ENGINE_UNSUPPORTED= 16'd255
    } lnp64_engine_e;

    typedef enum logic [15:0] {
        LNP64_RESPONSE_NONE        = 16'd0,
        LNP64_RESPONSE_CORE_TILE   = 16'd1,
        LNP64_RESPONSE_COMPLETION  = 16'd2,
        LNP64_RESPONSE_FAULT_EVENT = 16'd3
    } lnp64_response_route_e;

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
        logic [31:0] provenance_id;
        logic [15:0] source_engine;
        logic [15:0] destination_engine;
        logic [31:0] object_home_bank;
        logic [31:0] reset_epoch;
        logic [31:0] pid;
        logic [31:0] tid;
        logic [31:0] domain_id;
        logic [31:0] domain_gen;
        logic [15:0] latency_class;
        logic [15:0] budget_class;
        logic [31:0] wait_generation;
        logic [15:0] weight_index;
        logic [63:0] virtual_deadline;
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
        logic [15:0] response_route;
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
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] vma_id;
        logic [31:0] vma_generation;
        logic [7:0]  permissions;
        logic [63:0] fault_addr;
    } lnp64_m4_vma_commit_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] vma_id;
        logic [31:0] vma_generation;
        logic [7:0]  permissions;
        logic        guard_page_valid;
        logic        tlb_valid;
        logic        mapping_created;
        logic        load_permitted;
        logic        store_rejected;
        logic        nx_faulted;
        logic        guard_faulted;
        logic        stale_vma_rejected;
        logic        tlb_invalidation_observed;
        logic        wx_enforced;
    } lnp64_m4_state_projection_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] src_buffer_id;
        logic [31:0] dst_buffer_id;
        logic [31:0] dst_generation;
        logic [31:0] requester_domain;
        logic [31:0] dst_domain;
        logic [7:0]  dst_rights;
    } lnp64_m5_dma_commit_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] dst_buffer_id;
        logic [31:0] dst_generation;
        logic [31:0] requester_domain;
        logic [31:0] dst_domain;
        logic [7:0]  dst_rights;
        logic        dst_pinned;
        logic [31:0] completions;
        logic        dst_visible;
        logic        pin_completed;
        logic        unpin_completed;
        logic        copy_completed;
        logic        fill_completed;
        logic        permission_faulted;
        logic        revoke_rejected;
        logic        domain_isolation_enforced;
        logic        coherence_observed;
        logic        completions_exact;
    } lnp64_m5_state_projection_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] continuation_id;
        logic [31:0] continuation_generation;
        logic [31:0] caller_tid;
        logic [31:0] callee_tid;
        logic [15:0] mode;
    } lnp64_m2_gate_commit_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [1:0]  caller_loc;
        logic [1:0]  callee_loc;
        logic        continuation_valid;
        logic [31:0] continuation_id;
        logic [31:0] continuation_generation;
        logic [31:0] delivered_faults;
        logic        continuation_unique;
        logic        sync_roundtrip_ok;
        logic        async_delivery_ok;
        logic        handoff_delivery_ok;
        logic        stale_continuation_rejected;
        logic        fault_delivery_gate_ok;
        logic        signal_compatibility_ok;
    } lnp64_m2_state_projection_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] line_id;
        logic [31:0] line_generation;
        logic [31:0] domain_id;
        logic [31:0] metadata_epoch;
        logic [31:0] byte_len;
        logic [31:0] data_value;
    } lnp64_m11_ddr_commit_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] completions;
        logic [31:0] faults;
        logic        metadata_allocated;
        logic        metadata_domain_bound;
        logic        ddr_write_completed;
        logic        ddr_read_completed;
        logic        read_matches_write;
        logic        stale_generation_rejected;
        logic        cross_domain_rejected;
        logic        ecc_scrubbed;
        logic        barrier_quiescent;
        logic        counts_exact;
    } lnp64_m11_state_projection_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] object_id;
        logic [31:0] object_generation;
        logic [31:0] domain_id;
        logic [31:0] barrier_id;
        logic [31:0] block_index;
        logic [31:0] data_value;
    } lnp64_m12_storage_commit_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] completions;
        logic [31:0] faults;
        logic        boot_image_visible;
        logic        block_object_authorized;
        logic        block_write_completed;
        logic        storage_barrier_issued;
        logic        storage_barrier_quiescent;
        logic        stale_object_rejected;
        logic        cross_domain_rejected;
        logic        media_fault_terminal;
        logic        no_raw_device_authority;
        logic        counts_exact;
    } lnp64_m12_state_projection_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] requester_id;
        logic [31:0] bar_id;
        logic [31:0] bar_generation;
        logic [31:0] domain_id;
        logic [31:0] iommu_context;
        logic [31:0] dma_bytes;
    } lnp64_m13_pcie_commit_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] completions;
        logic [31:0] faults;
        logic        device_enumerated;
        logic        bar_capability_created;
        logic        iommu_bound_to_domain;
        logic        scoped_dma_completed;
        logic        msi_event_delivered;
        logic        unbound_bus_master_rejected;
        logic        stale_bar_rejected;
        logic        malformed_config_rejected;
        logic        no_raw_pcie_authority;
        logic        counts_exact;
    } lnp64_m13_state_projection_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] object_id;
        logic [31:0] generation;
        logic [31:0] threshold;
        logic [31:0] payload;
        logic [31:0] event_generation;
        logic [31:0] continuation;
    } lnp64_m15_object_commit_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] failures;
        logic [31:0] events;
        logic        counter_threshold_event;
        logic        queue_rights_valid;
        logic        queue_overflow_explicit;
        logic        event_source_generation_safe;
        logic        gate_continuation_unique;
        logic        counts_exact;
    } lnp64_m15_state_projection_t;

    // M16 endpoint per-op commit: a queue engine (EP-F). backing selects
    // Thread/Memory/Register; caps_resolved/installed track SCM_RIGHTS-style
    // cap transfer; sender/receiver domains scope cap-safety.
    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] endpoint_id;
        logic [31:0] endpoint_gen;
        logic [7:0]  backing;
        logic [31:0] bytes_len;
        logic [31:0] caps_len;
        logic [31:0] depth;
        logic [31:0] capacity;
        logic [31:0] caps_resolved;
        logic [31:0] caps_installed;
        logic [31:0] sender_domain_id;
        logic [31:0] sender_domain_gen;
        logic [31:0] receiver_domain_id;
        logic [31:0] receiver_domain_gen;
    } lnp64_m16_endpoint_commit_t;

    // M16 invariant projection (straight from EP-F): bounded (a), fail-closed
    // (b), cap-safety (c), framing (d).
    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] depth;
        logic [31:0] capacity;
        logic [31:0] failures;
        logic [31:0] events;
        logic        bounded_depth_le_capacity;
        logic        drain_bounded_by_capacity;
        logic        full_fails_closed;
        logic        empty_fails_closed;
        logic        oversize_fails_closed;
        logic        no_block_except_wait;
        logic        caps_resolve_sender_only;
        logic        caps_reject_out_of_range;
        logic        install_no_amplify;
        logic        framing_one_send_one_recv;
        logic        notify_raises_register_edge;
        logic        counts_exact;
    } lnp64_m16_state_projection_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] root_domain;
        logic [31:0] fault_count;
        logic [31:0] telemetry_reads;
        logic [31:0] audit_records;
        logic [31:0] quote_id;
        logic [31:0] reset_id;
    } lnp64_m10_ras_commit_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] fault_count;
        logic [31:0] telemetry_reads;
        logic [31:0] audit_records;
        logic [31:0] trace_writes;
        logic [31:0] trace_capacity;
        logic        boot_measured;
        logic        telemetry_fdr_present;
        logic        ecc_corrected;
        logic        parity_poison_faulted;
        logic        watchdog_timed_out;
        logic        local_reset_seen;
        logic        degraded_state;
        logic        telemetry_scoped;
        logic        telemetry_redacted;
        logic        trace_overflowed;
        logic        quote_measurement_bound;
        logic        quote_development_marked;
        logic        audit_recorded;
        logic        mls_denied;
        logic        debug_denied;
        logic        counts_exact;
    } lnp64_m10_state_projection_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] program_id;
        logic [31:0] attachment_generation;
        logic [31:0] cycle_budget;
        logic [31:0] cycles_used;
        logic [31:0] queue_id;
        logic [31:0] mark;
    } lnp64_m9_classifier_commit_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] attachment_generation;
        logic [31:0] packets;
        logic [31:0] ipc_records;
        logic [31:0] rejects;
        logic [31:0] cycle_budget;
        logic [31:0] cycles_used;
        logic        verifier_accepted;
        logic        verifier_rejected;
        logic        packet_steered;
        logic        ipc_steered;
        logic        action_emitted;
        logic        budget_enforced;
        logic        stale_attachment_rejected;
        logic        no_authority_created;
        logic        counts_exact;
    } lnp64_m9_state_projection_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] owner_tid;
        logic [31:0] pointer_generation;
        logic [31:0] heap_generation;
        logic [31:0] size_class;
        logic [63:0] heap_ptr;
    } lnp64_m8_heap_commit_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] pointer_generation;
        logic [31:0] owner_tid;
        logic [31:0] allocations;
        logic [31:0] frees;
        logic        allocated;
        logic        quarantined;
        logic        alloc_completed;
        logic        alloc_size_reported;
        logic        free_completed;
        logic        reuse_completed;
        logic        double_free_rejected;
        logic        stale_pointer_rejected;
        logic        cross_thread_handoff;
        logic        guard_faulted;
        logic        quarantine_observed;
        logic        heap_count_exact;
    } lnp64_m8_state_projection_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] service_id;
        logic [31:0] op_id;
        logic [31:0] continuation_generation;
        logic [31:0] service_generation;
        logic [63:0] requested_rights;
        logic [63:0] returned_rights;
    } lnp64_m6_service_commit_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] service_generation;
        logic [31:0] continuation_generation;
        logic [31:0] installed_caps;
        logic [31:0] completions;
        logic        envelope_validated;
        logic        namespace_dispatched;
        logic        service_continuation_created;
        logic        cap_return_installed;
        logic        returned_cap_narrowed;
        logic        cancel_terminal;
        logic        stale_service_rejected;
        logic        crash_completed;
    } lnp64_m6_state_projection_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] parent_tid;
        logic [31:0] child_tid;
        logic [31:0] child_generation;
        logic [31:0] join_generation;
        logic [31:0] exec_epoch;
        logic [31:0] exit_code;
    } lnp64_m3_process_commit_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [1:0]  parent_state;
        logic [1:0]  child_state;
        logic [31:0] parent_tid;
        logic [31:0] child_tid;
        logic [31:0] child_generation;
        logic [31:0] join_generation;
        logic [31:0] exec_epoch;
        logic        clone_created;
        logic        child_exit_signaled;
        logic        parent_join_completed;
        logic        exec_barrier_stopped_sibling;
        logic        stale_join_rejected;
        logic        exec_cancel_terminal;
        logic        exactly_one_thread_location;
    } lnp64_m3_state_projection_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] root_domain;
        logic [31:0] child_domain;
        logic [31:0] child_budget;
        logic [31:0] parent_budget;
        logic [63:0] requested_rights;
        logic [63:0] delegated_rights;
    } lnp64_m14_domain_commit_t;

    typedef struct packed {
        logic [7:0]  op;
        logic [15:0] status;
        logic [31:0] root_domain;
        logic [31:0] child_domain;
        logic [31:0] delegated_caps;
        logic [31:0] failures;
        logic [31:0] parent_used;
        logic        child_rights_subset_parent;
        logic        child_budget_within_parent;
        logic        excess_budget_rejected;
        logic        frozen_dispatch_rejected;
        logic        resumed_dispatch_allowed;
        logic        destroyed_dispatch_rejected;
        logic        usage_rollup_valid;
        logic        policy_fail_closed;
        logic        counts_exact;
    } lnp64_m14_state_projection_t;

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
        logic [15:0] weight_index;
        logic [63:0] virtual_deadline;
        logic        dispatch_eligible;
        logic [31:0] effective_tile_mask;
        logic [31:0] migration_generation;
        logic [31:0] active_location;
    } lnp64_thread_sched_t;

    typedef struct packed {
        logic [15:0] opcode;
        logic [15:0] profile;
        logic [7:0] rd;
        logic [7:0] rs1;
        logic [7:0] rs2;
        logic [7:0] rs3;
        logic [7:0] rs4;
        logic [7:0] rs5;
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
        logic [31:0] domain_id;
        logic [31:0] domain_gen;
        logic [31:0] pc;
        logic [7:0]  opcode;
        logic [15:0] arch_opcode;
        logic [15:0] action;
        logic [7:0]  operand_rd;
        logic [7:0]  operand_rs1;
        logic [7:0]  operand_rs2;
        logic [7:0]  operand_rs3;
        logic [63:0] operand_imm;
        logic        result_valid;
        logic [7:0]  result_reg;
        logic [63:0] result_value;
        logic [15:0] errno;
        logic [15:0] status;
        logic [15:0] latency_class;
        logic [63:0] wait_source;
        logic [31:0] event_id;
        logic [31:0] fault_id;
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

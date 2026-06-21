`timescale 1ns/1ps

// LNP64 MVS dynamic capability-derivation core.
//
// Models the Derive/Restrict hardware operation: an initiator holds a capability
// {lo, hi, w} (an address range plus write permission) and may derive a new one
// only if the request is a strict SUBSET of the capability it currently holds
// (tighter bounds, no added permission). There is no widening path, so an
// initiator can never forge authority it was not delegated. Memory writes are
// authorized only within the currently-held capability.
module lnp64_mvs_derive (
    input  logic       clk,
    input  logic       rst_n,

    // Derive/Restrict instruction (free adversarial inputs).
    input  logic       derive_valid,
    input  logic [7:0] req_lo,
    input  logic [7:0] req_hi,
    input  logic       req_w,

    // Memory access attempt through the held capability.
    input  logic       acc_req,
    input  logic       acc_w,
    input  logic [7:0] acc_addr,

    output logic [7:0] cap_lo,
    output logic [7:0] cap_hi,
    output logic       cap_w,
    output logic       derive_accepted,
    output logic       mem_we,
    output logic [7:0] mem_waddr
);
    // Root capability this initiator boots with (its maximal authority).
    localparam logic [7:0] ROOT_LO = 8'h10;
    localparam logic [7:0] ROOT_HI = 8'h7f;
    localparam logic       ROOT_W  = 1'b1;

    logic [7:0] c_lo, c_hi;
    logic       c_w;
    assign cap_lo = c_lo;
    assign cap_hi = c_hi;
    assign cap_w  = c_w;

    // A derive is accepted only if it is a subset of the CURRENTLY held cap:
    // a well-formed, tighter range with no added permission.
    logic valid_range, subset;
    assign valid_range = (req_lo <= req_hi);
    assign subset = valid_range && (req_lo >= c_lo) && (req_hi <= c_hi) && (!req_w || c_w);
    assign derive_accepted = derive_valid && subset;

    // Memory write authorized only within the held cap and with write permission.
    logic acc_auth;
    assign acc_auth = acc_req && acc_w && c_w && (acc_addr >= c_lo) && (acc_addr <= c_hi);
    assign mem_we    = acc_auth;
    assign mem_waddr = acc_addr;

    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n) begin
            c_lo <= ROOT_LO;
            c_hi <= ROOT_HI;
            c_w  <= ROOT_W;
        end else if (derive_accepted) begin
            c_lo <= req_lo;
            c_hi <= req_hi;
            c_w  <= req_w;
        end
    end
endmodule

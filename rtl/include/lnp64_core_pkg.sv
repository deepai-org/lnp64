package lnp64_core_pkg;
    function automatic logic [63:0] clz64(input logic [63:0] value);
        integer bit_idx;
        logic seen_one;
        begin
            clz64 = 64'd0;
            seen_one = 1'b0;
            for (bit_idx = 63; bit_idx >= 0; bit_idx = bit_idx - 1) begin
                if (!seen_one && value[bit_idx]) begin
                    seen_one = 1'b1;
                end else if (!seen_one) begin
                    clz64 = clz64 + 64'd1;
                end
            end
        end
    endfunction

    function automatic logic [63:0] ctz64(input logic [63:0] value);
        integer bit_idx;
        logic seen_one;
        begin
            ctz64 = 64'd0;
            seen_one = 1'b0;
            for (bit_idx = 0; bit_idx < 64; bit_idx = bit_idx + 1) begin
                if (!seen_one && value[bit_idx]) begin
                    seen_one = 1'b1;
                end else if (!seen_one) begin
                    ctz64 = ctz64 + 64'd1;
                end
            end
        end
    endfunction

    function automatic logic [63:0] popcnt64(input logic [63:0] value);
        integer bit_idx;
        begin
            popcnt64 = 64'd0;
            for (bit_idx = 0; bit_idx < 64; bit_idx = bit_idx + 1) begin
                popcnt64 = popcnt64 + {63'd0, value[bit_idx]};
            end
        end
    endfunction

    function automatic logic [63:0] bswap64(input logic [63:0] value);
        begin
            bswap64 = {
                value[7:0],
                value[15:8],
                value[23:16],
                value[31:24],
                value[39:32],
                value[47:40],
                value[55:48],
                value[63:56]
            };
        end
    endfunction

    function automatic logic [63:0] mulh_signed(input logic [63:0] lhs, input logic [63:0] rhs);
        logic signed [127:0] lhs_ext;
        logic signed [127:0] rhs_ext;
        logic signed [127:0] product;
        begin
            lhs_ext = {{64{lhs[63]}}, lhs};
            rhs_ext = {{64{rhs[63]}}, rhs};
            product = lhs_ext * rhs_ext;
            mulh_signed = product[127:64];
        end
    endfunction

    function automatic logic [63:0] mulh_unsigned(input logic [63:0] lhs, input logic [63:0] rhs);
        logic [127:0] lhs_ext;
        logic [127:0] rhs_ext;
        logic [127:0] product;
        begin
            lhs_ext = {64'd0, lhs};
            rhs_ext = {64'd0, rhs};
            product = lhs_ext * rhs_ext;
            mulh_unsigned = product[127:64];
        end
    endfunction

    function automatic logic [63:0] mulh_signed_unsigned(input logic [63:0] lhs, input logic [63:0] rhs);
        logic signed [127:0] lhs_ext;
        logic signed [127:0] rhs_ext;
        logic signed [127:0] product;
        begin
            lhs_ext = {{64{lhs[63]}}, lhs};
            rhs_ext = {64'd0, rhs};
            product = lhs_ext * rhs_ext;
            mulh_signed_unsigned = product[127:64];
        end
    endfunction

    function automatic logic [63:0] div_signed64(input logic [63:0] lhs, input logic [63:0] rhs);
        logic [63:0] lhs_abs;
        logic [63:0] rhs_abs;
        logic [63:0] quotient_abs;
        begin
            if (rhs == 64'd0) begin
                div_signed64 = 64'd0;
            end else begin
                lhs_abs = lhs[63] ? (~lhs + 64'd1) : lhs;
                rhs_abs = rhs[63] ? (~rhs + 64'd1) : rhs;
                quotient_abs = lhs_abs / rhs_abs;
                div_signed64 = lhs[63] ^ rhs[63] ? (~quotient_abs + 64'd1) : quotient_abs;
            end
        end
    endfunction

    function automatic logic [63:0] rem_signed64(input logic [63:0] lhs, input logic [63:0] rhs);
        logic [63:0] lhs_abs;
        logic [63:0] rhs_abs;
        logic [63:0] remainder_abs;
        begin
            if (rhs == 64'd0) begin
                rem_signed64 = 64'd0;
            end else begin
                lhs_abs = lhs[63] ? (~lhs + 64'd1) : lhs;
                rhs_abs = rhs[63] ? (~rhs + 64'd1) : rhs;
                remainder_abs = lhs_abs % rhs_abs;
                rem_signed64 = lhs[63] ? (~remainder_abs + 64'd1) : remainder_abs;
            end
        end
    endfunction
endpackage

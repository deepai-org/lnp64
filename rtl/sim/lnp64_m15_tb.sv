`timescale 1ns/1ps

import lnp64_pkg::*;

module lnp64_m15_tb;
    logic clk;
    logic reset_n;
    logic start;
    logic [31:0] scenario_seed;
    logic done;
    logic trace_valid;
    logic [7:0] trace_code;
    logic [63:0] trace_value;
    logic counter_threshold_event;
    logic queue_rights_valid;
    logic queue_overflow_explicit;
    logic event_source_generation_safe;
    logic gate_continuation_unique;
    logic counts_exact;

    lnp64_m15_object_profiles dut(
        .clk(clk),
        .reset_n(reset_n),
        .start(start),
        .scenario_seed(scenario_seed),
        .done(done),
        .trace_valid(trace_valid),
        .trace_code(trace_code),
        .trace_value(trace_value),
        .counter_threshold_event(counter_threshold_event),
        .queue_rights_valid(queue_rights_valid),
        .queue_overflow_explicit(queue_overflow_explicit),
        .event_source_generation_safe(event_source_generation_safe),
        .gate_continuation_unique(gate_continuation_unique),
        .counts_exact(counts_exact)
    );

    lnp64_m15_assertions assertions_i(
        .clk(clk),
        .reset_n(reset_n),
        .done(done),
        .counter_threshold_event(counter_threshold_event),
        .queue_rights_valid(queue_rights_valid),
        .queue_overflow_explicit(queue_overflow_explicit),
        .event_source_generation_safe(event_source_generation_safe),
        .gate_continuation_unique(gate_continuation_unique),
        .counts_exact(counts_exact)
    );

    always #5 clk = ~clk;

    task automatic require(input logic condition, input string message);
        if (!condition) begin
            $fatal(1, "%s", message);
        end
    endtask

    always_ff @(posedge clk) begin
        if (trace_valid) begin
            unique case (trace_code)
                8'd1: $display(
                    "TRACE boot object=%0d generation=%0d queue_capacity=1 counter_threshold=%0d",
                    trace_value[63:32],
                    trace_value[31:0],
                    seeded_threshold_for_display()
                );
                8'd2: $display(
                    "TRACE counter value=%0d threshold=%0d event=1",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd3: $display(
                    "TRACE queue_push value=%0d rights=0x%016h depth=1",
                    trace_value[63:32],
                    {32'd0, trace_value[31:0]}
                );
                8'd4: $display(
                    "TRACE queue_overflow errno=%0d pressure_event=1",
                    trace_value[15:0]
                );
                8'd5: $display(
                    "TRACE event_emit source_gen=%0d event_gen=%0d delivered=1",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                8'd6: $display(
                    "TRACE stale_event source_gen=%0d event_gen=%0d errno=%0d",
                    trace_value[63:32],
                    trace_value[31:0],
                    LNP64_ERR_EREVOKED
                );
                8'd7: $display(
                    "TRACE gate_profile continuation=%0d unique=1 duplicate_errno=%0d",
                    trace_value[63:32],
                    trace_value[15:0]
                );
                8'd8: $display(
                    "TRACE done failures=%0d events=%0d",
                    trace_value[63:32],
                    trace_value[31:0]
                );
                default: $display("TRACE unknown code=%0d value=%0d", trace_code, trace_value);
            endcase
        end
    end

    function automatic logic [31:0] seeded_threshold_for_display();
        if (scenario_seed == 32'd0) begin
            return 32'd3;
        end
        return {28'd0, scenario_seed[11:8]} + 32'd1;
    endfunction

    initial begin
        if (!$value$plusargs("seed=%d", scenario_seed)) begin
            scenario_seed = 32'd0;
        end
        clk = 1'b0;
        reset_n = 1'b0;
        start = 1'b0;

        repeat (4) @(posedge clk);
        reset_n = 1'b1;
        @(posedge clk);
        start = 1'b1;
        @(posedge clk);
        start = 1'b0;

        repeat (36) @(posedge clk);
        require(done, "M15 object-profile slice did not complete");
        require(counter_threshold_event, "M15 counter threshold event was not observed");
        require(queue_rights_valid, "M15 queue rights check was not observed");
        require(queue_overflow_explicit, "M15 queue overflow was not explicit");
        require(event_source_generation_safe, "M15 stale event source was not rejected");
        require(gate_continuation_unique, "M15 gate continuation was not unique");
        require(counts_exact, "M15 counts were not exact");
        $display("LNP64-RTL-M15 PASS");
        $finish;
    end
endmodule

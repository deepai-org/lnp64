create_clock -name lnp64_clk -period 20.000 [get_ports clk]
set_property CLOCK_DEDICATED_ROUTE FALSE [get_nets clk]

set_false_path -from [get_ports reset_n]

# Generic timing-only smoke constraints. Board/package pin constraints live in
# target-specific files such as lnp64_s0_ice40_hx8k_ct256.pcf.
set_property IOSTANDARD LVCMOS33 [get_ports clk]
set_property IOSTANDARD LVCMOS33 [get_ports reset_n]

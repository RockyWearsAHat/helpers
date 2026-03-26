# FPGA and ASIC Hardware

FPGAs and ASICs represent two ends of a customization spectrum: maximum time-to-market flexibility versus maximum performance and density.

## FPGA Architecture

**Logic Blocks** — fundamental compute unit. Typical structure: Configurable Logic Block (CLB) or Logic Array Block (LAB) containing:
- **LUT (Look-Up Table)**: implements combinational logic for K inputs (K=4-6 typical). 4-input LUT = 16-bit memory encoding all 16 input combinations
- **Multiplexers**: route LUT outputs or external signals
- **D flip-flops**: enable sequential (registered) operations
- **Adder chains**: dedicated hardwired adders for arithmetic, avoiding LUT usage

**Modern FPGAs** integrate:
- **DSP Blocks** (multipliers, accumulators) for signal processing
- **Block RAM** (BRAM) for on-chip memory
- **Hard processor cores** (ARM CPU, allowing HPS: Hard Processor System)
- **High-speed transceivers** (28 Gbps+) for 100G networking

**Routing Architecture** — interconnect between LBs and I/Os. Segmented (short-range + long-range wire segments) provides flexibility; full crossbar would be prohibitively large. Routing congestion during place-and-route is common; tools aim for "just-enough" routing to avoid cost, sometimes failing to route designs efficiently.

**Clock Distribution** — global H-tree distribution network ensures low skew. Phase-locked loops (PLLs) allow frequency synthesis (multiply/divide reference clock).

## ASIC Design

**Standard-Cell Flow**:
1. RTL specification (Verilog/VHDL)
2. Logic synthesis → gate-level netlist
3. Placement → assigns gates to physical locations
4. Routing → creates metal interconnect
5. Extraction → computes parasitics (R, C)
6. Sign-off → timing/power analysis

**Full-Custom vs Standard-Cell** — full-custom designs every transistor manually; standard-cell uses pre-characterized gates at cost of slight efficiency loss but massive NRE (non-recurring engineering) savings.

**Gate Array** — intermediate approach: base wafer predefined with transistors, customization only in metal layers. Faster turn-around, lower NRE than full-custom; deprecated in favor of FPGAs for low volume.

## Hardware Description Languages (HDL)

**Verilog** — C-like syntax, created 1983 by Prabhu Goel at Gateway (acquired by Cadence). IEEE 1364 standard. Two assignment operators: blocking (=) and non-blocking (<=). Non-blocking recommended for sequential logic (edges); enables writing state machines without explicit temporary variables.

**Example**: D flip-flop with async reset
```
always @(posedge clk or posedge reset)
  if (reset)
    q <= 1'b0;
  else
    q <= d;
```

**VHDL** — Ada-inspired syntax, more verbose than Verilog but arguably more readable. IEEE 1076 standard (1987). Stronger type system than Verilog; often preferred in aerospace/military (enforced by contracts).

**SystemVerilog** — Verilog superset (IEEE 1800) adding object-oriented features: classes, interfaces, assertions. Enables testbench randomization and coverage analysis; synthesis subset narrower than Verilog (some statements not synthesizable).

## High-Level Synthesis (HLS)

**C-to-HDL Compilation** — tools like Vivado HLS (Xilinx) or Catapult (Mentor) convert C/C++ to RTL, automating tedious handwritten HDL. Tradeoff: generated code often less optimal than hand-tuned HDL, but development velocity improves.

**Pipelining** — HLS automatically inserts pipeline stages (registers between computation blocks) to increase throughput at cost of latency. Designer specifies target initiation interval (II): lower II = higher throughput = more registers.

## FPGA vs ASIC vs GPU

**FPGAs** — reconfigurable at runtime (partial reconfiguration). Fixed area/power to implement any function; flexible but inefficient for any single workload. Good for: prototyping, low-volume products, heterogeneous workloads requiring rapid iteration. Cost: high per-unit (but amortized NRE savings over volume).

**ASICs** — fixed design, maximum area/power optimization for intended task. Cost: enormous NRE (tens of millions), justified only for high volume (>100k units). Turn-around: 9-18 months from tape-out to silicon.

**GPUs** — fixed parallel execution resource (thousands of cores). Excellent for massively parallel compute (ML training, simulation); poor for serial code or memory operations.

## Applications

**Networking** — packet processing (parsing, classification, modification) at line rate. FPGAs common in test equipment; ASICs in production switches/routers.

**ML Inference** — FPGA/ASIC accelerators for neural network inference (lower latency, deterministic than GPU). Google TPU is ASIC; Xilinx Alveo and Intel Stratix for FPGA inference.

**Cryptocurrency Mining** — ASICs dominate (Bitcoin SHA256, Ethereum Keccak). FPGAs viable for newer algorithms or rapid iteration during algorithm wars.

## Development Flow

1. High-level design: algorithm, architecture decisions
2. Register-transfer level (RTL): write Verilog/VHDL or use HLS
3. Simulation: verify functionality with test benches
4. Synthesis: compile to gates/LUTs
5. Place-and-route: physical layout
6. Timing analysis: verify clock constraints met
7. Power analysis: estimate/validate power budget
8. Sign-off: final checks before manufacturing (ASIC) or programming (FPGA)

**Simulation and Verification** — testbenches driving RTL with stimulus vectors, checking outputs against golden reference. Coverage metrics track how much design logic is exercised. Formal verification (model checking, theorem proving) proves correctness for critical paths; computationally expensive.

**Timing Closure** — meeting clock frequency targets requires iterative synthesis/P&R tuning: reduce fan-out (buffer critical signals), break long combinational paths (add pipeline stages), or relax clock period. Timing-driven place-and-route places cells on critical paths close together to minimize wire delay.

**Power Analysis** — static power (leakage) dominates at advanced nodes; dynamic power (switching) decreases. Techniques: clock gating (disable clock to inactive blocks), power gating (cut supply to unused regions), voltage scaling (reduce supply when performance headroom exists).

**Design for Manufacturing (DFM)** — account for lithography limitations, process variation, and reliability. At 5nm/3nm, variations in critical dimension (line width) ~±10%, causing speed/leakage shifts. DFM checks flag designs at risk before expensive tape-out.

**IP Core Integration** — reuse existing designs (memory compilers, high-speed SerDes, processors) via licensing. Integrating third-party IP requires interface compatibility, power/thermal budgeting, and design rule compliance.

**Partial Reconfiguration (FPGA)** — update portions of design at runtime without affecting others. Enables iterative hardware updates, dynamic service loading, or multi-tenancy in shared FPGA. Requires careful isolation between reconfigurable regions.


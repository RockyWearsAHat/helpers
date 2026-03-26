# Quantum Algorithms — Shor's, Grover's, VQE, QAOA, Error Correction, Post-Quantum Cryptography

## Overview

Quantum algorithms exploit quantum mechanical phenomena—superposition, interference, and entanglement—to solve certain classes of problems faster than known classical algorithms. The field distinguishes between **algorithms with proven exponential speedup** (Shor's factoring, Grover's search), **heuristic algorithms without known classical advantage** (VQE, QAOA), and **error correction schemes** that make practical quantum computing possible. Understanding quantum algorithms requires grasping both their mathematical foundations and the practical limitations that determine feasibility.

See also: [quantum-computing-concepts.md](quantum-computing-concepts.md), [security-cryptography-asymmetric.md](security-cryptography-asymmetric.md), [algorithms-graph.md](algorithms-graph.md).

## Shor's Algorithm — Polynomial-Time Factoring

### The Problem and Classical Difficulty

Factoring an N-bit integer $n = p \times q$ into primes $p$ and $q$ is the foundation of RSA cryptography's security. The fastest known classical algorithm (the general number field sieve) takes $O(2^{n^{1/3}})$ time—subexponential but prohibitively slow for large $n$. This asymmetry (easy to multiply, hard to factor) enables RSA.

### Shor's Approach: Period Finding

Shor's algorithm reduces factoring to **period finding**: given a function $f(x) = a^x \bmod n$, find the period $r$ where $a^r \equiv 1 \pmod{n}$.

**Key insight:** If you find an even period $r$, then $\gcd(a^{r/2} \pm 1, n)$ has a high probability of yielding a factor. The classical bottleneck is finding $r$; Shor's algorithm accelerates this via the quantum Fourier transform (QFT).

### Quantum Speedup via QFT

1. **Superposition**: Create superposition of $|0\rangle, |1\rangle, \ldots, |2^m\rangle$ for $m \approx 2n$ qubits.
2. **Phase encoding**: Apply $f(x) = a^x \bmod n$ to each state, encoding the period in relative phases.
3. **Quantum Fourier Transform**: Performs exponential-time classical algorithm in $O(m^2)$ quantum gates. The QFT's interference patterns concentrate amplitude at multiples of $1/r$.
4. **Measurement**: Measure a state leaking information about $r$. Repeat up to $O(\log n)$ times for high confidence.

**Complexity**: $O((n)^3)$ quantum gates for $n$-bit factoring, compared to $2^{O(n^{1/3})}$ classically.

**Current limitation**: Requires 2000–20,000 high-fidelity qubits without decoherence. Today's quantum computers (50–1000 noisy qubits) cannot run Shor's on cryptographically relevant sizes.

## Grover's Algorithm — Quantum Search with Amplitude Amplification

### The Problem and Classical Lower Bound

Search an unsorted database of $N$ items for a single item satisfying a criterion. Classically, this requires $O(N)$ evaluations (you might find it on the first try or the last). There is a **proven lower bound**: any algorithm needs at least $\Omega(N)$ queries.

### Grover's Quadratic Speedup

Grover's algorithm finds a marked item in $O(\sqrt{N})$ iterations. The mechanism: **amplitude amplification**.

**Phase 1: Superposition and Oracle**

1. Prepare superposition $\frac{1}{\sqrt{N}} \sum_{i=0}^{N-1} |i\rangle$.
2. Apply oracle $U_f$: if $f(i) = 1$ (item matches), flip the phase: $|i\rangle \to -|i\rangle$. Otherwise, leave unchanged.
3. After oracle, the marked item has negative amplitude; others are positive.

**Phase 2: Amplitude Amplification (Diffusion)**

1. Apply the **diffusion operator**: $U_s = 2|s\rangle\langle s| - I$ where $|s\rangle$ is the initial superposition.
2. This reflects all amplitudes around the average. Marked amplitudes (negative) grow; unmarked amplitudes (positive) shrink.
3. Repeat oracle + diffusion $\approx \frac{\pi \sqrt{N}}{4}$ times.

**Measurement**: After iteration, the marked item has amplitude close to 1, others near 0. Measure to find the answer with high probability.

**Complexity**: $O(\sqrt{N})$ oracle calls; $O(n)$ gates per iteration for $N = 2^n$ items.

**No exponential speedup, but quadratic matters**: For $N = 2^{128}$, Grover reduces searches from $2^{128}$ to $2^{64}$ iterations—a dramatic gap that motivates post-quantum cryptography.

### When Grover Applies (and When It Doesn't)

Grover is a **lower-level algorithmic primitive**, not a general-purpose search engine:

- **Applies**: Database queries where exact match is hard to structure classically; optimizing over unstructured search spaces.
- **Does not apply**: Problems with structure (e.g., finding factors—Shor's is exponentially better); problems with polynomial classical solutions (Grover's quadratic gain is marginal).
- **Misapplication risk**: Assuming Grover solves all search problems. It doesn't; structure matters.

## Quantum Fourier Transform (QFT)

The QFT is the quantum analog of the discrete Fourier transform, but computed in $O(n^2)$ gates instead of $O(n^2)$ classically (for $n$-qubit systems; classical is faster at small scale).

### Structure

For $N = 2^n$ amplitudes, the QFT applies:

$$|\psi\rangle \to \frac{1}{\sqrt{N}} \sum_{k=0}^{N-1} e^{2\pi i j k / N} |k\rangle$$

This concentrates amplitude at frequencies $k$ where the input had periodic structure.

### Use Cases

1. **Shor's algorithm**: Finding the period of $a^x \bmod n$.
2. **Phase estimation**: Given unitary $U$, find its eigenvalues (estimates phases).
3. **Hidden subgroup problem**: Generalizes period finding; applies to hidden structures in group actions.

**Not a speedup for all DFT applications**: The QFT's advantage depends on problem structure; input/output overhead (classical state preparation and measurement) often dominates.

## Variational Quantum Algorithms: VQE and QAOA

Unlike Shor's and Grover's (which are deterministic algorithms with proven speedups), **VQE and QAOA are heuristics** with no proven quantum advantage over classical methods. They are the workhorses of near-term quantum computing (NISQ era: Noisy Intermediate-Scale Quantum).

### VQE — Variational Quantum Eigensolver

**Goal**: Find the ground-state energy of a Hamiltonian $H$ (useful for molecular simulation, optimization).

**Approach**: Use a parameterized quantum circuit $U(\theta)$ to prepare a state $|\psi(\theta)\rangle$. Measure $\langle\psi(\theta)|H|\psi(\theta)\rangle$. Classically optimize $\theta$ to minimize energy.

**Workflow**:

1. Encode problem (Hamiltonian) classically.
2. Prepare trial state with quantum circuit; gates parameterized by $\theta$.
3. Measure expectation value of $H$ (requires many measurement repetitions).
4. Classical optimizer (gradient descent, Nelder-Mead) adjusts $\theta$.
5. Repeat until convergence.

**Hybrid nature**: Quantum resource (state preparation, measurement) paired with classical optimization. Avoids deep quantum circuits, reducing decoherence impact.

**Advantage claim (unproven)**: Chemistry simulations might benefit from quantum state space's size. Reality: For small molecules, classical methods (coupled-cluster, density functional theory) are often superior. For larger molecules, circuit depth explodes.

**Current use**: Demonstrator for NISQ devices; proofs-of-concept on 10–100 qubits.

### QAOA — Quantum Approximate Optimization Algorithm

**Goal**: Find approximate solutions to combinatorial optimization (MaxCut, graph coloring, satisfiability).

**Approach**: Encode problem as Ising Hamiltonian $H_C$ (cost function). Prepare superposition, apply cost Hamiltonian, then mixer Hamiltonian. Parameters control duration of each. Measure and classically optimize parameters.

**Circuit structure**:

1. Start in superposition from Hadamards.
2. Apply $e^{-i\gamma H_C}$ (cost evolution) for parameter $\gamma$.
3. Apply $e^{-i\beta H_M}$ (mixer evolution, e.g., $H_M = \sum_i X_i$) for parameter $\beta$.
4. Repeat $p$ times (circuit depth $~2p$).
5. Measure bitstring; evaluate cost. Classically optimize $\gamma, \beta$.

**Performance**: QAOA with $p = 1$ or $2$ layers can provide small approximation ratios (e.g., $0.7425$ for MaxCut vs. $0.5$ random), but no quantum advantage over classical algorithms verified at problem sizes solvable on today's hardware.

**Why use it?**: Early exploration of quantum advantage; some believe deeper circuits might outpace classical approaches; competitive advantage unclear.

## Quantum Error Correction and Fault Tolerance

Quantum states are fragile. Environmental interaction causes **decoherence**—loss of quantum information—in microseconds to milliseconds. Error correction is essential for practical quantum computers.

### The Error Correction Challenge

Classical error correction (redundancy, parity checks) works because bit value 0 or 1 doesn't degrade. Quantum states, measured, collapse—copying violates no-cloning; passive redundancy (naively) doesn't work.

### Stabilizer Codes

Stabilizer codes encode a logical qubit into multiple physical qubits, such that errors become detectable via parity measurements without collapsing the encoded state.

**Stabilizer intuition**: A stabilizer operator $S$ commutes with $S$ (i.e., $S^2 = I$). If the state $|\psi\rangle$ is an eigenstate of $S$ with eigenvalue $+1$, then $S|\psi\rangle = |\psi\rangle$. Errors anticommute with some stabilizers, flipping the eigenvalue to $-1$. Measuring stabilizers reveals error syndrome without destroying the state.

**Example: Shor Code** (9 physical qubits, 1 logical qubit):

- Encodes logical qubit redundantly.
- Corrects single-qubit bit-flip or phase-flip errors.
- Requires $~15$ CNOT gates to encode/decode.

### Surface Code and Topological Codes

Surface code is the leading candidate for practical quantum computers:

- **2D array** of qubits on a surface.
- **Plaquette stabilizers** (parity of 4 qubits around a face) detect errors locally.
- **Distance $d$**: errors up to weight $d-1$ correctable.
- **Threshold**: Error rates below $\sim 1\%$ allow arbitrarily long quantum computation with overhead growing as $O(\log(1/\epsilon))$ where $\epsilon$ is physical error rate.

**Practical footprint**: Quantum computer with $n$ logical qubits, distance $d$, requires $\sim 1000 \times d^2 \times n$ physical qubits. For large scale (millions of qubits), viable but represents enormous engineering challenge.

### Magic State Distillation

Stabilizer codes correct Pauli errors but don't directly enable non-Clifford gates (needed for universal computation). **Magic state distillation** creates special quantum states (e.g., $|T\rangle$) using stabilizer operations and measurement feedback, enabling $T$ gates (which perform phase rotations enabling non-Clifford operations).

Cost: Significant overhead; $~100$ physical qubits to produce one "magic" logical $T$ gate.

## Quantum Advantage and Claims

### What "Quantum Advantage" Actually Means

**Quantum supremacy claim** (2019, Google): Run a specific random circuit on 53 qubits and sample its output distribution. Classical simulation claimed to take $10,000$ years; quantum computer does it in 200 seconds.

**Critical issues**:

- **Sampling vs. useful computation**: Sampling random circuit output is different from solving a practical problem (factoring, optimization).
- **Classical simulation baseline disputed**: Refined classical methods later simulated portions of the circuit in hours.
- **No practical application**: The circuit was engineered to be hard for classical simulation, not to solve a real problem.

### Categories of Potential Advantage

1. **Proven exponential advantage**: Shor's (factoring), discrete log. Reality: Requires millions of qubits; not on horizon within a decade.
2. **Proven polynomial advantage**: Grover (quadratic); quantum database search. Limited scope.
3. **Heuristic no-proven-advantage**: VQE, QAOA, variational methods. Unclear if quantum or classical is better at scales solvable today; largest demonstrations don't clearly exceed classical.
4. **Demonstrations of quantum phenomena**: Entanglement, interference, quantum walks. Important for algorithm development; not practical advantage.

**Honest assessment**: No unambiguous quantum advantage on practical problems yet. Claims often conflate problem-specific speedups with general-purpose superiority.

## Post-Quantum Cryptography

Shor's algorithm threatens RSA and elliptic-curve cryptography. Organizations cannot wait for quantum computers to become practical; migration must start now.

### NIST Standardization (as of 2024)

NIST has standardized four families of post-quantum cryptographic algorithms (all believed resistant to known quantum and classical attacks):

1. **Lattice-based** (ML-KEM for key encapsulation; ML-DSA for signatures):
   - Based on hardness of shortest vector problem (SVP) in lattices.
   - Efficient; small keys; extensive cryptanalysis.
   - Leading candidate across use cases.

2. **Hash-based** (XMSS, LMS for signatures only):
   - Based on collision resistance of hash functions.
   - Proven secure in ROM; stateless variants exist.
   - Slower; larger signatures; no general encryption.

3. **Multivariate polynomial** (CRYSTALS-KYBER, similar): Deprecated in favor of lattice approaches due to practical issues.

4. **Code-based** (Classic McEliece): Based on decoding random linear codes. Large keys; not yet adopted.

### Migration Strategy

- **Immediate**: Inventory cryptographic dependencies.
- **Near-term**: Hybrid mode—use both classical and post-quantum algorithms in parallel (backward compatible, hedges against unknown quantum vulnerabilities in PQC).
- **Medium-term**: Phase out classical RSA/ECC; certify post-quantum algorithms through cryptanalysis and standardization.
- **Ongoing**: Monitor new attacks and algorithm developments.

## Practical Limitations and Cautions

### Dequantization and Limitations

Recent work shows some quantum algorithm advantages are weaker than advertised:

- **Dequantization**: Classical algorithms that simulate quantum behavior for certain problems (Grover-like search on structured data, some linear system solvers).
- **Implication**: Quantum advantage may apply to narrower problem classes than hoped.

### Mitigating Factors for Quantum Advantage

1. **Problem structure**: Does classical algorithm have shortcuts not accessible quantumly? E.g., RSA candidates for factoring.
2. **Constant factors**: Quantum algorithm may be asymptotically faster but slower in practice on solvable sizes (requires vast qubits or circuit depth).
3. **Hardware maturity**: 10–20 year timeline for error-corrected quantum computers; algorithm landscape may shift.

### Realistic Timeframe

- **Cryptographically relevant Shor's**: 15–20+ years, assuming Moore's-law-like qubit scaling and error correction breakthroughs.
- **Practical optimization advantage**: Unclear; no timeline.
- **PQC migration necessity**: Now—regulatory and adversarial threat models assume quantum computers eventually exist.
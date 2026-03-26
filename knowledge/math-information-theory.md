# Information Theory — Entropy, Compression & Coding

Information theory, founded by Claude Shannon in 1948, provides a mathematical framework for quantifying information, communication, and the fundamental limits of data compression and reliable transmission. Its concepts permeate software engineering — from compression algorithms and cryptographic security to machine learning loss functions and error detection.

## The Core Insight: Information as Surprise

Shannon's foundational observation: information content of an event is inversely related to its probability. A highly probable event (the sun rising) carries little information; an improbable event (a solar eclipse) carries much. This reframes "information" from a vague concept into a precise, measurable quantity.

**Self-information** of an event with probability p:

$$I(x) = -\log_2 p(x)$$

| Event probability | Self-information (bits) | Intuition                   |
| ----------------- | ----------------------- | --------------------------- |
| 1.0               | 0                       | Certain event — no surprise |
| 0.5               | 1                       | Fair coin flip — one bit    |
| 0.25              | 2                       | Two fair coin flips         |
| 0.125             | 3                       | Three fair coin flips       |
| 0.01              | ~6.64                   | Rare event — high surprise  |

The choice of base-2 logarithm gives information in **bits**. Base-e gives **nats**; base-10 gives **hartleys**. The choice is a unit convention, not a conceptual difference.

## Entropy: Average Surprise

**Shannon entropy** measures the average information content (average surprise) of a random variable:

$$H(X) = -\sum_{x} p(x) \log_2 p(x)$$

Entropy characterizes the uncertainty inherent in a distribution — the number of bits needed on average to describe an outcome.

**Key properties of entropy:**

- Non-negative: $H(X) \geq 0$, with equality iff one outcome is certain
- Maximized by the uniform distribution — maximum uncertainty when all outcomes are equally likely
- For a binary variable with probability p: $H = -p\log_2 p - (1-p)\log_2(1-p)$
- Adding a zero-probability event does not change entropy
- Entropy is concave — mixing distributions increases uncertainty

**Conceptual examples:**

| Source                       | Approximate entropy | Why                                       |
| ---------------------------- | ------------------: | ----------------------------------------- |
| Fair coin                    |             1.0 bit | Maximum uncertainty for 2 outcomes        |
| Biased coin (p=0.99)         |           0.08 bits | Almost certain — very little surprise     |
| Fair 6-sided die             |           2.58 bits | More outcomes, more uncertainty           |
| English text (per character) |       ~1.0–1.5 bits | Highly structured, redundant, predictable |
| Compressed random data       |      ~8.0 bits/byte | Approaches maximum — incompressible       |

## Joint Entropy and Conditional Entropy

When considering multiple random variables together:

**Joint entropy** measures the total uncertainty of two variables considered simultaneously:

$$H(X, Y) = -\sum_{x,y} p(x,y) \log_2 p(x,y)$$

**Conditional entropy** quantifies the remaining uncertainty in X given knowledge of Y:

$$H(X|Y) = H(X,Y) - H(Y)$$

The **chain rule** decomposes joint entropy: $H(X,Y) = H(X) + H(Y|X) = H(Y) + H(X|Y)$

If X and Y are independent: $H(X,Y) = H(X) + H(Y)$ — knowing one tells you nothing about the other. If Y fully determines X: $H(X|Y) = 0$ — no residual uncertainty.

## Mutual Information

**Mutual information** captures how much knowing one variable reduces uncertainty about another:

$$I(X;Y) = H(X) - H(X|Y) = H(Y) - H(Y|X) = H(X) + H(Y) - H(X,Y)$$

- Symmetric: $I(X;Y) = I(Y;X)$
- Non-negative: $I(X;Y) \geq 0$, with equality iff X and Y are independent
- Bounded: $I(X;Y) \leq \min(H(X), H(Y))$

Mutual information is central to feature selection in machine learning (how much does a feature tell about the target?), channel capacity analysis, and measuring statistical dependence without assuming linearity.

## Channel Capacity

A **communication channel** maps inputs to (possibly noisy) outputs. Shannon's **channel coding theorem** establishes that every noisy channel has a maximum rate — the **channel capacity** C — at which information can be communicated with arbitrarily low error probability:

$$C = \max_{p(x)} I(X;Y)$$

The theorem has two parts:

1. **Achievability**: For any rate R < C, there exist codes enabling communication with error probability approaching zero as codeword length grows
2. **Converse**: For any rate R > C, error probability is bounded away from zero regardless of coding scheme

This is a profound existence result — it guarantees reliable communication is possible up to the capacity, but the original proof was non-constructive. Practical codes approaching capacity (turbo codes, LDPC codes, polar codes) took decades to develop.

**The binary symmetric channel** (each bit flipped with probability p) has capacity $C = 1 - H(p)$. At p=0 (noiseless), capacity is 1 bit per use; at p=0.5 (purely random), capacity is 0.

## Source Coding: The Limits of Compression

The **source coding theorem** (Shannon's first theorem) establishes the fundamental limit of lossless compression:

> No lossless compression scheme can represent a source with entropy H at fewer than H bits per symbol on average.

The entropy rate of the source is the absolute floor. Any scheme claiming to compress below entropy either loses information or works only on non-representative data.

### Huffman Coding

Huffman coding constructs **optimal prefix codes** — variable-length codes where no codeword is a prefix of another.

**Construction approach:**

1. Sort symbols by probability
2. Repeatedly merge the two least-probable symbols, assigning 0/1 to each branch
3. The resulting binary tree defines the code

**Properties:**

- Optimal among all prefix codes for known symbol probabilities
- Average code length satisfies: $H(X) \leq L_{avg} < H(X) + 1$
- Requires known or estimated probability distribution
- Per-symbol encoding — does not exploit inter-symbol correlations

Arithmetic coding addresses Huffman's limitation by encoding entire messages as a single number, achieving average lengths closer to the entropy bound, especially when symbol probabilities are skewed or the alphabet is large.

### Lossless vs Lossy Compression

| Aspect            | Lossless                              | Lossy                                      |
| ----------------- | ------------------------------------- | ------------------------------------------ |
| Guarantee         | Perfect reconstruction                | Approximate reconstruction                 |
| Theoretical limit | Entropy rate of source                | Rate-distortion function                   |
| Typical ratios    | 2:1 to 10:1 depending on source       | 10:1 to 100:1+ depending on quality target |
| Applicable when   | Exact data required (code, databases) | Perceptual fidelity sufficient (media)     |
| Approach          | Exploit statistical redundancy        | Exploit perceptual irrelevance             |

**Rate-distortion theory** extends Shannon's framework to lossy compression: given a maximum acceptable distortion D, the minimum achievable rate R(D) is a well-defined function of the source statistics and distortion measure.

## Kolmogorov Complexity

While Shannon entropy measures the average information content of a random source, **Kolmogorov complexity** measures the information content of an individual string:

$$K(x) = \text{length of the shortest program that outputs } x \text{ and halts}$$

**Key properties:**

- Independent of the probability distribution — a property of the string itself
- Incomputable — no algorithm can determine K(x) for all x (related to the halting problem)
- Invariance theorem: the choice of universal Turing machine changes K(x) by at most a constant
- Most strings of length n have K(x) ≈ n — they are incompressible (random)
- A string is "random" in the Kolmogorov sense iff it has no shorter description than itself

**Relationship to entropy:** For a random variable X with distribution p, the expected Kolmogorov complexity $E[K(X)]$ approximately equals the Shannon entropy $H(X)$, up to a constant. Shannon entropy is a property of distributions; Kolmogorov complexity is a property of individual objects.

## Cross-Entropy and KL Divergence

When one distribution is used to model another:

**Cross-entropy** measures the average number of bits needed to encode samples from distribution p using a code optimized for distribution q:

$$H(p, q) = -\sum_x p(x) \log_2 q(x)$$

**Kullback-Leibler divergence** (relative entropy) measures the extra bits required — the inefficiency of assuming q when the true distribution is p:

$$D_{KL}(p \| q) = \sum_x p(x) \log_2 \frac{p(x)}{q(x)} = H(p,q) - H(p)$$

**Properties of KL divergence:**

- Non-negative: $D_{KL}(p \| q) \geq 0$, with equality iff p = q
- Asymmetric: $D_{KL}(p \| q) \neq D_{KL}(q \| p)$ in general
- Not a true metric (violates symmetry and triangle inequality)
- Forward KL ($D_{KL}(p \| q)$) penalizes q for placing low probability where p is high — "mean-seeking"
- Reverse KL ($D_{KL}(q \| p)$) penalizes q for placing probability where p is low — "mode-seeking"

In machine learning, minimizing cross-entropy loss is equivalent to minimizing KL divergence from the true distribution, since the entropy of the true distribution H(p) is constant with respect to model parameters.

## Error Detection and Correction

Redundancy enables reliability. Error-correcting codes add structured redundancy so that errors introduced during transmission or storage can be detected or corrected.

**Fundamental concepts:**

| Concept            | Description                                                            |
| ------------------ | ---------------------------------------------------------------------- |
| Hamming distance   | Number of positions where two codewords differ                         |
| Minimum distance d | Minimum Hamming distance between any two valid codewords               |
| Error detection    | A code with minimum distance d detects up to d-1 errors                |
| Error correction   | A code with minimum distance d corrects up to ⌊(d-1)/2⌋ errors         |
| Rate               | Ratio of information bits to total bits — efficiency of the code       |
| Shannon limit      | The theoretical maximum rate for a given channel and error probability |

**Classes of error-handling codes:**

- **Parity checks** — single bit detects odd numbers of errors; no correction
- **CRC (cyclic redundancy check)** — polynomial division over GF(2); strong burst-error detection; widely used in networking and storage
- **Checksums** — arithmetic sums; lightweight detection for accidental corruption
- **Hamming codes** — correct single-bit errors, detect double-bit errors; used in ECC memory
- **Reed-Solomon codes** — operate on symbols rather than bits; correct burst errors; used in QR codes, optical media, deep-space communication
- **LDPC and turbo codes** — approach Shannon capacity; used in modern wireless and storage standards

The trade-off is always between redundancy (overhead) and reliability (error tolerance). Shannon's channel coding theorem guarantees that codes exist achieving reliability at any rate below capacity, but practical code design involves balancing complexity, latency, and performance.

## Applications in Software Engineering

### Compression Algorithms

Compression implementations combine modeling (predicting the next symbol) with coding (encoding the prediction residual efficiently):

- **Dictionary methods** (LZ77, LZ78, LZW) — build dictionaries of recurring patterns; effective on structured data
- **Context modeling** (PPM, context mixing) — predict symbols from preceding context; higher compression, higher complexity
- **Transform-based** (Burrows-Wheeler, move-to-front) — rearrange data to expose redundancy before entropy coding
- **Deduplication** — identify and eliminate identical blocks at the storage level; entropy is measured per unique block

### Cryptographic Security

Information theory provides lower bounds on cryptographic security:

- **Perfect secrecy** (one-time pad): the ciphertext reveals zero information about the plaintext — $I(M;C) = 0$ — but requires a key as long as the message
- **Unicity distance**: the minimum ciphertext length at which a cipher is theoretically breakable — depends on the key entropy and the redundancy of the plaintext language
- **Min-entropy**: the most conservative entropy measure, based on the most probable event; used in analyzing password strength and random number generators

### Machine Learning

Information-theoretic quantities are foundational in learning:

- **Cross-entropy loss**: the standard loss function for classification, directly connected to maximum likelihood estimation
- **Mutual information**: used in feature selection (select features with high mutual information with the target) and as a training objective in representation learning
- **KL divergence**: the objective in variational inference; measures how well an approximate posterior matches the true posterior
- **Information bottleneck**: a framework for finding compressed representations that preserve relevant information about a target variable
- **Minimum description length (MDL)**: model selection principle — prefer the model that gives the shortest total description of model + data given model

### Error Detection in Practice

Software systems use information-theoretic error detection pervasively:

- Network protocols (TCP, Ethernet) use checksums and CRCs to detect corruption
- Storage systems (ZFS, Btrfs) use checksums on data blocks to detect bitrot
- Distributed systems use hash-based integrity verification
- Version control content-addressing (SHA hashing) detects any modification

### Data Compression and Deduplication

At the systems level, compression and deduplication trade computation for storage:

- **Inline vs post-process**: compress during write (latency cost) vs compress later (temporary space cost)
- **Block-level vs file-level deduplication**: granularity affects dedup ratio and metadata overhead
- **Content-defined chunking**: variable-size blocks based on content fingerprints; robust to insertions and deletions
- **Compression ratio estimation**: sampling and entropy estimation before committing to full compression

## Connections to Other Domains

| Domain            | Information-theoretic connection                                   |
| ----------------- | ------------------------------------------------------------------ |
| Thermodynamics    | Entropy in statistical mechanics parallels Shannon entropy         |
| Quantum computing | Quantum information theory extends to qubits, entanglement entropy |
| Linguistics       | Entropy rate measures language complexity and predictability       |
| Neuroscience      | Information processing capacity of neural systems                  |
| Economics         | Information asymmetry, signaling theory                            |
| Statistics        | Fisher information, sufficient statistics, exponential families    |

## Conceptual Pitfalls

- **Entropy is not randomness.** A deterministic system has zero entropy. High entropy means high uncertainty, not chaos.
- **Compression below entropy is impossible losslessly** — claims otherwise either lose information or measure entropy incorrectly.
- **Shannon entropy assumes known distributions.** In practice, distributions must be estimated, and model mismatch incurs the cost measured by cross-entropy.
- **Kolmogorov complexity is incomputable** — it is a theoretical tool for reasoning about strings, not a practical measurement.
- **Bits are not always binary digits.** "Bits" in information theory are a unit of information, not necessarily binary storage units.
- **Mutual information captures all statistical dependence**, not just linear correlation — but estimating it from finite samples is notoriously difficult.

## Summary of Key Quantities

| Quantity                   | Measures                                 | Formula sketch                         |
| -------------------------- | ---------------------------------------- | -------------------------------------- | --------------- |
| Self-information I(x)      | Surprise of one event                    | $-\log p(x)$                           |
| Entropy H(X)               | Average surprise of a source             | $-\sum p \log p$                       |
| Joint entropy H(X,Y)       | Total uncertainty of two variables       | $-\sum p(x,y) \log p(x,y)$             |
| Conditional entropy H(X    | Y)                                       | Residual uncertainty after observing Y | $H(X,Y) - H(Y)$ |
| Mutual information I(X;Y)  | Shared information between two variables | $H(X) + H(Y) - H(X,Y)$                 |
| Cross-entropy H(p,q)       | Encoding cost under wrong model          | $-\sum p \log q$                       |
| KL divergence              | Extra cost of wrong model                | $\sum p \log(p/q)$                     |
| Channel capacity C         | Maximum reliable communication rate      | $\max_{p(x)} I(X;Y)$                   |
| Kolmogorov complexity K(x) | Shortest program producing a string      | (incomputable)                         |

# Compression Algorithms — Lossless, Lossy, Entropy Coding, and Streaming

## Overview

**Data compression** reduces storage space or transmission bandwidth by exploiting redundancy and patterns in data. Compression is fundamental to storage systems, networks, and media distribution.

**Categories:**
- **Lossless:** Perfect reconstruction; used for data, executables, archives.
- **Lossy:** Discards imperceptible information; used for images, audio, video.

This note focuses on algorithmic foundations; compression ratios and speeds depend heavily on data characteristics and implementation details.

---

## Information-Theoretic Bounds

**Shannon's entropy:** The theoretical limit for lossless compression of independently distributed symbols:

$$H(X) = -\sum p_i \log_2 p_i$$

where $p_i$ is the probability of symbol $i$.

**Interpretation:** For a message with entropy $H$ bits per symbol, no lossless compression algorithm can achieve average size less than $H$ bits per symbol on average.

**Example:** A binary message where both symbols are equally likely (uniform distribution) has entropy 1 bit per symbol. Any algorithm achieving less is impossible (though on real data with non-uniform distributions, compression below the average is achievable).

**Practical consequence:** Current algorithms (Huffman, LZ-based, arithmetic coding) approach but do not exceed this bound.

---

## Lossless Compression

### Huffman Coding

**Idea:** Assign variable-length binary codes to symbols. Frequent symbols get shorter codes.

**Algorithm:**
1. Count symbol frequencies.
2. Build a binary tree (Huffman tree):
   - Start with each symbol as a single-node tree with weight = frequency.
   - Repeatedly combine the two smallest-weight trees into a new tree (weight = sum).
   - Continue until one tree remains.
3. Assign binary codes: left child = 0, right child = 1.

**Example:**
```
Symbols: A=45%, B=30%, C=15%, D=10%
Tree structure (from bottom up):
        (100)
       /     \
    (45)A   (55)
           /    \
        (30)B   (25)
               /    \
            (15)C   (10)D

Codes: A=0, B=10, C=110, D=111
```

**Analysis:**
- Average code length approaches entropy from above.
- Off-by-one optimal: no code is more than 1 bit longer than optimal.
- Construction is $O(n \log n)$ (where $n$ = number of distinct symbols).

**Pros:**
- Optimal among fixed-length code schemes.
- Low overhead (just the tree).
- Fast decoding (tree traversal).

**Cons:**
- Assumes fixed probability distribution; compression ratio varies if data distribution changes.
- Tree must be sent with compressed data (overhead for small files).
- Cannot adapt to changing statistics within a stream.

**Arithmetic coding:** An alternative to Huffman that achieves compression closer to entropy by treating the message as a single arithmetic interval. More complex to implement; slightly better compression but slower.

### LZ77 (Sliding Window)

**Idea:** Replace repeated sequences with references to earlier occurrences.

**Structure:** A sliding window (e.g., 32KB) of recent data. For each symbol, try to match a sequence in the window and output a (offset, length) reference.

**Algorithm:**
```
for each position in input:
    find the longest match in the sliding window
    if match found:
        output (offset, length)
        advance past the match
    else:
        output the literal symbol
        advance 1 position
    slide the window forward
```

**Parameters:**
- Window size: 16KB–32KB typical (trade-off between compression and search time).
- Min match length: 3–4 bytes (avoids overhead of small matches).

**Compression:** Highly effective for repetitive data (text, code, highly structured formats).

**Complexity:** $O(n \cdot w)$ with naive matching (where $w$ = window size); $O(n \log w)$ with hash tables or suffix structures.

**Variants:**
- **Simple LZ77:** As described.
- **DEFLATE:** Combines LZ77 with Huffman coding on the output (used in ZIP, gzip, PNG).

### LZ78 / Lempel-Ziv-Welch (LZW)

**Idea:** Build a dictionary of repeated sequences seen so far; replace sequences with dictionary indices.

**Algorithm:**
```
dictionary = {all single bytes}
current_code = 256 (or first available after single bytes)
output = []
position = 0

while position < input.length:
    find the longest sequence starting at position that's in dictionary
    output the dictionary index
    if there's one more byte:
        add (sequence + next byte) to dictionary if space available
        advance past the sequence
    else:
        break
```

**Parameters:**
- Dictionary size: often 4K–16K entries (fixed at 256 + N new entries).
- Stream-based: no explicit tree or table transmission (dictionary built dynamically).

**Pros:**
- Adaptive; learns patterns as it processes the stream.
- No need to transmit a tree or probability table.
- Effective for varied data (text, images, executables).

**Cons:**
- Cannot grow dictionary beyond a fixed size without resetting it.
- Compression can degrade once dictionary is full (no new patterns learned).

**Used in:** GIF images (Unisys LZW patent now expired), early Unix `compress` utility.

---

## Modern Lossless Algorithms

### DEFLATE (ZIP, gzip, PNG)

**Combination:** LZ77 + Huffman coding + byte alignment.

**Process:**
1. Scan input with LZ77; output literals and (offset, length) references.
2. Apply Huffman coding to the stream of literals/references.
3. Add framing and checksums.

**Parameters:**
- Window: 32KB.
- Compression levels (1–9): trade CPU time for compression ratio.

**Compression ratio:** 50–70% for text; 20–40% for images (after LZ77, Huffman adds little).

**Speed:** Fast (10–100 MB/s on modern CPUs depending on level).

**Standard:** RFC 1951 (core), RFC 1952 (gzip framing).

---

### zstd (Zstandard)

**Modern replacement for DEFLATE:** Designed by Facebook/Meta circa 2015.

**Features:**
- **Dictionary support:** Pre-load a dictionary (e.g., common JSON structure) to improve compression on similar data.
- **Multiple compression levels:** 1–22 (lower = faster; higher = better compression).
- **Streaming:** Incremental compression/decompression.
- **Faster than DEFLATE** at comparable compression ratios; or better compression at same speed.

**Architecture:**
- Hybrid approach: LZ77-style matching + entropy coding.
- Optimized for modern CPUs (SIMD-friendly).

**Compression ratio:** Slightly better than DEFLATE; can achieve 40–80% on text.

**Speed:** 100–500 MB/s for compression (depending on level); decompression ~500+ MB/s.

**Use cases:** HTTP content encoding, databases (RocksDB, ClickHouse), Linux kernel archives.

---

### Brotli

**Google's compression algorithm (circa 2013); standardized for HTTP.**

**Features:**
- **Slower than zstd but better compression** on many data types.
- **Context modeling:** Classifies data into contexts (text, HTML, JSON, etc.) and applies context-specific Huffman codes.
- **Pre-processing:** Moves-to-front, run-length encoding before compression.

**Compression levels:** 1–11.

**Compression ratio:** Often 10–20% smaller than gzip on web content.

**Speed:** 10–50 MB/s for compression (slow); 100–500 MB/s for decompression.

**Use cases:** HTTP compression (widely supported by browsers), web archives.

### LZ4

**Goal: Fast compression, acceptable ratio.**

**Design:**
- Simpler LZ77 variant; no Huffman, no entropy coding.
- Minimal processing overhead.

**Compression ratio:** 20–40% on typical data; worse than DEFLATE/zstd.

**Speed:** 200–500+ MB/s compression; 500+ MB/s decompression (fastest among these algorithms).

**Trade-off:** Sacrifices compression for speed. Useful when compression must not bottleneck processing.

**Use cases:** In-memory compression, cache compression, logging systems where speed >ratio.

---

### Snappy

**Google's compression (circa 2011); similar design goals to LZ4.**

**Characteristics:**
- Format is very simple; easy to implement.
- Fast but not as fast as LZ4.
- Ratio slightly better than LZ4.

**Speed:** 100–400 MB/s compression; 300–600 MB/s decompression.

**Use cases:** BigTable, Cassandra, data interchange within systems where speed is valued.

---

## Entropy Coding

Beyond Huffman, two main approaches to convert symbol probabilities into binary:

### Arithmetic Coding

**Idea:** Treat the entire message as a single rational number in the interval [0, 1). Iteratively narrow down the interval based on each symbol's probability.

**Example:**
```
Message: ABC with probabilities P(A)=0.5, P(B)=0.3, P(C)=0.2

Start: interval [0, 1)
Encode A (prob 0.5): new interval [0, 0.5)
Encode B (prob 0.3 of [0, 0.5)): new interval [0, 0.15)
Encode C (prob 0.2 of [0, 0.15)): new interval [0, 0.03)
Output: binary representation of any number in [0, 0.03), e.g., 0.000111... ≈ 0.029
```

**Properties:**
- Approaches entropy exactly; no off-by-one gap like Huffman.
- Slower than Huffman (need rational arithmetic).
- Range arithmetic can cause overflow issues; careful implementation required.

**Used in:** JPEG, MPEG (patent-encumbered historically, now mostly free).

### Asymmetric Numeral Systems (ANS)

**Recent (2009): A new entropy coding method.**

**Advantage:** Combines near-arithmetic-coding efficiency with Huffman-like speed. Used in modern compressors (zstd uses a variant).

---

## Lossy Compression

### Images: JPEG

**Goal:** Exploit human visual perception to discard imperceptible details.

**Process:**
1. **Color space conversion:** RGB → YCbCr (separates luminance from chrominance; humans see luminance in higher detail).
2. **Subsampling:** Reduce chrominance resolution (e.g., 4:2:0 = 1/4 chrominance data).
3. **DCT (Discrete Cosine Transform):** Convert 8×8 pixel blocks to frequency space.
4. **Quantization:** Round frequency coefficients, discarding high-frequency details (where humans are less sensitive).
5. **Entropy coding:** Huffman or arithmetic coding on the quantized coefficients.

**Compression ratio:** 10:1 to 30:1 for photographs (depending on quality parameter).

**Artifacts:** Blocking (8×8 blocks visible), ringing (halos around edges), banding (visible posterization in gradients).

### Audio: MP3 (High-level overview)

**Process:**
1. **Psychoacoustic analysis:** Identify sounds humans cannot hear (masked by louder sounds).
2. **FFT:** Convert audio to frequency domain.
3. **Quantization:** Reduce precision of masked frequencies.
4. **Huffman coding:** Compress the result.

**Compression ratio:** 10:1 to 20:1 (128 kbps MP3 ≈ 1.4 Mbps CD quality).

**Artifacts:** Loss of detail in masked frequencies; audible at very low bitrates.

---

## Streaming & Incremental Compression

**Challenge:** Compress data arriving incrementally without buffering the entire stream.

**Approaches:**

1. **Fixed block size:** Compress 1MB blocks independently. Ratio suffers (can't find matches across blocks).

2. **Rolling window:** Maintain a recent window; compress new data with references to the window. Reduces compression but enables streaming (used in DEFLATE, zstd).

3. **Dictionary reset:** Periodically reset the compressor's state (dictionary in LZW) to prevent degradation after long streams. Ratio drops periodically.

**Trade-off:** Streaming compression is inherently less efficient than offline compression (can't look ahead; can't optimize globally).

---

## Choosing Compression Algorithms

| Algorithm | Speed | Ratio | Adaptivity | Use Case |
|-----------|-------|-------|-----------|----------|
| DEFLATE | Medium | Medium | Low | Legacy systems, ZIP archives, web compatibility |
| zstd | Fast | Good | Medium | Modern systems, RocksDB, HTTP |
| Brotli | Slow | Excellent | High | Web (static), text-heavy data |
| LZ4 | Fastest | Poor | Low | Speed-critical (caches, logs) |
| Snappy | Fast | Fair | Low | In-memory, interchange format |
| JPEG | N/A | Excellent | N/A | Photographs |
| MP3 | N/A | Excellent | N/A | Audio distribution |

**Rules of thumb:**
- **For archives:** zstd (modern) or DEFLATE (compatibility).
- **For network transmission:** Brotli (precomputable) or zstd (streaming).
- **For caches:** LZ4 (speed) or Snappy.
- **For images/audio:** Format-specific codecs (JPEG, H.264, etc.).

---

## Domain-Specific Compression

### Time Series Data

- **Delta encoding:** Store differences between consecutive points instead of absolute values (reduces range, aids compression).
- **Gorilla (Facebook):** Specialized algorithm for timestamped metrics; 10x better compression than generic on time-series.

### Structured Data (JSON, Protocol Buffers)

- **Schema-aware compression:** Compressor knows the schema; optimizes for repeated field names, types.
- **Custom dictionaries:** Pre-load compressor with schema strings.

### Repeated Patterns

- **Dictionary pre-loading:** zstd supports custom dictionaries; significant improvement if the dictionary is well-chosen.

---

## Modern Trends

1. **Hardware acceleration:** zstd, LZ4 now have SIMD implementations and GPU variants.

2. **Machine learning:** Experiments with neural networks to model data, but not yet mainstream; traditional methods still dominate.

3. **Learned index structures:** Using ML to improve compression of index structures (e.g., B-trees).

4. **Hybrid approaches:** Combine multiple algorithms per data type (e.g., zstd for structured, JPEG for images).

---

## See Also

- math-information-theory.md (entropy, Shannon's limits)
- systems-binary-formats.md (file format compression)
- database-indexes.md (compression in index structures)
- algorithms-string.md (pattern matching, which underlies LZ algorithms)
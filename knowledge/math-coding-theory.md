# Coding Theory — Error Detection, Correction, and Reliable Transmission

## Overview: From Noise to Certainty

Coding theory addresses a fundamental reality: communication channels are noisy. Bits flip, packets drop, storage media degrade. Coding theory provides mathematical tools to detect and correct errors, enabling reliable transmission and storage despite imperfect media.

The core question: given a noisy channel with error probability ε, what's the maximum rate R at which information can be sent with arbitrarily small error probability? Shannon's channel coding theorem (math-information-theory.md) establishes that this rate is the **channel capacity** C, and codes achieving near-capacity performance exist. Coding theory builds practical codes that approach these limits.

## Parity and Checksum Codes — Detection Only

The simplest code adds redundancy: a field that summarizes the data.

### Even Parity

Append one bit such that the total number of 1-bits is even:

```
Data: 1011 → Append 1 → 10111 (five 1s, odd, error detected after transmission)
Data: 1010 → Append 1 → 10101 (four 1s, even, any single error flips parity)
```

**Detects:** Single-bit errors, odd numbers of errors.
**Cannot correct:** At least two errors produce the same parity as no error.

### Checksums

Internet protocols use checksums: sum data bytes modulo 2^16 or 2^32, append the complement.

```
TCP Checksum: sum all 16-bit words, take one's complement, append.
Receiver: sum all words including checksum. Valid if result is 0xFFFF.
```

Checksums detect **at least one-bit errors** if message length is less than the checksum width. They trade computational simplicity for mathematical weakness — errors in multiple fields might cancel.

**Application:** TCP/IP networking, storage integrity checks. Cost: one bit per datum or 4 bytes per packet.

## Hamming Codes — Single Error Correction

Richard Hamming (1950) invented codes that correct single-bit errors with minimal overhead.

### (7,4) Hamming Code

Encodes 4 data bits into 7 bits: 3 parity bits + 4 data bits.

```
Positions: 1 2 3 4 5 6 7       (1-indexed)
Type:      P P D P D D D       (P = parity, D = data)

Parity layout:
  p1 checks positions with bit 0 set: 1, 3, 5, 7 (binary: _001, _011, _101, _111)
  p2 checks positions with bit 1 set: 2, 3, 6, 7 (binary: _010, _011, _110, _111)
  p4 checks positions with bit 2 set: 4, 5, 6, 7 (binary: _100, _101, _110, _111)
```

Decoder computes three syndrome bits:
- s1 = XOR of positions {1, 3, 5, 7}
- s2 = XOR of positions {2, 3, 6, 7}
- s4 = XOR of positions {4, 5, 6, 7}

If all are 0, no error. Otherwise, (s4 s2 s1) in binary gives the error position.

**Example:** Data [1 0 1 1] → Codeword [1 0 1 1 0 1 1]. Transmit. Receive [1 1 1 1 0 1 1] (bit 2 flipped). Syndrome = [0 1 0] (binary 010 = 2), so bit 2 is wrong. Flip it. Recover [1 0 1 1 0 1 1].

**Distance:** Hamming code has minimum distance 3 (any two codewords differ in 3 bits). This guarantees single-error correction and double-error detection.

**Efficiency:** (7,4) encodes 4 bits with 7, overhead 75%. (2047, 2048) encodes 2048 bits with 11 parity bits, overhead ~0.5%.

**Application:** DRAM error correction (Hamming codes or SECDED variants), old communication protocols.

## Reed-Solomon Codes — Burst Error Correction

Hamming codes correct isolated errors. For burst errors (consecutive bits corrupted), Reed-Solomon codes are more efficient.

RS codes treat data as polynomials over finite fields (Galois Fields).

### Basic Idea

Encode k information symbols as a polynomial P(x) of degree k-1 over GF(2^m). Add n-k parity symbols by evaluating P at additional field elements. The n symbols are sent.

```
RS(n, k) encodes k symbols into n symbols with n-k parity checks.
Can correct up to ⌊(n-k)/2⌋ symbol errors (not bits, but m-bit symbols).

RS(255, 223) encodes 223 bytes with 32 parity bytes, can correct 16 byte errors.
Each error in a symbol (byte) costs one parity symbol to correct.
```

**Distance:** RS(n,k) has minimum distance n-k+1 (Singleton bound, optimal).

### Properties

- **Burst correction:** If errors affect consecutive symbols, RS efficiently uses parity. For random bit errors, RS is less efficient than concatenated codes.
- **Decoding:** Using syndrome decoding and discrete logarithm (computationally intensive for large fields), roots of error polynomial are found, enabling correction.
- **Alphabet size:** Operating over GF(2^m), each symbol is m bits. Larger alphabets reduce overhead but increase computation.

**Application:** QR codes (combine Reed-Solomon with interleaving for robustness), RAID storage systems, optical and cellular networks.

## LDPC and Turbo Codes — Approaching Capacity

Shannon proved that codes approaching capacity exist, but constructing them was open for decades. Modern codes achieve dramatically better performance.

### LDPC Codes (Low-Density Parity-Check)

LDPC codes (Gallager, 1962; rediscovered Calderbank et al., 1998) are defined by sparse parity-check matrices. Encoding and decoding operate on a bipartite graph where variable nodes represent bits and check nodes represent parity constraints.

Each bit participates in only a few parity checks (sparse); decoding is iterative message-passing between variable and check nodes.

```
Performance: LDPC codes can achieve transmission within 0.0045 dB of Shannon capacity
             (theoretically optimal rate-distortion limit) with low-complexity decoding.
             Practical codes: rate 1/2, can correct ~45% bit error rate.
```

**Advantage:** Linear-time encoding/decoding, parallelizable, works for long codewords.
**Application:** 5G mobile, DVB-S2 satellite, WiFi 802.11ac/n.

### Turbo Codes

Turbo codes (Berrou et al., 1993) concatenate two convolutional coders with an interleaver between them. An interleaver permutes bits before encoding, spreading burst errors.

Decoding uses iterative belief propagation: the output of one decoder feeds back to refine the input of the other, repeating until convergence. This soft-feedback dramatically improves performance.

```
Turbo codes achieve near-Shannon-capacity performance with iterative decoding.
Practical rate: 1/3, bit error rate ~10^-5 at 0.7 dB SNR (0.27 dB from capacity).
```

**Application:** 3G/4G cellular (LTE), satellite communications.

## Fountain Codes — Rateless Transmission

Fountain codes generate an essentially unlimited stream of encoded packets; any subset large enough can recover the original. This is powerful for unreliable broadcast channels where the number of receivers and their conditions are unknown.

### LT Codes (Luby Transform)

LT codes (Luby, 2002) are the first practical rateless code.

**Encoding:** Each output packet XORs a random subset of input packets. The subset size ("degree") is chosen from a degree distribution that optimizes recovery probability.

```
Input: k packets
Output: generate as many encoded packets as needed
Decoder: collect encoded packets, solve via Gaussian elimination over GF(2)
```

**Property:** The output stream can be truncated anywhere; decoder succeeds once it has received slightly more than k packets (overhead ~ε·k for small ε).

**Application:** Multicast where receivers have different channel quality, network coding.

### Raptor Codes

Raptor codes (Shokrollahi, 2006) improve on LT by pre-encoding input with a systematic code, then applying LT to the result. This ensures faster decoding and lower overhead.

```
Raptor codes: overhead approaches 1% for practical sizes.
Decoding complexity: O(k log k) vs. LT's O(k^2) Gaussian elimination.
```

**Application:** 3GPP multimedia broadcast (MBMS), DVB-H handheld TV.

## Applications and Context

### RAID Storage

RAID-6 uses dual-parity schemes (often Reed-Solomon) to survive two disk failures. Encoding adds one write per two data writes; decoding on failure reads all remaining disks and solves linear equations over GF(2^8).

**Trade-off:** RAID-6 survives correlated failures (e.g., manufacturer defect hitting multiple drives in a batch). Modern drives have high failure rates; RAID-6 is a practical necessity.

### QR Codes

QR codes interleave Reed-Solomon encoded data blocks. This spreads burst errors (physical damage, shadows) across redundancy capable of correction. Encoding overhead ~30%; can recover 30% missing data.

### Networking: TCP Checksums and 5G FEC

TCP checksums are weak (only ~0.1 bits of coverage per packet in practice due to bit correlation in network traffic). 5G uses LDPC and Turbo codes for wireless links; forward error correction reduces retransmission overhead dramatically.

## The Information-Theoretic Limit

All these codes operate under Shannon's insight: channel capacity C bits per use limits transmission rate. As codeword length approaches infinity, codes achieving arbitrarily close to C with arbitrarily low error probability exist, but finite codes trade-off rate, complexity, and error probability.

Modern codes (LDPC, Turbo, Polar) achieve within 0.1 dB of capacity for practical block sizes. Further gains require longer codewords and higher decoding complexity.

## Cross-References

See also: math-information-theory.md, security-cryptography-symmetric.md (for GF arithmetic), storage-systems.md.
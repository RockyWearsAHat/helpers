# String Algorithms — Pattern Matching, Tries, Suffix Structures, Hashing

String algorithms power search engines, text editors, DNA analysis, and data compression. The core challenge: find patterns, structures, or properties efficiently without scanning the entire text for every query.

## Exact Pattern Matching

All require scanning text once; performance depends on preprocessing the pattern or text.

### Naive/Brute Force — O(nm)

Compare pattern at every position in text. No preprocessing.

```
for i = 0 to n-m:
    if text[i..i+m-1] == pattern:
        return i
```

**Worst case**: O(nm) when false matches abound (e.g., pattern "aaa" in text "aaaa...ab").  
**Average case**: often O(n) due to early mismatch on first char.  
**When to use**: Short patterns, text too large to preprocess, or as fallback.

### Knuth-Morris-Pratt (KMP) — O(n + m)

Preprocess pattern to compute failure function: if mismatch at position j, where to resume?

**Failure function** (or prefix table): $fail[i]$ = length of longest proper prefix of pattern[0..i] that's also a suffix.

Example: pattern "ABAB" → fail = [0,0,1,2] (prefix "A" matches suffix at position 1, "AB" at position 3).

```
failures[0] = 0
for i = 1 to m-1:
    j = failures[i-1]
    while j > 0 and pattern[i] != pattern[j]:
        j = failures[j-1]
    if pattern[i] == pattern[j]: j++
    failures[i] = j

// Then match with no backtracking in text:
j = 0
for i = 0 to n-1:
    while j > 0 and text[i] != pattern[j]:
        j = failures[j-1]
    if text[i] == pattern[j]: j++
    if j == m: return i - m + 1
```

**Advantages**:
- **Linear O(n + m)**: scans text once, pattern processed once
- **No backtracking** in text pointer
- **Optimal** among single-pass algorithms

**Disadvantages**:
- Complex to implement correctly; off-by-one errors common
- Failure function construction also O(m)

### Boyer-Moore — O(n/m) average, O(nm) worst case

Preprocesses pattern to enable skipping large chunks of text. Scans pattern right-to-left.

Two heuristics:
- **Bad character rule**: If mismatch on text character c, skip pattern past the rightmost occurrence of c in pattern
- **Good suffix rule**: If a suffix matches, skip based on pattern repetitions

```
Example: text="ushers", pattern="she"
u s h e r s
s h e       ← mismatch on 'u'
    s h e   ← 'u' not in pattern, skip 3
      s h e ← mismatch on 'r'
        s h e ← 'r' not in pattern; 'e' suffix matches at end, skip
```

**Advantages**:
- **Fastest in practice** for long patterns on real text
- Average: O(n / m) — sub-linear, skipping large regions

**Disadvantages**:
- Worst case O(nm) on pathological inputs (pattern "aaab", text "aaaa...a")
- More complex than KMP

### Rabin-Karp — O(n + m) average, O(nm) worst case

Use rolling hash: compute pattern hash and text hash for each window. Compare hashes; verify matches.

```
pattern_hash = hash(pattern)
text_hash = hash(text[0..m-1])
for i = 0 to n-m:
    if text_hash == pattern_hash:
        if text[i..i+m-1] == pattern:  // verify
            return i
    text_hash = roll(text_hash, text[i], text[i+m])
```

**Rolling hash**: $h_{new} = (h_{old} - text[i] \times base^{m-1}) \times base + text[i+m]$

**Advantages**:
- **Multi-pattern matching**: One text scan vs. multiple preprocess/scans for each pattern
- **Useful for 2D patterns**, non-string data

**Disadvantages**:
- Hash collisions cause verification cost; adversarial inputs trigger O(nm)
- Constant factor larger than KMP

### Aho-Corasick — O(n + m + z) where z = number of matches

Extension of KMP for multiple patterns. Builds a **trie** of all patterns, augmented with failure links.

- Scans text once, reports all pattern occurrences
- **Used in**: virus scanning, keyword matching, spell-checking

## Tries and Prefix Trees

A trie is a tree where each node represents a character, paths from root to leaves represent strings.

```
Strings: ["cat", "car", "dog"]
    root
   /    \
  c      d
  |      |
  a      o
 / \     |
t   r    g
```

**Properties**:
- **Insertion/search/delete**: O(m) where m = string length
- **Prefix queries**: Find all strings with prefix p in O(p + |matches|)
- **Space**: O(number of strings × average length) in worst case; often much less due to sharing

**Compression**: 
- **Suffix trie**: Store suffixes instead of prefixes (less common)
- **Radix tree** (compressed trie): Merge single-child nodes to reduce space

## Suffix Structures

### Suffix Array — O(n log n) to build

Array of starting positions of all suffixes sorted lexicographically.

```
Text: "banana$"
Suffixes:
banana$   0
anana$    1
nana$     2
ana$      3
na$       4
a$        5
$         6

Sorted:   $ (6), a$ (5), ana$ (3), anana$ (1), banana$ (0), na$ (4), nana$ (2)
SA = [6, 5, 3, 1, 0, 4, 2]
```

**Pattern matching**: Binary search on suffix array for pattern. All occurrences are contiguous in SA.  
**Time**: O(m log n) where m = pattern length.

**Construction**:
- Naive (merge sort): O(n² log n) — too slow
- SA-IS algorithm: O(n) optimal but complex
- Most use O(n log n) practical algorithms (DC3, prefix doubling)

**Advantages**: Efficient suffix operations like longest repeated substring.

### Suffix Tree — O(n) to build, O(n) space

Compressed trie of all suffixes. Each edge labeled with substring, not single character.

- **Pattern search**: O(m) to find pattern
- **LCS**: Match two texts using joint suffix tree
- **Longest repeated substring**: DFS finding deepest internal node

**Construction**:
- **Ukkonen's algorithm**: O(n) with linear time suffix tree
- Complex but elegant; fewer practitioners know it than suffix arrays

**Disadvantage**: High constant factors; suffix arrays often faster in practice despite worse asymptotic.

## String Hashing

Quick equality check without character-by-character comparison. Uses polynomial rolling hash.

$$h(s) = (s_0 \cdot base^{m-1} + s_1 \cdot base^{m-2} + \ldots + s_{m-1}) \mod p$$

where base (often 31 or 256) and prime p are chosen to minimize collisions.

**Rolling hash**: Compute hash of text[0..m-1], then slide window in O(1) time.

**Risks**: 
- Collision on identical hashes → must verify
- Adversarial hashes can cause hash table slowdown
- Use strong hashing (SipHash, xxHash) if untrusted input

**Applications**: Rabin-Karp, fast substring removal/insertion, deduplication of large strings.

## Edit Distance & Similarity

### Levenshtein Distance — O(mn)

Minimum operations (insert, delete, replace) to transform one string to another. DP algorithm (see dynamic-programming.md).

### Longest Common Subsequence (LCS) — O(mn)

Longest sequence of characters common to both strings (not necessarily contiguous). DP-based.

### Longest Common Substring — O(mn)

Contiguous substring common to both. DP variant:

```
dp[i][j] = length of LCS ending at s1[i] and s2[j]
if s1[i] == s2[j]:
    dp[i][j] = dp[i-1][j-1] + 1
else:
    dp[i][j] = 0
result = max(dp[i][j])
```

## Regular Expression Engines

Regex matching can be compiled to finite automata:

- **DFA (Deterministic Finite Automaton)**: One state transition per input symbol
  - Fast matching O(n)
  - Exponential state blowup on complex patterns (catastrophic backtracking in practice)
  
- **NFA (Nondeterministic FA)**: Multiple possible transitions
  - Slower matching with backtracking
  - Simpler to construct
  - Sane performance on complex patterns

Most engines (Perl, Python, JavaScript) use NFA with backtracking (not the optimal Thompson NFA). Risk: pathological patterns on non-matching input cause exponential time (ReDoS – Regular Expression Denial of Service).

**Engine choice**:
- **Grep/Perl**: NFA-based; powerful but vulnerable to ReDoS
- **Go's regexp**: Thompson NFA; safe but limited features
- **PCRE**: Practical with safeguard features

## Data Compression

### Huffman Coding — O(n log n)

Variable-length prefix codes. Frequent symbols get shorter codes.

1. Build frequency table
2. Construct Huffman tree (prioritize low-frequency pairs)
3. Encode: traverse tree to get codeword

**Advantage**: Optimal prefix code for static frequency.  
**Disadvantage**: Must transmit codebook; not adaptive to changing text.

### LZ77 (Sliding Window) — O(n²) naive, O(n log n) with precomputation

Encode text as references to earlier occurrences (offset, length) plus literals.

```
Text: "abcabc..."
     a b c a b c
         ^^^
Encode as: (3, 3) meaning "repeat 3 chars from 3 positions back"
```

Used in: ZIP, GZIP, deflate.

### LZ78 / LZW — O(n) with hash table

Build dictionary of substrings seen. Output dictionary indices instead of literal text.

Used in: GIF, TIFF, early modem compression (V.42bis).

## Practical Recommendations

| Task | Algorithm |
|------|-----------|
| Single pattern search | Boyer-Moore (best) or KMP (simpler) |
| Multiple patterns | Aho-Corasick trie |
| Find all substrings | Suffix array / tree |
| Similarity/distance | DP (edit distance, LCS) |
| Fast hashing | Rolling hash (Rabin-Karp) |
| Compression | LZ77 + Huffman or adaptive scheme |

---

**See also**: algorithms-dynamic-programming, data-structures-algorithms, computational-thinking
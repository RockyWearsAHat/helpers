# Unicode & Text Processing — The Hard Parts

## The Basics (That Most People Get Wrong)

### Key Terminology
- **Unicode**: A standard that assigns a number (code point) to every character in every writing system
- **Code point**: A number, written as U+0041 (= 'A'). Range: U+0000 to U+10FFFF (~1.1 million possible)
- **Encoding**: How code points are stored as bytes (UTF-8, UTF-16, UTF-32)
- **Grapheme cluster**: What a user perceives as a single "character" (may be multiple code points)

### The Critical Distinction
```
"é" can be represented TWO ways:
  1. U+00E9  (one code point: LATIN SMALL LETTER E WITH ACUTE)
  2. U+0065 U+0301  (two code points: 'e' + COMBINING ACUTE ACCENT)

They LOOK identical but have different byte representations!
This breaks: string comparison, length counting, searching, sorting.
```

## UTF-8 — The Universal Encoding

```
Code point range       Bytes   Bit pattern
U+0000..U+007F         1       0xxxxxxx                (ASCII compatible!)
U+0080..U+07FF         2       110xxxxx 10xxxxxx
U+0800..U+FFFF         3       1110xxxx 10xxxxxx 10xxxxxx
U+10000..U+10FFFF      4       11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
```

**Why UTF-8 won:**
- ASCII-compatible (bytes 0-127 are identical)
- Self-synchronizing (can detect boundaries by looking at any byte)
- No byte-order issues (no BOM needed)
- Compact for Western text (1 byte per ASCII character)
- Variable-length prevents wasted space

**UTF-16 still exists because:**
- Windows NT and Java were designed around UCS-2 (predecessor, only 2 bytes)
- JavaScript strings are UTF-16 internally
- When Unicode exceeded U+FFFF, UTF-16 added surrogate pairs (2 × 16-bit)

## Grapheme Clusters vs Code Points vs Bytes

```python
text = "café"   # 4 graphemes, but...

# Python 3 (counts code points, not graphemes)
len("café")     # Could be 4 or 5, depending on normalization!
len("c\u0061\u0301fe\u0301")  # 6 code points, 4 graphemes

# The flag emoji
flag = "🇺🇸"
len(flag)       # 2 in Python (two code points: U+1F1FA U+1F1F8)
                # But it's ONE grapheme cluster!

# Family emoji
family = "👨‍👩‍👧‍👦"
len(family)     # 11 in Python/JS! (4 person emojis + 3 ZWJ characters)
                # But it's ONE grapheme cluster to the user
```

### What "String Length" Really Means
| Language | `len()`/`.length` counts... | "👨‍👩‍👧‍👦" length |
|----------|---------------------------|-----------------|
| Python 3 | Code points | 11 |
| JavaScript | UTF-16 code units | 11 |
| Rust `str::len()` | Bytes (UTF-8) | 25 |
| Rust `str::chars().count()` | Code points (scalar values) | 11 |
| Go `len(s)` | Bytes | 25 |
| Go `utf8.RuneCount(s)` | Code points (runes) | 11 |
| Swift `s.count` | Grapheme clusters | 1 ✓ |
| Elixir `String.length(s)` | Grapheme clusters | 1 ✓ |

**For user-facing text, grapheme clusters are the right count.** Only Swift and Elixir get this right by default.

## Normalization

### The Four Forms
```
NFC  (Canonical Decomposition + Canonical Composition)
  → Precomposed: é = U+00E9 (single code point)
  → RECOMMENDED FOR STORAGE AND COMPARISON

NFD  (Canonical Decomposition)
  → Decomposed: é = U+0065 + U+0301 (two code points)
  → Used by macOS filenames!

NFKC (Compatibility Decomposition + Canonical Composition)
  → Also maps compatibility characters: ﬃ → ffi, ² → 2
  → RECOMMENDED FOR IDENTIFIERS AND SEARCH

NFKD (Compatibility Decomposition)
  → Decomposes everything
```

### When Normalization Matters
```python
# String comparison WITHOUT normalization
s1 = "café"      # NFC: U+00E9
s2 = "cafe\u0301" # NFD: U+0065 + U+0301
s1 == s2          # False! (even though they look identical)

# With normalization
import unicodedata
unicodedata.normalize("NFC", s1) == unicodedata.normalize("NFC", s2)  # True

# Database: always normalize BEFORE storing
# Search: normalize both query and content
# Filenames: macOS uses NFD, others use NFC → cross-platform bugs!
```

## Collation (Sorting Text)

"Alphabetical order" is locale-dependent:
```
English:  a < b < c ... < z
German:   ä sorts with a (DIN 5007-1) or after az (phonebooks)
Swedish:  ä and ö sort AFTER z
Spanish:  historically, ch and ll were separate letters
Japanese: Multiple sort orders (kana, kanji by reading, kanji by stroke count)
```

```python
# Python: use locale or ICU
import locale
locale.setlocale(locale.LC_ALL, 'de_DE.UTF-8')
sorted(["Äpfel", "Birne", "Apfel"], key=locale.strxfrm)
# → ['Apfel', 'Äpfel', 'Birne']  (ä sorts with a in German)

# For serious collation, use PyICU:
import icu
collator = icu.Collator.createInstance(icu.Locale('sv_SE'))
sorted(["ö", "z", "a"], key=collator.getSortKey)
# → ['a', 'z', 'ö']  (ö after z in Swedish)
```

## Case Folding (It's Not Just `.toLowerCase()`)

```python
# German: 'ß'.upper() → 'SS', but 'SS'.lower() → 'ss' ≠ 'ß'
# Turkish: 'I'.lower() → 'ı' (dotless i), NOT 'i'
#          'i'.upper() → 'İ' (dotted I), NOT 'I'

# For case-insensitive comparison, use casefold():
"straße".casefold() == "strasse".casefold()  # True (Python)

# In DB: use COLLATE for case-insensitive queries
# SELECT * FROM users WHERE name COLLATE NOCASE = 'müller'
```

## Common Unicode Bugs

### 1. Truncation in the Middle of a Character
```python
# WRONG: Truncating UTF-8 bytes can split a multi-byte character
text = "café"
truncated = text.encode('utf-8')[:4]  # Might split the é!
truncated.decode('utf-8')  # UnicodeDecodeError or mojibake

# RIGHT: Truncate by code points or graphemes, then encode
```

### 2. Mojibake (Encoding Mismatch)
```
Stored as UTF-8: "café" → bytes: 63 61 66 C3 A9
Read as Latin-1:                → "cafÃ©"     (classic mojibake)
Read as ASCII:                  → Error or "caf??"
```
Fix: Ensure encoding is specified everywhere (HTTP headers, DB connection, file reading).

### 3. Bidirectional Text Attacks
```
A filename that LOOKS like "invoice.pdf" might actually be "invoice\u202Efdp.exe"
The U+202E (RIGHT-TO-LEFT OVERRIDE) reverses display direction.
Used for social engineering attacks. Filter control characters in user input.
```

### 4. Homoglyph Attacks
```
"apple.com" vs "аpple.com" (Cyrillic 'а' U+0430 vs Latin 'a' U+0061)
They look identical but are different domains!
IDN homograph attacks use this for phishing.
```

### 5. Zero-Width Characters in Code
```
Code that looks correct but contains invisible characters:
  - U+200B ZERO WIDTH SPACE
  - U+FEFF BYTE ORDER MARK (when not at start of file)
  - U+200C ZERO WIDTH NON-JOINER
These can break identifiers, comparisons, and URLs while being invisible in editors.
```

## Practical Guidelines

### For Storage
1. **Always use UTF-8** unless you have a specific reason not to
2. **Normalize to NFC** before storing
3. **Store locale info** alongside text if locale-dependent processing is needed

### For Comparison
1. **Normalize first** (NFC for exact match, NFKC for fuzzy/search)
2. **Use casefold** for case-insensitive comparison (not `.lower()`)
3. **Use ICU/locale-aware collation** for sorting user-visible text

### For Display
1. **Count grapheme clusters** for UI width/truncation, not code points
2. **Use terminal width** (East Asian characters are 2 columns wide)
3. **Test with**: Arabic (RTL), Chinese (multi-byte), emoji (multi-codepoint), combining marks

### For Databases
```sql
-- MySQL: Use utf8mb4 (NOT utf8, which is only 3 bytes = no emoji!)
CREATE TABLE t (name VARCHAR(255)) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

-- PostgreSQL: UTF-8 by default, use ICU collation for proper sorting
CREATE COLLATION german_phonebook (provider = icu, locale = 'de-u-co-phonebk');
```

---

*"There Ain't No Such Thing As Plain Text." — Joel Spolsky. Every string has an encoding. If you don't know what it is, you have a bug.*

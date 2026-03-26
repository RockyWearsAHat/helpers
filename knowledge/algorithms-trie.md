# Trie — Ordered Prefix Tree for Strings

A trie (prefix tree) organizes strings hierarchically by prefix, enabling efficient insertion, search, and prefix-based queries independent of the number of keys. It trades space for speed and naturally supports ordered iteration and pattern matching.

## Core Concept

A trie is a tree where:
- Each node represents a single character
- The path from root to a node spells a prefix
- A node marks "end of word" if any string in the set ends there
- Children of a node are indexed by outgoing characters

**Structure**: Root node has 26 children (lowercase letters) or 256 (bytes). Each child is also a node (or null).

**Mental model**: Like a dictionary where words are grouped by their first letter, then within each group by second letter, etc. Seeking a word means following edges down the tree.

**Example**: Trie storing {cat, car, dog}
```
         root
        / | \
       c  d  ...
      / \  \
     a   a  o
    / \   \  \
   t   r   g
   *   *   *
```

Asterisk marks word endings.

## Basic Operations

### Search: Find if Word Exists

```
search(word):
    node = root
    for char in word:
        if node.children[char] not in node:
            return false  (prefix not found)
        node = node.children[char]
    return node.is_word  (check if word ends here)
```

**Complexity**: $O(m)$ where $m$ is word length. Independent of number of keys.

### Insert: Add Word

```
insert(word):
    node = root
    for char in word:
        if node.children[char] is null:
            node.children[char] = new Node()
        node = node.children[char]
    node.is_word = true
```

**Complexity**: $O(m)$.

### Delete: Remove Word

```
delete(word):
    _delete(root, word, 0)

_delete(node, word, idx):
    if idx == len(word):
        if not node.is_word: return false  (word not found)
        node.is_word = false
        return node.children.empty()  (can prune node if no children)
    
    char = word[idx]
    if char not in node.children: return false
    child = node.children[char]
    should_prune = _delete(child, word, idx + 1)
    if should_prune:
        del node.children[char]
    return should_prune and node.children.empty() and not node.is_word
```

**Complexity**: $O(m)$.

## Prefix-Based Queries

Trie's key advantage: prefix queries run in $O(m + k)$ where $m$ is prefix length and $k$ is number of results.

### Autocomplete: Find Words with Prefix

```
autocomplete(prefix):
    node = root
    for char in prefix:
        if char not in node.children:
            return []
    results = []
    dfs(node, prefix, results)  (collect all words under node)
    return results
```

**Use case**: Search suggestions, IP prefix matching, filename completion.

### Dictionary-based Spell Checking

Find all words similar to a misspelled word by traversing the trie with allowed mutations:
- Insert character: branch to node's children
- Delete character: skip character in trie
- Substitute character: try all sibling nodes

## Compressed Trie (Radix Tree, Patricia Tree)

Standard tries are space-inefficient when words share long prefixes with no branching.

**Example**: Words {test, testing, tested} create nodes for t→e→s→t→i→n→g. If no branching, compress into single edge labeled "testing".

**Compressed trie**: Edges are labeled with strings, not single characters.

```
Standard trie:  t—e—s—t—i—n—g
                    *      *  *

Compressed:     t-e-s-t → i-n-g
                    *  /   *
                        (ing)
```

**Complexity**:
- Search: $O(m)$ character comparisons, not node traversals
- Space: $O(k)$ nodes where $k$ is number of keys (vs. $O(\text{tree height}) \approx O(|S|)$ for standard trie, $S$ is total string length)

**Tradeoff**: Simpler to implement than compressed tries; standard tries fine for most problems.

## Ternary Search Tree

Hybrid of trie and binary search. Each node has three children: left (smaller), middle (match), right (larger).

```
           m
          /|\
         / | \
        d  m  z
       / \
      a   s
```

**Advantage**: Space $O(k)$ like compressed trie but simpler to code than Patricia tree.

**Complexity**: $O(m \log k)$ for search (ternary search on alphabet + path depth).

**Use case**: When alphabet is large or keys are numeric (quick-sort ternary trees).

## IP Routing: Longest Prefix Match

Routers maintain Forwarding Information Base (FIB) of IP prefixes to next-hop mappings. Packet forwarding requires finding the longest matching prefix.

**Example**: FIB = {10.0.0.0/8 → X, 10.1.0.0/16 → Y, 10.1.2.0/24 → Z}. Packet to 10.1.2.5 matches all three; forward to Z (longest prefix).

**Trie solution**: Store IP prefixes as binary trie. Search for packet IP, track last marked node (end-of-prefix).

```
longest_prefix_match(ip):
    node = root
    result = null
    for bit in ip:
        if node.is_prefix:
            result = node.next_hop
        node = node.children[bit]
        if not node: break
    return result
```

**Complexity**: $O(\text{prefix length})$ — typically 32 bits for IPv4, 128 for IPv6.

**Alternative**: (Generalized) suffix trees; however, tries are simpler and industry-standard for routing.

## T9 Predictive Text (Mobile Keyboards)

Old phone keyboards mapped multiple letters to each key: 2=abc, 3=def, ..., 9=wxyz.

User presses key sequence; system predicts word.

**Solution**: Trie indexed by phone key sequence instead of letters.

```
Search for word: extract key sequence from each letter, find matching subtrie.
```

Query "[2][2][2]" → find all words spelled by pressing 2 three times: {aaa, aab, aba, abb, baa, bab, bba, bbb}.

Filter by frequency (most common words first).

## Memory Optimization

Standard trie uses hash map or array per node: $O(\sigma)$ space per node where $\sigma$ is alphabet size.

---

**Tradeoffs for reducing memory**:

1. **Hash map of children**: Use only as many children as needed. Slower lookups; saves memory for sparse tries.
2. **Bit-packed children**: Store bitmask of which children exist; array of pointers. Less wasted space; one lookup per child.
3. **Suffix compression**: Share common suffixes via DAG (directed acyclic graph). Complex to implement; saves space.
4. **Prefix tree with single-child compression**: Collapse chains of single children into strings (compressed trie).

## Concurrent Tries (Ctrie)

Mutable trie for concurrent environments. Standard tries require locking the entire structure for updates.

**Ctrie principle**: Share immutable subtrees; update-in-place at branching nodes using atomic operations.

**Structure**: Each node has an indirection level (contract node) that can be atomically replaced to redirect to new subtrees.

**Complexity**: Lock-free reads; updates acquire locks on $O(\log n)$ branching nodes.

**Use case**: Concurrent dictionaries, in-memory databases requiring lock-free reads.

## When to Use Tries

**Ideal for**:
- Lexicon-based problems (spell-checking, autocomplete, games like Scrabble)
- IP routing, URL matching, longest prefix matching
- Dictionary encoding (compress repeated prefixes)
- Telephone directory (T9, SMS prediction)
- Finding words matching a pattern with wildcards

**Advantages**:
- Search, insert, delete in $O(m)$ independent of number of keys
- Natural support for prefix queries, ordered traversal
- Space-efficient with compression
- Concurrent-friendly (Ctrie design)

**Disadvantages**:
- Space overhead: standard trie typically uses more memory than hash table (26 pointers per node)
- Cache-unfriendly: pointer-chasing through tree
- Slower than hash table for single-key lookups (in practice; theory says $O(m)$ vs. $O(1)$)

**Alternatives**:
- **Hash table**: Faster point lookups; no prefix queries
- **Sorted array + binary search**: Memory-efficient; prefix queries require iteration
- **Suffix tree**: For pattern matching and string processing (complex; rarely needed)

## Implementation Considerations

**Edge cases**:
- Empty string: mark root as word ending
- Repeated characters: standard trie handles naturally
- Deletion of non-existent word: guard against underflow

**Testing**:
- Insert, then search
- Search before insert (should fail)
- Delete non-existent word (should handle gracefully)
- Autocomplete with no matches (empty result)
- Very long words, large alphabets

See also: [algorithms-string.md](algorithms-string.md), [algorithms-trees.md](algorithms-trees.md), [data-structures-algorithms.md](data-structures-algorithms.md)
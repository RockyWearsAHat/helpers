//! The `HLM1` binary container every machine-cache lint artifact is stored in (LINTER.md,
//! "Save"). A trained model is mostly hypervectors — already uniform random bits — so the
//! format stores them verbatim and compresses only what has entropy to give:
//!
//! ```text
//! magic "HLM1" · format u8 · kind u8 · stamp (varint len + UTF-8)
//! raw_len varint · deflated_len varint · inflated_len varint
//! RAW stream  — hypervectors + integer arrays, little-endian, verbatim (bulk-copy decode)
//! DATA stream — structure + text (varints, strings), DEFLATE-compressed (miniz_oxide)
//! ```
//!
//! The stamp is provenance for PREFIX PROBES ([`probe`]) — staleness and the registry
//! publisher read it from the first bytes without touching the payload; decoding proper reads
//! every field from the streams, so the stamp can never diverge from the artifact (both are
//! written from the same struct in one save). Decode is bounds-checked and total: any
//! malformed byte yields `None`, which every caller already treats as "artifact absent —
//! reacquire" (the same contract `serde_json::from_str(..).ok()` gave the JSON era).

use crate::lint_ai::Hv;

/// Container magic — "Helpers Lint Model", format family 1.
pub const MAGIC: [u8; 4] = *b"HLM1";
/// Bumped on any layout change; a reader never guesses across versions.
pub const FORMAT: u8 = 1;

/// Artifact kinds — one per machine-cache file family, so a file can never decode as the
/// wrong struct even if renamed.
pub mod kind {
    pub const MODULE: u8 = 1;
    pub const OVERLAY: u8 = 2;
    pub const LEARNED: u8 = 3;
    pub const ENGLISH: u8 = 4;
    pub const POLARITY: u8 = 5;
    /// Per-project verdict replay cache (LINTER.md, "Warm runs replay per-file verdicts").
    pub const VERDICT: u8 = 6;
    /// Per-project whole-report replay container (LINTER.md, "An unchanged project replays
    /// the whole report").
    pub const REPLAY: u8 = 7;
    /// The machine-global extension map (learned claims folded from every saved module).
    pub const EXTMAP: u8 = 8;
}

/// Hard ceiling on a declared inflated DATA stream (defense in depth at load — a crafted
/// header must not allocate the machine away). Far above any real artifact: the largest
/// learned catalog observed is ~64 MB of JSON, whose text inflates to less.
const MAX_INFLATED: u64 = 1 << 31;

/// Types that write themselves into an [`Enc`] and read themselves back from a [`Dec`].
/// Implemented beside each struct's own definition so private fields stay private.
pub trait Bin: Sized {
    fn enc(&self, e: &mut Enc);
    fn dec(d: &mut Dec) -> Option<Self>;
}

// ── Encoder ───────────────────────────────────────────────────────────────────

/// Two-stream artifact encoder. Structure/text goes to the DATA stream (deflated at
/// [`Enc::finish`]); hypervectors and integer arrays go to the RAW stream verbatim.
#[derive(Default)]
pub struct Enc {
    data: Vec<u8>,
    raw: Vec<u8>,
}

impl Enc {
    pub fn new() -> Enc {
        Enc::default()
    }

    /// LEB128 varint — the one integer wire form for lengths and counts.
    pub fn u(&mut self, mut v: u64) {
        loop {
            let byte = (v & 0x7f) as u8;
            v >>= 7;
            if v == 0 {
                self.data.push(byte);
                return;
            }
            self.data.push(byte | 0x80);
        }
    }

    /// Fixed 8-byte little-endian — for hash-valued u64s, where a varint only loses.
    pub fn fixed_u64(&mut self, v: u64) {
        self.data.extend_from_slice(&v.to_le_bytes());
    }

    pub fn boolean(&mut self, v: bool) {
        self.data.push(v as u8);
    }

    /// Varint length + UTF-8 bytes.
    pub fn str(&mut self, s: &str) {
        self.u(s.len() as u64);
        self.data.extend_from_slice(s.as_bytes());
    }

    /// One hypervector into the RAW stream — 1 KiB verbatim, decode is a bulk copy.
    pub fn hv(&mut self, hv: &Hv) {
        for w in hv.as_words() {
            self.raw.extend_from_slice(&w.to_le_bytes());
        }
    }

    /// A u64 array into the RAW stream (count in DATA) — token seeds, headword sets.
    pub fn raw_u64s(&mut self, values: &[u64]) {
        self.u(values.len() as u64);
        for v in values {
            self.raw.extend_from_slice(&v.to_le_bytes());
        }
    }

    /// A u32 array into the RAW stream (count in DATA) — frequency counts.
    pub fn raw_u32s(&mut self, values: &[u32]) {
        self.u(values.len() as u64);
        for v in values {
            self.raw.extend_from_slice(&v.to_le_bytes());
        }
    }

    /// Seal the container: header (with `stamp` readable by prefix probe), RAW stream
    /// verbatim, DATA stream deflated.
    pub fn finish(self, kind: u8, stamp: &str) -> Vec<u8> {
        let deflated = miniz_oxide::deflate::compress_to_vec(&self.data, 8);
        let mut out = Vec::with_capacity(32 + stamp.len() + self.raw.len() + deflated.len());
        out.extend_from_slice(&MAGIC);
        out.push(FORMAT);
        out.push(kind);
        varint_into(&mut out, stamp.len() as u64);
        out.extend_from_slice(stamp.as_bytes());
        varint_into(&mut out, self.raw.len() as u64);
        varint_into(&mut out, deflated.len() as u64);
        varint_into(&mut out, self.data.len() as u64);
        out.extend_from_slice(&self.raw);
        out.extend_from_slice(&deflated);
        out
    }
}

/// LEB128 varint straight into a buffer — header fields, written outside any [`Enc`].
fn varint_into(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let byte = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

/// LEB128 varint out of a buffer at `pos`; advances `pos`. `None` on truncation/overflow.
fn varint_from(bytes: &[u8], pos: &mut usize) -> Option<u64> {
    let mut v: u64 = 0;
    for shift in (0..64).step_by(7) {
        let byte = *bytes.get(*pos)?;
        *pos += 1;
        v |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some(v);
        }
    }
    None
}

// ── Decoder ───────────────────────────────────────────────────────────────────

/// Two-stream artifact decoder over an opened container. Mirrors [`Enc`] exactly: reads must
/// come in encode order, and every read is bounds-checked (`None`, never a panic).
pub struct Dec {
    data: Vec<u8>,
    dp: usize,
    raw: Vec<u8>,
    rp: usize,
}

/// The container's probe-readable identity: artifact kind and provenance stamp, read from the
/// first bytes only — no payload allocation, no inflation.
pub struct Header {
    pub kind: u8,
    pub stamp: String,
}

/// Read a container's header from its leading bytes (a 512-byte file prefix suffices for any
/// real stamp). `None` when the bytes are not an `HLM1` container of the current format.
pub fn probe(bytes: &[u8]) -> Option<Header> {
    let mut pos = 0usize;
    if bytes.get(..4)? != MAGIC {
        return None;
    }
    pos += 4;
    if *bytes.get(pos)? != FORMAT {
        return None;
    }
    let kind = *bytes.get(pos + 1)?;
    pos += 2;
    let stamp_len = varint_from(bytes, &mut pos)? as usize;
    let stamp = std::str::from_utf8(bytes.get(pos..pos + stamp_len)?).ok()?.to_string();
    Some(Header { kind, stamp })
}

impl Dec {
    /// Open a full container of the expected `kind`: verify the header, take the RAW stream,
    /// inflate the DATA stream. Returns the stamp alongside the ready decoder.
    pub fn open(bytes: &[u8], kind: u8) -> Option<(String, Dec)> {
        let header = probe(bytes)?;
        if header.kind != kind {
            return None;
        }
        // Re-walk past the header the probe validated to find the stream offsets.
        let mut pos = 6usize;
        let stamp_len = varint_from(bytes, &mut pos)? as usize;
        pos += stamp_len;
        let raw_len = varint_from(bytes, &mut pos)? as usize;
        let deflated_len = varint_from(bytes, &mut pos)? as usize;
        let inflated_len = varint_from(bytes, &mut pos)?;
        if inflated_len > MAX_INFLATED {
            return None;
        }
        let raw = bytes.get(pos..pos + raw_len)?.to_vec();
        pos += raw_len;
        let deflated = bytes.get(pos..pos + deflated_len)?;
        let data = miniz_oxide::inflate::decompress_to_vec_with_limit(deflated, inflated_len as usize)
            .ok()?;
        Some((header.stamp, Dec { data, dp: 0, raw, rp: 0 }))
    }

    pub fn u(&mut self) -> Option<u64> {
        varint_from(&self.data, &mut self.dp)
    }

    pub fn fixed_u64(&mut self) -> Option<u64> {
        let b = self.data.get(self.dp..self.dp + 8)?;
        self.dp += 8;
        Some(u64::from_le_bytes(b.try_into().ok()?))
    }

    pub fn boolean(&mut self) -> Option<bool> {
        let b = *self.data.get(self.dp)?;
        self.dp += 1;
        Some(b != 0)
    }

    pub fn str(&mut self) -> Option<String> {
        let len = self.u()? as usize;
        let b = self.data.get(self.dp..self.dp + len)?;
        self.dp += len;
        let s = std::str::from_utf8(b).ok()?.to_string();
        Some(s)
    }

    pub fn hv(&mut self) -> Option<Hv> {
        let words = self.take_raw_u64s(crate::lint_ai::DIM / 64)?;
        Some(Hv::from_words(&words))
    }

    pub fn raw_u64s(&mut self) -> Option<Vec<u64>> {
        let n = self.u()? as usize;
        self.take_raw_u64s(n)
    }

    pub fn raw_u32s(&mut self) -> Option<Vec<u32>> {
        let n = self.u()? as usize;
        let bytes = self.raw.get(self.rp..self.rp + n * 4)?;
        self.rp += n * 4;
        Some(bytes.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().expect("chunk is 4"))).collect())
    }

    /// `n` u64s straight off the RAW stream — the bulk-copy primitive behind `hv`/`raw_u64s`.
    fn take_raw_u64s(&mut self, n: usize) -> Option<Vec<u64>> {
        let bytes = self.raw.get(self.rp..self.rp + n * 8)?;
        self.rp += n * 8;
        Some(bytes.chunks_exact(8).map(|c| u64::from_le_bytes(c.try_into().expect("chunk is 8"))).collect())
    }
}

// ── Bin for the common shapes ─────────────────────────────────────────────────

impl Bin for String {
    fn enc(&self, e: &mut Enc) {
        e.str(self);
    }
    fn dec(d: &mut Dec) -> Option<String> {
        d.str()
    }
}

impl Bin for bool {
    fn enc(&self, e: &mut Enc) {
        e.boolean(*self);
    }
    fn dec(d: &mut Dec) -> Option<bool> {
        d.boolean()
    }
}

impl Bin for u32 {
    fn enc(&self, e: &mut Enc) {
        e.u(u64::from(*self));
    }
    fn dec(d: &mut Dec) -> Option<u32> {
        u32::try_from(d.u()?).ok()
    }
}

impl Bin for u64 {
    fn enc(&self, e: &mut Enc) {
        e.u(*self);
    }
    fn dec(d: &mut Dec) -> Option<u64> {
        d.u()
    }
}

impl Bin for usize {
    fn enc(&self, e: &mut Enc) {
        e.u(*self as u64);
    }
    fn dec(d: &mut Dec) -> Option<usize> {
        usize::try_from(d.u()?).ok()
    }
}

impl<T: Bin> Bin for Vec<T> {
    fn enc(&self, e: &mut Enc) {
        e.u(self.len() as u64);
        for item in self {
            item.enc(e);
        }
    }
    fn dec(d: &mut Dec) -> Option<Vec<T>> {
        let n = d.u()? as usize;
        let mut out = Vec::with_capacity(n.min(1 << 20));
        for _ in 0..n {
            out.push(T::dec(d)?);
        }
        Some(out)
    }
}

impl<T: Bin> Bin for Option<T> {
    fn enc(&self, e: &mut Enc) {
        match self {
            Some(v) => {
                e.boolean(true);
                v.enc(e);
            }
            None => e.boolean(false),
        }
    }
    fn dec(d: &mut Dec) -> Option<Option<T>> {
        Some(if d.boolean()? { Some(T::dec(d)?) } else { None })
    }
}

impl<V: Bin> Bin for std::collections::BTreeMap<String, V> {
    fn enc(&self, e: &mut Enc) {
        e.u(self.len() as u64);
        for (k, v) in self {
            e.str(k);
            v.enc(e);
        }
    }
    fn dec(d: &mut Dec) -> Option<Self> {
        let n = d.u()? as usize;
        let mut out = Self::new();
        for _ in 0..n {
            let k = d.str()?;
            out.insert(k, V::dec(d)?);
        }
        Some(out)
    }
}

/// A set of token seeds with exactly ONE live representation: a FROZEN sorted array (decode
/// is a bulk copy off the RAW stream, membership is a binary search) or a HOT hash set once
/// something inserts. The lint path only ever asks membership, so a loaded set stays frozen
/// for its whole life — this is what keeps the English brain's 78k headwords at
/// microsecond-load (LINTER.md, "Save").
#[derive(Clone, Default)]
pub struct SeedSet {
    /// Sorted, deduplicated seeds (frozen representation; empty while `hot` is live).
    frozen: Vec<u64>,
    /// The building representation (empty while the frozen array is live).
    hot: std::collections::HashSet<u64>,
}

impl SeedSet {
    pub fn contains(&self, seed: u64) -> bool {
        if !self.hot.is_empty() {
            return self.hot.contains(&seed);
        }
        self.frozen.binary_search(&seed).is_ok()
    }

    /// Add a seed — thaws a frozen set first (building happens at setup, never at lint).
    pub fn insert(&mut self, seed: u64) {
        if !self.frozen.is_empty() {
            self.hot = self.frozen.drain(..).collect();
        }
        self.hot.insert(seed);
    }

    pub fn len(&self) -> usize {
        self.frozen.len() + self.hot.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frozen.is_empty() && self.hot.is_empty()
    }

    /// The seeds as one sorted array — the encode-side view, whatever the representation.
    fn sorted(&self) -> Vec<u64> {
        if self.hot.is_empty() {
            return self.frozen.clone();
        }
        let mut v: Vec<u64> = self.hot.iter().copied().collect();
        v.sort_unstable();
        v
    }

    /// Adopt an array as the frozen representation, sorting/deduplicating if needed.
    fn frozen_from(mut seeds: Vec<u64>) -> SeedSet {
        if !seeds.windows(2).all(|w| w[0] < w[1]) {
            seeds.sort_unstable();
            seeds.dedup();
        }
        SeedSet { frozen: seeds, hot: std::collections::HashSet::new() }
    }
}

/// JSON view (committed bootstraps, legacy stores): a sorted array in both directions —
/// deterministic committed diffs, and any legacy order reads back into the frozen form.
impl serde::Serialize for SeedSet {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&self.sorted(), s)
    }
}

impl<'de> serde::Deserialize<'de> for SeedSet {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<SeedSet, D::Error> {
        Ok(SeedSet::frozen_from(Vec::<u64>::deserialize(d)?))
    }
}

impl Bin for SeedSet {
    fn enc(&self, e: &mut Enc) {
        e.raw_u64s(&self.sorted());
    }
    fn dec(d: &mut Dec) -> Option<SeedSet> {
        Some(SeedSet::frozen_from(d.raw_u64s()?))
    }
}

/// A `HashSet<u64>` rides the RAW stream sorted — random seeds don't deflate, and sorted
/// arrays bulk-copy back.
impl Bin for std::collections::HashSet<u64> {
    fn enc(&self, e: &mut Enc) {
        let mut sorted: Vec<u64> = self.iter().copied().collect();
        sorted.sort_unstable();
        e.raw_u64s(&sorted);
    }
    fn dec(d: &mut Dec) -> Option<Self> {
        Some(d.raw_u64s()?.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every primitive survives the container round trip, and the probe reads the stamp from
    /// the file prefix alone.
    #[test]
    fn container_round_trips_and_probes() {
        let mut e = Enc::new();
        e.u(0);
        e.u(300);
        e.u(u64::MAX);
        e.fixed_u64(0xDEAD_BEEF_CAFE_F00D);
        e.boolean(true);
        e.str("naïve — utf8 ✓");
        let hv = Hv::random(42);
        e.hv(&hv);
        e.raw_u64s(&[1, 2, 3]);
        e.raw_u32s(&[7, 8]);
        let bytes = e.finish(kind::MODULE, "v1\u{1f}docs-v42");

        let header = probe(&bytes[..64.min(bytes.len())]).expect("prefix probes");
        assert_eq!(header.kind, kind::MODULE);
        assert_eq!(header.stamp, "v1\u{1f}docs-v42");

        assert!(Dec::open(&bytes, kind::OVERLAY).is_none(), "wrong kind must not decode");
        let (stamp, mut d) = Dec::open(&bytes, kind::MODULE).expect("opens");
        assert_eq!(stamp, "v1\u{1f}docs-v42");
        assert_eq!(d.u(), Some(0));
        assert_eq!(d.u(), Some(300));
        assert_eq!(d.u(), Some(u64::MAX));
        assert_eq!(d.fixed_u64(), Some(0xDEAD_BEEF_CAFE_F00D));
        assert_eq!(d.boolean(), Some(true));
        assert_eq!(d.str().as_deref(), Some("naïve — utf8 ✓"));
        assert_eq!(d.hv(), Some(hv));
        assert_eq!(d.raw_u64s(), Some(vec![1, 2, 3]));
        assert_eq!(d.raw_u32s(), Some(vec![7, 8]));
    }

    /// Truncated or bit-flipped containers decode to `None`, never panic — the load-time
    /// defense the JSON era got from serde for free.
    #[test]
    fn corruption_is_none_never_panic() {
        let mut e = Enc::new();
        e.str("payload");
        e.hv(&Hv::random(7));
        let bytes = e.finish(kind::LEARNED, "stamp");
        for cut in 0..bytes.len() {
            let _ = Dec::open(&bytes[..cut], kind::LEARNED); // must not panic
        }
        let mut flipped = bytes.clone();
        flipped[4] ^= 0xFF; // format byte
        assert!(Dec::open(&flipped, kind::LEARNED).is_none());
        assert!(probe(b"not a container").is_none());
    }
}

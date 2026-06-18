//! TF-IDF primitives shared by the session-memory and knowledge indexes.
//! These mirror the arithmetic in `lib/mcp-session-memory.js` and
//! `lib/mcp-knowledge-index.js` exactly (smoothed IDF, L2-normalized vectors,
//! dot-product cosine over the smaller vector).

use std::collections::HashMap;

pub type Vector = HashMap<String, f64>;

/// Term frequency: count / total_tokens (total floored at 1).
pub fn compute_tf(tokens: &[String]) -> Vector {
    let mut freq: HashMap<String, usize> = HashMap::new();
    for t in tokens {
        *freq.entry(t.clone()).or_insert(0) += 1;
    }
    let total = tokens.len().max(1) as f64;
    freq.into_iter()
        .map(|(term, count)| (term, count as f64 / total))
        .collect()
}

/// Smoothed inverse document frequency: `ln((N+1)/(df+1)) + 1`.
pub fn compute_idf(all_tfs: &[Vector], doc_count: usize) -> Vector {
    let mut df: HashMap<String, usize> = HashMap::new();
    for tf in all_tfs {
        for term in tf.keys() {
            *df.entry(term.clone()).or_insert(0) += 1;
        }
    }
    let n = doc_count as f64;
    df.into_iter()
        .map(|(term, count)| (term, ((n + 1.0) / (count as f64 + 1.0)).ln() + 1.0))
        .collect()
}

/// L2-normalize a sparse vector. A zero-magnitude vector maps to empty.
pub fn l2_normalize(vec: &Vector) -> Vector {
    let mag = vec.values().map(|v| v * v).sum::<f64>().sqrt();
    if mag == 0.0 {
        return Vector::new();
    }
    vec.iter().map(|(t, v)| (t.clone(), v / mag)).collect()
}

/// Cosine similarity of two pre-normalized vectors = their dot product.
/// Iterates the smaller map, matching the JS implementation.
pub fn cosine_sim(a: &Vector, b: &Vector) -> f64 {
    let (smaller, larger) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    let mut sum = 0.0;
    for (term, val) in smaller {
        if let Some(other) = larger.get(term) {
            sum += val * other;
        }
    }
    sum
}

/// Multiply a TF vector by IDF weights, keeping only terms present in `idf`.
pub fn tfidf(tf: &Vector, idf: &Vector) -> Vector {
    let mut out = Vector::new();
    for (term, tf_val) in tf {
        if let Some(idf_val) = idf.get(term) {
            out.insert(term.clone(), tf_val * idf_val);
        }
    }
    out
}

/// Sort `(term, weight)` pairs by descending weight and keep the top `n`. Ties
/// fall back to term order so the selection is deterministic.
pub fn top_n(vec: &Vector, n: usize) -> Vec<(String, f64)> {
    let mut pairs: Vec<(String, f64)> = vec.iter().map(|(k, v)| (k.clone(), *v)).collect();
    pairs.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    pairs.truncate(n);
    pairs
}

//! Weighted PageRank over the file reference graph — files that many important
//! files reference rank highest, giving the repo-map a relevance ordering.

/// Compute PageRank for `n` nodes given weighted directed `edges`
/// (`(from, to, weight)`). Dangling nodes (no outgoing edges) spread their rank
/// uniformly. Returns a rank per node index.
pub fn pagerank(n: usize, edges: &[(usize, usize, f64)], damping: f64, iters: usize) -> Vec<f64> {
    if n == 0 {
        return Vec::new();
    }
    let mut out_weight = vec![0f64; n];
    let mut incoming: Vec<Vec<(usize, f64)>> = vec![Vec::new(); n];
    for &(from, to, w) in edges {
        out_weight[from] += w;
        incoming[to].push((from, w));
    }

    let nf = n as f64;
    let base = (1.0 - damping) / nf;
    let mut rank = vec![1.0 / nf; n];
    for _ in 0..iters {
        let dangling: f64 = (0..n)
            .filter(|&i| out_weight[i] == 0.0)
            .map(|i| rank[i])
            .sum();
        let mut next = vec![0f64; n];
        for (i, slot) in next.iter_mut().enumerate() {
            let mut s = 0.0;
            for &(j, w) in &incoming[i] {
                if out_weight[j] > 0.0 {
                    s += rank[j] * w / out_weight[j];
                }
            }
            *slot = base + damping * (s + dangling / nf);
        }
        rank = next;
    }
    rank
}

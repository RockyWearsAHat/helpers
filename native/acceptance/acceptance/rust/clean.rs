//! Modern, alive Rust — expected findings: ZERO. trim_left and connect live only in prose.
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

/// Joins, trims, and compares the modern way.
pub fn modern(parts: &[&str], s: &str, cell: &AtomicUsize) -> String {
    let joined = parts.join(", ");
    let t = s.trim_start().trim_end();
    let t2 = t.trim_start_matches('x').trim_end_matches('x');
    let _ = cell.compare_exchange(1, 2, Ordering::SeqCst, Ordering::SeqCst);
    let _d = Duration::from_millis(100);
    format!("{joined}{t2}")
}

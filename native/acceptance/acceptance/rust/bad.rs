//! Planted: every enforced rust construct, one per line.
use std::sync::atomic::{AtomicUsize, Ordering};
fn probe(parts: Vec<&str>, s: &str, a: f64, b: f64, cell: &AtomicUsize, cvar: &std::sync::Condvar, guard: std::sync::MutexGuard<bool>) {
let joined = parts.connect(", ");
let piece = unsafe { s.slice_unchecked(0, 2) };
let t1 = s.trim_left();
let t2 = s.trim_right();
let t3 = s.trim_left_matches('x');
let t4 = s.trim_right_matches('x');
let d = a.abs_sub(b);
let r = cvar.wait_timeout_ms(guard, 100);
let old = cell.compare_and_swap(1, 2, Ordering::SeqCst);
}

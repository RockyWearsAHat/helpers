//! helpers-native — native Rust implementations of the hot Helpers MCP
//! tools. The Node MCP daemon shells out to the `helpers-native` binary for these
//! tools; everything is exposed here as a library so it can be unit-tested
//! without spawning a process.

pub mod cli;
pub mod doc_crawler;
pub mod embed;
pub mod git;
pub mod gitcli;
pub mod hv_batch;
pub mod index;
pub mod knowledge;
pub mod lint_ai;
pub mod lint_char;
pub mod lint_docs;
pub mod lint_english;
pub mod lint_lang;
pub mod lint_feedback;
pub mod lint_graph;
pub mod lint_match;
pub mod lint_probe;
pub mod lint_read;
pub mod lint_socrawl;
pub mod lint_trace;
pub mod lint_sign;
pub mod lint_toolchain;
pub mod lint_train;
pub mod lint_replay;
pub mod lint_kq;
pub mod lint_checkers;
pub mod lint_codec;
pub mod lint_corroborate;
pub mod lint_html_layer;
pub mod lint_ism;
pub mod lint_selftest;
pub mod linter;
pub mod mcp;
pub mod memory;
pub mod proc;
pub mod proto;
pub mod registry;
pub mod tfidf;
pub mod tools;
pub mod util;

/// Serializes tests that mutate PROCESS environment (`HOME`, `HELPERS_LINT_MODELS`): the
/// harness runs test modules in parallel threads, and an env mutation mid-flight corrupts
/// any concurrent test resolving paths through the same variable (measured: a models
/// redirect made the signing round-trip flake).
#[cfg(test)]
pub(crate) fn test_env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

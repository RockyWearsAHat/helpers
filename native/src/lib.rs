//! helpers-native — native Rust implementations of the hot Helpers MCP
//! tools. The Node MCP daemon shells out to the `helpers-native` binary for these
//! tools; everything is exposed here as a library so it can be unit-tested
//! without spawning a process.

pub mod cli;
pub mod embed;
pub mod git;
pub mod gitcli;
pub mod index;
pub mod knowledge;
pub mod lint_ai;
pub mod lint_ast;
pub mod lint_bpe;
pub mod lint_brain;
pub mod lint_moe;
pub mod lint_checkers;
#[cfg(feature = "gpu")]
pub mod lint_gpu;
#[cfg(feature = "gpu")]
pub mod lint_brain_gpu;
pub mod lint_index;
pub mod lint_metrics;
pub mod lint_sig;
pub mod lint_signature;
pub mod mcp;
pub mod memory;
pub mod reviewer;
pub mod proc;
pub mod proto;
pub mod registry;
pub mod tfidf;
pub mod tools;
pub mod util;

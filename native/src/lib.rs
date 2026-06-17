//! gsh-native — native Rust implementations of the hot Git Shell Helpers MCP
//! tools. The Node MCP daemon shells out to the `gsh-native` binary for these
//! tools; everything is exposed here as a library so it can be unit-tested
//! without spawning a process.

pub mod git;
pub mod gitcli;
pub mod index;
pub mod knowledge;
pub mod proc;
pub mod proto;
pub mod registry;
pub mod tfidf;
pub mod tools;
pub mod util;

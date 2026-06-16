//! MCP wire helpers shared by every native tool.
//!
//! Each `gsh-native call <tool>` invocation prints exactly one JSON line:
//!   success -> {"content":[{"type":"text","text":"..."}]}
//!   failure -> {"error":{"message":"..."}}
//! The Node bridge (`lib/mcp-native.js`) parses that line either way.

use serde::Serialize;
use serde_json::json;

/// A single MCP content block. Only text blocks are produced by native tools.
#[derive(Serialize, Clone)]
pub struct Content {
    #[serde(rename = "type")]
    pub kind: String,
    pub text: String,
}

/// Build a text content block.
pub fn text<S: Into<String>>(s: S) -> Content {
    Content {
        kind: "text".into(),
        text: s.into(),
    }
}

/// The result every tool handler returns: an MCP content list, or an error
/// message that becomes a JSON-RPC error on the Node side.
pub type ToolResult = Result<Vec<Content>, String>;

/// Print the success envelope.
pub fn emit_content(content: &[Content]) {
    let out = json!({ "content": content });
    println!(
        "{}",
        serde_json::to_string(&out).expect("serialize content")
    );
}

/// Print the error envelope.
pub fn emit_error(message: &str) {
    let out = json!({ "error": { "message": message } });
    println!("{}", serde_json::to_string(&out).expect("serialize error"));
}

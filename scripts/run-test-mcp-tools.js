#!/usr/bin/env node
// Wrapper that clears cache and runs test
delete require.cache[require.resolve("./test-mcp-tools.js")];
require("./test-mcp-tools.js");

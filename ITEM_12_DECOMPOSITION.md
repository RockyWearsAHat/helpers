# Item 12 Decomposition: Capability Gate Verification

## Context
- Failed dispatch: `dispatch team team-general-0: team team-general-0 already dispatched`
- Unsolved capability gates requiring investigation:
  - cap-lib-tests: suspect
  - cap-full-build: suspect
  - cap-binary-runs: unknown
  - cap-mcp-answers: unknown
  - cap-helpers-doctor: suspect

## Proposed Sub-Items (to be added via dx_append)

### Sub-Item 1: Verify library test suite
**Format:** `- [ ] [scout] [ask: charter-bin] [cap: cap-lib-tests] Verify lib tests: run 'cd native && cargo test --lib' in sandbox; verify exit code 0 and all tests pass`
- **Files to check:** native/src/lib.rs, native/tests/
- **Command:** `cd native && cargo test --lib 2>&1`
- **Verifiable outcome:** Exit code 0, all test results show PASS, no FAIL messages
- **Gate advanced:** cap-lib-tests

### Sub-Item 2: Verify binary execution  
**Format:** `- [ ] [scout] [ask: charter-bin] [cap: cap-binary-runs] Verify binary: run 'helpers-native --version'; verify output contains version number and exit code 0`
- **Files to check:** target/release/helpers-native (or cargo build output location)
- **Command:** `helpers-native --version 2>&1`
- **Verifiable outcome:** Exit code 0, output contains version string (e.g., "helpers-native 0.x.x")
- **Gate advanced:** cap-binary-runs

### Sub-Item 3: Verify helpers doctor check
**Format:** `- [ ] [scout] [ask: charter-bin] [cap: cap-helpers-doctor] Run doctor check: 'helpers doctor'; verify no fatal errors reported`
- **Files to check:** helpers CLI availability after build
- **Command:** `helpers doctor 2>&1`
- **Verifiable outcome:** Exit code 0, output contains "OK" or "PASS" indicators, no "FATAL" or "ERROR" labels
- **Gate advanced:** cap-helpers-doctor

### Sub-Item 4: Verify MCP schema response
**Format:** `- [ ] [scout] [ask: charter-bin] [cap: cap-mcp-answers] Verify schema: run 'helpers-native call list_workspace_tools 2>&1 | head -20'; verify JSON and exit code 0`
- **Files to check:** native/src/tools/ for MCP tool definitions
- **Command:** `helpers-native call list_workspace_tools 2>&1`
- **Verifiable outcome:** Exit code 0, output is valid JSON containing "type" field (e.g., "type":"tool" or "type":"text")
- **Gate advanced:** cap-mcp-answers

## Original Item to Tick
Once sub-items are created via dx_append, tick the original unchecked item that triggered this decomposition.

## Needs a Human

### dx Schema Version Incompatibility
**Blocker preventing decomposition completion:** The index.dx document was written by dx schema version 5, but the current dx MCP tools only understand schema version 4. This prevents using `dx_append` to write the sub-items to the worklist.

**Status:** 
- Report filed to dx: `report-cbd37d68` at https://rockywearsahat.com/report?dx
- Workaround: Sub-items documented in this file (ITEM_12_DECOMPOSITION.md) awaiting integration

**Required action:**
- Upgrade dx tool to schema version 5, OR
- Downgrade index.dx document to schema version 4

Once schema compatibility is restored, use dx_append to add these sub-items to the now-worklist block, then tick the parent item's checkbox.

## Path Forward
1. Human upgrades dx tool or downgrades document schema
2. Run: `dx_append D:\SARA\Desktop\bin\index.dx now-worklist` with the sub-items from this document
3. Tick the original unchecked item in the worklist
4. Commit with message: "item 12: decompose dispatch failure into mechanical verification sub-items"

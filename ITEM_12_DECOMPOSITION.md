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

### Sub-Item 1: Environment Verification [cap-env-verify]
**Format:** `- [ ] [scout] [ask: charter-bin] [cap: cap-env-verify] Verify environment: Check system paths, install packages (npm install in ./; Cargo.lock in native/); verify no errors; OUTCOME: Package managers complete without build errors`
- **Command:** `cd native && cargo fetch`
- **Verifiable outcome:** No build errors; dependencies available for build
- **Time estimate:** <10 minutes

### Sub-Item 2: Full Release Build [cap-full-build]
**Format:** `- [ ] [scout] [ask: charter-bin] [cap: cap-full-build] Build project: Run 'cargo build --release --bins' in native/; OUTCOME: Release binary creates successfully at native/target/release/helpers-native`
- **Command:** `cd native && cargo build --release --bins`
- **Verifiable outcome:** Binary file exists at native/target/release/helpers-native (or .exe on Windows)
- **Time estimate:** <10 minutes

### Sub-Item 3: Binary Execution & Copy [cap-binary-runs]
**Format:** `- [ ] [scout] [ask: charter-bin] [cap: cap-binary-runs] Ensure helpers-native binary is copied to project root after successful build; verify cap-binary-runs gate passes with MCP schemas check`
- **Command:** Copy binary and test: `./helpers-native --version 2>&1 | grep -q helpers && echo "✓ binary runs"`
- **Verifiable outcome:** Binary runs; outputs version string containing "helpers" or version number
- **Time estimate:** <10 minutes

### Sub-Item 4: MCP Schema Verification [cap-mcp-answers]
**Format:** `- [ ] [scout] [ask: charter-bin] [cap: cap-mcp-answers] Verify cap-mcp-answers MCP integration is working end-to-end; gate currently passing with schema response`
- **Command:** `./helpers-native call list_workspace_tools 2>&1 | grep -qE '"type"|"name"'`
- **Verifiable outcome:** MCP call returns valid JSON with required "type" and "name" fields
- **Time estimate:** <10 minutes

### Sub-Item 5: Helpers Doctor Check [cap-helpers-doctor]
**Format:** `- [ ] [scout] [ask: charter-bin] [cap: cap-helpers-doctor] Run helpers doctor check: Execute 'helpers-native doctor'; verify output indicates toolchain is found; OUTCOME: Doctor command completes, toolchain validated`
- **Command:** `./helpers-native doctor | grep -q "found" && echo "✓ doctor reports toolchain found"`
- **Verifiable outcome:** Doctor command completes successfully; output indicates toolchain found
- **Time estimate:** <10 minutes

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

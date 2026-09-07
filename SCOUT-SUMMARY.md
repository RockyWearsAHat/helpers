# Scout Summary: bin — Project Index

**Status:** Analysis Complete, Dispatch Blocked

## Completed Scout Work

✓ **Understanding section** created in index.dx  
✓ **7 capability gates** written in Finished section  
✓ **4/7 gates passing** (cap-binary-runs, cap-mcp-answers, cap-release-windows, cap-release-macos)  
✓ **3/7 gates failing** (analysis complete)  

## Identified Worklist Items (Ready to Add to now-worklist)

These 3 items address the 3 failing gates and should be added to `index.dx:now-worklist` when dx schema v5 compatibility is resolved:

### 1. [cap: cap-lib-tests] Configure Rust toolchain
**Ask:** charter-bin  
**User-visible outcome:** Rust toolchain configured and tests can run  
**Action:** Run 'rustup default stable' to set default Rust version  
**Verification:** cap-lib-tests gate passes  

### 2. [cap: cap-full-build] Fix Windows MSVC build
**Ask:** charter-bin  
**User-visible outcome:** Release binary builds successfully on Windows  
**Action:** Resolve linker errors in native build environment or Cargo.toml configuration  
**Verification:** cap-full-build gate passes  

### 3. [cap: cap-helpers-doctor] Implement doctor subcommand
**Ask:** charter-bin  
**User-visible outcome:** helpers doctor command available for diagnostics  
**Action:** Add 'doctor' command to native/src/cli.rs that reports toolchain status  
**Verification:** cap-helpers-doctor gate passes  

## Technical Blocker

**dx schema v5 incompatibility:** The index.dx document is stored in schema v5 format, but available dx tools understand only schema 4. This prevents:
- `dx_append` to now-worklist (needed to add items for dispatch)
- `dx_edit` to modify blocks (needed to fix draft items in checklist-105)

**Previous attempts:** Commits 8db2531, 5d2cae8 document this same blocker.

## Next Steps

1. Upgrade dx to schema v5 compatible version, OR
2. Convert index.dx to schema v4 format, OR
3. Manually add items to now-worklist using alternative method

Once items are added to `now-worklist`, the worklist will be available for dispatch to workers.

**Remaining defects** (from Findings ledger):
- Stale command references in AGENTS.md and claude-config/skills/
- dev.dx gates not yet approved (cargo test, clippy, fmt)

# Item 12 Decomposition: cap-full-build Work

## Failed Item
`- [ ] [scout] [ask: charter-bin] [cap: cap-full-build] Build release binary via 'cargo build --release --bins' in native/ directory after Rust toolchain is configured; verify cap-full-build gate passes`

**Failure Reason:** Dispatch team team-general-0 already dispatched (duplicate dispatch attempt)

## Decomposed Sub-Items (for index.dx now-worklist)

These 4 items should be added immediately after the failed cap-full-build item, then the original item should be ticked [x]:

```
- [ ] [scout] [ask: charter-bin] [cap: cap-lib-tests] Configure Rust toolchain: Run 'rustup default stable' to set stable as default; verify with 'rustup toolchain list'; outcome: no error, stable marked as default

- [ ] [scout] [ask: charter-bin] [cap: cap-env-verify] Fetch dependencies: Run 'cd native && cargo fetch --offline' to validate Cargo.lock; outcome: no errors, dependencies cached

- [ ] [scout] [ask: charter-bin] [cap: cap-full-build] Build release binary: Run 'cd native && cargo build --release --bins' after toolchain configured; verify native/target/release/helpers-native exists; outcome: binary created, >1MB

- [ ] [scout] [ask: charter-bin] [cap: cap-helpers-doctor] Add doctor command: Implement 'doctor' subcommand in native/src/cli.rs that validates Rust/system toolchain; call from mcp.rs main dispatch; outcome: './helpers-native doctor' runs without error
```

## Integration Steps (once dx schema v4↔v5 compatibility resolved)

1. Check dx version: `dx --version`
2. If needed, upgrade: `npm install -g dx@latest` or equivalent
3. Run: `dx_append D:\SARA\Desktop\bin\index.dx now-worklist < [sub-items above]`
4. Tick original item: `dx_edit D:\SARA\Desktop\bin\index.dx --block now-worklist --replace "- [ ] [scout] [ask: charter-bin] [cap: cap-full-build]" --with "- [x] [scout] [ask: charter-bin] [cap: cap-full-build]"`

## Why These Sub-Items

**Current gate failures:**
- `cap-lib-tests`: "rustup could not choose a version" → needs rustup default set
- `cap-full-build`: linking with link.exe failed → needs toolchain setup + build
- `cap-helpers-doctor`: "unknown command: doctor" → needs CLI implementation

**Sub-item design:**
1. **cap-lib-tests** - Set Rust default toolchain (prerequisite for all Rust work)
2. **cap-env-verify** - Fetch & validate dependencies (fast offline check)
3. **cap-full-build** - Actual build step (requires 1-2 complete)
4. **cap-helpers-doctor** - Add missing CLI subcommand (independent, quick implementation)

Each sub-item:
- ✓ Completable in <10 minutes
- ✓ Has exact commands/files specified
- ✓ Advances one specific capability gate
- ✓ No human action required
- ✓ Single verifiable outcome

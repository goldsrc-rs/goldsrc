### Summary

- High-level context and architectural motivation for this change.

### Changes

- **Core**: Structural changes, memory layouts, algorithms.
- **Adapters**: Integrations, bridges, protocol conversions.
- **Tooling / DX**: Tests, benchmarks, CI workflows.

### Verification

- [ ] `cargo fmt --all -- --check` passes cleanly.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` reports 0 warnings.
- [ ] `cargo nextest run --all-targets` passes cleanly.
- [ ] `cargo test --doc` passes cleanly.
- [ ] No emojis present in code, commits, or documentation.
- [ ] All unsafe blocks include explicit `// SAFETY:` justifications.

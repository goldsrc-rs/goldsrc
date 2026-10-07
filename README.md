# GoldSrc

<div align="center">

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![CI](https://github.com/goldsrc-rs/goldsrc/actions/workflows/ci.yml/badge.svg)](https://github.com/goldsrc-rs/goldsrc/actions)
[![Rust Version](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org)

**Next-generation GoldSrc technical platform, facade meta-crate, developer CLI, and architecture hub.**

[Architecture](ARCHITECTURE.md) • [Getting Started](#getting-started) • [CLI Tooling](#developer-cli-grs) • [Organization Repositories](#organization-ecosystem)

</div>

---

## Overview

`goldsrc` is the technical umbrella repository and primary developer entrypoint for the **GoldSrc.rs** ecosystem. It provides:

1. **Facade Meta-Crate (`goldsrc`)**: The unified crate published to crates.io that re-exports all essential plugin development primitives and preludes.
2. **Developer CLI (`grs`)**: The official command-line tool to inspect, validate, profile, and manage GoldSrc plugins and runtime artifacts.
3. **Technical Architecture Hub**: Canonical specifications for plugin lifecycle, memory layouts, capability security, and zero-allocation pipelines.

---

## Organization Ecosystem

| Repository | Scope | Audience |
| :--- | :--- | :--- |
| **[`goldsrc`](https://github.com/goldsrc-rs/goldsrc)** | Umbrella facade crate, Developer CLI (`grs`), and architectural specs | Plugin Authors & Integrators |
| **[`goldsrc-runtime`](https://github.com/goldsrc-rs/goldsrc-runtime)** | High-performance host engine, Wasmtime component runtime, and Metamod/Standalone backends | Server Administrators & DevOps |
| **[`goldsrc-sdk`](https://github.com/goldsrc-rs/goldsrc-sdk)** | Pure guest SDK, raw FFI bindings, SPI contracts, and proc-macros | Plugin Developers |
| **[`goldsrc-plugins-standard`](https://github.com/goldsrc-rs/goldsrc-plugins-standard)** | Official standard suite of production WASM plugins (`moderation`, `chat_director`, `map_manager`, etc.) | Server Administrators |
| **[`goldsrc-template-plugin-rust`](https://github.com/goldsrc-rs/goldsrc-template-plugin-rust)** | Canonical template repository for creating new plugins with `cargo generate` | Plugin Developers |

---

## Getting Started

### Using the Facade Crate

Add `goldsrc` to your plugin's `Cargo.toml`:

```toml
[dependencies]
goldsrc = { git = "https://github.com/goldsrc-rs/goldsrc.git", branch = "dev" }
```

Write your plugin using the standard prelude:

```rust
use goldsrc::prelude::*;

#[plugin]
pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn on_load(&mut self) -> Result<()> {
        log_info!("MyPlugin loaded successfully!");
        Ok(())
    }
}
```

---

## Developer CLI (`grs`)

Install or run the developer CLI tool:

```bash
cargo install --path crates/grs
```

### Inspecting Plugins

```bash
# View plugin metadata and configuration schema
grs pl info moderation

# Inspect plugin memory allocation telemetry
grs pl memory
```

---

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

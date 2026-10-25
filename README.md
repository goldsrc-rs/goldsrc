# GoldSrc

<div align="center">

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![CI](https://github.com/goldsrc-rs/goldsrc/actions/workflows/ci.yml/badge.svg)](https://github.com/goldsrc-rs/goldsrc/actions)

**Next-generation GoldSrc technical platform, architectural blueprint, and ecosystem hub.**

[Architecture Blueprint](ECOSYSTEM_BLUEPRINT.md) • [Organization Repositories](#organization-ecosystem)

</div>

---

## Overview

`goldsrc` is the primary architecture hub and developer entrypoint for the **GoldSrc.rs** ecosystem. It provides:

1. **Strategic Architecture Blueprint (`ECOSYSTEM_BLUEPRINT.md`)**: Canonical master specification defining the multi-layered platform vision from v0.20.0 to v2.0.0.
2. **Ecosystem Coordination**: Organization governance, contribution guidelines, and security policies across all GoldSrc.rs repositories.

---

## Organization Ecosystem

| Repository | Scope | Audience |
| :--- | :--- | :--- |
| **[`goldsrc`](https://github.com/goldsrc-rs/goldsrc)** | Strategic architecture hub, blueprint, and ecosystem governance | Architecture & Community |
| **[`goldsrc-sdk`](https://github.com/goldsrc-rs/goldsrc-sdk)** | Canonical SDK workspace, facade meta-crate (`goldsrc`), raw FFI (`goldsrc-sys`), SPI contracts (`goldsrc-spi`), API types (`goldsrc-api`), and proc-macros (`goldsrc-macros`) | Plugin Developers |
| **[`goldsrc-cli`](https://github.com/goldsrc-rs/goldsrc-cli)** | Official developer CLI tool (`grs`) to build, inspect, validate, and manage plugins and artifacts | Plugin Authors & Integrators |
| **[`goldsrc-runtime`](https://github.com/goldsrc-rs/goldsrc-runtime)** | High-performance host engine, Wasmtime component runtime, Metamod/Standalone backends, and modular services | Server Administrators & DevOps |
| **[`goldsrc-plugins-standard`](https://github.com/goldsrc-rs/goldsrc-plugins-standard)** | Official standard suite of production WASM plugins (`moderation`, `chat_director`, `map_manager`, etc.) | Server Administrators |
| **[`goldsrc-sdk-cstrike`](https://github.com/goldsrc-rs/goldsrc-sdk-cstrike)** | Counter-Strike 1.6 game domain crate, weapons, equipment, ReGameDLL integration | CS 1.6 Plugin Authors |
| **[`goldsrc-template-plugin-rust`](https://github.com/goldsrc-rs/goldsrc-template-plugin-rust)** | Canonical template repository for creating new plugins with `cargo generate` | Plugin Developers |

---

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

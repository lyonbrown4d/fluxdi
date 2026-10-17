# FluxDI

[English](./README.md) · [简体中文](./README.zh-CN.md)

Framework-independent dependency injection for Rust, with an explicit compile-time
assembly API and a dynamic runtime container. Optional Axum and Actix integrations
provide HTTP request scopes.

This checkout is unreleased and requires Rust 1.99.1. Read the
[crate guide](./fluxdi/README.md), [English book](./docs/src/introduction.md), or
[Chinese book](./docs/zh-CN/src/introduction.md).

```bash
cargo test --workspace --all-features --locked
cargo run -p fluxdi --example static_module --features macros --locked
cargo run -p fluxdi --example static_composition_async --features macros,async-factory --locked
cargo run --manifest-path examples/seaorm-sqlite/Cargo.toml --locked
```

Direct dependency versions are centralized in `[workspace.dependencies]`. The SeaORM
example has a separate workspace and lockfile to isolate its native SQLite binding
from the latest rusqlite used by the complex example.

Build both documentation languages and their shared landing page:

```bash
cargo install mdbook --locked
sh docs/build.sh
python3 -m http.server --bind 127.0.0.1 --directory docs/book 8000
```

Open `http://localhost:8000`. The language links keep the current chapter.
Every English chapter must have a Chinese counterpart for the build to pass.
See the [dependency audit](./docs/src/dependency-audit.md) for verified results
and the remaining upstream advisory.
Release notes are available in the [changelog](./CHANGELOG.md).

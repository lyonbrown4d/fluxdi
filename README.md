# FluxDI

[English](./README.md) · [简体中文](./README.zh-CN.md)

Framework-independent dependency injection for Rust, with an explicit compile-time
assembly API and a dynamic runtime container. Optional Axum and Actix integrations
provide HTTP request scopes.

FluxDI 2.0.0 requires Rust 1.99.1 or newer. Read the
[crate guide](./fluxdi/README.md), [English book](./docs/src/introduction.md), or
[Chinese book](./docs/zh-CN/src/introduction.md).

Install the core with its default synchronous features:

```toml
[dependencies]
fluxdi = "2.0.0"
```

Enable `features = ["macros"]` for `Injectable` and `static_module!`.
See the [2.0 migration notes](./CHANGELOG.md#200---2026-10-18) before upgrading from 1.x.
For local development, run from the repository root:

```bash
cargo test --workspace --all-features --locked
cargo run -p fluxdi --example static_module --features macros --locked
cargo run -p fluxdi --example static_composition_async --features macros,async-factory --locked
cargo run -p fluxdi --example static_conditionals --features macros --locked
cargo run -p fluxdi --example static_conditionals --no-default-features --features macros --locked
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
Maintainers can follow the [local release guide](./docs/src/local-release.md).

# Changelog

All notable changes to FluxDI are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `Injectable` support for type/lifetime/const generics and existing `where` clauses, named fields via `#[inject(name = "...")]`, optional dependencies, and set dependencies
- Generated `provider(scope)` and `with_injected_dependencies(provider)` methods to attach inferred graph hints; original constructors remain supported
- Fallible optional resolution helpers for sync/async unnamed and named bindings, preserving initialization errors
- Optional graph hints and `DependencyCardinality::Optional`; absent optional targets are valid, while registered targets participate in cycle detection
- Separate first-resolution, persistent-worker cached-resolution, and thread-startup/control benchmarks
- Fallible `Provider::try_root`, `try_singleton`, `try_scoped`, `try_transient`, and matching async constructors, returning `Result<Shared<T>, Error>`
- `Injectable::try_from_injector` for recoverable dependency resolution; the existing `from_injector` constructor still panics on errors
- `Error::factory_failed` and `ErrorKind::FactoryFailed` for application initialization errors; dependency errors retain their original kind and message
- Performance docs (`docs/src/performance.md`) with feature impact and benchmark commands
- **Epic H (continued)**: `BootstrapOptions::with_parallel_start()` for concurrent module `on_start` execution
- Bootstrap lifecycle error aggregation: `Error::bootstrap_aggregate()` when multiple modules fail during parallel startup
- Rollback policy: on bootstrap failure, `on_stop` is called on already-started modules (sequential and parallel)
- Graceful shutdown timeout: with `ShutdownOptions::with_timeout`, all modules' `on_stop` are attempted within the time budget; no partial abort

### Changed

- Downstream exhaustive matches on `DependencyCardinality` must handle `Optional`; derived types must reserve the generated `provider` and `with_injected_dependencies` method names
- The extended macro requires the updated core APIs. Before publishing, coordinate a major release of both crates so older core releases with a `^1.x` macro dependency cannot select incompatible generated code
- Concurrent benchmark groups now explicitly distinguish persistent workers from per-iteration thread startup; old combined measurements should not be compared as cached-resolution timings
- Minimum supported Rust and pinned development/CI baseline are now 1.99.1 across all workspace packages
- Adding `ErrorKind::FactoryFailed` requires downstream exhaustive matches on `ErrorKind` to handle the new variant
- Fallible factories cache only successful instances; failed initialization releases resource permits and allows a later caller or waiting caller to retry
- `Provider::singleton` now caches per calling injector context; clones share that context, while children and new runtime scopes have their own singleton cache
- Scoped instances belong to the nearest runtime scope; nested scopes isolate their scoped cache and ordinary children share the nearest scope

### Fixed

- Concurrent initialization, async cycle detection, and cancellation/panic cleanup across named, set, and ordinary bindings
- Lifecycle rollback after configuration/startup failures and cleanup after canceled bootstrap
- Async decorators, shared sync/async resource limits, and compilation without default features

## [1.2.2] - 2026-03-21

### Added

- **Epic H (partial)**: `BootstrapOptions` and `ShutdownOptions` for lifecycle control
- Optional bootstrap and shutdown timeouts via `lifecycle` feature
- `Application::bootstrap_with_options()` and `Application::shutdown_with_options()`
- Structured shutdown error aggregation: when multiple modules fail during `on_stop()`, all modules are attempted and a single aggregated error is returned
- `Error::shutdown_aggregate()` for combining multiple shutdown failures
- Production Patterns documentation (`docs/src/production-patterns.md`) for long-running servers
- **Epic I**: `Provider::with_decorator()` for service wrapping (logging, caching, retry)
- `examples/decorator` demonstrating logging and caching decorators
- `docs/src/decorators.md` documentation
- `bench_resolve_with_decorator` and `bench_resolve_decorator_baseline` benchmarks for decorator overhead

## [1.2.1] - 2026-03-21

### Changed

- Unified documentation version references to 1.2.1 across README, mdBook, and examples
- Fixed encoding issues in `fluxdi/README.md` (corrupted emoji in section headers)

### Added

- **Epic A completion**: Macro vs manual provider parity documentation
- New `examples/injectable-macro` demonstrating `#[derive(Injectable)]` as an alternative to manual provider closures
- "Manual vs Macro Parity" section in `docs/derive-macros.md` with side-by-side comparison
- Injectable macro example entry in main README and `docs/examples.md`

### Fixed

- Section headers `## Examples`, `## Usage Guide`, `### Architectural Patterns`, `### Developer Experience` display correctly

## [1.2.0]

### Added

- `Module::configure()` as the preferred registration hook (returns `Result<(), Error>`)
- `#[derive(Injectable)]` via `fluxdi-macros` crate (macros feature)
- Named bindings (`provide_named`, `resolve_named`) for multiple implementations of one trait
- Multi-binding (`provide_into_set`, `resolve_all`) for plugin pipelines
- Graph tooling (`dependency_graph()`, `validate_graph()`, DOT/Mermaid export)
- Scoped context (`create_scope()`, `Provider::scoped`)
- Testing override API (`try_override_provider`)
- Error suggestions for `ServiceNotProvided`, `AsyncFactoryRequiresAsyncResolve`, circular dependency
- Axum and Actix-web integration extractors (`Resolved<T>`)
- Module lifecycle hooks (`on_start`, `on_stop`) with async support
- `Application::bootstrap()` and `Application::shutdown()` for async lifecycle

## [1.1.0]

### Added

- Initial release with core DI functionality
- Transient, singleton (root), and module-scoped lifetimes
- Injector, Provider, Shared types
- Module system with imports
- Circular dependency detection
- Resource limits (Policy, Limits)
- Async factory support
- Metrics and Prometheus export
- Logging and tracing integration

[Unreleased]: https://github.com/DaiYuANg/fluxdi/compare/v1.2.2...HEAD
[1.2.2]: https://github.com/DaiYuANg/fluxdi/compare/v1.2.1...v1.2.2
[1.2.1]: https://github.com/DaiYuANg/fluxdi/compare/v1.2.0...v1.2.1
[1.2.0]: https://github.com/DaiYuANg/fluxdi/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/DaiYuANg/fluxdi/releases/tag/v1.1.0

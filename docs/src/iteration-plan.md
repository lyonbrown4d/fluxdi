# Iteration Plan

Each batch is implemented and validated before the next begins. Validation statements
below refer to the checks actually executed for that batch.

## Scope contract

`Provider::singleton()` caches in the calling injector context. Clones share the
context; children and new scopes have separate singleton caches. `Provider::root()`
shares across the injector tree. `Provider::scoped()` caches at the nearest runtime
scope: nested scopes isolate values, while ordinary children reuse the nearest scope.

## Batch 1: correctness

Completed and pushed as `2e61040`.

- Fixed async cycle-detection false positives, thread migration and initialization wait cycles.
- Coalesced concurrent first initialization and allowed retry after cancellation or panic.
- Located named, set and ordinary caches by effective provider identity and scope.
- Fixed bootstrap failure, timeout and cancellation cleanup, and legacy async module hooks.
- Fixed missing async decorators, separate sync/async creation quotas and no-default-feature builds.

Validation: 224 workspace tests, strict workspace and minimal-feature Clippy, relevant
feature combinations, formatting and local documentation links passed. mdBook was
not installed for this batch. Two old complex-example async-trait lint allowances
were scoped to generated code, keeping the strict workspace warning gate.

## Batch 2: fallible construction

Completed and pushed as `87612ae`.

- Added sync and async `Provider::try_*` factories for all four scopes.
- Added `Injectable::try_from_injector`, retaining the original panic-on-error constructor.
- Preserved factory error kinds/messages; cached successful values only, with retry and permit release.
- Added `ErrorKind::FactoryFailed`; exhaustive downstream matches need a new arm.
- Updated the Rust requirement and development/CI baseline to 1.99.1.

Validation: 234 workspace tests, eight feature combinations, strict Clippy,
Rust API docs, formatting and local links passed. mdBook was not installed.
See [Fallible Factories](./fallible-factories.md).

## Batch 3: derive and graph hints

Completed and pushed as `8658e87`.

- Preserved type/lifetime/const generics, where clauses and hygienic generated identifiers.
- Added required, named, optional and set dependencies; optional bindings preserve factory errors.
- Generated `provider(scope)` and `with_injected_dependencies(provider)` graph hints.
- Added optional graph cardinality and cycle checks for present optional targets.
- Added macro feature combinations and separated cold, cached and persistent-worker benchmarks.

Validation: 244 workspace tests, ten feature combinations, strict workspace and local
macro Clippy, Rust docs, mdBook 0.5.4, example execution, formatting and local links
passed. Short release benchmark results are historical local measurements, not
portable performance guarantees. See [Derive Macros](./derive-macros.md) and
[Performance](./performance.md).

`DependencyCardinality::Optional` affects exhaustive matches. Generated method names
must be reserved. Coordinate a major release of the core and macros before publishing
the new APIs, so older core versions cannot select incompatible macro-generated code.

## Batch 4: static assembly, HTTP scopes and maintenance

Completed and pushed as `842e331`. Local functional validation passed. The main dependency
advisory check retains one upstream finding, as detailed below.

- Added explicit synchronous `static_module!` graphs, typed factory wiring and external inputs.
- Reject missing/duplicate keys, constructor cycles and root captures of contextual bindings at compile time.
- Reused runtime initialization and cache semantics through typed provider handles and context bridging.
- Added opt-in Axum/Actix request middleware, shared extractors and cancellation/error cleanup.
- Updated all direct dependencies to the latest stable snapshot, including Syn 3, SeaORM 2 and the new OTel APIs.
- Centralized main-workspace versions; isolated the SeaORM SQLite native dependency graph with its own lockfile and CI checks.
- Added corresponding English/Chinese books, chapter-preserving language navigation and bilingual READMEs.
- Installed cargo-deny 0.20.2 and retained its visible upstream h2 finding; see [Dependency Audit](./dependency-audit.md).

Validation: 283 workspace tests and all 12 feature combinations passed. Strict
workspace and standalone-example Clippy, Rust API documentation, both mdBook
languages, chapter/code-example correspondence, local Markdown links and formatting
passed. The static module, in-memory CRUD and standalone SeaORM SQLite examples ran
successfully. Browser verification confirmed that language switching preserves the
current chapter. The standalone dependency audit passed all four checks; the main
workspace passed bans, licenses and sources but still fails advisories for h2.

The core remains framework-independent; HTTP adapters use optional features. The
implementation reuses syn/quote, native framework middleware and Tower test utilities.
Rust stable remains 1.99.1. The two obsolete async-trait lint allowances were removed
after adopting async-trait 0.1.92.

At the end of this batch, the static API covered one synchronous graph. Module imports, conditions,
automatic discovery, implicit optional/set selection, static async factories and
lifecycle ordering remain future work. Container freezing needs a registration and
override boundary; lifecycle validation needs module ownership and startup metadata.
Memory and performance work requires measurements across context/scope hierarchies.

Request scopes cover the downstream processing future. Streaming responses and
background tasks must retain the handles they need. See [Static Modules](./static-modules.md)
and [Web Integrations](./web-integrations.md).

## Batch 5: module imports and static async factories

Completed and pushed as `7b6b01e`. Local functional validation passed.

- Added explicit module imports with selected exports, local aliases and nested reexports.
- Reused typed provider identities; constructor inputs follow combined import/external declaration order.
- Preserved scope safety across module boundaries through compile-time metadata checks, including transient bridges.
- Added static async factories and transitive async getters for synchronous factories with async dependencies.
- Reused initialization, cancellation, retry and concurrency behavior in local and thread-safe modes.
- Kept new constructors synchronous and lazy; importing creates a fresh context, with explicit `in_context` for source-tree reuse.
- Updated English and Chinese documentation and added the `static_composition_async` example.

Validation: 300 workspace tests and all 12 feature combinations passed. Strict
workspace/all-target Clippy, standalone SeaORM Clippy, Rust API documentation,
both mdBook languages, paired code examples, local Markdown links and formatting
passed. The async composition and standalone SeaORM examples ran successfully.
A separate library/application fixture verified cross-crate imports without
requiring matching feature names in the downstream crates; seven negative fixtures
confirmed the intended import, async, scope and feature diagnostics. The dependency
graph is unchanged; the upstream h2 finding recorded in Batch 4 remains tracked in
[Dependency Audit](./dependency-audit.md).

The core remains framework-independent and has no new runtime dependencies. Async
graphs require `macros,async-factory`; sync-only modules still work without the
async feature. Imports require explicit source async declarations, checked against
generated metadata. At the end of Batch 5, conditions, discovery, implicit optional/set
selection and lifecycle ordering remained future work; cyclic module composition was unsupported.

## Batch 6: conditional bindings and static optional/collection injection

Completed; local functional validation passed.

- Added native `cfg`/`cfg_attr` filtering before semantic parsing and graph validation, covering modules, bindings, externals and imports.
- Delegated predicates to the defining crate's compiler configuration; grouped identical predicates to reduce recursive expansion.
- Added `optional(key)` for optional factory arguments, preserving errors from present bindings.
- Added local named groups via `#[set(group)]` and ordered `all(group)` factory arguments; empty groups are valid.
- Preserved type coercions, provider identity, scope caches and explicit import-site membership.
- Included only active optional and collection edges in cycle, lifetime and async checks, with correct diagnostic argument positions.
- Updated both documentation languages and added `static_conditionals` with debug-enabled/disabled execution examples.

Validation: 323 workspace tests and all 12 feature combinations passed. Strict
workspace/all-target Clippy,
standalone SeaORM Clippy, Rust API documentation, both mdBook languages, paired
code examples, local Markdown links and formatting passed. The conditional example
ran with debug enabled and disabled; the standalone SeaORM example ran successfully.
A separate library/application fixture passed four configurations and confirmed six
expected diagnostics for conditional imports, required/duplicate keys, expanded
collection cycles and root captures. The dependency graph is unchanged; the upstream
h2 finding remains tracked in [Dependency Audit](./dependency-audit.md).

Factory arguments remain explicit; automatic type-based candidate discovery is not
implemented. Unknown optional keys and collection groups are intentionally empty
requests. Importing an unavailable source export still fails compilation unless its
import is disabled. Conditions may change constructor parameters and inferred async
getters. Many independent conditions still consume Rust's native macro recursion
limit. Lifecycle ordering and cyclic module composition remain unsupported.

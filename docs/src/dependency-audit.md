# Dependency Audit

Snapshot: 2026-10-17. Tool: `cargo-deny 0.20.2`. Rust's official stable manifest
confirms 1.99.1. The following versions describe this checkout; consumers resolve
and audit their own application dependency graphs.

```bash
cargo install cargo-deny --locked
cargo deny --all-features check
cargo deny --manifest-path examples/seaorm-sqlite/Cargo.toml --all-features check
```

## Latest stable direct dependencies

All direct registry dependencies were checked against crates.io's latest non-yanked
stable versions. The main workspace centralizes versions in `[workspace.dependencies]`.
Cargo updates transitive dependencies within upstream constraints; older major lines
can remain where an upstream crate still requires them.

| Dependency | Version |
| --- | --- |
| actix-web | 4.17.0 |
| async-trait | 0.1.92 |
| axum | 0.8.9 |
| criterion | 0.8.2 |
| dashmap | 6.2.1 |
| futures | 0.3.34 |
| opentelemetry | 0.33.1 |
| opentelemetry_sdk | 0.33.1 |
| proc-macro2 | 1.0.107 |
| quote | 1.0.47 |
| rand | 0.10.3 |
| rusqlite | 0.40.2 |
| sea-orm | 2.0.4 |
| serde | 1.0.229 |
| serde_json | 1.0.154 |
| syn | 3.0.9 |
| tokio | 1.53.3 |
| tower | 0.5.3 |
| tower-http | 0.7.1 |
| tracing | 0.1.44 |
| tracing-opentelemetry | 0.34.0 |
| tracing-subscriber | 0.3.23 |

The latest rusqlite requires libsqlite3-sys 0.38, while SeaORM 2's SQLx 0.9 requires
an earlier native binding range. Cargo permits only one `links = "sqlite3"` package
in a dependency graph. The SeaORM example therefore has an independent workspace
and tracked lockfile. CI validates both graphs; neither integration was downgraded.

## Results

The initial audit reported 12 advisories. Dependency upgrades cleared 11 and removed
the yanked Spin version. The unmaintained proc-macro-error2 dependency was removed
through the SeaORM/sea-bae updates. No advisory ignores were added.

- Main workspace: bans, licenses and sources pass; advisories still fail with one h2 finding.
- Standalone SeaORM example: advisories, bans, licenses and sources all pass.

Multiple-version and missing manifest SPDX-license-field warnings remain warnings
under the existing policy. The workflow's existing non-blocking advisory-job setting
is unchanged; the full main-workspace audit is not green.

## Remaining upstream constraint

[RUSTSEC-2026-0258](https://rustsec.org/advisories/RUSTSEC-2026-0258.html) matches:

```text
fluxdi (actix feature) / examples/actix
  -> actix-web 4.17.0
     -> actix-http 3.19.0
        -> h2 0.3.27
```

The advisory describes unlimited empty DATA-frame queuing when streams are not
actively drained, potentially causing memory growth or panic. RustSec classifies
it as low severity and lists `h2 >= 0.4.16` as patched. Local evidence is the audit
and dependency constraint; an attack against Actix was not reproduced here.

`cargo update -p h2` cannot select a patched version within Actix's current range.
The upstream fix is [hyperium/h2#935](https://github.com/hyperium/h2/pull/935).
Protocol capabilities and audit rules remain unchanged.

Resolution requires Actix to adopt a fixed version, or a released 0.3 backport that
RustSec recognizes. After adoption, the advisory check and HTTP regression suite
must both pass.

## Prepared tracking draft

Intended upstream: `actix/actix-web`; local impact: `lyonbrown4d/fluxdi`. A search
by advisory ID found no matching upstream issue. No external issue was created.

Title: `actix-http 3.19.0 remains constrained to h2 0.3 with RUSTSEC-2026-0258`

- Environment: Rust 1.99.1, Actix Web 4.17.0, actix-http 3.19.0 and h2 0.3.27, HTTP/2 enabled.
- Reproduce: run the advisory check for a project using actix-web 4.17.0; updating h2 cannot select the patched 0.4 line.
- Expected: adopt the fixed h2 line or publish an upstream-recognized 0.3 fix.
- Impact: optional Actix integration and example fail dependency auditing; application exploitability needs upstream assessment.
- Acceptance: no ignore needed, and existing HTTP/1, HTTP/2 and cancellation regressions pass.
- Local tracking should retain upstream links, affected features, adopted version and verification results until the fix is used locally.

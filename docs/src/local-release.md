# Local Release

The core `fluxdi` and `fluxdi-macros` are released together. Version 2.0.0 separates
the extended macros from the 1.x core API. The core pins its macro dependency to the
matching version. Examples have `publish = false` and remain repository examples.

## Prepare the machine

Use Rust/Cargo 1.99.1 with rustfmt and Clippy. Run from the repository root on
`main`. Cargo's built-in `publish` command handles the two selected packages in
dependency order; this flow does not invoke the GitHub release workflow.

```bash
cargo --version
rustc --version
git fetch origin --tags
git status --short
cargo login --registry crates-io
```

Browser sign-in and Cargo authentication are separate. Create a crates.io API token
with permission to publish both packages and paste it at Cargo's prompt. Keep the
token out of source files, shell arguments, chat and logs. The authenticated account
must own both packages; the registry checks authorization when publishing.

## Prepare a release commit

Update `[workspace.package].version`, the exact `fluxdi-macros` dependency in
`fluxdi/Cargo.toml`, and the standalone SeaORM example version. Update both
changelogs and installation instructions. Review API and minimum-Rust changes
before choosing a version; incompatible macro expansions must not enter the old
core's compatible dependency range.

```bash
cargo update --workspace --offline
cargo update --manifest-path examples/seaorm-sqlite/Cargo.toml --workspace --offline
cargo fmt --all --check
cargo fmt --manifest-path examples/seaorm-sqlite/Cargo.toml --all --check
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
sh docs/build.sh
cargo package --list -p fluxdi-macros --locked
cargo package --list -p fluxdi --locked
```

Inspect the diff and both package lists, including their README and license files.
Commit the release changes before the final dry run, so package VCS metadata points
to the release commit. Use the [Dependency Audit](./dependency-audit.md) record when
reviewing advisory results; the known Actix/h2 finding is not a clean security audit.

## Verify and publish

Use explicit package names and the crates.io registry. The dry run packages and
builds the selected crates without uploading them. Publishing verifies the packages
again and uploads the macro before the core.

```bash
cargo publish -p fluxdi-macros -p fluxdi --registry crates-io --dry-run --all-features --locked
cargo publish -p fluxdi-macros -p fluxdi --registry crates-io --all-features --locked
```

Check that both expected versions appear on crates.io and build a consumer using
registry dependencies. If upload succeeds but index polling times out, check the
registry before retrying. If only the macro was published, publish the core after
the macro becomes available. Published version contents cannot be overwritten.

After both packages and the consumer check succeed, tag the release commit and
push the commit and tag together. For 2.0.0:

```bash
git tag -a v2.0.0 -m "Release 2.0.0"
git push --atomic origin main v2.0.0
git status --short
```

See Cargo's [publish command](https://doc.rust-lang.org/cargo/commands/cargo-publish.html)
and [publishing guide](https://doc.rust-lang.org/cargo/reference/publishing.html)
for registry authentication, verification and upload behavior.

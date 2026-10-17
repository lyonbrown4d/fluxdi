# 本地发布

核心 `fluxdi` 与 `fluxdi-macros` 协同发布。2.0.0 将扩展后的宏与 1.x 核心 API 分开，核心将宏依赖固定为同一版本。示例包均设置 `publish = false`，仅作为仓库示例维护。

## 准备机器

使用带 rustfmt、Clippy 的 Rust/Cargo 1.99.1。在仓库根目录的 `main` 分支执行。Cargo 内置的 `publish` 命令按依赖顺序处理指定的两个包；本流程不调用 GitHub 发布工作流。

```bash
cargo --version
rustc --version
git fetch origin --tags
git status --short
cargo login --registry crates-io
```

浏览器登录与 Cargo 认证相互独立。创建具有两个包发布权限的 crates.io API token，并在 Cargo 提示时粘贴。不要将 token 写入源码、命令参数、聊天或日志。登录账号必须拥有两个包；注册表会在发布时检查权限。

## 准备发布提交

更新 `[workspace.package].version`、`fluxdi/Cargo.toml` 中宏依赖的精确版本，以及独立 SeaORM 示例版本。同步更新中英文变更日志与安装说明。确定版本前检查 API 和最低 Rust 版本的变化；不兼容的宏展开不能进入旧核心允许的依赖版本范围。

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

检查代码差异及两个包的文件列表，包括 README 和许可证。最终发布演练前先提交版本变更，使包中的 VCS 元数据指向发布提交。检查安全公告时参照[依赖审计](./dependency-audit.md)记录；已知的 Actix/h2 问题仍存在，不能视为安全审计全部通过。

## 验证与发布

显式指定包名和 crates.io 注册表。发布演练会打包并构建指定的 crate，不上传包。实际发布会再次验证包，并先上传宏、再上传核心。

```bash
cargo publish -p fluxdi-macros -p fluxdi --registry crates-io --dry-run --all-features --locked
cargo publish -p fluxdi-macros -p fluxdi --registry crates-io --all-features --locked
```

确认两个目标版本均出现在 crates.io，并使用注册表依赖构建消费方。若上传成功后等待索引超时，先检查注册表再决定是否重试。若只发布了宏包，待宏可用后继续发布核心。已发布版本的内容不能覆盖。

两个包发布完成且消费方检查通过后，为发布提交创建标签，并一起推送提交与标签。2.0.0 对应命令：

```bash
git tag -a v2.0.0 -m "Release 2.0.0"
git push --atomic origin main v2.0.0
git status --short
```

注册表认证、包校验与上传行为参见 Cargo 的 [publish 命令](https://doc.rust-lang.org/cargo/commands/cargo-publish.html)和[发布指南](https://doc.rust-lang.org/cargo/reference/publishing.html)。

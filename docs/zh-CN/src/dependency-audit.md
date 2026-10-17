# 依赖审计

快照日期：2026-10-17。工具：`cargo-deny 0.20.2`。Rust 官方 stable 清单确认最新稳定版为 1.99.1。以下版本描述当前 checkout；下游应用需要审计自身解析到的依赖图。

```bash
cargo install cargo-deny --locked
cargo deny --all-features check
cargo deny --manifest-path examples/seaorm-sqlite/Cargo.toml --all-features check
```

## 最新稳定版直接依赖

全部直接 registry 依赖均已按 crates.io 最新、未撤回的稳定版本核对。主 workspace 的版本集中在 `[workspace.dependencies]`。传递依赖按上游约束更新；上游仍要求旧主版本时，该版本系列仍会保留。

| 依赖 | 版本 |
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

最新版 rusqlite 要求 libsqlite3-sys 0.38，而 SeaORM 2 使用的 SQLx 0.9 要求更早的原生绑定范围。Cargo 的同一依赖图只能包含一个 `links = "sqlite3"` 包。因此 SeaORM 示例采用独立 workspace 和纳入版本控制的锁文件，CI 分别验证两套依赖图，两者均保留最新稳定版。

## 检查结果

首次审计命中 12 条公告。依赖升级清除了其中 11 条，并移除了被撤回的 Spin 版本。通过 SeaORM/sea-bae 升级移除了停止维护的 proc-macro-error2 依赖。没有新增公告忽略项。

- 主 workspace：bans、licenses、sources 通过；advisories 仍有一条 h2 命中，未通过。
- 独立 SeaORM 示例：advisories、bans、licenses、sources 全部通过。

重复版本和 manifest 缺少 SPDX license 字段仍按现有规则显示为警告。CI 原有 advisory job 非阻断设置保持不变；主 workspace 的完整审计尚未全绿。

## 尚未解决的上游约束

[RUSTSEC-2026-0258](https://rustsec.org/advisories/RUSTSEC-2026-0258.html) 命中以下链路：

```text
fluxdi (actix feature) / examples/actix
  -> actix-web 4.17.0
     -> actix-http 3.19.0
        -> h2 0.3.27
```

公告描述的条件是：未及时消费 stream 时，连续的空 DATA frame 可能无限排队，引起内存增长或 panic。RustSec 将严重性列为低，修复版本范围为 `h2 >= 0.4.16`。本地证据是审计命中和依赖约束，尚未使用攻击流量复现 Actix 的具体触发条件。

`cargo update -p h2` 无法在 Actix 当前约束内选择已修复版本。上游修复见 [hyperium/h2#935](https://github.com/hyperium/h2/pull/935)。本轮保留现有协议能力和审计规则。

解除条件为 Actix 采用已修复版本，或上游发布被 RustSec 认可的 0.3 修复。采用修复后，公告检查和 HTTP 回归均需通过。

## 待提交的跟踪草稿

上游目标为 `actix/actix-web`，本地影响属于 `lyonbrown4d/fluxdi`。按公告 ID 搜索未发现匹配上游 issue，尚未创建外部 issue。

标题：`actix-http 3.19.0 remains constrained to h2 0.3 with RUSTSEC-2026-0258`

- 环境：Rust 1.99.1、Actix Web 4.17.0、actix-http 3.19.0、h2 0.3.27，启用 HTTP/2。
- 复现：对采用 actix-web 4.17.0 的项目运行公告检查；更新 h2 无法选择已修复的 0.4 系列。
- 期望：采用已修复 h2 系列，或发布被上游认可的 0.3 修复。
- 影响：可选 Actix 集成和示例的依赖审计失败；具体应用中的可利用性仍需上游评估。
- 验收：无需忽略公告，且现有 HTTP/1、HTTP/2 和取消回归通过。
- 本地跟踪应保留上游链接、受影响 feature、采用版本和验证结果，直至本地实际采用修复。

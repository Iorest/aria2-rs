# aria2-rs

A Rust rewrite of aria2, behaviour-checked with rust-anything (GPL-2.0).

用纯 Rust 实现与 [aria2](https://github.com/aria2/aria2) **1.37.0** 等价的下载器：原版只作只读参考，行为等价性以录制/回放判定（rust-anything whole-app 流程），不用「看起来像」验收。

## 方法

`analyze --whole-app → app record（原版 100% 回放）→ app front-door（回落外壳）→ 逐单元移植 + app wire（fallback 只降不升）→ fallback=0 → app lift（纯 Rust 二进制）`。

- 等价性口径与归一化规则：[docs/EQUIVALENCE.md](docs/EQUIVALENCE.md)
- 验收通道（C++ 无单元级 record，走 app replay + 手写契约测试）：[rust/DECISIONS.md](rust/DECISIONS.md) ADR-001
- 单元划分与 crate 选型：[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)、[docs/DEPENDENCIES.md](docs/DEPENDENCIES.md)

## 文档索引

| 路径 | 内容 |
|---|---|
| `docs/TEST-ENV.md` | 本地测试环境与夹具规格（loopback 三道防线、合成凭据、可复现命令） |
| `docs/RECORD-CASES.md` | 首轮 app record 用例清单与 G0–G4 闸门 |
| `docs/EQUIVALENCE.md` | 等价性契约：判定等级、归一化 N1–N10、反作弊 PR 硬门 |
| `docs/ARCHITECTURE.md` | 原版架构与单元划分（源码路径证据） |
| `docs/DEPENDENCIES.md` | 原版依赖清单与 Rust crate fit（N1–N9 手写单元登记处） |
| `docs/PROTOCOL-MATRIX.md` | 协议支持矩阵：HTTP/HTTPS、RPC、BT、Metalink、FTP/SFTP 分列 |
| `rust/DECISIONS.md` | ADR 注册表 |
| `rust/MAP.md` | 移植地图：全部单元状态与 claim 规则 |
| `rust/GLOSSARY.md` | 术语表（原版域术语 ↔ Rust 命名） |
| `rust/HANDOFF.md` | 会话交接：现状与下一步 |
| `rust/cutover.sh` | 切换指南（仅人工步骤与回退方式，交互式） |

## 状态

阶段 0 已收口；阶段 1（基线录制与 Rust 前门）已交付，CI replay 全绿。阶段表见上游任务 TELE-88（Multica）。前门 `rust/app` 建立后，未移植命令全部回落原版 aria2（首轮 fallback 100%）。


## 复现阶段 1

先按 `docs/TEST-ENV.md` §8 构建 pinned aria2 1.37.0 与安装依赖，再：

```sh
test-env/setup.sh
test-env/serve.sh start && test-env/serve.sh check
RUN='env/aria2-1.37.0/bin/aria2c --no-conf --dir=downloads --allow-overwrite=true --auto-file-renaming=false --all-proxy=http://127.0.0.1:9 --no-proxy=127.0.0.1,localhost --max-tries=1 --retry-wait=0 --ca-certificate=test-env/certs/ca.crt'
rust-anything app replay . --build "$RUN"
rust-anything app replay . --build rust/app/target/debug/aria2c
```

## 许可证

GPL-2.0（与原版 aria2 一致；衍生作品同许可证）。原版 aria2 保持只读，不修改、不并入本仓。

# GLOSSARY — 术语表

> TELE-95 交付物 · 初稿（2026-11-27）。每个术语：一句含义 + 权威出处 + Rust 侧名称（拟）。
> 目的：让每个单元对同一事物用同一个名字。随移植推进补条目；Rust 名称在对应单元落地时定稿并去「拟」。

## 原版（aria2）域术语

| 术语 | 含义（一句） | 权威出处 | Rust 名称（拟） |
|---|---|---|---|
| DownloadEngine | 单线程事件循环 + Command 队列，驱动所有下载活动 | `docs/ARCHITECTURE.md` §1；`aria2@1.37.0 src/DownloadEngine.cc` | `engine`（tokio 任务组） |
| Command | 一次可恢复的执行步骤，`execute()` 返回 EXEC_* 决定重入队 | ARCHITECTURE §1（138 个 Command 文件） | tokio async 状态机（逐个消失） |
| RequestGroup | 一个下载任务及其共享对象集合 | `src/RequestGroup.cc` | `DownloadGroup` |
| SegmentMan / PieceStorage | 分段调度与分片存储（多源分块核心） | `src/SegmentMan.cc`、`src/DefaultPieceStorage.cc` | `segment` / `piece_storage` |
| BitfieldMan | 位图表示已完成分片 | `src/BitfieldMan.cc` | `bitfield` |
| GID | 64 位随机任务标识；RPC 中 16 个小写 hex，读出行中前 6 hex 缩写 | `docs/EQUIVALENCE.md` E13；`src/GroupId.cc` | `Gid` |
| .aria2 控制文件 | 断点续传状态文件，二进制格式 version `0001` | EQUIVALENCE E12；`src/DefaultBtProgressInfoFile.cc` | 单元 `control-file` |
| 口径（EQ-\*） | 一条可判定的一致性声明，标判定等级 + 理由 + 主源 + 测试归属 | `docs/EQUIVALENCE.md` §1/§3 | 测试名 `EQ-<编号>` |
| 归一化（N1–N10） | 比较前对双方输出做的确定性替换/结构化规则 | EQUIVALENCE §4 | 对比器内规则模块 |
| 宽松 URI 解析 | aria2 自写的宽容解析，≠ WHATWG 严格解析 | DEPENDENCIES §3 URI 行；`src/uri.cc` | 单元 `uri-parse` |

## BitTorrent 域术语

| 术语 | 含义 | 权威出处 | Rust 名称（拟） |
|---|---|---|---|
| metainfo | 种子元数据（bencode 编码的 .torrent） | `docs/ARCHITECTURE.md` §2.3 | `metainfo` |
| choke 策略 | 做种/下载两态的 peer 上传配额算法 | ARCHITECTURE §2.3 | `choke` |
| MSE | BT 加密握手（RC4 + DH） | `src/MSEHandshake.cc` | 单元 `bt-mse` |
| DHT | BEP-5 分布式节点表 | ARCHITECTURE §2.3 | 单元 `bt-dht` |

## rust-anything 流程术语

| 术语 | 含义 | 权威出处 | Rust 名称 |
|---|---|---|---|
| front door | 早期接管入口的 Rust 外壳，未移植部分回落原版 | rust-anything whole-app skill §4 | `rust/app` crate |
| fallback rate | 仍由原版应答的用例占比，只降不升 | whole-app skill §4 | —（指标） |
| wire | 把已移植单元接进 front door 并停止回落 | whole-app skill §5 | `rust/app/src/units/<unit>.rs` |
| kill switch | `RUST_ANYTHING_UNIT_<NAME>=original` 让单单元回退原版 | whole-app skill §5 | —（环境变量） |
| lift | fallback=0 且全部单元 gate 通过后，删除回落、产出纯 Rust 应用 | whole-app skill §6；ADR-001 lift 收口定义 | —（步骤） |
| unit / claim | 移植单元 / 会话领取权 | `rust/MAP.md` §4 | —（流程） |
| gate | 单元验收：fmt / clippy / 测试 / unsafe 纪律 / 录制用例全匹配 | rust-anything SKILL §5 | —（命令） |
| wave | plan 中互不依赖、可并行的单元批次 | whole-app skill §5 | —（流程） |

约定：单元 crate 名 = 本表「单元 `<name>`」处登记的名字（`rust/MAP.md` §3）；模块/类型名在单元落地时进本表，不另立命名文档。

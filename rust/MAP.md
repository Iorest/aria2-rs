# MAP — aria2-rs 移植地图（计划与 claim）

> rust-anything whole-app 流程的会话间载体：目标、ADR 索引、全部单元及状态、claim 规则、未决问题。
> 本文件只做指针，不复制其他文件内容。实测状态由 `rust-anything map --write` 生成到 `rust/PORT_MAP.md`，每个 checkpoint 与本文件对账；PORT_MAP 是实测，MAP 是计划与 claim。
> 状态：初稿（TELE-95，2026-11-27）。引擎/底座单元的最终切分以阶段 1 `rust-anything analyze --whole-app` 的 plan 为准，plan 落地后回填 §3.2。

## 1. 目标

在 rust-anything 录制覆盖的行为范围内，用纯 Rust 实现与 aria2 等价的下载器；原版 aria2 1.37.0 只作只读参考。总目标与阶段闸门见父任务 TELE-88；等价性判定按 `docs/EQUIVALENCE.md`，验收通道按 `rust/DECISIONS.md` ADR-001。

## 2. ADR 索引

| # | 决策 | 状态 |
|---|---|---|
| ADR-001 | C++ 无单元级 bridge/record，采用 app replay + 手写契约测试 | Accepted |

详见 `rust/DECISIONS.md`；新 ADR 追加到该文件并回填本索引。

## 3. 单元清单与状态

状态取值：`open` ｜ `claimed by <会话> <日期>` ｜ `done`（必须附 gate/replay 结果）。会话 = 承载移植工作的 Multica 任务号。

### 3.1 具名手写单元（`docs/DEPENDENCIES.md` §4 N1–N9，来源与参考源码见该表，此处不重复）

| 单元 | 能力 | fit | 阶段 | 状态 |
|---|---|---|---|---|
| `uri-parse` | 宽松 URI 解析（唯一走完整单元 loop 的单元，含 `src/uri_split.c`） | none | 2 | done（claimed by TELE-98 2026-11-28：`diff` 237/237、`gate` pass、front-door replay 185/185、fallback 100%→50.8%） |
| `control-file` | .aria2 控制文件二进制格式（契约级，EQUIVALENCE E12） | none | 2 | done（claimed by TELE-98 2026-11-28：非 BT 字段逐字节与 cus-08 golden 一致；BT 扩展留阶段 4） |
| `rpc-methods` | RPC 方法层（约 30 个方法语义） | none | 3 | open |
| `rpc-xml` | XML-RPC 服务端（aria2 方言） | none | 3 | open |
| `bt-peer` | BT 全栈：Peer Wire 消息、choke、PEX、ut_metadata | none | 4 | open |
| `bt-dht` | DHT（BEP-5） | none | 4 | open |
| `bt-tracker` | UDP tracker（BEP-15） | none | 4 | open |
| `bt-mse` | MSE 加密握手（rc4 + num-bigint） | none | 4 | open |
| `metalink` | Metalink 3/4 语义（quick-xml + 摘要组合件） | none | 5 | open |

### 3.2 引擎与共享底座单元（切分待阶段 1 plan 定稿后回填；划分依据 `docs/ARCHITECTURE.md` §1/§2.6）

| 单元（暂名） | 能力 | 阶段 | 状态 |
|---|---|---|---|
| `engine` | 下载引擎核心：DownloadEngine / RequestGroup / SegmentMan / PieceStorage / 调度 | 2 | done（claimed by TELE-98 2026-11-28：单/多源 HTTP 下载、Range 分段、`-x/-k`、续传、校验、控制文件；app replay 185/185） |
| `option` | 选项系统（OptionParser / OptionHandlerFactory） | 2 | done（claimed by TELE-98 2026-11-28：前门必要选项子集，未识别选项回落原版） |
| `disk` | 磁盘写入与文件分配（falloc/trunc/prealloc） | 2 | open（阶段 2 只实现下载写入与控制文件） |
| `net-base` | socket 选项 / DNS / TLS / 摘要（tokio + socket2 + rustls + RustCrypto） | 2 | done（claimed by TELE-98 2026-11-28：rustls 单后端 + --ca-certificate 真实校验；sha-1 校验；app replay 185/185） |
| `ftp` | FTP 客户端（数据面手写，suppaftp 仅作协议参考） | 6 | open（暂名） |
| `sftp` | SFTP（russh + russh-sftp） | 6 | open（暂名） |

### 3.3 跨单元步骤（非单元，由阶段里程碑承载）

`app record` / `app front-door` / 逐阶段 `app wire` / `app lift` 对应 TELE-97..103（父任务 TELE-88 阶段表）；fallback rate 只降不升，状态随阶段里程碑核对更新，不在本表逐行登记。

**阶段 1 基线（2026-11-27，TELE-97 实测）**：`rust/app-cases/` 185 例（23 手工 + 162 traps）；`app replay` 原版 185/185、front-door 185/185，**fallback rate = 100.0%（185/185）**。阶段 2 起 G4 断言 ≤ 100%，每次 wire 后记录新基线。原版 SHA-256 `5fa3b455a29d4f8d9251617dd93037933f83f2b5e7747439ef14587f9aceab9f`。

**阶段 2 首个 wire（2026-11-28，TELE-98 / `uri-parse`）**：front-door 自己拒绝 `uri_split()` 判为不支持的 URI（E1/E5），`app replay` 185/185，**fallback rate = 50.8%（94/185）**；较阶段 1 基线下降 49.2 个百分点。仅下降，不回退。

**阶段 2 下载核心 wire（2026-11-28，TELE-98）**：前门自己完成 HTTP/HTTPS 单源与多源下载、Range 分段、`-x/-k` 多连接、续传、sha-1 校验、截断中断控制文件；TLS 收敛到 rustls 单后端并走 `--ca-certificate` 真实校验。`app replay` 185/185，**fallback rate = 42.7%（79/185）**。相对首个 wire 再降 8.1 个百分点，相对阶段 1 降 57.3 个百分点。

## 4. Claim 规则

1. **先领后动**：会话开工前在本文件把目标单元状态改为 `claimed by <会话> <日期>`；一次只领一个单元，完成后标 `done` 再领下一个。
2. **已领即归属**：`claimed` 单元归该会话，其他会话（含并行子会话）另选单元，不得同时移植。
3. **完成要带证据**：标 `done` 必须附结果——走 loop 的单元（当前仅 `uri-parse`）附 `diff` N/N 与 `gate` 通过；C++ 手写单元按 ADR-001 附 app replay 全绿 + EQ-\* 契约测试全绿 + fallback rate 下降。
4. **登记前置**：手写单元必须已登记在 `docs/DEPENDENCIES.md` §4（ADR-001 D3），未登记不许领、不许手写。
5. **阶段闸门**：单元所属阶段未被统筹放行前不可领取（TELE-88 规矩）。
6. **冲突裁决**：同单元被重复 claim 时，以 MAP 中先写入的为准；写入顺序以 Multica 任务时间线为准，后写者撤销其 claim。

## 5. 未决问题

- 引擎/底座单元的最终切分与命名（§3.2 暂名行）——阶段 1 `analyze --whole-app` plan 定稿后回填，同时校准 §3.1 是否拆分。
- `bt-peer` 是否随 plan 再拆（ARCHITECTURE §2.3 规模最大：77 个 `Bt*` + 111 个 `DHT*` 文件）——留到阶段 4 前。
- DHT/加密握手行为契约（TELE-100 阶段交付物要求单列 ADR）——阶段 4 开工前补。

## 6. 范围外（不移植、不引入）

- 不移植：5 个 `EventPoll` 后端（tokio 取代）、9 个可移植性 shim `.c`、`deps/wslay/`（tokio-tungstenite 取代）、gettext 本地化（U1，等 TELE-89 契约定英文基线）——`rust/DECISIONS.md` ADR-001 .c 处置表、`docs/DEPENDENCIES.md` §2。
- 不引入：ssh2（C 绑定）、brotli、HTTP Digest 认证、SOCKS 代理、tcmalloc/jemalloc、libuv、cppunit、`libc` 1.0-alpha——`docs/DEPENDENCIES.md` §5。

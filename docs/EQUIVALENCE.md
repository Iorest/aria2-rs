# EQUIVALENCE — aria2 ↔ aria2-rs 等价性契约

版本 v0.1（2026-11-27，TELE-89）。契约变更须经 owner 确认后修改本文件。

## 1. 范围与锚点

- **原版锚点**：aria2 **1.37.0**。事实核对于 2026-11-27 对 master 的浅克隆
  （HEAD `9e7273583f83e881e3ec067b523ba88724088d2f`，`configure.ac` `AC_INIT` 为 1.37.0）。
  测试基线一律用 1.37.0 release tarball 构建的原版二进制。
- **本仓**：Iorest/aria2-rs（GPL-2.0）。原版只读参考，不修改。
- **平台**：原版与 aria2-rs 必须在同一台机器、同一 fixture 上做对比；
  本地 macOS 与 CI Linux 各自内部对比，不跨平台比较输出。
- **locale**：测试固定 `LC_ALL=C`，只比对英文消息。aria2 走 gettext 本地化
  （非 C locale 消息会变），不在契约范围内。
- **网络边界、fixture 服务器、证书**：见 `docs/TEST-ENV.md`（权威出处，本文不重复）。
- **术语**：「口径」= 一个可判定的一致性声明；每个口径标注判定等级、理由、主源、测试归属。

## 2. 判定等级

| 等级 | 含义 |
|---|---|
| **完全一致** | 字面/字节级相等（数值、字符串、退出码、哈希）。差异即失败。 |
| **归一化后一致** | 先按 §4 规则做替换/排序/结构化，再比较。归一化后不等即失败。 |
| **不比较** | 无测试。理由必须写进口径表；取消「不比较」须改本文件。 |

## 3. 口径表

主源引用格式：`aria2@1.37.0 <文件> <符号或小节>`；手册 =
`doc/manual-src/en/aria2c.rst`（下称 MAN）。

### CLI

| # | 口径 | 判定 | 理由 | 主源 | 测试归属 |
|---|---|---|---|---|---|
| E1 | 退出码 0–32 | 完全一致 | 一等接口，文档化且确定性（按「最后遇到的错误」返回） | MAN「EXIT STATUS」；`src/error_code.h` `error_code::Value` | EQ-E1：每个错误类 fixture 各断言一个码 |
| E2 | stdout 终态汇总（`Download Results:` 表 + `Status Legend:`） | 归一化后一致（N1–N4、N8） | 表格结构、状态串（`OK`/`ERR`/`INPR`/`RM`，4 字符宽）、分隔线格式确定性；avg speed 与 gid 列随时序/GID 随机变化 | `src/RequestGroupMan.cc` `showDownloadResults`（表头 `gid|stat|avg speed|path/URI`，path 列 55 字符截断）；`src/RequestGroupMan.cc` `getStatusStr` | EQ-E2 golden |
| E3 | stdout 进度读出（console readout） | 归一化后一致（N1–N5、N8、N9） | 格式确定性（单任务 `[#gid done/total(%) CN:n …]`，多任务 compact 形式、`[FileAlloc:#gid …]`/`[Checksum:#gid …]` 段），数值与节拍随时序变化；非 TTY 下每秒一行 | `src/ConsoleStatCalc.cc` `printProgress`/`printProgressCompact`/`calculateStat`（gid 缩写=前 6 hex，`src/GroupId.cc` `toAbbrevHex`） | EQ-E3 golden（非 TTY 运行） |
| E4 | stdout 通知/日志行（console logger，NOTICE 及以上） | 归一化后一致（N1、N2、N8、N9） | 消息文本确定性（如 `Download complete: %s`）；与读出行的交错随时序变化 → 按 N9 先剥读出行再比序 | `src/Logger.cc`（console sink → stdout）；`src/message.h`（消息文案） | EQ-E4 golden |
| E5 | stderr（用法/参数错误/致命异常） | 归一化后一致（N2） | 用法文本与错误消息确定性；路径与程序名归一化 | `src/option_processing.cc`（`global::cerr()` 输出） | EQ-E5 golden |
| E6 | `--stderr=true` 行为 | 完全一致 | 开关语义固定：console 输出整体改写 stderr | `src/console.cc` `redirectStdoutToStderr`；`src/option_processing.cc`；MAN `--stderr` | EQ-E6 |
| E7 | `--version` | 不比较 | 版本号与编译特性列表属于实现自身身份（aria2-rs 报自己的版本），不属下载行为契约；协议能力由功能测试覆盖 | `aria2c --version` 输出 | 仅断言 exit 0 |

### 下载产物

| # | 口径 | 判定 | 理由 | 主源 | 测试归属 |
|---|---|---|---|---|---|
| E8 | 文件内容 | 完全一致 | 核心功能；以 SHA-256 断言字节级相等 | 行为本身 | EQ-E8 |
| E9 | 文件名与目录结构（含多文件 BT、Metalink 展开相对路径） | 完全一致 | 命名与展开逻辑确定（Content-Disposition、auto-renaming `.1` 后缀依赖 fixture 目录状态，测试前重置） | MAN「HTTP/FTP Download」等；`src/download_helper.cc` | EQ-E9 |
| E10 | 产物元数据（mtime、权限、owner、稀疏性、预分配方式） | 不比较 | 取决于文件系统/平台/时序，不构成行为契约 | — | — |

### .aria2 控制文件

| # | 口径 | 判定 | 理由 | 主源 | 测试归属 |
|---|---|---|---|---|---|
| E11 | 存在性语义 | 完全一致 | 完成即删、不可续传场景不创建、多文件 BT 控制文件取顶层目录名——均确定 | MAN「Control File」 | EQ-E11 |
| E12 | 控制文件内容 | 归一化后一致（N7） | 二进制格式确定性：version=`0001`、extension（BT=1）、infoHash、pieceLength、totalLength、bitfield 等逐字段比对（完全一致级）；BT `uploadLength` 随做种时长变化、in-flight piece 集合随调度变化 → 集合语义比对 | `src/DefaultBtProgressInfoFile.cc` `save()`/`load()`（整数字段网络字节序） | EQ-E12（fixture 用本地服务器截断触发中断） |

### RPC

| # | 口径 | 判定 | 理由 | 主源 | 测试归属 |
|---|---|---|---|---|---|
| E13 | GID 表示 | 归一化后一致（N1） | GID 为 64 位随机数 → 跨运行不可复现；表示固定：RPC/字符串 = 16 个小写 hex（缩写=前 6 hex）；结构断言 `^[0-9a-f]{16}$`，比较时用符号替换 | `src/GroupId.cc` `create()`/`toHex()`；`src/util.cc` `toHex`（小写）；MAN「JSON-RPC/XML-RPC SERVER」 | EQ-E13 |
| E14 | JSON 结构（tellStatus 等返回 struct） | 归一化后一致（N6） | 键集合与嵌套形状确定性、键序无关；条件键（未开始无 `bitfield`、BT 才有 `bittorrent`、停止后才有 `errorCode`）按 fixture 断言 | MAN `aria2.tellStatus` 键表 | EQ-E14 |
| E15 | JSON 字段值 | 拆两级：`status`/`totalLength`/`completedLength`/`pieceLength`/`numPieces`/`infoHash`/`errorCode`/`errorMessage`/`dir`/`files`/`seeder`/`belongsTo`/`followedBy`/`following`/`bitfield`（终态）= **完全一致**（gid 按 N1）；`downloadSpeed`/`uploadSpeed`/`connections`/`numSeeders` = **不比较** | 长度/状态/哈希/关系确定；速率与连接数随时序波动，无契约价值 | MAN `aria2.tellStatus`；`src/RpcMethodImpl.cc` | EQ-E15 |
| E16 | RPC 错误码与 HTTP 状态配对 | 完全一致 | 全部确定性：方法级错误 → JSON-RPC `error:{code,message}` 且 `code=1`、HTTP 400；传输级 `-32700 "Parse error."`、`-32600 "Invalid Request."`（缺 id/method 非 string）、`-32602 "Invalid params."`（named params 不支持）；`-32601` 在 HTTP 响应映射到 404，但 1.37.0 源内未发现产生点（见主源，按不可达处理）；未知方法 → `code=1, "No such method: <名>"`；鉴权失败 → `code=1, "Unauthorized"`；XML-RPC 错误 → `faultCode=1`/`faultString`；注意：方法级错误**不**用 JSON-RPC 2.0 标准 -32xxx 码——这是重写者最容易踩的差异 | `src/RpcMethod.cc` `execute`/`authorize`/`createErrorResponse`；`src/rpc_helper.cc` `processJsonRpcRequest`/`createJsonRpcErrorResponse`；`src/HttpServerBodyCommand.cc`（`-32700`、状态映射 1→400/-32600→400/-32601→404/默认→500）；`src/RpcMethodImpl.cc` `NoSuchMethodRpcMethod`；MAN「Error Handling」（`faultCode=1`） | EQ-E16：逐码一个用例 |
| E17 | 方法集与通知集 | 完全一致 | `system.listMethods`/`system.listNotifications` 输出为固定数组；WS 通知 = `aria2.onDownloadStart/Pause/Stop/Complete/Error` + `aria2.onBtDownloadComplete`，载荷为 tellStatus 形状 struct | MAN `system.listMethods`/`system.listNotifications`；`src/WebSocketSessionMan.cc`（通知名常量） | EQ-E17 golden |
| E18 | RPC 端点与传输 | 完全一致 | 路径固定：`/jsonrpc`（JSON-RPC，POST/GET/WebSocket 升级）、`/rpc`（XML-RPC）；批量调用、`token:` 鉴权前缀、GET 查询参数形式均确定 | `src/HttpServer.cc`（路径判断）；`src/RpcMethod.cc` `authorize`；MAN「JSON-RPC using HTTP GET」「JSON-RPC over WebSocket」 | EQ-E18 |
| E19 | 文件日志（`--log <file>`） | 归一化后一致（N4、N1、N2） | 行格式与级别标记确定；时间戳随时钟变化 | `src/Logger.cc`（file sink 格式） | EQ-E19 |

### 系统行为

| # | 口径 | 判定 | 理由 |
|---|---|---|---|
| E20 | 并发节奏、重试间隔观测值、缓冲区、内存、线程数、DNS/连接复用细节、TLS 指纹、peer 选择顺序 | 不比较 | 非契约行为；其正确性由 E1/E8/E12/E14 等可观测结果间接覆盖 |
| E21 | 性能（吞吐、延迟） | 不比较（本契约内） | 属阶段 7 `PERF.md` 范围（父任务 TELE-88 阶段表），不在等价性 gate 内 |

## 4. 归一化规则

比较器按序应用；所有替换作用于「原版输出」与「aria2-rs 输出」双方。

| # | 规则 |
|---|---|
| N1 | GID：`[#<6hex>` → `[#<GID>`；16 hex 的 gid 字符串 → `<GID>`。比较器用本进程内收集到的真实 GID 做映射，杜绝误替换；阶段 1 落地 `test-env/check-gid-consistency.py`，逐次运行断言结果表短 GID 与 `GID#` 长 GID 一一对应。 |
| N2 | 路径：fixture 沙箱根目录绝对路径 → `<DIR>`；两个实现各用独立沙箱目录，产物比对先转相对路径。 |
| N3 | 读出与汇总中的动态数值：速率、ETA、百分比、avg speed → `<N>`；`ETA:<h m s>` 整段 → `ETA:<T>`（格式见 `src/util.cc` `secfmt`）。 |
| N4 | 时间戳与日期：文件日志行首的 `MM/DD`（跨时区/跨日录制差异）、`Progress Summary as of <asctime>`、日志行首时间 → `<TS>`。 |
| N5 | 端口/地址：本地服务器端口 → `<PORT>`；BT peer `IP:port` → `<ADDR>`。 |
| N6 | JSON：解析后做结构比较（键序无关），字符串值按 N1/N2/N5 归一，数值按 E15 分级。 |
| N7 | 控制文件：解析二进制后逐字段比对；in-flight piece 条目按 index 排序后作为集合比较（条目内 bitfield 必须逐字节相等）；BT `uploadLength` 只断言 ≥ fixture 起始值。原始字节流不做整体比较。 |
| N8 | 终端效应：测试一律以管道运行（非 TTY）→ 双方无 `\r` 刷新、无 ANSI 颜色；比较前剥除 `\r` 与 ANSI 转义（防御）。固定 `--enable-color=false` 环境不预设。 |
| N9 | 行序：按 E3 规则识别并移除进度读出行后，通知/日志行保持相对顺序比较。 |
| N10 | 行尾空白剥除；不折叠空行（空行是格式的一部分）。 |
| N11 | 源码位置：aria2 错误行里的 `[File.cc:<line>]` 行号随构建平台不同（实测 macOS/Ubuntu 相差 1–3）→ `<SRCLINE>`；文件名、错误码、URI、消息正文不归一。 |
| N12 | getopt 用法错误：argv[0] 的绝对路径/程序名差异，以及未知选项的开引号（` vs '）→ `<ARGV0>`；选项名、闭引号、退出码逐字比对。 |

N11/N12 由 owner 2026-11-27T15:00:10Z「自主推进」授权与统筹 15:36:56Z 裁决确认，属于 E5（stderr）在 §2 的「归一化后一致」；对应实现只在比对两侧替换，不修改录制品、实现代码或闸门。`speed`/`timestamps` 分别归入 N3/N4。

## 5. 反作弊与实现纪律（PR 硬门）

1. **禁止复制答案**：
   - 实现代码不得读取 fixture、golden、期望值文件；不得按测试名、fixture 路径、环境变量、进程检测等特判行为（任何形式的「认出这是测试」即作弊，打回）。
   - golden 期望值由对比器从 fixture + 原版 aria2 实跑录制生成，只存在于测试资产中，不进实现。
   - 接口常量（退出码、RPC 错误码、状态串、字段名、格式模板）必须以本契约引用的主源为出处，实现处注释标明（如 `// aria2@1.37.0 src/error_code.h: CHECKSUM_ERROR=32`）。
   - 审查规则：新增字符串常量若与某 fixture 期望输出逐字匹配且非接口常量，审查者必须要求解释来源。
2. **unsafe 纪律**：每个 `unsafe` 块必须附 `// SAFETY:` 说明依赖的不变量与失效后果；无说明 unsafe 直接打回。等价性对比器自身默认纯 safe。
3. **禁止缺测试**：本表判定 ≠「不比较」的每个口径必须有映射的测试（EQ-* 编号，落在 aria2-rs 仓的 replay 用例或集成测试）；无映射的 PR 不合入。「不比较」行如需改为可比较，先改本文件（owner 确认）再补测试。
4. **原版使用边界**：原版源码仅作行为与格式参考；移植结构性算法时在 PR 中标注对应原版符号并说明等价理由，便于审查对照。

## 6. 验证方法

- **差分对比**：`aria2c` 与 `aria2-rs` 以相同参数、各自独立沙箱目录对同一 fixture 运行；采集 stdout/stderr/exit code/产物/控制文件；按 §4 归一化后比对。对比器与 golden 录制在阶段 1 落地（父任务「replay-report 入 CI」）。
- **控制文件**：由 fixture 触发的中断（本地服务器在指定字节数后截断/断开）后快照，按 E12 比对。
- **RPC**：录制请求/响应用例回放 + 本地活服务器差分（阶段 3）；WebSocket 通知按事件序列比对（载荷按 E14/E15）。
- 主源清单（aria2@1.37.0）：`src/error_code.h`、`src/GroupId.{h,cc}`、`src/ConsoleStatCalc.cc`、`src/RequestGroupMan.cc`、`src/Logger.cc`、`src/message.h`、`src/console.cc`、`src/option_processing.cc`、`src/DefaultBtProgressInfoFile.cc`、`src/RpcMethod.cc`、`src/rpc_helper.{h,cc}`、`src/RpcMethodImpl.cc`、`src/HttpServer.cc`、`src/HttpServerBodyCommand.cc`、`src/WebSocketSessionMan.cc`、`src/util.cc`（`toHex`/`secfmt`/`abbrevSize`）、`doc/manual-src/en/aria2c.rst`。

## 7. 契约变更流程

改本文件 = 契约变更：PR 到 aria2-rs 仓 `docs/EQUIVALENCE.md`，owner 确认后生效；
阶段任务中发现的口径缺口（新增消息、新端点、新字段）必须回填本表，不允许只改测试不改契约。

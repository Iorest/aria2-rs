# PROTOCOL-MATRIX — 协议支持矩阵

> TELE-95 交付物 · 原版基线 aria2 `release-1.37.0`。按协议分列：每个子能力给出原版证据落点、Rust 侧落点（单元 / crate）、目标阶段与移植状态。
> 源码路径证据与单元内文件清单的**权威出处**是 `docs/ARCHITECTURE.md` §2；crate 选型与 fit 的权威出处是 `docs/DEPENDENCIES.md` §3/§4——本文件只做矩阵导航，不复制细节。
> 状态：初稿（TELE-95，2026-11-27）。状态取值与 `rust/MAP.md` 一致：`open` / `claimed` / `done`；当前全部 `open`（阶段 0 未结束，阶段 1 未放行）。

## 1. HTTP/HTTPS

| 子能力 | 原版证据 | Rust 落点 | fit | 阶段 | 状态 |
|---|---|---|---|---|---|
| 单源下载 / Range 续传 / 多源分段 | ARCHITECTURE §2.1（客户端链） | `engine` + `net-base` | — | 2 | done（TELE-98） |
| 宽松 URI 解析 | `src/uri.cc`、`src/uri_split.c` | 单元 `uri-parse`（DEPENDENCIES §4 N8） | none | 2 | done（TELE-98，diff 237/237 + gate） |
| 代理（GET / CONNECT 隧道） | ARCHITECTURE §2.1（代理链） | `net-base` | — | 2 | open |
| Basic 认证 + netrc | ARCHITECTURE §2.1（认证行） | `net-base` | — | 2 | open |
| Cookie（Netscape / Firefox sqlite 导入） | ARCHITECTURE §2.1 | `net-base`；sqlite 走 rusqlite（可选 feature） | ok | 2 | open |
| gzip 内容解码 | ARCHITECTURE §2.1（内容解码行） | flate2 | ok | 2 | open |
| HTTPS/TLS | ARCHITECTURE §2.6（TLS ×4 后端收敛） | rustls 单后端 | ok | 2 | done（TELE-98，真实 CA 校验） |
| 控制文件（断点续传） | `src/DefaultBtProgressInfoFile.cc` | 单元 `control-file`（N6，契约级 EQ-E12） | none | 2 起 | done（TELE-98：非 BT 字段；BT 扩展留阶段 4） |

不做：brotli 解码、HTTP Digest 认证、SOCKS 代理——原版不支持（DEPENDENCIES §5）。

## 2. RPC（JSON-RPC / XML-RPC / WebSocket）

| 子能力 | 原版证据 | Rust 落点 | fit | 阶段 | 状态 |
|---|---|---|---|---|---|
| 内嵌 HTTP 服务端 | ARCHITECTURE §2.2（服务端行） | `engine` + tokio/axum（front door 先行，TELE-97） | — | 2–3 | open |
| JSON-RPC 通道 | `src/JsonParser.cc` | serde_json（DEPENDENCIES §3） | ok | 3 | open |
| RPC 方法层（addUri / tellStatus 等约 30 个） | `src/RpcMethodImpl.cc` | 单元 `rpc-methods`（N9） | none | 3 | open |
| XML-RPC 通道（aria2 方言） | ARCHITECTURE §2.2（XML-RPC 行） | 单元 `rpc-xml`（N5，quick-xml 之上） | none | 3 | open |
| WebSocket 通知 | `deps/wslay/` + ARCHITECTURE §2.2 | tokio-tungstenite；行为对齐 U3 待 replay 裁定 | partial | 3 | open |

对外契约（端点 `/jsonrpc`、`/rpc`、错误码配对、GID 表示）见 `docs/EQUIVALENCE.md` E13–E18。

## 3. BitTorrent

| 子能力 | 原版证据 | Rust 落点 | fit | 阶段 | 状态 |
|---|---|---|---|---|---|
| metainfo / bencode 解析 | ARCHITECTURE §2.3（元数据行） | bendy（serde_bencode 备选） | ok | 4 | open |
| Peer Wire 协议（消息族 + choke 策略） | ARCHITECTURE §2.3（Peer Wire 行） | 单元 `bt-peer`（N1） | none | 4 | open |
| 扩展协议（ut_metadata / PEX） | ARCHITECTURE §2.3（扩展行） | `bt-peer` | none | 4 | open |
| MSE 加密握手（RC4 + DH） | `src/MSEHandshake.cc`（参考内建纯实现） | 单元 `bt-mse`（N4，rc4 + num-bigint） | none | 4 | open |
| HTTP/UDP tracker | ARCHITECTURE §2.3（Tracker 行） | `bt-tracker`（N3，UDP BEP-15）；HTTP 通道并入 | none | 4 | open |
| DHT（BEP-5） | ARCHITECTURE §2.3（DHT 行，111 文件） | 单元 `bt-dht`（N2） | none | 4 | open |
| 控制文件 BT 扩展 | `src/DefaultBtProgressInfoFile.cc` | `control-file`（N6） | none | 2 起 | open |

DHT / 加密握手的行为契约在阶段 4 开工前单列 ADR（MAP §5）。

## 4. Metalink

| 子能力 | 原版证据 | Rust 落点 | fit | 阶段 | 状态 |
|---|---|---|---|---|---|
| Metalink 3 / 4 (RFC 5854) SAX 解析 | `src/MetalinkParserStateMachine.cc` 等 26 文件 | 单元 `metalink`（N7，quick-xml 之上） | none | 5 | open |
| 多源映射 → RequestGroup | `src/Metalink2RequestGroup.cc` | `metalink` + `engine` | — | 5 | open |
| 校验值（sha1/sha256/md5） | MessageDigest 门面 | sha1 / sha2 / md-5（RustCrypto） | ok | 5 | open |

底层 XML 复用 quick-xml（取代 expat/libxml2 双后端，DEPENDENCIES §3）。

## 5. FTP/SFTP

| 子能力 | 原版证据 | Rust 落点 | fit | 阶段 | 状态 |
|---|---|---|---|---|---|
| FTP 协商（USER/PASV/EPSV/REST）+ 数据通道 | ARCHITECTURE §2.5（FTP 行） | 单元 `ftp`；suppaftp 仅作协议参考，不进运行时 | partial | 6 | open |
| FTP 经 HTTP 代理 | `FtpTunnelRequestCommand`/`FtpTunnelResponseCommand` | `ftp` 复用 CONNECT 隧道 | — | 6 | open |
| SFTP | `src/SSHSession.cc`（libssh2） | 单元 `sftp`：russh + russh-sftp（ssh2 弃用，DEPENDENCIES §5） | partial | 6 | open |
| 凭据处理 | netrc / `--ftp-*` 选项 | `net-base`；凭据不落日志为阶段 6 验收项（TELE-102） | — | 6 | open |

## 6. 共享底座（五协议共用）

| 子能力 | 原版证据 | Rust 落点 | fit | 阶段 | 状态 |
|---|---|---|---|---|---|
| 事件循环 / 并发模型 | ARCHITECTURE §3（映射表） | tokio（单 worker 起步） | ok | 2 | open |
| socket 选项 / DNS | `src/SocketCore.cc`、`src/NameResolver.cc` | tokio::net + socket2；hickory-resolver 可选（partial） | ok | 2 | open |
| TLS | ARCHITECTURE §2.6 | rustls + rustls-pemfile | ok | 2 | done（TELE-98，真实 CA 校验） |
| 摘要 / CRC / adler32 | ARCHITECTURE §2.6 | RustCrypto + crc32fast；adler32 crate 未核实（U2） | ok/unknown | 2 | open |
| 选项系统 | `src/OptionParser.cc`、`src/OptionHandlerFactory.cc` | `option` | — | 2 | open |
| 日志与统计读出 | ARCHITECTURE §2.6 | `engine`；格式契约 EQ-E3/E4/E19 | — | 2 | open |

口径与判定等级（完全一致 / 归一化后一致 / 不比较）的权威出处是 `docs/EQUIVALENCE.md` §3；本矩阵的「状态」只反映移植进度，不改变等价性判定。

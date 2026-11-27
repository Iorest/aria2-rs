# aria2 架构调研

> TELE-90 交付物 · 调研基线：原版 aria2 `release-1.37.0`（commit `02f2d0d8`，2023-11-15 发布，github.com/aria2/aria2），2026-11-27 克隆取证。
> 依赖选型见 `docs/DEPENDENCIES.md`（同日取证）；rust-anything C++ 限制与替代路径见 TELE-91（`rust/DECISIONS.md`）。

## 1. 总体形态

**双入口**：

- CLI：`src/main.cc` → `src/Context.cc`（参数归并）→ `src/MultiUrlRequestInfo.cc`（组装 RequestGroup 集合并驱动引擎）
- 库：`src/aria2api.cc`（889 行，C API；公开头 `src/includes/aria2/aria2.h`）

**核心引擎**（所有协议单元共享）：

| 概念 | 源码证据 | 说明 |
|---|---|---|
| 事件循环 | `src/DownloadEngine.cc`（657 行） | 单线程事件循环 + Command 队列；`Command::execute()` 返回 EXEC_* 决定重入队。全仓 138 个 `Command.{cc,h}` |
| 事件后端 ×5（编译期选一） | `src/EpollEventPoll.cc`、`src/KqueueEventPoll.cc`、`src/SelectEventPoll.cc`、`src/PollEventPoll.cc`、`src/LibuvEventPoll.cc`；接口 `src/EventPoll.h` | tokio 取代（见 §4，不移植） |
| 任务生命周期 | `src/RequestGroup.cc`、`src/RequestGroupMan.cc` | 单个下载任务 / 全局调度（并发上限、按 host 限速） |
| 分段与分片 | `src/DownloadContext.cc`、`src/SegmentMan.cc`、`src/DefaultPieceStorage.cc` | 多源分块下载的核心数据结构 |
| 文件分配 | `src/FileAllocationMan.h` + `FileAllocation*Iterator` 家族 | falloc / trunc / prealloc |
| 选项系统 | `src/OptionParser.cc`（377 行）、`src/OptionHandlerFactory.cc`（2001 行） | 全部 CLI/RPC 选项定义 |
| 公共工具 | `src/util.cc`（2595 行）、`src/uri.cc`（宽松 URI 解析，C 辅助 `src/uri_split.c`） | util 按用途拆分移植，不整文件搬 |

## 2. 单元划分

五个协议单元 + 一个共享底座。每个单元的文件路径均经 `ls src` 逐一核实（2026-11-27，release-1.37.0）。

### 2.1 HTTP/HTTPS（36 个 `Http*` 文件）

- 客户端链：`HttpRequestCommand` → `HttpConnection` → `HttpRequest`/`HttpResponse` → `HttpDownloadCommand`（Range 续传、分段下载）；头解析 `HttpHeader` + `HttpHeaderProcessor`；空转 `HttpSkipResponseCommand`。
- 代理链：`AbstractProxyRequestCommand`/`AbstractProxyResponseCommand` + `HttpProxyRequestCommand`/`HttpProxyResponseCommand`（GET 经代理），`HttpTunnelRequestCommand`/`HttpTunnelResponseCommand`（CONNECT 隧道，供 HTTPS/FTP 复用）。
- 认证：Basic only。`AuthConfigFactory.cc`、`DefaultAuthResolver.cc`、`NetrcAuthResolver.cc`（netrc 解析 `src/Netrc.cc`）；无 Digest/NTLM 实现。
- Cookie：`src/Cookie.cc`、`src/CookieStorage.cc`、`src/NsCookieParser.cc`（Netscape 格式）、`src/Sqlite3CookieParser.cc`+`Sqlite3CookieParserImpl.cc`（Firefox sqlite 导入）、`src/cookie_helper.cc`。
- 内容解码：`src/GZipDecodingStreamFilter.cc`、`src/GZipEncoder.cc`、`src/GZipFile.cc`（zlib）。aria2 无 brotli 支持（全源码 grep 无 brotli 命中）。
- 服务端（RPC 复用）：见 §2.2。

### 2.2 RPC（JSON-RPC / XML-RPC / WebSocket）

- 内嵌 HTTP 服务端：`HttpListenCommand` → `HttpServer` → `HttpServerBodyCommand`/`HttpServerCommand`/`HttpServerResponseCommand`（复用 §2.1 的头解析与 socket）。
- 方法层：`RpcMethod.cc`、`RpcMethodFactory.cc`、`RpcMethodImpl.cc`（addUri/tellStatus 等全部方法）、`RpcRequest.cc`/`RpcResponse.cc`。
- JSON 通道：自写解析器 `src/JsonParser.cc`（765 行）+ `JsonDiskWriter.h`。
- XML-RPC 通道：SAX 状态机 `XmlRpcRequestParserStateMachine.cc`（245 行）+ `XmlRpcRequestParserController.cc` + `XmlRpcRequestParserState*.cc` + `XmlRpcDiskWriter.cc`，底层 XML 由 `src/XmlParser.cc`（expat）或 `src/Xml2XmlParser.cc`（libxml2）提供（编译期二选一），`src/XmlAttr.cc` 辅助。
- WebSocket 通知：`WebSocketSession.cc`（依赖捆绑库 `deps/wslay/`）+ `WebSocketInteractionCommand`/`WebSocketResponseCommand`/`WebSocketSessionMan.cc`/`NullWebSocketSessionMan.h`。

### 2.3 BitTorrent（77 个 `Bt*` + 111 个 `DHT*` + Peer/Tracker 文件，最大单元）

- Peer Wire 协议：消息族 `BtHandshakeMessage`、`BtBitfieldMessage`、`BtPieceMessage`、`BtChokeMessage`、`BtHaveMessage`、`BtRequestMessage`、`BtCancelMessage`、`BtKeepAliveMessage`、`BtRejectMessage`、`BtAllowedFastMessage`、`BtSuggestPieceMessage` 等（均带 Validator）；连接层 `src/PeerConnection.cc`；协议常量 `src/BtConstants.h`。
- 扩展协议：`ExtensionMessageRegistry.cc`、`DefaultExtensionMessageFactory.cc`、`HandshakeExtensionMessage.cc`、`UTMetadata*ExtensionMessage`（ut_metadata，6 文件）+ `UTMetadataRequestTracker.cc`、`UTPexExtensionMessage.cc`（PEX）。
- 加密（MSE）：`src/MSEHandshake.cc`（577 行）+ `InitiatorMSEHandshakeCommand.cc`/`ReceiverMSEHandshakeCommand.cc`；RC4 与 DH 有 5 套后端，其中**内建纯实现** `src/InternalARC4Encryptor.cc`、`src/InternalDHKeyExchange.cc`（另有 Libgcrypt/Libgmp/Libnettle/Libssl 变体）——Rust 侧参考内建实现移植。
- Tracker：HTTP 通道 `src/BtAnnounce.cc` + `src/AnnounceList.cc`/`AnnounceTier.cc`；UDP 通道 `src/UDPTrackerClient.cc` + `src/UDPTrackerRequest.cc`（BEP-15）。
- DHT（BEP-5）：消息族 `DHTFindNodeMessage`、`DHTGetPeersMessage`、`DHTAnnouncePeerMessage` 及 Reply、`DHTMessageDispatcherImpl.cc`、`DHTMessageFactoryImpl.cc`、`DHTMessageReceiver.cc`、`DHTBucketTree.cc`、`DHTConnectionImpl.cc`、`DHTGetPeersCommand.cc` 等 111 文件；注册 `src/DHTRegistry.cc`。
- 元数据与状态：bencode 解析 `src/BencodeParser.cc`；`src/MetadataInfo.cc`（magnet 元数据）；piece 分配 `src/BtPieceMessage` 侧 + `DefaultPieceStorage`；choke 策略 `BtSeederStateChoke.cc`/`BtLeecherStateChoke.cc`（做种/下载两态）；**控制文件** `src/DefaultBtProgressInfoFile.cc`（440 行，.aria2 二进制格式，契约级）。
- 组装：`src/BtRegistry.cc`、`src/BtSetup.cc`、`src/BtDependency.cc`、`src/BtPostDownloadHandler.cc`。

### 2.4 Metalink（26 个 `Metalink*` 文件）

- 解析：SAX 状态机 `MetalinkParserStateMachine.cc`（554 行）+ `MetalinkParserStateV3Impl.cc`（Metalink 3）/`MetalinkParserStateV4Impl.cc`（Metalink 4/RFC 5854）+ `MetalinkParserController.cc`；底层 XML 复用 §2.2 的 `XmlParser`/`Xml2XmlParser`。
- 模型与映射：`Metalinker.cc`、`MetalinkEntry.cc`、`MetalinkResource.cc`、`MetalinkMetaurl.cc`、`MetalinkHttpEntry.cc`、`Metalink2RequestGroup.cc`（多源 → RequestGroup）、`MetalinkPostDownloadHandler.cc`。校验值复用 MessageDigest（见 §2.6）。

### 2.5 FTP/SFTP

- FTP：`FtpInitiateConnectionCommand` → `FtpNegotiationCommand`（USER/PASV/EPSV/REST 协商）→ `FtpDownloadCommand`/`FtpFinishDownloadCommand`；控制通道解析 `src/FtpConnection.cc`；经 HTTP 代理走 `FtpTunnelRequestCommand`/`FtpTunnelResponseCommand`。主动/被动数据通道直接建 socket，与 SegmentMan 分段调度深度集成。
- SFTP：`SftpNegotiationCommand`/`SftpDownloadCommand`/`SftpFinishDownloadCommand` + `SftpNegotiationConnectChain.h`；SSH 层 `src/SSHSession.cc` 封装 libssh2（configure.ac `PKG_CHECK_MODULES([LIBSSH2])`）。

### 2.6 共享底座

- Socket：`src/SocketCore.cc`（TCP/UDP 封装、SO 选项、绑定接口）。
- DNS：`src/NameResolver.cc`；异步 DNS（c-ares，可选编译）`src/AsyncNameResolver.cc` + `src/AsyncNameResolverMan.cc`。
- TLS ×4 后端（编译期选一）：`src/LibsslTLSContext.cc`（OpenSSL）、`src/LibgnutlsTLSContext.cc`（GnuTLS）、`src/AppleTLSContext.cc`+`AppleTLSSession.cc`、`src/WinTLSContext.cc`+`WinTLSSession.cc`。
- 摘要/校验：`src/MessageDigest.cc`（统一门面）+ 后端 `LibsslMessageDigestImpl.cc`、`AppleMessageDigestImpl.cc`、`LibnettleMessageDigestImpl.cc` + `src/Adler32MessageDigestImpl.cc`（adler32 经 zlib）；CRC32 用于其它校验。
- 大整数：`src/a2gmp.cc`（GMP 可选）、`src/LibgmpDHKeyExchange.cc`。
- 认证入口：`src/AuthConfig.cc`、`src/AuthResolver.h`。
- 可移植性 shim（10 个 `src/*.c`）：`alloca.c`、`asctime_r.c`、`gai_strerror.c`、`getaddrinfo.c`、`gettimeofday.c`、`libgen.c`、`localtime_r.c`、`strptime.c`、`timegm.c`、`uri_split.c` —— Rust/std 已覆盖，除 `uri_split.c`（随 uri.cc 一起移植）外不移植。
- 日志：`src/LogFactory.cc`、`src/Logger.h`；统计输出 `src/ConsoleStatCalc.cc`、`src/SpeedCalc.cc`；Windows 控制台 `src/WinConsoleFile.cc`。
- 特性声明：`src/FeatureConfig.cc`（编译进二进制的特性/默认 CA 路径）。

## 3. 并发模型映射（C++ → Rust）

| aria2 (C++) | aria2-rs (Rust) | 说明 |
|---|---|---|
| `DownloadEngine` + 138 个 Command 状态机 | tokio task + async 状态机 | Command::execute 的 EXEC_* 轮转 → await 点；一次等价性重设计，行为按 replay 契约对齐 |
| `EventPoll` ×5 后端 | tokio runtime（内建 epoll/kqueue/IOCP） | 整体删除，不移植 5 个后端 |
| `SocketCore` | `tokio::net`（TcpStream/UdpSocket）+ `socket2`（SO 选项） | 选项逐条对照（TCP_NODELAY、SO_REUSEADDR、bind） |
| 单线程多路复用 | 默认单 worker runtime 起步 | 对齐"全局并发上限 + 每 host 连接数"语义，避免竞态重放差异；多核扩展留到 PERF 阶段 |
| TLS ×4 后端 | `rustls` 单后端 | 证书/校验行为差异由 EQUIVALENCE 契约裁定（TELE-89） |
| 摘要 4 后端 | RustCrypto 单后端（sha1/sha2/md-5/crc32fast + adler32 待核实） | 同上 |

## 4. 移植顺序依据（验收项：HTTP/HTTPS 与 RPC 先行）

1. **覆盖面**：HTTP(S) 是 aria2 最高频路径，RPC 是集成方的主要程序化入口（aria2.addUri/tellStatus 走 JSON-RPC over HTTP）；先移植这两块即覆盖大多数真实用例，front-door replay 的用例收益最大。
2. **依赖深度（倒逼核心成型）**：HTTP 链贯穿 `DownloadEngine`/`RequestGroup`/`SegmentMan`/`.aria2` 控制文件——先移植 HTTP 会强制把共享底座的接缝做出来，BT/Metalink/FTP 后续直接复用；RPC 又复用 HTTP 服务端代码与引擎，是第二自然落点。
3. **可测性**：本地 fixture 服务器（TELE-92）即可让 HTTP/RPC 的录制用例确定性可重放，不依赖外网；BT 需要 peer/tracker 仿真，FTP/SFTP 需要本地服务，基建更重，故排后。
4. **依赖方向**：BT 元数据可来自 HTTP（HTTP种子/torrent over HTTP）、tracker 走 HTTP、Metalink 文件本身经 HTTP 分发——协议栈自底向上天然是 HTTP 先行。

单元内顺序：HTTP 单源 → 多源分段 → HTTPS（TLS）→ RPC JSON → XML-RPC → WebSocket 通知。对应 Epic 阶段 2/3。

## 5. rust-anything 路线衔接

- C++ 单元（.cc/.h）无单元级 bridge/record（上游 #704 open）→ 全部走 **app replay + 手工契约测试**；细节与 doctor 证据由 TELE-91（`rust/DECISIONS.md` ADR-001）承接，本文档不重复。
- `.c` 辅助文件中，`src/uri_split.c` 是 HTTP/FTP 都依赖的解析辅助，可走完整单元 loop（record→port→gate）；`deps/wslay/*.c`（12 个 .c）是捆绑 WebSocket 库，Rust 侧由 tokio-tungstenite 替代，**不移植**，仅作行为参考。

# aria2 依赖调研与 Rust crate 选型

> TELE-90 交付物 · 取证日 2026-11-27 · 原版基线 aria2 `release-1.37.0`（commit `02f2d0d8`）· crate 数据来自 crates.io API 当日查询。
> fit 定义：**ok**（可直接等价替代）／**partial**（能力部分覆盖或有行为差异，需契约裁定或补充单元）／**none**（无可用 crate，必须手写，且本文档显式登记）／**unknown**（候选存在但等价性/维护性未核实）。

## 1. 规模统计来源（验收项：609 .cc / 560 .h / 22 .c）

统计对象：`git clone --depth 1 --branch release-1.37.0 https://github.com/aria2/aria2.git` 后整仓计数（不含 .git）。复核命令与输出：

```console
$ cd aria2-src
$ find . -path ./.git -prune -o \( -name "*.cc" -o -name "*.h" -o -name "*.c" \) -print \
  | awk -F. '{print $NF}' | sort | uniq -c
  22 c
 609 cc
 560 h
```

逐目录分解（与总数精确对账）：

| 位置 | .cc | .h | .c | 备注 |
|---|---|---|---|---|
| `src/` 顶层 | 415 | 522 | 10 | 主体实现；.c 为可移植性 shim |
| `src/includes/aria2/` | — | 1 | — | 公开 C API 头 `aria2.h` |
| `test/` | 190 | 24 | 0 | cppunit 测试 |
| `deps/wslay/` | 2 | 12 | 12 | 捆绑 WebSocket 库（lib 5 + tests 6 + examples 1 个 .c） |
| `examples/` | 2 | — | — | libaria2ex / libaria2wx |
| `lib/` | — | 1 | — | `gettext.h` |
| **合计** | **609** | **560** | **22** | |

`src/` 内 10 个 .c：`alloca.c`、`asctime_r.c`、`gai_strerror.c`、`getaddrinfo.c`、`gettimeofday.c`、`libgen.c`、`localtime_r.c`、`strptime.c`、`timegm.c`、`uri_split.c`（Rust/std 已覆盖前九个；`uri_split.c` 随 `src/uri.cc` 一起移植，可走 rust-anything 完整单元 loop）。

## 2. 原版外部依赖清单（configure.ac 取证）

| 原版依赖 | 用途 | 证据 |
|---|---|---|
| zlib ≥1.2.3 | HTTP gzip 解码、adler32 | `PKG_CHECK_MODULES([ZLIB])`；`src/GZipDecodingStreamFilter.cc`、`src/Adler32MessageDigestImpl.cc` |
| libxml2 ≥2.6.24 / expat | XML SAX（XML-RPC、Metalink） | `PKG_CHECK_MODULES([LIBXML2])/[EXPAT]`；`src/XmlParser.cc`/`src/Xml2XmlParser.cc` |
| OpenSSL ≥0.9.8 或 GnuTLS ≥2.2.0 | TLS + 摘要 | `PKG_CHECK_MODULES([OPENSSL])/[LIBGNUTLS]`；`src/LibsslTLSContext.cc` 等 |
| nettle + GMP | 备用加密后端 / 大整数（DH） | `PKG_CHECK_MODULES([LIBNETTLE])`、`AC_CHECK_LIB([gmp])`；`src/a2gmp.cc` |
| libssh2 | SFTP | `PKG_CHECK_MODULES([LIBSSH2])`；`src/SSHSession.cc` |
| libcares ≥1.7.0（可选） | 异步 DNS | `PKG_CHECK_MODULES([LIBCARES])`；`src/AsyncNameResolver.cc` |
| sqlite3（可选） | Firefox cookie 导入 | `PKG_CHECK_MODULES([SQLITE3])`；`src/Sqlite3CookieParser.cc` |
| libuv ≥1.13（可选） | 事件后端之一 | `PKG_CHECK_MODULES([LIBUV])`；`src/LibuvEventPoll.cc` |
| tcmalloc / jemalloc（可选） | 分配器 | `PKG_CHECK_MODULES([TCMALLOC])/[JEMALLOC]` |
| WinTLS（crypt32/secur32） | Windows TLS 后端 | `src/WinTLSContext.cc` |
| cppunit ≥1.10.2（测试） | 单元测试 | `PKG_CHECK_MODULES([CPPUNIT])` |
| wslay（捆绑，非系统依赖） | WebSocket RPC | `deps/wslay/`；`src/WebSocketSession.cc` |

## 3. Rust crate 候选与 fit（crates.io 2026-11-27 取证）

| 能力 | crate 候选 | 当日版本/更新 | fit | 说明与替代路径 |
|---|---|---|---|---|
| 事件循环/并发 | tokio | 1.54.1 / 2026-10-21 | ok | 取代 DownloadEngine 事件模型与 5 个 EventPoll 后端 |
| CLI 参数解析（前门阶段） | clap | 4.6.10 / 2026-11-27 锁定 | ok | `rust/app` 前门按录制品生成命令/flag 解析器；阶段 2 起逐单元接管，阶段 7 前不得把前门当最终 CLI |
| socket 选项 | socket2 | 0.6.5 / 2026-11-24 | ok | TCP_NODELAY、SO_REUSEADDR、bind 逐项对照 |
| TLS | rustls + rustls-native-certs | 0.24.0-dev.3 / 2026-11-12；0.8.5 / 2026-10-13 | ok | 收敛 4 个 TLS 后端为 1；证书行为差异交 EQUIVALENCE 契约 |
| DNS | tokio::net::lookup_host（getaddrinfo） | — | ok | 等价性基线（与原版默认路径一致） |
| 异步 DNS（--async-dns 对应） | hickory-resolver | 0.26.4 / 2026-11-24 | partial | 行为 ≠ c-ares；解析结果差异需契约裁定，列为可选后端 |
| JSON | serde + serde_json | 1.0.229 / 2026-07-18；1.0.154 / 2026-10-11 | ok | 取代自写 JsonParser |
| XML SAX | quick-xml | 0.42.0 / 2026-08-22 | ok | 取代 expat/libxml2 双后端（pull 型，对应 SAX 状态机） |
| XML-RPC | xmlrpc | 0.15.1 / 2021-11-02 | **none** | 客户端专用且 3 年未更新；服务端按 aria2 方言手写（RPC 单元，quick-xml 之上），不做静默替代 |
| WebSocket | tokio-tungstenite | 0.30.0 / 2026-07-11 | partial | 取代捆绑 wslay；aria2 会话/通知行为对齐依赖 TELE-97 replay，标注 unknown until replay |
| bencode | bendy；serde_bencode | 0.6.1 / 2025-11-10；0.2.4 / 2023-10-09 | ok / partial | 首选 bendy（维护中）；serde_bencode 备选但停滞 3 年 |
| 摘要 sha1/sha256/sha512 | sha1、sha2 | 0.11.0 / 2026-07-10；0.11.2 / 2026-11-25 | ok | Metalink/BT/校验共用 |
| 摘要 md5 | md-5 | 0.11.0 / 2026-03-27 | ok | Metalink/BT 用 |
| CRC32 | crc32fast | 1.5.6 / 2026-11-24 | ok | — |
| adler32 | （候选 `adler`） | 未核实 | **unknown** | 原版经 zlib adler32 提供；选型前需核实 crate 维护性，否则 30 行手写并记 ADR |
| MSE RC4 | rc4 | 0.2.0 / 2026-03-30 | ok | 参考移植 `src/InternalARC4Encryptor.cc`（原版自带纯实现） |
| MSE DH | num-bigint | 0.5.1 / 2026-07-05 | ok | 参考移植 `src/InternalDHKeyExchange.cc`；无现成 MSE crate（见 §4） |
| gzip/zlib | flate2 | 1.1.11 / 2026-11-06 | ok | 取代 zlib（GZip* 与 adler32 之源） |
| SQLite（cookie 导入） | rusqlite（bundled） | 0.43.0 / 2026-11-15 | ok | 可选 feature |
| FTP 客户端 | suppaftp | 13.0.0 / 2026-11-17 | partial | 控制通道协议可参考；但 aria2 的数据通道直连 socket 且与分段调度集成（`FtpNegotiationCommand` → `FtpDownloadCommand`），数据面按单元手写在 tokio socket 上，suppaftp 仅作协议参考，不引入运行时依赖 |
| SFTP | russh + russh-sftp；ssh2（弃） | 0.65.4 + 4.0.0 / 2026-11-21；0.9.7 / 2026-11-10 | partial | russh 纯 Rust 优先；ssh2 是 libssh2 C 绑定，违反纯 Rust 目标，**弃用**；libssh2 行为差异（认证方式、SFTP 语义）待 replay/契约核对 |
| URI 解析 | url | 2.5.9 / 2026-11-20 | partial | url 是 WHATWG 严格解析，aria2 是宽松自写解析（`src/uri.cc` + `src/uri_split.c`）；解析器按 uri.cc 移植为单元，url 仅作参考对照 |
| 基础编码 | base64、hex、percent-encoding | 0.23.1、0.4.3、2.3.2 | ok | hex 久未更新但稳定通用 |
| 随机（peer ID/节点 ID） | getrandom | 0.4.3 / 2026-06-17 | ok | — |
| 文件系统底层（falloc 等） | rustix（或 libc 0.3，勿用 libc 1.0-alpha） | 1.1.6 / 2026-10-27 | ok | fallocate/truncate 对照原版 FileAllocation*Iterator |
| mmap（--enable-mmap） | memmap2 | 0.9.12 / 2026-10-21 | ok | — |

## 4. fit=none / unknown 显式登记（验收项：不许静默手写）

以下能力无可用 crate（或等价性未核实），**必须**以具名单元手写并在 `rust/DECISIONS.md` 记 ADR；本文档即登记入口：

| # | 能力 | fit | 手写单元（具名） | 参考源码 | 阶段 |
|---|---|---|---|---|---|
| N1 | BT 全栈：Peer Wire 消息、choke 策略、PEX、ut_metadata | none | `bt-peer` | `src/Bt*.cc`（77 文件）、`src/PeerConnection.cc` | 4 |
| N2 | DHT（BEP-5） | none | `bt-dht` | `src/DHT*.cc`（111 文件） | 4 |
| N3 | UDP tracker（BEP-15） | none | `bt-tracker` | `src/UDPTrackerClient.cc` | 4 |
| N4 | MSE 加密握手 | none | `bt-mse`（rc4 + num-bigint 组合） | `src/MSEHandshake.cc` | 4 |
| N5 | XML-RPC 服务端（aria2 方言） | none | `rpc-xml`（quick-xml 之上） | `src/XmlRpcRequestParserStateMachine.cc` 等 | 3 |
| N6 | .aria2 控制文件二进制格式 | none | `control-file` | `src/DefaultBtProgressInfoFile.cc`（440 行） | 2 起，契约级 |
| N7 | Metalink 3/4 语义（状态机） | none（作为 crate） | `metalink`（quick-xml + sha*/md-5 组合，非静默——组合件已在上表登记） | `src/MetalinkParserStateMachine.cc` 等 26 文件 | 5 |
| N8 | 宽松 URI 解析 | none（url crate 不等价） | `uri-parse` | `src/uri.cc`、`src/uri_split.c` | 2 |
| N9 | RPC 方法层（约 30 个方法语义） | none | `rpc-methods`（serde_json 之上） | `src/RpcMethodImpl.cc` | 3 |
| U1 | gettext 本地化消息 | unknown | 等 TELE-89 契约定英文基线；若需本地化再议 i18n 方案 | `src/usage_text.h`、`lib/gettext.h` | — |
| U2 | adler32 crate | unknown | 核实 `adler` 维护性，否则手写 | `src/Adler32MessageDigestImpl.cc` | 2 |
| U3 | WebSocket 会话/通知行为对齐 | unknown until replay | 以 TELE-97 录制用例裁定 tokio-tungstenite 偏差 | `src/WebSocketSession.cc` | 3 |

## 5. 明确不需要 / 弃用的候选

- **ssh2（C 绑定）**：违反纯 Rust 目标，弃用（见 §3 SFTP 行）。
- **brotli**：aria2 全源码无 brotli 支持，不引入。
- **HTTP Digest 认证**：原版 Basic only（`AuthConfigFactory.cc` 无 Digest 实现），不引入 digest 认证 crate。
- **SOCKS 代理**：原版不支持（grep 无 socks 命中），不引入。
- **tcmalloc/jemalloc**：Rust 无需，不移植。
- **libuv / cppunit**：分别被 tokio、cargo test 取代。
- **GMP/nettle/Libgcrypt/Libssl 加密后端**：RustCrypto 单后端取代；原版自带纯实现（`InternalARC4Encryptor.cc`/`InternalDHKeyExchange.cc`）作为行为参考。
- **`libc` 1.0-alpha**：alpha 版，锁 0.3.x 或用 rustix。

## 6. 依赖纪律

1. 新增依赖必须在本表登记 fit 与理由；fit=none/unknown 不许静默手写替代——手写即记 ADR（TELE-91 文件 `rust/DECISIONS.md`）。
2. 每个协议单元的手写范围以上表 + `docs/ARCHITECTURE.md` §2 为准；单元级 claim 规则见 TELE-95 的 `rust/MAP.md`。
3. crate 版本与更新日期为 2026-11-27 快照，落地时以当时的 `cargo add` 结果为准并复核 fit。

# TEST-ENV — 测试环境与夹具规格

版本 v0.2（TELE-97 实测回填）：§2 `--fresh` 布局已实测；§3 增截断端点与无 Caddy 时的等价回退服务器、FTP 改用线程化 launcher；§7 解法 A 已落地；§8 录制/回放命令补齐实测归一化与完整 `--build`。本文是规格：它描述的 `test-env/` 脚本（setup.sh / serve.sh / Caddyfile）、夹具生成器与 `cases/` 由阶段 1（TELE-97）落地实现，本文是其验收契约；TELE-96 的 CI 设计引用本文 §6/§7 作为闸门。

## 1. 目的与边界原则

### 1.1 目的

为 rust-anything whole-app 流程（`analyze --whole-app` → `app record` → `app replay` → `app front-door` → 逐单元 `app wire` → `app lift`）提供：

1. 一个完全本地、确定性的网络世界：HTTP/HTTPS/FTP 三协议测试服务器与证书；
2. 目录与凭据隔离边界，保证录制/回放既不泄漏生产凭据、也不污染用户环境；
3. 阶段 1（TELE-97 录制原版基线）可直接执行、可复现的启动命令（§8）。

### 1.2 录制原则（验收项 1）

「录制跑真实代码、只用副本 + 本地测试服务器」：

- **真实代码**：`app record` / `app replay` 跑的是 `env/aria2-1.37.0/bin/aria2c` —— 从 aria2 官方 tag `release-1.37.0` 源码构建的原版二进制。不用 mock、stub、包装器或包管理器安装的二进制（保证行为与被读的源码一一对应、可溯源）。
- **只用副本**：全部输入文件来自本仓 `test-env/fixtures/`（自制确定性副本）。需要结构化样本（metainfo、Metalink XML、种子文件）时，自制或取自 aria2 源码树 `test/` 自带夹具（同 GPL-2.0，引用处标注原路径）。**禁止**把用户机器上的真实下载产物或真实种子站元数据当输入。
- **只连本地**：所有 URI/主机只能是 `127.0.0.1` / `localhost`，连接对象只有 §3 的本地测试服务器。**禁止外网真实下载**（验收项 2），强制手段见 §6 三道防线。
- **跑两次**：`app record` 对每个用例连跑原版两次并归类噪声（values / order / lines）；进度行、时间戳的归一化口径以 `docs/EQUIVALENCE.md`（TELE-89）为准，本文不重复定义。

## 2. 目录布局与隔离

```
aria2-rs/
├── env/                       # gitignore：第三方构建产物
│   ├── aria2-src/             # 原版源码 clone（pinned release-1.37.0）
│   └── aria2-1.37.0/          # make install 前缀 → bin/aria2c
├── test-env/
│   ├── setup.sh               # 生成证书+夹具+凭据，校验 manifest（幂等）
│   ├── serve.sh               # start | stop | check 本地服务器
│   ├── Caddyfile              # HTTP/HTTPS 服务器定义
│   ├── ports.env              # 端口配置（默认 §3 表）
│   ├── certs/                 # gitignore：ca.crt / ca.key / server.crt / server.key
│   ├── fixtures/              # gitignore：确定性夹具（生成物）
│   │   ├── http/              # HTTP 与 HTTPS 共用根
│   │   └── ftp/
│   ├── manifest.json          # 夹具名/大小/SHA-256，入 git，setup 幂等性闸门
│   ├── ports.env              # 入 git：端口 + 固定合成 FTP 常量（TELE-96 §7-2 解法 A）
│   ├── home/                  # gitignore：空 HOME（隔离 $HOME）
│   └── run/                   # gitignore：服务器日志等运行残留
├── cases/                     # app record --inputs JSONL（入 git；不含凭据）
├── downloads/                 # gitignore：--fresh 快照模板（.gitkeep 占位）
└── rust/app-cases/            # app record 产出（trycmd 文件），入 git
```

隔离规则：

1. **进程隔离**：录制命令以 `HOME=<repo>/test-env/home`（空目录）启动，并显式 `--no-conf` 双保险。rust-anything 的 replay sandbox 不继承该 HOME；阶段 2 起 G1/G2 统一用 `test-env/replay-home.sh` 作为 `--build` 前缀设置该 HOME，使原版与前门在同一隔离环境下对比。**产品代码不改子进程 HOME**（`original::run` 保持真实用户环境，避免破坏 `~/.netrc`）；wrapper 只覆盖回放/测试层，未使用 wrapper 的真实运行不受影响。
2. **写隔离**：aria2c 一律 `--dir=downloads`（相对 cwd）；`.aria2` 控制文件与日志都落在该子树，不落 repo 其他位置、不落 `$HOME`。
3. **case 隔离**：`app record --fresh <路径>` 把 fixtures / certs / downloads 快照进每个用例独立的全新工作目录，用例结束后对「写了什么 / 删了什么」做 diff —— 这是下载产物比对（EQUIVALENCE §下载产物）的机制基础。
   ✅ **TELE-97 实测**：布局不扁平化——快照落在 `<case>/files/test-env/fixtures/...`、`files/test-env/certs/...`、`files/downloads/...`，cases 与 `--ca-certificate` 的相对路径无需改写。

## 3. 本地测试服务器

| 服务 | 默认端口 | 绑定 | 根目录 | 契约 |
|---|---|---|---|---|
| HTTP | 18080 | 127.0.0.1 | fixtures/http | HTTP/1.1，**Range 请求必须回 206**（多连接分块下载的前提），keep-alive，支持 HEAD |
| HTTPS | 18443 | 127.0.0.1 | fixtures/http | 同上 + §4 证书；客户端走真实证书校验路径 |
| FTP | 1821 | 127.0.0.1 | fixtures/ftp | 禁匿名，合成凭据（§7），被动模式，只读 |
| HTTP 截断 | 18080/18443（`/trunc/*` 代理到 18081） | 127.0.0.1 | fixtures/http | `200` + 全长 `Content-Length`，发 N 字节后断开；cus-08 中断落 `.aria2`（RECORD-CASES §7-1） |
| FTPS | 1822 | 127.0.0.1 | fixtures/ftp | 阶段 6 启用（显式 TLS，同一证书） |

- HTTP/HTTPS 用 **Caddy** `file_server`（底层 `http.ServeContent`，原生支持 206 / If-Range），定义在 `test-env/Caddyfile`：

  ```
  :18080 {
      root * test-env/fixtures/http
      file_server
  }
  :18443 {
      root * test-env/fixtures/http
      tls test-env/certs/server.crt test-env/certs/server.key
      file_server
  }

  ```

`/trunc/<file>/<bytes>` 由 Caddy `reverse_proxy` 转到 `test-env/trunc.py`（仅 loopback）；普通文件仍由 `file_server` 提供。
若本机没有 Caddy（Homebrew bottle 下载不可用时），`serve.sh` 自动改用 `test-env/httpserver.py`：同一夹具目录、同源证书、`Range → 206`、HEAD、keep-alive 与 `/trunc/<file>/<bytes>` 行为等价；Caddy 仍在 `Caddyfile` 中作为首选实现。

- FTP 用 **pyftpdlib 线程服务器**（只读，无写权限）：`python3 test-env/ftpd.py` 直接使用 `ThreadedFTPServer`，凭据取自 `test-env/ports.env`（§7），根目录 `test-env/fixtures/ftp`。不用 `python3 -m pyftpdlib` CLI：其 `--concurrency multi-thread` 在 pyftpdlib 2.2.0 会向 `serve_forever` 传不支持的 `worker_processes`，而默认 async server 在 185 例并行回放时会偶发挂住 FTP 控制连接。
- 端口可经 `ports.env` 覆盖；任何改动必须同步 cases 与本文表格。
- 启动断言（`serve.sh check`）：`lsof -nP -iTCP:<port> -sTCP:LISTEN` 输出必须仅含 `127.0.0.1`，出现 `*:port` 即失败。
- 服务器访问日志写 `test-env/run/serve.log`，作为「录制期间只有 loopback 访问」的旁证。

## 4. 证书生成

证书只服务本测试环境，**严禁导入系统信任链**（不动 macOS 钥匙串，不用 `caddy trust`）。信任的唯一方式是 aria2c 的 `--ca-certificate=test-env/certs/ca.crt`。

```sh
# setup.sh 调用本仓的可复现生成器：固定种子 → 同一密钥对/信任锚；ECDSA 签名随机，证书字节不逐字节相同
python3 test-env/gen_certs.py   # 写 test-env/certs/{ca,server}.{crt,key}
```

规则：私钥全部留在 `test-env/certs/`（gitignore）；证书校验保持 aria2 默认 `--check-certificate=true`（走真实校验路径），**禁止出现 `--check-certificate=false`** 的用例——TLS 等价性不能靠关掉校验绕过。

## 5. 夹具清单与确定性

由 `test-env/setup.sh` 内嵌生成器产出：256 字节循环 pattern（`bytes(range(256))` 重复），跨机确定 → SHA-256 稳定。

| 文件（fixtures/http/） | 大小 | 用途 |
|---|---|---|
| empty.bin | 0 B | 空文件边界（exit 0、产物为空文件） |
| 1kib.bin | 1 KiB | 最小单连接 |
| 1mib.bin | 1 MiB | 单/多连接基础 |
| 20mib-plus1.bin | 20 MiB + 1 B | 跨越 aria2 默认 `--min-split-size` 20M 边界 → 强制第二连接 |
| 21mib.bin | 21 MiB | 多连接常规 |

`fixtures/ftp/`：1kib.bin、1mib.bin（同 pattern）与 `secret/1kib.bin`（凭据路径用例）；无凭据访问 → 530 失败用例。

- `manifest.json` 记录 `{"文件名": {"size", "sha256"}}`，**入 git**；`setup.sh` 每次重新生成并执行 `git diff --exit-code test-env/manifest.json` —— 确定性闸门，夹具漂移即失败。
- 夹具本体不进 git（生成物），经 `--fresh` 快照进每个 case（§2.3）。
- 阶段扩展点：RPC（阶段 3）复用 HTTP 端口（aria2c `--enable-rpc` 起 loopback 服务端，按工具语义用 `--http` 录制）；BT（阶段 4）增本地 tracker 与自制 metainfo；Metalink（阶段 5）增自制 XML。均遵循 §1.2「只用副本」原则，各阶段 ADR（`rust/DECISIONS.md`，TELE-91 起）产出后回填本节链接。**阶段 3 已落地**：`rpc-cases/` 独立录制根，服务端只绑 127.0.0.1，合成 token `local-rpc-test-only` 仅用于 loopback；CI 原版构建启用 expat 以保证 `/rpc` 端点存在（aria2@1.37.0 configure.ac:623-633）。

## 6. 网络边界与强制手段（验收项 2 之一）

三道防线，全部自动化：

1. **拓扑防线**：三个服务器只绑 127.0.0.1（§3）；`serve.sh check` 断言监听地址仅 loopback。
2. **死代理防线**：所有录制的 aria2c 命令统一前缀：

   ```
   --all-proxy=http://127.0.0.1:9 --no-proxy=127.0.0.1,localhost --max-tries=1 --retry-wait=0
   ```

   语义：除 loopback 外的一切 http/https/ftp 目标都会撞上死代理立刻失败 —— 任何意外的外网企图以显式错误收场，不可能静默成功。
3. **用例白名单防线**：cases 与录制品只允许 loopback URI；CI/评审闸门（并入 TELE-96 的 replay-report 流程）：

   ```sh
   grep -rnE '(https?|ftp)://' rust/app-cases cases \
     | grep -vE '127\.0\.0\.1|localhost' \
     && echo 'FAIL: non-loopback URI' && exit 1 || true
   ```

DHT/PEX：阶段 1–3 不传任何 DHT/BT 参数（aria2 默认关闭）；阶段 4 启用本地 tracker 时另立 ADR，并纳入防线 1/3。

## 7. 凭据边界（验收项 2 之一）

- **固定合成常量入 git**：`test-env/ports.env` 记录 `FTP_USER` / `FTP_PASS` 两个测试专用值（仅 loopback、无真实数据）。这是 TELE-96 §7-2 解法 A，经 owner 2026-11-27 评论「同意 §7 解法 A」批准；`args` 是字面量，`$VAR` 不会展开，因此凭据随 `cases/stage1.jsonl` 明文写入。FTPS（阶段 6）沿用同一常量。
- **不再随机轮换**：`creds.json` 不再服务 FTP 用例（无需 `env` 注入）；轮换合成常量必须同步改 `ports.env` + `cases/stage1.jsonl` + `serve.sh`，等同于重录 golden。
- **不用 netrc**：不传 `--netrc`；§2.1 的空 HOME 隔离是双保险。
- **录制品 scrub**（入评审 checklist）：提交 `rust/app-cases/` 前运行：

  ```sh
  grep -rnE 'creds\.json|BEGIN [A-Z ]*PRIVATE KEY' rust/app-cases cases \
    && echo 'FAIL: credential leak' && exit 1 || true
  ```

  注意：上一版模式含 `aria2test`，会把已批准的合成常量误报为泄漏；按解法 A 去掉。真实私钥或 `creds.json` 字样仍为硬失败。

## 8. 阶段 1 可复现启动命令（验收项 3）

前提：macOS + Homebrew；所有命令在 repo 根执行。

```sh
# (0) 一次性环境准备
python3 -m pip install pyftpdlib cryptography

# (1) 构建原版 aria2（pinned release-1.37.0 → env/aria2-1.37.0/bin/aria2c）
git clone --depth 1 --branch release-1.37.0 https://github.com/aria2/aria2.git env/aria2-src
cd env/aria2-src && autoreconf -i \
  && PKG_CONFIG_PATH="$(brew --prefix openssl@3)/lib/pkgconfig" \
     CPPFLAGS="-I$(brew --prefix openssl@3)/include" \
     LDFLAGS="-L$(brew --prefix openssl@3)/lib -framework Security" \
     ./configure --prefix="$PWD/../aria2-1.37.0" \
       --with-openssl --without-appletls --without-gnutls --without-libxml2 \
       --without-libexpat --disable-nls \
  && make -j"$(sysctl -n hw.ncpu)" && make install && cd ../..
shasum -a 256 env/aria2-1.37.0/bin/aria2c   # 结果记入 PR 描述

# (2) 生成证书 / 夹具 / 凭据（幂等；重跑安全）
test-env/setup.sh            # 末步: git diff --exit-code test-env/manifest.json

# (3) 起本地服务器并断言边界
test-env/serve.sh start && test-env/serve.sh check
lsof -nP -iTCP:18080 -sTCP:LISTEN   # 预期: 仅 127.0.0.1:18080

# (4) 全库分析与录制（真实原版二进制 + 本地服务器）
rust-anything analyze . --whole-app -o rust/plan.md
rust-anything app record . --timeout 120 \
  --normalize timestamps \
  --normalize 'date=\b\d{2}/\d{2}\b' \
  --normalize 'speed=\s+(?:n/a|[0-9]+(?:\.[0-9]+)?[KMGTP]?i?B/s)' \
  --normalize 'src-line=\[[A-Za-z0-9_.-]+\.cc:\d+\]' \
  --normalize "argv0=(?m)^[^\\n]*aria2c: unrecognized option [\`']" \
  --fresh test-env/fixtures --fresh test-env/certs --fresh downloads \
  --inputs cases/stage1.jsonl \
  -- env/aria2-1.37.0/bin/aria2c \
     --no-conf --dir=downloads --allow-overwrite=true --auto-file-renaming=false \
     --all-proxy=http://127.0.0.1:9 --no-proxy=127.0.0.1,localhost \
     --max-tries=1 --retry-wait=0 \
     --ca-certificate=test-env/certs/ca.crt
test-env/scrub-cases.sh       # 删除录制品中的 *.key 并以 PKCS#8 正则复核
python3 test-env/normalize-progress.py  # N3/N9：通配进度读出/摘要行
python3 test-env/restore-tables.py      # 恢复成功用例结果表的 OK/路径/图例逐字断言


# (5) 回放：原版与 Rust 前门都必须 100%
# --build 会替换整条运行命令，因此原版必须把 (4) 的公共参数一并引号包裹传入
RUN='env/aria2-1.37.0/bin/aria2c --no-conf --dir=downloads --allow-overwrite=true --auto-file-renaming=false --all-proxy=http://127.0.0.1:9 --no-proxy=127.0.0.1,localhost --max-tries=1 --retry-wait=0 --ca-certificate=test-env/certs/ca.crt'
rust-anything app replay . --build "$RUN"          # 预期: 185/185，exit 0
rust-anything app front-door .                      # 生成 rust/app 并回放
rust-anything app replay . --build rust/app/target/debug/aria2c  # 预期: 185/185, fallback 100%
```

说明：

- `cases/stage1.jsonl`：每行一个用例 `{"name", "args": [...]}`（必要时含 `"env"` / `"files"`），URI 全部指向 127.0.0.1；用例清单来源与阶段 1 验收标准见 TELE-96（0.8）。
- record 对每例连跑两次并自动归类噪声；两跑一致的固定值（时间戳、零速 `n/a`）不会自动通配，replay 跨分钟运行必然失配，故 §8 显式加 `timestamps`、`speed`、`src-line`、`argv0` 四条 normalizer（在比对两侧把值替换为 `[..]`，不改录制品原文）。`-j2` 并发下载行的顺序抖动由 `man-05` 用例的整行 `[..]` 吸收；macOS 独有的 `Allocating disk space` 提示由 `man-03` 的 `...` 吸收。原版必须在 macOS 上用 `--without-appletls --with-openssl` 构建，否则 AppleTLS 忽略 `--ca-certificate`，HTTPS 正向用例会被录成 TLS 失败。
- 用例失败时先读 `rust/app-cases/` 报告里的 diff，再单 case 复跑：`rust-anything app replay . --case <name>`。
- **CI 构建输入（TELE-97 实测修订）**：GitHub Actions 的 Ubuntu runner 不能用 `autoreconf` 从源码 clone 构建——aria2 1.37.0 自带 gettext 0.18 的 `po/Makefile.in.in` 与 runner 的 0.22 宏冲突（`make` 报 `check-macro-version` Error 1；`autopoint --force` 亦无效，连续两轮）。按 owner 授权改用同一 tag 的官方 release tarball：`https://github.com/aria2/aria2/releases/download/release-1.37.0/aria2-1.37.0.tar.xz`，SHA-256 `60a420ad7085eb616cb6e2bdf0a7206d68ff3d37fb5a956dc44242eb2f79b66b`，校验后直接 `./configure`（无 autoreconf）。CI 与本地都用 OpenSSL 而非 AppleTLS；证书由 `gen_certs.py` 确定性生成，`scrub-cases.sh` 保证录制品不含私钥。
- 构建依赖以 aria2 官方 README 为准；`--without-libssh2` 是阶段 1 基线，阶段 6 重建（加 `--with-libssh2`）后重录 FTP/SFTP 用例并在 `rust/DECISIONS.md` 记 ADR。

## 9. 验收对照

| TELE-92 验收项 | 本文落点 |
|---|---|
| 明确「录制跑真实代码、只用副本 + 本地测试服务器」 | §1.2、§2.3、§8 (4)(5) |
| 禁止生产凭据、禁止外网真实下载 | §6（三道防线）、§7（凭据边界） |
| 给出阶段 1 可复现的启动命令 | §8 全块 |

## 10. 关联

- `docs/EQUIVALENCE.md`（TELE-89）：归一化口径与测试归属；本文是它的运行环境前提。
- `rust/DECISIONS.md`（TELE-91 待产出）：C++ 单元替代路径；产出后回填 §5 阶段扩展点。
- TELE-96（0.8 阶段 1 验收清单与 CI 设计）：replay-report 的 CI 闸门引用本文 §6 白名单检查与 §7 scrub 检查。
- rust-anything 4.2.0 `app record --help` / `app replay --help`（本机实测 2026-11-27）：`--fresh` / `--inputs` / `--normalize` / `--case` 语义出处。

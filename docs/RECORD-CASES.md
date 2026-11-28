# RECORD-CASES — 首轮 app record 用例清单与 CI 比对方案

TELE-96（0.8）交付物。阶段 1 任务（TELE-97）可直接引用 §3 与 §6 作为验收标准。

> **v0.2**（owner「继续」指示后补测）：§4 归一化机制改为实测结论——具名 regex 无效、依赖工具双跑自动噪声通配；§7-2 解法 B 已排除；§7-1 给出截断端点契约草案；§5 补 `--build` 单词引号注意项。
> **v0.3**（owner 批准）：§7-2 解法 A 获 owner 明确授权（「同意 §7 解法 A」，2026-11-27 本 issue 评论），状态改已批准并补执行要点。
> **v0.4**（TELE-97 实测回填）：§4 归一化改为「自动噪声 + 两个具名 normalizer」；§5 G1 命令必须整体引号包裹完整运行串；§6 勾选清单按实测结果更新。

## 1. 范围与前提

- 录制对象：`env/aria2-1.37.0/bin/aria2c`（TEST-ENV §1.2，pinned `release-1.37.0` 源码自建）；工具 rust-anything 4.2.0。
- 录制形态：CLI 模式（`app record`）。RPC 服务端用 `--http` 形态另录，属阶段 3（TEST-ENV §5 阶段扩展点）；BT、Metalink 夹具分属阶段 4、5。**首轮只录 fixtures 已具备的 HTTP/HTTPS/FTP**。
- 工具语义（4.2.0 实测，见 §8）：`app record` 把每个用例存成 trycmd 文件（`<case>.toml` / `.stdout` / `.stderr`）+ `files/` 快照 + `rust-anything.json` 清单，落在 `rust/app-cases/`；`app replay` 跑同一批用例，全绿 exit 0、任一失败 exit 1，并把报告写到 **`rust/app-cases/replay-report.md`**（含 per-case diff 与 Fallback rate）。
- 录制命令即 TEST-ENV §8(4)：公共参数（`--no-conf --dir=downloads --allow-overwrite=true --auto-file-renaming=false` 与死代理防线前缀、`--ca-certificate`）由该命令统一注入，**用例 args 不重复**。

## 2. 用例来源与编号

| 来源 | 说明 | 编号前缀 |
|---|---|---|
| S1 手册 EXAMPLE 改编 | aria2@1.37.0 `doc/manual-src/en/aria2c.rst` §EXAMPLE（一手来源）。改编规则：URI 主机 → `http://127.0.0.1:18080/<夹具>`（HTTPS 18443、FTP 1821 按 TEST-ENV §3）；文件名 → TEST-ENV §5 夹具表 | `man-NN` |
| S2 自定义 inputs | `cases/stage1.jsonl`，覆盖错误路径与边界——手册没有、但等价性契约（EQUIVALENCE E1–E12）要求的面 | `cus-NN` |
| S3 语义陷阱 | rust-anything 自动生成（不加 `--no-traps`），覆盖参数边界；不计入手工清单，但计入验收总数 | （工具命名） |

工具「its own examples」取自二进制 help/文档；aria2c 的示例在手册里，预期录制输出为 `0 from examples`，S1 全部以 JSONL 手工转录（录制时核对 `N from inputs / K traps` 行）。

## 3. 首轮用例清单（`cases/stage1.jsonl`）

每行 `{"name", "args": [...]}`（必要时含 `"env"` / `"files"`）；URI 全部 loopback（TEST-ENV §6 白名单）。错误退出码不在清单里预设——golden 以原版录制为准（E1：每个错误类断言一个码）。

### 3.1 S1 手册 EXAMPLE 改编（14 条）

| # | 用例名 | 手册出处（§EXAMPLE 小节） | args 追加 | env / files | 主要口径 |
|---|---|---|---|---|---|
| 1 | man-01-single-http | HTTP/FTP Segmented Downloads › Download a file | `http://127.0.0.1:18080/1mib.bin` | — | E2/E3/E8 |
| 2 | man-02-two-uris | › from two different HTTP servers | `http://127.0.0.1:18080/1mib.bin http://localhost:18080/1mib.bin` | — | E2/E8；N5 端口归一验证 |
| 3 | man-03-multi-conn | › one host using multiple connections | `-x2 -k1M http://127.0.0.1:18080/21mib.bin` | — | E2/E3/E8 |
| 4 | man-04-http-ftp | › from HTTP and FTP servers at the same time | `http://127.0.0.1:18080/1kib.bin ftp://127.0.0.1:1821/1kib.bin` | FTP 凭据（§7-2 冲突待解） | E2/E8 |
| 5 | man-05-input-j2 | › files listed in a text file concurrently | `-i files.txt -j2` | files: `files.txt`（两行 loopback URI：1kib.bin、empty.bin） | E2/E8/E9 多产物 |
| 6 | man-06-cookies | More advanced HTTP features › Load cookies | `--load-cookies cookies.txt http://127.0.0.1:18080/1kib.bin` | files: `cookies.txt`（Netscape 格式自制） | E2/E8 |
| 7 | man-07-resume-partial | › Resume download started by web browsers | `-c -s2 http://127.0.0.1:18080/1mib.bin` | files: `1mib.bin` 前 64 KiB 部分副本 | E8/E11 无控制文件续传 |
| 8 | man-08-checksum-ok | › Verifying checksums | `--checksum=sha-1=<1kib.bin 真值> http://127.0.0.1:18080/1kib.bin` | 真值随夹具 manifest 确定，可入 cases | E1/E2/E8 |
| 9 | man-09-checksum-bad | 同上 | digest 改一位 | — | E1 错误码 golden |
| 10 | man-10-param-uri | › Parameterized URI support | `-Z -P "http://127.0.0.1:18080/{1kib.bin,empty.bin}"` | — | E2/E8/E9 |
| 11 | man-11-https-single | 改编自 Download a file（协议替换 https） | `https://127.0.0.1:18443/1mib.bin` | — | E8 真实证书校验路径 |
| 12 | man-12-https-multi | 改编自 multiple connections（协议替换） | `-x2 -k1M https://127.0.0.1:18443/21mib.bin` | — | E3/E8 |
| 13 | man-13-ftp-single | 改编自 HTTP and FTP（拆出 FTP 单协议） | `ftp://127.0.0.1:1821/1mib.bin` | FTP 凭据 | E2/E8 |
| 14 | man-14-ftp-secret | 改编自 Download a file（路径替换） | `ftp://127.0.0.1:1821/secret/1kib.bin` | FTP 凭据 | E8/E9 子目录产物 |

### 3.2 S2 自定义 inputs（9 条）

| # | 用例名 | 目的（口径） | args / files 要点 |
|---|---|---|---|
| 1 | cus-01-bad-uri | E1/E5 非法 URI | `:not-a-uri` |
| 2 | cus-02-http-404 | E1 404 路径 | `http://127.0.0.1:18080/nonexistent.bin` |
| 3 | cus-03-ftp-auth-fail | E1 530 未授权 | FTP 无凭据访问 `ftp://127.0.0.1:1821/1kib.bin` |
| 4 | cus-04-empty-file | E8/E1 边界 | `http://127.0.0.1:18080/empty.bin`，exit 0 空产物 |
| 5 | cus-05-dry-run | E2 + 产物必须无变化 | `--dry-run http://127.0.0.1:18080/1kib.bin` |
| 6 | cus-06-unknown-option | E5 用法错误 | `--not-an-option` |
| 7 | cus-07-multi-argv-j2 | E2/E3 多任务 compact 形态（与 man-05 的 `-i` 路径区分） | 两个命令行 URI + `-j2` |
| 8 | cus-08-trunc-interrupt | E11/E12 中断落控制文件 | 截断端点触发（依赖 §7-1）；未就绪则标 deferred |
| 9 | cus-09-resume-control | E8/E11 有控制文件续传 | files 预置 cus-08 的产物快照（部分文件 + `.aria2`） |

### 3.3 延后来源（不进首轮，各阶段任务引用本节回填）

- **RPC（阶段 3，TELE-99 已回填）**：手册 §EXAMPLE › RPC（SSL/TLS）与 OPTIONS 里 `aria2.addUri`/`tellStatus`/`system.listMethods` 等的 JSON-RPC/XML-RPC 示例；录制形态 `app record --http`，端口按 TEST-ENV §5 阶段扩展点。实测 27 例落在 `rpc-cases/rust/app-cases/http/`（独立于 185 条 CLI 用例，因 4.2.0 每个录制根只有一个 `config.http` 开关），输入清单 `cases/rpc.jsonl`，原版 27/27、前门 27/27、fallback=0.0%。
- **BitTorrent（阶段 4）**：§EXAMPLE › BitTorrent Download 全部（本地 torrent、magnet、-j2、-T 混跑、select-file、index-out、seed 条件、tracker 替换、DHT、加密）。夹具自制 metainfo + 本地 tracker。
- **Metalink（阶段 5）**：§EXAMPLE › Metalink Download 全部 + `-V` 修复（Throttle 示例顺带）。夹具自制 XML。
- **代理示例**（Using a proxy 两组）：与 TEST-ENV §6 死代理防线直接冲突，**永不录制**；代理失败语义已由公共参数在每条用例中体现。

首轮规模：14 + 9 + S3（工具定），预计 ≈ 30 条上下；每条由工具自动连跑两次归类噪声。

## 4. 录制执行

命令即 TEST-ENV §8(4)（`--fresh test-env/fixtures --fresh test-env/certs --fresh downloads --inputs cases/stage1.jsonl`）。补充三点（均 4.2.0 实测）：

- **归一化机制：自动噪声 + 具名 normalizer**（TELE-97 实测修订 v0.4/v0.5/v0.7/v0.8/v0.9/v0.10/v0.11）。record 双跑仍自动把不一致值打成 `[..]`；两跑恰好一致的固定值不会被通配，故写入 `timestamps`（内建）与 `date`、`speed`、`src-line`、`argv0` 四条具名 regex。这些规则在录制与回放两侧同时生效，golden 由 record 按同一规则生成；它们只影响比对，不修改任何实现代码。`date=\b\d{2}/\d{2}\b` 来自实测跨日问题：录制在 11/28、CI 在 11/27 时 22 个用例全部因日志行首日期失配；对应 EQUIVALENCE N4。`src-line` 只匹配 `\d+\]`（行号），文件名如 `download_helper.cc` 保留并逐字断言；`argv0` 只匹配到未知选项的开引号，选项名与闭引号仍逐字比对；两者分别对应 N11/N12。`speed` 对应 N3。经 owner 授权与统筹裁决确认。`-j2` 完成顺序抖动由 `man-05`/`man-10` 的整行 `[..]` 吸收；macOS 独有的 `Allocating disk space` 提示由 `man-03`/`man-12` 的 `...` 吸收；进度噪声只删除 `[#GID ...]`、`Download Progress Summary` 等形状的行，NOTICE 级别、校验通知与完成路径仍是逐字锚点（`normalize-progress.py`）。
- **录制来源核对**：录制完成时核对工具输出 `N from examples / M from inputs / K traps` 行与「Noise, …」归类行，确认与 §2 预期一致（预期 examples=0）。
- **跨行 GID**：自动通配不校验跨行耦合；阶段 1 已落地 `test-env/check-gid-consistency.py`（EQUIVALENCE N1），逐次运行把结果表短 GID 与 `GID#` 长 GID 做成一一映射断言，作为 G2 后的 CI 步骤。
- **正向覆盖与私钥**：HTTPS/FTP 正向用例必须 `exit 0` 且 `.out/` 有产物（CI 加显式断言）；录制品经 `test-env/scrub-cases.sh` 删除 `*.key`，scrub 正则覆盖 PKCS#8 `BEGIN [A-Z ]*PRIVATE KEY`。证书由 `test-env/gen_certs.py` 可复现生成：同一密钥对/信任锚，ECDSA 签名随机故字节不保证相同。
- **进度行批量归一化**：`test-env/normalize-progress.py` 在录制后对 22 个含下载通知的用例只删除 `[#GID ...]`、`*** Download Progress Summary ... ***` 等进度形状的行，以及 macOS 独有的 `Allocating disk space` 提示（平台噪声），并在保留的每条通知前插入 `...`；NOTICE 级别、`Verification finished successfully`、完成路径、退出码与结果表仍是逐字锚点。脚本幂等（先去掉旧 `...` 再重建）。这是 EQUIVALENCE N3/N9 的 trycmd 表达。录制前必须确保 `downloads/` 只有 setup 生成的夹具状态，否则初始文件会被快照进 `files/` 并让回放判定「未写入」。
- **结果表断言恢复**：`test-env/restore-tables.py` 把 record 因两跑不一致而整行通配的结果行恢复为 `[..]|OK  |[..]|<path>`，图例恢复为 `(OK):download completed.`；单一下载用例用字面路径，多 URI 用例保留 `downloads/[..].bin` 形状以容忍顺序。man-08 完成行保留 `NOTICE` 字面。四个反向变异（`OK`→`ERR`、图例改 `(ERR)`、完成行 `NOTICE`→`ERROR`、man-05 表行 `OK`→`ERR`）均变红。
- **已知限制**：锚点前的 `...` 会吸收任意多行，不只进度行（实测插入 `[ERROR] INJECTED bogus` 仍通过）；这是 trycmd 表达 N9 的边界。严格收紧需把进度行归一为固定形状/计数，留待阶段 2。

## 5. CI 比对方案（replay-report.md 闸门）

**提交纪律**：`rust/app-cases/` 全目录入 git（trycmd 文件、`rust-anything.json`、`replay-report.md`、`files/` 快照）。报告是 golden 漂移检测器，必须被追踪。

| 闸门 | 命令 | 通过条件 | 启用 |
|---|---|---|---|
| G0 环境与边界预检 | `test-env/serve.sh check`；TEST-ENV §6(3) 白名单 grep；§7 scrub grep | 三者全过 | 首轮 |
| G1 原版回放 | `rust-anything app replay . --build "test-env/replay-home.sh <完整运行串>"`，串内容和 §8 record `--` 之后一致 | exit 0，报告 `Passed: N/N`（原版 100% 是阶段闸门） | 首轮；阶段 2 起加 HOME wrapper |
| G2 前门回放 | `rust-anything app replay . --build "test-env/replay-home.sh <front-door 二进制>"` | exit 0，`Passed: N/N` 且 fallback 低于上一阶段基线 | TELE-97 落地 front-door 后；阶段 2 起加 HOME wrapper |
| G3 报告零漂移 | replay 后 `git diff --exit-code -- rust/app-cases` | diff 为空；有 diff 即 PR 必须有意更新快照并在描述中指出对应行为变化与口径编号 | 首轮 |
| G4 fallback 下降 | 同 G2，断言 Fallback rate ≤ 上一阶段基线（数字记入 rust/MAP.md） | 阈值递减 | 阶段 2 起，首轮占位 |

CI 步骤（GitHub Actions job 草案，可直帖）：

```yaml
# 每 PR：
# 1. checkout（rust/app-cases 随仓）
# 2. 构建/缓存 env/aria2-1.37.0/bin/aria2c（TEST-ENV §8(1)，SHA-256 入 step summary）
# 3. test-env/setup.sh（内含 manifest git diff --exit-code）；serve.sh start && serve.sh check
# 4. G0：白名单 grep（TEST-ENV §6(3)）+ scrub grep（§7）
# 5. build rust/app（front-door 或当前头）
# 6. G1：rust-anything app replay . --build "env/aria2-1.37.0/bin/aria2c --no-conf ... --ca-certificate=test-env/certs/ca.crt"
# 7. G2：rust-anything app replay . --build rust/app/target/debug/aria2c
# 8. G3：git diff --exit-code -- rust/app-cases
```

注意：`--build` 收**单个词**且会**替换整条运行命令**——传 `env/.../aria2c` 一个词会丢掉 `--no-conf` / `--dir=downloads` / 死代理 / CA 等公共参数，导致假失败；带参数的构建命令须整体引号包裹。

**失败处置**：`replay-report.md` 的 per-case diff 是唯一仲裁输出；单例复跑 `rust-anything app replay . --case <name>`（TEST-ENV §8）。时序噪声（速率/ETA 抖动）→ 修 §4 归一化规则或提 EQUIVALENCE §4 修订，**禁止**放宽闸门、删用例、重跑过审。整 job 允许 rerun 一次，二次失败即 fail。

## 6. TELE-97 验收清单（直接引用）

- [x] `cases/stage1.jsonl` 覆盖 §3.1 全部 14 条 + §3.2 全部 9 条（含由 `/trunc/` 端点落地的 cus-08/09），无 deferred
- [x] record 按 TEST-ENV §8(4) 执行，S3 陷阱开启：`0 from examples / 23 from inputs / 162 traps`
- [x] G1：`app replay` 原版 185/185，exit 0
- [x] G2：front-door 185/185 passed + Fallback rate 100.0%
- [x] `rust/app-cases/` 含 `replay-report.md` 入 git；G3 `git diff --exit-code -- rust/app-cases` 干净
- [x] G0 三项检查通过（白名单 / scrub / serve.sh check）
- [x] §4 归一化规则定稿并随 cases 提交（`timestamps` + `speed=...`）

## 7. 交叉引用与回填需求

1. **TEST-ENV §3 缺截断端点**：EQUIVALENCE §6 与 E12 假设「本地服务器在指定字节数后截断/断开」，但 §3 服务契约未列此端点。契约草案（TELE-97 落地 serve.sh 时回填 §3 表）：

   | 端点 | 行为 | 用途 |
   |---|---|---|
   | `/trunc/<file>/<bytes>`（HTTP，18080/18443） | 回 `200` + `Content-Length: <全文大小>`，发出 `<bytes>` 字节后**不结束响应直接断开** | cus-08 中断落 `.aria2`（E11/E12） |

   未落地则 cus-08/09 标 deferred，不阻塞首轮。
2. **FTP 凭据注入（TEST-ENV §7 修订）——已批准，走解法 A**：4.2.0 实测 `--inputs` 的 `env` 字段注入生效，但 `args` **严格字面量**（`$VAR` 原样到达进程）；aria2c 又无凭据类环境变量（仅代理走 env）→ 解法 B 排除。owner 已明确授权解法 A（「同意 §7 解法 A」，本 issue 评论，2026-11-27）。TELE-97 执行要点：
   - FTP 用户名与口令改为**固定合成常量**（如 `aria2test` / `aria2-local-test-only`）随 `cases/stage1.jsonl` 入 git；仅 loopback、无真实数据。
   - TEST-ENV §7 同步修订两处：①「口令不入 git」改为「FTP 合成常量入 git，`creds.json` 不再服务 FTP 用例」；② §7 scrub grep 的模式需去掉 `aria2test`（否则会把已批准的常量误报为泄漏）或改为「已知合成常量除外」。
   - TELE-97 的 PR 描述引用本 issue 授权评论作为修订依据。
   - 影响面：man-04/13/14 由 deferred 转为正常录制；FTPS（阶段 6）沿用同一常量。
3. **本文落库**：建议并入 TELE-95 文档骨架 PR（`docs/RECORD-CASES.md`），或由 TELE-97 首个 PR 带入。
4. EQUIVALENCE 无需变更：本文不新增口径，只落实其 §6「对比器与 golden 录制在阶段 1 落地」。

## 8. 证据与未验证

**已验证（rust-anything 4.2.0，微型 CLI probe 实测）**：
- `app record`：trycmd 文件布局（`<name>.toml/.stdout/.stderr` + `files/` + `rust-anything.json`）；来源计数行；每例连跑两次的自动噪声归类（两跑不一致的值片段 → `[..]` 通配），随机速率/ETA 用例通配后 12/12 回放全绿。
- `app replay`：`--build` 替换整条运行命令；全绿 exit 0 / 失败 exit 1 + per-case diff；报告路径 `rust/app-cases/replay-report.md`（含 Build / Passed / Fallback rate）；front-door 同一 build 串 3 次 replay 的报告 SHA-256 相同。
- `--inputs`：`env` 字段注入生效；`args` 严格字面量，无 env 展开。
- `--normalize name=regex`：正确语法（具名前缀 + 合法 Rust 正则）在 4.2.0 生效；v0.2 的否定结论来自错误写法。两跑一致的固定值只能靠具名 normalizer，不能依赖自动通配。
- 手册 EXAMPLE 一节全文：`doc/manual-src/en/aria2c.rst` @ tag `release-1.37.0`（GitHub raw）。

**未运行/未验证**：
- 真实 aria2 构建、首轮录制与回放（属 TELE-97）；自动通配对真实 aria2 输出（GID、进度读出、Download Results 表）的覆盖程度；死代理前缀下 21 MiB 用例的录制时长。
- caddy/pyftpdlib 行为（含 §7-1 截断端点落地）、§7-2 解法 A 的 owner 修订确认。

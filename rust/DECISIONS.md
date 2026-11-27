# DECISIONS — aria2-rs 决策记录（ADR 注册表）

> 用途：rust-anything whole-app 流程的 ADR 落点（官方 whole-app skill 把决策写入 `rust/DECISIONS.md`），
> 同时是 `docs/DEPENDENCIES.md` §6.1 规定的手写单元 ADR 登记处。
> 每条 ADR：状态、背景、决策、一手证据、触发重评条件。修改已生效 ADR = 决策变更，须经 owner 确认。

| # | 决策 | 状态 | 日期 |
|---|---|---|---|
| ADR-001 | C++ 无单元级 bridge/record，采用 app replay + 手写契约测试 | Accepted | 2026-11-27 |

---

## ADR-001 C++ 无单元级 bridge/record —— 采用 app replay + 手写契约测试

- **状态**：Accepted（TELE-91 交付物，阶段 0 核对通过后生效）
- **日期**：2026-11-27
- **影响范围**：阶段 2–7 全部移植工作与验收方式；不改变 `docs/EQUIVALENCE.md` 的任何口径，只定验收通道。

### 背景

aria2 主体是 C++：609 .cc / 560 .h / 22 .c（基线 release-1.37.0，统计来源见 `docs/DEPENDENCIES.md` §1）。
rust-anything 的单元级移植 loop（`record <crate>` → `diff` → `gate`）依赖语言适配器完成三件事：
analyze 识别单元、bridge 生成跨语言桩、record 把原单元编译为共享库录制行为。若 C++ 无适配器，
609 个 .cc 单元走不了 loop；本 ADR 定替代路径，并给 22 个 .c 一个明确处置，避免「整个仓库一刀切」。

### 决策

- **D1：不等待上游 C++ 支持。** .cc/.h 单元一律不使用单元级 bridge/record，移植与验收走
  whole-app 通道：`analyze --whole-app → app record → app front-door → 逐单元移植 + app wire
  → fallback=0 且 replay 全绿 → app lift`（官方流程，见证据 4）。等价性判定与归一化按
  `docs/EQUIVALENCE.md` §2/§4 执行。
- **D2：.c 单元照常走完整 loop。** C 是官方支持语言（unit = `.c` file，bridge = `extern "C"`）。
  22 个 .c 中只有 `src/uri_split.c` 是真实功能单元、无条件编译，随 `uri-parse` 单元（阶段 2，
  `docs/DEPENDENCIES.md` §4 N8）走 record → diff → gate 全程；其余 21 个不移植（见下表）。
- **D3：手写单元仍受 MAP 管辖。** 因 D1 而手写的每个单元（`docs/DEPENDENCIES.md` §4 具名清单）
  照常进 `rust/MAP.md` 的单元清单与 claim 流程，且手写前必须已在 DEPENDENCIES.md §4 登记，
  不许静默手写（`docs/DEPENDENCIES.md` §6.1）。

### 证据（一手来源）

1. **官方语言支持矩阵** `docs/languages.md`
   （https://github.com/erkamyaman/rust-anything/blob/main/docs/languages.md）：
   11 行 —— TypeScript、JavaScript、Python、C、Go、Ruby、Java、Kotlin、PHP、PowerShell、C#。
   **无 C++ 行**。C 行：unit = `.c` file；bridge = 原名 `extern "C"` 函数 + 头文件；
   record = 原单元编译为共享库、ctypes 调用、每用例一 fork。
2. **官方支持语言清单** https://rustanything.ai/llms.txt ：「Works with Node.js/TypeScript, Python,
   C, Go, Ruby and Java; PHP, C#, Kotlin and PowerShell in preview」—— 无 C++。
3. **上游 issue #704**（https://github.com/erkamyaman/rust-anything/issues/704，
   「C++ support: analyze, bridge and record C++ units」，**open**，2026-11-26 建，enhancement 标签，
   经 `gh api repos/erkamyaman/rust-anything/issues/704` 核实 state=open）：
   - 「The C adapter (`src/lang/c.rs`) only handles `.c` and `.h` files. A `.cpp`, `.cc`, `.cxx`,
     `.hpp` or `.hh` file is not detected or parsed.」—— 单元级 analyze/bridge/record 不覆盖 C++。
   - 「A C++ program can already be rewritten as a whole with `app record` and `app replay`, which
     compare the running programs whatever language they are in. This issue is about unit-by-unit
     ports.」—— app 级整程序录制/回放对 C++ 可用，这是 D1 的通道。
   - Done 条件包括 languages.md 出现 C++ 行（当前无），即上游自己把矩阵行作为支持标志。
4. **官方 whole-app 流程** https://rustanything.ai/docs/whole-apps/overview ：
   `analyze --whole-app ──> app record ──> app front-door ──> per unit: the loop, then app wire ──> app lift`；
   「app lift removes the original once replay passes with fallback rate 0 and every unit passes gate」。
5. **本机 rust-anything 4.2.0 doctor 输出**（2026-11-27，verbatim 见文末，exit 0）：
   required 项全 ok（cargo / node / python3 / **cc** / go / java / javac / mcp server /
   Claude Code mcp+skill / OpenCode mcp+skill）；optional 缺失项（ruby 过期、kotlinc、php、dotnet、
   pwsh、wasm、Android NDK/targets/adb）与本项目无关——aria2-rs 只需 `cc`（D2 的 .c 单元编译录制用），
   状态 ok。doctor 的语言检查清单与 languages.md 矩阵一致，均无 C++ 检查项，与 #704 互为印证。

### 22 个 .c 处置表（验收项：写清哪些可走完整 loop）

| 位置 | 文件 | 归类 | 处置 | 证据 |
|---|---|---|---|---|
| `src/uri_split.c`（+ `.h`） | 1 个 | aria2 真实功能单元（宽松 URI 切分） | **走完整单元 loop**：随 `uri-parse` 单元（阶段 2）record → diff → gate | `src/Makefile.am` release-1.37.0 **L255** 无条件编译进 `SRCS`；languages.md C 行 |
| `src/alloca.c`、`asctime_r.c`、`gai_strerror.c`、`getaddrinfo.c`、`gettimeofday.c`、`libgen.c`、`localtime_r.c`、`strptime.c`、`timegm.c` | 9 个 | 可移植性 shim：仅当平台缺 POSIX/标准函数时才编译 | **不移植**。Rust 侧由 std / libc / rustix 等价物直接覆盖（`docs/DEPENDENCIES.md` §1）；移植等于往纯 Rust 目标里加死代码 | `src/Makefile.am` release-1.37.0 **L628–658** 的 `if !HAVE_ASCTIME_R / BASENAME / GETADDRINFO / GAI_STRERROR / GETTIMEOFDAY / LOCALTIME_R / STRPTIME / TIMEGM` 条件块；`configure.ac` `AC_CHECK_FUNCS`（getaddrinfo/gettimeofday/strptime/timegm）；alloca 经 `@ALLOCA@`（AC_FUNC_ALLOCA） |
| `deps/wslay/`（lib 5 + tests 6 + examples 1） | 12 个 | vendored 第三方 WebSocket 库，非 aria2 单元 | **不移植**。由 tokio-tungstenite 替代（`docs/DEPENDENCIES.md` §3 WebSocket 行），会话/通知行为偏差由阶段 3 录制用例裁定（§4 U3） | `deps/wslay/` 目录结构（`git ls-tree`）；DEPENDENCIES.md §2 「wslay（捆绑，非系统依赖）」 |

技术备注：9 个 shim 在语法上是合法 C，若强行移植也能走 C loop；不做，因为等价目标是
「Rust 里不存在这个文件」，移植只会引入无用代码。此为范围决策，不是能力限制。

### C++ 单元的 gate 替代验收方式（验收项）

rust-anything 的自动 gate（`record`/`diff`/`gate`：逐用例回放 + 反作弊 + unsafe/缺测试检查）
只覆盖走完整 loop 的单元。按 D1/D2，本项目内即 `uri_split.c`（以及未来任何 .c 单元）。
其余全部 C++ 单元的验收通道是三层替代：

1. **app 层 replay gate（主闸门）**：`app record` 在原版 aria2 1.37.0 上录制整程序行为，
   `app replay` 对 aria2-rs 回放。比较面与判定等级按 `docs/EQUIVALENCE.md`：
   §2 三级判定、§4 归一化 N1–N10、§6 差分方法（stdout/stderr/exit code/下载产物/.aria2 控制文件）。
   阶段目标沿用父任务 TELE-88：阶段 1 原版回放 100%，逐阶段全绿，阶段 7 fallback=0 后 `app lift`。
2. **手写契约测试（单元层补偿）**：每个手写单元（DEPENDENCIES.md §4 N1–N9 具名单元）配
   EQ-\* 映射的测试，断言该单元的对外行为；golden 期望值由原版实跑录制生成（录制环境边界见
   `docs/TEST-ENV.md`，TELE-92 交付后生效），**禁止复制答案**。`docs/EQUIVALENCE.md` §5 三条 PR
   硬门（禁止复制答案 / `// SAFETY:` 纪律 / 禁止缺测试）对手写测试同样生效——手写测试伪造期望值
   视同实现作弊。
3. **审查与核对（闸门兜底）**：判定 ≠「不比较」的每个口径必须有映射测试，无映射的 PR 不合入
   （EQUIVALENCE §5.3）；阶段交付物由统筹核对后才放行下一阶段（TELE-88 规矩）。

**已知代价（显式声明）**：单元级 diff 能捕捉「内部函数 输入→输出」的细粒度行为，app replay 只见
可观测表面；替代方案的单元行为覆盖取决于 EQ-\* 用例设计的完备性，遗漏由「fallback 下降 +
replay 全绿」兜底而非自动发现。若某 C++ 单元在阶段后期暴露用例空洞，回补该单元的契约用例，
不回退 D1。

**lift 收口定义（对官方 lift 条件的替代读法）**：官方 lift 要求「replay passes with fallback
rate 0 and every unit passes gate」；本项目按 D1 读作：replay 全绿 + fallback=0 + 走 loop 的
单元 gate 通过 + 全部手写单元契约测试全绿。

### 触发重评条件

满足任一条即重开本 ADR（走文首变更流程）：
- 上游 #704 关闭，或 languages.md 出现 C++ 行，或 `rust-anything doctor` 出现 C++ 适配器项
  → C++ 单元可回填 record/diff/gate 完整 loop，手写契约测试降级为补充。
- rust-anything 主版本升级（>4.2.0）时重跑 doctor 并复核本 ADR 证据 1/2/5 是否仍成立。

### 本机 rust-anything 版本与 doctor 输出（2026-11-27，verbatim）

（首行版本号来自 `rust-anything --version`，其余为 `rust-anything doctor` 原样输出。）

```text
rust-anything 4.2.0
ok        cargo                   cargo 1.102.0-nightly (ae8f06eb4 2026-11-21)
ok        node                    v26.13.1
ok        python3                 Python 3.14.10
ok        cc                      Apple clang version 21.0.0 (clang-2100.4.1.1)
ok        go                      go version go1.27.4 darwin/arm64
outdated  ruby                    ruby 2.6.10p210 (2022-04-12 revision 67958) [universal.arm64e-darwin26]; rust-anything needs 3.0+; magnus/rb-sys extensions do not build on Ruby 2 (this is macOS's system Ruby) (optional)
                                  fix: brew install ruby && export PATH="$(brew --prefix ruby)/bin:$PATH"
ok        java                    openjdk version "17.0.21.1" 2026-11-17
ok        javac                   javac 17.0.21.1
missing   kotlinc                 `kotlinc` not found or did not run (optional)
                                  fix: brew install kotlin
missing   php                     `php` not found or did not run (optional)
                                  fix: brew install php
missing   dotnet                  `dotnet` not found or did not run (optional)
                                  fix: brew install --cask dotnet-sdk
missing   pwsh                    `pwsh` not found or did not run (optional)
                                  fix: brew install powershell
missing   wasm-bindgen            `wasm-bindgen` not found or did not run (optional)
                                  fix: cargo install wasm-bindgen-cli --version <the crate's wasm-bindgen version> --locked
missing   wasm32-unknown-unknown  wasm32-unknown-unknown not installed (optional)
                                  fix: rustup target add wasm32-unknown-unknown
missing   Android NDK             Android NDK not found: set ANDROID_NDK_HOME, pass `--ndk <path>`, or install one with `sdkmanager "ndk;28.0.13004108"` (optional)
                                  fix: sdkmanager "ndk;28.0.13004108"  (or set ANDROID_NDK_HOME to an installed NDK)
missing   Android Rust targets    not installed: aarch64-linux-android, armv7-linux-androideabi, x86_64-linux-android (optional)
                                  fix: rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
missing   Android adb             `adb` not found or did not run; recording and diffing on a device needs it (optional)
                                  fix: sdkmanager "platform-tools"  (or install Android Studio's SDK tools), then put its platform-tools folder on PATH
ok        mcp server              `/Users/iorest/.local/share/mise/installs/cargo-rust-anything/4.2.0/bin/rust-anything mcp` answered with 22 tools
ok        Claude Code mcp         /Users/iorest/.claude.json runs `/Users/iorest/.local/share/mise/installs/cargo-rust-anything/4.2.0/bin/rust-anything`
ok        Claude Code skill       /Users/iorest/.claude/skills/rust-anything/SKILL.md matches this version
ok        OpenCode mcp            /Users/iorest/.config/opencode/opencode.json runs `/Users/iorest/.local/share/mise/installs/cargo-rust-anything/4.2.0/bin/rust-anything`
ok        OpenCode skill          /Users/iorest/.config/opencode/skills/rust-anything/SKILL.md matches this version

Everything required is in place. Language runtimes and wasm are optional: each matters only for the languages you port.
```

（exit code 0。optional 项全部与 aria2-rs 无关，理由见证据 5。）

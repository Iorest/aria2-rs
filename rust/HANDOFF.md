# HANDOFF — 会话交接

> TELE-95 初稿；TELE-97 阶段 1 收口时更新。每个 checkpoint 后更新本文件给下一个新会话：现状、进行中/已坏、下一步。只做指针，不复制内容。

## 现状（截至 2026-11-27）

- **位置**：阶段 0 已收口；阶段 1（基线录制 + Rust 前门）已交付，等待统筹核对放行阶段 2（父任务 TELE-88）。
- **仓库**：https://github.com/Iorest/aria2-rs （GPL-2.0）。`main` 已有 EQUIVALENCE；本阶段 PR 把其余阶段 0 文档、`test-env/`、`cases/`、`rust/app-cases/`、`rust/app` 一并落仓。
- **权威文档**（读代码前先读）：
  - `docs/EQUIVALENCE.md` — 等价性契约与归一化规则
  - `docs/TEST-ENV.md` v0.2 / `docs/RECORD-CASES.md` v0.4 — 测试环境、用例清单、G0–G4 闸门与实测修订
  - `docs/ARCHITECTURE.md` / `docs/DEPENDENCIES.md` — 单元划分、源码证据、crate fit
  - `rust/DECISIONS.md` — ADR 注册表（ADR-001）
  - `rust/MAP.md` — 单元清单、claim 规则、fallback 基线
  - `docs/PROTOCOL-MATRIX.md` / `rust/GLOSSARY.md` / 本文件
- **fallback rate**：100.0%（185/185，Rust 前门全部回落原版）；阶段 2 起只降不升。

## 阅读顺序（新会话必读）

1. `rust/MAP.md` — 先领单元再动工（claim 规则 §4）
2. `rust/DECISIONS.md` — ADR-001：验收通道（app replay + 手写契约测试）
3. `docs/EQUIVALENCE.md` — 判定等级、归一化、反作弊硬门
4. `docs/ARCHITECTURE.md` §2 + `docs/DEPENDENCIES.md` §4 — 单元与参考源码
5. 载入 skill：`rust-anything`（whole-app 流程）

## 下一步（阶段 2，TELE-98；待统筹放行后开工）

1. 载入 `rust-anything` skill（whole-app），读 `rust/MAP.md` §3.2 与 `docs/ARCHITECTURE.md` §2，先 claim 一个单元。
2. `rust-anything analyze . --whole-app -o rust/plan.md` 定稿单元切分，回填 MAP §3.2。
3. 按 wave 移植首个单元（建议 `uri-parse`，唯一走完整 C loop），`app wire` 后 replay 100% 且 fallback < 100%。

## 工作循环（每单元）

领单元（MAP §4）→ 读该单元相关文档与 research → 按 rust-anything SKILL 步骤 1–5 移植 → `app wire` 接入 → `app replay` 全绿 + fallback 下降 → MAP 标 `done`（附证据）→ checkpoint 更新本文件。

## 进行中 / 已坏

（暂无。recording 期遇到的 Caddy bottle 下载停滞、pyftpdlib CLI 线程化 bug、时间戳/速率通配问题均已在本阶段修复并写入 `docs/TEST-ENV.md` v0.2 / `docs/RECORD-CASES.md` v0.4。）

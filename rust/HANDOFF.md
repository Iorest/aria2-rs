# HANDOFF — 会话交接

> TELE-95 初稿；TELE-97 阶段 1 收口时更新。每个 checkpoint 后更新本文件给下一个新会话：现状、进行中/已坏、下一步。只做指针，不复制内容。

## 现状（截至 2026-11-28）
- **位置**：阶段 0–2 已收口；阶段 3（RPC 双通道）已交付，等待统筹核对放行阶段 4（父任务 TELE-88）。
- **仓库**：https://github.com/Iorest/aria2-rs （GPL-2.0）。`main` 已有 EQUIVALENCE、185 条 CLI 用例与前门；阶段 3 PR 新增 `rust/app/src/rpc.rs`、`cases/rpc.jsonl`、`rpc-cases/` 与 CI 双通道回放。
- **fallback rate**：CLI 通道 42.7%（79/185）；RPC 通道 0.0%（0/28）；合并 37.1%（79/213），只降不升。
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

## 下一步（阶段 4，TELE-100；待统筹放行后开工）
1. 阶段 3 已落地 JSON-RPC / XML-RPC 双通道（28 例录制全绿）；下一步先核对本阶段交付物，再放行阶段 4（BitTorrent，TELE-100）。
2. 遗留：WebSocket 通知未实现。`docs/EQUIVALENCE.md` E17 写「载荷为 tellStatus 形状 struct」，但 aria2@1.37.0 `src/WebSocketSessionMan.cc:71-83` 实际只发 `{"jsonrpc":"2.0","method":...,"params":[{"gid":"<16hex>"}]}`；按契约 §7 须由 owner 确认回填后再补测试，不静默改口径。
3. CI 注意：原版构建已启用 expat（`--without-libexpat` 移除），否则 `/rpc` 端点在 1.37.0 中不存在（configure.ac:623-633）。
4. 明镜 11:03 审查的 5 条阻塞已在 11:xx 修复：XML 实体解码、addUri 异步 active、多源与 options.dir、终态 bitfield、错误态 E14/E15 字段；每条均有修复前后活服务器差分记录（见 PR 评论）。
5. `rpc-methods` 中的 `changeOption/getOption/changeUri/getServers/getPeers/changeGlobalOption/shutdown` 仍是常量桩，`tellStopped` 只返回 complete 集；这些不是本次阻塞项，阶段 4/5 处理。

## 进行中 / 已坏

（暂无。recording 期遇到的 Caddy bottle 下载停滞、pyftpdlib CLI 线程化 bug、时间戳/速率通配问题均已在本阶段修复并写入 `docs/TEST-ENV.md` v0.2 / `docs/RECORD-CASES.md` v0.4。）

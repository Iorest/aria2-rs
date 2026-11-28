# uri-parse — aria2 1.37.0 宽松 URI 解析单元

本单元把 `uri_split.c`（aria2@1.37.0 `src/uri_split.c`，GPL-2.0）的状态机逐行移植到
`port/src/port.rs`。`c-original/` 是该 C 文件与 `uri_split.h` 的逐字副本，外加一个只做
标量打包的测试包装器 `uri_split_wrap.c`（`uri_split()` 的 `uri_split_result*`
出参无法直接跨 C ABI，rust-anything 的 analyze 报告因此建议“bridge a wrapper with
plain arguments”）。包装器只序列化字段位图、端口和 IPv6 标志，不改变解析行为。

## 完整单元 loop

在仓库根执行（需要 rust-anything 4.2.0+）：

```sh
rust-anything analyze rust/uri-parse/c-original --lang c --json
rust-anything record rust/uri-parse/port --inputs rust/uri-parse/cases.jsonl --count 200
rust-anything build rust/uri-parse/port
rust-anything diff rust/uri-parse/port
rust-anything gate rust/uri-parse/port
```

`gate` 的结论在 PR 证据中逐行贴出；`diff` 的 237 条记录用例在
`port/cases.jsonl`。`cases.jsonl`（本目录）是人工补充的边界用例源，覆盖 cus-01 的非法
URI、非 ASCII URI、端口溢出、IPv6 字面量与参数化 URI 形态。

## 接入 front door

`rust/app/src/units/uri.rs` 调用本 crate 的 safe 入口 `summary(&[u8])`：只有
`uri_split()` 判为非流 URI、且不是 magnet / 本地种子 / metalink 文件时，front door
才自己输出 `MSG_UNRECOGNIZED_URI` 并返回 1；其余情况原样回落原版。

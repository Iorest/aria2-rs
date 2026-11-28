#!/usr/bin/env bash
# cutover.sh — aria2-rs 切换指南（TELE-95 交付物）
#
# rust-anything whole-app 流程第 7 步：切换由用户决定。本脚本把「只有人能做」的切换步骤
# 一次一步走完：每步说明做什么、在哪做、能查的先查，并给出回退方式。
# 脚本本身不删除任何文件；唯一的不可回退步骤（退役原版）要求逐字确认，且须 owner 单独授权
# （父任务 TELE-88 规矩：删除原版属不可回退）。
#
# 用法： bash rust/cutover.sh [aria2-rs 二进制路径]
#   二进制路径默认 $ARIA2RS_BIN，其次 ./rust/app/target/release/aria2-rs

set -u

BIN="${1:-${ARIA2RS_BIN:-./rust/app/target/release/aria2-rs}}"
BACKUP="$HOME/aria2c.reference.bak"   # 回退锚点：原版二进制的备份位置

say()  { printf '\n== %s\n' "$*"; }
pause(){ printf '完成后按 Enter 继续（Ctrl-C 退出）：'; read -r _; }

step_preflight() {
  say "第 0 步（脚本代做）· 预检"
  [ -x "$BIN" ] || { echo "未找到可执行文件: $BIN"; echo "先跑 rust-anything app lift 产出纯 Rust 二进制，或用第一个参数指定路径。"; exit 1; }
  echo "-- 二进制自检："; "$BIN" --version || { echo "--version 失败，中止。"; exit 1; }
  echo "-- 回放自检（需要 rust-anything）："
  echo "   rust-anything app replay <repo> --build $BIN"
  echo "   必须全绿后才继续。本脚本不代替你跑这一步。"
}

step_backup_original() {
  say "第 1 步（脚本代做）· 备份原版 aria2c"
  local orig; orig="$(command -v aria2c || true)"
  if [ -z "$orig" ]; then echo "PATH 上没有 aria2c（可跳过备份，直接第 2 步）。"; pause; return; fi
  cp "$orig" "$BACKUP" && echo "已备份 $orig → $BACKUP"
  echo "回退方式：任何时候用 $BACKUP 覆盖回原位置（第 2 步选择的位置）。"
  pause
}

step_install() {
  say "第 2 步（仅人工）· 安装 aria2-rs"
  echo "做什么：把 '$BIN' 复制到你的 PATH 位置，例如："
  echo "    install -m 0755 '$BIN' ~/.local/bin/aria2c     # 用户级，无需 sudo"
  echo "    sudo install -m 0755 '$BIN' /usr/local/bin/aria2c   # 系统级，需要你输 sudo"
  echo "注意：这是替换 aria2c 命令名——先确认第 1 步备份已存在。"
  echo "脚本无法替你选位置与输 sudo，这一步由你执行。"
  pause
  command -v aria2c >/dev/null && echo "检查：$(command -v aria2c) 已可解析。" || echo "检查：PATH 上仍无 aria2c（是否装到别的名字/位置？）"
  echo "回退方式：恢复第 1 步备份（cp '$BACKUP' <安装位置>/aria2c）。"
  pause
}

step_smoke() {
  say "第 3 步（仅人工）· 真实负载冒烟"
  echo "做什么：用你自己的真实下载任务跑 aria2-rs（真实站点、BT、Metalink 各来一个），"
  echo "        并与原版在同样参数下对比结果（SHA-256 对产物）。"
  echo "脚本不能替你选任务与核对产物，这一步由你执行。"
  pause
  echo "回退方式：恢复第 1 步备份；或临时用 RUST_ANYTHING_UNIT_<NAME>=original 回退单单元（lift 前）。"
  pause
}

step_switch_refs() {
  say "第 4 步（仅人工）· 切换引用"
  echo "做什么：把你环境里所有 aria2c 引用切到新二进制：脚本、cron、别名、systemd/launchd 服务、"
  echo "        GUI 工具的下载器路径。逐处 grep -r 'aria2c' 你自己的配置目录核对。"
  echo "脚本无法枚举你的私有配置，这一步由你执行。"
  pause
  echo "回退方式：逐处把路径改回原版（或恢复第 1 步备份到原位置）。"
  pause
}

step_retire() {
  say "第 5 步（仅人工 · 不可回退）· 退役原版"
  echo "做什么：确认运行稳定后，归档或删除原版 aria2c 及其备份以外的副本。"
  echo "规矩：删除原版属不可回退动作，须 owner 在父任务 TELE-88 明确确认后才做；"
  echo "      本脚本不执行删除，只提醒。"
  printf '若 owner 已确认，逐字输入 RETIRE 继续，否则回车跳过：'
  read -r ans
  if [ "$ans" = "RETIRE" ]; then
    echo "由你手动执行归档/删除（脚本不代做）。建议先归档：mv aria2c aria2c.retired-$(date +%Y%m%d)"
    echo "回退方式：无。这是不可回退步骤——再次确认备份与归档副本存在后再删。"
  else
    echo "跳过退役。原版保留作参考（推荐：与 aria2-rs 并行保留至少一个阶段周期）。"
  fi
  pause
}

say "aria2-rs cutover —— 每步都有回退方式，除第 5 步（不可回退，须 owner 确认）。"
step_preflight
step_backup_original
step_install
step_smoke
step_switch_refs
step_retire
say "结束。原版在任何未确认删除前都保留在 $BACKUP（第 1 步）。"

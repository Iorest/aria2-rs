#!/usr/bin/env bash
# start | stop | check the loopback-only HTTP/HTTPS/FTP test servers.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
# shellcheck source=ports.env
source test-env/ports.env
RUN=test-env/run
mkdir -p "$RUN"

pid_file() { echo "$RUN/$1.pid"; }

is_listening() { lsof -nP -iTCP:"$1" -sTCP:LISTEN >/dev/null 2>&1; }

start_one() {
  local name=$1 port=$2
  shift 2
  if is_listening "$port"; then
    echo "already listening: $name on $port"
    return 0
  fi
  nohup "$@" >"$RUN/$name.log" 2>&1 &
  echo $! >"$(pid_file "$name")"
  echo "started $name pid=$(cat "$(pid_file "$name")") on $port"
}

wait_http() {
  local url=$1 i
  for i in $(seq 1 80); do
    if curl -fsS --max-time 1 "$url" -o /dev/null 2>/dev/null; then return 0; fi
    sleep 0.25
  done
  echo "FAIL: $url did not become ready" >&2
  return 1
}

start() {
  if command -v caddy >/dev/null 2>&1; then
    start_one caddy "$HTTP_PORT" caddy run --config test-env/Caddyfile --adapter caddyfile
    start_one trunc "$TRUNC_PORT" python3 test-env/trunc.py
  else
    # Caddy unavailable: equivalent Range-capable loopback server (docs/TEST-ENV.md §3).
    start_one web "$HTTPS_PORT" python3 test-env/httpserver.py
  fi
  start_one ftpd "$FTP_PORT" env FTP_USER="$FTP_USER" FTP_PASS="$FTP_PASS" FTP_PORT="$FTP_PORT" python3 test-env/ftpd.py
  for i in $(seq 1 40); do
    is_listening "$FTP_PORT" && break
    sleep 0.25
  done
  if ! is_listening "$FTP_PORT"; then
    echo "FAIL: FTP server did not listen on $FTP_PORT" >&2
    cat "$RUN/ftpd.log" >&2 || true
    return 1
  fi
  wait_http "http://127.0.0.1:${HTTP_PORT}/1kib.bin"
  # wait_http cannot carry extra curl flags; verify TLS directly.
  for i in $(seq 1 80); do
    if curl -fsS --max-time 1 --cacert test-env/certs/ca.crt \
         "https://127.0.0.1:${HTTPS_PORT}/1kib.bin" -o /dev/null 2>/dev/null; then break; fi
    sleep 0.25
  done
  echo "servers started"
}

stop() {
  local name
  for name in caddy trunc web ftpd; do
    if [[ -f "$(pid_file "$name")" ]]; then
      kill "$(cat "$(pid_file "$name")")" 2>/dev/null || true
      rm -f "$(pid_file "$name")"
    fi
  done
  echo "servers stopped"
}

check() {
  local port line
  local ports="$HTTP_PORT $HTTPS_PORT $FTP_PORT"
  if command -v caddy >/dev/null 2>&1; then ports="$ports $TRUNC_PORT"; fi
  for port in $ports; do
    line=$(lsof -nP -iTCP:"$port" -sTCP:LISTEN 2>/dev/null || true)
    if [[ -z "$line" ]]; then echo "FAIL: port $port not listening" >&2; return 1; fi
    if echo "$line" | grep -qE '\*:'"$port"'|0\.0\.0\.0:'"$port"; then
      echo "FAIL: port $port bound to a non-loopback address" >&2; return 1
    fi
    echo "loopback-only: $port"
  done
  curl -fsS --max-time 3 "http://127.0.0.1:${HTTP_PORT}/1kib.bin" -o /dev/null
  curl -fsS --max-time 3 --cacert test-env/certs/ca.crt \
    "https://127.0.0.1:${HTTPS_PORT}/1kib.bin" -o /dev/null
  FTP_USER="$FTP_USER" FTP_PASS="$FTP_PASS" FTP_PORT="$FTP_PORT" python3 - <<'PY'
import ftplib, os
with ftplib.FTP() as ftp:
    ftp.connect("127.0.0.1", int(os.environ["FTP_PORT"]), timeout=5)
    ftp.login(os.environ["FTP_USER"], os.environ["FTP_PASS"])
    ftp.voidcmd("TYPE I")
    assert ftp.size("1kib.bin") == 1024
PY
  echo "HTTP/HTTPS/FTP loopback checks passed"
}

case "${1:-}" in
  start) start ;;
  stop) stop ;;
  check) check ;;
  *) echo "usage: $0 {start|stop|check}" >&2; exit 2 ;;
esac

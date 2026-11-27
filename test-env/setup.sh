#!/usr/bin/env bash
# Generate the deterministic local test world: certificates, fixtures, manifest.
# Idempotent: re-running changes nothing that is committed.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

mkdir -p test-env/certs test-env/fixtures/http test-env/fixtures/ftp/secret \
         test-env/home test-env/run downloads

# --- Certificates: test-only CA, never imported into the system trust chain. ---
if [[ ! -f test-env/certs/ca.crt || ! -f test-env/certs/server.crt ]]; then
  (
    cd test-env/certs
    openssl req -x509 -newkey rsa:2048 -sha256 -days 825 -nodes \
      -keyout ca.key -out ca.crt -subj "/CN=aria2-rs test CA" >/dev/null 2>&1
    openssl req -newkey rsa:2048 -nodes -keyout server.key -out server.csr \
      -subj "/CN=localhost" >/dev/null 2>&1
    printf 'subjectAltName=DNS:localhost,IP:127.0.0.1\nextendedKeyUsage=serverAuth\n' > server.ext
    openssl x509 -req -in server.csr -CA ca.crt -CAkey ca.key -CAcreateserial \
      -days 825 -sha256 -extfile server.ext -out server.crt >/dev/null 2>&1
    rm -f server.csr server.ext
  )
fi

# --- Deterministic fixtures: 256-byte cycle repeated to the requested size. ---
python3 - <<'PY'
import hashlib
import json
from pathlib import Path

root = Path("test-env/fixtures")
pattern = bytes(range(256))


def write(path, size):
    target = root / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes((pattern * (size // 256 + 1))[:size])


for name, size in (
    ("http/empty.bin", 0),
    ("http/1kib.bin", 1024),
    ("http/1mib.bin", 1024 * 1024),
    ("http/20mib-plus1.bin", 20 * 1024 * 1024 + 1),
    ("http/21mib.bin", 21 * 1024 * 1024),
    ("ftp/1kib.bin", 1024),
    ("ftp/1mib.bin", 1024 * 1024),
    ("ftp/secret/1kib.bin", 1024),
):
    write(name, size)


# Deterministic 64 KiB prefix for the resume cases, part of the --fresh
# downloads snapshot so every case starts from the same state.
Path("downloads/1mib.bin").write_bytes(pattern * 256)
manifest = {}
for path in sorted(p for p in root.rglob("*") if p.is_file()):
    data = path.read_bytes()
    manifest[str(path.relative_to(root))] = {
        "size": len(data),
        "sha256": hashlib.sha256(data).hexdigest(),
    }
Path("test-env/manifest.json").write_text(
    json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
)
PY

# Idempotence gate: committed manifest must describe exactly these bytes.
if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  git diff --exit-code -- test-env/manifest.json || {
    echo "FAIL: fixture manifest drift (test-env/manifest.json)" >&2
    exit 1
  }
fi

echo "test-env ready: fixtures + certs deterministic"

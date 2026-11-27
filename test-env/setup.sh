#!/usr/bin/env bash
# Generate the deterministic local test world: certificates, fixtures, manifest.
# Idempotent: re-running changes nothing that is committed.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

mkdir -p test-env/certs test-env/fixtures/http test-env/fixtures/ftp/secret \
         test-env/home test-env/run downloads

# --- Deterministic certificates: identical bytes on every fresh checkout. ---
python3 test-env/gen_certs.py

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

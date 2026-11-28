#!/usr/bin/env bash
# Remove private keys from the committed recording snapshot and verify scrub.
# Run after `rust-anything app record` and before committing rust/app-cases/.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

find rust/app-cases -type f \( -name '*.key' -o -name '*.pem' \) -print -delete

if grep -rnE 'creds\.json|BEGIN [A-Z ]*PRIVATE KEY' rust/app-cases cases; then
  echo 'FAIL: private key or credential reference in cases' >&2
  exit 1
fi

echo 'scrub PASS: no private keys in rust/app-cases'

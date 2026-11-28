#!/bin/sh
# Replay wrapper: run the given build with the recorded empty HOME so the
# original and the Rust front door are compared under the same isolation.
# Product code never changes HOME: a real user's ~/.netrc must stay readable.
set -eu
repo="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
export HOME="$repo/test-env/home"
mkdir -p "$HOME"
exec "$@"

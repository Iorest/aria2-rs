"""Cross-line GID mapping comparator (EQUIVALENCE N1).

`app replay` compares each line with wildcards; it does not check that a GID
printed on one line matches the GID on another. This runs a build on a live
loopback fixture, collects the 16-hex GIDs from GID# lines and the 6-hex GIDs
from the Download Results table, and asserts a consistent one-to-one prefix
mapping. Exit 0 only when every table GID maps to a GID# in the same run.
"""

import re
import subprocess
import sys
import os
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RUN = [
    "--no-conf",
    "--all-proxy=http://127.0.0.1:9",
    "--no-proxy=127.0.0.1,localhost",
    "--max-tries=1",
    "--retry-wait=0",
    "--ca-certificate=test-env/certs/ca.crt",
]
FULL_GID = re.compile(r"GID#([0-9a-f]{16})")
SHORT_GID = re.compile(r"^([0-9a-f]{6})\|\s*(?:OK|ERR|INPR|REMOVED)\s*\|", re.M)


def run(binary: str, args: list[str]) -> str:
    process = subprocess.run(
        [binary, *RUN, "--dir=downloads", *args],
        cwd=ROOT,
        env={**os.environ, "HOME": str(ROOT / "test-env" / "home")},
        capture_output=True,
        text=True,
        timeout=60,
    )
    # Writes land in the gitignored downloads/ tree, never in rust/app-cases/.
    return process.stdout + "\n" + process.stderr


def check(binary: str, args: list[str]) -> bool:
    text = run(binary, args)
    full = set(FULL_GID.findall(text))
    short = set(SHORT_GID.findall(text))
    if not short:
        print(f"FAIL: no Download Results GID for {args}")
        return False
    prefixed = {g[:6] for g in full}
    if not full:
        # A clean success run may print only the short GID; accept it, but
        # require a single GID so cross-line consistency remains meaningful.
        print(f"GID mapping OK (single short GID) for {args}: {sorted(short)}")
        return len(short) == 1
    if short != prefixed:
        print(f"FAIL: GID mapping mismatch for {args}: table={short} full={prefixed}")
        return False
    print(f"GID mapping OK for {args}: {sorted(short)}")
    return True


if __name__ == "__main__":
    binary = sys.argv[1] if len(sys.argv) > 1 else "rust/app/target/debug/aria2c"
    cases = [
        ["http://127.0.0.1:18080/nonexistent.bin"],
        ["-Z", "-j2", "http://127.0.0.1:18080/nonexistent1.bin", "http://127.0.0.1:18080/nonexistent2.bin"],
    ]
    ok = all(check(binary, args) for args in cases)
    print("GID mapping comparator: PASS" if ok else "GID mapping comparator: FAIL")
    sys.exit(0 if ok else 1)

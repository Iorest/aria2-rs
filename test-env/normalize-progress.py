"""Apply EQUIVALENCE N3/N9 progress noise to recorded stdout cases.

Only timing-dependent progress readouts are removed. Every other line between
`Downloading` and `Download Results:` (NOTICE level, checksum verification,
completion path, error text) stays literal and is kept as an ordered anchor;
an ellipsis before each anchor absorbs any number of progress lines.

Run after `app record` and before `scrub-cases.sh`.
"""

import re
from pathlib import Path

CASES = Path(__file__).resolve().parent.parent / "rust" / "app-cases"
DOWNLOADING = re.compile(r"Downloading \d+ item\(s\)")
RESULTS = re.compile(r"^Download Results:")
PROGRESS = re.compile(
    r"^\[#[0-9a-f]{6} |^\*\*\* Download Progress Summary"
    r"|^={10,}$|^-{10,}$|^FILE: |^\[FileAlloc:#"
)


def patch(path: Path) -> bool:
    lines = path.read_text().splitlines()
    start = next((i for i, line in enumerate(lines) if DOWNLOADING.search(line)), None)
    end = next((i for i, line in enumerate(lines) if RESULTS.match(line)), None)
    if start is None or end is None or end <= start:
        return False

    anchors = [
        line for line in lines[start + 1 : end] if line.strip() and not PROGRESS.match(line)
    ]
    if not anchors:
        return False

    rebuilt = lines[: start + 1]
    for anchor in anchors:
        rebuilt.extend(["...", anchor])
    rebuilt.extend(["...", *lines[end:]])
    path.write_text("\n".join(rebuilt) + "\n")
    return True


if __name__ == "__main__":
    changed = [p.name for p in sorted(CASES.glob("*.stdout")) if patch(p)]
    print(f"progress normalization applied to {len(changed)} cases:")
    print("\n".join(changed))

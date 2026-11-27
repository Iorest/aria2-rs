"""Apply EQUIVALENCE N3/N9 progress-line noise to recorded stdout cases.

aria2 prints readouts like `[#GID 1.0MiB/1.0MiB(100%) CN:1]` and a
`*** Download Progress Summary ... ***` block when a run is slow. Which of
those lines appear depends on timing, not behavior. The contract removes them
before comparison; in trycmd terms that is an ellipsis line around the stable
notification lines. Run after `app record` and before `scrub-cases.sh`.
"""

import re
from pathlib import Path

CASES = Path(__file__).resolve().parent.parent / "rust" / "app-cases"
DOWNLOADING = re.compile(r"Downloading \d+ item\(s\)")
COMPLETE = re.compile(r"Download complete: (\S+)")


def patch(path: Path) -> bool:
    lines = path.read_text().splitlines()
    start = next((i for i, line in enumerate(lines) if DOWNLOADING.search(line)), None)
    end = next((i for i, line in enumerate(lines) if line.startswith("Download Results:")), None)
    if start is None or end is None or end <= start:
        return False
    middle = lines[start + 1 : end]
    completed = [m.group(1) for line in middle if (m := COMPLETE.search(line))]
    if not completed:
        return False
    rebuilt = (
        lines[: start + 1]
        + ["..."]
        + [f"[..] Download complete: {path}" for path in completed]
        + ["..."]
        + lines[end:]
    )
    path.write_text("\n".join(rebuilt) + "\n")
    return True


if __name__ == "__main__":
    changed = [p.name for p in sorted(CASES.glob("*.stdout")) if patch(p)]
    print(f"progress normalization applied to {len(changed)} cases:")
    print("\n".join(changed))

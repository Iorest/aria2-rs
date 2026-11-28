"""Restore asserted result-table rows and status legend in success cases.

multi-URI ordering), and can widen the legend to `...`. That silently stops
asserting `OK`, the download path and the legend. This post-processing step
restores the stable parts while keeping GID/avg speed/order wildcards.
"""

import re
from pathlib import Path

CASES = Path(__file__).resolve().parent.parent / "rust" / "app-cases"
ROW = "[..]|OK  |[..]|downloads/[..].bin"
LEGEND = "(OK):download completed."


def patch(stdout: Path) -> bool:
    toml = stdout.with_suffix(".toml")
    if not toml.exists() or "status.code = 0" not in toml.read_text():
        return False
    lines = stdout.read_text().splitlines()
    try:
        head = lines.index("Download Results:")
        legend = lines.index("Status Legend:", head)
    except ValueError:
        return False

    paths = [
        match.group(1)
        for line in lines[:head]
        if (match := re.search(r"Download complete: (\S+)", line))
    ]
    row = ROW if len(set(paths)) != 1 else f"[..]|OK  |[..]|{paths[0]}"
    out = lines[: head + 1]
    for line in lines[head + 1 : legend]:
        out.append(row if line == "[..]" else line)
    out.extend(lines[legend:])
    for i, line in enumerate(out):
        if line.startswith("Status Legend:"):
            if i + 1 < len(out) and out[i + 1] == "...":
                out[i + 1] = LEGEND
            break
    for i, line in enumerate(out):
        if line.startswith("[..] Download complete:"):
            out[i] = line.replace("[..] Download complete:", "[..]NOTICE[..] Download complete:", 1)
    stdout.write_text("\n".join(out) + "\n")
    return True


if __name__ == "__main__":
    changed = [p.name for p in sorted(CASES.glob("*.stdout")) if patch(p)]
    print(f"result-table assertions restored in {len(changed)} cases:")
    print("\n".join(changed))

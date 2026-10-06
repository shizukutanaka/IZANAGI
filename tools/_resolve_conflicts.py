#!/usr/bin/env python3
"""Merge-conflict resolver for IZANAGI round branches.

Resolution policy:
- `pub mod X;` / `("x", x::detect),` hunks in lib.rs: union, dedupe, sort.
- Everything else: union preserving order (ours then theirs, dedupe).
Writes resolved files in place; run after `git merge` left markers.
"""
import re
import subprocess
import sys


def resolve(path: str) -> bool:
    src = open(path, encoding="utf-8").read()
    if "<<<<<<<" not in src:
        return False
    out_lines = []
    block_ours = None
    for line in src.split("\n"):
        if line.startswith("<<<<<<<"):
            block_ours = []
            continue
        if line.startswith("=======") and block_ours is not None:
            block_ours.append(None)  # separator marker
            continue
        if line.startswith(">>>>>>>") and block_ours is not None:
            sep = block_ours.index(None) if None in block_ours else len(block_ours)
            ours = block_ours[:sep]
            theirs = block_ours[sep + 1 :]
            merged = ours + [t for t in theirs if t not in ours]
            if all(
                re.match(r"^\s*(pub mod \w+;|\(\"\w+\",\s*\w+::detect\),)\s*$", l)
                or l == ""
                for l in merged
            ):
                merged = sorted(l for l in merged if l)
            out_lines.extend(merged)
            block_ours = None
            continue
        if block_ours is not None:
            block_ours.append(line)
        else:
            out_lines.append(line)
    open(path, "w", encoding="utf-8").write("\n".join(out_lines))
    return True


def main():
    files = subprocess.run(
        ["git", "diff", "--name-only", "--diff-filter=U"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.split()
    resolved = [f for f in files if resolve(f)]
    print("\n".join(resolved))
    if len(resolved) != len(files):
        print("UNRESOLVED:", set(files) - set(resolved), file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()

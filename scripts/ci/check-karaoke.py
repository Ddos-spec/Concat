#!/usr/bin/env python3
"""Checks that Karaoke highlight cut a title where its words were SAID.

    check-karaoke.py PROJECT_DIR

make-demo-project.py gives the first title (one two three four five six) the
times a transcriber would have heard, deliberately uneven: a proportional
split by letters would cut at about 0.9, 1.8, 3.2, 4.1 and 5.0 seconds, the
spoken times cut at 0.4, 1.0, 3.0, 3.4 and 5.0. The saved project must have
the six pieces start where the words were said.
"""
import json
import os
import sys

EXPECTED = [0.0, 0.4, 1.0, 3.0, 3.4, 5.0]
WORDS = ["one", "two", "three", "four", "five", "six"]
TOLERANCE = 0.08  # frame rounding at 24 to 30 fps, plus the split's own edge


def documents(root):
    for folder, _, files in os.walk(root):
        for name in files:
            if name.endswith((".json", ".concat")):
                try:
                    data = json.load(open(os.path.join(folder, name), encoding="utf-8"))
                except Exception:
                    continue
                node = data.get("project", data) if isinstance(data, dict) else None
                if isinstance(node, dict) and isinstance(node.get("timelines"), list):
                    yield node


def main() -> int:
    root = sys.argv[1]
    for doc in documents(root):
        clips = [
            c for tl in doc["timelines"] for c in tl.get("clips", [])
            if c.get("kind") == "text" and c.get("start", 0) < 5.99
        ]
        clips.sort(key=lambda c: c["start"])
        starts = [round(c["start"], 3) for c in clips]
        print("karaoke pieces start at:", starts)
        print("expected (spoken times):", EXPECTED)
        if len(clips) != len(EXPECTED):
            print(f"FAIL: {len(clips)} pieces, wanted {len(EXPECTED)}")
            return 1
        bad = [(s, e) for s, e in zip(starts, EXPECTED) if abs(s - e) > TOLERANCE]
        if bad:
            print("FAIL: pieces not cut at the spoken times:", bad)
            return 1
        lit = [(c.get("text") or {}).get("runs") or [] for c in clips]
        for k, runs in enumerate(lit):
            if not runs:
                print(f"FAIL: piece {k} lights nothing")
                return 1
        print("OK: karaoke cut at the times the words were said")
        return 0
    print("FAIL: no project document found under", root)
    return 1


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Prints what a saved Concat project holds, in numbers: the clips of each
timeline by kind, the words on each title, the keyed properties, and the
effects on each video clip.

    summarise-project.py PROJECT_DIR
"""
import json
import os
import sys


def find_documents(root):
    for folder, _, files in os.walk(root):
        depth = folder[len(root):].count(os.sep)
        if depth > 2:
            continue
        for name in files:
            if not name.endswith((".json", ".concat")):
                continue
            path = os.path.join(folder, name)
            try:
                if os.path.getsize(path) > 30_000_000:
                    continue
                data = json.load(open(path, encoding="utf-8"))
            except Exception:
                continue
            node = data
            if isinstance(node, dict) and "project" in node and "timelines" not in node:
                node = node["project"]
            if isinstance(node, dict) and isinstance(node.get("timelines"), list):
                yield path, node


def main() -> int:
    root = sys.argv[1]
    found = False
    for path, doc in find_documents(root):
        found = True
        print("document:", os.path.relpath(path, root))
        for timeline in doc["timelines"]:
            clips = timeline.get("clips", [])
            kinds = {}
            for clip in clips:
                kinds[clip.get("kind")] = kinds.get(clip.get("kind"), 0) + 1
            print(f"timeline {timeline.get('name')}: {len(clips)} clips {kinds}")
            for clip in sorted(clips, key=lambda c: (c.get("trackId"), c.get("start", 0))):
                text = (clip.get("text") or {}).get("content")
                runs = (clip.get("text") or {}).get("runs") or []
                keys = clip.get("keys") or []
                keyed = sorted({k.get("property") for k in keys})
                effects = [e.get("id") for e in clip.get("videoEffects") or []]
                adjust = {}
                for effect in clip.get("videoEffects") or []:
                    if effect.get("id") == "concat.adjust":
                        adjust = effect.get("params", {})
                line = (
                    f"  {clip.get('id'):>4} {clip.get('kind'):<6} track={clip.get('trackId')} "
                    f"start={clip.get('start'):.2f} dur={clip.get('duration'):.2f} "
                    f"src={clip.get('sourceStart', 0):.2f}"
                )
                if text is not None:
                    line += f" text={text!r} runs={[(r.get('start'), r.get('end'), r.get('color')) for r in runs]}"
                if keyed:
                    line += f" keys={len(keys)} on {keyed}"
                if effects:
                    line += f" effects={effects}"
                if adjust:
                    line += f" adjust={adjust}"
                print(line)
        break
    if not found:
        print("no project document found under", root)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())

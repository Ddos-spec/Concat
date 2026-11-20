#!/usr/bin/env python3
"""Makes a small project with the Concat API, for looking at the editor.

    make-demo-project.py CLI LOCATION SHOTS_MP4 MOVING_MP4

LOCATION/demo gets: the three-colour clip from 0 s, the panning clip from 6 s
on the same lane, and a six-word title over the first one.
"""
import json
import subprocess
import sys


def main() -> int:
    cli, location, shots, moving = sys.argv[1:5]
    project = f"{location}/demo"
    requests = [
        {"method": "project.create", "location": location, "name": "demo"},
        {"method": "media.import", "path": project, "file": shots},
        {"method": "media.import", "path": project, "file": moving},
        {"method": "edit.apply", "path": project,
         "command": {"op": "addClipAtFirstFree", "mediaId": "m1", "start": 0}},
        {"method": "edit.apply", "path": project,
         "command": {"op": "addClipAtFirstFree", "mediaId": "m2", "start": 6}},
        {"method": "edit.apply", "path": project,
         "command": {"op": "addTextClip", "above": True, "start": 0, "duration": 6,
                     "style": {"content": "one two three four five six"}}},
        # A title over the panning clip, placed in its upper right where the
        # picture has detail to hold on to: the one the follow action rides.
        {"method": "edit.apply", "path": project,
         "command": {"op": "addTextClip", "above": True, "start": 6, "duration": 3,
                     "style": {"content": "track me"}}},
        {"method": "edit.apply", "path": project,
         "command": {"op": "setClipTransform", "clipId": "c6",
                     "offsetX": 0.4, "offsetY": -0.4}},
        {"method": "project.save", "path": project},
    ]
    stdin = "".join(json.dumps(request) + "\n" for request in requests)
    done = subprocess.run([cli, "api"], input=stdin, capture_output=True, text=True)
    print(done.stdout[-3000:])
    print(done.stderr[-1500:], file=sys.stderr)
    return done.returncode


if __name__ == "__main__":
    sys.exit(main())

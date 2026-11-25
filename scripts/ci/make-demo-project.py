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
    proc = subprocess.Popen(
        [cli, "api"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
    )

    def call(request):
        proc.stdin.write(json.dumps(request) + "\n")
        proc.stdin.flush()
        line = proc.stdout.readline()
        print(line[:200].rstrip())
        reply = json.loads(line)
        if reply.get("error"):
            raise SystemExit(f"{request['method']} failed: {reply['error']}")
        return reply.get("result", {})

    call({"method": "project.create", "location": location, "name": "demo"})
    call({"method": "media.import", "path": project, "file": shots})
    call({"method": "media.import", "path": project, "file": moving})
    call({"method": "edit.apply", "path": project,
          "command": {"op": "addClipAtFirstFree", "mediaId": "m1", "start": 0}})
    second = call({"method": "edit.apply", "path": project,
                   "command": {"op": "addClipAtFirstFree", "mediaId": "m2", "start": 6}})
    second_id = second.get("createdId") or "c4"
    call({"method": "edit.apply", "path": project,
          "command": {"op": "updateClip", "clipId": second_id,
                      "patch": {"videoEffects": [{"id": "concat.hsl", "params": {"green_sat": -50.0}, "enabled": True}]}}})
    call({"method": "edit.apply", "path": project,
          "command": {"op": "addTextClip", "above": True, "start": 0, "duration": 6,
                      "style": {"content": "one two three four five six",
                                "spoken": [
                                    {"start": 0.0, "end": 0.4, "text": "one"},
                                    {"start": 0.4, "end": 1.0, "text": "two"},
                                    {"start": 1.0, "end": 3.0, "text": "three"},
                                    {"start": 3.0, "end": 3.4, "text": "four"},
                                    {"start": 3.4, "end": 5.0, "text": "five"},
                                    {"start": 5.0, "end": 5.9, "text": "six"}]}}})
    # A title over the panning clip, placed in its upper right where the
    # picture has detail to hold on to: the one the follow action rides.
    title2 = call({"method": "edit.apply", "path": project,
                   "command": {"op": "addTextClip", "above": True, "start": 6, "duration": 3,
                               "style": {"content": "track me"}}})
    title2_id = title2.get("createdId") or "c6"
    call({"method": "edit.apply", "path": project,
          "command": {"op": "setClipTransform", "clipId": title2_id,
                      "offsetX": 0.4, "offsetY": -0.4}})
    call({"method": "project.save", "path": project})
    proc.stdin.close()
    err = proc.stderr.read()
    if err:
        print(err[-1500:], file=sys.stderr)
    return proc.wait(timeout=60)


if __name__ == "__main__":
    sys.exit(main())

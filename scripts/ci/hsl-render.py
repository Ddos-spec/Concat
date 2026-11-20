#!/usr/bin/env python3
"""Renders a real frame through the editor's own compositor with and without
the HSL effect, and reads the pixel back.

    hsl-render.py CLI WORKDIR SHOTS_MP4

The clip is the three-colour one (red, green, blue, two seconds each). At 3 s
the picture is green. With the green band's saturation at minus a hundred the
same frame must come back grey; with the effect off it must stay green. The
frame is composited by the same code a preview and an export use.
"""
import json
import subprocess
import sys


def main() -> int:
    cli, work, shots = sys.argv[1:4]
    project = f"{work}/hslproj"
    proc = subprocess.Popen([cli, "api"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                            text=True, bufsize=1)

    def call(request):
        proc.stdin.write(json.dumps(request) + "\n")
        proc.stdin.flush()
        reply = json.loads(proc.stdout.readline())
        if "error" in reply and reply["error"]:
            raise SystemExit(f"{request['method']} failed: {reply['error']}")
        return reply.get("result", {})

    call({"method": "project.create", "location": work, "name": "hslproj"})
    call({"method": "media.import", "path": project, "file": shots})
    added = call({"method": "edit.apply", "path": project,
                  "command": {"op": "addClipAtFirstFree", "mediaId": "m1", "start": 0}})
    clip = added["createdId"]
    call({"method": "preview.frame", "path": project, "time": 3.0, "output": f"{work}/hsl-before.png"})
    call({"method": "edit.apply", "path": project, "command": {
        "op": "updateClip", "clipId": clip,
        "patch": {"videoEffects": [{"id": "concat.hsl", "params": {"green_sat": -100.0}, "enabled": True}]}}})
    call({"method": "preview.frame", "path": project, "time": 3.0, "output": f"{work}/hsl-after.png"})
    call({"method": "preview.frame", "path": project, "time": 1.0, "output": f"{work}/hsl-red-after.png"})
    proc.stdin.close()
    proc.wait(timeout=60)

    def centre(path):
        text = subprocess.run(["convert", f"{path}[1x1+960+540]", "txt:-"],
                              capture_output=True, text=True).stdout
        hexa = [t for t in text.split() if t.startswith("#")][0]
        return tuple(int(hexa[i:i + 2], 16) for i in (1, 3, 5))

    before, after, red = (centre(f"{work}/hsl-before.png"), centre(f"{work}/hsl-after.png"),
                          centre(f"{work}/hsl-red-after.png"))
    print(f"green frame, effect off : rgb{before}")
    print(f"green frame, green saturation -100 : rgb{after}")
    print(f"red frame, same effect  : rgb{red}")
    ok = (before[1] > before[0] + 60 and before[1] > before[2] + 60
          and max(after) - min(after) < 25
          and red[0] > red[1] + 60)
    print("HSL effect: green turned grey, red left alone" if ok else "HSL effect: UNEXPECTED")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env bash
# Starts the window on a project under Xvfb, right-clicks the clips on the
# timeline, and photographs each context menu.
#
#   drive-editor.sh CONCAT PROJECT OUTDIR
#
# Coordinates are for a 1600x900 screen and the demo project from
# make-demo-project.py (the three-colour clip and the panning clip on the
# bottom lane, the title above them), the timeline at its opening zoom.
set -u
concat=$1
project=$2
out=$3
mkdir -p "$out"

xvfb-run -a -s "-screen 0 1600x900x24" bash -c '
  concat=$1; project=$2; out=$3
  "$concat" --project "$project" > "$out/drive.log" 2>&1 &
  pid=$!
  sleep 30
  shot() { import -window root "$out/$1.png" || true; }
  shot 00-open
  for target in "shots 250 845" "text 257 761" "moving 347 845"; do
    set -- $target
    xdotool mousemove "$2" "$3" click 3
    sleep 2
    import -window root "$out/ctx-$1.png" || true
    xdotool key Escape
    sleep 1
  done
  kill $pid 2>/dev/null || true
' _ "$concat" "$project" "$out"

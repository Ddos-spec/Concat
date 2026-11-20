#!/usr/bin/env bash
# Starts the window on the demo project under Xvfb and uses the four clip-menu
# actions the way a person would: right-click a clip, click the row. The rows
# are found on the screenshot with tesseract, so the script does not depend on
# where a menu opens.
#
#   drive-editor.sh CONCAT PROJECT OUTDIR
#
# Coordinates are for a 1600x900 screen, the demo project from
# make-demo-project.py (the three-colour clip and the panning clip on the
# bottom lane, the title above them), and the timeline at its opening zoom.
set -u
concat=$1
project=$2
out=$3
here=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$out"

xvfb-run -a -s "-screen 0 1600x900x24" bash -c '
  concat=$1; project=$2; out=$3; here=$4
  "$concat" --project "$project" > "$out/drive.log" 2>&1 &
  pid=$!
  sleep 30
  shot() { import -window root "$out/$1.png" || true; }

  # Right-click at X Y, find WORD on the menu that opens, click it.
  menu() {
    xdotool mousemove "$1" "$2" click 3
    sleep 2
    shot "menu-$3"
    if pos=$(python3 "$here/find-text.py" "$out/menu-$3.png" "$4" 30); then
      echo "menu $3: clicking $4 at $pos" | tee -a "$out/drive-steps.log"
      set -- $pos
      xdotool mousemove "$1" "$2" click 1
    else
      echo "menu $3: row $4 not found" | tee -a "$out/drive-steps.log"
      xdotool key Escape
    fi
  }

  shot 00-open
  menu 250 845 scenes scenes
  sleep 12; shot 01-after-scenes
  menu 207 761 karaoke karaoke
  sleep 4;  shot 02-after-karaoke
  menu 347 845 colour colour
  sleep 12; shot 03-after-colour
  xdotool mousemove 1340 112 click 1; sleep 2; shot 04-adjust-panel
  menu 207 761 follow follow
  sleep 12; shot 05-after-follow
  xdotool key ctrl+s; sleep 4; shot 06-saved
  kill $pid 2>/dev/null || true
' _ "$concat" "$project" "$out" "$here"

# What the editor wrote, in numbers.
python3 "$here/summarise-project.py" "$project" | tee "$out/project-summary.txt"

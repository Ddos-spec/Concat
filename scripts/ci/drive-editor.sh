#!/usr/bin/env bash
# Starts the window on the demo project under Xvfb and uses the four clip-menu
# actions the way a person would: right-click a clip, click the row.
#
#   drive-editor.sh CONCAT PROJECT OUTDIR
#
# Coordinates are for a 1600x900 screen, the demo project from
# make-demo-project.py (the three-colour clip and the panning clip on the
# bottom lane, the title above them) and the timeline at its opening zoom. A
# menu opened near the bottom of the screen is pushed up against its edge, so
# its rows are at fixed heights: for a video clip Split by scenes is at 524,
# Match colour at 576 and Follow a point at 602; for a title Karaoke is at 707
# and Follow a point at 759. A row is clicked 90 px right of the click that
# opened the menu, which is inside it (the menu is 218 px wide).
set -u
concat=$1
project=$2
out=$3
here=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$out"

xvfb-run -a -s "-screen 0 1600x900x24" bash -c '
  concat=$1; project=$2; out=$3
  "$concat" --project "$project" > "$out/drive.log" 2>&1 &
  pid=$!
  sleep 30
  shot() { import -window root "$out/$1.png" || true; }

  # Right-click at X Y, then click the row at height ROW of the menu.
  menu() {
    echo "menu $4: right-click $1,$2 then row $3" | tee -a "$out/drive-steps.log"
    xdotool mousemove "$1" "$2" click 3
    sleep 2
    shot "menu-$4"
    xdotool mousemove "$(( $1 + 90 ))" "$3" click 1
  }

  shot 00-open
  menu 250 845 524 scenes
  sleep 12; shot 01-after-scenes
  menu 205 761 707 karaoke
  sleep 4;  shot 02-after-karaoke
  menu 347 845 576 colour
  sleep 12; shot 03-after-colour
  xdotool mousemove 1340 112 click 1; sleep 2; shot 04-adjust-panel
  menu 347 761 759 follow
  sleep 12; shot 05-after-follow
  xdotool mousemove 347 845 click 1; sleep 1
  xdotool mousemove 1330 68 click 1; sleep 2; shot 07-hsl-inspector
  xdotool key ctrl+s; sleep 4; shot 06-saved
  kill $pid 2>/dev/null || true
' _ "$concat" "$project" "$out"

# What the editor wrote, in numbers.
python3 "$here/summarise-project.py" "$project" | tee "$out/project-summary.txt"

# Karaoke must cut at the times the words were said, not by letter count.
python3 "$here/check-karaoke.py" "$project" | tee "$out/karaoke-check.txt"
test "${PIPESTATUS[0]}" -eq 0

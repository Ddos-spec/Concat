#!/usr/bin/env python3
"""Prints "X Y" of the centre of the first word on a screenshot that contains WORD.

    find-text.py IMAGE.png WORD [MIN_Y]

Uses the tesseract command line. Exit 1 when the word is not found.
"""
import csv
import io
import subprocess
import sys


def main() -> int:
    image, word = sys.argv[1], sys.argv[2].lower()
    min_y = int(sys.argv[3]) if len(sys.argv) > 3 else 0
    out = subprocess.run(
        ["tesseract", image, "-", "--psm", "11", "tsv"],
        capture_output=True, text=True,
    ).stdout
    rows = csv.DictReader(io.StringIO(out), delimiter="\t", quoting=csv.QUOTE_NONE)
    for row in rows:
        text = (row.get("text") or "").strip().lower()
        if word in text and int(row["top"]) >= min_y:
            x = int(row["left"]) + int(row["width"]) // 2
            y = int(row["top"]) + int(row["height"]) // 2
            print(x, y)
            return 0
    return 1


if __name__ == "__main__":
    sys.exit(main())

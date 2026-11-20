#!/usr/bin/env python3
"""Prints the first panic in a cargo test log that is not a poisoned lock.

A test that panics while holding a lock makes every later test that takes
it panic with PoisonError, which buries the real cause under a dozen copies.

    first-panic.py TEST.log
"""
import sys


def main() -> int:
    lines = open(sys.argv[1], errors="replace").read().splitlines()
    for i, line in enumerate(lines):
        if "panicked at" in line and "PoisonError" not in " ".join(lines[i + 1:i + 3]):
            print("\n".join(lines[max(0, i - 3):i + 25]))
            return 0
    print("no panic other than a poisoned lock")
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Reads one `catalogue.list` response on stdin and says whether the effect
package named on the command line is in it, with how many knobs it has.

    echo '{"method":"catalogue.list","kind":"effect"}' | concat-cli api | find-effect.py concat.hsl
"""
import json
import sys


def walk(node):
    if isinstance(node, dict):
        yield node
        for value in node.values():
            yield from walk(value)
    elif isinstance(node, list):
        for value in node:
            yield from walk(value)


def main() -> int:
    wanted = sys.argv[1]
    line = sys.stdin.readline()
    try:
        response = json.loads(line)
    except json.JSONDecodeError:
        print("no JSON response:", line[:200])
        return 1
    for item in walk(response):
        if item.get("id") == wanted:
            params = item.get("params") or item.get("parameters") or []
            print(f"found {wanted}: name={item.get('name')!r} category={item.get('category')!r} knobs={len(params)}")
            return 0
    print(f"{wanted} NOT in the catalogue")
    return 1


if __name__ == "__main__":
    sys.exit(main())

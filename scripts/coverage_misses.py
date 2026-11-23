#!/usr/bin/env python3
"""The regions no test runs, from the report `scripts/coverage.sh --json` writes.

    python3 scripts/coverage_misses.py cov.json

A region of generic code counts once for each type it is used with, so a
region is listed when it ran in none of its copies.
"""
import collections
import json
import os
import sys

data = json.load(open(sys.argv[1]))["data"][0]
runs = collections.defaultdict(int)
for function in data["functions"]:
    names = function["filenames"]
    for l1, c1, l2, c2, count, file_id, _, kind in function["regions"]:
        # Code of this file (not a macro's expansion) in a file of this repository
        # (cargo-llvm-cov leaves out any folder called examples)
        local = names[0].startswith(os.getcwd() + "/src/") and "/examples/" not in names[0]
        if kind == 0 and file_id == 0 and local:
            runs[(names[file_id], l1, c1, l2, c2)] += count
sources = {}
for (path, l1, c1, l2, c2), count in sorted(runs.items()):
    if count == 0:
        lines = sources.setdefault(path, open(path, encoding="utf-8").read().split("\n"))
        print(f"missed region {path}:{l1}:{c1}-{l2}:{c2}  {lines[l1 - 1].strip()[:100]}")

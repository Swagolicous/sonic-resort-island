"""Append every recompiled function in guest address ranges to sheets/lift_roots.csv
(skipping ones already listed), or with --count just report how many there are.

usage: python re/xenon/add_range_roots.py [--count] <system> <lo hex>-<hi hex>...
"""
import csv
import sys
from pathlib import Path

import fn

SHEET = Path(__file__).resolve().parents[2] / "sheets" / "lift_roots.csv"
args = sys.argv[1:]
count = args[0] == "--count"
if count:
    args = args[1:]
system, ranges = args[0], [tuple(int(x, 16) for x in r.split("-")) for r in args[1:]]
idx = fn.index()
with open(SHEET, encoding="utf-8-sig", newline="") as fh:
    have = {r["address"] for r in csv.DictReader(fh)}
for lo, hi in ranges:
    inside = [a for a in sorted(idx) if lo <= int(a, 16) < hi]
    new = [a for a in inside if f"0x{a}" not in have]
    lines = sum(idx[a][2] - idx[a][1] for a in inside)
    print(f"{lo:08X}-{hi:08X}: {len(inside)} functions ({lines} recompiled lines), {len(new)} not yet roots")
    if not count:
        with open(SHEET, "a", encoding="utf-8", newline="") as fh:
            w = csv.writer(fh)
            for a in new:
                w.writerow([f"0x{a}", system, f"Whole range {lo:08X}-{hi:08X}."])

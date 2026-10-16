"""Append every vtable method of the RTTI classes matching a regex to sheets/lift_roots.csv
(Decomp/rtti_classes.jsonl; only methods XenonRecomp recompiled, skipping ones listed).

usage: python re/xenon/add_class_roots.py <system> <class regex>"""
import csv
import json
import re
import sys
from pathlib import Path

import fn

HERE = Path(__file__).resolve().parent
SHEET = HERE.parents[1] / "sheets" / "lift_roots.csv"
RTTI = HERE.parents[2] / "Decomp" / "rtti_classes.jsonl"

system, pattern = sys.argv[1], re.compile(sys.argv[2])
idx = fn.index()
with open(SHEET, encoding="utf-8-sig", newline="") as fh:
    have = {r["address"] for r in csv.DictReader(fh)}
new, classes = {}, set()
for line in open(RTTI, encoding="utf-8"):
    r = json.loads(line)
    if not pattern.search(r["class"]):
        continue
    classes.add(r["class"])
    for m in r["methods"]:
        a = f"0x{m.upper()}"
        if m.upper() in idx and a not in have:
            new.setdefault(a, r["class"])
with open(SHEET, "a", encoding="utf-8", newline="") as fh:
    w = csv.writer(fh)
    for a, cls in new.items():
        w.writerow([a, system, f"{cls} vtable method."])
print(f"lift_roots: +{len(new)} from {len(classes)} classes")

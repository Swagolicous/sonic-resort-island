"""Append guest functions to sheets/lift_roots.csv (skipping ones already listed).
usage: python re/xenon/add_roots.py <system> <doc> <addr>..."""
import csv
import sys
from pathlib import Path

SHEET = Path(__file__).resolve().parents[2] / "sheets" / "lift_roots.csv"
system, doc, addrs = sys.argv[1], sys.argv[2], sys.argv[3:]
with open(SHEET, encoding="utf-8-sig", newline="") as fh:
    have = {r["address"] for r in csv.DictReader(fh)}
new = [f"0x{a.upper().removeprefix('0X')}" for a in addrs]
# Only guest code addresses (discovery runs skip calls, so garbage targets appear).
new = [a for a in dict.fromkeys(new) if a not in have and 0x822C0000 <= int(a, 16) < 0x822C0000 + 0xF9189C]
with open(SHEET, "a", encoding="utf-8", newline="") as fh:
    w = csv.writer(fh)
    for a in new:
        w.writerow([a, system, doc])
print(f"lift_roots: +{len(new)}")

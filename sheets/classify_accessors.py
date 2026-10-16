"""Classify trivial guest accessors (field getters / setters / address-of / vector
copies on the speed context) from their recompiled instructions, record the fields
they prove in context_fields.csv, and mark them ported as `field:<struct>.<field>`.

Only exact instruction shapes are accepted; anything else is left for a real port.
usage: python sheets/classify_accessors.py <addr>...
"""
import csv
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "re" / "xenon"))
import fn  # noqa: E402

SHAPES = [
    # (regex over ';'-joined instructions, type, what)
    (r"lfs f1,(\d+)\(r3\);blr", "f32", "get"),
    (r"stfs f1,(\d+)\(r3\);blr", "f32", "set"),
    (r"lbz r3,(\d+)\(r3\);blr", "u8", "get"),
    (r"lwz r3,(\d+)\(r3\);blr", "ptr", "get"),
    (r"addi r3,r3,(\d+);blr", "vec4", "address of"),
    (r"li r11,(\d+);lvx128 v63,r4,r11;stvx128 v63,r0,r3;blr", "vec4", "copy out"),
    (r"li r11,(\d+);lvx128 v63,r0,r4;stvx128 v63,r3,r11;blr", "vec4", "copy in"),
]


def asm(idx, a):
    return ";".join(l.strip()[3:].strip() for l in fn.body(idx, a) if l.strip().startswith("// "))


def main():
    idx = fn.index()
    with open(HERE / "context_fields.csv", encoding="utf-8-sig", newline="") as fh:
        rows = list(csv.reader(fh))
    header, fields = rows[0], rows[1:]
    by_offset = {(r[0], int(r[1], 16)): r for r in fields}
    marks = []
    for a in (x.upper().removeprefix("0X") for x in sys.argv[1:]):
        text = asm(idx, a)
        for rx, typ, what in SHAPES:
            m = re.fullmatch(rx, text)
            if not m:
                continue
            off = int(m[1])
            key = ("speed_context", off)
            if key not in by_offset:
                row = ["speed_context", f"0x{off:03X}", f"field_0x{off:03x}", typ, f"sub_{a}", "Not identified yet."]
                fields.append(row)
                by_offset[key] = row
            else:
                row = by_offset[key]
                if f"sub_{a}" not in row[4].split(";"):
                    row[4] = row[4] + f";sub_{a}"
            marks.append((a, f"field:speed_context.{row[2]}", what))
            break
        else:
            print(f"  {a}: not a plain accessor: {text[:90]}")
    with open(HERE / "context_fields.csv", "w", encoding="utf-8-sig", newline="") as fh:
        csv.writer(fh).writerows([header, *fields])
    with open(HERE / "port_functions.csv", encoding="utf-8-sig", newline="") as fh:
        prow = list(csv.reader(fh))
    for a, rust, what in marks:
        hits = [r for r in prow[1:] if r[1] == f"0x{a}"]
        for r in hits:
            r[6] = rust
        print(f"  {a}: {what} -> {rust}{'' if hits else ' (not tracked)'}")
    with open(HERE / "port_functions.csv", "w", encoding="utf-8-sig", newline="") as fh:
        csv.writer(fh).writerows(prow)


if __name__ == "__main__":
    main()

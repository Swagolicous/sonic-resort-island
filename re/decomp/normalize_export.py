"""Make the Ghidra export indexable by gamedb without losing the original.

Ghidra's C output trips two gamedb parser assumptions:
  * long signatures put the return type on its own line ("undefined4 *\\nFunc(...)")
  * methods in recovered namespaces are defined with qualified names (A::B::vf0)

This keeps Ghidra's output untouched in Decomp/src_raw and writes a normalized
copy to Decomp/src:
  * CRLF -> LF
  * a signature whose first line has no '(' is joined with the next line
  * "::" in identifiers becomes "__" everywhere (definitions and call sites alike,
    so name-based call edges still connect)

It then regenerates manifest.jsonl (line ranges + sha256 of the normalized body)
from the "// @function" markers, keeping every other field per address.
Re-runnable: always rebuilds src from src_raw.

usage: python normalize_export.py [--decomp D:\\UnleashedRecomp-Windows\\Decomp]
"""

import argparse
import hashlib
import json
import re
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
QUALIFIED = re.compile(r"\b([A-Za-z_]\w*)::(?=[A-Za-z_~])")
MARKER = re.compile(r"^// @function (\S+) ")


def normalize(text: str) -> str:
    text = text.replace("\r\n", "\n").replace("\r", "\n")
    lines = text.split("\n")
    out = []
    i = 0
    while i < len(lines):
        line = lines[i]
        out.append(line)
        if MARKER.match(line):
            # Ghidra may put /* WARNING ... */ lines (and blanks) before the signature.
            j = i + 1
            while j < len(lines) and (lines[j].lstrip().startswith("/*") or not lines[j].strip()):
                out.append(lines[j])
                j += 1
            if j + 1 < len(lines):
                sig = lines[j]
                if "(" not in sig and not sig.startswith("//") and lines[j + 1].strip():
                    out.append(sig.rstrip() + " " + lines[j + 1].lstrip())
                    i = j + 2
                    continue
            i = j
            continue
        i += 1
    text = "\n".join(out)
    # Repeat until stable so A::B::C collapses fully.
    while True:
        new = QUALIFIED.sub(r"\1__", text)
        if new == text:
            return text
        text = new


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--decomp", default=str(ROOT / "Decomp"))
    args = ap.parse_args()
    decomp = Path(args.decomp)
    src, raw = decomp / "src", decomp / "src_raw"
    manifest_path = decomp / "manifest.jsonl"
    raw_manifest = decomp / "manifest_raw.jsonl"

    if not raw.exists():
        # First run: the current export becomes the preserved original.
        src.rename(raw)
        shutil.copy2(manifest_path, raw_manifest)
    else:
        shutil.rmtree(src, ignore_errors=True)
    old = {json.loads(l)["address"]: json.loads(l) for l in raw_manifest.open(encoding="utf-8")}

    records = []
    for path in sorted(raw.rglob("*.c")):
        rel = path.relative_to(raw).as_posix()
        text = normalize(path.read_bytes().decode("utf-8"))
        target = src / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(text.encode("utf-8"))
        lines = text.split("\n")
        markers = [(i, MARKER.match(l).group(1)) for i, l in enumerate(lines) if MARKER.match(l)]
        for k, (i, addr) in enumerate(markers):
            end = markers[k + 1][0] if k + 1 < len(markers) else len(lines)
            # Body = lines after the marker up to the blank separator.
            j = end - 1
            while j > i and not lines[j].strip():
                j -= 1
            body = "\n".join(lines[i + 1:j + 1]) + "\n"
            r = dict(old[addr])
            r.update(file=rel, line_start=i + 1, line_end=j + 1,
                     sha256=hashlib.sha256(body.encode("utf-8")).hexdigest())
            records.append(r)

    missing = set(old) - {r["address"] for r in records}
    with manifest_path.open("w", encoding="utf-8") as f:
        for r in records:
            f.write(json.dumps(r) + "\n")
    print(f"normalized {len(list(src.rglob('*.c')))} files, {len(records)} records "
          f"({len(missing)} records lost: {sorted(missing)[:5]})")
    return 1 if missing else 0


if __name__ == "__main__":
    sys.exit(main())

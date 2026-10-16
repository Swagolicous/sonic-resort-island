"""Translate the Xenos programs listed in sheets/shader_programs.csv to HLSL.

Runs XenosRecomp (build-ninja, see build.bat) in single-file mode on each program's
microcode in shader/ and writes hlsl/<program>.hlsl. The sheet decides what gets
translated; nothing else is.

usage: python re/xenos/translate.py [--force]
"""
import csv
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
RS = HERE.parents[1]
EXE = HERE / "build-ninja" / "XenosRecomp" / "XenosRecomp.exe"
HEADER = RS.parent / "refs" / "UnleashedRecomp" / "tools" / "XenosRecomp" / "XenosRecomp" / "shader_common.h"


def main():
    with open(RS / "sheets" / "shader_programs.csv", encoding="utf-8-sig", newline="") as fh:
        rows = list(csv.DictReader(fh))
    (HERE / "hlsl").mkdir(exist_ok=True)
    failed = 0
    for r in rows:
        out = HERE / "hlsl" / f"{r['program']}.hlsl"
        if out.is_file() and "--force" not in sys.argv:
            continue
        res = subprocess.run([str(EXE), str(HERE / "shader" / r["source"]), str(out), str(HEADER)],
                             capture_output=True, text=True)
        if res.returncode or not out.is_file() or out.stat().st_size == 0:
            failed += 1
            print(f"FAIL {r['program']}: {res.stdout[-300:]}{res.stderr[-300:]}")
        else:
            print(f"ok   {r['program']} ({out.stat().st_size} bytes)")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()

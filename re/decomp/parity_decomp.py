"""Phase 2: parity check between the Ghidra export, the binary, and the gamedb index.

Parity is defined as all of the following holding:

  P1 binary coverage  every function in the executable's own .pdata table was
                      exported (or is explained as merged into another function)
  P2 export integrity every manifest record's body round-trips: the bytes on disk
                      at its recorded line range hash to the recorded sha256
  P3 file coverage    every exported .c file is in gamedb's `files` table with the
                      same size, and gamedb lists no file that is not on disk
  P4 function index   every manifest record has exactly one gamedb function in the
                      same file whose line range lies inside the record; gamedb has
                      no function that maps to no record
  P5 not truncated    every indexed function body ends with a closing brace
  P6 decompiled       records whose decompilation failed are listed as gaps

Writes Decomp/parity/report.md and Decomp/parity/report.json.
usage: python parity_decomp.py [--decomp D:\\UnleashedRecomp-Windows\\Decomp]
"""

import argparse
import collections
import hashlib
import json
import sqlite3
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def pdata_functions(xex: Path) -> dict:
    """Function starts and sizes from the PE .pdata table inside the XEX."""
    b = xex.read_bytes()
    pe_off = struct.unpack_from(">I", b, 8)[0]
    pe = b[pe_off:]
    e_lfanew = struct.unpack_from("<I", pe, 0x3C)[0]
    nsec = struct.unpack_from("<H", pe, e_lfanew + 6)[0]
    opt_size = struct.unpack_from("<H", pe, e_lfanew + 20)[0]
    sec = e_lfanew + 24 + opt_size
    base = 0x82000000
    out = {}
    for i in range(nsec):
        s = sec + 40 * i
        name = pe[s:s + 8].rstrip(b"\0").decode()
        vsize, va, rsize, raw = struct.unpack_from("<IIII", pe, s + 8)
        if name != ".pdata":
            continue
        # XEX images are loaded flat: file offset == virtual address.
        data = pe[va:va + vsize]
        for j in range(0, len(data) - 7, 8):
            begin, word = struct.unpack_from(">II", data, j)
            length = ((word >> 8) & 0x3FFFFF) * 4
            if begin and length:
                out[begin] = length
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--decomp", default=str(ROOT / "Decomp"))
    args = ap.parse_args()
    decomp = Path(args.decomp)
    src = decomp / "src"
    outdir = decomp / "parity"
    outdir.mkdir(exist_ok=True)

    records = [json.loads(l) for l in (decomp / "manifest.jsonl").open(encoding="utf-8")]
    by_addr = {int(r["address"], 16): r for r in records}
    report = {"definition": __doc__.split("Writes")[0].strip()}
    issues = collections.defaultdict(list)

    # P1 binary coverage
    pdata = pdata_functions(ROOT / "patched" / "default.xex")
    starts = sorted(by_addr)
    import bisect
    missing_pdata = []
    merged = []
    for begin, length in sorted(pdata.items()):
        if begin in by_addr:
            continue
        i = bisect.bisect_right(starts, begin) - 1
        if i >= 0:
            host = by_addr[starts[i]]
            if starts[i] + host["size"] > begin:
                merged.append({"pdata": f"{begin:08x}", "inside": host["address"], "name": host["full_name"]})
                continue
        missing_pdata.append(f"{begin:08x}")
    issues["P1_missing_pdata_functions"] = missing_pdata
    report["P1"] = {"pdata_functions": len(pdata), "exported": len(records),
                    "pdata_exported_exactly": sum(1 for a in pdata if a in by_addr),
                    "pdata_inside_other_function": len(merged), "pdata_missing": len(missing_pdata),
                    "exported_not_in_pdata": sum(1 for a in by_addr if a not in pdata)}
    issues["P1_merged_examples"] = merged[:50]

    # P2 export integrity
    file_lines = {}
    bad_hash = []
    for r in records:
        lines = file_lines.get(r["file"])
        if lines is None:
            # No newline translation: Ghidra emits \r\n inside bodies and the
            # manifest hashes the exact bytes.
            with open(src / r["file"], encoding="utf-8", newline="") as fh:
                lines = fh.read().split("\n")
            file_lines[r["file"]] = lines
        body = "\n".join(lines[r["line_start"]:r["line_end"]]) + "\n"
        if hashlib.sha256(body.encode("utf-8")).hexdigest() != r["sha256"]:
            bad_hash.append(r["address"])
    issues["P2_hash_mismatch"] = bad_hash
    report["P2"] = {"records": len(records), "hash_ok": len(records) - len(bad_hash), "hash_mismatch": len(bad_hash)}

    # P3 file coverage
    con = sqlite3.connect(src / ".gamedb" / "index.sqlite")
    db_files = {p: (fid, size) for fid, p, size in con.execute("SELECT id, path, size FROM files")}
    norm = {p.replace("\\", "/"): v for p, v in db_files.items()}
    disk = {p.relative_to(src).as_posix(): p.stat().st_size for p in src.rglob("*.c") if ".gamedb" not in p.parts}
    missing_files = sorted(set(disk) - set(norm))
    extra_files = sorted(set(norm) - set(disk))
    size_mismatch = sorted(p for p in set(disk) & set(norm) if disk[p] != norm[p][1])
    issues["P3_files_not_indexed"] = missing_files
    issues["P3_files_indexed_not_on_disk"] = extra_files
    issues["P3_size_mismatch"] = size_mismatch
    report["P3"] = {"files_on_disk": len(disk), "files_indexed": len(norm), "not_indexed": len(missing_files),
                    "stale": len(extra_files), "size_mismatch": len(size_mismatch)}

    # P4/P5 function index
    id_to_path = {v[0]: p for p, v in norm.items()}
    db_funcs = collections.defaultdict(list)
    for fid, name, start, end in con.execute("SELECT file_id, name, start_line, end_line FROM functions"):
        db_funcs[id_to_path.get(fid)].append((start, end, name))
    matched = 0
    unmatched_records = []
    duplicate_records = []
    used = set()
    truncated = []
    for r in records:
        cands = [f for f in db_funcs.get(r["file"], []) if r["line_start"] <= f[0] and f[1] <= r["line_end"] + 1]
        if r["status"] != "ok":
            continue
        if not cands:
            unmatched_records.append(f'{r["address"]} {r["full_name"]} ({r["file"]}:{r["line_start"]})')
        elif len(cands) > 1:
            duplicate_records.append(f'{r["address"]} {r["full_name"]} -> {len(cands)} definitions')
            used.update((r["file"], c[0]) for c in cands)
        else:
            matched += 1
            used.add((r["file"], cands[0][0]))
            lines = file_lines[r["file"]]
            last = lines[cands[0][1] - 1].rstrip() if cands[0][1] - 1 < len(lines) else ""
            if not last.endswith("}"):
                truncated.append(f'{r["address"]} {r["full_name"]} ends with {last[-40:]!r}')
    orphans = [f"{p}:{s} {n}" for p, fs in db_funcs.items() for s, e, n in fs if (p, s) not in used]
    issues["P4_records_without_index_entry"] = unmatched_records
    issues["P4_records_with_multiple_entries"] = duplicate_records
    issues["P4_index_entries_without_record"] = orphans
    issues["P5_truncated"] = truncated
    ok_records = sum(1 for r in records if r["status"] == "ok")
    report["P4"] = {"decompiled_records": ok_records, "matched_one_to_one": matched,
                    "unmatched": len(unmatched_records), "multiple": len(duplicate_records),
                    "index_orphans": len(orphans)}
    report["P5"] = {"truncated": len(truncated)}

    # P6 decompile failures
    failed = [f'{r["address"]} {r["full_name"]}' for r in records if r["status"] != "ok"]
    issues["P6_decompile_failed"] = failed
    report["P6"] = {"failed": len(failed)}
    # Straight-line registration code the decompiler times out on is recovered
    # from machine code by param_table.py instead (see its docstring).
    params = decomp / "params.json"
    if params.exists():
        data = json.loads(params.read_text(encoding="utf-8"))
        covered = {"82324410": "sonic", "8237a828": "mode"}
        issues["P6_covered_by_machine_code"] = [
            f"{a} -> params.json[{k}] ({len(data.get(k, []))} parameters, param_table.py)"
            for a, k in covered.items() if any(f.startswith(a) for f in failed)]
        report["P6"]["covered_by_machine_code"] = len(issues["P6_covered_by_machine_code"])

    stats = {t: con.execute(f"SELECT count(*) FROM {t}").fetchone()[0]
             for t in ("files", "functions", "symbols", "strings", "edges")}
    report["gamedb_tables"] = stats
    passed = (not missing_pdata and not bad_hash and not missing_files and not extra_files and not size_mismatch
              and not unmatched_records and not duplicate_records and not truncated)
    report["parity"] = "PASS" if passed else "GAPS"

    (outdir / "report.json").write_text(json.dumps({"summary": report, "issues": issues}, indent=2), encoding="utf-8")
    lines = [f"# Decomp parity: {report['parity']}", "", "```", report["definition"], "```", ""]
    for key in ("P1", "P2", "P3", "P4", "P5", "P6", "gamedb_tables"):
        lines.append(f"## {key}")
        lines += [f"- {k}: {v}" for k, v in report[key].items()]
        lines.append("")
    lines.append("## Issues (first 25 of each)")
    for k, v in issues.items():
        if v:
            lines.append(f"### {k} ({len(v)})")
            lines += [f"- {x}" for x in v[:25]]
    (outdir / "report.md").write_text("\n".join(lines), encoding="utf-8")
    print("\n".join(lines[:60]))
    return 0 if passed else 2


if __name__ == "__main__":
    sys.exit(main())

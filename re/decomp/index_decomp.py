"""Phase 1: index the Ghidra export into SQLite with gamedb, then attach the
port-reference tables gamedb itself does not carry.

Re-runnable: gamedb's own pass is incremental (mtime + size), and the
companion tables are rebuilt inside one transaction from the export's
manifest, so a re-run over unchanged input converges to the same database.

usage: python index_decomp.py [--decomp D:\\UnleashedRecomp-Windows\\Decomp] [--force]
"""

import argparse
import hashlib
import json
import sqlite3
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]  # D:\UnleashedRecomp-Windows
GAMEDB = ROOT / "refs" / "gamedb" / "target" / "release" / "gamedb.exe"

SCHEMA = """
CREATE TABLE IF NOT EXISTS port_functions(
    address TEXT PRIMARY KEY,          -- guest address, e.g. 82321c20
    name TEXT NOT NULL,                -- Ghidra short name
    full_name TEXT NOT NULL,           -- namespace-qualified name
    size INTEGER NOT NULL,             -- bytes in the function body
    path TEXT NOT NULL,                -- source file, relative to the gamedb root
    line_start INTEGER NOT NULL,       -- the '// @function' marker line
    line_end INTEGER NOT NULL,         -- last line of the body
    status TEXT NOT NULL,              -- ok | failed
    sha256 TEXT NOT NULL,              -- of the exported body text
    callees TEXT NOT NULL);            -- JSON array of guest addresses (Ghidra call graph)
CREATE INDEX IF NOT EXISTS port_functions_name ON port_functions(name);
CREATE INDEX IF NOT EXISTS port_functions_path ON port_functions(path);
CREATE TABLE IF NOT EXISTS port_edges(
    src TEXT NOT NULL, dst TEXT NOT NULL, PRIMARY KEY(src, dst)) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS port_edges_dst ON port_edges(dst);
CREATE TABLE IF NOT EXISTS port_classes(
    class TEXT NOT NULL, mangled TEXT NOT NULL, type_descriptor TEXT NOT NULL,
    vtable TEXT PRIMARY KEY, offset INTEGER NOT NULL, methods TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS port_classes_class ON port_classes(class);
CREATE TABLE IF NOT EXISTS port_files(
    path TEXT PRIMARY KEY, size INTEGER NOT NULL, sha256 TEXT NOT NULL,
    content BLOB NOT NULL);            -- raw source, for diffing the Rust port against
CREATE TABLE IF NOT EXISTS port_aliases(
    address TEXT NOT NULL,             -- guest address of a virtual method
    alias TEXT NOT NULL,               -- Class::vfN (most-base class whose vtable holds it)
    class TEXT NOT NULL, slot INTEGER NOT NULL,
    PRIMARY KEY(address, alias));
CREATE INDEX IF NOT EXISTS port_aliases_alias ON port_aliases(alias);
CREATE INDEX IF NOT EXISTS port_aliases_class ON port_aliases(class);
CREATE TABLE IF NOT EXISTS port_meta(key TEXT PRIMARY KEY, value TEXT);
"""


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--decomp", default=str(ROOT / "Decomp"))
    ap.add_argument("--force", action="store_true", help="pass --force to gamedb index")
    args = ap.parse_args()

    decomp = Path(args.decomp)
    src = decomp / "src"
    manifest = decomp / "manifest.jsonl"
    if not src.is_dir() or not manifest.is_file():
        print(f"missing export: {src} / {manifest}", file=sys.stderr)
        return 1
    logs = decomp / "logs"
    logs.mkdir(exist_ok=True)

    # gamedb pass (its own transaction; incremental unless --force).
    cmd = [str(GAMEDB), "index", "-r", str(src), "--verbose=5000"]
    if args.force:
        cmd.append("--force")
    print("running:", " ".join(cmd))
    proc = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", errors="replace")
    (logs / "gamedb_index.log").write_text(proc.stdout + "\n--- stderr ---\n" + proc.stderr, encoding="utf-8")
    print(proc.stdout.strip()[-2000:])
    if proc.returncode != 0:
        print(proc.stderr, file=sys.stderr)
        return proc.returncode

    db_path = src / ".gamedb" / "index.sqlite"
    con = sqlite3.connect(db_path)
    con.executescript(SCHEMA)
    with con:
        for table in ("port_functions", "port_edges", "port_classes", "port_files", "port_aliases", "port_meta"):
            con.execute(f"DELETE FROM {table}")

        functions = 0
        edges = 0
        with manifest.open(encoding="utf-8") as f:
            for line in f:
                r = json.loads(line)
                path = r["file"]  # relative to the gamedb root (Decomp/src)
                con.execute(
                    "INSERT INTO port_functions VALUES(?,?,?,?,?,?,?,?,?,?)",
                    (r["address"], r["name"], r["full_name"], r["size"], path, r["line_start"],
                     r["line_end"], r["status"], r["sha256"], json.dumps(r["callees"])))
                functions += 1
                for c in r["callees"]:
                    con.execute("INSERT OR IGNORE INTO port_edges VALUES(?,?)", (r["address"], c))
                    edges += 1

        classes = 0
        aliases = 0
        rtti = decomp / "rtti_classes.jsonl"
        if rtti.is_file():
            vtables = [json.loads(line) for line in rtti.open(encoding="utf-8")]
            # Each virtual method belongs to the class with the shortest vtable
            # that holds it (the most-base class that introduced the slot).
            owner = {}
            for r in vtables:
                con.execute("INSERT OR REPLACE INTO port_classes VALUES(?,?,?,?,?,?)",
                            (r["class"], r["mangled"], r["type_descriptor"], r["vtable"], r["offset"],
                             json.dumps(r["methods"])))
                classes += 1
                for slot, addr in enumerate(r["methods"]):
                    best = owner.get(addr)
                    if best is None or len(r["methods"]) < best[2]:
                        owner[addr] = (r["class"], slot, len(r["methods"]))
                    # Every class's own slot is also recorded, so overrides are findable.
                    con.execute("INSERT OR IGNORE INTO port_aliases VALUES(?,?,?,?)",
                                (addr.lower().lstrip("0") or "0", f'{r["class"]}::vf{slot}', r["class"], slot))
                    aliases += 1

        files = 0
        for p in sorted(src.rglob("*.c")):
            if ".gamedb" in p.parts:
                continue
            data = p.read_bytes()
            rel = p.relative_to(src).as_posix()
            con.execute("INSERT INTO port_files VALUES(?,?,?,?)", (rel, len(data), sha256_bytes(data), data))
            files += 1

        meta = {
            "gamedb": str(GAMEDB),
            "gamedb_sha256": sha256_bytes(GAMEDB.read_bytes()),
            "manifest_sha256": sha256_bytes(manifest.read_bytes()),
            "binary": str(ROOT / "patched" / "default.xex"),
            "binary_sha256": sha256_bytes((ROOT / "patched" / "default.xex").read_bytes()),
        }
        for k, v in meta.items():
            con.execute("INSERT INTO port_meta VALUES(?,?)", (k, v))

    stats = {t: con.execute(f"SELECT count(*) FROM {t}").fetchone()[0]
             for t in ("files", "functions", "symbols", "strings", "edges",
                       "port_functions", "port_edges", "port_classes", "port_aliases", "port_files")}
    con.close()
    print(json.dumps({"db": str(db_path), "inserted": {"functions": functions, "edges": edges,
                                                       "classes": classes, "files": files},
                      "tables": stats}, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())

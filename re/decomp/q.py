"""Query the indexed decompile for the Rust port.

usage:
  q.py class <Namespace::Class>     vtable slots -> address, name, size, file:line
  q.py fn <address|name>            print the decompiled body
  q.py callers <address>            functions that call it (Ghidra call graph)
  q.py callees <address>            functions it calls
  q.py str <substring>              functions whose body references a string literal
  q.py find <Class substring>       list RTTI classes matching
"""

import json
import re
import sqlite3
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SRC = ROOT / "Decomp" / "src"
DB = SRC / ".gamedb" / "index.sqlite"


def con():
    return sqlite3.connect(DB)


def norm(addr: str) -> str:
    return addr.lower().replace("0x", "").lstrip("0")


def body(c, address: str) -> str:
    row = c.execute("SELECT path, line_start, line_end FROM port_functions WHERE address=?", (address,)).fetchone()
    if not row:
        return f"<no function at {address}>"
    path, a, b = row
    lines = (SRC / path).read_text(encoding="utf-8").split("\n")
    return "\n".join(lines[a - 1:b])


def describe(c, address: str) -> str:
    row = c.execute("SELECT full_name, size, path, line_start FROM port_functions WHERE address=?", (address,)).fetchone()
    if not row:
        return f"{address} <not exported>"
    name, size, path, line = row
    return f"{address} {name:40s} {size:6d}B {path}:{line}"


def main(argv):
    if len(argv) < 3:
        print(__doc__)
        return 2
    cmd, arg = argv[1], " ".join(argv[2:])
    c = con()
    if cmd == "find":
        for (cls,) in c.execute("SELECT DISTINCT class FROM port_classes WHERE class LIKE ? ORDER BY class",
                                (f"%{arg}%",)):
            print(cls)
    elif cmd == "class":
        for vtable, offset, methods in c.execute(
                "SELECT vtable, offset, methods FROM port_classes WHERE class=?", (arg,)):
            print(f"vtable {vtable} (offset {offset})")
            for slot, addr in enumerate(json.loads(methods)):
                a = norm(addr)
                owner = c.execute(
                    "SELECT class FROM port_aliases WHERE address=? ORDER BY length(class) LIMIT 1", (a,)).fetchone()
                inherited = "" if owner is None or owner[0] == arg else f"  (from {owner[0]})"
                print(f"  vf{slot:<3} {describe(c, a)}{inherited}")
    elif cmd == "fn":
        a = norm(arg) if re.fullmatch(r"(0x)?[0-9a-fA-F]{8}", arg) else None
        if a is None:
            row = c.execute("SELECT address FROM port_functions WHERE name=? OR full_name=?", (arg, arg)).fetchone()
            row = row or c.execute("SELECT address FROM port_aliases WHERE alias=?", (arg,)).fetchone()
            a = row[0] if row else arg
        print(describe(c, a))
        print(body(c, a))
    elif cmd in ("callers", "callees"):
        a = norm(arg)
        q = ("SELECT src FROM port_edges WHERE dst=?" if cmd == "callers" else "SELECT dst FROM port_edges WHERE src=?")
        for (other,) in c.execute(q, (a,)):
            print(describe(c, other))
    elif cmd == "str":
        for path, line, text in c.execute(
                "SELECT f.path, s.line, s.text FROM strings s JOIN files f ON f.id=s.file_id WHERE s.text LIKE ? LIMIT 200",
                (f"%{arg}%",)):
            fn = c.execute(
                "SELECT address, full_name FROM port_functions WHERE path=? AND line_start<=? AND line_end>=?",
                (path.replace("\\", "/"), line, line)).fetchone()
            print(f"{path}:{line} {text!r} in {fn}")
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

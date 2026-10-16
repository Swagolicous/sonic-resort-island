"""Read-only data addresses (0x82000000..0x82200000) that recompiled functions build
(a register set to the high half, then + the low half), for the functions given and the
functions they call directly, to a depth. Prints `function address` pairs; feed the
addresses to the peek example to see the strings.

usage: python re/xenon/consts.py <depth> <addr>...
"""
import re
import sys

import fn

PAIR = re.compile(r"((?:ctx\.)?r\d+)\.s64 = (-\d+);")


def consts(lines):
    out, high = [], {}
    for l in lines:
        if l.lstrip().startswith("//"):
            continue
        m = PAIR.search(l)
        if m and int(m[2]) % 65536 == 0:
            high[m[1]] = int(m[2])
            continue
        m = re.search(r"((?:ctx\.)?r\d+)\.s64 = ((?:ctx\.)?r\d+)\.s64 \+ (-?\d+);", l)
        if m and m[2] in high:
            a = (high[m[2]] + int(m[3])) & 0xFFFFFFFF
            if 0x82000000 <= a < 0x82200000:
                out.append(a)
    return out


def main():
    depth, roots = int(sys.argv[1]), [fn.norm(a) for a in sys.argv[2:]]
    idx = fn.index()
    seen, frontier = set(roots), list(roots)
    for d in range(depth + 1):
        nxt = []
        for a in frontier:
            if a not in idx:
                continue
            body = fn.body(idx, a)
            for c in dict.fromkeys(consts(body)):
                print(f"{a} {c:08X}")
            if d < depth:
                for c in dict.fromkeys(re.findall(r"\bsub_([0-9A-F]{8})\(ctx, base\)", "\n".join(body))):
                    if c not in seen:
                        seen.add(c)
                        nxt.append(c)
        frontier = nxt


if __name__ == "__main__":
    main()

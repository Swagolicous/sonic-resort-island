"""Linear constant-propagation over a PowerPC function: resolves the arguments of
every call (r3-r10, f1-f4, and values pointed to by stack-address arguments).

Good enough for straight-line registration code (param tables, string tables).
usage: python ppc_calls.py <func_addr> <size_hex> [target_filter...]
"""
import struct
import sys

from ppc import BASE, Image

VOLATILE_G = [0] + list(range(3, 13))
VOLATILE_F = list(range(0, 14))


def s32(v):
    v &= 0xFFFFFFFF
    return v - (1 << 32) if v & 0x80000000 else v


def calls(img, start, size, hook=None):
    g = {}
    f = {}
    stack = {}
    out = []
    for ins in img.insns(start, size):
        m = ins.mnemonic
        ops = [o for o in ins.op_str.replace("(", ",").replace(")", "").split(",")]
        ops = [o.strip() for o in ops if o.strip()]

        def reg(o):
            return int(o[1:]) if o[0] in "rf" and o[1:].isdigit() else None

        def imm(o):
            return int(o, 0)

        try:
            if m == "lis":
                g[reg(ops[0])] = s32(imm(ops[1]) << 16)
            elif m == "li":
                g[reg(ops[0])] = imm(ops[1])
            elif m in ("addi", "subi"):
                d, a, i = reg(ops[0]), reg(ops[1]), imm(ops[2])
                if m == "subi":
                    i = -i
                if a == 1:
                    g[d] = ("sp", i)
                elif isinstance(g.get(a), int):
                    g[d] = s32(g[a] + i)
                else:
                    g.pop(d, None)
            elif m == "ori":
                d, a, i = reg(ops[0]), reg(ops[1]), imm(ops[2])
                g[d] = g[a] | i if isinstance(g.get(a), int) else None
            elif m == "mr":
                d, a = reg(ops[0]), reg(ops[1])
                if a in g:
                    g[d] = g[a]
                else:
                    g.pop(d, None)
            elif m == "fmr":
                d, a = reg(ops[0]), reg(ops[1])
                f[d] = f.get(a)
            elif m == "lfs":
                d, off, a = reg(ops[0]), imm(ops[1]), reg(ops[2])
                if a == 1:
                    f[d] = stack.get(off)
                elif isinstance(g.get(a), int):
                    f[d] = img.f32((g[a] + off) & 0xFFFFFFFF)
                else:
                    f[d] = None
            elif m == "stfs":
                s, off, a = reg(ops[0]), imm(ops[1]), reg(ops[2])
                if a == 1:
                    stack[off] = f.get(s)
                elif isinstance(g.get(a), tuple):
                    stack[g[a][1] + off] = f.get(s)
            elif m == "stw":
                s, off, a = reg(ops[0]), imm(ops[1]), reg(ops[2])
                if a == 1:
                    stack[off] = g.get(s)
                elif isinstance(g.get(a), tuple):
                    stack[g[a][1] + off] = g.get(s)
            elif m == "stb":
                s, off, a = reg(ops[0]), imm(ops[1]), reg(ops[2])
                if a == 1:
                    stack[off] = g.get(s)
                elif isinstance(g.get(a), tuple):
                    stack[g[a][1] + off] = g.get(s)
            elif m == "lwz":
                d, off, a = reg(ops[0]), imm(ops[1]), reg(ops[2])
                if a == 1:
                    g[d] = stack.get(off)
                elif isinstance(g.get(a), int):
                    g[d] = ("mem", (g[a] + off) & 0xFFFFFFFF)
                else:
                    g.pop(d, None)
            elif m == "bl":
                tgt = imm(ops[0])
                args = {f"r{r}": g.get(r) for r in range(3, 11) if r in g}
                fargs = {f"f{r}": f.get(r) for r in range(1, 5) if f.get(r) is not None}
                out.append((ins.address, tgt, args, fargs, dict(stack)))
                ret = hook(ins.address, tgt, g, f, stack) if hook else None
                for r in VOLATILE_G:
                    g.pop(r, None)
                for r in VOLATILE_F:
                    f.pop(r, None)
                if ret is not None:
                    g[3] = ret
            elif ops and reg(ops[0]) is not None and m not in ("cmplwi", "cmpwi", "cmplw", "cmpw", "stwu", "stwux",
                                                                 "stfd", "std", "b", "beq", "bne", "blt", "bgt",
                                                                 "ble", "bge", "fcmpu"):
                r = reg(ops[0])
                if ops[0][0] == "r":
                    g.pop(r, None)
                else:
                    f.pop(r, None)
        except (ValueError, IndexError):
            pass
    return out


def main():
    img = Image()
    start = int(sys.argv[1], 16)
    size = int(sys.argv[2], 16)
    filt = {int(x, 16) for x in sys.argv[3:]}
    import collections
    res = calls(img, start, size)
    if not filt:
        c = collections.Counter(t for _, t, *_ in res)
        for t, n in c.most_common(40):
            print(f"{t:08x} x{n}")
        return
    for addr, tgt, args, fargs, stack in res:
        if tgt in filt:
            print(f"{addr:08x} {tgt:08x} {args} {fargs}")


if __name__ == "__main__":
    main()

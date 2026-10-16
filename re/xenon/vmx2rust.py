"""Mechanically translate a straight-line VMX function from XenonRecomp's C++ to Rust.

Accepts only functions with no calls and no branches: vector loads/stores through
argument registers, simde intrinsics, the vslw loops and the vupkd3d128 temp
sequence. Anything else is an error, never a guess. The Rust uses
crates/unleashed-game/src/sonic/vmx.rs and keeps every operation and its order.

Each (register, offset) the function loads becomes an argument (guest x,y,z,w);
each it stores becomes a returned vector, in store order.

usage: python vmx2rust.py <addr>     (prints the Rust function)
"""
import re
import sys

import fn

LOAD = r"simde_mm_shuffle_epi8\(simde_mm_load_si128\(\(simde__m128i\*\)\(base \+ \(\((.+?)\) & ~0xF\)\)\), simde_mm_load_si128\(\(simde__m128i\*\)VectorMaskL\)\)"
REG = r"(?:ctx\.)?(v\d+)\.(?:u8|u32|f32)"


class Fail(Exception):
    pass


def split_args(s):
    out, depth, cur = [], 0, ""
    for ch in s:
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
        if ch == "," and depth == 0:
            out.append(cur.strip())
            cur = ""
        else:
            cur += ch
    out.append(cur.strip())
    return out


class Translator:
    def __init__(self, addr):
        self.addr = addr
        self.consts = {}  # scalar regs set to an immediate
        self.inputs = []  # (reg, offset) in first-load order
        self.outputs = []
        self.written = set()
        self.read = set()
        self.lines = []

    def address(self, text):
        parts = [p.strip() for p in text.split("+")]
        reg, off = None, 0
        for p in parts:
            m = re.fullmatch(r"ctx\.(r\d+)\.u32", p)
            if m and reg is None:
                reg = m[1]
                continue
            m = re.fullmatch(r"(?:ctx\.)?(r\d+)\.u32", p)
            if m and m[1] in self.consts:
                off += self.consts[m[1]]
                continue
            m = re.fullmatch(r"(-?\d+)", p)
            if m:
                off += int(m[1])
                continue
            raise Fail(f"address {text!r}")
        if reg is None or reg in ("r1",):
            raise Fail(f"address {text!r} (stack or no argument register)")
        return reg, off

    def var(self, name):
        if name not in self.written:
            raise Fail(f"{name} read before written (caller state)")
        return name

    def expr(self, e):
        e = e.strip()
        m = re.fullmatch(LOAD, e)
        if m:
            key = self.address(m[1])
            if key not in self.inputs:
                self.inputs.append(key)
            return f"from_guest({key[0]}_{key[1]})"
        m = re.fullmatch(r"simde_mm_load_ps\(" + REG + r"\)", e) or re.fullmatch(r"simde_mm_load_si128\(\(simde__m128i\*\)" + REG + r"\)", e)
        if m:
            return self.var(m[1])
        m = re.fullmatch(r"(\w+)\((.*)\)", e, re.S)
        if not m:
            raise Fail(f"expression {e[:80]!r}")
        name, args = m[1], split_args(m[2])
        unary = {"simde_mm_castsi128_ps": "", "simde_mm_castps_si128": ""}
        if name in unary:
            return self.expr(args[0])
        if name == "simde_mm_set1_epi32":
            v = re.fullmatch(r"int\((0x[0-9A-Fa-f]+|-?\d+)\)", args[0])
            if not v:
                raise Fail(f"set1_epi32 {args[0]}")
            return f"set1({int(v[1], 0) & 0xFFFFFFFF:#x})"
        if name == "simde_mm_set1_ps":
            return f"set1_ps({float(args[0].rstrip('f'))!r}_f32)"
        imm = {"simde_mm_shuffle_epi32": ("shuffle_epi32", 1), "simde_mm_permute_ps": ("permute_ps", 1),
               "simde_mm_blend_ps": ("blend_ps", 2), "simde_mm_dp_ps": ("dp_ps", 2), "simde_mm_alignr_epi8": ("alignr_epi8", 2)}
        if name in imm:
            fname, k = imm[name]
            vs = [self.expr(a) for a in args[:k]]
            return f"{fname}({', '.join(vs)}, {int(args[k], 0)})"
        binary = {"simde_mm_add_ps": "add_ps", "simde_mm_sub_ps": "sub_ps", "simde_mm_mul_ps": "mul_ps",
                  "simde_mm_div_ps": "div_ps", "simde_mm_min_ps": "min_ps", "simde_mm_max_ps": "max_ps",
                  "simde_mm_xor_si128": "xor", "simde_mm_xor_ps": "xor", "simde_mm_and_si128": "and",
                  "simde_mm_and_ps": "and", "simde_mm_or_si128": "or", "simde_mm_or_ps": "or",
                  "simde_mm_andnot_si128": "andnot", "simde_mm_andnot_ps": "andnot",
                  "simde_mm_unpackhi_epi32": "unpackhi_epi32", "simde_mm_unpacklo_epi32": "unpacklo_epi32",
                  "simde_mm_cmpeq_ps": "cmpeq_ps", "simde_mm_cmpge_ps": "cmpge_ps", "simde_mm_cmpgt_ps": "cmpgt_ps"}
        if name in binary:
            return f"{binary[name]}({self.expr(args[0])}, {self.expr(args[1])})"
        if name == "simde_mm_sqrt_ps":
            return f"sqrt_ps({self.expr(args[0])})"
        raise Fail(f"intrinsic {name}")

    def assign(self, name, value):
        self.lines.append(f"    {name} = {value};")
        self.written.add(name)

    def run(self):
        body = fn.body(fn.index(), self.addr)
        stmts = [l.strip() for l in body[1:-1] if l.strip() and not l.strip().startswith("//")]
        i = 0
        while i < len(stmts):
            s = stmts[i]
            i += 1
            if re.fullmatch(r"(PPC_FUNC_PROLOGUE\(\);|PPCV?Register \w+\{\};|PPCCRRegister \w+\{\};|uint32_t ea\{\};|ctx\.fpscr\.\w+\(\);|return;)", s):
                continue
            m = re.fullmatch(r"(?:ctx\.)?(r\d+)\.s64 = (-?\d+);", s)
            if m:
                self.consts[m[1]] = int(m[2])
                continue
            m = re.fullmatch(r"simde_mm_store_(?:si128|ps)\((?:\(simde__m128i\*\))?" + REG + r", (.*)\);", s, re.S)
            if m:
                self.assign(m[1], self.expr(m[2]))
                continue
            m = re.fullmatch(r"simde_mm_store_si128\(\(simde__m128i\*\)\(base \+ \(\((.+?)\) & ~0xF\)\), simde_mm_shuffle_epi8\(simde_mm_load_si128\(\(simde__m128i\*\)" + REG + r"\), simde_mm_load_si128\(\(simde__m128i\*\)VectorMaskL\)\)\);", s)
            if m:
                key = self.address(m[1])
                self.outputs.append(key)
                self.lines.append(f"    let out_{len(self.outputs) - 1} = to_guest({self.var(m[2])});")
                continue
            m = re.fullmatch(r"(?:ctx\.)?(v\d+)\.u32\[0\] = (?:ctx\.)?(v\d+)\.u32\[0\] << \((?:ctx\.)?(v\d+)\.u8\[0\] & 0x1F\);", s)
            if m:
                for k in range(1, 4):
                    want = f"{m[1]}.u32[{k}] = {m[2]}.u32[{k}] << ({m[3]}.u8[{4 * k}] & 0x1F);"
                    if stmts[i].replace("ctx.", "") != want:
                        raise Fail(f"vslw lane {k}: {stmts[i]}")
                    i += 1
                self.assign(m[1], f"slw({self.var(m[2])}, {self.var(m[3])})")
                continue
            m = re.fullmatch(r"temp\.f32 = (-?[\d.]+)f;", s)
            if m:
                self.lines.append(f"    temp = {float(m[1])!r}_f32.to_bits();")
                continue
            m = re.fullmatch(r"temp\.s32 \+= (?:ctx\.)?(v\d+)\.s16\[(\d)\];", s)
            if m:
                k = int(m[2])
                self.lines.append(f"    temp = (temp as i32).wrapping_add(({self.var(m[1])}[{k // 2}] >> {16 * (k % 2)}) as u16 as i16 as i32) as u32;")
                continue
            m = re.fullmatch(r"vTemp\.f32\[(\d)\] = (temp\.f32|-?[\d.]+f);", s)
            if m:
                val = "temp" if m[2] == "temp.f32" else f"{float(m[2].rstrip('f'))!r}_f32.to_bits()"
                self.lines.append(f"    vtemp[{m[1]}] = {val};")
                continue
            m = re.fullmatch(r"(?:ctx\.)?(v\d+) = vTemp;", s)
            if m:
                self.assign(m[1], "vtemp")
                continue
            raise Fail(f"statement {s[:100]!r}")
        if not self.outputs:
            raise Fail("no vector stores")
        return self.render()

    def render(self):
        a = self.addr
        args = ", ".join(f"{r}_{o}: [f32; 4]" for r, o in self.inputs)
        ret = "[f32; 4]" if len(self.outputs) == 1 else f"[[f32; 4]; {len(self.outputs)}]"
        regs = sorted(self.written - {"vtemp"}, key=lambda n: int(n[1:]))
        out = [f"/// sub_{a}, translated from XenonRecomp's C++ by re/xenon/vmx2rust.py.",
               f"/// Inputs: {', '.join(f'{r}+{o}' for r, o in self.inputs)}; outputs: {', '.join(f'{r}+{o}' for r, o in self.outputs)}.",
               "#[allow(unused_mut, unused_assignments, clippy::all)]",
               f"pub fn sub_{a.lower()}({args}) -> {ret} {{"]
        if regs:
            out.append(f"    let ({', '.join('mut ' + r for r in regs)}): ({', '.join('V' for _ in regs)});")
        if any("temp" in l for l in self.lines):
            out.append("    let mut temp: u32;")
            out.append("    let mut vtemp: V = [0; 4];")
        out += self.lines
        rets = [f"out_{k}" for k in range(len(self.outputs))]
        out.append(f"    {rets[0] if len(rets) == 1 else '[' + ', '.join(rets) + ']'}")
        out.append("}")
        return "\n".join(out)


def translate(addr):
    return Translator(addr.upper().removeprefix("SUB_").removeprefix("0X")).run()


if __name__ == "__main__":
    try:
        print(translate(sys.argv[1]))
    except Fail as e:
        sys.exit(f"cannot translate: {e}")

"""Lift XenonRecomp's C++ (re/xenon/ppc) to Rust, function for function.

The C++ is a small, regular subset: register-union fields, PPC_LOAD/STORE macros,
simde intrinsics, CR/XER updates, labels and gotos, calls. This parses each statement
into a typed expression tree, applies C's conversions (integer promotion, the usual
arithmetic conversions, LLP64 literal types: XenonRecomp is built with clang-cl) and
prints Rust with the same meaning, on crates/xenon-rt (which mirrors ppc_context.h).
simde on x86-64 maps 1:1 to SSE intrinsics, so vector code is printed as the same
core::arch intrinsics. Anything outside the subset stops the lift with an error;
nothing is guessed.

Labels become a `loop { match pc { .. } }` state machine; gotos set `pc`.
Mid-asm hooks are UnleashedRecomp's PC fixes: as in the oracle they are no-ops (bool
hooks are false), so the lift reproduces the console game.

usage: python ppc2rust.py <root addr>... --out <dir>     lift the call closure
       python ppc2rust.py --show <addr>                  print one lifted function
"""
import argparse
import csv
import json
import re
import sys
from pathlib import Path

import fn

HERE = Path(__file__).resolve().parent
RS = HERE.parents[1]

# Keep the retail call boundaries of the imported-course Havok routines. Inlining
# the new narrow-phase routines into the already large lifted call graph causes
# excessive LLVM optimization time. Instructions and PPC calling semantics remain
# exactly those emitted by Function.lift.
COURSE_COLLISION_BOUNDARIES = set("""
82633340 82F28EF0 82F28F70 82F28F88 82F28F98 82F29050 82F29138 82F29338
82F46668 82F46B58 82F470A0 82F477A0 82F49AD8 82F49B50 82F49C38 82F4A018
82F4A6A8 82F913C0
""".split())


class Fail(Exception):
    pass


# ---------------------------------------------------------------- tokens

TOKEN = re.compile(r"""
    (?P<ws>\s+)
  | (?P<num>(?:0[xX][0-9A-Fa-f]+|\d+\.\d*(?:[eE][-+]?\d+)?|\d+[eE][-+]?\d+|\.\d+(?:[eE][-+]?\d+)?|\d+)[uUlLfF]*)
  | (?P<id>[A-Za-z_][A-Za-z0-9_]*)
  | (?P<op><<=|>>=|<<|>>|<=|>=|==|!=|&&|\|\||\+\+|--|\+=|-=|\*=|/=|%=|&=|\|=|\^=|->|::|[-+*/%&|^~!<>=?:.,;()\[\]{}])
""", re.X)


def tokenize(s):
    out, i = [], 0
    while i < len(s):
        m = TOKEN.match(s, i)
        if not m:
            raise Fail(f"cannot tokenize at {s[i:i + 30]!r}")
        i = m.end()
        if m.lastgroup != "ws":
            out.append((m.lastgroup, m.group()))
    return out


# ---------------------------------------------------------------- types

INT_TYPES = {"i8": (8, True), "u8": (8, False), "i16": (16, True), "u16": (16, False),
             "i32": (32, True), "u32": (32, False), "i64": (64, True), "u64": (64, False), "bool": (1, False)}
FLOAT = ("f32", "f64")
CTYPE = {"int8_t": "i8", "uint8_t": "u8", "int16_t": "i16", "uint16_t": "u16", "int32_t": "i32",
         "uint32_t": "u32", "int64_t": "i64", "uint64_t": "u64", "int": "i32", "unsigned": "u32",
         "short": "i16", "char": "i8", "float": "f32", "double": "f64", "size_t": "u64", "bool": "bool"}
FIELD = {"s8": "i8", "u8": "u8", "s16": "i16", "u16": "u16", "s32": "i32", "u32": "u32",
         "s64": "i64", "u64": "u64", "f32": "f32", "f64": "f64"}
VEC = ("m128", "m128i", "m128d")
RUST_VEC = {"m128": "__m128", "m128i": "__m128i", "m128d": "__m128d"}


def is_int(t):
    return t in INT_TYPES


def promote(t):
    if t in ("i8", "u8", "i16", "u16", "bool"):
        return "i32"
    return t


def common(a, b):
    if a in FLOAT or b in FLOAT:
        return "f64" if "f64" in (a, b) else "f32"
    a, b = promote(a), promote(b)
    if a == b:
        return a
    (wa, sa), (wb, sb) = INT_TYPES[a], INT_TYPES[b]
    if sa == sb:
        return a if wa >= wb else b
    s, u = (a, b) if sa else (b, a)
    ws, wu = INT_TYPES[s][0], INT_TYPES[u][0]
    if wu >= ws:
        return u
    return s  # signed is wider: it represents every value of the unsigned


class E:
    """Typed Rust expression. `lit` keeps an integer literal's value for folding."""

    def __init__(self, code, t, lit=None, place=False):
        self.code, self.t, self.lit, self.place = code, t, lit, place


def lit_int(v, t):
    return E(f"({v}{t})" if v < 0 else f"{v}{t}", t, lit=v)


def conv(e, t):
    """C implicit / explicit conversion of `e` to type `t`."""
    if e.t == t:
        return e.code
    if e.lit is not None and is_int(t) and t != "bool":
        bits, signed = INT_TYPES[t]
        v = e.lit & ((1 << bits) - 1)
        if signed and v >= 1 << (bits - 1):
            v -= 1 << bits
        return f"({v}{t})" if v < 0 else f"{v}{t}"
    if e.lit is not None and t in FLOAT:
        return f"{float(e.lit)!r}{t}"
    if t == "bool":
        return f"({e.code} != 0{'.0' if e.t in FLOAT else ''})" if e.t != "bool" else e.code
    if is_int(e.t) and is_int(t):
        return f"({e.code} as {t})"
    if is_int(e.t) and t in FLOAT:
        return f"({e.code} as {t})"
    if e.t in FLOAT and t in FLOAT:
        return f"({e.code} as {t})"
    if e.t in FLOAT and is_int(t):
        # clang on x86-64: cvttsd2si (32-bit for int and narrower signed, else 64-bit).
        x = e.code if e.t == "f64" else f"({e.code} as f64)"
        if t in ("i32", "i16", "i8"):
            return f"(cvttsd_si32({x}) as {t})"
        return f"(cvttsd_si64({x}) as {t})"
    raise Fail(f"no conversion {e.t} -> {t} for {e.code[:60]}")


def cond(e):
    return conv(e, "bool")


# ---------------------------------------------------------------- simde

# name: (return type, parameter types); "imm" = immediate, printed as a literal.
SIMDE = {
    "load_si128": ("m128i", ["p_m128i"]), "load_ps": ("m128", ["p_f32"]), "load_sd": ("m128d", ["p_f64"]),
    "loadu_si128": ("m128i", ["p_m128i"]), "load_ss": ("m128", ["p_f32"]),
    "store_si128": (None, ["p_m128i", "m128i"]), "store_ps": (None, ["p_f32", "m128"]),
    "set1_epi32": ("m128i", ["i32"]), "set1_epi8": ("m128i", ["i8"]), "set1_epi16": ("m128i", ["i16"]),
    "set1_ps": ("m128", ["f32"]), "setzero_si128": ("m128i", []), "setzero_ps": ("m128", []),
    "castsi128_ps": ("m128", ["m128i"]), "castps_si128": ("m128i", ["m128"]),
    "shuffle_epi8": ("m128i", ["m128i", "m128i"]), "shuffle_epi32": ("m128i", ["m128i", "imm"]),
    "shuffle_ps": ("m128", ["m128", "m128", "imm"]),
    "permute_ps": ("m128", ["m128", "imm"]), "blend_ps": ("m128", ["m128", "m128", "imm"]),
    "blendv_ps": ("m128", ["m128", "m128", "m128"]), "blendv_epi8": ("m128i", ["m128i", "m128i", "m128i"]),
    "dp_ps": ("m128", ["m128", "m128", "imm"]), "alignr_epi8": ("m128i", ["m128i", "m128i", "imm"]),
    "round_ps": ("m128", ["m128", "imm"]),
    "add_ps": ("m128", ["m128", "m128"]), "sub_ps": ("m128", ["m128", "m128"]),
    "mul_ps": ("m128", ["m128", "m128"]), "div_ps": ("m128", ["m128", "m128"]),
    "min_ps": ("m128", ["m128", "m128"]), "max_ps": ("m128", ["m128", "m128"]),
    "sqrt_ps": ("m128", ["m128"]), "rcp_ps": ("m128", ["m128"]), "rsqrt_ps": ("m128", ["m128"]),
    "cmpeq_ps": ("m128", ["m128", "m128"]), "cmpge_ps": ("m128", ["m128", "m128"]),
    "cmpgt_ps": ("m128", ["m128", "m128"]), "cmple_ps": ("m128", ["m128", "m128"]),
    "cmplt_ps": ("m128", ["m128", "m128"]), "cmpunord_ps": ("m128", ["m128", "m128"]),
    "xor_ps": ("m128", ["m128", "m128"]), "and_ps": ("m128", ["m128", "m128"]),
    "or_ps": ("m128", ["m128", "m128"]), "andnot_ps": ("m128", ["m128", "m128"]),
    "xor_si128": ("m128i", ["m128i", "m128i"]), "and_si128": ("m128i", ["m128i", "m128i"]),
    "or_si128": ("m128i", ["m128i", "m128i"]), "andnot_si128": ("m128i", ["m128i", "m128i"]),
    "unpackhi_epi32": ("m128i", ["m128i", "m128i"]), "unpacklo_epi32": ("m128i", ["m128i", "m128i"]),
    "unpackhi_epi16": ("m128i", ["m128i", "m128i"]), "unpacklo_epi16": ("m128i", ["m128i", "m128i"]),
    "unpackhi_epi8": ("m128i", ["m128i", "m128i"]), "unpacklo_epi8": ("m128i", ["m128i", "m128i"]),
    "cmpeq_epi32": ("m128i", ["m128i", "m128i"]), "cmpgt_epi32": ("m128i", ["m128i", "m128i"]),
    "cmpeq_epi8": ("m128i", ["m128i", "m128i"]), "cmpeq_epi16": ("m128i", ["m128i", "m128i"]),
    "add_epi32": ("m128i", ["m128i", "m128i"]), "sub_epi32": ("m128i", ["m128i", "m128i"]),
    "add_epi16": ("m128i", ["m128i", "m128i"]), "sub_epi16": ("m128i", ["m128i", "m128i"]),
    "add_epi8": ("m128i", ["m128i", "m128i"]), "sub_epi8": ("m128i", ["m128i", "m128i"]),
    "max_epi32": ("m128i", ["m128i", "m128i"]), "min_epi32": ("m128i", ["m128i", "m128i"]),
    "max_epu32": ("m128i", ["m128i", "m128i"]), "min_epu32": ("m128i", ["m128i", "m128i"]),
    "cvtepi32_ps": ("m128", ["m128i"]), "cvtps_epi32": ("m128i", ["m128"]), "cvttps_epi32": ("m128i", ["m128"]),
    "packs_epi32": ("m128i", ["m128i", "m128i"]), "packus_epi32": ("m128i", ["m128i", "m128i"]),
    "packs_epi16": ("m128i", ["m128i", "m128i"]), "packus_epi16": ("m128i", ["m128i", "m128i"]),
    "slli_epi32": ("m128i", ["m128i", "imm"]), "srli_epi32": ("m128i", ["m128i", "imm"]),
    "srai_epi32": ("m128i", ["m128i", "imm"]), "slli_si128": ("m128i", ["m128i", "imm"]),
    "srli_si128": ("m128i", ["m128i", "imm"]), "movemask_ps": ("i32", ["m128"]), "movemask_epi8": ("i32", ["m128i"]),
    "insert_ps": ("m128", ["m128", "m128", "imm"]), "vctsxs": ("m128i", ["m128"]), "perm_epi8_": ("m128i", ["m128i", "m128i", "m128i"]), "cvtepu32_ps_": ("m128", ["m128i"]),
    "cmpgt_epu8": ("m128i", ["m128i", "m128i"]), "cmpgt_epu16": ("m128i", ["m128i", "m128i"]),
    "adds_epu32": ("m128i", ["m128i", "m128i"]), "avg_epi8": ("m128i", ["m128i", "m128i"]),
    "avg_epi16": ("m128i", ["m128i", "m128i"]), "vsr": ("m128i", ["m128i", "m128i"]),
    "cmpgt_epi8": ("m128i", ["m128i", "m128i"]), "cmpgt_epi16": ("m128i", ["m128i", "m128i"]),
    "avg_epu8": ("m128i", ["m128i", "m128i"]), "avg_epu16": ("m128i", ["m128i", "m128i"]), "perm_epi8_": ("m128i", ["m128i", "m128i", "m128i"]), "extract_ps": ("i32", ["m128", "imm"]),
    "cvttsd_si32": ("i32", ["m128d"]), "cvttsd_si64": ("i64", ["m128d"]),
    "cvtsd_si32": ("i32", ["m128d"]), "cvtsd_si64": ("i64", ["m128d"]),
}
# ppc_context.h's own helpers and AVX-only names: implemented in xenon-rt.
SIMDE_RT = {"permute_ps", "vctsxs", "perm_epi8_", "cvtepu32_ps_", "vsr", "adds_epu32", "avg_epi8",
            "avg_epi16", "cmpgt_epu8", "cmpgt_epu16"}
SIMDE_CONST = {"SIMDE_MM_FROUND_TO_NEAREST_INT": 0, "SIMDE_MM_FROUND_TO_NEG_INF": 1, "SIMDE_MM_FROUND_TO_POS_INF": 2,
               "SIMDE_MM_FROUND_TO_ZERO": 3, "SIMDE_MM_FROUND_NO_EXC": 8}
# Mid-asm hooks xenon-rt implements (run only with pc_fixes(); off = console).
VREG_RX = r"(ctx\.)?v\d+"
RT_HOOKS = {"DownForceDeltaTimeFixMidAsmHook_r", "DownForceDeltaTimeFixMidAsmHook_vr",
            "CameraDeltaTimeFixMidAsmHook_rr", "CameraDeltaTimeFixMidAsmHook_r", "CameraLerpFixMidAsmHook_r",
            "CameraTargetSideOffsetLerpFixMidAsmHook_vv", "Camera2DLerpFixMidAsmHook_rr",
            "Camera2DSlopeLerpFixMidAsmHook_rr", "ResetScoreOnRestartMidAsmHook_",
            "PostureDPadSupportInvertYMidAsmHook_rrr", "PostureDPadSupportMidAsmHook_rrr", "PostureDPadSupportXMidAsmHook_rr", "PostureDPadSupportYMidAsmHook_rr", "IsHomingAttackOnJump_", "CameraAspectRatioMidAsmHook_rr", "CameraFieldOfViewMidAsmHook_rr"}
NAMED_INT = {"INT_MAX": (0x7FFFFFFF, "i32"), "INT_MIN": (-0x80000000, "i32"), "UINT_MAX": (0xFFFFFFFF, "u32"),
             "LLONG_MAX": (0x7FFFFFFFFFFFFFFF, "i64"), "LLONG_MIN": (-0x8000000000000000, "i64")}


# ---------------------------------------------------------------- expressions

PREC = [("||",), ("&&",), ("|",), ("^",), ("&",), ("==", "!="), ("<", "<=", ">", ">="), ("<<", ">>"),
        ("+", "-"), ("*", "/", "%")]


class Parser:
    def __init__(self, fx, toks):
        self.fx, self.toks, self.i = fx, toks, 0

    def peek(self, k=0):
        return self.toks[self.i + k][1] if self.i + k < len(self.toks) else None

    def kind(self, k=0):
        return self.toks[self.i + k][0] if self.i + k < len(self.toks) else None

    def take(self, want=None):
        if self.i >= len(self.toks):
            raise Fail(f"unexpected end, wanted {want}")
        t = self.toks[self.i][1]
        if want is not None and t != want:
            raise Fail(f"expected {want!r}, got {t!r}")
        self.i += 1
        return t

    def done(self):
        return self.i >= len(self.toks)

    # ternary
    def expr(self):
        c = self.binary(0)
        if self.peek() == "?":
            self.take("?")
            a = self.expr()
            self.take(":")
            b = self.expr()
            if a.t in VEC or b.t in VEC:
                if a.t != b.t:
                    raise Fail(f"vector ternary {a.t} / {b.t}")
                return E(f"(if {cond(c)} {{ {a.code} }} else {{ {b.code} }})", a.t)
            t = a.t if a.t == b.t else common(a.t, b.t)
            return E(f"(if {cond(c)} {{ {conv(a, t)} }} else {{ {conv(b, t)} }})", t)
        return c

    def binary(self, level):
        if level == len(PREC):
            return self.unary()
        left = self.binary(level + 1)
        while self.peek() in PREC[level]:
            op = self.take()
            right = self.binary(level + 1)
            left = self.apply(op, left, right)
        return left

    def apply(self, op, a, b):
        if op in ("&&", "||"):
            return E(f"({cond(a)} {op} {cond(b)})", "bool")
        if a.t in VEC or b.t in VEC:
            raise Fail(f"vector operand of {op}")
        if a.t.startswith("p_") and op == "+" and is_int(b.t):
            return E(f"{a.code}.wrapping_add({conv(b, 'i64' if INT_TYPES[b.t][1] else 'u64')} as usize)", a.t)
        if not (is_int(a.t) or a.t in FLOAT) or not (is_int(b.t) or b.t in FLOAT):
            raise Fail(f"operands {a.t} {op} {b.t}")
        if op in ("<<", ">>"):
            t = promote(a.t)
            if a.t in FLOAT:
                raise Fail("float shift")
            if a.lit is not None and b.lit is not None:
                v = a.lit << b.lit if op == "<<" else a.lit >> b.lit
                return lit_int(wrap(v, t), t)
            sh = "wrapping_shl" if op == "<<" else "wrapping_shr"
            return E(f"{conv(a, t)}.{sh}({conv(b, 'u32')})", t)
        t = common(a.t, b.t)
        if op in ("==", "!=", "<", "<=", ">", ">="):
            return E(f"({conv(a, t)} {op} {conv(b, t)})", "bool")
        if a.lit is not None and b.lit is not None and is_int(t) and op in "+-*&|^":
            v = {"+": a.lit + b.lit, "-": a.lit - b.lit, "*": a.lit * b.lit, "&": a.lit & b.lit,
                 "|": a.lit | b.lit, "^": a.lit ^ b.lit}[op]
            return lit_int(wrap(v, t), t)
        x, y = conv(a, t), conv(b, t)
        if t in FLOAT:
            if op == "%":
                raise Fail("float %")
            return E(f"({x} {op} {y})", t)
        if op in "&|^":
            return E(f"({x} {op} {y})", t)
        fnm = {"+": "wrapping_add", "-": "wrapping_sub", "*": "wrapping_mul", "/": "cdiv", "%": "crem"}[op]
        return E(f"{x}.{fnm}({y})", t)

    def unary(self):
        p = self.peek()
        if p == "-":
            self.take()
            e = self.unary()
            if e.lit is not None:
                return lit_int(-e.lit, e.t)
            if e.t in FLOAT:
                return E(f"(-{e.code})", e.t)
            t = promote(e.t)
            return E(f"{conv(e, t)}.wrapping_neg()", t)
        if p == "!":
            self.take()
            return E(f"(!{cond(self.unary())})", "bool")
        if p == "~":
            self.take()
            e = self.unary()
            t = promote(e.t)
            if e.lit is not None:
                return lit_int(wrap(~e.lit, t), t)
            return E(f"(!{conv(e, t)})", t)
        if p == "&":
            self.take()
            e = self.unary()
            if not e.place:
                raise Fail("address of a non-place")
            return E(f"((&raw const {e.code}) as *mut {e.t})", "p_" + e.t)
        if p == "*":
            self.take()
            e = self.unary()
            if not e.t.startswith("p_") or e.t[2:] not in INT_TYPES:
                raise Fail(f"deref of {e.t}")
            return E(f"({e.code} as *const {e.t[2:]}).read_unaligned()", e.t[2:])
        if p == "(" and self.is_cast():
            self.take("(")
            t = self.ctype()
            self.take(")")
            e = self.unary()
            return self.cast(e, t)
        return self.postfix(self.primary())

    def is_cast(self):
        return self.kind(1) == "id" and (self.peek(1) in CTYPE or self.peek(1).startswith("simde__")) and \
            (self.peek(2) == ")" or self.peek(2) == "*")

    def ctype(self):
        name = self.take()
        if name.startswith("simde__"):
            t = {"simde__m128i": "m128i", "simde__m128": "m128", "simde__m128d": "m128d"}[name]
        else:
            t = CTYPE[name]
        if self.peek() == "*":
            self.take()
            return "p_" + t
        return t

    def cast(self, e, t):
        if t.startswith("p_"):
            rust = RUST_VEC.get(t[2:], t[2:])
            if e.t.startswith("a_"):  # a register's lane array decays to a pointer to it
                return E(f"((&raw mut {e.code.rsplit('.', 1)[0]}) as *mut {rust})", t)
            if not e.t.startswith("p_"):
                raise Fail(f"pointer cast of {e.t}")
            return E(f"({e.code} as *mut {rust})", t)
        return E(conv(e, t), t)

    def primary(self):
        k, v = self.kind(), self.peek()
        if k == "num":
            self.take()
            return number(v)
        if v == "(":
            self.take()
            e = self.expr()
            self.take(")")
            return E(f"({e.code})" if not e.place else e.code, e.t, e.lit, e.place)
        if k == "id":
            self.take()
            return self.ident(v)
        raise Fail(f"unexpected {v!r}")

    def ident(self, name):
        fx = self.fx
        if self.peek() == "(":
            return self.call(name)
        if self.peek() == "<" and name == "reinterpret_cast":
            self.take("<")
            t = self.ctype()
            self.take(">")
            self.take("(")
            e = self.expr()
            self.take(")")
            return self.cast(e, t)
        if name in NAMED_INT:
            return lit_int(*NAMED_INT[name])
        if name in SIMDE_CONST:
            return lit_int(SIMDE_CONST[name], "i32")
        if name == "base":
            return E("base", "p_u8")
        if name == "ea":
            return E("ea", "u32", place=True)
        if name in ("VectorMaskL", "VectorMaskR", "VectorShiftTableL", "VectorShiftTableR"):
            if self.peek() != "[":
                return E(f"({name}.0.as_ptr() as *mut u8)", "p_u8")
            self.take("[")
            i = self.expr()
            self.take("]")
            return E(f"{name}.0[{conv(i, 'u64')} as usize]", "u8", place=True)
        if name == "ctx":
            self.take(".")
            reg = self.take()
            if reg == "fpscr":
                return E("ctx.fpscr", "fpscr")
            return self.register(f"ctx.{reg}", reg)
        if name in fx.locals:
            return self.register(name, name)
        raise Fail(f"unknown identifier {name}")

    def register(self, code, reg):
        kind = self.fx.reg_kind(reg)
        if kind in ("cr", "xer"):
            return E(code, kind)
        if kind == "vreg":
            if self.peek() != ".":
                return E(code, "vreg", place=True)
            self.take(".")
            f = self.take()
            if self.peek() == "[":
                self.take("[")
                i = self.expr()
                self.take("]")
                return E(f"{code}.{f}[{conv(i, 'u64')} as usize]", FIELD[f], place=True)
            return E(f"{code}.{f}", "a_" + FIELD[f], place=True)
        if self.peek() != ".":
            raise Fail(f"bare register {reg}")
        self.take(".")
        f = self.take()
        return E(f"{code}.{f}", FIELD[f], place=True)

    def postfix(self, e):
        while self.peek() == ".":
            if e.t == "cr":
                self.take(".")
                f = self.take()
                if self.peek() == "(":
                    return self.cr_method(e, f)
                f = "so" if f == "un" else f
                e = E(f"{e.code}.{f}", "u8", place=True)
            elif e.t == "xer":
                self.take(".")
                e = E(f"{e.code}.{self.take()}", "u8", place=True)
            elif e.t == "fpscr":
                self.take(".")
                m = self.take()
                self.take("(")
                self.take(")")
                return E(f"{e.code}.{m}()", "u32" if m == "loadFromHost" else "unit")
            else:
                raise Fail(f"member of {e.t}")
        if self.peek() == "<" and e.t == "cr":
            raise Fail("cr template")
        return e

    def cr_method(self, cr, m):
        self.take("(")
        args = self.args()
        if m == "setFromMask":
            v, imm = args
            which = {"m128": "set_from_mask_ps", "m128i": "set_from_mask_si"}[v.t]
            return E(f"{cr.code}.{which}({v.code}, {conv(imm, 'i32')})", "unit")
        if m == "compare":
            a, b = args[:2]
            return E(f"{cr.code}.compare_f({conv(a, 'f64')}, {conv(b, 'f64')})", "unit")
        raise Fail(f"cr method {m}")

    def args(self):
        out = []
        if self.peek() == ")":
            self.take(")")
            return out
        while True:
            out.append(self.expr())
            if self.take() == ")":
                return out

    def call(self, name):
        self.take("(")
        if name in CTYPE:  # functional cast: double(x), int(x), ...
            e = self.expr()
            self.take(")")
            return self.cast(e, CTYPE[name])
        args = self.args()
        m = re.fullmatch(r"PPC_LOAD_U(8|16|32|64)", name)
        if m:
            return E(f"PPC_LOAD_U{m[1]}(base, {conv(args[0], 'u32')})", f"u{m[1]}")
        # PPC_MM_STORE_* (stores that may hit memory-mapped registers) is #defined as PPC_STORE_*.
        m = re.fullmatch(r"PPC_(?:MM_)?STORE_U(8|16|32|64)", name)
        if m:
            return E(f"PPC_STORE_U{m[1]}(base, {conv(args[0], 'u32')}, {conv(args[1], 'u' + m[1])})", "unit")
        if name == "sqrt":
            a = args[0]
            t = "f32" if a.t == "f32" else "f64"
            return E(f"{'c_sqrtf' if t == 'f32' else 'c_sqrt'}({conv(a, t)})", t)
        builtins = {"__builtin_rotateleft32": ("u32", ["u32", "u32"]), "__builtin_rotateleft64": ("u64", ["u64", "u64"]),
                    "__builtin_clz": ("i32", ["u32"]), "__builtin_clzll": ("i32", ["u64"]),
                    "__builtin_bswap16": ("u16", ["u16"]), "__builtin_bswap32": ("u32", ["u32"]),
                    "__builtin_bswap64": ("u64", ["u64"]), "__rdtsc": ("u64", []),
                    "__sync_bool_compare_and_swap": ("bool", ["p_u32", "i32", "i32"])}
        if name in builtins:
            ret, params = builtins[name]
            cargs = [a.code + " as *mut u8" if p.startswith("p_") else conv(a, p) for a, p in zip(args, params)]
            return E(f"{name}({', '.join(cargs)})", ret)
        if name.startswith("simde_mm_"):
            return self.simde(name[9:], args)
        raise Fail(f"call {name}")

    def simde(self, op, args):
        if op in ("cvttsd_si32", "cvttsd_si64", "cvtsd_si32", "cvtsd_si64") and args and args[0].code.startswith("_mm_load_sd("):
            inner = args[0].code[len("_mm_load_sd("):-1]
            return E(f"{op}(*{inner})", "i32" if op.endswith("32") else "i64")
        if op not in SIMDE:
            raise Fail(f"simde_mm_{op}")
        ret, params = SIMDE[op]
        if len(args) != len(params):
            raise Fail(f"simde_mm_{op}: {len(args)} args")
        cargs = []
        for a, p in zip(args, params):
            if p == "imm":
                if a.lit is None:
                    raise Fail(f"simde_mm_{op}: non-constant immediate")
                cargs.append(str(a.lit))
            elif p.startswith("p_"):
                cargs.append(self.pointer(a, p))
            elif p in VEC:
                if a.t != p:
                    raise Fail(f"simde_mm_{op}: {a.t} where {p}")
                cargs.append(a.code)
            else:
                cargs.append(conv(a, p))
        name = f"simde_mm_{op}" if op in SIMDE_RT else f"_mm_{op}"
        if op in SIMDE_RT and any(p == "imm" for p in params):
            imms = [c for c, p in zip(cargs, params) if p == "imm"]
            rest = [c for c, p in zip(cargs, params) if p != "imm"]
            return E(f"{name}::<{', '.join(imms)}>({', '.join(rest)})", ret or "unit")
        return E(f"{name}({', '.join(cargs)})", ret or "unit")

    def pointer(self, a, p):
        """Pointer argument of a simde load/store."""
        if a.t.startswith("a_") or a.t == "vreg":
            # A register's lane array (or the register): the union itself.
            reg = a.code.rsplit(".", 1)[0] if a.t.startswith("a_") else a.code
            return f"(&raw mut {reg})" + (" as *mut f32" if p == "p_f32" else " as *mut __m128i" if p == "p_m128i" else " as *mut f64")
        if a.t.startswith("p_"):
            rust = {"p_m128i": "__m128i", "p_f32": "f32", "p_f64": "f64"}[p]
            return f"({a.code} as *mut {rust})"
        raise Fail(f"pointer from {a.t}")


def wrap(v, t):
    bits, signed = INT_TYPES[t]
    v &= (1 << bits) - 1
    if signed and v >= 1 << (bits - 1):
        v -= 1 << bits
    return v


def number(s):
    low = s.lower()
    if re.fullmatch(r"[0-9.]+(e[-+]?\d+)?f?|\.\d+.*", low) and ("." in low or "e" in low and not low.startswith("0x")):
        if low.endswith("f"):
            return E(f"{low[:-1]}f32" if "." in low[:-1] or "e" in low else f"{low[:-1]}.0f32", "f32")
        return E(f"{low}f64", "f64")
    m = re.fullmatch(r"(0x[0-9a-f]+|\d+)(u?l{0,2}|l{0,2}u?)", low)
    if not m:
        raise Fail(f"number {s}")
    v, suf = int(m[1], 0), m[2]
    hexa = m[1].startswith("0x")
    if "ll" in suf:
        t = "u64" if "u" in suf else ("i64" if v < 1 << 63 else "u64")
    elif "u" in suf:
        t = "u32" if v < 1 << 32 else "u64"
    elif v < 1 << 31:
        t = "i32"
    elif hexa and v < 1 << 32:
        t = "u32"
    elif v < 1 << 63:
        t = "i64"
    elif hexa:
        t = "u64"
    else:
        raise Fail(f"literal {s} too large")
    return lit_int(v, t)


# ---------------------------------------------------------------- functions

DECL = re.compile(r"(PPCRegister|PPCVRegister|PPCCRRegister|PPCXERRegister|uint32_t) (\w+)\{\};")


class Function:
    def __init__(self, lifter, addr):
        self.lifter, self.addr = lifter, addr
        self.locals = {}  # name -> kind
        self.out = []

    def reg_kind(self, reg):
        if reg in self.locals:
            return self.locals[reg]
        if re.fullmatch(r"r\d+|f\d+", reg):
            return "reg"
        if re.fullmatch(r"v\d+", reg):
            return "vreg"
        raise Fail(f"register {reg}")

    def parse(self, text):
        p = Parser(self, tokenize(text))
        e = p.expr()
        if not p.done():
            raise Fail(f"trailing tokens in {text[:80]!r}")
        return e

    def stmt(self, s):
        """One C++ statement -> Rust statement(s) (control flow handled by the caller)."""
        if s.endswith(";"):
            s = s[:-1]
        m = re.fullmatch(r"(cr\d)\.compare<(\w+)>\((.*), xer\)", s)
        if m:
            t = CTYPE[m[2]]
            p = Parser(self, tokenize(m[3]))
            a = p.expr()
            p.take(",")
            b = p.expr()
            return f"{m[1]}.compare::<{t}>({conv(a, t)}, {conv(b, t)}, &xer);"
        m = re.fullmatch(r"--(.+)", s)
        if m:
            e = self.parse(m[1])
            return f"{e.code} = {e.code}.wrapping_sub(1);"
        m = re.fullmatch(r"(.+?) (=|\+=|-=|\*=|\|=|&=|\^=) (.+)", s)
        if m and "(" not in m[1].split(".")[0]:
            target = self.parse(m[1])
            if not target.place:
                raise Fail(f"assignment to {m[1]}")
            value = self.parse(m[3])
            if m[2] != "=":
                p = Parser(self, [])
                value = p.apply(m[2][0], target, value)
            if target.t == "vreg":
                if value.t != "vreg":
                    raise Fail("vector register assignment")
                return f"{target.code} = {value.code};"
            return f"{target.code} = {conv(value, target.t)};"
        e = self.parse(s)
        if e.t != "unit" and not e.code.startswith(("_mm_store", "PPC_STORE")):
            raise Fail(f"expression statement {s[:60]!r}")
        return e.code + ";"

    def simde_store(self, s):
        """simde stores to a register are the common case: print them as vst."""
        m = re.fullmatch(r"simde_mm_store_(si128|ps)\((?:\(simde__m128i\*\))?((?:ctx\.)?v\d+|vTemp)\.(?:u8|u32|f32|s32|u16|s16|s8|u64|f64), (.*)\);", s, re.S)
        if not m:
            return None
        value = self.parse(m[3])
        want = "m128i" if m[1] == "si128" else "m128"
        if value.t != want:
            raise Fail(f"store {m[1]} of {value.t}")
        return f"{{ let t = {value.code}; {'vst' if m[1] == 'si128' else 'vst_ps'}(&mut {m[2]}, t); }}"

    def lift(self):
        body = fn.body(self.lifter.idx, self.addr)
        if not re.fullmatch(rf"PPC_FUNC_IMPL\(__imp__sub_{self.addr}\) \{{", body[0]):
            raise Fail(f"header {body[0]}")
        stmts = [l.strip() for l in body[1:-1] if l.strip() and not l.strip().startswith("//")]
        # Declarations first.
        decls, rest = [], []
        for s in stmts:
            m = DECL.fullmatch(s)
            if m:
                kind = {"PPCRegister": "reg", "PPCVRegister": "vreg", "PPCCRRegister": "cr",
                        "PPCXERRegister": "xer", "uint32_t": "u32"}[m[1]]
                self.locals[m[2]] = kind
                init = {"reg": "PPCRegister::default()", "vreg": "PPCVRegister::default()", "cr": "PPCCRRegister::default()",
                        "xer": "PPCXERRegister::default()", "u32": "0u32"}[kind]
                decls.append(f"let mut {m[2]} = {init};")
            elif s != "PPC_FUNC_PROLOGUE();":
                rest.append(s)
        labels = [s[:-1] for s in rest if re.fullmatch(r"loc_[0-9A-F]+:", s)]
        self.block = {l: i + 1 for i, l in enumerate(labels)}
        out = list(decls)
        if labels:
            out.append("let mut pc: u32 = 0;")
            out.append("loop {")
            out.append("match pc {")
            out.append("0 => {")
        i = 0
        while i < len(rest):
            s = rest[i]
            i += 1
            if s.endswith(":") and s[:-1] in self.block:
                k = self.block[s[:-1]]
                out.append(f"pc = {k};")
                out.append("}")
                out.append(f"{k} => {{")
                continue
            if s.startswith("switch ("):
                i = self.switch(rest, i - 1, out)
                continue
            # A bool hook in block form: `if (Hook(..)) { goto L; } else { }`.
            bm = re.fullmatch(r"if \((!?)(\w+)\(([^()]*)\)\) \{", s)
            if bm and bm[2] in self.lifter.hooks and rest[i:i + 4][1:] == ["}", "else {", "}"] and re.fullmatch(r"goto loc_[0-9A-F]+;", rest[i]):
                label = rest[i][5:-1]
                regs = [a.strip() for a in bm[3].split(",") if a.strip()]
                kinds = "".join("v" if re.fullmatch(VREG_RX, r) else "r" for r in regs)
                name = f"{bm[2]}_{kinds}"
                call = f"{name}({', '.join(f'&mut {r}' for r in regs)})" if name in RT_HOOKS else f'unimplemented_bool_hook("{bm[2]}")'
                out.append(f"if {bm[1]}(pc_fixes() && {call}) {self.goto(label)} // UnleashedRecomp PC fix")
                i += 4
                continue
            # A hook with a jump_address replaces the instructions up to its label:
            # `Hook(..); goto L; <original>; L:`. On the console (pc_fixes off) the
            # original instructions run; only the PC fix takes the jump.
            if self.lifter.hook_call(s.rstrip(";")) is not None and i < len(rest) and re.fullmatch(r"goto loc_[0-9A-F]+;", rest[i]):
                label = rest[i][5:-1]
                out.append(f"if pc_fixes() {{ {self.hook_code(s)} {self.goto(label)} }} // UnleashedRecomp PC fix (replaces the code up to {label})")
                i += 1
                continue
            out.append(self.control(s))
        if labels:
            out.append("return;")
            out.append("}")
            out.append("_ => unreachable!(),")
            out.append("}")
            out.append("}")
        return out

    def hook_code(self, s):
        """The Rust for a void mid-asm hook call (run only under pc_fixes())."""
        m = re.fullmatch(r"(\w+)\(([^()]*)\);", s)
        regs = [a.strip() for a in m[2].split(",") if a.strip()]
        kinds = "".join("v" if re.fullmatch(VREG_RX, r) else "r" for r in regs)
        name = f"{m[1]}_{kinds}"
        if name in RT_HOOKS:
            return f"{name}({', '.join(f'&mut {r}' for r in regs)});"
        return f'unimplemented_hook("{m[1]}");'

    def goto(self, label):
        if label not in self.block:
            raise Fail(f"goto unknown {label}")
        return f"{{ pc = {self.block[label]}; continue; }}"

    def control(self, s):
        if s == "return;":
            return "return;"
        m = re.fullmatch(r"goto (loc_[0-9A-F]+);", s)
        if m:
            return self.goto(m[1])
        if s == "__builtin_unreachable();":
            return "unreachable!();"
        m = re.fullmatch(r"if \((.*)\) (goto (loc_[0-9A-F]+);|return;|(.+;))", s)
        if m:
            c = m[1]
            hook = self.lifter.hook_call(c)
            if hook is not None:
                hm = re.fullmatch(r"(!?)(\w+)\(([^()]*)\)", c.strip())
                # Console: false. Under pc_fixes() a bool hook must be implemented.
                c_code = f'{hm[1]}(pc_fixes() && unimplemented_bool_hook("{hm[2]}"))'
            else:
                c_code = cond(self.parse(c))
            then = self.goto(m[3]) if m[3] else "{ return; }" if m[2] == "return;" else f"{{ {self.control(m[4])} }}"
            return f"if {c_code} {then}"
        m = re.fullmatch(r"(sub_[0-9A-F]{8})\(ctx, base\);", s)
        if m:
            return self.lifter.direct_call(m[1][4:])
        m = re.fullmatch(r"__imp__(\w+)\(ctx, base\);", s)
        if m:
            if m[1].startswith("sub_"):
                return self.lifter.direct_call(m[1][4:], imp=True)
            return f'host_call(|h| h.import("{m[1]}", ctx, base));'
        m = re.fullmatch(r"PPC_CALL_INDIRECT_FUNC\((.*)\);", s)
        if m:
            return f"crate::call_indirect(ctx, base, {conv(self.parse(m[1]), 'u32')});"
        if self.lifter.hook_call(s.rstrip(";")) is not None:
            return f"if pc_fixes() {{ {self.hook_code(s)} }} // UnleashedRecomp PC fix"
        v = self.simde_store(s)
        if v is not None:
            return v
        if s.startswith("ctx.fpscr."):
            return s.replace("ctx.fpscr.", "ctx.fpscr.")
        return self.stmt(s)

    def switch(self, rest, i, out):
        m = re.fullmatch(r"switch \((.*)\) \{", rest[i])
        if not m:
            raise Fail(f"switch {rest[i]}")
        e = self.parse(m[1])
        arms = []
        i += 1
        while rest[i] != "}":
            c = re.fullmatch(r"case (\d+):", rest[i])
            if c:
                g = re.fullmatch(r"goto (loc_[0-9A-F]+);", rest[i + 1])
                if not g:
                    raise Fail(f"case body {rest[i + 1]}")
                arms.append(f"{c[1]} => {self.goto(g[1])}")
                i += 2
            elif rest[i] == "default:" and rest[i + 1] == "__builtin_unreachable();":
                arms.append("_ => unreachable!(),")
                i += 2
            else:
                raise Fail(f"switch body {rest[i]}")
        out.append(f"match {e.code} {{")
        out += [a if a.endswith(",") else a + "," for a in arms]
        out.append("}")
        return i + 1


# ---------------------------------------------------------------- whole program

class Lifter:
    def __init__(self):
        self.idx = fn.index()
        self.hooks = self.read_hooks()
        self.lifted = set()
        # Functions UnleashedRecomp replaces with host code: called through Host::hook.
        with open(RS / "sheets" / "host_functions.csv", encoding="utf-8-sig", newline="") as fh:
            self.host_fns = {r["address"][2:].upper() for r in csv.DictReader(fh)}

    def read_hooks(self):
        hooks = {}
        for p in sorted((HERE / "ppc").glob("ppc_recomp.*.cpp")):
            text = p.read_text(encoding="utf-8", errors="replace")
            for ret, name in re.findall(r"^extern (void|bool) (\w+)\([^)]*\);$", text, re.M):
                hooks[name] = ret
        return hooks

    def hook_call(self, s):
        """A mid-asm hook call: None if `s` is not one, else its value on the console."""
        m = re.fullmatch(r"(!?)(\w+)\(([^()]*)\)", s.strip())
        if not m or m[2] not in self.hooks:
            return None
        return "true" if m[1] else "false"

    def direct_call(self, a, imp=False):
        if a in self.host_fns:
            return f"host_call(|h| h.hook(0x{a}, ctx, base));"
        if a in self.lifted:
            return f"crate::sub_{a}(ctx, base);"
        return f"host_call(|h| h.missing(0x{a}, ctx, base));"

    def closure(self, roots, stop=()):
        seen, stack = set(), list(roots)
        while stack:
            a = stack.pop()
            if a in seen or a not in self.idx:
                continue
            if a in self.host_fns:
                continue
            seen.add(a)
            if a in stop:
                continue
            for c in re.findall(r"\bsub_([0-9A-F]{8})\(ctx, base\)", "\n".join(fn.body(self.idx, a))):
                stack.append(c)
        return seen

    def function(self, a):
        f = Function(self, a)
        body = f.lift()
        return "\n".join([
            *(["#[inline(never)]"] if a in COURSE_COLLISION_BOUNDARIES else []),
            f"#[target_feature(enable = \"sse4.1,ssse3,popcnt\")]",
            f"pub unsafe fn sub_{a}(ctx: &mut PPCContext, base: *mut u8) {{",
            "unsafe {",
            *body,
            "}",
            "}",
        ])


def partition_dispatch(lib, extra_cases=None):
    """Same address lookup, bounded match sizes for LLVM and incremental builds.

    The single 17,000-arm match made adding a few collision functions spend many
    minutes in one LLVM worker. Preserve trace_call and every arm exactly, while
    selecting a non-inlined table by the high 16 bits of the guest address.
    """
    start = lib.index("pub unsafe fn call_indirect(")
    end = lib.index("/// Every lifted guest address.", start)
    cases = dict(re.findall(r"^\s*0x([0-9A-F]{8}) => (.+),$", lib[start:end], re.M))
    if extra_cases:
        cases.update(extra_cases)
    groups = {}
    for address, action in sorted(cases.items()):
        groups.setdefault(address[:4], []).append((address, action))
    dispatch = [
        "pub unsafe fn call_indirect(ctx: &mut PPCContext, base: *mut u8, address: u32) {",
        "    trace_call(address);", "    unsafe {", "        match address >> 16 {",
    ]
    dispatch += [f"            0x{prefix} => dispatch_{prefix}(ctx, base, address)," for prefix in groups]
    dispatch += ["            _ => host_call(|h| h.missing(address, ctx, base)),", "        }", "    }", "}", ""]
    for prefix, group in groups.items():
        dispatch += ["#[inline(never)]",
            f"unsafe fn dispatch_{prefix}(ctx: &mut PPCContext, base: *mut u8, address: u32) {{",
            "    unsafe {", "        match address {"]
        dispatch += [f"            0x{address} => {action}," for address, action in group]
        dispatch += ["            _ => host_call(|h| h.missing(address, ctx, base)),", "        }", "    }", "}", ""]
    return lib[:start] + "\n".join(dispatch) + "\n" + lib[end:]


def write_crate(L, roots, out):
    """Lift the closure of `roots` into crate `out` (src/lib.rs + src/f_<n>.rs chunks)."""
    out = Path(out)
    (out / "src").mkdir(parents=True, exist_ok=True)
    funcs = sorted(L.closure(roots))
    L.lifted = set(funcs)
    bodies, failed = {}, {}
    for a in funcs:
        try:
            bodies[a] = L.function(a)
        except Fail as e:
            failed[a] = str(e)
        except (KeyError, IndexError, ValueError) as e:
            failed[a] = f"translator error {type(e).__name__}: {e}"
    if failed:
        L.lifted -= set(failed)
        # Re-print calls to failed functions as host.missing.
        bodies = {a: L.function(a) for a in bodies}
    # One module per 64 KiB of guest address space: a newly lifted function only changes
    # its own module (and its callers'), so an incremental build recompiles just those.
    names = sorted(bodies)
    groups = {}
    for a in names:
        groups.setdefault(f"f{a[:4]}", []).append(a)
    mods = sorted(groups)
    for mod in mods:
        text = ["#![allow(unused, clippy::all)]", "use xenon_rt::*;", "use xenon_rt::x86_64::*;", ""]
        text += [bodies[a] + "\n" for a in groups[mod]]
        write(out / "src" / f"{mod}.rs", "\n".join(text))
    for stale in (out / "src").glob("f*.rs"):
        if stale.stem not in groups:
            stale.unlink()
    lib = ["//! Guest code lifted from XenonRecomp's C++ by re/xenon/ppc2rust.py: generated, do not edit.",
           f"//! {len(roots)} roots (sheets/lift_roots.csv); {len(names)} functions; not lifted (host.missing): {len(failed)}.",
           "#![allow(unused, clippy::all, non_snake_case)]", "use xenon_rt::*;", ""]
    lib += [f"mod {m};\npub use {m}::*;" for m in mods]
    lib += ["", "/// PPC_CALL_INDIRECT_FUNC: the lifted function at a guest address, else the host.",
            "pub unsafe fn call_indirect(ctx: &mut PPCContext, base: *mut u8, address: u32) {",
            "    trace_call(address);", "    unsafe {", "        match address {"]
    lib += [f"            0x{a} => sub_{a}(ctx, base)," for a in names]
    lib += [f"            0x{a} => host_call(|h| h.hook(0x{a}, ctx, base))," for a in sorted(L.host_fns)]
    # Kernel / XAM import thunks reached through function pointers (ppc_func_mapping.cpp).
    mapping = (HERE / "ppc" / "ppc_func_mapping.cpp").read_text(encoding="utf-8", errors="replace")
    thunks = sorted(set(re.findall(r"\{ 0x([0-9A-F]{8}), __imp__((?!sub_)\w+) \}", mapping)))
    lib += [f'            0x{a} => host_call(|h| h.import("{n}", ctx, base)),' for a, n in thunks if a not in L.host_fns and a not in names]
    lib += ["            _ => host_call(|h| h.missing(address, ctx, base)),", "        }", "    }", "}", "",
            "/// Every lifted guest address.", f"pub const LIFTED: [u32; {len(names)}] = ["]
    lib += [f"    0x{a}," for a in names]
    lib += ["];", ""]
    write(out / "src" / "lib.rs", partition_dispatch("\n".join(lib)))
    (out / "lift_failures.json").write_text(json.dumps(failed, indent=1), encoding="utf-8")
    return names, failed


def write(p, text):
    if not p.is_file() or p.read_text(encoding="utf-8") != text:
        p.write_text(text, encoding="utf-8", newline="\n")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("roots", nargs="*")
    ap.add_argument("--show")
    ap.add_argument("--out")
    args = ap.parse_args()
    L = Lifter()
    if args.show:
        a = fn.norm(args.show)
        L.lifted = L.closure([a])
        try:
            print(L.function(a))
        except Fail as e:
            sys.exit(f"cannot lift {a}: {e}")
        return
    names, failed = write_crate(L, [fn.norm(r) for r in args.roots], args.out)
    print(f"lifted {len(names)} functions, {len(failed)} failed")
    from collections import Counter
    for k, n in Counter(v[:70] for v in failed.values()).most_common(30):
        print(f"  {n:4} {k}")


if __name__ == "__main__":
    main()

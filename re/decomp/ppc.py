"""Tiny PowerPC helpers over the flat XEX basefile (patched/default.xex).

Capstone handles the base ISA; VMX128 opcodes decode as None and are shown as .long.
usage: python ppc.py dis <addr> [count]
"""
import struct
import sys
from pathlib import Path

import capstone

ROOT = Path(__file__).resolve().parents[3]
BASE = 0x82000000


class Image:
    def __init__(self, xex=ROOT / "patched" / "default.xex"):
        b = xex.read_bytes()
        self.pe = b[struct.unpack_from(">I", b, 8)[0]:]
        self.md = capstone.Cs(capstone.CS_ARCH_PPC, capstone.CS_MODE_64 | capstone.CS_MODE_BIG_ENDIAN)
        self.md.detail = True
        self.md.skipdata = True

    def u32(self, va):
        return struct.unpack_from(">I", self.pe, va - BASE)[0]

    def f32(self, va):
        return struct.unpack_from(">f", self.pe, va - BASE)[0]

    def cstr(self, va, enc="ascii"):
        o = va - BASE
        e = self.pe.index(b"\0", o)
        return self.pe[o:e].decode(enc, "replace")

    def insns(self, va, size):
        o = va - BASE
        return self.md.disasm(self.pe[o:o + size], va)


def main():
    img = Image()
    if sys.argv[1] == "dis":
        va = int(sys.argv[2], 16)
        n = int(sys.argv[3]) if len(sys.argv) > 3 else 64
        for i, ins in enumerate(img.insns(va, n * 4)):
            print(f"{ins.address:08x}  {ins.mnemonic:8} {ins.op_str}")


if __name__ == "__main__":
    main()

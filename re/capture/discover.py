"""Find CPlayerSpeedContext fields in a discovery trace by correlation.

A discovery trace (Start-ReferenceCapture.ps1 -Discovery) carries the raw context
(0x800 bytes) and proxy (0x200 bytes) of every posture sample. Each context offset is
read as a big-endian float or vec3 per sample and compared with signals that are known
for sure:

  proxy velocity (proxy +0x130)   -> context vec3 equal to it (a context copy)
  horizontal velocity direction   -> unit vec3 that follows it while running (facing)
  left stick magnitude            -> unit-length-or-less vec3 that is 0 at rest (world input)

Prints ranked candidates. Nothing is decided here: confirmed fields go into
sheets/context_fields.csv with the trace and score as evidence.

usage: python re/capture/discover.py <trace.jsonl> [--top 8]
"""
import json
import math
import struct
import sys


def floats(hexstr):
    b = bytes.fromhex(hexstr)
    return [struct.unpack_from(">f", b, o)[0] for o in range(0, len(b) - 3, 4)]


def vec(fs, o):
    i = o // 4
    return fs[i], fs[i + 1], fs[i + 2]


def norm(v):
    return math.sqrt(sum(x * x for x in v))


def finite(v):
    return all(math.isfinite(x) and abs(x) < 1e7 for x in v)


def load(path):
    out = []
    for line in open(path, encoding="utf-8"):
        r = json.loads(line)
        if r.get("kind") == "sample" and r.get("context_bytes") and r.get("proxy_bytes") and r.get("proxy_valid"):
            out.append((floats(r["context_bytes"]), floats(r["proxy_bytes"]), r))
    return out


def main():
    path = sys.argv[1]
    top = int(sys.argv[sys.argv.index("--top") + 1]) if "--top" in sys.argv else 8
    samples = load(path)
    if len(samples) < 30:
        sys.exit(f"{len(samples)} usable samples; record longer (idle, run, turn, jump)")
    n_off = len(samples[0][0]) * 4 - 12
    vel = [vec(p, 0x130) for _, p, _ in samples]

    # 1. context vec3 equal to the proxy velocity
    copy = []
    for o in range(0, n_off, 4):
        err = [norm([a - b for a, b in zip(vec(c, o), v)]) for (c, _, _), v in zip(samples, vel)]
        if all(math.isfinite(e) for e in err):
            copy.append((sum(err) / len(err), o))
    print("velocity copies (mean |ctx - proxy velocity|):")
    for e, o in sorted(copy)[:top]:
        print(f"  ctx+0x{o:03X}  {e:.6f}")

    # 2. unit vec3 that follows the horizontal velocity direction while moving
    moving = [i for i, v in enumerate(vel) if math.hypot(v[0], v[2]) > 5]
    facing = []
    for o in range(0, n_off, 4):
        vs = [vec(samples[i][0], o) for i in range(len(samples))]
        if not all(finite(v) and abs(norm(v) - 1) < 1e-3 for v in vs):
            continue
        if moving:
            dots = []
            for i in moving:
                h = math.hypot(vel[i][0], vel[i][2])
                dots.append((vs[i][0] * vel[i][0] + vs[i][2] * vel[i][2]) / h)
            facing.append((sum(dots) / len(dots), o))
    print(f"facing candidates (unit vec3; mean dot with horizontal velocity over {len(moving)} moving samples):")
    for d, o in sorted(facing, reverse=True)[:top]:
        print(f"  ctx+0x{o:03X}  {d:.4f}")

    # 3. world input: length tracks the stick, zero with the stick released
    stick = [math.hypot(r["pad"]["lx"], r["pad"]["ly"]) if r.get("pad") else 0.0 for _, _, r in samples]
    inp = []
    for o in range(0, n_off, 4):
        vs = [vec(samples[i][0], o) for i in range(len(samples))]
        if not all(finite(v) and norm(v) <= 1.0001 for v in vs):
            continue
        err = [abs(norm(v) - min(s, 1.0)) for v, s in zip(vs, stick)]
        if max(stick) > 0.5:
            inp.append((sum(err) / len(err), o))
    print("world input candidates (mean ||ctx vec3| - stick length|):")
    for e, o in sorted(inp)[:top]:
        print(f"  ctx+0x{o:03X}  {e:.4f}")


if __name__ == "__main__":
    main()

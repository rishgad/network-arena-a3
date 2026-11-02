#!/usr/bin/env python3
"""Hypothesis decoder for the supplied logs (provenance unknown).

    python3 -I analysis/evidence/decode.py LOG CUSTOMERS [--tag N] [--csv]

Every field name below is an INFERENCE from structure (see
analysis/evidence/SCHEMA.md for the evidence and confidence of each). Field
names are prefixed `h_` to keep that visible. Records whose tag has no
hypothesis are printed raw. Offsets are file offsets of the tag byte.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
sys.path.insert(0, str(Path(__file__).resolve().parent))
from custmap import CustomerMap, int_to_ip  # noqa: E402
from tlv import frame  # noqa: E402


def u(p: bytes, off: int, n: int) -> int:
    return int.from_bytes(p[off : off + n], "little")


def varints(p: bytes) -> list[int]:
    out, v, s = [], 0, 0
    for b in p:
        v |= (b & 0x7F) << s
        s += 7
        if not b & 0x80:
            out.append(v)
            v, s = 0, 0
    if s:
        out.append(-1)  # trailing incomplete varint
    return out


# Fixed layouts, seen in both logs (tags 1, 2, 3, 8) or log 1 only (5, 6).
FIXED = {
    1: [("h_sw", 0, 4), ("h_nbr", 4, 4), ("h_port", 8, 2), ("h_ms", 10, 4), ("h_flag", 14, 1)],
    2: [("h_sw", 0, 4), ("h_prefix", 4, 4), ("h_ms", 8, 4), ("h_a", 12, 4), ("h_b", 16, 4), ("h_c64", 20, 8)],
    3: [("h_sw", 0, 4), ("h_prefix", 4, 4), ("h_ms0", 8, 4), ("h_ms", 12, 4)],
    5: [("h_sw", 0, 4), ("h_nbr", 4, 4), ("h_port", 8, 2), ("h_ms", 10, 4), ("h_a", 14, 4), ("h_b", 18, 4), ("h_c64", 22, 4)],
    6: [("h_sw", 0, 4), ("h_port", 4, 2), ("h_ms", 6, 4), ("h_n", 10, 4)],
    7: [("h_b0", 0, 1), ("h_sw", 1, 4), ("h_b5", 5, 1)],
    8: [("h_sw", 0, 4), ("h_ms", 4, 4), ("h_src", 8, 4), ("h_dst", 12, 4), ("h_rest", 16, 17)],
}
ADDR = {"h_prefix", "h_src", "h_dst"}


def decode(rec, cm: CustomerMap | None) -> dict:
    p = rec.payload
    d = {"idx": rec.index, "off": rec.offset, "tag": rec.tag}
    lay = FIXED.get(rec.tag)
    if lay and len(p) == sum(n for _, _, n in lay) or (lay and rec.tag == 5 and len(p) == 26):
        for name, off, n in lay:
            if name == "h_rest":
                d[name] = p[off : off + n].hex()
            else:
                v = u(p, off, n)
                d[name] = v
                if name in ADDR and cm:
                    d[name + "_who"] = cm.label(v)
    else:
        d["varints"] = varints(p)
    return d


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("log", type=Path)
    ap.add_argument("customers", type=Path)
    ap.add_argument("--tag", type=int, action="append")
    a = ap.parse_args()
    cm = CustomerMap.load(a.customers)
    recs, broke = frame(a.log.read_bytes())
    if broke is not None:
        sys.exit(f"framing broke at {broke}")
    for r in recs:
        if a.tag and r.tag not in a.tag:
            continue
        d = decode(r, cm)
        print(" ".join(f"{k}={int_to_ip(v) if k in ADDR else v}" for k, v in d.items()))
    return 0


if __name__ == "__main__":
    sys.exit(main())

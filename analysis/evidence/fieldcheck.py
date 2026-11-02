#!/usr/bin/env python3
"""Test candidate field layouts of the supplied logs against structure.

    python3 -I analysis/evidence/fieldcheck.py LOG CUSTOMERS

For every tag, and every 4-byte-aligned and unaligned u32/u16 window, it
reports evidence for a few field *types* (not meanings):
  addr   share of values whose top byte is 10 and that fall in a customer
         range by longest prefix (and how many are exact /24 network addrs)
  small  max value <= 64 (switch id / port / index candidates)
  mono   share of record-to-record steps that are non-decreasing, in log
         order (a time-like field written as it goes)
Nothing here names a field; the report is the evidence for doing so.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
sys.path.insert(0, str(Path(__file__).resolve().parent))
from custmap import CustomerMap  # noqa: E402
from tlv import frame  # noqa: E402


def main() -> int:
    log, cust = Path(sys.argv[1]), Path(sys.argv[2])
    cm = CustomerMap.load(cust)
    recs, broke = frame(log.read_bytes())
    assert broke is None
    tags = sorted({r.tag for r in recs})
    for t in tags:
        ps = [r.payload for r in recs if r.tag == t]
        n = min(len(p) for p in ps)
        print(f"== tag {t}: {len(ps)} recs, min len {n}")
        for w in (4, 2):
            for off in range(0, n - w + 1):
                vals = [int.from_bytes(p[off : off + w], "little") for p in ps]
                notes = []
                if w == 4:
                    tens = [v for v in vals if v >> 24 == 10]
                    if len(tens) >= 0.9 * len(vals):
                        hit = sum(cm.lookup(v) is not None for v in tens)
                        net = sum(v & 255 == 0 for v in tens)
                        notes.append(f"addr {hit}/{len(vals)} in map, {net} end .0")
                mx = max(vals)
                if mx <= 64 and len(set(vals)) > 1:
                    notes.append(f"small max {mx} distinct {len(set(vals))}")
                if len(vals) > 3 and len(set(vals)) > 3 and mx < 1 << 24:
                    steps = sum(b >= a for a, b in zip(vals, vals[1:]))
                    share = steps / (len(vals) - 1)
                    if share > 0.85:
                        notes.append(f"mono {share:.2f} range {min(vals)}..{mx}")
                if notes:
                    print(f"  u{w*8}@{off:<2} " + "; ".join(notes))
    return 0


if __name__ == "__main__":
    sys.exit(main())

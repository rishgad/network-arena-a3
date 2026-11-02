#!/usr/bin/env python3
"""Audit a team log of unknown provenance under a candidate TLV framing.

    python3 -I analysis/evidence/tlv.py LOG [--dump TAG] [--fields TAG]

Candidate framing: [tag u8][payload_len u16 LE][payload]. This tool only
checks whether that framing consumes the file exactly; it assigns no
meaning to tags or fields. Output: framing check, tag/length tables, and
(with --fields) per-byte-offset statistics for one tag so candidate fields
can be judged structurally. Read-only: never writes to the log.
"""

from __future__ import annotations

import argparse
import collections
import hashlib
import sys
from dataclasses import dataclass
from pathlib import Path


@dataclass
class Rec:
    index: int
    offset: int  # offset of the tag byte in the file
    tag: int
    payload: bytes


def frame(data: bytes) -> tuple[list[Rec], int | None]:
    """Records under the candidate framing, and the offset where framing
    broke (None if the file was consumed exactly)."""
    out, off = [], 0
    while off < len(data):
        if off + 3 > len(data):
            return out, off
        tag = data[off]
        n = int.from_bytes(data[off + 1 : off + 3], "little")
        if off + 3 + n > len(data):
            return out, off
        out.append(Rec(len(out), off, tag, data[off + 3 : off + 3 + n]))
        off += 3 + n
    return out, None


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("log", type=Path)
    ap.add_argument("--dump", type=int, help="hex-dump every record with this tag")
    ap.add_argument("--fields", type=int, help="per-offset byte statistics for this tag")
    ap.add_argument("--limit", type=int, default=40)
    a = ap.parse_args()
    data = a.log.read_bytes()
    recs, broke = frame(data)
    print(f"file {a.log}  bytes {len(data)}  sha256 {hashlib.sha256(data).hexdigest()}")
    print(f"records {len(recs)}  consumed {'exactly' if broke is None else f'NO: broke at offset {broke}'}")
    order = []
    for r in recs:
        if r.tag not in order:
            order.append(r.tag)
    lens = collections.defaultdict(collections.Counter)
    for r in recs:
        lens[r.tag][len(r.payload)] += 1
    print("tag  count  payload lengths (len:count)   first-offset")
    for t in order:
        first = next(r.offset for r in recs if r.tag == t)
        print(f"{t:>3}  {sum(lens[t].values()):>5}  {dict(sorted(lens[t].items()))}   {first}")
    if a.dump is not None:
        for r in [r for r in recs if r.tag == a.dump][: a.limit]:
            print(f"#{r.index:<5} @{r.offset:<6} {r.payload.hex(' ')}")
    if a.fields is not None:
        rs = [r.payload for r in recs if r.tag == a.fields]
        if rs:
            n = max(len(p) for p in rs)
            print(f"tag {a.fields}: {len(rs)} records; per-offset distinct values / min / max")
            for i in range(n):
                vals = [p[i] for p in rs if len(p) > i]
                c = collections.Counter(vals)
                print(f"  +{i:<3} distinct {len(c):>3}  min {min(vals):>3}  max {max(vals):>3}  top {c.most_common(4)}")
    return 0 if broke is None else 1


if __name__ == "__main__":
    sys.exit(main())

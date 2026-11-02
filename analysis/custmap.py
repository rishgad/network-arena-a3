"""Customer maps (customers.txt) and longest-prefix attribution.

Each line is `a.b.c.d/len  Name`. Ranges overlap (a /32 inside a /24) and
names repeat (two "Sync" ranges): every distinct range is its own entry,
keyed by its prefix, and a lookup returns the longest prefix holding the
address. Offline analysis only: a deployed detector never sees this file.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path


def ip_to_int(s: str) -> int:
    a, b, c, d = (int(x) for x in s.split("."))
    return a << 24 | b << 16 | c << 8 | d


def int_to_ip(v: int) -> str:
    return ".".join(str(v >> s & 255) for s in (24, 16, 8, 0))


@dataclass(frozen=True)
class Range:
    addr: int
    plen: int
    name: str

    @property
    def prefix(self) -> str:
        return f"{int_to_ip(self.addr)}/{self.plen}"

    def holds(self, ip: int) -> bool:
        m = (0xFFFFFFFF << (32 - self.plen)) & 0xFFFFFFFF if self.plen else 0
        return ip & m == self.addr

    @property
    def label(self) -> str:
        return f"{self.name} [{self.prefix}]"


class CustomerMap:
    def __init__(self, ranges: list[Range]):
        self.ranges = sorted(ranges, key=lambda r: -r.plen)

    @classmethod
    def load(cls, path: Path) -> "CustomerMap":
        rs = []
        for line in Path(path).read_text().splitlines():
            line = line.strip()
            if not line:
                continue
            pfx, name = line.split(None, 1)
            addr, plen = pfx.split("/")
            rs.append(Range(ip_to_int(addr), int(plen), name.strip()))
        return cls(rs)

    def lookup(self, ip: int) -> Range | None:
        for r in self.ranges:  # longest first
            if r.holds(ip):
                return r
        return None

    def label(self, ip: int) -> str:
        r = self.lookup(ip)
        return r.label if r else f"?{int_to_ip(ip)}"

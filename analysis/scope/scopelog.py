#!/usr/bin/env python3
"""Decode and summarise a scope v1 team log (programs/scope/FORMAT.md).

    python3 -I analysis/scope/scopelog.py LOG [--customers customers.txt]
        [--links] [--customers-table] [--ports] [--pairs] [--budget] [--delivery] [--all]

With --customers (a truth run's customers.txt), each customer index is
named by matching the ranges the log's header carries; without it the
index and range are shown. Offline only: detectors never read that file.
"""

from __future__ import annotations

import argparse
import collections
import sys
from dataclasses import dataclass, field
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from custmap import CustomerMap, Range, int_to_ip  # noqa: E402

TAG_HEADER, TAG_CUSTOMERS = 0xA3, 0xA4
TAG_WINDOW, TAG_LINK, TAG_PAIRS = 1, 2, 3
CF_DUP, CF_LOWTTL, CF_UDP = 2, 4, 8


class R:
    def __init__(self, b: bytes):
        self.b, self.i = b, 0

    def u8(self):
        v = self.b[self.i]
        self.i += 1
        return v

    def u16(self):
        return self.u8() | self.u8() << 8

    def u32(self):
        return self.u16() | self.u16() << 16

    def var(self):
        v = s = 0
        while True:
            x = self.u8()
            v |= (x & 0x7F) << s
            s += 7
            if not x & 0x80:
                return v


@dataclass
class Log:
    size: int = 0
    frames: int = 0
    bytes_by_tag: collections.Counter = field(default_factory=collections.Counter)
    count_by_tag: collections.Counter = field(default_factory=collections.Counter)
    header: dict = field(default_factory=dict)
    customers: dict = field(default_factory=dict)  # idx -> (addr, plen)
    windows: list = field(default_factory=list)
    links: list = field(default_factory=list)
    pairs: list = field(default_factory=list)
    samples: list = field(default_factory=list)
    errors: list = field(default_factory=list)
    trailing: int = 0


def parse(data: bytes) -> Log:
    L = Log(size=len(data))
    i = 0
    while i + 4 <= len(data):
        tag, n = data[i], data[i + 1]
        if n < 2 or i + 2 + n > len(data):
            break
        arrive = (data[i + 2] | data[i + 3] << 8) * 10
        body = data[i + 4 : i + 2 + n]
        L.frames += 1
        L.bytes_by_tag[tag] += n + 2
        L.count_by_tag[tag] += 1
        try:
            r = R(body)
            if tag == TAG_HEADER:
                L.header = dict(version=r.u8(), logger=r.u8(), window_ms=r.u16(), tick_ms=r.u16(), ncust=r.u8())
            elif tag == TAG_CUSTOMERS:
                k = r.u8()
                while r.i < len(body):
                    k += 1
                    L.customers[k] = (r.u32(), r.u8())
            elif tag == TAG_WINDOW:
                w = dict(arrive=arrive, sw=r.u8(), seq=r.u8(), t=r.u16() * 10, ports=[], cust=[], off=i)
                for _ in range(r.u8()):
                    w["ports"].append(dict(port=r.u8(), nbr=r.u8(), ka=r.u8(), dmin=r.var() * 10, dmax=r.var() * 10,
                                           drops=r.var(), pkts=r.var(), nsrc=r.u8(), util=r.u8()))
                for _ in range(r.u8()):
                    c = dict(idx=r.u8(), flags=r.u8())
                    c.update(pkts=r.var(), small=r.var(), mid=r.var(), new=r.var())
                    c["dup"] = r.var() if c["flags"] & CF_DUP else 0
                    c["lowttl"] = r.var() if c["flags"] & CF_LOWTTL else 0
                    c["udp"] = r.var() if c["flags"] & CF_UDP else 0
                    w["cust"].append(c)
                w["multi"] = [r.u8() for _ in range(r.u8())] if r.i < len(body) else []
                w["deliv"] = {}
                if r.i < len(body):
                    for _ in range(r.u8()):
                        k = r.u8(); w["deliv"][k] = r.var()
                L.windows.append(w)
            elif tag == TAG_LINK:
                L.links.append(dict(arrive=arrive, sw=r.u8(), port=r.u8(), nbr=r.u8(), silent=r.u8(), t=r.u16() * 10, off=i))
            elif tag == 7:
                sw, t = r.u8(), r.u16() * 10
                L.samples.append(dict(arrive=arrive, sw=sw, t=t, src=r.u32(), dst=r.u32(), proto=r.u8(), size=r.u16(), ttl=r.u8()))
            elif tag in (5, 6):
                pass
            elif tag == TAG_PAIRS:
                p = dict(arrive=arrive, sw=r.u8(), t=r.u16() * 10, items=[], off=i)
                for _ in range(r.u8()):
                    p["items"].append((r.u8(), r.u8(), r.var()))
                L.pairs.append(p)
            else:
                L.errors.append(f"unknown tag {tag} at {i}")
        except IndexError:
            L.errors.append(f"short body, tag {tag} at {i}")
        i += 2 + n
    L.trailing = len(data) - i
    return L


def namer(L: Log, cmap: CustomerMap | None):
    def name(idx: int) -> str:
        if idx == 0:
            return "unknown"
        a = L.customers.get(idx)
        if not a:
            return f"#{idx}"
        pfx = f"{int_to_ip(a[0])}/{a[1]}"
        if cmap:
            exact = [r for r in cmap.ranges if r.addr == a[0] and r.plen == a[1]]
            if exact:
                return f"{exact[0].name} [{pfx}]"
        return f"#{idx} [{pfx}]"

    return name


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("log", type=Path)
    ap.add_argument("--customers", type=Path)
    for f in ("samples", "multipath", "links", "customers-table", "ports", "pairs", "budget", "delivery", "all", "timeline"):
        ap.add_argument(f"--{f}", action="store_true")
    a = ap.parse_args()
    L = parse(a.log.read_bytes())
    cmap = CustomerMap.load(a.customers) if a.customers else None
    name = namer(L, cmap)
    every = a.all
    print(f"log {a.log}: {L.size} B, {L.frames} frames, trailing {L.trailing} B, errors {len(L.errors)}")
    print(f"header {L.header}  customers {len(L.customers)}")
    for e in L.errors[:5]:
        print("  ERROR", e)
    if a.budget or every:
        print("\n== budget by tag (bytes, frames)")
        for t, b in sorted(L.bytes_by_tag.items()):
            print(f"  tag {t:#04x}: {b:>6} B  {L.count_by_tag[t]:>5} frames")
        per_sw = collections.Counter()
        seqs = collections.defaultdict(list)
        lat = collections.defaultdict(list)
        for w in sorted(L.windows, key=lambda w: w["t"]):
            seqs[w["sw"]].append(w["seq"])
            lat[w["sw"]].append(w["arrive"] - w["t"])
        print("  windows per switch: received / expected-from-seq-span, lost, latency ms (median/max), latest obs t")
        for sw in sorted(seqs):
            s = seqs[sw]
            span = (s[-1] - s[0]) % 256 + 1 if s else 0
            ws = [w for w in L.windows if w["sw"] == sw]
            lt = sorted(lat[sw])
            # Internal gaps only: staggered timers can produce 39 or 40
            # windows. The sequence span cannot reveal missing first/last reports.
            expect = span
            print(f"   sw {sw:>2}: {len(s):>3}/{expect:<3} missing {expect - len(set(s)):>2}  lat {lt[len(lt)//2]:>4}/{lt[-1]:>5}  last t {max(w['t'] for w in ws)}")
        first = min((w["t"] for w in L.windows), default=0)
        last = max((w["t"] for w in L.windows), default=0)
        print(f"  coverage: windows from {first} ms to {last} ms")
    if a.samples or every:
        print("\n== sampled packets (src -> dst), one line per distinct customer pair")
        seen = {}
        for s in L.samples:
            k = (cmap.label(s["src"]) if cmap else int_to_ip(s["src"]), cmap.label(s["dst"]) if cmap else int_to_ip(s["dst"]))
            seen.setdefault(k, []).append(s)
        for (a_, b_), v in sorted(seen.items(), key=lambda kv: kv[1][0]["t"]):
            s = v[0]
            print(f"  {len(v):>3}x first t {s['t']:>6} sw {s['sw']:>2}  {a_} -> {b_}  proto {s['proto']} size {s['size']} ttl {s['ttl']}")
    if a.multipath or every:
        print("\n== sources arriving on 2+ ports of a switch (windows)")
        mp = collections.Counter()
        for w in L.windows:
            for i in w.get("multi", []):
                mp[(w["sw"], i)] += 1
        for (sw, i), n in sorted(mp.items()):
            print(f"  sw {sw:>2} {name(i):<45} {n} windows")
    if a.links or every:
        print("\n== link events (switch:port <- neighbour), t = last keepalive before silence / first after")
        for e in sorted(L.links, key=lambda e: (e["t"], e["sw"])):
            print(f"  t {e['t']:>6}  sw {e['sw']:>2} port {e['port']} nbr {e['nbr']:>3}  {'SILENT' if e['silent'] else 'heard'}  (logged {e['arrive']})")
    if a.ports or every:
        print("\n== per port: keepalive delay min/max us, drops, packets in (per window)")
        by = collections.defaultdict(list)
        for w in L.windows:
            for p in w["ports"]:
                by[(w["sw"], p["port"], p["nbr"])].append((w["t"], p))
        for k, v in sorted(by.items()):
            dmax = max(p["dmax"] for _, p in v)
            dmin = min((p["dmin"] for _, p in v if p["dmax"]), default=0)
            drops = sum(p["drops"] for _, p in v)
            print(f"  sw {k[0]:>2} port {k[1]} <- {k[2]:>3}: delay {dmin}..{dmax} us  drops {drops}\n     pkts in/win "
                  + " ".join(str(p["pkts"]) for _, p in v[:40]) + "\n     util out %  " + " ".join(str(p["util"]) for _, p in v[:40]))
    if a.customers_table or every:
        print("\n== local customers: per-window packets (delta), new destinations, size class")
        by = collections.defaultdict(list)
        for w in L.windows:
            for c in w["cust"]:
                by[(c["idx"], w["sw"])].append((w["t"], c))
        for (idx, sw), v in sorted(by.items()):
            last = v[-1][1]
            size = "small" if last["small"] > 0.8 * last["pkts"] else "mid" if last["mid"] > 0.8 * last["pkts"] else "large"
            prev, deltas = 0, []
            for _, c in v:
                deltas.append(c["pkts"] - prev)
                prev = c["pkts"]
            extra = f" dup {last['dup']}" if last["dup"] else ""
            extra += f" lowttl {last['lowttl']}" if last["lowttl"] else ""
            extra += f" udp {last['udp']}" if last["udp"] else ""
            print(f"  {name(idx):<45} sw {sw:>2} {size:<5}{extra}\n     pkts/win {' '.join(map(str, deltas))}\n     new dst  {' '.join(str(c['new']) for _, c in v)}")
    if a.delivery or every:
        print("\n== source ingress and host-egress counters (cumulative, per reporting switch)")
        print("  Counts have different observation times; do not subtract unaligned endpoints.")
        print("  Host-egress means routed toward a local host, before its final queue/link.")
        sent, delivered = {}, {}
        for w in sorted(L.windows, key=lambda w: w["t"]):
            for c in w["cust"]:
                sent[(c["idx"], w["sw"])] = (w["t"], c["pkts"])
            for idx, count in w.get("deliv", {}).items():
                delivered[(idx, w["sw"])] = (w["t"], count)
        sources = sorted({idx for idx, _ in sent} | {idx for idx, _ in delivered})
        if not delivered:
            print("  No nonzero host-egress counters recorded; check build mode and report coverage.")
        for idx in sources:
            print(f"  {name(idx)}")
            for label, readings in (("ingress", sent), ("host-egress", delivered)):
                for (source, sw), (t, count) in sorted(readings.items()):
                    if source == idx:
                        print(f"    {label:<11} sw {sw:>2}: {count:>9} packets at t={t} ms")
    if a.pairs or every:
        print("\n== pairs (last report per switch): source group -> destination, cumulative packets")
        last = {}
        for p in L.pairs:
            last.setdefault(p["sw"], {})
            for g, d, v in p["items"]:
                last[p["sw"]][(g, d)] = v
        for sw, items in sorted(last.items()):
            for (g, d), v in sorted(items.items()):
                print(f"  sw {sw:>2} group {g:>2} -> {name(d):<45} {v}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""Lossy wire (11-12 loses ~3-7% both ways on an idle link) with the truth
runs' other link incidents on as confounders.  python3 -I tests/claims/lossmatrix.py SW DET"""
import concurrent.futures as cf, subprocess, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
SIM = ROOT / "target/release/competitive_net_sim"
WORK = ROOT / "results/claims-loss"; WORK.mkdir(parents=True, exist_ok=True)
fail = lambda a, b, t, d: f'\n[[plant]]\nkind = "fail"\na = {a}\nb = {b}\nat_ms = {t}\nfor_ms = {d}\n'
def loss(a, b, pct, start=0, stop=None):
    s = f'\n[[plant]]\nkind = "wire_loss"\na = {a}\nb = {b}\nloss_pct = {pct}\nstart_ms = {start}\n'
    return s + (f"stop_ms = {stop}\n" if stop else "")
ALT = [[t, 4 if (t // 4000) % 2 else 10] for t in range(4000, 40000, 4000)]
CONF = ("".join(fail(1, 6, 5900 + k * 4000, 1600) for k in range(4)) + fail(0, 1, 21930, 8160) + fail(10, 11, 21930, 8160)
        + f'\n[[plant]]\nkind = "link_capacity"\na = 12\nb = 13\nsteps = {ALT}\n'
        + "\n[[harness.queue]]\nswitch = 6\ntoward = 7\npackets = 1\n")
CASES = {"on-3pct": (True, loss(11, 12, 3)), "on-6pct": (True, loss(11, 12, 6)), "on-other": (True, loss(3, 4, 5)),
         "brief": (False, loss(11, 12, 5, 15000, 20000)), "off": (False, ""),
         "queue": (False, "\n[[harness.queue]]\nswitch = 11\ntoward = 12\npackets = 1\n")}
def run(item):
    (name, (want, extra)), seed, load = item
    base = subprocess.run(["python3", "-I", str(ROOT / "worlds/local/gen_grid.py"), str(seed)], capture_output=True, text=True).stdout
    base = base.replace('preset = "steam"', 'preset = "netflix"\nrate_mbps = 3.0')
    p = WORK / f"{name}-s{seed}-l{load}.toml"; p.write_text(base + CONF + extra)
    team = p.with_suffix(".team")
    r = subprocess.run([SIM, "run-world", p, "--load-scale", str(load), "--program", sys.argv[1], "--team-log", team], capture_output=True, text=True)
    w = "true" if want else "false"
    if r.returncode: return p.stem, -1, "ERR " + (r.stderr or r.stdout)[-160:], w
    return p.stem, team.stat().st_size, subprocess.run([SIM, "detect", sys.argv[2], team], capture_output=True, text=True).stdout.strip(), w
items = [(c, s, l) for c in CASES.items() for s in (111, 112) for l in (1.0, 1.5)]
with cf.ThreadPoolExecutor(8) as ex: rows = list(ex.map(run, items))
bad = sum(r[2] != r[3] for r in rows)
for r in rows: print(f"{r[0]:<20} {r[1]:>6}B said {r[2]:<6} want {r[3]}  {'ok' if r[2] == r[3] else 'WRONG'}")
print(f"\n{len(rows) - bad}/{len(rows)} right")

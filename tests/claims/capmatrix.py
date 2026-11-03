#!/usr/bin/env python3
"""Capacity schedule (12->13 cut to 4 Mb/s for 4 s in every 8) on a calm
grid proxy with a heavy 12->13 sender.  python3 -I tests/claims/capmatrix.py SW DET"""
import concurrent.futures as cf, re, subprocess, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
SIM = ROOT / "target/release/competitive_net_sim"
WORK = ROOT / "results/claims-cap"; WORK.mkdir(parents=True, exist_ok=True)
def cap(a, b, steps): return f'\n[[plant]]\nkind = "link_capacity"\na = {a}\nb = {b}\nsteps = {steps}\n'
ALT = [[t, 4 if (t // 4000) % 2 else 10] for t in range(4000, 40000, 4000)]
CASES = {"on": (True, cap(12, 13, ALT)),
         "off": (False, ""), "one-cut": (False, cap(12, 13, [[20000, 4]])),
         "always-slow": (False, cap(12, 13, [[0, 4]])),
         "shallow-queue": (False, "\n[[harness.queue]]\nswitch = 12\ntoward = 13\npackets = 1\n")}
def world(seed, extra):
    base = subprocess.run(["python3", "-I", str(ROOT / "worlds/local/gen_grid.py"), str(seed)], capture_output=True, text=True).stdout
    base = base.replace('preset = "steam"', 'preset = "netflix"\nrate_mbps = 3.0')
    blocks = base.split("[[app]]")
    i = next(i for i, b in enumerate(blocks) if 'name = "Backup"\n' in b)
    blocks[i + 1] = re.sub(r"switch = \d+", "switch = 13", blocks[i + 1], count=1)
    j = next(i for i, b in enumerate(blocks) if 'name = "Cam-A"\n' in b)
    blocks[i + 1] = blocks[i + 1]
    # Backup stays a TCP download as fast as it can (as on the real network),
    # so it backs off when the link is cut, the way the truth runs show.
    blocks[i] = blocks[i].replace('preset = "netflix"\nrate_mbps = 3.0', 'preset = "steam"\nlaw = "aimd"')
    text = "[[app]]".join(blocks)
    return text + extra
def run(item):
    (name, (want, extra)), seed, load = item
    p = WORK / f"{name}-s{seed}-l{load}.toml"; p.write_text(world(seed, extra))
    team = p.with_suffix(".team")
    r = subprocess.run([SIM, "run-world", p, "--load-scale", str(load), "--program", sys.argv[1], "--team-log", team], capture_output=True, text=True)
    if r.returncode: return p.stem, -1, "ERR " + (r.stderr or r.stdout)[-160:], "true" if want else "false"
    a = subprocess.run([SIM, "detect", sys.argv[2], team], capture_output=True, text=True).stdout.strip()
    return p.stem, team.stat().st_size, a, "true" if want else "false"
items = [(c, s, l) for c in CASES.items() for s in (71, 72) for l in (1.0, 1.5)]
with cf.ThreadPoolExecutor(8) as ex: rows = list(ex.map(run, items))
bad = sum(r[2] != r[3] for r in rows)
for r in rows: print(f"{r[0]:<24} {r[1]:>6}B said {r[2]:<6} want {r[3]}  {'ok' if r[2] == r[3] else 'WRONG'}")
print(f"\n{len(rows) - bad}/{len(rows)} right")

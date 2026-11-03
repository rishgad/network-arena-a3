#!/usr/bin/env python3
"""Claim matrix for the shallow queue (6->7 drops at low utilisation).
    python3 -I tests/claims/queuematrix.py SWITCH.wasm DETECT.wasm"""
import concurrent.futures as cf, subprocess, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
SIM = ROOT / "target/release/competitive_net_sim"
WORK = ROOT / "results/claims-queue"; WORK.mkdir(parents=True, exist_ok=True)
q = lambda s, t, n: f"\n[[harness.queue]]\nswitch = {s}\ntoward = {t}\npackets = {n}\n"
CASES = {"on-6-7": (True, q(6, 7, 1)), "on-1-2": (True, q(1, 2, 1)), "off": (False, ""), "deep": (False, q(6, 7, 30))}
def run(item):
    (name, (want, extra)), seed, load = item
    base = subprocess.run(["python3", "-I", str(ROOT / "worlds/local/gen_grid.py"), str(seed)], capture_output=True, text=True).stdout
    base = base.replace('preset = "steam"', 'preset = "netflix"\nrate_mbps = 3.0')
    # Camera-like bursts from 6 to 7 and from 1 to 2, as truth run 1 had on 6->7.
    cams = "".join(f'''
[[app]]
id = {900 + i}
switch = {a}
prefix = "10.250.{i}.0/24"
hosts = [7]
name = "Cam-{i}"
preset = "amazon"
rate_mbps = 2.0
on_ms = 200
off_ms = 700
start_ms = 0
peer = {950 + i}

[[app]]
id = {950 + i}
switch = {b}
prefix = "10.251.{i}.0/24"
hosts = [9]
name = "Cam-{i}'s peer"
''' for i, (a, b) in enumerate([(6, 7), (1, 2)]))
    head, tail = base.split("[workload]", 1)
    p = WORK / f"{name}-s{seed}-l{load}.toml"; p.write_text(head + cams + "\n[workload]" + tail + extra)
    team = p.with_suffix(".team")
    r = subprocess.run([SIM, "run-world", p, "--load-scale", str(load), "--program", sys.argv[1], "--team-log", team], capture_output=True, text=True)
    if r.returncode: return p.stem, -1, "ERR " + (r.stderr or r.stdout)[-150:], want
    a = subprocess.run([SIM, "detect", sys.argv[2], team], capture_output=True, text=True).stdout.strip()
    return p.stem, team.stat().st_size, a, "true" if want else "false"
items = [(c, s, l) for c in CASES.items() for s in (41, 42) for l in (1.0, 1.5)]
with cf.ThreadPoolExecutor(8) as ex: rows = list(ex.map(run, items))
bad = sum(r[2] != r[3] for r in rows)
for r in rows: print(f"{r[0]:<18} {r[1]:>6}B said {r[2]:<6} want {r[3]}  {'ok' if r[2] == r[3] else 'WRONG'}")
print(f"\n{len(rows) - bad}/{len(rows)} right")

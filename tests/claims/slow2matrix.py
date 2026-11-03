#!/usr/bin/env python3
"""Claim matrix for the one-way slow link (egg 14) on the grid proxy.
Every world carries the truth run's other delay features as confounders:
13-14 slow both ways (25 ms) and 6->11 slightly slow (0.86 ms).
    python3 -I tests/claims/delaymatrix.py SWITCH.wasm DETECT.wasm
"""
import concurrent.futures as cf, subprocess, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
SIM = ROOT / "target/release/competitive_net_sim"
WORK = ROOT / "results/claims-delay"; WORK.mkdir(parents=True, exist_ok=True)
def delay(a, b, ab, ba): return f'[[plant]]\nkind = "link_delay"\na = {a}\nb = {b}\na_to_b_us = {ab}\nb_to_a_us = {ba}\n'
BASE = delay(1, 2, 2000, 30000) + delay(6, 11, 550, 860)  # one-way egg on as a confounder
CASES = {
    "on-13-14": (True, delay(13, 14, 25000, 25000)),
    "on-7-8": (True, delay(7, 8, 20000, 20000)),
    "on-weaker": (True, delay(3, 8, 8000, 9000)),
    "off": (False, ""),
    "one-way": (False, delay(13, 14, 25000, 550)),
    "slight-both": (False, delay(13, 14, 2000, 2000)),
}
def run(item):
    (name, (want, plants)), seed, load = item
    base = subprocess.run(["python3", "-I", str(ROOT / "worlds/local/gen_grid.py"), str(seed)], capture_output=True, text=True).stdout
    # Calmer, like truth run 1 (no lifted delay floors): steady senders, not flat-out.
    base = base.replace('preset = "steam"', 'preset = "netflix"\nrate_mbps = 3.0')
    p = WORK / f"{name}-s{seed}-l{load}.toml"; p.write_text(base + "\n" + BASE + plants)
    team = p.with_suffix(".team")
    subprocess.run([SIM, "run-world", p, "--load-scale", str(load), "--program", sys.argv[1], "--team-log", team], capture_output=True)
    a = subprocess.run([SIM, "detect", sys.argv[2], team], capture_output=True, text=True).stdout.strip()
    return p.stem, team.stat().st_size if team.exists() else -1, a, "true" if want else "false"
items = [(c, s, l) for c in CASES.items() for s in (21, 22) for l in (1.0, 1.5)]
with cf.ThreadPoolExecutor(8) as ex: rows = list(ex.map(run, items))
bad = 0
for name, size, got, want in rows:
    bad += got != want
    print(f"{name:<24} {size:>5}B said {got:<5} want {want}  {'ok' if got == want else 'WRONG'}")
print(f"\n{len(rows) - bad}/{len(rows)} right")

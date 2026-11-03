#!/usr/bin/env python3
"""Sync pair pause on a calm grid proxy (Sync starts on time; the egg is a
long pause of both Sync ranges).  python3 -I tests/claims/pausematrix.py SW DET"""
import concurrent.futures as cf, re, subprocess, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
SIM = ROOT / "target/release/competitive_net_sim"
WORK = ROOT / "results/claims-pause"; WORK.mkdir(parents=True, exist_ok=True)
def ids(text, name):
    return [int(re.search(r"id = (\d+)", b).group(1)) for b in text.split("[[app]]")[1:] if f'name = "{name}"\n' in b and "preset" in b]
def pause(a, at, dur): return f'\n[[plant]]\nkind = "pause"\napp = {a}\nat_ms = {at}\nfor_ms = {dur}\n'
def case(text, how):
    s = ids(text, "Sync")
    return {"on": "".join(pause(a, 1500, 18000) for a in s),
            "on-mid": "".join(pause(a, 12000, 14000) for a in s),
            "pair-short": "".join(pause(a, 10000, 3000) for a in s),
            "one-sync": pause(s[0], 8000, 15000),
            "other-cust": pause(ids(text, "Netflix")[0], 8000, 15000),
            "off": ""}[how]
WANT = {"on": True, "on-mid": True, "pair-short": False, "one-sync": False, "other-cust": False, "off": False}
def run(item):
    how, seed, load = item
    base = subprocess.run(["python3", "-I", str(ROOT / "worlds/local/gen_grid.py"), str(seed)], capture_output=True, text=True).stdout
    base = base.replace('preset = "steam"', 'preset = "netflix"\nrate_mbps = 3.0').replace("start_ms = 20000", "start_ms = 0")
    p = WORK / f"{how}-s{seed}-l{load}.toml"; p.write_text(base + case(base, how))
    team = p.with_suffix(".team")
    r = subprocess.run([SIM, "run-world", p, "--load-scale", str(load), "--program", sys.argv[1], "--team-log", team], capture_output=True, text=True)
    want = "true" if WANT[how] else "false"
    if r.returncode: return p.stem, -1, "ERR " + (r.stderr or r.stdout)[-160:], want
    a = subprocess.run([SIM, "detect", sys.argv[2], team], capture_output=True, text=True).stdout.strip()
    return p.stem, team.stat().st_size, a, want
items = [(h, s, l) for h in WANT for s in (81, 82) for l in (1.0, 1.5)]
with cf.ThreadPoolExecutor(8) as ex: rows = list(ex.map(run, items))
bad = sum(r[2] != r[3] for r in rows)
for r in rows: print(f"{r[0]:<22} {r[1]:>6}B said {r[2]:<6} want {r[3]}  {'ok' if r[2] == r[3] else 'WRONG'}")
print(f"\n{len(rows) - bad}/{len(rows)} right")

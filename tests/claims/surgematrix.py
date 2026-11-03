#!/usr/bin/env python3
"""Repeated surges (Steam: slow, with two 4 s surges to link speed) on a calm
grid proxy.  python3 -I tests/claims/surgematrix.py SW DET"""
import concurrent.futures as cf, re, subprocess, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
SIM = ROOT / "target/release/competitive_net_sim"
WORK = ROOT / "results/claims-surge"; WORK.mkdir(parents=True, exist_ok=True)
def aid(text, name):
    return next(int(re.search(r"id = (\d+)", b).group(1)) for b in text.split("[[app]]")[1:] if f'name = "{name}"\n' in b and "preset" in b)
def burst(a, start, dur, pps=1300, size=1000, proto="tcp"):
    return f'\n[[plant]]\nkind = "side_flow"\napp = {a}\nrate_pps = {pps}\nsize_bytes = {size}\nstart_ms = {start}\nstop_ms = {start + dur}\nproto = "{proto}"\n'
def base(seed):
    t = subprocess.run(["python3", "-I", str(ROOT / "worlds/local/gen_grid.py"), str(seed)], capture_output=True, text=True).stdout
    # Steam slow (~80 pkt/s) as on the real network; Backup stays a fast download.
    blocks = t.split("[[app]]")
    for i, b in enumerate(blocks):
        if 'name = "Steam"\n' in b:
            blocks[i] = b.replace('preset = "steam"', 'preset = "netflix"\nrate_mbps = 0.65')
    return "[[app]]".join(blocks).replace('preset = "steam"', 'preset = "steam"\nlaw = "aimd"')
CASES = {"on": (True, lambda t: burst(aid(t, "Steam"), 11000, 4000) + burst(aid(t, "Steam"), 27000, 4000)),
         "on-moved": (True, lambda t: burst(aid(t, "Steam"), 5000, 5000) + burst(aid(t, "Steam"), 22000, 3500)),
         "late": (True, lambda t: burst(aid(t, "Steam"), 21000, 4000) + burst(aid(t, "Steam"), 37000, 4000)),
         "len3-5": (True, lambda t: burst(aid(t, "Steam"), 9000, 3000) + burst(aid(t, "Steam"), 25000, 5000)),
         "one": (False, lambda t: burst(aid(t, "Steam"), 11000, 4000)),
         "half-speed": (False, lambda t: burst(aid(t, "Steam"), 11000, 4000, 500) + burst(aid(t, "Steam"), 27000, 4000, 500)),
         "long-hog": (False, lambda t: burst(aid(t, "Steam"), 8000, 20000)),
         "brief": (False, lambda t: burst(aid(t, "Steam"), 11000, 1500) + burst(aid(t, "Steam"), 27000, 1500)),
         "small-udp": (False, lambda t: burst(aid(t, "Home users"), 11000, 4000, 2000, 64, "udp") + burst(aid(t, "Home users"), 27000, 4000, 2000, 64, "udp")),
         "off": (False, lambda t: "")}
def run(item):
    (name, (want, f)), seed, load = item
    t = base(seed)
    p = WORK / f"{name}-s{seed}-l{load}.toml"; p.write_text(t + f(t))
    team = p.with_suffix(".team")
    r = subprocess.run([SIM, "run-world", p, "--load-scale", str(load), "--program", sys.argv[1], "--team-log", team], capture_output=True, text=True)
    w = "true" if want else "false"
    if r.returncode: return p.stem, -1, "ERR " + (r.stderr or r.stdout)[-160:], w
    return p.stem, team.stat().st_size, subprocess.run([SIM, "detect", sys.argv[2], team], capture_output=True, text=True).stdout.strip(), w
items = [(c, s, l) for c in CASES.items() for s in (101, 102) for l in (1.0, 1.5, 2.0)]
with cf.ThreadPoolExecutor(8) as ex: rows = list(ex.map(run, items))
bad = sum(r[2] != r[3] for r in rows)
for r in rows: print(f"{r[0]:<20} {r[1]:>6}B said {r[2]:<6} want {r[3]}  {'ok' if r[2] == r[3] else 'WRONG'}")
print(f"\n{len(rows) - bad}/{len(rows)} right")

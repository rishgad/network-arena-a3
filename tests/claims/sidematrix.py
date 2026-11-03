#!/usr/bin/env python3
"""Side stream: a steady trickle of small packets inside a large-packet
customer (truth runs: Netflix, 1.52% at 200 B).  python3 -I tests/claims/sidematrix.py SW DET"""
import concurrent.futures as cf, re, subprocess, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
SIM = ROOT / "target/release/competitive_net_sim"
WORK = ROOT / "results/claims-side"; WORK.mkdir(parents=True, exist_ok=True)
def aid(text, name):
    return next(int(re.search(r"id = (\d+)", b).group(1)) for b in text.split("[[app]]")[1:] if f'name = "{name}"\n' in b and "preset" in b)
def side(a, pps, size=200, start=0, stop=None, proto="tcp"):
    s = f'\n[[plant]]\nkind = "side_flow"\napp = {a}\nrate_pps = {pps}\nsize_bytes = {size}\nstart_ms = {start}\nproto = "{proto}"\n'
    return s + (f"stop_ms = {stop}\n" if stop else "")
def confound(text):
    blocks = text.split("[[app]]")
    i = next(i for i, b in enumerate(blocks) if 'name = "Vault"\n' in b)
    blocks[i] = blocks[i].replace('name = "Vault"\n', 'name = "Vault"\necho = true\n')
    t = "[[app]]".join(blocks)
    return t + side(aid(t, "Home users"), 2000, 64, 15000, 28000, "udp")
CASES = {"on": (True, lambda t: side(aid(t, "Netflix"), 5)),
         "on-other": (True, lambda t: side(aid(t, "Ledger"), 4)),
         "off": (False, lambda t: ""),
         "burst": (False, lambda t: side(aid(t, "Netflix"), 60, start=15000, stop=18000)),
         "heavy": (False, lambda t: side(aid(t, "Netflix"), 150))}
def run(item):
    (name, (want, f)), seed, load = item
    base = subprocess.run(["python3", "-I", str(ROOT / "worlds/local/gen_grid.py"), str(seed)], capture_output=True, text=True).stdout
    base = confound(base.replace('preset = "steam"', 'preset = "netflix"\nrate_mbps = 3.0'))
    p = WORK / f"{name}-s{seed}-l{load}.toml"; p.write_text(base + f(base))
    team = p.with_suffix(".team")
    r = subprocess.run([SIM, "run-world", p, "--load-scale", str(load), "--program", sys.argv[1], "--team-log", team], capture_output=True, text=True)
    w = "true" if want else "false"
    if r.returncode: return p.stem, -1, "ERR " + (r.stderr or r.stdout)[-160:], w
    return p.stem, team.stat().st_size, subprocess.run([SIM, "detect", sys.argv[2], team], capture_output=True, text=True).stdout.strip(), w
items = [(c, s, l) for c in CASES.items() for s in (91, 92) for l in (1.0, 1.5)]
with cf.ThreadPoolExecutor(8) as ex: rows = list(ex.map(run, items))
bad = sum(r[2] != r[3] for r in rows)
for r in rows: print(f"{r[0]:<18} {r[1]:>6}B said {r[2]:<6} want {r[3]}  {'ok' if r[2] == r[3] else 'WRONG'}")
print(f"\n{len(rows) - bad}/{len(rows)} right")

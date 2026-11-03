#!/usr/bin/env python3
"""Claim matrix for the shuffle (a split route). Calm grid proxy; truth-run
link eggs on as confounders.  python3 -I tests/claims/shufflematrix.py SWITCH DETECT"""
import concurrent.futures as cf, re, subprocess, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
SIM = ROOT / "target/release/competitive_net_sim"
WORK = ROOT / "results/claims-shuffle"; WORK.mkdir(parents=True, exist_ok=True)
NBR = {s: [b for a, b in L] for s in range(15) for L in [[(a, b) for a, b in
       [(r*5+c, r*5+c+1) for r in range(3) for c in range(4)] + [(r*5+c, (r+1)*5+c) for r in range(2) for c in range(5)]
       for (a, b) in [(a, b), (b, a)] if a == s]]}
fail = lambda a, b, t, d: f'\n[[plant]]\nkind = "fail"\na = {a}\nb = {b}\nat_ms = {t}\nfor_ms = {d}\n'
CONF = fail(0, 1, 21930, 8160) + fail(10, 11, 21930, 8160) + "".join(fail(1, 6, 5900 + k * 4000, 1600) for k in range(4))
def split(text, name):
    blocks = text.split("[[app]]")
    i = next(i for i, b in enumerate(blocks) if f'name = "{name}"\n' in b and "preset" in b)
    sw = int(re.search(r"switch = (\d+)", blocks[i]).group(1))
    pfx = re.search(r'prefix = "([^"]+)"', blocks[i + 1]).group(1)
    a, b = NBR[sw][0], NBR[sw][1]
    return f'\n[[harness.route]]\nswitch = {sw}\nprefix = "{pfx}"\nvia = {a}\nsplit = {b}\n'
CASES = {"on-ledger": (True, lambda t: split(t, "Ledger")), "on-zoom": (True, lambda t: split(t, "Netflix")),
         "off": (False, lambda t: ""), "reroute": (False, lambda t: fail(8, 9, 12000, 8000) + fail(3, 8, 12000, 8000))}
def run(item):
    (name, (want, f)), seed, load = item
    base = subprocess.run(["python3", "-I", str(ROOT / "worlds/local/gen_grid.py"), str(seed)], capture_output=True, text=True).stdout
    base = base.replace('preset = "steam"', 'preset = "netflix"\nrate_mbps = 3.0')
    extra = f(base)
    head, tail = base.split("[harness]", 1)
    text = head + "[harness]" + tail + extra.replace("[[plant]]", "[[plant]]") + CONF
    # harness.route rows must follow the [harness] table, plants after them is fine
    p = WORK / f"{name}-s{seed}-l{load}.toml"; p.write_text(text)
    team = p.with_suffix(".team")
    r = subprocess.run([SIM, "run-world", p, "--load-scale", str(load), "--program", sys.argv[1], "--team-log", team], capture_output=True, text=True)
    if r.returncode: return p.stem, -1, "ERR " + (r.stderr or r.stdout)[-160:], "true" if want else "false"
    a = subprocess.run([SIM, "detect", sys.argv[2], team], capture_output=True, text=True).stdout.strip()
    return p.stem, team.stat().st_size, a, "true" if want else "false"
items = [(c, s, l) for c in CASES.items() for s in (51, 52, 53) for l in (1.0, 1.5)]
with cf.ThreadPoolExecutor(8) as ex: rows = list(ex.map(run, items))
bad = sum(r[2] != r[3] for r in rows)
for r in rows: print(f"{r[0]:<22} {r[1]:>6}B said {r[2]:<6} want {r[3]}  {'ok' if r[2] == r[3] else 'WRONG'}")
print(f"\n{len(rows) - bad}/{len(rows)} right")

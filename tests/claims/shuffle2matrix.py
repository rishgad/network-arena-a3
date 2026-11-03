#!/usr/bin/env python3
"""Shuffle v2: split route over unequal paths (truth run 2's geometry:
Ledger at 9, its peer at 13, split at 9 via 8 and 14, 25 ms on 13-14).
    python3 -I tests/claims/shuffle2matrix.py SWITCH DETECT"""
import concurrent.futures as cf, re, subprocess, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
SIM = ROOT / "target/release/competitive_net_sim"
WORK = ROOT / "results/claims-shuffle2"; WORK.mkdir(parents=True, exist_ok=True)
fail = lambda a, b, t, d: f'\n[[plant]]\nkind = "fail"\na = {a}\nb = {b}\nat_ms = {t}\nfor_ms = {d}\n'
delay = lambda a, b, ab, ba: f'\n[[plant]]\nkind = "link_delay"\na = {a}\nb = {b}\na_to_b_us = {ab}\nb_to_a_us = {ba}\n'
CONF = fail(0, 1, 21930, 8160) + fail(10, 11, 21930, 8160) + delay(1, 2, 2000, 30000)
def build(seed, split_on, slow, alt=False):
    base = subprocess.run(["python3", "-I", str(ROOT / "worlds/local/gen_grid.py"), str(seed)], capture_output=True, text=True).stdout
    base = base.replace('preset = "steam"', 'preset = "netflix"\nrate_mbps = 3.0')
    blocks = base.split("[[app]]")
    i = next(i for i, b in enumerate(blocks) if 'name = "Ledger"\n' in b)
    blocks[i + 1] = re.sub(r"switch = \d+", "switch = 13", blocks[i + 1], count=1)
    pfx = re.search(r'prefix = "([^"]+)"', blocks[i + 1]).group(1)
    text = "[[app]]".join(blocks)
    extra = ""
    if split_on:
        extra += f'\n[[harness.route]]\nswitch = 9\nprefix = "{pfx}"\nvia = 8\nsplit = 14\n'
    if slow:
        extra += delay(13, 14, 25000, 25000) if not alt else delay(8, 13, 25000, 25000)
    return text + extra + CONF
CASES = {"on": (True, (True, True)), "on-slow-other-branch": (True, (True, True, True)),
         "split-equal": (False, (True, False)), "slow-no-split": (False, (False, True)), "off": (False, (False, False))}
def run(item):
    (name, (want, args)), seed, load = item
    p = WORK / f"{name}-s{seed}-l{load}.toml"; p.write_text(build(seed, *args))
    team = p.with_suffix(".team")
    r = subprocess.run([SIM, "run-world", p, "--load-scale", str(load), "--program", sys.argv[1], "--team-log", team], capture_output=True, text=True)
    if r.returncode: return p.stem, -1, "ERR " + (r.stderr or r.stdout)[-160:], "true" if want else "false"
    a = subprocess.run([SIM, "detect", sys.argv[2], team], capture_output=True, text=True).stdout.strip()
    return p.stem, team.stat().st_size, a, "true" if want else "false"
items = [(c, s, l) for c in CASES.items() for s in (61, 62) for l in (1.0, 1.5)]
with cf.ThreadPoolExecutor(8) as ex: rows = list(ex.map(run, items))
bad = sum(r[2] != r[3] for r in rows)
for r in rows: print(f"{r[0]:<30} {r[1]:>6}B said {r[2]:<6} want {r[3]}  {'ok' if r[2] == r[3] else 'WRONG'}")
print(f"\n{len(rows) - bad}/{len(rows)} right")

#!/usr/bin/env python3
"""Cloudflare DDoS: Cloudflare users send 100 B TCP at ~400 pkt/s to 8
Cloudflare hosts all run.  python3 -I tests/claims/cfmatrix.py SW DET"""
import concurrent.futures as cf, re, subprocess, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
SIM = ROOT / "target/release/competitive_net_sim"
WORK = ROOT / "results/claims-cf"; WORK.mkdir(parents=True, exist_ok=True)
def world(seed, how):
    t = subprocess.run(["python3", "-I", str(ROOT / "worlds/local/gen_grid.py"), str(seed)], capture_output=True, text=True).stdout
    t = t.replace('preset = "steam"', 'preset = "netflix"\nrate_mbps = 3.0')
    blocks = t.split("[[app]]")
    i = next(i for i, b in enumerate(blocks) if 'name = "Cloudflare users"\n' in b)
    rate = {"on": 0.32, "browse": 0.004, "udp": 0.32, "one-host": 0.32, "burst": 0.004, "off": None}[how]
    if rate is None:
        blocks[i] = blocks[i].replace('preset = "jane_street"', 'preset = "jane_street"\nproto = "tcp"')
        blocks[i] = re.sub(r"rate_mbps = [\d.]+", "rate_mbps = 0.004", blocks[i])
        blocks[i + 1] = blocks[i + 1].replace("Cloudflare users's peer", "Cloudflare")
    else:
        proto = "udp" if how == "udp" else "tcp"
        blocks[i] = blocks[i].replace('preset = "jane_street"', f'preset = "jane_street"\nproto = "{proto}"')
        blocks[i] = re.sub(r"rate_mbps = [\d.]+", f"rate_mbps = {rate}", blocks[i])
    if how != "one-host":
        blocks[i + 1] = re.sub(r"hosts = \[\d+\]", "hosts = [11, 23, 37, 52, 68, 81, 99, 140]", blocks[i + 1])
    text = "[[app]]".join(blocks)
    if how == "burst":
        aid = int(re.search(r"id = (\d+)", blocks[i]).group(1))
        text += f'\n[[plant]]\nkind = "side_flow"\napp = {aid}\nrate_pps = 400\nsize_bytes = 100\nstart_ms = 15000\nstop_ms = 20000\nproto = "tcp"\n'
    return text
WANT = {"on": True, "browse": False, "udp": False, "one-host": False, "burst": False, "off": False}
def run(item):
    how, seed, load = item
    p = WORK / f"{how}-s{seed}-l{load}.toml"; p.write_text(world(seed, how))
    team = p.with_suffix(".team")
    r = subprocess.run([SIM, "run-world", p, "--load-scale", str(load), "--program", sys.argv[1], "--team-log", team], capture_output=True, text=True)
    w = "true" if WANT[how] else "false"
    if r.returncode: return p.stem, -1, "ERR " + (r.stderr or r.stdout)[-160:], w
    return p.stem, team.stat().st_size, subprocess.run([SIM, "detect", sys.argv[2], team], capture_output=True, text=True).stdout.strip(), w
items = [(h, s, l) for h in WANT for s in (131, 132) for l in (1.0, 1.5)]
with cf.ThreadPoolExecutor(8) as ex: rows = list(ex.map(run, items))
bad = sum(r[2] != r[3] for r in rows)
for r in rows: print(f"{r[0]:<18} {r[1]:>6}B said {r[2]:<6} want {r[3]}  {'ok' if r[2] == r[3] else 'WRONG'}")
print(f"\n{len(rows) - bad}/{len(rows)} right")

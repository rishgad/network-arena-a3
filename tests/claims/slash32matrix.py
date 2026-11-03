#!/usr/bin/env python3
"""/32 hosts inside /24 ranges (Prime member in Amazon shoppers, Netflix live
in Netflix) on a calm grid proxy.  python3 -I tests/claims/slash32matrix.py SW PRIME=d LIVE=d"""
import concurrent.futures as cf, re, subprocess, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
SIM = ROOT / "target/release/competitive_net_sim"
WORK = ROOT / "results/claims-32"; WORK.mkdir(parents=True, exist_ok=True)
def block(text, name):
    return next(b for b in text.split("[[app]]")[1:] if f'name = "{name}"\n' in b and "preset" in b)
def base(seed, prime, live):
    t = subprocess.run(["python3", "-I", str(ROOT / "worlds/local/gen_grid.py"), str(seed)], capture_output=True, text=True).stdout
    t = t.replace('preset = "steam"', 'preset = "netflix"\nrate_mbps = 3.0')
    am = block(t, "Amazon (shoppers)"); amsw = re.search(r"switch = (\d+)", am).group(1); ampf = re.search(r'prefix = "(\d+\.\d+\.\d+)\.0/24"', am).group(1)
    nf = block(t, "Netflix"); nfsw = re.search(r"switch = (\d+)", nf).group(1); nfpf = re.search(r'prefix = "(\d+\.\d+\.\d+)\.0/24"', nf).group(1)
    extra = ""
    if prime:
        on = '' if prime == "steady" else 'on_ms = 8000\noff_ms = 8000\nstart_ms = 0\n'
        preset = 'preset = "netflix"\nrate_mbps = 0.4' if prime == "steady" else 'preset = "amazon"\nrate_mbps = 0.4'
        extra += f'''
[[app]]
id = 980
switch = {amsw}
prefix = "{ampf}.200/30"
hosts = [1]

name = "Amazon (Prime member)"
{preset}
size_bytes = 200
{on}peer = 981

[[app]]
id = 981
switch = 8
prefix = "10.252.1.0/24"
hosts = [70]
name = "Amazon (Prime member)'s peer"
'''
    if live:
        sched = {"live": 'preset = "amazon"\nrate_mbps = 2.4\non_ms = 3000\noff_ms = 1000\nstart_ms = 0\n',
                 "steady": 'preset = "netflix"\nrate_mbps = 1.8\n',
                 "one-gap": 'preset = "netflix"\nrate_mbps = 1.8\n'}[live]
        extra += f'''
[[app]]
id = 982
switch = {nfsw}
prefix = "{nfpf}.200/30"
hosts = [1]

name = "Netflix live"
{sched}peer = 983

[[app]]
id = 983
switch = 8
prefix = "10.252.2.0/24"
hosts = {list(range(10, 22))}
name = "Netflix live's viewers"
'''
        if live == "one-gap":
            extra += '\n[[plant]]\nkind = "pause"\napp = 982\nat_ms = 15000\nfor_ms = 3000\n'
    head, tail = t.split("[workload]", 1)
    return head + extra + "\n[workload]" + tail
CASES = {  # name: (prime, live, want PRIME, want LIVE)
    "both-on": ("steady", "live", True, True),
    "prime-synced": ("synced", "live", False, True),
    "no-prime": (None, "live", False, True),
    "live-steady": ("steady", "steady", True, False),
    "live-one-gap": ("steady", "one-gap", True, False),
    "no-live": ("steady", None, True, False),
}
def run(item):
    (name, (pr, lv, wp, wl)), seed, load, sw, dets = item
    p = WORK / f"{name}-s{seed}-l{load}.toml"; p.write_text(base(seed, pr, lv))
    team = p.with_suffix(".team")
    r = subprocess.run([SIM, "run-world", p, "--load-scale", str(load), "--program", sw, "--team-log", team], capture_output=True, text=True)
    want = {"PRIME": "true" if wp else "false", "LIVE": "true" if wl else "false"}
    if r.returncode: return p.stem, -1, {"err": (r.stderr or r.stdout)[-200:]}, want
    return p.stem, team.stat().st_size, {k: subprocess.run([SIM, "detect", d, team], capture_output=True, text=True).stdout.strip() for k, d in dets.items()}, want
sw = sys.argv[1]; dets = dict(a.split("=", 1) for a in sys.argv[2:])
items = [(c, s, l, sw, dets) for c in CASES.items() for s in (121, 122) for l in (1.0, 1.5)]
with cf.ThreadPoolExecutor(8) as ex: rows = list(ex.map(run, items))
bad = 0
for name, size, ans, want in rows:
    wrong = [k for k in dets if ans.get(k) != want[k]]
    bad += bool(wrong)
    print(f"{name:<22} {size:>6}B " + " ".join(f"{k}={v}" for k, v in ans.items()) + ("  WRONG " + ",".join(wrong) if wrong else "  ok"))
print(f"\n{len(rows) - bad}/{len(rows)} right")

#!/usr/bin/env python3
"""Claim matrix for the customer eggs DUP (Vault echoes), TTL (Mapper's
low TTLs) and FLOOD (Home users at 30x) on a calm grid proxy. Every world
has the other two eggs on (confounders); the target is on, off or a near miss.
    python3 -I tests/claims/custmatrix.py SWITCH.wasm DUP=d.wasm TTL=d.wasm FLOOD=d.wasm
"""
import concurrent.futures as cf, re, subprocess, sys
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
SIM = ROOT / "target/release/competitive_net_sim"
WORK = ROOT / "results/claims-ddos"

def app_id(text, name):
    for block in text.split("[[app]]")[1:]:
        if f'name = "{name}"\n' in block:
            return int(re.search(r"id = (\d+)", block).group(1))
    raise KeyError(name)

def set_on(text, name, line):
    return re.sub(r'(name = "' + re.escape(name) + r'"\n)', r'\1' + line + "\n", text, count=1)

def flood(text, name, start, dur, pps=2000, proto="udp"):
    a = app_id(text, name)
    return f'\n[[plant]]\nkind = "side_flow"\napp = {a}\nrate_pps = {pps}\nsize_bytes = 64\nstart_ms = {start}\nstop_ms = {start + dur}\nproto = "{proto}"\n'

def apply(text, egg, how):
    if egg == "DUP":
        return {"on": set_on(text, "Vault", "echo = true"), "off": text,
                "other-cust": set_on(text, "Netflix", "echo = true")}[how], ""
    if egg == "TTL":
        return {"on": set_on(text, "Mapper", "ttl_ladder = 8"), "off": text,
                "const8": set_on(text, "Mapper", "ttl_start = 8"),
                "const4": set_on(text, "Mapper", "ttl_start = 4"),
                "ladder-ping": set_on(text, "Ping", "ttl_ladder = 8")}[how], ""
    if egg == "FLOOD":
        return text, {"on": flood(text, "Home users", 15000, 15000), "off": "",
                      "flashcrowd": flood(text, "Home users", 15000, 15000, proto="tcp"),
                      "short": flood(text, "Home users", 15000, 3000),
                      "small": flood(text, "Home users", 15000, 15000, pps=150),
                      "weaker": flood(text, "Home users", 15000, 7000, pps=700),
                      "long": flood(text, "Home users", 5000, 30000),
                      "atstart": flood(text, "Home users", 0, 15000),
                      "steamsurge": flood(text, "Netflix", 15000, 5000, pps=1200, proto="tcp"),
                      "other-udp-flood": flood(text, "Netflix", 15000, 15000),
                      "login-udp-flood": flood(text, "Login", 15000, 15000),
                      "one-host": flood(text, "Home users", 15000, 15000),
                      "one-host-real": f'\n[[plant]]\nkind = "scan"\napp = {app_id(text, "Home users")}\nstart_ms = 15000\nfor_ms = 15000\naddresses = 1\neach = 30000\nsize_bytes = 64\n'}[how]

CASES = {"DUP": ["on", "off", "other-cust"], "TTL": ["on", "off", "const8", "const4", "ladder-ping"], "FLOOD": ["on", "off", "flashcrowd", "short", "small", "weaker", "long", "atstart", "steamsurge", "other-udp-flood", "login-udp-flood", "one-host", "one-host-real"]}

def world(target, how, seed):
    base = subprocess.run(["python3", "-I", str(ROOT / "worlds/local/gen_grid.py"), str(seed)], capture_output=True, text=True).stdout
    base = base.replace('preset = "steam"', 'preset = "netflix"\nrate_mbps = 3.0')
    if 'name = "Home users"\nproto' not in base:
        base = set_on(base, "Home users", 'proto = "tcp"')  # TCP, as on the real network
        # Home users send from many hosts (8 distinct in 9 samples on the real network)
        base = re.sub(r'(hosts = \[\d+\]\nname = "Home users"\n)', 'hosts = 30\nname = "Home users"\n', base)
        base = set_on(base, "Login", 'proto = "tcp"')
    if how == "one-host" and target == "FLOOD":
        base = re.sub(r'hosts = 30\nname = "Home users"\n', 'hosts = [77]\nname = "Home users"\n', base)
    extra = ""
    for e in CASES:
        t, x = apply(base, e, how if e == target else "on")
        base, extra = t, extra + x
    return base + extra

def run(item):
    target, how, seed, load, sw, dets = item
    p = WORK / f"{target}-{how}-s{seed}-l{load}.toml"; p.write_text(world(target, how, seed))
    team = p.with_suffix(".team")
    r = subprocess.run([SIM, "run-world", p, "--load-scale", str(load), "--program", sw, "--team-log", team], capture_output=True, text=True)
    if r.returncode:
        return p.stem, -1, {"err": (r.stderr or r.stdout)[-200:]}
    return p.stem, team.stat().st_size, {k: subprocess.run([SIM, "detect", d, team], capture_output=True, text=True).stdout.strip() for k, d in dets.items()}

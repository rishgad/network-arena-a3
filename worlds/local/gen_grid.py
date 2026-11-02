#!/usr/bin/env python3
"""A 3x5 grid proxy of the hidden network (topology inferred from the
supplied logs; customers placed where their tag-2 counters were seen).
A budget/load proxy, NOT a replica: rates, presets and peers are guesses.

    python3 -I worlds/local/gen_grid.py SEED [LOAD_NOTE] > world.toml
Extra [[plant]] rows can be appended by the caller.
"""
import random, sys
seed = int(sys.argv[1]) if len(sys.argv) > 1 else 1
rng = random.Random(seed)
links = [(r*5+c, r*5+c+1) for r in range(3) for c in range(4)] + [(r*5+c, (r+1)*5+c) for r in range(2) for c in range(5)]
# (switch, name, preset, rate, extra)
senders = [
    (0, "Zoom", "fortnite", 0.4, {"size_bytes": 200}), (10, "Zoom", "fortnite", 0.4, {"size_bytes": 200}),
    (1, "Offsite", "netflix", 2.0, {"law": "aimd"}), (1, "Ping", "ping", 0.01, {"size_bytes": 60}),
    (1, "Sync", "amazon", 0.8, {"on_ms": 2000, "off_ms": 3000, "start_ms": 20000}),
    (1, "Sync", "amazon", 0.8, {"on_ms": 2000, "off_ms": 3000, "start_ms": 20000}),
    (2, "Akamai users", "jane_street", 0.02, {"size_bytes": 200}), (2, "Cloudflare users", "jane_street", 0.3, {"size_bytes": 100}),
    (2, "Fortnite", "fortnite", 0.35, {"size_bytes": 200}), (2, "Home users", "jane_street", 0.12, {"size_bytes": 200}),
    (2, "Jane Street", "jane_street", 0.12, {"size_bytes": 200, "hosts": 3}),
    (3, "Netflix", "netflix", 2.7, {"law": "aimd"}), (3, "Vault", "netflix", 0.45, {"size_bytes": 500}),
    (4, "Fortnite", "fortnite", 0.35, {"size_bytes": 200}), (4, "Mapper", "ping", 0.01, {"size_bytes": 60}),
    (6, "Cam-A", "amazon", 1.25, {"on_ms": 200, "off_ms": 700, "start_ms": 0}),
    (6, "Cam-B", "amazon", 1.0, {"on_ms": 200, "off_ms": 700, "start_ms": 0}),
    (7, "Amazon (shoppers)", "amazon", 1.0, {"on_ms": 8000, "off_ms": 8000, "start_ms": 0}),
    (7, "Amazon (shoppers)", "amazon", 1.0, {"on_ms": 8000, "off_ms": 8000, "start_ms": 4000}),
    (7, "Login", "jane_street", 0.02, {"size_bytes": 100}), (7, "Steam", "steam", None, {}),
    (9, "Ledger", "netflix", 0.85, {}), (11, "Dropbox", "amazon", 2.5, {"on_ms": 1000, "off_ms": 5000, "start_ms": 500}),
    (11, "Dropbox", "amazon", 2.5, {"on_ms": 1000, "off_ms": 5000, "start_ms": 500}),
    (12, "Backup", "steam", None, {}), (12, "Dropbox-2", "amazon", 1.1, {"on_ms": 1000, "off_ms": 5000, "start_ms": 2500}),
]
print(f'''[world]
name = "grid-proxy-s{seed}"
seed = {seed}
version = 1
family = "harness"

[params]
link_capacity_mbps = 10
link_latency_us = 500
queue_capacity_bytes = 40000
warmup_ms = 3000
duration_ms = 40000
delivery_floor_pct = 0.0
packet_size_bytes = 1000
control_delay_us = 2000
''')
for s in range(15):
    print(f"[[switch]]\nid = {s}\n")
for a, b in links:
    print(f"[[link]]\na = {a}\nb = {b}\n")
used = set()
def prefix():
    while True:
        p = (10, rng.randrange(1, 63), rng.randrange(0, 256))
        if p not in used:
            used.add(p); return "%d.%d.%d.0/24" % p
aid = 100
for sw, name, preset, rate, extra in senders:
    peer_sw = rng.choice([x for x in range(15) if x != sw])
    h = extra.pop("hosts", 1)
    hosts = sorted(rng.sample(range(2, 250), h))
    rows = [f"id = {aid}", f"switch = {sw}", f'prefix = "{prefix()}"', f"hosts = {hosts}", f'name = "{name}"',
            f'preset = "{preset}"', f"peer = {aid+1}"]
    if rate:
        rows.append(f"rate_mbps = {rate * rng.uniform(0.9, 1.1):.2f}")
    for k, v in extra.items():
        rows.append(f'{k} = "{v}"' if isinstance(v, str) else f"{k} = {v}")
    print("[[app]]\n" + "\n".join(rows) + "\n")
    print(f'[[app]]\nid = {aid+1}\nswitch = {peer_sw}\nprefix = "{prefix()}"\nhosts = [{rng.randrange(2,250)}]\nname = "{name}\'s peer"\n')
    aid += 2
print('''[workload]
kind = "mixed"
size_bytes = 1000

[observe]
punt_events_per_s = 80
punt_backlog = 64
export_bytes_per_s = 16000
export_burst_bytes = 8000

[harness]
keepalive_ms = 100
log_cap_bytes = 65536
''')

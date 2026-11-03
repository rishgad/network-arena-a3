#!/usr/bin/env python3
"""End-to-end many-to-one DDoS matrix; preserves the existing matrix and kit.

Reuses only its world-generation helpers, never its detector or verdicts.
"""
import argparse
import concurrent.futures as cf
import hashlib
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def helpers():
    path = ROOT / 'tests/claims/ddos_world_helpers.py'
    ns = {'__file__': str(path), '__name__': 'world_helpers'}
    text = path.read_text()
    exec(compile(text, str(path), 'exec'), ns)
    return ns


H = None


def replace_hosts(text, name, hosts):
    return re.sub(r'hosts = [^\n]+\nname = "' + re.escape(name) + r'"',
                  f'hosts = {hosts}\nname = "{name}"', text)


CASES = {
    'on': True, 'weaker': True, 'long': True, 'atstart': True,
    'four-hosts': True, 'eight-hosts': True, 'moved-source': True,
    'off': False, 'flashcrowd': False, 'short': False, 'small': False,
    'steamsurge': False, 'other-udp-flood': False, 'login-udp-flood': False,
    'one-host': False, 'one-host-real': False, 'many-victims': False,
}


def world(seed, case):
    base_case = case if case in H['CASES']['FLOOD'] else 'off'
    text = H['world']('FLOOD', base_case, seed)
    if case in ('four-hosts', 'eight-hosts', 'moved-source'):
        count = 4 if case == 'four-hosts' else 8 if case == 'eight-hosts' else 30
        text = replace_hosts(text, 'Home users', str(list(range(11, 11 + count))))
        text += H['flood'](text, 'Home users', 12000, 16000)
        if case == 'moved-source':
            blocks = text.split('[[app]]')
            for i, block in enumerate(blocks):
                if 'name = "Home users"\n' in block:
                    blocks[i] = re.sub(r'switch = \d+', 'switch = 9', block)
            text = '[[app]]'.join(blocks)
    if case == 'many-victims':
        # Same total UDP load, same source pool, spread equally over 16 victims.
        apps, plants = [], []
        sender = H['app_id'](text, 'Home users')
        for i in range(16):
            aid = 9500 + i
            apps.append(f'[[app]]\nid = {aid}\nswitch = 7\nprefix = "10.250.{i}.0/24"\nhosts = [8]\nname = "Telemetry-{i}"\n')
            plants.append(f'[[plant]]\nkind = "side_flow"\napp = {sender}\nto = {aid}\nrate_pps = 125\nsize_bytes = 64\nstart_ms = 15000\nstop_ms = 30000\nproto = "udp"\n')
        cut = text.index('[workload]')
        text = text[:cut] + '\n'.join(apps) + '\n' + text[cut:] + '\n' + '\n'.join(plants)
    # Test official claim cap, not the proxy's 64 KB truth cap.
    text = text.replace('log_cap_bytes = 65536', 'log_cap_bytes = 24576')
    return text


def run(args, item):
    seed, load, case = item
    stem = f'{case}-s{seed}-l{load}'
    p = args.output / (stem + '.toml')
    text = world(seed, case)
    if args.network_confounders:
        def outage(a, b, at, dur):
            return f'\n[[plant]]\nkind = "fail"\na = {a}\nb = {b}\nat_ms = {at}\nfor_ms = {dur}\n'
        text += ''.join(outage(1, 6, 5900 + k * 4000, 1600) for k in range(4))
        text += outage(0, 1, 21930, 8160) + outage(10, 11, 21930, 8160)
        text += ''.join(outage(a, b, 30900, 5100) for a, b in [(0, 5), (5, 6), (5, 10)])
        text += '\n[[plant]]\nkind = "link_delay"\na = 1\nb = 2\na_to_b_us = 2000\nb_to_a_us = 30000\n'
        text += '\n[[plant]]\nkind = "link_delay"\na = 13\nb = 14\na_to_b_us = 25000\nb_to_a_us = 25000\n'
        text += '\n[[harness.queue]]\nswitch = 6\ntoward = 7\npackets = 1\n'
    p.write_text(text)
    log = p.with_suffix('.team')
    proc = subprocess.run([str(ROOT / 'target/release/competitive_net_sim'), 'run-world', str(p), '--load-scale', str(load), '--program', str(args.switch), '--team-log', str(log)], capture_output=True, text=True, timeout=120)
    output = proc.stdout + proc.stderr
    p.with_suffix('.stdout.txt').write_text(output)
    row = dict(case=case, seed=seed, load=load, expected=CASES[case], stopped='stopped' in output, simulation_exit=proc.returncode, bytes=log.stat().st_size if log.exists() else 0)
    if proc.returncode or row['stopped'] or not row['bytes']:
        row.update(passed=False, error=output[-300:])
        return row
    det = subprocess.run([str(ROOT / 'target/release/competitive_net_sim'), 'detect', str(args.detect), str(log)], capture_output=True, text=True, timeout=30)
    row.update(detect_exit=det.returncode, answer=det.stdout.strip(), passed=det.returncode == 0 and det.stdout.strip() == str(CASES[case]).lower() and row['bytes'] < 24576)
    return row


def main():
    global H
    parser = argparse.ArgumentParser()
    parser.add_argument('switch', type=Path)
    parser.add_argument('detect', type=Path)
    parser.add_argument('--network-confounders', action='store_true')
    parser.add_argument('--seeds', default='91,92')
    parser.add_argument('--loads', default='1.0,1.5')
    parser.add_argument('--cases', default=','.join(CASES))
    parser.add_argument('--output', type=Path, default=ROOT / 'results/claims-ddos-final')
    args = parser.parse_args()
    args.switch, args.detect = args.switch.resolve(), args.detect.resolve()
    args.output.mkdir(parents=True, exist_ok=True)
    H = helpers()
    items = [(int(s), float(l), c) for s in args.seeds.split(',') for l in args.loads.split(',') for c in args.cases.split(',')]
    with cf.ThreadPoolExecutor(max_workers=4) as ex:
        rows = list(ex.map(lambda x: run(args, x), items))
    for row in rows:
        if not row['passed']:
            print('FAIL', json.dumps(row), flush=True)
    result = {'network_confounders': args.network_confounders, 'switch_sha256': hashlib.sha256(args.switch.read_bytes()).hexdigest(), 'detect_sha256': hashlib.sha256(args.detect.read_bytes()).hexdigest(), 'passed': sum(r['passed'] for r in rows), 'total': len(rows), 'max_bytes': max(r['bytes'] for r in rows), 'rows': rows}
    (args.output / 'summary.json').write_text(json.dumps(result, indent=2) + '\n')
    print(f"{result['passed']}/{len(rows)} passed; max {result['max_bytes']} B", flush=True)
    return 0 if result['passed'] == len(rows) else 1


if __name__ == '__main__':
    raise SystemExit(main())

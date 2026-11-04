#!/usr/bin/env python3
"""Test a conjunction with other candidate patterns held ON for each target.

This is justified only by handout Section 2: other eggs stay switched on.
It does not establish that any hypothesized pattern is a hidden egg.
"""
import argparse
import concurrent.futures
import hashlib
import json
import re
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
ns={'__file__':str(ROOT/'tests/claims/ddos_final_matrix.py'),'__name__':'fixture_helpers'}
exec(compile(Path(ns['__file__']).read_text(),ns['__file__'],'exec'),ns)
ns['H']=ns['helpers']()
base_world=ns['world']
CASES={'all-on':True,'all-shifted':True,'steam-late':True,'live-shifted':True,
       'ddos-four-hosts':True,'ddos-eight-hosts':True,'ddos-weaker':True,
       'ddos-moved-source':True,'ddos-atstart':True,'ddos-long':True,
       'ddos-off':False,'ddos-one-host':False,'ddos-many-victims':False,
       'ddos-flashcrowd':False,'ddos-short':False,
       'steam-off':False,'steam-one':False,'steam-half-speed':False,
       'steam-other':False,'steam-brief':False,
       'live-steady':False,'live-one-gap':False,'live-one-viewer':False,
       'live-four-viewers':False,'live-few-pauses':False,
       'live-moved':True,'live-renumbered':True,
       'live-small-packets':False,'live-udp':False}


def world(seed,case):
    dc=case[5:] if case.startswith('ddos-') else 'on'
    t=base_world(seed,dc)
    blocks=t.split('[[app]]')
    for i,b in enumerate(blocks):
        if 'name = "Steam"\n' in b:
            blocks[i]=re.sub(r'rate_mbps = [^\n]+','rate_mbps = 0.5',b)
        # Match the measured low-rate Sync pair; the old grid used much higher asks.
        elif 'name = "Sync"\n' in b and 'preset' in b:
            b=re.sub(r'rate_mbps = [^\n]+','rate_mbps = 0.36',b)
            b=b.replace('on_ms = 2000','on_ms = 1000')
            blocks[i]=b
        elif any(f'name = "{n}"\n' in b for n in ('Cam-A','Cam-B')) and 'preset' in b:
            # The original grid makes these sparse 200 ms / 900 ms apps.
            # Measured cameras instead have steady 125+ pps backgrounds.
            b=b.replace('preset = "amazon"','preset = "netflix"')
            b=re.sub(r'rate_mbps = [^\n]+','rate_mbps = 1.22' if 'name = "Cam-A"' in b else 'rate_mbps = 1.0',b)
            blocks[i]=b
    t='[[app]]'.join(blocks)
    H=ns['H']; aid=H['app_id'](t,'Steam')
    steam_app=H['app_id'](t,'Netflix') if case=='steam-other' else aid
    spans=[(12000,4000),(28000,4000)]
    if case=='all-shifted':spans=[(6000,4000),(22000,4000)]
    if case=='steam-late':spans=[(21000,4000),(37000,4000)]
    if case=='steam-off':spans=[]
    if case=='steam-one':spans=spans[:1]
    if case=='steam-brief':spans=[(12000,1000),(28000,1000)]
    pps=500 if case=='steam-half-speed' else 1300
    plants=''.join(f'\n[[plant]]\nkind = "side_flow"\napp = {steam_app}\nrate_pps = {pps}\nsize_bytes = 1000\nstart_ms = {s}\nstop_ms = {s+d}\nproto = "tcp"\n' for s,d in spans)
    # Synchronized brief camera bursts as a confounder, not another claim.
    for name in ('Cam-A','Cam-B'):
        a=H['app_id'](t,name)
        for start_ms in range(1000,39000,1000):
            plants+=f'\n[[plant]]\nkind = "side_flow"\napp = {a}\nrate_pps = 2500\nsize_bytes = 1000\nstart_ms = {start_ms}\nstop_ms = {start_ms+20}\nproto = "tcp"\n'
    hosts=1 if case=='live-one-viewer' else 4 if case=='live-four-viewers' else 12
    start=6000 if case in ('all-shifted','live-shifted') else 0
    sched=f'preset = "amazon"\nrate_mbps = 1.872\non_ms = 2000\noff_ms = 2000\nstart_ms = {start}\n'
    if case in ('live-steady','live-one-gap'):
        sched='preset = "netflix"\nrate_mbps = 1.872\n'
    if case=='live-one-gap':plants+='\n[[plant]]\nkind = "pause"\napp = 980\nat_ms = 15000\nfor_ms = 2000\n'
    if case=='live-few-pauses':sched='preset = "amazon"\nrate_mbps = 1.872\non_ms = 9000\noff_ms = 1000\nstart_ms = 0\n'
    source_switch=9 if case=='live-moved' else 3
    source_prefix=f'10.251.{seed%200}.0/24' if case=='live-renumbered' else '10.250.240.0/24'
    receiver_prefix=f'10.252.{seed%200}.0/24' if case=='live-renumbered' else '10.250.241.0/24'
    size=100 if case=='live-small-packets' else 1000
    proto='proto = "udp"\n' if case=='live-udp' else ''
    extra=f'''\n[[app]]
id = 980
switch = {source_switch}
prefix = "{source_prefix}"
hosts = [200]
name = "Live audit sender"
{sched}{proto}size_bytes = {size}
peer = 981

[[app]]
id = 981
switch = 8
prefix = "{receiver_prefix}"
hosts = {list(range(10,10+hosts))}
name = "Live audit viewers"
'''
    cut=t.index('[workload]')
    return t[:cut]+extra+'\n'+t[cut:]+plants


def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('switch',type=Path);ap.add_argument('detect',type=Path)
    ap.add_argument('--output',type=Path,required=True)
    ap.add_argument('--seeds',default='94,95');ap.add_argument('--loads',default='1.0,1.25,1.5')
    ap.add_argument('--cases',default=','.join(CASES))
    ap.add_argument('--network-confounders',action='store_true')
    args=ap.parse_args();args.switch=args.switch.resolve();args.detect=args.detect.resolve()
    args.output.mkdir(parents=True,exist_ok=True)
    ns['CASES']=CASES;ns['world']=world
    items=[(int(s),float(l),c) for s in args.seeds.split(',') for l in args.loads.split(',') for c in args.cases.split(',')]
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as ex:
        rows=list(ex.map(lambda item:ns['run'](args,item),items))
    for r in rows:
        if not r['passed']:print('FAIL',json.dumps(r),flush=True)
    summary=dict(passed=sum(r['passed'] for r in rows),total=len(rows),max_bytes=max(r['bytes'] for r in rows),
        switch_sha256=hashlib.sha256(args.switch.read_bytes()).hexdigest(),
        detect_sha256=hashlib.sha256(args.detect.read_bytes()).hexdigest(),rows=rows)
    (args.output/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
    print(f"{summary['passed']}/{summary['total']} passed; max {summary['max_bytes']} B",flush=True)
    return int(summary['passed']!=summary['total'])


if __name__=='__main__':raise SystemExit(main())

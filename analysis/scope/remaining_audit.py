#!/usr/bin/env python3
"""Offline audit of all seven supplied truth logs. Never a claim detector.

Uses customer names only for offline attribution. The other team's raw data
stays in evidence and is not copied into this report. Activity detail rows
overlap main rows and are fitted separately, never added to the main totals.
"""
import argparse
import collections
import hashlib
import json
import math
import statistics
import sys
from pathlib import Path


def pulse_integral(t, start, on, period):
    x = max(0.0, t - start)
    cycles = math.floor(x / period)
    return cycles * on + min(x - cycles * period, on)


def pulse_fit(rows, start, on, period):
    x = [pulse_integral(m['end_ns']/1e9, start, on, period)
         - pulse_integral(m['start_ns']/1e9, start, on, period) for m in rows]
    y = [m['packets'] for m in rows]
    rate = sum(a*b for a, b in zip(x, y)) / max(1e-12, sum(a*a for a in x))
    err = sum((a*rate-b)**2 for a, b in zip(x, y))
    return dict(start=start, on=on, off=period-on, rate_pps=round(rate, 4),
                relative_rms=round(math.sqrt(err/max(1, sum(b*b for b in y))), 6))


def two_rate_fit(rows, start, on, period):
    """Fit base traffic plus a periodic excess, accounting for straddled bins."""
    durations=[(m['end_ns']-m['start_ns'])/1e9 for m in rows]
    active=[pulse_integral(m['end_ns']/1e9,start,on,period)
            -pulse_integral(m['start_ns']/1e9,start,on,period) for m in rows]
    y=[m['packets'] for m in rows]
    aa=sum(x*x for x in durations); ab=sum(a*b for a,b in zip(durations,active))
    bb=sum(x*x for x in active); ay=sum(a*b for a,b in zip(durations,y)); by=sum(a*b for a,b in zip(active,y))
    det=aa*bb-ab*ab
    quiet=(ay*bb-by*ab)/det; excess=(by*aa-ay*ab)/det
    err=sum((quiet*a+excess*b-n)**2 for a,b,n in zip(durations,active,y))
    return dict(start=start,on=on,off=period-on,quiet_pps=round(quiet,4),
                active_pps=round(quiet+excess,4),relative_rms=round(math.sqrt(err/max(1,sum(n*n for n in y))),6))


def source_rows(records, customer):
    rows = [(o,m) for t,o,m in records if t == 'Activity'
            and m['customer'] == customer and not m['detail']]
    if not rows:
        return None, []
    origin = max({o for o,m in rows}, key=lambda o: sum(m['packets'] for oo,m in rows if oo == o))
    return origin, sorted((m for o,m in rows if o == origin), key=lambda m:m['end_ns'])


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[2])
    ap.add_argument('--output', type=Path, required=True)
    args = ap.parse_args()
    root = args.root.resolve()
    sys.path.insert(0, str(root))
    from analysis.scope.scopelog import parse, namer
    from analysis.evidence.tlv import frame
    from analysis.custmap import CustomerMap, int_to_ip
    manifest = []
    def read(path):
        b = (root/path).read_bytes()
        manifest.append(dict(path=str(path), bytes=len(b), sha256=hashlib.sha256(b).hexdigest()))
        return b
    out = dict(scope={}, supplied_tlv={}, other_team={}, input_manifest=manifest)
    for k in range(1,4):
        path = Path(f'submissions/truth{k}/team.log')
        log = parse(read(path))
        read(path.parent/'customers.txt')
        cm = CustomerMap.load(root/path.parent/'customers.txt')
        name = namer(log, cm)
        table = []
        for idx in log.customers:
            ws = [(w,c) for w in log.windows for c in w['cust'] if c['idx'] == idx]
            if not ws:
                continue
            origin = max({w['sw'] for w,c in ws}, key=lambda s:max(c['pkts'] for w,c in ws if w['sw']==s))
            rows = sorted([(w,c) for w,c in ws if w['sw']==origin], key=lambda z:z[0]['t'])
            intervals = []
            previous_t, previous_n = 0, 0
            for w,c in rows:
                intervals.append(dict(start_ms=previous_t, end_ms=w['t'], packets=c['pkts']-previous_n,
                                      distinct_destinations_in_exported_window=c['new']))
                previous_t, previous_n = w['t'], c['pkts']
            table.append(dict(customer=name(idx), origin=origin, observations=len(rows),
                              last_ms=previous_t, cumulative_packets=previous_n, intervals=intervals))
        out['scope'][f'truth{k}'] = dict(bytes=log.size, errors=log.errors,
                trailing=log.trailing, samples=len(log.samples), customers=table)
    for k in (1,2):
        path=Path(f'evidence/provided-0{k}/team.log')
        rows, broken=frame(read(path))
        out['supplied_tlv'][f'provided-0{k}'] = dict(records=len(rows), framing_broken_at=broken,
                    tag_counts=dict(collections.Counter(r.tag for r in rows)),
                    limitation='Payload schema is inferred. No new binary-field meanings assumed.')
    base=Path('evidence/other-team-2026-11-03/a3-our-logs-2026-11-03')
    read(base/'customers.txt')
    cmap=CustomerMap.load(root/base/'customers.txt')
    for run in ('run-A','run-B'):
        data=json.loads(read(base/run/'decoded.json'))
        read(base/run/'team.log')
        records=[]
        for r in data['records']:
            (kind,m),=r['measurement'].items()
            records.append((kind,r['origin'],m))
        prefs=next(m['customers'] for t,o,m in records if t=='Run')
        def idx(ip):
            hits=[(p['len'],i) for i,p in enumerate(prefs,1)
                  if ip>>(32-p['len']) == p['addr']>>(32-p['len'])]
            return max(hits,default=(0,0))[1]
        packets={}
        for t,o,m in records:
            if t=='Packet' and m['kind']==0 and m['size']>0 and m['ingress']>=100:
                key=(o,m['at_ns'],m['src'],m['dst'],m['seq'],m['size'])
                packets[key]=(o,m)
        table=[]
        for c,prefix in enumerate(prefs,1):
            origin,rows=source_rows(records,c)
            if not rows:
                continue
            label=cmap.label(prefix['addr'])
            ps=[m for o,m in packets.values() if o==origin and idx(m['src'])==c]
            senders_by_destination=collections.defaultdict(set)
            destinations_by_sender=collections.defaultdict(set)
            for m in ps:
                senders_by_destination[m['dst']].add(m['src'])
                destinations_by_sender[m['src']].add(m['dst'])
            main_packets=sum(m['packets'] for m in rows)
            destinations=collections.Counter(cmap.label(m['dst']) for m in ps)
            details=[m for t,o,m in records if t=='Activity' and o==origin and m['customer']==c and m['detail']]
            receiver_switches=sorted({o for t,o,m in records if t=='Pairs' for r in m['cells'] if r[0]==c and r[2]>0})
            rec=dict(customer=label, index=c, origin=origin, main_windows=len(rows),
                main_packets=main_packets, mean_bytes=round(sum(m['bytes'] for m in rows)/max(1,main_packets),3),
                udp_packets=sum(m['udp'] for m in rows), local_samples=len(ps),
                sampled_source_hosts=len({m['src'] for m in ps}),
                sampled_destination_hosts=len({m['dst'] for m in ps}),
                max_sampled_sources_per_destination=max(map(len,senders_by_destination.values()),default=0),
                destinations_per_sender_histogram=dict(collections.Counter(map(len,destinations_by_sender.values()))),
                sampled_destinations=dict(destinations), receiver_switches=receiver_switches)
            if label.startswith('Netflix live '):
                rec['schedule_fits']=[pulse_fit(details,0,on,4) for on in (2,3)]
            elif label.startswith('Sync '):
                rec['schedule_fit']=pulse_fit(details,20,1,4)
            elif label.startswith('Dropbox '):
                rec['schedule_fit']=pulse_fit(details,.5,1,6)
            elif label.startswith('Dropbox-2 '):
                rec['schedule_fit']=pulse_fit(details,2.5,1,6)
            elif label.startswith('Steam '):
                rec['schedule_fit']=two_rate_fit(rows,12,4,16)
            if label.startswith('Akamai users '):
                rec['request_samples']=[dict(time_s=round(m['at_ns']/1e9,6),seq=m['seq'],
                    src=int_to_ip(m['src']),dst=int_to_ip(m['dst'])) for m in sorted(ps,key=lambda x:x['at_ns'])]
            if label.startswith('Home users '):
                flood=[m for m in ps if m['protocol']==17 and m['size']<=128]
                victims=collections.Counter(m['dst'] for m in flood)
                rec['flood_samples']=dict(count=len(flood),source_hosts=len({m['src'] for m in flood}),
                    destination_hosts=len(victims),dominant_share=round(max(victims.values(),default=0)/max(1,len(flood)),4),
                    first_time_s=round(min(m['at_ns'] for m in flood)/1e9,6),
                    last_time_s=round(max(m['at_ns'] for m in flood)/1e9,6))
                udp_rows=[dict(m,packets=m['udp']) for m in rows]
                rec['udp_schedule_fit']=pulse_fit(udp_rows,15,15,100)
            table.append(rec)
        out['other_team'][run]=dict(record_counts=dict(collections.Counter(t for t,o,m in records)),
                    quality=[m for t,o,m in records if t=='Quality'],customers=table)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(out,indent=2)+'\n')
    print('Wrote',args.output)
    for run,d in out['other_team'].items():
        print(run)
        for c in d['customers']:
            if 'schedule_fit' in c or 'schedule_fits' in c or 'flood_samples' in c or 'request_samples' in c:
                print(c['customer'],{k:v for k,v in c.items() if k in ('schedule_fit','schedule_fits','flood_samples','sampled_destination_hosts')})


if __name__=='__main__':
    main()

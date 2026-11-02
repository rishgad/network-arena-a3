//! Ground truth from a full simulator log (`--log run.simlog`), for checking
//! what a team log caught. Local practice only: the server never returns a
//! full log, and nothing here may become a detector input.
//!
//!     tracecheck RUN.simlog > truth.json
//!
//! Emits one JSON object:
//! - `reports`: controller-made packets by origin switch: made (first
//!   egress), refused at a port (report queue full / dead link), dropped
//!   elsewhere (with reason), refused by the export budget or size cap;
//! - `team_log`: bytes written, writes refused (full / not logger);
//! - `links`: failure and restore times by link id;
//! - `customers`: name, prefix, and data packets entering the network per
//!   100 ms from that customer's range (first switch, from an app port);
//! - `samples_lost`: punts lost to the event budget, by switch.

use std::collections::{BTreeMap, HashMap, HashSet};

use competitive_net_sim::sim_log::{self, DropReasonW, LogEvent, NodeRefW};
use serde_json::json;

const REPORT_BIT: u64 = 1 << 63;

fn main() {
    let path = std::env::args().nth(1).expect("usage: tracecheck RUN.simlog");
    let bytes = std::fs::read(&path).expect("read log");
    let frames = sim_log::writer::decode_all(&bytes).expect("decode");

    let mut made: BTreeMap<u32, u64> = BTreeMap::new();
    let mut seen: HashSet<u64> = HashSet::new();
    let mut refused_port: BTreeMap<u32, u64> = BTreeMap::new();
    let mut dropped: BTreeMap<String, u64> = BTreeMap::new();
    let mut obs_dropped: BTreeMap<String, u64> = BTreeMap::new();
    let mut log_bytes = 0u64;
    let mut log_refused: BTreeMap<u8, u64> = BTreeMap::new();
    let mut links: BTreeMap<u32, Vec<(u64, bool)>> = BTreeMap::new();
    let mut switch_fail: Vec<(u32, u64)> = Vec::new();
    let mut cust: Vec<(u32, u32, u8, String)> = Vec::new();
    let mut per_cust: HashMap<usize, BTreeMap<u64, u64>> = HashMap::new();
    let mut ingressed: HashSet<u64> = HashSet::new();
    let mut port_drops: BTreeMap<(u32, u16), u64> = BTreeMap::new();

    for f in &frames {
        let t = f.at_ns;
        match &f.event {
            LogEvent::CustomerNamed { app, prefix, len, name } => cust.push((*app, *prefix, *len, name.clone())),
            LogEvent::PacketEgress { switch, packet_id, .. } if packet_id & REPORT_BIT != 0 => {
                if seen.insert(*packet_id) {
                    *made.entry(((packet_id >> 32) & 0x7fff_ffff) as u32).or_default() += 1;
                    let _ = switch;
                }
            }
            LogEvent::DroppedAtPort { switch, port, packet_id } => {
                if packet_id & REPORT_BIT != 0 {
                    *refused_port.entry(((packet_id >> 32) & 0x7fff_ffff) as u32).or_default() += 1;
                } else {
                    *port_drops.entry((*switch, *port)).or_default() += 1;
                }
            }
            LogEvent::PacketDropped { at, reason, packet_id } if packet_id & REPORT_BIT != 0 => {
                let at = match at {
                    NodeRefW::Switch(s) => format!("S{s}"),
                    NodeRefW::App(a) => format!("A{a}"),
                };
                *dropped.entry(format!("{:?}@{at}", reason)).or_default() += 1;
            }
            LogEvent::ObservationDropped { switch, cap, .. } => {
                *obs_dropped.entry(format!("S{switch}:cap{cap}")).or_default() += 1;
            }
            LogEvent::TeamLogWritten { bytes, .. } => log_bytes += bytes.len() as u64,
            LogEvent::TeamLogRefused { why, .. } => *log_refused.entry(*why).or_default() += 1,
            LogEvent::LinkFailed { id } => links.entry(*id).or_default().push((t, false)),
            LogEvent::LinkRestored { id } => links.entry(*id).or_default().push((t, true)),
            LogEvent::SwitchFailed { id } => switch_fail.push((*id, t)),
            LogEvent::PacketIngress { port, packet, .. } if *port >= 100 && packet.kind == 0 => {
                if packet.id & REPORT_BIT == 0 && ingressed.insert(packet.id) {
                    let best = cust
                        .iter()
                        .enumerate()
                        .filter(|(_, (_, p, l, _))| {
                            let m = if *l == 0 { 0 } else { u32::MAX << (32 - *l as u32) };
                            packet.ip_src & m == *p
                        })
                        .max_by_key(|(_, (_, _, l, _))| *l)
                        .map(|(i, _)| i);
                    if let Some(i) = best {
                        *per_cust.entry(i).or_default().entry(t / 100_000_000).or_default() += 1;
                    }
                }
            }
            _ => {}
        }
        let _ = DropReasonW::Ttl;
    }
    let customers: Vec<_> = cust
        .iter()
        .enumerate()
        .map(|(i, (app, p, l, name))| {
            let series = per_cust.get(&i).cloned().unwrap_or_default();
            let last = series.keys().max().copied().unwrap_or(0);
            let v: Vec<u64> = (0..=last.max(399)).map(|k| *series.get(&k).unwrap_or(&0)).collect();
            json!({"app": app, "prefix": format!("{}.{}.{}.{}/{}", p >> 24, p >> 16 & 255, p >> 8 & 255, p & 255, l), "name": name, "per100ms": v})
        })
        .collect();
    let out = json!({
        "reports": {"made": made, "refused_at_port": refused_port, "dropped": dropped, "observation_dropped": obs_dropped},
        "team_log": {"bytes": log_bytes, "refused": log_refused},
        "links": links,
        "switch_failed": switch_fail,
        "customer_port_drops": port_drops.iter().map(|((s, p), n)| (format!("S{s}:{p}"), *n)).collect::<BTreeMap<_, _>>(),
        "customers": customers,
    });
    println!("{}", serde_json::to_string(&out).unwrap());
}

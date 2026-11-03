//! Detectors for customer-behaviour eggs, over scope's TAG_WINDOW records
//! (the `SCOPE_CUST_ONLY` claim logger). Customers are found by behaviour:
//! the index is only a key within one run. `DETECT_EGG` picks the rule:
//!
//! - DUP: a mid-size sender (Vault's fingerprint) repeats at least 30% of
//!   its sequence numbers (200+).
//! - TTL: at least half of some sender's packets start with TTL < 8 (100+).
//! - FLOOD: some sender runs at 10x its own usual rate for 8 s or more.

#[path = "../../common/format.rs"]
#[allow(dead_code)]
mod format;
use format::*;
use std::collections::HashMap;

#[derive(Clone, Copy, Default, Debug)]
pub struct Cust {
    pub t: u32,
    pub pkts: u64,
    pub dup: u64,
    pub lowttl: u64,
    pub udp: u64,
    pub small: u64,
    pub mid: u64,
}

/// Every customer reading, keyed by (switch, customer index), in time order.
pub fn readings(log: &[u8]) -> HashMap<(u8, u8), Vec<Cust>> {
    readings_and_multi(log).0
}

/// Customer readings, and for each (switch, source) the windows in which
/// that source came in on two or more of the switch's links.
pub fn readings_and_multi(log: &[u8]) -> (HashMap<(u8, u8), Vec<Cust>>, HashMap<(u8, u8), Vec<u32>>) {
    let (a, b, _) = readings_multi_via(log);
    (a, b)
}

/// As `readings_and_multi`, plus the neighbour pairs each multi-path source
/// came in from (only with the `MULTI_NBRS` log variant).
pub fn readings_multi_via(log: &[u8]) -> (HashMap<(u8, u8), Vec<Cust>>, HashMap<(u8, u8), Vec<u32>>, HashMap<(u8, u8), Vec<(u8, u8)>>) {
    let mut via: HashMap<(u8, u8), Vec<(u8, u8)>> = HashMap::new();
    let mut multi: HashMap<(u8, u8), Vec<u32>> = HashMap::new();
    let mut out: HashMap<(u8, u8), Vec<Cust>> = HashMap::new();
    for f in frames(log) {
        if f.tag != TAG_WINDOW {
            continue;
        }
        let mut r = Reader::new(f.body);
        let (Some(sw), Some(_seq), Some(t)) = (r.u8(), r.u8(), r.u16()) else { continue };
        let Some(np) = r.u8() else { continue };
        let mut ok = true;
        for _ in 0..np {
            ok &= (|| { r.u8()?; r.u8()?; r.u8()?; r.var()?; r.var()?; r.var()?; r.var()?; r.u8()?; r.u8() })().is_some();
        }
        let Some(nc) = r.u8().filter(|_| ok) else { continue };
        for _ in 0..nc {
            let row = (|| {
                let idx = r.u8()?;
                let flags = r.u8()?;
                let pkts = r.var()?;
                let (small, mid, _new) = (r.var()?, r.var()?, r.var()?);
                let dup = if flags & CF_DUP != 0 { r.var()? } else { 0 };
                let lowttl = if flags & CF_LOWTTL != 0 { r.var()? } else { 0 };
                let udp = if flags & CF_UDP != 0 { r.var()? } else { 0 };
                Some((idx, Cust { t: t as u32 * 10, pkts, dup, lowttl, udp, small, mid }))
            })();
            let Some((idx, c)) = row else { break };
            out.entry((sw, idx)).or_default().push(c);
        }
        if let Some(m) = r.u8() {
            for _ in 0..m {
                let Some(i) = r.u8() else { break };
                if option_env!("MULTI_NBRS").is_some() {
                    let (Some(a), Some(b)) = (r.u8(), r.u8()) else { break };
                    // Keyed by (switch, source); the neighbours pair is kept per window.
                    via.entry((sw, i)).or_default().push((a.min(b), a.max(b)));
                }
                multi.entry((sw, i)).or_default().push(t as u32 * 10);
            }
        }
    }
    for v in out.values_mut() {
        v.sort_by_key(|c| c.t);
        v.dedup_by_key(|c| c.t);
    }
    for v in multi.values_mut() {
        v.sort_unstable();
        v.dedup();
    }
    (out, multi, via)
}

/// Lowest one-way delay per (switch, neighbour) from TAG_DELAY records, us.
fn delay_floors(log: &[u8]) -> HashMap<(u8, u8), u32> {
    let mut lo: HashMap<(u8, u8), u32> = HashMap::new();
    for f in frames(log) {
        if f.tag != TAG_DELAY || f.body.len() < 4 {
            continue;
        }
        let mut r = Reader::new(&f.body[4..]);
        for _ in 0..f.body[3] {
            let (Some(nbr), Some(d)) = (r.u8(), r.var()) else { break };
            if d > 0 {
                let e = lo.entry((f.body[0], nbr)).or_insert(u32::MAX);
                *e = (*e).min(d as u32 * 10);
            }
        }
    }
    lo
}

/// The shuffle, v2: a split route over two paths of different speed, so
/// the source's packets arrive out of order. A source reaches one switch
/// from two neighbours in 20+ windows, and the last links in from those
/// two neighbours differ in delay floor by 5 ms or more.
pub fn shuffle_reorder(log: &[u8]) -> bool {
    let (_, multi, via) = readings_multi_via(log);
    let lo = delay_floors(log);
    multi.iter().any(|(&(sw, src), w)| {
        if w.len() < 20 {
            return false;
        }
        let mut pairs = via.get(&(sw, src)).cloned().unwrap_or_default();
        pairs.sort_unstable();
        pairs.dedup();
        pairs.iter().any(|&(a, b)| match (lo.get(&(sw, a)), lo.get(&(sw, b))) {
            (Some(&x), Some(&y)) => x.abs_diff(y) >= 5_000,
            _ => false,
        })
    })
}

/// The shuffle (egg 10): a source keeps arriving at one switch over two
/// or more links (a split route), in at least 20 windows. Reroutes around
/// a failure lasted at most 8 windows in both truth runs.
pub fn shuffle(log: &[u8]) -> bool {
    readings_and_multi(log).1.values().any(|w| w.len() >= 20)
}

fn last(v: &[Cust]) -> Cust {
    v.iter().copied().max_by_key(|c| c.pkts).unwrap_or_default()
}

pub fn dup(log: &[u8]) -> bool {
    readings(log).values().any(|v| {
        let c = last(v);
        // Vault's fingerprint: mid-size packets (256..640 B), not small.
        let mid_size = (c.mid - c.small) * 10 >= c.pkts * 8;
        c.dup >= 200 && c.dup * 10 >= c.pkts * 3 && mid_size
    })
}

pub fn ttl(log: &[u8]) -> bool {
    readings(log).values().any(|v| {
        let c = last(v);
        c.lowttl >= 100 && c.lowttl * 2 >= c.pkts
    })
}

pub fn flood(log: &[u8]) -> bool {
    readings(log).values().any(|v| {
        // Rate between consecutive readings, packets per second.
        let rates: Vec<(u32, u32, f64)> = v
            .windows(2)
            .filter(|w| w[1].t > w[0].t && w[1].pkts >= w[0].pkts)
            .map(|w| (w[0].t, w[1].t, (w[1].pkts - w[0].pkts) as f64 * 1000.0 / (w[1].t - w[0].t) as f64))
            .collect();
        if rates.len() < 10 {
            return false;
        }
        let mut sorted: Vec<f64> = rates.iter().map(|r| r.2).collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let usual = sorted[sorted.len() / 2].max(5.0);
        // Longest stretch of consecutive readings at 10x usual.
        let (mut best, mut start) = (0u32, None::<u32>);
        for &(a, b, r) in &rates {
            if r >= 10.0 * usual {
                let s = *start.get_or_insert(a);
                best = best.max(b - s);
            } else {
                start = None;
            }
        }
        best >= 8_000
    })
}

/// Lag switch: a sender holds its packets back and then lets them all go:
/// a window at no more than 60% of its usual rate, the next at 140% or
/// more, the two together close to twice the usual (nothing lost). Four
/// such pairs or more. Rates are per second over each reading's interval,
/// so a lost report cannot fake a dip.
pub fn lag(log: &[u8]) -> bool {
    readings(log).values().any(|v| {
        let rates: Vec<f64> = v
            .windows(2)
            .filter(|w| w[1].t > w[0].t && w[1].pkts >= w[0].pkts && w[1].t - w[0].t <= 1_500)
            .map(|w| (w[1].pkts - w[0].pkts) as f64 * 1000.0 / (w[1].t - w[0].t) as f64)
            .collect();
        if rates.len() < 10 {
            return false;
        }
        let mut sorted = rates.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let usual = sorted[sorted.len() / 2];
        if usual < 10.0 {
            return false;
        }
        let pairs = rates
            .windows(2)
            .filter(|w| w[0] <= 0.6 * usual && w[1] >= 1.4 * usual && ((w[0] + w[1]) / (2.0 * usual) - 1.0).abs() <= 0.15)
            .count();
        pairs >= 4
    })
}

/// Customer index -> (prefix address, length), from the logger's header.
fn customer_ranges(log: &[u8]) -> HashMap<u8, (u32, u8)> {
    let mut out = HashMap::new();
    for f in frames(log) {
        if f.tag != TAG_CUSTOMERS || f.body.is_empty() {
            continue;
        }
        let mut k = f.body[0];
        let mut r = Reader::new(&f.body[1..]);
        while let (Some(a), Some(l)) = (r.u32(), r.u8()) {
            k += 1;
            out.insert(k, (a, l));
        }
    }
    out
}

/// Every sampled packet: (t ms, src, dst).
fn samples(log: &[u8]) -> Vec<(u32, u32, u32)> {
    frames(log)
        .into_iter()
        .filter(|f| f.tag == TAG_SAMPLE && f.body.len() >= 11)
        .map(|f| {
            let b = f.body;
            let t = (b[1] as u32 | (b[2] as u32) << 8) * 10;
            let src = u32::from_le_bytes([b[3], b[4], b[5], b[6]]);
            let dst = u32::from_le_bytes([b[7], b[8], b[9], b[10]]);
            (t, src, dst)
        })
        .collect()
}

/// The longest stretch at 10x a source's usual rate, as (start, end) ms.
fn flood_stretch(v: &[Cust]) -> Option<(u32, u32)> {
    let rates: Vec<(u32, u32, f64)> = v
        .windows(2)
        .filter(|w| w[1].t > w[0].t && w[1].pkts >= w[0].pkts)
        .map(|w| (w[0].t, w[1].t, (w[1].pkts - w[0].pkts) as f64 * 1000.0 / (w[1].t - w[0].t) as f64))
        .collect();
    if rates.len() < 10 {
        return None;
    }
    let mut sorted: Vec<f64> = rates.iter().map(|r| r.2).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let usual = sorted[sorted.len() / 2].max(5.0);
    let (mut best, mut start) = (None::<(u32, u32)>, None::<u32>);
    for &(a, b, r) in &rates {
        if r >= 10.0 * usual {
            let s = *start.get_or_insert(a);
            if best.is_none_or(|(x, y)| b - s > y - x) {
                best = Some((s, b));
            }
        } else {
            start = None;
        }
    }
    best.filter(|(a, b)| b - a >= 8_000)
}

/// Flood, v2 (a DDoS): a sender's flood (10x its usual rate, 8 s or more)
/// goes somewhere it does not usually send: a packet sampled during the
/// flood has a destination /24 none of its samples outside the flood has.
/// A customer that just surges toward its own peer is not this egg.
pub fn flood_elsewhere(log: &[u8]) -> bool {
    let ranges = customer_ranges(log);
    let smp = samples(log);
    readings(log).iter().any(|(&(_, idx), v)| {
        let Some((a, b)) = flood_stretch(v) else { return false };
        let Some(&(addr, plen)) = ranges.get(&idx) else { return false };
        let mask = if plen == 0 { 0 } else { u32::MAX << (32 - plen as u32) };
        let mine: Vec<&(u32, u32, u32)> = smp.iter().filter(|s| s.1 & mask == addr).collect();
        // Samples queue a little, so the flood's own can trail its end.
        let usual: Vec<u32> = mine.iter().filter(|s| s.0 < a || s.0 > b + 5_000).map(|s| s.2 >> 8).collect();
        mine.iter().any(|s| s.0 >= a && s.0 <= b + 5_000 && !usual.contains(&(s.2 >> 8)))
    })
}

fn detect(log: &[u8]) -> bool {
    match option_env!("DETECT_EGG") {
        Some("DUP") => dup(log),
        Some("TTL") => ttl(log),
        Some("FLOOD") => flood(log),
        Some("SHUFFLE") => shuffle(log),
        Some("LAG") => lag(log),
        Some("SHUFFLE2") => shuffle_reorder(log),
        Some("FLOOD2") => flood_elsewhere(log),
        _ => false,
    }
}

switch_program_sdk::detect_program!(detect);

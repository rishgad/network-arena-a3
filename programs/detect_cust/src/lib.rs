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
    (out, multi)
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

fn detect(log: &[u8]) -> bool {
    match option_env!("DETECT_EGG") {
        Some("DUP") => dup(log),
        Some("TTL") => ttl(log),
        Some("FLOOD") => flood(log),
        Some("SHUFFLE") => shuffle(log),
        _ => false,
    }
}

switch_program_sdk::detect_program!(detect);

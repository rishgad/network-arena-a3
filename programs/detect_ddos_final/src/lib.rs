//! Detectors for customer-behaviour eggs, over scope's TAG_WINDOW records
//! (the `SCOPE_CUST_ONLY` claim logger). Customers are found by behaviour:
//! the index is only a key within one run. `DETECT_EGG` picks the rule:
//!
//! - DUP: a mid-size sender (Vault's fingerprint) repeats at least 30% of
//!   its sequence numbers (200+).
//! - TTL: at least half of some sender's packets start with TTL < 8 (100+).
//! - FLOOD: some sender runs at 10x its own usual rate for 8 s or more.

#[path = "format.rs"]
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
    pub ttl4: u64,
    /// New destinations this window (or distinct source hosts, in the
    /// `SCOPE_SRC_HOSTS` log variant).
    pub new: u64,
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
                let (small, mid, new) = (r.var()?, r.var()?, r.var()?);
                let dup = if flags & CF_DUP != 0 { r.var()? } else { 0 };
                let lowttl = if flags & CF_LOWTTL != 0 { r.var()? } else { 0 };
                let ttl4 = if flags & CF_LOWTTL != 0 && option_env!("TTL4").is_some() { r.var()? } else { 0 };
                let udp = if flags & CF_UDP != 0 { r.var()? } else { 0 };
                Some((idx, Cust { t: t as u32 * 10, pkts, dup, lowttl, udp, small, mid, ttl4, new }))
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
    flood_stretch_at(v, 10.0, 8_000)
}

/// The longest stretch at `k`x a source's usual rate lasting `min_ms`+.
fn flood_stretch_at(v: &[Cust], k: f64, min_ms: u32) -> Option<(u32, u32)> {
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
        if r >= k * usual {
            let s = *start.get_or_insert(a);
            if best.is_none_or(|(x, y)| b - s > y - x) {
                best = Some((s, b));
            }
        } else {
            start = None;
        }
    }
    best.filter(|(a, b)| b - a >= min_ms)
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

/// Traceroute (a TTL ladder): most of a sender's packets start with a low
/// TTL, and the TTLs climb: both TTL < 4 and 4 <= TTL < 8 are common (each
/// 20% or more). Truth run 3: Mapper delivers 62.6% = 5/8, a ladder 1..8 on
/// a 3-hop path. A constant low TTL fills only one band.
pub fn ladder(log: &[u8]) -> bool {
    readings(log).values().any(|v| {
        let c = last(v);
        let p = c.pkts.max(1);
        let lo = c.ttl4;
        let mid = c.lowttl.saturating_sub(c.ttl4);
        c.lowttl >= 100 && c.lowttl * 2 >= p && lo * 5 >= p && mid * 5 >= p
    })
}

/// Flood v3 (a UDP DDoS from a TCP customer): a sender runs at 5x its
/// usual rate for 5 s or more, the flood is at least 80% UDP, and before
/// it the sender was at most 20% UDP. Truth runs: Home users, normally
/// 200 B TCP to Fastly, send ~30x in 64 B UDP from many hosts 16.7-30 s.
/// A flash crowd (a TCP surge) is not this.
pub fn udp_flood(log: &[u8]) -> bool {
    readings(log).values().any(|v| {
        // 5x for 5 s: the UDP switch, not the size of the surge, is what
        // separates it from a flash crowd, so allow for weaker floods.
        let Some((a, b)) = flood_stretch_at(v, 5.0, 5_000) else { return false };
        let at = |t: u32| v.iter().filter(|c| c.t <= t).max_by_key(|c| c.t).copied();
        let (Some(s0), Some(s1)) = (at(a), at(b)) else { return false };
        let first = v[0];
        let (dp, du) = (s1.pkts.saturating_sub(s0.pkts), s1.udp.saturating_sub(s0.udp));
        let (bp, bu) = (s0.pkts.saturating_sub(first.pkts), s0.udp.saturating_sub(first.udp));
        dp > 0 && du * 10 >= dp * 8 && bp >= 50 && bu * 10 <= bp * 2
    })
}

/// The longest silence (no new packets) between a source's first reading
/// and a later resumption of 100+ packets, as (start, end) ms.
fn longest_silence(v: &[Cust]) -> Option<(u32, u32)> {
    let mut best: Option<(u32, u32)> = None;
    for (i, a) in v.iter().enumerate() {
        // The silence runs from `a` to the first later reading with more packets.
        let Some(b) = v[i + 1..].iter().find(|c| c.pkts > a.pkts) else { break };
        let resumed = v.last().map_or(0, |l| l.pkts.saturating_sub(a.pkts));
        if resumed >= 100 && best.is_none_or(|(x, y)| b.t - a.t > y - x) {
            best = Some((a.t, b.t));
        }
    }
    best
}

/// Long pause (Sync): two ranges at the same switch both stop for 12 s or
/// more, together (starts and ends within 2 s), then both carry on. No
/// other customer in the truth runs is ever silent that long (Amazon's off
/// spells are 8 s, Dropbox's ~5 s); a single customer pausing is not this.
pub fn pair_pause(log: &[u8]) -> bool {
    let rs = readings(log);
    let gaps: Vec<((u8, u8), (u32, u32))> = rs
        .iter()
        .filter_map(|(&k, v)| longest_silence(v).filter(|(a, b)| b - a >= 12_000).map(|g| (k, g)))
        .collect();
    gaps.iter().any(|&((sw, i), (a, b))| {
        gaps.iter().any(|&((sw2, j), (c, d))| sw2 == sw && j != i && a.abs_diff(c) <= 2_000 && b.abs_diff(d) <= 2_000)
    })
}

/// Flood v4: as v3 (a TCP customer's UDP flood) but robust to long floods
/// and floods at the start of the run. "Usual" is the 20th-percentile rate,
/// not the median (a flood longer than half the run made the median the
/// flood itself); the UDP share is checked outside the flood, before or
/// after it.
pub fn udp_flood_robust(log: &[u8]) -> bool {
    readings(log).values().any(|v| {
        let rates: Vec<(u32, u32, f64, u64, u64)> = v
            .windows(2)
            .filter(|w| w[1].t > w[0].t && w[1].pkts >= w[0].pkts && w[1].t - w[0].t <= 1_500)
            .map(|w| {
                let dt = (w[1].t - w[0].t) as f64;
                (w[0].t, w[1].t, (w[1].pkts - w[0].pkts) as f64 * 1000.0 / dt, w[1].pkts - w[0].pkts, w[1].udp.saturating_sub(w[0].udp))
            })
            .collect();
        if rates.len() < 10 {
            return false;
        }
        let mut sorted: Vec<f64> = rates.iter().map(|r| r.2).filter(|r| *r > 0.0).collect();
        if sorted.len() < 5 {
            return false;
        }
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let usual = sorted[sorted.len() / 5].max(5.0);
        // Longest run of consecutive windows at 5x usual.
        let (mut best, mut cur) = ((0usize, 0usize), None::<usize>);
        for (i, r) in rates.iter().enumerate() {
            if r.2 >= 5.0 * usual {
                let s = *cur.get_or_insert(i);
                if i + 1 - s > best.1 - best.0 {
                    best = (s, i + 1);
                }
            } else {
                cur = None;
            }
        }
        let (a, b) = best;
        if b <= a || rates[b - 1].1 - rates[a].0 < 5_000 {
            return false;
        }
        let (fp, fu) = rates[a..b].iter().fold((0, 0), |(p, u), r| (p + r.3, u + r.4));
        let (op, ou) = rates.iter().enumerate().filter(|(i, _)| *i < a || *i >= b).fold((0, 0), |(p, u), (_, r)| (p + r.3, u + r.4));
        // `FLOOD_HOSTS`: outside the flood, many source hosts per window
        // (Home users: 8 hosts in 9 samples; Prime 1, Login 3, Cloudflare 2-4).
        let hosts_ok = !option_env!("FLOOD_HOSTS").is_some() || {
            let mut h: Vec<u64> = v.iter().filter(|c| c.t < rates[a].0 || c.t > rates[b - 1].1).map(|c| c.new).collect();
            h.sort_unstable();
            !h.is_empty() && h[h.len() / 2] >= 10
        };
        let small_ok = !option_env!("FLOOD_SMALL").is_some() || {
            // Home users' fingerprint: outside the flood, small TCP packets.
            let os: u64 = rates
                .iter()
                .enumerate()
                .filter(|(i, _)| *i < a || *i >= b)
                .map(|(_, r)| small_between(v, r.0, r.1))
                .sum();
            os * 10 >= op * 8
        };
        // `FLOOD_UDP_HOSTS` (a DDoS, not a DoS): during the flood, the
        // median window has 8+ distinct UDP source hosts (the
        // `SCOPE_SRC_HOSTS_UDP` log). Truth: Home users' flood comes from
        // their whole pool (141-189 hosts, nearly every sample a new host).
        let ddos_ok = !option_env!("FLOOD_UDP_HOSTS").is_some() || {
            let mut h: Vec<u64> = v.iter().filter(|c| c.t > rates[a].0 && c.t <= rates[b - 1].1).map(|c| c.new).collect();
            h.sort_unstable();
            !h.is_empty() && h[h.len() / 2] >= 8
        };
        fp > 0 && fu * 10 >= fp * 8 && op >= 50 && ou * 10 <= op * 2 && small_ok && hosts_ok && ddos_ok
    })
}

/// Small packets a source sent between two of its readings.
fn small_between(v: &[Cust], t0: u32, t1: u32) -> u64 {
    let at = |t: u32| v.iter().find(|c| c.t == t).map_or(0, |c| c.small);
    at(t1).saturating_sub(at(t0))
}

/// Side stream: a large-packet sender (80%+ of its packets 640 B or more)
/// also carries a steady trickle of small packets (< 256 B): 0.5% to 15%
/// of its packets, 50 or more, still arriving in at least half its
/// windows. Truth runs: Netflix 1.52% (200 B TCP to its peer, ~5 pkt/s,
/// all run); every other large sender has exactly one small packet.
pub fn side_stream(log: &[u8]) -> bool {
    readings(log).values().any(|v| {
        let c = last(v);
        let p = c.pkts.max(1);
        let large = p.saturating_sub(c.mid);
        let steady = v.windows(2).filter(|w| w[1].small > w[0].small).count() * 2 >= v.len().saturating_sub(1);
        large * 10 >= p * 8 && c.small >= 50 && c.small * 200 >= p && c.small * 100 <= p * 15 && steady
    })
}

/// Repeated surges (Steam): a large-packet sender (80%+ at 640 B or more)
/// that at least twice jumps to 5x its quiet (20th-percentile) rate and
/// to link speed (1,200+ pkt/s; the truth runs pin at 1,275 at every load), for 3-7.5 s each time (a surge cut off
/// by the end of the run counts from 2 s).
/// Truth runs: Steam ~63-94 pkt/s with two 4 s surges at ~1,250 pkt/s.
/// A single surge, an always-fast sender, or a small-packet flood is not this.
pub fn surges(log: &[u8]) -> bool {
    readings(log).values().any(|v| {
        let c = last(v);
        let p = c.pkts.max(1);
        if p.saturating_sub(c.mid) * 10 < p * 8 {
            return false;
        }
        let rates: Vec<(u32, u32, f64)> = v
            .windows(2)
            .filter(|w| w[1].t > w[0].t && w[1].pkts >= w[0].pkts && w[1].t - w[0].t <= 1_500)
            .map(|w| (w[0].t, w[1].t, (w[1].pkts - w[0].pkts) as f64 * 1000.0 / (w[1].t - w[0].t) as f64))
            .collect();
        if rates.len() < 10 {
            return false;
        }
        let mut sorted: Vec<f64> = rates.iter().map(|r| r.2).collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let quiet = sorted[sorted.len() / 5].max(5.0);
        // A surge: consecutive readings at 5x quiet and near link speed.
        let mut spans: Vec<(u32, u32)> = Vec::new();
        let mut cur: Option<(u32, u32)> = None;
        for &(a, b, r) in &rates {
            if r >= 5.0 * quiet && r >= 1_200.0 {
                cur = Some(cur.map_or((a, b), |(s, _)| (s, b)));
            } else if let Some(sp) = cur.take() {
                spans.push(sp);
            }
        }
        let open = cur; // a surge still running when the run ends
        let done = spans.iter().filter(|(s, e)| (2_000..=7_500).contains(&(e - s))).count();
        let cut = open.filter(|(s, e)| e - s >= 2_000).is_some() as usize;
        done + cut >= 2
    })
}

/// Per-reading rates of a source, (t0, t1, pkts/s).
fn rates_of(v: &[Cust]) -> Vec<(u32, u32, f64)> {
    v.windows(2)
        .filter(|w| w[1].t > w[0].t && w[1].pkts >= w[0].pkts && w[1].t - w[0].t <= 1_500)
        .map(|w| (w[0].t, w[1].t, (w[1].pkts - w[0].pkts) as f64 * 1000.0 / (w[1].t - w[0].t) as f64))
        .collect()
}

/// Full-speed surges of a large-packet source: spans of 2-7.5 s at 5x its
/// quiet rate and 1,200+ pkt/s (one cut off by the run's end counts from 2 s).
fn surge_count(v: &[Cust]) -> usize {
    let c = last(v);
    let p = c.pkts.max(1);
    if p.saturating_sub(c.mid) * 10 < p * 8 {
        return 0;
    }
    let rates = rates_of(v);
    if rates.len() < 10 {
        return 0;
    }
    let mut sorted: Vec<f64> = rates.iter().map(|r| r.2).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let quiet = sorted[sorted.len() / 5].max(5.0);
    let (mut spans, mut cur) = (Vec::new(), None::<(u32, u32)>);
    for &(a, b, r) in &rates {
        if r >= 5.0 * quiet && r >= 1_200.0 {
            cur = Some(cur.map_or((a, b), |(s, _)| (s, b)));
        } else if let Some(sp) = cur.take() {
            spans.push(sp);
        }
    }
    spans.iter().filter(|(s, e)| (2_000..=7_500).contains(&(e - s))).count()
        + cur.filter(|(s, e)| e - s >= 2_000).is_some() as usize
}

/// SURGE-1: at least one full-speed surge (2-7.5 s) by a large-packet sender.
pub fn surge_any(log: &[u8]) -> bool {
    readings(log).values().any(|v| surge_count(v) >= 1)
}

/// SURGE-STEAM: two or more surges by Steam, fingerprinted as the slowest
/// steady large-packet sender (its quiet windows within 5% of each other);
/// in the truth runs Steam is 64-96 pkt/s, the next (Ledger) 105-139.
pub fn surge_steam(log: &[u8]) -> bool {
    let rs = readings(log);
    let mut steady: Vec<(f64, &Vec<Cust>)> = Vec::new();
    for v in rs.values() {
        let c = last(v);
        let p = c.pkts.max(1);
        if p.saturating_sub(c.mid) * 10 < p * 8 || p < 500 {
            continue;
        }
        let mut r: Vec<f64> = rates_of(v).iter().map(|x| x.2).filter(|x| *x > 0.0).collect();
        if r.len() < 10 {
            continue;
        }
        r.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let lo = r[r.len() / 10];
        let mid = r[r.len() / 2];
        if (mid - lo) <= 0.05 * mid {
            steady.push((mid, v));
        }
    }
    steady.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    steady.first().is_some_and(|(_, v)| surge_count(v) >= 2)
}

/// PRIME: a /32 customer keeps sending while the /24 customer covering it
/// is silent for 5 s or more, at least twice (it runs at half its usual
/// rate or more through those silences). The Amazon Prime member does
/// this through every 8 s spell the shoppers' range is off.
pub fn prime_member(log: &[u8]) -> bool {
    let ranges = customer_ranges(log);
    let rs = readings(log);
    let by_idx = |i: u8| rs.iter().find(|((_, k), _)| *k == i).map(|(_, v)| v);
    // A range carved out of a /24 customer (a /32 on the real network).
    ranges.iter().filter(|(_, (_, l))| *l > 24).any(|(&host, &(addr, _))| {
        let parent = ranges.iter().find(|(_, (a, l))| *l == 24 && addr & 0xFFFF_FF00 == *a).map(|(k, _)| *k);
        let (Some(pv), Some(hv)) = (parent.and_then(|p| by_idx(p)), by_idx(host)) else { return false };
        let hr = rates_of(hv);
        let mut hs: Vec<f64> = hr.iter().map(|x| x.2).collect();
        if hs.len() < 10 {
            return false;
        }
        hs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let usual = hs[hs.len() / 2];
        // Parent's silences of 5 s or more.
        let pr = rates_of(pv);
        let (mut spells, mut cur) = (Vec::new(), None::<(u32, u32)>);
        for &(a, b, r) in &pr {
            if r == 0.0 {
                cur = Some(cur.map_or((a, b), |(s, _)| (s, b)));
            } else if let Some(sp) = cur.take() {
                spells.push(sp);
            }
        }
        spells.extend(cur);
        let kept = spells
            .iter()
            .filter(|(s, e)| e - s >= 5_000)
            .filter(|(s, e)| {
                let during: Vec<f64> = hr.iter().filter(|x| x.0 >= *s && x.1 <= *e).map(|x| x.2).collect();
                !during.is_empty() && during.iter().sum::<f64>() / during.len() as f64 >= 0.5 * usual
            })
            .count();
        usual >= 5.0 && kept >= 2
    })
}

/// LIVE: a fan-out sender (8+ new destinations in its median window) with
/// regular dropouts: 6 or more windows with no packets, each a single
/// window (or two) between busy ones. Netflix live: 12 viewers, off 1 s in 4.
pub fn live_dropouts(log: &[u8]) -> bool {
    readings(log).values().any(|v| {
        let mut nd: Vec<u64> = v.iter().map(|c| c.new).collect();
        if nd.len() < 10 {
            return false;
        }
        nd.sort_unstable();
        if nd[nd.len() / 2] < 8 {
            return false;
        }
        let r: Vec<f64> = rates_of(v).iter().map(|x| x.2).collect();
        let mut drops = 0;
        let mut i = 1;
        while i + 1 < r.len() {
            if r[i] == 0.0 {
                let mut j = i;
                while j < r.len() && r[j] == 0.0 {
                    j += 1;
                }
                if j - i <= 2 && r[i - 1] > 0.0 && j < r.len() {
                    drops += 1;
                }
                i = j;
            } else {
                i += 1;
            }
        }
        drops >= 6
    })
}

/// Cloudflare DDoS (egg 11, named on another team's card): a sender of
/// small TCP packets (80%+ under 256 B, at most 20% UDP) hammering many
/// servers: a typical rate of 200+ pkt/s with 6+ new destinations in most
/// windows (80%+). Truth runs: Cloudflare users, 100 B TCP at 350-525
/// pkt/s to 8 Cloudflare hosts all run; its twin Akamai users runs 4-6.
pub fn cdn_ddos(log: &[u8]) -> bool {
    readings(log).values().any(|v| {
        let c = last(v);
        let p = c.pkts.max(1);
        if c.small * 10 < p * 8 || c.udp * 10 > p * 2 {
            return false;
        }
        let r: Vec<f64> = rates_of(v).iter().map(|x| x.2).collect();
        if r.len() < 10 {
            return false;
        }
        let mut sorted = r.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let typical = sorted[sorted.len() / 2];
        let spread = v.iter().filter(|c| c.new >= 6).count();
        typical >= 200.0 && spread * 10 >= v.len() * 8
    })
}

/// HUNG: a server stops answering. An unlisted local source (a server, not
/// a told customer) sent data steadily (5+ windows of 3+ packets), then
/// sends no data for 6+ windows spanning 5+ s while its hosts still ack
/// (3+ acks every one of those windows): the host is up, the app is not.
/// Rejects a brief stall, the client going quiet too (acks stop), and any
/// told customer pausing (`SCOPE_SERVERS` log variant, TAG_SERVER).
pub fn hung_server(log: &[u8]) -> bool {
    let mut by: HashMap<(u8, u64), Vec<(u32, u64, u64)>> = HashMap::new();
    for f in frames(log) {
        if f.tag != TAG_SERVER {
            continue;
        }
        let mut r = Reader::new(f.body);
        let (Some(sw), Some(t), Some(n)) = (r.u8(), r.u16(), r.u8()) else { continue };
        for _ in 0..n {
            let (Some(k), Some(d), Some(a)) = (r.var(), r.var(), r.var()) else { break };
            by.entry((sw, k)).or_default().push((t as u32 * 10, d, a));
        }
    }
    by.values_mut().any(|v| {
        v.sort();
        v.dedup_by_key(|x| x.0);
        for i in 5..v.len() {
            if v[i].1 != 0 || v[i - 1].1 == 0 {
                continue;
            }
            if !v[i - 5..i].iter().all(|x| x.1 >= 3) {
                continue;
            }
            let mut j = i;
            while j < v.len() && v[j].1 == 0 && v[j].2 >= 3 && (j == i || v[j].0 - v[j - 1].0 <= 1500) {
                j += 1;
            }
            if j - i >= 6 && v[j - 1].0 - v[i].0 >= 5000 {
                return true;
            }
        }
        false
    })
}

/// HUNG2: Login's server hangs for good. A bucket of small (<160 B) replies
/// from an untold local source (the `SCOPE_SERVERS_SMALL` log) that sent 3+
/// in at least 3 of the 5 windows before it stopped, then sent nothing in
/// every later window to the end of the log (4+ windows spanning 3+ s,
/// ending after 36 s), while its host acked in each. Rejects a stall that
/// recovers, any server with large replies, and the client stopping.
pub fn hung_for_good(log: &[u8]) -> bool {
    let mut by: HashMap<(u8, u64), Vec<(u32, u64, u64)>> = HashMap::new();
    for f in frames(log) {
        if f.tag != TAG_SERVER {
            continue;
        }
        let mut r = Reader::new(f.body);
        let (Some(sw), Some(t), Some(n)) = (r.u8(), r.u16(), r.u8()) else { continue };
        for _ in 0..n {
            let (Some(k), Some(d), Some(a)) = (r.var(), r.var(), r.var()) else { break };
            by.entry((sw, k)).or_default().push((t as u32 * 10, d, a));
        }
    }
    by.values_mut().any(|v| {
        v.sort();
        v.dedup_by_key(|x| x.0);
        let Some(last) = v.iter().rposition(|x| x.1 > 0) else { return false };
        let tail = &v[last + 1..];
        let before = &v[last.saturating_sub(4)..=last];
        tail.len() >= 4
            && tail.last().unwrap().0 >= 36_000
            && tail.last().unwrap().0 - tail[0].0 >= 3_000
            && tail.iter().all(|x| x.2 >= 1)
            && tail.windows(2).all(|w| w[1].0 - w[0].0 <= 1_500)
            && before.iter().filter(|x| x.1 >= 3).count() >= 3
    })
}

/// AMP: request/reply amplification (egg 2). For one local customer, 8+
/// windows (gaps under 1.5 s) where it sent 20+ requests, an untold server
/// handed back 0.5-2 replies per request, and the replies carried 5x+ the
/// request bytes (`SCOPE_AMP` log, TAG_AMP). Rejects request-only traffic,
/// like-for-like replies, a few big replies, and a brief spell.
pub fn amplification(log: &[u8]) -> bool {
    let mut by: HashMap<(u8, u8), Vec<u32>> = HashMap::new();
    for f in frames(log) {
        if f.tag != TAG_AMP {
            continue;
        }
        let mut r = Reader::new(f.body);
        let (Some(sw), Some(t), Some(n)) = (r.u8(), r.u16(), r.u8()) else { continue };
        for _ in 0..n {
            let (Some(c), Some(qp), Some(qb), Some(pp), Some(pb)) = (r.u8(), r.var(), r.var(), r.var(), r.var()) else { break };
            if qp >= 20 && pb >= 5 * qb && pp * 2 >= qp && pp <= 2 * qp {
                by.entry((sw, c)).or_default().push(t as u32 * 10);
            }
        }
    }
    by.values_mut().any(|v| {
        v.sort();
        v.dedup();
        let mut run = 1;
        for w in v.windows(2) {
            run = if w[1] - w[0] <= 1_500 { run + 1 } else { 1 };
            if run >= 8 {
                return true;
            }
        }
        false
    })
}

/// Samples must independently show distributed sources and one victim.
/// No fixed addresses or customer names: ranges come only from the log.
/// Sample timestamps are export times: the four-entry normal queue emits
/// two per ~1.02 s window. Allow 3 s of bounded queueing after the rate window.
fn concentrated_udp(log: &[u8], sw: u8, idx: u8, start: u32, end: u32) -> bool {
    let ranges = customer_ranges(log);
    let Some(&(addr, plen)) = ranges.get(&idx) else { return false };
    let holds = |ip: u32, base: u32, bits: u8| {
        bits <= 32 && (if bits == 0 { true } else { ip & (u32::MAX << (32 - bits as u32)) == base })
    };
    let mut hosts = std::collections::HashSet::new();
    let mut victims: HashMap<u32, usize> = HashMap::new();
    let mut windows = std::collections::HashSet::new();
    let mut n = 0usize;
    for f in frames(log) {
        if f.tag != TAG_SAMPLE || f.body.len() < 15 { continue }
        let b = f.body;
        let t = u16::from_le_bytes([b[1], b[2]]) as u32 * 10;
        if b[0] != sw || b[11] != 17 || t < start || t > end + 3000 { continue }
        let src = u32::from_le_bytes([b[3], b[4], b[5], b[6]]);
        let dst = u32::from_le_bytes([b[7], b[8], b[9], b[10]]);
        if !holds(src, addr, plen) || ranges.values().any(|&(a, p)| p > plen && holds(src, a, p)) {
            continue;
        }
        hosts.insert(src);
        *victims.entry(dst).or_default() += 1;
        windows.insert(t / 1000);
        n += 1;
    }
    n >= 6 && hosts.len() >= 4 && windows.len() >= 3
        && victims.values().copied().max().unwrap_or(0) * 5 >= n * 4
}

pub fn udp_flood_concentrated(log: &[u8]) -> bool {
    readings(log).iter().any(|(&(sw, idx), v)| {
        let rates: Vec<(u32, u32, f64, u64, u64)> = v
            .windows(2)
            .filter(|w| w[1].t > w[0].t && w[1].pkts >= w[0].pkts && w[1].t - w[0].t <= 1_500)
            .map(|w| {
                let dt = (w[1].t - w[0].t) as f64;
                (w[0].t, w[1].t, (w[1].pkts - w[0].pkts) as f64 * 1000.0 / dt, w[1].pkts - w[0].pkts, w[1].udp.saturating_sub(w[0].udp))
            })
            .collect();
        if rates.len() < 10 {
            return false;
        }
        let mut sorted: Vec<f64> = rates.iter().map(|r| r.2).filter(|r| *r > 0.0).collect();
        if sorted.len() < 5 {
            return false;
        }
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let usual = sorted[sorted.len() / 5].max(5.0);
        // Longest run of consecutive windows at 5x usual.
        let (mut best, mut cur) = ((0usize, 0usize), None::<usize>);
        for (i, r) in rates.iter().enumerate() {
            if i > 0 && r.0 != rates[i - 1].1 {
                cur = None;
            }
            if r.2 >= 5.0 * usual {
                let s = *cur.get_or_insert(i);
                if i + 1 - s > best.1 - best.0 {
                    best = (s, i + 1);
                }
            } else {
                cur = None;
            }
        }
        let (a, b) = best;
        if b <= a || rates[b - 1].1 - rates[a].0 < 5_000 {
            return false;
        }
        let (fp, fu) = rates[a..b].iter().fold((0, 0), |(p, u), r| (p + r.3, u + r.4));
        let (op, ou) = rates.iter().enumerate().filter(|(i, _)| *i < a || *i >= b).fold((0, 0), |(p, u), (_, r)| (p + r.3, u + r.4));
        // `FLOOD_HOSTS`: outside the flood, many source hosts per window
        // (Home users: 8 hosts in 9 samples; Prime 1, Login 3, Cloudflare 2-4).
        let hosts_ok = !option_env!("FLOOD_HOSTS").is_some() || {
            let mut h: Vec<u64> = v.iter().filter(|c| c.t < rates[a].0 || c.t > rates[b - 1].1).map(|c| c.new).collect();
            h.sort_unstable();
            !h.is_empty() && h[h.len() / 2] >= 10
        };
        let small_ok = !option_env!("FLOOD_SMALL").is_some() || {
            // Home users' fingerprint: outside the flood, small TCP packets.
            let os: u64 = rates
                .iter()
                .enumerate()
                .filter(|(i, _)| *i < a || *i >= b)
                .map(|(_, r)| small_between(v, r.0, r.1))
                .sum();
            os * 10 >= op * 8
        };
        // `FLOOD_UDP_HOSTS` (a DDoS, not a DoS): during the flood, the
        // median window has 8+ distinct UDP source hosts (the
        // `SCOPE_SRC_HOSTS_UDP` log). Truth: Home users' flood comes from
        // their whole pool (141-189 hosts, nearly every sample a new host).
        let ddos_ok = !option_env!("FLOOD_UDP_HOSTS").is_some() || {
            let mut h: Vec<u64> = v.iter().filter(|c| c.t > rates[a].0 && c.t <= rates[b - 1].1).map(|c| c.new).collect();
            h.sort_unstable();
            !h.is_empty() && h[h.len() / 2] >= 8
        };
        fp > 0 && fu * 10 >= fp * 8 && op >= 50 && ou * 10 <= op * 2 && small_ok && hosts_ok && ddos_ok
            && concentrated_udp(log, sw, idx, rates[a].0, rates[b - 1].1)
    })
}

fn detect(log: &[u8]) -> bool {
    match option_env!("DETECT_EGG") {
        Some("DDOSFINAL") => udp_flood_concentrated(log),
        Some("DUP") => dup(log),
        Some("TTL") => ttl(log),
        Some("FLOOD") => flood(log),
        Some("SHUFFLE") => shuffle(log),
        Some("LAG") => lag(log),
        Some("SHUFFLE2") => shuffle_reorder(log),
        Some("FLOOD2") => flood_elsewhere(log),
        Some("LADDER") => ladder(log),
        Some("FLOOD3") => udp_flood(log),
        Some("PAUSE2") => pair_pause(log),
        Some("FLOOD4") => udp_flood_robust(log),
        Some("SIDE") => side_stream(log),
        Some("SURGE") => surges(log),
        Some("SURGE1") => surge_any(log),
        Some("SURGESTEAM") => surge_steam(log),
        Some("PRIME") => prime_member(log),
        Some("LIVE") => live_dropouts(log),
        Some("CFDDOS") => cdn_ddos(log),
        Some("HUNG") => hung_server(log),
        Some("HUNG2") => hung_for_good(log),
        Some("AMP") => amplification(log),
        _ => false,
    }
}

switch_program_sdk::detect_program!(detect);

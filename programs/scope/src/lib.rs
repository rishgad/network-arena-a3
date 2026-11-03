//! # scope — the broad, exploratory logger (format v1)
//!
//! Every switch counts every packet in its data plane and reports a
//! compact window of cumulative counters once a second, staggered by
//! switch id; link silences are reported the moment they are seen. The
//! logging switch writes its own windows and everyone else's reports.
//! Byte layout: `FORMAT.md`. Budget and blind spots: `DESIGN.md`.
//!
//! Data plane (8 stages, one memory resource each):
//!  1. T1 (LPM, ip_src) -> M_SRC = customer index (1..63, 0 unknown)
//!  2. T2 (LPM, ip_dst) -> M_DST = customer index; our reports and
//!     beacons are punted or steered here
//!  3. T3 (exact, key 0) -> M_EPOCH, the window number, bumped by the
//!     controller at every window
//!  4. R1: repeated data seq per source -> M_DUP; at the ingress switch a
//!     (source group, ip_dst & 255) bitmap stamped with the epoch -> M_NEW
//!     the first time a destination is seen in the window
//!  5. C1: packets by (source, ingress port class); keepalives by port
//!  6. C1, ingress switch only: new destinations, repeats, TTL < 8, UDP,
//!     size classes
//!  7. C2: (source & 15) x destination pairs, ingress switch only
//!  8. a 1-in-N sample of keepalives to the controller, for delay
//!
//! Every timer event, counters included, must fit the host's 4,096-byte
//! input buffer (src/wasm.rs `call_export`): C1 + C2 are 1,736 slots.


use std::collections::{HashMap, VecDeque};
use switch_program_sdk::wire::{PacketFieldW as F, TableActionW, TableEntryW};
use switch_program_sdk::*;

#[path = "../../common/asm.rs"]
mod asm;
#[path = "../../common/format.rs"]
pub mod format;
use format::*;

/// The logging switch, set at build time (`SCOPE_LOGGER=7 cargo build`).
pub const LOGGER: u32 = match option_env!("SCOPE_LOGGER") {
    Some(s) => parse_u32(s),
    None => 0,
};
const fn parse_u32(s: &str) -> u32 {
    let b = s.as_bytes();
    let mut i = 0;
    let mut v = 0;
    while i < b.len() {
        v = v * 10 + (b[i] - b'0') as u32;
        i += 1;
    }
    v
}

/// Claim mode: write link events only (`SCOPE_LINKS_ONLY=1`), for the
/// claims' 24 KB cap.
const LINKS_ONLY: bool = option_env!("SCOPE_LINKS_ONLY").is_some();

pub const PROTO_REPORT: u8 = 93;
pub const PROTO_BEACON: u8 = 94;
const REPORT_IP: u32 = 0xe000_0007;
const BEACON_IP: u32 = 0xe000_0008;
const SELF_BASE: u32 = 0xe000_0100;

const TICK_NS: u64 = 100_000_000;
/// Ticks per window.
const WINDOW_TICKS: u64 = 10;
/// Pairs are reported every this many windows.
const PAIR_EVERY: u64 = 4;
/// A port is silent after this many ticks without a keepalive.
const QUIET_TICKS: u64 = 3;
const BEACON_EVERY_TICKS: u64 = 20;

const M_SRC: u32 = 1;
const M_DST: u32 = 2;
const M_DUP: u32 = 3;
const M_LOCAL: u32 = 4;
const M_EPOCH: u32 = 5;
const M_NEW: u32 = 6;
const M_HIT: u32 = 9;

const T_SRC: u32 = 1;
const T_DST: u32 = 2;
const T_EPOCH: u32 = 3;
const R1: u32 = 1;
const C1: u32 = 1;
const C2: u32 = 2;
const ROUTE_ENTRY: u64 = 9_001;
const EPOCH_ENTRY: u64 = 1;

// R1 layout: [(src & 7) * 256 + (dst & 255)] epoch stamps, then [2048 + src] last seq.
const R1_SEQ: u64 = 2048;
const R1_SAMPLE: u64 = 2112; // [2112 + src] packets seen, for 1-in-128 samples
const R1_SIZE: u32 = 2176;
const M_SAMPLE: u32 = 7;
// C1 layout.
const C1_PKTS: usize = 0; // [pidx*64 + src]: pidx = port & 3 for switch ports, 4 = local hosts
const LOCAL: usize = 4;
const C1_NEW: usize = 320;
const C1_DUP: usize = 384;
const C1_LOWTTL: usize = 448;
const C1_UDP: usize = 512;
const C1_SMALL: usize = 576; // size < 256
const C1_MID: usize = 640; // size < 640
const C1_KA: usize = 704; // [port & 7]
const C1_OUT: usize = 712; // bytes the harness sends out of switch port [p & 7]
const C1_DELIV: usize = 720; // [720 + src] packets handed to a local host here
const C1_TTL4: usize = 784; // [784 + src] TTL < 4 (ingress switch only)
const C1_SIZE: u32 = 848;
/// A port whose way out ran above this share of its capacity last window
/// is busy: reports queue behind customers there and may never leave.
const BUSY_PCT: u64 = 85;
const C2_SIZE: u32 = 1024; // [(src & 15) * 64 + dst]

#[derive(Default, Clone)]
struct PortState {
    nbr: Option<u32>,
    ka_last: u64,
    ka_changed_ns: u64,
    silent: bool,
    seen: bool,
    dmin: u64,
    dmax: u64,
    drops: u64,
    /// Drops in the last whole window: this port's way out is congested.
    last_drops: u64,
    cap_bps: u64,
    /// Share of capacity the harness used out of this port last window.
    util_pct: u64,
    /// Lowest keepalive delay since the last TAG_DELAY record (0 = none).
    pmin: u64,
}

pub struct Scope {
    me: u32,
    logger: bool,
    ports: Vec<u16>,
    port: HashMap<u16, PortState>,
    tick: u64,
    started: bool,
    wseq: u8,
    epoch: u64,
    /// The port toward the logging switch, and the beacon round it came in.
    parent: Option<u16>,
    round: u16,
    /// Hops to the logger this round, and the ports a copy came in on from
    /// a switch strictly closer to it (only those may be a parent: no loops).
    hops: u8,
    alts: Vec<u16>,
    /// Hops each entry of `alts` reported.
    alt_hops: Vec<u8>,
    held: VecDeque<Vec<u8>>,
    customers: Vec<(u32, u8)>,
    prev: Vec<u64>,
    /// At the logger: reports already written, so a copy sent two ways is
    /// written once. Bounded: cleared when it grows past 4096.
    written: std::collections::HashSet<Vec<u8>>,
    /// Sampled packets waiting to be reported, and (src /24, dst /24) pairs
    /// already reported: novel pairs go first.
    samples: VecDeque<[u8; 12]>,
    sampled_pairs: std::collections::HashSet<(u32, u32)>,
}

fn entry(id: u64, key: u64, plen: u8, action: TableActionW) -> TableEntryW {
    TableEntryW { id: wire::EntryIdW(id), key, prefix_len: plen, priority: 0, action }
}

fn set_meta(k: u32, v: u64) -> TableActionW {
    TableActionW::SetMeta { key: wire::MetaKeyW(k), value: v }
}

/// Skip our own reports and beacons (registers 0..2).
fn skip_ours(s: &mut asm::Stage) {
    s.load(0, F::IpProto).konst(1, PROTO_REPORT as u64).eq(2, 0, 1).bif(2, "end")
        .konst(1, PROTO_BEACON as u64).eq(2, 0, 1).bif(2, "end");
}

fn program(sample_mask: u64) -> TinyProgram {
    use asm::Stage;
    let mut stages = Vec::new();
    // 1. source customer.
    stages.push(Stage::new().load(0, F::IpSrc).table(T_SRC, 0, M_HIT).build(4, 1));
    // 2. destination customer, and our own packets.
    stages.push(Stage::new().load(0, F::IpDst).table(T_DST, 0, M_HIT + 1).build(4, 1));
    // 3. the window number.
    stages.push(Stage::new().konst(0, 0).table(T_EPOCH, 0, M_HIT + 2).build(4, 1));
    // 4. repeated seq; first sight of a destination this window.
    let mut s = Stage::new();
    skip_ours(&mut s);
    s.load(3, F::Kind).konst(4, 0).eq(5, 3, 4).bifnot(5, "end")
        .loadm(7, M_SRC).eq(8, 7, 4).bif(8, "end")
        .konst(6, R1_SEQ).add(6, 7, 6).rread(R1, 6, 9).load(10, F::Seq).rwrite(R1, 6, 10)
        .eq(11, 9, 10).eq(12, 10, 4).konst(6, 1).xor(12, 12, 6).and(11, 11, 12).storem(M_DUP, 11)
        .load(13, F::IngressPort).konst(14, 100).lt(15, 13, 14).konst(6, 1).xor(15, 15, 6).storem(M_LOCAL, 15)
        .bifnot(15, "end")
        // Every 128th packet of each local source is sampled.
        .konst(30, R1_SAMPLE).add(30, 7, 30).rread(R1, 30, 31).konst(32, 1).add(31, 31, 32).rwrite(R1, 30, 31)
        .konst(33, 127).and(34, 31, 33).eq(35, 34, 4).storem(M_SAMPLE, 35)
        .konst(16, 7).and(17, 7, 16).shl(18, 17, 8)
        .load(19, F::IpDst).konst(20, 255).and(21, 19, 20).add(18, 18, 21)
        .rread(R1, 18, 22).loadm(23, M_EPOCH).eq(24, 22, 23).bif(24, "end")
        .rwrite(R1, 18, 23).konst(25, 1).storem(M_NEW, 25)
        .label("end");
    stages.push(s.build(64, 6));
    // 5. packets by (source, ingress port class); keepalives by port.
    let mut s = Stage::new();
    skip_ours(&mut s);
    s.konst(14, 1).load(3, F::IngressPort)
        .load(0, F::Kind).konst(1, KIND_KEEPALIVE).eq(2, 0, 1).bif(2, "ka")
        .konst(4, 100).lt(5, 3, 4).bif(5, "sw")
        .konst(6, LOCAL as u64).jmp("havep")
        .label("sw").konst(8, 3).and(6, 3, 8)
        .label("havep")
        .shl(11, 6, 6).loadm(12, M_SRC).add(13, 12, 11).counter(C1, 13, 14)
        // Bytes out of each switch port (harness_egress is port + 1, 0 none);
        // a packet leaving by a host port (100+) is delivered here.
        .load(20, F::HarnessEgress).konst(21, 0).eq(22, 20, 21).bif(22, "end")
        .konst(23, 101).lt(24, 20, 23).bif(24, "swout")
        .konst(25, C1_DELIV as u64).add(26, 12, 25).counter(C1, 26, 14).jmp("end")
        .label("swout")
        .konst(25, 1).sub(26, 20, 25).konst(25, 7).and(26, 26, 25)
        .konst(25, C1_OUT as u64).add(26, 26, 25).load(27, F::Size).counter(C1, 26, 27)
        .jmp("end")
        .label("ka").konst(8, 7).and(6, 3, 8).konst(15, C1_KA as u64).add(16, 6, 15).counter(C1, 16, 14)
        .label("end");
    stages.push(s.build(64, 2));
    // 6. ingress switch only: new destinations, repeats, low TTL, UDP, sizes.
    let mut s = Stage::new();
    s.loadm(0, M_LOCAL).bifnot(0, "end")
        .loadm(1, M_SRC).konst(2, 0).eq(3, 1, 2).bif(3, "end")
        .konst(14, 1)
        .loadm(4, M_NEW).bifnot(4, "nonew").konst(6, C1_NEW as u64).add(9, 1, 6).counter(C1, 9, 14)
        .label("nonew")
        .loadm(8, M_DUP).bifnot(8, "nodup").konst(6, C1_DUP as u64).add(9, 1, 6).counter(C1, 9, 14)
        .label("nodup")
        .load(10, F::IpTtl).konst(6, 8).lt(11, 10, 6).bifnot(11, "nolow")
        .konst(6, C1_LOWTTL as u64).add(9, 1, 6).counter(C1, 9, 14)
        .konst(6, 4).lt(11, 10, 6).bifnot(11, "nolow")
        .konst(6, C1_TTL4 as u64).add(9, 1, 6).counter(C1, 9, 14)
        .label("nolow")
        .load(12, F::IpProto).konst(6, 17).eq(13, 12, 6).bifnot(13, "noudp")
        .konst(6, C1_UDP as u64).add(9, 1, 6).counter(C1, 9, 14)
        .label("noudp")
        .load(15, F::Size).konst(6, 256).lt(16, 15, 6).bifnot(16, "nosmall")
        .konst(6, C1_SMALL as u64).add(9, 1, 6).counter(C1, 9, 14)
        .label("nosmall")
        .konst(6, 640).lt(16, 15, 6).bifnot(16, "end")
        .konst(6, C1_MID as u64).add(9, 1, 6).counter(C1, 9, 14)
        .label("end");
    stages.push(s.build(64, 7));
    // 7. (source & 15) x destination, ingress switch only, data packets.
    let mut s = Stage::new();
    s.loadm(0, M_LOCAL).bifnot(0, "end")
        .loadm(4, M_SRC).konst(2, 0).eq(3, 4, 2).bif(3, "end")
        .konst(5, 15).and(6, 4, 5).shl(7, 6, 6).loadm(8, M_DST).add(9, 7, 8).konst(10, 1).counter(C2, 9, 10)
        .label("end");
    stages.push(s.build(24, 1));
    // 8. sample keepalives.
    let mut s = Stage::new();
    if option_env!("SCOPE_SAMPLES").is_some() {
        s.loadm(8, M_SAMPLE).bifnot(8, "nosample").punt(4).label("nosample");
    }
    s.load(0, F::Kind).konst(1, KIND_KEEPALIVE).eq(2, 0, 1).bifnot(2, "end")
        .load(3, F::Seq).konst(4, sample_mask).and(5, 3, 4).konst(6, 0).eq(7, 5, 6).bifnot(7, "end")
        .punt(1)
        .label("end");
    stages.push(s.build(16, 0));
    TinyProgram { stages }
}

fn at(v: &[u64], i: usize) -> u64 {
    v.get(i).copied().unwrap_or(0)
}

impl Scope {
    fn now10(ns: u64) -> u16 {
        (ns / 10_000_000).min(u16::MAX as u64) as u16
    }

    /// A record this switch made: written here, or sent toward the logger.
    fn emit(&mut self, now_ns: u64, tag: u8, body: Vec<u8>, out: &mut Vec<Action>) {
        if self.logger {
            out.push(actions::log_write(frame(tag, Self::now10(now_ns), &body)));
            return;
        }
        let mut p = Vec::with_capacity(body.len() + 1);
        p.push(tag);
        p.extend_from_slice(&body);
        // Link events are rare and are what claims hang on: with
        // `SCOPE_LINK_FLOOD`, send one copy out of every live port, each
        // then taking its neighbour's way up; the logger writes the first.
        if tag == TAG_LINK && option_env!("SCOPE_LINK_FLOOD").is_some() {
            let live: Vec<u16> = self.ports.iter().copied().filter(|q| self.port.get(q).is_some_and(|s| s.seen && !s.silent)).collect();
            if !live.is_empty() {
                for q in live {
                    self.send(q, p.clone(), out);
                }
                return;
            }
        }
        let dead = |q: &u16| self.port.get(q).is_some_and(|s| s.silent);
        match self.parent.filter(|q| !dead(q)) {
            Some(port) => {
                // A congested way out: send a second copy by the quietest
                // other live port. The logger writes the first to arrive.
                let busy = |q: &u16| {
                    self.port.get(q).map_or(0, |s| s.last_drops + if s.util_pct >= BUSY_PCT { s.util_pct } else { 0 })
                };
                if busy(&port) > 0 {
                    let alt = self
                        .ports
                        .iter()
                        .copied()
                        .filter(|q| *q != port && self.port.get(q).is_some_and(|s| s.seen && !s.silent))
                        .min_by_key(|q| busy(q));
                    if let Some(a) = alt {
                        self.send(a, p.clone(), out);
                    }
                }
                self.send(port, p, out)
            }
            None => {
                if self.held.len() >= 16 {
                    self.held.pop_front();
                }
                self.held.push_back(p);
            }
        }
    }

    fn send(&self, port: u16, payload: Vec<u8>, out: &mut Vec<Action>) {
        out.push(actions::inject_packet(port, SELF_BASE | self.me, REPORT_IP, PROTO_REPORT, 32, payload));
    }

    fn set_parent(&mut self, port: u16, out: &mut Vec<Action>) {
        if self.parent == Some(port) {
            // Same way up: still send whatever waited for it.
            if !self.port.get(&port).is_some_and(|s| s.silent) {
                while let Some(p) = self.held.pop_front() {
                    self.send(port, p, out);
                }
            }
            return;
        }
        self.parent = Some(port);
        out.push(actions::delete_entry(T_DST, ROUTE_ENTRY));
        out.push(actions::install_entry(
            T_DST,
            entry(ROUTE_ENTRY, REPORT_IP as u64, 32, TableActionW::SetEgress { port: wire::PortIdW(port) }),
        ));
        while let Some(p) = self.held.pop_front() {
            self.send(port, p, out);
        }
    }

    fn beacon(&self, round: u16, hops: u8, skip: Option<u16>, out: &mut Vec<Action>) {
        let quiet = |p: &u16| self.port.get(p).is_some_and(|s| s.silent);
        for &p in self.ports.iter().filter(|p| Some(**p) != skip && !quiet(p)) {
            out.push(actions::inject_packet(p, SELF_BASE | self.me, BEACON_IP, PROTO_BEACON, 32, vec![round as u8, (round >> 8) as u8, hops]));
        }
    }

    fn window(&mut self, now: u64, c1: &[u64], out: &mut Vec<Action>) {
        let mut b = vec![self.me as u8, self.wseq, 0, 0];
        b[2..4].copy_from_slice(&Self::now10(now).to_le_bytes());
        self.wseq = self.wseq.wrapping_add(1);
        // Customers-only claim mode (`SCOPE_CUST_ONLY`): no port section.
        let cust_only = option_env!("SCOPE_CUST_ONLY").is_some();
        b.push(if cust_only { 0 } else { self.ports.len() as u8 });
        let mut pb: Vec<u8> = Vec::new();
        for &p in &self.ports.clone() {
            let pidx = (p & 3) as usize;
            let st = self.port.entry(p).or_default();
            pb.push(p as u8);
            pb.push(st.nbr.map_or(255, |n| n as u8));
            pb.push(at(c1, C1_KA + (p & 7) as usize) as u8);
            put_varint(&mut pb, if st.dmax == 0 { 0 } else { st.dmin / 10_000 });
            put_varint(&mut pb, st.dmax / 10_000);
            put_varint(&mut pb, st.drops);
            // Packets in on this port this window, and how many sources sent them.
            let (mut pk, mut n) = (0u64, 0u8);
            for src in 0..64 {
                let i = C1_PKTS + pidx * 64 + src;
                let d = at(c1, i).saturating_sub(at(&self.prev, i));
                if d > 0 {
                    pk += d;
                    n = n.saturating_add(1);
                }
            }
            put_varint(&mut pb, pk);
            pb.push(n);
            let out_i = C1_OUT + (p & 7) as usize;
            let sent = at(c1, out_i).saturating_sub(at(&self.prev, out_i));
            let win_s_bits = st.cap_bps.max(1) * WINDOW_TICKS * TICK_NS / 1_000_000_000;
            st.util_pct = (sent * 8 * 100 / win_s_bits.max(1)).min(255);
            pb.push(st.util_pct as u8);
            st.dmin = u64::MAX;
            st.dmax = 0;
            st.last_drops = st.drops;
            st.drops = 0;
        }
        if !cust_only {
            b.extend_from_slice(&pb);
        }
        // Local customers: anything that entered from a host port.
        let base = C1_PKTS + LOCAL * 64;
        let srcs: Vec<usize> = (1..64).filter(|s| at(c1, base + s) > 0).collect();
        b.push(srcs.len() as u8);
        for s in srcs {
            let new = at(c1, C1_NEW + s).saturating_sub(at(&self.prev, C1_NEW + s));
            let dup = at(c1, C1_DUP + s);
            let low = at(c1, C1_LOWTTL + s);
            let udp = at(c1, C1_UDP + s);
            let mut flags = 0u8;
            if dup > 0 {
                flags |= CF_DUP;
            }
            if low > 0 {
                flags |= CF_LOWTTL;
            }
            let ttl4 = at(c1, C1_TTL4 + s);
            if udp > 0 {
                flags |= CF_UDP;
            }
            b.push(s as u8);
            b.push(flags);
            put_varint(&mut b, at(c1, base + s));
            put_varint(&mut b, at(c1, C1_SMALL + s));
            put_varint(&mut b, at(c1, C1_MID + s));
            put_varint(&mut b, new);
            if flags & CF_DUP != 0 {
                put_varint(&mut b, dup);
            }
            if flags & CF_LOWTTL != 0 {
                put_varint(&mut b, low);
                if option_env!("SCOPE_TTL4").is_some() {
                    put_varint(&mut b, ttl4);
                }
            }
            if flags & CF_UDP != 0 {
                put_varint(&mut b, udp);
            }
        }
        // Sources that came in on two or more switch ports this window.
        // With `SCOPE_MULTI_NBRS`, each entry also names the first two
        // neighbours it came from: `[src][nbr a][nbr b]`.
        let nbr_of = |pidx: usize| {
            self.ports.iter().find(|p| (**p & 3) as usize == pidx).and_then(|p| self.port.get(p)).and_then(|s| s.nbr).map_or(255, |n| n as u8)
        };
        let multi: Vec<(u8, Vec<usize>)> = (1..64)
            .map(|s| (s as u8, (0..4).filter(|p| at(c1, C1_PKTS + p * 64 + s) > at(&self.prev, C1_PKTS + p * 64 + s)).collect::<Vec<_>>()))
            .filter(|(_, ps)| ps.len() >= 2)
            .collect();
        b.push(multi.len() as u8);
        for (s, ps) in &multi {
            b.push(*s);
            if option_env!("SCOPE_MULTI_NBRS").is_some() {
                b.push(nbr_of(ps[0]));
                b.push(nbr_of(ps[1]));
            }
        }
        // `SCOPE_DELIVERED`: packets handed to local hosts, cumulative, by source.
        if option_env!("SCOPE_DELIVERED").is_some() {
            let got: Vec<usize> = (0..64).filter(|s| at(c1, C1_DELIV + s) > 0).collect();
            b.push(got.len() as u8);
            for s in got {
                b.push(s as u8);
                put_varint(&mut b, at(c1, C1_DELIV + s));
            }
        }
        b.truncate(250); // DESIGN.md sizes the worst case well below this
        self.prev = c1.to_vec();
        self.emit(now, TAG_WINDOW, b, out);
    }

    fn pairs(&mut self, now: u64, c2: &[u64], out: &mut Vec<Action>) {
        let mut items: Vec<(u8, u8, u64)> = Vec::new();
        for (i, &v) in c2.iter().enumerate() {
            if v > 0 {
                items.push(((i / 64) as u8, (i % 64) as u8, v));
            }
        }
        for chunk in items.chunks(40) {
            let mut b = vec![self.me as u8, 0, 0, chunk.len() as u8];
            b[1..3].copy_from_slice(&Self::now10(now).to_le_bytes());
            for &(g, d, v) in chunk {
                b.push(g);
                b.push(d);
                put_varint(&mut b, v);
            }
            self.emit(now, TAG_PAIRS, b, out);
        }
    }

    fn bump_epoch(&mut self, out: &mut Vec<Action>) {
        self.epoch += 1;
        out.push(actions::delete_entry(T_EPOCH, EPOCH_ENTRY));
        out.push(actions::install_entry(T_EPOCH, entry(EPOCH_ENTRY, 0, 0, set_meta(M_EPOCH, self.epoch))));
    }
}

impl SwitchProgram for Scope {
    fn init(switch_id: u32, local_ports: Vec<u16>) -> (Self, ProgramSetup) {
        Self::init_with(InitInput {
            switch_id,
            local_ports,
            port_capacity_bps: vec![],
            customers: vec![],
            observe: None,
            harness: None,
        })
    }

    fn init_with(input: InitInput) -> (Self, ProgramSetup) {
        let me = input.switch_id;
        let logger = me == LOGGER;
        let mut setup = ProgramSetup::new();
        setup.declare_table(T_SRC, MatchKind::Lpm, 80);
        setup.declare_table(T_DST, MatchKind::Lpm, 80);
        setup.declare_table(T_EPOCH, MatchKind::Exact, 2);
        setup.declare_register_array(R1, R1_SIZE);
        setup.declare_counter_array(C1, C1_SIZE);
        setup.declare_counter_array(C2, C2_SIZE);
        let customers: Vec<(u32, u8)> = input.customers.iter().take(63).map(|c| (c.prefix_addr, c.prefix_len)).collect();
        for (i, &(addr, plen)) in customers.iter().enumerate() {
            let idx = i as u64 + 1;
            setup.add_entry(T_SRC, entry(idx, addr as u64, plen, set_meta(M_SRC, idx)));
            setup.add_entry(T_DST, entry(idx, addr as u64, plen, set_meta(M_DST, idx)));
        }
        let punt = |r: u32| TableActionW::Punt { reason: wire::PuntReasonW::Custom(r) };
        setup.add_entry(T_DST, entry(9_100, BEACON_IP as u64, 32, punt(3)));
        if logger {
            setup.add_entry(T_DST, entry(ROUTE_ENTRY, REPORT_IP as u64, 32, punt(2)));
        }
        setup.add_entry(T_EPOCH, entry(EPOCH_ENTRY, 0, 0, set_meta(M_EPOCH, 1)));
        setup.set_program(program(if logger { 7 } else { 3 }));
        let ports: Vec<u16> = input.local_ports.iter().copied().filter(|p| *p < 100).collect();
        let mut port = HashMap::new();
        for (i, p) in input.local_ports.iter().enumerate() {
            let cap = input.port_capacity_bps.get(i).copied().unwrap_or(10_000_000);
            port.insert(*p, PortState { cap_bps: cap, ..Default::default() });
        }
        let me = Self {
            me,
            logger,
            ports,
            port,
            tick: 0,
            started: false,
            wseq: 0,
            epoch: 1,
            parent: None,
            round: 0,
            hops: u8::MAX,
            alts: Vec::new(),
            alt_hops: Vec::new(),
            held: VecDeque::new(),
            customers,
            prev: Vec::new(),
            written: std::collections::HashSet::new(),
            samples: VecDeque::new(),
            sampled_pairs: std::collections::HashSet::new(),
        };
        (me, setup)
    }

    fn on_punt(&mut self, ev: PuntEvent) -> Vec<Action> {
        let mut out = Vec::new();
        match ev.ip_proto {
            PROTO_REPORT => {
                if self.logger {
                    if self.written.len() > 4096 {
                        self.written.clear();
                    }
                    if !self.written.insert(ev.payload.clone()) {
                        return out;
                    }
                    if let Some((&tag, body)) = ev.payload.split_first() {
                        out.push(actions::log_write(frame(tag, Self::now10(ev.now_ns), body)));
                    }
                } else if let Some(p) = self.parent {
                    // Reached us before our route did: pass it on by hand.
                    self.send(p, ev.payload.clone(), &mut out);
                }
            }
            PROTO_BEACON => {
                if self.logger || ev.payload.len() < 3 {
                    return out;
                }
                let round = u16::from_le_bytes([ev.payload[0], ev.payload[1]]);
                let h = ev.payload[2];
                let p = ev.ingress_port;
                if round > self.round {
                    self.round = round;
                    self.hops = h.saturating_add(1);
                    self.alts.clear();
                    self.alt_hops.clear();
                    self.alts.push(p);
                    self.alt_hops.push(h);
                    self.beacon(round, self.hops, Some(p), &mut out);
                } else if round == self.round && h < self.hops && !self.alts.contains(&p) {
                    // Another way upstream; never from as far as we are.
                    self.alts.push(p);
                    self.alt_hops.push(h);
                } else {
                    return out;
                }
                // Parent: the first upstream port whose way out is not busy,
                // else the first upstream port.
                let ok = |q: &u16| {
                    self.port.get(q).is_none_or(|s| !s.silent && s.util_pct < BUSY_PCT && s.last_drops == 0)
                };
                let pick = (0..self.alts.len())
                    .filter(|&i| ok(&self.alts[i]))
                    .min_by_key(|&i| self.alt_hops[i])
                    .map_or(self.alts[0], |i| self.alts[i]);
                self.set_parent(pick, &mut out);
            }
            _ if matches!(ev.reason, PuntReason::Custom(4)) => {
                let key = (ev.ip_src >> 8, ev.ip_dst >> 8);
                let novel = self.sampled_pairs.insert(key);
                let mut rec = [0u8; 12];
                rec[0..4].copy_from_slice(&ev.ip_src.to_le_bytes());
                rec[4..8].copy_from_slice(&ev.ip_dst.to_le_bytes());
                rec[8] = ev.ip_proto;
                rec[9..11].copy_from_slice(&(ev.packet_size.min(65535) as u16).to_le_bytes());
                rec[11] = ev.ip_ttl;
                if novel {
                    self.samples.push_front(rec);
                } else if option_env!("SCOPE_SAMPLES_NOVEL").is_some() {
                    // Claim mode: only first sightings of a pair.
                } else if self.samples.len() < 4 {
                    self.samples.push_back(rec);
                }
                self.samples.truncate(8);
            }
            _ => {
                if let Some(k) = keepalive(&ev.payload) {
                    let st = self.port.entry(ev.ingress_port).or_default();
                    st.nbr = Some(k.switch);
                    let d = ev.arrived_ns.saturating_sub(k.sent_ns);
                    st.dmin = if st.dmin == 0 { d } else { st.dmin.min(d) };
                    st.dmax = st.dmax.max(d);
                    st.pmin = if st.pmin == 0 { d } else { st.pmin.min(d) };
                }
            }
        }
        out
    }

    fn on_timer(&mut self, ev: TimerEvent) -> Vec<Action> {
        let mut out = Vec::new();
        let now = ev.now_ns;
        if !self.started {
            self.started = true;
            if self.logger {
                out.push(actions::name_logging_switch());
                let mut h = vec![VERSION, self.me as u8];
                h.extend_from_slice(&((WINDOW_TICKS * TICK_NS / 1_000_000) as u16).to_le_bytes());
                h.extend_from_slice(&((TICK_NS / 1_000_000) as u16).to_le_bytes());
                h.push(self.customers.len() as u8);
                out.push(actions::log_write(frame(TAG_HEADER, 0, &h)));
                for (ci, chunk) in self.customers.chunks(40).enumerate() {
                    let mut b = vec![(ci * 40) as u8];
                    for &(a, l) in chunk {
                        b.extend_from_slice(&a.to_le_bytes());
                        b.push(l);
                    }
                    out.push(actions::log_write(frame(TAG_CUSTOMERS, 0, &b)));
                }
            }
            for &p in &self.ports {
                self.port.entry(p).or_default().ka_changed_ns = now;
            }
        }
        if self.logger && self.tick % BEACON_EVERY_TICKS == 0 {
            self.round = self.round.wrapping_add(1);
            self.beacon(self.round, 0, None, &mut out);
        }
        for d in &ev.port_drops {
            self.port.entry(d.port).or_default().drops += d.dropped as u64;
        }
        let c1: Vec<u64> = ev.counters.iter().find(|c| c.id.0 == C1).map_or_else(Vec::new, |c| c.values.clone());
        let c2: Vec<u64> = ev.counters.iter().find(|c| c.id.0 == C2).map_or_else(Vec::new, |c| c.values.clone());

        // Silence and recovery, from the exact keepalive counters.
        let mut events = Vec::new();
        for p in self.ports.clone() {
            let ka = at(&c1, C1_KA + (p & 7) as usize);
            let st = self.port.entry(p).or_default();
            if ka != st.ka_last {
                st.ka_last = ka;
                st.ka_changed_ns = now;
                st.seen = true;
                if st.silent {
                    st.silent = false;
                    events.push((p, st.nbr, false, now));
                }
            } else if st.seen && !st.silent && now.saturating_sub(st.ka_changed_ns) >= QUIET_TICKS * TICK_NS {
                st.silent = true;
                events.push((p, st.nbr, true, st.ka_changed_ns));
            }
        }
        // A silent parent: fall back first, to another port this round's
        // beacon came in on, else any port still speaking, so the news
        // itself does not go out the dead port.
        if let Some(p) = self.parent.filter(|p| self.port.get(p).is_some_and(|s| s.silent)) {
            let live = |a: &u16| *a != p && self.port.get(a).is_some_and(|s| s.seen && !s.silent);
            let alt = self.alts.iter().copied().find(|a| live(a));
            if let Some(a) = alt {
                self.set_parent(a, &mut out);
            }
        }
        for (p, nbr, silent, t) in events {
            let mut b = vec![self.me as u8, p as u8, nbr.map_or(255, |n| n as u8), silent as u8];
            b.extend_from_slice(&Self::now10(t).to_le_bytes());
            self.emit(now, TAG_LINK, b, &mut out);
        }

        if self.tick > 0 && self.tick % WINDOW_TICKS == (self.me as u64 * 7) % WINDOW_TICKS {
            if self.prev.is_empty() {
                self.prev = vec![0; c1.len()];
            }
            // Claim mode with delays (`SCOPE_DELAY=1`): every 5 windows.
            if option_env!("SCOPE_DELAY").is_some() && (self.tick / WINDOW_TICKS) % 5 == 0 {
                let mut b = vec![self.me as u8, 0, 0, 0];
                b[1..3].copy_from_slice(&Self::now10(now).to_le_bytes());
                for p in self.ports.clone() {
                    let st = self.port.entry(p).or_default();
                    if let Some(n) = st.nbr {
                        b.push(n as u8);
                        put_varint(&mut b, st.pmin / 10_000);
                        b[3] += 1;
                    }
                    st.pmin = 0;
                }
                self.emit(now, TAG_DELAY, b, &mut out);
            }
            if LINKS_ONLY {
                // Still measure utilisation for routing; write nothing.
                let mut sink = Vec::new();
                let parent = self.parent.take();
                let logger = std::mem::replace(&mut self.logger, false);
                self.window(now, &c1, &mut sink);
                self.parent = parent;
                self.logger = logger;
                self.held.clear();
                // `SCOPE_DROPS`: every port that refused packets this window.
                if option_env!("SCOPE_DROPS").is_some() {
                    let mut b = vec![self.me as u8, 0, 0, 0];
                    b[1..3].copy_from_slice(&Self::now10(now).to_le_bytes());
                    for p in self.ports.clone() {
                        let st = &self.port[&p];
                        // `SCOPE_UTIL_HI`: busy ports too, so a link's
                        // fast spells show between its dropping ones.
                        if st.last_drops > 0 || (option_env!("SCOPE_UTIL_HI").is_some() && st.util_pct >= 80) {
                            b.push(st.nbr.map_or(255, |n| n as u8));
                            put_varint(&mut b, st.last_drops);
                            b.push(st.util_pct.min(255) as u8);
                            b[3] += 1;
                        }
                    }
                    if b[3] > 0 {
                        self.emit(now, TAG_DROPS, b, &mut out);
                    }
                }
            } else {
                self.window(now, &c1, &mut out);
            }
            self.bump_epoch(&mut out);
            for _ in 0..if option_env!("SCOPE_SAMPLES_NOVEL").is_some() { 1 } else { 2 } {
                let Some(rec) = self.samples.pop_front() else { break };
                let mut b = vec![self.me as u8, 0, 0];
                b[1..3].copy_from_slice(&Self::now10(now).to_le_bytes());
                b.extend_from_slice(&rec);
                self.emit(now, TAG_SAMPLE, b, &mut out);
            }
            let w = self.tick / WINDOW_TICKS;
            if !LINKS_ONLY && option_env!("SCOPE_CUST_ONLY").is_none() && option_env!("SCOPE_NO_PAIRS").is_none() && w % PAIR_EVERY == (self.me as u64) % PAIR_EVERY && !c2.is_empty() {
                self.pairs(now, &c2, &mut out);
            }
        }
        self.tick += 1;
        out.push(actions::schedule_timer(TICK_NS));
        out
    }
}

switch_program!(Scope);

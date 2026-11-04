//! The scope log format, shared by the switch program and every detector.
//! Version 1. See programs/scope/FORMAT.md for the byte layout.
//!
//! The log is a sequence of frames: `[tag u8][len u8][arrive u16][body]`,
//! `len` counting `arrive` and `body`. `arrive` is when the logging switch
//! wrote the frame, in 10 ms units; the body carries its own observation
//! time. Numbers inside bodies are LEB128 varints unless said otherwise.

#![allow(dead_code)]

pub const VERSION: u8 = 1;

pub const TAG_HEADER: u8 = 0xA3;
/// A switch's window: its ports, then its local customers.
pub const TAG_WINDOW: u8 = 1;
/// A switch-to-switch port went quiet or spoke again.
pub const TAG_LINK: u8 = 2;
/// Source -> destination customer pair counts at the ingress switch.
pub const TAG_PAIRS: u8 = 3;
/// The logging switch saw a gap in a switch's report sequence numbers.
pub const TAG_LOST: u8 = 4;
/// Minimum one-way keepalive delay per port since the last such record:
/// `[sw u8][t_10ms u16][n u8] n x [nbr u8][dmin_10us varint]` (0 = no
/// sample). The time keeps two equal readings from looking like one copy.
pub const TAG_DELAY: u8 = 5;
/// Ports that refused packets last window: `[sw u8][t_10ms u16][n u8]
/// n x [nbr u8][drops varint][util_pct u8]`.
pub const TAG_DROPS: u8 = 6;
/// A sampled packet from a local host: `[sw u8][t_10ms u16][src u32]
/// [dst u32][proto u8][size u16][ttl u8]`.
pub const TAG_SAMPLE: u8 = 7;
/// Ports that received some but not all keepalives last window (1..=9):
/// `[sw u8][t_10ms u16][n u8] n x [nbr u8][count u8]`.
pub const TAG_KA: u8 = 8;
/// Unlisted local sources' data and acks last window, by host bucket
/// (source bits 1-9): `[sw u8][t_10ms u16][n u8] n x [bucket varint]
/// [data varint][acks varint]`; a bucket appears once it sent 3+ data.
pub const TAG_SERVER: u8 = 9;
/// Customers whose untold servers' replies outweigh their requests last
/// window: `[sw u8][t_10ms u16][n u8] n x [cust u8][req_pkts varint]
/// [req_bytes varint][reply_pkts varint][reply_bytes varint]`.
pub const TAG_AMP: u8 = 10;
/// The customer ranges the switches were told at init, in their index order.
pub const TAG_CUSTOMERS: u8 = 0xA4;

/// Customer flags in a window record: which optional fields follow.
pub const CF_DUP: u8 = 2;
pub const CF_LOWTTL: u8 = 4;
pub const CF_UDP: u8 = 8;

pub fn put_varint(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let b = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

/// A cursor over a body. Reads past the end return `None`.
pub struct Reader<'a> {
    pub b: &'a [u8],
    pub i: usize,
}

impl<'a> Reader<'a> {
    pub fn new(b: &'a [u8]) -> Self {
        Self { b, i: 0 }
    }
    pub fn u8(&mut self) -> Option<u8> {
        let v = *self.b.get(self.i)?;
        self.i += 1;
        Some(v)
    }
    pub fn u16(&mut self) -> Option<u16> {
        Some(self.u8()? as u16 | (self.u8()? as u16) << 8)
    }
    pub fn u32(&mut self) -> Option<u32> {
        Some(self.u16()? as u32 | (self.u16()? as u32) << 16)
    }
    pub fn var(&mut self) -> Option<u64> {
        let mut v = 0u64;
        for s in 0..10 {
            let b = self.u8()?;
            v |= ((b & 0x7f) as u64) << (7 * s);
            if b & 0x80 == 0 {
                return Some(v);
            }
        }
        None
    }
    pub fn done(&self) -> bool {
        self.i >= self.b.len()
    }
}

/// One frame of the log.
pub struct Frame<'a> {
    pub tag: u8,
    pub arrive_10ms: u16,
    pub body: &'a [u8],
}

/// Every whole frame in `log`; stops at the first that does not fit.
pub fn frames(log: &[u8]) -> Vec<Frame<'_>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 4 <= log.len() {
        let tag = log[i];
        let len = log[i + 1] as usize;
        if len < 2 || i + 2 + len > log.len() {
            break;
        }
        let arrive = log[i + 2] as u16 | (log[i + 3] as u16) << 8;
        out.push(Frame { tag, arrive_10ms: arrive, body: &log[i + 4..i + 2 + len] });
        i += 2 + len;
    }
    out
}

pub fn frame(tag: u8, arrive_10ms: u16, body: &[u8]) -> Vec<u8> {
    let mut f = Vec::with_capacity(body.len() + 4);
    f.push(tag);
    f.push((body.len() + 2).min(255) as u8);
    f.extend_from_slice(&arrive_10ms.to_le_bytes());
    f.extend_from_slice(body);
    f
}

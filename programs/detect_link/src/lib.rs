//! Detectors for the link-failure eggs seen in truth run 1, over scope's
//! TAG_LINK records (`SCOPE_LINKS_ONLY` claim logger or the full logger).
//! Which egg is chosen at build time: `DETECT_EGG=F1|F2|F3`.
//!
//! An outage is one end of a link silent for at least `MIN_MS`: a silent
//! record (`t` = last keepalive heard) to the next heard record on that
//! end, or open to the end of the run. Congestion alone made one-way gaps
//! of 0.4-0.6 s in the truth run; real outages there were 1.5 s and more.
//!
//! - F1: link 1-6 has at least 3 outages that each recovered (a flapping link).
//! - F2: links 0-1 and 10-11 both start an outage within `TOGETHER_MS`.
//! - F3: at least two of switch 5's neighbours (0, 6, 10) lose it within
//!   `TOGETHER_MS` (the switch, not one of its links).

#[path = "../../common/format.rs"]
#[allow(dead_code)]
mod format;
use format::*;

const MIN_MS: u32 = 1000;
const TOGETHER_MS: u32 = 500;
const RUN_END_MS: u32 = 40_000;

#[derive(Clone, Copy, Debug)]
pub struct Outage {
    pub sw: u8,
    pub nbr: u8,
    pub start: u32,
    pub end: Option<u32>,
}

/// Every outage of at least `MIN_MS`, per reporting end.
pub fn outages(log: &[u8]) -> Vec<Outage> {
    let mut open: Vec<Outage> = Vec::new();
    let mut done = Vec::new();
    for f in frames(log) {
        if f.tag != TAG_LINK || f.body.len() < 6 {
            continue;
        }
        let b = f.body;
        let (sw, nbr, silent) = (b[0], b[2], b[3] != 0);
        let t = (b[4] as u32 | (b[5] as u32) << 8) * 10;
        if nbr == 255 {
            continue;
        }
        if silent {
            if !open.iter().any(|o| o.sw == sw && o.nbr == nbr) {
                open.push(Outage { sw, nbr, start: t, end: None });
            }
        } else if let Some(i) = open.iter().position(|o| o.sw == sw && o.nbr == nbr) {
            let mut o = open.remove(i);
            o.end = Some(t);
            done.push(o);
        }
    }
    done.extend(open);
    done.retain(|o| o.end.unwrap_or(RUN_END_MS).saturating_sub(o.start) >= MIN_MS);
    done.sort_by_key(|o| o.start);
    done
}

fn on_link(o: &Outage, a: u8, b: u8) -> bool {
    (o.sw, o.nbr) == (a, b) || (o.sw, o.nbr) == (b, a)
}

/// Outages of link a-b, the two ends merged when they overlap.
fn link_outages(os: &[Outage], a: u8, b: u8) -> Vec<(u32, Option<u32>)> {
    let mut v: Vec<(u32, Option<u32>)> = Vec::new();
    for o in os.iter().filter(|o| on_link(o, a, b)) {
        let end = o.end.unwrap_or(RUN_END_MS);
        if let Some(last) = v.last_mut() {
            if o.start <= last.1.unwrap_or(RUN_END_MS) {
                if o.end.is_none() || last.1.is_some_and(|e| e < end) {
                    last.1 = o.end;
                }
                continue;
            }
        }
        v.push((o.start, o.end));
    }
    v
}

/// The claim logger is switch 6, so its own end of link 1-6 is always in
/// the log; switch 1's end is used when it arrives.
pub fn f1(log: &[u8]) -> bool {
    let os = outages(log);
    let end = |sw: u8, nbr: u8| os.iter().filter(|o| o.sw == sw && o.nbr == nbr && o.end.is_some()).count();
    // Switch 6's end is complete (it writes the log); a report from switch 1
    // can be lost, so its end is only the fallback.
    let mine = os.iter().any(|o| o.sw == 6 && o.nbr == 1);
    (if mine { end(6, 1) } else { end(1, 6) }) >= 3
}

pub fn f2(log: &[u8]) -> bool {
    let os = outages(log);
    let a = link_outages(&os, 0, 1);
    let b = link_outages(&os, 10, 11);
    a.iter().any(|(s, _)| b.iter().any(|(t, _)| s.abs_diff(*t) <= TOGETHER_MS))
}

pub fn f3(log: &[u8]) -> bool {
    let os: Vec<Outage> = outages(log).into_iter().filter(|o| o.nbr == 5 && [0, 6, 10].contains(&o.sw)).collect();
    os.iter().any(|o| {
        let mut who: Vec<u8> = os.iter().filter(|p| p.start.abs_diff(o.start) <= TOGETHER_MS).map(|p| p.sw).collect();
        who.sort_unstable();
        who.dedup();
        who.len() >= 2
    })
}

fn detect(log: &[u8]) -> bool {
    match option_env!("DETECT_EGG") {
        Some("F1") => f1(log),
        Some("F2") => f2(log),
        Some("F3") => f3(log),
        _ => false,
    }
}

switch_program_sdk::detect_program!(detect);

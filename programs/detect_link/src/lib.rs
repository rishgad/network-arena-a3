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
    outages_min(log, MIN_MS)
}

/// Every outage of at least `min_ms`, per reporting end.
pub fn outages_min(log: &[u8], min_ms: u32) -> Vec<Outage> {
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
    done.retain(|o| o.end.unwrap_or(RUN_END_MS).saturating_sub(o.start) >= min_ms);
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

/// SRLG (egg 19): links 0-1 and 10-11 both start an outage within
/// `TOGETHER_MS`. The claim logger is switch 11, so the 11-10 end is complete.
pub fn f2(log: &[u8]) -> bool {
    let os = outages(log);
    let pick = |pairs: &[(u8, u8)]| -> Vec<Outage> { os.iter().filter(|o| pairs.contains(&(o.sw, o.nbr))).copied().collect() };
    let a = pick(&[(0, 1), (1, 0)]);
    let b = pick(&[(11, 10), (10, 11)]);
    // Together: down at the same moment, or back at the same moment. A
    // report of 0-1 going down can be lost while its recovery arrives, so
    // a bare "heard" on 0-1 counts as the end of an outage too.
    let heard_01 = heard(log).into_iter().filter(|&(sw, nbr, _)| (sw, nbr) == (0, 1) || (sw, nbr) == (1, 0)).map(|h| h.2);
    let back_01: Vec<u32> = a.iter().filter_map(|x| x.end).chain(heard_01).collect();
    b.iter().any(|y| {
        a.iter().any(|x| x.start.abs_diff(y.start) <= TOGETHER_MS)
            || y.end.is_some_and(|f| back_01.iter().any(|e| e.abs_diff(f) <= TOGETHER_MS))
    })
}

/// Every heard record, (switch, neighbour, t).
fn heard(log: &[u8]) -> Vec<(u8, u8, u32)> {
    frames(log)
        .into_iter()
        .filter(|f| f.tag == TAG_LINK && f.body.len() >= 6 && f.body[3] == 0)
        .map(|f| (f.body[0], f.body[2], (f.body[4] as u32 | (f.body[5] as u32) << 8) * 10))
        .collect()
}

/// Rebooting switch (egg 20): switch 6 (the claim logger) loses switch 5
/// and gets it back, and another neighbour of 5 (0 or 10) loses it within
/// `TOGETHER_MS` of the same moment: the switch, not one link, and not for good.
pub fn f3(log: &[u8]) -> bool {
    let os = outages(log);
    // Every heard record, (switch, neighbour, t), including switch 5's own
    // ports coming back: what a rebooted switch says first.
    let mut heard: Vec<(u8, u8, u32)> = Vec::new();
    for f in frames(log) {
        if f.tag == TAG_LINK && f.body.len() >= 6 && f.body[3] == 0 {
            heard.push((f.body[0], f.body[2], (f.body[4] as u32 | (f.body[5] as u32) << 8) * 10));
        }
    }
    os.iter().filter(|o| o.sw == 6 && o.nbr == 5).any(|m| {
        let Some(end) = m.end else { return false };
        let down_too = os.iter().any(|o| o.nbr == 5 && (o.sw == 0 || o.sw == 10) && o.start.abs_diff(m.start) <= TOGETHER_MS);
        let back_too = heard.iter().any(|&(sw, nbr, t)| {
            t.abs_diff(end) <= TOGETHER_MS && ((nbr == 5 && (sw == 0 || sw == 10)) || (sw == 5 && nbr != 6))
        });
        down_too || back_too
    })
}

/// One-way slow link (egg 14), any link: its lowest one-way keepalive
/// delay is at least 5 ms one way and at least 4x the other way. The low
/// end ignores queueing (congestion raises the high end, not the floor).
pub fn slow_one_way(log: &[u8]) -> bool {
    // (sw, nbr) -> every per-record floor of the delay nbr -> sw, in us.
    let mut lo: std::collections::HashMap<(u8, u8), Vec<u32>> = std::collections::HashMap::new();
    for f in frames(log) {
        if f.tag != TAG_DELAY || f.body.len() < 4 {
            continue;
        }
        let mut r = Reader::new(&f.body[4..]);
        let sw = f.body[0];
        for _ in 0..f.body[3] {
            let (Some(nbr), Some(d)) = (r.u8(), r.var()) else { break };
            if d > 0 {
                lo.entry((sw, nbr)).or_default().push(d as u32 * 10);
            }
        }
    }
    // A planted delay is the same in every record; a full queue wobbles.
    // The floor is reached again and again (3+ records within 3% of it).
    let steady = |v: &Vec<u32>| {
        let mn = *v.iter().min().unwrap();
        v.iter().filter(|&&x| (x - mn) * 100 <= mn * 3).count() >= 3
    };
    lo.iter().any(|(&(a, b), slow)| {
        let floor = *slow.iter().min().unwrap();
        floor >= 5_000
            && steady(slow)
            && lo.get(&(b, a)).is_some_and(|other| *other.iter().min().unwrap() * 4 <= floor)
    })
}

/// Rebooting switch (egg 20), any switch: at least two of a switch's
/// neighbours lose it within `TOGETHER_MS` (or get it back within it), and
/// it comes back.
pub fn any_reboot(log: &[u8]) -> bool {
    let os = outages(log);
    let back = heard(log);
    (0u8..15).any(|x| {
        let lost: Vec<&Outage> = os.iter().filter(|o| o.nbr == x).collect();
        lost.iter().any(|m| {
            let Some(end) = m.end else { return false };
            let mut who: Vec<u8> = vec![m.sw];
            for o in &lost {
                if o.start.abs_diff(m.start) <= TOGETHER_MS {
                    who.push(o.sw);
                }
            }
            for &(sw, nbr, t) in &back {
                if nbr == x && t.abs_diff(end) <= TOGETHER_MS {
                    who.push(sw);
                }
            }
            who.sort_unstable();
            who.dedup();
            who.len() >= DEGREE[x as usize].saturating_sub(1).max(2)
        })
    })
}

/// Links per switch on the published map (SUBMITTING.md): a 3x5 grid.
const DEGREE: [usize; 15] = [2, 3, 3, 3, 2, 3, 4, 4, 4, 3, 2, 3, 3, 3, 2];

/// Shallow queue: some switch port refuses at least 5 packets a window,
/// in at least 5 windows, while running at no more than half its capacity
/// (ordinary congestion drops come with a full link).
pub fn shallow_queue(log: &[u8]) -> bool {
    let mut hits: std::collections::HashMap<(u8, u8), u32> = std::collections::HashMap::new();
    for f in frames(log) {
        if f.tag != TAG_DROPS || f.body.len() < 4 {
            continue;
        }
        let mut r = Reader::new(&f.body[4..]);
        for _ in 0..f.body[3] {
            let (Some(nbr), Some(d), Some(u)) = (r.u8(), r.var(), r.u8()) else { break };
            if d >= 5 && u <= 50 && nbr != 255 {
                *hits.entry((f.body[0], nbr)).or_default() += 1;
            }
        }
    }
    hits.values().any(|&n| n >= 5)
}

/// Flapping link, any link: some link of the published map has at least
/// three outages of `MIN_MS` or more that each came back.
pub fn any_flap(log: &[u8]) -> bool {
    let os = outages(log);
    // Both ends silent at once: a link down. Congestion starves one
    // direction's keepalives at a time, never both.
    LINKS.iter().any(|&(a, b)| {
        let ab: Vec<&Outage> = os.iter().filter(|o| o.sw == a && o.nbr == b && o.end.is_some()).collect();
        let ba: Vec<&Outage> = os.iter().filter(|o| o.sw == b && o.nbr == a && o.end.is_some()).collect();
        ab.iter()
            .filter(|x| ba.iter().any(|y| x.start < y.end.unwrap() && y.start < x.end.unwrap()))
            .count()
            >= 3
    })
}

/// The published map's links (SUBMITTING.md).
const LINKS: [(u8, u8); 22] = [
    (0, 1), (0, 5), (1, 2), (1, 6), (2, 3), (2, 7), (3, 4), (3, 8), (4, 9), (5, 6), (5, 10),
    (6, 7), (6, 11), (7, 8), (7, 12), (8, 9), (8, 13), (9, 14), (10, 11), (11, 12), (12, 13), (13, 14),
];

/// Rebooting switch v2, any switch: its neighbours lose it in outages of
/// `MIN_MS` or more that all overlap one another and all end (it came
/// back), from at least all-but-one of its neighbours (and at least two).
pub fn reboot_overlap(log: &[u8]) -> bool {
    let os = outages(log);
    (0u8..15).any(|x| {
        let lost: Vec<&Outage> = os.iter().filter(|o| o.nbr == x && o.end.is_some()).collect();
        let need = DEGREE[x as usize].saturating_sub(1).max(2);
        lost.iter().any(|m| {
            let (ms, me) = (m.start, m.end.unwrap());
            let mut who: Vec<u8> = lost
                .iter()
                .filter(|o| o.start < me && ms < o.end.unwrap() && o.start.abs_diff(ms) <= TOGETHER_MS)
                .map(|o| o.sw)
                .collect();
            who.sort_unstable();
            who.dedup();
            who.len() >= need
        })
    })
}

/// Rebooting switch v3, any switch, short reboots too: the links between
/// switch X and at least all-but-one of its neighbours (2 or more) go
/// silent in outages of 300 ms or more that overlap, start within
/// `TOGETHER_MS` and all end. Either end of a link may report it (X's own
/// report, or the neighbour's). Congestion silences one direction of one
/// link at a time, so it cannot make this.
pub fn reboot_short(log: &[u8]) -> bool {
    let os: Vec<Outage> = outages_min(log, 300).into_iter().filter(|o| o.end.is_some()).collect();
    (0u8..15).any(|x| {
        // Outages of X's links, labelled by the neighbour on the other end.
        let mine: Vec<(u8, u32, u32)> = os
            .iter()
            .filter_map(|o| {
                if o.nbr == x {
                    Some((o.sw, o.start, o.end.unwrap()))
                } else if o.sw == x {
                    Some((o.nbr, o.start, o.end.unwrap()))
                } else {
                    None
                }
            })
            .collect();
        let need = DEGREE[x as usize].saturating_sub(1).max(2);
        mine.iter().any(|&(_, ms, me)| {
            let mut who: Vec<u8> = mine
                .iter()
                .filter(|&&(_, s, e)| s < me && ms < e && s.abs_diff(ms) <= TOGETHER_MS)
                .map(|&(n, _, _)| n)
                .collect();
            who.sort_unstable();
            who.dedup();
            who.len() >= need
        })
    })
}

/// Rebooting switch v4: **every** link of some switch X goes down at once
/// and comes back. One outage on X's links (300 ms or more, either end
/// reporting) is the reference; each neighbour's link counts if either
/// end reports an outage starting within `TOGETHER_MS` of it that
/// overlaps it, or either end reports it heard again within `TOGETHER_MS`
/// of the reference's end (a lost "silent" report is common; its "heard"
/// still arrives). A switch losing only some of its links is not a reboot.
pub fn reboot_all_links(log: &[u8]) -> bool {
    let os: Vec<Outage> = outages_min(log, 300).into_iter().filter(|o| o.end.is_some()).collect();
    let back = heard(log);
    (0u8..15).any(|x| {
        let nbrs: Vec<u8> = LINKS.iter().filter_map(|&(a, b)| if a == x { Some(b) } else if b == x { Some(a) } else { None }).collect();
        // A two-link corner switch is too easy to fake by coincidence.
        if nbrs.len() < 3 {
            return false;
        }
        let on = |o: &Outage, n: u8| (o.sw == x && o.nbr == n) || (o.sw == n && o.nbr == x);
        os.iter().filter(|o| nbrs.iter().any(|&n| on(o, n))).any(|m| {
            let (ms, me) = (m.start, m.end.unwrap());
            let down = |n: u8| os.iter().any(|o| on(o, n) && o.start < me && ms < o.end.unwrap() && o.start.abs_diff(ms) <= TOGETHER_MS);
            let back_too = |n: u8| back.iter().any(|&(sw, nb, t)| ((sw == x && nb == n) || (sw == n && nb == x)) && t.abs_diff(me) <= TOGETHER_MS);
            // Every link down together; at most one known only by its recovery.
            let seen_down = nbrs.iter().filter(|&&n| down(n)).count();
            let only_back = nbrs.iter().filter(|&&n| !down(n) && back_too(n)).count();
            seen_down + only_back == nbrs.len() && only_back <= 1
        })
    })
}

/// Shallow queue v2: a port that drops while quiet and never while busy:
/// 10+ windows with 5+ drops at <= 50% utilisation, and at most 10% of its
/// drop windows above 50%. (Truth runs: 6->7 39/0; the congested 12->13 also has
/// 13-15 quiet-drop windows but 7-11 busy ones — v1 fired on it.)
pub fn shallow_queue2(log: &[u8]) -> bool {
    let mut quiet: std::collections::HashMap<(u8, u8), u32> = std::collections::HashMap::new();
    let mut busy: std::collections::HashMap<(u8, u8), u32> = std::collections::HashMap::new();
    for f in frames(log) {
        if f.tag != TAG_DROPS || f.body.len() < 4 {
            continue;
        }
        let mut r = Reader::new(&f.body[4..]);
        for _ in 0..f.body[3] {
            let (Some(nbr), Some(d), Some(u)) = (r.u8(), r.var(), r.u8()) else { break };
            if nbr == 255 {
                continue;
            }
            let k = (f.body[0], nbr);
            if u > 50 && d > 0 {
                *busy.entry(k).or_default() += 1;
            } else if d >= 5 {
                *quiet.entry(k).or_default() += 1;
            }
        }
    }
    // Busy drop windows at most 10% of all its drop windows (12->13: 28-44%).
    quiet.iter().any(|(k, &n)| {
        let b = busy.get(k).copied().unwrap_or(0);
        n >= 10 && b * 10 <= n + b
    })
}

/// Capacity schedule: a link's capacity is cut and restored, repeatedly.
/// In a cut it is full at a fraction of its speed: 20+ drops a window at
/// <= 60% of nominal utilisation. Between cuts it runs at >= 80%. Two or
/// more cuts that each end in a fast spell. (Truth runs: 12->13 capped at
/// ~4 Mb/s for 4 s in every 8.) A shallow queue never runs fast; ordinary
/// congestion drops only near full speed; a single permanent cut never
/// recovers.
pub fn capacity_steps(log: &[u8]) -> bool {
    let mut per: std::collections::HashMap<(u8, u8), Vec<(u32, u64, u8)>> = std::collections::HashMap::new();
    for f in frames(log) {
        if f.tag != TAG_DROPS || f.body.len() < 4 {
            continue;
        }
        let t = (f.body[1] as u32 | (f.body[2] as u32) << 8) * 10;
        let mut r = Reader::new(&f.body[4..]);
        for _ in 0..f.body[3] {
            let (Some(nbr), Some(d), Some(u)) = (r.u8(), r.var(), r.u8()) else { break };
            if nbr != 255 {
                per.entry((f.body[0], nbr)).or_default().push((t, d, u));
            }
        }
    }
    per.values_mut().any(|v| {
        v.sort_by_key(|x| x.0);
        v.dedup_by_key(|x| x.0);
        let (mut cuts, mut in_cut, mut low) = (0, false, 0);
        for &(_, d, u) in v.iter() {
            if d >= 20 && u <= 60 {
                in_cut = true;
                low += 1;
            } else if u >= 80 {
                if in_cut {
                    cuts += 1;
                }
                in_cut = false;
            }
        }
        cuts >= 2 && low >= 4
    })
}

fn detect(log: &[u8]) -> bool {
    match option_env!("DETECT_EGG") {
        Some("F1") => f1(log),
        Some("F2") => f2(log),
        Some("F3") => f3(log),
        Some("E14") => slow_one_way(log),
        Some("R") => any_reboot(log),
        Some("Q") => shallow_queue(log),
        Some("FLAP") => any_flap(log),
        Some("R2") => reboot_overlap(log),
        Some("R3") => reboot_short(log),
        Some("R4") => reboot_all_links(log),
        Some("Q2") => shallow_queue2(log),
        Some("CAP") => capacity_steps(log),
        _ => false,
    }
}

switch_program_sdk::detect_program!(detect);

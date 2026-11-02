# Supplied logs: what is verified, what is inferred

Files are untouched; sha256 in `evidence/manifest.json` re-verified 2026-11-02.
Provenance, run settings and logger source are **unknown**. Nothing here is a
detector input.

## Verified (mechanical)

| Fact | Evidence |
|---|---|
| Both logs parse exactly as `[tag u8][len u16 LE][payload]` | `tlv.py`: 2059 / 2082 records, consumed exactly |
| Tag counts log 1: 7:1 8:568 2:859 6:57 3:122 5:332 1:120 | `tlv.py` |
| Tag counts log 2: 7:1 8:286 11:44 3:128 2:543 9:392 12:392 10:260 1:36 | `tlv.py` |
| Tags 1,2,3,6,7,8 (and 5 in log 1) have one fixed length each; tags 9–12 vary | `tlv.py` length table |
| customers.txt files are byte-identical; 53 ranges, overlapping /32-in-/24, repeated names | `cmp`, `custmap.py` |
| Fields at tag2@4, tag3@4 are customer-map *network* addresses (/24 or /32); tag8@8,@12 are host addresses in the map (100%) | `fieldcheck.py` |
| One field per tag is ~monotone in log order with range ≈1000..39900 | `fieldcheck.py` |

## Inferred (structural, not proven) — confidence H/M/L

| Tag | Layout (LE) | Inferred meaning | Conf. | Why |
|---|---|---|---|---|
| 7 | u8,u32,u8 | header: version?=1, logging switch=7, ?=1 | M | single record at offset 0; 7 matches the switch flagged as noisy |
| 1 | sw u32, nbr u32, port u16, ms u32, flag u8 | port went silent (1) / heard again (0) | H | same shape as kit `broken_link` + neighbour; transitions pair up; both ends of a link agree |
| 2 | sw u32, prefix u32, ms u32, pkts u32, ? u32 (always 0), bytes u64 | cumulative per-source-range counters at ingress switch | H | bytes/pkts ≈ packet sizes (1000, 200, 100, 60); monotone per key |
| 3 | sw u32, prefix u32, ms0 u32, ms u32 | customer quiet from ms0 until ms | M | intervals match on/off cadence of known-bursty names (Amazon 8 s) |
| 5 | sw u32, nbr u32, port u16, ms u32, delay_us u32, ka_count u32, ? u32 | keepalive one-way delay & cumulative count per port | H | 551 µs = 500 µs + 64 B at 10 Mb/s; count ≈ 10/s |
| 6 | sw u32, port u16, ms u32, n u32 | port drops since last report | M | name-free; counts plausible |
| 8 | sw u32, ms u32, src u32, dst u32, 17 B | packet sample | M | addresses in map; rest undecoded |
| 9–12 | LEB128 varints | compact successors of 5 (9), ? (10,11,12) | L | `f0 1f`=4080, `29`=41 at matching positions |

Times are multiples of 102 ms → the producing logger ran on a ~102 ms timer.

## Leads (recurring in both logs at near-identical times)

Same plant times in both logs is consistent with the practice truth runs,
whose plants keep their times across A/B/C (only seed and load change).

| # | Observation | Log refs | Candidate |
|---|---|---|---|
| L1 | link 1–6 both ends silent ~0.6 s at 6.9, 10.9/11.0, 15.0, 19.0 s, then fine | tag 1, both logs | flapping link (repeated short `fail`) |
| L2 | links 0–1 and 10–11 silent together 22.95–30.09 s | tag 1, both | two links down ~7 s and back (one egg or two) |
| L3 | all of switch 5's links (0,6,10) silent 31.9–36.0/36.1 s | tag 1, both | switch 5 down ~4 s (or 3 links) |
| L4 | delay 2→1 ≈30 ms, 1→2 ≈2 ms | tag 5 (1,2),(2,1) | asymmetric `link_delay` 1–2 |
| L5 | delay 13↔14 ≈25 ms both ways | tag 5 | symmetric `link_delay` 13–14 |
| L6 | delay 12→13 0.55–80 ms, variable | tag 5 (13,12) | congestion / `link_capacity` drop on 12→13 |
| L7 | delay 6→11 ≈0.86 ms vs 0.55 ms | tag 5 (11,6) | small asymmetric delay (near-miss scale?) |
| L8 | switch 2 hears ~no keepalives from 1 or 7 | tag 5 (2,1),(2,7); tag 1 @1122 | keepalive gap / loss toward 2, or logger artefact at 2 |
| L9 | steady ~84 drops/s at sw 6 port 2; variable drops at sw 12 port 2 | tag 6 | persistent congestion / shallow `queue` |
| L10 | Sync (both ranges) silent until ~20 s, then on/off ~3 s gaps | tags 2,3 | late start / rate change |
| L11 | Fortnite player /32: quiet ~1.1 s every 5 s | tag 3 | periodic pause of one host |
| L12 | Netflix live /32: quiet ~1.9 s every 4 s; viewers 3+ dsts | tags 3,8 | live stream on/off (maybe normal) |
| L13 | Amazon Prime member /32 sends 200 B packets inside the 1000 B shoppers /24 | tag 2 | distinct host behaviour (fingerprint) |
| L14 | names: Mapper→"Mapper's target", Login×2, Vault/Ledger, Backup/Offsite | customers.txt | scan / brute-force / exfil themes (no log evidence yet) |

Inferred grid (from tag 5 neighbour pairs): rows 0–4, 5–9, 10–14; columns
linked vertically. Unconfirmed until the official map is published.

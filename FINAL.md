# A3 final status (2026-11-03)

**120 / 200 points (80% of 150) · 12 eggs · 25 of 30 claims used · 3 of 3 truth runs used.**

| Egg | Category | Level | Claim | Rule (detector) |
|---|---|---|---|---|
| 1 Netflix side channel | Customer | easy | 22 SIDE | large-packet sender with a steady 0.5-15% small-packet trickle |
| 7 The traceroute | Traffic | easy | 16 LADDER | >=50% TTL<8 with both TTL<4 and 4-7 common (a climbing ladder) |
| 8 The echo | Traffic | medium | 6 DUP | mid-size sender repeats >=30% of sequence numbers |
| 9 The microburst | Traffic | medium | 17 Q2 | port drops at <=50% util, <=10% of drop windows busy |
| 10 The shuffle | Traffic | medium | 10 SHUFFLE2 | split route whose two last links differ >=5 ms in delay floor |
| 13 Fortnite cheater | Traffic | hard | 11 LAG | >=4 dip-then-burst pairs summing to ~2x usual |
| 14 The one-way slow link | Link | medium | 4 E14 | steady delay floor >=5 ms one way, >=4x the other |
| 15 The breathing link | Link | hard | 19 CAP | >=2 cuts (drops at <=60% util) each followed by >=80% util |
| 16 The leaky link | Link | hard | 24 LOSS | >=6 partial-keepalive windows each way on a link that stays up |
| 18 The flapping link | Failure | medium | 13 FLAP | any link, both ends silent together, >=3 recovered outages |
| 19 SRLG | Failure | medium | 3 F2 | links 0-1 and 10-11 down (or back) within 0.5 s |
| 20 The rebooting switch | Failure | hard | 15 R4 | every link of a 3+-link switch down together and back |

Missed claims: F1, F3 (rules too specific); TTL (no ladder check); FLOOD x5
(near misses never matched); SHUFFLE v1 (no delay check); Q (12->13 false
positive); R2 (all-but-one links); PAUSE2 (Sync's start is likely config);
SLOW2 (13-14 delay belongs to the shuffle); SURGE (Steam, generic).

Ready but NOT sent (all true on all three truth logs, no crashes):
`submissions/claim-SURGESTEAM/` (~30%), `claim-PRIME/` (~20%),
`claim-LIVE/` (~15-20%), `claim-SURGE1/` (~15%), `claim-FLOOD6/` (~35%, the
user chose to stop on the flood).

Unwon: Customer 2-6, Traffic 11-12, Link 17 (by points:
2 easy, 3 medium, 3 hard). Every anomaly in the five hidden-network logs is
either won or one of the claims above; the rest needs measurements no
logger took.

Lessons: win = generic or fingerprinted rule + explicit rejection of the
nearest near miss + confirmation on the real truth logs. See HANDOFF.md and
KNOWLEDGE.md for the full record.

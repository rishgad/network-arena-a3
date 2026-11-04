# A3 final status (2026-11-03)

**128 / 200 points (85% of 150) · 14 eggs · 29 of 30 claims used · 3 of 3 truth runs used.**

Late wins: claim 28 HUNG2 won 17 (The hung server, easy); claim 29 AMP won
2 (Cloudflare one-way mirror, easy). Misses: 26 CFDDOS, 27 HUNG. Unsent
candidate: claim-DDOS (~35-40%, egg 11?). See HANDOFF.md ledger.

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

## Late lead (from another team's card: egg 11 "Cloudflare DDoS", egg 17 "The hung server")
`submissions/claim-CFDDOS/`: Cloudflare users send 100 B TCP at 350-525
pkt/s to 8 Cloudflare hosts all run (its twin Akamai users: 4-6 pkt/s).
Rule: small TCP sender, typical 200+ pkt/s, 6+ new destinations in 80%+ of
windows. Only Cloudflare users match, in all three truth runs; matrix 24/24.
"The hung server": no switch controller stalled (report sequence vs time),
no client collapsed, no reply stream dipped; not visible in our logs.


## New unsubmitted review candidate (2026-11-03)

State remains 128 points / 14 eggs / 29 claims used. No claim 30 uploaded.
See candidates/ddos-many-to-one/README.md: distributed, concentrated UDP
flood from an otherwise small-TCP customer; 118/118 simulated cases pass,
true on real truth2/3, empty false, largest log 18,768 B. Truth1 lacks the
required samples. This may be egg 11 but is not confirmed by a Canvas card.
Existing submissions/ files are preserved. Review the theory before using
the final claim. Earlier FINAL sections describe prior stages of the work.


## Comprehensive remaining-egg audit and final conjunction (latest)

Audited our truth1/2/3, both supplied TLV logs, and both richer other-team
decodes. Reproducible offline tool: analysis/scope/remaining_audit.py.
Summary: analysis/scope/REMAINING_EGGS_REVIEW.md. Raw other-team evidence
and the detailed derived audit JSON remain local, not published.

Newly checked: Cloudflare request samples form eight one-to-one pairs; the
Home-users UDP flood instead has 141/189 sampled source hosts converging
on one destination. Corrected Netflix live to 2 s on / 2 s off, Steam
bursts to 12-16 and 28-32 s. Dropbox pair has synchronized 1 s / 6 s
bursts, with Dropbox-2 staggered by 2 s. Akamai destination churn is real
but does not establish the practice scanner predicate. Old PRIME/LIVE
fixture was 0/24 because of invalid overlapping prefixes.

Prepared candidates/final-conjunction/ for human review, outside submissions/.
Handout Section 2 keeps other eggs on during a target's tests; SUBMITTING.md
explicitly permits combined rules. Final detector requires all three:
concentrated distributed UDP flood AND Steam double surge AND large-TCP
video fan-out with repeated dropouts. A conjunction can match any one
of those eggs, provided the other guards remain true. No mapping is proven.

Final pair passes 162/162 local cases: 132 primary, 16 network holdouts,
8 moved/renumbered/video-type holdouts, 6 flood-delivery holdouts. Max log
18,230 B. Final detector replayed on every log, true on real truth2/3,
false empty, unique against 38 earlier detectors. Truth1 lacks samples.

Earlier logger-2 combination missed two late Steam positives under network
failures because reports stopped before the second surge. Logger 7 writes
Steam locally and fixes both without relaxing the rule. Practice has no
switch 7: final artifact finishes without stops but writes empty logs;
nonempty logger-2 checks use byte-identical instrumentation source, and
full-grid cases verify the actual logger-7 pair.

Isolated crates: programs/scope_final_conjunction, programs/detect_final_conjunction.
Matrix: tests/claims/conjunction_matrix.py. Final checks:
results/conjunction-final-review/validation.json. Failed earlier results
are retained. Existing sources, evidence/, submissions/, and the standalone
DDoS candidate are preserved. State remains 128 points / 29 claims used;
no official upload has happened. Human reviews the theory before claim 30.

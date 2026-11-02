# A3 progress (team working notes)

Last updated 2026-11-02. Deadline: end of Tue 2026-11-03, Central.

## Official attempts

| | Used | Left | Notes |
|---|---|---|---|
| Truth runs | 0 | 3 | A run counts only when a log comes back; first 2 error runs free |
| Claims | **unknown** (not established) | ≤30 | one egg per claim; duplicates/non-installs don't count |

Nothing has been submitted. Every submission waits for an explicit go-ahead.

## Confirmed rules (SUBMITTING.md sha256 712c3fdb…, handout, kit source)

- Canvas: "A3 — Truth run" (switch.wasm) → team.log + customers.txt;
  "A3 — Claim" (switch.wasm + detect.wasm). One upload at a time.
- Caps: truth 64 KB (65,536 B); claim 24 KB (24,576 B); practice 16 KB.
- Map: 3×5 grid, switches 0–14, links 0-1 0-5 1-2 1-6 2-3 2-7 3-4 3-8 4-9
  5-6 5-10 6-7 6-11 7-8 7-12 8-9 8-13 9-14 10-11 11-12 12-13 13-14.
- 20 eggs: 4 easy ×4, 9 medium ×8, 7 hard ×16; 150 = 100%.
- Per switch: 80 punts/s (backlog 64), export 16 kB/s burst 8 kB (practice
  values; the real network's are told at init), report queue 4 packets,
  report ≤ packet size, 4 M fuel/call, ≤8 TinyVM stages, ≤64 instrs/stage,
  one memory resource per stage.
- **Undocumented, found in `src/wasm.rs::call_export`:** every event into
  the program (timer events with their counter snapshots included) must
  encode to ≤4,096 B or the switch program dies. Keep total counter slots
  ≈ <2,000.
- Harness: only explicit punts reach the controller (no NoRoute punts);
  switch ports are 0..n, app ports ≥100; reports go by our own tables;
  reports wait behind customer packets (strict priority), so a saturated
  link starves them; keepalives share the customer queue, so congestion
  alone can make keepalives vanish (a false "link down").
- Plant vocabulary in this kit (`src/sim/plant.rs`): side_flow, link_delay,
  wire_loss, link_capacity, fail, pause, scan; plus harness keepalive_gap,
  route via/backup/split, shallow queue; app echo, ttl_ladder/ttl_start,
  proto; switch failure. The `ping` preset is documented as "used by the
  eggs that need a light sender of their own on a quiet link". The hidden
  eggs may use mechanisms beyond these.
- In practice truth runs the plant times are identical across A/B/C; only
  seed and load change. Claim runs move the egg in time and reshuffle ranges.

## Evidence (evidence/, untouched, checksums re-verified)

See `analysis/evidence/SCHEMA.md`: framing verified for both logs; field
meanings inferred with H/M/L confidence; 14 leads. The inferred topology
from tag 5 matches the official map exactly (supports the tag-5 reading).
Strongest leads (both logs, same times): link 1–6 flapping ~0.6 s ×4;
links 0–1 + 10–11 down 22.95–30.1 s; all of switch 5 silent 31.9–36.1 s;
asymmetric delay 2→1 ≈30 ms; 13↔14 ≈25 ms; 12→13 variable to 80 ms;
6→11 +0.3 ms; Sync silent until ~20 s; Fortnite player /32 periodic quiet.

## Instruments

- `programs/scope/` — broad logger, format v1 (`programs/common/format.rs`,
  decoder `analysis/scope/scopelog.py`). 8-stage data plane counts every
  packet (no packet punts): per (source customer × ingress port class),
  local-sender extras (new destinations per window via an epoch-stamped
  register bitmap, repeated seq, TTL<8, UDP, size classes), (src&15)×dst
  pairs, keepalives per port, outbound bytes per port. Controller: 100 ms
  tick, staggered 1 s windows of cumulative counters + delay min/max +
  drops + outbound utilisation; immediate link silence/recovery events;
  reports routed by data-plane entries along a hop-count beacon tree.
- `tools/tracecheck/` — ground truth from a full simlog (report fate,
  link events, per-customer ingress per 100 ms). Local only.
- `worlds/local/gen_grid.py` — 15-switch grid proxy (budget/load proxy, not
  a replica).

## Validation so far

| Run | Log size | Windows delivered | Eggs visible |
|---|---|---|---|
| practice truth A/B/C (1.0/1.25/1.5), logger 1 | 11.3 KB | all (after util fix) | link 1–2 both ends; Spotify 12 s pause; scan 9–14 new dst/s |
| grid proxy s1, logger 7, load 1.0 / 1.5 | 25.3 KB | **2,3,4,8 stop ≈11.6 s; 9 loses ~50%** | — |

Baseline: kit broken_link wins egg 21 10/10 (`results/baseline/`).

## Open problem (blocks the first truth run)

Report delivery from the top-right of the grid fails under load: reports
are refused at full report queues on saturated links into the logger
(trace: refused at 2,3,4,8,9; customer drops S2:0, S7:1, S7:2). Options,
in order: (1) logger placement away from saturated links (the supplied
logs also had a blind spot at switch 2 with logger 7); (2) let a busy
switch route via an equal-hop neighbour (loop-safe tie-break by id);
(3) fewer, larger reports aggregated at relays; (4) proxy-world realism
check — my Steam/Backup placements may over-saturate.

## Hypotheses

Not started beyond the leads table; detectors start after the first truth run.

## Reproduce

```sh
cargo build --release                                   # simulator
(cd tools/tracecheck && cargo build --release)
(cd programs/scope && SCOPE_LOGGER=7 cargo build --release --target wasm32-unknown-unknown)
python3 -I analysis/evidence/tlv.py evidence/provided-01/team.log
python3 -I analysis/evidence/decode.py evidence/provided-01/team.log evidence/provided-01/customers.txt --tag 5
python3 -I worlds/local/gen_grid.py 1 > worlds/local/grid-s1.toml
./target/release/competitive_net_sim run-world worlds/local/grid-s1.toml --load-scale 1.5 \
  --program programs/scope/target/wasm32-unknown-unknown/release/scope.wasm --team-log g.team --log g.simlog
python3 -I analysis/scope/scopelog.py g.team --budget --links
tools/tracecheck/target/release/tracecheck g.simlog
```

Current scope build (logger 7, NOT truth-run ready):
`results/scope-v1/scope-L7.wasm` sha256 1fa7def3…f066.

## Next most useful action

Fix report delivery on the grid proxy (above), re-validate on the three
practice truth worlds at 1.0/1.25/1.5, then present the truth-run package.

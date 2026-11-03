# A3 handoff — state as of 2026-11-02 ~21:00 CT

Deadline: end of Tue 2026-11-03 (Central). Repo: github.com/rishgad/network-arena-a3.
Read this file, then `SUBMITTING.md`, `docs/a3-logging.md`, `docs/LIMITS.md`.
Submissions are uploaded to Canvas **by the human**, one at a time (a new
upload replaces a pending one). An agent prepares files; it cannot submit.

## Score and budgets

- Eggs won (7, 72 pts): 8 echo · 10 shuffle · 13 Fortnite cheater · 14 one-way
  slow link · 18 flapping link · 19 SRLG · **20 rebooting switch (claim 15, R4)**.
- Claims used: 15 of 30 (#15 R4 WON egg 20). Truth runs used: 2 of 3.
- Known to exist (another team won it): 20 · Failure · The rebooting switch.

| # | Claim dir | Result | Rule |
|---|---|---|---|
| 1 | claim-F1 | miss | link 1-6 ≥3 recovered outages (logger 6) |
| 2 | claim-F3 | miss | switch 5 reboot, ≥1 s, 2 nbrs, recovered |
| 3 | claim-F2 | **won 19** | links 0-1 & 10-11 down (or back) within 0.5 s |
| 4 | claim-E14 | **won 14** | any link: delay floor ≥5 ms, steady, ≥4x reverse |
| 5 | claim-TTL | miss | ≥50% of a sender's pkts TTL<8 |
| 6 | claim-DUP | **won 8** | mid-size sender repeats ≥30% seqs |
| 7 | claim-FLOOD | miss | sender at 10x usual for ≥8 s |
| 8 | claim-SHUFFLE | miss | sender on 2+ links of a switch ≥20 windows |
| 9 | claim-Q | miss | port drops ≥5/window at ≤50% util, 5+ windows |
| 10 | claim-SHUFFLE2 | **won 10** | split + the two last links' delay floors differ ≥5 ms |
| 11 | claim-LAG | **won 13** | ≥4 dip(≤60%)-then-burst(≥140%) pairs summing ≈2x |
| 12 | claim-FLOOD2 | miss | flood (10x, 8 s) to a /24 not sampled outside it |
| 13 | claim-FLAP | **won 18** | any link, both ends silent overlapping, ≥3 recovered |
| 14 | claim-R2 | miss | any switch, all-but-one nbrs, overlapping ≥1 s, recovered |
| — | claim-R3 | NOT SENT | fires on "2 of 3 links" near miss — rejected |
| 15 | claim-R4 | **won 20** | every link of a 3+-link switch down together (≥300 ms), back; ≤1 link known only by its recovery |

**Lesson from the wins:** each winner was (a) generic or tied to a measured
fingerprint, and (b) explicitly rejected the most plausible near miss
(both ends silent, unequal delays, mid-size sender, steady floor). Misses
mostly caught the right event but drew the line where a near miss sits.

## Hidden network facts (truth runs 1 and 2, identical plant times)

Map: 3x5 grid, 22 links (SUBMITTING.md). Base link: 10 Mb/s, 500 us; every
keepalive delay floor is 550 us except 2->1 30.05 ms, 1->2 2.05 ms, 13<->14
25.05 ms. Events: 1-6 down 5.9-7.5, 10.0-11.5, 14.0-15.5, 18.0-19.6 s; 0-1 and
10-11 down 21.93-30.09; all of switch 5's links silent 30.90-36.00 (its
controller kept running; neighbours rerouted). Ledger split route seen at 13
(via 8 and via 14). Home users -> Fastly flood 16.7-30 s (~30x). Fortnite
player /32 lag pattern every 5 s. Vault echo. Mapper TTL 8 to Mapper's target.
6->7 drops ~90/s at 22% util. 12->13 ~90% util, one-way keepalive gaps.
Unclaimed: Steam 80 pps + 4 s bursts at 11-15 and 27-31 s; Cam-A/B spikes
together at ~11, 25, 38 s; Sync silent until ~19 s; Netflix live 3 s on/1 s
off; Amazon Prime /32 steady while shoppers are 8/8 on/off.
`customers.txt` is identical in both truth runs and in `evidence/`.

## Code map

- `programs/scope/` — the switch program (one crate, many build-time modes via
  env vars, all read with `option_env!`, so **touch src/lib.rs before each
  build** and pass env vars explicitly, not through `env $1` in zsh):
  - `SCOPE_LOGGER=N` logging switch (default 0)
  - `SCOPE_LINKS_ONLY=1` write only link events (+ optional records below)
  - `SCOPE_LINK_FLOOD=1` send link events out every live port
  - `SCOPE_DELAY=1` TAG_DELAY min keepalive delay per port every 5 s
  - `SCOPE_DROPS=1` TAG_DROPS ports with drops + util (links-only mode)
  - `SCOPE_CUST_ONLY=1` windows without the port section, no pairs
  - `SCOPE_SAMPLES=1` 1-in-128 per-source packet samples (TAG_SAMPLE);
    `SCOPE_SAMPLES_NOVEL=1` only first sightings of a (src/24,dst/24) pair
  - `SCOPE_MULTI_NBRS=1` multi-path entries name the two neighbours
  - `SCOPE_DELIVERED=1` per-source packets delivered to local hosts
  - `SCOPE_NO_PAIRS=1` drop pair records
  Data plane: 8 TinyVM stages, built in Rust via `programs/common/asm.rs`.
  Log format: `programs/common/format.rs` (frames `[tag][len][arrive u16][body]`).
- `programs/detect_link/` — link/failure detectors, `DETECT_EGG=F1|F2|F3|E14|R|Q|FLAP|R2|R3|R4`.
- `programs/detect_cust/` — customer/traffic detectors, `DETECT_EGG=DUP|TTL|FLOOD|SHUFFLE|LAG|SHUFFLE2|FLOOD2`
  (`MULTI_NBRS=1` for SHUFFLE2).
- `analysis/scope/scopelog.py` — decoder: `--budget --links --ports --customers-table --samples --multipath`, `--customers customers.txt` to name ranges (offline only).
- `analysis/evidence/` — parsers for the supplied third-party logs (SCHEMA.md).
- `tools/tracecheck/` — ground truth from a local `.simlog` (report fate, link events).
- `worlds/local/gen_grid.py SEED` — 15-switch grid proxy. It is MORE congested
  than the real network (reports from switches 2/3/4/8 get lost); the claim
  matrices replace `steam` presets with 3 Mb/s netflix to calm it.
- `tests/claims/*.py` — per-egg matrices (positive / off / near misses, other
  eggs as confounders). Usage: `python3 -I tests/claims/<m>.py SWITCH DET...`.
- `submissions/` — every built claim (`claim-*/switch.wasm`, `detect.wasm`),
  `truth1/`, `truth2/` (logs + customers.txt), `truth3/switch.wasm` (unused:
  adds delivered-per-source counters; ~50 KB expected).

## Build / test recipe

```sh
cargo build --release                       # simulator (once)
(cd tools/tracecheck && cargo build --release)
cd programs/scope && touch src/lib.rs && SCOPE_LOGGER=7 SCOPE_LINKS_ONLY=1 \
  cargo build -q --release --target wasm32-unknown-unknown && cd ../..
cd programs/detect_link && touch src/lib.rs && DETECT_EGG=R4 \
  cargo build -q --release --target wasm32-unknown-unknown && cd ../..
./target/release/competitive_net_sim detect DET.wasm submissions/truth2/team.log
for r in a:1.0 b:1.25 c:1.5; do ./target/release/competitive_net_sim run-world \
  worlds/practice/truth/run-${r%%:*}.toml --load-scale ${r##*:} --program SW.wasm --team-log /tmp/x; done
```

Before handing over a claim: true on the truth logs where applicable, false
on an empty log, no "stopped" on the 3 practice truth worlds, log < 24 KB,
detect.wasm differs from every earlier claim's, matrix with near misses.

## Undocumented limits found

- Every event into the program (timer events with counter snapshots) must
  encode to ≤4096 B (`src/wasm.rs call_export`): keep counter slots ≲1,800.
- 64 instructions per TinyVM stage, one memory resource per stage.
- Reports wait behind customer traffic; a saturated link starves them.
- The logger de-duplicates reports by payload: every record needs a time.

## Next steps (ranked)

1. If R4 misses, the rebooting switch probably isn't "all links silent"; use
   truth run 3 or think about what else a reboot changes (queue flush,
   counters, a short outage, harness not rerouting).
2. Truth run 3 (`submissions/truth3/switch.wasm`): measures delivery per
   source — tests Mapper TTL expiry, black holes, wire loss on customer data.
3. Re-examine misses with near-miss thinking: flood (maybe defined by the
   victim, multiple sources), TTL (constant TTL 8 vs a traceroute ladder),
   shallow queue (maybe normal config, or defined per link).
4. Unclaimed anomalies above — decide which are normal behaviour.


## Takeover audit (2026-11-02; R4 still pending)

PROGRESS.md has been refreshed; its former zero-attempt ledger was stale.
No new upload or reply is inferred. Six eggs / 56 points remain confirmed.

- R4 rechecked true on both real truth logs, false on empty; no duplicate
  detector hash. Existing matrix remains 38/44 with six missed positives.
- Practice map has switches 0-5. Saved R4 (logger 7) and truth3 (logger 10)
  produce empty practice logs despite completing without stops. Use a
  practice-only logger-0 build to test instrumentation, and the saved
  logger-10 artifact on the 15-switch proxy. Do not swap upload binaries.
- Truth3 build reproduced byte-for-byte with SCOPE_LOGGER=10
  SCOPE_DELIVERED=1 SCOPE_NO_PAIRS=1 SCOPE_SAMPLES=1 (touch before build).
- Logger-0 truth3 variant passes all three practice truth loads with real
  logs and delivery sections. Saved truth3 passes four calm-grid checks
  at 48.8-49.8 KB, with all 15 switches reporting and no decode errors.
- `scopelog.py --delivery` now displays ingress/host-egress counts and
  times. Host-egress occurs before the final host queue/link, so interpret
  deficits as leads rather than proof of application loss. Original truth
  logs lack the new delivery section. Budget missing counts now use
  sequence span; first/last report loss still needs coverage inspection.
- See analysis/scope/NEXT_STEPS.md for ranked investigations and a truth3
  decision plan. No new claim is recommended for upload yet.
- Latest programs/scope/target build is the logger-0 practice variant;
  submissions/truth3/switch.wasm is unchanged and still logger 10.

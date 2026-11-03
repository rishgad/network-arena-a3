# Everything we know (knowledge dump, 2026-11-02 ~21:15 CT)

Companion to `HANDOFF.md` (state, code map, recipes). This file is the raw
knowledge: rules, measurements, reasoning, dead ends. Raw decodes:
`results/decoded/truth{1,2}-all.txt`, `results/decoded/customer-fingerprints.txt`.

## 1. Rules (verified from handout, SUBMITTING.md, kit source)

- 40 s simulated runs. 20 eggs: easy 4×4 pts, medium 9×8, hard 7×16 = 200;
  150 = 100% of A3. A4 bonus levels at 100/150/200.
- Claim = switch.wasm + detect.wasm, tried on every not-yet-won egg; wins
  at most one egg; must be right on all 10 fixed runs of that egg; runs mix
  on / off / near miss; all other eggs stay on. Duplicate pair or
  non-installing program does not use a claim; >1 h does.
- Truth run: 64 KB log back + customers.txt. Claim log cap 24 KB. Practice 16 KB.
- First 2 error runs free. One pending upload at a time (newest replaces).
- Truth runs share the deal (identical customers.txt in all our runs and in
  evidence/); plants keep their times across truth runs (verified: identical
  event times in truth 1 and 2 and the supplied logs). Claim runs move the
  tested egg in time and reshuffle ranges; other eggs presumably at usual times.
- Per switch: 80 punts/s, backlog 64; export 16 kB/s burst 8 kB (practice
  values; real ones arrive at init and behaved the same); report queue 4
  packets behind customer traffic (strict priority); report ≤ packet size.
- Undocumented: whole event into program ≤4096 B (counter snapshots
  included). ≤8 stages, ≤64 instrs/stage, 1 memory resource/stage, 4M fuel.
- Harness: only explicit punts reach controller; switch ports 0..n, host
  ports ≥100; reports use our own tables; keepalive 100 ms every switch port,
  64 B, payload {switch, port, seq, sent_ns}; failed link: no keepalives
  either way, harness reroutes via backup.
- Kit plant kinds (`src/sim/plant.rs`): side_flow, link_delay, wire_loss,
  link_capacity, fail(link), pause, scan; harness route via/backup/split,
  shallow queue, keepalive_gap; app echo, ttl_ladder, ttl_start, proto. The
  kit has fail_switch but NO restore_switch: the server's reboot is its own.
  `ping` preset doc: "used by the eggs that need a light sender of their own
  on a quiet link".

## 2. Hidden network (truth runs 1 & 2)

Map 3×5 grid (SUBMITTING.md). 30 sender ranges are given to switches at
init (receivers are NOT in that list → their packets read "unknown" dst).

### Links (keepalive delay floors per direction)
All 550 us (=500 us + 64 B at 10 Mb/s) except: 2→1 30,050 us; 1→2 2,050 us;
13→14 and 14→13 25,050 us. Floors are exactly constant for planted delays;
queueing makes them wobble (7→12 during Steam bursts, 12→13 all run up to 80 ms).

### Timeline (identical in both truth runs)
| t (s) | event |
|---|---|
| 5.9–7.5, 10.0–11.5, 14.0–15.5, 18.0–19.6 | link 1–6 down (both ends) — flapping link, WON 18 |
| 11–15, 27–31 | Steam 78 → 1,250 pkt/s bursts; 7→12 at 102% |
| ~11–13, ~25–27, ~38–39 | Cam-A and Cam-B both spike (+50%) |
| 16.7–30.0 | Home users → Fastly flood (86 → 2,586 pkt/s, 64 B UDP + 200 B TCP) |
| ~19 | Sync (both ranges) starts (silent before, 1 announce pkt at 0.7 s) |
| 21.93–30.09 | links 0–1 and 10–11 down together — SRLG, WON 19 |
| 30.90–36.00 | all links of switch 5 silent; its controller keeps running (seq 29→35 continuous, counters continue); neighbours' traffic to 5 drops to 0 and reroutes (0→1, 10→11 rise) |
| all run | Ledger split at 9, arriving at 13 via 8 and via 14 (+25 ms) — shuffle, WON 10 |
| all run | Fortnite player /32: every 5 s one window dips (24,19,15,10,5,0) and the next bursts by the same — WON 13 |
| all run | Vault repeats 50% of seqs — echo, WON 8 |
| all run | 6→7 ~90 drops/s at 22% util (claim 9 missed) |
| all run | 12→13 ~90% util, one-way keepalive gaps 0.4–0.6 s, drops |
| all run | Mapper → Mapper's target, TTL 7 at first hop (start 8), 64 B UDP, 25 pkt/s — identical to Ping→Ping's peer except TTL 63 |
| all run | Netflix live /32 3 s on / 1 s off to ~12 viewer hosts |
| all run | Amazon Prime member /32 steady 47 pkt/s 200 B inside shoppers' /24 (8 s on/off) |

Customer fingerprints (rate, size, UDP, dup, TTL, destinations):
`results/decoded/customer-fingerprints.txt`. Placement: Zoom@0,10;
Offsite/Ping/Sync×2@1; Akamai users/Cloudflare users/Fortnite/Fortnite
player/Home users/Jane Street/Ping's peer@2; Netflix/Netflix live/Vault@3;
Fortnite/Mapper@4; Cam-A/B@6; Amazon×2/Prime/Login/Steam@7; Ledger@9;
Dropbox×2@11; Backup/Dropbox-2@12.

Customer names in customers.txt hint at themes: Mapper / Mapper's target,
Login (×2, talk to each other), Ledger, Vault, Backup / Offsite, Cam-A/B,
Netflix live + viewers, Fortnite player, Amazon Prime member, Fastly,
Cloudflare (+users), Akamai (+users), Home users, Sync, Dropbox, Dropbox-2.

## 3. Eggs: won, known, candidates

Won: 8 echo (Traffic), 10 shuffle (Traffic), 13 Fortnite cheater (Traffic,
hard), 14 one-way slow link (Link), 18 flapping link (Failure), 19 SRLG
(Failure) = 56 pts: egg 13 is worth 16, the other five 8 each.
Another team's card also showed 20 · Failure · The rebooting switch.
Categories seen: Failure, Link, Traffic (no Customer egg won yet; the
practice set had a Customer egg — "the long pause").

Candidates not won, with what failed and the best current theory:
- **Rebooting switch (20)**: = switch 5 event. Misses: F3 (switch 5, ≥1 s,
  2 nbrs), R2 (any, all-but-one, ≥1 s). R3 rejected (fires on 2-of-3 links).
  R4 pending (every link of a 3+-link switch, ≥300 ms, back). Open theories:
  reboot is short; near misses "2 of 3 links", "stays down", "another switch".
- **Mapper TTL**: Mapper ≡ Ping except TTL → near-miss pair by design.
  Miss with "≥50% TTL<8". Theory: egg is packets *expiring* (TTL too small
  for the path); near miss TTL 8 that still arrives. Truth run 3's
  delivered-per-source counters answer this.
- **Flood (Home users → Fastly)**: misses with generic 10×/8 s and with
  "to an unusual /24". Theories: defined by the victim (Fastly receives),
  many hosts, or rate/duration bands; or Home users' surge is normal.
- **6→7 drops at low util**: miss. Theory: a shallow queue is normal config,
  or defined per link with other near misses (bursty).
- Unclaimed: Steam bursts, Cam spikes, Sync start, Netflix live, Prime /32,
  12→13 congestion, Netflix's occasional 200 B packet. Possibly normal.
- Unseen classes we cannot measure yet: wire loss on customer data, black
  hole, TTL expiry, keepalive gap with data flowing (looked: none), capacity
  step (looked: none), scan (looked: no source exceeds 3 new dst/window
  except Cloudflare users 8 and Netflix live 12, both steady all run).

## 4. Lessons

- Wins = generic or fingerprinted rule + explicit rejection of the nearest
  near miss. Misses = right event, line drawn where a near miss sits.
- Steady all-run quirks CAN be eggs (echo, shuffle, lag were all run long).
- Logging switch writes its own observations directly: put it at one end of
  the thing a claim needs (logger 6 for 1–6, 11 for SRLG, 13 for shuffle).
- Never let a record be byte-identical to another (logger dedup) — add time.
- Local proxy over-congests 2/3/4/8; real network delivered ≥34/39 windows
  from every switch in both truth runs.

## 5. Supplied evidence (third-party logs, `analysis/evidence/SCHEMA.md`)

Two logs from another logger (tags 1–12, TLV `[tag][len u16][payload]`),
logger at switch 7, same deal and same event times as ours; they also lost
switch 2's view. Two wasm files later supplied (`evidence/wasm/`, sha256
5073…, 1311…), not run. Their leads all matched ours.

## 6. Bugs we hit (avoid)

- zsh does not word-split `$1` → build env vars must be inline.
- option_env! changes need `touch src/lib.rs`.
- Timer event > 4096 B kills a switch (too many counter slots).
- Delay records without time were deduped → added time.
- Beacon tree loops when accepting non-upstream copies → hop counts.
- Held reports not flushed when the same parent came back → fixed.

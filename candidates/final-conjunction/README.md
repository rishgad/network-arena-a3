# Final conjunction — unsubmitted candidate for human review

**128/200 points, 14 eggs, 29/30 claims used, one claim left.** No official
attempt or new win has been inferred. The user uploads to Canvas. Every
historical file under submissions/ remains untouched.

## Why combine these rules?

Handout Section 2 states that each egg's ten runs have every other egg
switched on. SUBMITTING.md explicitly allows combining rules, with at most
one egg won per claim. Therefore a conjunction can hedge several theories:
all three must be present for true; removing one makes false. It can match
whichever one of those eggs has the right on/off/near-miss distinction.

This is an **AND**, not an OR. It does not guarantee an egg or multiply the
number of eggs won. An extra guard that fails in a positive run can lose the
claim. The tests specifically keep the other two candidate patterns on
while changing one target, then stress their telemetry under failures.

## Exact rule

The detector returns true only when all three are found in the team log:

1. **Distributed, concentrated UDP flood.** An otherwise small-TCP source
   surges >=5x for >=5 s, >=80% UDP during and <=20% outside. Six or more
   samples cover >=3 exported windows, >=4 source IPs, and >=80% go to one
   destination. This is the unchanged standalone DDoS predicate.
2. **Steam's repeated full-speed surges.** Identify the slowest steady
   large-packet sender by its measured baseline. It has >=2 surges lasting
   2–7.5 s at >=5x quiet and >=1,200 packets/s. A final surge cut off by run
   end may count after 2 s. Identity uses behavior, never its range or switch.
3. **Video fan-out with repeated dropouts.** A predominantly large TCP
   sender has median >=8 destinations/window and >=6 short zero-packet
   spells bounded by busy windows. The added size/protocol guard rejects
   small-packet and UDP fan-out near misses.

No customer names, customers.txt, or fixed customer addresses enter the
detector. Customer indices/ranges are taken only from the log's header.
Logger placement at switch 7 uses the published fixed network map.

## Evidence

- Other-team source samples: Home-users flood has 141 / 189 distinct
  senders, all to one host, at 15–30 s. Cloudflare request samples instead
  show eight separate one-to-one client/server pairs. "Cloudflare DDoS"
  remains an unconfirmed name/mapping for the observed Home/Fastly event.
- Steam fits ~62.5 / 78 quiet packets/s and ~1,250 during 12–16 and
  28–32 s. This is a stronger temporal measurement than the older report-
  window estimates, but the matching hidden egg is still unknown.
- Netflix live fits 2 s on / 2 s off with 12 viewers, not the earlier
  assumed 3 s on / 1 s off. It may be an egg or a normal customer profile.
- Real truth2/3: true. Empty log: false. Truth1: false because that logger
  recorded no packet samples; it cannot validate the flood sample guard.

## Instrumentation and failed experiments

The isolated scope_final_conjunction source is byte-identical to the
validated scope_ddos_final source, with **SCOPE_LOGGER=7** at build time.
It writes customer counters and sampled small UDP packets; its format is
unchanged. Steam's source counters are now written locally.

The first logger-2 conjunction passed its main 132 cases but missed two
late-burst positives with network failures. Reports from Steam's switch
stopped after 36.6 s, before the second burst. Moving the logger to 7 fixed
both without weakening the detector. Earlier smoke tests also exposed the
old proxy's unrealistically sparse cameras; the revised fixture uses the
measured steady camera backgrounds plus short synchronized bursts.
Failed outputs remain in results/conjunction-smoke/ and
results/conjunction-network/. They are not included in the final pass count.

Prime is deliberately excluded: the old PRIME/LIVE fixture was invalid
because the unmodified kit rejects overlapping app ranges. A /32 inside
the shoppers' /24 cannot be validated by that fixture.

## Verification

**162/162 cases pass; largest log 18,230 bytes (<24,576).**

Final results are in validation.json. Every matrix log is replayed against
the final detector bytes, with original simulator exits/stopped flags,
parser checks, log size and log hash preserved.

- 132 main cases: 22 target/near-miss scenarios, two seeds, three loads.
- 16 network holdouts: flaps, SRLG-like failures, reboot-like outages,
  asymmetric/symmetric delay and a shallow queue.
- 8 extra holdouts: moved/renumbered video sender, small TCP, large UDP.
- 6 delivery holdouts: moved flood source, early flood and long flood, with
  network confounders, at loads 1.0 and 1.5.
- Final logger-7 artifact completes all three practice truth worlds without
  stops. Practice has only switches 0–5, so these logs are empty. The
  byte-identical instrumentation with logger 2 produced nonempty practice
  logs; see practice-instrumentation-logger2.json. The 15-switch matrices
  validate actual logger-7 reporting and budget use.
- Detector differs from every earlier claim detector. Standalone DDoS files
  under candidates/ddos-many-to-one/ are preserved as a simpler alternative.

Local models cannot establish the hidden predicates, customer fingerprints
in every claim deal, or exact near-miss thresholds. This remains a candidate
for the human to review before spending claim 30.

SHA-256:

- switch.wasm: `de622aca4aa56142399b785da90f0e0258f52641b0fd547195b1cece52e20a35`
- detect.wasm: `f969bb6fd667cd4dcb763b43fa97ae0f9b4b928905031c48a149069f19868c85`

## Build and reproduce

```sh
(cd programs/scope_final_conjunction && touch src/lib.rs && SCOPE_LOGGER=7 SCOPE_CUST_ONLY=1 SCOPE_SAMPLES=1 cargo build -q --release --target wasm32-unknown-unknown)
(cd programs/detect_final_conjunction && touch src/lib.rs && DETECT_EGG=FINALAND FLOOD_SMALL=1 cargo build -q --release --target wasm32-unknown-unknown)
python3 -I tests/claims/conjunction_matrix.py candidates/final-conjunction/switch.wasm candidates/final-conjunction/detect.wasm --output results/conjunction-reproduce
```

The default matrix includes additional cases added after the first 132-case
batch. Use --cases to replay the exact cases listed in each saved summary.
Network holdouts use --network-confounders; see validation.json for seeds,
loads and case names. Do not rebuild into or overwrite submissions/.

If this candidate is chosen, the only two Canvas **A3 — Claim** upload files
are `switch.wasm` and `detect.wasm` in this folder. No upload has occurred.

# Unsubmitted review candidate: distributed, many-to-one UDP flood

Official state: 128/200 points, 14 eggs, 29/30 claims used, all truth runs used.
This candidate has NOT been submitted or assigned an official claim number.
No file in submissions/ was changed. Review the theory before spending the last claim.

## Theory and evidence

A normally small-TCP customer suddenly sends at least 5x its usual packet
rate for at least 5 seconds, with at least 80% UDP during the surge and at
most 20% outside it. During the surge, packet samples must independently
show at least 4 distinct source IPs and at least 80% directed at ONE victim.
Require at least 6 samples covering at least 3 exported time windows.

This adds the distributed-source and single-victim requirements absent
from the earlier unsuccessful flood claims. It is consistent with the
remaining Cloudflare DDoS egg, but the exact hidden predicate and egg
mapping are NOT confirmed. The name alone does not establish the victim.
It is also possible this is traffic egg 12 or neither egg.

Your truth2 log has 21 small-UDP Home-users samples from 21 source hosts;
truth3 has 18 from 18 hosts. All target the same host. In the other team's
two logs, the sampled flood spans 141 and 189 distinct source hosts, also
one victim. Customer names and addresses are used only for this offline
evidence analysis. The deployed detector reads only its team log, including
ranges written in the header; it contains no fixed customer addresses.

## What changed

- Separate crates programs/scope_ddos_final and programs/detect_ddos_final
  preserve the existing working sources. Each has its own format snapshot.
- Log customer counters and small (<=128 B) UDP samples only; logger 2.
- A bounded per-source 32-bit LCG selects about 1/128 packets from its high
  bits, avoiding the periodic sampler's demonstrated four-host aliasing.
- Samples carry EXPORT times, not original packet-arrival times. The
  detector allows up to 3 seconds after the rate window for the normal
  four-entry sample queue (two exports per ~1.02-second window).
- Rate intervals separated by a lost report are not joined into one surge.

## Verification

- Current detector: TRUE on real truth2 and truth3; FALSE on an empty log.
- Truth1 returns FALSE because that logger recorded no packet samples.
  It cannot validate the distribution/concentration guard; this is an
  unavailable measurement, not a claim that the target event was absent.
- 118/118 complete simulator-generated cases accepted by the final detector:
  102 primary cases (2 seeds, 3 loads) plus 16 holdout network-confounder cases.
  The final latency adjustment was checked by replaying the detector on
  those exact unchanged-switch logs, preserving simulation failure checks.
- Positives: normal/weaker/long/early floods, 4 and 8 hosts, moved source.
- Negatives: off, TCP flash crowd, short/weak surge, other customer types,
  single-host floods, and equal UDP load spread over 16 victims.
- Every primary world includes echo and TTL-ladder confounders. Holdouts
  additionally include flaps, SRLG-like failures, reboot-like outages,
  asymmetric and symmetric delays, and a shallow queue.
- Largest log: 18,768 bytes, below the 24,576-byte claim cap. No stopped
  switches in the matrices. Proxy log sizes are not a hidden-network bound.
- Final logger survives all three practice truth worlds at 1.0/1.25/1.5,
  with nonempty logs through 39,880 ms and no parser errors.
- Detector SHA256 differs from every existing submissions/claim-*/detect.wasm.

## Build and reproduce (repository root)

```sh
(cd programs/scope_ddos_final && touch src/lib.rs && SCOPE_LOGGER=2 SCOPE_CUST_ONLY=1 SCOPE_SAMPLES=1 cargo build -q --release --target wasm32-unknown-unknown)
(cd programs/detect_ddos_final && touch src/lib.rs && DETECT_EGG=DDOSFINAL FLOOD_SMALL=1 cargo build -q --release --target wasm32-unknown-unknown)
python3 -I tests/claims/ddos_final_matrix.py candidates/ddos-many-to-one/switch.wasm candidates/ddos-many-to-one/detect.wasm --seeds 91,92 --loads 1.0,1.25,1.5 --output results/claims-ddos-final-current
python3 -I tests/claims/ddos_final_matrix.py candidates/ddos-many-to-one/switch.wasm candidates/ddos-many-to-one/detect.wasm --seeds 93 --loads 1.0,1.5 --cases on,weaker,eight-hosts,moved-source,off,one-host,many-victims,flashcrowd --network-confounders --output results/claims-ddos-final-current-network
```

Earlier test failures are retained: periodic sampling missed two four-host
positives; the first LCG version missed one shorter-flood positive because
its samples left the queue after the measurement window. The final detector
passes all those saved cases; no near-miss expectation was changed.

## Artifact hashes

- switch.wasm: 5adbde9f626b7bfc63ea1540fdc907a449295bc71ce603c48d99b041ea7b8bb5
- detect.wasm: 03f732666d3a20f4da12162336dccb0e5ab489454c4cf707d5fb5c7b960ec915

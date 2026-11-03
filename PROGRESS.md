# A3 progress — 2026-11-03

Confirmed current state: **128/200 points, 14 eggs, 29/30 claims used,
all 3 truth runs used. ONE claim remains.** User uploads to Canvas.

Won: Customer 1 Netflix side channel, 2 Cloudflare one-way mirror;
Traffic 7 traceroute, 8 echo, 9 microburst, 10 shuffle, 13 Fortnite cheater;
Link 14 one-way slow link, 15 breathing link, 16 leaky link;
Failure 17 hung server, 18 flapping link, 19 SRLG, 20 rebooting switch.
Unwon: Customer 3-6; Traffic 11 Cloudflare DDoS and Traffic 12.

## Current work

Prepared an isolated, unsubmitted review candidate in
candidates/ddos-many-to-one/. Nothing in submissions/ was overwritten.
Existing uncommitted work in the main scope/detect_cust crates was preserved.
See the candidate README for the exact rule, code paths, build flags,
artifact hashes, evidence, limitations, and reproduction commands.

Theory: the failed flood claims missed the distribution/concentration
predicate. A normally small-TCP sender surges >=5x for >=5 s with >=80% UDP;
sampled flood traffic must show >=4 source IPs and >=80% to one victim.
The detector has no customer names or fixed customer addresses and reads
only its log. Whether this is the hidden egg 11 predicate remains unknown.

## Validation

- 118/118 simulator-generated cases pass the final detector: 102 primary
  cases across 2 seeds and 3 loads; 16 holdout cases with network confounders.
- Final latency adjustment checked by detector replay of those exact
  unchanged-switch logs. Original simulator exits and stopped flags checked.
- Negatives: off, TCP flash crowd, short/weak surge, other customer types,
  one-host flood, and equal UDP traffic spread over 16 victim hosts.
- Confounders: echo and TTL ladder in every case; holdouts add flapping,
  shared-risk failures, reboot-like outages, delays, and shallow queues.
- Largest log 18,768 B versus 24,576 B cap; no stopped switches.
- Real truth2/3 TRUE, empty FALSE. Truth1 lacks packet samples needed for
  this guard and returns FALSE; it cannot validate distribution/concentration.
- All three practice truth worlds at 1.0/1.25/1.5 produce nonempty logs
  through 39,880 ms, no stops, no parse errors.
- Final detector differs from every prior submissions/claim-*/detect.wasm.
- Results: results/ddos-final-review/validation.json, candidate validation.json,
  candidate practice-runtime.json. Earlier misses retained in test outputs.

## Lessons from this iteration

Periodic sampling missed hosts in a four-host positive even though all
four attacked. A bounded 32-bit per-source LCG sampled on high bits fixes
that aliasing. Only small UDP packets are exported to protect the log cap.
The normal four-entry sample queue exports two per ~1.02 s window;
export timestamps can lag observations, so the detector accounts for up
to 3 s after the rate window. A shorter-flood miss was resolved this way;
no near-miss label was changed.

## Next action

Human reviews the theory against the logs before spending claim 30.
No submission has been made or assumed. Preserve every historical file
under submissions/ and do not overwrite any of them.

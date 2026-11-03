# Next investigations (R4 won; truth3 ready)

2026-11-02. These are ranked investigations, NOT validated claims or upload
recommendations. No new claim package has been created. Official state:
15/30 claims used, #15 R4 won egg 20; 2/3 truth runs used; 7 eggs / 72 points.

## R4 confirmed

Claim 15 won egg 20 (+16 points); the user pasted the reply. See
submissions/claim-R4/canvas-reply.txt. No further reboot variants needed.
The previous 38/44 proxy score reflected six local missed positives; the
Canvas result is the authority for the hidden claim. Do not infer a unique
internal reboot mechanism from a successful observable predicate.

## Ranked candidates

| Rank | Investigation | Confidence in anomaly / exact hidden predicate | Main false cases to reject |
|---|---|---|---|
| 1 | Mapper: low TTL with actual path expiry or ladder | High / low-medium | Low but sufficient constant TTL; long paths with ordinary TTL; route changes; missing reports; congestion drops |
| 2 | Cam-A/B: synchronized additive bursts | High / low-medium | One bursty source; independent staggered bursts; equal average smooth traffic; ordinary shared bottleneck; one-off coincidence |
| 3 | Flood: distinguish victim-side attack from ordinary surge | High / low | Own-peer surge; unrelated victim; short burst; several benign senders; report congestion; normal customer load scaling |
| 4 | Shallow queue / loss at modest average use | Medium / low | Normal microburst queue overflow; reduced capacity; wire loss with no queue drops; a different port |

These rankings concern investigation value, not likelihood of a successful
submission. The previous TTL, FLOOD/FLOOD2, and Q claims already missed.
Do not submit renamed copies or arbitrary threshold variants.

### 1. Mapper / TTL

Both truth logs show almost all Mapper packets with observed TTL below 8:
773/774 and 967/968. Truth2's four source samples have TTL 7. This does NOT
prove a constant TTL: the logger samples each 128th packet, which can alias
a short periodic TTL ladder. Source counts alone also do not tell whether
packets expire before reaching the destination.

Truth3 can compare ingress to host-egress counts by source, locate loss
consistent with expiry, and compare with queue-drop/link evidence. It does
not record a TTL histogram or prove a ladder. Before claiming, test a
ladder, a constant low but sufficient TTL, a constant insufficient TTL,
ordinary long routes, queue drops, moved sources, and randomised ranges.
Use the truth evidence to decide which of these is actually the target.

### 2. Synchronized cameras

Both official logs show Cam-A and Cam-B co-located on switch 6, with
coincident excess traffic around 11, 25, and 38 s. Their baselines differ
and change between truth loads; the additive spikes remain similar.
This is stronger evidence than a single large sender, but whether the egg
is synchrony, microbursts, queue loss, or something else is unresolved.

Construct positive and staggered near-miss schedules at matched total
volume and per-source baselines. Keep other observed link/traffic events
as confounders. A generic correlation detector without customer
fingerprints may fire on normal jointly affected flows. One-second source
windows cannot precisely establish subsecond synchronization; use focused
instrumentation if that distinction matters.

### 3. Flood

Home users rises from about 69 to 2,069 packets/s in truth1, and about 86
to 2,586 in truth2; sampled traffic targets Fastly. FLOOD2 already rejects
its proxy own-peer near miss and passed 20/20 locally, yet missed on Canvas.
That local matrix establishes the chosen rule, not the hidden predicate.

Next distinguish customer fingerprint, victim identity/behavior, concurrent
sources, duration, and harm at the victim. Avoid treating an unseen sampled
destination as proof that it was absent earlier: sparse samples can miss
ordinary traffic. Truth3 could expose host-egress changes but cannot by
itself distinguish all victim/duration alternatives.

### 4. Shallow queue / loss

Truth evidence includes drops on 6->7 at roughly 22% mean use. Low mean
utilization does not exclude normal short bursts overflowing a normal
queue. The prior Q matrix has one missed positive and limited near misses;
it is not enough to establish an artificial shallow queue.

Compare matched microbursts on normal versus shallow queues, actual link
capacity changes, and wire loss with no queue drops. Truth3 can indicate
source deficits but cannot uniquely identify a queue's configuration.

### Lower priority

Steam's two four-second surges, both Sync senders' delayed activity,
Netflix-live's 3 s/1 s pattern, and Prime versus shoppers remain leads.
They may be normal presets or customer fingerprints. Do not consume claims
without a measured distinction and credible off/near-miss worlds.

## Last truth-run decision

R4 has won and the user instructed truth3 next. Recommend the human upload
the following unchanged, locally validated artifact; upload is not yet confirmed:

```
submissions/truth3/switch.wasm
SHA256 ac231c6b197e8d295e18caf03b8289d4d19844c5a12a2d102632aa7557efe34d
SCOPE_LOGGER=10 SCOPE_DELIVERED=1 SCOPE_NO_PAIRS=1 SCOPE_SAMPLES=1
```

It adds cumulative per-source host-egress counts, retains ingress counts,
ports/link events and sparse samples, and omits pair reports. On two calm
15-switch proxy seeds at loads 1.0/1.5 it emits 48,757-49,823 B with all 15
switches represented, no stops, and no parse errors. This leaves at least
15,713 B of cap headroom in those worlds, not a hidden-network guarantee.
The actual hidden network has more source entries and a heavier third
load. Per-window truncation and lost reports remain possible.

What it could teach us: which sources have fewer host-egress observations than
ingress packets, which destination switches report them, and whether
deficits correlate with link or queue evidence.
This supports the TTL, black-hole/loss, and victim-side investigations.
It cannot by itself identify a reboot, distinguish all causes of packet
loss, or resolve a TTL ladder. Explain that tradeoff before asking the
human to spend the final run. Only the human uploads it.

After a response arrives, preserve both returned files without replacing
prior evidence, then run from the repository root:

```sh
python3 -I analysis/scope/scopelog.py submissions/truth3/team.log --customers submissions/truth3/customers.txt --budget --links --ports --samples --customers-table --delivery
```

First check parser errors, trailing bytes, cap use, per-switch coverage,
latency, and late observations. Only then compare source and destination
counters over matched times (reports are asynchronous and cumulative).
The --delivery view prints each count and observation time; use parse()
for aligned interval analysis. Check source switches' announcements and
unknown-source traffic. Host-egress counters are taken BEFORE the final
queue/link, so deficits at a receiver can be larger than these counters
suggest. Never give customers.txt or fixed truth addresses to a detector.

## Before a new claim can be proposed

Require true on both real truth logs where the needed records exist,
false on empty, no stops on all three practice truth worlds, log below
24,576 B, unique detector hash versus all earlier claims, and a matrix
under tests/claims/ with yes/off/near-miss worlds and other eggs present.
A practice map lacking the chosen logger gives empty logs: use an explicit
practice variant for instrument validation as well as the saved upload
artifact for runtime checks and 15-switch tests. Existing failed local
cases must be explained and addressed, not hidden behind aggregate scores.

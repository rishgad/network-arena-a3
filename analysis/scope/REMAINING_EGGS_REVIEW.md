# Remaining-egg audit — November 3

State: **128/200 points, 14 eggs, 29/30 claims used, all truth runs used.**
No official upload or new win is inferred. Every historical submission is preserved.

The audit covers our truth1/2/3, the two original supplied TLV logs, and the
other team's run-A/run-B. `audit.json` records input hashes, source-side
customer summaries, schedules, samples and decoder coverage. Reproduce with:

```sh
python3 -I analysis/scope/remaining_audit.py --output /tmp/remaining-audit.json
```

## Strongest remaining event: distributed, concentrated UDP flood

At the originating switch in the other team's logs:

| Measurement | Run A | Run B |
|---|---:|---:|
| Small UDP flood samples | 157 | 325 |
| Distinct sampled source addresses | 141 | 189 |
| Destination addresses among those samples | 1 | 1 |
| Share to the single destination | 100% | 100% |

The first samples occur at 15.000551 s, the last just before 30 s. Main
source counters record 30,000 / 37,500 UDP packets. This is consistent with
2,000 / 2,500 packets/s for 15 s. Normal Home-users requests are 200-byte
TCP; flood traffic is 64-byte UDP. The shared host pool alone is normal;
the protocol change, surge, distribution and concentration form the theory.

Cloudflare's request samples show a different structure: eight clients each
paired with a different server, in both rich runs. Maximum observed senders
per destination is one, versus 141 / 189 for the Home-users flood. This
helps explain why a generic "many destinations" request detector was weak.
It does **not** establish the grading predicate. The remaining egg's name
"Cloudflare DDoS" still does not match the observed flood destination's
offline name, Fastly. Egg 11, egg 12, or another interpretation remain possible.

The existing `candidates/ddos-many-to-one/` pair is unchanged. Its independent
118-case validation and the actual Canvas result are different kinds of
evidence; there is no claim that its local score guarantees an egg.

## Corrected timings and alternatives

These are fitted to source-side counters. Detail and main rows overlap and
are never summed together. Fits integrate a schedule across each interval,
so an interval that straddles a boundary is not mistaken for a different rate.

| Lead | Measured pattern | Implication |
|---|---|---|
| Netflix live | 2 s on, 2 s off, 12 sampled viewers | Earlier 3-on/1-off account was wrong. The 2/2 model's relative RMS error is 0.24–0.39%; 3/1 is about 53.5%. This is a genuine schedule, not proof of an egg. |
| Steam | ~62.5 / 78 quiet packets/s, ~1,250 during 12–16 and 28–32 s | Both rate levels fit the counters closely. Earlier 11–15 / 27–31 timings described report windows rather than the event boundary. |
| Dropbox pair | Both send for 1 s every 6 s, beginning at 0.5 s | Strong synchrony to two receivers on switch 12. Dropbox-2 instead starts at 2.5 s. Normal configured schedules can reproduce this; synchrony alone is insufficient. |
| Sync pair | Begins at 20 s; 1 s on, 3 s off | Initial silence is not a recovered pause. Both clients also target switch 6. A generic synchrony detector would match both Sync and Dropbox. |
| Akamai users | 299-byte TCP; 6 / 14 distinct destinations in 9 / 18 request samples | Address churn is real, also corroborated by our sparse sample. It does not demonstrate the practice scanner's one-host, 50-address, 5-second condition. Ordinary configured fan-out remains a near miss. |
| Prime | Small steady /32 inside shoppers' bursty /24 | Still a lead, but its saved local matrix never ran valid worlds. |

The old `results/claims-32.txt` is **0/24 due to invalid overlapping
prefixes**, not evidence validating PRIME or LIVE. The kit rejects any
overlapping app prefixes; changing /30 to /32 does not fix that restriction.
No kit sources or old test result files were changed.

## Combining theories: a permissible experiment

Handout Section 2 says every other egg remains switched on while one egg is
tested. `SUBMITTING.md` explicitly discusses a detector combining several
rules, while still winning at most one egg.

Consequently, **AND**, rather than OR, can hedge several independently
plausible rules: all patterns must be present for true; removing one makes
false. Such a conjunction could match any one of those eggs' ten-run grids.
This works only if the additional guards reliably remain true in that grid's
positive runs. It does not identify the eggs, guarantee a win, or allow
multiple wins from one claim.

An isolated experiment in `programs/detect_final_conjunction/` combines:

1. The existing distributed, many-to-one UDP flood rule.
2. The existing Steam fingerprint plus two full-speed surges.
3. The existing fan-out sender with repeated short dropouts rule.

Prime is excluded because its local end-to-end fixture cannot be represented
by the unmodified kit. The combined experiment uses byte-identical DDoS
instrumentation source, built with **logger 7** so Steam's counters are
written locally. It returns true on
truth2/3 and false on empty. Truth1 has no packet samples and returns false.

The first smoke test failed two positives because the old grid proxy models
cameras as sparse low-rate apps; the Steam rule selected one as the slowest
sender. A revised fixture uses the measured steady camera backgrounds plus
brief synchronized bursts, and the smoke test passes 6/6. Both result sets
are retained. The first logger-2 build then passed 132/132 main cases but
missed two late Steam positives under network failures: its reports stopped
before the last burst. Logger 7 fixed this transport issue without weakening
the rule. The live guard also now requires large TCP packets, rejecting
small TCP and large UDP fan-out near misses.

The final pair passes **162/162** cases including network, moved/renumbered
video, and moved/early/long flood holdouts. Every saved simulator log was
replayed with the final detector bytes. Largest log: **18,230 bytes**.
The final logger survives all three practice truth worlds, which have no
switch 7 and therefore yield empty logs; logger-2 nonempty instrumentation
checks and full-grid logger-7 checks are both retained. Final artifacts are
in candidates/final-conjunction/. The standalone DDoS candidate is preserved
as the simpler alternative. No official claim has been submitted.

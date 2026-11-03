# Normal-customer baseline guidance (2026-11-03)

Source: user-provided Ed discussion #53, instructor replies Nov 2-3:
https://edstem.org/us/courses/102831/discussion/8472667
The instructor confirms customer eggs exist and points to the practice
worlds as a normal-behavior reference. The final anonymous interpretation
is not an instructor guarantee that all hidden customers/configurations
are identical to practice ones.

## What the local kit establishes

- worlds/practice/practice.toml explicitly describes a network with no egg.
- Amazon normally has 8 s on / 8 s off shopping bursts there.
- Spotify uses the netflix preset with AIMD; congestion response can change
  observed rates without a planted customer anomaly.
- Steam is normally unlimited download demand, constrained by its law,
  link, and congestion. A rate change alone does not identify an egg.
- src/world.rs::Preset and flow_for separate demand from congestion law:
  constant demand for Netflix/gaming/ping, on/off for Amazon, unlimited for
  Steam; Fixed/AIMD/RateMatch laws determine response.
- src/world.rs::AppSpec hosts and src/app.rs permit ordinary round-robin
  source hosts and destination hosts. Fan-out or many source hosts alone
  is not evidence of an attack.
- One configured app has a fixed transport protocol; congestion-control
  response does not by itself turn a TCP stream into UDP.

## Implication for the remaining candidates

Prime's steady /32 inside a bursty /24 and Netflix-live's regular fan-out
and gaps may be customer fingerprints, eggs, or both. The discussion does
not prove their labels. Their rules need off/near-miss worlds and an
identity that survives shuffled ranges. Do not reject them just because
the patterns look like ordinary application behavior.

The many-to-one DDoS candidate remains a joint predicate: an otherwise
small-TCP source abruptly has a sustained high-rate UDP surge, from many
hosts, concentrated on one destination. Its evidence is not just normal
bursts or a large client pool. The host pool is already present in normal
Home-users TCP traffic, so distribution is a distinguishing requirement,
not proof that a separate anomaly exists.

The local matrix already rejects off cases, a normal TCP flash crowd,
short/weak surges, a single attacking host, other source fingerprints, and
traffic spread across many destinations. The kit comparison supports those
controls; it does not establish that the hidden egg 11 uses the same rule.

No WASM changes or official attempts resulted from this discussion.
Current state remains 128 points, 29 claims used, one unused claim.

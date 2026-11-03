# Truth run 3 — ready for human upload

State: prepared; upload/result NOT confirmed. This is the final available truth run.
Upload exactly ONE file to Canvas **A3 — Truth run**:

`switch.wasm`

Size: 98,497 bytes.
SHA256: ac231c6b197e8d295e18caf03b8289d4d19844c5a12a2d102632aa7557efe34d
Logging switch: 10 (15-switch grid).
Build flags: SCOPE_LOGGER=10 SCOPE_DELIVERED=1 SCOPE_NO_PAIRS=1 SCOPE_SAMPLES=1
The saved artifact is byte-for-byte reproducible; do not use the current
crate target output, which is a logger-0 practice variant.

## Purpose

Compare each source's cumulative ingress with host-egress observations,
starting with Mapper versus Ping. Both are small UDP senders with similar
rates; Mapper samples have TTL 7 versus Ping 63. Determine whether Mapper
fails to reach a host-facing egress, then distinguish TTL expiry from queue
loss, routing, and missing reports. Also inspect deficits for other sources
and victim-side flood behavior. This run does not identify an egg by itself.

Host-egress means the harness selected an app port BEFORE its final queue
and link; it is not a confirmed application receipt. Observation times
are staggered; do not subtract unmatched snapshots. Source announcements
can count as local host-egress and need to be excluded for real delivery.

## Completed validation

- Saved binary completes all three practice truth worlds at 1.0/1.25/1.5
  without stops. Their switch IDs lack logger 10, so those logs are empty.
- Logger-0 practice variant with the same measurement flags produces
  real logs of 16,422 / 16,535 / 16,510 B, nonzero host-egress sections,
  and no stops or decoder errors on the same three worlds.
- Saved binary on calm 15-switch grids (seeds 11/12, loads 1.0/1.5):
  48,757 / 49,710 / 48,915 / 49,823 B logs, all 15 switches represented,
  no stops or decoder errors. Proxy size does not guarantee hidden size.
- Full metadata: results/takeover-audit/ from repository root.

## When Canvas replies

Return team.log and customers.txt plus the reply text. Do not upload a
second file set while this run is pending. Keep prior logs intact.
From repository root, after saving the response here:

```sh
python3 -I analysis/scope/scopelog.py submissions/truth3/team.log --customers submissions/truth3/customers.txt --budget --links --ports --samples --customers-table --delivery
```

First verify parse/cap/coverage, then align source and destination
snapshots. customers.txt is for offline analysis only.

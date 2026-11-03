# A3 progress (current team state)

Last updated 2026-11-02. Deadline: end of Tue 2026-11-03, Central.
User uploads to Canvas, one file set at a time. No agent submissions.

## Official attempts and score

| Item | Used | Left | State |
|---|---|---|---|
| Truth runs | 2 of 3 | 1 | `submissions/truth3/switch.wasm` built, unused |
| Claims | 15 of 30 | 15 | User confirms #15 R4 is pending; no reply yet |
| Score | 6 eggs / 56 points | 14 eggs unwon | Eggs 8, 10, 13, 14, 18, 19 |

The earlier zero-attempt progress notes were stale. Current user state
supersedes them. Do not infer an upload from a directory existing.
See HANDOFF.md for the full claim ledger. Do not replace a pending upload.

## Confirmed constraints

- Official log caps: truth 65,536 B, claim 24,576 B. Practice claim 16,384 B.
- Fixed 15-switch, 22-link grid; customer ranges reshuffled in claims.
- Detector reads only the team log. Customer maps are offline analysis aids.
- Do not change evidence/ or kit sources src/, switch_program_sdk/,
  switch_program_types/, examples/, worlds/practice/.
- Build switches use option_env!: touch the crate's src/lib.rs before each
  build; pass variables inline. Never use `env $1` in zsh.
- Incoming WASM event encodings must fit 4,096 B (including counter snapshots).
- Reports can be starved by customers. The proxy is more congested than
  real truth runs; proxy loss is not evidence of equivalent hidden loss.

## Takeover verification

- Read HANDOFF, SUBMITTING, logging API, limits, and previous progress in order.
- No official reply or new submission received during takeover.
- R4 saved detector: true on both official truth logs, false on empty log,
  SHA256 differs from every other saved claim detector.
- Existing R4 matrix: 38/44, six false negatives, no false positives. This
  is a limitation, not a fully passing validation. Do not resubmit R4.
- Saved R4 and truth3 switches complete practice truth A/B/C at
  1.0/1.25/1.5 without stops, but logs are EMPTY: their logging switches
  (7 and 10) do not exist on the six-switch practice map. This checks
  runtime survival only, not report delivery or detector operation.
- Rebuilt truth3 with explicit flags and reproduced its saved SHA256:
  ac231c6b197e8d295e18caf03b8289d4d19844c5a12a2d102632aa7557efe34d.
- Practice-only truth3 variant with logger 0: 16,422 / 16,535 / 16,510 B;
  no stops, decode errors, or trailing bytes; 196 windows with nonzero
  host-egress counters per world. These are truth worlds with a 64 KB cap.
- Unmodified saved truth3 on calm 15-switch grids, seeds 11/12 and loads
  1.0/1.5: 48,757 / 49,710 / 48,915 / 49,823 B, no stops or decode errors,
  all 15 switches represented, observations through 39,980 ms. These
  checks are local proxies, not a bound on hidden-network log size.
- Machine-readable summaries: results/takeover-audit/.

## Analysis changes

`analysis/scope/scopelog.py --delivery` now prints cumulative source ingress
and host-egress counters with each observation time. It does not compute
misleading loss ratios across asynchronous snapshots. The budget report
now uses observed sequence span rather than a fixed 39-window expectation,
which previously produced negative missing counts for 40-window switches.
Sequence span detects internal gaps only, not lost first/last reports.

Host-egress counters are incremented when the harness selects an app port,
before the final queue/link; they are NOT confirmed application delivery.
Announcements and unknown-source traffic can appear. Report loss and
unequal snapshot times must be accounted for before diagnosing data loss.

## Next actions

1. Wait for the user's claim #15 R4 reply. Keep 56 points until confirmed.
2. Work from the ranked investigations in analysis/scope/NEXT_STEPS.md;
   no new claim is upload-ready. Reproduce each predicate with off and
   near-miss cases before creating another submission package.
3. Hold truth3 for now. After R4, consider spending it to measure
   source-ingress versus host-egress deficits and where they occur.
   It will not directly establish a reboot or a TTL ladder.

## Reproduce truth3 build and analysis

From repository root:

```sh
(cd programs/scope && touch src/lib.rs && SCOPE_LOGGER=10 SCOPE_DELIVERED=1 SCOPE_NO_PAIRS=1 SCOPE_SAMPLES=1 cargo build -q --release --target wasm32-unknown-unknown)
python3 -I analysis/scope/scopelog.py submissions/truth2/team.log --customers submissions/truth2/customers.txt --budget --links --samples --customers-table --delivery
```

For a future truth3 response, use the same decoder with its returned
team.log and customers.txt. First inspect parse errors and per-switch
coverage, then compare counters at matched observation intervals. Old
truth logs do not contain the new delivery section.

For a six-switch practice variant use SCOPE_LOGGER=0 with the other flags
unchanged. Never replace the saved truth3 submission with that variant.
The latest local crate target currently contains the LOGGER=0 practice
variant; the saved submissions/truth3/switch.wasm remains LOGGER=10.

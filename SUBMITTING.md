# Submitting

You submit everything on Canvas, through three assignments. One person
uploads for the whole team. A server picks up each upload within a few
minutes and replies in the submission's comments.

| Canvas assignment | What you upload | What comes back |
|---|---|---|
| **A3 — Truth run** | one file: your `switch.wasm` | `team.log` and `customers.txt` |
| **A3 — Claim** | two files: `switch.wasm` and `detect.wasm` | the eggs you have won, and your points |
| **A2 — Hidden practice** | one file: your A2 switch program | an A2 report card, and the score it would have earned |

## Before you upload

Build both programs for `wasm32-unknown-unknown` in release mode, as the
README shows. The files to upload are the `.wasm` files in
`target/wasm32-unknown-unknown/release/`. Try them on the practice
network first, with `scripts/practice-check.py`.

**Send one upload at a time, and wait for its reply before the next.**
The server only looks at your team's newest upload, so a second upload
sent while the first is still waiting takes its place.

## The two log caps

The handout promised two numbers. Here they are:

| Your log in a... | Handout name | Cap |
|---|---|---|
| **truth run** | *T* | **64 KB** (65,536 bytes) |
| **claim** | *S* | **24 KB** (24,576 bytes) |

A write that does not fit is lost completely, not cut short. The
practice network caps the log at 16 KB (16,384 bytes), so a log that
fits there will fit in a claim.

## Truth runs: 3 per team

Upload your switch program to **A3 — Truth run**. Every egg is present
in all three runs. Each run has its own seed, and each is busier than
the one before.

The reply attaches two files:

- `team.log`: what your logging switch wrote, up to 64 KB;
- `customers.txt`: which address range belongs to which named customer,
  one per line.

That is all you ever get back. You never see the full log, or where any
egg is.

**What uses up a run.** A run is used when you get a log back. If your
program does not install, the reply tells you why, and the run is not
used.

Sometimes a program installs but the run does not finish: it crashes or
loops forever on some switch, or the run takes too long. The reply tells
you when this happens. **Your team's first two runs that end in an error
like this are free**: they do not use a truth run, and nothing comes
back. From the third error run on, each one uses one of your 3, and the
reply attaches whatever it can: what your switch wrote before it stopped,
or nothing if the run took too long or used too much memory.

So before each upload, run your switch on all three practice truth
worlds: `worlds/practice/truth/run-a.toml`, `run-b.toml` and `run-c.toml`,
at loads 1.0, 1.25 and 1.5, as the practice README shows. The real
network is bigger than the practice one, so passing there makes a crash
unlikely but cannot rule it out.

## Claims: 30 per team

Upload two files to **A3 — Claim**: your switch program, named
`switch.wasm`, and your detector, named `detect.wasm`. You don't say
which egg the claim is for. The server tries it on every egg you have
not won yet.

Each egg has 10 fixed runs. If your detector is right on all 10, you win
that egg, and once won, it stays won. **A claim wins one egg at most**,
so send a separate claim for each egg. A detector that combines several
eggs' rules still wins only one. On these runs your switch's log is
capped at 24 KB.

The reply lists the eggs you have won, by number, category and name,
with your points. It never says which egg your claim was tried on, how
far it got, or why it missed.

**What does not use a claim:** the wrong files, a program that does not
install, and the same two files as a claim you already sent.

**One hour at most.** A claim that is still running after an hour on the
server is stopped. It wins nothing, and it does use a claim. A claim
whose detector is right only on its own egg finishes well within the
hour. Only a very slow switch, with a detector that fits several eggs,
can run that long.

Most claims take a couple of minutes, some up to about ten. All claims
from the whole class wait in one queue, so don't leave yours for the
last few hours.

## Points

| Level | Eggs | Points each |
|---|---|---|
| easy | 4 | 4 |
| medium | 9 | 8 |
| hard | 7 | 16 |

That makes 200 points in all, and 150 is 100%. Canvas shows your points
out of 150 as you win eggs. Points above 150 count toward A4's bonus
levels (100, 150 and 200), and the reply shows your full total.

## The map

Every truth run and every claim runs on this network: switches 0 to 14
and the links between them. It is the same in every run. Link speeds and
delays are not given.

```
 0 ---- 1 ---- 2 ---- 3 ---- 4
 |      |      |      |      |
 5 ---- 6 ---- 7 ---- 8 ---- 9
 |      |      |      |      |
10 ---- 11 --- 12 --- 13 --- 14
```

Links: 0-1, 0-5, 1-2, 1-6, 2-3, 2-7, 3-4, 3-8, 4-9, 5-6, 5-10, 6-7, 6-11,
7-8, 7-12, 8-9, 8-13, 9-14, 10-11, 11-12, 12-13, 13-14.

## A2 hidden practice: 10 extra attempts on the hidden A2 worlds

Your A2 grade is final, but you can keep improving your A2 switch
program. Upload it to **A2 — Hidden practice** as one `.wasm` file, built the
same way as in A2. Each team gets **10 practice attempts**. They are
**ungraded**, so your A2 grade does not change.

A3 comes first. The server runs a practice attempt only when no truth
run or claim is waiting, and only one at a time, so when A3 is busy a
reply can take a while. You can send practice attempts until A3 closes.

## Deadline and questions

Truth runs and claims are taken until the end of Tuesday, November 3
(Central time). If you have any questions, email Ritul
(ritul@utexas.edu).
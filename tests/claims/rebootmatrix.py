#!/usr/bin/env python3
"""Claim matrix for the link eggs F1-F3 on the grid proxy.

    python3 -I tests/claims/linkmatrix.py SWITCH.wasm F1=det.wasm F2=det.wasm F3=det.wasm

Every world carries the *other* link eggs too (confounders), at moved
times; the target egg is on, off, or replaced by a near miss. Prints one
row per world and each detector's answer; exit 1 if any is wrong.
"""
import concurrent.futures as cf, random, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SIM = ROOT / "target/release/competitive_net_sim"
WORK = ROOT / "results/claims-matrix"
WORK.mkdir(parents=True, exist_ok=True)

def fail(a, b, at, dur=None):
    s = f'[[plant]]\nkind = "fail"\na = {a}\nb = {b}\nat_ms = {at}\n'
    return s + (f"for_ms = {dur}\n" if dur else "")

# Truth run 1 times; a claim run moves only the egg under test.
TRUTH = {"F1": 5900, "F2": 21930, "F3": 30900}

def egg(name, rng, fixed=False):
    t = TRUTH[name] if fixed else rng.randrange(4000, 20000)
    if fixed and name == "F2":
        return fail(0, 1, t, 8160) + fail(10, 11, t, 8160)
    if fixed and name == "F3":
        return fail(0, 5, t, 5100) + fail(5, 6, t, 5100) + fail(5, 10, t, 5100)
    if name == "F1":
        return "".join(fail(1, 6, t + k * 4000, 1600) for k in range(4))
    if name == "F2":
        d = rng.randrange(5000, 9000)
        return fail(0, 1, t, d) + fail(10, 11, t, d)
    if name == "F3":
        d = rng.randrange(3000, 6000)
        return fail(0, 5, t, d) + fail(5, 6, t, d) + fail(5, 10, t, d)

NEAR = {
    "F1": {"once": lambda r, t: fail(1, 6, t, 2000), "other-link": lambda r, t: "".join(fail(2, 7, t + k * 4000, 1600) for k in range(4)),
           "stays-down": lambda r, t: fail(1, 6, t), "short-flaps": lambda r, t: "".join(fail(1, 6, t + k * 4000, 1600) for k in range(2))},
    "F2": {"only-0-1": lambda r, t: fail(0, 1, t, 7000), "apart": lambda r, t: fail(0, 1, t, 4000) + fail(10, 11, t + 6000, 4000),
           "other-pair": lambda r, t: fail(2, 3, t, 7000) + fail(12, 13, t, 7000)},
    "F3": {"one-link": lambda r, t: fail(5, 6, t, 5000), "other-switch": lambda r, t: fail(2, 7, t, 5000) + fail(6, 7, t, 5000) + fail(7, 8, t, 5000) + fail(7, 12, t, 5000),
           "two-links-apart": lambda r, t: fail(0, 5, t, 3000) + fail(5, 10, t + 8000, 3000),
           "stays-down": lambda r, t: fail(0, 5, t) + fail(5, 6, t) + fail(5, 10, t)},
}

def worlds():
    out = []
    for target in TARGETS:
        for seed in (11, 12, 13):
            for load in (1.0, 1.5):
                for case in ["on", "off"] + list(NEAR[target]):
                    if case not in ("on", "off") and load == 1.0 and seed != 11:
                        continue
                    rng = random.Random(hash((target, seed, case)) & 0xffff)
                    base = subprocess.run(["python3", "-I", str(ROOT / "worlds/local/gen_grid.py"), str(seed)], capture_output=True, text=True).stdout
                    plants = "".join(egg(o, rng, fixed=True) for o in ("F1", "F2", "F3") if o != target)
                    if case == "on":
                        plants += egg(target, rng)
                    elif case != "off":
                        plants += NEAR[target][case](rng, rng.randrange(4000, 20000))
                    out.append((target, seed, load, case, base + "\n" + plants))
    return out

def run(sw, dets, w):
    target, seed, load, case, text = w
    name = f"{target}-s{seed}-l{load}-{case}"
    p = WORK / f"{name}.toml"; p.write_text(text)
    team = WORK / f"{name}.team"
    r = subprocess.run([SIM, "run-world", p, "--load-scale", str(load), "--program", sw, "--team-log", team], capture_output=True, text=True)
    if r.returncode or "stopped" in r.stdout + r.stderr:
        return name, target, case, {"run": "FAIL"}, team
    ans = {k: subprocess.run([SIM, "detect", d, team], capture_output=True, text=True).stdout.strip() for k, d in dets.items()}
    return name, target, case, ans, team

TARGETS = ("F1", "F2", "F3")
POSITIVE_NEAR = tuple(a.split("=", 1)[1] for a in __import__("os").environ.get("POSITIVE_NEAR", "").split(",") if "=" in a) or ()

def main():
    global TARGETS
    sw = sys.argv[1]
    dets = dict(a.split("=", 1) for a in sys.argv[2:])
    TARGETS = tuple(dets)
    wrong = 0
    with cf.ThreadPoolExecutor(8) as ex:
        rows = list(ex.map(lambda w: run(sw, dets, w), worlds()))
    for name, target, case, ans, team in rows:
        expect = {k: ("true" if (k == target and case in ("on",) + POSITIVE_NEAR) or (k != target) else "false") for k in dets}
        bad = [k for k in dets if ans.get(k) != expect[k]]
        wrong += bool(bad)
        size = team.stat().st_size if team.exists() else -1
        print(f"{name:<34} {size:>5}B  " + "  ".join(f"{k}={ans.get(k)}" for k in dets) + ("   WRONG " + ",".join(bad) if bad else "   ok"))
    print(f"\n{len(rows) - wrong}/{len(rows)} worlds fully right")
    return 1 if wrong else 0

sys.exit(main())

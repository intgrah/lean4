#!/usr/bin/env python3
import argparse
import os
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
LEAN = REPO / "build" / "release" / "stage1" / "bin" / "lean"
IR_RE = re.compile(rb"I\s+refs:\s+([\d,]+)")


def measure_ir(pile: str, bench: str, disabled: bool) -> int:
    lean_file = bench if bench.endswith(".lean") else f"{bench}.lean"
    pile_dir = REPO / "tests" / pile
    env = os.environ.copy()
    env["PATH"] = f"{LEAN.parent}:{env.get('PATH', '')}"
    if disabled:
        env["LEAN_RIIR_DISABLE"] = "1"
    else:
        env.pop("LEAN_RIIR_DISABLE", None)
    cmd = [
        "valgrind",
        "--tool=cachegrind",
        "--cachegrind-out-file=/dev/null",
        str(LEAN),
        "--root=..",
        "-DprintMessageEndPos=true",
        "-Dlinter.all=false",
        "-DElab.inServer=true",
        lean_file,
    ]
    result = subprocess.run(cmd, cwd=pile_dir, env=env, capture_output=True)
    if result.returncode != 0:
        sys.stderr.buffer.write(result.stdout)
        sys.stderr.buffer.write(result.stderr)
        raise SystemExit(
            f"bench failed (rc={result.returncode}): {pile}/{lean_file} disabled={disabled}"
        )
    match = IR_RE.search(result.stderr)
    if not match:
        sys.stderr.buffer.write(result.stderr)
        raise SystemExit(f"no cachegrind 'I refs' for {pile}/{lean_file} disabled={disabled}")
    return int(match.group(1).replace(b",", b""))


def render(rows, summary) -> None:
    tot_base = sum(r[1] for r in rows)
    tot_active = sum(r[2] for r in rows)
    tot_delta = tot_active - tot_base
    tot_pct = 100.0 * tot_delta / tot_base if tot_base else float("nan")
    lines = [
        "## RIIR `is_level_def_eq` instruction A/B (cachegrind Ir)",
        "",
        "Baseline = `LEAN_RIIR_DISABLE=1` (override passes through to Lean).",
        "Active = Rust dispatch on. Negative delta = fewer instructions = improvement.",
        "",
        "| bench | baseline (Ir) | active (Ir) | delta | % |",
        "|---|---:|---:|---:|---:|",
    ]
    for name, base, active in rows:
        delta = active - base
        pct = 100.0 * delta / base if base else float("nan")
        lines.append(f"| `{name}` | {base:,} | {active:,} | {delta:+,} | {pct:+.4f}% |")
    lines.append(
        f"| **total** | {tot_base:,} | {tot_active:,} | {tot_delta:+,} | {tot_pct:+.4f}% |"
    )
    table = "\n".join(lines)
    print(table)
    if summary:
        with open(summary, "a") as handle:
            handle.write(table + "\n")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Instruction-count A/B of the RIIR is_level_def_eq override "
        "(Rust dispatch active vs LEAN_RIIR_DISABLE passthrough) via cachegrind, "
        "which counts instructions by simulation and needs no hardware PMU."
    )
    parser.add_argument("--pile", default="elab_bench")
    parser.add_argument("--summary", default=os.environ.get("GITHUB_STEP_SUMMARY"))
    parser.add_argument("benches", nargs="+")
    args = parser.parse_args()

    if not LEAN.exists():
        raise SystemExit(f"lean binary not found (build stage1 first?): {LEAN}")

    rows = []
    for bench in args.benches:
        base = measure_ir(args.pile, bench, disabled=True)
        active = measure_ir(args.pile, bench, disabled=False)
        rows.append((bench, base, active))

    render(rows, args.summary)


if __name__ == "__main__":
    main()

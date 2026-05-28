#!/usr/bin/env python3
import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent


def read_instructions(jsonl: Path) -> float:
    if not jsonl.exists():
        raise SystemExit(f"no measurements written: {jsonl}")
    total = 0.0
    found = False
    for line in jsonl.read_text().splitlines():
        line = line.strip()
        if not line:
            continue
        data = json.loads(line)
        if str(data.get("metric", "")).endswith("//instructions"):
            total += float(data["value"])
            found = True
    if not found:
        raise SystemExit(f"no //instructions metric in {jsonl}")
    return total


def run_bench(env_wrapper: Path, pile: str, bench: str, disabled: bool) -> float:
    lean_file = bench if bench.endswith(".lean") else f"{bench}.lean"
    env = os.environ.copy()
    if disabled:
        env["LEAN_RIIR_DISABLE"] = "1"
    else:
        env.pop("LEAN_RIIR_DISABLE", None)
    jsonl = REPO / "tests" / pile / f"{lean_file}.measurements.jsonl"
    if jsonl.exists():
        jsonl.unlink()
    cmd = ["bash", str(env_wrapper), f"tests/{pile}/run_bench.sh", lean_file]
    result = subprocess.run(cmd, cwd=REPO, env=env)
    if result.returncode != 0:
        raise SystemExit(
            f"bench run failed (rc={result.returncode}): {pile}/{lean_file} disabled={disabled}"
        )
    return read_instructions(jsonl)


def render(rows, summary: str | None) -> None:
    tot_base = sum(r[1] for r in rows)
    tot_active = sum(r[2] for r in rows)
    tot_delta = tot_active - tot_base
    tot_pct = 100.0 * tot_delta / tot_base if tot_base else float("nan")
    lines = [
        "## RIIR `is_level_def_eq` instruction A/B",
        "",
        "Baseline = `LEAN_RIIR_DISABLE=1` (override passes through to Lean).",
        "Active = Rust dispatch on. Negative delta = fewer instructions = improvement.",
        "",
        "| bench | baseline (insns) | active (insns) | delta | % |",
        "|---|---:|---:|---:|---:|",
    ]
    for name, base, active in rows:
        delta = active - base
        pct = 100.0 * delta / base if base else float("nan")
        lines.append(f"| `{name}` | {base:,.0f} | {active:,.0f} | {delta:+,.0f} | {pct:+.3f}% |")
    lines.append(
        f"| **total** | {tot_base:,.0f} | {tot_active:,.0f} | {tot_delta:+,.0f} | {tot_pct:+.3f}% |"
    )
    table = "\n".join(lines)
    print(table)
    if summary:
        with open(summary, "a") as handle:
            handle.write(table + "\n")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Instruction-count A/B of the RIIR is_level_def_eq override "
        "(Rust dispatch active vs LEAN_RIIR_DISABLE passthrough) over bench files."
    )
    parser.add_argument("--env-wrapper", default="tests/with_stage1_bench_env.sh")
    parser.add_argument("--pile", default="elab_bench")
    parser.add_argument("--summary", default=os.environ.get("GITHUB_STEP_SUMMARY"))
    parser.add_argument("benches", nargs="+")
    args = parser.parse_args()

    env_wrapper = (REPO / args.env_wrapper).resolve()
    if not env_wrapper.exists():
        raise SystemExit(f"env wrapper not found (build stage1 first?): {env_wrapper}")

    rows = []
    for bench in args.benches:
        base = run_bench(env_wrapper, args.pile, bench, disabled=True)
        active = run_bench(env_wrapper, args.pile, bench, disabled=False)
        rows.append((bench, base, active))

    render(rows, args.summary)


if __name__ == "__main__":
    main()

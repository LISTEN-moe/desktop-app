#!/usr/bin/env python3
"""Measure a profile_widget run without extra Python packages (Linux only)."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import time


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--mode", choices=["idle", "audio", "visual"], default="visual")
    parser.add_argument("--seconds", type=int, default=30)
    parser.add_argument("--cover", type=Path)
    parser.add_argument("--original", action="store_true")
    parser.add_argument("--output", type=Path, default=Path("/tmp/listen-moe-profile"))
    args = parser.parse_args()
    if args.seconds < 10:
        parser.error("Use at least 10 seconds to exclude startup")
    if args.original and args.cover is None:
        parser.error("--original requires --cover")
    command = [str(args.binary.resolve()), args.mode, str(args.seconds)]
    if args.cover:
        command.append(str(args.cover.resolve()))
    if args.original:
        command.append("original")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    hz = os.sysconf("SC_CLK_TCK")
    samples: list[dict[str, float]] = []
    with args.output.with_suffix(".log").open("w") as log:
        process = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
        root = Path(f"/proc/{process.pid}")

        def cpu_ticks() -> int:
            # comm is parenthesized and may contain spaces.
            fields = (root / "stat").read_text().rpartition(")")[2].split()
            return int(fields[11]) + int(fields[12])

        previous_ticks, previous_time = cpu_ticks(), time.monotonic()
        try:
            while process.poll() is None:
                time.sleep(1)
                try:
                    now, ticks = time.monotonic(), cpu_ticks()
                    status = dict(line.split(":", 1) for line in (root / "status").read_text().splitlines())
                    samples.append({
                        "cpu": (ticks - previous_ticks) / hz / (now - previous_time) * 100,
                        "rss_mib": int(status["VmRSS"].split()[0]) / 1024,
                    })
                    previous_ticks, previous_time = ticks, now
                    if len(samples) == args.seconds - 3:
                        args.output.with_suffix(".smaps").write_text((root / "smaps").read_text())
                        args.output.with_suffix(".rollup").write_text((root / "smaps_rollup").read_text())
                except (FileNotFoundError, ProcessLookupError, KeyError):
                    break
        finally:
            if process.poll() is None:
                process.terminate()
            process.wait()
    steady = samples[6:-2]
    if not steady or process.returncode:
        raise SystemExit(f"Profile failed; inspect {args.output.with_suffix('.log')}")
    summary = {
        "command": command,
        "cpu_mean": round(sum(sample["cpu"] for sample in steady) / len(steady), 2),
        "rss_mean_mib": round(sum(sample["rss_mib"] for sample in steady) / len(steady), 1),
        "samples": samples,
    }
    args.output.with_suffix(".json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps({key: value for key, value in summary.items() if key != "samples"}, indent=2))


if __name__ == "__main__":
    main()

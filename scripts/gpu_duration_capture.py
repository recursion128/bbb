#!/usr/bin/env python3
"""API-neutral per-client GPU execution duration capture.

The metric is measured below both graphics APIs. radeonsi (the official client's
OpenGL) and RADV (bbb's Vulkan) submit through the same amdgpu command-submission
path, so one capture covers both without instrumenting either client:

    amdgpu:amdgpu_cs_ioctl        submission, raised in the client's own process
                                  context, so `pid` attributes the job to a client
    amdgpu:amdgpu_sched_run_job   the job is dispatched to its ring
    dma_fence:dma_fence_signaled  driver=drm_sched, the job's own context ->
                                  execution complete

`drm/scheduler` allocates two fence contexts per entity: the `scheduled` fence at
`entity->fence_context` and the `finished` fence at `fence_context + 1`. amdgpu
traces the finished fence, so a job's own `(context, seqno)` joins to its
completion signal while `(context - 1, seqno)` is the moment it started running.

From that chain, per job:

    queue wait      = run_job          - cs_ioctl
    execution       = finished signal  - run_job

Compositor work is excluded by process, not by guesswork: Weston submits from its
own pid and never appears under a client's. This module deliberately reports
nothing derived from application-frame intervals, process means, surface commits,
or GPU busy/load telemetry -- those are different quantities and the parity ledger
forbids relabelling them as GPU frame duration.

Capture runs unprivileged once the runner grants the two standard permissions
(see `preflight`): tracefs readable by the benchmark group so perf can resolve
tracepoint ids, and `kernel.perf_event_paranoid <= -1` so it can open them.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import json
import os
from pathlib import Path
import re
import statistics
import subprocess
import sys

CS_IOCTL_EVENT = "amdgpu:amdgpu_cs_ioctl"
RUN_JOB_EVENT = "amdgpu:amdgpu_sched_run_job"
FENCE_SIGNALED_EVENT = "dma_fence:dma_fence_signaled"
CAPTURE_EVENTS = (CS_IOCTL_EVENT, RUN_JOB_EVENT, FENCE_SIGNALED_EVENT)

# `perf script -F comm,pid,time,event,trace --ns` emits one line per event:
#   <comm> <pid> <seconds>.<nanoseconds>: <group>:<event>: <trace fields>
_LINE_RE = re.compile(
    r"^\s*(?P<comm>\S.*?)\s+(?P<pid>\d+)\s+(?P<seconds>\d+)\.(?P<nanos>\d{9}):\s+"
    r"(?P<event>[\w:]+):\s*(?P<fields>.*)$"
)
_AMDGPU_JOB_RE = re.compile(
    r"sched_job=(?P<sched_job>\d+),\s*timeline=(?P<timeline>\S+?),\s*"
    r"context=(?P<context>\d+),\s*seqno=(?P<seqno>\d+),\s*ring_name=(?P<ring>\S+?),"
)
_FENCE_RE = re.compile(
    r"driver=(?P<driver>\S+)\s+timeline=(?P<timeline>\S+)\s+"
    r"context=(?P<context>\d+)\s+seqno=(?P<seqno>\d+)"
)

SCHEDULER_FENCE_DRIVER = "drm_sched"
GRAPHICS_RING_PREFIX = "gfx"


@dataclass(frozen=True)
class GpuJob:
    """One GPU job joined across the submission -> dispatch -> completion chain."""

    pid: int
    comm: str
    ring: str
    context: int
    seqno: int
    submit_ns: int
    run_ns: int
    finished_ns: int

    @property
    def queue_wait_ns(self) -> int:
        return self.run_ns - self.submit_ns

    @property
    def execution_ns(self) -> int:
        return self.finished_ns - self.run_ns


def _timestamp_ns(match: re.Match[str]) -> int:
    return int(match.group("seconds")) * 1_000_000_000 + int(match.group("nanos"))


def parse_perf_script(lines) -> dict[str, dict]:
    """Indexes the three tracepoints by their join keys.

    Returns `{"submits": {...}, "runs": {...}, "signals": {...}}` where submits and
    runs are keyed by `(context, seqno)` and signals by `(driver, context, seqno)`.
    """
    submits: dict[tuple[int, int], dict] = {}
    runs: dict[tuple[int, int], dict] = {}
    signals: dict[tuple[str, int, int], int] = {}
    for line in lines:
        line_match = _LINE_RE.match(line.rstrip("\n"))
        if line_match is None:
            continue
        event = line_match.group("event")
        fields = line_match.group("fields")
        timestamp = _timestamp_ns(line_match)
        if event in (CS_IOCTL_EVENT, RUN_JOB_EVENT):
            job_match = _AMDGPU_JOB_RE.search(fields)
            if job_match is None:
                continue
            key = (int(job_match.group("context")), int(job_match.group("seqno")))
            record = {
                "timestamp_ns": timestamp,
                "pid": int(line_match.group("pid")),
                "comm": line_match.group("comm").strip(),
                "ring": job_match.group("ring"),
            }
            target = submits if event == CS_IOCTL_EVENT else runs
            # A resubmitted job repeats the key; the first observation is the one
            # the completion signal below belongs to.
            target.setdefault(key, record)
        elif event == FENCE_SIGNALED_EVENT:
            fence_match = _FENCE_RE.search(fields)
            if fence_match is None:
                continue
            key = (
                fence_match.group("driver"),
                int(fence_match.group("context")),
                int(fence_match.group("seqno")),
            )
            signals.setdefault(key, timestamp)
    return {"submits": submits, "runs": runs, "signals": signals}


def join_jobs(indexed: dict[str, dict]) -> list[GpuJob]:
    """Joins the indexed tracepoints into complete jobs, dropping partial chains.

    A job is complete only when its submission, dispatch and finished-fence signal
    were all captured and ordered; a capture window that opens or closes mid-job
    therefore contributes nothing rather than a truncated duration.
    """
    submits = indexed["submits"]
    runs = indexed["runs"]
    signals = indexed["signals"]
    jobs: list[GpuJob] = []
    for key, submit in submits.items():
        run = runs.get(key)
        if run is None:
            continue
        context, seqno = key
        finished_ns = signals.get((SCHEDULER_FENCE_DRIVER, context, seqno))
        if finished_ns is None:
            continue
        if not submit["timestamp_ns"] <= run["timestamp_ns"] <= finished_ns:
            continue
        jobs.append(
            GpuJob(
                pid=submit["pid"],
                comm=submit["comm"],
                ring=submit["ring"],
                context=context,
                seqno=seqno,
                submit_ns=submit["timestamp_ns"],
                run_ns=run["timestamp_ns"],
                finished_ns=finished_ns,
            )
        )
    jobs.sort(key=lambda job: job.submit_ns)
    return jobs


def association_ratio(indexed: dict[str, dict], jobs) -> float:
    """Share of client submissions that reached a complete chain.

    The denominator is `amdgpu_cs_ioctl` submissions, not every dispatched job:
    the scheduler also runs kernel-internal jobs (TTM buffer moves on the SDMA
    rings) that no client submitted and that carry no client attribution. Counting
    those would understate an association that is in fact complete. This is the
    evidence that the chain holds, not a quality knob -- a capture below the
    caller's floor is reported as failed rather than summarized.
    """
    submitted = len(indexed["submits"])
    if submitted == 0:
        return 0.0
    return len(jobs) / submitted


def _percentile(values: list[int], fraction: float) -> float:
    if not values:
        raise ValueError("no values")
    ordered = sorted(values)
    if len(ordered) == 1:
        return float(ordered[0])
    position = fraction * (len(ordered) - 1)
    lower = int(position)
    upper = min(lower + 1, len(ordered) - 1)
    weight = position - lower
    return ordered[lower] * (1.0 - weight) + ordered[upper] * weight


def _duration_stats(values: list[int]) -> dict:
    return {
        "count": len(values),
        "mean_us": statistics.fmean(values) / 1000.0,
        "p50_us": _percentile(values, 0.50) / 1000.0,
        "p95_us": _percentile(values, 0.95) / 1000.0,
        "p99_us": _percentile(values, 0.99) / 1000.0,
        "max_us": max(values) / 1000.0,
        "total_ms": sum(values) / 1_000_000.0,
    }


def summarize(
    jobs,
    *,
    client_pids=None,
    excluded_pids=(),
    graphics_only: bool = True,
) -> dict:
    """Per-client GPU execution duration, excluding the compositor's own jobs.

    `client_pids` restricts the report to the measured clients; `excluded_pids`
    carries the compositor (and anything else the caller proved is not the client)
    so its submissions never enter a client's numbers.
    """
    excluded = set(excluded_pids)
    selected = [job for job in jobs if job.pid not in excluded]
    if client_pids is not None:
        wanted = set(client_pids)
        selected = [job for job in selected if job.pid in wanted]
    if graphics_only:
        selected = [
            job for job in selected if job.ring.startswith(GRAPHICS_RING_PREFIX)
        ]
    clients: dict[int, dict] = {}
    for job in selected:
        entry = clients.setdefault(
            job.pid,
            {"pid": job.pid, "comm": job.comm, "rings": set(), "execution": [], "queue": []},
        )
        entry["rings"].add(job.ring)
        entry["execution"].append(job.execution_ns)
        entry["queue"].append(job.queue_wait_ns)
    report = []
    for entry in sorted(clients.values(), key=lambda item: item["pid"]):
        report.append(
            {
                "pid": entry["pid"],
                "comm": entry["comm"],
                "rings": sorted(entry["rings"]),
                "gpu_execution": _duration_stats(entry["execution"]),
                "queue_wait": _duration_stats(entry["queue"]),
            }
        )
    return {
        "schema": 1,
        "metric": "per-client GPU execution duration",
        "chain": [CS_IOCTL_EVENT, RUN_JOB_EVENT, FENCE_SIGNALED_EVENT],
        "graphics_only": graphics_only,
        "excluded_pids": sorted(excluded),
        "job_count": len(selected),
        "clients": report,
    }


def preflight(trace_root: Path | None = None) -> dict:
    """Reports whether an unprivileged capture can run, and how to provision it.

    Both requirements are ordinary runner provisioning, not a capture-time
    privilege: perf reads tracepoint ids out of tracefs and opens them as a normal
    user once `perf_event_paranoid` allows it.
    """
    roots = (
        [trace_root]
        if trace_root is not None
        else [Path("/sys/kernel/tracing"), Path("/sys/kernel/debug/tracing")]
    )
    events: dict[str, bool] = {}
    resolved_root: Path | None = None
    for root in roots:
        if not root.is_dir():
            continue
        readable = {}
        for event in CAPTURE_EVENTS:
            group, name = event.split(":", 1)
            readable[event] = os.access(root / "events" / group / name / "id", os.R_OK)
        if all(readable.values()):
            resolved_root = root
            events = readable
            break
        if not events:
            events = readable
    paranoid_path = Path("/proc/sys/kernel/perf_event_paranoid")
    try:
        paranoid = int(paranoid_path.read_text(encoding="utf-8").strip())
    except (OSError, ValueError):
        paranoid = None
    blockers = []
    if resolved_root is None:
        blockers.append(
            {
                "code": "tracepoint_ids_unreadable",
                "detail": "perf cannot resolve the amdgpu/dma_fence tracepoint ids",
                "provision": (
                    "sudo mount -o remount,gid=<benchmark-group>,mode=0750 "
                    "/sys/kernel/tracing"
                ),
            }
        )
    if paranoid is None or paranoid > -1:
        blockers.append(
            {
                "code": "perf_event_paranoid_too_strict",
                "detail": (
                    "kernel.perf_event_paranoid must be -1 for unprivileged "
                    "tracepoint recording"
                ),
                "provision": "sudo sysctl -w kernel.perf_event_paranoid=-1",
            }
        )
    return {
        "schema": 1,
        "trace_root": str(resolved_root) if resolved_root else None,
        "tracepoints": events,
        "perf_event_paranoid": paranoid,
        "unprivileged_capture_ready": not blockers,
        "blockers": blockers,
    }


def capture(output: Path, command: list[str]) -> dict:
    """Records the three tracepoints around `command` and returns the raw paths."""
    output.mkdir(parents=True, exist_ok=True)
    perf_data = output / "gpu-duration.perf.data"
    script_path = output / "gpu-duration.script.txt"
    record = subprocess.run(
        [
            "perf",
            "record",
            "-q",
            "-o",
            str(perf_data),
            "-e",
            ",".join(CAPTURE_EVENTS),
            "-a",
            "--",
            *command,
        ],
        check=False,
        stdin=subprocess.DEVNULL,
    )
    if not perf_data.exists():
        raise SystemExit("perf record produced no capture")
    with script_path.open("w", encoding="utf-8") as handle:
        script = subprocess.run(
            [
                "perf",
                "script",
                "-i",
                str(perf_data),
                "-F",
                "comm,pid,time,event,trace",
                "--ns",
            ],
            check=False,
            stdout=handle,
            stderr=subprocess.DEVNULL,
        )
    return {
        "command_returncode": record.returncode,
        "script_returncode": script.returncode,
        "perf_data": str(perf_data),
        "script": str(script_path),
    }


def summarize_script(
    script: Path,
    *,
    client_pids=None,
    excluded_pids=(),
    graphics_only: bool = True,
) -> dict:
    with script.open(encoding="utf-8") as handle:
        indexed = parse_perf_script(handle)
    jobs = join_jobs(indexed)
    report = summarize(
        jobs,
        client_pids=client_pids,
        excluded_pids=excluded_pids,
        graphics_only=graphics_only,
    )
    report["submitted_jobs"] = len(indexed["submits"])
    report["associated_jobs"] = len(jobs)
    # Scheduler jobs with no client submission are kernel-internal (TTM buffer
    # moves); they are counted here rather than dropped silently.
    report["kernel_internal_jobs"] = max(
        len(indexed["runs"]) - len(indexed["submits"]), 0
    )
    report["association_ratio"] = association_ratio(indexed, jobs)
    return report


def _pid_list(value: str | None):
    if not value:
        return ()
    return tuple(int(item) for item in value.split(",") if item.strip())


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="action", required=True)

    preflight_parser = subparsers.add_parser("preflight")
    preflight_parser.add_argument("--trace-root", type=Path, default=None)

    capture_parser = subparsers.add_parser("capture")
    capture_parser.add_argument("--output", type=Path, required=True)
    capture_parser.add_argument("command", nargs=argparse.REMAINDER)

    summarize_parser = subparsers.add_parser("summarize")
    summarize_parser.add_argument("--script", type=Path, required=True)
    summarize_parser.add_argument("--client-pids", default=None)
    summarize_parser.add_argument("--exclude-pids", default=None)
    summarize_parser.add_argument("--all-rings", action="store_true")

    args = parser.parse_args(argv)
    # `summarize` only reads a capture file, so it stays host-neutral; the two
    # commands that touch tracefs and perf say where they run instead of
    # failing on an absent kernel interface.
    if args.action in {"preflight", "capture"} and sys.platform != "linux":
        print(
            f"gpu duration {args.action} needs Linux tracefs and perf; "
            f"this host is {sys.platform}. Summarize an existing capture instead",
            file=sys.stderr,
        )
        return 2
    if args.action == "preflight":
        report = preflight(args.trace_root)
    elif args.action == "capture":
        command = list(args.command)
        if command and command[0] == "--":
            command = command[1:]
        if not command:
            parser.error("capture needs a command to run")
        report = capture(args.output, command)
    else:
        report = summarize_script(
            args.script,
            client_pids=_pid_list(args.client_pids) or None,
            excluded_pids=_pid_list(args.exclude_pids),
            graphics_only=not args.all_rings,
        )
    json.dump(report, sys.stdout, indent=2, sort_keys=True)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

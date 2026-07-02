# Cargo Build Performance

This document records the current Cargo build baseline and the local workflow
used to keep focused tests fast without weakening the final merge gate.

## Goals

- Preserve the final gate:
  - `cargo fmt --check`
  - `git diff --check`
  - `CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test --workspace`
- Reduce daily focused-test and multi-worktree cold compile cost.
- Keep build output out of the repo-local `target` directory.
- Keep caches stable across slices and clean them deliberately, not after every
  worker task.

## Current Slice Policy

Cargo build performance work is an engineering-efficiency slice, not a reason
to relax correctness checks. The preferred order is:

1. Keep the main worktree and worker worktrees on stable external target
   directories.
2. Measure clean full workspace, warm focused, warm full workspace, and target
   size before changing profiles or cache policy.
3. Evaluate optional `sccache` only when it is installed locally, and compare
   runs with and without `RUSTC_WRAPPER=sccache`.
4. Use `fast-test` only for daily focused iteration.
5. Keep the final merge gate on the default profile with
   `CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test --workspace`.

## Current sccache Decision

`sccache` is installed locally and remains opt-in. Use
`RUSTC_WRAPPER=sccache` or `BBB_USE_SCCACHE=1` only when explicitly measuring
or experimenting; do not add a repo-local `.cargo/config.toml` wrapper setting.

The latest 2026-06-19 recheck used:

```sh
scripts/cargo-dev.sh sccache-eval 20260619T145029 -p bbb-world command_tree --quiet
```

Results:

- Clean full workspace with `sccache`: 175.78s, 3.3G target, all tests passed,
  timing report copied to
  `/tmp/bbb-cargo-timings/cargo-timing-sccache-clean-20260619T145029.html`.
- Fresh worker focused test with `sccache`: 53.29s, 657M target, 1 test passed,
  0 Rust cache hits.
- Fresh worker focused test without `sccache`: 50.64s, 655M target, 1 test
  passed.
- Warm focused default profile on `/tmp/bbb-target-main` with `sccache`: 0.22s,
  1 test passed, no compilations executed.

This keeps the policy unchanged: stable external `CARGO_TARGET_DIR` values are
the default developer-speed path, and `sccache` is available for explicit
future rechecks rather than default worker prompts.

## Target Directories

Use stable external target directories through the helper script:

```sh
scripts/cargo-dev.sh test -p bbb-world <filter>
BBB_CARGO_TARGET_NAME=renderer scripts/cargo-dev.sh test -p bbb-renderer <filter>
BBB_CARGO_TARGET_NAME=world scripts/cargo-dev.sh test -p bbb-world <filter>
BBB_CARGO_TARGET_NAME=net scripts/cargo-dev.sh test -p bbb-net <filter>
```

Do not run parallel Cargo commands against the same target directory. Cargo will
serialize on package/build locks, which removes most of the benefit of parallel
workers.

Do not commit a repo-local `.cargo/config.toml` that forces one target
directory. It would make independent worker worktrees share the same cache and
lock unexpectedly. Prefer `BBB_CARGO_TARGET_NAME` or explicit
`CARGO_TARGET_DIR` in commands and worker prompts.

Repo-local `target` stays ignored and should not be generated during normal
agent work.

## Worker Worktrees

Use `scripts/worker-worktree.sh` to create and retire temporary worker
worktrees instead of hand-assembling path, branch, and target-directory names:

```sh
scripts/worker-worktree.sh create world
scripts/worker-worktree.sh status
scripts/worker-worktree.sh env world
scripts/worker-worktree.sh shell-env world
scripts/worker-worktree.sh cleanup world
```

The helper uses these conventions:

- worktree: `../bbb-wt-<name>`
- branch: `worker/<name>`
- cargo target name: `<name>`
- target: `/tmp/bbb-target-<name>`

If `worker/<name>` already exists, `create <name>` uses a timestamp-suffixed
temporary branch. `cleanup <name>` refuses dirty worker worktrees, removes the
worktree, safely deletes the temporary branch when Git allows it, and keeps the
matching target directory for future focused tests. Use
`cleanup <name> --remove-target` only when the worker target is intentionally
disposable or disk pressure matters. Use `--force` only after the worker diff
has been reviewed and discarded, integrated, or duplicated elsewhere.

`create <name>` prints a ready-to-run focused-test command using
`scripts/cargo-dev.sh` and `BBB_CARGO_TARGET_NAME=<name>`, so worker prompts
can use the same test entry as the main worktree.

## Helper Script

Use `scripts/cargo-dev.sh` to avoid retyping target-cache commands during
focused development:

```sh
scripts/cargo-dev.sh test -p bbb-world <filter>
scripts/cargo-dev.sh fast-test -p bbb-world <filter>
BBB_CARGO_TARGET_NAME=world scripts/cargo-dev.sh test -p bbb-world <filter>
scripts/cargo-dev.sh timings --workspace --timings
scripts/cargo-dev.sh timings-clean clean-baseline-YYYYMMDD --workspace --timings
scripts/cargo-dev.sh sccache-eval YYYYMMDD -p bbb-world command_tree --quiet
scripts/cargo-dev.sh size
scripts/cargo-dev.sh clean-target clean-baseline-YYYYMMDD
scripts/cargo-dev.sh sccache-status
```

The script defaults to `CARGO_TARGET_DIR=/tmp/bbb-target-main`. Set
`BBB_CARGO_TARGET_NAME=renderer`, `world`, or `net` to use
`/tmp/bbb-target-renderer`, `/tmp/bbb-target-world`, or `/tmp/bbb-target-net`.
An explicit `CARGO_TARGET_DIR` still wins.

`scripts/cargo-dev.sh gate` runs the same merge gate commands documented in
this file and always uses `/tmp/bbb-target-main`; it is a convenience wrapper,
not a weaker test path.

`scripts/cargo-dev.sh timings-clean <target-suffix>` uses
`/tmp/bbb-target-<target-suffix>` and refuses to run if that target already
exists. Use it for disposable clean baselines where reusing a warm cache would
invalidate the measurement.

`scripts/cargo-dev.sh clean-target <target-suffix>` removes only targets under
`/tmp/bbb-target-*`. It is for explicit periodic cleanup and disposable
baselines, not end-of-slice worker cleanup.

`scripts/cargo-dev.sh sccache-eval <run-suffix> [focused cargo test args...]`
runs the repeatable local `sccache` experiment:

- clean full workspace with `RUSTC_WRAPPER=sccache`
- new worker target focused test with `RUSTC_WRAPPER=sccache`
- new worker target focused test without `sccache` for comparison
- warm focused default-profile test on `/tmp/bbb-target-main` with `sccache`

The focused test defaults to `-p bbb-world command_tree --quiet` when no cargo
test arguments are supplied. The command refuses to reuse existing disposable
measurement targets so warm cache state cannot accidentally invalidate the
result. It copies the clean full-workspace timing report to
`/tmp/bbb-cargo-timings/cargo-timing-sccache-clean-<run-suffix>.html` and
prints the matching cleanup commands. Remove those disposable targets with
`scripts/cargo-dev.sh clean-target` after recording the numbers.

## Command Matrix

Use these commands as the default local workflow:

- Main focused test:
  `scripts/cargo-dev.sh test -p <crate> <filter>`
- Worker focused test:
  `BBB_CARGO_TARGET_NAME=<domain> scripts/cargo-dev.sh test -p <crate> <filter>`
- Daily fast focused test:
  `scripts/cargo-dev.sh fast-test -p <crate> <filter>`
- Warm full workspace timing:
  `scripts/cargo-dev.sh timings --workspace --timings`
- Clean full workspace baseline:
  `scripts/cargo-dev.sh timings-clean clean-baseline-YYYYMMDD --workspace --timings`
- `sccache` evaluation:
  `scripts/cargo-dev.sh sccache-eval YYYYMMDD -p bbb-world command_tree --quiet`
- Target cache size:
  `scripts/cargo-dev.sh size`
- Disposable target cleanup:
  `scripts/cargo-dev.sh clean-target clean-baseline-YYYYMMDD`
- Final merge gate:
  `scripts/cargo-dev.sh gate`

Keep focused/default-profile tests, `fast-test`, timings, target-size checks,
and `sccache` inspection behind this script where practical. The script keeps
daily commands consistent while preserving the documented final gate.

## sccache

`sccache` is useful for repeated dependency and workspace crate compilation
across multiple target directories and worktrees, but it should remain optional
local tooling.

Do not commit a mandatory `rustc-wrapper` setting. Machines without `sccache`
would fail before compiling.

Use it explicitly when installed:

```sh
BBB_USE_SCCACHE=1 scripts/cargo-dev.sh test -p bbb-world <filter>
RUSTC_WRAPPER=sccache scripts/cargo-dev.sh test -p bbb-world <filter>
scripts/cargo-dev.sh sccache-eval YYYYMMDD -p bbb-world <filter> --quiet
scripts/cargo-dev.sh sccache-status
```

Record before/after timings before making `sccache` part of a default local
shell profile. Workspace incremental builds and `sccache` do not always improve
the same workload, so use measured data.

## Profiles

The default `test` profile remains the authoritative gate. The workspace also
has an opt-in `fast-test` profile for daily focused tests:

```sh
scripts/cargo-dev.sh fast-test -p bbb-world <filter>
```

`fast-test` has its own profile output directory. Its first run can be slower
than a warm default-profile focused test because it has to build a separate
cache. Use it when you expect multiple focused iterations in the same profile,
not as a replacement for the final gate.

## Timing Commands

Warm full workspace timing:

```sh
scripts/cargo-dev.sh timings --workspace --timings
```

Clean full workspace timing should use a disposable external target:

```sh
scripts/cargo-dev.sh timings-clean clean-baseline-YYYYMMDD --workspace --timings
du -sh /tmp/bbb-target-clean-baseline-YYYYMMDD
```

Remove disposable clean-baseline targets after recording the result if disk
pressure matters. Do not remove stable `/tmp/bbb-target-*` caches after every
slice.

## Periodic Cleanup

Use cache size, not slice boundaries, to decide cleanup:

```sh
scripts/cargo-dev.sh size
```

Remove disposable clean baselines after their numbers and timing report paths
are recorded:

```sh
scripts/cargo-dev.sh clean-target clean-baseline-YYYYMMDD
```

Remove completed worker targets through the worker helper only when they are
intentionally disposable:

```sh
scripts/worker-worktree.sh cleanup world --remove-target
```

Clean long-lived targets such as `main`, `world`, `net`, and `renderer` only
when disk pressure is real, when changing Rust toolchains or build profiles, or
when a dependency feature experiment has made the cache misleadingly large.
Expect the next focused or full test on that target to rebuild.

## Baseline Template

Record build-performance experiments in this shape:

- Environment:
  - Operating system.
  - `cargo --version`.
  - `rustc --version`.
  - `scripts/cargo-dev.sh sccache-status` result.
- Clean full workspace:
  - Command.
  - Wall time.
  - Target size.
  - Timing report path.
  - Result.
- Warm focused test:
  - Command.
  - Wall time.
  - Result.
- Warm full workspace:
  - Command.
  - Wall time.
  - Target size.
  - Result.
- Notes:
  - Top timing entries.
  - Profile, dependency feature, or `sccache` changes being compared.
  - Any cache removed after measurement.

## Baseline: 2026-06-18

Environment:

- macOS development machine.
- `cargo 1.96.0-nightly (cbb9bb8bd 2026-03-13)`.
- `rustc 1.96.0-nightly (bcf3d36c9 2026-03-19)`.
- `sccache` was not installed on `PATH`.

Measured commands:

- Clean full workspace:
  `CARGO_TARGET_DIR=/tmp/bbb-target-clean-baseline-20260618 cargo test --workspace --timings`
  - Cargo compile: 2m39s.
  - Wall time: 165.35s.
  - Target size: 3.1G.
  - Result: all tests passed.
- Warm full workspace:
  `CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test --workspace --timings`
  - Cargo compile: 0.23s.
  - Wall time: 2.75s.
  - Target size: 5.7G.
  - Result: all tests passed.
- Warm focused default:
  `CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test -p bbb-world command_tree`
  - Cargo compile: 0.09s.
  - Wall time: 0.25s.
  - Result: 1 test passed.
- Cold focused `fast-test`:
  `CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test --profile fast-test -p bbb-world command_tree`
  - Cargo compile: 4.57s.
  - Wall time: 5.67s.
  - Target size: 582M `fast-test` profile directory.
  - Result: first `fast-test` use, 1 test passed.
- Warm focused `fast-test`:
  `CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test --profile fast-test -p bbb-world command_tree`
  - Cargo compile: 0.06s.
  - Wall time: 0.19s.
  - Result: 1 test passed.

Timing reports:

- Clean: copied to `/tmp/bbb-cargo-timings/cargo-timing-clean-20260618.html`;
  the disposable clean target was removed after recording.
- Warm main: `/tmp/bbb-target-main/cargo-timings/cargo-timing.html`

Top cold timing entries included:

- `bbb-world` test unit: 67.5s.
- `bbb-pack` test unit: 51.2s.
- `bbb-pack` library: 47.9s.
- `bbb-protocol` test unit: 40.4s.
- `image`: 39.0s.
- `wgpu`: 37.3s.
- `bbb-world` library: 35.8s.
- `naga`: 35.1s.
- macOS ObjC/Foundation stack: 33.9s.
- `syn`: 33.1s.
- `bbb-net` test unit: 32.5s.
- `bbb-native` test unit: 26.0s.
- `wgpu-core`: 25.8s.

## sccache Evaluation: 2026-06-19

Environment:

- macOS development machine.
- `cargo 1.96.0-nightly (cbb9bb8bd 2026-03-13)`.
- `rustc 1.96.0-nightly (bcf3d36c9 2026-03-19)`.
- `sccache 0.15.0`.
- `RUSTC_WRAPPER` is unset by default. No repo-local `.cargo/config.toml`
  enables `sccache`.

Measured command:

- `scripts/cargo-dev.sh sccache-eval 20260619 -p bbb-world command_tree --quiet`

Results:

- Clean full workspace with `sccache`:
  `RUSTC_WRAPPER=sccache CARGO_TARGET_DIR=/tmp/bbb-target-sccache-clean-20260619 cargo test --workspace --timings --quiet`
  - Wall time: 171.69s.
  - Target size: 3.3G.
  - Timing report:
    `/tmp/bbb-cargo-timings/cargo-timing-sccache-clean-20260619.html`
  - Result: all tests passed.
  - `sccache` stats: 217 compile requests, 156 executed, 155 Rust misses,
    0 Rust hits.
- New worker target focused test with `sccache`:
  `RUSTC_WRAPPER=sccache CARGO_TARGET_DIR=/tmp/bbb-target-sccache-worker-20260619 cargo test -p bbb-world command_tree --quiet`
  - Wall time: 51.41s.
  - Target size: 646M.
  - Result: 1 test passed.
  - `sccache` stats: 46 compile requests, 29 executed, 29 Rust misses,
    0 Rust hits.
- New worker target focused test without `sccache`:
  `CARGO_TARGET_DIR=/tmp/bbb-target-nosccache-worker-20260619 cargo test -p bbb-world command_tree --quiet`
  - Wall time: 51.04s.
  - Target size: 646M.
  - Result: 1 test passed.
- Warm focused default-profile test on `/tmp/bbb-target-main` with `sccache`:
  `RUSTC_WRAPPER=sccache CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test -p bbb-world command_tree --quiet`
  - Wall time: 0.22s.
  - Result: 1 test passed.
  - No compilations were executed.

Conclusion:

- This measurement does not show a clean-workspace or new-worker focused-test
  improvement from `sccache` on the current macOS workload.
- Keep `sccache` explicit and opt-in with `RUSTC_WRAPPER=sccache` or
  `BBB_USE_SCCACHE=1`; do not add a repo-local mandatory `rustc-wrapper`.
- Do not default worker prompts to `BBB_USE_SCCACHE=1` unless a later
  measurement shows a real benefit for that slice.
- Multi-worktree throughput should continue to rely on stable external target
  directories and not deleting warm caches after every slice.
- The disposable measurement targets were removed after recording these
  numbers.

## Next Evaluation Points

- Recheck `sccache` only after dependency, profile, or toolchain changes. Local
  2026-06-19 measurements with `sccache 0.15.0` did not show Rust cache hits or
  a worker cold-compile improvement.
- Recheck whether dependency opt-level settings in `[profile.dev.package."*"]`
  are worth the cold compile cost for the current test mix.
- Keep renderer/audio dependency work focused. `wgpu`, `naga`, `image`, `cpal`,
  and `kira` dominate clean compilation, so feature changes there should include
  focused timing before broad profile changes.
- Consider `cargo-nextest` only for test execution time. It will not remove the
  dominant cold compilation cost.
- Do not prioritize mold/lld on this macOS machine; target caching, sccache, and
  profile measurement are the higher-confidence local optimizations.

## Recheck History

This section compacts the repeated post-2026-06-18 warm-update measurements and
the repeated `sccache` rechecks (originally recorded as many separate sections)
into one history table. Every date, measured wall time, and result target size
is preserved. Per-run values that were constant or incidental — the sub-second
cargo-compile fragments of each wall time, the identical `sccache` counters, the
growing disposable-cache sizes, and the "stable target caches found" environment
inventories — are summarized here once instead of repeated per row.

All runs passed. Every `sccache` recheck reported the same `sccache` stats — the
clean full workspace: 217 compile requests, 156 executed, 0 Rust cache hits (155
Rust misses, an occasional 1 C/C++ hit, 59 non-cacheable calls); the new worker
focused test: 46 requests, 29 executed, 0 Rust hits, 17 non-cacheable calls. The
disposable `sccache` cache grew from ~216M to ~4GiB across the series, and the
disposable measurement targets were removed after each run. Warm focused
`sccache` rechecks used `bbb-world command_tree`, except the first recheck
(`20260619110904`) which used
`bbb-world local_player_respects_piglin_wall_head_wider_collision`; the warm
focused runs compiled nothing (they hit the stable `/tmp/bbb-target-main`
cache). Every recheck reached the same conclusion: `sccache` stays opt-in
through `RUSTC_WRAPPER=sccache` / `BBB_USE_SCCACHE=1` with no repo-local
`rustc-wrapper`, and stable external `CARGO_TARGET_DIR` caches remain the primary
developer-speed path. This history is append-only.

| Date | Run / focused test | Clean full ws (s) | Worker sccache / none (s) | Warm full / focused (s) | Target size (clean; worker) | Conclusion |
| --- | --- | --- | --- | --- | --- | --- |
| 2026-06-19 | Warm Update (full ws) | — | — | 3.01 / — | 8.9G | all passed; size from retained cache |
| 2026-06-19 | After Lectern (`bbb-native lectern`) | — | — | 3.03 / 0.28 | 10G | 9 focused tests passed |
| 2026-06-19 | After Cartography (`bbb-native cartography`) | — | — | 3.20 / 0.20 | 11G | 6 focused tests passed; merge gate 38.85s |
| 2026-06-19 | After Sign Text (full ws) | — | — | 3.15 / — | 11G | all passed |
| 2026-06-19 | After Bundle Click (`bbb-native bundle`) | — | — | 3.06 / 0.22 | 12G | 19 focused tests passed |
| 2026-06-19 | Cargo Efficiency (`bbb-world water_movement_efficiency`) | — | — | 3.71 / 0.09 | 14G | 3 focused tests passed |
| 2026-06-19 | After Sprint Jump (`bbb-world local_player_sprint_jump`) | — | — | 3.67 / 0.13 | 16G | 1 focused test passed |
| 2026-06-19 | After Randomized Level Event Sound (`bbb-world command_tree`) | — | — | 4.74 / 0.11 | 18G | fast-test 0.20s; 1 focused test passed |
| 2026-06-19 | After Small Object Collision (`bbb-world local_player_respects_piglin_wall_head_wider_collision`) | — | — | 4.94 / 0.13 | 19G | 1 focused test passed |
| 2026-06-19 | sccache `20260619110904` | 171.76 | 50.71 / 50.39 | — / 0.24 | 3.2G; 636M/635M | 0 Rust hits; no cold-worker gain |
| 2026-06-19 | sccache `20260619123234` | 164.61 | 50.20 / 49.99 | — / 0.22 | 3.2G; 637M | 0 Rust hits; no cold-worker gain |
| 2026-06-19 | sccache `20260619-installed3` | 165.43 | 50.46 / 49.29 | — / 0.18 | 3.2G; 637M/638M | 0 Rust hits; no cold-worker gain |
| 2026-06-19 | sccache `20260619-r2` | 169.42 | 50.31 / 49.08 | — / 1.42 | 3.2G; 638M | 0 Rust hits; no cold-worker gain |
| 2026-06-19 | sccache `20260619143652` | 169.73 | 50.81 / 50.42 | — / 0.19 | 3.2G; 641M/640M | 0 Rust hits; no cold-worker gain |
| 2026-06-19 | sccache `20260619-live` | 175.35 | 52.37 / 50.29 | — / 0.19 | 3.2G; 644M/642M | 0 Rust hits; no gain (2.08s faster w/o sccache) |
| 2026-06-19 | sccache `20260619161103` | 170.77 | 51.87 / 50.94 | — / 5.87 | 3.2G; 647M | 0 Rust hits; no cold-worker gain |
| 2026-06-19 | sccache `20260619173725` | 173.85 | 51.60 / 50.45 | — / 5.59 | 3.3G; 648M/647M | 0 Rust hits; no cold-worker gain |
| 2026-06-19 | sccache `20260619-efficiency` | 170.85 | 51.79 / 50.76 | — / 8.54 | 3.3G; 649M/651M | 0 Rust hits; no cold-worker gain |
| 2026-06-19 | sccache `20260619T132708` | 175.27 | 53.30 / 51.06 | — / 0.22 | 3.3G; 655M | 0 Rust hits; no gain (2.24s faster w/o sccache) |
| 2026-06-19 | sccache `20260619T145029` | 175.78 | 53.29 / 50.64 | — / 0.22 | 3.3G; 657M/655M | 0 Rust hits; no cold-worker gain |

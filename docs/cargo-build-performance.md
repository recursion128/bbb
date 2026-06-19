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

## Warm Update: 2026-06-19

Environment change:

- At this measurement time, `sccache` was not installed on `PATH`.

Measured command:

- Warm full workspace:
  `CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test --workspace --timings`
  - Cargo compile: 0.17s.
  - Wall time: 3.01s.
  - Target size: 8.9G.
  - Result: all tests passed.
  - Timing report:
    `/tmp/bbb-target-main/cargo-timings/cargo-timing-20260618T181448362Z-14ffed61c5c1036c.html`

The increased target size is from retained external cache data, which is
intentional for daily development. Reclaim it with explicit periodic cleanup,
not after each slice or worker run.

## Warm Update: 2026-06-19 After Lectern Slice

Environment:

- At this measurement time, `sccache` was not installed on `PATH`.
- Stable target caches found:
  - `/tmp/bbb-target-main`: 10G.
  - `/tmp/bbb-target-net`: 726M.
- No clean target was created for this update. The clean full-workspace baseline
  remains the 2026-06-18 disposable-target measurement above because this slice
  did not change Cargo profiles, dependencies, or cache policy.

Measured commands:

- Warm focused default:
  `scripts/cargo-dev.sh test -p bbb-native lectern`
  - Cargo compile: 0.11s.
  - Wall time: 0.28s.
  - Result: 9 tests passed.
- Warm full workspace:
  `scripts/cargo-dev.sh timings --workspace --timings`
  - Cargo compile: 0.10s.
  - Wall time: 3.03s.
  - Target size: 10G.
  - Result: all tests passed.
  - Timing report:
    `/tmp/bbb-target-main/cargo-timings/cargo-timing-20260618T194434598Z-14ffed61c5c1036c.html`

The current data supports keeping stable external target caches for daily work.
Later installed `sccache` measurements kept the same policy: do not add a
repo-local mandatory `rustc-wrapper`.

## Warm Update: 2026-06-19 After Cartography Slice

Environment:

- At this measurement time, `sccache` was not installed on `PATH`.
- Stable target caches found:
  - `/tmp/bbb-target-main`: 11G.
  - `/tmp/bbb-target-net`: 726M.
- No clean target was created for this update. The slice did not change Cargo
  profiles, dependencies, or cache policy.

Measured commands:

- Warm focused default:
  `scripts/cargo-dev.sh test -p bbb-native cartography`
  - Cargo compile: 0.09s.
  - Wall time: 0.20s.
  - Result: 6 tests passed.
- Warm full workspace:
  `scripts/cargo-dev.sh timings --workspace --timings`
  - Cargo compile: 0.28s.
  - Wall time: 3.20s.
  - Target size: 11G.
  - Result: all tests passed.
  - Timing report:
    `/tmp/bbb-target-main/cargo-timings/cargo-timing-20260618T203631338Z-14ffed61c5c1036c.html`

The cartography merge gate rebuilt changed workspace crates before this warm
update. That gate compiled in 38.85s and passed, which is expected for a warm
incremental run after protocol and native runtime edits rather than a clean
baseline.

## Warm Update: 2026-06-19 After Sign Text Slice

Environment:

- At this measurement time, `sccache` was not installed on `PATH`.
- Stable target caches found:
  - `/tmp/bbb-target-main`: 11G.
  - `/tmp/bbb-target-net`: 726M.
- No clean target was created for this update. The slice did not change Cargo
  profiles, dependencies, or cache policy.

Measured command:

- Warm full workspace:
  `scripts/cargo-dev.sh timings --workspace --timings`
  - Cargo compile: 0.27s.
  - Wall time: 3.15s.
  - Target size: 11G.
  - Result: all tests passed.
  - Timing report:
    `/tmp/bbb-target-main/cargo-timings/cargo-timing-20260618T211121498Z-14ffed61c5c1036c.html`

This confirms the current external-target workflow keeps a post-slice warm full
workspace gate in the low single-digit seconds once incremental rebuild work is
complete. Later installed `sccache` measurements kept it optional rather than a
repo default.

## Warm Update: 2026-06-19 After Bundle Click Slice

Environment:

- At this measurement time, `sccache` was not installed on `PATH`.
- Stable target caches found:
  - `/tmp/bbb-target-main`: 12G.
  - `/tmp/bbb-target-net`: 726M.
  - `/tmp/bbb-target-world`: 609M.
- No clean target was created for this update. The slice did not change Cargo
  profiles, dependencies, or cache policy.

Measured commands:

- Warm focused default:
  `scripts/cargo-dev.sh test -p bbb-native bundle --quiet`
  - Wall time: 0.22s.
  - Result: 19 tests passed.
- Warm full workspace:
  `scripts/cargo-dev.sh timings --workspace --timings --quiet`
  - Wall time: 3.06s.
  - Target size: 12G.
  - Result: all tests passed.
  - Timing report:
    `/tmp/bbb-target-main/cargo-timings/cargo-timing-20260618T214641126Z-14ffed61c5c1036c.html`

Running two focused tests concurrently against `/tmp/bbb-target-main` showed
Cargo package/build lock waits. Keep worker tests on distinct external target
directories when parallelism matters.

## Warm Update: 2026-06-19 Cargo Efficiency Check

Environment:

- At this measurement time, `sccache` was not installed on `PATH`.
- Stable target caches found:
  - `/tmp/bbb-target-main`: 14G.
  - `/tmp/bbb-target-native`: 3.3G.
  - `/tmp/bbb-target-net`: 721M.
  - `/tmp/bbb-target-pack`: 443M.
  - `/tmp/bbb-target-renderer`: 2.0G.
  - `/tmp/bbb-target-world`: 3.0G.
- No clean target was created for this update. The slice did not change Cargo
  profiles, dependencies, or cache policy, so the clean full-workspace baseline
  remains the 2026-06-18 disposable-target measurement above.

Measured commands:

- Warm focused default:
  `scripts/cargo-dev.sh test -p bbb-world water_movement_efficiency --quiet`
  - Wall time: 0.09s.
  - Result: 3 tests passed.
- Warm full workspace:
  `scripts/cargo-dev.sh timings --workspace --timings --quiet`
  - Wall time: 3.71s.
  - Target size: 14G.
  - Result: all tests passed.
  - Timing report:
    `/tmp/bbb-target-main/cargo-timings/cargo-timing-20260619T000416951Z-14ffed61c5c1036c.html`

The current workflow is already using retained external target caches, opt-in
`fast-test`, and per-worker target directories. `sccache` should stay explicit
and optional; the 2026-06-19 measurement below did not show a multi-target
cold-compile improvement on this machine. Keep `rustc-wrapper` out of
repo-local Cargo config unless future measurements show a stable benefit.

## Warm Update: 2026-06-19 After Sprint Jump Slice

Environment:

- At this measurement time, `sccache` was not installed on `PATH`.
- Stable target caches found:
  - `/tmp/bbb-target-main`: 16G.
  - `/tmp/bbb-target-native`: 3.3G.
  - `/tmp/bbb-target-net`: 720M.
  - `/tmp/bbb-target-pack`: 438M.
  - `/tmp/bbb-target-renderer`: 2.0G.
  - `/tmp/bbb-target-world`: 3.0G.
- No clean target was created for this update. The slice did not change Cargo
  profiles, dependencies, or cache policy, so the clean full-workspace baseline
  remains the 2026-06-18 disposable-target measurement above.

Measured commands:

- Warm focused default:
  `scripts/cargo-dev.sh test -p bbb-world local_player_sprint_jump --quiet`
  - Wall time: 0.13s.
  - Result: 1 test passed.
- Warm full workspace:
  `scripts/cargo-dev.sh timings --workspace --timings --quiet`
  - Wall time: 3.67s.
  - Target size: 16G.
  - Result: all tests passed.
  - Timing report:
    `/tmp/bbb-target-main/cargo-timings/cargo-timing-20260619T014734366Z-14ffed61c5c1036c.html`

The target cache is larger than earlier updates because it is now deliberately
retained across native-client slices. Keep using explicit external
`CARGO_TARGET_DIR` values and only clean stable targets when disk pressure is
real or a cache becomes misleading.

## Warm Update: 2026-06-19 After Randomized Level Event Sound Slice

Environment:

- At this measurement time, `sccache` was not installed on `PATH`.
- Stable target caches found:
  - `/tmp/bbb-target-main`: 18G.
  - `/tmp/bbb-target-native`: 3.3G.
  - `/tmp/bbb-target-net`: 720M.
  - `/tmp/bbb-target-pack`: 438M.
  - `/tmp/bbb-target-renderer`: 2.0G.
  - `/tmp/bbb-target-world`: 3.0G.
- No clean target was created for this update. The clean full-workspace baseline
  remains the 2026-06-18 disposable-target measurement above because this slice
  did not change Cargo profiles, dependencies, or cache policy.

Measured commands:

- Warm focused default:
  `scripts/cargo-dev.sh test -p bbb-world command_tree --quiet`
  - Wall time: 0.11s.
  - Result: 1 test passed.
- Warm focused `fast-test`:
  `scripts/cargo-dev.sh fast-test -p bbb-world command_tree --quiet`
  - Wall time: 0.20s.
  - Result: 1 test passed.
- Warm full workspace:
  `scripts/cargo-dev.sh timings --workspace --timings --quiet`
  - Wall time: 4.74s.
  - Target size: 18G.
  - Result: all tests passed.
  - Timing report:
    `/tmp/bbb-target-main/cargo-timings/cargo-timing-20260619T023138170Z-14ffed61c5c1036c.html`

For this warm focused sample, the default profile was faster than the warmed
`fast-test` profile. Keep `fast-test` opt-in for repeated focused iteration
after measurement, not as a blanket replacement for default-profile tests.

## Warm Update: 2026-06-19 After Small Object Collision Slice

Environment:

- At this measurement time, `sccache` was not installed on `PATH`.
- Stable target caches found:
  - `/tmp/bbb-target-main`: 19G.
  - `/tmp/bbb-target-native`: 3.3G.
  - `/tmp/bbb-target-net`: 720M.
  - `/tmp/bbb-target-pack`: 438M.
  - `/tmp/bbb-target-renderer`: 2.0G.
  - `/tmp/bbb-target-world`: 3.0G.
- No clean target was created for this update. The clean full-workspace baseline
  remains the 2026-06-18 disposable-target measurement above because this slice
  did not change Cargo profiles, dependencies, or cache policy.

Measured commands:

- Warm focused default:
  `scripts/cargo-dev.sh test -p bbb-world local_player_respects_piglin_wall_head_wider_collision --quiet`
  - Wall time: 0.13s.
  - Result: 1 test passed.
- Warm full workspace:
  `scripts/cargo-dev.sh timings --workspace --timings --quiet`
  - Wall time: 4.94s.
  - Target size: 19G.
  - Result: all tests passed.
  - Timing report:
    `/tmp/bbb-target-main/cargo-timings/cargo-timing-20260619T030451664Z-14ffed61c5c1036c.html`

The cache policy is behaving as intended for fast focused tests, but the main
target is now large enough that periodic size checks should stay part of the
workflow. Do not clean it automatically at slice boundaries; clean only for
real disk pressure, toolchain changes, or disposable clean-baseline runs.

## sccache Evaluation: 2026-06-19

Environment:

- `sccache 0.15.0` installed at `/opt/homebrew/bin/sccache`.
- `cargo 1.96.0-nightly (cbb9bb8bd 2026-03-13)`.
- `rustc 1.96.0-nightly (bcf3d36c9 2026-03-19)`.
- `RUSTC_WRAPPER=sccache` or `BBB_USE_SCCACHE=1` was used only through
  environment variables. No repo-local `.cargo/config.toml` was added.
- `sccache` stats were zeroed before each measured `sccache` run with
  `scripts/cargo-dev.sh sccache-zero-stats`.
- Disposable measurement targets were removed after recording the numbers to
  avoid growing `/tmp` by another 4G+.

Measured commands:

- Clean full workspace with `sccache`:
  `RUSTC_WRAPPER=sccache scripts/cargo-dev.sh timings-clean sccache-clean-20260619110904 --workspace --timings --quiet`
  - Wall time: 171.76s.
  - Target size: 3.2G.
  - Result: all tests passed.
  - Timing report:
    `/tmp/bbb-target-sccache-clean-20260619110904/cargo-timings/cargo-timing-20260619T030913544Z-14ffed61c5c1036c.html`
    before the disposable target was removed.
  - `sccache` stats:
    - compile requests: 217
    - executed: 156
    - cache hits: 0
    - cache misses: 156
    - non-cacheable calls: 59
    - cache size after run: 216M
- New worker target focused test with `sccache`:
  `BBB_USE_SCCACHE=1 BBB_CARGO_TARGET_NAME=sccache-worker-world-20260619110904 scripts/cargo-dev.sh test -p bbb-world local_player_respects_piglin_wall_head_wider_collision --quiet`
  - Wall time: 50.71s.
  - Target size: 636M.
  - Result: 1 test passed.
  - `sccache` stats:
    - compile requests: 46
    - executed: 29
    - cache hits: 0
    - cache misses: 29
    - non-cacheable calls: 17
    - cache size after run: 240M
- New worker target focused test without `sccache` for comparison:
  `CARGO_TARGET_DIR=/tmp/bbb-target-nosccache-worker-world-20260619110904 cargo test -p bbb-world local_player_respects_piglin_wall_head_wider_collision --quiet`
  - Wall time: 50.39s.
  - Target size: 635M.
  - Result: 1 test passed.
- Warm focused default with `sccache` on the stable main target:
  `RUSTC_WRAPPER=sccache CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test -p bbb-world local_player_respects_piglin_wall_head_wider_collision --quiet`
  - Wall time: 0.24s.
  - Result: 1 test passed.
  - `sccache` stats:
    - compile requests: 3
    - executed: 0
    - cache hits: 0
    - cache misses: 0

Conclusion:

- This measurement did not confirm a `sccache` benefit for multi-worktree cold
  focused tests. The new worker target with `sccache` was effectively the same
  wall time as the no-`sccache` comparison and reported zero cache hits.
- Do not make `sccache` the default repo setting.
- Keep using stable external `CARGO_TARGET_DIR` values as the main speedup.
- Keep `BBB_USE_SCCACHE=1` available for explicit future experiments, especially
  after dependency/profile/toolchain changes.
- Remove disposable measurement targets after recording results when disk
  pressure matters.

Follow-up measurement with `scripts/cargo-dev.sh sccache-eval`:

- Command:
  `scripts/cargo-dev.sh sccache-eval 20260619123234 -p bbb-world command_tree --quiet`
- Clean full workspace with `sccache`:
  - Wall time: 164.61s.
  - Target size before cleanup: 3.2G.
  - Result: all tests passed.
  - Timing report copied to:
    `/tmp/bbb-cargo-timings/cargo-timing-sccache-clean-20260619123234.html`
  - `sccache` stats:
    - compile requests: 217
    - executed: 156
    - cache hits: 1 C/C++ hit
    - Rust cache hits: 0
    - Rust cache misses: 155
    - non-cacheable calls: 59
    - cache size after run: 457M
- New worker target focused test with `sccache`:
  - Command:
    `RUSTC_WRAPPER=sccache CARGO_TARGET_DIR=/tmp/bbb-target-sccache-worker-20260619123234 cargo test -p bbb-world command_tree --quiet`
  - Wall time: 50.20s.
  - Target size before cleanup: 637M.
  - Result: 1 test passed.
  - `sccache` stats:
    - compile requests: 46
    - executed: 29
    - cache hits: 0
    - Rust cache misses: 29
    - non-cacheable calls: 17
    - cache size after run: 481M
- New worker target focused test without `sccache`:
  - Command:
    `CARGO_TARGET_DIR=/tmp/bbb-target-nosccache-worker-20260619123234 cargo test -p bbb-world command_tree --quiet`
  - Wall time: 49.99s.
  - Target size before cleanup: 637M.
  - Result: 1 test passed.
- Warm focused default with `sccache` on `/tmp/bbb-target-main`:
  - Command:
    `RUSTC_WRAPPER=sccache CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test -p bbb-world command_tree --quiet`
  - Wall time: 0.22s.
  - Result: 1 test passed.
  - `sccache` compile requests: 0
- Disposable measurement targets removed after recording:
  - `/tmp/bbb-target-sccache-clean-20260619123234`
  - `/tmp/bbb-target-sccache-worker-20260619123234`
  - `/tmp/bbb-target-nosccache-worker-20260619123234`

This second measurement keeps the same conclusion: `sccache` should not be made
default for this repo yet. It did not reduce the new worker target focused test
time, and Rust cache hits remained zero for the measured focused workload.

Installed Recheck:

- Command:
  `scripts/cargo-dev.sh sccache-eval 20260619-installed3 -p bbb-world command_tree --quiet`
- Clean full workspace with `sccache`:
  - Wall time: 165.43s.
  - Target size before cleanup: 3.2G.
  - Result: all tests passed.
  - Timing report copied to:
    `/tmp/bbb-cargo-timings/cargo-timing-sccache-clean-20260619-installed3.html`
  - `sccache` stats:
    - compile requests: 217
    - executed: 156
    - cache hits: 1 C/C++ hit
    - Rust cache hits: 0
    - Rust cache misses: 155
    - non-cacheable calls: 59
    - cache size after run: 697M
- New worker target focused test with `sccache`:
  - Wall time: 50.46s.
  - Target size before cleanup: 637M.
  - Result: 1 test passed.
  - `sccache` stats:
    - compile requests: 46
    - executed: 29
    - cache hits: 0
    - Rust cache misses: 29
    - non-cacheable calls: 17
    - cache size after run: 721M
- New worker target focused test without `sccache`:
  - Wall time: 49.29s.
  - Target size before cleanup: 638M.
  - Result: 1 test passed.
- Warm focused default with `sccache` on `/tmp/bbb-target-main`:
  - Wall time: 0.18s.
  - Result: 1 test passed.
  - `sccache` compile requests: 0
- Disposable measurement targets removed after recording:
  - `/tmp/bbb-target-sccache-clean-20260619-installed3`
  - `/tmp/bbb-target-sccache-worker-20260619-installed3`
  - `/tmp/bbb-target-nosccache-worker-20260619-installed3`

This installed recheck again does not support making `sccache` the default.
The new worker focused test was slightly faster without `sccache`, and measured
Rust cache hits stayed at zero. Keep `sccache` explicit through
`RUSTC_WRAPPER=sccache` or `BBB_USE_SCCACHE=1`.

Installed Recheck 2:

- Command:
  `scripts/cargo-dev.sh sccache-eval 20260619-r2 -p bbb-world command_tree --quiet`
- Clean full workspace with `sccache`:
  - Wall time: 169.42s.
  - Target size before cleanup: 3.2G.
  - Result: all tests passed.
  - Rust cache hits: 0.
- New worker target focused test with `sccache`:
  - Wall time: 50.31s.
  - Target size before cleanup: 638M.
  - Result: 1 test passed.
  - Rust cache hits: 0.
- New worker target focused test without `sccache`:
  - Wall time: 49.08s.
  - Target size before cleanup: 638M.
  - Result: 1 test passed.
- Warm focused default with `sccache` on `/tmp/bbb-target-main`:
  - Wall time: 1.42s.
  - Result: 1 test passed.
- Disposable measurement targets removed after recording:
  - `/tmp/bbb-target-sccache-clean-20260619-r2`
  - `/tmp/bbb-target-sccache-worker-20260619-r2`
  - `/tmp/bbb-target-nosccache-worker-20260619-r2`

This repeat run keeps the policy unchanged: `sccache` is available for explicit
experiments, but stable external target directories remain the practical speed
path for day-to-day focused tests and worker worktrees.

Installed Recheck 3:

- Command:
  `scripts/cargo-dev.sh sccache-eval 20260619143652 -p bbb-world command_tree --quiet`
- Clean full workspace with `sccache`:
  - Wall time: 169.73s.
  - Target size before cleanup: 3.2G.
  - Result: all tests passed.
  - Timing report copied to:
    `/tmp/bbb-cargo-timings/cargo-timing-sccache-clean-20260619143652.html`
  - `sccache` stats:
    - compile requests: 217
    - executed: 156
    - cache hits: 1 C/C++ hit
    - Rust cache hits: 0
    - Rust cache misses: 155
    - non-cacheable calls: 59
- New worker target focused test with `sccache`:
  - Wall time: 50.81s.
  - Target size before cleanup: 641M.
  - Result: 1 test passed.
  - `sccache` stats:
    - compile requests: 46
    - executed: 29
    - cache hits: 0
    - Rust cache misses: 29
    - non-cacheable calls: 17
- New worker target focused test without `sccache`:
  - Wall time: 50.42s.
  - Target size before cleanup: 640M.
  - Result: 1 test passed.
- Warm focused default with `sccache` on `/tmp/bbb-target-main`:
  - Wall time: 0.19s.
  - Result: 1 test passed.
  - `sccache` compile requests: 0
- Disposable measurement targets removed after recording:
  - `/tmp/bbb-target-sccache-clean-20260619143652`
  - `/tmp/bbb-target-sccache-worker-20260619143652`
  - `/tmp/bbb-target-nosccache-worker-20260619143652`

This recheck keeps the same policy: do not make `sccache` a default repo
setting. For this workload, explicit `RUSTC_WRAPPER=sccache` did not reduce
new-worker focused test time, and Rust cache hits stayed at zero.

Installed Live Recheck:

- Command:
  `scripts/cargo-dev.sh sccache-eval 20260619-live -p bbb-world command_tree --quiet`
- Clean full workspace with `sccache`:
  - Wall time: 175.35s.
  - Target size before cleanup: 3.2G.
  - Result: all tests passed.
  - Timing report copied to:
    `/tmp/bbb-cargo-timings/cargo-timing-sccache-clean-20260619-live.html`
  - `sccache` stats:
    - compile requests: 217
    - executed: 156
    - cache hits: 1 C/C++ hit
    - Rust cache hits: 0
    - Rust cache misses: 155
    - non-cacheable calls: 59
    - cache size after run: 2 GiB
- New worker target focused test with `sccache`:
  - Wall time: 52.37s.
  - Target size before cleanup: 644M.
  - Result: 1 test passed.
  - `sccache` stats:
    - compile requests: 46
    - executed: 29
    - cache hits: 0
    - Rust cache misses: 29
    - non-cacheable calls: 17
- New worker target focused test without `sccache`:
  - Wall time: 50.29s.
  - Target size before cleanup: 642M.
  - Result: 1 test passed.
- Warm focused default with `sccache` on `/tmp/bbb-target-main`:
  - Wall time: 0.19s.
  - Result: 1 test passed.
  - `sccache` compile requests: 0
- Disposable measurement targets removed after recording:
  - `/tmp/bbb-target-sccache-clean-20260619-live`
  - `/tmp/bbb-target-sccache-worker-20260619-live`
  - `/tmp/bbb-target-nosccache-worker-20260619-live`

This live recheck keeps `sccache` opt-in. The new worker focused test was
2.08s faster without `sccache`, Rust cache hits remained zero, and warm focused
iteration came from the stable `/tmp/bbb-target-main` cache rather than from
`sccache`.

Requested Recheck:

- Command:
  `scripts/cargo-dev.sh sccache-eval 20260619161103 -p bbb-world command_tree --quiet`
- Clean full workspace with `sccache`:
  - Wall time: 170.77s.
  - Target size before cleanup: 3.2G.
  - Result: all tests passed.
  - Timing report copied to:
    `/tmp/bbb-cargo-timings/cargo-timing-sccache-clean-20260619161103.html`
  - `sccache` stats:
    - compile requests: 217
    - executed: 156
    - cache hits: 1 C/C++ hit
    - Rust cache hits: 0
    - Rust cache misses: 155
    - non-cacheable calls: 59
    - cache size after run: 2 GiB
- New worker target focused test with `sccache`:
  - Wall time: 51.87s.
  - Target size before cleanup: 647M.
  - Result: 1 test passed.
  - `sccache` stats:
    - compile requests: 46
    - executed: 29
    - cache hits: 0
    - Rust cache misses: 29
    - non-cacheable calls: 17
- New worker target focused test without `sccache`:
  - Wall time: 50.94s.
  - Target size before cleanup: 647M.
  - Result: 1 test passed.
- Warm focused default with `sccache` on `/tmp/bbb-target-main`:
  - Wall time: 5.87s.
  - Result: 1 test passed.
  - `sccache` compile requests: 5
  - `sccache` executed compilations: 0
- Disposable measurement targets removed after recording:
  - `/tmp/bbb-target-sccache-clean-20260619161103`
  - `/tmp/bbb-target-sccache-worker-20260619161103`
  - `/tmp/bbb-target-nosccache-worker-20260619161103`

This requested recheck keeps the policy unchanged. `sccache` is installed and
usable through environment variables, but it did not reduce the measured worker
cold focused test time and still produced zero Rust cache hits. Keep it opt-in
through `RUSTC_WRAPPER=sccache` or `BBB_USE_SCCACHE=1`; do not add a
repo-local `.cargo/config.toml` wrapper setting.

Requested Recheck 2:

- Command:
  `scripts/cargo-dev.sh sccache-eval 20260619173725 -p bbb-world command_tree --quiet`
- Clean full workspace with `sccache`:
  - Wall time: 173.85s.
  - Target size before cleanup: 3.3G.
  - Result: all tests passed.
  - Timing report copied to:
    `/tmp/bbb-cargo-timings/cargo-timing-sccache-clean-20260619173725.html`
  - `sccache` stats:
    - compile requests: 217
    - executed: 156
    - cache hits: 1 C/C++ hit
    - Rust cache hits: 0
    - Rust cache misses: 155
    - non-cacheable calls: 59
    - cache size after run: 2 GiB
- New worker target focused test with `sccache`:
  - Wall time: 51.60s.
  - Target size before cleanup: 648M.
  - Result: 1 test passed.
  - `sccache` stats:
    - compile requests: 46
    - executed: 29
    - cache hits: 0
    - Rust cache misses: 29
    - non-cacheable calls: 17
- New worker target focused test without `sccache`:
  - Wall time: 50.45s.
  - Target size before cleanup: 647M.
  - Result: 1 test passed.
- Warm focused default with `sccache` on `/tmp/bbb-target-main`:
  - Wall time: 5.59s.
  - Result: 1 test passed.
  - `sccache` compile requests: 4
  - `sccache` executed compilations: 0

This recheck keeps the policy unchanged. `sccache` is installed and can be
used explicitly through environment variables, but the current multi-worktree
focused-test workload still shows zero Rust cache hits and no cold-worker
improvement. Keep stable external `CARGO_TARGET_DIR` caches as the primary
developer-speed path.

Requested Recheck 3:

- Command:
  `scripts/cargo-dev.sh sccache-eval 20260619-efficiency -p bbb-world command_tree --quiet`
- Clean full workspace with `sccache`:
  - Wall time: 170.85s.
  - Target size before cleanup: 3.3G.
  - Result: all tests passed.
  - Timing report copied to:
    `/tmp/bbb-cargo-timings/cargo-timing-sccache-clean-20260619-efficiency.html`
  - `sccache` stats:
    - compile requests: 217
    - executed: 156
    - cache hits: 1 C/C++ hit
    - Rust cache hits: 0
    - Rust cache misses: 155
    - non-cacheable calls: 59
    - cache size after run: 3 GiB
- New worker target focused test with `sccache`:
  - Wall time: 51.79s.
  - Target size before cleanup: 649M.
  - Result: 1 test passed.
  - `sccache` stats:
    - compile requests: 46
    - executed: 29
    - cache hits: 0
    - Rust cache misses: 29
    - non-cacheable calls: 17
- New worker target focused test without `sccache`:
  - Wall time: 50.76s.
  - Target size before cleanup: 651M.
  - Result: 1 test passed.
- Warm focused default with `sccache` on `/tmp/bbb-target-main`:
  - Wall time: 8.54s.
  - Result: 1 test passed.
  - `sccache` compile requests: 4
  - `sccache` executed compilations: 0

This recheck again shows no cold-worker improvement from `sccache` for the
current focused test path. Keep `sccache` optional and explicit, and continue
using stable external Cargo target directories as the default development
speed strategy.

Requested Recheck 4:

- Command:
  `scripts/cargo-dev.sh sccache-eval 20260619T132708 -p bbb-world command_tree --quiet`
- Clean full workspace with `sccache`:
  - Wall time: 175.27s.
  - Target size before cleanup: 3.3G.
  - Result: all tests passed.
  - Timing report copied to:
    `/tmp/bbb-cargo-timings/cargo-timing-sccache-clean-20260619T132708.html`
  - `sccache` stats:
    - compile requests: 217
    - executed: 156
    - cache hits: 1 C/C++ hit
    - Rust cache hits: 0
    - Rust cache misses: 155
    - non-cacheable calls: 59
    - cache size after run: 4 GiB
- New worker target focused test with `sccache`:
  - Wall time: 53.30s.
  - Target size before cleanup: 655M.
  - Result: 1 test passed.
  - `sccache` stats:
    - compile requests: 46
    - executed: 29
    - cache hits: 0
    - Rust cache misses: 29
    - non-cacheable calls: 17
- New worker target focused test without `sccache`:
  - Wall time: 51.06s.
  - Target size before cleanup: 655M.
  - Result: 1 test passed.
- Warm focused default with `sccache` on `/tmp/bbb-target-main`:
  - Wall time: 0.22s.
  - Result: 1 test passed.
  - `sccache` compile requests: 3
  - `sccache` executed compilations: 0

This installed recheck keeps the same conclusion: `sccache` is available but
did not reduce the measured new-worker cold focused test. The no-`sccache`
worker focused run was 2.24s faster, and Rust cache hits stayed at zero. Keep
it opt-in through `RUSTC_WRAPPER=sccache` or `BBB_USE_SCCACHE=1`; do not add a
repo-local mandatory `rustc-wrapper`.

Requested Recheck 5:

- Command:
  `scripts/cargo-dev.sh sccache-eval 20260619T145029 -p bbb-world command_tree --quiet`
- Clean full workspace with `sccache`:
  - Wall time: 175.78s.
  - Target size before cleanup: 3.3G.
  - Result: all tests passed.
  - Timing report copied to:
    `/tmp/bbb-cargo-timings/cargo-timing-sccache-clean-20260619T145029.html`
  - `sccache` stats:
    - compile requests: 217
    - executed: 156
    - cache hits: 1 C/C++ hit
    - Rust cache hits: 0
    - Rust cache misses: 155
    - non-cacheable calls: 59
    - cache size after run: 4 GiB
- New worker target focused test with `sccache`:
  - Wall time: 53.29s.
  - Target size before cleanup: 657M.
  - Result: 1 test passed.
  - `sccache` stats:
    - compile requests: 46
    - executed: 29
    - cache hits: 0
    - Rust cache misses: 29
    - non-cacheable calls: 17
- New worker target focused test without `sccache`:
  - Wall time: 50.64s.
  - Target size before cleanup: 655M.
  - Result: 1 test passed.
- Warm focused default with `sccache` on `/tmp/bbb-target-main`:
  - Wall time: 0.22s.
  - Result: 1 test passed.
  - `sccache` compile requests: 3
  - `sccache` executed compilations: 0
- Disposable targets removed after recording:
  - `/tmp/bbb-target-sccache-clean-20260619T145029`
  - `/tmp/bbb-target-sccache-worker-20260619T145029`
  - `/tmp/bbb-target-nosccache-worker-20260619T145029`

This recheck keeps the policy unchanged. `sccache` is installed and available,
but it did not reduce the measured cold worker focused test, and Rust cache
hits remained at zero. Keep stable external `CARGO_TARGET_DIR` caches as the
primary speedup and keep `sccache` explicit through `RUSTC_WRAPPER=sccache` or
`BBB_USE_SCCACHE=1`.

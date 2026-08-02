# Cargo Build Performance

> Canonical scope：当前构建/缓存规则见
> [`docs/wiki/development/build-and-cache.md`](wiki/development/build-and-cache.md)。本页仅
> 保留详细命令参考和版本化测量证据；若与 Wiki 或 `goal.md` 冲突，以后两者为准。

This document records the current Cargo build baseline and the local workflow
used to keep focused tests fast without weakening checkpoint or final merge gates.

## Goals

- Preserve the final gate defined in `goal.md` under "提交前门禁"
  (`scripts/cargo-dev.sh gate` runs it against `/tmp/bbb-target-main`).
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
5. Keep checkpoint and final merge gates on the default profile, running the
   `goal.md` gate against `CARGO_TARGET_DIR=/tmp/bbb-target-main`.

## Checkpoint Gate Policy

The authoritative rules live in `goal.md`; this section only explains their build-cache
impact. Each atomic-slice commit and engineering-slice phase commit runs affected-crate
focused tests and the required cheap checks before its clean commit. Two to four related
low-risk atomic-slice commits may share one full workspace gate at the batch tip.
Engineering slices use the serial milestone plan established before work; high-risk
engineering milestones may cover several buildable phase commits but never join an
ordinary low-risk batch. Domain changes, delivery/merge boundaries, abnormal focused
tests, and the high-risk conditions named in `goal.md` force an earlier checkpoint. This
reduces repeated workspace test cost without changing the requirement that delivered or
merged HEAD has passed the full gate.

For a normal green batch or engineering checkpoint, run `scripts/cargo-dev.sh gate` once
at the latest commit. Do not run `batch-gate` merely because the checkpoint contains
multiple commits. `batch-gate` is reserved for a failed tip gate whose introducing commit
is still unclear after focused diagnosis; after locating the regression, fix it with a
normal commit and rerun the full gate at the new tip.

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
scripts/cargo-dev.sh check -p bbb-world
scripts/cargo-dev.sh timings --workspace --timings
scripts/cargo-dev.sh timings-clean clean-baseline-YYYYMMDD --workspace --timings
scripts/cargo-dev.sh sccache-eval YYYYMMDD -p bbb-world command_tree --quiet
scripts/cargo-dev.sh size
scripts/cargo-dev.sh prune
scripts/cargo-dev.sh clean-target clean-baseline-YYYYMMDD
scripts/cargo-dev.sh sccache-status
```

The script defaults to `CARGO_TARGET_DIR=/tmp/bbb-target-main`. Set
`BBB_CARGO_TARGET_NAME=renderer`, `world`, or `net` to use
`/tmp/bbb-target-renderer`, `/tmp/bbb-target-world`, or `/tmp/bbb-target-net`.
An explicit `CARGO_TARGET_DIR` still wins.

`scripts/cargo-dev.sh gate` runs the authoritative merge gate defined in
`goal.md`. A normal invocation uses `/tmp/bbb-target-main`; an explicit target
selection is honored for isolated worktrees. It is a convenience wrapper, not a
weaker test path.

After a normal tip gate has failed and focused diagnosis is insufficient,
`scripts/cargo-dev.sh batch-gate <good>..<bad>` tests a committed first-parent
range without checking out commits in the caller's worktree. It creates a
temporary detached worktree and runs the authoritative gate once at `<bad>`.
When the tip fails, it verifies `<good>` is green and then binary-searches the
first-parent commits for the first failure. Do not use this command for a normal
green checkpoint. The range must be monotonic (a gate failure remains failing
through `<bad>`); use the recorded last-gated commit as `<good>`.
The command returns 0 when the tip passes, 1 when it finds a failure or the
baseline is already failing, and 2 for an invalid range or setup error. Per-
commit logs are retained under a printed `/tmp/bbb-batch-gate-logs.*` path.
Batch runs use the distinct `/tmp/bbb-target-batch-gate` cache by default
(`BBB_BATCH_GATE_TARGET_NAME` changes the suffix). Keeping this cache separate
prevents compile-time source paths such as `CARGO_MANIFEST_DIR` from a temporary
worktree contaminating `/tmp/bbb-target-main` after that worktree is removed.

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
- Failed-checkpoint regression locator:
  `scripts/cargo-dev.sh batch-gate <last-gated-commit>..HEAD`
- Target cache size:
  `scripts/cargo-dev.sh size`
- Stale target cleanup preview:
  `scripts/cargo-dev.sh prune`
- Stale target cleanup:
  `scripts/cargo-dev.sh prune --force`
- Disposable target cleanup:
  `scripts/cargo-dev.sh clean-target clean-baseline-YYYYMMDD`
- Checkpoint/final merge gate:
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

`fast-test` builds at opt-level 0, whose stack frames are roughly 3x larger
than the default profile's (measured 2026-07-21 on the depth-64 dialog
reference walk: ~33.4 KiB/level vs ~10.3 KiB/level). Deep or dispatch-heavy
tests can therefore exceed the default 2 MiB test-thread stack only in this
profile. `scripts/cargo-dev.sh fast-test` exports
`RUST_MIN_STACK=16777216` for exactly this reason; raw
`cargo test --profile fast-test` runs need the same variable. The default
gate profile intentionally keeps standard 2 MiB stacks as its frame-size
discipline, and the dialog semantic-depth budget test pins the one known deep
recursion in bytes across profiles.

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
scripts/cargo-dev.sh prune
```

`scripts/cargo-dev.sh prune` is the single entry point for stale-cache
cleanup. It scans `/tmp/bbb-target-*`, prints a keep/delete classification
with sizes, and deletes nothing without `--force`. The keep-list covers the
long-lived caches (`main`, `batch-gate`, `playable`) and the stable
per-domain caches (`world`, `renderer`, `net`, `native`, `protocol`, `pack`,
`control`, `audio`, `item-model`, `render-types`); every other target is
treated as a disposable slice cache. The list lives in
`scripts/cargo-dev.sh` (`PRUNE_KEEP_LIST`) and must stay in sync with this
section. `prune --force` also removes a repo-local `target/` directory,
which repo policy keeps out of normal agent work.

Remove disposable clean baselines after their numbers and timing report paths
are recorded; they are never on the keep-list, so prune covers them too, and
`clean-target` remains available for deleting one specific target:

```sh
scripts/cargo-dev.sh clean-target clean-baseline-YYYYMMDD
```

Remove completed worker targets through the worker helper only when they are
intentionally disposable:

```sh
scripts/worker-worktree.sh cleanup world --remove-target
```

Clean keep-list targets such as `main`, `world`, `net`, and `renderer` only
when disk pressure is real, when changing Rust toolchains or build profiles,
or when a dependency feature experiment has made the cache misleadingly
large. Expect the next focused or full test on that target to rebuild.

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

## Linux Baseline and Linker Evaluation: 2026-07-16

Environment:

- Ubuntu Linux, kernel `6.8.0-124-generic`, `x86_64`.
- `cargo 1.96.0-nightly (cbb9bb8bd 2026-03-13)`.
- `rustc 1.96.0-nightly (bcf3d36c9 2026-03-19)`.
- `sccache` was not installed on `PATH`.
- `/usr/bin/ld.lld` was available as Ubuntu LLD 18.1.3; `mold` was not
  installed. The lld runs used `RUSTFLAGS='-C link-arg=-fuse-ld=lld'`.
- `cargo-nextest` was not installed. It was not downloaded or installed for
  this measurement, so no nextest wall-time comparison is claimed.
- Wall time came from Bash `time -p`. The host did not provide
  `/usr/bin/time`; `scripts/cargo-dev.sh` now detects that condition and falls
  back to the Bash timing keyword.

Measured results (all tests passed):

| Workload | Default linker | lld | Difference |
| --- | ---: | ---: | ---: |
| Clean full workspace | 125.19s | 124.84s; 126.97s report repeat | lld ranged from 0.3% faster to 1.4% slower |
| Warm full workspace | 6.23s | 6.28s | lld 0.05s / 0.8% slower |
| Warm focused `bbb-world command_tree` | 0.13s | 0.14s | lld 0.01s slower |
| Cold worker focused `bbb-world command_tree` | 81.51s | 81.83s | lld 0.32s / 0.4% slower |

Targets and reports:

- Default clean full:
  `CARGO_TARGET_DIR=/tmp/bbb-target-linux-default-clean-20260716 cargo test --workspace --timings --quiet`;
  10G target; timing report copied to
  `/tmp/bbb-cargo-timings/cargo-timing-linux-default-clean-20260716.html`.
- lld clean full:
  `RUSTFLAGS='-C link-arg=-fuse-ld=lld' CARGO_TARGET_DIR=/tmp/bbb-target-linux-lld-clean-report-20260716 cargo test --workspace --timings --quiet`;
  126.97s and 10G target on the retained-report repeat; timing report copied to
  `/tmp/bbb-cargo-timings/cargo-timing-linux-lld-clean-20260716.html`.
- The default and lld cold-worker targets were 2.4G each. Warm runs reused the
  matching clean target sequentially; no two Cargo commands shared a target
  concurrently in the recorded measurements.
- An earlier warm-focused attempt waited on a still-running same-target Cargo
  process and was discarded. The table records the clean subsequent runs.
- The five disposable measurement targets (including the lld clean-report
  repeat) were removed after their sizes and
  timing reports were recorded. Stable developer/worker caches were not
  removed.

Top default-linker clean timing entries were `bbb-world` test (83.7s),
`bbb-item-model` test with `test-support` (57.8s), `bbb-native` test (40.3s),
`bbb-renderer` test (36.6s), `bbb-world` library (26.7s), and
`x11rb-protocol` (23.3s). This confirms that compile/codegen work, rather than
the final linker choice, dominates this baseline.

Conclusion:

- lld did not produce a material win in any measured workload, so no linker
  environment switch was added to `scripts/cargo-dev.sh` and no repository-
  level linker configuration is justified. Keep the explicit `RUSTFLAGS` form
  for future rechecks. `mold` remains unmeasured because it was unavailable.
- nextest remains unadopted and unmeasured on this host. Re-evaluate it only
  when already installed, using the warm full-workspace test workload; it is
  not expected to change clean compilation cost.
- Stable external targets remain the effective throughput control: a warm
  focused test was 0.13s, while a new worker target took 81.51s.

## dev `debug = 1` Evaluation and Adoption: 2026-07-16

Environment:

- Same Linux host and toolchain as the 2026-07-16 linker baseline above.
- The measurement used the `CARGO_PROFILE_DEV_DEBUG=1` environment override
  so `Cargo.toml` stayed unchanged until the adoption decision; the
  comparison row is the same-day default-linker clean baseline (125.19s,
  10G target).

Measured results (all tests passed):

| Workload | dev debug = 2 | dev debug = 1 | Difference |
| --- | ---: | ---: | ---: |
| Clean full workspace | 125.19s, 10G target | 104.03s, 6.5G target | 17% faster, 35% smaller |
| Warm focused `bbb-world command_tree` | 0.13s | 0.13s | no change |

Commands and artifacts:

- debug=1 clean:
  `CARGO_PROFILE_DEV_DEBUG=1 CARGO_TARGET_DIR=/tmp/bbb-target-debug1-eval cargo test --workspace --timings --quiet`;
  timing report copied to
  `/tmp/bbb-cargo-timings/cargo-timing-debug1-clean-20260716.html`.
- The first focused run on the fresh eval target rebuilt the `-p bbb-world`
  unit set once (67.56s) because its feature unification differs from the
  `--workspace` build; that cost exists on any fresh target, is unrelated to
  the debug level, and did not repeat (0.13s on both reruns).
- The disposable eval target was removed after recording. `main` was cleaned
  before the post-adoption gate because a profile change invalidates its
  cached fingerprints (see "Periodic Cleanup").

Conclusion:

- Adopted: `[profile.dev] debug = 1` cuts clean target size by ~35% and clean
  wall time by ~17% with no warm focused regression. Test backtraces keep
  line numbers; variable-level debugger inspection is the accepted
  trade-off.
- `[profile.fast-test] debug = 1` was removed as redundant (it inherits
  `dev`).
- Per-domain caches keep old-profile artifacts as dead weight until their
  next use; clean them with `scripts/cargo-dev.sh prune --force` if disk
  pressure matters more than one rebuild per domain.

## deny-warnings Moved from RUSTFLAGS to Workspace Lints: 2026-07-16

Environment: same Linux host and toolchain as the 2026-07-16 linker baseline.

Problem: the gate ran `RUSTFLAGS='-D warnings'` while daily focused tests ran
without `RUSTFLAGS`, and both used `/tmp/bbb-target-main`. Cargo hashes
`RUSTFLAGS` into unit fingerprints, so every focused-test → gate → focused-test
transition invalidated the whole cache in both directions.

Measured on `/tmp/bbb-target-main` (cache left in the `-D warnings` state by
the previous gate):

| Command | Wall time | CPU time |
| --- | ---: | ---: |
| `RUSTFLAGS='-D warnings' cargo check --workspace` (matching state) | 0.16s | — |
| `cargo check --workspace` (flag removed, full re-check) | 29.0s | 5m30s |

The `cargo test` side of the same toggle is larger because it redoes codegen
and linking. Each slice paid this twice (gate direction and back).

Adopted fix:

- `[workspace.lints.rust] warnings = "deny"` in the root `Cargo.toml`, with
  `[lints] workspace = true` in every crate manifest.
- The gate now runs plain `cargo check --workspace` / `cargo test --workspace`
  with no `RUSTFLAGS`, so gate and focused tests share one warm cache state.
- Do not set `RUSTFLAGS='-D warnings'` manually anymore; it reintroduces the
  thrash. Denied warnings now also fail focused tests immediately, which
  surfaces lint breakage before the gate instead of at gate time.
- One-time costs: the manifest change rebuilt the ten workspace crates once
  (16.5s check side), and each stable target rebuilds once on first use after
  the change.

The gate also skips the scripts python suite (185 tests, 6.7s) when the change
under gate does not touch `scripts/`; `BBB_GATE_FULL=1` forces every step. A
dirty tree is diffed against `HEAD`; a clean tree (batch-gate bisection) is
diffed against the commit's parent.

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
- The recorded baselines below were taken on a macOS machine; the current
  development host is Linux (`x86_64`). Re-measure before trusting absolute
  numbers, and evaluate `mold`/`lld` — the earlier "do not prioritize a fast
  linker" guidance was specific to the macOS toolchain and does not carry over.
- Entity projection / client-tick hot-path baselines live in
  `docs/performance-baselines.md`.

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

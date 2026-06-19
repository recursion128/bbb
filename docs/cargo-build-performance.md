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

## Target Directories

Use stable external target directories:

```sh
CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test -p bbb-world <filter>
CARGO_TARGET_DIR=/tmp/bbb-target-renderer cargo test -p bbb-renderer <filter>
CARGO_TARGET_DIR=/tmp/bbb-target-world cargo test -p bbb-world <filter>
CARGO_TARGET_DIR=/tmp/bbb-target-net cargo test -p bbb-net <filter>
```

Do not run parallel Cargo commands against the same target directory. Cargo will
serialize on package/build locks, which removes most of the benefit of parallel
workers.

Do not commit a repo-local `.cargo/config.toml` that forces one target
directory. It would make independent worker worktrees share the same cache and
lock unexpectedly. Prefer explicit `CARGO_TARGET_DIR` in commands and worker
prompts.

Repo-local `target` stays ignored and should not be generated during normal
agent work.

## Helper Script

Use `scripts/cargo-dev.sh` to avoid retyping target-cache commands during
focused development:

```sh
scripts/cargo-dev.sh test -p bbb-world <filter>
scripts/cargo-dev.sh fast-test -p bbb-world <filter>
BBB_CARGO_TARGET_NAME=world scripts/cargo-dev.sh test -p bbb-world <filter>
scripts/cargo-dev.sh timings --workspace --timings
scripts/cargo-dev.sh timings-clean clean-baseline-YYYYMMDD --workspace --timings
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
- Target cache size:
  `scripts/cargo-dev.sh size`
- Disposable target cleanup:
  `scripts/cargo-dev.sh clean-target clean-baseline-YYYYMMDD`
- Final merge gate:
  `scripts/cargo-dev.sh gate`

## sccache

`sccache` is useful for repeated dependency and workspace crate compilation
across multiple target directories and worktrees, but it should remain optional
local tooling.

Do not commit a mandatory `rustc-wrapper` setting. Machines without `sccache`
would fail before compiling.

Use it explicitly when installed:

```sh
RUSTC_WRAPPER=sccache CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test -p bbb-world <filter>
BBB_USE_SCCACHE=1 scripts/cargo-dev.sh test -p bbb-world <filter>
scripts/cargo-dev.sh sccache-status
```

Record before/after timings before making `sccache` part of a default local
shell profile. Workspace incremental builds and `sccache` do not always improve
the same workload, so use measured data.

## Profiles

The default `test` profile remains the authoritative gate. The workspace also
has an opt-in `fast-test` profile for daily focused tests:

```sh
CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test --profile fast-test -p bbb-world <filter>
```

`fast-test` has its own profile output directory. Its first run can be slower
than a warm default-profile focused test because it has to build a separate
cache. Use it when you expect multiple focused iterations in the same profile,
not as a replacement for the final gate.

## Timing Commands

Warm full workspace timing:

```sh
/usr/bin/time -p env CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test --workspace --timings
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

## Next Evaluation Points

- Install and test `sccache` with clean and warm focused workloads across two
  different external target directories.
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

- `sccache` is still not installed on `PATH`.

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

- `sccache` is still not installed on `PATH`.
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
Do not add a repo-local mandatory `rustc-wrapper` until `sccache` is installed
and measured against clean and warm focused workloads.

## Warm Update: 2026-06-19 After Cartography Slice

Environment:

- `sccache` is still not installed on `PATH`.
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

- `sccache` is still not installed on `PATH`.
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
complete. Keep `sccache` optional until it is installed and measured locally.

## Warm Update: 2026-06-19 After Bundle Click Slice

Environment:

- `sccache` is still not installed on `PATH`.
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

- `sccache` is still not installed on `PATH`.
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
`fast-test`, and per-worker target directories. The next build-performance
change should be measured `sccache` adoption after installing it locally; keep
`rustc-wrapper` out of repo-local Cargo config until that comparison exists.

## Warm Update: 2026-06-19 After Sprint Jump Slice

Environment:

- `sccache` is still not installed on `PATH`.
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

- `sccache` is still not installed on `PATH`.
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

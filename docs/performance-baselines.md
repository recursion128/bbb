# Performance Baselines

First recorded performance baselines for the repo. These are reference numbers for
the entity projection and client-tick hot paths, captured with
[criterion](https://docs.rs/criterion). They exist so that changes to the
projection chain can be compared against a known starting point, not treated as
hard budgets.

## What is benched

The bench lives in `crates/bbb-world/benches/entity_projection.rs` and drives three
public `WorldStore` entry points that the renderer and control layers hit every
frame / tick, at entity counts 100 / 500 / 2000:

- `entity_model_sources_at_partial_tick(0.5)` — the render-model projection chain
  (position lerp, equipment, minecart rail interpolation, per-type animation
  state).
- `entity_pick_targets_at_partial_tick(0.5)` — the pick/hover bounds projection.
- `advance_entity_client_animations(1)` — one client tick of the animation / lerp
  advance loop.

Every entity is built through the same public packet-apply API a downstream crate
would use (`apply_add_entity`, `apply_move_minecart_along_track`,
`apply_set_equipment`). The type mix is mostly common mobs (zombie / cow / pig /
sheep / skeleton / creeper) with a steady minority of minecarts (lerp branch) and
horses / llamas (equipment branch). Positions and the type mix are index-derived
and fully deterministic — no randomness — so runs are comparable across commits.

Coverage limitation: `bbb-native` is a binary-only crate (no `lib` target), so its
runtime orchestration cannot host a criterion bench. The projection cost is
measured at its owning crate, `bbb-world`, which is where the hot code actually
lives. `bbb-world` is currently the only crate with a bench harness.

## Environment

| Field | Value |
| --- | --- |
| CPU | AMD Ryzen 9 7945HX (32 logical CPUs) |
| OS | Linux 6.8.0-124-generic (Ubuntu), x86_64 |
| rustc | 1.96.0-nightly (bcf3d36c9 2026-03-19) |
| criterion | 0.5.1 |
| Profile | `bench` (release: opt-level 3, thin LTO, codegen-units 1) |

Numbers are wall-clock and machine/load dependent; treat them as an order-of-
magnitude reference, not an exact contract. The bench caps warm-up at 500 ms and
measurement at 2 s per case so the full suite finishes in well under two minutes.

## Command

```sh
CARGO_TARGET_DIR=/tmp/bbb-target-wt-bench cargo bench -p bbb-world
```

## Results

Times are criterion's reported median with the 95% confidence interval in
brackets `[lower .. upper]`.

| Benchmark | 100 entities | 500 entities | 2000 entities |
| --- | --- | --- | --- |
| `entity_model_sources_at_partial_tick` | 277 µs [239 .. 316] | 843 µs [807 .. 880] | 5.12 ms [4.70 .. 5.57] |
| `entity_pick_targets_at_partial_tick` | 18.6 µs [17.9 .. 19.4] | 87.9 µs [87.5 .. 88.3] | 467 µs [420 .. 527] |
| `advance_entity_client_animations` (1 tick) | 13.7 µs [13.3 .. 14.1] | 69.5 µs [69.3 .. 69.7] | 323 µs [308 .. 343] |

Model-source projection dominates: it is the widest chain and the one most worth
watching. The `100`-entity model-source case showed noticeably wider variance than
the others on this run (background load); re-measure on an idle machine before
reading much into small deltas there.

## When to re-measure

- After any change to the entity projection chain
  (`entity_model_sources_at_partial_tick`, `model_source`, `model_targets_*`, the
  per-type animation projection, or minecart/lerp interpolation).
- Before starting an optimization pass on entity ticking or projection — capture a
  fresh "before" on the current machine, since absolute numbers are hardware- and
  load-dependent, then compare "after" on the same machine and load.
- When entity counts or the constructed type mix in the bench change, so the table
  keeps describing the same workload.

Regenerate this table by re-running the command above and copying criterion's
median and CI for each case.

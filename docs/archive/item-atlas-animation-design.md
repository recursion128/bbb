# Item Atlas Animation Parity

Status: completed on 2026-07-26.

This document freezes the engineering contract for closing the remaining
resource-pack atlas-animation gap. It refines the pack-backed consumer boundary
in [`native-client-architecture.md`](../native-client-architecture.md) and the
accepted-generation/reload contract in
[`resource-pack-lifecycle-design.md`](../resource-pack-lifecycle-design.md).

## Goal and Completion

Every animated sprite selected into the item atlas must advance on the client
tick clock and update every implemented consumer of that atlas. Completion
requires:

- `bbb-item-model` to retain the selected `SpriteImage` animation inputs and
  produce a deterministic whole-atlas frame for a supplied client tick;
- generated-item side geometry to use the union of opaque pixels across every
  unique animation frame, matching Vanilla's frame scan;
- `bbb-native` to advance the accepted item generation at 50 ms per client tick
  and upload one coherent frame to both renderer-owned item atlas copies; and
- `bbb-renderer` to update the HUD item texture and the shared model/entity/item-
  particle texture in place, without invalidating static mesh residency.

The initial upload remains animation tick zero. A resource reload replaces the
whole `NativeItemRuntime`, which also resets its animation clock; a failed frame
stitch or GPU write leaves the last successfully uploaded frame visible and
emits a warning.

## Non-Goals

- No mip levels for item or particle atlases. Vanilla 26.1 creates mipmaps only
  for the blocks atlas.
- No change to terrain or particle animation, standalone texture samplers,
  atlas definitions, resource selection, item model selection, tint, glint,
  lighting, UV layout, or draw ordering.
- No renderer-side parsing of `.mcmeta`, no second animation schema, and no
  per-frame atlas recreation.
- No implementation of other atlas families that do not yet have a repo-native
  presentation consumer.

## Ownership and Integration Contract

- `bbb-pack` remains the sole owner of `.png.mcmeta` animation parsing, frame
  validation, frame timing, interpolation, and atlas stitching.
- `bbb-item-model` owns the immutable accepted item images and `AtlasPacker`.
  It exposes only `has_texture_animation()` and
  `texture_animation_atlas_frame(tick)`, plus its existing atlas dimensions and
  base pixels. Its generated-item alpha mask is the opaque union of every
  unique referenced frame.
- `bbb-native` owns the wall-clock-to-client-tick bridge. The clock lives inside
  the accepted `NativeItemRuntime`, advances by elapsed whole 50 ms intervals,
  and resets when reload swaps that runtime.
- `bbb-renderer` owns the two GPU textures. Narrow update methods validate the
  original dimensions and byte length, then write mip level zero in place.
  Updating pixels must not recreate bind groups or invalidate item-model atlas
  residency because atlas layout and UVs do not change.

## Vanilla 26.1 Evidence

Authoritative sources under
`/home/zgy/Work/mc-code/sources/26.1/`:

- `net/minecraft/client/resources/model/sprite/AtlasManager.java` declares
  `blocks` with `createMipmaps=true`, while `items` and `particles` are false.
- `net/minecraft/client/renderer/texture/TextureAtlas.java#tick` advances every
  animated sprite state and uploads dirty or interpolated frames to the
  existing atlas texture.
- `SpriteContents.AnimationState#tick` advances before drawing the next client
  tick, and `drawToAtlas` interpolates the current and next frame by
  `subFrame / frameTime`.
- `net/minecraft/client/resources/model/cuboid/ItemModelGenerator.java`
  processes every unique animation frame when deriving the generated model's
  side spans.

## Ordered Stages, Tests, and Safe Stops

### A. Immutable item animation owner

Retain the atlas packer/images, expose deterministic animation-frame queries,
and build generated-item alpha masks from the union of unique animation frames.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-item-model item_texture
scripts/cargo-dev.sh test -p bbb-item-model animated
```

This is a buildable safe stop. No production caller advances the frame yet, so
the existing tick-zero presentation remains unchanged.

### B. Native clock and renderer consumers

Add in-place updates for both item GPU atlases, advance the accepted runtime at
the 50 ms client-tick cadence, and invoke it once per rendered client frame.

Focused tests:

```text
scripts/cargo-dev.sh test -p bbb-renderer item_atlas
scripts/cargo-dev.sh test -p bbb-native item_texture_animation
```

This is the integration safe stop. Failures localize to pack stitching,
wall-clock advancement, or one of the two named GPU owners.

### C. Closure and gate

Delete the atlas-animation todo, append the engineering milestone to
`docs/goal-archive.md`, archive this contract, run per-commit checks, commit,
then run:

```text
scripts/cargo-dev.sh gate
scripts/audit-p0.sh
```

The final HEAD must be clean and fully gated. Fixes move forward; animation
failure never rolls back an already accepted resource generation.

# Full Native Rewrite Plan

This repo is being rewritten as a native Minecraft Java 26.1 client. The target is
not compatibility with an older scaffold; old files and code may be replaced when
they block a correct native client.

## Source Of Truth

- Local vanilla reference: `<MC_CODE_ROOT>/sources/26.1/`.
- Packet ids, field order, nullable semantics, flags, and enum ordinals must be
  checked against vanilla sources before protocol or world-state changes.
- Renderer behavior should use vanilla assets and may be compared against the
  official client when visual details are ambiguous.

## Target Architecture

- `bbb-protocol`: codecs, packet ids, packet structs, encode/decode tests.
- `bbb-world`: canonical client state, packet application, counters, query APIs.
- `bbb-net`: connection lifecycle, event stream, commands, probes.
- `bbb-pack`: vanilla pack/model/texture/colormap/atlas loading.
- `bbb-renderer`: terrain mesh generation, HUD geometry, GPU resources, render
  pipelines.
- `bbb-control`: snapshots, control API, probe/report formatting.
- `bbb-native`: runtime orchestration, input, camera, upload scheduling.
- `bbb-platform`: window/input platform boundary.

Root `lib.rs` and `main.rs` files should converge toward indexes and thin glue.
Feature behavior belongs in semantic modules with local tests.

## Rewrite Phases

1. Establish maintainable module boundaries.
   - Split large root files only when a clear semantic boundary improves the
     current or near-term feature work.
   - Keep public API stable unless a downstream crate is updated in the same
     verified slice.
   - Move tests next to the module that owns the behavior.

2. Complete protocol coverage for required play/configuration packets.
   - Verify packet ids and wire order against vanilla 26.1.
   - Add encode/decode tests for representative payloads and edge cases.

3. Build canonical world state.
   - Apply all implemented clientbound events into structured state.
   - Track counters for received/applied/ignored events.
   - Provide query APIs used by native runtime, renderer, and control.

4. Replace scaffold networking with native client behavior.
   - Drive login, configuration, play transition, compression, keepalive, and
     resource-pack responses.
   - Encode movement, interaction, inventory, and command requests from native
     input/state.

5. Load vanilla assets and models.
   - Resolve blockstates, model parents, textures, tints, colormaps, and atlas
     layout from 26.1 assets.
   - Preserve vanilla-shaped model geometry and render layers.

6. Implement native rendering and interaction.
   - Mesh terrain by material layer with vanilla geometry/tints/light.
   - Render HUD, selection outline, camera pose, screenshots, and interaction
     feedback.
   - Compare rendering details with the official client when local tests are not
     sufficient.

7. Harden runtime workflows.
   - Keep event draining deterministic.
   - Preserve focus/input edge cases.
   - Add integration tests around probe, snapshot, and command loops.

## Verification Gates

Every slice should run the narrow package tests it affects. Before commit, run:

```sh
cargo fmt --check
git diff --check
cargo test --workspace
```

If a slice depends on local vanilla sources, document the path and any skipped
tests. Do not treat visual behavior as complete until it is either covered by a
deterministic renderer test or manually compared with the official client.

## Agent Workflow

- Main agent owns planning, task split, review, full test gate, and commits.
- Worker agents get disjoint file/module ownership and do not commit.
- No agent rewrites git history unless the user explicitly asks for it.
- Consider mechanical module extraction before large feature additions only when
  `docs/code-organization-style.md` says the split is justified.

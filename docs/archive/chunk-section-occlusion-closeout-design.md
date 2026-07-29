# Chunk Section Occlusion Closeout

Status: completed engineering-slice contract.

## Goal and completion

Close the `chunk_section_paths`, `chunk_section_octree`, and
`chunk_section_visibility` gap with one repo-native section owner:

- every compiled resident section carries Vanilla 26.1's six-by-six
  `facesCanSeeEachother` result derived from exact `BlockState.isSolidRender`
  facts;
- native propagation follows `SectionOcclusionGraph` direction/source ordering
  and builds the camera-ordered octree used for frustum traversal;
- renderer terrain draws and `visible_sections` counters consume the resulting
  visible owner set without replacing unchanged GPU buffers; and
- the three debug entries render from that same graph/matrix/octree state.

Completion requires focused matrix, propagation, octree, renderer-filter, and
debug-gizmo tests; removal of the owning feature-ledger item; a deterministic
debug screenshot or equivalent pixel readback; full gate; and live playable
gate because this changes production terrain visibility.

## Non-goals and fallback

- Do not expand the existing 49-chunk terrain residency budget or build a
  second chunk cache.
- Do not add GPU occlusion queries or reproduce Vanilla's executor/future
  scheduling. The immutable graph result and observable traversal semantics
  are the contract.
- Do not put presentation state in serializable `bbb-world`.
- Missing block support facts are transparent for the visibility flood fill.
  Missing/uncompiled graph data remains visible. A stale or malformed visible
  owner set must never hide all resident terrain.

## Vanilla 26.1 facts

Authoritative sources are local
`~/Work/mc-code/sources/26.1/net/minecraft/client/renderer/`:

- `chunk/SectionCompiler.compile` marks `VisGraph` cells only when
  `BlockState.isSolidRender()` is true.
- `chunk/VisGraph.resolve` returns all-visible below 256 opaque cells,
  all-hidden when all 4096 cells are opaque, otherwise flood-fills transparent
  boundary cells in `Direction.values()` order.
- `chunk/VisibilitySet` stores a symmetric six-by-six matrix in Vanilla
  direction order: down, up, north, south, west, east.
- `SectionOcclusionGraph.runUpdates` performs FIFO propagation in that same
  direction order, rejects immediate backtracking, checks every accumulated
  source direction against the current section matrix, and applies the
  60-block distant continuity check before adding a neighbour.
- `Octree` uses the smallest power-of-two root covering
  `2 * viewDistance + 1` sections, camera-relative axis sorting, and
  branch-level frustum acceptance before visiting leaves.
- `ChunkCullingDebugRenderer` draws source-direction paths and red lines for
  matrix pairs that cannot see each other; `OctreeDebugRenderer` draws every
  visited branch/leaf bound and numbers section leaves.

## Frozen ownership and API

- `bbb-world` exposes the generated `solid_render` fact in each extracted
  terrain cell; it does not own the graph.
- `bbb-renderer::terrain` owns the compact face matrix carried by
  `TerrainSectionMeshLayers`.
- `bbb-native::terrain_runtime::occlusion` owns section positions, propagation
  nodes, octree hierarchy, frustum traversal, and debug projections.
- `bbb-renderer::Renderer` accepts a set of stable section owners
  `(level_generation, chunk_x, section_y, chunk_z)` and filters all terrain
  layers at draw time. Mesh upload/revision ownership stays unchanged.
- Block-entity/entity compiled-section gates consume the same visible set
  published by native; no parallel heuristic set remains.

## Serial stages and validation

1. **Compiled face matrix.** Add exact solid-render extraction and a compact
   symmetric matrix to every section result, including empty sections.
   Focused tests:
   `scripts/cargo-dev.sh test -p bbb-renderer section_visibility` and
   `scripts/cargo-dev.sh test -p bbb-native terrain_runtime`.
2. **Graph, octree, and production filter.** Build the immutable graph from
   resident revisions, loaded-neighbour facts, camera section, smart-cull
   option, and render distance; traverse it through `CameraFrustum`; filter
   opaque/cutout/translucent draws and counters by stable owner. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-native terrain_runtime::occlusion` and
   `scripts/cargo-dev.sh test -p bbb-renderer terrain_visibility`.
3. **Debug closeout.** Project paths, blocked face pairs, octree bounds, and leaf
   labels into existing debug lines/boxes/text surfaces, then delete the ledger
   item and archive this contract. Focused tests:
   `scripts/cargo-dev.sh test -p bbb-native chunk_section_debug`.

Each stage is a buildable fix-forward commit. A failed stage is localized by
its named tests; retain the last committed stage and repair forward. Run the
full gate after stage 3 (or earlier at four commits/cross-boundary failure), then
run live playable with the standard local Vanilla assets/server environment.

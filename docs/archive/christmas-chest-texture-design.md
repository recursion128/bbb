# Christmas Chest Texture Engineering Contract

Status: completed on 2026-07-30.

## Goal and completion

Close the active Vanilla 26.1 Christmas chest gap by snapshotting the local
calendar date at the native presentation boundary and selecting the resident
`christmas`, `christmas_left`, or `christmas_right` chest material from
December 24 through December 26 inclusive.

Completion requires ordinary and trapped world chests to use the Christmas
family during that interval, normal textures outside it, ender and every
copper chest family to remain unchanged, all three Christmas textures to
participate in renderer resource-pack residency, deterministic boundary and
precedence tests, removal of the ledger item, archival of this contract, a
concise milestone entry, and a passing full gate at the final head.

## Authoritative behavior

The authoritative local sources are:

- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/util/SpecialDates.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/blockentity/ChestRenderer.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/Sheets.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/special/ChestSpecialRenderer.java`;
  and
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/data/models/BlockModelGenerators.java`.

`SpecialDates.dayNow()` derives `MonthDay` from `ZonedDateTime.now()`, so the
system-local time zone owns the date. `isExtendedChristmas()` is true only on
December 24, 25, and 26. `ChestRenderer` snapshots that result in its
constructor. Its material precedence is copper state, ender, Christmas, then
trapped or normal; consequently Christmas replaces both normal and trapped
world-chest materials but never copper or ender materials.

`Sheets.chooseSprite` selects the single/left/right member of the Christmas
family. `ChestSpecialRenderer` names those resources `christmas`,
`christmas_left`, and `christmas_right`. Generated chest and trapped-chest item
models already own their independent local-time conditional item definition;
ender and copper item definitions do not select Christmas.

## Ownership and frozen interfaces

- `bbb-renderer::ChestModelTexture` gains `Christmas`. Its existing
  `chest_texture_ref` authority maps that family and `ChestModelHalf` to the
  three 64×64 Vanilla paths. `chest_entity_texture_refs` includes all three so
  startup and resource reload admit pack overrides without a date-dependent
  load path.
- `bbb-native::chest_scene` owns a small immutable Christmas-date value.
  `Default` snapshots the current system-local month/day once; a deterministic
  month/day constructor is available to module tests. Chest projection receives
  that value explicitly and centralizes the Vanilla material precedence.
- `NonEntityRenderExtractionScratch`, the render-thread-only block-entity
  extraction owner, stores the snapshot created with the renderer runtime and
  exposes it read-only to frame extraction. Neither `SimulationPublication`
  nor `bbb-world` gains wall-clock state.
- Existing world chest kind/half/openness/light ownership and the single
  `RendererFrame.entity_model_instances` submission path remain unchanged.

## Bounds and fallback rules

1. The inclusive interval is exactly `(12, 24)` through `(12, 26)`; every other
   month/day is ordinary.
2. Normal and trapped chest kinds both select `Christmas` inside the interval.
3. Ender and all four copper weathering kinds ignore the date.
4. The snapshot does not change at midnight during one process lifetime,
   matching Vanilla's constructor-cached flag.
5. Missing or invalid resource-pack Christmas images use the existing entity
   texture residency fallback; date selection does not add I/O or mutation per
   frame.

## Non-goals

- This slice does not add command-line or in-game date overrides.
- It does not change item-model local-time selection, special-item chest
  rendering, world serialization, chest block-state decoding, lid animation,
  lighting, geometry, or resource reload transaction ordering.
- It does not generalize wall-clock ownership beyond the bounded chest
  presentation value.

## Serial stages and verification

1. **Renderer material residency.** Add the typed Christmas material, three
   texture references, selection mapping, and semantic pass/residency tests.
   Run `scripts/cargo-dev.sh test -p bbb-renderer chest`.
2. **Native date and precedence projection.** Add the one-time local-date
   snapshot to the render extraction owner and pass it through chest scene
   extraction. Prove the December boundaries, normal/trapped substitution,
   ender/copper exclusion, and runtime binding with
   `scripts/cargo-dev.sh test -p bbb-native chest`.
3. **Closeout.** Re-run both focused families, remove the ledger item, archive
   this contract, append the milestone index, run the standard per-commit
   checks, review the integrated diff, and run the full gate.

The pre-start full gate passed at
`bc477d08914576aa56186f1f3e27d827ace7eee7`. The final full gate is the
engineering checkpoint. No playable gate is required because this slice
changes no packet, input-command, world-mutation, or control-command boundary.

The contract, renderer material residency, and native date/precedence stages
are the clean commits `1ca434190`, `11b9c1b60`, and `9d7f53bbf`.

## Failure localization and safe stops

Renderer failures compare typed material, half selection, texture path,
residency, and pass metadata before native code. Native failures compare
month/day classification and chest-kind precedence before frame integration.

The clean contract commit and renderer material commit are safe stops. Any
focused or gate failure is fixed forward before another slice; history is not
rewritten.

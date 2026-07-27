# Equipment Asset Layer Rendering Engineering Contract

## Goal and completion

Close the official resource-pack ledger item “Load equipment asset layers for
armor/equipment rendering” by making the selected resource generation's
`equipment/*.json` catalog the single texture/layer source for every equipment
geometry surface that bbb already renders.

The engineering effort is complete when:

- all 19 `EquipmentClientInfo.LayerType` values can resolve arbitrary
  namespaced equipment assets and texture paths from the selected pack stack;
- the effective `Equippable.asset_id` on each equipped stack, including an
  explicit component patch or removal, selects the renderer asset without
  falling back to a material/color table;
- humanoid armor, baby humanoid armor, wings, wolf/horse/nautilus body armor,
  llama decor, happy-ghast harnesses, and every supported saddle model consume
  the resource-defined layer list in declaration order;
- dyeable layers implement `EquipmentLayerRenderer.getColorForLayer`: an item
  dye wins, `color_when_undyed` is forced opaque, a dyeable layer with neither
  is skipped, and a non-dyeable layer is white;
- foil is submitted immediately after the first renderable base layer only,
  then trim follows all base/foil layers; `HUMANOID_BABY` suppresses trim while
  every other layer type retains it;
- `use_player_texture` selects a ready player elytra texture, then the visible
  cape texture, and otherwise uses the equipment layer texture;
- every resource-defined texture submission is recorded before atlas lookup,
  missing textures retain a deterministic missing-texture fallback, and reload
  never exposes a mixed old/new catalog or atlas;
- focused pack, world, native, renderer metadata, and pixel/mesh tests cover a
  custom namespaced asset with multiple layers, dye skipping/defaults, foil,
  trim, player override, and one representative consumer from each geometry
  family; and
- the corresponding todo is removed from
  `docs/unsupported-features.md`, with the engineering closure indexed in
  `docs/goal-archive.md`.

## Non-goals

- Adding new entity models or equipment geometry that bbb does not already
  render.
- Connecting synchronized custom block/entity registry identities to pack
  models.
- General non-profile dynamic texture loading, item-tint parity, atlas
  animation, or mip generation.
- Inventing `Equippable` defaults for custom item registries whose prototype
  components are not available. An explicitly decoded `Equippable` component
  may still reference a custom equipment asset.
- Changing armor-trim material/pattern registry codecs or trim-atlas
  generation beyond selecting the exact equipment asset and layer prefix.

## Ownership and frozen integration surface

### `bbb-pack`

`equipment_assets.rs` remains the only parser and selected-stack owner. It
normalizes asset ids and texture locations, preserves each JSON layer list's
order, and assigns a generation-local compact asset id from the sorted
`BTreeMap` asset order. The same catalog exposes reverse asset lookup and an
indexed iterator; native derives and deduplicates texture descriptors while
building the upload.

No renderer type or GPU handle enters `bbb-pack`.

### `bbb-render-types`

A small copyable equipment-asset id crosses native into
`EntityModelInstance`. It is a selected-generation index, not canonical
network state and not persistent across reloads.

### `bbb-world`

`WorldStore` remains canonical for equipped `ItemStackSummary` values and
registry-backed trim state. Item profiles receive the selected generation's
default item-to-equipment-asset ids plus asset-name lookup. Entity extraction
resolves the effective `Equippable` value once:

1. an explicit component value wins;
2. an explicit component removal yields no asset;
3. otherwise the item prototype default is used.

Humanoid armor additionally requires the component slot to match the armor
slot, as `HumanoidArmorLayer.shouldRender` does. `SimpleEquipmentLayer` and
`WingsLayer` require only a non-empty item with an asset and their existing
model/age gates. Trim sprite ids use the exact resolved asset resource id and
layer type.

World snapshots carry only compact asset ids, dye/foil/trim state, and the
existing semantic booleans. They never own texture bytes, atlas UVs, or
renderer catalogs.

### `bbb-item-model` and `bbb-native`

`NativeItemRuntime` derives item prototype asset ids and the asset-name table
from its already-owned `EquipmentAssetCatalog`. Native converts world asset ids
to renderer value types without reclassifying asset names.

`entity_assets.rs` converts the same prepared catalog into a renderer-owned CPU
equipment catalog, loads its unique PNGs as owned entity-atlas images, and
uploads both inside the existing transactional resource-reload barrier. Startup
and reload call the same preparation path.

### `bbb-renderer`

The renderer owns:

- the generation-local equipment asset/layer catalog;
- owned entity-atlas entries for resource-defined equipment textures; and
- texture-source metadata that distinguishes static, owned equipment, and
  existing dynamic player/trim textures before lookup.

The catalog is passed into textured entity mesh extraction beside the atlas
layout. Geometry helpers ask it for `(asset id, layer type)` and do not contain
asset/material/color-to-texture tables. Existing model trees, pose preparation,
render buckets, outline/light/overlay semantics, and submission sequence owners
remain unchanged.

`bbb-renderer` must not depend on `bbb-pack`.

## Vanilla 26.1 authority

- `EquipmentClientInfo`: JSON codec, 19 serialized layer types,
  `getTextureLocation`, dyeable state, and `use_player_texture`.
- `EquipmentAssetManager`: selected resource reload and asset lookup.
- `Equippable`: optional asset identity carried by the effective item
  component.
- `EquipmentLayerRenderer.renderLayers`: layer order, dye selection, first
  renderable foil, trim lookup/order, render types, light and no-overlay.
- `HumanoidArmorLayer`: chest/legs/feet/head order, slot validation,
  `HUMANOID_LEGGINGS`, baby layer selection, and armor-stand exception.
- `WingsLayer`: baby model selection, `Z + 0.125`, and player
  elytra/cape override.
- `SimpleEquipmentLayer`: non-empty asset gate, optional baby model, and
  constructor-supplied order.
- Existing concrete renderer registrations remain the authority for which
  layer type and geometry model each entity consumes.

## Serial stages and verification

1. **Catalog and atlas foundation**
   - Add stable asset ids/reverse lookup and unique texture descriptors.
   - Add renderer CPU asset/layer values and owned texture submission identity.
   - Upload equipment catalog plus images through startup and transactional
     reload.
   - Focused tests: `bbb-pack` equipment catalog/precedence, item-runtime id
     mapping, native entity-asset custom texture load, renderer owned-atlas
     metadata and missing fallback.
2. **Canonical/effective projection**
   - Add generic effective asset resolution to item profiles/entity extraction.
   - Project exact asset ids and asset-based trim ids for humanoid, wings, body,
     decor, harness, and saddle consumers.
   - Preserve current rendering until the corresponding consumer migrates.
   - Focused tests: `bbb-world` equipment projection plus `bbb-native`
     world-to-renderer conversion.
   - Run a full gate at this checkpoint because the resource-generation and
     world/render boundary has crossed a new state identity.
3. **Humanoid armor and wings**
   - Replace material/static texture selection with catalog layer iteration.
   - Cover adult/baby/armor-stand models, dye skip/default, first-visible foil,
     trim suppression/order, and player texture override.
   - Focused renderer mesh/submission tests plus native integration tests.
4. **Simple/body equipment closure**
   - Migrate every saddle, body armor, llama decor, and happy-ghast harness
     helper to the same catalog iteration.
   - Retain wolf crack overlays and built-in trader-llama fallback outside
     equipment asset rendering.
   - Delete obsolete static equipment texture/classification tables, update the
     ledger/archive, review the integrated diff, and run the final full gate.

Each stage is a normal, buildable commit. A failed stage is fixed forward.
Stages 2 and 4 are safe stopping points only after their declared full gate;
the final HEAD must be fully gated before another slice starts.

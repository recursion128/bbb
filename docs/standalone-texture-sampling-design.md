# Standalone Texture Sampling Engineering Contract

Status: active from 2026-07-30.

## Goal and completion

Close the active Vanilla 26.1 standalone mip/sampler gap for every pack-backed
image that the current renderer keeps as its own GPU texture. Replace
consumer-specific blur/clamp booleans and hard-coded sampler descriptors with
one renderer-facing value contract, preserve `.png.mcmeta` through the native
upload boundary, and give item and armor glint their distinct Vanilla texture
owners.

Completion requires:

- a shared pure `StandaloneTextureSampling` value to map `blur` to equal
  minification/magnification filters and `clamp` to equal U/V address modes;
- every current `SpriteImage`-loaded standalone GPU texture to consume its
  selected-pack `texture.blur` and `texture.clamp` metadata, including direct
  scene, entity-feature, misc-overlay, and legacy full-GUI images;
- item/entity glint to use `enchanted_glint_item.png` and armor glint to use
  `enchanted_glint_armor.png`, with their own resident texture/sampler bind
  groups in world and GUI entity-preview draws;
- standalone textures to remain one mip level, while the existing atlas mip
  generator and terrain upload path are reverse-audited and mechanically
  guarded as the complete Vanilla 26.1 mip implementation;
- focused pack/native/renderer semantic and deterministic GPU tests, removal
  of the standalone ledger item, archival of this contract, a concise
  milestone entry, and a passing full gate at the final committed head.

## Authoritative behavior

The authoritative local sources are:

- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/texture/ReloadableTexture.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/texture/TextureContents.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/resources/metadata/texture/TextureMetadataSection.java`;
- `/home/zgy/Work/mc-code/sources/26.1/com/mojang/blaze3d/systems/SamplerCache.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/texture/MipmapGenerator.java`;
- `/home/zgy/Work/mc-code/sources/26.1/com/mojang/blaze3d/platform/TextureUtil.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/util/ARGB.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/texture/TextureAtlas.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/resources/model/sprite/AtlasManager.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/rendertype/RenderTypes.java`; and
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/feature/ItemFeatureRenderer.java`.

`ReloadableTexture.apply` chooses `CLAMP_TO_EDGE` versus `REPEAT` from
`TextureContents.clamp`, chooses both minification and magnification as
`LINEAR` versus `NEAREST` from `TextureContents.blur`, and disables mipmaps.
`doLoad` allocates exactly one mip level. Missing metadata therefore means
repeat plus nearest/nearest, not the mixed nearest/linear default left on an
unloaded `AbstractTexture`.

Vanilla atlas sampling is a separate owner. Non-block atlases, including
celestials and GUI, use one mip level plus clamp-to-edge nearest sampling.
The block atlas uses its configured mip count. `MipmapGenerator` resolves
AUTO from transparency, applies non-item solidify/dark fill, uses the exact
1024-entry `ARGB` linear-light tables, preserves alpha coverage for cutout
strategies, and applies `alpha_cutoff_bias`.

`RenderTypes.armorEntityGlint` binds
`textures/misc/enchanted_glint_armor.png`; ordinary entity, item,
first-person, and GUI foil paths bind
`textures/misc/enchanted_glint_item.png`. Both Vanilla assets declare
`"blur": true`, but selected packs may independently override blur/clamp.

## Ownership and frozen interfaces

- `bbb-render-types` owns the dependency-free
  `StandaloneTextureSampling { blur, clamp }` value. It describes only
  one-level `ReloadableTexture` sampling; atlas mip policy remains with the
  atlas owners.
- `bbb-pack::SpriteTextureMetadata` remains the parser/source of truth for
  `.png.mcmeta`. `bbb-native` performs the explicit conversion at the
  renderer upload boundary; `bbb-renderer` does not depend on `bbb-pack`.
- `bbb-renderer` owns one sampler-descriptor mapping helper. Scene, entity
  feature, vignette, and HUD texture uploaders delegate to it rather than
  reconstructing blur/clamp policy.
- Direct scene image structs carry sampling when an upload is batched
  (`WeatherTextureImage`). Narrow upload methods accept the sampling value
  when one image is uploaded directly.
- Entity model pack images preserve sampling long enough to extract the two
  glint resources. The ordinary shared entity atlas retains its fixed
  one-level clamp/nearest state.
- Renderer residency owns separate item- and armor-glint GPU resources.
  Ordinary item paths continue to consume the item owner. Typed
  `EntityGlint` and `ArmorEntityGlint` world/PIP ranges bind the matching
  dedicated owner and no longer sample either glint image through the shared
  entity atlas.

## Bounds and fallback rules

1. Invalid or absent `.mcmeta` follows the existing pack metadata error and
   precedence rules; absent metadata yields repeat plus nearest/nearest.
2. A standalone image has exactly one mip level. Its sampler explicitly clamps
   LOD to level zero; `mipmap_strategy` and `alpha_cutoff_bias` are atlas-sprite
   inputs and do not generate standalone mips.
3. A missing item glint suppresses only item/entity-glint ranges that use that
   texture. A missing armor glint suppresses only armor-glint ranges. Neither
   falls back to the other image or to an atlas rectangle.
4. Resource reload remains transactional: previous item and armor glint owners
   are restored together with the other renderer resources after a failed
   reload.
5. GUI-atlas sprites and celestial-atlas sprites keep their Vanilla global
   clamp/nearest atlas state. Cloud pixels remain CPU mesh-generation input and
   are not classified as a sampled standalone GPU texture.
6. Pack `ClientAsset.ResourceTexture` entries currently coalesced into the
   dynamic entity atlas are tracked by the separate ledger item introduced
   with this contract; their per-entry sampling cannot be represented by this
   standalone owner without a distinct atlas-sampling design.

## Non-goals

- This engineering slice does not add per-entry filtering/addressing to a
  coalesced atlas, mipmap remote profile textures, add camera-overlay features,
  change texture paths, alter resource-pack precedence, or introduce a generic
  material graph.
- It does not change terrain atlas packing, sprite animation, render-target
  samplers, lightmap filtering, dynamic skin/cape decode, entity geometry,
  render-type ordering, world state, protocol, input, or audio.
- It does not add graphics configuration or an in-game configuration UI.

## Serial stages and verification

1. **Value boundary and mip audit.** Add the shared sampling value and the
   renderer descriptor mapping. Strengthen `bbb-pack` tests for exact linear
   lookup/alpha, strategy resolution, solidify/dark fill, coverage scaling,
   and standalone metadata precedence. Run
   `scripts/cargo-dev.sh test -p bbb-render-types texture_sampling`,
   `scripts/cargo-dev.sh test -p bbb-pack mip`, and
   `scripts/cargo-dev.sh test -p bbb-pack resource_texture`.
2. **Scene and HUD standalone owners.** Propagate metadata through direct End
   sky, weather, world-border, shadow, vignette, spyglass, and legacy full-GUI
   image uploads. Pin default repeat/nearest and custom clamp/linear mapping;
   retain the fixed sampler state of atlas-backed sprites. Run
   `scripts/cargo-dev.sh test -p bbb-native sky_assets`,
   `scripts/cargo-dev.sh test -p bbb-native hud_assets`, and
   `scripts/cargo-dev.sh test -p bbb-renderer standalone_texture`.
3. **Distinct glint owners.** Preserve glint metadata, create item and armor
   resident resources, route typed world and PIP glint ranges to the matching
   bind groups, and add deterministic pixels that distinguish the two images
   and linear from nearest sampling. Run
   `scripts/cargo-dev.sh test -p bbb-native entity_assets`,
   `scripts/cargo-dev.sh test -p bbb-renderer glint`, and
   `scripts/cargo-dev.sh test -p bbb-renderer entity_preview`.
4. **Closeout.** Re-run the mip/sampler reverse audit, remove the standalone
   ledger item, archive this contract, append the milestone index, run all
   per-commit checks, review the integrated diff, and run the full gate.

The pre-start full gate passed at
`6cef5ae05084c2cfd1ac76990ec72cd599ae61e5`. Because this changes central GPU
resource ownership and entity render dispatch, the final full gate is the
engineering checkpoint. No playable gate is required because protocol, world
mutation, input, audio, and control-command paths do not change.

## Failure localization and safe stops

Pack failures compare metadata precedence and byte-exact mip output before any
renderer code. Scene/HUD failures compare the pure sampling mapping and one
owner at a time. Glint failures compare item/armor residency and typed dispatch
before shader or pixel evidence.

The clean contract commit and each buildable implementation stage are safe
stops. Focused or gate failures are fixed forward before another slice;
history is not rewritten.

# Coalesced Resource-Texture Sampling Engineering Contract

Status: active from 2026-07-30.

## Goal and completion

Close the active Vanilla 26.1 sampling gap for pack
`ClientAsset.ResourceTexture` images that BBB coalesces into the existing
dynamic entity texture atlas. Preserve each resource texture's selected-pack
`texture.blur` and `texture.clamp` metadata through the pack/runtime/renderer
boundaries, apply the four independent sampling combinations in world and
entity-in-UI draws, and prevent one entry from sampling any neighbouring atlas
entry.

Completion requires:

- a typed renderer upload contract that distinguishes standalone-resource
  sampling from existing atlas-owned images;
- metadata propagation for both current pack-backed producers: synchronized
  registry entity variants and local resource-backed player body/cape/elytra
  patches;
- one coalesced dynamic entity atlas and the existing generation/session cache
  owner, with no per-resource GPU texture owner or parallel pack stack;
- exact level-zero nearest/linear and repeat/clamp sampling bounded to each
  participating entry;
- unchanged sampling and fallback behavior for downloaded profile textures,
  dynamic player skins, and pack atlas sprites coalesced by the same runtime;
- focused native/item-runtime/renderer semantic tests, deterministic GPU pixel
  evidence for filtering and entry isolation, removal of the active ledger
  item, an archive index entry, and a passing full gate at the final committed
  head.

## Authoritative behavior

The authoritative local sources are:

- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/core/ClientAsset.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/texture/TextureManager.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/texture/SimpleTexture.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/texture/ReloadableTexture.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/texture/DynamicTexture.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/entity/CowRenderer.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/entity/PigRenderer.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/entity/player/AvatarRenderer.java`;
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/entity/layers/CapeLayer.java`; and
- `/home/zgy/Work/mc-code/sources/26.1/net/minecraft/client/renderer/entity/layers/WingsLayer.java`.

`ClientAsset.ResourceTexture` maps its asset id to
`textures/<asset-path>.png`. Entity renderers submit that `texturePath`
directly. `TextureManager.getTexture` registers an absent path as a
`SimpleTexture`, whose `ReloadableTexture.apply` selects `CLAMP_TO_EDGE`
versus `REPEAT` from `clamp`, selects both minification and magnification as
`LINEAR` versus `NEAREST` from `blur`, disables mipmaps, and uploads one mip
level.

Downloaded profile images are a distinct Vanilla owner. `DynamicTexture`
creates one mip level with repeat/nearest sampling and does not consume pack
`.png.mcmeta`. Dynamic player skins and atlas sprites likewise do not acquire
`ResourceTexture` metadata merely because BBB currently coalesces their pixels
into the same runtime atlas.

## Ownership and frozen interfaces

- `bbb-pack` remains the parser and selected-pack owner of
  `SpriteTextureMetadata`. `ResourceTextureImage.texture_metadata` and
  `SpriteImage.texture_metadata` are the two existing metadata sources.
- `bbb-render-types::DynamicEntityTextureImage` adds
  `standalone_sampling: Option<StandaloneTextureSampling>`. `Some` means the
  pixels came from a pack standalone resource and require independent
  ReloadableTexture semantics. `None` retains the current atlas-owned
  treatment. The renderer does not depend on `bbb-pack`.
- `bbb-native::NativeEntityVariantTextureRuntime` maps
  `ResourceTextureImage.texture_metadata` to `Some(...)`.
  `bbb-item-model::load_local_dynamic_player_texture` does the same for local
  profile resource patches. Atlas-sprite uploads and downloaded profile
  decodes set `None`.
- `bbb-renderer` preserves this optional value in
  `EntityDynamicPlayerTextureAtlasEntry` together with the entry's integer
  pixel origin and size. Atlas packing remains the single CPU/GPU owner.
- `EntityModelTexturedVertex` gains a disabled-by-default resource-sampling
  payload. `render_textured_dynamic_player_texture_submission` fills the
  newly emitted vertex range from the resolved atlas entry only when its
  sampling value is `Some`; static atlas, dynamic-skin, and non-resource
  dynamic vertices retain the disabled value.
- The core textured and textured-cull shader families consume the payload.
  Their derived solid, cutout, z-offset, translucent, world, GUI, and HUD3D
  pipelines therefore share one implementation. Special dynamic atlas
  consumers whose current sources are atlas sprites keep the existing
  hardware atlas sampler.

No other public upload method, dynamic texture handle, render-type bucket, draw
range, generation key, or fallback API changes.

## Sampling and isolation rules

1. A disabled vertex payload continues to call the existing atlas
   `textureSample` path byte-for-byte.
2. An enabled payload carries local resource UV, integer atlas origin,
   integer resource size, and blur/clamp flags. Fragment interpolation applies
   only to local UV; origin, size, and flags are identical for every vertex of
   a submission.
3. Resource sampling is pinned to mip level zero. Nearest maps normalized UV
   to `floor(uv * size)`. Linear maps it to `uv * size - 0.5`, fetches the four
   surrounding texels, and bilinearly blends them.
4. Each integer texel coordinate is resolved against the resource entry before
   adding its atlas origin: clamp saturates to `[0, size - 1]`; repeat uses
   positive modulo. Every `textureLoad` is consequently inside that entry,
   including linear taps at all four edges. No padding or atlas-global address
   mode is relied upon.
5. The atlas remains one mip level. Filtering never reads a generated mip or
   an adjacent packed image. Existing checked dimension/byte limits and
   duplicate-handle rejection remain in force.
6. Missing, invalid, oversized, or failed resource textures keep the existing
   exact built-in presentation fallback and diagnostics. A failed atlas
   rebuild restores the previous image set and GPU owner transactionally.

## Non-goals

- This slice does not add another pack stack, cache, GPU texture per entry,
  texture array, material graph, or configurable graphics UI.
- It does not change pack precedence, resource/session invalidation, handle
  allocation, atlas column packing order, image limits, or renderer draw
  ordering.
- It does not reinterpret atlas sprite metadata, change the standalone
  texture owners completed by the preceding engineering slice, generate
  mipmaps, or alter terrain/item/GUI atlases.
- It does not change downloaded skin/cape/elytra decoding or broaden their
  sampling behavior beyond the existing path.
- It does not change protocol, registry identity, canonical world state,
  entity geometry, lighting, overlay, outline, animation, input, audio, or
  configuration.

## Serial stages and verification

1. **Typed upload boundary.** Add the optional sampling value, preserve
   ResourceTexture metadata in native registry variants and item-runtime local
   profile resources, and explicitly classify every existing image producer.
   Run `scripts/cargo-dev.sh test -p bbb-render-types player_skin`,
   `scripts/cargo-dev.sh test -p bbb-native entity_variant_textures`, and
   `scripts/cargo-dev.sh test -p bbb-item-model dynamic_player_texture`.
2. **Atlas and mesh contract.** Preserve integer entry geometry and sampling
   in atlas layouts, add the disabled-by-default vertex payload, fill only
   resource-backed dynamic submissions, and test multi-column layout,
   metadata identity, local-UV recovery, and unchanged non-resource vertices.
   Run `scripts/cargo-dev.sh test -p bbb-renderer dynamic_player_texture` and
   `scripts/cargo-dev.sh test -p bbb-renderer dynamic_texture`.
3. **Bounded shader sampling and closeout.** Implement the shared level-zero
   texel resolver in the textured shader families; add deterministic offscreen
   evidence distinguishing nearest from linear and proving that clamp/repeat
   edge taps cannot read a sentinel neighbour. Re-run focused world/PIP
   routing tests, remove the ledger item, archive this contract, append the
   milestone index, review the integrated diff, and run the full gate.

The pre-start full gate passed at
`7921aacb679ffd8333f071b55d1ae3a5e41cf766`. This changes a central entity
vertex ABI and shader sampling path, so the final committed head is the
engineering checkpoint and must pass `scripts/cargo-dev.sh gate`. No playable
gate is required because protocol, world mutation, input, audio, and
control-command behavior do not change.

## Failure localization and safe stops

Stage-one failures stay at metadata/source classification. Stage-two failures
stay in pure atlas/geometry layout before WGSL changes. Stage-three structural
tests distinguish payload routing from coordinate resolution; deterministic
GPU pixels distinguish filter selection from cross-entry isolation.

The clean contract commit and each buildable stage commit are safe stops.
Focused or full-gate failures are fixed forward before another slice; history
is not rewritten.

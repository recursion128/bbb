# Player-Head Block Profile Skin Closure Design

Status: active engineering contract.

## Goal and completion standard

Close the player-head block-entity `profile` presentation gap as one P3
vertical slice. The existing selected resource generation, profile resolver,
bounded dynamic player-texture cache, renderer dynamic-skin atlas, skull block
model, and BlockEntityData replacement path remain the only owners. This slice
adds the missing connection:

- decode the skull block entity's optional `profile` with the same
  `ResolvableProfile.CODEC` semantics already used by item data components;
- retain that profile in the canonical block-entity record and expose it only
  to the player-skull source at the same absolute block position;
- use the existing asynchronous profile-resolution and skin download
  generations for dynamic name- or UUID-only profiles;
- render a resolved or patched body texture through the existing dynamic
  player-skin atlas and preserve exact vanilla fallback behavior while profile
  or texture resolution is pending or failed;
- replace or remove an old owner atomically on BlockEntityData updates and
  clear it with the existing chunk/level lifecycle;
- remove the player-head block-entity TODO, archive this contract, and pass the
  full gate plus applicable playable/P0 checks.

Completion evidence must cover initial chunk data, BlockEntityData replacement
and omission, malformed optional-profile folding, string/name-only and stored
profile Codec forms, profileless versus profiled fallback, asynchronous
profile-to-skin resolution, dynamic ready/failed states, and native skull
projection at the exact block position.

## Scope and non-goals

Owned boundaries are:

- `bbb-world`: shared `ResolvableProfile` NBT decoding, canonical skull
  block-entity profile state, update/lifecycle semantics, and immutable skull
  render-source projection;
- `bbb-item-model`: reusable profile default selection plus the existing
  profile-resolution and dynamic-texture owners;
- `bbb-native`: world-to-renderer player-skull skin selection and the existing
  dynamic download/upload drain;
- existing `bbb-renderer` skull-block dynamic skin routing and focused
  regression evidence;
- unsupported-feature and milestone closeout documentation.

This slice does not add non-profile remote textures, custom synchronized
block/entity identities, new resource-pack stacks or caches, player-head item
behavior, custom skull names, note-block sounds, capes or elytra on a skull,
an in-game configuration UI, or a new renderer pipeline. Player entities and
player-head items continue to share the same profile owner without acquiring a
second resolution path.

Block-entity packets with a mismatched type or position retain their existing
rejection behavior. A syntactically valid block entity whose optional
`profile` value fails its Codec is accepted with no owner, matching vanilla's
`ValueInput.read(...).orElse(null)`. A malformed NBT root remains a packet
decode error under the existing boundary.

## Authoritative vanilla 26.1 facts

Local sources:

- `SkullBlockEntity.saveAdditional` stores nullable field `profile` through
  `ResolvableProfile.CODEC`; `loadAdditional` reads that Codec and folds an
  absent or invalid optional value to `null`.
- `SkullBlockEntity.getUpdateTag` returns the custom fields, so a later
  BlockEntityData payload is a replacement snapshot rather than a patch over
  the previous owner.
- `ResolvableProfile.CODEC` accepts either the full record or a plain validated
  player name. The record combines stored `GameProfile`/partial
  `name`-`id`-`properties` data with flattened `PlayerSkin.Patch` fields
  `texture`, `cape`, `elytra`, and `model`.
- A partial profile with empty properties and exactly one of `name` or `id` is
  dynamic and resolves through `ProfileResolver`; all other decoded forms are
  static. Name-only profiles derive their temporary UUID through the vanilla
  offline-player UUID algorithm.
- `SkullBlockRenderer.resolveSkullRenderType` uses
  `PlayerSkinRenderCache.getOrDefault(owner)` only for a player skull with an
  owner. A player skull without an owner uses
  `DefaultPlayerSkin.getDefaultTexture()` through the ordinary
  `entityCutoutZOffset` skull path.
- `PlayerSkinRenderCache` starts resolution and returns a profile-specific
  default immediately. That default is selected from the 18 vanilla skins by
  Java UUID hash; resolved player skins use
  `SkullBlockRenderer.getPlayerSkinRenderType`, which is
  `entityTranslucent(texture)`.
- `PlayerSkin.Patch.with` overrides the resolved/default body, cape, elytra,
  and model independently. The skull consumes only the body texture and player
  model while retaining the same patch-aware profile result.

## Frozen ownership and integration contract

`bbb-world` owns one shared decoder returning the existing
`ResolvableProfileSummary`. Both dialog/data-component decoding and skull
block-entity decoding call it; neither keeps a divergent Codec
implementation. `BlockEntityRecord` gains
`skull_profile: Option<ResolvableProfileSummary>`. Only the vanilla skull
block-entity type decodes this field. Decode failure for that field becomes
`None`, while raw NBT/root failures continue through the existing error path.

Every initial chunk record and accepted BlockEntityData update is constructed
as a complete replacement. Omitting `profile` therefore removes a previous
owner. Chunk unload/replacement and level generation replacement need no new
cache owner. `WorldStore::skull_model_source_states` joins the record at the
source's exact absolute position and clones the optional profile only for
`SkullBlockKind::Player`; other skull kinds cannot consume a stray profile.

`NativeItemRuntime::player_skin_for_profile` first asks the existing bounded
profile-resolution runtime for the current resolved-or-fallback summary, then
selects/queues the body texture and other already-owned profile textures from
that same summary. Full/static profiles are unchanged. Dynamic partial
profiles produce at most the existing deduplicated resolution request, then
the existing deduplicated skin URL request after resolution.

Native skull extraction receives an optional `NativeItemRuntime`:

1. non-player skulls retain their fixed model kind;
2. a player skull without an owner is
   `EntityPlayerSkin::Default(SlimSteve)`, correcting the current hard-coded
   wide-Steve mismatch and preserving vanilla's static default
   texture and cutout-z-offset renderer branch;
3. a player skull with an owner uses
   `NativeItemRuntime::player_skin_for_profile` when available;
4. if the runtime is unavailable, or resolution/download is pending or failed,
   it uses `EntityPlayerSkin::ProfiledDefault` selected from the owner's
   UUID/offline-name hash, preserving the profiled translucent branch;
5. a ready remote or selected-pack body patch uses the existing
   `EntityPlayerSkin::Dynamic` atlas route. No stale handle is invented or
   retained outside the resource/session generation owners.

The renderer's established distinction between `Default` and
`ProfiledDefault`/`Dynamic` remains authoritative; this slice does not add a
parallel skull texture surface.

## Serial stages, focused tests, and checkpoints

1. **Canonical Codec and world state.** Factor the shared profile decoder, add
   `BlockEntityRecord.skull_profile`, decode initial/update records with
   optional-field folding, and project the exact player-skull owner. Focused
   commands:
   `scripts/cargo-dev.sh test -p bbb-world profile` and
   `scripts/cargo-dev.sh test -p bbb-world skull`.
2. **Resolution and native projection.** Make the generic
   `player_skin_for_profile` resolve dynamic partial profiles, export the exact
   profile-default helper, and select the owner skin during native skull
   projection. Keep the existing renderer dynamic atlas route pinned by its
   focused tests. Focused commands:
   `scripts/cargo-dev.sh test -p bbb-item-model player_skin_for_profile`,
   `scripts/cargo-dev.sh test -p bbb-native skull`, and
   `scripts/cargo-dev.sh test -p bbb-renderer custom_head_skull`.
3. **Closeout.** Delete only the player-head block-entity profile TODO, preserve
   the broader non-profile/custom-registry TODOs, archive this contract, and
   append the milestone index. Run docs lint, formatting, all affected focused
   tests, and diff checks.

Every stage is a buildable normal commit. The contract plus three stages reach
the four-commit engineering checkpoint, so the final tip runs
`scripts/cargo-dev.sh gate`, the available playable modes, and
`scripts/audit-p0.sh`.

Failures are fixed forward. After stage one the canonical owner is observable
but the existing profileless default renderer behavior remains safe. After
stage two, lack of HTTP/profile runtime or failed downloads degrades to the
owner-specific vanilla default without suppressing the skull or reusing stale
texture state.

## Hardest-ready decision

The existing dynamic profile/skin/cape/elytra owner satisfies the restart
condition that previously deferred this terrain edge, and the active P3
roadmap now names player-head block-entity profiles as its first concrete
consumer. OpenScreen/container polish and remaining client game-event
presentation are still in the ledger's `Deferred` section. Equal-load
official-client performance comparison remains blocked by the unavailable
Java executable, while the broader physics and custom-registry inventories do
not yet have one frozen semantic target. This is therefore the hardest current
ready engineering slice: it crosses Codec reuse, canonical block-entity
replacement, resource/session-scoped asynchronous resolution, native render
extraction, and dynamic GPU texture fallback without creating a second owner.

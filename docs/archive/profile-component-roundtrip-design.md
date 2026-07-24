# Resolvable profile ItemStack component round-trip

## Goal and completion contract

Close all six ItemStack component coverage dimensions for Vanilla 26.1
`PROFILE` (type 70).

Completion means its catalog descriptor owns the exact
`ResolvableProfile.STREAM_CODEC` encoder: the `Either<GameProfile, Partial>`
branch, UUID/name optionals, ordered property multimap, and
`PlayerSkin.Patch` all round-trip without loss. Encoding and decoding enforce
Vanilla's Java UTF-16 limits and 16-property budget. Canonical summaries must
carry `profile_textures` derived from the retained property values and
`ResourceTexture.texture_path` derived from a normalized asset identifier;
missing full-profile fields, stale derived fields, invalid identifiers, and
oversized values fail before patch bytes are appended.

Creative ItemStacks must encode the component through the existing per-value
delimiter. Existing persistent SNBT, dialog publication, dynamic-profile
tooltip, profile resolution, resource-patch, and player-head skin consumers
must provide named evidence before the ledger row becomes `complete`.

## Non-goals

- This slice does not encode component types 5 through 57, 61 through 69, or
  71 through 109, and does not mark
  `SERVERBOUND_SET_CREATIVE_MODE_SLOT` complete.
- It does not fetch, verify, or mutate Mojang profile properties while
  encoding. Property order and signatures are opaque retained wire values.
- It does not change profile resolver, HTTP, disk cache, dynamic texture, or
  renderer policy.
- It does not accept noncanonical shorthand or stale derived texture paths on
  the network summary boundary; persistent dialog shorthand remains decoded
  by its existing owner before publication.

## Ownership and integration boundary

- `bbb-protocol::packets::data_components::catalog` owns profile network
  encoding, exact branch/optional order, bounds, and descriptor registration.
- `bbb-protocol::packets::data_components` owns strict profile decoding,
  derived-field canonicalization, patch preflight, and delimited framing.
- `bbb-protocol::packets::profile_textures` remains the only property-to-texture
  projection owner.
- `bbb-world` retains persistent dialog profile decoding and canonical patch
  publication.
- `bbb-native` retains persistent SNBT projection. `bbb-item-model` retains
  dynamic profile resolution, skin-patch application, tooltip, and resource
  texture consumers.
- Sparse coverage overrides remain the only hand-maintained ledger input.

## Vanilla 26.1 evidence

- `DataComponents.PROFILE` is persistent with `ResolvableProfile.CODEC`,
  network-synchronized with `ResolvableProfile.STREAM_CODEC`, and cache
  encoded.
- `ResolvableProfile.STREAM_CODEC` writes
  `ByteBufCodecs.either(GAME_PROFILE, Partial.STREAM_CODEC)` followed by
  `PlayerSkin.Patch.STREAM_CODEC`. `ByteBufCodecs.either` writes `true` for
  the full `GameProfile` branch and `false` for `Partial`.
- `GAME_PROFILE` writes UUID, 16-Java-character player name, then at most 16
  properties. Each property writes a 64-character name, 32767-character
  value, and optional 1024-character signature.
- `Partial.STREAM_CODEC` writes optional name, optional UUID, then the same
  ordered property collection.
- `PlayerSkin.Patch.STREAM_CODEC` writes optional body, cape, elytra resource
  textures and optional model. A resource texture sends only its identifier;
  its texture path is derived as `textures/<path>.png`. Model is a boolean
  (`true` slim, `false` wide).
- The persistent Codec supports the full record plus player-name shorthand;
  shorthand decoding stays outside the network encoder.
- `DataComponentPatch.DELIMITED_STREAM_CODEC` wraps the complete value codec
  in an independent VarInt byte length.

Authoritative sources are under `~/Work/mc-code/sources/26.1/net/minecraft/`,
notably `core/component/DataComponents.java`,
`world/item/component/ResolvableProfile.java`,
`world/entity/player/PlayerSkin.java`,
`world/entity/player/PlayerModelType.java`, `core/ClientAsset.java`,
`network/Utf8String.java`, `network/codec/ByteBufCodecs.java`, and
`core/component/DataComponentPatch.java`.

## Serial stages and validation

1. Tighten profile-only network string decoding to Java UTF-16 limits and add
   canonical derived-field reconstruction. Focused protocol tests cover both
   Either branches, exact-limit astral names, over-limit rejection, property
   limits, texture path derivation, and malformed canonical summaries. Safe
   stop: no encode callback or ledger claim yet.
2. Add the descriptor encoder and canonical patch preflight. Focused tests
   prove exact descriptor bytes, property/signature order, skin patch
   optionals, per-value framing, atomic rejection, and creative packet
   integration. Safe stop: type 70 remains the only newly encodable value.
3. Audit existing persistent/dialog/tooltip/profile-skin evidence, close the
   six ledger cells, narrow the creative TODO, and archive this contract. Run
   affected-crate focused tests, generator checks, and every per-commit check.

This is protocol/profile high risk: commit the complete slice, then run
`scripts/cargo-dev.sh gate`. Any failure is fixed forward before the next
slice.

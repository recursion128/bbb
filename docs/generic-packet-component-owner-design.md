# Generic packet Component RegistryAccess closure

## Goal and completion contract

Close the remaining world/session-owner gap for Vanilla 26.1 Components that
are embedded directly in clientbound packet codecs. A registry-aware packet
Component must cross three explicit stages:

1. `bbb-protocol` decodes the bounded network NBT, validates the
   registry-independent Component envelope, and retains its exact NBT together
   with plain/styled projections.
2. `bbb-world` validates every owner-sensitive player-profile, dialog-holder,
   and show-item component-patch payload against the current frozen session
   registry before any canonical mutation or packet-derived side effect.
3. Canonical state retains the exact rich value wherever the packet field is
   itself observable; presentation code may derive text/runs but must not
   reconstruct the Component from a lossy string.

Completion means every direct registry-aware carrier listed below has an
atomic world preflight. A malformed or unresolved payload rejects the complete
packet update, records a bounded apply diagnostic, leaves prior canonical state
unchanged, and does not poison a later valid update. Direct-holder and
server-reordered registry tests prove that validation uses the current session
rather than a built-in ordinal.

The companion standalone `Style` codecs in direct `ChatType` decorations and
styled scoreboard number formats are included because their click/hover fields
can contain the same owner-sensitive Component payloads. Their exact style NBT
must survive protocol decode until world preflight.

## Frozen inventory

The registry-aware direct carriers are:

- boss-bar add/update names, action-bar/title/subtitle text, system chat,
  combat-kill messages, tab-list header/footer, and open-screen titles;
- objective display names, score display names, team display/prefix/suffix,
  fixed number-format Components, and styled number-format Style;
- disguised-chat content, player-chat unsigned content, bound chat sender and
  target names, direct chat-type decoration Style, and command-suggestion
  tooltips;
- advancement title/description, player-info display names, map-decoration
  names, and test-instance status;
- entity metadata Component/optional-Component values and inline painting
  variant title/author Components.

`RichTextComponent` already owns the exact value for most of these fields.
Existing chat and command-suggestion projections already retain an adjacent
exact NBT value. Protocol work is limited to the remaining lossy
player-info/map/painting values and the two standalone Style carriers; it does
not replace already-correct packet representations.

The authoritative Vanilla sources are:

- `ComponentSerialization.STREAM_CODEC`, `TRUSTED_STREAM_CODEC`, and
  `TRUSTED_OPTIONAL_STREAM_CODEC`;
- `Style.Serializer.TRUSTED_STREAM_CODEC`;
- the `Clientbound*Packet` classes named by the carrier list;
- `ChatType.Bound.STREAM_CODEC`, `FixedFormat.STREAM_CODEC`,
  `DisplayInfo.read`, `MapDecoration.STREAM_CODEC`,
  `PaintingVariant.STREAM_CODEC`, and
  `EntityDataSerializers.COMPONENT` / `OPTIONAL_COMPONENT`; and
- `ClientPacketListener` handlers plus `SynchedEntityData.assignValues`.

All source paths are below
`~/Work/mc-code/sources/26.1/net/minecraft/`.

## Non-goals

- Do not inject session registries into
  `TRUSTED_CONTEXT_FREE_STREAM_CODEC` packets such as disconnect,
  resource-pack prompt, server-data MOTD, or server links. The protocol
  Component canonicalizer continues to return an explicit `None` when an
  owner-sensitive value cannot be encoded without a higher owner; this slice
  pins that behavior instead of guessing from raw NBT size.
- Do not reopen Component values owned inside `DataComponentType`, Dialog,
  command arguments, or general dynamic-registry entry codecs. Those keep
  their existing semantic owners and coverage ledgers.
- Do not change packet ids, field order, Component depth/count limits,
  chat-signature policy, scoreboard semantics, map pixels, screen lifecycle,
  renderer interaction policy, or resource-pack behavior.
- Do not add a mutable registry reference to protocol values, a global registry,
  a side table keyed by Component identity, or a second packet dispatcher.
- Do not turn semantic rejection into a connection disconnect. The current
  world boundary fails closed with diagnostics, matching existing entity
  metadata preflight behavior.

## Ownership and frozen integration surface

`bbb-protocol::RichTextComponent` remains the exact Component value.
Player-info display names, map-decoration names, and inline painting
title/author fields migrate from `Option<String>` to
`Option<RichTextComponent>`.

`bbb-protocol` adds one exact standalone-style value that contains a
`ComponentStyle` projection plus its validated source NBT. Direct chat-type
decorations and `NumberFormatSummary::Styled` use it. The projected style stays
available through a narrow accessor; callers cannot supply mismatched NBT and
projection through deserialization.

`bbb-world::client` keeps the sole higher-owner validator, reusing the complete
Dialog/player-profile/data-component-patch decoders. A small world helper
records `packet_component` diagnostics and is called by each packet-family
apply owner before mutation. Packet-specific methods keep their existing
public ownership; `WorldStore::apply_play_packet` remains only a dispatcher.

Player-info and map canonical state retain `RichTextComponent` rather than
only its text projection. Existing renderer/control callers derive text or
styled runs from that value. Entity metadata keeps its existing atomic
preflight and extends it to inline painting title/author fields. Chat raw NBT
fields and command-suggestion tooltip NBT stay in their current compatible
shapes.

## Serial stages and validation

1. **Protocol exactness.** Add the exact Style wrapper and migrate the three
   lossy carrier families. Focused protocol tests cover wire order, styled
   projection, exact NBT retention, serde mismatch rejection, and a
   context-free canonical `None` for owner-sensitive payloads. Run
   `bbb-protocol` component/player-info/map/entity/chat/scoreboard tests,
   formatting, diff-check, and a full gate at this protocol milestone.
2. **World preflight.** Add the shared diagnostic helper, preflight every
   frozen carrier before mutation, retain rich player-info/map state, and
   extend metadata painting validation. Focused world tests cover each carrier
   family, multi-entry atomic rejection, custom/reordered dialog resolution,
   recovery, retained old publication isolation, and exact state.
3. **Consumer integration.** Update native/control projections to consume the
   rich canonical values and add focused integration tests proving styled
   player-list/map output still derives from world state. Existing online and
   probe dispatch paths remain unchanged.
4. **Closeout.** Remove the exact ledger todo, archive this contract, append a
   concise milestone index entry, run docs lint, formatting, focused tests,
   diff-check, and full diff review, then commit and run the final full gate.

Each stage is buildable and reviewable. Protocol decode failures localize to
stage 1; mutation/registry failures localize to packet-family world tests in
stage 2; presentation regressions localize to stage 3. A failed stage is fixed
forward. Safe stops are the clean protocol-milestone gate and the final gated
tip; no unrelated slice starts between them.

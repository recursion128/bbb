# In-world game-rules screen closure

## Goal and completion contract

Close the remaining applicable coverage cells for
`ClientboundGameRuleValuesPacket` and `ServerboundSetGameRulePacket`, and replace
the unconditional game-rule diagnostic with the canonical lifecycle of an active
in-world game-rules screen.

Completion requires one world-owned screen state that:

- opens only for the local player with gamemaster permission and queues
  `REQUEST_GAMERULE_VALUES` exactly once;
- accepts only the first `GameRuleValues` response while that screen is active;
- resolves normalized keys through the generated 59-entry built-in game-rule
  catalog, typed-deserializes boolean and bounded integer values, ignores
  unknown/invalid entries, and leaves loading state after the first response;
- preserves separate initial and current typed values, validates integer edits,
  filters and presents the server-provided rules, and disables Done while an
  integer edit is invalid;
- sends one `ServerboundSetGameRulePacket` containing only changed valid rules
  when Done is selected, then closes the screen;
- discards unchanged screens directly, confirms Escape/Cancel when changes are
  pending, and closes without submission when discard is confirmed;
- closes on local gamemaster-permission loss, level replacement, disconnect, or
  replacement by another screen; and
- keeps the raw last-packet diagnostic and packet counter independent of whether
  a screen accepted the response.

The final coverage ledger has no partial cell for either packet. The in-world
GameRules todo is removed in the same closeout commit, focused semantic tests
cover every lifecycle edge above, and the final HEAD passes the full gate,
applicable playable gate, and P0 audit.

## Non-goals

- Do not implement an `IntegratedServer`, authoritative server game-rule state,
  or optimistic gameplay effects from submitted values. The server remains the
  authority and its existing system-chat response is the acknowledgement path.
- Do not build the general Options/World Options/configuration UI. The existing
  `net.request_game_rule_values` control action becomes the explicit opening
  surface for this bounded screen until the unrelated Options navigation surface
  exists. This is a server gameplay-management screen, not startup
  configuration.
- Do not accept custom game-rule keys. Vanilla resolves the packet against
  `BuiltInRegistries.GAME_RULE` and ignores unknown keys.
- Do not port the Java screen class hierarchy, narrator framework, or every
  widget styling detail. Preserve the observable loading, search, category/list,
  boolean/integer editing, tooltip, Done/Cancel, validation, scroll, hover, and
  discard-confirmation behavior through repo-native HUD primitives.
- Do not change the already complete `ClientboundGameRuleValuesPacket` wire
  decoder except for tests needed by the typed screen consumer.
- Do not expand this slice to the client-command packet's unrelated respawn or
  stats response gaps.

## Authoritative vanilla facts

- `GameProtocols.java` registers
  `SERVERBOUND_SET_GAME_RULE` with
  `ServerboundSetGameRulePacket.STREAM_CODEC` and
  `CLIENTBOUND_GAME_RULE_VALUES` with
  `ClientboundGameRuleValuesPacket.STREAM_CODEC`.
- `ServerboundSetGameRulePacket.java` encodes a VarInt-sized list. Each entry is
  a `ResourceKey<Registries.GAME_RULE>` resource location followed by the
  default `STRING_UTF8` value.
- `ClientPacketListener.handleGameRuleValues` forwards a response only when the
  active screen is `InWorldGameRulesScreen`.
- `InWorldGameRulesScreen.initContent` installs the loading widget before
  sending `REQUEST_GAMERULE_VALUES`. `onGameRuleValuesUpdated` is guarded by
  `receivedServerValues`, resolves every key through
  `BuiltInRegistries.GAME_RULE`, typed-deserializes it, removes loading, and
  builds the list from only the server-provided rules.
- `InWorldGameRulesScreen.onDone` compares initial/current typed values and sends
  no set packet for an empty change set. Escape/Cancel asks for discard
  confirmation only when a value changed.
- `AbstractGameRulesScreen` uses exact `true`/`false` boolean values and
  Brigadier integer parsing with the generated per-rule bounds. Invalid integer
  text is retained in the edit box, colored invalid, and disables Done without
  replacing the last valid typed value.
- The list groups rules by `GameRuleCategory` identifier and sorts rules by
  description id. Filtering checks the rule id, localized name, category, and
  optional localized description case-insensitively.
- `ServerGamePacketListenerImpl.sendGameRuleValues` and `handleSetGameRule`
  require gamemaster permission. The handler ignores unknown keys and invalid
  serialized values; valid changes remain server-authoritative and are
  broadcast to operators through system chat.
- `InWorldGameRulesScreen.onGamemasterPermissionChanged(false)` leaves the
  screen immediately.

Authoritative sources are under
`/home/zgy/Work/mc-code/sources/26.1/net/minecraft/`: `network/protocol/game/`
`GameProtocols.java`, `ClientboundGameRuleValuesPacket.java`, and
`ServerboundSetGameRulePacket.java`; `client/multiplayer/ClientPacketListener.java`;
`client/gui/screens/options/InWorldGameRulesScreen.java` and
`WorldOptionsScreen.java`;
`client/gui/screens/worldselection/AbstractGameRulesScreen.java`;
`server/network/ServerGamePacketListenerImpl.java`; and
`world/level/gamerules/{GameRules,GameRule,GameRuleCategory}.java`.

## Ownership and frozen integration surface

### Protocol and generated catalog

- `scripts/gameplay_id_catalog_extractor.py` extends `GameRuleEntry` with the
  exact `GameRuleCategory` identifier. The generator remains the only complete
  built-in game-rule table owner.
- `bbb_protocol::game_rules` exposes category metadata plus one typed
  `GameRuleValue` parser/serializer on `GameRuleCatalogEntry`. Boolean parsing is
  exact lowercase `true`/`false`; integer parsing accepts only an optional
  leading `-` followed by decimal digits, consumes the whole string, rejects
  overflow, and applies the generated inclusive bounds.
- `bbb_protocol::packets` adds `SetGameRule` and `SetGameRuleEntry`, and
  `encode_play_set_game_rule(&SetGameRule) -> Result<(i32, Vec<u8>)`. The
  repo-native UI emits at most the 59 built-in entries; encoder validation
  rejects more entries, invalid/unknown resource ids, invalid typed values,
  duplicates, and strings beyond Vanilla's 32,767 UTF-16-unit bound before
  encoding.

### Net transport

- `bbb_net::NetCommand` adds `SetGameRule(SetGameRule)`.
- `bbb-net` adds one `send_set_game_rule` command path in Play. Non-Play command
  drains ignore it consistently with other Play-only commands.
- `bbb-control::NetCounters` adds `set_game_rule_commands_queued`. No canonical
  rule values move into net counters.

### Canonical world/UI state

- `crates/bbb-world/src/client/game_rules.rs` owns
  `GameRulesScreenState`, `GameRuleScreenEntryState`, the typed initial/current
  values, packet application, screen lifecycle, edits, pending-change queries,
  and change collection.
- `ClientUiState` owns
  `current_game_rules_screen: Option<GameRulesScreenState>` and a monotonically
  increasing local screen generation. `WorldStore` exposes narrow query/edit
  methods; callers do not mutate the state directly.
- `WorldStore::apply_game_rule_values` continues to update the raw
  `last_game_rule_values` diagnostic and counter for every packet, then offers
  the packet to the active loading screen. Duplicate keys use the final wire
  value, matching map decode semantics. Accepted rules are materialized in
  generated catalog order for deterministic publications and submissions;
  entry order has no server semantic.
- The screen closes from level-bound cleanup and when a local permission entity
  event leaves the gamemaster levels. Other incoming screens use the existing
  replacement policy and do not retain stale game-rule edits.

### Native input, control, and rendering

- `crates/bbb-native/src/input/game_rules.rs` owns native-only search text,
  cursor/selection, scroll, hover/focus, invalid integer drafts, and discard
  confirmation. It implements open, key/text/mouse/wheel handling, world-screen
  generation synchronization, and Done/Cancel submission.
- `net.request_game_rule_values` calls that open operation. A failed permission
  or conflicting-screen guard queues nothing. A successful open first releases
  gameplay input, creates the world screen, and then queues exactly one request.
- `queue_set_game_rule_command` sends the change packet returned by the
  world-owned Done transition and increments the new net counter only on a
  successful queue write.
- `crates/bbb-native/src/runtime/game_rules_screen.rs` localizes and projects the
  active screen into one `HudGameRulesScreen`. It reads the same immutable world
  generation as other HUD surfaces and never owns canonical rule values.
- `crates/bbb-renderer/src/hud/game_rules.rs` owns the game-rules HUD data,
  sanitization, and draw geometry. `hud.rs` remains a facade/integration point.
  The screen participates in the existing single-screen suppression and blur
  ordering before ordinary HUD overlays.
- `world.client_ui` exposes the canonical screen state through the existing
  paired world/control snapshot. No second control-owned snapshot is added.

## Serial stages and verification

1. Extend the generated catalog schema, add typed parsing/serialization, add the
   serverbound packet encoder, net command, queue counter, and transport.
   Focused checks:
   `scripts/vanilla-source-tools.sh python -m unittest
   scripts.tests.test_gameplay_id_catalog_extractor`,
   `scripts/vanilla-source-tools.sh scripts/generate-gameplay-id-catalog.py
   --check`,
   `scripts/cargo-dev.sh test -p bbb-protocol game_rule`, and
   `scripts/cargo-dev.sh test -p bbb-net set_game_rule`.
2. Add world-owned screen state and lifecycle. Focused tests cover permission
   and screen guards, first-response-only apply, final duplicate value,
   unknown/invalid filtering, integer bounds, changed-entry collection,
   discard/apply close, permission loss, and level reset. Run
   `scripts/cargo-dev.sh test -p bbb-world game_rules_screen`.
3. Add control opening, native input and renderer HUD surface. Focused tests
   cover one request per successful open, queue failure counters, loading/list
   projection, localization/filtering/category order, boolean and integer edit
   flows, invalid Done, scroll/hover/tooltips, changed-only submit, Escape
   confirmation, and screen suppression. Run
   `scripts/cargo-dev.sh test -p bbb-native game_rules_screen`,
   `scripts/cargo-dev.sh test -p bbb-renderer game_rules_screen`, and
   `scripts/cargo-dev.sh test -p bbb-control request_game_rule_values`.
4. Remove the completed todo, mark both packet rows complete with exact
   evidence, regenerate the packet ledger, append the milestone index, archive
   this contract, and run the final full gate, `scripts/playable-gate.sh live`,
   `scripts/playable-gate.sh auto`, and `scripts/audit-p0.sh`.

The fully gated `11a5c510b` is the pre-start checkpoint. Each stage ends in a
clean, buildable commit. Run a checkpoint full gate after stage 2 because the
wire and canonical lifecycle then form a complete non-rendering boundary; run
the final full gate after closeout. Failures are fixed forward without rewriting
history. The explicit live gate is currently expected to report the external
Java/server/EULA/display prerequisites rather than expand this slice into
machine provisioning; the policy `auto` fixture must still pass.

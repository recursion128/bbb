# Scoreboard client presentation closure

Status: active engineering contract.

## Goal and completion

Close the vanilla 26.1 client-visible path for the scoreboard packet family:
`ClientboundResetScorePacket`, `ClientboundSetDisplayObjectivePacket`,
`ClientboundSetObjectivePacket`, `ClientboundSetScorePacket`, and
`ClientboundSetPlayerTeamPacket`. Completion requires:

- objective and team packets accept every signed method byte with vanilla's
  conditional payload shape, while known methods retain generated enum
  semantics and unknown methods become handler no-ops;
- canonical objective, score, display-slot, team-membership, rich-component,
  render-type, and number-format state follows `ClientPacketListener` and
  `Scoreboard`, including missing-owner/objective behavior;
- the right-side sidebar implements team-color-slot precedence, hidden-owner
  filtering, score/name ordering, 15-entry truncation, rich team formatting,
  score-specific/objective/default number formats, and vanilla layout;
- the multiplayer player list consumes the `list` objective, preserves rich
  team prefix/name/suffix runs, renders integer and heart objectives, and owns
  vanilla's UUID-keyed delayed health/blink lifecycle;
- packet coverage, the feature ledger, focused tests, the full workspace gate,
  the applicable playable smoke, and the P0 audit all close at the final tip.

## Authority and non-goals

Authoritative sources are vanilla 26.1
`ClientboundResetScorePacket`, `ClientboundSetDisplayObjectivePacket`,
`ClientboundSetObjectivePacket`, `ClientboundSetScorePacket`,
`ClientboundSetPlayerTeamPacket`, `ClientPacketListener`, `Scoreboard`,
`PlayerScoreEntry`, `DisplaySlot`, `Gui`, and `PlayerTabOverlay`.

This slice does not add server scoreboard commands or criteria evaluation.
Player heads and skin security, rich TabList header/footer preservation,
narrator visibility announcements, and sprite-backed ping icons are independent
player-list presentation work and remain open. The integrated-server
single-player suppression branch is not applicable to bbb's remote native
client. Configuration remains command-line only.

## Frozen semantics and ownership

- `bbb-protocol` owns the raw signed objective/team method byte. Existing
  generated `SetObjectiveMethod` and `PlayerTeamMethod` catalogs remain the
  only known-method tables; packet accessors resolve a raw byte to an optional
  known enum. Objective parameters exist only for methods 0/2, and team
  parameters/player lists only for methods 0/2 and 0/3/4 respectively.
- `bbb-world::client::scoreboard` owns canonical state and query projections.
  Unknown methods increment the existing packet counters but do not mutate
  state. Sidebar entries filter owner names beginning with `#`, sort value
  descending then owner case-insensitively ascending, and take 15. A
  `sidebar.team.<color>` objective for the local player's team overrides
  `sidebar`; non-color formatting values have no team slot.
- Team-formatted names compose rich prefix, base name/display component, and
  suffix while applying the team color only where the child component has not
  selected a color. Score-specific formats override the objective format.
  Sidebar's absent default is red styled decimal; player-list's absent default
  is yellow styled decimal. Blank produces no score width, styled formats the
  decimal with inherited style, and fixed ignores the numeric value.
- `bbb-native` localizes canonical components and projects one immutable
  scoreboard sidebar plus optional player-list score per frame. The list
  projection carries profile UUID and the simulation publication's client tick;
  it does not own animation history.
- `bbb-renderer` owns font measurement, sidebar/list layout, UUID-keyed
  player-list health state, and heart geometry. Health state clears whenever
  list visibility changes and prunes absent UUIDs. Score changes blink for 20
  ticks on decrease and 10 on increase; the displayed value catches up after
  20 ticks. Existing heart assets are extended with the exact blinking variants
  used by `PlayerTabOverlay`.
- The world-to-renderer path extends `RendererFrame`; no protocol type crosses
  into `bbb-renderer`, and no renderer history crosses into serializable world
  state.

## Serial stages and verification

1. Preserve raw objective/team method bytes, apply known/unknown methods with
   vanilla behavior, and add canonical rich name/score/sidebar/list query
   tests. Focused checks: `bbb-protocol scoreboard`, `bbb-world scoreboard`,
   `bbb-net scoreboard`, and `bbb-control scoreboard`. This is the protocol
   milestone and receives a full gate before later presentation work.
2. Add canonical-to-native sidebar extraction and renderer-owned right-side
   sidebar layout, including team-color precedence, ordering, truncation,
   component styles, and number formats. Focused checks: `bbb-world
   scoreboard`, `bbb-native hud_scoreboard`, and `bbb-renderer
   hud_scoreboard`.
3. Add player-list rich team names, integer score layout, heart state/rendering,
   localized compact HP text, and blinking heart asset upload. Focused checks:
   `bbb-native hud_player_list`, `bbb-renderer hud_player_list`, and
   `bbb-native hud_assets`.
4. Update packet coverage and the TODO ledger, review the integrated diff, then
   run the full gate, playable smoke, and P0 audit.

Each stage is a buildable normal commit after focused tests, fmt, diff check,
docs lint, and review. Run a full gate after stage 1, at any four-commit
checkpoint, and at the final tip. A failed stage stops safely at the preceding
clean commit; failures are fixed forward without rewriting history.

# PauseScreen closure

Status: completed at `5a77766a2`.

Pre-start fully gated baseline:
`f5b29cb7a7f66dd87f20c231566136c5438928d9`.

## Goal and completion contract

Close the remaining applicable Vanilla 26.1 `PauseScreen` behavior without
introducing an in-game configuration surface. Completion requires:

- both the ordinary menu and F3+Escape no-menu screen have an explicit native
  tick-freeze eligibility contract;
- a remote native client continues raw client ticks and running level ticks
  while either pause screen is visible;
- the ordinary menu chooses the same custom dialog addition as Vanilla from
  `minecraft:pause_screen_additions`, `minecraft:custom_options`, and
  non-empty server links;
- a chosen custom addition replaces the direct feedback buttons with
  `Feedback...` and the dialog's external title, and opens the existing
  world-owned dialog stack while retaining the pause screen as its parent;
- the feedback sub-screen has the Vanilla title, two link buttons, Back button,
  layout, Escape/back return, and menu background;
- feedback and bug-report links use the existing startup link-policy,
  confirmation, open/copy/cancel, and platform-effect path instead of logging
  or opening an unconfirmed URI;
- existing advancements, stats, player reporting, disconnect, F3 passthrough,
  cursor ownership, dialog restoration, and screen suppression do not regress;
  and
- focused tests, the unsupported-feature ledger, archive index, final full
  gate, and P0 audit close at the final tip.

## Authority and non-goals

The authoritative sources are local Vanilla 26.1 `PauseScreen`,
`ConfirmLinkScreen`, `Screen.clickUrlAction`, `Minecraft.pauseGame`,
`Minecraft.runTick`, `HeaderAndFooterLayout`, `GridLayout`, `Dialogs`, and
`DialogTags`.

`bbb` owns no `IntegratedServer`. Vanilla freezes only when an unpublished
integrated server exists and the active screen or overlay is pausing, so both
native pause variants are permanently ineligible to freeze ticks. This slice
does not add an integrated server, LAN publishing, singleplayer disconnect
wording, or sound-pause behavior.

Repository configuration remains command-line-only. The existing disabled
Options row stays visible as an explicit unavailable action; no `OptionsScreen`
or in-game configuration UI is added. A Share to LAN row is inapplicable
without an integrated server, so the existing Player Reporting row remains.
Draft-report decoration is inapplicable until a report-draft composition owner
exists. Music-toast presentation, general screen narration, and
`DebugOptionsScreen` polish remain separate work.

## Frozen Vanilla behavior

- `Minecraft.runTick` sets the pause flag only when
  `hasSingleplayerServer()`, the current screen or overlay pauses, and the
  integrated server is not published. `pauseGame` still opens
  `PauseScreen(true)` for a remote client, but that screen does not freeze the
  client or level tick loop.
- `PauseScreen.getCustomAdditions` reads the dialog tag. One tagged holder is
  selected directly; two or more select `minecraft:custom_options`; an absent
  or empty tag falls back to `minecraft:server_links` only when server links
  are non-empty. A missing or invalid selected registry holder produces no
  custom addition.
- With no custom addition, the third row is Give Feedback / Report Bugs. With
  one, the row is Feedback... / the chosen dialog's computed external title,
  and the custom button carries `menu.custom_options.tooltip`.
- `FeedbackSubScreen` uses a 33-pixel header and footer. Its title is centered
  at y=12, the 98-pixel link buttons are at
  `y = min(67, height - 53)`, and the 200-pixel Back button is centered at
  `y = height - 27`. Escape and Back restore the same main pause screen.
- Pause links are trusted `ConfirmLinkScreen` actions: Open in Browser, Copy to
  Clipboard, and Cancel all return to their parent. The existing native
  non-dialog link path additionally preserves the startup links-enabled and
  prompt-enabled policy.

## Frozen ownership and integration surface

### World and registry ownership

- `bbb-world::client::ui` adds one owned
  `PauseScreenCustomDialogState`. It contains the selected, fully decoded
  `DialogState` plus the computed external title; native callers cannot mutate
  its registry identity or payload.
- `WorldStore::pause_screen_custom_dialog` performs the exact tag/cardinality/
  server-links selection against one frozen registry access and returns an
  owned snapshot. Unresolved tags, absent built-ins, stubs without synchronized
  data, and semantic decode failures fail closed to no custom addition.
- `WorldStore::open_pause_screen_custom_dialog` opens that frozen target through
  the existing dialog lifecycle. Dialog input, warning, waiting, nested link,
  and platform-effect behavior stay single-sourced in the dialog owner.
- Existing `WorldStore::open_chat_url` remains the only PauseScreen link-policy
  and confirmation entry point. No PauseScreen-specific URI or clipboard
  state is added to world.

### Native input and runtime

- `ClientInputState` snapshots the world-selected custom addition when the main
  pause menu is created, matching `PauseScreen.init`; later server-link updates
  do not rewrite an already open menu. The selection is cleared on final close
  and rebuilt when returning from Player Reporting through a newly created
  pause menu.
- Native pause state adds a two-value Main/Feedback page. Escape closes Main
  but returns Feedback to Main. The no-menu variant never enters Feedback.
- Main hit testing uses either the direct link pair or Feedback/custom pair
  from the frozen selection. Custom activation opens the owned dialog; the
  existing native dialog-parent synchronization suspends the pause state and
  restores the exact page, cursor, custom selection, and disconnect state when
  the dialog stack closes.
- Link requests call `open_chat_url`, collect immediate URI effects when prompt
  is disabled, or let the existing dialog confirmation surface suspend and
  restore the pause page when prompt is enabled.
- A named `native_pause_freezes_client_ticks` eligibility function is consulted
  at the client-tick boundary and returns false for this external-only runtime.
  Focused proof covers menu/no-menu states and advancing tick-owned state while
  each is visible.

### Renderer

- `HudPauseScreen` distinguishes no-menu, main, and feedback views and carries
  the chosen external-title label plus custom tooltip projection. Renderer
  geometry remains the only button-layout owner; native hit-test constants
  are mechanically locked to the same Vanilla rectangles.
- Main preserves the existing title and rows. Feedback always draws the menu
  background, its header/content/footer layout, and only its three buttons.
  The custom-options tooltip reuses the existing tooltip sprites and rich
  wrapping path.
- Dialog/link-confirmation projection keeps higher priority than PauseScreen
  through the existing single-screen suppression and native previous-screen
  stack. No parallel dialog HUD is added.

## Serial stages and verification

1. Add world selection/open APIs and semantic tests for empty/one/many tags,
   server-link fallback, missing/invalid holders, external-title selection, and
   opening the selected dialog. Run
   `scripts/cargo-dev.sh test -p bbb-world pause_screen`.
2. Add native Main/Feedback state, frozen custom selection, dialog and link
   routing, explicit no-freeze eligibility, renderer projection/geometry,
   tooltip, and restoration tests. Run
   `scripts/cargo-dev.sh test -p bbb-native pause_screen`,
   `scripts/cargo-dev.sh test -p bbb-native dialog_clear_restores_the_native_pause`,
   and `scripts/cargo-dev.sh test -p bbb-renderer pause_screen`.
3. Remove the completed ledger item, archive this contract, append the
   engineering milestone index, review the integrated diff, then run the full
   gate and P0 audit.

Every stage is a buildable normal commit after its focused tests,
`cargo fmt --all --check`, `git diff --check`, documentation lint when
applicable, and full diff review. The contract plus three stages reaches the
four-commit engineering checkpoint, so the final tip runs
`scripts/cargo-dev.sh gate`. A failure stops at the preceding clean commit and
is fixed forward without rewriting history.

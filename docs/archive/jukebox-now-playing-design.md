# Jukebox now-playing presentation engineering contract (archived)

## Goal and completion

Close the HUD ledger item for the Vanilla jukebox “Now Playing” overlay by
making accepted level event 1010 resolve the exact current-session
`JukeboxSong`, start its positioned audio, publish the animated 60-tick overlay,
and enqueue immediate system narration from one shared rich component.

The engineering effort is complete when:

- the generated Vanilla 26.1 jukebox-song catalog has one shared owner in
  `bbb-protocol`, while `bbb-pack` and `bbb-world` consume that catalog instead
  of maintaining parallel resource/sound/order tables;
- level event 1010 follows the synchronized session holder order whenever that
  registry exists, including custom ordering, custom sounds, and exact rich
  descriptions;
- clean/offline worlds use the generated Vanilla bootstrap order; synchronized
  data-less built-in stubs resolve only by their resource identity, while an
  unknown holder or unresolved custom entry produces no start, overlay, or
  narration side effect;
- an accepted start stops the sound already tracked at that block position,
  starts the selected song at the block center, sets the action bar for 60
  ticks with animated color, and enqueues `SystemNow` narration using the same
  `record.nowPlaying` component;
- level event 1011 retains the existing stop behavior without creating a new
  overlay or narration request;
- focused protocol, pack, world, and native tests cover catalog identity,
  clean fallback, synchronized custom order and rich description, unknown
  holders, replacement, stop, localized action-bar projection, animation, and
  narration; and
- the corresponding todo is removed from
  `docs/unsupported-features.md`, the contract is archived, and the engineering
  closure is indexed in `docs/goal-archive.md`.

## Non-goals

- Resource-pack sound decoding, streaming, attenuation, or duration changes.
- Jukebox block-entity inventory prediction, note particles, comparator output,
  or nearby-entity dancing behavior.
- A general overlay-message queue or changes to title/action-bar packet
  precedence.
- New in-game configuration UI or narrator settings.
- Falling back to Vanilla holder ordinals after a server has synchronized a
  jukebox-song registry for the session. A recognized data-less built-in stub
  may use its resource identity, never its synchronized ordinal.

## Ownership and frozen integration surface

### `bbb-protocol`

`jukebox_song_types` owns the generated Vanilla 26.1 descriptor table and
lookup helpers by holder id, song resource id, and sound event id. A descriptor
contains the generated bootstrap ordinal, Java constant name, song resource id,
and sound event resource id. The description translation key is derived from
the song resource id with Vanilla `Util.makeDescriptionId`; it is not another
generated table.

### `bbb-pack`

`JukeboxSongRegistry` remains the pack/runtime sound lookup and serialized
registry owner. Its clean fallback and compatibility query methods delegate to
`bbb_protocol::jukebox_song_types`; the old private generated module is
removed.

### `bbb-world`

The existing typed synchronized `JukeboxSongPresentation` remains canonical for
server-supplied entries. A single level-event resolver returns one complete
presentation:

1. if the session has a synchronized `minecraft:jukebox_song` registry, resolve
   only through that holder order and typed entry; a data-less built-in stub
   may delegate to the shared catalog by its exact resource id;
2. otherwise resolve the holder through the shared Vanilla catalog and derive
   the built-in translatable description component.

It never substitutes a fixed ordinal for a missing or malformed synchronized
entry. The accepted event uses one exact
`Component.translatable("record.nowPlaying", song.description())` value for
both `ActionBarState` and `ChatNarrationRequest`. World owns the 60-tick timer,
the `animate_color = true` bit, and `SystemNow` queue semantics. Existing
protocol action-bar packets continue to set `animate_color = false`.

### `bbb-native`

Native remains the sink for the already-structured jukebox sound transition
and the consumer of action-bar/narration state. It localizes the shared rich
component through the existing component projection, proves animated rendering
and immediate system narration in integration tests, and does not reconstruct
song descriptions or translation keys.

## Vanilla 26.1 authority

- `JukeboxSongs.create`, `register`, and `bootstrap`: registry order and
  `Util.makeDescriptionId("jukebox_song", registryKey.identifier())`.
- `LevelEventHandler.levelEvent` cases 1010/1011 and `playJukeboxSong`: holder
  lookup, replacement order, block-center sound, overlay call, and stop
  behavior.
- `Gui.setNowPlaying` and `setOverlayMessage`: the
  `record.nowPlaying` wrapper, `SystemNow` narration, 60 ticks, and animated
  overlay color.
- `JukeboxSong.DIRECT_CODEC`: exact sound holder and rich description supplied
  by synchronized custom entries.

All references are read from the local
`~/Work/mc-code/sources/26.1/` source tree.

## Serial stages and verification

1. **Shared generated catalog**
   - Move generator output to `bbb-protocol::jukebox_song_types`, expose the
     descriptor and lookups, and migrate `bbb-pack` to delegation without
     changing registry behavior.
   - Focused tests:
     `scripts/vanilla-source-tools.sh python -m unittest scripts.tests.test_jukebox_song_catalog_extractor`,
     generator `--check`, and jukebox-filtered tests in `bbb-protocol` and
     `bbb-pack`.
   - Run the full gate after the clean stage commit because generator ownership
     is high risk.
2. **World event semantics**
   - Add the no-session fallback projection, exact now-playing component,
     animated action-bar transition, component-aware `SystemNow` enqueue, and
     strict accepted/unknown event behavior.
   - Focused tests: jukebox registry and level-event tests in `bbb-world`.
3. **Native integration and closeout**
   - Prove sound replacement/stop plus localized animated overlay and narration
     through the native event sink, remove the ledger item, archive this
     contract, and index the engineering closure.
   - Focused tests: jukebox/level-event/action-bar/narrator filters in
     `bbb-native`, plus affected world tests.
   - Review the integrated diff and run the final full gate.

Each stage is a normal, buildable commit. The fully gated `303d8defe` is the
pre-start checkpoint. Stage 1 is a safe stopping point only after its declared
full gate; the final HEAD must be fully gated before another slice starts.
Failures are fixed forward without rewriting history.

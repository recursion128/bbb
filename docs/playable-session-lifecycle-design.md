# Playable Session Lifecycle Engineering Slice

> Canonical scope：当前门禁和生命周期分类见
> [`docs/wiki/development/testing-and-gates.md`](wiki/development/testing-and-gates.md)与
> [`docs/wiki/architecture/client-boundaries.md`](wiki/architecture/client-boundaries.md)。
> 本页只保留 playable lifecycle 的专门 evidence。

This document freezes the high-risk engineering contract for extending the
repository-native playable gate from one initial login and interaction smoke to
the Vanilla 26.1 session lifecycle. The gate remains an integration test of the
shipping `bbb-native` binary against the unmodified official dedicated-server
jar; it is not a substitute for complete packet, entity, or long-session
coverage.

## Semantic goal and completion criteria

One explicit `scripts/playable-gate.sh live` invocation must drive and assert:

- the existing status, login, configuration, Play, movement, and block
  interaction closure;
- death followed by a real server-authoritative respawn request and a usable
  replacement local player;
- Overworld to Nether and Nether to Overworld respawns, with old chunks and
  level-bound runtime state absent from the replacement level;
- an accepted server resource-pack push, applied reload, matching pop, and
  restoration of the previous resource stack;
- Play to configuration re-entry and return to Play, preserving only the state
  Vanilla carries across `ClientPacketListener` replacement;
- a server-directed transfer that reconnects the same client process, performs
  a second login/configuration/Play sequence, commits a fresh registry session,
  and exposes usable second-session chunks and local-player state; and
- no unsupported required packet, canonical apply error, stale old-level state,
  stale server-pack runtime, or pending control request left across a boundary.

The live artifact must retain ordered lifecycle observations, before/after
canonical snapshots, renderer/audio counters where relevant, client/server
logs, and actionable failure text. A failure must identify its last completed
boundary instead of presenting the initial stable snapshot as the failure-time
state.

## Non-goals

- This slice does not add visible gameplay, protocol variants, an in-game
  configuration UI, or an independently mutable world/session owner.
- It does not use a fake protocol peer to claim that a Vanilla live lifecycle
  passed. Narrow unit fixtures remain useful for failure localization only.
- It does not make the debug commands part of `bbb` production behavior. The
  gate enables commands already present in the official Vanilla 26.1 jar.
- It does not claim all resource-pack consumers, all Play packets, all entity
  behavior, or long-running gameplay complete.
- It does not weaken effect-before-publication ordering, immutable generation
  pairing, renderer/control sharing, shutdown/join behavior, or stale-result
  rejection.

## Owners and integration boundary

- `scripts/playable-gate.sh` owns the official server processes, current-HEAD
  client process, resource-pack HTTP fixture, artifact directory, timeouts, and
  cleanup.
- `scripts/playable_gate.py` owns control driving, lifecycle polling, evidence
  schema, phase ordering, and semantic assertions.
- `bbb-net` remains the sole transport/state-machine owner, including
  Play-to-configuration acknowledgement and Transfer reconnect.
- `bbb-world` remains the sole mutable canonical world owner under simulation;
  registry session revisions, level replacement, pack state, and counters are
  read through the immutable publication used by both control and rendering.
- `bbb-native` remains the only runtime side-effect owner for renderer/audio
  resets and server-pack reload. The gate observes these paths but must not add
  a second test-only implementation.
- `bbb-control` remains a derived read/write boundary. Writes enter the existing
  simulation mailbox; reads use one published generation and world `Arc`.

## Vanilla 26.1 evidence

The contract was checked against local sources under
`/home/zgy/Work/mc-code/sources/26.1/`:

- `ClientPacketListener.handleConfigurationStart` flushes pending chat,
  acknowledges configuration, stores carried chat/session state, clears the
  client level, and changes both protocol directions to Configuration.
- `ClientPacketListener.handleRespawn` replaces the level only when the
  dimension changes, rebuilds the local player according to the keep-data bits,
  closes the active container, clears the death screen, and stops music only
  for a dimension change.
- `DebugConfigCommand` calls `ServerGamePacketListenerImpl.switchToConfig`.
  `ServerResourcePackConfigurationTask` emits the production resource-pack
  push from dedicated-server properties and wraps its nullable prompt with
  `Optional.ofNullable`. The 26.1 debug `ServerPackCommand.pushPack` instead
  passes null to an `Optional` codec and disconnects in the official packet
  encoder, so the gate uses the production push and only the debug pop path.
- `Commands` registers those two commands when
  `SharedConstants.DEBUG_DEV_COMMANDS` is true. `SharedConstants.debugFlag`
  derives it from the official JVM properties `MC_DEBUG_ENABLED` and
  `MC_DEBUG_DEV_COMMANDS`; the gate can therefore exercise the unmodified
  official jar with `-DMC_DEBUG_ENABLED -DMC_DEBUG_DEV_COMMANDS`.
- `TransferCommand` is a normal dedicated-server command and emits
  `ClientboundTransferPacket`. The second connection must use the Transfer
  handshake intent rather than a fresh externally launched client.
- `DedicatedServerProperties` names the target opt-in `accepts-transfers` and
  defaults it to false. `ServerHandshakePacketListenerImpl` rejects a Transfer
  intent unless that exact target property is enabled.
- `PlayerList.placeNewPlayer` calls `sendServerStatus` only when
  `CommonListenerCookie.transferred()` is false. A transferred login therefore
  carries the first server's server-list presentation rather than receiving a
  target MOTD/icon packet; target identity must be proven from the Transfer
  host/port and the second official server's login log, not from a fabricated
  `ServerData` expectation.
- `ClientPacketListener`, `ClientLevel`, `LocalPlayer`,
  `ServerConfigurationPacketListenerImpl`, and `PlayerList.reloadResources`
  define the state retained or replaced at each boundary. A normal `/reload`
  only broadcasts updated tags/recipes and is not evidence of configuration
  re-entry.

## Baseline

At gated commit `0ab5ba41865f`, the explicit live gate passed against Vanilla
26.1 with artifact `/tmp/bbb-playable-gate.UCeKs1`; `auto` selected the live
path and passed with artifact `/tmp/bbb-playable-gate.BDd0Ou`. The evidence
contains the eight initial ordered phases, canonical Play state, movement, and
destroy/place interaction. It does not drive or assert any lifecycle boundary
listed above.

The fixed environment is:

- Java: `/home/zgy/Work/mc-code/downloads/java/jdk-25.0.3+9-jre/bin/java`;
- server: `/home/zgy/Work/mc-code/downloads/26.1/server-26.1.jar`;
- code root: `/home/zgy/Work/mc-code`;
- compositor socket: `/run/user/1000/bbb-gate`, backed by a real persistent
  Weston process; and
- binary: `/tmp/bbb-target-main/debug/bbb-native`, rebuilt from the exact
  checkpoint `HEAD` before each live milestone.

## Frozen live contract

- The harness writes a level-4 offline operator entry for the deterministic
  `bbb-playablegate` profile. Lifecycle commands travel through
  `net.send_chat_command`, proving the shipping client-to-server command path.
- The headless client sends `input.focus {"focused": true}` before every
  synthesized keyboard, mouse, or look sequence.
- Death is server authoritative (`kill @s`); the client waits for canonical
  dead health before `net.perform_respawn`, then waits for a respawn packet and
  stable replacement pose.
- Dimension changes are server authoritative `execute in <dimension> run tp`
  commands. Each direction must increment the respawn counter, replace the
  dimension resource id, decode destination chunks, and leave the connection
  in Play.
- The server-pack fixture is a valid minimal zip served by a loopback HTTP
  server. The client runs with explicit `accept` policy and a per-artifact cache.
  A push is not complete until runtime reports applied/active and the expected
  serverbound responses are observed; a pop is not complete until the pack is
  absent and the base resource revision is active again.
- Reconfiguration uses `debugconfig config @s`, waits for Configuration and
  cleared level state, then uses the server-side `debugconfig unconfig <uuid>`
  path and waits for the next Play login/chunk/local-player closure. The harness
  may issue `unconfig` through a persistent server command channel because the
  player is no longer in Play and cannot send commands at that moment.
- Transfer targets a second concurrently running official server. It must occur
  in the same `bbb-native` process. Registry `revision.session` must increase,
  the second server must produce a new login/configuration/Play closure, and
  level-bound first-session state must not survive. The target sets the official
  `accepts-transfers=true` property. Evidence retains the canonical transfer
  host/port, target-server join log, fresh chunk, cleared death location and
  resource-pack set, and Vanilla's deliberately carried server-list
  presentation.
- Cumulative counters may increase but may not regress. Assertions use deltas
  plus current canonical state; they do not assume every counter resets.
- On any failure, the harness writes the most recent stage snapshot, lifecycle
  observations, logs, and server directories before process-group cleanup.

## Serial phases, tests, and checkpoints

1. **Contract and observation baseline.** Land this contract and the evidence
   schema/harness primitives without changing client semantics. Focused tests:
   `scripts/vanilla-source-tools.sh python -m unittest discover -s scripts/tests`,
   shell syntax checks, fixture assertion, docs lint, formatting, and diff check.
   The previously gated clean boundary is the launch checkpoint.
2. **Death, respawn, dimension, pack, and reconfiguration.** Add one-server
   driving and semantic evidence. Focused tests include the script suite plus
   affected `bbb-control`, `bbb-net`, `bbb-world`, and `bbb-native` lifecycle
   filters. Rebuild the current `HEAD`, run the workspace full gate, then the
   explicit live gate.
3. **Transfer and second session.** Add the second official server, same-process
   Transfer reconnect, fresh registry-session assertion, and cleanup/leak
   evidence. Repeat focused tests, current-HEAD build, full gate, and explicit
   live gate.
4. **Closeout.** Update the P1 queue, goal archive, unsupported ledger, and
   architecture documentation with the exact delivered coverage and residual
   risks. The final `HEAD` must pass `scripts/cargo-dev.sh gate`, an explicit
   `scripts/playable-gate.sh live`, and `scripts/playable-gate.sh auto` with the
   fixed environment. No milestone may use an older binary.

All commits are normal, reviewable commits. No amend, rebase, history rewrite,
or unrelated feature work is allowed.

### Phase 2 compatibility findings

The first full live runs exposed two production-format assumptions that fixture
tests could not exercise:

- Vanilla 26.1 `ListTag` encodes heterogeneous elements as compound tags and
  wraps non-compound values under an empty-name marker. The shared protocol NBT
  summary codec now mirrors `ListTag.wrapIfNeeded`/`addAndUnwrap`, including the
  double wrapper needed to preserve a literal one-entry empty-name compound.
  Data-component decoding uses that single codec rather than a divergent copy.
- A model layer's declared texture width and height are logical UV extents, not
  validation constraints on a resource-pack PNG. For example, the ghast layer
  is 64x32 while the Vanilla 26.1 image is 128x64. Entity assets now retain the
  physical pack resolution separately, and the static GPU atlas packs full
  images into device-height-bounded columns without resampling. Geometry keeps
  using the logical extent, preserving Vanilla normalized UV behavior.

The control-driven movement assertion remains entirely synthetic. A
server-authoritative `execute align xz` teleport is used only to prepare the
player for the subsequent synthetic pick/destroy/place sequence; this removes
renderer-frame timing from centering without weakening any movement or block
interaction assertion.

### Phase 3 transfer result

Commit `d890952a6` passed the workspace full gate and a current-HEAD explicit
live run with artifact `/tmp/bbb-playable-gate.GXeoYO`. One native/control
process observed one Transfer packet and a third Play login; registry revision
advanced from session 1 to 2, the first-server chunk at `(-100,-100)` was absent,
the target decoded a fresh chunk, the prior death location and resource-pack set
were empty, and the target official server logged the transferred player. The
final publication remained connected in Overworld Play with zero unsupported
packets and zero canonical apply errors.

The first development run, `/tmp/bbb-playable-gate.1ppmEq`, already completed
the Transfer but timed out on an incorrect target-MOTD expectation. Local
`PlayerList.placeNewPlayer` evidence showed that Vanilla intentionally skips
`ServerData` for a transferred cookie. The final assertion now preserves that
carry rule and uses the canonical target plus the second official server log as
the target identity proof.

## Failure localization and safe stops

The lifecycle evidence records a monotonic current stage plus the observations
from every completed boundary. The top-level `world_snapshot` intentionally
remains the opening stable-period sample on a failure; it must not be read as
failure-time state. Server command rejection, missing packet transition,
canonical apply failure, runtime reload failure, transfer connection failure,
control timeout, client exit, and cleanup leak are distinct failures. ANSI is
stripped from `client.log` before searching; server-authoritative disagreement
is checked against the gzip NBT player data under each artifact server world.

A phase may stop only at its clean committed checkpoint after reporting an
external environment blocker, or when an official command/packet cannot produce
the frozen semantics. A test failure is fixed forward in a normal commit. The
slice does not proceed to another feature while a lifecycle phase is partially
landed.

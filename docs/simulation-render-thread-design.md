# Simulation / Render Thread Split

> Canonical scope：当前 runtime 状态边界见
> [`docs/wiki/architecture/client-boundaries.md`](wiki/architecture/client-boundaries.md)。
> 本页只保留 thread/queue/shutdown 工程的详细 evidence。

This document freezes the ownership and migration contract for moving native
client simulation off the winit/render thread. It is the prerequisite design
for the `goal.md` simulation/render decoupling engineering slice.

Implementation status: all six phases are complete. The named simulation
thread, deadline/producer wakeups, immutable publication, ordered effect
transport, measured render backlog, and joined failure/shutdown path are
active.

The migration did not make the former `pump_network_and_terrain` function
`Send`. It separated packet application, input, client ticks, backend effects,
render extraction, terrain uploads, and GPU state before introducing the
thread boundary; the old pump and render-thread running-snapshot adapter no
longer exist.

## Semantic Goal

One named simulation thread owns the live mutable `WorldStore`, applies
ordered network and input events, and advances the client at its fixed-rate
tick cadence. The winit thread owns the window and renderer and renders the
latest immutable simulation publication at frame cadence. A slow terrain mesh,
asset upload, screenshot, or GPU frame must not delay network application or a
due client tick.

Final completion requires all of the following:

- The live mutable `WorldStore`, simulation `ClientInputState`, net-event
  receiver, tick clock, and world-facing runtime state have exactly one owner:
  the simulation thread.
- Winit callbacks and control requests submit typed commands to the one input
  path. They never mutate the live world or run a second copy of gameplay input
  behavior.
- Packet application and client ticks preserve their verified ordering and
  emit ordered backend effects without borrowing a renderer, audio backend, or
  platform object.
- The render thread reads immutable published state. It owns the window,
  renderer, terrain upload/cache state, particle renderer, asset upload state,
  and platform adapters.
- Control reads reuse the published world generation and remain lock-free;
  control writes join the simulation command path.
- Shutdown closes producers, asks the simulation thread to stop, joins it, and
  then drops backend resources. No detached task retains session state.
- Focused concurrency/order tests, the full gate, and the applicable playable
  gate pass at the delivered `HEAD`.

## Non-Goals

- No protocol or canonical gameplay behavior change is part of this slice.
- No second prediction model, render-only world, or control-owned canonical
  state is introduced.
- The renderer and winit objects are not made `Send` and never move to the
  simulation thread.
- This slice does not add a separate audio thread, a job system, rollback,
  extrapolation, or new frame interpolation beyond the existing partial-tick
  behavior.
- Immutable world generations structurally share unchanged chunk columns and
  session-static registry/item-profile owners. The ownership, measurement,
  isolation, and residual-cost record is
  [`simulation-publication-snapshot-design.md`](simulation-publication-snapshot-design.md);
  it does not change the single-owner or one-publication boundary.
- Resource-pack behavior, particle visuals, HUD fidelity, and terrain content
  are not expanded while their execution boundary is moved.

## Authoritative Behavior

Local vanilla 26.1 sources establish the ordering contract:

- `Minecraft.runTick` calls `packetProcessor.processQueuedPackets()` and
  scheduled tasks before its client-tick loop, limits a catch-up batch to ten
  calls to `Minecraft.tick`, and performs render extraction/rendering after the
  batch (`Minecraft.java`).
- `Minecraft.tick` processes GUI/screen/keybind state before renderer, entity,
  block-entity, and level ticks. It ticks particles before sending
  `ServerboundClientTickEndPacket` (`Minecraft.java`).
- `ClientLevel.tick` advances the world border and time only while the tick-rate
  manager runs normally, but decrements sky flash on every client tick
  (`ClientLevel.java`).
- `DeltaTracker.Timer.advanceGameTime` converts elapsed monotonic time with the
  current target MSPT, carries a fractional residual, and exposes that residual
  as the render partial tick (`DeltaTracker.java`).
- `ClientPacketListener` routes world-mutating handlers through
  `PacketUtils.ensureRunningOnSameThread`. BBB uses a different thread topology,
  but retains the same single ordered mutation owner.

Repo facts that constrain the migration:

- `runtime::events::drain_net_events_with_sinks_and_debug` is the only online
  dispatcher into `WorldStore::apply_play_packet`, but its current native
  effect adapter directly borrows audio, particle, renderer, and item runtimes.
- `runtime::pump_network_and_terrain` currently advances the client tick and
  builds one `RendererFrame`, while also performing renderer particle ticks and
  terrain/texture uploads.
- Winit callbacks currently call the same input helpers with mutable
  `ClientInputState` and `WorldStore`; several UI helpers also use renderer text
  measurement.
- Control already publishes paired immutable counter/world generations through
  `ArcSwap`; this is the publication model to reuse, not another authority.

## Ownership Boundary

### Simulation thread

`bbb-native::simulation` is the semantic owner. Its `SimulationCore` owns:

- the only mutable `WorldStore`;
- simulation `ClientInputState` and the canonical input helper calls;
- `mpsc::Receiver<NetEvent>`, `mpsc::Sender<NetCommand>`, and `NetCounters`;
- the fixed-rate client tick clock and tick-owned animation/FOV/lightmap state;
- packet-apply random state and client systems that mutate the world;
- server resource-pack lifecycle state up to a renderer reload request/ack;
- the simulation command inbox and ordered effect/publication sequence numbers.

The core must be runnable directly in tests and, during migration, on the winit
thread. `SimulationCore::step(now, tick_deadline_reached)` is the only operation
that drains inputs, applies net events, advances due ticks, and publishes
effects/state. A producer wake with no queued command/event before the deadline
returns no output and preserves the existing deadline. Starting a thread
changes its executor, not its semantics.

### Render / winit thread

The existing event-loop thread owns:

- `Window`, cursor/IME state, clipboard, URL opening, narrator process, and
  other platform adapters;
- `bbb_renderer::Renderer`, `TerrainUploadState`, GPU texture/model caches, and
  screenshot/readback work;
- the native audio and particle backend runtimes;
- resource-pack asset materialization and GPU reload work after a simulation
  reload request;
- frame profiling and frame-rate scheduling.

It may inspect only the latest immutable publication when deciding cursor,
overlay, and render presentation. Renderer-dependent text measurement stays on
this thread and returns pure measurement data in a typed command or render-only
projection; it does not grant the render thread mutable world access.

### Network runtime

`bbb-net` keeps connection I/O ownership. It sends ordered `NetEvent` values to
the simulation owner and accepts `NetCommand` values from it. Required
connection answers and gameplay commands stay FIFO; no connection task waits
for a simulation response through a blocking round trip.

## Frozen Integration Surface

Names may gain fields during the serial phases, but their roles and direction
are fixed:

```text
winit/control producers
  -> SimulationMailbox<SimulationCommand>
  -> SimulationCore::step(now, tick_deadline_reached)
       -> NetCommand sender
       -> RenderEffectBatch sender
       -> ArcSwap<SimulationPublication>
  -> render/platform consumers
```

### Commands

`SimulationCommand` is an ordered native value enum. It covers:

- focus, key, mouse-button, mouse-motion, mouse-wheel, text, and IME input;
- surface/layout facts or renderer measurements required by an existing input
  helper;
- the existing control input/net requests;
- renderer reload success/failure and level/terrain readiness feedback;
- shutdown.

Discrete transitions and text are lossless and ordered. High-rate mouse motion,
wheel deltas, surface changes, and profiler samples may be coalesced only when
the coalescing rule preserves their sum/latest-value semantics and cannot cross
a discrete command ordering barrier. Saturation must never silently lose a key
release, mouse release, control action, or shutdown. The render thread may take
a short mailbox mutex but never waits for simulation progress.

`TerrainReadiness` enters this ordered queue only for the first value of a
client-level generation or a readiness edge. Renderer/audio counters use a
separate latest-value mailbox slot: replacement neither advances the command
sequence nor wakes simulation, and the next real tick/network/input step
absorbs the latest value. Thus neither per-frame feedback source drives the
simulation scheduler.

Every command carries a monotonic sequence number assigned at enqueue. Tests
pin ordering across winit and control producers. Platform-only results return as
new commands; a simulation handler never calls a window, renderer, clipboard,
browser, or narrator directly.

Rich chat pointer input follows the same rule. The render owner resolves the
cursor against the exact wrapped glyph cache and enqueues only a revision-scoped
hit token. Simulation revalidates the level, delivery, resource generation, and
canonical Component run before executing URL/clipboard/dialog/network actions.
Chat narration is drained from `WorldStore` into the lossless platform-effect
batch before `SimulationPublisher::publish`; empty narration produces no effect
batch. Render consumes queued speech before the paired publication is made
visible, and a blocked renderer cannot mutate or make simulation reread the
retained generation.

### Publication

`SimulationPublication` is immutable and contains at least:

- a monotonic publication generation and client tick index;
- `Arc<WorldStore>` for the canonical read view;
- the tick interval, residual/partial-tick anchor, and frozen/running facts
  needed to compute the frame partial tick without advancing simulation;
- immutable input/HUD projection needed by render extraction;
- terrain/world revisions and other pure frame extraction inputs;
- the latest runtime counters that share this generation.

Publication uses `ArcSwap`; the render thread always renders one loaded
generation. Control publishes/reuses that exact world `Arc` and generation, so
it does not clone a different point in time. A renderer may skip generations;
it must never combine fields loaded from different generations.

World construction goes only through `WorldStore::publication_snapshot`.
Unchanged chunk-column palette/light payloads and session-static registry/item-
profile tables retain immutable structural owners; the first later mutation
copies only its semantic owner. Entity ECS state remains an explicit shell
copy, while heap-heavy component fields retain per-component immutable owners,
so sparse mutation copies only its changed payload and dense animation ticks do
not encounter a whole-store COW. Generation-matched runtime counters expose
recent/process-peak total publication, entity snapshot, and non-empty entity
client-tick batch durations plus scanned, mutated, and rebuilt entity counts.
If a generation changes only the narrow render/input runtime facts audited by
the command classifier, `SimulationPublisher` reuses the previous exact world
`Arc` and reports zero rebuilt entity shells. Network work, due ticks, gameplay
input, readiness/load transitions, reloads, and control mutations remain
conservative rebuild boundaries.

The input member is a narrow `RenderInputPublication`, not a complete clone of
the mutable `ClientInputState`. It includes the key/button and screen/debug state
that render/event-loop readers actually consume, while simulation movement,
timing, request, query, and crash owners remain absent. Render borrows the
generation's `SimulationFrameState` heap fields. Its entity extraction scratch
retains candidate, survivor, relationship-index, full-source, and final-instance
capacity across frames; the renderer returns the previous instance allocation
when applying a changed frame.

Stable item and map assets do not belong to the publication. Native extraction
reads the loaded generation and emits exact small model/map requests plus
ordered instances; the render thread alone owns the derived model-space CPU
assets, persistent GPU vertex/index buffers, fixed-page map textures and
resource-generation atlases. Resource and atlas generations are part of exact
owner equality before any large geometry is built. Partial tick, camera, pose,
light, overlay, foil/glint time and GUI target remain frame-local instance data.
Consequently an input-only publication can reuse the exact world `Arc`, an
unchanged render can reuse exact GPU owners, and neither optimization can mutate
an old publication or form a second canonical item/map store.

The first implementation publishes after any drained packet/input batch that
changes observable state and after every due tick batch. It may coalesce
multiple changes handled in one `step`, but must not wait for a rendered frame.

The remote debug subscriber is simulation-owned publication state. Startup
renderer flags and the FPS-chart toggle derive one deterministic set; accepted
network request changes update that canonical set before expiry is purged at
the current client game time. The immutable input selection and typed world
maps therefore generation-pair with render extraction. A failed bounded-channel
send leaves the old set intact for retry, disconnect clears the listener owner,
and no control-specific debug copy exists.

### Effects

`RenderEffectBatch` is an ordered, sequence-numbered value batch produced while
applying packets or ticks. It contains backend commands for audio, particles,
renderer events, resource reloads, narration/platform requests, and other
non-canonical side effects. The effect adapter passed to
`WorldStore::apply_play_packet` collects these values; it never borrows a
backend.

The render thread drains effects before extracting/rendering the publication
that first exposes their sequence. Effects are delivered at most once and in
simulation order. On backlog, idempotent latest-value effects may coalesce by an
explicit rule; one-shot sounds, particles, platform requests, and reload
transitions are lossless. A failed mandatory reload is acknowledged to
simulation and follows the existing rollback/disconnect policy.

Network commands are not render effects. They are sent directly by
`SimulationCore` in the canonical action order. `ClientTickEnd` is queued after
all simulation work and particle-spawn commands for that logical tick have been
emitted; it does not wait for the GPU/backend to display those commands.

## Tick and Frame Ordering

One `SimulationCore::step(now, tick_deadline_reached)` performs:

1. Drain a bounded batch of already queued net events in FIFO order, then
   ordered simulation commands up to the captured barrier.
2. Apply packet/control/input work through the existing unique dispatcher and
   input helpers.
3. Convert elapsed monotonic time with the current world target MSPT, preserve
   the fractional residual, and run at most ten due client ticks in a catch-up
   batch.
4. For each tick preserve the verified order: GUI/input systems, renderer-camera
   tick state, entity/block-entity/world systems, particle command generation,
   then `ClientTickEnd`.
5. Suppress an empty effect collector without consuming sequence space; seal
   non-empty effects, build one immutable publication if state changed, and
   compute the next deadline.

If more than ten ticks remain due, the residual remains pending and the next
step continues without sleeping. Network and shutdown commands are checked
between catch-up batches. Server tick rates above 20 Hz do not accelerate client
ticks; slower valid tick rates extend the target interval as today. Freeze gates
retain the existing raw-client-tick versus running-world-tick distinction.

The render thread loads one publication per frame, drains effects through its
sequence, computes/clamps partial tick from the publication anchor, extracts one
`RendererFrame`, services at most the existing terrain budgets, and renders.
Frame time never feeds back into the simulation accumulator.
Entity extraction follows `LevelRenderer.extractVisibleEntities`: a narrow ECS
batch is filtered by dispatcher distance/frustum (including leash and generated
renderer exceptions), compiled terrain, camera/local-player, sleeping,
spectator, and indirect-passenger rules before exactly one full projection of
each survivor. Relationship rules reuse the same candidate id index rather than
performing a full-table lookup per survivor.
Terrain servicing keeps its CPU snapshot/mesh cache on the render thread and
synchronizes stable level/chunk/section/layer GPU owners. A changed chunk
rebuilds only the exact Vanilla-invalidated section rows and replaces only
those section-layer buffers; unchanged sibling sections retain exact wgpu
handles. Unload removes only absent owners, and camera transparency resort
writes indices only. Neither the simulation owner nor the mesh worker may read
or mutate renderer/wgpu state. The measured mesh residual enables one named
worker with a four-job local-prediction/nearest-first queue and five-result
bound. It receives only immutable snapshot/atlas Arcs and dirty section
id/revision rows; render validates level/resource/aggregate mesh revision before
merging a result, discards stale work, performs the GPU upload itself, and joins
the worker on owner drop.
Simulation output wakes use a pending flag. Multiple queued outputs share one
event-loop wake until the consumer drains and race-safely clears the flag;
shutdown and panic exit retain their unconditional terminal wake.

## Resource Reload Barrier

The simulation owner tracks server pack session/generation and emits a reload
request containing immutable roots/identity. The render owner performs pack,
audio, particle, item, terrain, entity, HUD, and GPU reload work without holding
simulation state. It replies with the same generation and success/failure data.
Simulation ignores stale replies and commits world lifecycle state plus the
server response only for the current generation. Until acknowledged, rendering
may retain the previous active resources exactly as the existing barrier does.

## Shutdown and Failure Policy

- Window close, control `app.quit`, renderer fatal error, or network terminal
  state enqueues one idempotent shutdown command.
- The simulation thread stops accepting ordinary input, flushes required
  connection answers already produced, publishes `running=false`, and returns
  its owned state/result.
- The simulation shutdown path requests network disconnect; `LoopExiting`
  joins after all shorter winit/render borrows have ended and before dropping
  renderer/audio/platform resources.
- A panic or unexpected simulation exit is surfaced as a fatal native error;
  rendering stale state indefinitely is forbidden.
- Channels closing is a state transition covered by tests, not an ignored send
  error.

## Serial Engineering Phases

Only one phase advances at a time. Each phase is a normal, buildable commit and
keeps the current user-visible behavior unless its completion criteria say
otherwise.

1. **Simulation owner shell.** Add `simulation.rs`, move the world-facing state
   and current ordering into `SimulationCore`, and call `step` synchronously
   from the existing event loop. Renderer/platform borrows remain explicit
   adapters. Focused tests pin one owner and packet-before-tick/catch-up order.
2. **Effect isolation.** Replace direct dispatcher/tick backend borrows with
   ordered value batches and render-side consumers, still on one thread. Tests
   pin at-most-once FIFO effects, required net-command ordering, and stale
   resource-reload acknowledgements. Run a full gate.
3. **Single input mailbox.** Route winit and control mutation through typed
   commands drained by `SimulationCore`; keep renderer measurements and platform
   execution as explicit request/results. Source guards reject direct mutable
   world/input calls from event callbacks. Tests cover lossless releases,
   coalescing barriers, and existing input closure behavior.
4. **Immutable render publication.** Make render extraction, terrain, particles,
   cursor/overlay decisions, and control reads consume one publication; remove
   render-thread mutable world access. Run a full gate before thread activation.
5. **Thread activation.** Move `SimulationCore` to the named 20 Hz simulation
   thread, drive it from deadlines plus mailbox/network wakeups, implement join
   and failure propagation, and prove slow render work does not stall ticks or
   network apply. Run the full gate and playable gate.
6. **Closeout.** Remove compatibility adapters and old pump ownership, update
   architecture/ledger/archive evidence, measure snapshot/backlog behavior, and
   run the final full and playable gates at delivered `HEAD`.

The closeout instruments the unbounded simulation-output FIFO with current and
peak pending counts. A slow-frame record also reports the newest-publication
generation lag and ordered-effect sequence lag relative to the generation the
renderer may safely consume. The blocked-render regression proves pending
outputs grow while tick deadlines and external network wakeups continue, then
return to zero when the renderer drains the FIFO. These metrics diagnose a slow
render owner without turning observation into simulation backpressure.

No phase may introduce a temporary second gameplay path. Phase 3 switches each
input family atomically to commands; phase 4 switches each render projection to
the publication while the simulation remains synchronous. The actual thread
move in phase 5 is therefore a small executor/ownership change.

## Validation and Checkpoints

Every phase runs `bbb-native` focused tests, formatting, diff checks,
documentation lint when applicable, and complete diff review. Affected
`bbb-world`, `bbb-control`, `bbb-net`, `bbb-audio`, or `bbb-renderer` tests are
added when their surfaces change.

Full gates are required after phases 2 and 4, immediately after phase 5, and at
final delivery. Any Cargo/workspace, protocol, `unsafe`, or additional
concurrency change triggers the forced gate policy in `goal.md`. Phase 5 uses
the applicable playable checkpoint. Final delivery with the fixed Java/Weston
fixture must build the committed `HEAD`, run `scripts/playable-gate.sh live`
without fallback, then run `auto` when project policy still requests it. The
live portion must demonstrate that an injected slow render frame does not delay
client ticks, movement acknowledgements, or network application.

Useful deterministic regressions include:

- packet/input ordering around a tick barrier and the ten-tick catch-up cap;
- frozen versus running tick counters and partial-tick projection;
- a blocked fake renderer while simulation tick/publication generations
  continue;
- no lost key/mouse release under a saturated/coalescing mailbox;
- effect FIFO/at-most-once delivery and stale resource reload ack rejection;
- matched control/render generation publication;
- clean shutdown, producer closure, simulation panic propagation, and join.

## Failure Localization and Safe Stops

Phases 1-4 deliberately ran the core synchronously. A regression before the
thread activation was therefore attributable to ownership extraction, effect
routing, input routing, or snapshot projection rather than scheduling. Phase 5
added fake-clock and blocked-render tests that isolate scheduler failures; the
phase 6 pending/peak and generation-lag metrics localize render-consumer stalls.

Safe stops are the clean, gated tips after phases 2 and 4. After phase 5 starts,
the slice stops only at its successful gate, an explicit blocker, or a normal
fix-forward commit that restores the build; the implementation must not be
delivered with both threaded and direct mutation paths selectable at runtime.

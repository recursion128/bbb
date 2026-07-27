# Native profiler and metrics closure

Status: completed and archived.

Pre-start fully gated baseline:
`c37a14cd268a1288a950e2f01c7a14265cc4e8f8`.

## Goal and completion contract

Close the remaining applicable Vanilla 26.1 profiler work without introducing
a second runtime or a synthetic metrics-only execution path. Completion
requires:

- F3+1 reads a real nested profile tree for the latest native client frame,
  with Vanilla `ProfileResults.getTimes` percentages, counts, `unspecified`
  accounting, sorting, and numeric navigation;
- the native tree covers every applicable `Minecraft.runTick`,
  `Minecraft.tick`, and `Minecraft.renderFrame` section at the existing
  simulation/render ownership boundary, and carries repo-native child sections
  only where they measure real additional work;
- F3+L starts one recording, stops manually or automatically after ten seconds,
  cannot silently lose ordered start/stop requests, and resets its input state
  after automatic completion;
- a recording accumulates the same profile paths shown by F3+1 plus tick-time,
  CPU (when the platform exposes it), process-memory, chunk-rendering,
  chunk-dispatch, and GPU-utilization samples;
- tick-time increases of at least 200 percent retain the corresponding
  single-frame profile tree as a deviation report;
- report output is bounded, written off the render thread, and archived as an
  actual collision-safe `debug/profiling/*.zip` containing `system.txt`,
  `client/profiling.txt`, category CSV files, and applicable deviations;
- start, stopped-summary, successful archive, and failure feedback enter the
  canonical client-system chat path at their real lifecycle edges; the saved
  archive path is the clickable OpenFile target;
- unsupported GPU timestamp queries and unavailable platform samplers fail
  closed to absent metrics rather than fabricated values, while the debug GPU
  entry uses the latest real sample when one exists; and
- focused tests, the unsupported-feature ledger, archive index, final full
  gate, and P0 audit close at the final tip.

## Authority and non-goals

The authoritative sources are local Vanilla 26.1 `Minecraft.run`,
`Minecraft.runTick`, `Minecraft.tick`, `Minecraft.renderFrame`,
`Minecraft.constructProfiler`, `Minecraft.debugClientMetricsStart`,
`ActiveProfiler`, `FilledProfileResults`, `ProfilerPieChart`,
`ActiveMetricsRecorder`, `ClientMetricsSamplersProvider`,
`ServerMetricsSamplersProvider`, `MetricSampler`, and `MetricsPersister`.

The native client owns no integrated server, so one F3+L archive contains only
the client report. This slice does not add server profiling, Tracy capture,
general benchmarking, a profiler configuration UI, or an unbounded historical
chart. CLI configuration is not dumped into the archive because it is the
configuration owner and may contain connection-sensitive values.

Vanilla JVM heap and OSHI CPU samplers are translated to the native process
and platform rather than relabeled: process resident memory is the native
memory metric, per-core CPU rows exist only when a supported platform source
is available, and GPU utilization comes only from a supported wgpu timestamp
query. Missing sources are omitted exactly like Vanilla's caught CPU sampler
failure; a CPU wall-time proxy must not be called GPU utilization.

## Frozen Vanilla behavior

- `Minecraft.run` calls recorder `startTick` before `runTick` and `endTick`
  after it. `ActiveMetricsRecorder` stops at the first end-of-tick after a
  manual request or after its ten-second deadline.
- `ActiveProfiler` accumulates duration, count, min, max, and counters by paths
  separated with U+001E. `FilledProfileResults.getTimes` compares direct-child
  duration to the current node, inserts `unspecified` for the difference,
  sorts children by percentage/name, and keeps the root duration as the global
  denominator.
- `Minecraft.runTick` uses root sections
  `scheduledPacketProcessing`, `scheduledExecutables`, `tick`, `sound`,
  `toasts`, `mouse`, and `frame`. The applicable `Minecraft.tick` children are
  `gui`, `gameMode`, `screen`, `Keybindings`, `gameRenderer`, `entities`,
  `blockEntities`, `level`, `animateTick`, `particles`, and `keyboard`.
  `renderFrame` contributes `update`, `extract`, `gpuAsync`, `render`,
  `present`, `swapBuffers`, `frameLimiter`, and `fpsUpdate` where the native
  owner has a distinct measured phase.
- The tick-time threshold test fires when a later sample is greater than the
  prior sample by at least 200 percent. Each deviation captures the complete
  single-tick profile result before the profiler is reset.
- Metrics CSV files group samplers by category, use `@tick` as the first
  column, and write one row for every recorded tick. Profiler and deviations
  live below the client report root before that root is zipped with the system
  report.
- Starting emits `debug.profiling.start`; stopping first emits
  `commands.debug.stopped` with seconds, ticks, and rate, then successful IO
  emits `debug.profiling.stop` with the actual archive path. Output failure
  must be visible and must not leave the recorder active.

## Frozen ownership and integration surface

### Profile tree and simulation publication

- `bbb-native::debug_profiler` remains the only profile-result, recording, and
  report owner. Entries retain duration/count/max and named counters; chart
  projection and text persistence both consume that same tree.
- `DebugProfilerSectionSample` becomes a nested-path sample instead of a
  top-level name. Duplicate paths accumulate in call order, invalid or
  over-budget paths fail closed, and one latest-frame tree remains bounded.
- `SimulationFrameState` carries one immutable, bounded list of measured
  simulation sections. Network draining maps to
  `scheduledPacketProcessing`, ordered mailbox/application work maps to
  `scheduledExecutables`, and tick-owned groups map to the applicable Vanilla
  children above. The render owner merges that list with its current redraw
  sections; it never reads mutable simulation timing state.
- Instrumentation is conditional on F3+1 visibility or active recording.
  Ordinary frames retain only the existing coarse performance counters and do
  not allocate a profile path map.

### Renderer and metrics samplers

- `bbb-renderer` owns optional frame-bounding GPU timestamp queries because it
  owns the command encoder, queue, and device. Device creation requests the
  feature only when the adapter supports it; unsupported adapters preserve the
  existing renderer path and publish `None`.
- Timestamp readback is pipelined and non-blocking. The latest completed GPU
  duration pairs with the saved CPU frame duration to derive Vanilla's
  utilization ratio; stale generations and malformed/non-finite results are
  discarded.
- The renderer exposes only a small `gpu_utilization() -> Option<f64>` query.
  Native owns metric naming, CSV grouping, and debug text.
- Existing `RendererCounters` provide total/rendered sections, view distance,
  free mesh capacity/queue depth equivalents, and other native metrics without
  a parallel renderer snapshot.

### Recording, output, and feedback

- Ordered platform effects carry individual Start/Stop commands, not aggregate
  counts. Main applies them sequentially to the single `DebugProfiler`.
- A bounded recorder stores at most the frames possible in the ten-second
  window plus a narrow safety margin. A second bounded writer owner receives
  immutable report jobs, writes temporary content and the final ZIP, reports
  success/failure, and is joined at shutdown.
- Main polls writer completions and enqueues a typed simulation command.
  Simulation is the only owner that changes the F3+L input flag and appends
  final client-system chat. Render/platform code never mutates `WorldStore`.
- Archive names use a sanitized UTC timestamp, remote-server identity when
  available, protocol version, and collision suffix. Temporary directories
  are outside the archive root and are removed after success or failure.
- Tests inject time, sampler values, and output roots. They never write the
  repository tree or rely on wall-clock sleeps.

## Serial stages and verification

1. Replace the flat current-frame map with the bounded nested
   profile/result/recording model; add exact percentage/count/deviation,
   ten-second/manual-stop, CSV/profile text, ZIP layout, collision, and failure
   tests. Run
   `scripts/cargo-dev.sh test -p bbb-native debug_profiler`.
2. Add conditional simulation/render section publication, ordered F3+L
   lifecycle and chat completion commands, optional renderer timestamp query,
   and live metric projection. Run
   `scripts/cargo-dev.sh test -p bbb-native profiling`,
   `scripts/cargo-dev.sh test -p bbb-native profiler`, and
   `scripts/cargo-dev.sh test -p bbb-renderer gpu_timing`.
3. Remove the completed ledger item, archive this contract, append the
   engineering milestone index, review the integrated diff, then run the full
   gate and P0 audit.

Every stage is a buildable normal commit after its focused tests,
`cargo fmt --all --check`, `git diff --check`, documentation lint when
applicable, and full diff review. The contract plus three stages reaches the
four-commit engineering checkpoint, so the final tip runs
`scripts/cargo-dev.sh gate`. A failure stops at the preceding clean commit and
is fixed forward without rewriting history.

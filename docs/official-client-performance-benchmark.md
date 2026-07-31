# Official Minecraft 26.1 Equal-Load Benchmark

This document owns the repeatable comparison contract between the repo-native
client and the unmodified official Minecraft Java 26.1 client. The launcher,
replay generator, capture parsers, and smoke runners live in
`scripts/official_client_benchmark.py`; the shell entry point is
`scripts/official-client-benchmark.sh`.
The existing repo-native owner and frame-tail baselines remain in
`docs/simulation-render-performance-design.md`; this document adds only the
cross-client launch, workload, and external-measurement boundary.

The current host can execute both clients and the common replay, but it cannot
produce a publishable comparison. `summary.json` must keep
`formal_comparison_complete=false` and `metrics_publishable=false` until every
preflight and visible-content blocker below is cleared.

## Scope and safety boundary

The harness:

- validates the official version metadata, client and server jars, Linux
  libraries and natives, asset index, and every indexed asset by declared size
  and SHA-1;
- launches the official jar without patching, agents, mods, or a real access
  token;
- extracts native classifiers into a per-run directory while rejecting
  absolute paths, `..`, backslashes, NULs, symlinks, and basename collisions;
- uses a separate game directory, fixed `options.txt`, an owned Weston
  compositor, and process groups with bounded cleanup;
- keeps Mojang assets, jars, libraries, worlds, native libraries, tool packages,
  screenshots, and raw captures outside the repository.

It does not drive either client with `bbb-control`, synthetic mouse movement, or
keyboard replay. `bbb-control` is read only during a bbb smoke and records the
authoritative local pose plus renderer visible/resident counters. The only
control socket used during capture belongs to MangoHud and starts or stops
logging without displaying an overlay.

## Vanilla offline launch evidence

The launch contract is derived from the local 26.1 sources:

- `client/main/Main.java` registers `--offlineDeveloperMode`, keeps
  `--accessToken` syntactically required, derives an offline UUID when `--uuid`
  is absent, and maps `--quickPlayMultiplayer` to the requested server.
- `client/Minecraft.java` selects
  `YggdrasilAuthenticationService.createOffline`, `UserApiService.OFFLINE`, an
  already-completed profile future, and
  `ProfileKeyPairManager.EMPTY_KEY_MANAGER` in offline developer mode.
- `client/User.java` merely stores the supplied token. The harness always uses
  the fixed non-secret value `offline` and redacts it from launch evidence.
- `ClientHandshakePacketListenerImpl.authenticateServer` calls `joinServer`
  only when the server requests authenticated encryption. The benchmark server
  fixes `online-mode=false` and `enforce-secure-profile=false`.

The official command therefore includes:

```text
--offlineDeveloperMode
--username bbb-benchmark
--accessToken offline
--version 26.1
--assetIndex 30
--width 1280 --height 720
--quickPlayMultiplayer 127.0.0.1:<port>
```

It also supplies the verified full assets directory, classpath, and extracted
native directory. No launcher profile or online account is a remaining
prerequisite.

## Server-authoritative replay

`scripts/fixtures/official-client-benchmark/replay-26.1.json` is the canonical
schema-1 replay. It contains 1,440 contiguous server ticks and three equal
segments:

1. slow static orbit for stable draw and pipeline load;
2. preloaded flight for frustum, entity extraction, and translucent ordering;
3. step-and-hold chunk-boundary crossing for chunk updates, meshing, GPU upload,
   and tail latency.

The generated datapack gates replay start behind a scoreboard flag. After the
client joins, the server places it at the first pose, waits a fixed 30-second
warm-up, then releases the replay. Every tick runs `tp` with absolute
`x/y/z/yaw/pitch`; every segment emits start, capture-start, static-anchor, and
capture-end markers.

This follows the vanilla command path
`TeleportCommand.performTeleport` → `ServerPlayer.teleport` →
`ServerGamePacketListenerImpl.teleport` →
`ClientboundPlayerPositionPacket.of`. Both clients consequently receive the
same server-owned position packet sequence. The harness rejects a reordered or
numerically different marker sequence and writes `server-camera.jsonl` with its
own SHA-256.

The server fixes the seed, time, weather, difficulty, random tick speed, mob
spawning, patrols, traders, phantoms, and fire spread. It preloads all replay
view chunks before saving the source world, freezes a bounded entity scene, and
records the replay, source-world, and resource hashes separately.

## Equal-load settings

Both launch plans pin a 1280×720 window inside the same owned 1600×900 Weston
output, view and simulation distance 12, entity distance scale 1, fancy
graphics (including AO, player-affected chunk priority, cloud distance 64,
cutout leaves, entity shadows, weather radius 10, and RGSS filtering), clouds,
all particles, mipmap level 4, visible HUD, GUI scale 1, view bobbing on,
master audio disabled, VSync off, unlimited FPS, and the vanilla resource pack.
Vanilla uses the `custom` preset label so its post-load preset application
cannot overwrite the explicitly pinned Fancy-equivalent components, 12-chunk
distance, or zero menu blur. The harness validates the official client's parsed
post-launch options rather than trusting only the input file. The bbb smoke records
the actual renderer width/height, visible sections, resident bytes, draw calls,
chunk view, and local pose at every anchor.

A matching replay/world/resource/settings contract hash is necessary but not
sufficient. Formal runs additionally require reviewed screenshot pairs and an
API-neutral official-client draw or visible-work observation. A resident cap or
missing terrain must invalidate the run rather than make bbb appear faster.

## Capture and metric contract

The fixed MangoHud 0.8.4 release is audited before use. Its relocated OpenGL and
Vulkan libraries and manifest must match the hashes embedded in the harness.
The configuration sets `no_display` and `permit_upload=0`. Because MangoHud
0.8.4 skips its autostart check while `no_display` is active, the harness uses
the release's abstract Unix control socket to start logging at the first
`capture_start` marker and stop it at the final `capture_end`. Closed raw and
summary files are accepted as the stop acknowledgement when the release fails
to return `LoggingFinished`.

Raw MangoHud samples support only these labels:

- end-to-end presented-frame interval p50/p95/p99;
- mean FPS, 1% low, and explicit hitch counts;
- sampled GPU load, clock, and temperature when the driver exposes them.

They are not CPU render durations or GPU frame durations. The same client PID
must also be measured with `perf stat` for task-clock, cycles, instructions,
context switches, CPU migrations, and page faults. Those process totals are
reported only as means per presented frame; `perf stat` supplies no CPU-frame
percentiles.

An AMD ftrace/GPUVis duration claim is permitted only after one client context
can be followed across `amdgpu_cs_ioctl`, scheduler job, fence signal, vblank,
and scheduler switch events for both radeonsi OpenGL and RADV Vulkan while
excluding Weston. Otherwise the report stops at common frame intervals and
GPU load. RenderDoc is anchor-only pass/draw/content evidence, never a
percentile source.

## Formal sampling and invalidation

A publishable comparison requires at least three valid captures per client in
Java → bbb → bbb → Java order, the same fixed warm-up, and a measured MangoHud
overhead at or below 1% median. Each scenario is summarized separately.

A run is invalid when any replay, world, resource, settings, resolution, pose,
or visible-content check differs; when a position correction, resource failure,
capture loss, or throttling occurs; or when the capture cannot be associated
with the client. Raw files and machine-readable `summary.json` are mandatory.

## Current external blockers

Run `capture-preflight` immediately before sampling. On the current host it
proves:

- `/dev/dri/renderD128` is `root:render` mode `0660`, and the benchmark user
  cannot open it, so both smoke runs fall back to software rendering;
- `kernel.perf_event_paranoid=4` rejects the required process counters;
- tracefs does not expose the required AMD/fence/DRM/scheduler events as
  writable to the benchmark user;
- RenderDoc is absent, and anchor review currently shows different visible
  terrain between the clients even though camera and contract hashes match;
- MangoHud closes raw captures successfully on both APIs, but its native
  Wayland teardown crashes both clients, so the harness uses post-capture
  `SIGKILL` after logging has closed.

The first three conditions require an external host policy or maintained runner
change. The visible-content mismatch requires a bbb rendering/workload fix plus
new draw evidence before formal sampling. Do not publish the software-rendered
smoke numbers as a performance comparison.

## Commands and focused tests

Provisioning audit and host preflight:

```sh
scripts/official-client-benchmark.sh audit \
  --mc-code-root /home/zgy/Work/mc-code \
  --java /home/zgy/.local/jdks/jdk-25.0.3+9/bin/java

scripts/official-client-benchmark.sh capture-preflight \
  --java /home/zgy/.local/jdks/jdk-25.0.3+9/bin/java \
  --mangohud-root /tmp/bbb-benchmark-tools/MangoHud-0.8.4/package \
  --output /tmp/bbb-benchmark-capture-preflight.json
```

The `run-official-smoke` and `run-bbb-smoke` subcommands take the same replay
and world manifest, unique artifact directory, unique Weston socket, and an
explicit `--accept-eula`. `summarize-blocked` combines both smoke manifests and
the preflight without promoting their metrics.

Focused tests are network-free:

```sh
scripts/vanilla-source-tools.sh python -m unittest \
  scripts.tests.test_official_client_benchmark
```

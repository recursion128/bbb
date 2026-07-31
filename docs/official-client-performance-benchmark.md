# Official Minecraft 26.1 Equal-Load Benchmark

This document owns the repeatable comparison contract between the repo-native
client and the unmodified official Minecraft Java 26.1 client. The launcher,
replay generator, capture parsers, and smoke runners live in
`scripts/official_client_benchmark.py`; the shell entry point is
`scripts/official-client-benchmark.sh`.
The existing repo-native owner and frame-tail baselines remain in
`docs/simulation-render-performance-design.md`; this document adds only the
cross-client launch, workload, and external-measurement boundary.

The current host completed two ABBA rounds (four valid captures per client) on
the AMD Radeon 610M. The machine-readable result is
`/tmp/bbb-formal-abba-v1/summary.json`, SHA-256
`370a87331c8bdb3eb15e9c6ebc824608b9a1329a948d9b2d4df321b5cfde3fcb`;
large external assets, clients, tools, worlds, screenshots, and captures remain
outside the repository.

## Scope and safety boundary

The harness:

- validates the official version metadata, client and server jars, Linux
  libraries and natives, asset index, and every indexed asset by declared size
  and SHA-1;
- launches the official jar without patching, agents, mods, or a real access
  token;
- extracts native classifiers into a per-run directory while rejecting
  absolute paths, `..`, backslashes, NULs, symlinks, and basename collisions;
- uses a separate game directory, fixed `options.txt`, one sequential
  Weston/Xwayland compositor, and process groups with bounded cleanup;
- keeps Mojang assets, jars, libraries, worlds, native libraries, tool packages,
  screenshots, and raw captures outside the repository.

It does not drive either client with `bbb-control`, synthetic mouse movement, or
keyboard replay. `bbb-control` is read only during a bbb smoke and records the
authoritative local pose plus renderer visible/resident counters. The only
mutating client-side capture control belongs to MangoHud and starts or stops
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

Both launch plans pin a 1280×720 window inside the same 1280×720 hardware
Weston/Xwayland output, view and simulation distance 12, entity distance scale 1, fancy
graphics (including AO, player-affected chunk priority, cloud distance 64,
cutout leaves, entity shadows, weather radius 10, and RGSS filtering), clouds,
minimal particles, mipmap level 4, visible HUD, GUI scale 1, view bobbing on,
master audio disabled, VSync off, unlimited FPS, and the vanilla resource pack.
Vanilla uses the `custom` preset label so its post-load preset application
cannot overwrite the explicitly pinned Fancy-equivalent components, 12-chunk
distance, or zero menu blur. `maxFps:260` is Vanilla's named unlimited value;
`inactivityFpsLimit:"minimized"` disables the otherwise automatic 30 FPS
short-AFK limit while retaining iconified-window throttling. The harness
validates the official client's parsed post-launch options rather than trusting
only the input file. The bbb smoke records the actual renderer width/height,
visible sections, resident bytes, draw calls, chunk view, and local pose at
every anchor.

A matching replay/world/resource/settings contract hash is necessary but not
sufficient. The reviewed screenshot evidence hash is
`cbf036d934a21c757f239956aa01f1e741174df79918975ec20bebcc0cf79b2b`.
RenderDoc 1.45 raw anchors prove both clients submitted work on the hardware
GPU: Vanilla OpenGL has 330/234/248 draw API calls and bbb Vulkan has
684/701/668 for static/flight/boundary. The RenderDoc evidence hash is
`1c13956244098e4763039fd14524b4269edea7824ef47a8772979aa0c998a4b4`.
Those API counts establish non-empty visible draw work; different APIs and
batching make the counts unsuitable as an equality or performance metric.

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

- end-to-end application-frame interval at the API present boundary,
  p50/p95/p99;
- mean FPS, 1% low, and explicit hitch counts;
- sampled GPU load, clock, and temperature when the driver exposes them.

They are not compositor presentation-completion times, CPU render durations,
or GPU frame durations. The same client PID
must also be measured with `perf stat` for task-clock, cycles, instructions,
context switches, CPU migrations, and page faults. Those process totals are
reported only as means per presented frame; `perf stat` supplies no CPU-frame
percentiles.

Weston timeline records Xwayland surface-commit intervals only for measuring
capture overhead. The measured MangoHud median effects are +0.0106% for
Vanilla and -0.0317% for bbb, both below the frozen 1% rejection threshold.

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

## Current-host formal result

All eight formal captures passed with zero position corrections, the same
camera-log SHA-256
`61a3e901230da541718fb17fb889f27cf18d61ac0a4ababfc3632a274b8190bd`,
the replay/world/resource/settings/scene hashes below, and the release bbb
binary SHA-256
`9d99bc50ac4aee6e9f732b9e5eb1b1e0b97a96ae5bef519ec65daf86e62d501d`.
The fresh-credential preflight at
`/tmp/bbb-benchmark-capture-preflight-final-fresh-groups.json` records the
benchmark user in `video` and `render`, a successful read-write open of
`/dev/dri/renderD128`, `perf_event_paranoid=1`, and all six required `perf`
events available. The long-lived development shell predates those group
memberships, so captures use a fresh credential process rather than treating
that stale shell's group list as host capability.

| Contract | SHA-256 |
| --- | --- |
| replay | `c40b92e6686a9bfbc32dd4192020c4c95caa2ab1607383e33dffa6b8562eae14` |
| world | `abe09627d1e9e300ec4dc5b9bdcc870785f1527b265bdb09061f347fd9a74b66` |
| resources | `8aa8d02d6806ef9396564b83b7b64f73014ea654d8059a1b8ae6b09f3f67df0c` |
| settings | `51081ecb561de7f053ddf3ed40255865fc037da5ba895cc8dcc2d73aaa4236c9` |
| scene | `6acedfc5cb5323032c034c04630b7a878acce98974b6d8d7c1ec9a8daf60b747` |

Pooled MangoHud application-frame results:

| Scene/client | Samples | p50 ms | p95 ms | p99 ms | 1% low FPS | ≥33.333 ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| overall Vanilla | 152,383 | 1.433 | 2.445 | 3.695 | 253.62 | 0 |
| overall bbb | 9,150 | 8.452 | 142.407 | 150.685 | 6.12 | 1,087 |
| static Vanilla | 35,504 | 2.152 | 2.474 | 2.769 | 313.00 | 0 |
| static bbb | 4,373 | 8.595 | 24.750 | 144.475 | 6.62 | 174 |
| flight Vanilla | 57,165 | 1.205 | 1.765 | 1.988 | 444.91 | 0 |
| flight bbb | 3,667 | 6.304 | 137.327 | 145.499 | 6.44 | 294 |
| boundary Vanilla | 27,806 | 1.468 | 3.446 | 3.799 | 250.98 | 0 |
| boundary bbb | 577 | 127.313 | 151.610 | 175.038 | 5.42 | 314 |

Overall mean GPU telemetry is 97.50% / 2,172.76 MHz / 61.33°C for
Vanilla and 35.74% / 1,308.04 MHz / 52.45°C for bbb. Maximum observed
CPU/GPU temperatures were 90/64°C for Vanilla and 88/55°C for bbb, below the
frozen 95°C invalidation threshold.

`perf stat` process means per captured frame:

| Client | task-clock ms | cycles | instructions | ctx switches | migrations | page faults |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Vanilla | 0.659 | 3,216,082 | 4,821,124 | 14.154 | 0.212 | 12.405 |
| bbb | 55.442 | 293,755,625 | 684,868,191 | 56.038 | 1.292 | 88.090 |

These means do not provide CPU-frame percentiles. The measurements identify a
real bbb long-tail regression, strongest during chunk-boundary crossing; later
optimization work must diagnose its native owner without reducing visible or
resident work.

Raw formal artifacts are under
`/tmp/bbb-formal-abba-v1/{01-vanilla,02-bbb,03-bbb,04-vanilla,05-vanilla,06-bbb,07-bbb,08-vanilla}`.
Each directory contains its launch manifest, server/camera logs, raw MangoHud
CSV, perf CSV, and `smoke.json`; scenario membership is encoded by elapsed time
against the replay's capture markers. Common draw artifacts are under
`/tmp/bbb-benchmark-renderdoc-{vanilla,bbb}-v2` and
`/tmp/bbb-benchmark-renderdoc-xml-v1`.

## Tool identities, invalid trials, and residual blocker

- MangoHud 0.8.4 archive SHA-256 is
  `aa0b2a009552f5e7c50baabf361267a79ea5e5c0fea440786f3b202d51f57753`.
- RenderDoc 1.45 commit is
  `2fc0bc04cb95499635f63986a55bc6f67849dd9f`; archive SHA-256 is
  `b0a7ee8ec78c4fa511eb44137380d99a748472e5fd24c877f8afcc860a172a42`.
- Earlier llvmpipe captures, the official AFK-limited probe, the
  `opt-level=1` bbb probe, and the initial Weston timebase parse are invalid
  trials and are excluded from `summary.json`.
- RenderDoc full replay fails with `APIReplayFailed` on this
  Mesa/Xwayland combination, while replay-independent XML conversion succeeds.
  Raw `.rdc` and XML event streams are therefore content/draw evidence only.
- The kernel exposes `dma_fence:dma_fence_signaled`, not the requested
  `fence:fence_signaled`. No common radeonsi/RADV proof currently associates a
  client context through submission, scheduler queue, fence signal, and the
  headless Xwayland presentation path while excluding Weston. Consequently no
  “GPU frame duration” is reported; common MangoHud intervals and GPU
  load/clock/temperature remain valid.
- MangoHud's teardown path is isolated after its logging socket has
  acknowledged closed raw output; the harness then performs bounded client
  cleanup. This occurs after the measured interval and is not counted.

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
and world manifest, unique artifact directory, one sequential compositor, and
an explicit `--accept-eula`. `summarize-renderdoc`, `summarize-workload`, and
`summarize-formal` reject incomplete evidence rather than promoting smoke
metrics.

The real formal client command surfaces were:

```sh
python3 scripts/official_client_benchmark.py run-official-smoke \
  --replay scripts/fixtures/official-client-benchmark/replay-26.1.json \
  --mc-code-root /home/zgy/Work/mc-code \
  --java /home/zgy/.local/jdks/jdk-25.0.3+9/bin/java \
  --world-manifest /tmp/bbb-benchmark-world-v8/world-snapshot.json \
  --x-display :0 --wayland-display bbb-renderdoc-xwayland \
  --mangohud-root /tmp/bbb-benchmark-tools/MangoHud-0.8.4/package \
  --perf /usr/bin/perf --accept-eula --artifact-dir <unique-run-dir>

python3 scripts/official_client_benchmark.py run-bbb-smoke \
  --replay scripts/fixtures/official-client-benchmark/replay-26.1.json \
  --mc-code-root /home/zgy/Work/mc-code \
  --java /home/zgy/.local/jdks/jdk-25.0.3+9/bin/java \
  --bbb-binary /tmp/bbb-target-main/release/bbb-native \
  --world-manifest /tmp/bbb-benchmark-world-v8/world-snapshot.json \
  --x-display :0 --wayland-display bbb-renderdoc-xwayland \
  --mangohud-root /tmp/bbb-benchmark-tools/MangoHud-0.8.4/package \
  --perf /usr/bin/perf \
  --extra-library-path \
    /tmp/bbb-benchmark-tools/x11-client-deps/usr/lib/x86_64-linux-gnu \
  --accept-eula --artifact-dir <unique-run-dir>
```

Focused tests are network-free:

```sh
scripts/vanilla-source-tools.sh python -m unittest \
  scripts.tests.test_official_client_benchmark
```

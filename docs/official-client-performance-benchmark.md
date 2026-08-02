# Official Minecraft 26.1 Equal-Load Benchmark

This document owns the repeatable comparison contract between the repo-native
client and the unmodified official Minecraft Java 26.1 client. The launcher,
replay generator, capture parsers, and smoke runners live in
`scripts/official_client_benchmark.py`; the shell entry point is
`scripts/official-client-benchmark.sh`.
The existing repo-native owner and frame-tail baselines remain in
`docs/simulation-render-performance-design.md`; this document adds only the
cross-client launch, workload, and external-measurement boundary.

The current publishable result is
`/tmp/bbb-formal-comparison-summary-v4.json`, SHA-256
`ced023ba8a4369b617ac801093d29fc26e56d04dbe3ca869ba7ff6ec0f2faa00`.
It follows a rendering-mechanism and visible-workload P0 gate; earlier captures
that predate that gate remain diagnostic only. Large external assets, clients,
tools, worlds, screenshots, and captures remain outside the repository.

## Scope and safety boundary

The harness:

- validates the official version metadata, client and server jars, Linux
  libraries and natives, asset index, and every indexed asset by declared size
  and SHA-1;
- launches the official jar without patching, agents, mods, or a real access
  token;
- extracts native classifiers into a per-run directory while rejecting
  absolute paths, `..`, backslashes, NULs, symlinks, and basename collisions;
- uses a separate game directory, fixed `options.txt`, clients launched
  sequentially on one X11 compositor, and process groups with bounded cleanup;
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

Both launch plans pin a 1280×720 client area on the same X11 display and
compositor, view and simulation distance 12, entity distance scale 1, fancy
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
sufficient. Same-tick screenshots, zero-error geometry assertions, 11
material-specific ROIs, pixel diffs, same-client noise calibration, and full
manual review form the visible-workload gate. Its evidence hash is
`576b14ec01439bc64c3ddcde335b387841a592cb4a006f0404423f3d07e22604`.
At the three bbb anchors the frozen resident set is 637 chunks / 15,288
sections; visible sections are 2,098 / 2,130 / 2,109.

RenderDoc 1.45 raw anchors prove both clients submitted work on the hardware
GPU: Vanilla OpenGL has 340/257/285 draw API calls and bbb Vulkan has
337/330/283 for static/flight/boundary. The RenderDoc evidence hash is
`6b38c475615448aa408b28ca943f407d8bee02f3341fa042888c24f354e65a19`.
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

A publishable comparison requires two complete Java → bbb → bbb → Java rounds,
four valid captures per client, the same fixed warm-up, and a measured MangoHud
overhead at or below 1% median. Each scenario is summarized separately.

A run is invalid when any replay, world, resource, settings, resolution, pose,
or visible-content check differs; when a position correction, resource failure,
capture loss, or throttling occurs; or when the capture cannot be associated
with the client. Raw files and machine-readable `summary.json` are mandatory.

## Current-host formal result

All eight formal captures passed with zero position corrections, zero
`>=33.333 ms` segment hitches, the same camera-log SHA-256
`f3749c011859c34d97f5ce0f36fdfec45e2ab7c81b51b405ad046489943a0b3a`,
the replay/world/resource/settings/scene hashes below, and the gated release
bbb binary SHA-256
`18b6fffe1238b280bce8292b935f317eb74933128834f2d79cb5721196649aaf`.
The fresh-credential preflight at
`/tmp/bbb-benchmark-capture-preflight-final-fresh-groups.json` records the
benchmark user in `video` and `render`, a successful read-write open of
`/dev/dri/renderD128`, `perf_event_paranoid=1`, and all six required `perf`
events available. The long-lived development shell predates those group
memberships, so captures use a fresh credential process rather than treating
that stale shell's group list as host capability.

| Contract | SHA-256 |
| --- | --- |
| replay | `5073908b5e5cd7eeca023540b9c2844e9c4d196610a2fbb67866058c5421c388` |
| world | `59bc552530d1c13ce3d618be5669e5d99d052e01d53e99a879a44008c7ba4bca` |
| resources | `8aa8d02d6806ef9396564b83b7b64f73014ea654d8059a1b8ae6b09f3f67df0c` |
| settings | `4ca64c1649a9ada45f8d9d7903abd488902accab7d227c5d9017b714a8ac8093` |
| scene | `6f77811b5deafacea3c7666414cc1705c396a987b015e2a5f1f4832e872f73bc` |

Pooled MangoHud application-frame results:

| Scene/client | Samples | p50 ms | p95 ms | p99 ms | 1% low FPS | ≥33.333 ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| overall Vanilla | 57,798 | 3.345 | 4.981 | 5.751 | 166.07 | 0 |
| overall bbb | 71,890 | 2.722 | 4.229 | 4.683 | 138.21 | 0 |
| static Vanilla | 17,120 | 3.903 | 4.171 | 4.361 | 207.70 | 0 |
| static bbb | 22,808 | 2.888 | 3.208 | 3.503 | 194.99 | 0 |
| flight Vanilla | 22,626 | 2.942 | 3.422 | 3.734 | 251.43 | 0 |
| flight bbb | 27,476 | 2.393 | 2.676 | 2.881 | 153.75 | 0 |
| boundary Vanilla | 18,052 | 3.385 | 5.611 | 6.019 | 160.47 | 0 |
| boundary bbb | 21,606 | 2.744 | 4.573 | 4.961 | 110.51 | 0 |

Overall mean GPU telemetry is 97.48% / 2,160.26 MHz for Vanilla and 97.19% /
2,157.40 MHz for bbb. Maximum observed CPU/GPU temperatures were 77/63°C for
Vanilla and 76/63°C for bbb, below the frozen 95°C invalidation threshold.

`perf stat` process means per captured frame:

| Client | task-clock ms | cycles | instructions | ctx switches | migrations | page faults |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Vanilla | 1.083 | 4,479,328 | 6,833,680 | 19.069 | 0.258 | 17.949 |
| bbb | 1.309 | 5,853,361 | 10,208,141 | 13.652 | 0.244 | 0.332 |

These means do not provide CPU-frame percentiles. bbb uses about 20.8% more
task-clock, 30.7% more cycles, and 49.4% more instructions per presented frame,
even though its application-frame p50/p95/p99 are lower in every scene. Its 1%
low remains lower because the sparse sub-33.333 ms tail beyond p99 is wider;
neither client has a formal hitch. This is maintenance evidence, not permission
to reduce visible or resident work in a later optimization.

Raw formal artifacts are under
`/tmp/bbb-formal-abba-final-v4/{01-vanilla,02-bbb,03-bbb,04-vanilla,05-vanilla,06-bbb,07-bbb,08-vanilla}`.
Each directory contains its launch manifest, server/camera logs, raw MangoHud
CSV, perf CSV, and `smoke.json`; scenario membership is encoded by elapsed time
against the replay's capture markers. The manifest and summary are
`/tmp/bbb-formal-comparison-manifest-v4.json` and
`/tmp/bbb-formal-comparison-summary-v4.json`; visible-workload evidence is
`/tmp/bbb-benchmark-workload-v82-gated-summary.json`. Common draw artifacts are
listed by `/tmp/bbb-benchmark-renderdoc-summary-direct-depth-final-v1.json`.

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
  --world-manifest /tmp/bbb-benchmark-world-v15/world-snapshot.json \
  --x-display :0 --wayland-display bbb-formal-final \
  --mangohud-root /tmp/bbb-benchmark-tools/MangoHud-0.8.4/package \
  --perf /usr/bin/perf --accept-eula --artifact-dir <unique-run-dir>

python3 scripts/official_client_benchmark.py run-bbb-smoke \
  --replay scripts/fixtures/official-client-benchmark/replay-26.1.json \
  --mc-code-root /home/zgy/Work/mc-code \
  --java /home/zgy/.local/jdks/jdk-25.0.3+9/bin/java \
  --bbb-binary /tmp/bbb-target-main/release/bbb-native \
  --world-manifest /tmp/bbb-benchmark-world-v15/world-snapshot.json \
  --x-display :0 --wayland-display bbb-formal-final \
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

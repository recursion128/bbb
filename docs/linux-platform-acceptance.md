# Linux x86_64 Platform Acceptance

This contract declares one supported automated release row for the scoped
external-multiplayer client:

`Linux x86_64 / Wayland / Vulkan / windowed`

It does not infer support from `cfg` compilation, historical measurements, or
unit tests alone. `scripts/platform-acceptance-gate.sh` must run at a clean
fully gated `HEAD` and retain a report for the exact commit.

## Required evidence

One invocation performs and validates all of the following:

- a fresh `cargo build --locked --release -p bbb-native` in a new external
  target directory;
- `bbb-native --build-info` JSON with the client version, Minecraft 26.1
  protocol and resource/data pack formats, target OS and target architecture;
- the checked-in playable fixture, a real official-server headless
  Login/Configuration/Play probe, and the complete live playable scenario;
- actual winit Wayland and wgpu Vulkan selection from stable production logs;
- ordered `app.quit`, network disconnect, process reap and zero exit status;
- existing focused tests for focus/key/mouse projection, lossless release
  delivery, clipboard, IME and ordered network shutdown; and
- the current lifecycle-soak report for resize/minimize/restore,
  surface-recovery, deterministic no-audio-device, terminal gauges and clean
  process exit.

The report records the exact commands and artifact paths. The release row fails
if the worktree changes during execution, the full-gate stamp is not exactly
`HEAD`, the lifecycle-soak stamp is not an ancestor, any required receipt is
missing, or any live/build/backend assertion fails.

## Evidence boundary

The maintained headless Wayland runner has no physical compositor seat/input
injector. Focus, keyboard, mouse, clipboard and IME evidence therefore proves
bbb's platform-event semantic owners and the real Wayland window construction,
but does not claim compositor-originated physical injection. Its fixed scale
factor is recorded but does not prove a live DPI transition.

Fullscreen, DPI transitions and an X11 release row remain delivery-validation
gaps until a maintained interactive runner exists. They do not expand the
declared windowed row and do not create a second input/window owner.
macOS and Windows remain governed by their ledger restart condition.

## Local evidence

On success the gate writes `.git/bbb/last-linux-platform`,
`linux-platform-log.tsv`, and `last-linux-platform-report.json`. These are
checkout-local evidence, never tracked release artifacts. Re-run the gate when
this contract, the manifest, startup/window/GPU selection, live playable
contract, input platform adapter, or release build configuration changes.

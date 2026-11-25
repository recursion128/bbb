# Packaging the desktop client, dedicated server, and Web gateway

This directory holds what an operator or user needs to package and run
`bbb-client` on Linux and Windows desktops, `bbb-server` and `bbb-web-host`
as supervised services on Linux and macOS, and TLS reverse-proxy configs for
browser cross-origin isolation. All launcher, supervisor, and gateway files
are pinned by `scripts/tests/test_packaging.py`: every option they pass has to
be one the corresponding binary's command line accepts, so a renamed flag fails
the gate instead of a binary or service that will not start.

## Supported release targets

| Platform | Target triple | Packaged artifacts |
| --- | --- | --- |
| Linux | `x86_64-unknown-linux-gnu` | Desktop client, dedicated server, admin CLI, and Web gateway |
| macOS | `aarch64-apple-darwin` | Dedicated server, admin CLI, and Web gateway |
| Windows | `x86_64-pc-windows-msvc` | Desktop client |

The browser WASM bundle (`wasm32-unknown-unknown`) is built alongside
`bbb-web-host` via `cargo dev build-web --release` and served as static assets
by the host gateway.

## Building release binaries

Workspace builds never write to a repo-local `target/` directory; `.cargo/config.toml`
only aliases `cargo dev`/`worker`/`build-web`/`web-up` (compiling `bbb-xtask` into
`.local/xtask-target`), while `cargo dev` or an explicit `CARGO_TARGET_DIR` places
workspace build outputs in an external Cargo target directory
(`/tmp/bbb-target-main` on Linux/macOS, `%TEMP%\bbb-target-main` on Windows,
or under `BBB_TARGET_ROOT`). Bare `cargo build` without `CARGO_TARGET_DIR` writes
to a repo-local `target/`, so always pass `CARGO_TARGET_DIR` or invoke `cargo dev`.
Build with optimisation on and the lockfile enforced so artifacts are reproducible
from the commit alone:

```sh
# Dedicated server, admin CLI, and Web gateway (Linux / macOS)
CARGO_TARGET_DIR=/tmp/bbb-target-main cargo build --locked --release --target <triple> -p bbb-server -p bbb-admin -p bbb-web-host

# Desktop client (Linux)
CARGO_TARGET_DIR=/tmp/bbb-target-main cargo build --locked --release --target x86_64-unknown-linux-gnu -p bbb-client
```

```powershell
# Desktop client (Windows)
$env:CARGO_TARGET_DIR = "$env:TEMP\bbb-target-main"
cargo build --locked --release --target x86_64-pc-windows-msvc -p bbb-client
```

Equivalently, `cargo dev build --release --target <triple> -p <package>...`
passes `--locked` and resolves the external target directory automatically.

- `bbb-client` is the native desktop Minecraft 26.1 multiplayer client.
- `bbb-server` is the dedicated Minecraft 26.1 server.
- `bbb-admin` is the MCP management CLI an operator runs against `bbb-server`.
- `bbb-web-host` is the loopback HTTP/WebSocket gateway that serves the threaded
  WASM + WebGPU client, the assembled `core-26.1.zip` asset pack, `/proxy/*`
  skin/pack fetches, and the `/ws` binary WebSocket-to-TCP bridge.

Cross-building host binaries is not part of the recipe: each target is built on
its own platform, because the gate that certifies a build runs on the platform
it certifies.

## Packaging the desktop client (`bbb-client`)

Interactive launches require Minecraft Java 26.1 assets via `--vanilla-root <PATH>`,
`BBB_VANILLA_ROOT`, the managed Vanilla cache (`~/.cache/bbb/vanilla` on Linux,
`%LOCALAPPDATA%\bbb\vanilla` on Windows), or an existing `.minecraft` directory.
Both desktop launcher wrappers (`packaging/bbb-client-launch.sh` and
`packaging/bbb-client-launch.ps1`) pin `--game-dir` to a per-user data
directory so `options.txt`, screenshots, and recordings never clutter the install
folder, and accept optional environment overrides matching the normalized flags
in `crates/bbb-client/src/startup.rs`:

- `BBB_GAME_DIR` -> `--game-dir <PATH>` (default `${XDG_DATA_HOME:-$HOME/.local/share}/bbb-client` on Linux, `%APPDATA%\bbb-client\game` on Windows)
- `BBB_VANILLA_ROOT` -> `--vanilla-root <PATH>`
- `BBB_SERVER` -> `--server <ADDR>` (automatically connects on startup)
- `BBB_FULLSCREEN=1` -> `--fullscreen` (also accepts `true` or `yes`, case-insensitive)

Explicit flags passed on the command line take precedence over wrapper defaults
and are forwarded directly to `bbb-client`.

### Linux (`x86_64-unknown-linux-gnu`)

Runtime dependencies: Wayland desktop session, Vulkan loader/driver (such as
`mesa-vulkan-drivers`), and OpenAL (`libopenal1`). Install system-wide or run
directly from an extracted release tarball (`./bbb-client-launch.sh`):

```sh
install -m 0755 /tmp/bbb-target-main/x86_64-unknown-linux-gnu/release/bbb-client /usr/local/bin/bbb-client
install -m 0755 packaging/bbb-client-launch.sh /usr/local/bin/bbb-client-launch
install -m 0644 packaging/bbb-client.desktop /usr/local/share/applications/bbb-client.desktop
```

`packaging/bbb-client.desktop` invokes `/usr/local/bin/bbb-client-launch` and
registers a `Launch Fullscreen` desktop action passing `--fullscreen`.

### Windows (`x86_64-pc-windows-msvc`)

`bbb-client.exe` uses native Win32 windows, the Direct3D 12 backend (Windows builds compile only `dx12` in `crates/bbb-client-render/Cargo.toml`; set `BBB_SOFTWARE_GPU=1` to force the Direct3D 12 WARP software adapter on headless hosts), and the OpenAL DLL verified from the Vanilla 26.1 manifest. Stage the binary alongside the Windows launcher wrappers from `%TEMP%\bbb-target-main\x86_64-pc-windows-msvc\release\bbb-client.exe`:

```powershell
$dist = "$env:LOCALAPPDATA\Programs\bbb-client"
New-Item -ItemType Directory -Force -Path $dist | Out-Null
Copy-Item "$env:TEMP\bbb-target-main\x86_64-pc-windows-msvc\release\bbb-client.exe" $dist
Copy-Item packaging\bbb-client-launch.ps1, packaging\bbb-client-launch.cmd $dist
```

Double-click `bbb-client-launch.cmd` (which invokes `bbb-client-launch.ps1`
with `-NoProfile -ExecutionPolicy Bypass`) or run `bbb-client-launch.ps1` from
PowerShell with any `bbb-client` CLI flags.

## Installing `bbb-server`

Both server supervisor files assume `bbb-server` at `/usr/local/bin/bbb-server`,
run as the unprivileged `bbb` user/group, store the save under a writable state
directory, bind the offline Minecraft listener to loopback (`127.0.0.1:25565`)
by default, and bind the MCP management endpoint to loopback (`127.0.0.1:25585`).
To expose `bbb-server` on a trusted LAN or VPN, change `--bind` to
`0.0.0.0:25565` and enable `--white-list true --enforce-whitelist true`. A
management endpoint on any non-loopback address is refused unless TLS and
OAuth 2.1 are configured; see `bbb-server --help`.

### Linux (systemd)

```sh
install -m 0755 /tmp/bbb-target-main/x86_64-unknown-linux-gnu/release/bbb-server /usr/local/bin/
install -m 0755 /tmp/bbb-target-main/x86_64-unknown-linux-gnu/release/bbb-admin /usr/local/bin/
install -d -o bbb -g bbb /var/lib/bbb
install -m 0644 packaging/bbb-server.service /etc/systemd/system/
systemctl daemon-reload
systemctl enable --now bbb-server
```

### macOS (launchd)

```sh
install -m 0755 /tmp/bbb-target-main/aarch64-apple-darwin/release/bbb-server /usr/local/bin/
install -m 0755 /tmp/bbb-target-main/aarch64-apple-darwin/release/bbb-admin /usr/local/bin/
install -d -o bbb -g bbb /usr/local/var/bbb
mkdir -p /usr/local/var/log
touch /usr/local/var/log/bbb-server.log
chown bbb:bbb /usr/local/var/log/bbb-server.log
cp packaging/com.bbb.server.plist /Library/LaunchDaemons/
launchctl load -w /Library/LaunchDaemons/com.bbb.server.plist
```

## Packaging the Web client gateway (`bbb-web-host`)

### 1. Build the threaded WASM static bundle

Build the release `wasm32-unknown-unknown` bundle (`index.html`, worker glue,
and `pkg/bbb_client_bg.wasm`) into an external staging directory:

```sh
cargo dev build-web --release --out-dir /tmp/bbb-web-dist
```

`bbb-web-host` also requires Minecraft Java 26.1 assets via `--vanilla-root` (or `BBB_VANILLA_ROOT` / the managed Vanilla cache) so it can assemble and cache `core-26.1.zip` under `--cache-dir` on startup. It accepts any of three directory layouts:

1. Prepared Vanilla workspace (`sources/26.1` + `downloads/26.1`, produced by `cargo dev vanilla prepare 26.1`).
2. Downloaded client JAR layout (`downloads/26.1/{client-26.1.jar, client.jar, 26.1.jar}` + `26.1.json` + `assets/`).
3. Official Minecraft Java 26.1 `.minecraft` directory (`versions/26.1/{26.1.jar, client.jar}` + `26.1.json` + `assets/`).

### 2. Install the loopback gateway service

Both `packaging/bbb-web-host.service` and `packaging/com.bbb.web-host.plist`
bind `bbb-web-host` on loopback (`127.0.0.1:8080`), forward `/ws` to the local
`bbb-server` (`127.0.0.1:25565`; add `--allow-server <host:port>` for extra
permitted upstreams), keep the WASM bundle and Vanilla directory read-only, and
leave `--mcp-bind` / `--control` disabled in production.

#### Linux (systemd)

```sh
install -m 0755 /tmp/bbb-target-main/x86_64-unknown-linux-gnu/release/bbb-web-host /usr/local/bin/
install -d -o root -g root /var/lib/bbb/web-dist /var/lib/bbb/vanilla
cp -R /tmp/bbb-web-dist/. /var/lib/bbb/web-dist/
install -d -o bbb -g bbb /var/cache/bbb-web
install -m 0644 packaging/bbb-web-host.service /etc/systemd/system/
systemctl daemon-reload
systemctl enable --now bbb-web-host
```

#### macOS (launchd)

```sh
install -m 0755 /tmp/bbb-target-main/aarch64-apple-darwin/release/bbb-web-host /usr/local/bin/
install -d -o root -g wheel /usr/local/var/bbb/web-dist /usr/local/var/bbb/vanilla
cp -R /tmp/bbb-web-dist/. /usr/local/var/bbb/web-dist/
install -d -o bbb -g bbb /usr/local/var/cache/bbb-web
mkdir -p /usr/local/var/log
touch /usr/local/var/log/bbb-web-host.log
chown bbb:bbb /usr/local/var/log/bbb-web-host.log
cp packaging/com.bbb.web-host.plist /Library/LaunchDaemons/
launchctl load -w /Library/LaunchDaemons/com.bbb.web-host.plist
```

### 3. Terminate HTTPS and enforce COOP/COEP at a reverse proxy

Outside `http://localhost` / `http://127.0.0.1`, browsers only expose
`SharedArrayBuffer` (required by the threaded WASM client) and WebGPU over an
HTTPS secure context with cross-origin isolation headers on the top-level
document and subresources:

- `Cross-Origin-Opener-Policy: same-origin`
- `Cross-Origin-Embedder-Policy: require-corp`
- `Cross-Origin-Resource-Policy: same-origin`

`bbb-web-host` already emits those three headers on HTTP responses, but public
deployments should terminate TLS in front of `127.0.0.1:8080`, preserve HTTP/1.1
`Upgrade: websocket` on `/ws`, enforce the three isolation headers at the edge,
and reject `/control-ws` (`403`):

- **Caddy**: copy `packaging/Caddyfile` to `/etc/caddy/Caddyfile`, replace
  `play.example.com` with your domain, and reload `caddy`.
- **Nginx**: copy `packaging/nginx-bbb-web.conf` into `/etc/nginx/conf.d/`,
  configure your certificate paths and `server_name`, and reload `nginx`.

## Container image (GHCR / Docker)

`packaging/Dockerfile` builds a non-root (`bbb` user) `linux/amd64` runtime image containing `/usr/local/bin/bbb-server`, `/usr/local/bin/bbb-admin`, `/usr/local/bin/bbb-web-host`, and the prebuilt WASM bundle in `/opt/bbb/web-dist` (with `BBB_WEB_OUT_DIR=/opt/bbb/web-dist` and `BBB_VANILLA_ROOT=/var/lib/bbb/vanilla`).

- **Default entrypoint (`CMD`)**: starts `bbb-server --bind 0.0.0.0:25565 --world /var/lib/bbb/world --admin-dir /var/lib/bbb/admin` (mount a persistent volume at `/var/lib/bbb`).
- **Running `bbb-web-host` from the same image**: override the command with `bbb-web-host --bind 0.0.0.0:8080 --vanilla-root /vanilla --cache-dir /var/cache/bbb-web --server <server:25565>` and bind-mount a read-only 26.1 asset directory (`.minecraft` or managed Vanilla workspace) at `/vanilla`.
- **Published tags**: [`.github/workflows/container.yml`](../.github/workflows/container.yml) publishes `ghcr.io/recursion128/bbb` on `main` pushes (`latest`, `sha-<commit>`) and `v*` tags (`<major>.<minor>.<patch>`, `<major>.<minor>`).

## Automated CI release archives

Pushing a `v*` tag or running the GitHub Actions workflow manually with
`build_release_artifacts: true` in [`.github/workflows/ci.yml`](../.github/workflows/ci.yml)
builds and uploads release archives (plus `.sha256` digests) via `actions/upload-artifact@v4`:

- `bbb-client-x86_64-unknown-linux-gnu.tar.gz` — Linux desktop binary, `bbb-client-launch.sh`, `bbb-client.desktop`, README, and licenses
- `bbb-client-x86_64-pc-windows-msvc.zip` — Windows desktop binary, `bbb-client-launch.ps1`, `bbb-client-launch.cmd`, README, and licenses
- `bbb-server-x86_64-unknown-linux-gnu.tar.gz` — Linux `bbb-server`, `bbb-admin`, `bbb-web-host`, `web-dist/`, systemd/proxy configs, README, and licenses
- `bbb-server-aarch64-apple-darwin.tar.gz` — macOS `bbb-server`, `bbb-admin`, `bbb-web-host`, `web-dist/`, launchd/proxy configs, README, and licenses

On `v*` tags — or on manual dispatch with both `build_release_artifacts: true` and `publish_github_release: true` — the `publish-release` job assembles `SHA256SUMS.txt` and publishes the archives to a GitHub Release (versioned tag release for `v*`, or the rolling `nightly` prerelease on manual dispatch).

## Stopping `bbb-server`

Both server supervisors stop `bbb-server` with `SIGTERM`. The server treats it
as a graceful shutdown: it logs `server.signalled` naming the signal, saves the
world, waits out its connection tasks and logs `server.stopped` with what it
served. Give it room to finish — `TimeoutStopSec` and `ExitTimeOut` are set to
60 seconds for that reason, well above `--shutdown-timeout-seconds`.

A save can also be requested without stopping, through the management endpoint:

```sh
bbb-admin call server.save
```

## Reading the `bbb-server` log

The server writes JSON to stderr, one record per line, which both supervisors
collect. The records a host operator cares about:

| Event | What it means |
| --- | --- |
| `server.ready` | listening; carries the address and protocol version |
| `server.signalled` | a stop was asked for, and by which signal |
| `server.stopped` | the clean exit, with connection counters |
| `server.failed` | the process could not do its job; exits non-zero |
| `server.panicked` | a panic, with its message and location |
| `connection.rejected` | a connection past `--max-connections` |
| `world.tick_health` | one hundred-tick window of tick cost and queue depth |

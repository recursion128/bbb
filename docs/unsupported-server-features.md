# 服务端功能差异

本账本仅登记 `bbb` dedicated server 与 Minecraft Java 26.1 服务端之间的未关闭功能缺口（active / 带 `Reopen when ...` 的 deferred）以及 `# Not needed` 范围裁决；不保留完成历史，功能落地时直接删除对应条目，历史由 Git 维护。

维护规则见[文档工作流](wiki/development/documentation-workflow.md#ledger-与接手信息)。

# Todo

以下只登记主分支生产路径中玩家/存档可观察的功能缺口，验收按 [AGENTS.md](../AGENTS.md) 的推进原则。

# Not needed

## Product and compatibility exclusions

- **Integrated server and singleplayer hosting are `not-needed`.** Vanilla 26.1
  `IntegratedServer` extends `MinecraftServer` but is owned by the client lifecycle,
  pause state and local world selection. BBB ships only an independent dedicated process.
  Reopen only if the product adds an in-process singleplayer host.

- **Online-mode authentication is `not-needed`.** Vanilla
  `ServerLoginPacketListenerImpl` can enter encryption/session verification, while the
  declared bbb client and server are offline-only (strictly following Minecraft Java 26.1
  wire packets, fields, phases, ordering and sync within that offline boundary).
  `/fetchprofile name|id` belongs to the same decision: `FetchProfileCommand.java:115-151`
  asks the `Services.java:24-29` profile resolver, which queries Mojang's profile service
  even in offline mode (`DedicatedServer.java:280` only toggles the offline-name fallback);
  bbb publishes `fetchprofile entity` alone. Reopen only if both products add online
  identity, encryption and secure-profile lifecycles.

- **The JVM-profiler commands `/jfr` and `/perf` are `not-needed`.** Both drive a
  Java Flight Recorder: `JfrCommand` calls `JvmProfiler.INSTANCE`, which vanilla
  itself registers conditionally on the JDK carrying `jdk.jfr`
  (Commands.java:259-261), and `PerfCommand` writes the same profiler's metrics
  directory. This server is a Rust process with no JVM to record, and its
  performance surface is the checked-in evidence and the management protocol.
  Reopen only if a profiling command is added whose subject is this server
  rather than a JVM.

- **The gametest command `/test` is `not-needed`.** Its 48 nodes drive
  `gametest.framework`, a developer harness that places structure templates and
  runs test instances from a registry; the `TestBlock` and `TestInstanceBlock` blocks exist only to
  author and run those tests. It is not part of what a player or an
  agent asks this server to do, and this repository's equivalent is its own gate
  chain. Reopen only if running Vanilla gametest saves becomes a declared
  capability.

- **Vanilla RCON, Query, stdin console and GUI are `not-needed`.** Vanilla
  `DedicatedServer.initServer` separately starts the console handler, `RconThread`,
  `QueryThreadGs4` and optional GUI; BBB uses MCP plus a same-protocol human CLI, structured
  logs and OS signals/supervisors. Reopen only if compatibility with a third-party Vanilla
  administration client becomes a product requirement.

- **Bukkit, Fabric and external runtime plugin APIs are `not-needed`.** Vanilla
  datapacks and [BBB's startup content extensions](wiki/architecture/server-boundaries.md#语义-owner-与依赖方向)
  provide the data-driven extension surfaces. Reopen only when an external plugin ABI
  or a specific third-party mod ecosystem enters the product scope.

- **Legacy pre-26.1 world upgrade (`DataFixerUpper`) is `not-needed`.** Vanilla 26.1
  `DataFixers` (`net.minecraft.util.datafix.DataFixers`) ships multi-version schema
  migration chains for worlds saved by older Minecraft versions. Per `AGENTS.md`,
  `bbb-server` targets official 26.1 saves and BBB's own round-trip saves only: when a
  non-26.1 `DataVersion` is encountered on load, `bbb-server` refuses to overwrite or
  regenerate it and instructs the operator to upgrade a backup copy with the official
  Minecraft Java 26.1 server's `--forceUpgrade` first. Reopen only if in-process
  multi-version legacy save migration enters product scope.

- **Windows release service packaging and any windowed server UI are `not-needed`.** The
  declared headless release platforms with supervisor packaging (`systemd`/`launchd`) and
  release acceptance are Linux x86_64 and macOS aarch64. `bbb-server` still builds and
  runs on Windows for local development, `cargo dev web-up`, and cross-platform E2E
  testing, without a Windows service package or windowed GUI. Reopen only if Windows
  dedicated-server release packaging or a server GUI enters the product scope.

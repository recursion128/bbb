# 服务端功能差异

本账本仅登记 `bbb` 独立服务端与 Minecraft Java 26.1 服务端之间的未关闭功能缺口（active / 带 `Reopen when ...` 的 deferred）以及 `# Not needed` 范围裁决；不保留完成历史，功能落地时直接删除对应条目，历史由 Git 维护。

维护规则见[文档工作流](wiki/development/documentation-workflow.md#ledger-与接手信息)。

# Todo

以下只登记主分支生产路径中玩家/存档可观察的功能缺口，验收按 [`AGENTS.md`](../AGENTS.md) 的推进原则。

# Not needed

## 产品边界与兼容性排除项

- **内置单人服务端与单人主机为 `not-needed`。** Vanilla 26.1 `IntegratedServer` 继承自 `MinecraftServer`，但由客户端生命周期、暂停状态与本地选世界流程托管。BBB 仅交付独立专用进程。重开条件：仅当产品新增进程内单人主机时重开。

- **正版在线认证模式为 `not-needed`。** Vanilla `ServerLoginPacketListenerImpl` 可进入加密与会话校验，而声明的 `bbb` 客户端与服务端均为仅离线模式（在仅离线边界内严格兼容 Minecraft Java 26.1 线协议包、字段、阶段、顺序与同步）。`/fetchprofile name|id` 归属同一裁决：`FetchProfileCommand.java:115-151` 查询 `Services.java:24-29` 档案解析器，即便在离线模式下仍访问 Mojang 档案服务（`DedicatedServer.java:280` 仅控制离线名称回退）；`bbb` 仅公开 `fetchprofile entity`。重开条件：仅当双端同时新增在线身份、加密传输与安全档案生命周期时重开。

- **JVM 性能分析命令 `/jfr` 与 `/perf` 为 `not-needed`。** 两者均驱动 Java Flight Recorder：`JfrCommand` 调用 `JvmProfiler.INSTANCE`（Vanilla 自身在 `Commands.java:259-261` 也仅当 JDK 带有 `jdk.jfr` 时条件注册），`PerfCommand` 写入同一分析器的指标目录。本服务端是无 JVM 的 Rust 进程，其性能观测面为仓库内基准与 MCP 管理协议。重开条件：仅当新增面向本 Rust 服务端而非 JVM 的分析命令时重开。

- **GameTest 命令 `/test` 为 `not-needed`。** 其 48 个节点驱动 `gametest.framework` 开发测试框架（从注册表放置结构模板并执行测试实例）；`TestBlock` 与 `TestInstanceBlock` 仅用于编写与运行此类测试，不属于玩家或代理对服务端的操作面，本仓库的等价机制为自有门禁体系。重开条件：仅当运行 Vanilla gametest 存档成为声明能力时重开。

- **Vanilla RCON、Query、stdin 交互控制台与窗口 GUI 为 `not-needed`。** Vanilla `DedicatedServer.initServer` 分别启动控制台处理器、`RconThread`、`QueryThreadGs4` 与可选 GUI；BBB 使用 MCP 协议、同协议人工运维 CLI（`bbb-admin`）、结构化日志与系统信号/守护进程托管。重开条件：仅当兼容第三方 Vanilla 管理客户端成为产品需求时重开。

- **Bukkit、Fabric 与外部运行时插件 ABI 为 `not-needed`。** 数据驱动扩展由 Vanilla 数据包与[BBB 启动内容扩展](wiki/architecture/server-boundaries.md#语义-owner-与依赖方向)提供。重开条件：仅当外部插件 ABI 或特定第三方模组生态纳入产品范围时重开。

- **26.1 之前旧版本世界原地升级（`DataFixerUpper`）为 `not-needed`。** Vanilla 26.1 `DataFixers`（`net.minecraft.util.datafix.DataFixers`）内置跨版本存档结构迁移链。按 `AGENTS.md` 边界，`bbb-server` 仅面向官方 26.1 存档与 BBB 自身往返存档：加载时遇到非 26.1 `DataVersion` 会拒绝覆盖或重新生成，并提示运维先使用官方 Minecraft Java 26.1 服务端的 `--forceUpgrade` 升级副本。重开条件：仅当进程内多版本旧存档迁移纳入产品范围时重开。

- **Windows 正式发布服务托管包与窗口化服务端 UI 为 `not-needed`。** 声明带守护进程服务包（`systemd`/`launchd`）与发布验收的无头发布平台为 Linux x86_64 与 macOS aarch64。`bbb-server` 仍支持在 Windows 本地编译运行、`cargo dev web-up` 与跨平台 E2E 测试，但不提供 Windows 服务包或窗口化 GUI。重开条件：仅当 Windows 独立服务端发布服务包或服务端 GUI 纳入产品范围时重开。

# bbb

`bbb` 是 Minecraft Java 26.1 的仓库原生外部多人客户端、WebGPU 浏览器客户端与仅离线模式无头独立服务端，拥有独立的协议、客户端/服务端世界、渲染器、控制面、MCP 管理与运行时边界，不包装官方 JAR、Bevy、Azalea 或其它客户端/服务端技术栈。

桌面与 Web 平台指南见 [Windows 客户端](docs/wiki/development/windows-client.md)、[macOS 与 Linux 开发](docs/wiki/development/macos-and-linux.md) 与 [Web 客户端与网关](docs/wiki/development/web-client.md)。

## 前置工具与系统依赖

Windows、macOS 与 Linux 共用统一的 `cargo dev`（`crates/bbb-xtask`）跨平台入口。开始前需准备以下工具链与系统依赖：

- **Nightly Rust 工具链**：安装 `rustup`，仓库由 [`rust-toolchain.toml`](rust-toolchain.toml) 固定 Nightly Rust（`nightly-2026-08-10`，含 `rustfmt`、`clippy`、`rust-src` 与 `wasm32-unknown-unknown` 目标）。
- **`uv` 与 `Git`**：安装 `uv 0.12.3`（用于管理锁定的 CPython `3.12.13` 与脚本依赖）与 `git`。
- **`Java 25`（完整游玩 / Vanilla 对账必需）**：完整 Vanilla 26.1 源码反编译、生成事实对账、门禁核验以及准备客户端运行资产需要 `Java 25`（或使用本机已有的官方 26.1 `.minecraft` 安装）；暂不准备 Vanilla 环境时，可对 `bootstrap`、`lint`、`docs-lint`、`gate` 传 `--skip-vanilla` 运行便携模式。
- **平台系统依赖**：
  - **Linux**：Wayland 桌面会话、Vulkan 驱动/加载器（如 `mesa-vulkan-drivers`）与 OpenAL（`libopenal1`）。
  - **Windows**：Visual Studio 2022 C++ Build Tools（MSVC x64 工具链与 Windows SDK；原生渲染默认使用 Direct3D 12）。
  - **macOS**：Xcode Command Line Tools（Cocoa / Metal）。

## 玩家上手路径

### 1. 准备资源与环境（游玩 vs `--skip-vanilla`）

- **真实游玩桌面客户端或 Web 端**：必须拥有 Vanilla 26.1 资源。在仓库根目录运行完整初始化（需要 `Java 25` 与网络，约耗时数分钟下载并反编译 26.1）：
  ```sh
  cargo dev bootstrap
  ```
  如果本机已有官方启动器安装的 Minecraft Java 26.1（`.minecraft`）：桌面端 `bbb-client` 会自动探测默认 `.minecraft` 目录（Windows `%APPDATA%\.minecraft`、macOS `~/Library/Application Support/minecraft`、Linux `~/.minecraft`）；Web 端可通过 `--vanilla-root <path-to-.minecraft>` 或环境变量 `BBB_VANILLA_ROOT` 指定该目录。
- **仅做离线静态检查 / 无资产开发**：若暂不游玩且未安装 `Java 25`，可运行 `cargo dev bootstrap --skip-vanilla` 只准备 Python 与 Rust 工具链。该模式**不能**启动带 26.1 资源的桌面或 Web 客户端（仅可跑服务端与离线测试）。

### 2. 磁盘占用与首次编译预期

- **磁盘占用**：Vanilla 26.1 缓存（JAR、资产与反编译源码）约 `0.7 GiB`；Rust 构建缓存（`bbb-target-main`、`bbb-target-wasm`）随构建目标增长，单个开发/测试目标通常占数 GiB 至十数 GiB，全量多目标检查与 WASM 发布缓存合计可达 20–30+ GiB。可通过 `BBB_TARGET_ROOT` 将编译缓存放到大容量磁盘，用 `cargo dev size` / `cargo dev prune` 查看与清理。
- **首次编译耗时**：首次构建桌面端、服务端或 Web WASM 需编译全部工作区与图形/音频依赖，通常需要数分钟至十余分钟；后续增量编译仅重编改动 crate。

### 3. 启动服务器与客户端（第一次进服）

在第一个终端启动本地离线服务器，`--world` 指向用于保存世界的目录，空目录会创建新世界。导入现有世界时先备份整个世界目录；如果世界曾由旧版本生成，先用官方 Java 26.1 服务端的 `--forceUpgrade` 升级副本再交给 BBB（仅在 26.1 打开过仍可能留下未加载的旧版区块），导入和版本规则见[服务端存档说明](docs/wiki/mechanisms/persistence/server-save-and-reload.md#判断与处理顺序)。

```sh
cargo dev run -p bbb-server --bin bbb-server -- --bind 127.0.0.1:25565 --world ../bbb-world
```

看到 `server.ready` 后，在第二个终端启动桌面客户端（另一个玩家同时连接时请使用不同的用户名）：

```sh
cargo dev client --server localhost:25565 --username Alex
```

`cargo dev client [--release] [--] [args...]` 会自动构建并在检测到已准备好的缓存时注入 `BBB_VANILLA_ROOT`，再把其余参数透传给 `bbb-client`。桌面端（Windows `x86_64-pc-windows-msvc`、Linux `x86_64-unknown-linux-gnu`）与服务端/Web 网关的发布打包配方与启动器见 [`packaging/README.md`](packaging/README.md)。

浏览器玩家也可以用下面的命令单独启动一套本地服务器和 Web 网关（先关闭上面占用 25565 端口的服务器，再打开终端打印的地址；**首次执行 `cargo dev web-up` 或 `cargo dev build-web` 时，若环境中缺少匹配版本的 `wasm-bindgen`，会自动通过 `cargo install wasm-bindgen-cli --locked` 编译安装该 CLI，首次耗时较长**）：

```sh
cargo dev web-up --world ../bbb-world
```

桌面与浏览器首次收到服务器资源包时都会显示服务器说明，可选择接受或拒绝。自动化客户端可显式传入 `--server-resource-pack-policy accept` 或 `decline`，浏览器使用 URL 参数 `server_resource_pack_policy`。

想用 MCP 查方块、勘察地形或批量建造独立存档，可从[本地建图流程](docs/wiki/mechanisms/control/server-mcp-administration.md#用-mcp-建图)开始。

## 开发与验证

`cargo dev gate` 运行里程碑门禁。日常验证命令和适用范围见[开发流程与测试](docs/wiki/development/testing-and-gates.md)。

`cargo dev bootstrap` 安装仓库固定的 `uv` 托管 CPython、同步锁定依赖、拉取 Rust 依赖并准备 Vanilla 26.1 workspace；已有离线缓存时可加 `--offline`。只准备开发工具链而暂不下载 Vanilla 时使用 `cargo dev bootstrap --skip-vanilla`。

工作区共 24 个 crate，生产依赖有向无环图（DAG）见[总体架构](docs/wiki/architecture/overview.md#workspace-24-crate-生产依赖有向无环图dag)，逐 crate 的语义归属见[仓库地图](docs/wiki/development/repository-map.md)。

文档从 [`docs/README.md`](docs/README.md) 进入；长期机制与架构见 [`docs/wiki/README.md`](docs/wiki/README.md)，命令行与查询参数见[CLI 参考](docs/wiki/development/cli-reference.md)，安全模型与生产部署见[安全与部署指南](docs/wiki/development/security-and-deployment.md)，推进原则与协作见 [`AGENTS.md`](AGENTS.md)，客户端/服务端功能差异分别见 [`docs/unsupported-client-features.md`](docs/unsupported-client-features.md) 与 [`docs/unsupported-server-features.md`](docs/unsupported-server-features.md)。

安全漏洞披露见 [`SECURITY.md`](SECURITY.md)。
本项目采用 [MIT](LICENSE-MIT) 或 [Apache-2.0](LICENSE-APACHE) 双许可。

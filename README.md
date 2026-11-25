# bbb

`bbb` 是 Minecraft Java **26.1** 的仓库原生外部多人客户端、WebGPU 浏览器客户端与仅离线模式无头独立服务端，拥有独立的协议、双端世界、渲染器、控制面、MCP 管理与运行时边界，不包装官方 JAR、Bevy、Azalea 或其它客户端/服务端技术栈。

[项目网站与在线文档](https://recursion128.github.io/bbb/) · [GitHub Releases](https://github.com/recursion128/bbb/releases) · [GHCR 容器镜像](https://github.com/recursion128/bbb/pkgs/container/bbb) · [文档总入口](docs/README.md) · [CLI 参数参考](docs/wiki/development/cli-reference.md)

| 交付形态 | 核心二进制 / 产物 | 目标平台与运行时栈 | 详细指南 |
| --- | --- | --- | --- |
| **桌面多人客户端** | `bbb-client` | Windows x64（Win32 / Direct3D 12）、Linux x64（Wayland / Vulkan / OpenAL）；源码兼容 macOS | [Windows 客户端](docs/wiki/development/windows-client.md) · [macOS 与 Linux](docs/wiki/development/macos-and-linux.md) |
| **WebGPU 浏览器客户端** | `bbb-web-host` + `web-dist/`（多线程 WASM） | 支持 WebGPU 与 `SharedArrayBuffer` 的桌面浏览器（Chrome / Chromium 等） | [Web 客户端与网关](docs/wiki/development/web-client.md) |
| **无头独立服务端与管理** | `bbb-server`、`bbb-admin`（MCP CLI） | Linux x64 / macOS Apple Silicon 原生包、Linux x64 容器镜像（Windows 支持源码开发与 E2E） | [服务端边界](docs/wiki/architecture/server-boundaries.md) · [安全与部署](docs/wiki/development/security-and-deployment.md) |

## 快速开始

已有 Minecraft Java 26.1 离线服地址时，可直接用桌面客户端加入；想在本机开服，先启动服务端，再用桌面端或浏览器连接。以下步骤直接使用预编译包与容器镜像。

### 1. 用桌面客户端加入服务器

先用官方启动器安装并启动一次 **Minecraft Java 26.1**，让启动器下载完整资源。下载并解压对应平台的客户端包，在解压目录运行启动命令（连接已有服务器时，将 `127.0.0.1:25565` 替换为目标地址）：

| 平台 | 预编译包 | 启动命令 |
| --- | --- | --- |
| Windows x64（PowerShell） | [Nightly ZIP](https://github.com/recursion128/bbb/releases/download/nightly/bbb-client-x86_64-pc-windows-msvc.zip) | `.\bbb-client-launch.cmd --server 127.0.0.1:25565 --username Alex` |
| Linux x64 | [Nightly tar.gz](https://github.com/recursion128/bbb/releases/download/nightly/bbb-client-x86_64-unknown-linux-gnu.tar.gz) | `./bbb-client-launch.sh --server 127.0.0.1:25565 --username Alex` |

- 客户端会自动探测默认 `.minecraft` 目录与本机托管缓存；资源位于自定义路径时，设置 `BBB_VANILLA_ROOT` 或传入 `--vanilla-root <路径>`。
- Linux 桌面端运行时需要 Wayland 会话、Vulkan 驱动与 OpenAL（`libopenal1`）。完整启动器变量与系统安装见[桌面打包指南](packaging/README.md#packaging-the-desktop-client-bbb-client)。

### 2. 在本机开服

选择 Docker 或原生包。终端输出 `server.ready` 后即可连接 `127.0.0.1:25565`。BBB 服务端以仅离线模式运行，默认同时在本机 `127.0.0.1:25585/mcp` 提供 MCP 管理接口（可用 `bbb-admin` 运维）。

**Docker（Linux x64 镜像）**：首次运行先创建隔离网络（只需执行一次），使用 `bbb-data` 卷保存世界与管理数据，方便后续同网段挂载 Web 网关：

```sh
docker network create bbb-quickstart
docker run --rm --name bbb-server --network bbb-quickstart \
  -p 127.0.0.1:25565:25565 \
  -v bbb-data:/var/lib/bbb \
  ghcr.io/recursion128/bbb:sha-85fd2fc
```

**原生包**：下载 [Linux x64](https://github.com/recursion128/bbb/releases/download/nightly/bbb-server-x86_64-unknown-linux-gnu.tar.gz) 或 [macOS Apple Silicon](https://github.com/recursion128/bbb/releases/download/nightly/bbb-server-aarch64-apple-darwin.tar.gz) 服务端包，解压后运行（世界保存在 `./world`）：

```sh
./bbb-server --bind 127.0.0.1:25565 --world ./world
```

### 3. 用浏览器连接

已有网关时，直接在支持 WebGPU 与 `SharedArrayBuffer` 的桌面浏览器打开网关地址。自行部署时，保持上一步服务端运行，在另一终端启动 `bbb-web-host`。下面命令中的 `/path/to/vanilla` 请替换为实际绝对路径，支持官方 Minecraft Java 26.1 的 `.minecraft` 目录或 `cargo dev vanilla prepare 26.1` 托管缓存目录。

**原生包**（在服务端包解压目录运行；若本机已有托管缓存或已设 `BBB_VANILLA_ROOT`，可省略 `--vanilla-root`）：

```sh
./bbb-web-host --vanilla-root /path/to/vanilla --wasm-dir ./web-dist --server 127.0.0.1:25565
```

**Docker**（复用上一步的镜像与 `bbb-quickstart` 网络，通过容器名 `bbb-server` 连接游戏服务端）：

```sh
docker run --rm --network bbb-quickstart \
  -p 127.0.0.1:8080:8080 \
  --mount "type=bind,source=/path/to/vanilla,target=/vanilla,readonly" \
  ghcr.io/recursion128/bbb:sha-85fd2fc \
  bbb-web-host --bind 0.0.0.0:8080 --vanilla-root /vanilla --cache-dir /var/cache/bbb-web --server bbb-server:25565
```

启动后在浏览器访问 [http://127.0.0.1:8080/](http://127.0.0.1:8080/)。公网或远程联机需前置配置跨源隔离（COOP/COEP/CORP）的 HTTPS 反向代理，详见 [Web 客户端与网关](docs/wiki/development/web-client.md)与[安全与部署指南](docs/wiki/development/security-and-deployment.md)。

## 发布渠道与版本

BBB 使用独立的发布版本号，首个正式发布计划为 **BBB v0.1.0（Minecraft Java 26.1）**。通过[发布门禁](docs/wiki/development/testing-and-gates.md#发布)后，正式包将在 [GitHub Releases](https://github.com/recursion128/bbb/releases) 发布。

以下为 2026-11-25 核对的预览构建，最新版本以对应发布页为准：

| 渠道 | 构建版本 | 包含内容 |
| --- | --- | --- |
| [GitHub Releases Nightly](https://github.com/recursion128/bbb/releases/tag/nightly) | `nightly`（源码提交 `6567d9f`） | Windows / Linux x64 桌面包；Linux x64 / macOS Apple Silicon 服务端包 |
| [GHCR 容器镜像](https://github.com/recursion128/bbb/pkgs/container/bbb) | `latest`（当时指向 `sha-85fd2fc`） | Linux x64 运行时镜像 |

原生服务端包与容器镜像均内置 `bbb-server`、`bbb-admin`、`bbb-web-host` 与 `web-dist/`，浏览器客户端与所在包或镜像版本一致。预览镜像可用 `sha-<提交>` 固定；正式 Git 标签 `v0.1.0` 将对应镜像标签 `0.1.0` 与 `0.1`。参见[打包指南](packaging/README.md)与[镜像发布规则](.github/workflows/container.yml)。

## 前置工具与系统依赖

从源码构建与开发时，Windows、macOS 与 Linux 统一使用 `cargo dev`（`crates/bbb-xtask`）入口。开始前需准备：

- **Nightly Rust 工具链**：安装 `rustup`，仓库由 [`rust-toolchain.toml`](rust-toolchain.toml) 固定 `nightly-2026-08-10`（含 `rustfmt`、`clippy`、`rust-src` 与 `wasm32-unknown-unknown` 目标）。
- **`uv` 与 `Git`**：安装 `uv 0.12.3`（管理锁定的 CPython `3.12.13` 与脚本依赖）与 `git`。
- **`Java 25`**（完整游玩与 Vanilla 对账必需）：用于准备 Vanilla 26.1 反编译源码、资产缓存与完整门禁核验；仅做离线静态检查或服务端开发时，可通过 `--skip-vanilla` 跳过。
- **平台系统依赖**：
  - **Linux**：Wayland 桌面会话、Vulkan 驱动/加载器（如 `mesa-vulkan-drivers`）与 OpenAL（`libopenal1`）。
  - **Windows**：Visual Studio 2022 C++ Build Tools（MSVC x64 工具链与 Windows SDK；原生渲染仅编译 Direct3D 12 后端）。
  - **macOS**：Xcode Command Line Tools（Cocoa / Metal）。

## 从源码启动

### 1. 初始化环境与资源

- **完整开发与游玩**（安装锁定 CPython、同步依赖、准备 Vanilla 26.1 工作区，需 `Java 25` 与网络，已有离线缓存可加 `--offline`）：
  ```sh
  cargo dev bootstrap
  ```
  如本机已用官方启动器安装过 Minecraft Java 26.1：桌面端会自动探测默认 `.minecraft`（Windows `%APPDATA%\.minecraft`、macOS `~/Library/Application Support/minecraft`、Linux `~/.minecraft`）；Web 网关可通过 `--vanilla-root <path-to-.minecraft>` 或 `BBB_VANILLA_ROOT` 指定该目录。
- **便携 / 无资产开发**（暂不安装 `Java 25`，仅准备 Rust 与 Python 工具链，适用于服务端开发及 `--skip-vanilla` 静态检查与离线测试，**无法**启动带 26.1 资源的客户端）：
  ```sh
  cargo dev bootstrap --skip-vanilla
  ```

### 2. 磁盘占用与首次编译预期

- **磁盘占用**：Vanilla 26.1 本地缓存（JAR、资产与反编译源码）约 `0.7 GiB`；Cargo 编译缓存默认隔离在仓库外（`bbb-target-main`、`bbb-target-wasm`），单个开发/测试目标占数 GiB 至十数 GiB，全量多目标检查与 WASM 发布缓存合计可达 20–30+ GiB。可通过 `BBB_TARGET_ROOT` 指定大容量磁盘目录，并用 `cargo dev size` / `cargo dev prune` 查看与清理。
- **首次编译耗时**：首次编译工作区与图形/音频依赖通常需数分钟至十余分钟，后续增量编译仅重编变更 crate。

### 3. 启动服务端、桌面端与浏览器端

在第一个终端启动本地离线服务端（`--world` 指向存档目录，空目录自动创建新世界）：

```sh
cargo dev run -p bbb-server --bin bbb-server -- --bind 127.0.0.1:25565 --world ../bbb-world
```

> 导入现有存档前请先完整备份；由 26.1 之前旧版本生成的世界须先用官方 Java 26.1 服务端的 `--forceUpgrade` 升级副本（仅在 26.1 打开过仍可能遗留未转换的旧区块），规则见[服务端保存与重载](docs/wiki/mechanisms/persistence/server-save-and-reload.md#判断与处理顺序)。

看到 `server.ready` 后，在第二个终端启动桌面客户端（多开联机请使用不同 `--username`）：

```sh
cargo dev client --server localhost:25565 --username Alex
```

`cargo dev client [--release] [--] [args...]` 会自动构建、在检测到已准备缓存时注入 `BBB_VANILLA_ROOT` 并透传其余参数给 `bbb-client`。

若使用浏览器游玩，先停止占用 `25565` 端口的服务端，再用一条命令同时拉起本地服务端与 Web 网关（打开终端打印的地址即可）：

```sh
cargo dev web-up --world ../bbb-world
```

- 首次运行 `cargo dev web-up` 或 `cargo build-web` 时，若环境中缺少匹配版本的 `wasm-bindgen`，会自动执行 `cargo install wasm-bindgen-cli --locked` 安装（首次耗时较长，后续复用缓存）。
- 首次收到服务端资源包时，桌面端与浏览器均会弹出说明供接受或拒绝；自动化场景可通过 `--server-resource-pack-policy accept|decline` 或 URL 参数 `server_resource_pack_policy` 显式指定。
- 使用 MCP 查方块、勘察地形或批量建造独立存档，见[用 MCP 建图](docs/wiki/development/cli-reference.md#用-mcp-建图cargo-dev-world-build)。

## 开发、架构与文档导航

里程碑验收运行 `cargo dev gate`（14 步完整门禁；无 Vanilla 环境可传 `--skip-vanilla`）。工作区共 24 个 crate，生产依赖关系见[总体架构 DAG](docs/wiki/architecture/overview.md#workspace-24-crate-生产依赖有向无环图dag)，逐 crate 职责见[仓库地图](docs/wiki/development/repository-map.md)。

| 主题 | 文档入口 |
| --- | --- |
| 文档导航与中文机制 Wiki | [`docs/README.md`](docs/README.md) · [`docs/wiki/README.md`](docs/wiki/README.md) |
| 命令行与 Web 查询参数 | [CLI 与查询参数参考](docs/wiki/development/cli-reference.md) |
| 打包、启动器、托管与反代 | [`packaging/README.md`](packaging/README.md) |
| 安全边界与生产部署 | [安全边界与生产部署指南](docs/wiki/development/security-and-deployment.md) |
| 开发流程、测试与发布门禁 | [开发流程、测试与发布](docs/wiki/development/testing-and-gates.md) · [构建与缓存](docs/wiki/development/build-and-cache.md) |
| 推进原则与协作规范 | [`AGENTS.md`](AGENTS.md) |
| 功能差异与扩展账本 | [客户端差异](docs/unsupported-client-features.md) · [服务端差异](docs/unsupported-server-features.md) · [客户端扩展](docs/client-extensions.md) |

安全漏洞披露见 [`SECURITY.md`](SECURITY.md)。本项目采用 [MIT](LICENSE-MIT) 或 [Apache-2.0](LICENSE-APACHE) 双许可。

# Windows 客户端

`bbb-client` 在 Windows x86_64 上使用 `winit` 原生窗口、`wgpu` 的 Direct3D 12 后端（Windows 构建在 `crates/bbb-client-render/Cargo.toml` 中仅编译 `dx12` 后端，不含 `vulkan`；无显卡环境可通过环境变量 `BBB_SOFTWARE_GPU=1` 启用 WARP 软件光栅适配器，E2E 门禁可传 `cargo dev e2e client --software-gpu`）、`arboard` 剪贴板
与官方 26.1 清单锁定的 OpenAL DLL（TrueType 字体由纯 Rust `ab_glyph` 跨平台统一光栅化）。客户端连接外部离线模式服务器；
启动参数与 Linux/macOS 客户端共用，不提供游戏内配置界面。

## 构建与启动

安装 Git、Rustup 与 Visual Studio 2022 C++ Build Tools（MSVC x64 工具链与 Windows SDK）。
仓库的 `rust-toolchain.toml` 固定 Rust 版本。通过统一的 `cargo dev`（`crates/bbb-xtask`）跨平台入口运行：

```powershell
cargo dev bootstrap
cargo dev client -- --vanilla-root D:/bbb-cache/vanilla --server localhost:25565 --username Player
```

`cargo dev client` 自动注入已准备的 `BBB_VANILLA_ROOT` 并透传全部 `bbb-client` 参数（传入 `--server <ADDR>` 即自动连接；需要优化构建时使用 `cargo dev client --release -- ...`）。`--vanilla-root` 可指向 `cargo dev vanilla prepare 26.1` 准备的工作区或官方 26.1 `.minecraft` 目录；未显式传入时按 `BBB_VANILLA_ROOT` → `BBB_VANILLA_DIR` → `--game-dir` → `XDG_CACHE_HOME/bbb/vanilla` → `%LOCALAPPDATA%\bbb\vanilla` → `%APPDATA%\.minecraft` 顺序自动探测。也可以从已有 Linux/macOS 工作区复制
`sources/26.1` 与 `downloads/26.1`；必须保留资产、清单、客户端 JAR 和依赖库中的
`natives-windows` 包。客户端启动不下载这些资源，DLL 提取仍核验清单中的文件大小和 SHA-1。资源目录必须放在仓库之外。

`cargo dev` 与 `cargo worker` 在 Windows、macOS、Linux 上提供完全一致的四组跨平台入口：

- **Rust 构建与测试**：`build`、`check`、`test`、`fast-test`、`bench`、`run`、`clippy`、`fmt`、`cargo`、`env`；
- **工具链、检查与门禁**：`bootstrap`、`vanilla`、`python`、`world-build`、`py-test`、`py-lint`、`doctor`、`generate --check`、`docs-lint`、`audit-p0`、`anchors`、`lint`、`cross-check`、`check-targets`、`e2e`、`gate`、`batch-gate`、`install-hooks`、`pre-commit`；
- **缓存清理与并行工作树**：`prune`、`size`、`clean-target`、`worker`（独立别名 `cargo worker`）；
- **桌面与 Web 客户端**：`client`、`build-web`（独立别名 `cargo build-web`）、`web-up`（独立别名 `cargo web-up`）。

构建、检查、测试与运行命令默认带 `--locked`。Windows 默认目标目录（target）是 `%TEMP%\bbb-target-main`（Linux/macOS 为
`/tmp/bbb-target-main`），显式 `CARGO_TARGET_DIR` 优先。发布（release）构建追加 `--release`。

## 平台行为与验证

BBB 私有配置存储使用 `%APPDATA%\bbb-client`；显式 `BBB_*_STORE` 路径优先。
`options.txt` 等 Vanilla 游戏文件仍位于 `--game-dir`。URI 与文件打开通过
`rundll32 url.dll,FileProtocolHandler`，文件路径先转 `file://` URI；不会通过命令行解释器拼接。
窗口使用 16px 标题栏图标和 256px 任务栏图标，平台身份报告 `win32`。
启用旁白且未指定 `--narrator-command` 时自动使用内置桌面操作系统语音合成（TTS）旁白（传入 `--narrator-command` 仍优先覆盖）。
**F3+S** 会将地形、GUI、字体、粒子及动态玩家皮肤/纹理图集转储为 PNG，写入 `<game-dir>/screenshots/debug`。

**F9** 开始/停止原生 MP4 视频录制，默认保存到 `--game-dir` 下的 `videos/`。
可用 `--record-video <path.mp4>` 启动即录制，`--record-fps` 与 `--record-bitrate`
控制采样帧率和码率。MP4 包含 H.264 视频和 AAC 48 kHz 双声道游戏音轨，保留游戏音量与空间定位，
不录入麦克风或其他应用的声音；控制端与 MCP 的录制启停与状态接口详见
[原生视频导出](../mechanisms/runtime/client-runtime-and-control.md#原生视频导出windows)。

必要的定向检查与本机 E2E：

```powershell
cargo dev check -p bbb-client --all-targets
cargo dev test -p bbb-client --lib platform::tests
cargo dev test -p bbb-client --lib window_icon::tests
cargo dev test -p bbb-client-assets native_library
cargo dev e2e client
```

无物理显卡的 Windows 云运行器可传入 `cargo dev e2e client --software-gpu`（或设 `BBB_SOFTWARE_GPU=1`），
通过 Direct3D 12 WARP 软光栅适配器完成进服、移动、挖放、背包交互、MCP 截图与两轮确定性回放校验。
真实主机交付仍需在物理 GPU/音频设备上检查字体、音频输出、鼠标抓取、剪贴板、输入法与全屏切换。

## Hyper-V GPU-PV 虚拟机注意事项

在 Hyper-V GPU 分区（GPU-PV）Windows 客体中运行或诊断 `bbb-client` 时，遵守以下通用约束：

- **驱动映射**：将宿主显卡驱动包原样复制到客体 `C:\Windows\System32\HostDriverStore\FileRepository\`，具体约定参见[微软 GPU 虚拟化文档](https://learn.microsoft.com/en-us/windows-hardware/drivers/display/gpu-paravirtualization)。
- **硬件后端确认**：宿主列出 `Get-VMPartitionableGpu` 或客体目录存在 DLL 不能代替实际适配器验证；运行前须确认 Direct3D 12 选中物理 GPU 分区适配器而非 Microsoft Basic Render Driver（WARP）等软件光栅路径。
- **网络与结果边界**：客体的 `127.0.0.1` 不指向宿主，连接宿主或远端测试服需使用虚拟交换机可达地址；GPU-PV 客体计时须与裸机 Windows 原生性能结果分开标注，不替代物理主机交付证据。

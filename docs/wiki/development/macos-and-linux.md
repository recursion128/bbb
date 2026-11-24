# macOS 与 Linux 开发

bbb 维护两条 POSIX 声明发布行：Linux x86_64 / Wayland / Vulkan 与 macOS aarch64 /
Cocoa / Metal。cross-check 只能提前发现类型错误；正式 platform receipt 必须由对应
主机的真实窗口、GPU、音频与输入链路产生。**两条 POSIX 行中只有 Linux 行进入交付判定**（ledger 的
`platform-macos-release` scope 裁决）：没有维护的 macOS runner，macOS 行不进入交付判定；
macOS 构建、cross-check 与本页的 macOS 合同保留，裁决被推翻且出现维护的 macOS runner 时重开。

Windows x86_64 原生客户端同样属于交付范围，与 macOS/Linux 共用统一的 `cargo dev` 开发与门禁入口，
平台特性与本机验收详见 [Windows 客户端](windows-client.md)；其自动化 host receipt（`platform-windows-release`）
与 macOS 行同按 ledger scope 裁决不进入自动化平台门禁。本页的 POSIX live 门禁合同适用于 macOS/Linux。
X11 代码保持可编译，声明 Linux 发布行只接受 Wayland，其 scope 裁决及重开条件以 ledger 为准。

## 外部工具

仓库不安装系统依赖。主机前置工具链（`rustup` / `rust-toolchain.toml`、`git`、`uv 0.12.3`、
`Java 25` 或便携模式 `--skip-vanilla`）及 Linux 运行时依赖见[仓库 README](../../../README.md#前置工具与系统依赖)。
`cargo dev bootstrap` 准备 uv-managed Python、Rust 依赖和 repo-managed Vanilla workspace；
日常命令统一经过 `cargo dev`（及 POSIX live 脚本复用的 `scripts/python`），不依赖系统全局 Python 包。

`scripts/tests/test_dev_chain_portability.py` 拒绝 POSIX live 门禁脚本中的 GNU-only 写法、`/bin/sh`
bashism 和表外下载/容器工具。macOS full/platform gate 负责 BSD userland 与 GUI 真路径，
Linux gate 负责 GNU/Wayland 真路径。

## 共用开发入口

macOS、Linux 与 Windows 完全共用同一组日常入口：

- `cargo dev fast-test` 或 `cargo dev test -p <crate> <filter>`：focused Rust 测试。
- `cargo dev doctor --profile portable`：按能力探测主机。
- `cargo dev py-lint` 与 `cargo dev py-test --suite ...`：Python 工具链检查与测试。
- `cargo dev gate`：完整 14 步门禁并写 `<git-common-dir>/bbb/last-gated`；里程碑结束或发布前运行（无 Vanilla 环境支持 `--skip-vanilla` 便携模式）。
- `cargo dev cross-check`：第二平台类型检查。
- `cargo dev docs-lint`：当前文档、ledger、生成事实与链接检查（便携模式传 `--skip-vanilla`）。
- `cargo dev vanilla prepare|verify|path|clean 26.1`：Vanilla workspace 管理入口。
- `cargo worker create|run|status|cleanup <name>`：并行工作树与暖池 target 租约管理。

target 缓存路径、worker lane 租约与禁止生成 repo-local `target/` 的规则见[构建与缓存](build-and-cache.md)。

## 客户端主机发布验收

`scripts/platform-acceptance-gate.sh` 只接受两条精确 POSIX host 行（其中只有 Linux 行进入交付，Windows 主机验收另走 [Windows 客户端](windows-client.md)）：

- Linux x86_64：Wayland/Vulkan，真实输入来自持久 `uinput` → libinput → sway →
  `wl_seat`，并验证 DPI/fullscreen 与 fcitx5 IME。
- macOS aarch64：Cocoa/Metal，真实输入来自获得 Accessibility 权限的 CoreGraphics；
  验证最终 client PID 的键盘、按钮、相对移动和 Command+V。音频必须通过 Vanilla
  manifest 锁定的 arm64 OpenAL 打开真实 CoreAudio device。

普通 synthetic playable 路径不能冒充平台输入证据。macOS 运行前必须解锁登录 GUI
session，并给调用终端或 agent 进程 Accessibility 权限。所有 client/server/tool 进程
进入独立 process group，并由 bounded cleanup 回收。

## 第二平台 cross-check

`scripts/cross_check.py` 在非 macOS 主机默认用 `cargo check --locked --target aarch64-apple-darwin`
（在 macOS 主机默认选用已安装的 `x86_64-unknown-linux-gnu` 或 `x86_64-pc-windows-msvc`，也可通过
`--target <triple>` 显式指定）检查可达的 workspace crate。只有错误明确来自跨 OS 缺失 C/汇编工具链的
已知 build script（当前为 `ring`）才归因跳过；其它失败都是类型错误。crate 依赖变化后脚本通过
`cargo metadata --filter-platform` 重新分类，不维护静态 crate 豁免清单。macOS 本机默认 target
检查不替代 Linux 发布行，反之亦然。full gate 每次都运行本步。

## macOS 系统调用归属

AppKit 调用收敛在 `bbb-client` 的 `platform_macos` 模块。`bbb-client` 的依赖闭包含需要
Apple 工具链的 build script，跨主机 cross-check 不编译这些调用，只有 macOS 主机检查覆盖它们。

`bbb_client::platform` 是 URI/路径打开和 BBB client-store 默认位置的唯一
平台分派 owner。`PlatformTarget` 区分 `Windows`、`MacOs` 与 `Unix`；新增平台差异必须进入该 owner，
不能在消费者中新增分散的 `cfg!` 阶梯。

## 原生平台能力边界

- **原生 MP4 视频录制仅支持 Windows**：`crates/bbb-client/src/recording.rs` 的 H.264 视频与
  AAC 双声道游戏音轨编码器依赖 Windows Media Foundation。在 macOS 与 Linux 上触发 `F9`、
  `--record-video` 或 `renderer.recording_start` 会立即失败并返回明确错误
  （`native MP4 recording currently requires Windows`，`RecordingState::Failed`），不会静默生成空文件。
- **跨平台截图、图集转储、旁白与会话回归**：`F2` 与 `renderer.screenshot` PNG 截图、`F3+S` 多图集转储（地形/GUI/字体/粒子/动态玩家图集写入 `<game-dir>/screenshots/debug`）、内置桌面 OS TTS 旁白（传入 `--narrator-command` 仍优先覆盖）以及 `scripts/client_replay.py`
  的确定性会话录制与回放在 Linux、macOS 与 Windows 上均完整可用。

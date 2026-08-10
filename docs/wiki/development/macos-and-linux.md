# macOS 与 Linux 开发

bbb 在 macOS 与 Linux 上开发并维护两条发布验收行：Linux x86_64 / Wayland / Vulkan
与 macOS aarch64 / Cocoa / Metal。跨平台构建检查可以提前发现另一平台的类型错误，但
正式 platform receipt 必须由对应主机的真实窗口、GPU、音频与输入链路产生。

**Windows 按用户裁决为非目标，平台面已整体删除**：`PlatformTarget::Windows`
及其 URI/路径打开与 `%APPDATA%` store 根、`WindowIconTarget::Windows` 与
`WindowExtWindows` 任务栏图标、`Window.getPlatform` 的 `win32` 标识都不再存在。
`bbb-native` 的 `main.rs` 用 `compile_error!` 直接拒绝 Windows target，所以这条裁决
由编译器执行而不是由文档承诺；`test_dev_chain_portability.py` 另外拒绝任何
`cfg(windows)` 分支或 Windows 专有 API 长回来——**这条守卫是必需的**：本仓库没有
任何主机编译 Windows target，长回来的分支不会被任何构建或测试看见。

唯一保留的 Windows 相关派生是 vanilla 的 options 事实而非平台支持：
`syncChunkWrites` 的默认值在 vanilla 里是 `Util.getPlatform() == Util.OS.WINDOWS`
（Options.java），生成的 catalog 继续读它，host 侧由 `platform::HOST_IS_WINDOWS`
恒为 false 提供。

X11 同为 `not-needed`，但处理不同：bbb 仍保留 winit 的 X11 分支（Linux 上真实可达、
window icon 走 X11 路径），只是不声明 X11 发布行。

## 外部工具

两个平台相同，都不由仓库安装：

| 工具 | 用途 | macOS 获取方式 |
| --- | --- | --- |
| `cargo` | 构建与测试；工具链版本由 `rust-toolchain.toml` 钉死 | `rustup` |
| `rustup` | 安装锁定 Rust 工具链与 macOS cross-check target | 官方 rustup installer |
| `git` | 版本控制与门禁 stamp | Xcode Command Line Tools |
| `rg` | docs lint 的模式检索 | `brew install ripgrep` |
| `uv` 0.12.3 | 安装锁定 Python、同步根环境并运行全部 Python 工具 | 安装官方 0.12.3 release |
| `java` 25 | 首次准备 Vanilla 26.1 workspace | 安装 JDK 25 并加入 `PATH` |

仓库不再要求系统 Python。首次 clone 后运行 `scripts/bootstrap`；它按根
`.python-version` 安装 uv-managed CPython 3.12.13，按 `uv.lock` 同步 `.venv`，拉取
锁定 Rust 依赖并准备 Vanilla workspace。日常入口只离线读取这些 cache；确实只需要
工具链时可先用 `scripts/bootstrap --skip-vanilla`，已有完整 cache 时可加 `--offline`。

`scripts/tests/test_dev_chain_portability.py` 守住这张表：它拒绝开发链路脚本里的
GNU-only 写法（`readlink -f`、`sha256sum`、`stat -c`、`sed -i`、`grep -P`、`nproc` 等）、
`/bin/sh` 脚本里的 bashism（macOS 的 `/bin/sh` 不是 bash），以及一组常被顺手引入的
表外工具（`curl`、`wget`、`jq`、`docker`）。结构测试让两个平台在进入昂贵 live gate 前
就能归因脚本移植错误，Mac full/platform gate 再覆盖真实 BSD userland 与 GUI 路径。

## 两个平台都跑的

- `scripts/cargo-dev.sh fast-test` / `test -p <crate> <filter>`：日常 focused 测试。
- `scripts/bootstrap`：新工作树的一次性解释器、依赖与 Vanilla workspace 初始化。
- `scripts/dev doctor --profile portable`：能力探测，不用 `uname` 猜行为。
- `scripts/dev lint python` / `scripts/dev test python --suite unit|integration|vanilla|all`。
- `scripts/cargo-dev.sh gate`：完整门禁，10 步，写 `.git/bbb/last-gated`。
- `scripts/cargo-dev.sh cross-check`：第二平台类型检查，见下节。
- `scripts/docs-lint.sh`：文档与账本 lint。
- `scripts/vanilla prepare|verify|path|clean 26.1`：同一锁定 Python 环境中的标准库实现。

target 目录约定（`/tmp/bbb-target-<name>`）在 macOS 上同样成立。

## 声明发布主机才跑的

`scripts/platform-acceptance-gate.sh` 按 `uname -s/-m` 选择且只接受两条精确行：

- Linux x86_64 使用 Wayland/Vulkan，输入 receipt 来自持久
  `uinput` → libinput → sway → `wl_seat`，并额外验证 compositor DPI/fullscreen 与 fcitx5 IME。
- macOS aarch64 使用 Cocoa/Metal，输入 receipt 来自受 Accessibility 授权的 CoreGraphics；
  按最终 client PID 聚焦 Accessibility window，验证服务端确认的键盘/按钮、锁定光标相对
  转向和 Command+V。音频必须从 Vanilla manifest 锁定的 arm64 OpenAL dylib 打开真实
  CoreAudio device，并具有 native capacity 与成功 command submission receipt。

其它 OS/arch 会在 build 前按名称拒绝。`scripts/input --backend linux-release` 与
`scripts/input --backend macos-quartz` 分别拥有两条输入路径；Mac gate 的调用进程必须先在
System Settings → Privacy & Security → Accessibility 获得授权。

`scripts/python python scripts/gpu_duration_capture.py preflight|capture` 仍只在 Linux 运行，
因为它需要 tracefs 与 perf；`summarize` 只读既有 capture，与主机无关。

`scripts/playable-gate.sh` 与 `scripts/lifecycle-soak-gate.sh` 在两平台都可 live：Linux 要求
`DISPLAY`/`WAYLAND_DISPLAY`，macOS 使用当前登录 GUI session。playable 的普通 synthetic
路径不冒充平台输入证据；只有显式 backend 才生成可归因 input receipt。

`scripts/macos_native_retina.py detect|run` 只在 macOS 登录 GUI session 执行，将内建
显示器的 panel-native、逻辑 bounds、scale 和当前 CoreGraphics backing 分开记录；性能
合同以两侧 F2 截图实测的 drawable pixels 为准。该 profile 独立于 Linux 1280x720
基线，需要与 platform gate 相同的 Accessibility 权限；像素不匹配只产生 blocker，
不产生性能比较。当前保留的[正式报告](../../evidence/macos-native-retina/report.json)
在 full-gated `5460aa12c` 上检测到逻辑 1440x900、scale 2、CoreGraphics backing
目标 2880x1800，而官方 Java 26.1 fullscreen F2 实际 drawable 为 1920x1200；runner
已在 profiling 和 BBB 启动前拒绝该轮，故报告不含可发布的 ABBA 指标。

## 第二平台 cross-check

Linux 主机看不见 `cfg(target_os = "macos")` 后面的代码，这些代码因此可能无声腐坏。
`scripts/cross_check.py`（`scripts/cargo-dev.sh cross-check`）用
`cargo check --target aarch64-apple-darwin` 把这个缺口关到能关的程度，作为 gate
的第二平台步骤在 crate 源码或 manifest 改动时运行。Linux 开发机需要一次
`rustup target add aarch64-apple-darwin`；target 缺失时该步归因跳过而不是假装通过，
所以缺 target 的机器不提供这份证据。`--target` 可换其它 triple（如 Intel Mac 的
`x86_64-apple-darwin`）。macOS aarch64 主机运行默认 target 时，11 个 workspace crate
全部接受逐 crate 本机检查；Linux 专有分支由 Linux full/platform gate 负责，Mac 本机
检查不声称替代另一发布行。

它关不掉全部：一些第三方 build script 要为目标平台编译 C 或 Objective-C，需要
Linux 主机没有的 Apple 工具链，凡是依赖闭包碰到它们的 crate 都会停在那个 build
script 上，读不到任何 bbb 源码。**哪些 crate 属于这一类是实测的，不是写死的**：
脚本对每个 workspace 成员实跑一次检查，只把错误指名了已知 Apple 工具链 build
script（`ring`、`objc_exception`、`coreaudio-sys`）的失败算作受阻，其余一律算真实
类型错误并让命令失败。于是 crate 增减依赖会自动重新分类，真实错误也不会被误当作
工具链边界。

Linux cross-host 的当前分区：`bbb-audio`、`bbb-control`、`bbb-pack`、`bbb-platform-macos`、
`bbb-protocol`、`bbb-render-types`、`bbb-world` 受检；`bbb-item-model`、
`bbb-native`、`bbb-net` 受阻于 `ring`，`bbb-renderer` 受阻于 `objc_exception`。

## macOS 系统调用的归属

Linux cross-host 检查会在 `bbb-native` 的第三方闭包处受阻，所以留在它里面的 AppKit
调用无法由 Linux gate 编译。这些调用因此收敛在 `crates/bbb-platform-macos`——它的依赖
闭包是纯 Rust，Linux 上真的会编译，Mac 本机则连同 `bbb-native` 全部编译。约束由两条守卫维持：`cross_check.py` 在该
crate 掉出受检集合时报错，`test_dev_chain_portability.py` 的同族测试拒绝
`bbb-native` 里出现 `objc2`。

`bbb-native` 侧只剩一句转调，没有平台分支。

## 平台分派 owner

`crates/bbb-native/src/platform.rs` 是"操作系统决定行为"的唯一 owner：URI/路径
打开进程与 bbb 自有 client store 的默认路径（`BBB_CODE_OF_CONDUCT_STORE`、
`BBB_HOTBAR_PRESET_STORE` 可覆盖）都在这里分派。`PlatformTarget` 只有 `MacOs` 与
`Unix` 两个分支。分类走目标名字符串而不是 `cfg!`，所以 macOS 分支在 Linux 上一样
被编译、一样在测试里被走到：`~/Library/Application Support`、`XDG_CONFIG_HOME`、
`~/.config` 以及工作目录兜底各有断言，`"windows"` 这个目标名落到 `Unix` 也有断言
钉住（它不该长出自己的分支）。新增按平台分叉的行为加到这里，不要在消费者里再写
一条 `cfg!` 阶梯。

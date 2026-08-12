# macOS 与 Linux 开发

bbb 维护两条声明发布行：Linux x86_64 / Wayland / Vulkan 与 macOS aarch64 /
Cocoa / Metal。cross-check 只能提前发现类型错误；正式 platform receipt 必须由对应
主机的真实窗口、GPU、音频与输入链路产生。

Windows 是产品 non-goal，`bbb-native` 用 `compile_error!` 拒绝 Windows target，
portability 测试拒绝 `cfg(windows)` 或 Windows 专有 API 回流。Vanilla options 中
`syncChunkWrites` 的 Windows 默认值仍是目标版本事实，由生成 catalog 消费，不代表
Windows 平台支持。X11 代码保持可编译，但声明 Linux 发布行只接受 Wayland；两项 scope
裁决及重开条件以 ledger 为准。

## 外部工具

仓库不安装系统依赖。`cargo`/`rustup`、`git`、`rg`、`java` 25 与锁定版本的 `uv`
必须由主机提供。`scripts/bootstrap` 准备 uv-managed Python、Rust 依赖和 repo-managed
Vanilla workspace；日常 Python 入口统一经过 `scripts/python`，不依赖系统 Python。

`scripts/tests/test_dev_chain_portability.py` 拒绝开发脚本中的 GNU-only 写法、`/bin/sh`
bashism 和表外下载/容器工具。macOS full/platform gate 负责 BSD userland 与 GUI 真路径，
Linux gate 负责 GNU/Wayland 真路径。

## 共用开发入口

- `scripts/cargo-dev.sh fast-test` 或 `test -p <crate> <filter>`：focused Rust 测试。
- `scripts/dev doctor --profile portable`：按能力探测主机。
- `scripts/dev lint python` 与 `scripts/dev test python --suite ...`：Python 工具链。
- `scripts/cargo-dev.sh gate`：完整门禁并写 `.git/bbb/last-gated`。
- `scripts/cargo-dev.sh cross-check`：第二平台类型检查。
- `scripts/docs-lint.sh`：当前文档、ledger、matrix 与 evidence 链接检查。
- `scripts/vanilla prepare|verify|path|clean 26.1`：唯一 Vanilla workspace 入口。

Cargo target 始终放在 `/tmp/bbb-target-<name>`，不得生成 repo-local `target`。

## 主机发布验收

`scripts/platform-acceptance-gate.sh` 只接受两条精确 host 行：

- Linux x86_64：Wayland/Vulkan，真实输入来自持久 `uinput` → libinput → sway →
  `wl_seat`，并验证 DPI/fullscreen 与 fcitx5 IME。
- macOS aarch64：Cocoa/Metal，真实输入来自获得 Accessibility 权限的 CoreGraphics；
  验证最终 client PID 的键盘、按钮、相对移动和 Command+V。音频必须通过 Vanilla
  manifest 锁定的 arm64 OpenAL 打开真实 CoreAudio device。

普通 synthetic playable 路径不能冒充平台输入证据。macOS 运行前必须解锁登录 GUI
session，并给调用终端或 Codex 进程 Accessibility 权限。所有 client/server/tool 进程
进入独立 process group，并由 bounded cleanup 回收。

## macOS 原生 Retina 性能合同

`scripts/macos_native_retina.py detect|run` 分别记录内建显示器标识、panel-native pixels、
逻辑 bounds、scale、CoreGraphics backing 和实际 drawable。目标分辨率取 panel-native
pixels，最后仍以两侧 F2 截图和 BBB 的 CAMetalLayer runtime receipt 为准；任何 drawable
不匹配在性能比较前 fail-closed。

当前基线的唯一当前事实入口是
[`current-release-baseline.json`](../../evidence/macos-native-retina/current-release-baseline.json)。
它绑定正式报告、capture commit、release binary、固定候选 margin 和 SHA-256；
这些值不得在 Wiki 或 goal 中复制。profile 固定：

- AC Power、fullscreen 2560x1600、VSync off、FPS unlimited；
- 相同 world、replay、settings、resource pack、input 和 visible/resident/draw workload；
- BBB 使用 registry wgpu/core/hal/types/naga 30.0.0、Metal Immediate、latency hint 2、
  实际三个 drawable 和一个 controller-owned render submission；
- 顺序为 wait→acquire→submit→present，等待 exact render-submission fence；
- 官方对比采用 Java→BBB→BBB→Java ×2，BBB candidate 先采用 B→C→C→B ×2。

validator 先核对 binary/dependency/runtime receipt、drawable、surface、timestamp strategy
和 workload identity，再检查 combined、三个 scene、两个 round、全部 nearest pair 与逐
scene pair。FPS、App/GPU p99、App 1% low、App/GPU hitch 和同仪器 input software latency
是 candidate 硬门禁；p50/p95/stddev/p99-p50/CV 等字段保留为诊断，不能单独阻止，也不能
被 combined 隐藏。候选必须采用 receipt 中预声明且不超过 1% 的 margin。

wgpu 供应链由
[`wgpu-30.0.0.json`](../../evidence/dependency-receipts/wgpu-30.0.0.json)固定：不得出现
path patch、vendor 或直接 `wgpu-hal` 依赖。runtime 必须同时输出 build-info 和 surface
receipt；AppKit layer probe 只读同一 CAMetalLayer 的 `drawableSize` 与
`maximumDrawableCount`。兼容 schema 只定义 parser 行为，不赋予非当前 report 当前状态；
其演进只从 Git history 追溯。

## 第二平台 cross-check

`scripts/cross_check.py` 在 Linux 用 `cargo check --target aarch64-apple-darwin` 编译能到达的
workspace crate。只有错误明确来自缺失 Apple C/Objective-C 工具链的已知 build script
才能归因跳过；其它失败都是类型错误。crate 依赖变化后脚本重新实测分类，不维护静态豁免
清单。macOS 本机默认 target 检查不替代 Linux 发布行，反之亦然。

## macOS 系统调用归属

AppKit 调用收敛在 `crates/bbb-platform-macos`。该 crate 的依赖闭包允许 Linux cross-check
真正编译其 Rust 边界；`bbb-native` 只保留窄转调。portability 测试拒绝 `bbb-native`
直接引入 `objc2`，cross-check 则拒绝平台 crate 掉出受检集合。

`crates/bbb-native/src/platform.rs` 是 URI/路径打开和 BBB client-store 默认位置的唯一
平台分派 owner。`PlatformTarget` 只有 `MacOs` 与 `Unix`；新增平台差异必须进入该 owner，
不能在消费者中新增分散的 `cfg!` 阶梯。

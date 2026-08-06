# macOS 与 Linux 开发

bbb 在 macOS 与 Linux 上开发，只在 Linux 上发布验收。这两件事的边界是本页的
全部内容：**哪些命令两个平台都跑，哪些只跑在声明发布行的 Linux 主机上，以及
Linux 主机如何为它编译不了的那部分 macOS 代码负责**。

Windows 按用户裁决为非目标，没有发布行，也不作为开发平台；仓库仍保留
vanilla 语义的 Windows 分派分支（见下方"平台分派 owner"），与同样 `not-needed`
的 X11 行处理一致。

## 外部工具

两个平台相同，都不由仓库安装：

| 工具 | 用途 | macOS 获取方式 |
| --- | --- | --- |
| `cargo` | 构建与测试；工具链版本由 `rust-toolchain.toml` 钉死 | `rustup` |
| `git` | 版本控制与门禁 stamp | Xcode Command Line Tools |
| `python3` | `scripts/` 分析与门禁脚本 | Xcode Command Line Tools |
| `rg` | docs lint 的模式检索 | `brew install ripgrep` |
| `uv` | `scripts/vanilla-source-tools.sh` 的离线 Python 环境 | `brew install uv` |

`scripts/tests/test_dev_chain_portability.py` 守住这张表：它拒绝开发链路脚本里的
GNU-only 写法（`readlink -f`、`sha256sum`、`stat -c`、`sed -i`、`grep -P`、`nproc` 等）、
`/bin/sh` 脚本里的 bashism（macOS 的 `/bin/sh` 不是 bash），以及一组常被顺手引入的
表外工具（`curl`、`wget`、`jq`、`docker`）。这些差异只会在 macOS 上暴露，而门禁不跑在
macOS 上——所以由这条测试代替 macOS 主机发现它们。

## 两个平台都跑的

- `scripts/cargo-dev.sh fast-test` / `test -p <crate> <filter>`：日常 focused 测试。
- `scripts/cargo-dev.sh gate`：完整门禁，7 步，写 `.git/bbb/last-gated`。
- `scripts/cargo-dev.sh cross-check`：第二平台类型检查，见下节。
- `scripts/docs-lint.sh`：文档与账本 lint。

target 目录约定（`/tmp/bbb-target-<name>`）在 macOS 上同样成立。

## 只跑在 Linux 发布主机上的

声明发布行是 Linux x86_64 / Wayland / Vulkan，它的 receipts 由一条真实
`wl_seat` 链路产生（`uinput` → libinput → sway）。这些接口在 macOS 上不存在，
相关脚本会指名主机后拒绝，而不是在缺失的内核接口上失败：

- `scripts/platform-acceptance-gate.sh`：发布行验收。
- `scripts/seat_device.py` / `scripts/seat_inject.py`：seat 注入。
- `scripts/gpu_duration_capture.py preflight|capture`：需要 tracefs 与 perf；
  `summarize` 只读既有 capture，与主机无关，不受限制。

`scripts/playable-gate.sh` 与 `scripts/lifecycle-soak-gate.sh` 不按主机拒绝：
它们本来就按前置条件（server jar、Java、显示环境）逐项降级并给出归因。

## 第二平台 cross-check

Linux 主机看不见 `cfg(target_os = "macos")` 后面的代码，这些代码因此会无声腐坏。
`scripts/cross_check.py`（`scripts/cargo-dev.sh cross-check`）用
`cargo check --target aarch64-apple-darwin` 把这个缺口关到能关的程度，作为 gate
的第 6 步在 crate 源码或 manifest 改动时运行，rustup target 缺失时归因跳过。

它关不掉全部：一些第三方 build script 要为目标平台编译 C 或 Objective-C，需要
Linux 主机没有的 Apple 工具链，凡是依赖闭包碰到它们的 crate 都会停在那个 build
script 上，读不到任何 bbb 源码。**哪些 crate 属于这一类是实测的，不是写死的**：
脚本对每个 workspace 成员实跑一次检查，只把错误指名了已知 Apple 工具链 build
script（`ring`、`objc_exception`、`coreaudio-sys`）的失败算作受阻，其余一律算真实
类型错误并让命令失败。于是 crate 增减依赖会自动重新分类，真实错误也不会被误当作
工具链边界。

当前分区：`bbb-audio`、`bbb-control`、`bbb-pack`、`bbb-platform-macos`、
`bbb-protocol`、`bbb-render-types`、`bbb-world` 受检；`bbb-item-model`、
`bbb-native`、`bbb-net` 受阻于 `ring`，`bbb-renderer` 受阻于 `objc_exception`。

## macOS 系统调用的归属

`bbb-native` 不可 cross-check，所以留在它里面的任何 AppKit 调用都是本仓库任何
主机都不编译的代码。这些调用因此收敛在 `crates/bbb-platform-macos`——它的依赖
闭包是纯 Rust，Linux 上真的会编译。约束由两条守卫维持：`cross_check.py` 在该
crate 掉出受检集合时报错，`test_dev_chain_portability.py` 的同族测试拒绝
`bbb-native` 里出现 `objc2`。

`bbb-native` 侧只剩一句转调，没有平台分支。

## 平台分派 owner

`crates/bbb-native/src/platform.rs` 是"操作系统决定行为"的唯一 owner：URI/路径
打开进程与 bbb 自有 client store 的默认路径（`BBB_CODE_OF_CONDUCT_STORE`、
`BBB_HOTBAR_PRESET_STORE` 可覆盖）都在这里分派。分类走目标名字符串而不是
`cfg!`，所以 macOS 与 Windows 分支在 Linux 上一样被编译、一样在测试里被走到：
`%APPDATA%`、`~/Library/Application Support`、`XDG_CONFIG_HOME`、`~/.config`
以及工作目录兜底各有断言。新增按平台分叉的行为加到这里，不要在消费者里再写
一条 `cfg!` 阶梯。

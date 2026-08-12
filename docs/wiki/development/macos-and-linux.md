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
目标取硬件 panel-native pixels，并通过 `fullscreenWidth`/`fullscreenHeight` 与 BBB
的 exact preferred video mode 同时传入两端，最终仍以两侧 F2 截图实测的 drawable
pixels 为准。该 profile 独立于 Linux 1280x720 基线，需要与 platform gate 相同的
Accessibility 权限；像素不匹配只产生 blocker，不产生性能比较。正式结果保存在
[独立报告](../../evidence/macos-native-retina/report.json)。

2026-08-10 的正式两轮 ABBA 在 full-gated `54f20a62a` 上通过：内建 `Color LCD`
（display id 1）逻辑 bounds 为 1440x900、scale 2.0、当前 backing mode 为
2880x1800，硬件 panel-native 及两端每段 F2 截图实测 drawable 均为 2560x1600。
两端均为 fullscreen、VSync 关闭、帧率上限解除；BBB 使用 Metal / Apple M1，官方
Java 26.1 使用 OpenGL / Apple M1。八轮的 world/replay/settings/resource-pack/input
哈希完全相同。合并三段、四轮后的 application frame-time 汇总为：BBB 120.738 FPS、
8.282 ms、1% low 43.240 FPS、p95/p99 17.945/21.520 ms、CV 0.5822；官方为
80.646 FPS、12.400 ms、1% low 55.267 FPS、p95/p99 15.248/16.806 ms、CV 0.1201。
进程 CPU/frame 均值分别为 2.902/5.184 ms；GPU duration 均值分别为
13.281/10.353 ms。报告 SHA-256 为
`84713476aff92e2fce1ecfc5df3ecbef6cc250149ce2f4610c91c828503b016b`。

2026-08-11 在 full-gated `d698c8db0` 上以完全相同的 2560x1600、两轮
Java→BBB→BBB→Java 合同复测。schema 2 从原 profiler archives 与 raw sidecars 逐值
互证后重算：combined BBB/官方 application mean 为 6.578/12.370 ms，p50 为
6.513/12.049 ms，p95 为 8.627/15.070 ms，p99 为 10.583/16.542 ms，stddev 为
1.226/1.456 ms，p99-p50 为 4.070/4.493 ms，1% low 为 79.617/56.079 FPS，
>=33.333 ms hitch 为 0/1；BBB mean FPS 为 152.029。GPU mean 为
0.435/10.317 ms，p50 为 0.446/10.161 ms，p95 为 0.661/12.456 ms，p99 为
0.805/13.400 ms，stddev 为 0.141/0.961 ms，p99-p50 为 0.359/3.239 ms，
hitch 为 0/1。

CV 仍记录为 application 0.1864/0.1177、GPU 0.3237/0.0932，但不再单独阻止。
combined 与所有 GPU scope 通过；active 的精确原因是 application `p99-p50` 在
`static_orbit`、`preloaded_flight` scene aggregate 回退，以及七个 nearest-run-pair
scene scope 中共九条 stddev/p99-p50 失败分类。报告保留 metrics publishable，同时以
`blocked` 明示这些绝对 scope；capture commit `d698c8db0` 当时用于 BBB FPS baseline，
现只保留历史复核，不再参与新候选比较或 margin 推导。原 schema-1
[capture report](../../evidence/macos-native-retina/report.json)
保持 SHA-256 `2864ea8d003cf53cc98ce782ecfe9d89112b5092d6786b9c221af1e634e42b9b`；
raw-backed schema-2 [stability report](../../evidence/macos-native-retina/stability-report.json)
的 SHA-256 为 `134c45cb3156333827533d58345787854dc69b9ab1a07c5e2618336177eaf289`。

后续 schema 3 capture 在 BBB 的 profiler zip 中增加
`client/metrics/frame_phases.csv`。每个 application frame 成对记录
`cpu_encode_before_wait`、`previous_submission_wait`、`surface_acquire`、
`presentation_tail_encode`、`queue_submit`、`swap_buffers_present`、
`gpu_async_poll`、`render_call_total` 与 `application_frame_interval`；validator
要求 archive、raw sidecar 与报告 summary 逐值一致，再输出每阶段
p50/p95/p99/stddev。render 入口的 interval 已在本帧工作开始前结束，因此主相关性明确按
phase[N]→application interval[N+1] 计算；同 row 的 prior-interval correlation 只保留作
时序诊断。旧 schema-1
capture 与 schema-2 重判仍可复核；只有当前 schema 的 BBB capture 缺阶段列时才拒绝。
这些字段只在 GPU HUD 或正式 profiler 开启时采集，不改变 Immediate 单 in-flight、
drawable acquire、submit、present、surface-error 或 Linux 路径。

首份真实阶段归因见
[frame-phase diagnostic](../../evidence/macos-native-retina/frame-phase-diagnostic.json)：
capture commit `f0cc03ad7` 与 lag-1 analysis commit `c7aa715af` 均已 full gate，三场景
共 4,588 帧、实际 drawable 2560x1600。combined submission wait mean/p95/p99 为
5.025/5.808/6.874 ms；逐场景 wait→next-interval correlation 为
0.567/0.589/0.682。surface acquire p99 为 3.450/2.820/2.182 ms，相关性为
0.546/0.417/0.290。encode-before-wait p99 不超过 0.719 ms且三场景相关性为负；因此只触发
保持行为的 FrameFlightController 重构。full-gated `9802b1ef5` 已把 Immediate 单 in-flight
budget、wait/acquire、surface fallback 与 submit/present 收敛到该 owner，顺序和调度未变；
upload arena、FramePlan、多帧排队与后台 acquire 仍未触发。

Slice B 的首个独立策略只把 macOS Immediate 的 drawable acquire 提前到 previous
submission wait 之前，仍保持单 in-flight 且不启用后台 acquire。full-gated candidate
`ff7c7c968` 在 AC Power 下完成 baseline→candidate→candidate→baseline 两轮交错 A/B，
随后由正常 fix-forward commit `6851825cd` 恢复原顺序并 full gate。
[AC steady baseline](../../evidence/macos-native-retina/ac-steady-baseline.json) 的四次
重复 FPS 为 154.546/154.917/154.657/155.073，预声明噪声 margin 为 0.3404%；其
SHA-256 为 `69e4186722cd649c080235b2588ccab7a75669440b01533f77b64827b3a25a93`。

[AC candidate report](../../evidence/macos-native-retina/acquire-before-wait-ac-candidate.json)
中 combined FPS 为 baseline 155.168、candidate 155.152（-0.010%，非劣），但 App
p95/p99/stddev/p99-p50 由 8.240/9.987/1.199/3.701 ms 回退到
8.280/10.467/1.231/4.190 ms，1% low 从 78.088 降到 75.675 FPS；GPU
mean/p95/p99/p99-p50 也从 0.473/0.699/0.886/0.404 ms 回退到
0.479/0.712/0.891/0.408 ms。四个 all-scene pair 的 FPS delta 为
-0.069%/+0.239%/+0.035%/-0.246%，两个 ABBA round 的 delta 为 +0.085%/-0.106%；
两个 round 的 App/GPU 绝对稳定性都失败，另有 pair-1 static 与 pair-4 preloaded 两个
scene-pair 超过固定 margin。因此在正式 Vanilla ABBA 前拒绝。报告 SHA-256 为
`163dfb5557ad8002a5f097db6c627dbc6268744b76a9c36c2549fa46c510fe07`。

第二项有界 candidate 在 full-gated `36bd347d7` 保持 macOS Immediate 单 in-flight 和
wait→acquire→submit→present：先用 `Device::poll(Poll)` 非阻塞检查上一 submission，只有
queue 仍 busy 才执行原 `WaitForSubmissionIndex`。frame-phase schema v2 将
`previous_submission_poll` 与 `previous_submission_blocking_wait` 从 total
`previous_submission_wait` 中分列；validator 逐 tick 要求 poll+blocking 不超过 total，并
继续校验 archive/raw sidecar/summary。

接电、无温控告警的两轮 B→C→C→B 在八轮均实测 fullscreen 2560x1600，且 workload/input/
settings/resource/scene hash 完全一致。17,252 个候选帧仅 110 帧（0.638%）免于 blocking
wait；poll mean 0.00235 ms，而 blocking wait mean/p95/p99 仍为
4.849/5.919/6.970 ms。combined FPS 为 baseline 143.916、candidate 143.717
（-0.138%，在固定 0.3404% margin 内），但 App p50/p95/p99/stddev/p99-p50 从
6.800/9.071/10.731/1.356/3.931 ms 回退到
6.817/9.095/10.768/1.412/3.951 ms，1% low 从 73.428 降到 70.951 FPS；GPU p50/p95
也从 0.4890/0.8428 回退到 0.4892/0.8493 ms。round-2 FPS -0.514%，另有七个 pair/scene
FPS scope 加上该 round 超 margin；绝对稳定性在 combined、两个 round 与多项 pair/scene
失败，共 49 条 validator failure。因此
[completion-poll AC candidate report](../../evidence/macos-native-retina/completion-poll-ac-candidate.json)
以 SHA-256 `bce5d7bce10a75f46dde3b1bd978cc0502b29a2c51be726865aedce39bda9c10`
在 Vanilla ABBA 前拒绝，正常 fix-forward 恢复 exact blocking fence。schema v2 仍保留两列，
accepted policy 将 poll 列显式写零，不回退已有 parser/Linux 兼容性。
Vanilla `Minecraft.java:1338-1356` 仍是顺序锚点：main target blit、GPU profile end、
swap/`RenderSystem.flipFrame` 先完成，显式 FPS limiter 最后运行；该 Metal 内部策略不改变
这一可见 present/cap 顺序。

第三项 Slice B candidate 没有提高 FrameFlightController 的 submission budget，而只缩小
Metal presentation engine 的 drawable 队列。wgpu-types 0.19.2 对
`desired_maximum_frame_latency=1` 的权威说明是“low latency from frame recording to frame
display”；wgpu-hal 0.19.5 Metal capabilities 在支持 maximum drawable count 的系统给出
`1..=2`，configure 时执行 `maximum_frame_latency + 1`，所以 baseline 2 对应三个
CAMetalLayer drawables，candidate 1 对应两个。候选仅在 macOS Immediate 请求 1；
Linux Immediate、FIFO/FifoRelaxed/Mailbox/Auto 模式与 offscreen 保持 2，显式 in-flight
submission budget 保持 1，wait→acquire→submit→present 和 surface fallback 未改变。

`macos-native-retina-bbb-ab-v2` 报告在性能比较前要求这组 2→1、3→2、budget 1→1、
`queue_depth_increased=false` 合同；run report 从 BBB log 记录 present mode、请求 hint 与
wgpu Metal 预期 drawable count，validator 按 B-C-C-B×2 逐 run 核对，不能由顶层合同
代替实际观测。历史 v1 candidate report 不补造该字段，仍按 v1 规则
复核。full-gated `969ea38f7` 的接电 2560x1600 实测逐 run surface 合同全过，但全部
22 个 FPS scope、44 个 App/GPU 绝对稳定性比较失败；combined FPS 153.996→142.297，
App p95/p99 8.318/10.135→11.804/15.927 ms，GPU p95/p99
0.679/0.877→0.713/0.893 ms。`surface_acquire` p99 为 8.795 ms 且与下一 application
interval 相关系数 0.847，证明两个 drawable 反而把显著等待转移到 acquire。报告
`low-latency-surface-ac-candidate.json` SHA-256 为
`662d716afdf23442713dbab6ed70892948db8246949c00b36ddaaabe9c08a939`；候选已拒绝、默认
三个 drawable 恢复，不进入官方 Java ABBA。

后续基础设施不再维护本地 wgpu-hal 0.19.5 backport，而是完整迁移到 crates.io wgpu
30.0.0。repo 的 [dependency receipt](../../evidence/dependency-receipts/wgpu-30.0.0.json)
把 wgpu、wgpu-core、wgpu-hal、wgpu-types、naga 的 registry version/checksum 与
Cargo.toml/Cargo.lock hash 固定下来，并记录 wgpu 30 首次发布的 Metal
completion-handler/condition-variable wait 修复 commit
`e0a6065c65b5b6c2e92ee7052ed084cd8910d486`。runtime 同时从 BBB `--build-info` 和
renderer log 输出该 receipt；正式采集还从 wgpu 安装后的 AppKit layer tree 只读查询
CAMetalLayer `drawableSize` 与 `maximumDrawableCount`，不再把 hint+1 单独称为实际值。
固定 0.19 baseline 不重写源码；capture 只通过
`scripts/fixtures/official-client-benchmark/metal-layer-probe.m` 在第一次
`nextDrawable` 时读取同一 CAMetalLayer，立即恢复原 IMP，再进入预热和正式采样。

`macos-native-retina-bbb-ab-v3` 的 0.19→wgpu 30 比较只保留为历史迁移报告：
[`wgpu30-ac-candidate.json`](../../evidence/macos-native-retina/wgpu30-ac-candidate.json)，
SHA-256 `7a5fc89662d9da50cb6f9556c1af68f9ce46c777f8778e3f30b6d651c90bfcfa`。它不再提供
active baseline、noise margin 或 rollback target。

active `macos-native-retina-bbb-ab-v4` 把 clean/full-gated `5b1903221` 与 release SHA-256
`849a9300ed1b1325137506f7519782b57c264eb31347d36725c96686afa20817` 固定为 wgpu 30
baseline。四次 AC repeat 为 195.828/194.414/194.422/194.180 FPS，预声明 margin 为
0.8465%。以后 baseline/candidate 每个 run 都必须解析到 registry wgpu/core/hal/types/naga
30.0.0，绑定相同 `PassBoundary` timestamp strategy、Immediate/hint 2、实际三个 drawable、
单 in-flight、2560x1600 fullscreen 和同一 workload；任一 identity/scope mismatch 都在解析
FPS 前拒绝。full-gated `62555c6bf` 的 command-buffer-boundary diagnostic 保持 budget 1
和一次 submit，combined FPS 194.454→205.274，但最近 C→B 配对的 App p99/stddev/p99-p50/
1% low 与 GPU mean/p50/p95 回退；combined/scene 共 28 个绝对指标失败。因此不进入正式
B→C→C→B×2 或 Java ABBA，单 encoder 行为已 fix-forward 恢复。raw-hash-backed 报告为
[`prefinish-command-buffer-ac-diagnostic.json`](../../evidence/macos-native-retina/prefinish-command-buffer-ac-diagnostic.json)，
SHA-256 `a33af85ee8c5b48a96af3c1086e2dcec72c1435f07b03cb50c49ff59c0607c96`。
后续 callback-observation 的正式接电 B→C→C→B×2 保持相同 surface/workload；combined FPS
183.626→184.874 且 combined App p99/stddev/p99-p50 改善，但 pair-2 all-scenes/preloaded FPS
和多项逐 scene/pair App/GPU 绝对指标失败，因此不进入 Java ABBA。报告
[`callback-observation-ac-candidate.json`](../../evidence/macos-native-retina/callback-observation-ac-candidate.json)
SHA-256 为 `37f4070b0ddfe5f57cd4d2c0a2353ba01c787305533c4e1716b4c5834fdc7505`。观察到的 callback
几乎紧邻 wait 返回，surface acquire 才是对下一 application interval 相关性最强的 CPU phase；
GPU scope 的失败要求先定位具体 GPU owner。

`renderer-gpu-stages-v1` 诊断已完成 8/8 个接电、fullscreen 2560x1600 有效 run。十个
额外 marker pass 使详细路径不能作为性能 candidate：22 个 FPS scope 与 44 个绝对
稳定性 scope 均拒绝它。它的同 submission 归因仍可用：transparency combine 的 combined
mean 为 2.396 ms（占 56.05%）、p99 为 3.410 ms；opaque world 的 combined p99 为
2.363 ms，与总 GPU duration 相关性为 0.760，chunk-boundary 中分别为 2.449 ms/0.869。
报告为 [`gpu-stage-ac-diagnostic.json`](../../evidence/macos-native-retina/gpu-stage-ac-diagnostic.json)，
SHA-256 `c0a1a336c74faacdf42cd17bf21509b410ec798afedb5aee86d41a7914e7a5cb`。普通路径仍只有两个
query。下一单一有界 candidate 只将五个可选透明层的 depth fetch 延迟到该像素 alpha
非零之后；六张纹理、插入顺序、深度排序、预乘 alpha 混合、pass/draw/workload 均不变。

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

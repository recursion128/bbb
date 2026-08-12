# 测试、门禁与证据

## 证据层级

1. Vanilla source evidence：确认机制，不证明 BBB wiring。
2. unit/focused test：证明一个 owner 的输入、顺序、fallback 和错误边界。
3. integration test：证明 projection/consumer 使用同一事实。
4. real evidence：真实 assets/server/GPU/window/audio 等只在需要时验收。
5. full/playable gate：证明整仓或声明可玩表面没有回归。

不得用截图“看起来接近”替代机制断言，也不得用 unit test 声称真实平台已通过。

## 工程切片门禁

`goal.md` 是 checkpoint/full-gate 唯一排程 owner。每次 commit 前执行其 per-commit
checks，包括 focused tests、docs lint、format、diff check 和完整 diff review。
`scripts/cargo-dev.sh gate` 在 `/tmp/bbb-target-main` 运行完整门禁，并把通过 commit 写入
`.git/bbb/last-gated`；`batch-gate` 只用于失败定位。门禁先执行 Vanilla 完整性校验和
portable host capability 检查；Ruff 与 pytest 取代散落的 unittest discovery。十步中
scripts Python lint/套件只在 `scripts/` 改动时跑，第二平台 `cross-check` 只在 crate 源码或
manifest 改动时跑，`BBB_GATE_FULL=1` 强制两者。**改动范围按门禁实际认证的范围取**：
工作树脏时对 `HEAD`，干净时对 `last-gated`（不是 `HEAD^`）——批次 tip 若只对上一个
commit 取 diff，批次里较早那个 commit 需要的步骤会被静默跳过。stamp 缺失或不是 `HEAD`
祖先时才回落到 `HEAD^`。cross-check 用
`cargo check --target aarch64-apple-darwin` 覆盖 Linux 主机编译不到的 macOS 分支，
边界与 crate 分区见 [macOS 与 Linux 开发](macos-and-linux.md)。

所有 Python 步骤由根 `.python-version`、`pyproject.toml`、`uv.lock` 和
`scripts/python` 共同固定；门禁只做 offline locked sync，不下载解释器或依赖。新机器
必须先运行 `scripts/bootstrap`，让环境准备与门禁验证保持两个可归因阶段。

playable gate 需要真实 live session 时才运行并更新 `.git/bbb/last-playable-live`。
纯 docs/workflow slice 不改变可玩行为，可验证旧 stamp 是最终 HEAD 祖先；如果修改产品
行为，这个豁免立即失效。

## 生命周期与平台验收

`scripts/lifecycle-soak-gate.sh` 在五次独立 live session 中交替验证 quit/kick、
resource generation、reconfiguration、Transfer、feature-gated resize/minimize/restore、
surface lost/outdated/timeout、no-audio fallback、终态 queue/gauge、RSS/FD/thread 趋势和
clean process exit。唯一场景输入是
`scripts/fixtures/lifecycle-soak/manifest-26.1.json`；成功后写入
`.git/bbb/last-lifecycle-soak` 与本地 report。fixture 重放不能刷新 live stamp。

趋势判据是「post-warmup 严格单调上升 **且** spread 超限」，两个条件都要满足。iteration
数直接决定它的强度：3 次只留 2 个 post-warmup 采样，"严格单调"退化成一次两两比较，
噪声就有约一半概率触发。FD 与 thread 在实跑中跨 iteration 完全持平（spread 0，限额
8/4），是精确探针；退出时 RSS 则受 allocator 保留量与 terrain streaming 进度支配，同一
场景实测在 1250–1451 MB 之间摆动（三轮共 11 个采样），噪声带约 200 MB。因此 iteration
取 5（4 个 post-warmup 采样）、`rss_bytes` spread 限额取 256 MiB 抬到噪声带以上——低于
噪声带的阈值不是更严格，只是掷硬币。
manifest 同时唯一拥有 per-iteration 与 terminal-phase timeout；quiescence、
kick-disconnect、process-exit 使用三个互不复用且总和受 iteration budget 约束的 deadline。
kick 必须同时具有目标 Vanilla 服的精确 receipt、control `connected=false`、可机器解析
的断线页 diagnostic 与 kick 后仍存活的进程指标；随后只通过 control `app.quit` 的
`running=false` 回执和零退出关闭客户端。服务器权威状态查询按
有界 cadence 重取新日志行，不能把跨 console/network 通道的一次旧读复用到整个预算。

`scripts/platform-acceptance-gate.sh` 声明两条 windowed external-multiplayer 行：
`Linux x86_64 / Wayland / Vulkan` 与 `macOS aarch64 / Cocoa / Metal`。其它 host/arch 在
build 前按名称拒绝。两者都要求 fully gated clean HEAD、fresh locked release build、
`--build-info`、headless/live/quit、实际 winit/wgpu backend 日志、输入 focused receipts、
当前 lifecycle-soak 祖先 stamp 与 attributable platform-input receipts；成功后分别写
`.git/bbb/last-linux-platform` 或 `.git/bbb/last-macos-platform` 及本地 report。Mac 行还
要求受信 Accessibility、服务端确认的 Quartz keyboard/pointer/clipboard 与真实
OpenAL/CoreAudio device/capacity/submission 收据。

Linux seat receipts 来自真实 `wl_seat`：headless backend 不广播 `wl_seat`，而
`scripts/seat_device.py` 常驻一个 `uinput` 设备后，compositor 的 seat 才持续带
pointer/keyboard capability。设 `BBB_PLAYABLE_INPUT_BACKEND=linux-release` 后，
live playable gate 只调用 repo-owned `scripts/input`，由它选择持久 uinput seat；不再接受
任意外部命令字符串，也不再发任何合成
`input.*`（`_seat_input_only` 让这类写在 seat 场景内直接抛错）。

`scripts/process` 是 live gate 与 benchmark 的进程监管合同：子进程进入独立 POSIX
process group，超时使用 monotonic deadline，退出时先 TERM、到期再 KILL 并回收。
shell 不再依赖 GNU `setsid` 或 `timeout`。该入口直接 exec 已 bootstrap 的 venv Python，
因为 `uv run` 会保留一个父监督进程并改变最终 PID/进程组；其它 Python 工具仍统一经过
offline/locked uv runner。fixture/unit 层用 pytest marker 区分
`integration`、`vanilla`、`live` 与 `linux_release`；普通 `scripts/dev test python`
默认不运行后两类真实环境场景。

Parity evidence 与 full gate 分层：实现提交保持 matrix `partial`，checkpoint gate 只证明
整仓无回退；clean gated code freeze 后才按 protocol/state、resource/platform、rendering
domain sweep 收集 formal receipts，最后运行两轮 ABBA。`scripts/python python
scripts/parity_acceptance.py plan` 用当前
owner/test/capture hash 规划局部失效；diagnostic smoke 一律带非 formal classification。
交付命令额外使用 `--require-final-head`，要求当前 HEAD clean、last-gated 对齐、artifact 双向
Mechanism ID 与全部正式 identity 有效。

三条经实跑得出的硬性前提，缺一条 receipts 就取不到：① **设备必须常驻**——capability
只在设备存在期间广播，客户端也只在其间绑定 `wl_keyboard`/`wl_pointer` 并收到焦点，
每次调用才建设备的工具会在客户端绑定完之前就消失；② **必须走 `uinput` 而非
virtual-keyboard 协议**——后者按调用方自带 keymap 传 keysym，而本客户端（与 vanilla
一样）读物理键码，收到的会是工具随手挑的扫描码；③ **compositor 的按键重复必须关**
（`input type:keyboard repeat_rate 0`，注入器会检查）——gate 是刻意长按的，600ms/25Hz
下一次长按等于二十多次按下，会把暂停菜单这类开关切回原状。此外**焦点要在运行中重新取回**：
任何后 map 的客户端都会夺走焦点，而在 Play 中失焦的客户端会自我暂停，事后补焦点不会关掉
暂停菜单——只有 Escape 会，且要用 hotbar 键复核（vanilla 只在无 screen 时执行 hotbar
keybind）。

seat 设备刻意常驻，也就一直跑着它启动时的那份代码：注入器在 probe 阶段向它要一次键表，
缺键就当场按名字报错，而不是在几十分钟后按下时才发现。

同一个 seat 还驱动 **display transition receipts**：把窗口钉成固定逻辑尺寸后改 output
scale（tiled 窗口的工作区会按同一倍数缩小，物理表面纹丝不动，改动不可观测），再用 seat 的
F11 切换全屏；两侧都取证——compositor 确实换了 scale/给了全屏，客户端表面也确实跟上并回得来。
固定尺寸动作由注入器机器化拥有：先去掉 compositor decoration，再将窗口切为 floating，等待
Wayland 的 stateless-size configure 稳定后才发出无歧义的宽高 resize，并回读 compositor 的精确
逻辑矩形。若把 floating 与 resize 背靠背发送，前一个异步 configure 会把后一个覆盖回启动尺寸；
若保留标题栏，sway 的外框尺寸也不等于客户端 surface，二者都必须在客户端 surface 回执前失败。
客户端没有 `ScaleFactorChanged` 分支是**忠实**而非缺口：vanilla 也没有 content-scale 回调，
GUI scale 只由 framebuffer 尺寸算出，而 winit 每次 scale 变化后都跟一条 `Resized`。

还有 **IME receipts**：gate 的 compositor 上跑 fcitx5（`~/.config/fcitx5/profile` 里 pinyin
入组、`config` 里 `ActiveByDefault=True`——不设它输入上下文默认是英文直通态，拼音根本不参与
合成），seat 用 `text` 动作逐个按下拉丁物理键，判据取**普通键盘产生不出来的东西**：打
`nihao`，规范 chat draft 回来的是非 ASCII 且不等于所打的字。能用 `T` 打开 chat 本身是半个
判据——IME 若全程开着会把 `T` 也吞去合成。

Linux 行的 limitation 集合为空。macOS 行同样以空 limitation 为通过条件，但它不执行
Wayland display transition/fcitx5 IME，而是要求 Cocoa/Metal、Quartz 与 CoreAudio 的
host-native receipts。X11 不属于声明 Linux release receipt；Windows 整体为产品非目标，
两者只在 ledger 的 scope 条件变化时重开。

## 文档测试

`scripts/docs-lint.sh` 是入口；Wiki 检查由其调用的 repo-owned helper 完成。fixture
必须无网络且覆盖通过、断链、重复 mechanism id、非法 parity 状态、孤儿 canonical
页面、失效 goal/ledger route 与禁止的历史文档布局。不能通过排除目录或放宽规则绕过
失败内容。

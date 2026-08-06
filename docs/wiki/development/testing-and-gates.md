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
`.git/bbb/last-gated`；`batch-gate` 只用于失败定位。

playable gate 需要真实 live session 时才运行并更新 `.git/bbb/last-playable-live`。
纯 docs/workflow slice 不改变可玩行为，可验证旧 stamp 是最终 HEAD 祖先；如果修改产品
行为，这个豁免立即失效。

## 生命周期与平台验收

`scripts/lifecycle-soak-gate.sh` 在三次独立 live session 中交替验证 quit/kick、
resource generation、reconfiguration、Transfer、feature-gated resize/minimize/restore、
surface lost/outdated/timeout、no-audio fallback、终态 queue/gauge、RSS/FD/thread 趋势和
clean process exit。唯一场景输入是
`scripts/fixtures/lifecycle-soak/manifest-26.1.json`；成功后写入
`.git/bbb/last-lifecycle-soak` 与本地 report。fixture 重放不能刷新 live stamp。
manifest 同时唯一拥有 per-iteration 与 terminal-phase timeout；quiescence、
kick-disconnect、process-exit 使用三个互不复用且总和受 iteration budget 约束的 deadline。
kick 必须同时具有目标 Vanilla 服的精确 receipt、control `connected=false`、可机器解析
的断线页 diagnostic 与 kick 后仍存活的进程指标；随后只通过 control `app.quit` 的
`running=false` 回执和零退出关闭客户端。服务器权威状态查询按
有界 cadence 重取新日志行，不能把跨 console/network 通道的一次旧读复用到整个预算。

`scripts/platform-acceptance-gate.sh` 当前只声明
`Linux x86_64 / Wayland / Vulkan / windowed external-multiplayer`。它要求 fully gated
clean HEAD、fresh locked release build、`--build-info`、headless/live/quit 证据、实际
winit/wgpu backend 日志、输入语义 focused receipts，以及当前 lifecycle-soak 祖先
stamp、以及 **seat receipts**；成功后写入 `.git/bbb/last-linux-platform` 与本地 report。

seat receipts 来自真实 `wl_seat`：headless backend 不广播 `wl_seat`（`wayland-info` 的
global 列表里没有该接口），而 weston 的 remote-desktop backend 在客户端接入时即广播带
pointer/keyboard capability 的 seat。设 `BBB_PLAYABLE_SEAT_INJECT` 指向
`scripts/seat_inject.py` 后，live playable gate 改由该 seat 驱动：不再发任何合成
`input.*`（`_seat_input_only` 让这类写在 seat 场景内直接抛错），焦点靠点击激活 surface
取得。**此时客户端会先按 vanilla 语义自我暂停**——未获焦点进入 Play 后
`Minecraft.pauseIfInactive` 开暂停菜单，且事后补焦点并不会关掉它——所以 gate 不去抢
那 4 秒窗口，而是让它暂停、再用 seat 的 Escape 把世界打开；每次 Escape 都用 seat 的
hotbar 键复核（vanilla 只在无 screen 时执行 hotbar keybind，故这是世界级而非仅键盘级
判据）。seat 场景随后再取一次带服务端确认的 hotbar 键与一次指针按钮 swing。

seat 的边界要照实记：remote-desktop seat 是绝对定位设备，compositor 不为其合成
relative-pointer 事件，因此**锁定光标的游戏内视角拿不到指针移动**——这是 runner 边界
不是客户端缺陷。GPU 侧同理——headless 默认走 pixman，加 `--use-gl` 才让 compositor 与其
Xwayland 客户端拿到硬件 radeonsi/RADV；`scripts/gpu_duration_capture.py` 的采集依赖这一点。
剩余的 DPI/fullscreen、relative pointer、clipboard、IME 重开条件由[缺口账本](../../unsupported-features.md)
维护；X11 行按 2026-08-06 用户裁决为非目标。macOS/Windows runner 同样只由该账本维护。

## 文档测试

`scripts/docs-lint.sh` 是入口；Wiki 检查由其调用的 repo-owned helper 完成。fixture
必须无网络且覆盖通过、断链、重复 mechanism id、非法 parity 状态、孤儿 canonical
页面、失效 goal/ledger route 与禁止的历史文档布局。不能通过排除目录或放宽规则绕过
失败内容。

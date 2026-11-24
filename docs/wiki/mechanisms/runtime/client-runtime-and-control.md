# 客户端运行时、控制请求与会话退出

机制 ID：`lifecycle-client-runtime`

## 机制目的和边界

本页拥有客户端 simulation 调度、外部控制写入、会话替换、资源恢复和退出顺序。
世界内部的状态应用见[客户端世界生命周期](../world/client-world-lifecycle.md)，播放消费见
[音频链路](../audio/client-audio-pipeline.md)。平台、网络和 backend 各自拥有资源，
但只有 simulation 决定客户端状态变化与 effect 顺序。

## Vanilla 26.1 权威实现

- `Minecraft.java:1232-1262`：`runTick` 先处理 queued packet 和 task，再执行受限 tick 循环。
- `Minecraft.java:2628-2639`：client task 的运行线程、identity wrapping 与可运行判断。
- `Minecraft.java:2884-2893`：`getTickTargetMillis` 以默认间隔为下限，冻结时仍返回默认值。
- `Minecraft.java:2165-2186`：disconnect 清理任务与连接，按 keepResourcePacks 决定包清理。
- `Minecraft.java:2210-2216`：退出世界时把 stopSound 交给 engine 更新，并清 player。

BBB 与 Vanilla 一样只有一条主线程循环：winit 每轮迭代的 `AboutToWait` 就是 `runTick`
的 tick 段，`RedrawRequested` 是 render 段。主线程独占 live world/input 与 step/render 调度；
Tokio 连接/服务任务和 rayon terrain mesh pool 仍在后台执行。对照的是状态所有权、先后关系与会话生命周期。

## 决策输入与状态所有权

| 输入/状态 | 唯一 owner | 消费者 |
| --- | --- | --- |
| window/control/services/render feedback 命令 FIFO | `CommandQueue` / `CommandInbox` | 下一次 step |
| tick deadline、WorldStore、input state、cursor capture、GUI scale | `SimulationCore` | step 与 render 借用 |
| control/MCP 只读视图 | `ControlViews`（仅 control 或 MCP 启用时） | control 读者 |
| 跨线程唤醒 | `LoopWake`（合并成一个 user event） | event loop |
| connection future 与取消信号 | `NetworkRuntime` | event loop 统一 join |
| resource generation 与 reload acknowledgement | simulation 和 native transaction 各自的阶段 | 下一次 step |
| 首个 RuntimeFailure 与 bounded suppressed 列表 | `crash_report` | 外层 runtime 保存 |

## 判断与处理顺序

1. startup 先建 event loop 与 `LoopWake`，再建命令队列并绑定 control sink，随后才暴露 producer。
2. window 事件在主线程入命令 FIFO；control/services completion 跨线程入队并唤醒 loop。
   网络事件使用独立的有界 FIFO，同样经 LoopWake 唤醒主循环。
3. `AboutToWait` 取走全部命令，`SimulationCore::step` 按 deadline、网络和命令判断是否推进；
   无事可做时不产生任何状态变化。
4. step 按网络事件和输入命令顺序改变 world 与 Screens。音频事件交给 audio sink；粒子 spawn
   进入 core 持有的 `bbb_client_world::particles::ParticleEngine` 并在 client tick 模拟，render 只提取。
   平台动作收进 `PlatformEffects`，step 返回后由同一 event loop 按序执行。
5. step 留下的 resource reload 在下一次 step 前完成，acknowledgement 作为下一次 step 的命令。
6. frame limiter 到期时请求 redraw；`RedrawRequested` 直接借用 world/input 渲染。
7. terminal close 先释放 cursor、请求网络断开，再 join network 并发布终态。

### 控制请求与命令队列

`bbb-client::control` 只拥有外部 DTO、只读 snapshot 和构造时绑定的
`SharedControlRequests` sink handle；control 不保存 command queue，也没有 sink 后装或替换阶段。
`bbb-client` 用同一个 `bbb-mcp` 传输层把每个 control 方法投影成 MCP tool（无参读方法同时是
`bbb://client/<方法>` resource），每次调用都走 `bbb_client::control::dispatch`，与行式 control 协议共用参数校验；
只允许 loopback 绑定，由 `--mcp-bind` 启用，agent 因此用同一套 MCP 同时管理客户端与服务端。

MCP 写调用与行式 control 共用 `dispatch_inner` 的参数校验和命令 FIFO。按键 enum、serde 名称、
parser 与 MCP schema 从同一张表派生；`input.text` 的 4096 上限按 UTF-16 单元计算。
行式 control 保留排队回执；MCP 等该请求经过原输入/网络 handler 且状态 publication 完成后，
返回 `processed: true` 与对应 `generation`。这只确认处理阶段；移动、挖放和背包结果仍通过
`world.client_local_player`、`world.probe_block`、`world.probe_blocks`（最多 256 格，支持
`skip_air`）、`world.probe_surface`（最多 256 列）、`world.crosshair_target`（含世界空间距离、
命中的面及相邻放置坐标 `adjacent_pos`）、`world.nearby_entities`（按距离排序，覆盖不可拾取的
掉落物 `minecraft:item` 及其物品名与数量）与 `world.client_inventory`（附带 `item_name`、
`menu_type` 并压缩默认空 `component_patch`）等真实状态确认。

面向实时自主游玩的控制与节拍原语同样经唯一命令 FIFO 执行：`input.focus` 支持 `capture_cursor`
在后台桌面窗口及无用户手势的 Headless Web 会话内建立逻辑光标捕获；`input.look` 与 `input.look_at`
直接按世界坐标或实体锚点（`center`/`eyes`/`feet`）转向并同步旧帧插值朝向，在焦点丢失、光标未捕获、
打开界面或行为准则遮罩时明确拒绝；`input.scroll` 沿滚轮路径驱动快捷栏与可滚动菜单；`input.key` /
`input.mouse_button` 支持 `hold_ticks`（1..=100）在指定 client tick 后自动释放（实时会话等待释放完成，
`--lockstep` 立即返回并在步进后自动释放，打开界面或断线时提前释放）；`sim.wait_ticks`（1..=100）
等待指定数量真实 client tick（`--lockstep` 下明确拒绝，改用 `sim.step`）。
pointer click 展开为 cursor、press、release，完成回执附在最后一个实际事件上。退出会关闭
inbox，拒绝未处理请求和未完成截图；`app.quit` 的最终退出以进程终态确认。

`renderer.screenshot` 沿既有 platform effect、GPU readback 与编码路径执行。surface 跳帧时
保留队首请求；成功后同一份编码 bytes 写文件并作为 MCP image content 返回，structuredContent
携带 `completed`、`path`、尺寸和 `mime_type`。编码或写盘失败只失败本次截图；真实 render fatal
仍由原退出路径处理。operation 日志只保存元信息，不保留图片 payload。超过 MCP 请求 deadline
的调用按既有 deferred 失败合同返回 operation URI；随后可确认文件完成和路径，需要图片时重新
请求截图，operation resource 不重发原图。

`bbb_client::simulation::commands::CommandQueue` 是 window、control、services 与 render
feedback 的唯一 production 写路径：一条 std mpsc FIFO，入队顺序即应用顺序，单个
`ControlWriteRequest` 由与 window 输入相同的 `apply_control_*` 处理。loop 退出后入队返回
`closed`，control dispatcher 把拒绝作为失败响应。reload/readiness/services completion 的
generation 判定仍只归原 consumer，队列不复制 session state。

render 直接借用 `SimulationCore::render_parts` 返回的 `SimulationView`。只有 control/MCP
启用时，event loop 才在每次有效 step 后通过一个 `ArcSwap` 发布 `ControlSnapshot`，
将运行时诊断和同 generation 的冻结 world 投影一起交给读者。未改变 world 时复用 world Arc。
`client_ui` 每次发布都从真实 Screens 与 world 协议事实派生，输入改变焦点或当前页不依赖重建 world。
每个查询只读取一次快照，冻结投影不作为动作输入。

### MCP 游玩与回归录制

`scripts/client_replay.py --transport mcp` 复用原 `Client`、`Scenario`、holding proxy 与
`Session`，所有客户端输入、查询、step 和截图经真实 HTTP MCP；默认 `control` 入口保持可用。
`--lockstep` 可配 `--control` 或 `--mcp-bind`。MCP TCP 接收端启用 `TCP_NODELAY`，避免持久连接
拆分 header/body 时的小包延迟；HTTP、执行和 SSE deadline 保留既有预算。

`record` 在 capture 中保存 `scenario.json`：窗口、实际有序输入、客户端启动选项与截图 masks。
新 capture 的回放只读这份计划，旧 capture 缺少计划时保留原场景 factory。每次回放输出也保存
实际计划，比较双方计划并使用各自输出的 masks，不从后来覆盖的 capture 偷换输入或基准。
显式动作的结果保存在 `actions.jsonl`，MCP 工具 schema 保存在 `mcp-tools.json`。

真实闭环入口：

```sh
cargo dev e2e client --out /tmp/bbb-mcp-playable
```

它通过同一录制器启动进服、移动、挖放、背包 pointer click 和截图，读取客户端事实及服务端
条件命令的结构化结果，并运行两轮录制回放比较。capture 副本、请求结果、服务端回执、截图及
报告保存在输出目录。单独回放已保存的输入可用：

```sh
cargo dev python scripts/client_replay.py --transport mcp replay --out /tmp/bbb-mcp-replay mcp_play_loop
```

这是 harness 代理连接的回归会话录制：保存 packet 窗口、输入计划、音效事件、图片与本地文件
验证材料；没有第二套 Rust recorder/player。它不生成 MP4/WebM，不编码连续视频或声音流，也不
宣称能后台录制任意已运行会话。

多场景零作弊自主游玩闭环验证入口（覆盖生存开局砍树合成木镐、挖矿熔炼插火把、战斗进食、箱子与村民交易、
建房写告示牌、以及界面恢复与死亡重生捡装备 S1–S6，服务端 MCP 仅负责建场与独立裁决，游玩全程仅调用客户端
`/mcp`，并支持 `--client native` 与 `--client web` 及外部 LLM 盲测 `--mode serve`）：

```sh
cargo dev python scripts/mcp_agent_scenarios.py --mode policy --client native --out /tmp/bbb-mcp-scenarios-native
cargo dev python scripts/mcp_agent_scenarios.py --mode policy --client web --out /tmp/bbb-mcp-scenarios-web
```

### 原生视频导出（Windows）

Windows 客户端的 **F9** 切换视频录制，文件保存在 `--game-dir` 下的 `videos/`。
游戏内本地消息提示开始和保存路径。也可启动时传
`--record-video <path.mp4> --record-fps 30 --record-bitrate 8000000`。
正常退出会完成当前文件；已有同名文件会明确报错。

control/MCP 共用三个入口：

| 方法 | 参数与结果 |
| --- | --- |
| `renderer.recording_start` | `path` 必填；`fps` 默认 30（1..120），`bitrate` 默认 8000000 bit/s |
| `renderer.recording_status` | 状态、路径、固定输出尺寸、接受/丢弃帧数、最终编码帧数、时长、编码错误 |
| `renderer.recording_stop` | 排空已接受的帧并完成 MP4；MCP 完成结果只在文件封装与快照发布之后返回 |

输出为 H.264 视频与 AAC 192 kbit/s、48 kHz 双声道音轨，编码器由 Windows Media Foundation 提供。
音轨由游戏 OpenAL 引擎生成，保留游戏音量、音高和空间定位，同时保持正常扬声器播放；
不会录入麦克风或其它应用的声音。录制开始时接续已经播放的声音，静音期间仍输出连续音轨。
音频和视频共享录制时钟，正常停止会排空音频并完成两个轨道。
renderer 的同一 GPU readback 去除 row padding 并转换到 RGBA，截图与录像同时请求时只读取一次。
编码线程通过容量为 2 的帧队列接收像素；满队列丢帧并记数，按实际时间轴重复上一帧，
保持固定帧率输出的时长。Media Foundation 的默认节流只阻塞编码线程。
音频通过独立容量为 128 的 PCM 队列交给同一编码线程，每块最多 1024 个双声道采样帧；
音频队列溢出或编码失败会停止录制并报告错误，正常游戏声音继续播放。
窗口尺寸变化时等比缩放并加黑边，输出尺寸固定为开始时 framebuffer 的偶数宽高。
当前读回会等待 GPU，该成本只在录制采样帧发生；其它平台启动录制会明确返回不支持。

`world.client_ui` 保留查询字段的既有形状和含义，初始快照也具有完整对象形状；Dialog 内容、
编辑值、链接与等待信息来自该次 native 投影，不写回 `WorldStore`。probe 没有 UI 消费者，
只丢弃 ScreenRequest FIFO，`ProbeReport.world` 不进入 CLI JSON。

## 生命周期和 fallback

启动创建平台与资源基础，连接依次经过 handshake、configuration、play。configuration
建立 registry/data projection；play 建立 world。resource reload 可跨 play 独立发生，
但一帧 renderer input 必须引用同一 resource generation。断线先停止新 packet 应用，
再清理 world/backend；control 只能看到最后一个明确标记状态的 snapshot。

client tick 的唯一 timer 在 `SimulationCore`。正常世界以 50ms 为下限，低于 20 TPS 时使用
server tick rate，freeze 仍保持 50ms client tick；外部多人 pause screen 不冻结 client/world tick。
与 Vanilla `DeltaTracker.Timer` 相同，一次 step 最多跑 10 tick，多出的到期 tick 丢弃；
render 用上次 step 的 residual 投影到帧开始时刻得到 partial tick，freeze 时为 1。render 侧
tick 消费者按距上一帧的 tick 数推进。loop 以 `WaitUntil(min(下个 tick, 下次 redraw))` 睡眠，
网络、control 与 `sim.step` 经同一个合并的 `EventLoopProxy` user event 唤醒；lockstep 与
不限帧时为 `Poll`。terminal close 固定由 event loop 请求断开、join network，再发布
`running=false/connected=false`，不存在第二条 detached shutdown 路径。

### 远端断线状态页边界

`bbb-client-net::DisconnectReason` 是远端 packet/transport terminal outcome 的唯一 bounded
presentation 输入：event channel 保持 FIFO，主动本地 close 不合成 reason，裸 event channel 关闭也不
伪造远端状态页。simulation 在同一步、应用 window/control input 前只调用
`WorldStore::end_connection_session`：它清除 client level、registry、server-pack、listener
presentation 与 transfer-only carry，再由 `ClientInputState` 发布 generation-tagged
`DisconnectedScreenRuntime`；Login/尚未建 phase 使用 `connect.failed`，Configuration/Play 使用
`disconnect.lost`。关闭 receiver 后不会保留一个永久 productive wake source。

声音 teardown 沿用同一 step 的音频顺序，而不建立断线专用 backend 路径：Login/connect
failure 对应 Vanilla `ConnectScreen` 的 `stopSound=false`，不提交 stop；Configuration/Play 在
level、registry 与 server-pack session 清理后，向既有 `AudioEventSink` 追加一次无 category/name
过滤的 stop-all。它排在本次 step 先行 packet audio 之后、resource reload apply 之前；随后
空 tickable projection 清除旧 generation 的 entity-bound identity。`bbb-client-audio` 只按序消费，
不反向读取 phase、world 或 pack，stop-all 同时释放 active、streaming 与 delayed playback owner。

`ClientInputState` 唯一拥有 focus、hover、Esc-ignore、details/back action 和 bounded control
diagnostics；render 只读借用它。renderer 只拥有居中 layout、200px button、
menu-background GPU generation 与 generation-tagged hit target，不反向读取 socket、world 或 pack。
HTTP(S) 和本地 report path 经 step 之后的 platform effect 执行；CLI-direct 产品的
back action 进入正常 app shutdown。live kick gate 必须同时观察服务器 receipt、
`net.connected=false`、仍存活进程及 `app.status.app.disconnected_screen`，随后才以 `app.quit` 退出。
非 in-game screen 的 panorama/overlay 由 pack 的 immutable `PanoramaCatalog`、native resource
transaction 与 renderer-private cube generation 单向投影；断线页只携带 realtime rotation，不能
反向读取 pack。renderer 固定在 blur/menu background/widgets 之前提交 cube 与 overlay，reload
失败保留上一完整 GPU generation，startup catalog 缺失则退回 menu background。

### Connection replacement 与 reconfiguration

`WorldStore` 是 connection/session replacement 的唯一 canonical owner。新 transport 的
`Connected` edge 调用 `begin_connection_session`；play→configuration 不经过此 edge，因此保留
`CommonListenerCookie` 的 registry、feature、brand、chat、report 与 links。`StartConfiguration`
先 flush chat acknowledgement，再恰好清一次 level 并在同一 step 内 stop-all；后续
`StateChanged(Configuration)` 只建立 registry candidate，不会再次推进 level generation。初次
login→configuration 没有旧 level，也不制造 audio stop。

configuration 与 play 的 Transfer 共享一个 teardown：记录 target 后清旧 level、stop-all，
但对应 Vanilla `Minecraft.disconnect(..., isTransferring=true)` 保留已下载/激活 server pack。
replacement `Connected` 重建 built-in registry session，并清旧 brand/custom payload/report/links、
feature flags 与 Code of Conduct packet state；同时保留 `TransferState` 的 cookies、seen players、
insecure-chat warning guard，以及 `ConnectScreen` 单独传递的 `ServerData`。`ServerData` 拥有的
accepted Code of Conduct hash 也跨 transfer 保留；`last_transfer` 仅是单调 packet diagnostic，
连接链终止时由 `end_connection_session` 清除。server-pack async result 仍以 push sequence/reload
generation 判 stale；transfer 不触发无变化 reload，普通 terminal disconnect 则原子提交空 stack。

### Task handoff、resource recovery 与 fatal join

Vanilla `Minecraft.getRunningThread/wrapRunnable/shouldRun` 在外部 client path 的语义是所有 client
task 回到唯一 game thread、存活期间始终可执行。BBB 的对应 owner 就是 event loop 所在的主线程与唯一
`CommandQueue`；不存在第二 task queue 或第二个状态线程。

resource reload 在产生它的 step 之后、下一次 step 之前完成：同 generation 的 CPU prepare、
renderer/audio transaction 与 acknowledgement 入队一次做完，后续 disconnect/session replacement
因而不能越过旧 reload。acknowledgement 先核对 generation，再应用 world 默认值，最后确认资源包成功；
旧 generation 的结果不修改 world。world 解码失败保留旧默认值，失败 stack 只做一次 recovery reload；recovery 完成后
由 world-owned 单调 UI fact 投影 Vanilla `PACK_LOAD_FAILURE` system toast。二次失败冻结相同 stack、
防止无界热循环，并依次关闭 transport、替换 connection generation、停止 audio、发布 bounded
disconnected screen 和释放 cursor；不会留下仍可交互的半失败 session。download/materialization 使用
UUID+push-sequence，reload completion 使用 generation，任一旧结果都只做清理或拒绝。

step panic、network task panic 与 network join timeout 都转成 typed `RuntimeFailure`，step panic
附带当时 world 的 level 细节。第一 failure 是 canonical report owner，后续 shutdown failure 只进入
有界 suppressed 列表，report 原子 save-once 后以非零状态退出。

### 网络任务退出

`bbb-client::network_runtime::NetworkRuntime` 是 external connection future 的唯一 lifecycle
owner。startup 只构造 `ConnectionOptions`；simulation 取得有界 net event receiver 与 command
sender，既看不到 task handle，也不拥有 terminal close state。connection 直接写入这一条 event
channel，每次交付都经 `LoopWake` 唤醒 loop；receiver 关闭时立即停止，不把旧 session event 留给
下一进程。

gameplay `NetCommand::Disconnect` 仍由 simulation 按正常 session 语义发送。terminal close 另有
独立 one-shot signal，所以 command queue 已满、已关闭或 simulation fatal 都不能阻止 connection
future 被取消并通过 drop 关闭 socket。连接自然结束、服务器断开或 terminal signal 最多产生一个
最终 `Disconnected` event；重复 shutdown 幂等。

`LoopExiting` 的固定顺序是：释放 cursor capture、仍在运行时请求网络断开、signal 并 join network
task、发布最终 `running=false/connected=false` snapshot，随后 event-loop closure 才释放
renderer/audio/window 和 Tokio runtime。network task 有 5 秒总 deadline；task panic/join failure
进入既有 `RuntimeFailure`/crash-report owner，超时会 abort 并 reap 后同样报告。无连接、正常远端
结束和成功 close 不产 fatal report。`JoinSet` drop 只负责 event-loop 建立前发生 startup failure
时的 abort safety net，不能替代 production `shutdown_and_join`。

### 致命错误与报告

`bbb-client::crash_report` 是 fatal diagnostics 的唯一 owner。startup `Result`、main panic、
step panic、network join error，以及 resize、frame 和 platform event-loop fatal
只提交 `RuntimeFailure`；首个 failure 保持 canonical，最多 8 个后续 failure 作为 suppressed
diagnostics。worker、renderer、control 或 snapshot 都不直接写报告。

报告复刻 Vanilla `CrashReport`/`Minecraft.crash` 的 header、UTC time、description、error、
head/thread/backtrace、category、system details 和 save-once 判断。单字段、category/entry 数、
suppressed 数、retained bytes（512 KiB）和最终 UTF-8 文本（1 MiB）都有明确预算，Unicode
截断会写入诊断。目录只来自启动参数 `--crash-report-dir`，默认是 game-directory 语义下的
`crash-reports`；安全时间戳文件名不含 server/user 输入，临时文件 sync 后用同目录 hard-link
原子 no-clobber 发布，重名追加序号。

fatal event 先请求 event-loop exit；`LoopExiting` join network，closure drop 释放
renderer/audio/window，`event_loop.run_app` 返回后外层 runtime 才持久化。保存成功退出 1，写失败
仍把 bounded report 输出 stderr 并退出 2；正常 close/quit、probe/status/build-info 成功不产报告。
focused process test 与 production-binary acceptance 都验证“失败生成一份且非零退出，下一进程
成功且不覆盖旧报告”。

## BBB 实现映射

- `bbb_client::simulation`：step、tick deadline 与 render 借用视图。
- `bbb_client::runtime::simulation_step`：client tick 与网络/输入推进。
- `bbb_client::runtime::events::dispatcher`：每个网络 event 后消费 ScreenRequest。
- `bbb_client::runtime::frame`：一帧提取与当前 Screen HUD；`render_extract` 定义帧值和提交辅助。
- `bbb_client::simulation::commands`：命令 FIFO 与 loop 唤醒。
- `bbb_client::simulation::publication`：control/MCP 读视图的原子发布。
- `bbb_client::network_runtime`：socket task pair 的生命周期。
- `bbb_client::resource_reload`：资源 transaction 与恢复。
- `bbb_client::crash_report`：有界报告与 save-once。
- `bbb_client::control`、`bbb_client::mcp`：控制 DTO 与 MCP 投影。

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| task 顺序与单一状态写入 | `Minecraft.runTick` / game thread | 命令队列、主线程 step | `aligned` | FIFO 测试与 lockstep 回放 |
| tick 时钟 | `getTickTargetMillis` / `DeltaTracker.Timer` | simulation timer | `aligned` | 10 tick 上限与 partial tick 测试 |
| 会话替换和清理 | `disconnect` / listener replacement | WorldStore 与同一 step 的音频顺序 | `aligned` | session protocol、断线音频测试 |
| 资源失败恢复 | `Minecraft` reload lifecycle | resource transaction | `aligned` | generation 与恢复测试 |
| terminal close | client shutdown | event loop、NetworkRuntime | `aligned` | cancellation、join 与 process evidence |

## 结构性不变量

所有写请求只有一条命令 FIFO；snapshot 不能回写世界。完成消息必须保留 session 或
resource generation 判定。shutdown 不依赖可被业务请求塞满的 queue 空位。第一份 fatal
failure 拥有报告，不能用后续 join failure 覆盖。

## 聚焦测试（focused tests）和真实证据

`bbb_client::simulation::commands` 的 `commands_drain_in_fifo_order_and_wake_once_until_rearmed`
断言多 producer 的应用顺序、合并唤醒与关闭后拒绝；`bbb_client::simulation` 覆盖 partial tick
投影，`runtime::tests` 的 `entity_client_tick_catch_up_runs_ten_ticks_and_drops_the_rest` 覆盖
tick 追赶上限。`bbb_client::network_runtime` 的 `shared_deadline_aborts_and_reaps_a_blocked_final_event`
与 `closed_event_receiver_does_not_stall_join` 覆盖独立取消和 join。整条循环由
`scripts/client_replay.py` 的 lockstep 回放（serverbound packet、音频、截图逐项比较）验收；
实际进程退出与断线显示仍需对应 runtime/process evidence。

定向入口：`cargo dev test -p bbb-client simulation::`，
`cargo dev test -p bbb-client network_runtime::`。修改资源或 session 行为时另选其 focused filter，
不把纯队列测试当作 reload 或 live kick 验收。

## 当前差异

本页记录执行合同，不登记实时覆盖结论；硬件显示、live session 和外部验收的有效性仍由
live 门禁判定。

## 账本归属

沿用 `lifecycle-client-runtime` 及相邻机制分类；未关闭差异只写
[客户端账本](../../../unsupported-client-features.md)，自有控制扩展写
[客户端扩展](../../../client-extensions.md)，推进原则见 [AGENTS.md](../../../../AGENTS.md)。

## 精确重开条件

新增 producer、改变命令应用顺序或唤醒语义、加入 session replacement 分支、改变 reload
acknowledgement 或 join 顺序，以及 Vanilla 上述 owner 的决策变化，均须重核对应合同与测试。

## 相关页面和源码索引

- [客户端边界](../../architecture/client-boundaries.md)：状态分类和 crate 所有权。
- [协议状态流](../protocol/client-state-flow.md)：packet decode、apply 与 projection。
- [服务端 MCP](../control/server-mcp-administration.md)：服务端查询经 tick 的不同路径。
- [离线资产可信链](../resources/client-offline-asset-trust.md)：资源输入和 generation。

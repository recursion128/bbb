# 客户端运行时、控制请求与会话退出

机制 ID：`lifecycle-client-runtime`

## 机制目的和边界

本页定义客户端模拟调度、外部控制与 MCP 写入、会话替换、资源重载恢复以及进程退出顺序。世界内部状态应用见[客户端世界生命周期](../world/client-world-lifecycle.md)，音频播放消费见[客户端音频链路](../audio/client-audio-pipeline.md)。窗口平台、网络传输与各渲染/音频后端各自拥有底层资源，但只有主线程模拟层决定客户端状态变化与副作用执行顺序。

## Vanilla 26.1 权威实现

- `Minecraft.java:1232-1262`：`runTick` 先处理排队数据包与主线程任务，再执行受上限约束的 tick 循环。
- `Minecraft.java:2628-2639`：客户端主线程判定、任务包装与可运行检查。
- `Minecraft.java:2884-2893`：`getTickTargetMillis` 以默认 50 ms 间隔为下限，世界冻结时仍返回默认间隔。
- `Minecraft.java:2165-2186`：断开连接时清理挂起任务与连接，并按 `keepResourcePacks` 决定是否卸载服务器资源包。
- `Minecraft.java:2210-2216`：退出世界时提交 `stopSound` 并清空玩家引用。

BBB 与 Vanilla 同样只有一条主线程循环：winit 每轮迭代的 `AboutToWait` 对应 `runTick` 的模拟推进阶段，`RedrawRequested` 对应渲染阶段。主线程独占活动世界、输入状态与 step/render 调度；Tokio 网络/服务任务与 Rayon 地形构网线程池在后台并发运行。两者对照的核心是状态所有权、先后顺序与会话生命周期。

## 决策输入与状态所有权

| 输入/状态 | 唯一所有者 | 消费者 |
| --- | --- | --- |
| 窗口、控制、服务与渲染反馈命令 FIFO | `CommandQueue` / `CommandInbox` | 下一次 `SimulationCore::step` |
| tick 截止时间、`WorldStore`、输入状态、光标捕获与 GUI 缩放 | `SimulationCore` | 模拟步进与渲染只读借用 |
| 控制接口与 MCP 只读快照 | `SharedSnapshot` / `ControlSnapshot`（含 `ControlWorldSnapshot`，仅在启用 control 或 MCP 时发布） | 控制与 MCP 读接口 |
| 跨线程事件循环唤醒 | `LoopWake`（合并为单个用户事件） | 主事件循环 |
| 网络连接 future 与独立取消信号 | `NetworkRuntime` | 事件循环统一退出并 join |
| 资源代际与重载确认 | 模拟层与客户端事务各自阶段 | 下一次 `SimulationCore::step` |
| 首个 `RuntimeFailure` 与有界抑制列表 | `crash_report` | 外层运行时统一落盘 |

## 判断与处理顺序

1. 启动先初始化事件循环与 `LoopWake`，再构建命令队列并绑定控制请求入口，最后暴露给各产生方。
2. 窗口事件在主线程直接入队；控制接口与后台服务的完成回调跨线程入队并触发 `LoopWake`。网络事件走独立有界 FIFO，同样通过 `LoopWake` 唤醒主循环。
3. `AboutToWait` 取出全部待处理命令，`SimulationCore::step` 依据 tick 截止时间、网络事件与命令判断是否推进；无事可做时不产生任何状态变更。
4. 模拟步按网络事件与输入命令到达顺序更新 `WorldStore` 与 `Screens`。音频事件交付 `AudioEventSink`；粒子生成进入模拟核心持有的 `bbb_client_world::particles::ParticleEngine` 并在客户端 tick 推进，渲染阶段只做提取。平台动作收集进 `PlatformEffects`，步进返回后由同一事件循环按序执行。
5. 模拟步产生的资源重载请求在下一次步进之前完成，重载确认作为下一模拟步的输入命令入队。
6. 帧率限制器到期时请求重绘；`RedrawRequested` 直接借用当前世界与输入状态执行渲染。
7. 终态关闭先释放光标捕获、请求网络断开，再 join 网络任务并发布最终快照。

### 控制请求与命令队列

`bbb-client::control` 仅拥有外部 DTO、只读快照类型以及构造时即绑定的 `SharedControlRequests` 发送句柄；控制模块不持有命令队列本体，也不存在延迟装配或替换发送端阶段。`bbb-client` 通过共享 `bbb-mcp` 传输层将每个控制方法映射为 MCP 工具（无参只读方法同时暴露为 `bbb://client/<方法>` 资源），每次调用均经 `bbb_client::control::dispatch` 与行式控制协议共用参数校验；仅允许 loopback 绑定，由 `--mcp-bind` 启用，使自动化代理可用同一套 MCP 协议管理客户端与服务端。

MCP 写入调用与行式控制共用 `dispatch_inner` 的参数校验与命令 FIFO。按键枚举、serde 名称、解析器与 MCP schema 派生自同一张表；`input.text` 的 4,096 长度上限按 UTF-16 单元计算。行式控制保留排队回执；MCP 等待请求经过原输入或网络处理器并完成状态快照发布后，返回 `processed: true` 与对应 `generation`。这仅确认处理阶段完成；位移、挖放与背包变化仍须通过 `world.client_local_player`、`world.probe_block`、`world.probe_blocks`（最多 256 格，支持 `skip_air`）、`world.probe_surface`（最多 256 列）、`world.crosshair_target`（含世界空间距离、命中面与相邻放置坐标 `adjacent_pos`）、`world.nearby_entities`（按距离排序，覆盖不可拾取掉落物 `minecraft:item` 及其物品名与数量）与 `world.client_inventory`（附带 `item_name`、`menu_type` 并压缩默认空 `component_patch`）等真实状态确认。

面向实时自主游玩的控制与节拍原语同样经过唯一命令 FIFO 执行：`input.focus` 支持 `capture_cursor` 在后台桌面窗口及无用户手势的 Headless Web 会话内建立逻辑光标捕获；`input.look` 与 `input.look_at` 直接按世界坐标或实体锚点（`center`/`eyes`/`feet`）转向并同步旧帧插值朝向，在焦点丢失、光标未捕获、打开界面或行为准则遮罩时明确拒绝；`input.scroll` 沿滚轮路径驱动快捷栏与可滚动菜单；`input.key` / `input.mouse_button` 支持 `hold_ticks`（`1..=100`）在指定客户端 tick 后自动释放（实时会话等待释放完成，`--lockstep` 立即返回并在步进后自动释放，打开界面或断线时提前释放）；`sim.wait_ticks`（`1..=100`）等待指定数量真实客户端 tick（`--lockstep` 下明确拒绝，改用 `sim.step`）。指针点击展开为移动、按下、释放三个事件，完成回执绑定在最后一个实际事件上。退出时关闭命令收件箱，拒绝未处理请求与未完成截图；`app.quit` 的最终退出以进程终态为准。

`renderer.screenshot` 沿既有平台副作用、GPU 像素读回与 PNG 编码路径执行。窗口 surface 跳帧时保留队首请求；成功后将同一份编码字节写盘并作为 MCP 图像内容返回，`structuredContent` 携带 `completed`、`path`、尺寸与 `mime_type`。编码或写盘失败仅令本次截图失败，真实渲染致命错误仍走原有退出路径。操作日志仅记录元信息，不留存图片载荷。超过 MCP 超时的调用按既有延迟失败契约返回 operation URI，后续可查询落盘结果，需要图像时重新发起请求。

`bbb_client::simulation::commands::CommandQueue` 是窗口、控制接口、服务与渲染反馈的唯一生产写入口：一条 `std::sync::mpsc` FIFO，入队序即应用序，单条 `ControlWriteRequest` 走与窗口输入相同的 `apply_control_*` 处理器。循环退出后入队返回 `closed`，控制分派器将其转为失败响应。重载、就绪度与服务完成的代际校验仍归各自消费者，队列本身不复制会话状态。

渲染直接借用 `SimulationCore::render_parts` 返回的 `SimulationView`。仅当启用控制接口或 MCP 时，事件循环才在每次有效模拟步后通过 `ArcSwap` 发布 `ControlSnapshot`，将运行时诊断与同代际冻结世界投影（`ControlWorldSnapshot`）一并交付读者；世界未变更时复用原世界 `Arc`。`client_ui` 每次发布均从真实 `Screens` 与世界协议事实派生，焦点或当前页切换不依赖重建世界快照。每次只读查询只读取一次快照，冻结投影绝不回流为动作输入。

### MCP 游玩与回归录制

`scripts/client_replay.py --transport mcp` 复用既有 `Client`、`Scenario`、代理与 `Session`，全部输入、查询、步进与截图均走真实 HTTP MCP，同时保留默认行式控制入口。`--lockstep` 可配合 `--control` 或 `--mcp-bind` 使用。MCP 服务端套接字启用 `TCP_NODELAY`，避免持久连接拆分报文头与正文引起的小包延迟，HTTP、执行与 SSE 超时保留既有预算。

`record` 在 capture 目录中保存 `scenario.json`，记录数据包窗口、有序输入、启动选项与截图掩码。新 capture 回放直接读取该计划，旧 capture 缺计划时回退原场景工厂；每次回放同样保存自身实际计划，对比双方计划并使用各自输出的掩码。显式动作结果保存在 `actions.jsonl`，MCP 工具 schema 保存在 `mcp-tools.json`。

真实闭环入口：

```sh
cargo dev e2e client --out /tmp/bbb-mcp-playable
```

该入口通过同一录制器执行进服、移动、挖放、背包指针点击与截图，读取客户端状态与服务端结构化回执，并完成两轮录制回放对比。单独回放已保存输入可用：

```sh
cargo dev python scripts/client_replay.py --transport mcp replay --out /tmp/bbb-mcp-replay mcp_play_loop
```

该工具用于代理连接的回归会话录制（保存数据包窗口、输入计划、音频命令日志、截图与本地文件验证材料），不存在第二套 Rust 录制回放器；它不编码 MP4/WebM 视频流，也不宣称后台录制任意已运行会话。

多场景零作弊自主游玩闭环验证入口（覆盖生存开局砍树合成木镐、挖矿熔炼插火把、战斗进食、箱子与村民交易、建房写告示牌以及界面恢复与死亡重生捡装备 S1–S6；服务端 MCP 仅负责建场与独立裁决，游玩全程仅调用客户端 `/mcp`，支持 `--client native` 与 `--client web` 及外部 LLM 盲测 `--mode serve`）：

```sh
cargo dev python scripts/mcp_agent_scenarios.py --mode policy --client native --out /tmp/bbb-mcp-scenarios-native
cargo dev python scripts/mcp_agent_scenarios.py --mode policy --client web --out /tmp/bbb-mcp-scenarios-web
```

### 原生视频导出（Windows）

Windows 客户端按 **F9** 切换视频录制，输出文件保存在 `--game-dir` 下的 `videos/` 目录，并在游戏内通过本地消息提示录制开始与保存路径。也可在启动时传入 `--record-video <path.mp4> --record-fps 30 --record-bitrate 8000000`。正常退出会自动封口完成当前录像；目标路径已存在同名文件时直接报错而不覆盖。

控制接口与 MCP 共用三个方法：

| 方法 | 参数与结果 |
| --- | --- |
| `renderer.recording_start` | `path` 必填；`fps` 默认 30（`1..120`），`bitrate` 默认 `8000000` bit/s |
| `renderer.recording_status` | 状态、路径、固定输出尺寸、接受/丢弃帧数、最终编码帧数、时长与编码错误 |
| `renderer.recording_stop` | 排空已接受帧并封装 MP4；MCP 完成回执仅在文件封口与快照发布后返回 |

输出封装为 H.264 视频轨与 AAC 192 kbit/s、48 kHz 双声道音轨，编码器由 Windows Media Foundation 提供。音轨直接由游戏 OpenAL 引擎采样生成，完整保留音量、音高与空间定位且不中断正常扬声器播放，不采集麦克风或其他应用音频。录制开始时自然衔接已在播声音，静音时段持续输出连续音轨；音视频共享录制时钟，正常停止时排空音频并同时完成双轨封口。

渲染器复用同一 GPU 读回去除行填充并转为 RGBA，截图与录像同帧请求时只读回一次。编码线程通过容量为 2 的帧队列接收画面，队列满时丢帧计数并按时间轴重复上一帧以维持恒定帧率时长，Media Foundation 节流仅阻塞编码线程。音频经独立容量为 128 的 PCM 队列送入同一编码线程，每块最多 1,024 个双声道采样帧；音频队列溢出或编码失败时中止录制并报错，不影响游戏正常发声。窗口缩放时画面等比适配并加黑边，输出尺寸锁定为开始录制时帧缓冲的偶数宽高。当前 GPU 读回开销仅发生在录像采样帧，其他平台调用录像接口明确返回不支持。

`world.client_ui` 保持既有查询结构，初始快照也具备完整字段；对话框内容、编辑值、链接与等待信息均来自本次投影，不写回 `WorldStore`。probe 模式无 UI 消费者，仅丢弃 `ScreenRequest` FIFO，`ProbeReport.world` 不进入 CLI JSON 输出。

## 生命周期和 fallback

启动阶段先建立平台窗口与资源基础，连接按 `Handshake → Configuration → Play` 依次推进：Configuration 建立注册表与数据投影，Play 建立世界。资源重载可在 Play 期间独立发生，但同一帧渲染输入必须全部引用同一资源代际。断线时先停止应用新数据包再清理世界与后端，控制接口只能观测到明确标记终态的最后一份快照。

客户端 tick 唯一时钟位于 `SimulationCore`：正常世界步长下限为 50 ms，低于 20 TPS 时跟随服务端 tick 速率，世界冻结时仍保持 50 ms 客户端 tick；外部多人暂停菜单不冻结服务端或客户端世界 tick。与 Vanilla `DeltaTracker.Timer` 一致，单步最多追赶 10 tick，超出部分丢弃；渲染使用上次步进残余时间投影到帧起始时刻得出 `partial_tick`，冻结时固定为 `1.0`。渲染侧按距上一帧经过的 tick 数推进视觉时钟。事件循环在限帧或空闲时以 `WaitUntil(min(下个 tick, 下次重绘))` 休眠，网络、控制与 `sim.step` 经同一合并用户事件唤醒；`--lockstep` 或不限帧时采用 `Poll`。终态关闭固定由事件循环发起断连、join 网络任务并发布 `running=false/connected=false`，不存在第二条脱离事件循环的关闭路径。

### 远端断线状态页边界

`bbb-client-net::DisconnectReason` 是远端数据包或传输层终态结果的唯一有界呈现输入：事件通道保持 FIFO，本地主动关闭不伪造断线原因，裸事件通道关闭也不伪造远端断线页。模拟层在同一模拟步且处理窗口/控制输入前仅调用一次 `WorldStore::end_connection_session`，清除客户端世界、注册表、服务器资源包、监听器呈现与仅限转服携带的状态，随后由 `ClientInputState` 发布带代际标记的 `DisconnectedScreenRuntime`：Login 或尚未建立阶段使用 `connect.failed`，Configuration/Play 使用 `disconnect.lost`。关闭事件接收端后不残留持续唤醒源。

断线时的声音清理沿用同一步音频顺序，不另建专用路径：Login/连接失败对应 Vanilla `ConnectScreen` 的 `stopSound=false`，不发送停止命令；Configuration/Play 在清理世界、注册表与服务器包会话后，向既有 `AudioEventSink` 追加一次无过滤的 `stop-all`。该停止命令排在本步先行数据包音频之后、资源重载应用之前，随后空 tickable 投影清除旧代际实体绑定身份。`bbb-client-audio` 只按序消费，不反向读取连接阶段、世界或资源包，`stop-all` 同时释放活跃、流式与延迟播放声源。

`ClientInputState` 唯一拥有断线页焦点、悬停、`Esc` 忽略、详情/重连/退出动作与有界控制诊断，渲染阶段只读借用。渲染器只负责居中布局、200 px 按钮、菜单背景 GPU 代际与带代际标记的命中目标，不反向读取套接字、世界或资源包。HTTP(S) 链接与本地报告目录经模拟步之后的 `PlatformEffects` 执行。桌面端 `DisconnectedScreen` 在可选报告按钮之后同时提供 `Reconnect`（`HudDisconnectedScreenButton::Reconnect` → `DisconnectedScreenPlatformRequest::Reconnect`，按当前会话保存的 `ConnectionOptions` 关闭既有网络任务并重启 `NetworkRuntime`、关闭断线页恢复连接）与 `Back` / `Quit Game`（`DisconnectedScreenPlatformRequest::Exit`，进入正常应用关闭流程）；Web 端不提供桌面退出按钮，`HudDisconnectedScreenButton::Back` 直接映射为 `DisconnectedScreenPlatformRequest::Reconnect` 触发页面重连。联机踢出验收需同时观测服务端回执、`net.connected=false`、客户端进程存活及 `app.status.app.disconnected_screen`，再经 `app.quit` 正常退出。

非游戏内屏幕的全景图与覆盖层沿「不可变 `PanoramaCatalog` → 客户端资源事务 → 渲染器私有立方体贴图代际」单向流动；断线页仅携带实时旋转角，不反向读取资源包。渲染器固定在模糊、菜单背景与控件之前绘制立方体与覆盖层；重载失败保留上一完整 GPU 代际，启动缺少目录时回退菜单背景。

### 连接替换与重配置

`WorldStore` 是连接与会话替换的唯一权威所有者。新传输连接的 `Connected` 边沿调用 `begin_connection_session`；Play → Configuration 重配置不经过该边沿，因此保留 `CommonListenerCookie` 中的注册表、特性标志、品牌、聊天状态与服务器链接。`StartConfiguration` 先刷新聊天确认，再恰好清理一次世界并在同一模拟步提交 `stop-all`；紧随其后的 `StateChanged(Configuration)` 仅初始化注册表候选，不再重复递增世界代际。首次 Login → Configuration 没有旧世界，也不触发音频停止。

Configuration 与 Play 阶段的转服（Transfer）共用同一套清理逻辑：记录转服目标后清理旧世界并提交 `stop-all`，同时对应 Vanilla `Minecraft.disconnect(..., isTransferring=true)` 保留已下载并激活的服务器资源包。新连接的 `Connected` 边沿重建内建注册表会话，清理旧品牌、自定义载荷、链接、特性标志与行为准则包状态，同时保留 `TransferState` 中的 cookie、已见玩家、不安全聊天警告标记以及 `ConnectScreen` 独立传递的 `ServerData`（其内已接受的行为准则哈希也跨转服保留）。`last_transfer` 仅作为单调数据包诊断，在连接链最终终止时由 `end_connection_session` 清除。服务器资源包异步结果仍以推送序号与重载代际判过期；转服不触发无变化重载，而普通终态断开则原子提交空资源包栈。

### 任务交接、资源恢复与致命错误回收

Vanilla `Minecraft.getRunningThread`/`wrapRunnable`/`shouldRun` 在外部客户端路径上的核心语义是所有客户端任务均回到唯一主线程并在存活期间有序执行。BBB 的对应所有者即事件循环所在主线程与唯一 `CommandQueue`，不存在第二个状态写入线程或平行任务队列。

资源重载在产生请求的模拟步之后、下一次模拟步之前完成：同代际 CPU 准备、渲染器/音频事务提交与确认入队一气呵成，后续断连或会话替换不得越过旧重载屏障。确认处理先核对代际号、再应用世界默认值、最后向服务器确认资源包成功，陈旧代际结果不修改世界。世界默认值解码失败保留旧值，失败资源包栈仅执行一次恢复重载；恢复完成后由世界层单调 UI 事实驱动 `PACK_LOAD_FAILURE` 系统提示。若恢复重载再度失败，则冻结相同栈防止死循环热重试，并依次关闭传输、切换连接代际、停止音频、打开有界断线页并释放光标，绝不残留半失效可交互会话。下载/物化按「UUID + 推送序号」、重载完成按代际号过滤过期结果。

模拟步 panic、网络任务 panic 与网络退出 join 超时统一转化为强类型 `RuntimeFailure`，模拟步 panic 额外附带当时世界详情。首个错误成为权威崩溃报告所有者，后续退出期错误仅记入有界抑制列表；报告原子写盘一次后以非零退出码终止进程。

### 网络任务退出

`bbb_client::network_runtime::NetworkRuntime` 是外部连接异步任务的唯一生命周期所有者。启动只构造 `ConnectionOptions`；模拟层仅拿到有界网络事件接收端与命令发送端，既不接触任务句柄，也不持有终态关闭权。连接任务直接写入该事件通道并在每次交付时经 `LoopWake` 唤醒主循环；接收端关闭时任务立即停止，避免旧会话事件残留。

玩法层 `NetCommand::Disconnect` 仍由模拟层按正常会话语义发送；终态关闭则另设独立一次性取消信号，确保命令队列已满、已关闭或模拟层发生致命错误时仍能立即取消连接任务并通过 drop 关闭套接字。无论自然结束、远端断开还是终态取消，最多只产生一个最终 `Disconnected` 事件，重复调用 shutdown 幂等。

`LoopExiting` 严格执行固定顺序：释放光标捕获 → 若仍运行则请求网络断开 → 发出终态取消信号并 join 网络任务 → 发布最终 `running=false/connected=false` 快照 → 最后由事件循环闭包 drop 释放渲染器、音频、窗口与 Tokio 运行时。网络任务 join 设有 5 秒总截止时间：任务 panic 或 join 失败汇入 `RuntimeFailure`/崩溃报告所有者，超时后立即 abort 并回收同样记为致命错误。无连接启动、正常远端断线与正常关闭均不产生崩溃报告。`JoinSet` drop 仅作为事件循环启动前失败时的兜底清理，不替代生产 `shutdown_and_join`。

### 致命错误与报告

`bbb_client::crash_report` 是致命诊断报告的唯一所有者。启动错误、主线程 panic、模拟步 panic、网络 join 错误以及 resize、渲染帧与平台事件循环致命错误均只提交 `RuntimeFailure`；首个错误保持权威，最多记录 8 条后续抑制错误。工作线程、渲染器、控制层与快照发布层均不直接写盘。

崩溃报告复刻 Vanilla `CrashReport`/`Minecraft.crash` 的页眉、UTC 时间、描述、错误、线程栈、分类详情、系统信息与只写一次语义。单字段长度、分类/条目数、抑制错误数、驻留字节（512 KiB）与最终 UTF-8 输出（1 MiB）均有硬上限，Unicode 截断同时写入诊断。输出目录仅来自 `--crash-report-dir`（默认游戏目录下的 `crash-reports`）；安全时间戳文件名不含任何用户输入，临时文件 `sync` 后通过同目录硬链接原子无覆盖发布，重名时追加序号。

发生致命错误时先请求退出事件循环；`LoopExiting` join 网络任务并由闭包 drop 释放渲染器、音频与窗口，`event_loop.run_app` 返回后外层运行时才落盘报告。保存成功退出码为 `1`，写盘失败仍向 stderr 输出有界报告并退出 `2`；正常关闭/退出以及 probe/status/build-info 成功运行不产出报告。

## BBB 实现映射

- `bbb_client::simulation`：模拟步进、tick 截止时钟与渲染只读借用视图。
- `bbb_client::runtime::simulation_step`：客户端 tick 与网络/输入推进。
- `bbb_client::runtime::events::dispatcher`：每个网络事件后消费 `ScreenRequest`。
- `bbb_client::runtime::frame`：单帧提取与当前屏幕 HUD；`render_extract` 定义帧值与提取辅助。
- `bbb_client::simulation::commands`：唯一命令 FIFO 与事件循环唤醒。
- `bbb_client::simulation::publication`：控制与 MCP 只读快照（`SharedSnapshot` / `ControlSnapshot` / `ControlWorldSnapshot`）的原子发布。
- `bbb_client::network_runtime`：网络收发任务对的生命周期与退出屏障。
- `bbb_client::resource_reload`：资源事务准备、提交与失败恢复。
- `bbb_client::crash_report`：有界崩溃报告与原子无覆盖写盘。
- `bbb_client::control`、`bbb_client::mcp`：控制 DTO、参数校验与 MCP 工具投影。

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 任务顺序与单一状态写入 | `Minecraft.runTick` / 游戏主线程 | `CommandQueue`、主线程 `SimulationCore::step` | `aligned` | 命令 FIFO 测试与 lockstep 回放 |
| 客户端 tick 时钟 | `getTickTargetMillis` / `DeltaTracker.Timer` | 模拟层 tick 计时器 | `aligned` | 10 tick 追赶上限与 `partial_tick` 投影测试 |
| 会话替换与断线清理 | `disconnect` / 监听器替换 | `WorldStore` 与同步音频停止顺序 | `aligned` | 会话协议状态与断线音频测试 |
| 资源重载失败与恢复 | `Minecraft` 重载生命周期 | 客户端资源事务与恢复重载 | `aligned` | 代际校验、回滚与二次失败冻结测试 |
| 终态关闭与网络回收 | 客户端退出流程 | 事件循环 + `NetworkRuntime` | `aligned` | 独立取消、join 超时与进程退出验收 |

## 结构性不变量

- 全部外部与平台写入请求只有唯一一条 `CommandQueue` FIFO；只读快照绝不回写世界状态。
- 异步完成消息必须携带会话或资源代际标识，并由原消费者做过期校验。
- 终态关闭通过独立取消信号驱动，不得依赖可能被业务请求填满的普通命令队列空位。
- 首个致命错误独占崩溃报告，不得被后续关闭或 join 失败覆盖。

## 聚焦测试（focused tests）和真实证据

- `bbb_client::simulation::commands::commands_drain_in_fifo_order_and_wake_once_until_rearmed`：验证多产生方 FIFO 应用序、合并唤醒与关闭后拒绝。
- `bbb_client::simulation` 测试与 `runtime::tests::entity_client_tick_catch_up_runs_ten_ticks_and_drops_the_rest`：验证 `partial_tick` 投影与单步 10 tick 追赶上限。
- `bbb_client::network_runtime` 的 `shared_deadline_aborts_and_reaps_a_blocked_final_event` 与 `closed_event_receiver_does_not_stall_join`：验证独立取消信号与限时 join。
- 完整主循环由 `scripts/client_replay.py` 的 lockstep 回放（逐项比对出站数据包、音频日志与截图）及 `cargo dev e2e client` 验收。

常用定向命令：`cargo dev test -p bbb-client simulation::`、`cargo dev test -p bbb-client network_runtime::`。涉及资源重载或会话断连改动时应加跑对应模块聚焦测试，不能仅凭纯队列测试替代重载或联机断线验收。

## 当前差异

本页记录运行时执行契约，[客户端账本](../../../unsupported-client-features.md)当前无未关闭的 `# Todo` 缺口；平台显示、真实联机与外部验收有效性仍由对应实机与端到端门禁裁决。

## 账本归属

主机制 ID 为 `lifecycle-client-runtime`。范围裁决与未关闭差异统一查[客户端账本](../../../unsupported-client-features.md)，自有控制与录像扩展登记于[客户端扩展](../../../client-extensions.md)，推进原则见 [AGENTS.md](../../../../AGENTS.md)。

## 精确重开条件

当新增命令产生方、调整命令应用序或唤醒语义、新增会话替换分支、改变重载确认或网络退出 join 顺序，或 Vanilla 上述核心所有者决策变化时，均须重新核验对应契约与聚焦测试。

## 相关页面和源码索引

- [客户端架构边界](../../architecture/client-boundaries.md)：状态分类与 crate 所有权。
- [客户端协议状态流](../protocol/client-state-flow.md)：数据包解码、状态应用与投影发布。
- [服务端 MCP 管理](../control/server-mcp-administration.md)：服务端查询经主 tick 的独立路径。
- [离线资产可信链](../resources/client-offline-asset-trust.md)：资源输入与代际事务边界。

# 客户端音频、种子与播放生命周期

机制 ID：`audio-sound-manager-engine`

## 机制目的和边界

音频链路将有序的世界声音事实解析为确定的音源、位置与音量，再交给播放后端执行。世界层拥有游戏语义与随机种子，客户端运行时负责资源解析，音频后端管理声道、设备与缓存。音乐、环境音及实体绑定声音均复用同一链路，不得各自建立直接读取世界状态的播放线程。

## Vanilla 26.1 权威实现

- `ClientLevel.java:606-638`：本地声音从 `random.nextLong` 获取随机种子；定位声音按与相机的距离决定是否延迟，仅当距离平方严格大于 100 时延迟，延迟 tick 数为距离除以 40 秒再乘以 20 后向下取整。
- `SoundManager.java:96-116`：应用新的声音注册表与缓存后调用引擎重载。
- `SoundEngine.java:84-101`：重载时检查声音事件，并销毁后重新加载音频库。
- `SoundEngine.java:157-170`：全部停止（stop-all）会中止所有句柄，并清理延迟队列、tickable 列表、声源映射与增益状态。
- `SoundEngine.java:229-281`：每 tick 更新 tickable 声音的位置与参数，并按删除时刻回收已停止实例；`SoundEngine.java:51` 规定声源最小存活期为 20 tick。
- `SoundEngine.java:378-413`：计算播放参数后判断听者与零音量条件，再从静态（static）或流式（streaming）声道池申请声道；无法启动或声道耗尽时返回 `NOT_STARTED`。
- `DecoratedPotBlock.java:212-214`：破裂状态播放陶罐碎裂声，普通状态播放陶罐破坏声。

## 决策输入与状态所有权

| 输入/状态 | 唯一所有者 | 下游投影 |
| --- | --- | --- |
| 数据包声音 holder、位置、音量、音高与种子 | `bbb-client-world` | 有序声音事件 |
| 本地声效种子与实体静音/tickable 状态 | `WorldStore` | 本地、实体与环境声音投影 |
| 内建方块声型及逐状态覆盖 | `bbb-facts::block_sound_facts_26_1` | 世界层直接查询击打/破坏/放置声效事实 |
| 声音目录、事件注册表、唱片机注册表与 CLI 音量 | `bbb_client::audio_runtime::NativeAudioRuntime` | `AudioCommandResolver` |
| 事件引用、加权条目、OGG 路径与资源代际 | `bbb-client-assets` 目录与客户端解析器 | `ResolvedSound` |
| 听者姿态、声道池、延迟队列与解码驻留 | `bbb-client-audio`（桌面 `NativeAudioRuntime` / Web `WebAudioRuntime`） | 后端运行观测 |
| 字幕事件与播放诊断 | 客户端运行时从实际播放事件派生 | HUD 与控制接口只读投影 |

`active_tickable_sounds` 是 `bbb_client::audio_runtime::NativeAudioRuntime` 用于防止重复启动的局部集合，不是第二份实体状态。播放实例是实体绑定声源身份的唯一所有者；客户端启动本帧新 tickable 声源后，仅按当前绑定 ID 读取世界位置。后端单遍应用本帧位置与 tickable 参数表，位置未变时跳过重复提交；实体或 tickable 消失时仍立即停止声源，听者姿态、流式缓冲、延迟播放与生命周期回收继续每帧推进。游戏中的生死、静音、环境与事件切换仍由不可变世界代际决定。

方块击打与破坏事件按合法方块状态 ID 查询声型，放置声按方块类型 ID 查询；未知状态不回退为石头声。资源包仅替换声音素材，不改变内建声型规则，世界层也不保存可独立漂移的声音目录副本。本地脚步声由 `local_player_movement::sounds` 按每步真实碰撞位移推进独立距离阈值，通过既有移动副作用队列提交；它不复用渲染步行距离，也不等待服务端向本人回发脚步声。声型取实际支撑面与覆盖层，地面潜行、飞行、骑乘与旁观各自遵守对应的发声条件。

## 判断与处理顺序

1. 网络声音包沿 `apply_sound_event` / `apply_sound_entity_event` 保留数据包原始随机种子；非本地私有校验在调用端完成，网络定位声音不启用距离延迟，实体绑定路径检查目标实体的 `is_silent`。
2. 本地实体、玩家与定位音效各自从唯一的 `sound_seed_random` 消费一次 `nextLong`。播放后端不重新抽取游戏随机数，而是将同一随机种子交给解析器选择加权声音条目。
3. 解析器先解释注册表 holder，再解析目录中的事件/文件引用，组合条目与事件的音量及音高，按声音分类设置计算增益，并保留文件完整性信息与资源代际。遇到循环引用、过深引用、缺失事件或非法文件时返回结构化错误并记入解析失败计数。
4. 客户端运行时通过既有 FIFO `AudioEventSink` 提交强类型 `AudioCommand`。本地、定位、实体、唱片机与停止命令共用同一后端，不按命令类别拆分为可重排的多条队列。
5. 本地定位声音仅在启用距离延迟且距离平方严格大于 100 时进入延迟队列。`vanilla_distance_delay_ticks` 唯一实现「距离除以 2 后向下取整」的延迟 tick 计算；每 tick 递减剩余 tick 数，到期释放原命令，不在每帧重新测距或重抽种子。
6. 播放阶段应用音量、衰减范围以及静态/流式声道池预算；非音乐且不允许静音启动的零音量声音直接跳过。tickable 更新只使用同一代际的实体投影。
7. 实际播放事件再派生字幕；诊断数据同时投影声道占用、解码驻留、预加载状态与失败计数。

## 生命周期和 fallback

桌面端启动时，客户端音频门面 `bbb_client::audio_runtime::NativeAudioRuntime::load` 先从已验证的 Vanilla 资产物化 OpenAL 动态库，再调用播放后端 `bbb_client_audio::NativeAudioRuntime::new_with_device_preference` 初始化实际 OpenAL 播放路径（`kira` 仅作为 `dev-dependencies` 用于测试）；在 `wasm32` 浏览器目标上，则通过 `NativeAudioRuntime::load_web` 初始化 `bbb_client_audio::WebAudioRuntime` 播放后端。默认非设备单元测试可在无真实音频设备环境下执行；设备热切换、真实发声输出与硬件声道行为仍须以对应原生实机证据为准。

`bbb_client::audio_runtime::NativeAudioRuntime::prepare_reload` 先完整构建新声音目录与预加载请求；提交阶段（`commit_reload`）才调用播放后端的 `reset_resource_generation`，停止旧播放、清空延迟队列与事件状态、释放旧 OpenAL 静态缓冲、取消旧预加载令牌，安装新目录并清空 `active_tickable_sounds`。旧代际的预加载请求会被直接拒绝，异步解码完成时也不能将过期资源写入当前驻留缓存。跨渲染器与音频的事务提交以及失败时保留旧代际的规则由[客户端运行时](../runtime/client-runtime-and-control.md)拥有。

每次音频 tick 前先轮询设备变化；若音频库重载成功，后端会停止所有既有句柄，客户端运行时清除防重集合并在下一 tick 从权威 tickable 投影重建，同时重新提交当前目录的预加载请求。设备观测与错误信息只进入诊断，不修改世界状态。

停止播放实例与释放声道属于不同生命周期：句柄停止后即可归还声道池槽位，而实例外壳仍保留至 20 tick 最小存活边界才删除。带分类或名称过滤的停止命令仅中止匹配实例；无过滤的全部停止（stop-all）才会清空全部在播声源与延迟队列，避免局部停止误删未到期的延迟声音。

会话断开时的音频清理由模拟层决策：Login 阶段失败不执行 stop-all；Configuration/Play 阶段在先行数据包音频之后恰好追加一次 stop-all，随后才应用终态资源变更。空 tickable 投影会清空旧实体绑定身份；音频后端不缓存连接阶段，也不主动决定重连或结束会话。

## BBB 实现映射

- `bbb_client::audio_runtime`：客户端音频运行时门面（`NativeAudioRuntime`），负责初始化、重载编排、字幕派生与诊断统计。
- `bbb_client::audio_runtime::resolver`、`bbb_client::audio_runtime::random`：声音 holder、目录引用解析与确定性加权选择。
- `bbb_client::runtime::events::dispatcher`、`bbb_client::runtime::particles`：世界事件与粒子音效向 `AudioEventSink` 的有序分派。
- `bbb_client::environment_audio_runtime`、`bbb_client::music_runtime`：环境音与背景音乐决策，复用既有音频输出口。
- `bbb_client_audio::command`：跨 crate 音频命令与观测数据契约。
- `bbb_client_audio::scheduler`：桌面 OpenAL 与 WebAudio 共用的距离延迟（`vanilla_distance_delay_ticks`）、停止过滤与 20 tick 最小存活期调度器。
- `bbb_client_audio::runtime`（`NativeAudioRuntime`、含 Windows 录像用的 OpenAL 48 kHz 双声道回环音频采样支路）与 `bbb_client_audio::webaudio`（`WebAudioRuntime`）：桌面 OpenAL 与 WebAudio 后端的声道池、播放与代际生命周期。

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 随机种子与音源选择 | `ClientLevel` / 声音注册表 | `bbb-client-world` + `audio_runtime::resolver` | `aligned` | 确定性音源解析测试 |
| 距离延迟计算 | `ClientLevel.playSound` | `bbb_client_audio::scheduler::vanilla_distance_delay_ticks` | `aligned` | 严格阈值、向下取整与剩余 tick 递减测试 |
| 停止与最小保留期 | `SoundEngine.tick` | `bbb_client_audio::scheduler` | `aligned` | 20 tick 删除边界与声道/外壳分离测试 |
| 资源重载与预加载 | `SoundManager.apply` / 引擎重载 | 客户端事务编排 + 后端驻留缓存 | `aligned` | 旧代际取消与过期预加载拒绝测试 |
| 实际设备播放 | `SoundEngine` / OpenAL 库 | `bbb_client_audio::NativeAudioRuntime`（OpenAL）与 `WebAudioRuntime`（Web） | `aligned` | 真实音频设备验收证据，不以 mock 替代 |

## 结构性不变量

- 随机种子、声音 holder、位置、实体身份与资源代际沿单向链路传递；音频后端不得反向读取 `WorldStore` 或资源包栈。
- 延迟队列与局部停止不得破坏其他音频事件的 FIFO 顺序。
- 缓存内存预算与声道池准入拒绝必须暴露为可观测诊断计数。
- 跨代际的重载、异步预加载与 tickable 实体身份必须同步失效。

## 聚焦测试（focused tests）和真实证据

`bbb_client_audio::scheduler` 与 `bbb_client_audio::runtime` 中可定位的语义测试：

- `bbb_client_audio::scheduler::tests::distance_delay_ticks_match_vanilla_threshold_and_flooring`：验证 10 格不延迟、10.1 格延迟 5 tick。
- `bbb_client_audio::scheduler::tests::delayed_positioned_sounds_release_after_remaining_ticks`：验证递减两次后释放原命令。
- `bbb_client_audio::scheduler::tests::stopped_sound_lifetime_matches_vanilla_twenty_tick_boundary`：验证 19/20 tick 删除边界。
- `bbb_client_audio::runtime::tests::generation_reset_cancels_pending_preload_and_rejects_old_requests`：验证旧代际 3 的预加载不能安装到新代际 4。

常用命令入口为 `cargo dev test -p bbb-client-audio`；客户端解析器行为使用 `cargo dev test -p bbb-client audio_runtime::`。`bbb_client::runtime::events::tests::audio_and_death_sounds` 与 `bbb_client::runtime::events::tests::session_protocol_events` 覆盖有序副作用与会话清理。上述单元测试不能替代真实音频设备与持续运行验收。

## 当前差异

本页不将可运行的无设备单元测试等同于真实音频设备验收；[客户端账本](../../../unsupported-client-features.md)当前无未关闭的音频 `# Todo` 缺口。

## 账本归属

未关闭差异及产品范围裁决见[客户端账本](../../../unsupported-client-features.md)，推进原则见 [`AGENTS.md`](../../../../AGENTS.md)；新增 BBB 自有音频扩展登记在[客户端扩展](../../../client-extensions.md)。

## 精确重开条件

当 Vanilla 调整随机种子、距离延迟、声源最小存活期或重载规则，或 BBB 新增音频产生方、替换播放后端、修改声道池/驻留预算或资源代际契约时，均须重新核验对应处理顺序与测试证据。

## 相关页面和源码索引

- [客户端世界生命周期](../world/client-world-lifecycle.md)：权威声音事实的产生。
- [客户端运行时](../runtime/client-runtime-and-control.md)：批次屏障、资源重载与会话退出。
- [离线资产可信链](../resources/client-offline-asset-trust.md)：声音目录与已验证音频资源。

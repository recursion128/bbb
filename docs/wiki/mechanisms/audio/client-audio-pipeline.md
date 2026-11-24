# 客户端音频、种子与播放生命周期

机制 ID：`audio-sound-manager-engine`

## 机制目的和边界

音频链路把有序 world 声音事实解析为确定的音源、位置与音量，再交给播放 backend。
world 拥有游戏语义与随机种子，native 拥有资源解析，audio backend 拥有 channel、设备和缓存。
音乐、环境音与 entity-bound 声音复用该链路，不能各建一个读取 world 的播放线程。

## Vanilla 26.1 权威实现

- `ClientLevel.java:606-638`：本地声音取得 `random.nextLong`；positioned sound 用 camera 距离
  判断 delay，距离平方严格大于 100 才延迟，以距离除以 40 秒再乘 20 转成整数 tick。
- `SoundManager.java:96-116`：应用新的 sound registry/cache 后调用 engine reload。
- `SoundEngine.java:84-101`：reload 检查 sound event 并 destroy/load library。
- `SoundEngine.java:157-170`：stop-all 停止 handle，清理 queued、ticking、source map 与 gain 状态。
- `SoundEngine.java:229-281`：tick 更新 tickable 声音、位置与参数，并按删除时刻回收停止实例；
  `SoundEngine.java:51` 的最小 source lifetime 是 20 tick。
- `SoundEngine.java:378-413`：计算参数后进行 listener/零音量判断，再从 static 或 streaming
  pool 申请 channel；不能启动或没有 channel 时返回 NOT_STARTED。
- `DecoratedPotBlock.java:212-214`：破裂状态使用陶罐碎裂声，普通状态使用陶罐破坏声。

## 决策输入与状态所有权

| 输入/状态 | 唯一 owner | 下游投影 |
| --- | --- | --- |
| packet sound holder、position、volume、pitch、seed | `bbb-client-world` | 有序 sound event |
| 本地声效种子与 entity 静音/tickable 状态 | WorldStore | local/entity/ambient projection |
| 内建方块声型及逐 state 覆盖 | `bbb-facts::block_sound_facts_26_1` | world 直接查询 hit/break/place 声音事实 |
| sounds catalog、registry、jukebox registry、CLI 音量 | `NativeAudioRuntime` | `AudioCommandResolver` |
| event 引用、weighted entry、ogg 路径、resource generation | `bbb-client-assets` catalog 与 native resolver | `ResolvedSound` |
| listener、channel、delayed queue、decoded residency | `bbb-client-audio` | backend observation |
| 字幕事件与播放诊断 | native 从实际 playback event 派生 | HUD/control 只读投影 |

`active_tickable_sounds` 是 native backend 的重复启动保护，不是第二份 entity 状态。
播放实例是 entity-bound 声源身份的唯一 owner；native 启动本帧新 tickable 后，只按当前绑定 ID
读取世界位置。backend 单遍应用本帧位置与 tickable 参数表，位置未变时跳过重复提交；实体
或 tickable 消失仍停止声源，listener、stream、延迟播放与生命周期继续每帧处理。
游戏中的生死、静音、环境和事件切换仍由不可变 world generation 决定。
方块击打与破坏事件按合法 state id 查询声型，放置声按 block type id 查询；未知 state 不回落
成石头声。资源包替换声音素材，不改变内建声型规则，world 不保存其可独立漂移的目录副本。
本地脚步由 `local_player_movement::sounds` 按每步真实碰撞位移推进独立距离阈值，
通过既有 movement-effect 队列提交；不复用渲染 walk distance，也不等待服务端向本人回发声音。
声型取实际支撑面和覆盖层，地面潜行、飞行、乘骑与旁观遵守各自的发声门。

## 判断与处理顺序

1. 网络 sound packet 沿 `apply_sound_event` / `apply_sound_entity_event` 保留 packet seed；
   private-except-local 校验在调用端已经满足，packet positioned sound 不启用 distance delay，
   entity-bound 路径检查目标的 `is_silent`。
2. 本地 entity/player/positioned 音效各从唯一 `sound_seed_random` 消费一次 `nextLong`。
   backend 不重新抽取游戏随机数；同一 seed 交给 resolver 选择 weighted sound entry。
3. resolver 先解释 registry holder，再解析 catalog event/file 引用，组合 entry 与事件的
   volume/pitch，按 category 设置计算 gain，并保留 file integrity 与 resource generation。
   cycle、过深引用、缺失 event 或非法文件返回 typed error，记录 resolve failure。
4. native 通过既有 FIFO `AudioEventSink` 提交 typed `AudioCommand`。local、positioned、entity、
   jukebox 和 stop 共享同一个 backend；不按命令种类拆成可重排队列。
5. positioned 本地声音只有 distanceDelay 且 distance² > 100 才进入 delayed queue。
   `vanilla_distance_delay_ticks` 唯一实现距离除以 2 后向下取整；每 tick 递减 remaining ticks，
   到期释放原命令，不在每帧重新测距或重抽 seed。
6. playback 应用音量、range/attenuation、static/streaming pool 预算；非 music 且不能静音启动的
   零音量声音不启动。tickable 更新走同 generation 的 entity projection。
7. 实际 playback event 再产生字幕；诊断同时投影 channel、residency、preload 与失败计数。

## 生命周期和 fallback

native startup 从已验证 Vanilla 资产物化 OpenAL library，并调用
`bbb_client_audio::NativeAudioRuntime::new_with_device_preference` 初始化实际 OpenAL 路径。
默认非 device 单元测试可在无真实设备下执行，
设备切换、输出声音和硬件 channel 行为需要对应 native evidence。

`NativeAudioRuntime::prepare_reload` 先构造完整 catalog 与 preload 请求；commit 才调用
`reset_resource_generation`，停止旧播放、清空 delayed/event 状态、释放旧 OpenAL static buffer、
取消旧 preload token，安装新 catalog 并清 native tickable identity。旧 generation 的 preload
请求被拒绝，异步完成不能把旧资源写入当前 residency。跨 renderer/audio 的 transaction 与
失败保留旧 generation 由[运行时资源恢复](../runtime/client-runtime-and-control.md)拥有。

每次音频 tick 前检查设备变化；成功 library reload 会停止 backend handle，native 清除重复启动
集合，下一 tick 从 canonical tickable projection 重建。设备观察与错误进入诊断，不修改世界。

停止实例与释放播放 channel 是不同生命周期：handle 停止后可以归还 pool slot，实例 shell
仍保留到最小 20 tick 删除边界。按 category/name 的 stop 停止匹配实例；无过滤 stop-all 才清空
全部播放和 delayed queue，不能让一次局部停止误删未到期声音。

session teardown 由 simulation 决定：Login failure 不 stop，Configuration/Play 在先行 packet
音频后恰好追加一次 stop-all，随后才应用 terminal 资源变更。空 tickable projection 清旧 entity
identity；audio backend 不缓存连接 phase，也不主动决定重连或结束 session。

## BBB 实现映射

- `bbb_client::audio_runtime`：native 初始化、reload、subtitle 与诊断。
- `bbb_client::audio_runtime::resolver`、`bbb_client::audio_runtime::random`：
  sound holder、catalog 引用与确定性选择。
- `bbb_client::runtime::events::dispatcher`、`bbb_client::runtime::particles`：world event 与粒子效果分派到 `AudioEventSink`。
- `bbb_client::environment_audio_runtime`、`bbb_client::music_runtime`：
  环境/音乐决策与现有 effect sink。
- `bbb_client_audio::command`：backend 输入合同。
- `bbb_client_audio::runtime`：播放、delay、stop、pool 和 generation 生命周期。

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| seed 与音源选择 | `ClientLevel` / sound registry | world + resolver | `aligned` | 确定性解析测试 |
| 距离延迟 | `ClientLevel.playSound` | `vanilla_distance_delay_ticks` | `aligned` | 严格阈值、floor、remaining ticks |
| 播放停止与保留期 | `SoundEngine.tick` | audio runtime | `aligned` | 20 tick 边界与 channel/shell 分离 |
| reload 与 preload | `SoundManager.apply` / engine reload | native transaction、residency | `aligned` | 旧 generation 取消与拒绝 |
| 实际设备播放 | `SoundEngine` / library | native OpenAL | `aligned` | 真实设备 evidence，不能用 mock 替代 |

## 结构性不变量

seed、holder、position、entity identity 与 resource generation 沿单向链路传递；backend 不读取
WorldStore 或 pack stack。延迟与局部 stop 不能改变其它事件的 FIFO 身份。缓存预算和 pool
拒绝必须可观察。跨 generation 的 reload、preload 和 tickable identity 必须同时失效。

## 聚焦测试（focused tests）和真实证据

`bbb_client_audio::runtime` 中可定位的语义测试：

- `distance_delay_ticks_match_vanilla_threshold_and_flooring`：10 blocks 不延迟，10.1 对应 5 tick。
- `delayed_positioned_sounds_release_after_remaining_ticks`：递减两次后释放原命令。
- `stopped_sound_lifetime_matches_vanilla_twenty_tick_boundary`：19/20 tick 删除边界。
- `generation_reset_cancels_pending_preload_and_rejects_old_requests`：generation 3 的工作不能安装到 4。

入口为 `cargo dev test -p bbb-client-audio`；native resolver 行为使用
`cargo dev test -p bbb-client audio_runtime::`。
`bbb_client::runtime::events::tests::audio_and_death_sounds` 与
`bbb_client::runtime::events::tests::session_protocol_events` 提供有序 effect/session 测试。
这些代码证据不自动满足真实设备与持续运行的验收要求。

## 当前差异

本页不把可运行的 mock 测试提升为真实播放验收；未关闭差异只取 ledger。

## 账本归属

未关闭差异见[客户端账本](../../../unsupported-client-features.md)，推进原则见
[AGENTS.md](../../../../AGENTS.md)。自有音频功能的选定工作登记在[客户端扩展](../../../client-extensions.md)。

## 精确重开条件

Vanilla 的 seed、distance delay、sound lifetime 或 reload 决策变化，BBB 新增音频 producer、
替换设备 backend、改变 pool/residency 预算或资源 generation 合同，均须重核对应顺序和证据。

## 相关页面和源码索引

- [客户端世界生命周期](../world/client-world-lifecycle.md)：canonical 声音事实的产生。
- [客户端运行时](../runtime/client-runtime-and-control.md)：batch barrier、reload 与退出。
- [离线资产可信链](../resources/client-offline-asset-trust.md)：catalog 与已验证音频资源。

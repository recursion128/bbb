# 边界与状态分类

## 四类状态

| 类别 | 定义 | 典型 owner | 发布规则 |
| --- | --- | --- | --- |
| canonical state | packet 或本地预测后仍可查询的权威客户端副本 | `bbb-world::WorldStore`、registry/resource generation | 只在所属线程/阶段变更 |
| derived snapshot | 为 control、renderer、诊断提取的只读投影 | `RendererFrame`、`ControlSnapshot` | 在一致边界原子替换，不回写 world |
| transient observed event | 声音、粒子、一次性 UI 等有序副作用 | `PlayApplyEffects` 实现 | exactly-once 顺序消费，不伪装成字段 |
| backend runtime state | socket、GPU buffer、audio voice、window handle | net/renderer/audio/native runtime | 后端生命周期内有效，重建不改变游戏真值 |

## 所有权规则

- 一项长期事实只有一个 semantic owner。消费者持有 id、handle 或 projection，不维护
  可独立漂移的副本。
- protocol 类型描述 wire；world 类型描述客户端语义。不能把原始 packet 当持久世界。
- renderer input 必须携带 Vanilla 决策需要的独立事实。不能从
  `Opaque/Cutout/Translucent`、方块名称或 render shape 猜测 `canOcclude`、
  `isSolidRender`、`isViewBlocking`、光衰减、shade、shape、`skipRendering` 或 fluid
  overlay。
- reload 先构造完整 generation，再原子发布；失败时旧 generation 仍可用。
- 本地预测必须可被服务端 packet 校正；诊断 snapshot 不得成为控制输入。

## 本地输入与 LocalPlayer sprint owner

原始按键与玩家语义状态不能共用一个字段。`bbb-native::ClientInputState` 只保存
`sprint_key_down`、双击前进 edge/timer 和最后成功发送的 sprint baseline；
`bbb-world::LocalPlayerState` 唯一保存 canonical sprint latch，并按 Vanilla
`canStartSprinting`、`shouldStopRunSprinting`、`shouldStopSwimSprinting` 处理 food、mobility、
vehicle、water、pose、collision 与 attack。movement、vehicle、combat 和 camera/FOV 只读取
world latch，不反向读取原始按键。

raw `ServerboundPlayerInputPacket` 只投影按键位；双击前进不会伪造其 sprint bit，松开 sprint
键也不会单独清除已经启动的 latch。非乘客 tick 先发送 latch transition，再发送 player
movement；乘客 tick 保持 rotation、authoritative vehicle movement、latch transition 顺序。
level generation 变化时丢弃旧 edge/timer 并把 outbound baseline 对齐新 world；队列发送失败
不推进 baseline，所以下一 tick 会重试而不形成 shadow state。

## LocalPlayer 与 game-mode 决策 inventory 边界

本地 26.1 `LocalPlayer` 的 104 个方法、`MultiPlayerGameMode` 的 39 个方法，以及
`Minecraft.continueAttack/startAttack/startUseItem/canSwitchGameMode/pickBlockOrEntity/pick`
六个调用入口，由共享 Tree-sitter frontend 提取 signature、source line 和 source hash。
`scripts/client_mechanism_inventory.py` 对这 149 个 owner 使用显式、fail-closed 的同质 route：
未知方法会使生成失败；Java overload 必须进入同一语义 route；只有 CST 证明为单条直接
field/parameter/constant return 的方法才允许归入 `scope-support-accessors`，任何 mutation、调用或
多语句 body 都会失败。版本化 JSON 是排序稳定 evidence，不是第二份任务账本。

路由边界按生命周期而不是 Java 类名划分。玩家侧区分 session/respawn、vitals、movement
reporting、input movement、vehicle、permission/chat、client status、item-use latch、inventory action、
world-edit UI、environment effect 与 camera targeting；game-mode 侧区分 mode lifecycle/guard、
block breaking、prediction sequence、carried-slot sync、item/entity/container/creative/pick action 和
primary-action routing。一个 owner 只进入一个 route；`LocalPlayer.tick`、`aiStep` 这类官方复合 tick
方法归其外层 lifecycle/order owner，内部 canonical state 仍遵守本页 world/native/net 的单向边界。

生成 evidence 中的 BBB path 只表示 physical implementation 可定位，不自动等于 consumer
等价。matrix 的每个 route 必须独立取得 decision→canonical state→runtime/presentation 以及适用
live evidence 后才能标记 `equivalent`；未闭合项只由 `docs/unsupported-features.md` 排程。这样
packet encode 存在、world 字段存在或某个单元测试都不能重新抬高整类状态，也不能恢复原先两个
broad resolver/coverage claim。

同一规则覆盖本地 26.1 `ClientLevel.java` 的 121 个 constructor/method owner（包含 nested
`ClientLevelData` 与 `EntityCallbacks`）：显式表将其拆成 18 条同质 route，其中 prediction 4、
直接 accessor 17、官方空 hook 5，其余 95 个 owner 分属 level lifecycle、clock/weather、
chunk/light、entity、ambient animation、session service、map、level event、respawn data、
block-entity tracking、terrain dirty、particle、biome tint、sound 与 diagnostics。未知 method 使生成
fail closed；同名 overload 必须同 route；accessor 仍由 CST body 约束。生成输出中不再存在
`world-client-level-decisions` broad 行，physical BBB path 也不推导 `equivalent` 状态。

这些 route 固定跨边界方向：packet/resource/input → `bbb-world` canonical level generation →
immutable native effect/renderer/audio projection。renderer、audio 和 diagnostics consumer 不得反向
读取 packet、pack 或另一个 consumer cache；level replacement 必须一次丢弃旧 generation 的
transient projection，而保留/清除 canonical state 的判断只归对应 world lifecycle owner。

### ClientLevel identity lifecycle owner

`WorldStore::client_level_generation` 是三个 `ClientLevel` identity owner 的唯一对应状态。login 构造
新 level，跨维度 respawn 才重建 level，同维度 respawn 只替换 `LocalPlayer`；configuration、transfer
和 terminal clear 推进 generation 并清空 level-bound canonical state。远端 disconnect 由 typed outcome
先结束 connection session，再发布断线 screen 并按 phase 停音；`ClientLevel.disconnect` 不产生第二个
world-side transport owner。旧 immutable publication 保持原值，terrain/particle/map/remote-debug 等
completion 或 cache 只接受当前 generation。

Vanilla `registerForCleaning` 把 item-model registry-context `CacheSlot` 挂到 listener，clear 时统一失效。
BBB 不保存以 `ClientLevel` 地址为 key 的可变 item cache：item/resource projection 从当前 frozen registry 与
immutable resource generation 查询；真正 level-bound 的 native cache 统一以 world generation 判 stale。
`LevelLoadTracker`、本地 ambient handler 和 sprint baseline 的 owner 与 level identity 不同：每次 respawn
都会重建，所以它们复用 canonical `level_load.starts`，与 level generation 组成窄 projection；terrain
readiness 也携带同一 load identity，不能用旧 level 的 ready edge 放行新 player load。source guard 锁定
generation 只有一个 mutator、respawn advance 只在 dimension-change 分支，并禁止这些 local/load consumer
恢复成只看 level generation 的旧路径。

### ClientLevel chunk-light lifecycle owner

`WorldStore` 的 chunk store 与 light 队列是五个 `ClientLevel` chunk/light owner 的唯一状态。
chunk-with-light 包同步替换 chunk 内容（同 pos 原位、越界丢弃），light 数据以 initial-load 形态
入队；`advance_simulation` 每 pump 在唯一 net drain 之后、presentation 同步之前按 vanilla 公式
（size<1000 时 `max(10, size/10)` 否则全量）drain 队列。terrain 失效不在 insert 时发生：initial
light 应用点执行 vanilla `setSectionRangeDirty(±1 全高)` 等价的 chunk+邻居标记，standalone light
更新只标记变化的 light section 邻域。

`chunks_pending_initial_light` 是 vanilla light-engine per-chunk `setLightEnabled` 标志的逆向等价：
packet 装载的 chunk 在其 queued light 应用前保持 pending，native mesh target 扫描跳过 pending chunk
（fresh chunk 不可见、re-send chunk 保留旧 mesh），但仍作为邻接 snapshot 上下文参与 face culling。
forget/prune/level teardown 与队列一致地丢弃 pending；publication clone 携带 pending 集合而不携带
队列本身；restore 的空队列世界默认全 enabled。`onChunkLoaded` 的 tint 失效由 mesh 重建时的 tint
再推导承担，`startTicking`/`stopTicking` 由 `entity_is_ticking` 的 chunk 驻留派生承担；不存在第二个
immediate light-apply 入口。

### ClientLevel level-dirty handoff owner

`WorldStore` 的 `mark_terrain_block_changed`/`mark_terrain_light_sections_changed`/
`mark_terrain_chunk_and_neighbors_changed` 家族是六个 `ClientLevel` dirty delegate 的唯一失效
resolver。`set_block_state_id` 对同态写入返回 false 且不产生任何标记/通知（vanilla
`LevelChunk.setBlockState` 同态返回 null）；`Level.setBlock` 的双通知（requiresRender 门 +
flag 2/4/8）折叠为单次 ±1 block→section 展开，requiresRender 的模型级比较以 state-id 变化
为保守超集。piston 写入方在调用前过滤同态状态，与 vanilla 忽略 setBlock 返回值的行为对齐。

vanilla flag-8 的 player 优先重建映射为 mesh target 的 distance-first 扫描：玩家所在/邻近
chunk 恒为最近目标，由 order focused test 锁定。destroy progress 由 world 侧
`block_destruction_render_stages` 单点合并 server 与本地进度（同 pos 取 max、≥10 移除、
400 tick 过期），native/renderer 只消费该查询。`onSectionBecomingNonEmpty` 的 occlusion
传播由 `occlusion_section_updates` 增量派生承担：section mesh/空态变化直接更新 occlusion
graph，无独立空 section 事件通道，也不得再建第二个失效或合并入口。

### ClientLevel clock-weather owner

`WorldStore` 的 world_time/weather/ticking/sky_flash 是八个 time/weather owner 的唯一状态，
native 侧没有影子时钟。生产入口 `advance_client_level_entity_ticks_with_observation` 经
`consume_running_render_ticks` 实现 tickRateManager 的 runsNormally 门：frozen 时 gameTime 与
clock 实例都不推进，而 skyFlash 衰减、HUD tick 等使用原始 client tick；SetTime 重同步无条件生效
（`apply_world_time` 先按 gameTime delta tick 既有 clock 再覆盖包内 holder，转写
`ClientClockManager.handleUpdates`）。rain/thunder 事件是 26.1 snap 语义（START→0、STOP→1、
LEVEL_CHANGE clamp，o/current 同写故 lerp 恒等）。

vanilla `updateSkyBrightness` 与 `invalidateTickCache` 折叠为按需派生：skyDarken 由
`entity_shadow_sky_darken` 从 environment attributes + weather 每次查询重算，lightmap/sky/fog
环境每 pump 从 world 重建，无可失效缓存。skyFlash 消费链（hideLightningFlash 门、SKY_COLOR
srgbLerp(0.22) 与 SKY_LIGHT_FACTOR=1 layer、End flash 时钟）在 render extract/lightmap 侧有
成对 focused tests；不得另建缓存式 attribute system 或第二个时间推进入口。

### ClientLevel ambient-animation owner

`environment_animate_runtime` 是九个 animate/ambient owner 的唯一 exactly-once effect 投影：
`tick` 每 simulation tick 先从 world 解析一次 marker target，再做 667×(r=16,r=32) 采样对；
`do_animate_tick` 保持 vanilla 顺序——block dispatch→fluid animate+1/10 drip→marker 比对→
`!isCollisionShapeFullBlock` 门内 biome ambient particles。marker 决策
（`local_marker_particle_target_block`：creative + 主手 BARRIER/LIGHT）归 world，native 只消费
解析结果并在匹配方块中心以零速度出携带采样 state 的 BLOCK_MARKER 粒子，全碰撞方块（如
barrier 本体）也出粒子，由 full-block 门前置位置锁定。

fluid drip 三分支（sturdy-top 0.95 面、impermeable 抑制、bottom-shape −0.05 与 two-below
递归）折叠在 `append_fluid_drip_particle`；destroy block effect 走 level event 2001 的
`destroy_block_particle_batch`，breaking crack 走 `local_destroy_hit_particle_batch`，
fireworks 走 firework resolver batch。不得为任一 owner 另建第二个采样循环或 effect 通道。

### ClientLevel respawn-data owner

`WorldStore` 是九个 `ClientLevelData` owner 的唯一状态：difficulty（`client_hud.difficulty`，
login 默认 Normal、`ChangeDifficulty` 包覆盖、respawn 不重置=vanilla 从旧 level 携带）、
hardcore（login 写入随 respawn 携带）、is_flat（每次 spawn info 重取）与 default_spawn
（同维度 respawn 携带、跨维度清除，compass 消费）。`apply_default_spawn_position` 转写
`Level.getWorldBorderAdjustedRespawnData`：越界 spawn 回落到 border center 的
MOTION_BLOCKING heightmap 位置（未加载 chunk 用 `Level.getHeight` 的 minY 分支），
dimension/yaw/pitch 保留。

两个派生值由 world 唯一决策：`client_horizon_height`（is_flat?minY:63）门控 renderer 的
below-horizon dark disc（`buildSkyDisc(-16)`+translate(0,12,0) 烘焙为 y=−4 黑盘，sky pass
最后绘制，门=player eye−horizon<0）；`client_void_darkness_onset_range`（is_flat?1:32）作
fog 颜色的 void darkness 基值（clamp((range+minY−camera.y)/range)，效果环境以 max() 抬升后
(1−darkness)² 应用；bbb 无 POWDER_SNOW camera fog 变体，排除面即 LAVA）。不得另建第二个
difficulty/spawn 存储或 renderer 侧 horizon 判定。

### ClientLevel session-services owner

八个 session-services owner 全部是 connection-scoped 委托：`WorldStore` 持
scoreboard/recipes/enabled features/brewing/fuel 的唯一状态，任意 respawn（同/跨维度）全部
携带；`clear_client_level`（login/configuration 重入=新 listener）只清 scoreboard（vanilla
fresh final field）与 recipes（待重下发），enabled features 经 CommonListenerCookie 携带，
brewing/fuel 由 registry-dependent resource reload 覆盖式重建（旧值保留至新值到达）。分界由
复合生命周期测试锁定。

`getBlockTicks`/`getFluidTicks` 恒为 BlackholeTickAccess：客户端不模拟 scheduled tick，方块/
流体变化只来自服务器包（唯一入口 `set_block_state_id`）；`sendPacketToServer` 只委托当前
connection——world crate 无 socket、无 scheduled-tick 队列，由 source guard 断言，出站包唯一
经 `bbb-net` 的 `send_packet`。不得为任一服务另建可独立替换的缓存或第二条出站路径。

### ClientLevel biome-tint owner

六个 biome-tint owner 收敛为 world biome 数据→native 派生链，renderer 只消费成品颜色：
`build_biome_blend`/`build_chunk_biome_blend` 以 vanilla 默认 biomeBlendRadius=2 的固定 y
x/z 窗口平均 resolver 色（无 options UI，半径不可变）；vanilla per-resolver `BlockTintCache`
与 `clearTintCaches` 折叠为 meshing/粒子 spawn 时按需重算 + section remesh 失效链，无可失效
缓存实体；cardinal lighting 由 `level.cardinal_lighting`→`terrain_cardinal_lighting` 投影；
TINTED_LEAVES 粒子色经 `client_leaf_tint_color` + `ClientLeaf` color source 唯一解析。

已记录差异：vanilla `getUncachedNoiseBiome` 对未加载 chunk fabricate 恒 PLAINS，bbb 的
`biome_id_at` 返回 None 并把该列诚实剔除出 blend 平均（不伪造样本）；重开判据=出现可见
tint 分歧证据。不得在 renderer 侧新增 world biome 读取或第二条 tint 解析路径。

### ClientLevel sound-routing owner

六个 sound-routing owner 收敛为 world canonical audio state + 确定性 seed 链，audio crate 只
消费 command：包驱动 seeded sound 经 `apply_sound_event`/`apply_sound_entity_event`（vanilla
handleSoundEvent 的 except==local player 门在调用点恒真，positioned 变体 distanceDelay=false，
entity-bound 受 is_silent 门）；本地变体（entity/player/positioned）每次调用从
`sound_seed_random` 消耗一个 seed（等价 `random.nextLong()`，结构不变量已锁定该链）；
positioned local sound 携带 distance_delay 标志，私有 `playSound` 的延迟决策由
`vanilla_distance_delay_ticks` 唯一实现（distSqr>100 → sqrt(dist)×0.5 tick 延迟入队，
delayed queue 按剩余 tick 释放）。不得绕过该 seed 链或另建第二条声音提交路径。

### ClientLevel particle-routing owner

六个 particle-routing owner 收敛为 particle runtime 的单一准入 gate：四个 add 变体
（addParticle×2/addAlwaysVisibleParticle×2）合成 override=type.overrideLimiter||packet flag
与 always-show 标志，per-type override 由 generated particle catalog 携带；
`should_spawn_level_particle` 按 override→无条件、距 camera >1024 sqr 丢弃、
particleLevel==MINIMAL 拒绝的顺序唯一裁决准入；`calculate_particle_level` 从 options
particles 档起始，always-show&&MINIMAL 有 1/10 升 DECREASED、DECREASED 有 1/3 降
MINIMAL（共享 random 链）。environment spawns、level-particles 包与 firework 路径全部
经该 gate，不得另建第二条粒子准入路径或在调用点内联距离/降档判断。

### ClientLevel level-events owner

三个 level-events owner 收敛为 packet apply 的唯一分派链：`global_level_event_sound` 按
event.global 门控仅 1023/1028/1038（wither spawn/dragon death/end portal），在 camera
方向 2.0 距离处生成 local sound（dragon death 音量 5）；非全局事件由
`apply_level_event_with_effects` 唯一分派——jukebox 状态经 `apply_level_event` 记录、
seeded 音效与粒子效果消耗同一 level-event random 链（sink-less 路径也推进同样的
random 序列）；explosion 包典型化后经 `ClientExplosionTracker::track` 确定性入队
（512 cap、按 level generation 清理）。不得绕过该分派链或另建第二条事件效果路径。

### ClientLevel map-data owner

四个 map-data owner 收敛为 `WorldStore.maps` 唯一 store：`apply_map_item_data` 唯一拥有
handleMapItemData 的缺失即建（scale/locked+零 colors=createForClient）与 bounds 校验
patch merge（patch 外字节保留，非法 patch 原子拒绝不动旧表）；`map_item` 是 getMapData
读路径（item frame 贴图消费）；dimension-change respawn 保留整表（等价
getAllMapData→addMapData 携带），login/transfer 重建空表并归零 map counter，二者由同一
测试对照锁定。不得在 store 外缓存 map 字节或绕过 respawn/login 生命周期规则。

### ClientLevel diagnostics-report owner

四个 diagnostics-report owner 收敛为 `bbb-world` 的单一活值装配：
`client_level_report_details` 在调用时从当前 world 代产出 "Affected level" 全部九项
detail（All players 计数+debugInfo、Chunk stats、Level dimension、Level time、Level spawn
location、Server brand/Server type/Tracked entity count/Client weather，vanilla 顺序）；
`chunk_source_stats_line` 是 `Chunks[C] W: … E: …` 行的唯一 owner，debug HUD 行与 report
Chunk stats 共同消费；spawn location 由 `level_spawn_location_detail` 按
CrashReportCategory.formatLocation 逐段转写（含 inclusive min/max build height）；level
identity 恒 "ClientLevel"。crash report 在失败边界经 `attach_level_details` 从当帧
publication 的同代 world 附加段落，首个非空捕获生效，无 level 时诚实省略该段。已记录
差异：bbb 无 integrated server，Server type 恒 "Non-integrated multiplayer server"。不得
另建第二条 report 装配路径或用陈旧快照填充段落。

### ClientLevel entity lifecycle owner

`bbb-world::WorldStore` 与其 private `EntityStore` 是 17 个 `ClientLevel` entity-lifecycle owner 的
唯一语义边界。protocol add 以 insertion order 建立 fresh incarnation，同 id 先 discard 旧实例；remove
在一次 canonical mutation 中清除 mount/passenger/leash、player、dragon part 与 transient owner。
id/UUID/count、render enumeration、pick 与 multipart query 都读取该 store，native/renderer 只消费
immutable generation projection，不保存可独立替换的 entity cache。

每个 raw client tick 从 store 重建有界 schedule：只进入 loaded client section 的 root；freeze 时只保留
Player root 或含 Player passenger 的树；root 先 tick，再按 mount order 递归 ride tick。living death tick
只额外受 local-player chunk 与 server simulation distance 的 Chebyshev gate，不能借此缩小普通 tick 或
render workload。publication 在完整 tick 后原子替换，旧 publication 保持不变；level/session replacement
整体丢弃旧 store。

`ClientLevel.getPushableEntities` 的 client-only local-player target 也属于同一路径。schedule 使用生成的
runtime-class dispatch 排除 Bat/ArmorStand/Parrot override，world 在 mutable pass 前一次解析 local
health/spectator/climbable、same-root、scoreboard team 与 pusher dynamic pushability；ordered tick 再用当前
pose、attribute scale 与 strict AABB 查询。boat 与 ordinary living 共用唯一 0.01/0.05 impulse helper，
pusher 是否可推动只控制 reciprocal impulse；renderer、native 和 pack 均不拥有第二个 collision resolver。
生成器 exact-output/source guard 锁定 override 表、单一 scoreboard query 与旧 boat fallback 调用点为零。

`interaction-block-breaking` 的六个 `MultiPlayerGameMode` owner 进一步要求逐 owner evidence。
`bbb-world::LocalPlayerInteractionState` 是 target、face、selected-item signature、progress、stage 和
delay 的唯一 canonical owner；native mouse 只按 Vanilla 的
restriction → 初始 world-border → prediction/attack → command 顺序编排，net 只编码已确定的
`START/ABORT/STOP_DESTROY_BLOCK`。生存模式进入同一 target 后不重新查询 world border；若 block
已变成 air，则 world 清理 active progress，native 不再发 swing、hit particle 或声音。creative 的
每次 continue 仍重新经过 border gate。真正发布 local destroy 前先执行同一个
`Item.canDestroyBlock` 投影：默认 Tool 的 `canDestroyBlocksInCreative` 只限制 instabuild，Tree-sitter item
catalog 机械锁定唯一直接拒绝覆写 `DebugStickItem`；随后读取 block catalog 从 `GameMasterBlock`
interface 机械生成的七项 membership。任一 gate 拒绝时仍发送 Vanilla START/STOP，但不把目标本地预测为空气。

有界 net queue 是本地 mutation 的 commit barrier：start/finish/abort 必须先取得一个 command permit，
retarget 必须一次取得 ABORT 与 START 两个 permit；full、closed 或 absent sender 时 prediction sequence、
target、tutorial、block prediction 与 effect 均保留上一完整状态，下一 input/tick 重试。release/focus-loss
也调用同一个 atomic abort owner，不另行清 cache。

`MultiPlayerGameMode.startPrediction` 的唯一对应 owner 是 `WorldStore` 中与
`local_block_predictions` 同生命周期的 level-bound sequence/teleport state，而不是随 respawn 替换的
`LocalPlayerInteractionState`。每次 block breaking、block use-on 或 item use 都先取得一个 net permit，
再按 signed Java `int` wrapping 分配一次 sequence、执行 world predictive action，最后用同一 permit 发送
matching packet；full、closed 或 absent sender 不推进 sequence，也不发布 item/block mutation、effect 或
retained prediction。同位置再次变化只更新 sequence，保留首次 server state/player position；
`BlockChangedAck` 以 signed `<=` inclusive 清理，且只有 `lastTeleportSequence < ack sequence` 才允许碰撞
校正移动玩家。同维度 respawn 只替换 `LocalPlayer`，所以保留该 handler；dimension、ClientLevel 或 session
replacement 一次清除 sequence、teleport gate 与 retained prediction。native、net 与 renderer 均不缓存
第二份 handler 状态，旧非预留 use resolver 由 reverse source guard 禁止恢复。

## Control command 与 simulation mailbox owner

`bbb-control` 只拥有外部 DTO、只读 snapshot 和构造时绑定的
`SharedControlRequests` sink handle；它不保存 command queue，也没有 sink 后装或替换阶段。
`bbb-native::simulation::commands::SimulationMailbox` 是 window、control、services producer 的
唯一 production command owner。startup 先创建 mailbox，再把其 `ControlRequestSink` 注入
`ControlState`，因此 producer 可见前已经只有一条写路径。

所有 producer 在 mailbox 的同一短 mutex 内取得全局 sequence。simulation 按 barrier drain，
单个 `ControlWriteRequest` 直接进入 code-of-conduct、net 或 input handler；不会再按 request
family 复制到可独立重排的 shadow queue。shutdown 仍是 ordered command，screenshot 仍由同一
sequence 产生 platform effect。platform input、control、render feedback、async completion、
lifecycle 五类各有独立 count/retained-logical-byte 分区；soft threshold 只合并相邻高频 tail
value，硬上限饱和则在 drain condvar 背压，不能向其它 class 借位，也不静默丢失离散命令、
key/button release、ordered interaction 或 shutdown。logical bytes 计入 command inline owner、
递归 owned capacity；大型 resource-reload ack 以 boxed ownership 入队，但 box owner 与 data/image
generation charged bytes 仍全部记账。

单命令越过本 class byte limit 返回 `oversized`。simulation worker 无论正常返回、runtime failure
或 panic unwind 都先原子 close mailbox，再唤醒所有 producer；后续提交返回 `closed`，control
dispatcher 会把拒绝作为失败响应而非伪报 queued。`SimulationCore::step` 在 barrier drain 后把
总量及每 class 的 current/peak count/bytes、blocked/coalesced 和 rejection counters 写入同一
generation 的 `NetCounters`。所有 production producer 使用显式 result 或
`enqueue_or_report`；reverse source guard 禁止恢复 unchecked enqueue。reload/readiness/services
completion 的 generation 判定仍只归原 consumer，mailbox 不复制 session state。

## HUD item resident projection owner

Vanilla `GuiGraphicsExtractor.item` 只构造一个 `ItemStackRenderState`，
`ItemModelResolver.updateForTopItem` 按 resolver child 顺序追加 geometry/special layer，最后由
`ItemStackRenderState.submit` 顺序提交。BBB 对应地只跨边界传递
`HudBlockItemModel { layers, lighting, foil }`：native/item owner 在 GUI query context 下解析
resource id、display/local transform、atlas、geometry class、source 和 tint；renderer 只把同一份
immutable ordered layers 投影为 resident asset、instance 或 special-item submission，不反向查询
item runtime、terrain model 或 pack stack。

普通 block quad 只形成存在几何的 solid→translucent layers，并继续使用 blocks atlas；generated
与 special/composite child 使用各自 items/entity owner，保持 child order、foil phase、full-bright、
scissor 与 submit order。顶层 compatibility `identity/quads/gui_display`、empty-layer sentinel 和
第二条 HUD submit helper 已删除；source guard 固定 projection 只有 layers/lighting/foil，且 renderer
生产消费只遍历一次 `model.layers`。

CPU projection 随 resource/world/input snapshot 重建，但 static geometry 仍由 renderer residency
按 asset identity 复用。frame commit 只消费不可变 projection；stale resource generation 被拒绝，
reload prepare 或后续 GPU/audio owner 失败时继续保留上一完整 generation。真实 26.1 default
resources 的 1506 个 item definition/registry、stone block projection，以及 offscreen hotbar/glint/
scissor pixels共同验证这条唯一数据流。

## Entity equipment asset catalog owner

Vanilla `EquipmentAssetManager` 先把当前 resource generation 的 `equipment/*.json` 解析为
`EquipmentClientInfo`，`EquipmentLayerRenderer` 再按 layer type 和声明顺序查询；未知 asset、缺失
layer type 或空 layer 都不退回静态材质表。BBB 对应的唯一 CPU owner 是 pack/item generation
投影出的 `EntityEquipmentAssetCatalog`。native 在同一 reload prepare 中把静态 texture identity 与
owned atlas entry 解析为 renderer-private `ResolvedEntityEquipmentAssetCatalog`，entity frame 只携带
同 generation 的 `EntityEquipmentAssetId` 以及 dye、foil、trim 和 player-texture 查询事实。

humanoid/armor-stand、wings、wolf、equine、llama、pig、strider、camel、happy-ghast 和 nautilus
layer 全部查询这一份 resolved catalog；renderer 不按 armor material、dye color 或 saddle flag
重建另一份 texture resolver。无 asset id、未知 id、stale generation 或缺失 layer 都提交零层；
dye skip/default、首个可绘制 layer 后的 foil、末层 texture 上的 trim、baby/leggings、profile wings
override、outline 与 submit order 仍由共享 layer emitter 决定。普通测试 helper 使用显式空 catalog，
equipment 测试则提供带 generation/id 的 catalog fixture，不能通过测试开关改变 production 分支。
pack 的 item registry 将 humanoid/wolf/mount armor、saddle、carpet、harness 与直接 `setAsset`
声明统一投影到唯一 `equippable_assets` 映射；item-model 只把 item 转成 asset id，不查询 layer
presence。world 只决定有效 slot、实体/幼体 admission 及独立 stack facts；
`SimpleEquipmentLayer` 的 SADDLE/BODY stack 不借用 armor-slot classifier，native 只转换
generation/id，cape/wings 与所有静态/owned texture 查询都留在 renderer catalog owner。

装备 stack 自身影响模型的语义不从 catalog readiness 反推。例如 world 从 nautilus BODY slot
独立投影 `nautilus_body_item_present`，native 只转交该 frame fact；renderer 的
`ZombieNautilusCoralModel` 对任意非空 stack 隐藏 corals，即使其 asset 为 custom、missing 或
stale。asset id 仅决定 `NAUTILUS_BODY` layer 是否可绘制，二者没有 fallback 或双写关系。

catalog/owned texture/atlas/GPU pipeline 均在现有 renderer resource transaction 中准备。只有完整
generation 成功后才替换；任一步失败继续保留上一 resolved catalog 和 atlas，旧 generation id 不会
命中新 generation。反向 source guard 禁止恢复 legacy flag、静态 armor/saddle/wings resolver 或
asset-free compatibility 分支。

## Entity textured shader owner

Vanilla `RenderPipelines.ENTITY_SNIPPET` 只在 no-cull pipeline 启用 `PER_FACE_LIGHTING`，
`entity.fsh` 再用 `gl_FrontFacing` 选择正、反法线；启用 back-face cull 时，存活 fragment
恒走正面光照。BBB 因而只保留一份 renderer-private `ENTITY_MODEL_TEXTURED_SHADER`，统一拥有
atlas/standalone texture sampling、alpha cutout、tint、overlay、lightmap、正反面 diffuse 与 fog。
pipeline descriptor 的 cull mode 是单双面行为的唯一决策，不再用第二份 shader source 表达同一事实。

solid pipeline 从共享 source 派生时只删除 alpha-cutout block；z-offset 变体只替换 projection
matrix。cutout/solid/translucent 的 blend、depth、cull、draw order 与 texture fallback 均保持在各自
descriptor/资源 owner。共享 shader 仍随 renderer resource transaction 一次 prepare 和发布：任一
pipeline 编译失败保留上一完整 generation，不产生独立 shader cache 或 compatibility 路径。反向
source guard 锁定共享声明唯一，并禁止旧 cull shader owner 恢复。

## Menu screen 注册与生命周期

Vanilla `MenuScreens.register` 的 25 个 menu type→screen class 映射及声明顺序由
`scripts/menu_screen_catalog_extractor.py` 通过共享 Tree-sitter frontend 提取，并生成
native-private `menu_screens_26_1` catalog。它是 screen-kind 的唯一 resolver；protocol 的
`MenuType` catalog 只拥有 registry id，world 的 `InventoryState` 只拥有当前 container、title、
slot/data 与 open-container revision，renderer 只消费 native 已完成的 HUD projection。

合法 `ClientboundOpenScreenPacket` 先由 protocol 拒绝 catalog 外 id，world 再以单次 replace
发布 container state，native layout 对 id 只查询一次生成 catalog。25 项各自沿用既有 layout、
hit-test、click transaction 与 render owner；`LecternScreen` 投影到专用 book surface，其余映射
投影到 inventory HUD。clientbound/local close、respawn 与 session replacement 清除同一个 world
state，不存在 native shadow screen cache。未知 id 的 native 查询返回无 surface；重复、遗漏、顺序
变化及生成输出陈旧均在 extractor fixture、catalog exact-output check 和 reverse source guard 失败。

## DeathScreen 与 report-editor 返回 owner

Vanilla `DeathScreen`、`ReportingContext.draftReportHandled` 与 `ConfirmScreen` 对应一条单向
数据流：combat packet 只由 world `ClientDeathScreenState` 捕获 generation、cause、score 与
hardcore；report draft 只由 world reporting state 保存；native death input 唯一决定第 20 tick
激活、focus、normal/hardcore、draft Edit/Discard、URL 与 respawn/disconnect action。report editor
只保存一个 `SocialInteractions | Death` previous-screen 投影；matching close 恢复同 generation，
finish、environment replacement 或 stale generation 丢弃它，不存在 social-only shadow cache。

renderer/control 只消费不可变窄投影。主屏与 delayed title confirmation 使用 death gradient；
通用 draft/link `ConfirmScreen` 使用 in-world menu background/blur；`icon/draft_report` 随现有 HUD
resource transaction 原子 prepare/commit，失败保留上一 generation。offscreen test 锁定 icon、
background、button 与 confirmation 像素；playable lifecycle 必须观察 disabled→第 20 tick active，
再经 `input.key:enter` 进入唯一 native action，reverse guard 禁止恢复 `net.perform_respawn` 绕路。

## Piston fixed facts 与 world movement owner

Vanilla `PistonStructureResolver`/`PistonBaseBlock.isPushable` 以 global block-state 为查询坐标。
BBB 的 reaction 与 `destroySpeed == -1.0F` 只由 `bbb-protocol::block_state_types` 的 checked 26.1
runtime facts 拥有；block-entity 关系复用同 crate 已有 block-type→block-entity catalog，不复制第二个
boolean owner。29,873 个 state 的 packed piston facts 只占一个私有 u8/state array，公开面限于既有
support-facts query、一个五值 reaction enum 和 unbreakable accessor。

`bbb-world::piston_blocks` 是唯一 gameplay consumer：loaded state id 直接查询 protocol facts，保持
12-block、line/collision/branch、reaction、边界与 sticky 顺序，再由 `moving` 一次应用完整结果。
unloaded 仍按 air，loaded unknown/fact 缺失仍失败；纯 resolver 失败不改 world，destination apply
失败保留旧 world。facts 只随 protocol version 变化，没有 resource/session cache 或 reload rollback；
pack 不再导出 piston profile、source-expression resolver 或 `PackRoots` load API。generator `--check`
固定 source→output 新鲜度，world reverse guard 禁止旧 pack/name-keyed 路径恢复。

## 生命周期

启动创建平台与资源基础，连接依次经过 handshake、configuration、play。configuration
建立 registry/data projection；play 建立 world。resource reload 可跨 play 独立发生，
但一帧 renderer input 必须引用同一 resource generation。断线先停止新 packet 应用，
再清理 world/backend；control 只能看到最后一个明确标记状态的 snapshot。

外部 client tick 的唯一 deadline 来自 simulation scheduler。正常世界以 50ms 为下限，
低于 20 TPS 时使用 server tick rate，freeze 仍保持 50ms client tick；外部多人 pause screen
不冻结 client/world tick。event loop 不另存 tick timer，render/control 只读 publication 的
tick anchor、interval 与 residual。terminal close 固定由 event loop 依次 join simulation、network，
再发布 `running=false/connected=false`，不存在第二条 detached shutdown 路径。

机械 owner inventory 不再把 `Minecraft.java` 的 187 个方法当成一个 lifecycle 判断：共享
Tree-sitter frontend 将 9 个 run/tick/terminal owner、12 个 session replacement owner、3 个 task
handoff owner、9 个 resource recovery owner和 1 个 options-only dynamic mipmap owner分别路由，
其余决策进入既有 input/render/security/diagnostics 或 source-backed 范围外机制。仅“单条 return 且表达式为 field、parameter 或 constant”
的 64 个 owner 可标记 `support-non-decision`；出现调用、分支、分配、写入或新方法时 checked output
立即失败，不能借 accessor 分类隐藏新机制。

### 远端断线状态页边界

`bbb-net::DisconnectReason` 是远端 packet/transport terminal outcome 的唯一 bounded
presentation 输入：relay 保持 FIFO，主动本地 close 不合成 reason，裸 event channel 关闭也不
伪造远端状态页。simulation 在同一步、应用 window/control input 前只调用
`WorldStore::end_connection_session`：它清除 client level、registry、server-pack、listener
presentation 与 transfer-only carry，再由 `ClientInputState` 发布 generation-tagged
`DisconnectedScreenRuntime`；Login/尚未建 phase 使用 `connect.failed`，Configuration/Play 使用
`disconnect.lost`。关闭 receiver 后不会保留一个永久 productive wake source。

声音 teardown 沿用同一 terminal effect barrier，而不建立断线专用 backend 路径：Login/connect
failure 对应 Vanilla `ConnectScreen` 的 `stopSound=false`，不提交 stop；Configuration/Play 在
level、registry 与 server-pack session 清理后，向既有 `AudioEventSink` 追加一次无 category/name
过滤的 stop-all。它排在本批次先行 packet audio 之后、terminal resource reload apply 之前；随后
空 tickable projection 清除旧 generation 的 entity-bound identity。`bbb-audio` 只消费 FIFO effect，
不反向读取 phase、world 或 pack，stop-all 同时释放 active、streaming 与 delayed playback owner。

`ClientInputState` 唯一拥有 focus、hover、Esc-ignore、details/back action 和 bounded control
diagnostics；render publication 只携带不可变投影。renderer 只拥有居中 layout、200px button、
menu-background GPU generation 与 generation-tagged hit target，不反向读取 socket、world 或 pack。
HTTP(S) 和本地 report path 经 simulation→platform effect barrier 执行并返回结果；CLI-direct 产品的
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
先 flush chat acknowledgement，再恰好清一次 level 并在同一 FIFO effect batch stop-all；后续
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
task 回到唯一 game thread、identity wrapping、存活期间始终可执行。BBB 的对应 owner 是命名
`bbb-simulation` thread 与唯一 `SimulationMailbox`：window/control/render feedback/services/lifecycle
producer 共享单调 sequence 和输入 barrier，network relay 只推进同一 wake generation；owner 退出时
close guard 同时唤醒被 backpressure 阻塞的 producer，之后所有 completion 都以 `Closed` 拒绝，
不存在第二 task queue 或 unchecked enqueue。

resource reload 是 batch-terminal effect。event loop 每遇到一个 reload 就先完成同 generation 的
CPU prepare、renderer/audio transaction 与 acknowledgement handoff，再读取下一 simulation batch；
后续 disconnect/session replacement 因而不能越过旧 reload。失败保留上一完整 generation，并只做
一次 recovery reload；recovery 完成后由 world-owned 单调 UI fact 投影 Vanilla
`PACK_LOAD_FAILURE` system toast。二次失败冻结相同 stack、防止无界热循环，并在 simulation owner
内依次关闭 transport、替换 connection generation、停止 audio、发布 bounded disconnected screen 和
释放 cursor；不会留下仍可交互的半失败 session。download/materialization 使用
UUID+push-sequence，reload completion 使用 generation，任一旧结果都只做清理或拒绝，不能改写
current barrier。

simulation panic、network task panic 与 network join timeout 都转成 typed `RuntimeFailure`。event loop
先 join simulation、再以共享 deadline join/reap network tasks，最后发布 terminal snapshot；第一
failure 是 canonical report owner，后续 shutdown failure 只进入有界 suppressed 列表，report 原子
save-once 后以非零状态退出。

## External network backend terminal close

`bbb-native::network_runtime::NetworkRuntime` 是 external connection future 和 event-relay
task 的唯一 lifecycle owner。startup 只构造 `ConnectionOptions`；simulation 取得有界 net
event receiver 与 command sender，既看不到 task handle，也不拥有 terminal close state。relay
保持 connection event 顺序，并在每次成功交付后唤醒 simulation；simulation receiver 关闭时立即
停止，不把旧 session event 留给下一进程。

gameplay `NetCommand::Disconnect` 仍由 simulation 按正常 session 语义发送。terminal close 另有
独立 one-shot signal，所以 command queue 已满、已关闭或 simulation fatal 都不能阻止 connection
future 被取消并通过 drop 关闭 socket。连接自然结束、服务器断开或 terminal signal 最多产生一个
最终 `Disconnected` event；重复 shutdown 幂等。

`LoopExiting` 的固定顺序是：释放 cursor capture、请求并 join simulation、signal 并 join network
task pair、发布最终 `running=false/connected=false` snapshot，随后 event-loop closure 才释放
renderer/audio/window 和 Tokio runtime。两个 network task 共用 5 秒总 deadline；task panic/join
failure 进入既有 `RuntimeFailure`/crash-report owner，超时会 abort 并 reap 剩余 task 后同样报告。
无连接、正常远端结束和成功 close 不产 fatal report。`JoinSet` drop 只负责 event-loop 建立前发生
startup failure 时的 abort safety net，不能替代 production `shutdown_and_join`；focused source guard
锁定唯一调用点及 simulation→network→snapshot 顺序。

<a id="profile-texture-trust-and-presentation"></a>

## Profile texture 信任与 presentation

`bbb-item-model::skin_runtime` 是 profile property、默认皮肤、skin/cape/elytra 集合和远端
PNG CPU 生命周期的唯一 semantic owner。net 在连接前 bounded prepare Services 的
`PROFILE_PROPERTY` key；world 只保存不可变 profile projection；native 传入“本地/远端”查询坐标，
drain 已解码图像并在 render thread 回报 GPU commit；renderer 只拥有 atlas、handle 和当前
presentation generation，不反向读取 profile、HTTP 或磁盘 cache。

lookup key 使用 UUID 与完整 packed `textures` property。带签名 property 按 SHA-1/RSA 和 bounded
Services key set 验证；远端 `PlayerInfo` 的 unsigned/invalid 集合直接使用 UUID 默认皮肤且不发
HTTP，本地玩家与 profile item 可使用 insecure 集合。URL 只接受 `http|https` 和精确
`textures.minecraft.net`，按 authlib basename 复用 type-specific texture，完整 URL 冲突则失败；
磁盘 identity 使用 Guava `hashUnencodedChars` 等价 SHA-1。skin、cape、elytra 必须全部成功后才发布，
任一 URL/download/decode/upload 失败都会使整个 base set 保持默认 fallback；pack-backed patch 仍从
同一 immutable resource generation 查询并遵守 priority/filter。

profile-result access cache 的 TTL 是 15 秒；bounded texture entry、decoded PNG 和磁盘 identity 属于
进程生命周期。disconnect、transfer 或 resource generation 边沿只失效 renderer projection，保留的
CPU 图像在新 generation 重交付而不重复 HTTP。resource reload 先完成所有 CPU/GPU/audio fallible
prepare/commit，再把 process owner 移入新 `NativeItemRuntime`；失败时旧 generation 和其 texture owner
完全不动。control counters 同时报告 profile-set/asset/key bytes 与 async queue/resident/delivery bytes；
source guard 禁止恢复 URL-only skin cache、分离 cape/elytra resolver 或 generation-keyed CPU shadow
cache。

## Fatal failure 与 crash report

`bbb-native::crash_report` 是 fatal diagnostics 的唯一 owner。startup `Result`、main panic、
simulation panic/join error，以及 render-effect、resize、frame 和 platform event-loop fatal
只提交 `RuntimeFailure`；首个 failure 保持 canonical，最多 8 个后续 failure 作为 suppressed
diagnostics。worker、renderer、control 或 snapshot 都不直接写报告。

报告复刻 Vanilla `CrashReport`/`Minecraft.crash` 的 header、UTC time、description、error、
head/thread/backtrace、category、system details 和 save-once 判断。单字段、category/entry 数、
suppressed 数、retained bytes（512 KiB）和最终 UTF-8 文本（1 MiB）都有明确预算，Unicode
截断会写入诊断。目录只来自启动参数 `--crash-report-dir`，默认是 game-directory 语义下的
`crash-reports`；安全时间戳文件名不含 server/user 输入，临时文件 sync 后用同目录 hard-link
原子 no-clobber 发布，重名追加序号。

fatal event 先请求 event-loop exit；`LoopExiting` join simulation，closure drop 释放
renderer/audio/window，`event_loop.run` 返回后外层 runtime 才持久化。保存成功退出 1，写失败
仍把 bounded report 输出 stderr 并退出 2；正常 close/quit、probe/status/build-info 成功不产报告。
focused process test 与 production-binary acceptance 都验证“失败生成一份且非零退出，下一进程
成功且不覆盖旧报告”。

## 测试责任

protocol 测 byte/类型边界，world 测 ordered apply 与 canonical state，native 测
publication/consumer wiring，renderer/audio 测 backend 输入合同。跨边界测试证明同一
id、generation、tick 与顺序，而不是重复每层的内部测试。

<a id="mojang-client-telemetry"></a>

## Mojang 客户端遥测边界

BBB 不收集或上传 Mojang client telemetry。Vanilla 26.1 的
`WorldSessionTelemetryManager` 会把 server brand 写入 `WorldLoadEvent` 并尝试通过独立
event sender 发送；这不是 multiplayer protocol canonical state。BBB 仍可把对应 wire
字段解码为诊断事实，但不得为它创建外部 telemetry runtime owner。只有项目明确把该
外部服务纳入产品表面时才重新审计；当前裁决由
[缺口账本](../../unsupported-features.md)唯一维护。

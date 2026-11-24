# 客户端世界生命周期、tick 与效果分派

机制 ID：`world-client-level-lifecycle`

## 机制目的和边界

本页解释 ClientLevel identity、chunk/light、entity tick、时钟与 session 服务怎样形成一致的
世界，以及何时清除派生状态。相关 `world-client-*` route 仍独立验收；音频、粒子和 renderer
只有消费投影的权限，不能反向读取 packet、pack 或另一 consumer 的 cache。

`WorldStore` 是 canonical level owner；`client_level_generation` 标识 level 替换，
`level_load.starts` 标识每次玩家加载。两者不能合并：同维度 respawn 替换玩家但保留世界。
transport close、线程 join 与 resource transaction 归[客户端运行时](../runtime/client-runtime-and-control.md)。

## Vanilla 26.1 权威实现

源码根由 `scripts/vanilla path 26.1` 解析，关键决策如下：

- `net/minecraft/client/multiplayer/ClientLevel.java:254-270`：light update 入队与预算 drain，
  size < 1000 时预算为 `max(10,size/10)`，否则消费整个队列。
- `net/minecraft/client/multiplayer/ClientLevel.java:276-285`：正常运行门限制 border/time，
  sky flash 衰减在该门之外。相邻 tick 还推进 explosion、chunk source 与环境失效。
- `net/minecraft/client/multiplayer/ClientLevel.java:328-369`：root tick、death simulation-distance
  门与递归 passenger 顺序；移除或不匹配 vehicle 的 passenger 停止骑乘。
- `net/minecraft/client/multiplayer/ClientPacketListener.java:1249-1299`：只有 dimension 改变
  才构造新 ClientLevel 并携带 map；任意 respawn 都构造玩家并重新等待 level loading。

`ClientLevel` 构造器建立 chunk cache、tick manager、level data 和环境层；`unload/onChunkLoaded`
关闭/开启 light/entity ticking 并处理 tint。`registerForCleaning` 委托 listener 注册 CacheSlot，
`disconnect` 也只委托 connection，不独占 transport。`ClientLevelData` 拥有 difficulty、hardcore
与 flat 等世界参数；listener 则拥有 scoreboard、recipes 和 enabled features 等服务。

## 决策输入与状态所有权

| 数据及坐标 | canonical owner | 派生/消费边界 |
| --- | --- | --- |
| login/respawn dimension key、spawn info | `WorldStore` 的 level/player state | 决定 level identity 与玩家 load identity |
| chunk pos、section、block state/light | chunk store 与 queued light | native mesh 使用 revision 和 pending gate |
| entity id/UUID、mount order、incarnation | private `EntityStore` | 每 tick schedule，immutable render enumeration |
| raw client tick、running tick、SetTime | world time/weather/ticking/sky flash | HUD、sky、lightmap 不自建时钟 |
| map id、colors、decorations | `WorldStore.maps` | item-frame/GUI 按查询结果取材 |
| recipes/scoreboard/features/brewing/fuel | connection-scoped world service | respawn 携带；listener/resource 边沿分别更新 |
| audio seed、level-event random、explosion | world audio/event state | FIFO command 或有界 effect batch |
| biome 与 block-entity 可见性输入 | world 查询 → native 派生 | renderer 只消费颜色/可见模型 |

## 判断与处理顺序

### Chunk、light 与 terrain dirty

chunk-with-light 包同步替换 `ChunkMap`（`bbb_client_world::chunks::{store, map}`）中同 pos 的 chunk，维护到达顺序、按需计算的 64 列 `FamilyGroup` 位图及按类型的方块实体槽位索引；越界丢弃，light 以 initial-load 形态入队。
`advance_simulation` 每 pump 在唯一 net drain 后、presentation 同步前按 Vanilla 预算 drain。
initial light 应用时才标记 chunk+邻居全高 dirty；standalone light 只标记变化 section 邻域。

`chunks_pending_initial_light` 对应 light engine enabled 的逆向状态：pending chunk 不成为 mesh
重建目标。fresh chunk 暂不显示，re-send 保持旧 mesh；两者仍参与邻接 snapshot 的 face culling。
forget/prune/teardown 同时丢队列与 pending；publication clone 携带 pending 集合，不携带队列。
直接 restore 的空队列世界默认 enabled，不新增 immediate packet light apply 入口。

方块变化的光照属性不同则在 world 中合并待传播位置；网络更新、预测与回滚共用此入口。
simulation 在本步变更和 packet light 应用之后、publication 之前运行本地 block/sky 传播。
待更新位置的已加载邻居仍等待 initial light 时保留该位置；否则从 canonical light 建立全高度
临时窗口，复用 `bbb-protocol::chunk_light` 的传播核心，只回写实际变化层并失效邻接地形。
clear、unload 和同位置完整 chunk 替换丢弃对应待更新位置；publication clone 不携带运行队列。

`mark_terrain_block_changed`、`mark_terrain_light_sections_changed` 和
`mark_terrain_chunk_and_neighbors_changed` 是唯一失效家族。同态 `set_block_state_id` 返回
false 且不通知；状态变化把 `Level.setBlock` 的 requiresRender 与 flag 2/4/8 通知折叠成一次
±1 block→section 展开，state-id 变化是模型级 requiresRender 的保守超集。
flag-8 的 player 优先重建映射为 mesh distance-first；piston caller 先过滤同态写入。

`block_destruction_render_stages` 合并 server/local progress：同 pos 取 max，≥10 移除，
400 tick 过期。section mesh/空态变化经 `occlusion_section_updates` 增量更新 graph，
没有第二个空 section 事件或 destruction 合并通道。

### Entity schedule 与 tick 时钟

add 按 insertion order 建 fresh incarnation，同 id 先 discard 旧实例；remove 在同次 mutation
清 mount/passenger/leash、player、dragon part 与 transient owner。所有 id/UUID/count/pick/
multipart/render 查询都读取同一个 store。

每 raw client tick 重建 bounded schedule，只纳入 loaded client section 的 root。freeze 保留
Player root 或含 Player passenger 的树，root 先 tick，再按 mount order 递归 ride tick。
living death 额外使用 local-player chunk 与 server simulation distance 的 Chebyshev 门；
该门不能缩小普通 tick 或 render workload。完整 tick 后原子发布，新旧 publication 相互独立。

client-only push target 是本地玩家。generated runtime-class dispatch 排除 Bat/ArmorStand/Parrot
覆写；mutable pass 前解析 health/spectator/climbable、same-root、team 与 dynamic pushability。
ordered tick 使用当前 pose、scale 和 strict AABB；boat 与 ordinary living 共用 0.01/0.05 impulse
helper，pusher pushability 只控制 reciprocal impulse。不得恢复 native/pack collision resolver。

`advance_client_level_entity_ticks_with_observation` 经 `consume_running_render_ticks` 推进
running time；freeze 不推进 gameTime 与 clock，skyFlash/HUD 使用 raw tick。SetTime 无条件
重同步：先按 gameTime delta 推进已有 clock，再覆盖 packet 中出现的 holder。
rain/thunder 使用 26.1 snap：START→0、STOP→1、LEVEL_CHANGE clamp，old/current 同写。

`simulation` 每步计算一次 `sky_darken` 供暗角、眼部遮罩与实体阴影共用，`world` 自身不保存可失效的 tick 缓存；lightmap/sky/fog environment 按需从 attributes/weather 推导。
flash 由 hideLightningFlash 门、SKY_COLOR srgbLerp(0.22)、SKY_LIGHT_FACTOR=1 与 End flash
时钟投影，不建立第二个 cached attribute system 或 time advance 入口。

## 生命周期和 fallback

| 边沿 | level/player | 携带或清除 |
| --- | --- | --- |
| login | 新 level、新 player/load | 清旧 chunk/entity/map 与 listener 状态 |
| 同维度 respawn | level identity 不变，新 player/load | 保留 level prediction、map 与 session services；关闭旧容器 |
| 跨维度 respawn | 新 level、新 player/load | 清 level transient/chunk/entity；携带 map、difficulty/hardcore 和 session services |
| configuration、transfer、terminal clear | 推进 level generation，清 level-bound state | connection cookie/server-pack 的额外规则由 session owner 决定 |

旧 publication 保持原值，terrain/particle/map/remote-debug completion 必须检查当前 generation。
BBB 不保存以 ClientLevel 地址为 key 的可变 item cache，item/resource 查询使用 frozen registry 和
immutable resource generation。level-bound native cache 统一判 generation stale。
load tracker、本地 ambient handler 与 sprint baseline 每次 respawn 都重建，使用
`(client_level_generation, level_load.starts)`；旧 ready edge 不能放行新 player load。

scoreboard、recipes、features、brewing/fuel 在同/跨维度 respawn 全部携带。fresh listener 的
`clear_client_level` 清 scoreboard 与待重下发 recipes；features 经 CommonListenerCookie 携带，
brewing/fuel 由 registry-dependent reload 覆盖，旧值保留至新值到达。
客户端 `getBlockTicks/getFluidTicks` 对应 BlackholeTickAccess，不模拟 scheduled block/fluid tick。
world 没有 socket 或 scheduled-tick 队列，出站只经 `bbb-client-net`，局部预测也使用既有写入 owner。

### Respawn data 与 map

difficulty 在 login 默认 Normal，由 ChangeDifficulty 覆盖，respawn 携带；hardcore 从 login
携带，is_flat 每次 spawn info 重取。default spawn 同维度保留、跨维度清除。
越界 spawn 回落 border center 的 MOTION_BLOCKING heightmap；未加载 chunk 使用 minY 分支，
dimension/yaw/pitch 保留。compass 从同一 default-spawn 查询读取。

`client_horizon_height` 使用 flat?minY:63，门控 player eye 低于 horizon 时的黑盘，烘焙 y=-4，
最后绘制；`client_void_darkness_onset_range` 使用 flat?1:32，fog 先计算
clamp((range+minY-camera.y)/range)，与效果 darkness 取 max 后乘 `(1-darkness)^2`。
LAVA/POWDER_SNOW 保留颜色暗化排除，renderer 不自建 difficulty/spawn/horizon 判定。

`apply_map_item_data` 缺 map 时建 scale/locked+零 colors，再按 bounds 原子 patch merge；
patch 外字节不变，非法 patch 不改旧表。`map_item` 是唯一读路径。dimension respawn 携带整表，
login/transfer 清表及 map counter；store 外不得缓存另一份可变 map 字节。

### 环境动画、声音与粒子

`environment_animate_runtime::tick` 每 simulation tick 解析一次 marker target，然后做
667 对 r=16/r=32 采样。每点先用预计算的 `block.has_animate_tick` 快速跳过无动画方块，再按 block dispatch → fluid animate+1/10 drip → marker →
非 full-collision 门内 biome ambient particle 排序；宝库与试炼刷怪笼等方块实体动画音效在 `block_entity_effects.rs` 经定位 `SoundEventState`（`play_sound_event`）发布。creative 主手 BARRIER/LIGHT 的 marker
由 world 裁决；native 在匹配块中心生成零速 BLOCK_MARKER，故 barrier 自身也能显示 marker。
单次 animate tick 只读借用 world，并按直接 quart biome holder 缓存 ambient particle 查询结果；
dimension → biome → timeline 的组合仍由 world 查询拥有。每点仍执行粒子概率和位置采样，
缓存不跨 tick 保留，因此下一 tick 会读取新的 registry、biome 和时钟。

fluid drip 的 sturdy-top 0.95 面、impermeable 抑制、bottom-shape -0.05 与 two-below 递归
由 `append_fluid_drip_particle` 拥有。destroy 使用 level event 2001 的 particle batch，
crack 经 `local_destroy_hit_particle_state_with_biome_blend_radius` 与 `spawn_local_destroy_hit_particles` 发射，firework 交给现有 resolver，不增第二采样循环。

seeded sound 包经 `apply_sound_event/apply_sound_entity_event`；本地声音共用 world seed 链，
audio 只消费 FIFO command，不读取 world。silent、distance-delay、seed 消耗与 delayed queue
的完整合同见[客户端音频链路](../audio/client-audio-pipeline.md)。

particle 准入统一在 `should_spawn_level_particle`：type.overrideLimiter OR packet override
直接通过，否则距 camera 平方 >1024 丢弃，再按 particle level 拒绝 MINIMAL。
`calculate_particle_level` 从 options 起步：always-show+MINIMAL 有 1/10 升 DECREASED，
DECREASED 有 1/3 降 MINIMAL，共用 random。environment、packet 与 firework 都经过该 gate。

global event 只对 1023/1028/1038 生成 camera 朝向 2.0 距离的声音，dragon death 音量 5；
非全局 `apply_level_event_with_effects` 先走唯一分派，jukebox 更新 state，sound/particle 共用
level-event random，sink-less 也推进相同序列。explosion 由 `ClientExplosionTracker::track`
确定性入队，cap 512，随 level generation 清除。

### Tint、block entity 与 diagnostics

biome blend 默认 radius=2（可通过 `--biome-blend-radius <0..=7>` 或 `options.txt` 的
`biomeBlendRadius` 配置），按固定 y 的 x/z 窗口平均；mesh/粒子 spawn 按需重算和 section remesh
替代独立 BlockTintCache。cardinal lighting 经 world→terrain 投影，TINTED_LEAVES 用
`client_leaf_tint_color` 与 ClientLeaf color source。未加载 biome 的差异见下节。

`block_entity_visibility` 同址裁决两种提取路径：普通模型先过 compiled 16³ section、frustum
和 renderer distance（default 64、end gateway 256）；off-screen 成员不经 section/frustum。
`off_screen_owner_is_visible` 对 beacon 用水平 renderDistance×16，对 bounding-box renderer
用球面 96，test-instance 用二者并集；`renders_off_screen` 唯一登记 global membership。
beam 在 projection 处过门后不重复判距，moving piston 经 `MovingPistonVisibility` 回到 section 门。

BlockEntityRenderers 注册绑定以实际 projection 函数为准，不复制 BBB 注册表。sign/hanging sign、
各 chest、各 spawner、end portal/gateway 各共享相应 scene；campfire/brushable/vault/shelf
共享 `item_models::block_entity`，按 renderer transform 分支；piston attachment 经
`item_models::dropped`。generated inventory 对每个注册保留 implementation/test evidence。

`client_level_report_details` 当次从 world 装配 Affected level：players、chunk stats、dimension、
time、spawn、brand/type、entity count、weather，保持 Vanilla 顺序。`chunk_source_stats_line`
共供 HUD/report，spawn 使用包含 min/max height 的 location 格式。fatal `attach_level_details`
只捕获当帧同代 publication，首个非空结果生效；没有 level 则省略，不能补陈旧快照。

### 活塞固定事实与客户端 world movement

Vanilla `PistonStructureResolver`/`PistonBaseBlock.isPushable` 以 global block-state 为查询坐标。
BBB 的 reaction 与 `destroySpeed == -1.0F` 只由 `bbb-facts::block_state_types` 的 checked 26.1
runtime facts 拥有；block-entity 关系复用同 crate 已有 block-type→block-entity catalog，不复制第二个
boolean owner。全部 state 的 packed piston facts 只占一个私有 u8/state array，公开面限于既有
support-facts query、一个五值 reaction enum 和 unbreakable accessor。

`bbb-client-world::blocks::piston_blocks` 是唯一 gameplay consumer：loaded state id 直接查询 generated facts，保持
12-block、line/collision/branch、reaction、边界与 sticky 顺序，再由 `moving` 一次应用完整结果。
unloaded 仍按 air，loaded unknown/fact 缺失仍失败；纯 resolver 失败不改 world，destination apply
失败保留旧 world。facts 只随 protocol version 变化，没有 resource/session cache 或 reload rollback；
pack 不再导出 piston profile、source-expression resolver 或 `PackRoots` load API。generator `--check`
固定 source→output 新鲜度。


## BBB 实现映射

| 语义 owner | repo-local 路径 | 消费者 |
| --- | --- | --- |
| level/load/time | `bbb_client_world::level` | simulation、load runtime |
| chunk/light | `bbb_client_world::chunks::{store, map}` | `ChunkMap` 到达顺序/位图/方块实体索引、mesh targets、terrain revisions |
| entity schedule | `bbb_client_world::entities::animations` | immutable world publication |
| map | `bbb_client_world::maps` | map texture/item-frame |
| audio/events | `bbb_client_world::client::audio` | native FIFO effect |
| environment/particle | `bbb_client::environment_animate_runtime`、`particle_runtime` | renderer/audio 输入 |
| block-entity visibility | `bbb_client::block_entity_visibility` | scene projection |
| diagnostics | `bbb_client_world::client::level_report` | HUD/crash report |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| level 与 player load identity | `handleRespawn` | `level` | `aligned` | level/load tests |
| queued light visibility | `pollLightUpdates`、light enabled | `chunks::store` | `aligned` | pending initial light test |
| freeze 与 SetTime | `ClientLevel.tick`、listener clock | `level` | `aligned` | frozen time test |
| map carry | `getAllMapData/addMapData` | `maps`、`level` | `aligned` | map lifecycle test |

## 结构性不变量

- level generation 只有一个 mutator，同维度 respawn 不能借清理 cache 推进 generation。
- packet/resource/input → world → immutable native effect/backend；不存在 consumer 回读 canonical 输入。
- ClientLevel 关键生命周期、时钟、区块、实体与环境效果分别绑定唯一生产 owner，不以旁路缓存替代权威 world 状态。

## 聚焦测试（focused tests）和真实证据

| 可定位测试 symbol | 文件与验证边界 |
| --- | --- |
| `frozen_ticking_gates_production_time_advance_but_not_set_time_resync` | `bbb_client_world::level`：冻结、raw flash 与 SetTime |
| `login_resets_map_store_while_dimension_respawn_carries_it_over` | 同上：新 listener 与 respawn 的 map 差别 |
| `pending_initial_light_lifecycle_follows_load_apply_prune_and_direct_insert` | `bbb_client_world::chunks::tests`：pending/re-send/prune/direct insert |
| `frozen_client_tick_schedule_keeps_player_roots_and_player_passenger_trees_running` | `bbb_client_world::entities::tests::passenger_and_vehicle_movement`：递归玩家树 |
| `same_dimension_respawn_restarts_load_timeout_without_replacing_level_identity` | `bbb_client::level_load_runtime`：新 load、旧 level |

适用 focused 命令为 `cargo dev test -p bbb-client-world pending_initial_light`、
`cargo dev test -p bbb-client-world frozen_`、
`cargo dev test -p bbb-client-world login_resets_map_store` 与
`cargo dev test -p bbb-client same_dimension_respawn`。
这些 focused 结果不能替代 official/live evidence。

## 当前差异

Vanilla 未加载 chunk 的 noise biome 回退 PLAINS；BBB `biome_id_at` 返回 None，并从平均中剔除
该列。biome blend 半径默认 2，可通过 `--biome-blend-radius <0..=7>` 或 `options.txt` 的
`biomeBlendRadius` 在启动期配置（无游戏内 options UI）。world report 的 server type 恒为
`Non-integrated multiplayer server`，因为 BBB 只覆盖外部多人客户端。开放项状态只查 ledger。

## 账本归属

主机制是 `world-client-level-lifecycle`，chunk/light/entity/clock/map/ambient/sound/particle/
tint/respawn-data/services/dirty/diagnostics 各保留独立 ID。
见 [客户端 ledger](../../../unsupported-client-features.md)
与 [AGENTS.md](../../../../AGENTS.md)。

## 精确重开条件

Vanilla 新增 ClientLevel method、改变 respawn carry 或 tick/freeze 顺序时重新取证；
新 off-screen renderer 或注册 owner 搬迁需更新对应 inventory/visibility evidence；出现未加载
biome 的可见 tint 分歧、旧 load-ready 误放行、旧 generation completion 被消费或第二个
light/time/entity owner 时，按可复现失败登记相应 route。

## 相关页面和源码索引

- [本地输入与交互](../player/client-input-and-interaction.md)：玩家重建、输入与 prediction。
- [状态流](../protocol/client-state-flow.md)：连接顺序与 publication。
- [方块渲染](../rendering/client-block-rendering.md)：terrain mesh 与资源消费者。

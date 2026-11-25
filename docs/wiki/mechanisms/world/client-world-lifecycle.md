# 客户端世界生命周期、tick 与效果分派

机制 ID：`world-client-level-lifecycle`

## 机制目的和边界

本页定义 `ClientLevel` 世界身份、区块与光照存储、实体 tick 调度、世界时钟以及会话级世界服务如何协同构成一致的客户端世界，并明确各类派生状态的清理边沿。音频、粒子与渲染器只能只读消费世界投影，严禁反向读取网络包、资源包或其他消费者的缓存。

`WorldStore` 是权威世界所有者：`client_level_generation` 标识世界替换，`level_load.starts` 标识每次玩家加载。两者绝不能合并——同维度重生会替换玩家并重启加载流程，但保留原世界实例。传输层关闭、网络线程回收与资源事务提交归属[客户端运行时](../runtime/client-runtime-and-control.md)。

## Vanilla 26.1 权威实现

源码根由 `cargo dev vanilla path 26.1` 解析，关键决策锚点如下：

- `net/minecraft/client/multiplayer/ClientLevel.java:254-270`：光照更新入队与预算消费——队列长度 `< 1000` 时每轮最多消费 `max(10, size / 10)` 条，否则一次排空整个队列。
- `net/minecraft/client/multiplayer/ClientLevel.java:276-285`：世界正常运行门限制世界边界与时钟推进，而闪电天空闪白（`skyFlashTime`）衰减位于该门之外；相邻逻辑还推进爆炸、区块源与环境失效。
- `net/minecraft/client/multiplayer/ClientLevel.java:328-369`：根实体 tick、生物死亡模拟距离门与递归乘客 tick 顺序；载具被移除或不匹配时解除骑乘关系。
- `net/minecraft/client/multiplayer/ClientPacketListener.java:1249-1299`：仅当维度改变时才构造新 `ClientLevel` 并跨维携带地图数据；任意重生都会重建玩家实体并重新进入世界加载等待。

`ClientLevel` 构造器建立区块缓存、tick 管理器、世界数据与环境层；`unload`/`onChunkLoaded` 开关区块光照与实体 tick 并处理群系着色缓存。`registerForCleaning` 委托监听器注册清理槽，`disconnect` 也只委托连接对象而不独占传输。`ClientLevelData` 持有难度、极限模式与超平坦标志；监听器持有计分板、配方与已启用特性标志等会话级服务。

## 决策输入与状态所有权

| 数据及坐标 | 权威所有者 | 派生与消费边界 |
| --- | --- | --- |
| 登录/重生维度键与出生信息 | `WorldStore` 的世界与玩家状态 | 决定世界代际与玩家加载身份 |
| 区块坐标、区段、方块状态与光照 | 区块存储与待应用光照队列 | 客户端地形构网读取修订号与光照就绪门 |
| 实体 ID/UUID、骑乘顺序与代次 | 私有 `EntityStore` | 每 tick 调度表与不可变渲染枚举 |
| 原始客户端 tick、运行 tick 与 `SetTime` | 世界时间、天气、冻结状态与天空闪白 | HUD、天空与光照图不得自建独立时钟 |
| 地图 ID、像素与装饰图标 | `WorldStore.maps` | 物品展示框与 GUI 按查询结果渲染 |
| 配方、计分板、特性标志、酿造与燃料 | 会话级世界服务 | 重生携带；在监听器或资源重载边沿更新 |
| 声音随机种子、世界事件随机流与爆炸 | 世界音频与事件状态 | FIFO 音频命令或有界副作用批次 |
| 群系与方块实体可见性输入 | 世界查询 → 客户端派生 | 渲染器只消费最终颜色与可见模型 |
| 模组内容注册表与 ICU 本地时间格式化 | `WorldStore`（`bbb:content_registry` 冻结快照）、`bbb-client-world::local_time` | 实体/方块/物品扩展查询与文本组件本地日期时间求值 |

## 判断与处理顺序

### 区块、光照与地形脏区标记

`ChunkWithLight` 数据包同步替换 `ChunkMap`（`bbb_client_world::chunks::{store, map}`）中同坐标区块，维护区块到达顺序、按需计算的 64 列 `FamilyGroup` 位图及按类型的方块实体槽位索引；越界区块直接丢弃，随包光照以初始加载形态入队。`advance_simulation` 每轮在唯一网络事件消费之后、呈现同步之前，按 Vanilla 预算排空光照队列。初始光照应用时标记本区块及邻区块全高度脏区；独立光照更新包仅标记变更区段及其邻域。

`chunks_pending_initial_light` 对应光照引擎就绪状态的反向集合：尚在等待初始光照的区块不会进入地形构网目标；新到达区块暂不显示，重发区块保留旧网格，但两者均参与邻接快照的面剔除查询。遗忘、裁剪与拆世同步丢弃待应用队列与等待集合；快照克隆携带等待集合但不携带运行队列。直接恢复的空队列世界默认光照就绪，不另增绕过队列的立即光照应用入口。

当方块变化引起光学属性改变时，世界层合并记录待传播坐标；网络方块更新、客户端本地预测与预测回滚共用同一入口。模拟层在本步变更与数据包光照应用之后、快照发布之前运行本地块光与天空光传播。若待更新坐标的已加载邻居仍在等待初始光照则暂留该坐标；否则从权威光照建立全高度临时窗口，复用 `bbb-protocol::chunk_light` 传播核心，只回写发生变化的层并标记邻接地形失效。清空、卸载与整区块替换同步丢弃对应待更新坐标；快照克隆不携带运行队列。

`mark_terrain_block_changed`、`mark_terrain_light_sections_changed` 与 `mark_terrain_chunk_and_neighbors_changed` 是唯一地形失效家族。同态写入 `set_block_state_id` 返回 `false` 且不发通知；状态变化将 `Level.setBlock` 的 `requiresRender` 与标志 `2`/`4`/`8` 通知折叠为一次 `±1` 方块到区段展开（状态 ID 变化构成模型级重绘的保守超集）。标志 `8` 的玩家优先重建映射为近距优先构网；活塞调用方先过滤同态写入。

`block_destruction_render_stages` 合并服务端与本地破坏进度：同坐标取最高阶段，`>= 10` 移除，存活超 400 tick 过期。区段网格与空态变动经 `occlusion_section_updates` 增量同步至遮挡图，不存在第二条空区段通知或破坏进度合并通道。

### 实体调度与世界时钟

新增实体按插入顺序建立全新实例，同 ID 先销毁旧实例；移除实体在同次变更中一并清理骑乘/乘客、拴绳、本地玩家引用、末影龙部件与瞬态状态。所有 ID/UUID 查找、数量统计、准星拾取、多部件与渲染枚举均读取同一 `EntityStore`。

每个原始客户端 tick 重建有界调度表，仅纳入已加载客户端区段内的根实体。世界冻结时保留玩家根实体以及包含玩家乘客的载具树；根实体先 tick，再按骑乘顺序递归推进乘客 tick。生物死亡额外检查本地玩家区块与服务端模拟距离的切比雪夫距离门，该门不缩小普通实体 tick 或渲染范围。完整 tick 结束后原子发布快照，新旧快照彼此独立。

客户端实体推挤仅作用于本地玩家：生成运行时类分派排除蝙蝠、盔甲架与鹦鹉覆写；可变阶段前解析生命、旁观、可攀爬、同根载具、队伍与动态可推性。有序 tick 使用当前姿态、缩放与严格包围盒；船与普通生物共用 `0.01`/`0.05` 冲量辅助函数，推挤者属性仅控制反向冲量，不得在客户端或资源包层恢复碰撞解析器。

`advance_client_level_entity_ticks_with_observation` 经 `consume_running_render_ticks` 推进运行时间：冻结时不推进 `gameTime` 与世界时钟，闪电天空闪白与 HUD 仍消耗原始客户端 tick。`SetTime` 无条件重同步：先按 `gameTime` 增量推进既有世界时钟，再覆写包内显式出现的时钟 holder。雨雪与雷暴遵循 26.1 瞬切语义（`START → 0`、`STOP → 1`、`LEVEL_CHANGE` 钳制并同时写入新旧值）。

模拟层每步计算一次 `sky_darken` 供暗角、眼部遮罩与实体阴影共享，`WorldStore` 自身不维护可失效的 tick 缓存；光照图、天空与雾效环境按需从维度属性与天气推导。闪电闪白受 `hideLightningFlash` 门控，经 `SKY_COLOR` sRGB 插值（`0.22`）、`SKY_LIGHT_FACTOR = 1` 与末地闪光时钟投影，不另建缓存属性系统。

## 生命周期和 fallback

| 边沿 | 世界与玩家身份 | 携带或清除策略 |
| --- | --- | --- |
| 登录（Login） | 新世界、新玩家/加载身份 | 清空旧区块、实体、地图与监听器状态 |
| 同维度重生 | 世界代际不变，新玩家/加载身份 | 保留世界预测处理器、地图与会话级服务；关闭旧容器 |
| 跨维度重生 | 新世界、新玩家/加载身份 | 清空旧世界区块、实体与瞬态状态；携带地图、难度/极限模式与会话级服务 |
| 重配置、转服、终态清理 | 递增世界代际，清空绑定当前世界状态 | cookie 与服务器资源包的跨会话携带由运行时会话所有者决定 |

旧快照始终保持不可变原值，地形、粒子、地图与远端调试的异步完成均须核对当前世界代际。BBB 不建立以 `ClientLevel` 内存地址为键的可变物品缓存，物品与资源查询统一基于冻结注册表与不可变资源代际，绑定当前世界的客户端缓存统一按代际失效。加载追踪器、本地环境音处理器与疾跑发送基线在每次重生均以 `(client_level_generation, level_load.starts)` 重建，旧加载就绪边沿不能误放行新玩家加载。

计分板、配方、特性标志、酿造与燃料在同维与跨维重生中均完整保留。新监听器的 `clear_client_level` 清理计分板与待重下发配方；特性标志由 `CommonListenerCookie` 携带；酿造与燃料在依赖注册表的重载时整体替换，旧值保留至新值到达。客户端 `getBlockTicks`/`getFluidTicks` 对应 `BlackholeTickAccess`，不模拟服务端计划刻。世界层不持有套接字或计划刻队列，所有出站动作走 `bbb-client-net`。

### 重生数据与地图存储

难度在登录时默认为 Normal，由 `ChangeDifficulty` 覆盖并跨重生保留；极限模式标志自登录起保留，`is_flat` 每次从出生信息重新获取。默认出生点在同维度重生时保留、跨维度清除；越界出生点回退到世界边界中心的 `MOTION_BLOCKING` 高度图，区块未加载时回退至维度最小高度 `minY`，维度与朝向保留。指南针直接读取同一默认出生点查询。

`client_horizon_height` 使用 `flat ? minY : 63`，门控玩家眼部低于地平线时的黑色圆盘（烘焙于 `y = -4` 并在天空末尾绘制）；`client_void_darkness_onset_range` 使用 `flat ? 1 : 32`，雾效先计算 `clamp((range + minY - camera.y) / range)`，与失明/黑暗状态效果取最大值后再乘 `(1 - darkness)^2`。熔岩与细雪雾效保留颜色暗化排除规则，渲染器不自建难度、出生点或地平线判定。

`apply_map_item_data` 在地图条目缺失时先以指定缩放、锁定状态与全零像素初始化，再按更新区域原子合并补丁；补丁外字节保持不变，非法补丁不污染旧表。`map_item` 是唯一读入口。跨维重生保留完整地图表，登录与转服清空地图表及计数；世界存储之外不得缓存第二份可变地图像素副本。

### 环境动画、声音与粒子

`environment_animate_runtime::tick` 每模拟 tick 解析一次标记方块目标，再执行 667 对半径 16 与半径 32 的随机采样。每个采样点先用预计算的 `block.has_animate_tick` 快速跳过无动画方块，再依次执行「方块动画分派 → 流体动画与 1/10 滴落 → 屏障/光源方块标记 → 非完全碰撞门内的群系环境粒子」。宝库与试炼刷怪笼等方块实体的客户端定时粒子与局部音效在 `environment_animate_runtime/block_entity_effects.rs` 中推进，其中定位音效经 `audio.play_positioned_sound` 提交至音频出口。创造模式主手持 `BARRIER`/`LIGHT` 的标记显示由世界层裁决，客户端在匹配方块中心生成零速 `BLOCK_MARKER` 粒子。单次动画 tick 只读借用世界，并在当前 tick 内按四格群系（quart biome）holder 缓存环境粒子查询结果；缓存绝不跨 tick 保留，确保下一 tick 读取最新注册表、群系与时钟。

流体滴落的顶部稳固面 `0.95` 检查、防渗透方块抑制、底部形状 `-0.05` 与向下两格递归由 `append_fluid_drip_particle` 拥有。破坏碎屑走 `LevelEvent 2001` 粒子批次，挖掘击打粒子经 `local_destroy_hit_particle_state_with_biome_blend_radius` 与 `spawn_local_destroy_hit_particles` 发射，烟花交由既有解析器，不增第二采样循环。

携带种子的声音包经 `apply_sound_event`/`apply_sound_entity_event` 应用；本地声音共用世界层种子随机链，音频层只消费 FIFO 命令而不读取世界。静音、距离延迟、种子消耗与延迟队列完整契约见[客户端音频链路](../audio/client-audio-pipeline.md)。

粒子准入统一由 `should_spawn_level_particle` 裁决：`type.overrideLimiter` 或数据包覆盖标志直接放行，否则与相机距离平方大于 `1024` 丢弃，再按 `calculate_particle_level` 结果拒绝 `MINIMAL`。`calculate_particle_level` 从客户端选项起步：`always_show + MINIMAL` 有 1/10 概率升为 `DECREASED`，`DECREASED` 有 1/3 概率降为 `MINIMAL`，共用同一随机源。环境动画、网络粒子包与烟花统一经过该准入门槛。

全局世界事件仅对 `1023`、`1028`、`1038` 在相机朝向 2.0 格处生成声音（末影龙死亡音量 5）；非全局 `apply_level_event_with_effects` 走唯一分派，唱片机更新状态，声音与粒子共用世界事件随机流，无音频出口时也推进相同随机步数。客户端爆炸经 `ClientExplosionTracker::track` 确定性入队，上限 512 条，随世界代际清除。

### 着色、方块实体与诊断详情

群系颜色混合半径默认为 2（可通过 `--biome-blend-radius <0..=7>` 或 `options.txt` 的 `biomeBlendRadius` 在启动期配置），按固定 `y` 层的 `x`/`z` 窗口求平均；构网与粒子生成按需计算并依赖区段重绘，替代独立 `BlockTintCache`。方向光照经世界到地形投影传递，`TINTED_LEAVES` 使用 `client_leaf_tint_color` 与客户端树叶着色源。

`block_entity_visibility` 统一裁决两条可见性路径：普通方块实体先过已编译 16³ 区段可见性、视锥与渲染距离（默认 64 格，活塞头 `PISTON_HEAD_VIEW_DISTANCE = 68` 格，末地折跃门 256 格）；离屏渲染成员绕过区段与视锥。`off_screen_owner_is_visible` 对信标使用水平 `renderDistance × 16`，对包围盒渲染器使用球面 96 格，测试实例方块取两者并集；`renders_off_screen` 是唯一全局离屏成员表。光束在投影入口过门后不再重复判距，移动活塞经 `MovingPistonVisibility` 回到区段可见性门槛。

方块实体渲染绑定直接以实际投影函数为准，不再复制平行注册表：告示牌与悬挂告示牌、各类箱子、刷怪笼、末地传送门/折跃门各自共用对应场景路径；篝火、可刷方块、宝库与架子共用 `item_models::block_entity` 并按各自变换分支；活塞附着物经 `item_models::dropped` 提交。

`client_level_report_details` 按需从世界装配崩溃报告中的 `Affected level` 段落（玩家、区块统计、维度、时间、出生点、服务端品牌/类型、实体数量与天气），严格保持 Vanilla 字段顺序。`chunk_source_stats_line` 共供 HUD 与崩溃报告，出生点采用含最小/最大高度的位置格式。致命错误 `attach_level_details` 仅捕获当帧同代快照，首个非空结果生效，无活动世界时省略而不补录陈旧快照。

### 活塞固定事实与客户端方块推移

Vanilla `PistonStructureResolver` 与 `PistonBaseBlock.isPushable` 以全局方块状态为查询坐标。BBB 的活塞推动反应与不可破坏标志（`destroySpeed == -1.0F`）唯一归属 `bbb-facts::block_state_types` 的 26.1 运行时静态事实表，方块实体关系直接复用同 crate 的「方块类型 → 方块实体」目录。全部状态的打包活塞事实仅占一个私有 `u8` 数组，公开接口限于支撑事实查询、五值反应枚举与不可破坏查询。

`bbb-client-world::blocks::piston_blocks` 是唯一客户端玩法消费者：已加载状态 ID 直接查询生成事实表，保持 12 格上限、推线/碰撞/分支、推动反应、边界与粘液块/蜂蜜块顺序，再由 `moving` 原子应用完整推移结果。未加载位置视作空气，已加载的未知状态或事实缺失直接失败；纯解析失败不改世界，落位应用失败保留原世界。活塞事实仅随协议版本更新，不存在资源包或会话缓存。

## BBB 实现映射

| 语义所有者 | 仓库内路径 | 消费者 |
| --- | --- | --- |
| 世界、加载身份与时钟 | `bbb_client_world::level` | 模拟层与加载等待运行时 |
| 区块与光照存储 | `bbb_client_world::chunks::{store, map}` | `ChunkMap` 到达序/位图/方块实体索引、构网目标与修订号 |
| 实体调度与动画 | `bbb_client_world::entities::animations` | 不可变世界快照发布 |
| 地图数据存储 | `bbb_client_world::maps` | 地图纹理与物品展示框 |
| 声音种子与世界事件 | `bbb_client_world::client::audio` | 客户端 FIFO 音频副作用 |
| 环境动画与粒子运行时 | `bbb_client::environment_animate_runtime`、`particle_runtime` | 渲染与音频输入（含 `block_entity_effects.rs`） |
| 方块实体可见性 | `bbb_client::block_entity_visibility` | 场景帧投影 |
| 世界诊断与崩溃详情 | `bbb_client_world::client::level_report` | F3 调试 HUD 与崩溃报告 |
| ICU 区域与时区日期格式化 | `bbb_client_world::local_time` | 文本组件与界面日期/时间格式化 |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 世界与玩家加载身份 | `ClientPacketListener.handleRespawn` | `bbb_client_world::level` | `aligned` | 同维/跨维重生身份隔离测试 |
| 队列化光照与可见门 | `pollLightUpdates`、光照就绪标志 | `bbb_client_world::chunks::store` | `aligned` | 初始光照等待、重发与裁剪测试 |
| 冻结与 `SetTime` 同步 | `ClientLevel.tick`、时钟更新 | `bbb_client_world::level` | `aligned` | 冻结时间、闪电闪白与重同步测试 |
| 地图数据跨维携带 | `getAllMapData`/`addMapData` | `maps`、`level` | `aligned` | 登录清空与跨维重生保留测试 |

## 结构性不变量

- `client_level_generation` 只有一个写入者，同维度重生绝不能借清理缓存递增世界代际。
- 数据沿「网络包/资源/输入 → 世界权威状态 → 不可变副作用/后端投影」单向流动，下游消费者不得反读权威输入。
- `ClientLevel` 的生命周期、时钟、区块、实体与环境效果分别绑定唯一生产所有者，不得以旁路缓存冒充权威世界状态。

## 聚焦测试（focused tests）和真实证据

| 可定位测试名称 | 所属模块与验证边界 |
| --- | --- |
| `frozen_ticking_gates_production_time_advance_but_not_set_time_resync` | `bbb_client_world::level`：冻结门、原始闪电闪白与 `SetTime` 重同步 |
| `login_resets_map_store_while_dimension_respawn_carries_it_over` | `bbb_client_world::level`：新登录清空与跨维重生保留地图表 |
| `pending_initial_light_lifecycle_follows_load_apply_prune_and_direct_insert` | `bbb_client_world::chunks::tests`：初始光照等待、应用、裁剪与直接插入 |
| `frozen_client_tick_schedule_keeps_player_roots_and_player_passenger_trees_running` | `bbb_client_world::entities::tests::passenger_and_vehicle_movement`：冻结时保留含玩家的载具树 |
| `same_dimension_respawn_restarts_load_timeout_without_replacing_level_identity` | `bbb_client::level_load_runtime`：同维重生重启加载超时且不替换世界代际 |

常用验证命令：`cargo dev test -p bbb-client-world pending_initial_light`、`cargo dev test -p bbb-client-world frozen_`、`cargo dev test -p bbb-client-world login_resets_map_store` 以及 `cargo dev test -p bbb-client same_dimension_respawn`。单元测试不能替代实机联机回放验收。

## 当前差异

Vanilla 对未加载区块的噪声群系默认回退为 `PLAINS`，BBB 的 `biome_id_at` 返回 `None` 并在群系混合平均时排除该列。群系混合半径默认 2，可通过 `--biome-blend-radius <0..=7>` 或 `options.txt` 的 `biomeBlendRadius` 在启动期配置（无游戏内配置界面）。崩溃报告中的服务端类型恒为 `Non-integrated multiplayer server`，符合外部多人客户端的产品定位。[客户端账本](../../../unsupported-client-features.md)当前无未关闭的 `# Todo` 缺口。

## 账本归属

本页对应机制 `world-client-level-lifecycle`（涵盖世界代际、区块/光照、实体调度、时钟、地图、环境动画与诊断）。产品范围裁决与未关闭差异统一查[客户端账本](../../../unsupported-client-features.md)，推进原则见 [AGENTS.md](../../../../AGENTS.md)。

## 精确重开条件

当 Vanilla 调整 `ClientLevel` 生命周期、重生携带策略或 tick/冻结顺序，或 BBB 新增离屏方块实体渲染器与注册绑定时，须重新核验对应测试。若出现未加载群系导致肉眼可见着色偏差、旧加载就绪边沿误放行、旧代际异步结果被消费，或出现第二个光照/时钟/实体状态所有者，则按对应模块定位修复。

## 相关页面和源码索引

- [客户端本地输入、移动与交互](../player/client-input-and-interaction.md)：玩家重建、输入与本地预测。
- [客户端协议状态流](../protocol/client-state-flow.md)：连接顺序与快照发布。
- [客户端方块渲染](../rendering/client-block-rendering.md)：地形构网与资源消费。

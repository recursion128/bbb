# 服务端边界与 Agent 管理

## 产品边界

BBB 服务端是 Minecraft Java 26.1 的 repo-native、offline-only、headless dedicated
server，以 Linux x86_64 与 macOS aarch64 为正式无头发布平台（Windows 支持本地编译、运行与 E2E 联调，
不提供 Windows 服务托管包或窗口化 GUI）。它是独立进程，不是 `IntegratedServer`，
不由客户端 pause/world-selection 生命周期托管。启动配置全部来自 CLI；world、playerdata、
allowlist、ban、operator 与 datapack 选择属于运行数据和持久状态，不构成第二套启动配置 owner。

服务端基本对齐 Vanilla 玩家可见功能，实现自由并追求更少代码与更强性能（原则见
[AGENTS.md](../../../AGENTS.md)）。在声明的 offline-only 认证边界内，协议严格兼容
Minecraft Java 26.1 wire（可观察的 packet、字段、phase、顺序与同步）。必须正确读取官方
26.1 存档与 datapack 的合法内容；BBB 自身存档只需 BBB round-trip。worldgen 对齐生成机制，
对同一 BBB seed/datapack 自身确定，但不要求与官方服务端生成相同 chunk、结构坐标或随机序列。
online-mode、RCON、Query、stdin 交互控制台、窗口化服务端 GUI、第三方插件/mod ABI 与
Windows 正式发布服务托管不在当前产品发布面。

## Vanilla 26.1 权威 owner

关键事实只从 `cargo dev vanilla path 26.1`（或 `scripts/vanilla path 26.1`）返回的已校验源码重新取证：

- 启动、world/datapack load 与 shutdown：`net.minecraft.server.Main`、
  `WorldLoader`、`LevelStorageSource`、`ServerPacksSource`、`MinecraftServer.spin`。
- server loop、tick、save、reload 与 dimension 生命周期：`MinecraftServer`、
  `DedicatedServer`、`ServerLevel`、`ServerChunkCache`、`ChunkMap`。
- 玩家与权威玩法入口：`PlayerList`、`ServerPlayer`、`ServerGamePacketListenerImpl`
  及各 block/entity/item/menu owner。
- 连接与 phase：`ServerConnectionListener`、`ServerHandshakePacketListenerImpl`、
  `ServerLoginPacketListenerImpl`、`ServerConfigurationPacketListenerImpl`、
  `ServerCommonPacketListenerImpl`、`ServerGamePacketListenerImpl`。
- packet direction/id/wire：`GameProtocols.java`、各 `*Protocols.java` 与对应
  `Serverbound*Packet`/`Clientbound*Packet` 的 `STREAM_CODEC`。
- 管理语义：`IncomingRpcMethods`、`MinecraftApi`、
  `MinecraftExecutorServiceImpl`、`JsonRpcNotificationService` 与
  `DedicatedServer.tickServer`。

Vanilla 26.1 已有 WebSocket JSON-RPC management server：它提供 discovery、player、
allowlist/ban/operator、server status/save/stop/message、settings、gamerules 与 lifecycle/player
notifications；`MinecraftApi.submit` 把操作交回 server executor，management connection 在
`DedicatedServer.tickServer` 中推进。BBB 复刻这些重叠语义和线程所有权，但不复制其
WebSocket/API-key wire；对外协议只有 MCP。

## 语义 owner 与依赖方向

| 边界 | 拥有 | 不拥有 | 输入到输出 | 生命周期与 fallback |
| --- | --- | --- | --- | --- |
| shared protocol | 26.1 packet catalog、wire value、Component/NBT、registry/item 生成事实 | socket、client/server world | bytes ↔ typed packet/value | 版本固定；越界输入拒绝；复用现有 `bbb-protocol`（wire）、`bbb-facts`（生成事实）与 `bbb-registry-codec`（registry/persistent codec） |
| server network | listener、connection phase、compression、收发队列、backpressure | world 规则 | socket frame → ordered server command；outgoing event → frame | connection scoped；关闭唤醒 waiter；不复用 client `bbb-client-net` 状态机 |
| authoritative world | dimension、chunk、entity、player、inventory、tick、玩法结果 | renderer/audio/backend | packet/admin command + tick → canonical mutation + outgoing events | server/world scoped；单 tick writer；不复用 client `bbb-client-world` canonical state |
| server data | datapack、dynamic registry、tag、recipe/loot/function/worldgen、save schema | client resource generation | pack/save providers → validated immutable data generation/world restore | reload 原子发布；失败保留旧 generation 或拒绝启动 |
| runtime | CLI、world 线程、存档写线程、signal、任务调度、crash/soak | gameplay truth | process/config → owned services → graceful close | Linux/macOS headless 发布，Windows 支持本地开发与 E2E；无 window/renderer/audio/winit |
| admin service | typed query/command、scope、audit、operation identity | MCP transport、直接 world borrow | principal request → bounded tick mailbox → typed result | server scoped；同 tick canonical ordering；查询在 tick 内构造 owned 结果 |
| MCP adapter | tool/resource/subscription schema、Streamable HTTP、auth/version projection | authoritative state、第二命令系统 | MCP JSON-RPC ↔ typed admin service | 默认 loopback；远程显式 TLS/OAuth/gateway；版本 pin/conformance |

crate owner 为 `bbb-server-net`（连接状态机）、`bbb-server`（headless runtime/binary）、
`bbb-server-admin`（MCP adapter）、`bbb-admin`（仅依赖 `bbb-mcp` 的轻量 human CLI client，零服务端生产依赖），以及权威 world/tick/data
的五个 crate（均不依赖 `bbb-render-types`）：

| crate | 拥有 |
| --- | --- |
| `bbb-server-types` | 错误/结果、save 合同（含 `save::world_gen`）、物品值、fixture、几何/方向/interaction、纯方块查询/参数解析、sculk cursor 保存值、玩家经验（`experience`）/槽位范围（`slot_ranges`）/统计/进度文件值（`advancements`）/食物与伤害值、熔炉燃料燃烧时长重导出（权威表归 `bbb-facts::fuel_burn_times`）、random sequence 保存值、流体状态投影、chunk/section/biome/heightmap 与 chunk tick 容器（纯光照传播与体转向归 `bbb-protocol`，实体默认属性归 `bbb-facts::default_attributes_26_1`） |
| `bbb-server-datapack` | datapack 加载与 dynamic registry（`/reload` 保持启动冻结注册表，不重读磁盘动态注册表 JSON）、预设与平坦/调试生成源（`flat_generator`、`debug_generator`）、damage type、food、玩家进度树与内嵌 vanilla datapack snapshot |
| `bbb-server-worldgen` | feature、noise/density、carver、structure、biome source；服务端生产依赖只有 `bbb-server-types` 和 `bbb-server-datapack`（另共享 `bbb-facts`、`bbb-protocol` 与 `bbb-registry-codec`） |
| `bbb-server-persistence` | `level.dat`/region/chunk/entity/poi/player schema、SavedData、`atomic_file` 与 `session_lock`（不保留原始 chunk NBT AST，与 `bbb-server-worldgen`、`bbb-server-datapack` 及玩法完全解耦） |
| `bbb-server-core` | 方块、流体、机器、背包/物品/loot、区块/光照/生成编排、实体/AI/tracking 与 `PluginMobBehavior`（扫掠碰撞复用 `bbb_facts::collision`）、玩家/session、世界创建（`world_creation`）、tick、命令、管理（含 `serversettings/game_mode` 等设置）、mailbox、`registry_sync` 与 outgoing packet |

启动时用 `--mod <slug>` 启用内置玩法扩展，例如 `demo_boss`。用 `--mod-packs <path>`（支持可见别名 `--mod-pack <path>`）加载 JSON 清单，目录参数读取其中的 `content_registry.json`。清单声明实体、方块、物品、声音和粒子，资源仍由客户端资源包提供。服务端冻结 `ServerPluginRegistry` / `ContentRegistry`，并在 Configuration 阶段经 `bbb:content_registry` 下发。使用这些内容的会话需要 BBB 客户端。原版会话使用 Vanilla `BuiltInRegistries`。

双端共用的熔炉燃烧时长规则与实体默认属性分别归口 `bbb_facts::fuel_burn_times` 与 `bbb_facts::default_attributes_26_1`。

### 运行时存储与借用上下文

Block、Chunk、Entity 与 World 的玩法模块由 Core 统一拥有；纯区块与计划刻容器归 `bbb-server-types`，
两端共用的纯光照存储/传播核心（`chunk_light`）与生物体转向（`entity_body_rotation`）归 `bbb-protocol`。
Core 的 `chunk`、`entity` 模块各拥有实际存储与执行，其余玩法模块保留语义归属。
光照传播核心借用方块与区块查询，不依赖 Core；服务端生成、恢复、源扫描与传播编排仍归 Core，
客户端只复用算法和批次临时存储，不共享权威世界状态。
内置方块声音事实由共享的 `bbb-facts::block_sound_facts_26_1` 拥有，经 protocol 提供给两端。
纯类型消费者直接依赖 `bbb-server-types`，Core 不转发错误、save、物品、几何、方向或 chunk tick 类型。

世界生成的方块生存、放置与形状更新由 Core 的实际区域 host 调用唯一方块行为 owner；
Core 的只读方块和亮度访问统一使用 `BlockGetter`，玩法与生成 host 共用查询规则。
实时方块行为直接调用 `LevelCtx`；形状更新的 `BlockShapeCtx` 只承载实时世界、生成区域和存档升级
所需的读取、标签、随机数与计划刻，不携带库存、事件或空写入回调。
实时玩法只借用一个平铺的 `world::simulation_level::LevelCtx`：区块、规则、事件、计划刻、
邻更队列与实体上下文直接位于该借用中，由 `LevelCtx::new` 唯一构造，子调用重新借用同一存储；
运行时改方块只经 `LevelCtx::set_block`（区块段原始写入只经 `write_block`），计划刻直接写入关卡存储。
未达到 FULL 的写入目标由 `chunk_lifecycle/full_preparation` 同步准备；World 与 `LevelCtx` 共用
生成、原始生物生成、存档恢复与实体发布 owner，实体在关卡借用结束后装入实际 EntityStore。
`WorldGenLevel` 只有两个宿主：生成区域 `FeatureWorldGenRegion` 与实时 `LevelCtx`；`/place`、末地平台与
折跃门、结构方块加载、幽匿催发体与附魔圆盘谓词都在一个 `LevelCtx` 帧内执行，特征随机数从关卡随机数
派生，结构生物在帧结束后由 World 终结发布。

### 实体、AI、随机数与刷怪所有权

实体移动、伤害、AI 和接触直接调用 `LevelCtx`；临时离开 EntityStore 的真实 body 以借用链供来源
和乘客查询使用，实体或活塞批次不包裹第二个世界宿主或冻结标签事务。
玩法随机数只有一条流：`crate::random::GameRandom`（wyrand）由 `ServerWorld.random` 拥有，各关卡、
EntityStore 与 AI runtime 持同一流的句柄，`LevelCtx` 借出它；实体、玩家、声音与方块随机刻不另存随机状态，
新实体 UUID 取 `Uuid::new_v4`。只有 seed 非零的战利品表与结构模板按 seed 新建一条流。

EntityStore 每个实体 id 一个槽位：body（tick 期间借出为空）、AI runtime、乘骑关系同在槽内，
不维护按 id 平行的 map。在线玩家也是所在关卡 EntityStore 的槽位：`PlayerState` 以 body 实体 id
入槽，按同一 section 索引与乘骑边查询，通用实体查询与实体区块存档看不到它；连接级的区块发送与
tracking view 留在按 ConnectionId 索引的 `PlayerSession`，`LevelArena` 维护 ConnectionId→(关卡, id)
并在重生/传送门时把玩家移到目标关卡；实体回调需要玩家、地图、命令或整关卡的工作统一排入 `EntityCommand`
队列，由 World 唯一的 `run_entity_commands` 按序执行（后续命令先于旧命令）。

生物体 `GeneralLivingData` 只持有 Mob/AgeableMob/Animal 基类、内部物品栏、脑与跨族共享变种，
其余按类型收进一个 `MobVariantState`；存档读写、同步元数据与命令重载都从这份状态分派。
实体与会话玩家共用的规则（流体/接触扫描读 `ChunkStore`，方块接触、踩踏、落地与移动发声）
返回决策值，由实体 tick 与玩家 session 各自应用；物品伤害事实读物品发布，脑 AI 只读
`BrainBodyFacts` 值与真实寻路/采样查询。刷怪谓词统一读 `SpawnPredicateLevel`，
生物终结统一经 `finalize_spawn` 与 `SpawnLevel`（实体存储、`SpawnContext`、附魔、无碰撞与可骑载具查询），
World 侧只有 `MinecartSpawnerRuntime`（`ServerWorld::with_spawn_runtime`）与自然刷怪的 `NaturalSpawnLevel`
两个实现（生成区域为 `NaturalEnvironment` 的标志）。

### 持久化、事件、战利品与命令边界

结构方块捕获直接选择 World 或 `LevelCtx` 的实际存储；关卡捕获保持边界检查与区块加载规则。
方块事件直接由 `LevelCtx` 执行当前方块或方块实体行为，World 负责实际会话的声音与事件广播。
游戏事件的 Section 只持有监听数据，位置解析、范围筛选与投递由 World 直接执行。
Persistence 只读写配方簿列表，运行时配方簿由 Core inventory 构造。

战利品抽样直接读关卡随机流，不按表名维护命名序列；`random_sequences.dat` 仍由 World 的
序列存储完整读写，只供 `/random` 命令使用。嵌套容器填充不经过世界宿主回调。
战利品上下文的实体键直接借用真实存储实体（`StoredLootEntity`）或携带会话玩家/击杀快照值
（`LootEntityFacts`），伤害来源与方块实体同样传实际值，不经过 loot 视图 trait 回调。
玩家保存快照（`world/player_save`）直接接收真实 RootVehicle 读取结果并携带实时 warning tracker 值；
保存、命令与 loot 消费者调用同一个快照 owner；保存请求与恢复分别归 `world/save`、`world/restore`。

存档区块恢复本体归 `ChunkStore`；启动、运行时发布和冷加载调用同一个 owner，World 负责激活
后的票据、实体生命周期与计划刻登记（`ChunkStore::register_ticks`），不通过反向恢复宿主回调。
Core 内部 helper 仅在 `cfg(test)` 下编译；数据 crate 不提供 `test-support` feature。
方块形状增长抬升实体由 World 的实际存储与会话借用直接执行，查询和移动不经过反向 access trait。
火焰环境以 `FireFacts` 借用真实规则、天气、registry 投影与玩家位置，不通过反向环境宿主提供事实。
物品放置候选直接在 World 上读取、预约计划刻与抬升实体，共用真实形状变化 owner；
权限、光照与碰撞不通过 placement effects 宿主或整维度实体副本查询。
刷怪请求的实际应用由 World 执行，直接发布关卡事件与实体动画；解码计划与规则数据由实体模块提供。
命令经 `CommandCx` 直接写 `ServerWorld`，不构造快照、不回放 effect；多目标部分失败不回滚。
跨边界测试使用真实 NBT、light packet、chunk 生成与资源解析 API，不暴露调度、分配或 RNG 探针。
`bbb-server` 与 `bbb-server-admin` 直接消费 Core 的实际 API，`bbb-admin` 仅通过 `bbb-mcp` 协议交互。

### 线程模型、网络路由与背压

`bbb-server-core` 的 socket/admin/lifecycle producer 共享全局 sequence，但各自拥有独立
entry/retained-byte quota；饱和返回可归因 backpressure。线程所有权：`ServerWorld` 只在
`bbb-world` OS 线程上运行并按 `MinecraftServer.runServer` 节拍（含追赶与过载跳过）tick；
tokio 只做 listener/连接、MCP 与信号。入站经 world mailbox，出站由 world 线程经非阻塞
`AsyncRouteDispatcher` 投递到网络运行时的有界单连接命令队列（广播包与跨观察者共享的 `LevelChunkWithLight` 经 `SharedWorldPacket` 单次预编码/压缩，连接写循环聚合并发帧后单次写出，超额慢客户端在网络侧保序关闭，管理结果仍排在其前序包之后）。连接驱动与
世界输入投递独立轮询，避免入站背压阻塞出站；Configuration 阶段支持 `SelectKnownPacks`（`minecraft:core` `26.1`）与预编码注册表同步帧，Play 阶段支持 `StartConfiguration` → `AcknowledgeReconfiguration` 重配置屏障及 `PlayerCombatEnter`/`PlayerCombatEnd`，额度与关闭规则见[连接机制](../mechanisms/protocol/server-connections-and-sessions.md)。

listener 读取 world 线程每 tick 发布的 status/registry 配置。存档读写分离到独立的
`bbb-save-read` 与 `bbb-save-write` 线程（通过 `RegionFileCache` 共享读/写锁协同，按需读取
不再被后台写队列阻塞）：存档请求在 tick 边界冻结元数据与轻量延迟区块快照
（`DeferredChunkWrite`），由 `bbb-save-write` 离世界线程物化区块 NBT 并通过 `rayon` 并行
压缩写盘；无人请求的 autosave 与批量 `ChunkUnload` 均以 epoch 跟踪异步完成（确认只退休
epoch/内容复核的 intent），仅管理保存与关闭 flush 排空在途读取并等待写完确认。
停止（信号或 `server/stop`）→ world 线程跑关闭 tick、等写线程落盘 → 关闭连接 → 退出。
区块生成的纯阶段（STRUCTURE_STARTS、BIOMES、noise/surface/carvers 与 INITIALIZE_LIGHT 源扫描）在
启动时建好的 rayon 全局池（`bbb-worldgen-N`）上算，只读冻结 program 与拥有的输入；FEATURES、LIGHT
传播、SPAWN、FULL 与所有 `ChunkStore` 写入只在 world 线程按请求顺序提交，详见[服务端区块](../mechanisms/world/server-chunks-and-worldgen.md)。
tick 按 sequence 串行应用命令，每 tick 发布在线玩家状态（遵守 `allows_listing` 匿名规则）、准入快照与同步注册表快照；存档或配置的 `level-type`/数据包维度按真实世界运行，仅未传 `--world` 时使用内置 flat 临时世界。未知的已登记 play packet 在语义 owner 落地前关闭连接。
运行中的维度使用 `LevelId(u32)` 与 arena，资源键只在输入、协议和存档边界解析或投影；
自定义维度保持既有数量支持，不添加 256 个的上限。资源名不可变并由关卡与索引共享，
reload 保持 ID，跨维度转移直接借用两个关卡。

### 分派表与强类型 Worldgen

`bbb-server-core` 里按 resource id 分派的 owner 族只保留一张表作为 id→owner 事实源
（`block_entity::block_entity_payloads!`、`world::block_use::block_use_dispatch!`）：表同时
展开分派函数与 id 常量，Rust 测试断言表 key ⊆ generated vanilla 注册表。新增 owner = 新建 `foo/<owner>.rs` 放 owner 函数与其行为测试 +
facade 表里一行（外加 `mod` 行）；不复制 dispatch arm、`ensureCanWrite` 前置、ledger 常量或
逐 id 样板测试，也不改 facade 的任何分支。这也是 lane 并行的文件边界：各 lane 只写自己的
owner 文件，共享 facade 只接受表行/`mod` 行追加。

worldgen 的 density/noise/settings/surface、configured/placed feature、configured carver、
placement modifier、provider、rule test、structure processor 与树 placer 在 datapack 加载时
一次解码为 `bbb_server_datapack::worldgen` 强类型，
非法内容在加载时拒绝；`bbb-server-worldgen` 只读这些类型，Feature 放置是对
`ConfiguredFeature` 枚举的穷尽 `match`，不经 resource id 表或 NBT 归一化层。
这些强类型注册表保留原始资源字节与解码值，不保留多余的 `serde_json::Value` 投影；
其它仍有 JSON 读取者的注册表按实际需要保留投影。
`IntProvider`、`FloatProvider` 的值类型、字段解码和范围规则由 `bbb-registry-codec` 单一拥有；worldgen 直接
消费该值，注册表发布只在输出时消费投影为 NBT，不经 NBT 反解回运行时值。
附魔从同代 retained codec 读取浮点 provider 并委托相同解码器；玩法与 worldgen 各自保留采样路径。
世界生成的随机流只来自 `bbb-server-worldgen::random`（种子哈希派生）；`bbb-registry-codec` 的
Java LCG/xoroshiro 只服务客户端、玩法与 random sequence。

## 优先复用客户端已有工作

复用以语义中立为前提，而不是按文件数量追求共享：

- 直接复用 `bbb-protocol` 的 packet catalog、Encoder/Decoder primitives、Component/NBT 与
  resource location，`bbb-registry-codec` 的 registry holder/persistent codec，以及 `bbb-facts`
  的 item/data component、block/entity/particle 等 generated Vanilla facts。新增 serverbound decode/clientbound encode 必须落在同一事实源。
- `bbb-client-net` 保持客户端 socket/phase/event owner；服务端只复用底层 frame/codec 中真正中立
  的部分，不能把 client command/event enum 改名后当服务端 listener。
- `bbb-client-world` 是可见客户端副本、本地预测和 presentation state，不能成为服务端世界。
  可以下沉无状态数学、碰撞形状或 generated catalog，但必须先证明双向 consumer。
- `bbb-client-assets` 是客户端 resource-pack generation；服务端 datapack 有不同 schema、reload
  consumer 与持久化生命周期，不能反向依赖 renderer 或 client pack catalog。
- bbb client 的 playable fixture、command encoder、world queries 和 control observations
  优先作为服务端 integration 测试驱动。同一 replay 对官方 offline 26.1 server 的归一化比较
  只用于调试定位。

flat replay 的身份、预测与发包验证见[服务端连接](../mechanisms/protocol/server-connections-and-sessions.md)。

## 核心机制与交接合同

| 机制 | canonical 页面 | 边界 |
| --- | --- | --- |
| tick、光照屏障与回调阶段 | [服务端 tick 与世界](../mechanisms/world/server-tick-and-world.md) | 唯一 writer；维度、sequence、phase 与 lifecycle 顺序 |
| 红石与活塞 | [服务端红石](../mechanisms/blocks/server-redstone-and-pistons.md) | 信号、邻更、scheduled tick 与 block event 分属各 owner；活塞实体交互有明确限制 |
| 保存、恢复与 datapack reload | [服务端保存与重载](../mechanisms/persistence/server-save-and-reload.md) | tick snapshot 与 candidate 分离；whole-file journal 不提供跨 region 回滚 |
| MCP、scope、deadline 与操作结果 | [服务端 MCP 管理](../mechanisms/control/server-mcp-administration.md) | 普通查询和 mutation 都经 mailbox；outcome 不代表磁盘 flush 完成 |
| 连接、配置与 play 交接 | [服务端连接](../mechanisms/protocol/server-connections-and-sessions.md) | net phase 与 world admission 分离；压缩在准确 packet 边沿生效 |
| 区块与世界生成 | [服务端区块](../mechanisms/world/server-chunks-and-worldgen.md) | ticket、生成状态、光照与 FULL publication 分开 |
| 方块、流体与机器 | [服务端方块](../mechanisms/blocks/server-block-fluid-and-block-entities.md) | setBlock、neighbor 与 payload/ticker 各有明确 owner |
| 实体与 AI | [服务端实体](../mechanisms/entities/server-entities-and-ai.md) | UUID、network ID、tracking baseline 与运行瞬态分开 |
| 玩家、背包与战斗 | [服务端玩家](../mechanisms/player/server-player-inventory-and-combat.md) | 权威 slots 与 remote prediction 分开，回调遵守 world 命令序 |
| datapack 与 loot | [服务端规则资源](../mechanisms/resources/server-datapack-and-loot.md) | 加载、引用和玩法执行分别验收 |
| 命令、成就与计分板 | [服务端命令](../mechanisms/control/server-commands-and-progression.md) | 命令直接写世界、部分失败不回滚，同调用树与奖励共享预算 |

network、signal、MCP 和资源准备者提交 owned 请求，不借用可变世界。在线玩家列表提供
窄 publication；服务端 MCP 普通 resource 在 tick 内读取 `AdminWorldView`，operation URI
读取 transport 的结果记录。HTTP 等待超时与 world tick deadline 是两个独立边界。

MCP 是唯一远程管理协议；`bbb-admin` 与 agent 共用 endpoint、schema 与授权，内部
`AdminGateway` 是 typed 接口。`command.execute` 委托唯一 command dispatcher。
SIGTERM/SIGINT 交给 graceful save/stop owner；systemd/launchd 负责进程启动、崩溃重启和
强制终止。stdout/stderr 输出结构化日志，stdin 不读取命令，不启动 RCON/Query。

## 验收层级

1. focused：packet codec、phase、budget、datapack 加载、admin scope 与玩家可见规则结果。
2. integration：bbb client ↔ BBB server 的确定性 flat fixture 与双客户端同步。
3. persistence：读取官方 26.1 存档不丢合法数据，BBB 自身 save/restart/reload round trip。
4. management：MCP conformance、auth/scope、retry/idempotency、audit、CLI client。
5. release：20 clients、save/restart/reload、malformed input、RSS/FD/thread/tick soak，以及
   Linux/macOS signal/supervisor headless contract。

官方 differential/world-oracle 只作调试工具。

具体 active/deferred/not-needed 状态只在
[`docs/unsupported-server-features.md`](../../unsupported-server-features.md)；推进原则见
[`AGENTS.md`](../../../AGENTS.md)，开发与提交流程只取
[开发流程](../development/testing-and-gates.md)。

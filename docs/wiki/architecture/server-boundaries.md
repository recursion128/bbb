# 服务端边界与 Agent 管理

## 产品边界

BBB 服务端是 Minecraft Java 26.1 的仓库原生、仅离线模式、无头独立服务端，以 Linux x86_64 与 macOS aarch64 为正式无头发布平台（Windows 支持本地编译、运行与 E2E 联调，不提供 Windows 服务托管包或窗口化 GUI）。它是独立进程，不是 `IntegratedServer`，不由客户端暂停或选世界生命周期托管。启动配置全部来自 CLI；世界存档、玩家数据、白名单、封禁表、管理员表与数据包选择属于运行数据和持久状态，不构成第二套启动配置所有者。

服务端基本对齐 Vanilla 玩家可见功能，实现自由并追求更少代码与更强性能（原则见 [`AGENTS.md`](../../../AGENTS.md)）。在声明的仅离线模式认证边界内，协议严格兼容 Minecraft Java 26.1 线协议（可观察的数据包、字段、阶段、顺序与同步）。必须正确读取官方 26.1 存档与数据包的合法内容；BBB 自身存档只需保证 BBB 往返一致。世界生成对齐生成机制，对同一 BBB 种子与数据包自身确定，但不要求与官方服务端生成相同区块、结构坐标或随机序列。正版在线模式、RCON、Query、stdin 交互控制台、窗口化服务端 GUI、第三方插件/模组 ABI 与 Windows 正式发布服务托管不在当前产品发布面。

## Vanilla 26.1 权威所有者

关键事实只从 `cargo dev vanilla path 26.1`（或 `scripts/vanilla path 26.1`）返回的已校验源码重新取证：

- 启动、世界/数据包加载与关闭：`net.minecraft.server.Main`、`WorldLoader`、`LevelStorageSource`、`ServerPacksSource`、`MinecraftServer.spin`。
- 服务端主循环、tick、保存、重载与维度生命周期：`MinecraftServer`、`DedicatedServer`、`ServerLevel`、`ServerChunkCache`、`ChunkMap`。
- 玩家与权威玩法入口：`PlayerList`、`ServerPlayer`、`ServerGamePacketListenerImpl` 及各对应方块/实体/物品/菜单所有者。
- 连接与协议阶段：`ServerConnectionListener`、`ServerHandshakePacketListenerImpl`、`ServerLoginPacketListenerImpl`、`ServerConfigurationPacketListenerImpl`、`ServerCommonPacketListenerImpl`、`ServerGamePacketListenerImpl`。
- 数据包方向、ID 与线协议格式：`GameProtocols.java`、各 `*Protocols.java` 与对应 `Serverbound*Packet`/`Clientbound*Packet` 的 `STREAM_CODEC`。
- 管理语义：`IncomingRpcMethods`、`MinecraftApi`、`MinecraftExecutorServiceImpl`、`JsonRpcNotificationService` 与 `DedicatedServer.tickServer`。

Vanilla 26.1 已有 WebSocket JSON-RPC 管理服务：它提供服务发现、玩家、白名单/封禁/管理员、服务端状态/保存/停服/广播、设置、游戏规则与生命周期/玩家通知；`MinecraftApi.submit` 把操作交回服务端执行器，管理连接在 `DedicatedServer.tickServer` 中推进。BBB 对齐这些管理语义和线程所有权，但不复制其 WebSocket/API-key 线协议；对外管理协议统一为 MCP。

## 语义 owner 与依赖方向

| 边界 | 拥有 | 不拥有 | 输入到输出 | 生命周期与回退行为 |
| --- | --- | --- | --- | --- |
| 共享协议层 | 26.1 数据包目录、线协议值类型、文本组件/NBT、注册表/物品生成事实 | 套接字、客户端/服务端世界 | 字节流 ↔ 强类型数据包/值 | 版本固定；越界输入拒绝；复用现有 `bbb-protocol`（线协议）、`bbb-facts`（生成事实）与 `bbb-registry-codec`（注册表/持久化编解码） |
| 服务端网络层 | 监听器、连接阶段、压缩、收发队列、背压控制 | 世界规则 | 套接字帧 → 有序服务端命令；出站事件 → 网络帧 | 绑定单连接；关闭唤醒等待者；不复用客户端 `bbb-client-net` 状态机 |
| 权威世界层 | 维度、区块、实体、玩家、背包、tick、玩法结果 | 渲染器/音频/图形后端 | 数据包/管理命令 + tick → 权威变更 + 出站事件 | 绑定服务端/世界；唯一 tick 写者；不复用客户端 `bbb-client-world` 权威状态 |
| 服务端数据层 | 数据包、动态注册表、标签、配方/战利品/函数/世界生成、存档结构 | 客户端资源代次 | 数据包/存档提供者 → 校验后的不可变数据代次与世界恢复 | 重载原子发布；失败保留旧代次或拒绝启动 |
| 服务端运行时 | CLI、世界线程、存档读写线程、系统信号、任务调度、崩溃/长跑诊断 | 玩法真值 | 进程/配置 → 自有服务 → 优雅关闭 | Linux/macOS 无头发布，Windows 支持本地开发与 E2E；无窗口/渲染器/音频/`winit` |
| 管理服务层 | 强类型查询/命令、权限范围、审计、操作标识 | MCP 传输、直接借用世界 | 委托方请求 → 有界 tick 邮箱 → 强类型结果 | 绑定服务端；同 tick 规范排序；查询在 tick 内构造自有结果 |
| MCP 适配层 | 工具/资源/订阅 schema、Streamable HTTP、鉴权/版本投影 | 权威状态、第二命令系统 | MCP JSON-RPC ↔ 强类型管理服务 | 默认环回地址；远程需显式 TLS/OAuth/反代；版本锁定与一致性验证 |

crate 归属为 `bbb-server-net`（连接状态机）、`bbb-server`（无头运行时/主程序）、`bbb-server-admin`（MCP 适配层）、`bbb-admin`（仅依赖 `bbb-mcp` 的轻量运维 CLI 客户端，零服务端生产依赖），以及权威世界/tick/数据的五个 crate（均不依赖 `bbb-render-types`）：

| crate | 拥有 |
| --- | --- |
| `bbb-server-types` | 错误/结果、存档合同（含 `save::world_gen`）、物品值、测试夹具、几何/方向/交互、纯方块查询/参数解析、幽匿游标保存值、玩家经验（`experience`）/槽位范围（`slot_ranges`）/统计/进度文件值（`advancements`）/食物与伤害值、熔炉燃料燃烧时长重导出（权威表归 `bbb-facts::fuel_burn_times`）、随机序列保存值、流体状态投影、区块/区段/群系/高度图与计划刻容器（纯光照传播与体转向归 `bbb-protocol`，实体默认属性归 `bbb-facts::default_attributes_26_1`） |
| `bbb-server-datapack` | 数据包加载与动态注册表（`/reload` 保持启动冻结注册表，不重读磁盘动态注册表 JSON）、预设与平坦/调试生成源（`flat_generator`、`debug_generator`）、伤害类型、食物、玩家进度树与内嵌 Vanilla 数据包快照 |
| `bbb-server-worldgen` | 地形地物、噪声/密度函数、雕刻器、结构、群系源；服务端生产依赖只有 `bbb-server-types` 和 `bbb-server-datapack`（另共享 `bbb-facts`、`bbb-protocol` 与 `bbb-registry-codec`） |
| `bbb-server-persistence` | `level.dat`/region/区块/实体/POI/玩家存档格式、`SavedData`、`atomic_file` 与 `session_lock`（不保留原始区块 NBT AST，与 `bbb-server-worldgen`、`bbb-server-datapack` 及玩法完全解耦） |
| `bbb-server-core` | 方块、流体、机器、背包/物品/战利品、区块/光照/生成编排、实体/AI/追踪与 `PluginMobBehavior`（扫掠碰撞复用 `bbb_facts::collision`）、玩家/会话、世界创建（`world_creation`）、tick、命令、管理（含 `serversettings/game_mode` 等设置）、邮箱、`registry_sync` 与出站数据包 |

启动时用 `--mod <slug>` 启用内置玩法扩展，例如 `demo_boss`。用 `--mod-packs <path>`（支持可见别名 `--mod-pack <path>`）加载 JSON 清单，目录参数读取其中的 `content_registry.json`。清单声明实体、方块、物品、声音和粒子，资源仍由客户端资源包提供。服务端冻结 `ServerPluginRegistry` / `ContentRegistry`，并在配置阶段经 `bbb:content_registry` 下发。使用这些内容的会话需要 BBB 客户端。原版会话使用 Vanilla `BuiltInRegistries`。

双端共用的熔炉燃烧时长规则与实体默认属性分别归口 `bbb_facts::fuel_burn_times` 与 `bbb_facts::default_attributes_26_1`。

### 运行时存储与借用上下文

方块、区块、实体与世界的玩法模块由核心层 `bbb-server-core` 统一拥有；纯区块与计划刻容器归 `bbb-server-types`，两端共用的纯光照存储/传播核心（`chunk_light`）与生物体转向（`entity_body_rotation`）归 `bbb-protocol`。核心层的 `chunk`、`entity` 模块各拥有实际存储与执行，其余玩法模块保留语义归属。光照传播核心借用方块与区块查询，不依赖核心层；服务端生成、恢复、光源扫描与传播编排仍归核心层，客户端只复用算法和批次临时存储，不共享权威世界状态。内置方块声音事实由共享的 `bbb-facts::block_sound_facts_26_1` 拥有，经 `bbb-protocol` 提供给两端。纯类型消费者直接依赖 `bbb-server-types`，核心层不转发错误、存档、物品、几何、方向或计划刻类型。

世界生成的方块生存、放置与形状更新由核心层的实际区域宿主调用唯一方块行为所有者；核心层的只读方块和亮度访问统一使用 `BlockGetter`，玩法与生成宿主共用查询规则。实时方块行为直接调用 `LevelCtx`；形状更新的 `BlockShapeCtx` 只承载实时世界、生成区域和存档升级所需的读取、标签、随机数与计划刻，不携带库存、事件或空写入回调。实时玩法只借用一个平铺的 `world::simulation_level::LevelCtx`：区块、规则、事件、计划刻、邻更队列与实体上下文直接位于该借用中，由 `LevelCtx::new` 唯一构造，子调用重新借用同一存储；运行时改方块只经 `LevelCtx::set_block`（区块段原始写入只经 `write_block`），计划刻直接写入关卡存储。未达到 `FULL` 的写入目标由 `chunk_lifecycle/full_preparation` 同步准备；世界层与 `LevelCtx` 共用生成、原始生物生成、存档恢复与实体发布所有者，实体在关卡借用结束后装入实际 `EntityStore`。`WorldGenLevel` 只有两个宿主：生成区域 `FeatureWorldGenRegion` 与实时 `LevelCtx`；`/place`、末地平台与折跃门、结构方块加载、幽匿催发体与附魔圆盘谓词都在一个 `LevelCtx` 帧内执行，特征随机数从关卡随机数派生，结构生物在帧结束后由世界层终结发布。

### 实体、AI、随机数与刷怪所有权

实体移动、伤害、AI 和接触直接调用 `LevelCtx`；临时离开 `EntityStore` 的真实实体主体以借用链供来源和乘客查询使用，实体或活塞批次不包裹第二个世界宿主或冻结标签事务。玩法随机数只有一条流：`crate::random::GameRandom`（wyrand）由 `ServerWorld.random` 拥有，各关卡、`EntityStore` 与 AI 运行时持同一流的句柄，`LevelCtx` 借出它；实体、玩家、声音与方块随机刻不另存随机状态，新实体 UUID 取 `Uuid::new_v4`。只有种子非零的战利品表与结构模板按种子新建一条流。

`EntityStore` 每个实体 ID 一个槽位：实体主体（tick 期间借出为空）、AI 运行时、乘骑关系同在槽内，不维护按 ID 平行的映射表。在线玩家也是所在关卡 `EntityStore` 的槽位：`PlayerState` 以主体实体 ID 入槽，按同一区段索引与乘骑边查询，通用实体查询与实体区块存档看不到它；连接级的区块发送与追踪视图留在按 `ConnectionId` 索引的 `PlayerSession`，`LevelArena` 维护 `ConnectionId → (关卡, id)` 并在重生/传送门时把玩家移到目标关卡；实体回调需要玩家、地图、命令或整关卡的工作统一排入 `EntityCommand` 队列，由世界层唯一的 `run_entity_commands` 按序执行（后续命令先于旧命令）。

生物体 `GeneralLivingData` 只持有 Mob/AgeableMob/Animal 基类、内部物品栏、大脑与跨族共享变种，其余按类型收进一个 `MobVariantState`；存档读写、同步元数据与命令重载都从这份状态分派。实体与会话玩家共用的规则（流体/接触扫描读 `ChunkStore`，方块接触、踩踏、落地与移动发声）返回决策值，由实体 tick 与玩家会话各自应用；物品伤害事实读物品发布，大脑 AI 只读 `BrainBodyFacts` 值与真实寻路/采样查询。刷怪谓词统一读 `SpawnPredicateLevel`，生物终结统一经 `finalize_spawn` 与 `SpawnLevel`（实体存储、`SpawnContext`、附魔、无碰撞与可骑载具查询），世界侧只有 `MinecartSpawnerRuntime`（`ServerWorld::with_spawn_runtime`）与自然刷怪的 `NaturalSpawnLevel` 两个实现（生成区域为 `NaturalEnvironment` 的标志）。

### 持久化、事件、战利品与命令边界

结构方块捕获直接选择世界层或 `LevelCtx` 的实际存储；关卡捕获保持边界检查与区块加载规则。方块事件直接由 `LevelCtx` 执行当前方块或方块实体行为，世界层负责实际会话的声音与事件广播。游戏事件的区段只持有监听数据，位置解析、范围筛选与投递由世界层直接执行。持久化层只读写配方簿列表，运行时配方簿由核心层背包模块构造。

战利品抽样直接读关卡随机流，不按表名维护命名序列；`random_sequences.dat` 仍由世界层的序列存储完整读写，只供 `/random` 命令使用。嵌套容器填充不经过世界宿主回调。战利品上下文的实体键直接借用真实存储实体（`StoredLootEntity`）或携带会话玩家/击杀快照值（`LootEntityFacts`），伤害来源与方块实体同样传实际值，不经过战利品视图 trait 回调。玩家保存快照（`world/player_save`）直接接收真实 `RootVehicle` 读取结果并携带实时警告追踪器值；保存、命令与战利品消费者调用同一个快照所有者；保存请求与恢复分别归 `world/save`、`world/restore`。

存档区块恢复本体归 `ChunkStore`；启动、运行时发布和冷加载调用同一个所有者，世界层负责激活后的票据、实体生命周期与计划刻登记（`ChunkStore::register_ticks`），不通过反向恢复宿主回调。核心层内部辅助代码仅在 `cfg(test)` 下编译；数据 crate 不提供测试专用特性开关。方块形状增长抬升实体由世界层的实际存储与会话借用直接执行，查询和移动不经过反向访问 trait。火焰环境以 `FireFacts` 借用真实规则、天气、注册表投影与玩家位置，不通过反向环境宿主提供事实。物品放置候选直接在世界层上读取、预约计划刻与抬升实体，共用真实形状变化所有者；权限、光照与碰撞不通过放置副作用宿主或整维度实体副本查询。刷怪请求的实际应用由世界层执行，直接发布关卡事件与实体动画；解码计划与规则数据由实体模块提供。命令经 `CommandCx` 直接写 `ServerWorld`，不构造快照、不回放副作用；多目标部分失败不回滚。跨边界测试使用真实 NBT、光照数据包、区块生成与资源解析 API，不暴露调度、分配或随机数探针。`bbb-server` 与 `bbb-server-admin` 直接消费核心层的实际 API，`bbb-admin` 仅通过 `bbb-mcp` 协议交互。

### 线程模型、网络路由与背压

`bbb-server-core` 的套接字/管理/生命周期生产者共享全局序号，但各自拥有独立条数与驻留字节配额；饱和时返回可归因背压错误。线程所有权：`ServerWorld` 只在 `bbb-world` 操作系统线程上运行并按 `MinecraftServer.runServer` 节拍（含追赶与过载跳过）执行 tick；`tokio` 只做监听器/连接、MCP 与系统信号。入站经世界邮箱，出站由世界线程经非阻塞 `AsyncRouteDispatcher` 投递到网络运行时的有界单连接命令队列（广播包与跨观察者共享的 `LevelChunkWithLight` 经 `SharedWorldPacket` 单次预编码/压缩，连接写循环聚合并发帧后单次写出，超额慢客户端在网络侧保序关闭，管理结果仍排在其前序包之后）。连接驱动与世界输入投递独立轮询，避免入站背压阻塞出站；配置阶段支持 `SelectKnownPacks`（`minecraft:core` `26.1`）与预编码注册表同步帧，游玩阶段支持 `StartConfiguration` → `ConfigurationAcknowledged` 重配置屏障及 `PlayerCombatEnter`/`PlayerCombatEnd`，额度与关闭规则见[服务端连接与会话](../mechanisms/protocol/server-connections-and-sessions.md)。

监听器读取世界线程每 tick 发布的服务器状态与注册表配置。存档读写分离到独立的 `bbb-save-read` 与 `bbb-save-write` 线程（通过 `RegionFileCache` 共享读/写锁协同，按需读取不再被后台写队列阻塞）：存档请求在 tick 边界冻结元数据与轻量延迟区块快照（`DeferredChunkWrite`），由 `bbb-save-write` 离世界线程物化区块 NBT 并通过 `rayon` 并行压缩写盘；后台自动触发（未显式要求同步落盘）的自动保存与批量 `ChunkUnload` 均以代次跟踪异步完成（确认只退休代次与内容复核通过的写意图），仅管理保存与关闭刷盘排空在途读取并等待写完确认。停止（信号或 `server/stop`）→ 世界线程跑关闭 tick、等写线程落盘 → 关闭连接 → 退出。
区块生成的纯阶段（`STRUCTURE_STARTS`、`BIOMES` 与 `NOISE`/`SURFACE`/`CARVERS`）在启动时建好的 `rayon` 全局池（`bbb-worldgen-N`）上并行计算，只读冻结程序与自有输入；`FEATURES` 按 3×3 邻域与结构占用范围分层为互不重叠的波次在 `rayon` 池上并行计算后按确定性顺序合并回世界；`INITIALIZE_LIGHT` 光源扫描、`LIGHT` 传播、`SPAWN`、`FULL` 与所有 `ChunkStore` 写入只在世界线程提交，详见[服务端区块与世界生成](../mechanisms/world/server-chunks-and-worldgen.md)。
tick 按全局序号串行应用命令，每 tick 发布在线玩家状态（遵守 `allows_listing` 匿名规则）、准入快照与同步注册表快照；存档或配置的 `level-type`/数据包维度按真实世界运行，仅未传 `--world` 时使用内置平坦临时世界。全部 69 种已登记的 26.1 服务端入站游玩数据包均已解码与分派，仅未登记/未知数据包 ID 触发协议错误并关闭连接。运行中的维度使用 `LevelId(u32)` 与竞技场表，资源键只在输入、协议和存档边界解析或投影；自定义维度保持既有数量支持，不添加 256 个的上限。资源名不可变并由关卡与索引共享，重载保持 ID，跨维度转移直接借用两个关卡。

### 分派表与强类型世界生成

`bbb-server-core` 里按资源 ID 分派的所有者族只保留一张表作为 ID 到所有者的唯一事实源（`block_entity::block_entity_payloads!`、`world::block_use::block_use_dispatch!`）：表同时展开分派函数与 ID 常量，Rust 测试断言表键集合包含于生成的 Vanilla 注册表。新增所有者 = 新建 `foo/<owner>.rs` 放所有者函数与其行为测试 + 门面表里一行（外加 `mod` 行）；不复制分派分支、`ensureCanWrite` 前置、账本常量或逐 ID 样板测试，也不改门面的任何分支。这也是多工作分支并行的文件边界：各分支只写自己的所有者文件，共享门面只接受表行/`mod` 行追加。

世界生成的密度函数/噪声/设置/地表规则、已配置/已放置地物、已配置雕刻器、放置修饰器、值提供器、规则测试、结构处理器与树放置器在数据包加载时一次解码为 `bbb_server_datapack::worldgen` 强类型，非法内容在加载时拒绝；`bbb-server-worldgen` 只读这些类型，地物放置是对 `ConfiguredFeature` 枚举的穷尽 `match`，不经资源 ID 表或 NBT 归一化层。这些强类型注册表保留原始资源字节与解码值，不保留多余的 `serde_json::Value` 投影；其它仍有 JSON 读取者的注册表按实际需要保留投影。`IntProvider`、`FloatProvider` 的值类型、字段解码和范围规则由 `bbb-registry-codec` 单一拥有；世界生成直接消费该值，注册表发布只在输出时投影为 NBT，不经 NBT 反解回运行时值。附魔从同代保留编解码器读取浮点提供器并委托相同解码器；玩法与世界生成各自保留采样路径。世界生成的随机流只来自 `bbb-server-worldgen::random`（种子哈希派生）；`bbb-registry-codec` 的 Java LCG/xoroshiro 只服务客户端、玩法与随机序列。

## 优先复用客户端已有工作

复用以语义中立为前提，而不是按文件数量追求共享：

- 直接复用 `bbb-protocol` 的数据包目录、编解码原语、文本组件/NBT 与资源定位符，`bbb-registry-codec` 的注册表 holder/持久化编解码器，以及 `bbb-facts` 的物品/数据组件、方块/实体/粒子等 Vanilla 生成事实。新增服务端入站解码/出站编码必须落在同一事实源。
- `bbb-client-net` 保持客户端套接字/阶段/事件所有者；服务端只复用底层帧/编解码中真正中立的部分，不能把客户端命令/事件枚举改名后当服务端监听器。
- `bbb-client-world` 是可见客户端副本、本地预测和呈现状态，不能成为服务端世界。可以下沉无状态数学、碰撞形状或生成目录，但必须先证明双向消费者。
- `bbb-client-assets` 是客户端资源包代次；服务端数据包有不同格式、重载消费者与持久化生命周期，不能反向依赖渲染器或客户端资源包目录。
- BBB 客户端的可玩测试夹具、命令编码器、世界查询和控制观察优先作为服务端集成测试驱动。同一回放对官方离线 26.1 服务端的归一化比较只用于调试定位。

平坦世界回放的身份、预测与发包验证见[服务端连接与会话](../mechanisms/protocol/server-connections-and-sessions.md)。

## 核心机制与交接合同

| 机制 | 规范页面 | 边界 |
| --- | --- | --- |
| tick、光照屏障与回调阶段 | [服务端 tick 与世界](../mechanisms/world/server-tick-and-world.md) | 唯一写者；维度、序号、阶段与生命周期顺序 |
| 红石与活塞 | [服务端红石](../mechanisms/blocks/server-redstone-and-pistons.md) | 信号、邻更、计划刻与方块事件分属各所有者；活塞方块移动与实体/玩家推动均已对齐 |
| 保存、恢复与数据包重载 | [服务端保存与重载](../mechanisms/persistence/server-save-and-reload.md) | tick 快照与候选代次分离；单文件临时写入并 `fsync` 原子重命名（`atomic_file::StagedFile`），`.mca` 原地更新扇区（`.mcc` 走暂存重命名），不提供跨 region 回滚 |
| MCP、权限范围、超时与操作结果 | [服务端 MCP 管理](../mechanisms/control/server-mcp-administration.md) | 普通查询和变更都经邮箱；操作结果返回不代表磁盘刷盘完成 |
| 连接、配置与游玩交接 | [服务端连接](../mechanisms/protocol/server-connections-and-sessions.md) | 网络阶段与世界准入分离；压缩在准确数据包边沿生效 |
| 区块与世界生成 | [服务端区块](../mechanisms/world/server-chunks-and-worldgen.md) | 票据、生成状态、光照与 `FULL` 发布分开 |
| 方块、流体与机器 | [服务端方块](../mechanisms/blocks/server-block-fluid-and-block-entities.md) | `setBlock`、邻更与载荷/定时驱动各有明确所有者 |
| 实体与 AI | [服务端实体](../mechanisms/entities/server-entities-and-ai.md) | UUID、网络 ID、追踪基线与运行瞬态分开 |
| 玩家、背包与战斗 | [服务端玩家](../mechanisms/player/server-player-inventory-and-combat.md) | 权威槽位与客户端预测分开，回调遵守世界命令序 |
| 数据包与战利品 | [服务端规则资源](../mechanisms/resources/server-datapack-and-loot.md) | 加载、引用和玩法执行分别验收 |
| 命令、成就与计分板 | [服务端命令](../mechanisms/control/server-commands-and-progression.md) | 命令直接写世界、部分失败不回滚，同调用树与奖励共享预算 |

网络、系统信号、MCP 和资源准备者提交自有请求，不借用可变世界。在线玩家列表提供窄投影发布；服务端 MCP 普通资源在 tick 内读取 `AdminWorldView`，操作 URI 读取传输层的结果记录。HTTP 等待超时与世界 tick 截止时间是两个独立边界。

MCP 是唯一远程管理协议；`bbb-admin` 与代理工具共用端点、schema 与授权，内部 `AdminGateway` 是强类型接口。`command.execute` 委托唯一命令分派器。`SIGTERM`/`SIGINT` 交给优雅保存与停服所有者；`systemd`/`launchd` 负责进程启动、崩溃重启和强制终止。stdout/stderr 输出结构化日志，stdin 不读取命令，不启动 RCON/Query。

## 验收层级

1. **聚焦测试**：数据包编解码、连接阶段、安全预算、数据包加载、管理权限范围与玩家可见规则结果。
2. **集成测试**：BBB 客户端 ↔ BBB 服务端的确定性平坦世界夹具与双客户端同步。
3. **持久化测试**：读取官方 26.1 存档不丢合法数据，以及 BBB 自身保存/重启/重载往返一致。
4. **管理面测试**：MCP 协议一致性、鉴权与权限范围、重试与幂等、审计日志与 CLI 客户端。
5. **发布级测试**：20 客户端并发、保存/重启/重载、畸形输入、RSS/FD/线程/tick 长跑稳定性，以及 Linux/macOS 信号与守护进程无头合同。

官方差分与世界对照工具只作调试用途。

未关闭的功能缺口与范围裁决统一维护于[`docs/unsupported-server-features.md`](../../unsupported-server-features.md)；推进原则见[`AGENTS.md`](../../../AGENTS.md)，开发与提交流程只取[开发流程](../development/testing-and-gates.md)。

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

- **所有权与共享算法**：方块、区块、实体与世界的玩法模块由 `bbb-server-core` 统一拥有（`chunk`、`entity` 各自持有实际存储与执行）；纯区块与计划刻容器归 `bbb-server-types`，纯光照传播核心（`chunk_light`）与生物体转向（`entity_body_rotation`）归 `bbb-protocol`，内置方块声音事实归 `bbb-facts::block_sound_facts_26_1`。纯类型消费者直接依赖 `bbb-server-types`，核心层不重导出错误、存档、物品、几何、方向或计划刻类型。
- **实时与生成借用上下文**：只读方块和亮度访问统一使用 `BlockGetter`；形状更新的 `BlockShapeCtx` 仅承载实时世界、生成区域与存档升级所需的读取、标签、随机数与计划刻，不携带库存、事件或空写回调。实时玩法借用平铺的 `world::simulation_level::LevelCtx`（由 `LevelCtx::new` 唯一构造，子调用重借同一存储）；运行时改方块只经 `LevelCtx::set_block`（区段原始写入只经 `write_block`），未达 `FULL` 的写入目标由 `chunk_lifecycle/full_preparation` 同步准备。
- **`WorldGenLevel` 双宿主**：仅存在生成区域 `FeatureWorldGenRegion` 与实时 `LevelCtx` 两个宿主。`/place`、末地平台/折跃门、结构方块加载、幽匿催发体与附魔圆盘谓词均在单个 `LevelCtx` 帧内执行，地物随机数从关卡随机数派生，新生成实体在帧结束后装入 `EntityStore`。

### 实体、AI、随机数与刷怪所有权

- **借用与随机流**：实体移动、伤害、AI 与接触直接调用 `LevelCtx`；tick 期间临时借出的实体主体通过借用链供来源与乘客查询，不引入第二套世界宿主或冻结事务。玩法随机流唯一归口 `crate::random::GameRandom`（wyrand，由 `ServerWorld.random` 拥有），各关卡、`EntityStore` 与 AI 持句柄借用，新实体 UUID 取 `Uuid::new_v4`；仅非零种子战利品表与结构模板按种子新建流。
- **统一实体槽位与命令队列**：`EntityStore` 每个实体 ID 对应单个槽位（同槽保存实体主体、AI 运行时与乘骑关系）。在线玩家以主体实体 ID 入槽（通用实体查询与实体区块存档跳过玩家），连接级发送/追踪视图留在按 `ConnectionId` 索引的 `PlayerSession`，`LevelArena` 维护 `ConnectionId → (关卡, id)`。需要玩家、地图、命令或整关卡的实体回调排入 `EntityCommand`，由 `run_entity_commands` 按序执行（后续命令先于旧命令）。
- **生物变种与刷怪**：`GeneralLivingData` 持有 Mob/AgeableMob/Animal 基类、内部物品栏、大脑与跨族变种，其余按类型收进 `MobVariantState`（存档、同步元数据与命令重载统一分派）。流体/接触/踩踏/落地发声规则返回决策值供实体与玩家各自应用；大脑 AI 仅读 `BrainBodyFacts` 与真实寻路采样。刷怪谓词统一读 `SpawnPredicateLevel`，生物终结经 `finalize_spawn` 与 `SpawnLevel`（世界侧仅 `MinecartSpawnerRuntime` 与 `NaturalSpawnLevel` 两个实现）。

### 持久化、事件、战利品与命令边界

- **结构、事件与配方簿**：结构方块捕获直接读世界层或 `LevelCtx` 存储；方块事件由 `LevelCtx` 执行，世界层负责声音与事件广播；游戏事件区段仅保存监听数据，位置解析、范围筛选与投递由世界层执行。持久化层只读写配方簿列表，运行时配方簿由核心层背包模块构造。
- **战利品与玩家快照**：战利品抽样直接读关卡随机流，不按表名维护命名序列；`random_sequences.dat` 仍由世界层完整读写，专供 `/random` 命令。战利品上下文直接借用 `StoredLootEntity` 或传值 `LootEntityFacts`，不经视图 trait 回调。玩家保存快照（`world/player_save`）接收真实 `RootVehicle` 与警告追踪器值，供保存、命令与战利品共享；保存与恢复分别归 `world/save` 与 `world/restore`。
- **区块恢复、放置与命令**：存档区块恢复归 `ChunkStore`，世界层负责激活后票据、实体生命周期与 `ChunkStore::register_ticks`。方块形状增长抬升实体、火焰环境（`FireFacts`）、物品放置候选与刷怪应用均在世界层直接借用真实存储与会话，不经反向宿主 trait。命令经 `CommandCx` 直接写 `ServerWorld`，多目标部分失败不回滚。内部辅助仅在 `cfg(test)` 编译，数据 crate 不设测试专用 feature；`bbb-server` 与 `bbb-server-admin` 直接消费核心层 API，`bbb-admin` 仅经 `bbb-mcp` 交互。

### 线程模型、网络路由与背压

- **世界线程与非阻塞网络路由**：`ServerWorld` 仅在 `bbb-world` 操作系统线程按 `MinecraftServer.runServer` 节拍（含追赶与过载跳过）运行 tick；`tokio` 负责监听器、连接、MCP 与系统信号。套接字/管理/生命周期生产者共享全局序号并各自持有条数与字节配额。出站经非阻塞 `AsyncRouteDispatcher` 投递至单连接有界队列（广播包与跨观察者区块包分别经 `SharedWorldPacket` / `SharedChunkPacket` 单次预编码压缩，写循环合并刷写，超额慢客户端保序关闭），配置期支持 `SelectKnownPacks`（`minecraft:core` `26.1`），游玩期支持 `StartConfiguration` → `ConfigurationAcknowledged` 重配置屏障与 `PlayerCombatEnter`/`PlayerCombatEnd`（详见[服务端连接与会话](../mechanisms/protocol/server-connections-and-sessions.md)）。
- **异步存档与并行世界生成**：存档读写分离到 `bbb-save-read` 与 `bbb-save-write` 线程（通过 `RegionFileCache` 读写锁协同）：tick 边界冻结元数据与 `DeferredChunkWrite`，后台线程物化 NBT 并经 `rayon` 并行压缩写盘；自动保存与批量卸载按代次异步确认，仅管理保存与关闭刷盘同步排空。纯世界生成阶段（`STRUCTURE_STARTS`、`BIOMES`、`NOISE`/`SURFACE`/`CARVERS`）在启动时建好的 `rayon` 池（`bbb-worldgen-N`）并行计算，`FEATURES` 按 3×3 邻域与结构范围分层波次并行后确定性合并，光照与 `FULL` 发布仅在世界线程提交（详见[服务端区块与世界生成](../mechanisms/world/server-chunks-and-worldgen.md)）。
- **维度与游玩包边界**：tick 按全局序号串行应用命令，每 tick 发布在线玩家状态（遵守 `allows_listing`）、准入快照与同步注册表快照；未传 `--world` 时使用内置平坦临时世界。69 种已登记的 26.1 服务端入站游玩数据包均已解码与分派，未知包 ID 触发协议错误并断连。运行维度使用 `LevelId(u32)` 与竞技场表（不设 256 维上限），资源名不可变且重载保持 ID，跨维度转移直接借用双关卡。

### 分派表与强类型世界生成

- **单一分派表宏**：`bbb-server-core` 按资源 ID 分派的所有者族只保留一张表作为唯一事实源（`block_entity::block_entity_payloads!`、`world::block_use::block_use_dispatch!`），同时展开分派函数与 ID 常量并由测试校验包含于 Vanilla 注册表。新增所有者 = 新建 `foo/<owner>.rs` + 表中追加一行（及 `mod` 行），兼顾多工作线并行边界。
- **强类型世界生成解码**：密度函数、噪声、地表规则、已配置/已放置地物、雕刻器、放置修饰器、值/浮点提供器、规则测试、结构处理器与树放置器在数据包加载时一次解码为 `bbb_server_datapack::worldgen` 强类型，非法内容加载期拒绝；`bbb-server-worldgen` 通过穷尽 `match` 直接消费，不留多余 `serde_json::Value` 投影。`IntProvider`/`FloatProvider` 由 `bbb-registry-codec` 单一拥有；世界生成随机流仅来自 `bbb-server-worldgen::random`，`bbb-registry-codec` 的 Java LCG/xoroshiro 只服务客户端、玩法与随机序列。

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

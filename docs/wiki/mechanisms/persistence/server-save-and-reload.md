# 服务端保存与资源重载

机制 ID：`server-save-and-reload`

## 机制目的和边界

本页定义 dedicated server 读取官方 26.1 存档、新建世界预设、后台增量保存与 dirty epoch 确认、按需 region 读取，以及运行时 datapack 资源重载的完整契约。
区块生成细节归[服务端区块与世界生成](../world/server-chunks-and-worldgen.md)，数据包内容解码与战利品求值归[服务端 datapack 与战利品](../resources/server-datapack-and-loot.md)。

## Vanilla 26.1 权威实现

- `LevelStorageSource.java:586-598`：使用同目录临时文件写出并通过原子替换更新 `level.dat` 等单文件元数据。
- `ChunkMap.java:577-603`：区块读取/解析失败时记录错误并对 26.1 同版本坏列回退重生成；非当前 `DataVersion` 不静默覆盖。

## 决策输入与状态所有权

| 输入 | owner | 下游 |
| --- | --- | --- |
| `level.dat`、SavedData、playerdata、region（chunk/entity/POI）与 `session.lock` | `bbb-server-persistence` | 强类型存档读写、压缩预算与单文件原子替换 |
| 新世界预设（`--level-type`、`--level-seed`、`--generate-structures`） | `bbb-server-core::world_creation` + `bbb-server-datapack` | 初始化 `level.dat` 与 Vanilla/平坦/调试预设维度 |
| 冷启动恢复、维度源绑定与按需列激活 | `world::restore`、`world::level_sources` | 运行时 `ServerWorld` 与生成源 |
| Tick 边界增量快照与 dirty epoch 对账 | `world::save`、`world::player_save` + `bbb-server` `save_io` 读写线程 | 非阻塞分摊 autosave/unload 与停机保存屏障 |
| Datapack 候选构建与热替换（保持启动冻结注册表） | `bbb-server-datapack` + `world::datapack_install` | 配方、标签、战利品、进度、函数与环境/气候缓存刷新 |

`bbb-server-persistence` 与 `bbb-server-worldgen`、`bbb-server-datapack` 及玩法 crate 完全解耦（不保留原始 chunk NBT AST）：世界生成配置、随机序列状态对、玩家经验/统计/进度与配方簿 ID 列表通过 `bbb-server-types` 或纯 schema 传递，在 `bbb-server-core` 恢复边界转换为运行时结构。

## 判断与处理顺序

1. **世界创建与打开**：空 `--world` 按 `--level-type`（`normal`/`flat`/`large_biomes`/`amplified`）与种子规则写出 `initialized=0` 存档，首次恢复执行 `setInitialSpawn` 搜索出生点；打开已有存档获取 `session.lock`，仅预读 `level.dat`、SavedData、玩家文件与 region 头。
2. **按需加载与版本门控**：区块、实体与 POI 在票据或查询首次触及时由专用 `bbb-save-read` 线程经共享 `RegionFileCache` 与 `region_io` 读锁读取，不再阻塞于 `bbb-save-write` 写队列。遇到 `DataVersion` 非 26.1 的记录时绝不重生成或覆盖，先正常保存已加载脏数据再报错退出，提示使用官方 26.1 服务端 `--forceUpgrade` 升级。
3. **实体与玩家跨树恢复**：生物内部库存、装备与未知合法 NBT 字段在 BBB round-trip 中保留；在线玩家的飞行末影珍珠树只随玩家数据写入，恢复官方存档时将同 UUID 世界珍珠归并至待加入玩家记录，重连后恢复实体树与临时票据。
4. **分摊 Autosave、异步 Unload 与 Dirty 确认**：未指定 flush 的 autosave 在 tick 边界按每 tick 列数上限冻结轻量延迟区块快照（`DeferredChunkWrite`），再由 `bbb-save-write` 离世界线程物化整列区块 NBT 文档（`materialize_deferred_chunks`）、通过 `rayon` 并行压缩并异步落盘；`ChunkUnload` 批量卸载同样以 epoch 跟踪异步完成，不阻塞于 `settle_reads`（跳过存在在途读取的列，仅 `Management` 与 `Shutdown` 保存全量排空在途读取）。
   所有区块写出统一走整列单路径（携带保留的 `blending_data`、`UpgradeData`、`below_zero_retrogen` 与未建模方块实体记录，不做 `.mca` 读-改-写补丁且不在内存保留生成前像列）；落盘成功后按 epoch 与当前内容核销 dirty 标记，保留冻结期间产生的新写入留待下次保存。
5. **资源重载**：`/reload` 或管理请求在后台基于启动冻结的动态注册表（`registries`、`trades` 与 `registry_scope` 保持不变，不重读磁盘动态注册表 JSON）构建新 `bbb-server-datapack` 候选，校验通过后原子替换世界内配方、标签位图、战利品表、进度、函数与群系气候/环境缓存，并同步在线客户端；候选失败保留当前生效资源。

## 生命周期和 fallback

单文件（`level.dat`、playerdata、SavedData）使用同目录临时文件 + rename 原子替换；region 文件原位扇区更新，不承诺跨文件或跨 region 事务原子性。
保存失败保留未确认的 dirty 状态供后续重试；26.1 版本内的单列损坏记录错误后回退重生成（坏 POI 仅丢弃该列 POI，坏实体记录跳过且不覆盖原盘数据）。

## BBB 实现映射

| 职责 | 生产路径 | 边界说明 |
| --- | --- | --- |
| 磁盘格式、region 缓存与原子写 | `crates/bbb-server-persistence` | `session.lock` 单进程所有权、O(1) LRU `.mca` 句柄缓存、并行压缩、`0x80` 外部 `.mcc` 位门控、NBT 预算与 `atomic_file` |
| 世界创建、恢复与增量保存投影 | `crates/bbb-server-core` | `world_creation`、`restore`、`save`、`player_save`、`datapack_install` |
| 读写分离 IO 线程与停服屏障 | `crates/bbb-server` | `bbb-save-read` / `bbb-save-write` 调度、epoch 回执与异常紧急保存 |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 单文件临时写出与安全替换 | `LevelStorageSource.java:586-598` | `bbb-server-persistence::atomic_file` | `aligned` | 单文件保存与异常恢复测试 |
| 官方 26.1 存档读取与 BBB 自身 round-trip | 26.1 Anvil / NBT / SavedData schema | `bbb-server-persistence` + `bbb-server-core` | `aligned` | 官方存档夹具读取与重启往返测试 |
| 坏列隔离与旧版本拒绝覆盖 | `ChunkMap.java:577-603` / DataFixer 边界 | `restore` / `ChunkStore` | `aligned` | 坏列恢复与非 26.1 `DataVersion` 保护测试 |
| 旧版存档内置 DataFixerUpper 自动升级 | Vanilla `DataFixers` | 拒绝覆盖并提示官方 `--forceUpgrade` | `not-needed` | 见服务端 ledger 与 `AGENTS.md` |

## 结构性不变量

- 合法官方 26.1 存档与 datapack 数据（含容器内合法物品组件与未建模透传字段）不得在读取、修改与 BBB round-trip 中静默丢失。
- 旧批次保存回执绝不能清除该批次冻结之后新产生的世界写入。
- 所有外部 NBT、压缩流与包路径均受大小、嵌套深度、条目数与路径逃逸预算约束。

## 聚焦测试（focused tests）和真实证据

- `cargo dev test -p bbb-server-persistence`：覆盖 region 读写、单文件原子替换、压缩/NBT 安全预算与 `session.lock`。
- `cargo dev test -p bbb-server-core save` 与 `cargo dev test -p bbb-server-core restore`：覆盖区块/实体/玩家/末影珍珠往返、dirty epoch 确认与非 26.1 版本保护。

## 当前差异

BBB 写出的存档仅承诺 BBB 自身完整 round-trip，不要求官方服务端反向读取；缺少 UUID 的损坏实体直接丢弃，NBT 字符串中的孤立 UTF-16 代理归一化为 `U+FFFD`。未关闭的存档读取缺口见[服务端 ledger](../../../unsupported-server-features.md)。

## 账本归属

存档与重载缺口归属[服务端 ledger](../../../unsupported-server-features.md)，存档原则见 [AGENTS.md](../../../../AGENTS.md)。

## 精确重开条件

当 Minecraft 26.1 存档 `DataVersion` 或区块/玩家/实体 schema 更新、发现官方 26.1 合法存档字段在 round-trip 中丢失，或 dirty 确认出现漏存/重复写放大时重验本页。

## 相关页面和源码索引

- [服务端区块与世界生成](../world/server-chunks-and-worldgen.md)：区块聚合体、升级数据与按需加载激活。
- [服务端 datapack 与战利品](../resources/server-datapack-and-loot.md)：数据包加载、冻结与重载消费者。
- [服务端玩家、背包与战斗](../player/server-player-inventory-and-combat.md)：玩家库存、重生点与统计状态。

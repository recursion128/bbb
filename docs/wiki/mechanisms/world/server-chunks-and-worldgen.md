# 服务端区块与世界生成

机制 ID：`server-chunks-and-worldgen`

## 机制目的和边界

本页定义区块数据结构、票据（ticket）驱动的加载/模拟生命周期、并行纯阶段 + 确定性串行提交的世界生成管线，以及冷区块恢复与升级规则。
磁盘 region 读写格式归[服务端保存与资源重载](../persistence/server-save-and-reload.md)，自然刷怪循环与实体生命周期归[服务端实体、生成与 AI](../entities/server-entities-and-ai.md)。

## Vanilla 26.1 权威实现

- `ChunkPyramid.java:9-54`：区分全新生成路径（执行 `FEATURES` / `SPAWN`）与已保存区块加载路径（跳过重复特征放置与初始刷怪，保留独立 `LIGHT` 与 `FULL` 入口）。
- `ChunkMap.java:580-603`：卸载列重读或恢复失败时记录错误后丢弃并回退重新生成，不阻塞同维度其余区块。
- `ChunkHolder` / `DistanceManager`：区分区块存在、达到 `FULL`、光照就绪与模拟距离票据等级，仅在 `FULL` 完成后纳入客户端发送批次。

## 决策输入与状态所有权

| 输入 | owner | 下游 |
| --- | --- | --- |
| Section 方块/群系调色板、heightmap、光照、方块实体记录、计划刻容器 | `bbb-server-types::chunk_data`、`chunk_ticks`、`bbb-protocol::chunk_light` | `ChunkStore` 与持久化直接消费 |
| 噪声、密度函数后序程序、群系源、carver、feature、结构算法与 wyrand 随机源 | `bbb-server-worldgen` | 纯生成计算与区域修饰 |
| Datapack 强类型 worldgen 定义与 `BiomeClimate` 表 | `bbb-server-datapack::worldgen` | 启动/重载一次解码，生成与天气共享只读表 |
| `Chunk` 聚合体存储、高度图增量维护、光照传播与 `UpgradeData` | `bbb-server-core::chunk::ChunkStore` | 单维度权威区块库 |
| 票据距离图、后台生成批次队列与 `FULL` 激活发送 | `bbb-server-core::world::chunk_lifecycle` | 区块模拟准入与客户端地形发送 |

每个已加载区块在 `FxHashMap<(i32, i32), Chunk>` 中聚合方块列、生成状态、blending 见证、高度图、原型实体、计划刻、方块实体与编辑补丁；模拟距离图缓存上次票据源（存档票据、玩家、飞行末影珍珠、末影龙战斗），仅在源集合变化或票据到期时重建。

## 判断与处理顺序

1. **票据刷新与批次规划**：`generation_queue` 汇总当次新增的目标区块列，先按确定性顺序恢复关联存档列；含旧版 `blending_data` 邻居或 retrogen 起点的列留归串行路径。
2. **并行纯阶段（`parallel_generation`）**：在 `rayon` 线程池并行计算缺失的 `STRUCTURE_STARTS`、`BIOMES` 以及新生成/续算至 `CARVERS` 的地形雕刻；`ChunkStore::initialize_light_batch`（`chunk/lighting.rs`）则在世界线程串行执行，避免世界 tick 等待排队中的地形生成任务。每条随机流种子由世界种子与 `(chunk_pos, step/index, block_pos, name)` 经 wyrand 派生，与线程调度顺序无关。
3. **主线程有序提交与波次并行装饰**：`background_generation` 开启时，world tick 仅按归档顺序提交已完成的 `6×6` tile 批次（`world/chunk_lifecycle/generation_queue.rs` 中 `BATCH_TILE = 6`；启动恢复、出生点搜索、玩家所在列与同步 `set_block` 则同步等待所需窗口及 `BATCH_REACH = 8` 区块内已归档批次）。提交阶段按批次合并收集前置 `INITIALIZE_LIGHT` 依赖与整块 `FULL` 目标、在世界线程串行执行 `initialize_light_batch` 并冲刷待重算实时光照；在 `features_orchestration::parallel_waves` 中按每个 `FEATURES` 目标的 `3×3` 邻域与引用 `StructureStart` 源区块足迹做 DAG 分层，互不重叠的波次在 `rayon` 线程池并行执行并按规范顺序确定性归并区块、高度图、方块实体、POI 与光照变更，整批统一完成 `LIGHT` 传播、`SPAWN`（仅新生成列）与 `FULL` 晋升。
4. **激活与下发**：晋升 `FULL` 后统一发布原型实体、登记保存计划刻、应用 `UpgradeData` 邻接修正，并在 `FULL` 就绪后进入客户端发送队列（首次登录仅同步准备出生点所在列，其余视距地形随后续 tick 流式下发）。

## 生命周期和 fallback

恢复已保存的 `FULL` 区块走加载分支，绝不重复运行 `FEATURES` 或初始生物生成；带 `blending_data` 的旧世代区块常驻保留混合见证。
运行时重读损坏列按 `ChunkMap.java:580-603` 记录错误并回退重新生成（非 26.1 `DataVersion` 则交由保存/恢复层安全拒绝并提示 `--forceUpgrade`）。生成中途失败保留已写入区域，不复制整库或回滚邻列。

## BBB 实现映射

| 职责 | 生产路径 | 边界说明 |
| --- | --- | --- |
| 区块容器与计划刻 | `crates/bbb-server-types` | `chunk_data`、`chunk_ticks` 零核心层依赖类型 |
| 地形/群系/特征/结构生成 | `crates/bbb-server-worldgen` | 密度图 VM、噪声、carver、feature、结构与确定性随机源 |
| Worldgen 数据包解码 | `crates/bbb-server-datapack` | 密度函数、noise settings、surface 规则与群系气候解码 |
| 区块存储与生命周期编排 | `crates/bbb-server-core` | `chunk`、`world::chunk_lifecycle` |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 生成与加载阶段分离 | `ChunkPyramid.java:9-54` | `bbb-server-core::chunk` / `world::chunk_lifecycle::full_preparation` | `aligned` | 生成与重加载实体/特征不重复测试 |
| Datapack 驱动密度图、群系、结构与地物 | `NoiseBasedChunkGenerator` / `BiomeSource` / `Structure` | `bbb-server-datapack` + `bbb-server-worldgen` | `aligned` | 主世界/下界/末地生成与同种子确定性测试 |
| 坏列恢复回退重生成 | `ChunkMap.java:580-603` | `bbb-server-core::chunk::saved_column_restore` / `world::restore` | `aligned` | 损坏区块恢复测试 |
| 同种子与官方逐方块完全一致 | Vanilla `LegacyRandomSource` / `XoroshiroRandomSource` 调用序列 | `bbb-server-worldgen::random`（wyrand 位置哈希） | `not-needed` | [`AGENTS.md`](../../../../AGENTS.md) 明确仅要求同一 BBB 种子自身确定 |

## 结构性不变量

- `bbb-server-worldgen` 生产代码不反向依赖 `bbb-server-core`。
- 同一 BBB 世界种子与同一 datapack 输入生成的地形、群系、结构与装饰完全确定，不受后台 rayon 线程快慢或票据批次切分时序影响。
- 官方存档中已生成的区块原样保留合法数据，交界处允许与新生成区块存在接缝。

## 聚焦测试（focused tests）和真实证据

- `cargo dev test -p bbb-server-worldgen`：验证密度函数编译、群系分布、carver、结构与同种子确定性。
- `cargo dev test -p bbb-server-core chunk`：验证并行生成提交、光照传播、票据生命周期与存档恢复。

## 当前差异

BBB 使用位置哈希派生的 wyrand 流代替 Vanilla 调用序敏感的 RNG 流，因此不追求与官方同种子逐方块一致；当前服务端差异账本的开放项与范围裁决统一以[服务端差异账本](../../../unsupported-server-features.md)为准，若后续发现世界生成或区块生命周期未关闭缺口则登记于该账本。

## 账本归属

世界生成、结构与区块生命周期的范围裁决及后续发现的未关闭缺口统一归属[服务端差异账本](../../../unsupported-server-features.md)，跨 crate 依赖方向见[服务端边界](../../architecture/server-boundaries.md)，推进原则见 [AGENTS.md](../../../../AGENTS.md)。

## 精确重开条件

当 26.1 worldgen 注册表 schema 变更、新增结构/地物类型、或并行阶段触及非纯共享状态导致同种子确定性被破坏时重验本页。

## 相关页面和源码索引

- [服务端 tick 与权威世界](server-tick-and-world.md)：tick 内批次提交与关卡借用。
- [服务端保存与资源重载](../persistence/server-save-and-reload.md)：region 按需读取与 dirty 补丁确认。
- [服务端实体、生成与 AI](../entities/server-entities-and-ai.md)：初始刷怪、自然刷怪与结构刷怪覆盖。

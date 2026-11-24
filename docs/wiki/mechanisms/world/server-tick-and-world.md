# 服务端 tick 与权威世界

机制 ID：`server-tick-and-world`

## 机制目的和边界

本页定义 dedicated server 的单写者世界线程模型、game tick 推进契约、跨维度实体/玩家迁移以及传送门目的地规则。
区块生成细节见[服务端区块与世界生成](server-chunks-and-worldgen.md)，方块/流体行为见[服务端方块、流体与方块实体](../blocks/server-block-fluid-and-block-entities.md)，实体 AI 见[服务端实体、生成与 AI](../entities/server-entities-and-ai.md)，落盘确认见[服务端保存与资源重载](../persistence/server-save-and-reload.md)。

## Vanilla 26.1 权威实现

- `MinecraftServer.java:975-986`：推进 `ServerTickRateManager`、各维度关卡 tick、连接冲洗及周期性自动保存。
- `ServerLevel.java:340-355`：在正常 tick 运行态下推进天气、世界边界、睡眠唤醒、计划刻、随机刻、方块事件与实体模拟。

## 决策输入与状态所有权

| 输入 | owner | 下游 |
| --- | --- | --- |
| 网络输入、管理请求、IO 完成与停止信号 | `bbb-server::runtime` 有界 mailbox | `ServerWorld::tick` 顺序消费 |
| 权威世界状态、关卡 arena（`LevelId(u32)`）与在线玩家 | `bbb-server-core::world::ServerWorld` | 唯一状态写者 |
| 单关卡模拟上下文（区块、规则、计划刻、邻更、事件） | `bbb-server-core::world::simulation_level::LevelCtx` | 方块、流体、结构与实体借用 |
| 世界时钟、freeze/step/tick-rate | 世界调度与时钟 owner | 各维度时间线、环境属性与 `time_check` |
| 传送门搜索/建门/票据与跨维度子树迁移 | `nether_portal`、`entity_transfer` | 玩家与载具/乘客跨关卡传送 |

关卡通过 arena 与数值 `LevelId(u32)` 定位，资源键仅在命令、协议与存档边界转换；资源重载不重建已有 `LevelId`。

## 判断与处理顺序

1. **输入吸纳与解码**：world 线程从有界 mailbox 取入站消息，逐条解码 `PlayServerbound` 与管理请求并按序应用；延迟加入或排队命令在执行或丢弃前持续占用字节预算。
2. **调度门控**：根据 tick rate、freeze 与 step 状态决定是否推进模拟；freeze 冻结世界推进而非停滞网络帧。
3. **关卡推进（`simulation_tick`）**：依次推进世界时钟、天气、世界边界、睡眠结算、区块票据与后台生成批次提交、到期计划刻、随机刻、方块事件、方块实体与存储实体模拟。
4. **玩家推进与同步（`player_tick`）**：推进玩家环境接触、饥饿、自然恢复、伤害、地图扫描、区块发送与实体 tracking 差量广播。
5. **跨维度迁移**：下界门搜索、建门与票据由 `nether_portal` 统一拥有；末地门进末地生成黑曜石平台，末地出口对未看过字幕的玩家先发 `WIN_GAME` 等待 `PERFORM_RESPAWN`（拒绝载具直接载离未看字幕玩家），并随 playerdata 持久化 `seenCredits`。载具与乘客树经 `ServerWorld::teleport_entity_subtree` 整体迁移并同步座位关系。
6. **生产输出**：`TickResult` 在生产路径仅输出遵守 `allows_listing` 匿名化规则的在线玩家摘要，不克隆全量世界或背包快照。

## 生命周期和 fallback

世界状态在调用时原地更新，不维护全量世界事务回滚；多步操作或区块生成中途报错时保留已完成写入，后续 tick 从实际状态继续。
自动保存请求在 tick 边界收集冻结快照交由 IO 线程写入，仅在落盘成功后按 epoch 确认对应 dirty 标志。

## BBB 实现映射

| 职责 | 生产路径 | 边界说明 |
| --- | --- | --- |
| 运行时循环与 mailbox | `crates/bbb-server` | tick 计时、网络/管理/保存线程协调 |
| 权威世界与关卡模拟 | `crates/bbb-server-core` | `ServerWorld::tick`、`simulation_tick`、`LevelCtx`、`player_tick` |
| 跨维度与传送门 | `crates/bbb-server-core` | `nether_portal`、`entity_transfer` |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| Tick 速率、freeze 与 step 控制 | `MinecraftServer.java:975-986` / `ServerTickRateManager` | `bbb-server` + `bbb-server-core::world` | `aligned` | `/tick` 命令与世界冻结测试 |
| 关卡天气、时钟、睡眠与计划刻推进 | `ServerLevel.java:340-355` | `simulation_tick` / `LevelCtx` | `aligned` | `bbb-server-core` 关卡模拟测试 |
| 下界/末地传送门与骑乘子树跨维迁移 | `BaseFireBlock` / `EndPortalBlock` / `Entity.teleport` | `nether_portal` / `entity_transfer` | `aligned` | 跨维度传送与乘客迁移测试 |

## 结构性不变量

- `ServerWorld::tick` 是权威世界状态的唯一生产写者；网络、MCP 管理与后台世界生成均不得并发直接写关卡状态。
- `LevelCtx` 平铺借用当前关卡的区块、计划刻、邻更队列与事件，不复制整份关卡快照。
- 协议可观察的同步顺序（如先发维度/位置变更再配对周围地形与实体）严格遵守 26.1 wire 契约。

## 聚焦测试（focused tests）和真实证据

- `cargo dev test -p bbb-server-core portal`：验证下界门建门/搜索、末地平台与通关重生流程。
- `cargo dev test -p bbb-server-core teleport`：验证跨维度玩家与载具乘客子树迁移。
- `cargo dev test -p bbb-server`：验证运行时 mailbox 预算、tick 推进与优雅停机保存屏障。

## 当前差异

tick 内部子阶段细粒度交错、回调重入与 RNG 抽取次序不作为对齐目标；玩家可观察规则、常量与协议同步顺序保持对齐。当前服务端差异账本的开放项与范围裁决统一以[服务端差异账本](../../../unsupported-server-features.md)为准，若后续发现世界调度或维度规则未关闭缺口则登记于该账本。

## 账本归属

世界调度、维度与环境规则的范围裁决及后续发现的未关闭缺口统一归属[服务端差异账本](../../../unsupported-server-features.md)，推进与非目标原则见 [AGENTS.md](../../../../AGENTS.md)。

## 精确重开条件

当 Vanilla 更新 `MinecraftServer`/`ServerLevel` 主循环的玩家可见调度规则、新增维度传送门机制，或跨关卡借用模型调整时重验本页。

## 相关页面和源码索引

- [服务端区块与世界生成](server-chunks-and-worldgen.md)：区块生命周期、票据传播与异步生成提交。
- [服务端方块、流体与方块实体](../blocks/server-block-fluid-and-block-entities.md)：计划刻、随机刻与方块事件消费。
- [服务端实体、生成与 AI](../entities/server-entities-and-ai.md)：实体模拟与 tracking。
- [服务端保存与资源重载](../persistence/server-save-and-reload.md)：autosave 与 dirty epoch 确认。

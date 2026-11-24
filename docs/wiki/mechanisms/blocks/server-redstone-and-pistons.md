# 服务端红石与活塞

机制 ID：`server-redstone-and-pistons`

## 机制目的和边界

本页定义红石信号查询、导线/二极管/侦测器/火把/按钮/绊线状态更新，以及活塞结构解析与移动方块实体推挤契约。
通用方块放置、流体传播与容器方块实体生命周期见[服务端方块、流体与方块实体](server-block-fluid-and-block-entities.md)。

## Vanilla 26.1 权威实现

- `SignalGetter.java:65-82`：聚合导体自身信号与相邻方块的直接充能信号，区分强充能与弱充能。
- `PistonStructureResolver.java:13-35`：解析活塞推动/拉回分支结构、黏性方块连带关系、不可移动方块阻挡以及最多 12 格推动上限。

## 决策输入与状态所有权

| 输入 | owner | 下游 |
| --- | --- | --- |
| 直接信号、间接充能与模拟（比较器）信号查询 | `bbb-server-core::redstone` | 红石元件、门窗、发射器、漏斗与铁轨 |
| 红石线、中继器、比较器、侦测器、红石火把、按钮、压力板、绊线 | `bbb-server-core::block_behavior` 各元件子模块 | 方块 state 写入与计划刻预约 |
| 邻更深度优先栈与形状更新预算 | `bbb-server-core::world::simulation_level::LevelCtx` | 有界传播，防止无限递归爆栈 |
| 活塞结构求解与移动活塞方块实体 | `block_behavior::piston`、`block_entity::piston` | 方块搬迁、破坏掉落与实体/玩家推挤 |

## 判断与处理顺序

1. **信号求值**：元件通过 `redstone` 模块读取方向、相邻强弱信号与真实容器/方块实体的比较器输出，不构造整块世界副本。
2. **邻更与计划刻**：方块写入后沿深度优先栈触发有界邻居更新与形状更新；延时元件（中继器、比较器、火把、侦测器、按钮、绊线）向区块计划刻容器预约到期刻。
3. **实体触发元件**：木按钮按当前碰撞轮廓查询驻留箭矢并在箭离开前续期（石按钮不受箭触发）；绊线对非旁观玩家与存储实体共用细线碰撞检测，占用期间每 10 tick 复查并联动两端绊线钩。
4. **方块事件与活塞推进**：活塞伸缩经 `LevelCtx` 方块事件去重队列在当 tick 执行，调用 `PistonStructureResolver` 校验 12 格上限与黏附分支，生成移动活塞方块实体逐步推进碰撞盒、推挤实体与在线玩家，并在到位后落地最终方块状态与网络广播。

## 生命周期和 fallback

邻更栈与形状更新均受深度上限保护；活塞推动受阻或超出 12 格上限时保持原位且不破坏现有电路。所有写入直接作用于 `LevelCtx` 区块存储，不维护整片电路的事务回滚。

## BBB 实现映射

| 职责 | 生产路径 | 边界说明 |
| --- | --- | --- |
| 信号查询 | `crates/bbb-server-core`（`redstone`） | 直接/间接/比较器信号计算 |
| 红石元件与活塞结构 | `crates/bbb-server-core`（`block_behavior`） | `redstone_wire`、`button`、`tripwire`、`piston` 等 |
| 移动活塞推进 | `crates/bbb-server-core`（`block_entity::piston`） | 移动插值、动态碰撞盒与实体推挤 |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 强/弱充能与比较器信号读取 | `SignalGetter.java:65-82` | `bbb-server-core::redstone` | `aligned` | 红石信号与容器比较器测试 |
| 活塞 12 格上限、黏附分支与实体推挤 | `PistonStructureResolver.java:13-35` | `block_behavior::piston` / `block_entity::piston` | `aligned` | 活塞推动、拉回、黏液块与玩家推挤测试 |
| 按钮箭矢驻留与绊线钩联动 | `ButtonBlock` / `TripWireBlock` | `block_behavior::{button,tripwire}` | `aligned` | 箭触发木按钮与绊线复查测试 |
| 同 tick 微观邻更枚举顺序 | Vanilla `CollectingNeighborUpdater` 内部顺序 | `LevelCtx` 有界邻更栈 | `not-needed` | `AGENTS.md` 以真实红石机器正常工作为准 |

## 结构性不变量

- 信号查询、元件触发与活塞搬迁共享同一个 `LevelCtx` 区块与实体存储，不经过测试替身或双写镜像。
- 移动中的活塞提供动态碰撞形状（`FluidCollisionShape::Dynamic` 与实体碰撞），确保流体阻挡与玩家/实体推挤一致。

## 聚焦测试（focused tests）和真实证据

- `cargo dev test -p bbb-server-core redstone`：覆盖红石线衰减、中继器锁存、比较器模式、火把烧毁与侦测器脉冲。
- `cargo dev test -p bbb-server-core piston`：覆盖 12 格推动上限、黏液/蜂蜜分支、不可推方块与实体推挤。

## 当前差异

BBB 不复刻 Vanilla 内部邻更回调顺序、同 tick 微观更新次序或对象身份；验收标准为红石电路、时钟、脉冲发生器、活塞门与自动农场等机器正常工作且无刷物/吞物。未关闭的红石元件缺口见[服务端 ledger](../../../unsupported-server-features.md)。

## 账本归属

未关闭的红石或活塞元件差异归属[服务端 ledger](../../../unsupported-server-features.md)，红石验收口径见 [AGENTS.md](../../../../AGENTS.md)。

## 精确重开条件

当真实红石机器出现信号死锁、活塞推拉丢块/刷块、客户端状态不同步，或 Vanilla 26.1 调整红石元件可观察规则时重验本页。

## 相关页面和源码索引

- [服务端方块、流体与方块实体](server-block-fluid-and-block-entities.md)：计划刻调度、发射器/投掷器/漏斗与通用方块更新。
- [服务端 tick 与权威世界](../world/server-tick-and-world.md)：方块事件与计划刻在 tick 中的执行位置。

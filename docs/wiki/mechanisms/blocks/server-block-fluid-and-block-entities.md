# 服务端方块、流体与方块实体

机制 ID：`server-block-fluid-and-block-entities`

## 机制目的和边界

本页定义方块状态写入、支撑/形状更新、随机刻与计划刻、流体扩散、玩家破坏与放置交互，以及容器/机器/幽匿/蜂巢/发射器等方块实体的权威契约。
红石信号与活塞推动归[服务端红石与活塞](server-redstone-and-pistons.md)，玩家菜单与背包规则归[服务端玩家、背包与战斗](../player/server-player-inventory-and-combat.md)。

## Vanilla 26.1 权威实现

- `Level.java:217-263`：写入目标区块方块状态并维护高度图/光照脏标记，再按更新标志位触发客户端同步、邻更与比较器信号通知、有界形状更新及 POI 更新。
- `FlowingFluid.java:118-140` 与 `FlowingFluid.java:368-407`：流体传播优先向下流动并分发至相邻水平方向（`spread`），按水平坡度搜索与动态阻挡形状决定横向传播目标（`getSpread`）。
- `DispenseItemBehavior.bootStrap`：按物品族分派发射器行为（投射物、桶、骨粉、刷怪蛋、船/矿车、剪毛/采蜜、装备穿戴、方块召唤与默认弹出）。

## 决策输入与状态所有权

| 输入 | 所有者 | 下游 |
| --- | --- | --- |
| 纯方块分类、支撑几何、属性算术、流体投影与声音事实 | `bbb-server-types` / `bbb-facts` | 核心层与生成宿主零拷贝查询 |
| 方块行为分派、形状更新（`BlockShapeCtx`）、骨粉/火焰/树苗生长 | `bbb-server-core::block_behavior` | 实时世界、生成区域与 `upgrade_data` 共用 |
| 流体扩散与桶拾取/倾倒 | `bbb-server-core::fluid` | 区块流体/含水状态写入与掉落 |
| 玩家挖掘进度纯计算、破坏掉落、放置与 `item_use_on` | `bbb-server-core::block_destroy` + `world::{block_destroy,block_use,item_use_on}` | 权限校验、方块/实体碰撞抬升、组件应用 |
| 强类型方块实体（熔炉、漏斗、蜂巢、唱片机、结构方块、宝库、试炼刷怪笼、幽匿振动） | `bbb-server-core::block_entity` + `world` | 计划刻/ticker 推进、库存转移与游戏事件 |

方块实体以 `(pos, block_entity_type)` 作为唯一身份，随所属 `Chunk` 存放与持久化；`ChunkStore` 内的 `LevelBlockEntities`（`chunk::block_entities::LevelBlockEntities`）维护活跃 ticker 位置、区段振动监听器与 tick 期间的临时借出表。

## 判断与处理顺序

1. **玩家挖掘与放置**：按玩家所在 `LevelId` 校验出生点保护、世界边界、游戏模式与挖掘进度（`bbb-server-core::block_destroy` 与 `world::block_destroy`）；方块默认交互拒绝后进入 `world::item_use_on`（斧/锄/锹/蜜脾状态转换、骨粉陆生/水生/群系地物催熟、打火石等）或方块放置（碰撞抬升、应用物品 `block_state` 与方块实体组件、预约计划刻）。
2. **统一状态写入**：`ChunkStore::set_block_state` 负责区段方块/流体状态写入、高度图与光照脏标记；实时世界经 `LevelCtx::set_block` 进一步维护方块实体创建/失效，随后沿有界栈传播形状更新与邻居通知。
3. **计划刻与随机刻**：区块按列存放计划刻，每 tick 收集已加载可模拟列中到期刻并按 `(trigger_tick, priority, sub_tick_order)` 执行（当 tick 新预约刻最早下一 tick 消费）。随机刻驱动作物、树苗 `TreeGrower` 数据包地物放置、火焰蔓延与维度条件冰融化（`water_evaporates`）。
4. **流体与爆炸**：流体传播对移动活塞、潜影盒等无静态形状方块按 `(level, pos)` 求值 `FluidCollisionShape::Dynamic`；爆炸在真实区块采样射线暴露率，统一结算方块掉落、连锁引爆与实体伤害推力。
5. **方块实体与自动化机器**：
   - **漏斗与漏斗矿车**：经共享容器转移入口（`world::simulation_tick::hopper`）先验目标可接纳性再扣减来源库存，保持有效组件合并规则与顶部碰撞/`does_not_block_hoppers` 阻挡语义。
   - **发射器与投掷器**：发射器复用玩家物品交互的同一行为所有者（桶、骨粉、剪刀采蜜/剪毛、投射物、穿戴等），投掷器执行默认投出或向相邻容器输送。
   - **生态与声学方块实体**：蜂巢统一管理驻巢计时、烟熏判定、环境留巢属性与紧急放蜂激怒；唱片机按当前注册表解析 `jukebox_playable` 曲目时长与比较器输出；幽匿感测体、催发体与尖啸体通过 `VibrationData` 推进振动监听与监守者警告等级。

## 生命周期和 fallback

更换方块类型或破坏方块时，立即清理不匹配的旧方块实体、注销 ticker/振动监听，并在掉落求值（含动态容器战利品展开）中共享同一份方块实体记录，确保合法物品组件零丢失。放置候选被碰撞拒绝时不回滚已发生的实体微调抬升。

## BBB 实现映射

| 职责 | 生产路径 | 边界说明 |
| --- | --- | --- |
| 方块/流体纯查询与属性步幅 | `crates/bbb-server-types/src`、`crates/bbb-facts/src` | 静态目录、形状、支撑与流体映射 |
| 方块行为、破坏进度、流体与方块实体 | `crates/bbb-server-core/src/block_behavior`、`crates/bbb-server-core/src/block_destroy.rs`、`crates/bbb-server-core/src/fluid.rs`、`crates/bbb-server-core/src/block_entity` | 支撑/生长规则、挖掘进度、流体传播与机器载荷 |
| 玩家交互、机器转移与振动投递 | `crates/bbb-server-core/src/world/block_use`、`crates/bbb-server-core/src/world/block_destroy.rs`、`crates/bbb-server-core/src/world/simulation_tick` | 放置/使用分派、漏斗转移与计划/随机刻推进 |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 方块写入、标志位同步、比较器与邻更/形状通知 | `Level.java:217-263` | `ChunkStore::set_block_state` / `LevelCtx::set_block` | `aligned` | 方块放置、破坏与邻更测试 |
| 流体纵向优先与横向坡度传播 | `FlowingFluid.java:118-140` / `FlowingFluid.java:368-407` | `bbb-server-core::fluid` | `aligned` | 水/熔岩扩散、含水方块与黑曜石/圆石生成测试 |
| 计划刻优先级排序与隔 tick 生效 | `LevelTicks` | `bbb-server-types::chunk_ticks` + `world::simulation_tick` | `aligned` | 计划刻顺序与机器时序测试 |
| 方块实体 Java 对象引用身份与回调重入 | `BlockEntity` 实例身份 | `(pos, type)` 值身份与借出表 | `not-needed` | [`AGENTS.md`](../../../../AGENTS.md) 非目标 |

## 结构性不变量

- 实时世界、`bbb-server-worldgen` 生成区域与存档 `upgrade_data` 通过 `BlockShapeCtx` 共享同一套支撑与形状更新算法。
- 容器间物品转移（漏斗、投掷器、命令）必须在目的地确认接纳后才扣减来源槽位，绝不因拒绝导致吞物或刷物。
- 方块实体在保存、掉落与组件收集间使用同一份强类型状态与保留字段，合法物品组件不丢失。

## 聚焦测试（focused tests）和真实证据

- `cargo dev test -p bbb-server-core block_behavior`、`block_entity` 与 `block_destroy`：覆盖支撑更新、骨粉催熟、挖掘进度、蜂巢、唱片机、箱子阻挡与计划刻。
- `cargo dev test -p bbb-server-core fluid`：覆盖水源形成、流动阻挡、含水方块与桶交互。
- `cargo dev test -p bbb-server-core hopper` 与 `cargo dev test -p bbb-server-core dispenser`：覆盖容器流转与发射器全物品族行为。

## 当前差异

方块实体按坐标与类型标识，不模拟 Java 对象引用相等性或嵌套重入回调。当前服务端账本无未关闭的方块、流体或方块实体功能缺口。

## 账本归属

方块、流体与方块实体的范围裁决及后续未关闭缺口统一归属[服务端差异账本](../../../unsupported-server-features.md)，推进原则见 [`AGENTS.md`](../../../../AGENTS.md)。

## 精确重开条件

当 26.1 方块状态目录、含水/支撑标签、`DispenseItemBehavior` 分派族或方块实体持久化字段变更，或发现容器流转组件丢失时重验本页。

## 相关页面和源码索引

- [服务端红石与活塞](server-redstone-and-pistons.md)：红石信号计算、元件延时与活塞移动。
- [服务端玩家、背包与战斗](../player/server-player-inventory-and-combat.md)：容器菜单点击、物品使用与有效组件合并。
- [服务端保存与资源重载](../persistence/server-save-and-reload.md)：区块内方块实体与计划刻落盘。

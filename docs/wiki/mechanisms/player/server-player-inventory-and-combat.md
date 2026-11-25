# 服务端玩家、背包与战斗

机制 ID：`server-player-inventory-and-combat`

## 机制目的和边界

本页定义在线玩家的权威移动校验与传送确认、背包/菜单交互、合成与功能方块菜单、手持物品使用（`use` / `useOn`）、近战/远程/矛攻击、伤害格挡与死亡重生契约。
连接与协议分帧归[服务端连接与会话](../protocol/server-connections-and-sessions.md)，有效组件比较与配方/战利品数据归[服务端数据包与战利品](../resources/server-datapack-and-loot.md)。

## Vanilla 26.1 权威实现

- `ServerGamePacketListenerImpl.java:525-547`：挂起服务端发起的传送，仅在收到匹配 ID 的 `AcceptTeleportation` 后解除挂起并接受后续客户端移动。
- `ServerGamePacketListenerImpl.java:1915-1951`：容器点击校验 `containerId`、菜单可用性、槽位边界与 `stateId`，`stateId` 失配时下发全量容器与光标重同步。
- `Item.use` / `Item.useOn`、`CombatRules`、`Player`：定义物品使用分派、攻击冷却、护甲/附魔减伤、盾牌格挡与死亡掉落。

## 决策输入与状态所有权

| 输入 | 所有者 | 下游 |
| --- | --- | --- |
| 在线玩家会话、移动包校验、传送挂起表 | `bbb-server-core::world::{session,player_packets,player_movement}` | 权威坐标、姿态、统计与纠偏包 |
| 物品栈运行时表示与有效组件等价比较 | `bbb-server-types::item_stack`（`RuntimeItemStack`）+ `bbb-protocol`（`ItemStackSummary`、`HashedStack`） | 背包、菜单、漏斗与战利品共享 |
| 玩家背包、容器菜单、配方放置与功能台（铁砧/附魔/锻造/信标等） | `bbb-server-core::inventory::{player_inventory,recipe_placement,utility_menu}` + `world::{containers,utility_menus}` | `RuntimeItemStack` 原地更新与 `HashedStack` 校对 |
| 持续使用物品、收纳袋、弓/弩/三叉戟/矛与可穿戴切换 | `bbb-server-core::world::{using_item,item_use,item_use_on}` + `entity::spear` | 物品消耗、使用余物、投射物与冲锋状态 |
| 近战/远程伤害结算、附魔触发、`CombatTracker` 与死亡掉落 | `bbb-server-datapack::damage`（`CombatTracker`）、`world::combat`、`world::player_death_drops` | 生命/饥饿/护甲损耗、死亡消息与经验/物品掉落 |
| 睡眠、床/重生锚占用与安全出生点解析 | `bbb-server-core::world::player_sleep` | 跳过夜晚、重生位置与 `LastDeathLocation` |

`bbb-server-types::item_stack::RuntimeItemStack` 包装规范化的 `ItemStackSummary`，堆叠合并、商人扣款与创造余物查重统一复用[服务端数据包与战利品](../resources/server-datapack-and-loot.md)的有效组件比较入口。

## 判断与处理顺序

1. **移动与骑乘同步**：校验挂起传送 ID、碰撞箱（随当前姿态与尺寸缩放）及速度预算；不合法移动回发服务端纠偏，受控坐骑经 `MoveVehicle` 应用动态碰撞、`maxUpStep` 与摔落传递。
2. **菜单点击与合成**：核验 `containerId` 与槽位合法性，执行拾取、拆分、拖动、双击归集、收纳袋权重存取（对应 Vanilla `BundleContents`，由 `bbb-server-core::inventory::bundle` 实现）、2×2/3×3 合成或功能台结算；取结果同步触发 `world::advancement_rewards::award_recipes`（`RecipeBookAdd` + `recipe_unlocked`）及地图扩展后处理，`stateId` 不一致时立即全量重同步。载具容器菜单直接读写 `EntityStore` 槽位并在首次非旁观打开时填充战利品表。
3. **物品与方块交互**：按玩家实际 `LevelId` 分派方块默认点击（触发 `default_block_use` / `any_block_use`）、`item_use_on`（磁石绑定、实体放置、末影之眼等）与对空 `item_use`（桶、钓竿、烟花助推、成书解析、知识之书、快速换装、弓弩蓄力与三叉戟激流）。手持地图按玩家所在维度渐进扫描地形并增量下发 `MapItemData`。
4. **战斗、附魔与死亡**：结算攻击冷却、矛戳刺（`STAB` / `post_piercing_attack`）、护甲/保护减伤、盾牌格挡及附魔效果（方块替换、属性修改、`run_function`、引雷/荆棘）。致死时按队伍可见性广播 `CombatTracker` 死亡消息，销毁消失诅咒物品，经 `ENTITY_DIE` 游戏事件结算幽匿催发体吸经验或生成经验球，并在第 20 tick 移除身体或提前重生时清空菜单自有格。

## 生命周期和 fallback

玩家关闭菜单或断线时，工作台合成格、商人付款格等菜单自有槽位自动退回背包（满包掉落，信标付款槽按 Vanilla 规则直接掉落）。断线时保存完整玩家状态、肩上鹦鹉与所属飞行末影珍珠树。

## BBB 实现映射

| 职责 | 生产路径 | 边界说明 |
| --- | --- | --- |
| 物品栈运行时表示与协议校验 | `crates/bbb-server-types/src/item_stack`、`crates/bbb-protocol/src` | `RuntimeItemStack`（`bbb-server-types`）与 `ItemStackSummary` / `HashedStack`（`bbb-protocol`） |
| 背包与功能菜单规则 | `crates/bbb-server-core/src/inventory` | 槽位规则、收纳袋、配方放置与功能台纯计算 |
| 会话、移动、使用与战斗 | `crates/bbb-server-core/src/world`、`crates/bbb-server-core/src/entity` | `world::{player_movement,containers,utility_menus,using_item,item_use,item_use_on,combat,player_death_drops,player_sleep,maps}` 与 `entity::spear` |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 传送 ID 确认门控 | `ServerGamePacketListenerImpl.java:525-547` | `bbb-server-core::world::player_movement` | `aligned` | 传送确认与越界回拉测试 |
| 容器点击与 `stateId` 重同步 | `ServerGamePacketListenerImpl.java:1915-1951` | `bbb-server-core::world::containers` / `inventory` | `aligned` | 菜单点击、快速移动与失配重同步测试 |
| 合成、功能台、收纳袋与地图同步 | `CraftingMenu` / `AnvilMenu` / `BundleItem` / `MapItemSavedData` | `bbb-server-core::inventory` + `world::{utility_menus,maps}` | `aligned` | 合成、铁砧/附魔、收纳袋与地图测试 |
| 近战/远程/矛战斗与死亡掉落 | `Player` / `LivingEntity` / `CombatTracker` | `bbb-server-datapack::damage` + `world::{combat,player_death_drops}` | `aligned` | 伤害减免、盾挡、附魔与死亡重生测试 |

## 结构性不变量

- 合法物品组件在背包点击、快速移动、容器交换、合成余物、地面掉落与存档往返间绝不丢失。
- 玩家所在维度以数值 `LevelId` 为唯一权威，所有射线检测、方块交互、实体生成与地图绘制均作用于同一关卡。
- `item modify` 等操作将物品数量置零变空后，后续修改不会使空栈重新复活。

## 聚焦测试（focused tests）和真实证据

- `cargo dev test -p bbb-server-core inventory` 与 `cargo dev test -p bbb-server-core container`：覆盖菜单点击、收纳袋、配方放置与功能台。
- `cargo dev test -p bbb-server-core combat`：覆盖近战、矛攻击、盾挡、附魔触发与死亡掉落。
- `cargo dev e2e server`：真实进程验证玩家进服、移动、挖掘放置与背包交互。

## 当前差异

BBB 不维护物品 Java 实例引用身份或背包事务回滚。当前服务端账本无未关闭的玩家移动、菜单、物品或战斗功能缺口。

## 账本归属

玩家移动、菜单、物品与战斗的范围裁决及后续未关闭缺口统一归属[服务端差异账本](../../../unsupported-server-features.md)，推进原则见 [`AGENTS.md`](../../../../AGENTS.md)。

## 精确重开条件

当 26.1 容器协议字段（如 `HashedStack` 或菜单目录）、战斗公式、武器数据组件或重生规则变更，或出现客户端槽位失步时重验本页。

## 相关页面和源码索引

- [服务端连接与会话](../protocol/server-connections-and-sessions.md)：游玩包有序解码与广播。
- [服务端数据包与战利品](../resources/server-datapack-and-loot.md)：有效组件比较、配方与战利品表求值。
- [服务端命令、计分板与进度](../control/server-commands-and-progression.md)：玩家进度触发器与统计计分。

# 服务端实体、生成与 AI

机制 ID：`server-entities-and-ai`

## 机制目的和边界

本页定义每个维度关卡的权威实体存储（`EntityStore`）、实体与玩家碰撞/骑乘几何、增量视野配对同步（追踪）、自然刷怪与统一生成终结（`finalize_spawn`）、生物 Goal/Brain AI 与寻路导航，以及投射物、载具、袭击和末影龙/监守者等首领与生物群落交互契约。
在线玩家自身移动与菜单归[服务端玩家、背包与战斗](../player/server-player-inventory-and-combat.md)，区块初始生物生成编排归[服务端区块与世界生成](../world/server-chunks-and-worldgen.md)，实体区域读写归[服务端保存与资源重载](../persistence/server-save-and-reload.md)。

## Vanilla 26.1 权威实现

- `ServerEntity` / `ChunkMap.TrackedEntity`：按区块列与追踪半径增量配对观察会话，同步生成、元数据、属性、装备、乘客关系与移除事件。
- `NaturalSpawner`、`Mob.finalizeSpawn`：执行分群上限检查、群系/结构刷怪列表采样与统一的装备/变种/幼年/骑乘生成终结。
- `LivingEntity`、`GoalSelector`、`Brain`、`PathNavigation`：驱动真实受击硬直/击退判定（区分无敌帧内差量伤害与完整受击）、地面/水中/两栖/飞行 A* 寻路、目标选择与物种行为。

## 决策输入与状态所有权

| 输入 | 所有者 | 下游 |
| --- | --- | --- |
| 关卡实体槽位、UUID/区段索引、乘客树与在线玩家碰撞投影 | `bbb-server-core::entity::EntityStore` | 空间范围查询、互推（`entity::push`）与借出更新 |
| 包围盒、迈步碰撞求解（`collide_with_step`）、弹道与动态支撑面几何；属性定义与基础值 | `bbb-server-types::geometry`；`bbb-facts::{attribute_types,default_attributes_26_1}` | 生物移动、玩家受控坐骑与潜影贝/上岸支撑共用 |
| 生成的实体运行时事实与音源分类 | `bbb-server-core::entity_runtime_facts_26_1`、`bbb-facts` | 尺寸、基础属性、挂点与声音类别 |
| Goal 表（`MobSpec` / `MobGoal`）、`Brain` 记忆与地面/水中/两栖/飞行导航 | `bbb-server-core::entity::ai` | 索敌（`TargetView`）、路径规划、近战（`entity::ai::mob::melee::do_hurt_target`）与远程/施法 |
| 自然刷怪、刷怪笼/试炼刷怪笼、袭击（`world::raids`）与末地龙战（`world::dragon_fight`） | `bbb-server-core::entity::natural_spawn` + `bbb-server-core::world` | 刷怪准入、波次/Boss 栏维护与跨实体 `EntityCommand` 落地 |
| 增量配对与协议差量编码 | `bbb-server-core::entity::tracking` + `bbb-protocol` | 单次编码、多观察者共享下发 |

在线玩家同时登记在所属关卡的 `EntityStore` 中供碰撞互推（遵守队伍 `collisionRule`）、骑乘挂点与追踪直接读取，但不混入通用非玩家实体存档。

## 判断与处理顺序

1. **生成与统一终结**：区块初始生成、自然刷怪、普通/试炼/矿车刷怪笼、结构模板、刷怪蛋、袭击波次与未传显式 NBT 的 `/summon` 统一经 `entity::general_living::finalize_spawn` 初始化属性、变种、音色、装备、幼体与骑手链；存档加载实体恢复 UUID、乘客树与保留的 `Team` 记分板绑定，不重复执行 `finalize_spawn`。
2. **感知、导航与 AI 决策**：
   - 区块方块变更去重后按批次仅通知受影响的活跃导航重算路径；地面、水中、两栖与飞行导航共享有界 A* 求解器（`NoAI` 生物暂停自主 AI 与移动，受玩家控制的坐骑仅运行 `tickRidden` 并由客户端 `MoveVehicle` 驱动）。
   - 物种行为由静态 `MobSpec` 优先级表或原生 `Brain` 记忆驱动：涵盖被动动物（繁殖/诱惑/恐慌/锁龄/装桶）、驯服宠物（跟随/上肩/陪睡）、村民（工作站认领/补货/八卦/铁傀儡召唤）、猪灵与疣猪兽（易物/怒气/狩猎/驱避）、监守者（振动增怒/嗅探/音波/黑暗脉冲）、水生生物及亡灵/袭击者战斗。
3. **物理移动、方块接触与投射物扫掠**：
   - 生物应用流体浮力/阻尼、粉雪攀出、炼药锅灭火/灼伤与作物/睡莲/海龟蛋踩踏（遵守 `mobGriefing` 与权限策略）。
   - 箭、三叉戟与投掷物（珍珠、药水、风弹等）完成射线与实体盒扫掠：命中方块经 `block_behavior::projectile_hit::on_projectile_hit` 分派；命中实体排入 `EntityCommand` 以在伤害与进度结算前保持直接来源身份（在线玩家的飞行末影珍珠跨维度持有 40 tick 刷新临时加载/模拟票据）。
4. **伤害、转换与生命周期落地**：生物近战统一经 `entity::ai::mob::melee::do_hurt_target` 结算，真实全额受击才触发击退与受击事件；跨实体副作用（闪电转化、僵尸感染/治愈、繁殖产子、刷怪、Boss 事件、玩家受伤/药水/蜂刺）经有界 `EntityCommand` 在实体循环归位后统一应用到 `ServerWorld`，随后处理死亡掉落（`entity::mob_death_loot` 与 `world::simulation_level::mob_death_loot`）、装备附魔掉率与远距清除。
5. **增量追踪同步**：非玩家追踪器按区块列索引，仅在会话跨界、新区块下发或实体跨区段时重算配对集；元数据、属性、装备、运动量与父载具骑乘图差量每包只编码一次并分发给所有已配对观察者（末影龙 8 个部件盒随父龙移动与受击路由，不单独入盘）。

## 生命周期和 fallback

嵌套伤害、掉落或谓词查询期间暂时移出 `EntityStore` 的当前实体与祖先通过 `LevelCtx::detached_entity` 只读借回，无需克隆实体副本；跨维度 UUID 引用通过 `world::level_keys::OtherLevels` 按需解析，卸载或不存在的目标保持不可用。缺少合法 UUID 的损坏存档实体直接丢弃。

## BBB 实现映射

| 职责 | 生产路径 | 边界说明 |
| --- | --- | --- |
| 几何与运行时目录事实 | `crates/bbb-server-types/src/geometry`、`crates/bbb-facts/src` | 包围盒、`collide_with_step`、实体音源与默认属性 |
| 实体存储、物理、物种状态与同步 | `crates/bbb-server-core/src/entity` | `EntityStore`、骑乘挂点、伤害、投射物、自然刷怪、存档投影与 `tracking` |
| Goal/Brain AI 与寻路导航 | `crates/bbb-server-core/src/entity/ai` | `MobSpec`、`Brain` 行为、感知节流与四类导航 |
| 刷怪编排、袭击、龙战与命令落地 | `crates/bbb-server-core/src/world` | `world::natural_spawn`、`world::raids`、`world::dragon_fight`、`world::ender_pearl` |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 增量实体配对、乘客树与差量广播 | `ChunkMap.TrackedEntity` / `ServerEntity` | `bbb-server-core::entity::tracking` | `aligned` | `tracking` 增量配对、跨维与骑乘同步测试 |
| 自然刷怪、刷怪笼与统一 `finalizeSpawn` | `NaturalSpawner` / `Mob.finalizeSpawn` | `entity::natural_spawn` + `world::natural_spawn` / `entity::general_living::finalize_spawn` | `aligned` | 群系/结构刷怪、群组变种与刷怪笼测试 |
| 寻路导航、Goal/Brain AI、袭击与龙战 | `GoalSelector` / `Brain` / `Raid` / `EndDragonFight` | `bbb-server-core::{entity,entity::ai,world}` | `aligned` | 各物种 AI、村民交易、猪灵易物、监守者、袭击与末影龙测试 |
| 内部 Goal tick 相位、selector 枚举顺序与 RNG 流 | Vanilla 内部 Goal/Brain 对象身份与调用序 | `MobSpec` 表驱动与 `EntityCommand` 队列 | `not-needed` | [`AGENTS.md`](../../../../AGENTS.md) 非目标，只对齐玩家可观察行为与存档 |

## 结构性不变量

- 每个关卡的实体状态由所属 `EntityStore` 唯一持有；AI 选择器只读借用 `TargetView`，跨实体增删改统一经 `EntityCommand` 在借用结束后落地，无回调重入或悬垂别名。
- 乘客挂点由共享 `entity::passenger::vehicle_geometry` 结合实时姿态、年龄倍率与 `scale` 属性动态计算，不向存档写入冗余派生的乘客坐标。
- 生成事实表（`entity_runtime_facts_26_1.rs`）由生成脚本从 Vanilla 26.1 派生并受 `docs-lint` 精确输出校核，不手改。

## 聚焦测试（focused tests）和真实证据

- `cargo dev test -p bbb-server-core --lib tracking`：验证实体配对、区块补配对、乘客同步与跨区块移动。
- `cargo dev test -p bbb-server-core entity::ai`、`natural_spawn`、`ender_dragon_fight` 与 `raid_saved_data`：验证寻路、村民/猪灵/监守者 AI、自然刷怪、末影龙战斗与袭击。

## 当前差异

BBB 不对齐内部 Goal 相位、随机流抽取次数或 Java 实例身份。当前服务端账本无未关闭的实体、刷怪或 AI 功能缺口。

## 账本归属

实体、生成与 AI 的范围裁决及后续未关闭缺口统一归属[服务端差异账本](../../../unsupported-server-features.md)，服务端新功能落点遵守[架构规则](../../development/architecture-rules.md#服务端新功能写法)与 [`AGENTS.md`](../../../../AGENTS.md)。

## 精确重开条件

当 26.1 实体目录、基础属性/尺寸挂点、刷怪规则或物种可观察交互变更，或 `generate-server-entity-runtime-facts.py` 对账出现差异时重验本页。

## 相关页面和源码索引

- [服务端玩家、背包与战斗](../player/server-player-inventory-and-combat.md)：在线玩家移动、坐骑控制与玩家伤害结算。
- [服务端区块与世界生成](../world/server-chunks-and-worldgen.md)：区块初始刷怪、结构刷怪覆盖与末影珍珠票据。
- [服务端数据包与战利品](../resources/server-datapack-and-loot.md)：死亡/赠礼/易物/剪毛战利品与实体谓词求值。
- [服务端保存与资源重载](../persistence/server-save-and-reload.md)：实体区域按需加载、未知字段透传与珍珠树保存。

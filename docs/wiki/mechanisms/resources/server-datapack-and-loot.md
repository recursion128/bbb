# 服务端数据包与战利品

机制 ID：`server-datapack-and-loot`

## 机制目的和边界

本页定义独立服务端的数据包发现与叠加载入、动态注册表/标签冻结、强类型战利品表（loot table）/谓词（predicate）/物品修饰器（item modifier）求值、环境属性时间线编译，以及跨系统的物品有效组件比较契约。
存档与 `/reload` 原子切换归[服务端保存与资源重载](../persistence/server-save-and-reload.md)，命令与进度消费归[服务端命令、计分板与进度](../control/server-commands-and-progression.md)。

## Vanilla 26.1 权威实现

- `PackRepository.java:82-96`：按包选择顺序、依赖与必选（required）约束构建有序的生效数据包列表（overlay 与 filter 由资源包加载层叠加应用）。
- `LootDataType`、`LootContext`、`LootItemCondition`、`LootItemFunction`、`ItemPredicate`：定义战利品池、数值提供器、环境/实体/位置/槽位谓词与物品修饰流水线。
- `DataComponentMap` / `ItemStack.isSameItemSameComponents`：定义默认组件叠加补丁与显式移除后的有效组件等价性。

## 决策输入与状态所有权

| 输入 | 所有者 | 下游 |
| --- | --- | --- |
| 数据包发现、overlay/filter、动态注册表、标签、配方、战利品、谓词、进度、函数与结构模板解码 | `bbb-server-datapack` | 启动与 `/reload` 构建不可变资源发布代 |
| 冻结注册表发布、方块标签紧凑位图与群系/环境属性预编译表 | `bbb-server-datapack` + `bbb-server-core` | 热路径零 JSON 重解析查询 |
| 战利品、谓词与物品修饰器求值器（`loot_runtime`、`LevelLootEnvironment`） | `bbb-server-core::loot_runtime` + `world` | 实体/方块掉落、钓鱼、考古、宝库、试炼刷怪笼、装备表、进度与命令共享 |
| 战利品抽样随机流（`GameRandom`）与 `/random` 命名随机序列（`random_sequences.dat`） | `LevelCtx`（战利品抽样）；`ServerWorld.cold.random_sequences`（仅 `/random` 与持久化） | 战利品抽样与 `/random` 命令严格分流 |
| 物品栈表示、有效组件等价比较与槽位范围 | `bbb-server-types::item_stack`（`RuntimeItemStack`）、`bbb-server-types::slot_ranges` + `bbb-protocol::packets::data_components` | 背包合并、漏斗、熔炉、商人交易与精确/局部组件谓词 |

## 判断与处理顺序

1. **包叠加载入与强类型预编译**：按优先级合并数据包，一次解码注册表、配方、战利品条目/条件/函数与全部环境属性（标量、离散音频/粒子/规则、RGB/ARGB 颜色时间线），为每代方块标签生成紧凑位图；单个损坏战利品元素记警告并跳过，内联命令语法错误直接拒绝。
2. **统一上下文绑定（`LevelLootEnvironment`）**：求值前按调用场景（`BLOCK`、`ENTITY`、`CHEST`、`FISHING`、`GIFT`、`PIGLIN_BARTER`、`ARCHAEOLOGY`、`VAULT`、`EQUIPMENT`、`COMMAND`、`ADVANCEMENT_*` 等）借读真实关卡天气、具名世界时钟、环境属性（叠加维度、quart 群系、时间线与天气修正）、计分板、命令 `storage`、已加载区块光照/结构部件及地图存储；未加载区块绝不因位置谓词触发世界生成，缺失必需读面的上下文安全拒绝求值。
3. **实体、载具与容器读面解析**：
   - 在线玩家按 UUID 按需借用真实库存与规范保存 NBT（含非空 `SelectedItem`），已移除的爆炸/风弹/药水来源复用当次捕获的规范快照。
   - `slots` 条目与槽位谓词共享 `bbb-server-types::slot_ranges`；遇到待展开 `LootTable` 的方块容器或载具容器（箱船/箱子矿车/漏斗矿车）时，仅在条件放行且触及有效槽位时按原关卡天气与种子就地展开并落脏（合成器 Crafter 保持原始槽位不展开）。
   - `targeted_entity` 与 `team` 条件直接读取真实 AI 目标资格与记分板队伍（驯服生物优先自身队伍，否则继承根主人队伍）。
4. **有效组件比较与物品修饰**：`bbb-server-types::item_stack::RuntimeItemStack` 的 `same_item_same_effective_components` 与 `matches_component` 结合当前世界冻结注册表，统一比较物品默认组件、显式补丁与移除标记；精确组件匹配与局部子谓词（`custom_data` NBT 子集、书页、容器/收纳袋集合、烟花、纹饰、唱片、属性修饰器、附魔、耐久）共享同一套类型化求值器。物品修饰器顺序执行函数与栈拆分，数量归零变空后不再由后续 `set_count` 复活。

## 生命周期和 fallback

资源发布按代不可变持有；动态注册表在启动期冻结，`/reload` 复用同一份启动冻结注册表并整体替换世界内配方集、标签索引、环境属性编译表、函数、进度与战利品/谓词表，已发布的空标签或缺失标签不静默回退内置成员。压缩、JSON/NBT 深度、递归表引用与路径穿越全程受安全预算保护。

## BBB 实现映射

| 职责 | 生产路径 | 边界说明 |
| --- | --- | --- |
| 数据包发现、overlay 与强类型解码 | `crates/bbb-server-datapack/src` | 注册表、标签、配方、战利品、进度、函数与世界生成解码 |
| 有效组件比较与槽位范围 | `crates/bbb-server-types/src/item_stack`、`crates/bbb-server-types/src/slot_ranges.rs` | `RuntimeItemStack`、`same_item_same_effective_components`、`matches_component`、`slot_ranges` |
| 环境音频/粒子与数据组件字段描述符 | `crates/bbb-protocol/src`、`crates/bbb-registry-codec/src` | 客户端与服务端共享的组件字段归属与源解码 |
| 运行时战利品与谓词求值 | `crates/bbb-server-core/src/loot_runtime`、`crates/bbb-server-core/src/world` | `LevelLootEnvironment`、容器展开与组件匹配 |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 数据包选择优先级与必选包合并 | `PackRepository.java:82-96` | `bbb-server-datapack` | `aligned` | 包叠加、filter 与 `/datapack` 测试 |
| 战利品池、条件、数值提供器与修饰函数 | `LootTable` / `LootItemCondition` / `LootItemFunction` | `bbb-server-datapack::loot` + `bbb-server-core::loot_runtime` | `aligned` | 全参数集抽表、环境/槽位/实体谓词测试 |
| 有效组件等价比较与组件子谓词 | `DataComponentMap` / `DataComponentPredicates` | `bbb-server-types::item_stack` + `bbb-protocol` | `aligned` | 背包/漏斗/熔炉合并与组件谓词测试 |
| DataFixer `Codec` 多模式与 `DataResult` 诊断文本 | Mojang DFU `Codec` 内部对象 | 单一强类型解码器与有界预算 | `not-needed` | [`AGENTS.md`](../../../../AGENTS.md) 非目标 |

## 结构性不变量

- 数据包 JSON 在加载/重载期一次解码为强类型结构；tick 热路径（方块标签查询、环境属性采样、战利品抽样）不重复解析原始 JSON。
- 背包合并、漏斗传输、熔炉接纳、商人扣款、转化配方与战利品/命令谓词全部通过 `RuntimeItemStack` 唯一有效组件比较入口判定等价性。
- 战利品位置与结构谓词只读借用已加载区块，绝不在谓词求值期隐式触发区块生成。

## 聚焦测试（focused tests）和真实证据

- `cargo dev test -p bbb-server-datapack`：覆盖包发现、overlay/filter、注册表冻结与战利品/配方/进度解码。
- `cargo dev test -p bbb-server-core loot`：覆盖各参数集战利品抽样、环境属性条件、载具/容器延迟展开与组件子谓词。

## 当前差异

不存在的 `#tag` 引用或颠倒的 `min`/`max` 范围仅跳过对应战利品元素而不拒绝整包加载；战利品抽样直接消费关卡 `GameRandom`（`random_sequences.dat` 专供 `/random` 与持久化），DFU `Codec` 异常消息与 RNG 内部调用次数不做复刻。当前服务端账本无未关闭的数据包或战利品功能缺口。

## 账本归属

数据包、战利品与谓词的范围裁决及后续未关闭缺口统一归属[服务端差异账本](../../../unsupported-server-features.md)，推进原则见 [`AGENTS.md`](../../../../AGENTS.md)。

## 精确重开条件

当 26.1 新增数据组件、战利品条件/函数、环境属性类型或数据包 `pack_format` 规则变更，或发现有效组件比较在容器/菜单间不一致时重验本页。

## 相关页面和源码索引

- [客户端连接、协议阶段与注册表](../protocol/client-connection-and-registries.md)：同步注册表与标签在客户端的提交与校验。
- [服务端保存与资源重载](../persistence/server-save-and-reload.md)：存档加载与运行时 `/reload` 切换。
- [服务端命令、计分板与进度](../control/server-commands-and-progression.md)：`/loot`、`/item`、`/execute if items|predicate` 与进度条件求值。

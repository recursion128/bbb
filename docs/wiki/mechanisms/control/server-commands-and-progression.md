# 服务端命令、计分板与进度

机制 ID：`server-commands-and-progression`

## 机制目的和边界

本页定义游戏内命令解析/补全与执行（含 `execute` 链、实体选择器、物品/方块谓词与宏函数）、服务端计分板/队伍/Boss 栏同步，以及全量 26.1 数据驱动进度（advancement）触发、条件求值与奖励发放契约。
外部 MCP 管理面归[服务端 MCP 管理](server-mcp-administration.md)，战利品与通用谓词底层求值器归[服务端数据包与战利品](../resources/server-datapack-and-loot.md)。

## Vanilla 26.1 权威实现

- `Commands`、`EntitySelector`、`ExecuteCommand`、`CommandFunction`：定义权限等级过滤、选择器解析、`execute` 重定向/条件分支与函数执行预算。
- `ClearInventoryCommands.java:61-93`：遍历玩家背包、装备、合成输入与光标累计清除数量并同步菜单，区分 `maxCount=0` 只读计数、零匹配反馈与显示名称反馈。
- `AdvancementRewards.java:39-90`：向达成进度的在线玩家发放经验、按 `ADVANCEMENT_REWARD` 抽取战利品（满包转地面掉落）、解锁配方并静默执行奖励函数。

## 决策输入与状态所有权

| 输入 | 所有者 | 下游 |
| --- | --- | --- |
| 命令语法树、UTF-16 解析/补全、选择器、物品谓词与函数宏实例化 | `bbb-server-core::commands` | 玩家聊天命令、函数、命令方块与 MCP 共享入口 |
| 命令世界读写上下文（`CommandCx`、`CommandOrigin`、`EntityReads`） | `bbb-server-core::commands`（`CommandCx`）、`commands::types`（`CommandOrigin`）、`world::command_reads`（`EntityReads`） | 只读借用查询与直接修改目标状态 |
| 计分板、队伍、统计联动与自定义 Boss 栏 | `world::server_scoreboard`、`world::score_criteria`、`world::boss_events` | 分数计算、击杀/队伍准则更新、客户端广播与持久化 |
| 58 类进度触发器加载校验（含 1 类被动触发器 `minecraft:impossible`）与 57 类主动触发求值/奖励 | `bbb-server-datapack::advancement` + `world::advancement_{triggers,loot,rewards,flush}` | 条件按需投影、进度达成公告、奖励与存档同步 |

## 判断与处理顺序

1. **编码与解析预算**：协议层限制网络命令为 32767 个 UTF-16 单元（MCP 管理命令独立限制 256 个 Unicode 标量字符）；解析器、签名参数、命令方块与函数宏统一基于 UTF-16 单元与 `JavaString` 处理，补全替换偏移按 UTF-16 单元输出。长 `execute` 链使用显式工作列表迭代展开，不消耗线程递归调用栈；函数执行受递归与操作配额限制。
2. **只读借用与原地执行**：选择器、`/execute if|unless (entity|block|items|predicate|score|data)` 与动态文本组件先通过 `EntityReads` 短借各维度玩家、实体库、区块与计分板完成筛选（未生成交易的商人在 `/data` 读取前按当前数据包生成报价）；随后 `CommandCx` 直接在 `ServerWorld` 上执行写入（如 `/tp` 准备目标 `FULL` 列并迁移乘客子树、`/summon` 校验 UUID 冲突与递归乘客、`/loot`、`/item`、`/clear`、`/ride`、`/rotate`、`/swing`）。多目标命令中途报错时保留已生效目标的写入。
3. **计分板与统计联动**：玩家统计增减在 `score_criteria` 同步更新对应全部计分板目标；击杀归属同步更新 `totalKillCount`、`playerKillCount` 及按当次双方队伍颜色结算的 `teamkill.*` / `killedByTeam.*`，非玩家实体按 UUID 计分。
4. **进度触发与奖励**：`bbb-server-datapack::advancement` 在加载期以冻结注册表校验全部 58 类触发器条件并裁剪孤立子节点；运行时先匹配轻量事件参数（槽位、配方、方块状态、伤害值、距离起点等），命中后才按需构造一次真实玩家/关卡/来源实体 `LevelLootEnvironment`（`world::simulation_level::mob_death_loot`）求值复合条件（含已移除爆炸/药水来源的保留快照），达成后发放 `bbb-server-datapack::advancement::AdvancementRewards` 并在 tick 末尾批量同步进度增量包。

## 生命周期和 fallback

数据包重载时重新编译函数库与进度触发器表，保留玩家已达成的准则（`JavaString`）记录。命令语法解析通过不代表未支持分支静默成功；对于范围外或未支持的命令分支，执行时明确报错。

## BBB 实现映射

| 职责 | 生产路径 | 边界说明 |
| --- | --- | --- |
| 命令解析、补全、选择器与函数定义 | `crates/bbb-server-core/src/commands` | 语法扫描、物品/方块谓词编译与 UTF-16 补全 |
| 命令落地、计分板与 Boss 栏 | `crates/bbb-server-core/src/world` | `world::command_reads`、`world::command_execution`、`world::server_scoreboard`、`world::score_criteria`、`world::boss_events` |
| 进度加载与运行时触发 | `crates/bbb-server-datapack/src/advancement`、`crates/bbb-server-core/src/world` | `bbb-server-datapack::advancement` 触发器/奖励解码与 `world::advancement_*` 执行/同步 |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| `/clear` 跨槽位计数、限额与反馈文本 | `ClearInventoryCommands.java:61-93` | `world::command_execution` | `aligned` | `/clear` 谓词、限额与 `maxCount=0` 测试 |
| `/execute` 重定向/条件链、选择器与函数宏 | `ExecuteCommand` / `EntitySelector` / `CommandFunction` | `commands` + `world::command_execution` | `aligned` | `execute` 子命令、选择器与宏函数测试 |
| 58 类进度触发器、条件求值与奖励交付 | `CriteriaTriggers` / `AdvancementRewards.java:39-90` | `bbb-server-datapack::advancement` + `world::advancement_*` | `aligned` | 各族进度触发器、来源快照与奖励测试 |
| Brigadier `ExecutionContext` 内部帧对象图 | Brigadier 内部回调队列 | 显式工作列表与函数配额 | `not-needed` | [`AGENTS.md`](../../../../AGENTS.md) 仅对齐玩家可见反馈、结果与配额 |

## 结构性不变量

- 玩家命令、数据包函数、命令方块与 MCP 管理端共享同一命令执行语义，不维护分叉实现。
- 命令查询直接借用当前世界，不为单条命令克隆整份玩家或实体库快照，也不提供世界级事务回滚。
- 进度玩家条件仅在事件前置条件命中后按需投影真实服务端状态（如真实燃烧剩余时间、真实 `CROUCHING` 姿态、致死前装备快照），不以客户端按键或事后清空槽位替代。

## 聚焦测试（focused tests）和真实证据

- `cargo dev test -p bbb-server-core command_execution` 与 `cargo dev test -p bbb-server-core commands::parser`：覆盖语法解析、UTF-16 补全、`execute` 链、选择器及 `item`/`loot`/`clear`/`summon`/`tp`/`ride` 等命令行为。
- `cargo dev test -p bbb-server-core advancement`：覆盖进度触发器编译、跨维度来源求值、奖励发放与存档恢复。
- `cargo dev test -p bbb-server-core scoreboard`：覆盖目标准则、队伍颜色击杀计分与同步。

## 当前差异

Brigadier 内部调度对象图与内部异常堆栈文本不做复刻，但玩家可见的命令成功返回值、报错/反馈文本、权限过滤与世界效果保持对齐；`/jfr`、`/perf`、`/test` 与在线 `/fetchprofile name|id` 已在账本裁决为 `not-needed`。当前服务端账本无未关闭的命令或进度功能缺口。

## 账本归属

命令、计分板与进度的范围裁决及后续未关闭缺口统一归属[服务端差异账本](../../../unsupported-server-features.md)，推进原则见 [`AGENTS.md`](../../../../AGENTS.md)。

## 精确重开条件

当 26.1 新增命令节点、选择器参数、进度触发器类型或计分板准则，或发现命令补全 UTF-16 偏移/反馈文本与 Vanilla 不一致时重验本页。

## 相关页面和源码索引

- [服务端 MCP 管理](server-mcp-administration.md)：外部管理鉴权与邮箱投递。
- [服务端数据包与战利品](../resources/server-datapack-and-loot.md)：函数加载、物品/实体谓词与战利品求值器。
- [服务端实体、生成与 AI](../entities/server-entities-and-ai.md)：`/summon`、`/ride`、队伍绑定与实体交互进度来源。

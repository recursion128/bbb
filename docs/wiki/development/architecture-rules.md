# 架构与代码组织规则

## 语义优先

模块按语义所有者（semantic owner）命名和拆分；协议线层（wire）、客户端世界、服务端权威世界、
渲染器输入（renderer input）、管理适配器与底层后端类型不混为一层。新增功能先从 Vanilla 源码确认
玩家可见规则和所需状态，再按 bbb 自身语义选择现有 crate/模块，不按 Java 类层级建模。
同等功能优先更少的代码、更少的状态和更快的路径。不要用兜底式的
`lib.rs`/`main.rs` 积累跨域逻辑，也不要为了行数创建单函数模块。

## 拆分条件

小于约 1000 行的内聚文件默认保持完整。拆分必须同时具备命名明确的所有者、窄
API、聚焦测试（focused test）或明确测试计划，以及可陈述的依赖、编译或理解收益。单一调用方的辅助函数
留在调用处附近；Rust 使用 2018 模块布局 `foo.rs` + `foo/bar.rs`，不新增 `foo/mod.rs`。

所有 crate 的手写 `.rs` 文件另有硬上限：单文件 ≤ 3000 行，由 `scripts/source_file_size_ratchet.py`
的 `check` 子命令（文档检查一步）强制执行；存量超限文件固定在 `scripts/data/source-file-size-ratchet.json`，
上限只许缩小、不许新增（新的超限文件必须拆分，不能登记）；`// @generated` 文件豁免。拆分方式固定为外观层 + 域子模块：
外观层只留类型定义、入口函数、分发表与 `mod` 声明，每个域一个 `foo/<domain>.rs`（`impl Foo {}`
可跨文件），搬动的私有项改 `pub(super)`，只搬不改、锚点一字不动；按资源 ID 接线的同类所有者
进入外观层里的一张表宏（`block_entity_payloads!` 形状），此后新增一个
所有者即新建 `foo/<owner>.rs` 并在表中追加一行。这是多工作线（lane）并行的前提：多条工作线只在各自的域文件里
修改，共享外观层只接受表行/`mod` 行追加，编译与评审范围也随之缩小。

## API 和状态

- 优先结构化数据包与状态类型，不跨边界传临时裸字节或字符串。
- `pub` 只暴露消费者真正需要的最小类型与查询；测试不得迫使生产 API 变宽。
- 派生快照不回写权威状态；后端句柄不进入世界状态。
- 协议、常量和关键规则注明 Vanilla 类/方法；输入、坐标与结果对齐，内部执行顺序、RNG 流
  与回退分支只在影响协议或玩家可见结果时才对齐。
- 渲染器不能凭渲染层或名称反推独立的方块状态事实。
- 客户端可见世界不能升级为服务端权威世界；服务端数据包应用、tick、存档与 MCP
  写操作必须各有单一所有者，并通过强类型命令/事件投影跨边界。
- MCP/RPC/套接字处理器不直接借用运行中的服务端世界；服务端 MCP 的查询与写请求统一进入
  有界 tick 邮箱，在 tick 内生成独占结果。客户端控制查询读取冻结发布状态；
  两端具体路径分别见[服务端管理](../mechanisms/control/server-mcp-administration.md)与
  [客户端运行时](../mechanisms/runtime/client-runtime-and-control.md)。

## 服务端新功能写法

当前服务端架构约定是新功能的默认写法，机制现状见[服务端边界](../architecture/server-boundaries.md)。

- 落点：纯值进 `bbb-server-types`；数据包（datapack）内容在 `bbb-server-datapack` 加载时一次解码为强类型，
  非法内容加载时拒绝；世界生成纯算法进 `bbb-server-worldgen`；存档结构进 `bbb-server-persistence`；
  玩法、方块、实体、区块、命令都在 `bbb-server-core`。不新增 crate、外观层重导出、`*_bridge.rs` 或测试专用 feature。
- 直接写真实状态：玩法借 `LevelCtx`（只由 `LevelCtx::new` 构造），改方块只经 `LevelCtx::set_block`；
  命令经 `CommandCx` 直接写 `ServerWorld`，失败不回滚。不新增反向 Level/World trait、视图/效果
  回放、快照回写或事务；trait 只在已有两个以上生产实现时引入。
- 实体：新状态放 `EntityStore` 槽位（`GeneralLivingData` 基类或 `MobVariantState` 变种），在线玩家
  也是槽位；需要玩家、地图、命令或整关卡的副作用排入 `EntityCommand`，由 `run_entity_commands` 执行。
  新生物行为优先在 `MobSpec`/`GoalSpec` 表加一行，表达不了再写钩子函数；生成终结只经 `finalize_spawn`。
- 随机：玩法只用 `GameRandom`（`ServerWorld.random` 的句柄，经 `LevelCtx` 借出），不另存 RNG 状态，
  句柄不跨线程；世界生成只用 `bbb_server_worldgen::random`。不为 Java RNG 流、tick 内部顺序写代码。
- 分派：按资源 ID 派发的同类所有者新增即 `foo/<owner>.rs` + 外观层表一行；强类型枚举用穷尽 `match`。
- 线程：`ServerWorld` 只在 `bbb-world` 线程上修改；耗时的纯计算拿冻结输入交由 Rayon 并行计算，结果回 world 线程
  按确定顺序提交；存档读写交由 `bbb-save-read` / `bbb-save-write` 线程。不用锁共享世界状态。
- 热路径：使用 `FxHashMap`、`LevelId(u32)` 与按值查表；不每 tick 克隆快照、重解析 JSON 或分配临时集合。
- 测试：断言玩家或存档可观察结果，使用真实 NBT、数据包与生成 API；不测内部顺序、RNG 流或调用次数，
  不为测试放宽 `pub`。

## 重构与验证

任意 Java 字符串复用 `JavaString` 保存原始 UTF-16 单元；资源 ID 等受限标识符保留各自
校验，不机械迁移所有 Rust `String`。先补真实调用所需的封装操作与共享编解码器，再按
输入、类型状态、消费者、保存/网络/显示的完整调用链验收。显示替换、网络 UTF-8 与 NBT
改进 UTF-8（Modified UTF-8）各自集中在对应边界；禁止隐式有损转换和 `Deref<Target=str>`，原始键的查询、
相等和哈希不得使用显示替换值。显示缓存只能从单一规范状态派生并同步维护，普通文本热查询
不应增加分配。不复刻 Java 字符串哈希或集合迭代顺序，除非协议或存档输出直接依赖。
统一公开字符串变体前先评估存档、`serde`、API 与调用范围，只有具体收益足以覆盖兼容成本时实施。

结构重构不能顺带改变功能合同。先保留行为测试，再迁移所有者，最后删除旧路径。新增
模块的测试与所有者同位置；跨层只添加能证明集成合同的测试。完整
推进原则与协作规则只取 [`AGENTS.md`](../../../AGENTS.md)，执行流程只取
[测试门禁](testing-and-gates.md)。

架构收敛后的常规维护不产生开放式优化队列。只有机械审计发现两个
生产所有者、平行分派器或解析器、已声明删除的兼容路径回流，或本页结构
不变量失效时，才在账本（ledger）中登记具体所有者路径、影响范围、删除目标、验证证据和重开条件；修复后
删除活跃项，历史从 Git 查询。

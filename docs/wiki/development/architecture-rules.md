# 架构与代码组织规则

## 语义优先

模块按 semantic owner 命名和拆分；wire、client world、authoritative server world、
renderer-input、admin adapter 与 backend 类型不混为一层。新增功能先从 Vanilla 源码确认
玩家可见规则和所需 state，再按 bbb 自身语义选择现有 crate/module，不按 Java 类层级建模。
同等功能优先更少的代码、更少的状态和更快的路径。不要用
catch-all `lib.rs`/`main.rs` 积累跨域逻辑，也不要为了行数创建单函数模块。

## 拆分条件

小于约 1000 行的 cohesive 文件默认保持完整。拆分必须同时有命名明确的 owner、窄
API、focused test 或明确测试计划，以及可陈述的依赖/编译/理解收益。单 caller helper
留在 caller 附近；Rust 使用 2018 布局 `foo.rs` + `foo/bar.rs`，不新增 `foo/mod.rs`。

所有 crate 的手写 `.rs`（2026-09-07 起从 server 侧扩到全仓）另有
硬上限：单文件 ≤ 3000 行，由 `scripts/source_file_size_ratchet.py check`（docs-lint 一步）enforce；
规则落地时已超限的文件钉在 `scripts/data/source-file-size-ratchet.json`，pin 只许缩小、不许新增
（新的超限文件必须拆，不能登记）；`// @generated` 文件豁免。拆法固定为 facade + 域子模块：
facade 只留类型定义、入口 fn、分发表与 `mod` 声明，每个域一个 `foo/<domain>.rs`（`impl Foo {}`
可跨文件），搬动的私有项改 `pub(super)`，只搬不改、锚点一字不动；按 resource id 接线的 owner 族
进 facade 里的一张表宏（`block_entity_payloads!` 形状），此后新增一个
owner = 新建 `foo/<owner>.rs` + 表里一行。这是 lane 并行的前提：多条 lane 只在各自的域文件里
写，共享 facade 只接受表行/`mod` 行追加，编译与 review 面也随之缩小。

## API 和状态

- 优先 structured packet/state type，不跨边界传 ad-hoc bytes/string。
- `pub` 只暴露消费者真正需要的最小类型与查询；测试不要迫使生产 API 变宽。
- derived snapshot 不回写 canonical state；backend handle 不进入 world。
- 协议、常量和关键规则注明 Vanilla 类/方法；输入、坐标与结果对齐，内部执行顺序、RNG 流
  与 fallback 分支只在影响协议或玩家可见结果时才对齐。
- renderer 不能用 layer/name 推导独立 block-state facts。
- 客户端可见世界不能升级为服务端权威世界；server packet application、tick、save 和 MCP
  mutation 必须各有单一 owner，并通过 typed command/event 投影跨边界。
- MCP/RPC/socket handler 不直接借用 live server world；服务端 MCP 的查询和写请求都进入
  有界 tick mailbox，在 tick 内生成 owned 结果。客户端 control 查询读取冻结 publication；
  两端具体路径分别见[服务端管理](../mechanisms/control/server-mcp-administration.md)与
  [客户端运行时](../mechanisms/runtime/client-runtime-and-control.md)。

## 服务端新功能写法

服务端重构（M2–M7）形成的写法是新功能的默认形状，机制现状见[服务端边界](../architecture/server-boundaries.md)。

- 落点：纯值进 `bbb-server-types`；datapack 内容在 `bbb-server-datapack` 加载时一次解码为强类型，
  非法内容加载时拒绝；worldgen 纯算法进 `bbb-server-worldgen`；存档 schema 进 `bbb-server-persistence`；
  玩法、方块、实体、区块、命令都在 `bbb-server-core`。不新增 crate、façade 重导出、`*_bridge.rs` 或 test-support feature。
- 直接写真实状态：玩法借 `LevelCtx`（只由 `LevelCtx::new` 构造），改方块只经 `LevelCtx::set_block`；
  命令经 `CommandCx` 直接写 `ServerWorld`，失败不回滚。不新增反向 Level/World trait、view/effect
  回放、快照回写或事务；trait 只在已有两个以上生产实现时引入。
- 实体：新状态放 EntityStore 槽位（`GeneralLivingData` 基类或 `MobVariantState` 变种），在线玩家
  也是槽位；需要玩家、地图、命令或整关卡的副作用排入 `EntityCommand`，由 `run_entity_commands` 执行。
  新 mob 行为优先在 `MobSpec`/`GoalSpec` 表加一行，表达不了才写 hook；生成终结只经 `finalize_spawn`。
- 随机：玩法只用 `GameRandom`（`ServerWorld.random` 的句柄，经 `LevelCtx` 借出），不另存 RNG 状态，
  句柄不跨线程；worldgen 只用 `bbb_server_worldgen::random`。不为 Java RNG 流、tick 内部顺序写代码。
- 分派：按 resource id 的 owner 族新增 = `foo/<owner>.rs` + facade 表一行；强类型枚举用穷尽 `match`。
- 线程：`ServerWorld` 只在 `bbb-world` 线程上改；耗时的纯计算拿冻结输入上 rayon，结果回 world 线程
  按确定顺序提交；存档读写交 `bbb-save-read` / `bbb-save-write` 线程。不用锁共享世界状态。
- 热路径：`FxHashMap`、`LevelId(u32)` 与按值查表；不每 tick 克隆快照、重解析 JSON 或分配临时集合。
- 测试：断言玩家/存档可观察结果，用真实 NBT、packet 与生成 API；不测内部顺序、RNG 流或调用次数，
  不为测试放宽 `pub`。

## 重构与验证

任意 Java 字符串复用 `JavaString` 保存原始 UTF-16 单元；资源 ID 等受限标识符保留各自
校验，不机械迁移所有 Rust `String`。先补真实调用所需的 wrapper 操作与共享 codec，再按
输入、类型状态、消费者、保存/网络/显示的完整调用链验收。显示替换、网络 UTF-8 与 NBT
modified UTF-8 各自集中在对应边界；禁止隐式有损转换和 `Deref<Target=str>`，原始键的查询、
相等和哈希不得使用显示替换值。显示缓存只能从单一规范状态派生并同步维护，普通文本热查询
不应增加分配。不复刻 Java 字符串哈希或集合迭代顺序，除非协议或存档输出直接依赖。
统一公开字符串变体前先评估存档、serde、API 与调用范围，只有具体收益足以覆盖兼容成本时实施。

结构重构不能顺带改变功能合同。先保留行为测试，再移动 owner，最后删除旧路径。新增
module 的测试与 owner 同位置；跨层只添加能证明 integration contract 的测试。完整
推进原则与 agent 协作只取 [`AGENTS.md`](../../../AGENTS.md)，执行流程只取
[测试门禁](testing-and-gates.md)。

architecture-convergence 的稳定 maintenance 不产生开放式优化队列。只有机械审计发现两个
production owner、平行 dispatch/resolver、已声明删除的 compatibility path 回流，或本页结构
不变量失效时，才在 ledger 登记具体 owner/path、影响、删除目标、evidence 和重开条件；修复后
删除 active 项，历史从 Git 查询。

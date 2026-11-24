# AGENTS.md

## 任务范围与权威入口

先运行 `git status --short`，按用户本次请求确定范围。审计、文档维护、指定修复是有界任务，完成请求、
适用验证和落地后结束；用户要求持续推进产品时，落地后直接选下一项，不在每次落地后停下汇报。
已授权范围内的本地修改、验证、修复与落地自主推进；只在缺少必要授权或关键输入时询问。
发现范围外缺口登记到 ledger 或报告；接续其它 session 的 WIP 需已有授权并保留现场。

| 权威 | 唯一职责 |
| --- | --- |
| [客户端 ledger](docs/unsupported-client-features.md)、[服务端 ledger](docs/unsupported-server-features.md) | 未关闭的功能缺口与验收结论 |
| [客户端扩展](docs/client-extensions.md) | 已选定的 BBB 自有客户端能力 |
| [开发流程与测试](docs/wiki/development/testing-and-gates.md) | 日常循环、测试选择、落地、full gate、性能基准、发布与接手 |
| [架构规则](docs/wiki/development/architecture-rules.md) | 语义模块、API、文件大小与单一 production owner |
| [架构总览](docs/wiki/architecture/overview.md)、[客户端边界](docs/wiki/architecture/client-boundaries.md)、[服务端边界](docs/wiki/architecture/server-boundaries.md) | 产品范围、crate 所有权与跨边界结构不变量 |
| [构建与缓存](docs/wiki/development/build-and-cache.md) | 环境准备、工具链、target 目录与 worker 租约 |
| [文档工作流](docs/wiki/development/documentation-workflow.md) | 文档归属与 ledger 写法 |

表格是按需查阅的入口，不是每次必读清单。完成史只查 Git，不新增 archive/index。

## 产品与推进原则

`bbb` 自己实现 Minecraft Java 26.1 外部多人客户端、WebGPU 浏览器客户端及 offline-only headless dedicated server，
不包装 Bevy、Azalea 或其它客户端/服务端栈。不做游戏内配置 UI。
Configuration is supplied at command-line startup.
BBB 自有客户端能力登记到客户端扩展 ledger，不伪装为 Vanilla 差异。

推进原则：基本对齐官方功能，实现自由发挥，以比官方少得多的代码和更强的性能实现同等功能。
- **协议**：26.1 wire 严格兼容（可观察的 packet、字段、phase、顺序与同步）。
- **存档**：正确读取官方 26.1 存档与 datapack 且合法数据不丢失；BBB 自身存档只需 BBB round-trip，不要求官方服务端能读；非法输入保留既有大小、深度、数量与路径安全预算。
- **功能与生成**：玩家可见规则、常量、掉落、合成、AI、命令报错/反馈文本与红石结果基本对齐（红石只要求机器正常工作）。worldgen 对齐 datapack 驱动的群系、地形形态、结构放置、矿物与植被机制，同一 BBB 种子与输入自身确定，不要求与官方同种子逐方块一致；官方存档已生成区块原样保留，交界允许接缝。
- **渲染**：所有图形后端共用通用 WGSL 与 `wgpu` 公共 API，不加厂商/底层 API 专用调优，不靠降低视距、画质、实体或粒子负载换性能。
- **非目标**：tick 内部顺序、RNG 流（含 worldgen）、goal 相位、对象身份/别名、回调重入、事务回滚、Codec 多模式与异常域、内部诊断文本不是对齐目标；不为它们写代码、测试或 ledger 项，遇到已有此类机制优先删除或收缩。

- Vanilla 源码用 `cargo dev vanilla path 26.1` 解析（需要时 `cargo dev vanilla prepare 26.1`），
  不从 `$HOME` 猜路径，不凭记忆写协议、常量或存档格式。
- 协议查 `GameProtocols.java` 与对应 packet；玩家可见行为从源码确认规则、常量与结果，不逐类
  复刻内部调用链。
- 行号锚点只用于协议、常量和关键规则；写前必须打开那段源码，改动锚点时运行
  `cargo dev anchors`。
- 旧代码可以按目标替换或删除；primary 历史只能 fix-forward。

## 协作

主 agent 拥有选题、关键 Vanilla 核验、接口合同、集成、完整 diff review 与落地。选题优先 P0，
再按玩家影响和解锁价值从 ledger 选功能项；一个功能可跨 crate 和多次提交。

至少两个任务具备独立价值、路径不重叠、接口稳定且可分别验收，且主 agent 有关键路径工作可做时，
启动两到三个 worker；否则保持 solo。不要为占满槽位制造工作；只读探索可以随时并行。

- 不同 worker 不写同一文件；worker 只在自己的分支提交，可把分支 rebase 到 primary，
  不碰 primary、不回退别人改动。
- worker 隔离（`cargo worker`）、`git merge --ff-only` 落地、`cleanup` 回收与中断保留规则见
  [开发流程与测试：落地](docs/wiki/development/testing-and-gates.md#落地)及
  [中断与接手](docs/wiki/development/testing-and-gates.md#中断与接手)。

Worker prompt 给出：工作目录与分支、`CARGO_TARGET_DIR`、owned paths、目标/非目标、接口、关键
Vanilla 事实、focused test 命令与验收结论；说明其他 agent 同时工作、只在自己分支提交、代码注释
用英文。中间消息只报告阻塞或推翻合同的事实；最终报告给出 commit、行为变化、验证结果与未验证边界。

用户指定的模型优先。effort 按风险选：机械修改与搜索用 low，普通实现用 medium，复杂协议、
并发、安全与高风险 review 用 high；委派时显式选择。

## 执行与验证

验证细则见[开发流程与测试](docs/wiki/development/testing-and-gates.md)。要点：

- 测试断言玩家/存档可观察的功能，只做证明本次改动必需的验证；优先复用已有测试，不默认补
  边界测试，不为同一行为铺多层套件。测试数量和提交数不是进展。
- 提交粒度：每次 commit 只包含一个内聚的主题或完整功能，禁止将不相关子系统打包进 mega-commit，
  保持每次提交可二分定位（bisectable）且可独立回退（revertible）。
- 按完整优化 feature 集中实现；明显简化、去包装、删除重复实现，review 完整 diff 后可直接提交，
  不例行增加检查、审计或测试。只有行为、数据格式、共享状态或已知热点发生实质变化，确实可能
  造成回退时，才在 feature 完成后做必要的定向验证；已通过且未受影响的结果直接复用。
- 不按单次编辑或小提交重复验证，不增加多轮审查和报告。里程碑结束或发布前跑
  `cargo dev gate`。
- 日常开发与测试在 Windows、macOS、Linux 上统一走 `cargo dev` 与 `cargo worker`（`crates/bbb-xtask`）：
  Rust 用 `cargo dev test|fast-test|check|build|run`，Python 用 `cargo dev py-test|py-lint|python`，
  文档与审计用 `cargo dev docs-lint|audit-p0|anchors`。保留 `Cargo.lock`，不建 worker-local `.venv`，
  不跑全仓 `cargo fmt` 重写；target 隔离（禁止 repo-local `target/`）与禁止临时设置 `RUSTFLAGS`
  的规则见[构建与缓存](docs/wiki/development/build-and-cache.md)。
- `docs/evidence/` 只由生成脚本写，不手改。
- 仓库 owner 已接受 Mojang Minecraft EULA，并授权官方 Vanilla 26.1 differential/world-oracle harness；
  它们只作调试工具。仅通过单次命令环境传递接受，不写仓库或用户配置，不重复询问。

## 实现纪律

按[架构规则](docs/wiki/development/architecture-rules.md)组织 owner、API 与文件；同一语义只有
一个 production owner。优先现有 crate 边界和 structured packet/state type，只暴露实际需要的 API。
协议改动覆盖 decode → net event → world 应用，并以玩家可观察结果验收。
服务端新功能按[服务端新功能写法](docs/wiki/development/architecture-rules.md#服务端新功能写法)落点与实现，
worker prompt 一并引用。

同等功能下选择更少代码、更少状态、更快路径；不为假想需求加抽象、事务或兼容层。

注释只保留 Vanilla 锚点、代码说不出的 why，或最短 module purpose；不复述实现、不写迁移史。
不为改注释翻修旧文件，不引入无关 formatting 或 metadata churn。

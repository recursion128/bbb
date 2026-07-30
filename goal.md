# 目标：bbb 原生客户端与 Minecraft Java 26.1 对齐

## 总目标

让 `bbb` 的 protocol / net / world / native / pack / renderer / audio / control
边界逐步对齐 Minecraft Java 26.1 官方客户端可观察行为，形成能稳定连接、游玩、
呈现和诊断的 repo-native 客户端。以 `docs/unsupported-features.md` 为事实清单：
除有 vanilla 依据的 `not-needed` 或附明确重启判据的 `deferred` 外，其余协议、状态、
交互、资源、音画与平台差异都必须进入可验收队列，不能只收敛 renderer 表面差异。

## 本文件维护规则

- 本文件只保留：工作流、硬性约束与结构不变量、未完成项、优先级、完成标准。
- 完成史不入本文件：原子 slice 的完成记录就是 commit message 加账本/矩阵
  delta，不再写归档散文；工程 slice、foundation/milestone 关闭或工作流变更
  在 `docs/goal-archive.md` 索引追加 1～3 行。更早完成史见 git 历史；跨会话
  方法论与长期状态进 memory。
- 剩余工作必须正面列举（要做什么）；禁止“X（不含 …）”排除式定义——排除
  清单随完成度无限增长。逐项追踪型 checklist 放 `docs/unsupported-features.md`
  对应条目，本文件只指向它。
- 随 slice 变化的快照数据（audit 计数、coverage 计数等）不写入本文件；只
  记录动作与判据。
- `docs/unsupported-features.md` 是纯 TODO list，不存任何完成史；其每个一级
  TODO 域必须映射到本文件一个开放 queue、结转项或带 vanilla 依据的
  `not-needed`；新增 ledger 域时在同一 slice 更新路由。
- 本文件与开发流程本身是持续优化对象：流程摩擦在日常 slice 中随手记录，
  改进按工作流第 8 步以专门 slice 落地，不在功能 slice 中顺手改流程。
- 行数上限与完成史散文由 `scripts/docs-lint.sh` 机械 enforce。

## 硬性约束

- 严格按 vanilla 26.1 源码转写：优先使用 `~/Work/mc-code/sources/26.1/`。
- 协议、metadata 索引、render type、layer order、transform、texture path、
  tint、light、overlay 不能凭记忆实现。
- `bbb-renderer` 不得依赖 `bbb-pack`；`bbb-item-model` 不得依赖 `bbb-renderer`
  （renderer/native 共享的纯值类型放 `bbb-render-types`）。
- 每个 slice 保持单一、明确且有界的语义目标，避免顺手重构和无关格式 churn。
  “有界”约束目标、非目标、完成标准和验证方式，不限制文件数、crate 数、提交数
  或实现规模。架构重构作为专门 refactor slice 立项，不与无关 parity 目标混合；
  重构落地时同步把新约定写成下方结构不变量，能加反向断言的加反向断言。
- 触及账本域的 slice 必须在同一 slice 同步 `docs/unsupported-features.md`；
  需要长期状态记录时同步更新 memory。
- 测试必须断言语义（submission metadata / 状态化断言 / deterministic pixel
  readback）或留明确手动对比记录；不得新增只验证 vertex count 的 textured
  regression。
- 禁止手工实现可推导数据：凡存在权威来源（vanilla 源码/注册表、仓内单一
  事实源）的常量、索引 id、映射表、样板字段，一律由来源推导（宏/编译期
  查表/代码生成），或至少配机械断言锁定对齐；不得手写字面量副本任其静默
  漂移。既有机制沿用：`entity_types` 常量、`particle_type_ids!`、
  `world_counters!`；新增同类数据先建派生/断言机制，再落数据。
- 禁止同一语义多处重复实现（多 source of truth）：同一 packet 处理、行为、
  分发或投影逻辑只允许一个权威实现，其它调用路径必须委托、投影或派生，
  不得各写一份平行 arm/分支/表任其分叉（历史病灶：包分发三路重复、实体
  常量三处副本、实体模型双路径）。需要新消费点时接到权威实现上；发现
  既有多路实现时按 refactor slice 立项收敛，不得再添第 N 份副本。
- 从 vanilla Java 源码生成 catalog 时统一复用 Tree-sitter Java source
  frontend；禁止各 generator 另写易漂移的正则、tokenizer 或括号扫描器。通用
  frontend 只拥有 CST、source span 与声明/调用 IR，领域 extractor 只拥有领域
  识别/结构校验。尚未迁移的旧 generator 按独立语义 slice 逐项迁移。

## 结构不变量（新 slice 不得回退；出处见 goal-archive 索引与 memory）

- 新增 clientbound play packet 的处理只进 `bbb-world::apply_play_packet`；
  运行时副作用走 `PlayApplyEffects` trait。probe/dispatcher 不得再各写 arm。
  新增 play-packet 副作用时，无 sink 的 `PlayApplyEffects` 调用方与 native
  sink 调用方必须走同一条确定性 random/context 路径。
- 实体 type id 常量一律取自 `bbb_protocol::entity_types`；粒子 type id 常量
  一律取自 `bbb_protocol::particle_types` 的 generated catalog；均不得内联数字。
  非 Vanilla 注册表的本地粒子哨兵必须单独命名，不能冒充 catalog entry。
- server 已下发的 dynamic registry 是该 session 的唯一权威顺序/内容来源；固定
  catalog 只负责 clean/offline fallback 与 built-in registry，fallback 启用须
  记录来源。新增 Holder/HolderSet 消费点接 resolver 并在同一 slice 登记进
  `docs/registry-holder-consumer-coverage-26.1.json`；不得重建独立 resource
  数组、生产裸 holder id 或绕开 resolver 的下标查询。
- 实体热路径（spawn/apply/clone）只写窄组件；`EntityState` 投影仅限
  probe/serialize/debug 边界。
- world→renderer 每帧状态经 `runtime/render_extract.rs` 的 `RendererFrame`
  单次提交；不得在 pump 中新增散置 `renderer.set_*`。新增 world→renderer
  字段时，须在同一 slice 内对照 vanilla tick→render 帧序核对其提取时机：
  字段要么保持原位并在绑定处附 vanilla 出处，要么其 `let` 连同同一出处
  一起移到相应 tick advance 的另一侧。
- `render()` 是纯编排器：新增 pass/copy 必须做成 step 方法并登记
  `FRAME_STEPS`；meta 测试双向强制“定义顺序==执行顺序”与“step 调用数==
  登记数”。
- 新建 render pipeline 用 `pipeline_builder::RenderPipelineBuilder`；每帧
  重建的顶点/索引流用 `frame_buffers::FrameDataBuffer`，不得每帧
  `create_buffer_init`。
- item 侧全局默认表进 `bbb-world` 的 `ItemProfiles` 子 store；inventory 行为
  方法挂 `InventoryState`/`ItemProfiles`，跨域依赖经 `InventoryCtx` 显式传参
  （split-borrow）；后续子域收敛仿此模式。
- `WorldCounters` 字段只能经 `counters.rs` 的 `world_counters!` 宏声明；
  字段名与顺序由 serde 形状测试锁定。
- control 层是纯派生视图：`ControlSnapshot` 只承载 runtime counters 与发布
  generation，world 查询与 renderer 复用同 generation、同一
  `Arc<WorldStore>` 的 `SharedWorld` / simulation publication；发布由
  `SimulationCore::step` 驱动而非 rendered frame 驱动；
  控制写请求经 `SharedControlRequests` sink 进入单一 simulation mailbox。live
  `WorldStore` 只归 native 所有，不得把它或请求队列塞进 `ControlSnapshot`。
- `bbb-world` 可序列化状态路径不得引入 wall-clock（SystemTime/Instant）；
  音效 seed 走 `ClientAudioState.sound_seed_random` 确定性链。
- 大文件布局：particle_runtime / particles / item_models 为 facade + 域
  子模块 + 外置 tests.rs；新增代码进对应子模块，不得回填 facade 或新开巨型
  内联测试块。
- `docs/unsupported-features.md` 是纯 TODO list：只列还要做什么，slice 落地
  即删对应 todo，不写“已完成”。两种非待办状态：`deferred`（留在所属
  Todo 域、附 `Reopen when ...` 重启判据）与 `not-needed`（附 vanilla
  判据），删除即视为可重开，故不得删；顶层 Deferred 区域由门禁禁止，避免
  状态面板静默漏项。

## 提交前门禁

每个原子 slice 提交或工程 slice 阶段提交前必须完成：受影响 crate 的 focused tests、
`cargo fmt --all --check`、`git diff --check`、完整 diff review，以及文档有变化时的
`scripts/docs-lint.sh`。纯文档/memory slice 没有 focused test，其它检查不省略。
每次检查通过后正常提交保持工作树干净；完整门禁位置由下方 checkpoint 规则决定。

完整门禁统一入口 `scripts/cargo-dev.sh gate`，等价于：

- `scripts/docs-lint.sh`
- `cargo fmt --all --check`
- `scripts/vanilla-source-tools.sh python -m unittest discover -s scripts/tests`
  （仅当被门禁的改动触及 `scripts/`；`BBB_GATE_FULL=1` 强制全跑）
- `git diff --check`
- `CARGO_TARGET_DIR=/tmp/bbb-target-main cargo check --workspace`
- `CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test --workspace`

deny-warnings 由根 Cargo.toml 的 `[workspace.lints.rust] warnings = "deny"`
承担；禁止手工设置 `RUSTFLAGS='-D warnings'`——它会双向作废
`/tmp/bbb-target-*` 缓存，代价是整仓重编（见 `docs/cargo-build-performance.md`）。
`cargo check` 一条不可省：只跑 `cargo test` 会漏掉非测试（交付 bin）构建的
unused/dead-code lint。

## Slice 工作流（每轮）

选片 auto-start 是默认分支，只有新的 P0 blocker 能抢占；P0 clean 时不得停下
等待确认，也不因历史 ledger 文字重开 P0。

### Slice 形态与 ready 契约

- 同一时间只推进一个语义目标。**原子 slice** 是适合一次正常提交的小型内聚
  闭环；**工程 slice** 是跨 crate/所有权边界、需要多个串行阶段和提交的单一
  语义工程。两者平等参与 hardest-ready-first；规模大本身不是 blocker，也
  不得把完整工程无理由切碎成低价值微提交。
- ready 条件：单一明确语义目标；明确范围、非目标与最终完成标准；真实前置
  已满足；跨边界集成契约能在开工前冻结；可分阶段验证、提交、定位失败。
  blocked/deferred、目标持续扩张、无法定义完成标准或混合无关功能的候选排除。
- 工程 slice 开工前记录：目标与完成标准、非目标、涉及 crate/边界、vanilla
  或 wire 依据、串行阶段与依赖顺序、每阶段 focused tests、checkpoint /
  full-gate 位置、失败定位与安全停止方式。选中后持续推进到完成、明确
  blocker 或预定义安全停止点，不穿插无关功能。

### 每轮步骤

1. **收尾**：确认工作树干净；已验证未提交的 slice 先提交。
2. **audit**：`scripts/audit-p0.sh`；exit 0 且 `.git/bbb/` 的 last-gated 与
   last-playable-* stamp（gate/playable-gate 通过时自动写入）无回退迹象 ⇒ 无新
   P0；milestone 关闭后 live playable stamp 不得落后于该 milestone。exit 1 时
   人工归类新增命中，有明确非 P0 依据的附注释写入 allowlist。不与历史计数对比。
3. **P0 分支**：命中重开条件 → P0 regression slice 只修 blocker；未命中 →
   立即进入候选选择，不等待确认。
4. **Hardest-ready 选择**：先跑 `scripts/coverage-status.py` 取 active TODO
   item 面板（显式 `deferred` / `not-needed` 与 invariant 不进入候选），与
   P1～P4 路线图开放项合并建候选集（原子与工程形态并列），在完整语义工程
   层面按五项评估难度：架构/协议/状态机风险、跨边界集成范围、vanilla/wire
   不确定性、验证复杂度、失败影响与定位难度。默认选最难的 ready 工程；只有真实 blocker 或集成契约无法冻结
   时，才选择能解锁它的最难且 ready 的有界前置 slice。难度相当时依次看：
   解锁更多后续目标、更接近可玩交付闭环、下节裁决顺序。进度更新只需简记
   最终选择与一句难度依据、被排除的最强候选及其真实 blocker。
5. **实现循环**（原子 slice 或工程阶段）：
   1. 先 grep 当前实现，确认缺口真实存在（历史上多次“账本以为缺失但代码
      已实现”）；
   2. 定位 vanilla 26.1 源码依据（常量/公式/顺序）；已批量提取过的先查
      memory `vanilla-feature-quickref`；
   3. 实现 + focused test；随机行为用确定性 seed 或固定样本；
   4. 更新账本/矩阵条目；仅工程或 milestone 关闭时在 `docs/goal-archive.md`
      索引追加 1～3 行，原子 slice 不写归档；
   5. 运行提交前检查，审查 diff 后正常提交（英文祈使句单行 message）；阶段
      提交必须可构建、可审查，失败一律 fix-forward，不重写历史。
   工程 slice 的进度更新报告当前阶段、已完成提交、focused test 结果和下一
   checkpoint。
6. **Checkpoint 批次与门禁位置**：
   - 普通批次：连续 2～4 个语义相关、风险可局部界定的低风险原子 slice，
     各自完成提交前检查与干净提交，批次 tip 跑一次
     `scripts/cargo-dev.sh gate`。累计第 4 个提交、切换语义领域，或交付/
     停止/push/merge 前，必须结束批次并 gate（提前结束的批次可以只有
     1 个提交）。
   - 同族机械批次：连续 ≤5 个同一派生模板的低风险机械 slice，每个只过
     docs-lint+fmt+focused+diff-check 即提交，批次结束跑一次完整 gate。
   - 高风险（协议编解码、Cargo/workspace 配置、构建脚本、生成器、`unsafe`、
     并发、核心架构）：开始前先 gate 并关闭已有批次；原子高风险 slice
     一 slice 一提交一次完整 gate；高风险工程按预定义里程碑推进，在里程碑、
     累计 4 个提交、跨越新协议/架构状态、准备停止或交付时 gate，最终 HEAD
     必须通过。focused test 异常时立即完整 gate。
   - gate 失败：先按失败项与 focused tests 定位；仍无法确定引入提交时才用
     `scripts/cargo-dev.sh batch-gate <last-gated>..HEAD`（它只是失败定位
     工具）。fix-forward 修复后重新 gate tip；通过前不得开始下一批或交付。
   - 最近通过完整 gate 的 commit 由 gate 自动记录于 `.git/bbb/last-gated`
     （gate-log.tsv 含各步耗时），batch-gate 无参数时默认以它为 `<good>`；
     进度更新与交付报告批次内提交、focused test 结果与 gate 结果。交付/合并
     的 HEAD 必须已通过完整 gate。
7. **路线图维护**：P1～P4 记录依赖、里程碑、完成标准与裁决顺序，不是默认
   串行消化队列。新发现工作在账本立条目并参与下一轮候选；结转项只在重启
   判据满足后恢复为 ready 候选。
8. **流程维护（持续优化 goal.md 与开发流程）**：
   - 触发判据（日常 slice 中观察到即记录，不当场改流程）：每 slice 固定
     开销上升（gate 步时、文档/叙述负担）；规则被误读、互相矛盾或反复
     解释；同类失误重复出现；文档超限、指向失效或积累完成史；出现能缩短
     闭环的新机制（脚本化、生成、批量门禁、subagent 分工）。
   - 落地方式：积累到值得动手时立专门的工作流 slice（触及 scripts/ 按
     高风险门禁）。改规则必须同 slice 改 enforce 机制（docs-lint / gate /
     脚本）与 AGENTS.md 指针，删除被取代的条文，不留第二份表述。
   - 验收判据可测：门禁耗时（以 `.git/bbb/gate-log.tsv` 为准）、goal.md 行数、
     每 slice 固定步骤数只许下降或持平；新增或收紧规则须附真实事故依据。
     每次流程变更按 vN 版本号记入 `docs/goal-archive.md` 索引与 memory。

## P0 重新打开条件与 audit 分类

只有出现以下任一情况才开 P0 slice（普通视觉精度、长尾功能缺口和性能优化不算）：

- vanilla 26.1 合法 offline-mode session 无法完成 login/configuration/play handoff，
  或既有 playable smoke 在连接、spawn、chunk、movement、inventory、interaction、
  respawn/reconnect 任一基线阶段回退。
- packet state/field/wire order 错误导致流失步，合法 packet 触发 panic/hang/无界分配，
  或 required serverbound command 静默未发送、错误排序、错误确认。
- server-synced registry holder 被解析成另一个 resource、旧 session registry 泄漏到
  新连接，或 canonical world 的 entity/chunk/inventory/local-player 状态发生可复现
  损坏、跨 dimension/session 残留。
- 来自服务器或资源包的不可信 packet/NBT/JSON/PNG/ZIP 能造成路径逃逸、解压炸弹、
  无界递归/分配、进程崩溃，或绕开既有 size/depth/count 限制。
- world→runtime side effect 或用户输入/交互命令在既有支持路径静默丢失、重复执行，
  或 probe/online dispatcher 对同一 canonical 行为重新分叉。

- 新的 texture-backed direct mesh bypass / residual emit 路径
  （`render_textured_submission` / dynamic texture submission helpers 之外
  出现 texture-backed mesh 写入入口）。
- texture-backed / dispatch-owned submission 路径出现主要遗漏。
- submission metadata（texture、render type、tint、transform、light、
  overlay、outline color、`order`、`submit_sequence`）任一缺失，或
  missing-atlas / pending dynamic texture 丢失 submission-first 记录。
- RenderType 语义合并或退化（`entityCutout` / `entityCutoutCull` /
  `entityCutoutZOffset` / `entityTranslucent` / `Eyes` / `EnergySwirl` 等）。
- 新 layer/renderer 继承错误 light/overlay，或 invisible、self-visible
  translucent、hidden glowing outline 分支回退到普通可见路径。

已知非 P0 分类基准见 `scripts/data/p0-audit-allowlist.txt`；分类变化时更新该文件并附依据。

P0 完成标准：触发回退有最小复现和对应 semantic/integration regression test；修复后
完整 gate 与适用的 playable/security smoke 通过。renderer 问题断言完整 submission
metadata；协议/world/runtime 问题断言状态转换、ordering、canonical state 与 side
effect，不以“未崩溃”代替语义验证。狭义 P0 只按本节跨域重开条件进入队列；开放式
视觉 parity 留在对应优先级。

## 客户端差异优先级

本节只列开放路线图。P1～P4 保留为依赖/里程碑说明与 hardest-ready 难度相当时的
裁决依据，不要求按章节顺序串行消化。protocol/session、RegistryAccess typed
foundation、entity matrix、playable session lifecycle 四个
foundation milestone 均已闭环（见 goal-archive 索引；RegistryAccess 消费长尾
整队列随之闭环，权威来源规则转入上方结构不变量维护），后续裁决顺序调整为：

1. P2 非实体 closeout；
2. P2 屏幕/HUD；
3. P3 资源；
4. P4 健壮性/性能/平台。

（playable 基础游玩交互与扩展失败证据两阶段均已闭环，见下节；playable 门禁
转入维护态：新交互场景随对应垂直 slice 扩展，不再单列队列。）

关闭 milestone 后照常运行适用的 playable-gate；暴露的依赖进入下一轮候选，
不得借机扩大当前 slice。

### 结转项（blocked / defer，不进 ready 候选集）

重启判据见对应账本条目；条件满足前不列入候选集，也不因 ledger 文字重开。

- entity-in-UI preview 的 `item_layers` GPU 绘制（原 P1-4）：defer，需 native
  侧 baked hand/head item model 元数据。
- font unihex（原 P1-3 font 五连剩余）：defer，需 consumed assets 提供
  unifont 源。

### P1：可玩 Session 端到端门禁

用真实 vanilla 26.1 offline-mode server 和可复现 fixture 验证跨 crate 集成，
不让 focused unit tests 掩盖 protocol→net→world→native→renderer/audio 的连接级
断裂。命令入口 `scripts/playable-gate.sh`；lifecycle 工程合同见
`docs/playable-session-lifecycle-design.md`。session lifecycle（death/respawn、
Overworld↔Nether、resource-pack push/pop、Play→Configuration→Play、双 server
Transfer）与基础游玩交互（movement、破坏/放置、实体攻击、容器
开箱/quick-move/关箱、chat round-trip、hotbar 切槽、声音/粒子推送、
deterministic screenshot、inventory 背包操作）均已闭环。失败证据同样闭环
（evidence schema 10）：任何失败按 server_start / network / canonical_apply /
runtime_side_effect / presentation 五档分类并标注失败场景，附失败时刻同代
renderer/audio/net/world counters、pending 写探针双采样和最终 screenshot
（control 不可达时逐级降级，不掩盖原始失败）。阶段队列已清空；新交互场景随
对应垂直 slice 扩展本门禁，不再单列。

持续架构优化（随 slice 顺带落地，不单独排队）：rust 类型状态/newtype 收紧
协议状态机非法空间、`Arc` 共享替代克隆；tokio 连接/控制任务零阻塞 await、
`watch`/arc-swap 快照、JoinSet 任务生命周期；hecs 把 per-tick 实体动画/物理
收敛为 ECS 查询批处理，作为 dual-path 实体迁移统一落点。线上语义 vanilla
一致（P0 红线），focused tests + live gate 为回归判据。

完成标准：fixture smoke 可在 clean checkout 标准测试环境运行；live
`playable-gate` 一条命令完成上述场景并稳定回收；合法 session 无 panic、hang、
required unknown packet 或状态泄漏；每个核心用户动作同时有 server 结果、world
状态和适用呈现断言；大 milestone 关闭前必须通过该门禁。

### P1：消除可机械推导的 vanilla 手工维护（收尾审计）

生成器基建与主体 catalog 已落地。生成器只在维护时读取
`~/Work/mc-code/sources/26.1/` 或明确输入；生成物必须提交，普通 build/test
不读取工作区外源码。开放审计方向：

1. **资源与 tag 派生表**：继续审计手写 block/item/entity membership；有精确
   vanilla tag 的直接从 tag 派生，没有的只在 class/property/registry source
   fact 能证明等价时生成，禁止名称启发式或近似 tag 扩大集合。block terrain
   render layer 由 baked material/sprite 实际 transparency 动态决定，缺失
   贴图用明确 opaque missing-sprite fallback。
2. **跨 crate 去重与反向门禁**：审计残余重复 registry/ordinal/UI 静态
   membership 表和生产裸 id；消费者改用 typed catalog API，并加机械扫描或
   结构测试阻止第二事实源重现。生成物内的反向门禁必须由同一 generator
   模板输出，禁止手改生成物。

完成标准：修改生成输入而未更新生成物会在门禁失败；clean checkout 无本地
vanilla 源码也能构建测试；版本升级时注册顺序变化表现为集中可审查的生成
diff；生产代码不再出现已纳入 catalog 的平行资源表、裸 registry id 或手抄
ordinal。

### P2：非实体客户端行为 closeout

给 entity matrix、screen/HUD 和 resource 队列之外的客户端行为一个明确关闭
路径，覆盖账本中 protocol 已解码但尚未完整呈现的 level/chunk/block entity、
input/movement/interaction、inventory/gameplay、particle/level event、audio
与跨域 side effect。按用户可观察 vertical slice 推进，不按 crate 各做半条链。

1. **ledger 路由**：维护 unsupported 一级域→goal queue 的双向清单；已由其它
   队列负责的条目只引用；其余进入 world、interaction、effects 三个 owner，
   新增 orphan heading 令结构测试失败。
2. **world 与 block entity**：补齐 dimension/time/weather/chunk/light/biome/
   block state、block event/entity、map/scoreboard/recipe/advancement 等
   canonical state 与可观察呈现，重点关闭 sign/banner/chest/shulker/spawner/
   bell/vault 等窄缺口和清理生命周期。
3. **input、movement 与 interaction**：覆盖 local player movement/collision/
   fluid/portal、crosshair/pick、attack/use/destroy/place、vehicle、
   inventory/container prediction 与 server acknowledgement；服务端权威结果
   不伪造，本地 prediction 与 rollback 有明确边界。
4. **particle、level event 与 audio**：从 packet/world event 经唯一 effects
   sink 到 resolver/runtime/render submission，覆盖 provider options、
   block/item context、entity following、category/attenuation、delayed/stop
   与 missing asset diagnostics；随机链保持 online/probe/fixture 确定性一致。
5. **垂直关闭门禁**：slice 完成时同时断言 protocol summary、canonical
   state、runtime command/counter 与适用 screenshot/audio resolver 结果；只
   完成 decode、只加 counter 或只画 fallback 不得删除账本条目。

完成标准：账本每个非实体条目都有唯一 queue owner；目标客户端适用条目完成或
仅剩有依据的 `not-applicable`/`deferred`；playable-gate 覆盖基础 movement/
interaction/inventory/world effects；不存在已 decode 但任一段无主的 required
packet 行为。

### P2：屏幕、HUD、字体与截图

开放项见账本 "HUD overlay and screen render surfaces" 节；整体低优先；
F3+I entity save payload 由 entity matrix 唯一拥有。唱片 now-playing 工程
的活跃合同见 `docs/jukebox-now-playing-design.md`。

完成标准：不做临时配置 UI，启动配置从命令行进入；上述开放项及账本归属本
队列的条目全部完成或有窄 deferred/not-needed 依据；视觉结果有 deterministic
screenshot/pixel readback，无法自动化的留可复现 vanilla 对比步骤与截图证据。

### P3：动态资源 presentation breadth

selected pack、server push/pop、原子 reload generation 和 profile/skin/cape/
elytra 动态纹理 owner 已落地；owner/失效合同以
`docs/resource-pack-lifecycle-design.md` 为准；coalesced ResourceTexture
per-entry sampler 工程合同见
`docs/archive/coalesced-resource-texture-sampling-design.md`。剩余工作不得
重建平行 pack stack、cache 状态机或 selectable old/new reload path。

1. **non-profile 动态纹理**：后续真实远端 producer 复用既有 bounded
   pending/ready/failed owner。
2. **同步 registry 到 custom presentation**：server/datapack 的 custom
   block/entity identity 接现有 model/texture loader 与 exact fallback；不用
   bootstrap ordinal、名称猜测或第二套 schema 代替 session RegistryAccess。
3. **诊断与集成证据**：missing/invalid/unsupported resource 有精确
   resource-id/fallback 观测；live playable 增加 deterministic
   screenshot/audio 变化证明 presentation 随 push→apply→pop 切换。

完成标准：新增 consumer 全部加入同一 immutable resource generation 和有界
动态纹理 surface；无 stale handle 或跨 session id reuse；custom presentation
有 semantic test，服务器 pack 的可见/可听变化有 deterministic live 证据。

### P4：健壮性、运行时性能与平台交付

目标客户端必须能长期连接不可信服务器、加载不可信资源并在支持平台稳定运行。
优化不得改变 canonical 语义或删 diagnostics/fallback 换指标；性能基线只用
可复现场景和稳定统计。simulation/render 性能工程的开放边界与对比合同以
`docs/simulation-render-performance-design.md` 为准；具体缺口在账本 runtime
performance 条目维护。

1. **输入与资源安全**：packet frame/compression、NBT/component、JSON/model/
   font、image/audio/ZIP 的 size/depth/count/ratio/time 限制、路径规范化与
   fuzz/property tests；malformed input 返回有上下文错误，不 panic、越界或
   预分配攻击者声明的容量。
2. **队列、异步与生命周期**：net event、chunk/mesh upload、resource
   download/decode、profile/texture、audio/particle 的 backpressure、cancel、
   generation 与 shutdown；disconnect/reload/window close 能终止 worker。
3. **性能预算**：代表性 view distance 场景的 frame CPU/GPU、chunk decode/
   mesh/upload、entity extraction/render、memory/cache 与 network baseline；
   优化走 profiling 证据，设宽容 regression budget；构建性能遵循
   `docs/cargo-build-performance.md`。
4. **soak 与故障恢复**：长时间 movement/chunk churn/entity/particle/resource
   reload soak，覆盖断网、kick、音频设备失败、resize/minimize、GPU surface
   丢失与 clean shutdown；检查内存/handle/queue 单调增长与 stale session。
5. **平台与发布面**：明确支持的 OS/GPU/backend/窗口输入矩阵，验证 DPI/
   fullscreen/focus、keyboard/mouse/clipboard/IME、无音频设备和 headless
   probe；启动配置只走 CLI，提供版本/协议/pack-format、日志、诊断快照。
6. **分层门禁**：快速 workspace gate 保持现有内容；fuzz corpus、benchmark、
   live playable 与 soak 进入可单独执行的 milestone/CI gate，失败产物可归档
   且命令、seed、fixture 可复现。

完成标准：合法 session/resource 不因防御限制误拒绝，恶意输入在明确预算内
失败；代表性场景无无界增长，性能回退可量化定位；故障不污染下一 session；
声明支持的平台通过 clean build、fixture playable smoke 和启动/关闭生命周期
测试。

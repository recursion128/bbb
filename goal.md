# 目标：bbb 原生客户端与 Minecraft Java 26.1 对齐

## 总目标

让 `bbb` 的 protocol / net / world / native / pack / renderer / audio / control
边界逐步对齐 Minecraft Java 26.1 官方客户端可观察行为，形成能稳定连接、游玩、
呈现和诊断的 repo-native 客户端。以 `docs/unsupported-features.md` 为事实清单：
除有 vanilla 依据的 `not-needed` 或附明确重启判据的 `deferred` 外，其余协议、状态、
交互、资源、音画与平台差异都必须进入可验收队列，不能只收敛 renderer 表面差异。

这里的完成范围是**外部多人原生客户端**：用户提供启动参数与可用的 session/token
材料后，客户端连接外部服务器并完成上述行为。Vanilla 26.1 `Main` 将
`--accessToken` 作为必需参数并用它构造 `User`，所以 Microsoft/Mojang token
获取不属于客户端入口责任；依赖 `Minecraft.getSingleplayerServer()` /
`IntegratedServer` 的单人世界和 server-side debug 也不属于外部客户端。
`WorldSessionTelemetryManager` 通过独立 event sender 发送 telemetry，不是多人
协议 canonical state。Vanilla `OptionsScreen.removed` 确实调用 `options.save()`，
但游戏内配置 UI 是本项目明确 non-goal，而不是“Vanilla 没有该行为”。这些
non-goals 不因进入维护态而自动转成 TODO。

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
- `docs/unsupported-features.md` 是 active/deferred/not-needed 事实账本，不存
  任何完成史；每个 active 项必须映射到下方一个 `active` 路线，deferred 项只在
  `Reopen when ...` 满足后恢复为 active，not-needed 项必须保留 vanilla 依据。
  新增 ledger 域时在同一 slice 更新路由状态。
- 本文件与开发流程本身是持续优化对象：流程摩擦在日常 slice 中随手记录，
  改进按工作流第 9 步以专门 slice 落地，不在功能 slice 中顺手改流程。
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
- `docs/unsupported-features.md` 是无完成史的事实账本：active 项落地即删，
  不写“已完成”。两种保留决策：`deferred`（留在所属 Todo 域、附
  `Reopen when ...` 重启判据）与 `not-needed`（附 vanilla 判据），删除即视为
  可重开，故不得删；顶层 Deferred 区域及 Not needed 下的 deferred 条目由门禁
  禁止，避免状态面板静默漏项。

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

有 ready 工作时选片 auto-start 是默认分支，只有新的 P0 blocker 能抢占；P0
clean 时不等待确认，也不因历史 ledger 文字重开 P0。无 ready 工作时必须走下方
机械维护态分支，不能用临时 grep 或宽泛路线愿景制造候选。

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
2. **audit**：运行 `scripts/audit-p0.sh` 与 `scripts/coverage-status.py`；exit 0
   且 `.git/bbb/` 的 last-gated 与 last-playable-* stamp（gate/playable-gate
   通过时自动写入）无回退迹象 ⇒ 无新 P0。milestone 关闭后 live playable stamp
   不得落后于该 milestone。audit exit 1 时人工归类新增命中，有明确非 P0 依据的
   附注释写入 allowlist；不与历史计数对比。
3. **P0 分支**：命中重开条件 → P0 regression slice 只修 blocker；未命中 →
   立即审计路线状态与 deferred 重启条件，不等待确认。
4. **ready / 静默态裁决**：
   - 只把 coverage 面板的 active item 与已满足 `Reopen when ...`、并在同一
     slice 先改回 active 的 deferred 放进候选集。P1～P4 只声明 owner/status，
     不是第二份待办队列。
   - active 为 0 时逐项确认：P0 clean；每条路线为 `maintenance`/`closed`，或
     只剩重启条件未满足的 `deferred`；`HEAD` 等于 `.git/bbb/last-gated`；适用的
     fixture/live playable stamp 是 `HEAD` 祖先且没有新 milestone 要求重跑。
   - 上述条件全部满足 ⇒ 输出 P0、coverage、路线状态、full-gate commit 与
     playable stamp 证据，进入“目标维护态 / declared multiplayer-client 当前
     范围完成”并停止。不得从手工 grep、宽泛 P4 愿景或持续审计口号临时制造
     无界微型 slice。
   - 任一 P4 目标仍真实未关闭却没有账本项 ⇒ 先把它具体化为少量、带 owner、
     验收判据与适用重启条件的 active/deferred 项；在此之前不得进入完成态。
5. **Hardest-ready 选择**：对候选集中的原子/工程形态，在完整语义工程层面按
   五项评估难度：架构/协议/状态机风险、跨边界集成范围、vanilla/wire 不确定性、
   验证复杂度、失败影响与定位难度。默认选最难的 ready 工程；只有真实 blocker
   或集成契约无法冻结时，才选择能解锁它的最难且 ready 的有界前置 slice。
   难度相当时依次看：解锁更多后续目标、更接近可玩交付闭环、下节路线 owner。
   进度更新只需简记最终选择与一句难度依据、被排除的最强候选及其真实 blocker。
6. **实现循环**（原子 slice 或工程阶段）：
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
7. **Checkpoint 批次与门禁位置**：
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
8. **路线图维护**：P1～P4 记录 owner、状态、完成/维护证据，不是默认串行
   消化队列。新发现工作先在账本立 active 项并把对应路线改为 `active`；结转项
   只在重启判据满足、账本状态更新后恢复为 ready。路线与账本状态由
   `scripts/docs-lint.sh` 交叉检查。
9. **流程维护（持续优化 goal.md 与开发流程）**：
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

路线状态是机械索引，不是独立任务源；具体 ready/deferred/not-needed 事实只在
`docs/unsupported-features.md`。`scripts/docs-lint.sh` 要求下列 marker 完整，并在
active TODO 为 0 时拒绝仍声明 active 路线。

<!-- roadmap-status:start -->
- p1-playable: maintenance
- p1-generated-facts: maintenance
- p2-non-entity: active
- p2-hud: maintenance
- p3-resources: maintenance
- p4-delivery: maintenance
<!-- roadmap-status:end -->

有 active 项时仍按 hardest-ready-first；难度相当才按 P1→P2→P3→P4 owner 顺序
裁决。关闭 milestone 后照常运行适用的 playable-gate，暴露的真实缺口先入账本，
不得借机扩大当前 slice。

### P1：可玩 Session 端到端门禁

状态：`maintenance`。fixture/live 命令、session lifecycle、基础交互与分层失败
证据由 `scripts/playable-gate.sh` 和
`docs/playable-session-lifecycle-design.md` 持续维护；archive 的 lifecycle、
interaction 与 failure-evidence milestone 是关闭依据。新垂直 slice 若改变可玩
行为，必须扩展适用场景并按 checkpoint 规则刷新 stamp；门禁回退按 P0 处理。

### P1：可机械推导的 vanilla 事实

状态：`maintenance`。生成物提交、普通 build/test 不读取工作区外源码、共享
Tree-sitter frontend 与反向 source guard 是持续不变量。只有以下触发之一出现时
才开有界 slice：结构门禁命中具体第二事实源；目标版本升级产生权威 source diff；
生成输入/模板变化；或代码 review 指出一个可定位的重复 registry/ordinal/tag/
membership owner。没有具体文件、事实源与验收测试的手工 grep 不能在 active=0
时生成审计 slice。

### P2：非实体客户端行为 closeout

状态：`active`。packet/registry/item/entity coverage 均无 open row，但
unsupported 账本仍有相机流体/fog 与 trial-spawner display 两个 ready 工程；
具体合同只由账本维护并按 hardest-ready-first 串行推进。
其余 world/block-entity、movement/interaction、particle/level-event、audio 与跨域
side-effect 的关闭证据见 archive 对应 milestone 及 playable gate。vertical slice
仍须贯通 protocol summary、canonical state、runtime effect/counter 与适用
presentation。

### P2：屏幕、HUD、字体与截图

状态：`maintenance`。账本对应域没有 active item；screen/HUD/font/debug 与
deterministic screenshot/pixel evidence 的关闭索引见 archive。
`context_entity_type` 与 unihex 仍按各自 `Reopen when ...` 留在 Todo 域但不进
ready。启动配置继续只走 CLI，F3+I entity payload 继续由 entity owner 唯一负责。

### P3：动态资源 presentation breadth

状态：`maintenance`。selected/startup/server pack、push/pop、原子 immutable
resource generation、profile/map/variant/painting/pattern/trim 动态资源与诊断
owner 已由 `docs/resource-pack-lifecycle-design.md`、资源预算合同、archive 和
playable 证据覆盖；剩余 non-profile producer 属于当前不存在且未声明进入范围的
server/world-list 与 pack-selection screen，按账本保留为 `not-needed`。

- 固定内置 block/entity identity 继续使用现有资源包模型、纹理和表现，并进入同一
  immutable resource generation；`BuiltInRegistries.BLOCK` / `ENTITY_TYPE`
  是客户端内建 registry。
- 只有 Vanilla 26.1 实际通过
  `RegistryDataLoader.SYNCHRONIZED_REGISTRIES` 同步、且有客户端 presentation
  consumer 的 registry 才支持自定义条目；
  `RegistryDataCollector.loadNewElementsAndTags` 将该精确列表传给网络 registry
  loader。该集合不含 `BLOCK` 或 `ENTITY_TYPE`，账本将 synchronized custom
  block/entity identity 保留为 `not-needed`。例如 `WolfVariant.NETWORK_CODEC`
  的 `ClientAsset.ResourceTexture` 与 `PigVariant.NETWORK_CODEC` 的
  model/texture payload 属于允许的 presentation 数据。
- 新 consumer 必须复用现有 generation、RegistryAccess resolver、bounded
  pending/ready/failed owner 与 exact fallback；不得新增第二套 registry、
  schema、pack stack 或 cache owner。

当前无 ready P3 工作；只有声明新 screen surface、目标版本产生新 producer 或发现
当前表面的具体 active producer 时才重开。

### P4：健壮性、运行时性能与平台交付

状态：`maintenance`。账本 P4 域没有 active item；安全预算、bounded runtime、
lifecycle/fault soak 与已声明平台行均有有界 gate，剩余官方 Java 同载对照、
扩展 Linux 交互矩阵和非 Linux 发布行只按各自 `Reopen when ...` deferred。

已有维护证据：

- packet/NBT/component 与 resource JSON/model/font/image/audio 的不可信输入预算、
  原子回滚和诊断由协议测试、`docs/resource-input-safety-design.md`、
  `docs/resource-data-audio-budget-design.md` 及 full gate 维护；
- simulation/net/resource/mesh/audio owner 的 bounded queue、generation、
  shutdown/join 与 panic propagation 由
  `docs/simulation-render-thread-design.md` 和 focused tests 维护；
- repo-native before/after 与 runtime observation 已有稳定基线；只有同载官方
  Java 对比按账本条件 deferred，不能伪造胜负结论；
- `scripts/lifecycle-soak-gate.sh` 依
  `docs/lifecycle-soak-gate-design.md` 组合三轮真实 live playable、
  quit/kick、feature-gated surface/window/no-audio 故障、终态 gauge、进程
  指标趋势与 clean exit；`last-lifecycle-soak`/归档 report 是 maintenance
  证据，fixture 重放不能替代该 gate。
- `scripts/platform-acceptance-gate.sh` 依
  `docs/linux-platform-acceptance.md` 声明并验收 Linux x86_64 / Wayland /
  Vulkan / windowed external-multiplayer 行：fresh locked release build、
  build-info、headless probe、完整 live/quit、实际 backend 日志、输入语义
  focused receipts 与 lifecycle-soak 证据统一写入 report/stamp。

维护触发只来自上述 gate/stamp 回退、已声明行/构建配置/runtime owner 变化，或
账本 deferred 重启条件满足；不能由宽泛 soak、性能、平台愿景重新生成 ready
工作。active=0 时走工作流第 4 步的机械终止判据。

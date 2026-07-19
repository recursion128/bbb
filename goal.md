# 目标：bbb 原生客户端与 Minecraft Java 26.1 对齐

## 总目标

让 `bbb` 的 protocol / net / world / native / pack / renderer / audio / control
边界逐步对齐 Minecraft Java 26.1 官方客户端可观察行为，形成能稳定连接、游玩、
呈现和诊断的 repo-native 客户端。以 `docs/unsupported-features.md` 为事实清单：
除有 vanilla 依据的 `not-needed` 或附明确重启判据的 `deferred` 外，其余协议、状态、
交互、资源、音画与平台差异都必须进入可验收队列，不能只收敛 renderer 表面差异。

## 本文件维护规则

- 本文件只保留：工作流、硬性约束与结构不变量、未完成项、优先级、完成标准。
- 已完成的 slice 描述在提交当轮迁入 `docs/goal-archive.md` 对应小节；
  "仍在推进"下只允许可验证的未完成项，不堆已完成史。
- 剩余工作必须正面列举（要做什么）；禁止"X（不含 …）"排除式定义——排除
  清单随完成度无限增长。逐项追踪型 checklist 放 `docs/unsupported-features.md`
  对应条目，本文件只指向它。
- 随 slice 变化的快照数据（audit 计数等）不写入本文件；只记录动作与判据，
  不记录当时的数字。
- 已完成内容及时归档，避免用完成史淹没开放队列。历史归属：
  `docs/goal-archive.md`（已完成目标）、git（更早的完成史）、memory（跨会话
  方法论与长期状态）。`docs/unsupported-features.md` 是纯 TODO list，不存任何
  完成史；coverage matrix 与长期执行契约可放独立 checked-in 文档，本文件保留其
  优先级、当前入口与完成标准。
- `docs/unsupported-features.md` 的每个一级 TODO 域必须能映射到本文件一个开放
  queue、结转项或带 vanilla 依据的 `not-needed`；禁止出现没有优先级和关闭路径的
  orphan TODO。新增 ledger 域时在同一 slice 更新路由，结构测试锁定双向覆盖。
- 本节规则由 `scripts/docs-lint.sh` 机械 enforce。

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
- 每个 slice 必须更新 `docs/unsupported-features.md`；需要长期状态记录时
  同步更新 memory。
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
- 后续从 vanilla Java 源码生成 catalog 时统一复用 Tree-sitter Java source
  frontend；禁止各 generator 另写易漂移的正则、tokenizer 或括号扫描器。通用
  frontend 只拥有 CST、source span 与声明/调用 IR，领域 extractor 只拥有领域
  识别/结构校验，typed/virtual 归一化留给对应 generator/semantic resolver。既有
  generator 按独立语义 slice 逐项迁移，不在新消费者中复制旧解析路径。

## 结构不变量（新 slice 不得回退；出处与日期见 goal-archive 与 memory）

- 新增 clientbound play packet 的处理只进 `bbb-world::apply_play_packet`；
  运行时副作用走 `PlayApplyEffects` trait。probe/dispatcher 不得再各写 arm。
  新增 play-packet 副作用时，无 sink 的 `PlayApplyEffects` 调用方与 native
  sink 调用方必须走同一条确定性 random/context 路径。
- 实体 type id 常量一律取自 `bbb_protocol::entity_types`；粒子 type id 常量
  一律取自 `bbb_protocol::particle_types` 的 generated catalog；均不得内联数字。
  非 Vanilla 注册表的本地粒子哨兵必须单独命名，不能冒充 catalog entry。
- 实体热路径（spawn/apply/clone）只写窄组件；`EntityState` 投影仅限
  probe/serialize/debug 边界。
- world→renderer 每帧状态经 `runtime/render_extract.rs` 的 `RendererFrame`
  单次提交；不得在 pump 中新增散置 `renderer.set_*`。新增 world→renderer
  字段时，须在同一 slice 内对照 vanilla tick→render 帧序核对其提取时机：
  字段要么保持原位并在绑定处附 vanilla 出处，要么其 `let` 连同同一出处
  一起移到相应 tick advance 的另一侧。
- `render()` 是纯编排器：新增 pass/copy 必须做成 step 方法并登记
  `FRAME_STEPS`；meta 测试双向强制"定义顺序==执行顺序"与"step 调用数==
  登记数"。
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
  即删对应 todo，不写"已完成"。两种非待办状态：`deferred`（附重启判据）与
  `not-needed`（附 vanilla 判据），删除即视为可重开，故不得删。不再有溢出
  拆分规则。

## 提交前门禁

每个原子 slice 提交或工程 slice 阶段提交前必须完成：受影响 crate 的 focused tests、
`cargo fmt --all --check`、`git diff --check`、完整 diff review，以及文档有变化时的
`scripts/docs-lint.sh`。纯文档/memory slice 没有受影响 crate 的 focused test，但其它
检查不省略。每次检查通过后以正常提交保持工作树干净；低风险相关原子 slice 是否立即
运行完整门禁，以及工程 slice 在何处设置里程碑门禁，由下方 checkpoint 规则决定。

完整门禁统一入口 `scripts/cargo-dev.sh gate`，等价于：

- `scripts/docs-lint.sh`
- `cargo fmt --all --check`
- `scripts/vanilla-source-tools.sh python -m unittest discover -s scripts/tests`
  （仅当被门禁的改动触及 `scripts/`；`BBB_GATE_FULL=1` 强制全跑）
- `git diff --check`
- `CARGO_TARGET_DIR=/tmp/bbb-target-main cargo check --workspace`
- `CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test --workspace`

deny-warnings 自 2026-07-16 起由根 Cargo.toml 的 `[workspace.lints.rust]
warnings = "deny"` 承担，gate 与日常 focused test 共享同一构建指纹；禁止再
手工设置 `RUSTFLAGS='-D warnings'`——它会双向作废 `/tmp/bbb-target-*` 缓存，
每次切换代价是整仓重编（实测见 `docs/cargo-build-performance.md`）。
`cargo check` 一条不可省：test 构建把 test-only 用法算作使用，只跑
`cargo test` 会漏掉非测试（交付 bin）构建的 unused/dead-code lint。

## Slice 工作流（每轮）

选片 auto-start 是默认分支，只有新的 P0 blocker 能抢占；P0 clean 时不得停下
等待确认，也不因历史 ledger 文字重开 P0。

### Slice 形态与 ready 契约

- 同一时间只推进一个语义目标，不并行展开多个完整功能。大型 slice 选中后持续推进到
  最终完成、明确 blocker 或开工前定义的安全停止点，不在阶段之间穿插无关功能。
- **原子 slice** 是适合一次正常提交的小型内聚闭环；**工程 slice** 是跨 crate、
  跨所有权边界、涉及协议/状态机/架构，或需要多个串行阶段和多个正常提交的大型内聚
  工程。两者平等参与 hardest-ready-first；工程 slice 无需先压缩成单提交微任务。
- slice 的“有界”只表示语义目标、非目标、完成标准和验证方式有界，不表示文件、crate、
  提交或实现规模必须很小。
- 工程 slice 进入 ready 集必须具备：单一明确的语义目标；明确范围、非目标和最终完成
  标准；关键前置依赖已满足；跨 crate/所有权边界的集成契约能在开工前冻结；可制定
  串行阶段计划；每阶段都有可运行的 focused validation、可构建且可审查的正常提交；
  失败能借阶段 checkpoint 定位。
- 仍排除 `blocked`、`deferred`、真实前置依赖未满足、目标边界不清或持续扩张、无法定义
  完成标准、无法分阶段验证/提交/定位失败，以及实际混合多个无关功能的目标。不得仅因
  涉及文件多、crate 多、工作量大或需要多个提交而排除。
- 工程 slice 开工前必须记录：最终语义目标与完成标准、明确非目标、涉及 crate/所有权
  边界、vanilla 或 wire 依据、串行阶段及依赖顺序、每阶段 focused tests、checkpoint /
  full-gate 位置，以及失败定位和安全停止方式。

1. **收尾**：确认工作树；已验证未提交的 slice 先完成提交。
2. **audit**：运行 `scripts/audit-p0.sh`；exit 0 且最近一次完整 gate 与已有
   playable/session smoke 没有回退迹象 ⇒ 无新 P0 blocker。exit 1 时人工归类新增命中，
   并在同一 slice 将有明确非 P0 依据的新分类附注释写入 allowlist。不与历史计数对比，
   只看新的不可归类命中或行为回退。
3. **P0 分支**：命中重开条件 → 开 P0 regression slice，只修 blocker，不顺手
   吸收视觉 parity；未命中 → 立即进入 hardest-ready 候选选择，不等待确认。
4. **Hardest-ready 候选选择**：从 P1～P4 路线图与
   `docs/unsupported-features.md` 的全部开放项建立候选集，不按章节顺序串行取项。
   - 按上方 ready 契约同时建立原子 slice 与工程 slice 候选；不得把能冻结契约、分阶段
     验证的完整语义工程仅因不能压成一个小提交而降级成依赖方向。
   - 在完整语义工程层面按五项综合评估实现难度：核心架构/协议/状态机风险，跨 crate
     或跨所有权边界的集成范围，vanilla 行为或 wire 事实不确定性，测试/fixture/迁移/
     回归验证复杂度，以及失败影响范围与定位难度。默认选择难度最高者；最难工程目标
     已 ready 时直接选择该工程 slice。
   - 只有最难工程存在真实 blocker 或集成契约尚不能冻结时，才选择能解锁它的“最难且
     ready 的有界前置 slice”。实现规模较大本身不是 blocker；不得借 hardest-first
     同时展开多个功能，也不得把完整工程无理由永久切碎成低价值微提交。
   - 难度相当时，依次选择：能解锁更多后续目标者；更接近 playable session 或最终
     交付闭环者；最后才按下方 P1～P4 与各节原有路线顺序裁决。这不是 P4→P1 倒序。
   - 每轮开始的进度更新必须简记主要候选及其原子/工程形态、被排除候选及真实原因、
     最终选择和上述难度依据，并说明它为何在完整工程层面是 hardest ready。排除大型
     目标时必须写真实 blocker，不能只写“范围太大”；子 agent 是否使用仍只按
     `AGENTS.md` 的既有并行条件判断。
5. **实现循环**（原子 slice 或工程 slice 的每个串行阶段）：
   1. 先 grep 当前实现，确认缺口真实存在（历史上多次出现"账本以为缺失但
      代码已实现"）；
   2. 定位 vanilla 26.1 源码依据（常量/公式/顺序）；已批量提取过的先查
      memory `vanilla-feature-quickref`；
   3. 实现 + focused test；随机行为用确定性 seed 或固定样本；
   4. 更新账本条目；已完成描述写入 `docs/goal-archive.md`（不是本文件）；
      本文件对应队列项划掉或改写其剩余部分；
   5. 运行本节规定的提交检查，审查 diff 后正常提交（英文祈使句单行 message，沿用
      仓库风格）；工程阶段提交必须保持可构建、可审查，不得以最终会修好为由提交明显
      损坏的中间状态，失败一律 normal fix-forward，不重写历史；再按下方 checkpoint
      规则继续或结束。
   工程 slice 的每次进度更新还必须报告当前阶段、已完成提交、focused test 结果和下一
   checkpoint；在完成、明确 blocker 或预定义安全停止点之前不得插入无关 slice。
6. **Checkpoint 批次与工程里程碑**：仍一次只实现一个完整 slice，不并行启动多个功能。
   连续 2～4 个语义相关且风险可局部界定的低风险 slice 可以组成一个批次；每个
   slice 各自完成 focused tests、fmt check、diff check、按需 docs lint、diff review
   和干净提交。正常批次只在最新提交运行一次 `scripts/cargo-dev.sh gate`，通过后批次
   内所有提交视为已验证，不对每个提交重复完整 workspace gate。
   - 累计第 4 个提交后立即结束批次。即将切换语义领域，或即将向用户交付、停止、
     push、merge、发布时，也必须先结束当前批次并在 tip 运行完整门禁。因这些条件
     提前结束时，实际批次可以只有 1 个提交。
   - 工程 slice 按开工前定义的阶段和里程碑推进；它的多个阶段提交仍属于同一个语义
     目标，不是普通低风险 slice 批次，也不授权穿插其它功能。低风险工程在预定
     checkpoint 以及上述强制边界运行完整门禁。
   - 修改协议编解码、Cargo/workspace 配置、构建脚本、代码生成器、`unsafe`、并发或
     核心架构属于高风险。开始前先门禁并关闭已有低风险批次；原子高风险 slice 保持
     “一 slice、一提交、一次完整门禁”。高风险工程 slice 可以包含多个串行正常提交，
     但不得并入普通低风险 checkpoint 批次；必须在预定义高风险里程碑、累计最多 4 个
     提交、跨越新的协议/架构状态、风险无法局部界定、准备停止或交付时运行完整门禁，
     最终 HEAD 也必须通过。不得为减少 gate 次数把独立高风险目标伪装成一个工程 slice。
     focused test 异常时同样立即运行完整门禁。
   - 批次 tip 或工程 checkpoint 完整门禁失败时，先依据失败项和 focused tests 定位；
     仍无法确定引入提交时，才运行
     `scripts/cargo-dev.sh batch-gate <last-gated-commit>..HEAD` 查找首个失败提交。
     `batch-gate` 不是正常批次或工程 checkpoint 入口。采用正常 fix-forward 提交修复，
     不重写历史，然后重新门禁 tip；完整门禁通过前不得开始下一批、跨过安全停止点或交付。
   - 主 agent 必须记录最近一次通过完整门禁的 commit hash。进度更新和最终交付必须
     报告当前普通批次或工程里程碑包含的提交、focused test 结果、完整门禁结果和最近
     已门禁 commit。任何准备交付或合并的 HEAD 都必须已通过完整门禁。
7. **路线图维护**：P1～P4 继续记录 owner、依赖、里程碑、完成标准与同分顺序，
   不再作为默认串行消化队列。新发现工作在账本立条目并参与下一轮全局候选评估；
   结转项只在重启判据满足后恢复为 ready 候选。只有真实依赖或尚不能冻结的集成契约
   阻塞高难目标时，才前移一个最难且有界的必要前置 slice。

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
完整 gate 与适用的 playable/security smoke 通过。renderer 问题继续断言完整
submission metadata，而不是只看顶点数量；协议/world/runtime 问题断言状态转换、
ordering、canonical state 与 side effect，不以“未崩溃”代替语义验证。

## 当前边界

狭义 P0 只按上节跨域重开条件进入队列；开放式视觉 parity 留在对应优先级。
历史边界与已完成队列见 `docs/goal-archive.md` 的 2026-07-15 workflow-v3 归档小节。

## 客户端差异优先级

本节只列开放路线图。P1～P4 保留为 owner/依赖/里程碑说明，以及 hardest-ready 难度
相当时的最终裁决依据；不再要求按章节或下列路线顺序串行消化，也不是把 P4→P1 倒序。
原有路线顺序仍为：机械 catalog → protocol/session foundation → RegistryAccess foundation →
playable-gate foundation → entity matrix foundation/families → 非实体 closeout →
screen/HUD → resource reload → 健壮性/性能/平台交付。明确的 foundation 和垂直依赖仍是
readiness 条件；protocol/registry/entity 的长尾按文中命名 milestone 切成有界候选，
与其它 ready 项一起按上方 hardest-ready 规则评估。关闭 milestone 后照常运行适用的
playable-gate；暴露出的依赖进入下一轮候选，不得借机扩大当前 slice。

### 结转项（blocked / defer，不进 ready 候选集）

重启判据见对应账本条目；条件满足前不列入候选集，也不因 ledger 文字重开。

- creative inventory-tab preview（原 P1-3）：blocked on creative-screen 基建
  （客户端 `CreativeModeInventoryScreen` 外壳 + `CreativeModeTabs` 物品目录
  均缺失；玩家预览调用点零成本，随外壳落地折叠）。
- entity-in-UI preview 的 `item_layers` GPU 绘制（原 P1-4）：defer，需 native
  侧 baked hand/head item model 元数据。
- font bidi / unihex（原 P1-3 font 五连剩余）：defer。
- player-head BE `profile` owner skin（原 P2 terrain 边界）：随 P3 动态纹理 /
  profile 管线落地；无 profile 的 player head 已走 vanilla 默认皮肤 fallback。

### P1：消除可机械推导的 vanilla 手工维护

目标不是生成客户端行为或复制 Mojang Codec/Java 类层级，而是把注册顺序、数字
ID、resource id、枚举 ordinal、metadata index、tag/class membership 等可从
vanilla 26.1 源码、reports 或资源包确定的数据，收敛为确定性生成物或单一声明源。
packet 字段语义、world apply、移动/碰撞、渲染 shader/mesh、动画、tooltip 与
音频算法仍由对应 crate 手写并用 vanilla 来源和语义测试锁定。

生成器只在维护时读取 `~/Work/mc-code/sources/26.1/` 或明确输入的 vanilla
reports/resources；生成物必须提交，普通 build/test 不得依赖工作区外源码。每类
数据需要可重复的 update/check 入口、来源说明、顺序/唯一性/覆盖测试，并让所有
消费者引用同一权威模块，禁止再保留平行字面量表。

entity metadata extractor 是统一 Tree-sitter Java extraction foundation 的首个
消费者。尚未迁移的 Java generator 后续按独立 slice 处理；本阶段不要求一次改完，
也不得把未迁移 generator 记为已完成。

开放方向如下；每项形成满足上文 ready 契约的原子或工程 slice 后参与全局
hardest-ready 评估：

1. **资源与 tag 派生表**：继续审计手写 block/item/entity membership；有精确 vanilla tag
   的直接从 tag 派生，没有精确 tag 的只在 class/property/registry source fact 能证明等价时
   生成，禁止用名称启发式或近似 tag 扩大集合。block terrain render layer 不是固定
   membership：必须由 baked material/sprite 的实际 transparency 动态决定，缺失贴图使用
   明确的 opaque missing-sprite fallback，不能按 block 名或旧 render-type 表猜测。
2. **跨 crate 去重与反向门禁**：审计 protocol/world/native/item-model/renderer 中
   重复 registry/ordinal/UI 静态 membership 表和生产代码裸 id；消费者改用 typed catalog API，并增加
   机械扫描或结构测试，阻止已收敛语义重新出现第二事实源。测试 fixture 可保留
   局部数字，但须标明 wire 样本，不得被生产逻辑引用。反向门禁若位于 generated
   catalog 中，必须由同一 generator 模板输出，禁止手改生成物造成 update/check 漂移。

完成标准：上述三类各有唯一权威输入和 deterministic check；修改生成输入而未
更新生成物会在门禁失败；clean checkout 无本地 vanilla 源码也能正常构建测试；
版本升级时注册顺序变化表现为集中、可审查的生成 diff；repo 生产代码不再出现
已纳入 catalog 的平行资源表、裸 registry id 或手抄 ordinal；新 vanilla Java
extractor 可直接消费统一 CST/IR 而不复制 tokenizer/parser，普通 build/test 仍不
读取本地 vanilla source。

### P1：Protocol / session 行为覆盖矩阵

目标是把生成的各 state/direction packet catalog 从“ID 清单”推进为完整行为账本，
确保每个 vanilla 26.1 packet 都有明确的 wire、状态机与客户端归属。catalog 仍只拥有
机械元数据；decode/encode、world apply 和 runtime side effect 保持手写，但必须能从
同一 catalog 证明没有遗漏、重复分发或无主 packet。

路线图与依赖顺序如下；前置未满足的项不进入 ready 候选集：

1. **建立派生矩阵**：由 `VANILLA_PACKET_CATALOGS` 生成 checked-in coverage ledger，
   clientbound 至少记录 decode、connection/world ownership、canonical apply、runtime
   side effect 与 tests；serverbound 至少记录 command surface、encoder、sequence/state
   guard、ack/response 与 tests。状态仅允许 `complete/partial/deferred/not-applicable`，
   非 complete 项必须链接 unsupported ledger 的具体条目。
2. **状态机与 framing**：锁定 handshake/status/login/configuration/play 的合法转换，
   compression、disconnect、configuration re-entry、bundle、known packs、cookies、
   resource pack 与 code-of-conduct sequencing；错误 state 的 packet 不得被当成另一阶段
   解析，合法 unknown/custom payload 保留明确策略和 diagnostics。
3. **clientbound 全路径**：每个支持 packet 从 protocol decode 经唯一 net event/
   `WorldStore::apply_play_packet` 到 canonical state 或 `PlayApplyEffects`；纯连接包和
   vanilla no-op 必须写明 owner/依据，不能用 generic unsupported counter 冒充完成。
4. **serverbound 全路径**：覆盖移动、交互、inventory/container、chat/command、book、
   recipe、resource-pack、difficulty/game-mode、query 与 configuration response；验证
   transaction/state id、sequence、签名/ack、nullable/enum/holder 和发送时序。
5. **边界与恶意输入**：为 string/list/NBT/component/chunk/plugin payload 的长度、深度、
   count、VarInt 与压缩边界增加 focused tests；合法边界 round-trip，越界在分配前失败，
   malformed input 不污染 decoder/session state。
6. **反向覆盖门禁**：catalog 新增 packet 默认令矩阵测试失败；dispatch arm 数与 catalog
   双向一致；生产 decode/apply/encode 不允许使用无 catalog descriptor 的裸 packet id，
   probe/online/native 不得重建平行 packet switch。

逐格状态与 evidence 见 `docs/packet-coverage-26.1.json`（由
`scripts/data/packet-coverage-overrides-26.1.json` 经
`scripts/generate-packet-catalog.py` 生成）；历史叙述见 `docs/goal-archive.md` 的
2026-07-15 workflow-v3 归档小节。

matrix inventory + state/framing + required login/config/play handoff 是本队列的 foundation
milestone；完成后即可进入 RegistryAccess/playable foundation，不等待全部长尾 packet
family 清空。之后按 configuration、world-state、movement/interaction、inventory/UI、chat/
command、resource/custom payload 等 family 分别关闭，并由 playable 或 non-entity 垂直
场景决定下一个可执行 family。
各 packet family 的逐格状态/evidence 与历史叙述见上述 coverage ledger 和归档指针。
签名聊天的 local key/session、outbound chain、server echo trust 与 DeleteChat lifecycle
按 `docs/signed-chat-session-design.md` 的四阶段工程合同推进；完整 ChatListener 呈现保持
独立 screen/HUD slice，避免把密码学 transport 与 UI/reporting 混为一个不可定位阶段。

完成标准：全部 vanilla 26.1 catalog entry 都有矩阵行；所有目标客户端适用项为
`complete`，其余只有带 vanilla 依据的 `not-applicable` 或带重启判据的窄 `deferred`；
required login/config/play 与 serverbound control families 有端到端测试；unknown/custom
packet 不会流失步；`docs/unsupported-features.md` 不再保留笼统 packet-family 缺口，只
保留具体字段、状态转换或服务端能力阻塞。

### P1：动态 RegistryAccess 全链路

目标是让所有 network-synced/dynamic registry 的 holder id、resource id、tag 与
inline/direct value 都通过当前连接的统一 registry access 解析。固定 vanilla catalog
只负责 clean/offline fallback 和 built-in registry；服务器已下发某个 dynamic
registry 时，其顺序与内容必须成为该 session 的唯一权威来源，不能继续用 bootstrap
数组解释服务器 holder id。不要复制 Mojang `RegistryAccess` 类层级或一次性实现通用
Codec 框架；protocol 保留无损 wire summary，world 拥有 session registry，具体语义
owner 按消费需要实现 typed value decode。

本路线在 protocol/session foundation 完成后才 ready；下列编号表达依赖与路线图，
不是跨全部开放项的默认串行调度：

1. **来源与生命周期审计**：对照 vanilla 26.1 `ClientboundRegistryDataPacket`、
   `RegistryDataLoader`、`RegistryAccess`/`RegistryFriendlyByteBuf` 及 configuration
   handoff，列出全部 network-synced registry、built-in-only registry 和 holder/tag
   wire 消费点；冻结同一 registry 多 packet、stub、duplicate、session reset、
   reconnect 与 configuration→play 的 replace/merge 语义。
2. **统一查询 API**：在 world registry owner 提供 typed/narrow `RegistryView`/resolver，
   至少覆盖 `(registry, numeric id) -> resource id`、反向 resource id -> numeric id、
   ordered entries、tag membership、raw/decoded entry value 与来源（server/fallback）；
   protocol/native/item-model/renderer 不得各自重建 lookup 表。固定 item/block/entity-type
   catalog 的双向查询和 tag value resolution 已共享该来源 surface；其余 static registry
   与最终 frozen typed view 仍开放。
3. **上下文贯通**：把 registry resolver 作为显式只读上下文传入需要它的 component
   persistent encoder、item-model property、entity metadata/variant、tooltip、audio、
   particle、HUD/debug 与本地保存路径；不得以全局变量、native→protocol 反向依赖或
   raw network bytes 代替 registry-aware Codec 结果。
4. **holder 语义统一**：named holder、direct holder、holder set、tag holder set 与
   optional holder 使用共同的 resolution surface；wire summary 可继续保存 numeric id
   或 inline value，但 resource/SNBT/render 投影必须在拥有当前 session registry 时才
   解析。unknown id、unknown key、缺 entry data 和不支持的 direct value 必须安全省略
   或保留明确 diagnostics，禁止误套 vanilla 下标。
5. **typed entry decode**：按 coverage ledger 仍标记 `partial` 且已有实际消费者的
   dynamic registry family，逐个实现所需 typed NBT/Codec 摘要；未消费 registry 仍可
   无损保留 ordered key/raw NBT，不为抽象完整度提前复制所有 Mojang Codec。
6. **消费者迁移**：清除 component/item-model/entity-scene/audio/HUD 中已可由 session
   registry 解析的 bootstrap 数组和 numeric-id match；sound event、jukebox song、
   enchantment/attribute/trim、villager/animal variant 等既有动态消费统一改用同一 API，
   fixed fallback 只能在服务器没有下发对应 registry 时启用并记录来源。
7. **重排与自定义覆盖**：focused tests 至少覆盖前插 custom entry、vanilla entry 重排、
   删除/unknown entry、stub/inline data、duplicate packet、tag 更新、断线重连清空；对
   damage-type component SNBT、variant render、item model、audio/jukebox、biome/dimension
   各保留一条端到端“numeric id 按服务器顺序解析”的证明。
8. **反向门禁**：增加结构测试或机械扫描，阻止已迁移 registry 重新出现独立完整
   resource 数组、生产裸 holder id 或绕开 resolver 的下标查询；测试 fixture 中的数字
   必须标明 vanilla wire/session 样本，不能被生产逻辑引用。

来源/lifecycle inventory + 统一查询 API + session reset 是 foundation milestone；完成后
playable-gate 可开始接入真实 registry。上下文贯通与消费者迁移按 damage/dimension、
item/enchantment/trim、audio/jukebox、entity/villager/animal variant 等 registry family
分别关闭，不要求所有 family 完成才启动 entity matrix foundation，但对应 registry-backed
cell 在其 family custom-reorder test 通过前不得标为 complete。

逐项状态与 evidence 见 `docs/registry-holder-consumer-coverage-26.1.json`；历史叙述见
`docs/goal-archive.md` 的 2026-07-15 workflow-v3 归档小节。

完成标准：由 vanilla 26.1 network-synced registry 清单和全部 Holder/HolderSet codec
消费点派生覆盖 inventory，每项都映射到 resolver consumer、带证据的 `not-applicable`
或带重启判据的窄 `deferred`，不能以“repo 尚未支持”排除；服务器自定义插入/重排不会
被解释成另一个 vanilla resource；named/direct/tag holder 在目标范围内行为一致，
unsupported 值安全保留或省略且可诊断；连接切换不泄漏旧 registry；无服务器 registry
的 offline/fixture 路径仍有明确、经测试的 26.1 fallback；ledger 不再保留笼统的
“dynamic registry ordering/remap”总项，只保留具体 registry、value variant 和消费者。

### P1：可玩 Session 端到端门禁

目标是用真实 vanilla 26.1 offline-mode server 和可复现 fixture 同时验证跨 crate
集成，不让大量 focused unit tests 掩盖 protocol→net→world→native→renderer/audio 的
连接级断裂。标准 workspace checkpoint gate 保持快速；新增 `playable-gate` 作为 P1 milestone、
session/registry 大改和队列关闭时的权威慢门禁，CI/本地缺 Java server 时必须运行等价
recorded fixture smoke 并明确报告未运行 live 部分。阶段 1/2 的实现与证据归档见
`docs/goal-archive.md`；当前命令入口为 `scripts/playable-gate.sh`。生命周期高风险
工程切片的冻结合同、Vanilla 依据、串行阶段与门禁见
`docs/playable-session-lifecycle-design.md`。

剩余阶段：

1. **基础游玩**：通过 control/CLI 驱动实体交互、hotbar、容器、inventory、
   chat/command、声音/粒子与 deterministic screenshot/readback，验证 server
   acknowledgement、world 结果和呈现 side effect，而不只检查命令入队。已闭环：
   移动与视角、破坏/放置、连接活性（pending-state 门控消除 main↔stream 死锁
   环）——见 `docs/goal-archive.md` 2026-07-18 归档小节。
2. **生命周期场景**：覆盖 respawn、dimension change、death、resource-pack push/pop、
   configuration re-entry、disconnect/reconnect 与第二 session registry；断言旧 world、
   registry、renderer/audio runtime 和 pending request 不泄漏。
3. **扩展失败证据**：在已有 packet/session timeline、world/control snapshot 与 server log
   基础上增加 renderer/audio counters 和最后 screenshot；失败继续区分 server 启动、
   网络、canonical apply、runtime side effect 和 presentation 阶段。

持续架构优化（高优先级跟进，2026-07-18 排队）：

1. **rust/tokio/hecs 特性驱动的持续优化**（长期方向，随 slice 顺带落地）：rust 用
   类型状态/newtype 收紧协议状态机非法空间、`Arc` 共享替代克隆；tokio 连接/控制
   任务零阻塞 await、`watch`/arc-swap 快照、JoinSet 管理任务生命周期；hecs 把
   per-tick 实体动画/物理收敛为 ECS 查询批处理，作为 dual-path 实体迁移统一落
   点。线上语义 vanilla 一致（P0 红线），focused tests + live gate 为回归判据。

完成标准：fixture smoke 可在 clean checkout 的标准测试环境运行；live `playable-gate`
能一条命令完成上述场景并稳定回收；合法 vanilla session 无 panic、hang、required unknown
packet 或状态泄漏；每个核心用户动作同时有 server 结果、world 状态和适用呈现断言；
protocol/registry/entity/non-entity 大 milestone 关闭前都必须通过该门禁。

### P1：157 entity family 客户端行为覆盖矩阵

目标是覆盖 vanilla 26.1 固定 `EntityType` catalog 中全部 157 项在客户端可观察的
生命周期与呈现行为，而不是把 Java 实体继承树、服务端 AI/goal、寻路、刷怪、战利品
或服务端权威战斗逻辑移植进客户端。共同父类行为按 family/schema 实现一次，具体
entity 只登记差异；禁止为了把矩阵涂绿而增加空分支、恒等 placeholder 或复制模型/
metadata match。

本队列在 RegistryAccess 与 playable-gate foundation 之后开始，并按 family 单切片推进：

1. **建立权威矩阵**：新增由 `VANILLA_ENTITY_TYPES` 驱动的 checked-in coverage ledger，
   157 项不得手写第二份名字/id 表。每项至少记录 spawn/default、metadata/schema、
   transform/interpolation、client tick/movement、dimensions/collision/pick、model/texture/
   layers、animation、equipment/passenger/leash、event/particle/audio、save/debug 十类状态，
   取值仅允许 `complete`、`partial`、`deferred`、`not-applicable`；`complete` 必须
   引用权威实现与 semantic test，后三者必须链接 `docs/unsupported-features.md` 的
   具体原因与 vanilla 证据；共同 parent/family 实现只引用一次，不为 157 行复制证明。
   当前入口为 `docs/entity-coverage-26.1.json`；分类输入与共享证据在
   `scripts/data/entity-coverage-overrides-26.1.json`，通过
   `scripts/generate-entity-coverage.py` 更新或检查。
2. **继承 schema 与默认值**：先完成 Entity→LivingEntity→Mob/Animal/Monster/Vehicle/
   Projectile/Display 等父链 metadata index、serializer、默认值、flag mask 和 dimensions
   schema，再登记子类差异；测试锁定继承偏移、同 index 不同 family serializer、spawn
   packet data 与 constructor/client default，替换散落裸 index/flag match。
3. **生命周期与网络状态**：逐 family 验证 add/remove、UUID/type、位置/旋转、delta、
   teleport/interpolation、passenger/vehicle、equipment/effect/attribute、entity event 与
   level clear/respawn；unknown 或未呈现实体仍必须保留可 probe 的 canonical state，不能
   因缺 renderer 丢掉网络状态。
4. **客户端 tick、移动与物理**：只实现 vanilla multiplayer 客户端实际推进的 local-player
   prediction、仅 locally-controlled vehicle 的 movement/serverbound 回报、remote transform
   interpolation，以及 `ClientLevel` 确实执行的 subclass tick、animation counters、粒子、音频、
   projectile motion/trail/presentation；server correction 覆盖预测。命中、伤害、爆炸、方块
   修改、权威移除、mob AI/goal、寻路、刷怪、繁殖、战利品和不可重建 server RNG 均标为
   `not-applicable`，不得进入客户端完成矩阵或以 synthetic state 冒充。
5. **渲染与动画**：逐 family 补齐 model family、baby/scale、texture/variant、equipment/
   emissive/overlay/held-item/armor 等 layer、keyframe/procedural animation、name/tag、
   shadow/culling/bounds 与 render-state interpolation；missing asset 保留明确 vanilla
   fallback 和 diagnostics，不能用 raw-cuboid/identity 长期冒充 complete。
6. **事件与交互呈现**：覆盖客户端可观察的 hurt/death/attack/tame/status event、粒子、
   声音、拾取、乘骑/缰绳、选取/准星、HUD/debug 和本地交互反馈；协议 decode、world
   canonical apply 与 runtime side effect 必须走现有唯一分发路径。
7. **保存与调试投影**：普通 F3+I 必须以匹配 transaction 的 server entity-tag query
   response 为完整 NBT 权威；null/stale response、query 失败或目标消失不得生成伪造命令。
   Shift-local `saveWithoutId` 只投影 synced metadata、spawn/equipment/component、client-owned
   runtime 等实际可观察字段；server inventory、owner/target/private timer/RNG 等不可观察状态
   不得由 default/source expression/heuristic 猜测，需在矩阵标为 `not-applicable`。
8. **family 关闭门禁**：每个 family slice 同时更新矩阵和 unsupported ledger，运行其
   protocol/world/native/renderer focused tests；增加 coverage test，保证 157 项都有行、
   每个 `complete` cell 有实现/测试引用、catalog 新增项默认令门禁失败，且生产中的
   entity type 分支只引用 catalog 常量/schema API。

matrix/schema/base lifecycle 是后续 family 的 readiness 前置。该前置完成后，
base/display/projectile/vehicle 与 living 的 aquatic/passive/tamable/villager/illager/
undead/arthropod/boss family 都按有界 slice 参与全局 hardest-ready 评估；上述原顺序仅作
难度相当时的本节内最终裁决。
registry-backed variant cell 必须在上一队列的 custom reorder 测试通过后才能标为
`complete`。允许一个 family 因明确 server-only 或不可重建随机行为保留窄
`not-applicable`，不允许以“已有通用 fallback”把缺专属模型、动画或 client tick 标绿。

该矩阵不是一个必须 157 项全部清空才允许后续工作的原子队列：matrix+schema+base
lifecycle 为 foundation milestone，之后每个上述 family 是独立 milestone。每关闭一个
family 都运行 playable-gate；其暴露的 registry、non-entity、screen 或 resource 垂直依赖
进入下一轮全局候选；只有真实依赖或尚不能冻结的集成契约阻塞更难目标时，才选择其中
最难且 ready 的有界前置 slice。不得无证据跳过必要依赖，也不得
用“仍在 P1”长期阻塞已经具备独立验收面的 P2/P3 可玩性修复。

完成标准：矩阵恰好覆盖 catalog 157 项；所有客户端适用 cell 均为 `complete`，其余仅
为有 vanilla 证据的 `not-applicable`，不再存在 `partial/deferred`；AddEntity/metadata/
movement/event 的 canonical state 对所有类型可保留和 probe；每个有官方 renderer 的
entity 都有正确 model/texture/layer/animation 路径或官方 fallback；所有客户端实际 tick
和可观察 side effect 有 family 测试；`docs/unsupported-features.md` 不再包含 vanilla
entity family 行为缺口，只保留服务端专属、外部资源/权限阻塞或明确不适用于外部客户端
的事项。

### P2：非实体客户端行为 closeout

目标是给 entity matrix、screen/HUD 和 resource 队列之外的客户端行为一个明确关闭
路径，覆盖 `docs/unsupported-features.md` 中 protocol 已解码但尚未完整呈现的 level/
chunk/block entity、input/movement/interaction、inventory/gameplay、particle/level event、
audio 与跨域 side effect。按用户可观察 vertical slice 推进，不按 crate 各做半条链。

1. **ledger 路由**：建立 unsupported 一级域→goal queue/milestone 的双向清单；已由
   protocol/entity/screen/resource 队列负责的条目只引用，不复制；其余条目全部进入本
   队列的 world、interaction、effects 三个 owner，新增 orphan heading 令结构测试失败。
2. **world 与 block entity**：补齐 dimension/time/weather/chunk/light/biome/block state、
   block event/entity、map/scoreboard/recipe/advancement 等 canonical state 与可观察呈现，
   重点关闭 sign/banner/chest/shulker/spawner/bell/vault 等现有窄缺口和清理生命周期。
3. **input、movement 与 interaction**：覆盖 local player movement/collision/fluid/portal、
   crosshair/pick、attack/use/destroy/place、vehicle、inventory/container prediction 与
   server acknowledgement；服务端权威结果不在客户端伪造，本地 prediction 与 rollback
   必须有明确边界。
4. **particle、level event 与 audio**：从 packet/world event 经唯一 effects sink 到
   resolver/runtime/render submission，覆盖 provider options、block/item context、entity
   following、category/attenuation、delayed/stop 与 missing asset diagnostics；随机链保持
   online/probe/fixture 确定性一致。
5. **垂直关闭门禁**：原子 slice 或工程 slice 最终完成时同时断言 protocol summary、
   canonical state、runtime command/counter 与适用 screenshot/audio resolver 结果；只完成
   decode、只加 counter 或只画 fallback 均不得删除最终用户可观察的 ledger 条目。

完成标准：unsupported ledger 每个非实体客户端行为条目都有唯一 queue owner；所有目标
客户端适用条目完成或仅剩有 vanilla 依据的 `not-applicable`/带重启条件的 `deferred`；
playable-gate 覆盖基础 movement/interaction/inventory/world effects；不再存在已 decode
但在线 dispatcher、world apply 或 presentation 任一段无主的 required packet 行为。

### P2：屏幕、HUD、字体与截图

既有 screen/HUD/font 完成史见 `docs/goal-archive.md` 的 2026-07-15 workflow-v3 归档小节。

仍在推进（消化顺序即列出顺序，todo 见账本 "HUD overlay and screen render
surfaces" 节；整体低优先；F3+I entity save payload 由上方 entity matrix 唯一拥有）：

- vanilla profiler section 全覆盖 + profiling metrics recorder/output。
- DebugOptionsScreen narration / focus / 完整 widget styling polish。
- native pause tick-freeze eligibility 与 PauseScreen 剩余 action/subscreen。

完成标准：新 UI/screen 工作不做临时配置 UI，启动配置仍从命令行进入；上述开放项及
ledger 中归属本队列的 HUD/screen 条目全部完成或有窄 deferred/not-needed 依据；视觉结果
必须有 deterministic screenshot/pixel readback，无法自动化的分支留下可复现 vanilla
对比步骤与截图证据，不以 widget 存在或 draw-call 数量代替呈现语义。

### P3：资源包加载、reload 与动态资源泛化

目标是实现 vanilla 26.1 客户端 pack stack 和 reload 生命周期，使内置资源、用户指定
资源包、服务器 resource-pack push/pop、datapack registry asset 与运行时下载纹理能按
统一优先级和状态机组合；不能只让首次启动路径正确，reload 后继续引用 stale atlas/
model/font/sound/texture handle。

1. **pack stack 与来源**：冻结 vanilla/client built-in、启动参数 pack、server pack 的
   优先级、namespace/path 解析、metadata/pack-format 校验与 selected/required/prompt
   状态；继续禁止 in-game 配置 UI，用户选择从命令行或 control surface 进入。
2. **server pack 生命周期**：覆盖 push/pop、UUID、URL/hash、required、prompt、download/
   verify/cache/apply/fail/decline 状态与 serverbound response 时序；重复、替换、断线和
   session 切换不复用错误 pack 或 pending request。
3. **统一 reload barrier**：资源集合变化时按依赖顺序重建语言/font、atlases/textures、
   blockstate/model/item model、particle、waypoint、sound 与 renderer/audio caches；world
   canonical state 不被资源 reload 重置，旧 generation 的异步 decode/upload 结果不得
   回写新 generation。
4. **动态纹理与自定义资源**：完成 broader non-profile dynamic texture、资源包 override、
   custom block/item/entity model、datapack registry asset 的组合；profile/skin、cape/
   elytra、painting/map/banner/sign/GUI 等 consumer 复用统一 loading/pending/ready/failed
   surface，不各建缓存状态机。
5. **fallback 与 diagnostics**：missing/invalid/unsupported 资源遵循明确 vanilla fallback，
   submission metadata 在 atlas/texture 未就绪时仍先记录；不画 stale texture，不把解析
   失败伪装成成功，control/debug 可定位 pack、resource id、generation 和失败阶段。
6. **reload 集成测试**：用两个最小 pack 验证优先级覆盖、push→apply→pop 恢复、模型/
   atlas/font/sound 同步切换、异步旧结果丢弃、失败保留上一可用或 missing fallback；
   playable-gate 覆盖一次服务器 pack 生命周期与 deterministic screenshot/audio 变化。

完成标准：pack stack、download/cache、reload generation 与所有资源 consumer 共享唯一
生命周期；资源切换不要求重启客户端且无 stale handle；server response 与实际 apply
状态一致；custom model/texture/sound/font 的支持范围由 semantic tests 锁定；资源缺失和
恶意声明安全失败并保留可诊断 fallback。

### P4：健壮性、运行时性能与平台交付

功能 parity 之外，目标客户端必须能长期连接不可信服务器、加载不可信资源并在支持平台
稳定运行。优化不得改变 canonical 语义或以删除 diagnostics/fallback 换取指标；性能基线
只使用可复现场景和稳定统计，不把单机瞬时 wall-clock 当成脆弱单元测试。
当前 simulation/publication/render extraction/terrain residency 工程切片的开放边界、
等负载 official 对比合同与阶段门禁以
`docs/simulation-render-performance-design.md` 为准；剩余具体缺口只在
`docs/unsupported-features.md` 的 runtime performance 条目维护。

1. **输入与资源安全**：为 packet frame/compression、NBT/component、JSON/model/font、
   image/audio/ZIP 增加 size/depth/count/ratio/time 限制、路径规范化与 fuzz/property tests；
   所有 parser 对 malformed/untrusted input 返回有上下文错误，不 panic、越界或在限制前
   分配攻击者声明的容量。
2. **队列、异步与生命周期**：验证 net event、chunk/mesh upload、resource download/
   decode、profile/texture、audio/particle 的 backpressure、cancel、generation 与 shutdown；
   disconnect/reload/window close 能终止 worker，慢 consumer 不造成无界队列。
3. **性能预算**：为代表性 view distance 场景记录 frame CPU/GPU、chunk decode/mesh/upload、
   entity extraction/render、memory/cache 与 network throughput baseline；优化走 profiling
   证据，设宽容 regression budget 和可解释 variance，构建性能继续遵循
   `docs/cargo-build-performance.md`。
4. **soak 与故障恢复**：增加长时间 movement/chunk churn/entity/particle/resource reload
   soak，覆盖断网、server kick、音频设备失败、窗口 resize/minimize、GPU surface 丢失与
   clean shutdown；检查内存/handle/queue 单调增长、stale session 状态和可恢复错误。
5. **平台与发布面**：明确支持的 OS/GPU/backend/窗口输入矩阵，验证 DPI/fullscreen/
   focus、keyboard/mouse/clipboard/IME、无音频设备和 headless probe；启动配置只走 CLI，
   提供版本/协议/pack-format、日志、诊断快照和可复现启动参数，不增加 in-game options UI。
6. **分层门禁**：快速 workspace checkpoint gate 保持现有命令内容；fuzz corpus、performance benchmark、live
   playable 与 soak 进入可单独执行的 milestone/CI gate，失败产物可归档且命令、seed、
   pack/world fixture 可复现。

完成标准：合法 vanilla session/resource 不因防御限制误拒绝，恶意输入在明确预算内失败；
代表性场景无无界内存/队列/handle 增长，性能回退可量化定位；断线/reload/device/surface
故障不会污染下一 session；所有声明支持的平台至少通过 clean build、fixture playable
smoke 和启动/关闭生命周期测试，发布诊断足以复现失败。

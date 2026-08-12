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
- 完成史不入当前文档：完成记录只存在于 Git commit、diff 与 path history；不得
  另建 milestone/archive 索引、完成清单或取证散文。长期仍成立的机制与不变量写入
  canonical Wiki，机器验收输入与当前回归基线写入 `docs/evidence/`。
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
- 行数上限与历史文档布局由 `scripts/docs-lint.sh` 机械 enforce。

## 硬性约束

- 严格按 vanilla 26.1 源码转写：只通过 `scripts/vanilla path 26.1` 解析并使用已校验 workspace。
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
  不得各写一份平行 arm/分支/表任其分叉。需要新消费点时接到权威实现上；发现
  既有多路实现时按 refactor slice 立项收敛，不得再添第 N 份副本。
- 从 vanilla Java 源码生成 catalog 时统一复用 Tree-sitter Java source
  frontend；禁止各 generator 另写易漂移的正则、tokenizer 或括号扫描器。通用
  frontend 只拥有 CST、source span 与声明/调用 IR，领域 extractor 只拥有领域
  识别/结构校验。尚未迁移的旧 generator 按独立语义 slice 逐项迁移。

## 结构不变量（新 slice 不得回退；出处见代码、测试与 canonical Wiki）

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
  `docs/evidence/vanilla-26.1/registry-holder-consumer-coverage.json`；不得重建独立 resource
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
- simulation command mailbox 是 window/control/render feedback/services/lifecycle
  producer 的唯一 FIFO owner；五类 count/retained-byte quota 独立分区，饱和用
  backpressure，不能丢 release/interaction/shutdown。worker 退出须原子 close 并唤醒
  waiter；新增 producer 必须分类、递归计入 owned capacity、处理 oversized/closed，且
  不得绕过 `enqueue_or_report`/显式 `Result` 恢复 unchecked enqueue。
- `MenuScreens` 的 menu-type→screen-kind 事实只来自共享 Tree-sitter 生成的 native-private
  catalog；protocol 只拥有 registry id，world 只拥有 container/session state，layout 只能做一次
  catalog 查询。新增注册须通过 exact-output stale check，不得恢复手写 screen resolver。
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

- `scripts/vanilla verify 26.1`
- `scripts/dev doctor --profile portable`
- `scripts/docs-lint.sh`
- `cargo fmt --all --check`
- `scripts/dev lint python`
- `scripts/dev test python --suite all`
  （仅当被门禁的改动触及 `scripts/`；`BBB_GATE_FULL=1` 强制全跑）
- `git diff --check`
- `CARGO_TARGET_DIR=/tmp/bbb-target-main cargo check --workspace`
- `scripts/cargo-dev.sh cross-check`（crate/manifest 改动时；`BBB_GATE_FULL=1` 强制）
- `CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test --workspace`

deny-warnings 由根 Cargo.toml 的 `[workspace.lints.rust] warnings = "deny"`
承担；禁止手工设置 `RUSTFLAGS='-D warnings'`——它会双向作废
`/tmp/bbb-target-*` 缓存，代价是整仓重编（见
`docs/wiki/development/build-and-cache.md`）。
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
2. **audit**：运行 `scripts/audit-p0.sh` 与
   `scripts/python python scripts/coverage-status.py`；exit 0
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
   1. 先 grep 当前实现，确认缺口真实存在且账本没有落后于代码；
   2. 定位 vanilla 26.1 源码依据（常量/公式/顺序）；已批量提取过的先查
      memory `vanilla-feature-quickref`；
   3. 实现 + focused test；随机行为用确定性 seed 或固定样本；
   4. 更新账本/矩阵条目；完成项从当前状态文档删除，追溯只走 Git；
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
     流程变更与 enforcement 同 commit 落地，不另写版本史或归档索引。

## P0 重新打开条件与 audit 分类

只有出现以下任一情况才开 P0 slice（未进入正式交付合同的普通视觉精度、长尾
功能缺口和开放式性能优化不算；下方已冻结的官方同载渲染机制与可见 workload
等价合同属于正式 P0）：

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
- 已声明完成的正式同载性能合同出现可复现的 camera/world/resource/settings、
  visible/resident geometry、颜色域或 capture 时序不一致，或在同载成立后出现足以
  破坏可玩交付的稳定帧尾回退。修复不得减少任一侧实际可见/resident 工作量。

已知非 P0 分类基准见 `scripts/data/p0-audit-allowlist.txt`；分类变化时更新该文件并附依据。

P0 完成标准：触发回退有最小复现和对应 semantic/integration regression test；修复后
完整 gate 与适用的 playable/security smoke 通过。renderer 问题断言完整 submission
metadata；协议/world/runtime 问题断言状态转换、ordering、canonical state 与 side
effect，不以“未崩溃”代替语义验证。狭义 P0 只按本节跨域重开条件进入队列；没有
进入正式交付合同的开放式视觉 parity 留在对应优先级。

### P0 合同：官方 26.1 同载渲染与性能闭环

同载门禁的唯一长期定义见[等负载机制页](docs/wiki/mechanisms/performance/equal-load-benchmark.md)，
当前 Retina baseline 身份只取其 committed receipt。重开时作为一个高风险工程 slice：先
校验 Vanilla 源码事实、画面、replay、world/resource/settings/input、drawable 与
visible/resident/draw workload，再做性能比较；任何 identity/workload 不一致都先于指标
比较拒绝。

禁止 patch/mod 官方 jar，或以较低 view distance、resident cap、实体/粒子/HUD/透明层、
分辨率、VSync、FPS cap、sleep 或 pacing 换取结果。正式采样保留三场景、每客户端至少
四次 capture、两轮 ABBA 及 combined/scene/round/nearest-pair scope；candidate 先以当前
BBB baseline 做交错 A/B。FPS margin 由至少四次重复噪声预声明且不得超过 1%；App/GPU
p99、App 1% low、App/GPU hitch 与同仪器 input software latency 是硬门禁，stddev、
p99-p50 与 CV 只作诊断。

关闭重开项需要 focused/integration tests、完整同载 evidence、适用 playable/platform
receipt、提交前检查和 full-gated clean HEAD 全部通过。失败必须 fix-forward；截图看起来
接近、combined 掩盖失败 scope 或缺失外部证据都不是关闭依据。

## 客户端差异优先级

路线状态是机械索引，不是独立任务源；具体 ready/deferred/not-needed 事实只在
`docs/unsupported-features.md`。`scripts/docs-lint.sh` 要求下列 marker 完整，并在
active TODO 为 0 时拒绝仍声明 active 路线。

<!-- roadmap-status:start -->
- p1-playable: maintenance
- p1-generated-facts: maintenance
- p2-non-entity: closed
- p2-hud: closed
- p3-resources: closed
- p4-delivery: active
- architecture-convergence: maintenance
<!-- roadmap-status:end -->

有 active 项时仍按 hardest-ready-first；难度相当才按 P1→P2→P3→P4 owner 顺序
裁决。关闭 milestone 后照常运行适用的 playable-gate，暴露的真实缺口先入账本，
不得借机扩大当前 slice。

### 架构收敛与防漂移

状态：`maintenance`。新候选只有在机械审计登记真实重复 owner/path、删除目标、evidence
与 reopen condition 后才重开为 `active`，不得保留无界优化任务。

### P1：可玩 Session 端到端门禁

状态：`maintenance`。fixture/live 命令、session lifecycle、基础交互与分层失败
证据由 `scripts/playable-gate.sh`、Wiki 测试门禁页、本地 stamp/report 与 Git history
持续维护。新垂直 slice 若改变可玩
行为，必须扩展适用场景并按 checkpoint 规则刷新 stamp；门禁回退按 P0 处理。

### P1：可机械推导的 vanilla 事实

状态：`maintenance`。外部客户端 owner inventory、source/generator stale check、
inventory→matrix 与 ledger↔matrix 双向映射由唯一版本化 JSON、共享 Tree-sitter frontend 和
docs lint fixtures 维护。新增或移动 Vanilla owner、BBB evidence、状态或 route 必须先使离线
机械检查恢复为 unmapped/断链 0；inventory 自身不拥有任务状态。

### P2：非实体客户端行为 closeout

状态：`closed`。新增 Vanilla owner、route 状态回退或对应 matrix 行的重开条件触发时，
把具体差异登记为 active 并重开本节。

### P2：屏幕、HUD、字体与截图

状态：`closed`。对应 matrix route、owner/evidence 映射、地形过滤合同或全屏
toast/F3 overlay 合同回退时，把具体差异登记为 active 并重开本节。

### P3：动态资源 presentation breadth

状态：`closed`。资源 generation、reload consumer、pack stack、window icon、panorama 或
对应 presentation owner 的既有合同回退时，把具体差异登记为 active 并重开本节；
built-in pack ZIP 与 Realms 对应 surface 按账本保持 `not-needed`。

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

以后只有目标 index/pack 语义变化、统一 resolver 回退或新 current-surface consumer 出现时
重开。

### P4：健壮性、运行时性能与平台交付

状态：`active`。当前 ready 路线取自账本的 release revalidation 与 committed
direct-dependency receipt/依赖现代化审计；依赖迁移从 winit 0.30 开始。平台、生命周期、
音频、安全预算与有界队列按各自 gate/stamp 和 Wiki 合同维护。
macOS 与 Linux 是声明平台，Windows 是账本附 Vanilla/产品依据的 `not-needed`。

Retina 性能基础设施固定为 crates.io wgpu/core/hal/types/naga 30.0.0；不得恢复 0.19、
path patch、vendor 或直接 `wgpu-hal` 依赖。当前 baseline 身份、重复采集噪声与 SHA
只从 `docs/evidence/macos-native-retina/current-wgpu30-baseline.json` 读取；后续 candidate
只与该 full-gated wgpu 30 baseline 比较，噪声 margin 最大不得超过 1%。失败 candidate
不进入当前文档，追溯只走 Git。

当前接受的 macOS Immediate 策略保持一个 controller-owned render submission、latency hint 2、
三个 drawable 与 wait→acquire→submit→present；FIFO、Linux、surface fallback 和可见 workload
不变。`renderer-frame-flight-v1`、`renderer-gpu-stages-v1` 与
`renderer-input-latency-v1` 继续作为 opt-in 诊断面，frame-phase v3 的
`post_present_fence_submit` 字段继续报零；未开启详细诊断时保持两个 timestamp query。
不可变 FramePlan/PassPlan 继续锁定活跃 pass、
attachment/copy consumer、presentation source 与 pipeline/material batch 顺序；不得引入通用
render graph 或改变可见 workload。只有 raw stage/pass/copy 证据证明无消费者或重复创建时，
才允许单独 candidate 消除该成本。

候选在 clean full-gated commit 上按 AC Power、fullscreen 2560x1600、VSync off、FPS
unlimited、相同 world/replay/settings/resource/input 运行 B→C→C→B×2。validator 先拒绝
binary/runtime dependency receipt、drawable、surface、timestamp strategy 或 workload 身份
不一致，再检查 combined、三个 scene、两个 round、全部 nearest pair 及逐 scene pair。
FPS 使用预声明 margin；App/GPU p99、App 1% low、App/GPU hitch 与同仪器 input latency
是 candidate 硬门禁。App p50/p95/stddev/p99-p50/CV 与 GPU 的 mean/p50/p95/stddev/
p99-p50/CV 继续逐 scope 记录为诊断，不因这些字段单独略差而阻止。Java 对比的硬门禁为
BBB mean FPS 不低于官方、App p99 不高于官方、1% low 不低于官方、App/GPU hitch 不高于
官方；诊断字段局部回退必须显式列出。禁止 sleep、pacing、cap、VSync 或
降低工作量。只有所有 BBB A/B 硬门禁 scope 通过才运行正式 Java→BBB→BBB→Java ABBA。

Retina maintenance 在 drawable/surface/画面/workload/input identity 不一致，或相对当前
receipt 的 FPS 超其预声明 noise margin、App/GPU p99、App 1% low、App/GPU hitch、input
latency 任一显著回退时重开。
依赖现代化每次只处理一个有界依赖组：winit 0.30 后依次审计 pollster、认证/网络、glam、
hecs、音频/归档/开发工具，并按 runtime、协议、平台与性能风险决定升级、保留或 deferred。

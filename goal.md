# 目标：bbb 渲染管线与 Minecraft Java 26.1 对齐

## 总目标

让 `bbb` 的 renderer / world / native / pack / protocol 边界逐步对齐
Minecraft Java 26.1 官方客户端行为。以 `docs/unsupported-features.md` 为事实清单：
除明确决定不支持的 feature 外，其余 renderer 差异必须收敛为 repo-native 实现。

## 本文件维护规则

- 本文件只保留：工作流、硬性约束与结构不变量、未完成项、优先级、完成标准。
- 已完成的 slice 描述在提交当轮迁入 `docs/goal-archive.md` 对应小节；
  "仍在推进"下只允许可验证的未完成项，不堆已完成史。
- 剩余工作必须正面列举（要做什么）；禁止"X（不含 …）"排除式定义——排除
  清单随完成度无限增长。逐项追踪型 checklist 放 `docs/unsupported-features.md`
  对应条目，本文件只指向它。
- 随 slice 变化的快照数据（audit 计数等）不写入本文件；只记录动作与判据，
  不记录当时的数字。
- 本文件超过 300 行即触发归档瘦身。历史归属：`docs/goal-archive.md`
  （已完成目标）、git（更早的完成史）、memory（跨会话方法论与长期状态）。
  `docs/unsupported-features.md` 是纯 TODO list，不存任何完成史。

## 硬性约束

- 严格按 vanilla 26.1 源码转写：优先使用 `~/Work/mc-code/sources/26.1/`。
- 协议、metadata 索引、render type、layer order、transform、texture path、
  tint、light、overlay 不能凭记忆实现。
- `bbb-renderer` 不得依赖 `bbb-pack`；`bbb-item-model` 不得依赖 `bbb-renderer`
  （renderer/native 共享的纯值类型放 `bbb-render-types`）。
- 每个 slice 保持小而可合并，避免顺手重构和无关格式 churn。架构重构作为
  专门 refactor slice 立项，不与 parity slice 混合；重构落地时同步把新约定
  写成下方结构不变量，能加反向断言的加反向断言。
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

## 结构不变量（新 slice 不得回退；出处与日期见 goal-archive 与 memory）

- 新增 clientbound play packet 的处理只进 `bbb-world::apply_play_packet`；
  运行时副作用走 `PlayApplyEffects` trait。probe/dispatcher 不得再各写 arm。
  新增 play-packet 副作用时，无 sink 的 `PlayApplyEffects` 调用方与 native
  sink 调用方必须走同一条确定性 random/context 路径。
- 实体 type id 常量一律取自 `bbb_protocol::entity_types`；粒子 type id 常量
  一律经 `particle_registry::particle_type_ids!` 宏从 `PARTICLE_TYPES_26_1`
  编译期派生；均不得内联数字。
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
- control 层是纯派生视图：`ControlSnapshot` 只承载 counters 派生数据，world
  查询走 `SharedWorld`（`Arc<RwLock<WorldStore>>`）读锁按请求执行，输入请求
  走 `ControlRequests`；不得把 WorldStore 或请求队列塞回快照。
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

- `cargo fmt --all --check`
- `git diff --check`
- `RUSTFLAGS='-D warnings' CARGO_TARGET_DIR=/tmp/bbb-target-main cargo check --workspace`
- `RUSTFLAGS='-D warnings' CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test --workspace`

单条 `-D warnings` workspace 命令覆盖全部 crate（含 control/net/protocol/
audio），2026-07-05 起取代原"workspace 无 -D + 六个 crate 逐个 -D"的九条
命令组合。`cargo check` 一条不可省：test 构建把 test-only 用法算作使用，
只跑 `cargo test` 会漏掉非测试（交付 bin）构建的 unused/dead-code lint。
纯文档/memory slice 只需 `git diff --check`。

## Slice 工作流（每轮）

队列 auto-start 是默认分支，只有新的 P0 blocker 能抢占；P0 clean 时不得停下
等待确认，也不因历史 ledger 文字重开 P0。

1. **收尾**：确认工作树；已验证未提交的 slice 先完成提交。
2. **audit**：`rg -n 'residual|fallback|unsupported'` 扫 crates/ 代码侧命中，
   逐个归类。全部命中均可归入下节"已知非 P0 分类"且未触发重开条件 ⇒ 无新
   P0 blocker。不与历史计数对比，只看是否出现新的不可归类命中。
3. **分支**：命中重开条件 → 开 P0 regression slice，只修 blocker，不顺手
   吸收视觉 parity；未命中 → 本轮立即从当前最高优先级队列取首个可执行小
   slice 进入实现，不等待确认。
4. **实现循环**（每个 slice）：
   1. 先 grep 当前实现，确认缺口真实存在（历史上多次出现"账本以为缺失但
      代码已实现"）；
   2. 定位 vanilla 26.1 源码依据（常量/公式/顺序）；已批量提取过的先查
      memory `vanilla-feature-quickref`；
   3. 实现 + focused test；随机行为用确定性 seed 或固定样本；
   4. 更新账本条目；已完成描述写入 `docs/goal-archive.md`（不是本文件）；
      本文件对应队列项划掉或改写其剩余部分；
   5. 过门禁，独立提交（英文祈使句单行 message，沿用仓库风格）。
5. **队列维护**：队列消化顺序见"渲染管线差异优先级"首行；当前队列无可执行
   项时顺延下一条；结转项只在其重启判据满足后入队；新发现的工作按优先级
   插队并在账本立条目。

## P0 重新打开条件与 audit 分类

只有出现以下任一情况才开 P0 slice（GPU path 的视觉精度差异不算）：

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

已知非 P0 分类（audit 归类基准；分类变化时更新此清单，不记计数）：

- `residual`：仅 `entity_models/dispatch.rs` 的"无 residual mesh-emitting
  arm"注释。
- `unsupported`：screenshot surface format bail、dynamic-player texture
  render-type defensive panic、pack / font 等资产解析校验 bail、unknown
  packet / component diagnostics、tests 和 docs 指针。
- `fallback`：colored debug/profile/terrain/HUD/map/test 或 vanilla fallback
  注释。

P0 完成标准：audit 能把每个命中归类为非 P0 或给出对应修复；相关测试断言
submission metadata，而不是只看顶点数量。

## 当前边界

- 狭义 P0 pipeline closeout 不再吸收开放式视觉 parity；closeout freeze 后
  连续多轮 audit 无新 P0 blocker；lighting / overlay / outline 无 blocker
  记录。
- 架构重构两批次已完成并固化为上方结构不变量：2026-07-02 #1-7
  （2d19d2a3..cae7bcbf）与 2026-07-04 旧账清偿（1de8f14b..44244ea3）；明细
  见 memory `architecture-refactor-progress` 与 goal-archive。
- P1 全部五个子队列 2026-07-05 清空（a0031d3a..bb4e8d34 loop run），P2
  Terrain / Block Render Presentation 2026-07-08 闭项；两者原文（含完成
  标准）2026-07-09 迁入 goal-archive，本文件不再保留其章节。
- 开放队列只剩「P2 屏幕、HUD、字体与截图」与「P3 资源与动态纹理泛化」两节，
  外加下方 P1/P2 结转的 blocked/defer 项。

## 渲染管线差异优先级

P1 五节与 P2 Terrain 节均已闭项并迁入 goal-archive；本节只列开放队列。按
队列规则从 P2 屏幕/HUD 节取最小可关闭 checklist 的 slice 消化，该节耗尽后
进入 P3；新发现工作按优先级插队并在账本立条目。

### 结转项（blocked / defer，不进消化队列）

重启判据见对应账本条目；条件满足前不列入队列，也不因 ledger 文字重开。

- creative inventory-tab preview（原 P1-3）：blocked on creative-screen 基建
  （客户端 `CreativeModeInventoryScreen` 外壳 + `CreativeModeTabs` 物品目录
  均缺失；玩家预览调用点零成本，随外壳落地折叠）。
- entity-in-UI preview 的 `item_layers` GPU 绘制（原 P1-4）：defer，需 native
  侧 baked hand/head item model 元数据。
- font bidi / unihex（原 P1-3 font 五连剩余）：defer。
- player-head BE `profile` owner skin（原 P2 terrain 边界）：随 P3 动态纹理 /
  profile 管线落地；无 profile 的 player head 已走 vanilla 默认皮肤 fallback。

### P2：屏幕、HUD、字体与截图

font 全部子项已在 P1-3 五连 slice 完成（bidi/unihex 见上方结转项）。
2026-07-05 入口审计：hotbar/crosshair/生命/饥饿/经验条基础档、22 变体
container screen 家族、merchant 交易 UI、book 阅读均已完成；HUD 深度/颜色
load/clear 语义与 vanilla 对齐（stratum/blur depth-clear 随 blur 型 screen
再进入）。debug overlay 的 F3 toggle/keymap、左右列基础 entry、F3+1..4 图表与
lightmap、F3+B/G/H/A/C/D/I/L/N/P/S/T/V/Esc/F4/F6 各动作与 shell、GameModeSwitcher、
PauseScreen、DebugOptionsScreen 主体均已落地（完成史见 goal-archive）。

仍在推进（消化顺序即列出顺序，todo 见账本 "HUD overlay and screen render
surfaces" 节；整体低优先）：

- 默认关闭的 debug entry 逐项 renderer。
- entity hitbox 的 local-server mirror（绿框 + delta 箭头）与 3D debug-text
  billboard 绘制。
- advanced tooltip 其余 component 专属行与选项持久化：逐条对照 vanilla
  `ItemStack.addDetailsToTooltip` 的 component 分派补齐，已实现者由代码确认。
- F3+I 本地实体 `saveWithoutId` 继续逐实体补全：对照各实体 vanilla
  `addAdditionalSaveData`，推进剩余实体族中仍需额外本地 owner、private timer /
  reference 投影、registry-backed variant 投影、codec-backed SNBT 投影，或实体
  专属 tropical fish variant、age、container、equipment 投影的保存状态。
- vanilla profiler section 全覆盖 + profiling metrics recorder/output。
- DebugOptionsScreen narration / focus / 完整 widget styling polish。
- native pause tick-freeze eligibility 与 PauseScreen 剩余 action/subscreen。

完成标准：新 UI/screen 工作不做临时配置 UI，启动配置仍从命令行进入；视觉
结果尽量用 deterministic screenshot/readback 验证。

### P3：资源与动态纹理泛化

仍在推进：

- broader non-profile dynamic texture loading。
- 资源包 override / custom model / datapack registry asset 的动态组合。
- 失败、pending、ready 状态必须有明确 fallback，不画 stale texture。

完成标准：动态资源路径区分 decode、cache、upload、ready 状态；submission
metadata 在缺 atlas entry 时仍可记录，折叠 geometry 可按 vanilla fallback
或等待策略处理。

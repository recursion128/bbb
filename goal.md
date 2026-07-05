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
  清单随完成度无限增长，属于账本不属于 TODO。逐项追踪型 checklist 放
  `docs/unsupported-features.md` 对应条目（或其 `docs/unsupported/` 明细
  文件），本文件只指向它。
- 随 slice 变化的快照数据（audit 计数等）不写入本文件；只记录动作与判据，
  不记录当时的数字。
- 本文件超过 300 行即触发归档瘦身。历史归属：`docs/goal-archive.md`
  （已完成目标）、`docs/unsupported-features.md`（feature 缺口账本）、
  memory（跨会话方法论与长期状态）。

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
- 实体 type id 常量一律取自 `bbb_protocol::entity_types`；粒子 type id 常量
  一律经 `particle_registry::particle_type_ids!` 宏从 `PARTICLE_TYPES_26_1`
  编译期派生；均不得内联数字。
- 实体热路径（spawn/apply/clone）只写窄组件；`EntityState` 投影仅限
  probe/serialize/debug 边界。
- world→renderer 每帧状态经 `runtime/render_extract.rs` 的 `RendererFrame`
  单次提交；不得在 pump 中新增散置 `renderer.set_*`。
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
  内联测试块。账本条目超 800 行拆 `docs/unsupported/`（见其 Update Rules）。

## 提交前门禁

- `cargo fmt --all --check`
- `git diff --check`
- `RUSTFLAGS='-D warnings' CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test --workspace`

单条 `-D warnings` workspace 命令覆盖全部 crate（含 control/net/protocol/
audio），2026-07-05 起取代原"workspace 无 -D + 六个 crate 逐个 -D"的九条
命令组合。纯文档/memory slice 只需 `git diff --check`。

## Slice 工作流（每轮）

P1 auto-start 是默认分支，只有新的 P0 blocker 能抢占；P0 clean 时不得停下
等待确认，也不因历史 ledger 文字重开 P0。

1. **收尾**：确认工作树；已验证未提交的 slice 先完成提交。
2. **audit**：`rg -n 'residual|fallback|unsupported'` 扫 crates/ 代码侧命中，
   逐个归类。全部命中均可归入下节"已知非 P0 分类"且未触发重开条件 ⇒ 无新
   P0 blocker。不与历史计数对比，只看是否出现新的不可归类命中。
3. **分支**：命中重开条件 → 开 P0 regression slice，只修 blocker，不顺手
   吸收视觉 parity；未命中 → 本轮立即从 P1 队列取首个可执行小 slice 进入
   实现，不等待确认。
4. **实现循环**（每个 slice）：
   1. 先 grep 当前实现，确认缺口真实存在（历史上多次出现"账本以为缺失但
      代码已实现"）；
   2. 定位 vanilla 26.1 源码依据（常量/公式/顺序）；已批量提取过的先查
      memory `vanilla-feature-quickref`；
   3. 实现 + focused test；随机行为用确定性 seed 或固定样本；
   4. 更新账本条目；已完成描述写入 `docs/goal-archive.md`（不是本文件）；
      本文件对应队列项划掉或改写其剩余部分；
   5. 过门禁，独立提交（英文祈使句单行 message，沿用仓库风格）。
5. **队列维护**：P1 子队列消化顺序见"渲染管线差异优先级"首行；当前子队列
   无可执行项时顺延下一条；新发现的工作按优先级插队并在账本立条目。

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
  render-type defensive panic、pack parser validation bail、unknown packet /
  component diagnostics、tests 和 docs 指针。
- `fallback`：colored debug/profile/terrain/HUD/map/test 或 vanilla fallback
  注释。

P0 完成标准：audit 能把每个命中归类为非 P0 或给出对应修复；相关测试断言
submission metadata，而不是只看顶点数量。

## 当前边界

- 狭义 P0 pipeline closeout 不再吸收开放式视觉 parity；closeout freeze 后
  连续多轮 audit 无新 P0 blocker；lighting / overlay / outline 无 blocker
  记录。
- P1-1 render-state/render-graph fidelity、P1-2 狭义实体 renderer closeout、
  P1-4 GUI lighting surface 均已收口；`RendererFrame` 逐字段提取时机清单已
  审完（明细见 goal-archive P1-1）。
- 架构重构两批次已完成并固化为上方结构不变量：2026-07-02 #1-7
  （2d19d2a3..cae7bcbf）与 2026-07-04 旧账清偿（1de8f14b..44244ea3）；明细
  见 memory `architecture-refactor-progress` 与 goal-archive。

## 渲染管线差异优先级

P1 子队列消化顺序：P1-1 target ownership 小 slice → P1-5 → P1-3 宽面；宽面
项只按能关闭 checklist 的最小 slice 进入。P2/P3 在 P1 队列清空后进入。

### P1-1：GPU Render-State / Render-Graph Fidelity

已完成项见 docs/goal-archive.md#p1-1gpu-render-state--render-graph-fidelity。

仍在推进：

- target ownership：main / itemEntity / translucent / particles / weather /
  clouds / entity_outline target 的 draw ownership 继续收紧；text / item /
  block / crumbling / line / selection 等 feature pass 的相对顺序继续按
  vanilla `LevelRenderer` 和 `FeatureRenderDispatcher` 拆分。render.rs 的
  step 方法 + `FRAME_STEPS` 已把此类调整降为"挪一个方法 + 改一行常量"。

完成标准：每个 GPU state slice 有 vanilla `RenderTypes.*`、shader json、
post-chain 或 `LevelRenderer` 依据；测试覆盖 render plan / pipeline key /
target order；能 readback 的视觉路径补 deterministic pixel proof。

### P1-2：实体专用 Renderer 行为

已完成项见 docs/goal-archive.md#p1-2实体专用-renderer-行为；狭义 closeout
已完成。

仍在推进：

- GPU `DISSOLVE` mask sampling 精度（后续视觉 parity，不阻塞其它队列）。
- 通用 `EntityRenderState` submit 管线（vanilla `EntityRenderDispatcher.submit`
  的泛化路径；当前唯一已知消费场景是 ItemPickupParticle 捡箭/三叉戟的
  3-tick 闪现，2026-07-05 自 P1-5 移入，payoff 低、按宽面最小 slice 进入）。

完成标准：每个实体差异先定位 vanilla renderer/model/layer 源码再改测试；
每个特殊 renderer branch 至少一个状态化测试；不再新增只验证 vertex count
的 textured regression。

### P1-3：物品、Frame 与第一人称表现

已完成项见 docs/goal-archive.md#p1-3物品frame-与第一人称表现（含 item
glint 全家族与 first-person 手持/手臂/use-pose 完成史）。

仍在推进：

- HUD / inventory：vanilla font / count / durability / cooldown / tooltip /
  screen depth behavior。
- creative inventory-tab preview（需要 creative screen state）与 entity
  preview 实际 GPU PIP drawing（后续视觉渲染面）。
- touchscreen snapback（宽面，按最小 slice 进入）。

完成标准：每个 item consumer 都以 vanilla `ItemDisplayContext`、display
transform 和 renderer 源码为依据；GUI/world 使用不同 lighting context 时
必须在测试或手动对比记录中说明。

### P1-4：GUI Lighting Surface / Entity-In-UI

狭义 surface closeout 已完成，无未完成项；已完成项与完成标准见
docs/goal-archive.md#p1-4gui-lighting-surface--entity-in-ui。creative
preview / GPU PIP 归 P1-3。

### P1-5：透明排序、粒子与 Level Events

已完成项见 docs/goal-archive.md#p1-5透明排序粒子与-level-events（含
falling_dust mapColor 全目录、provider 初速度与 alpha/color 档收敛、
collision/on-ground 接入、firework 与 entity/level event 粒子及声效、
particle-target carried submit、透明排序审计 + 跨 section 段间序修复
完成史）。

仍在推进：无 open 项。透明排序 2026-07-05 收口（跨 section 段间序修复，
审计确认段内 quad 序/合成序/粒子序/within-target 序均已一致）。逐 provider
追踪表放账本 particle 条目 `docs/unsupported/particle-runtime-vanilla-parity.md`
（30 个 todo 已清零，本文件不复制清单）；新增缺口一律走账本表流程（先加行/
立 todo 再切 slice）。

完成标准：每个 particle slice 记录 vanilla provider 类和精确公式；对随机
行为使用确定性 seed 或固定样本测试。

### P2：Terrain / Block Render Presentation

已完成项（fluid 反转背面、CardinalLighting、water_overlay、datapack
`cardinal_light` 解码、cube crack overlay）见 goal-archive P2 小节。

仍在推进：

- 继续核查 block render shape、face culling、AO、tint、biome tint、fluid
  overlay 与 vanilla 的差异。
- selection overlay、block entity 特殊 renderer、透明块排序等剩余
  presentation；完整模型形状 crack decal 随 block destroy presentation
  后续推进。
- 复核 terrain 与 entity/item 共用 atlas、mip、sampler、lightmap 时的状态
  差异。

完成标准：每个 block/render shape 差异必须有 vanilla source 或资源 JSON
依据；对视觉 slice 使用确定性 pixel/readback 测试或明确手动对比记录。

### P2：屏幕、HUD、字体与截图

仍在推进：

- vanilla font rendering：glyph atlas、shadow、bidi / style / color、width
  metrics。
- HUD：hotbar、crosshair、status bars、boss bars、titles、subtitles、debug
  overlay；screen 与 world pass 的深度/颜色 load/clear 顺序。
- Screens：inventory / container / merchant / recipe / book / sign /
  advancement 等 screen 的 vanilla 布局。
- Screenshot / readback：保证 renderer output 可稳定测试；将更多视觉 slice
  接入 deterministic screenshot/readback，而不是只依赖手动对比。

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

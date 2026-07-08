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
- `RUSTFLAGS='-D warnings' CARGO_TARGET_DIR=/tmp/bbb-target-main cargo check --workspace`
- `RUSTFLAGS='-D warnings' CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test --workspace`

单条 `-D warnings` workspace 命令覆盖全部 crate（含 control/net/protocol/
audio），2026-07-05 起取代原"workspace 无 -D + 六个 crate 逐个 -D"的九条
命令组合。`cargo check` 一条不可省：test 构建把 test-only 用法算作使用，
只跑 `cargo test` 会漏掉非测试（交付 bin）构建的 unused/dead-code lint。
纯文档/memory slice 只需 `git diff --check`。

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
- P1-1 render-state/render-graph fidelity、P1-2 狭义实体 renderer closeout、
  P1-4 GUI lighting surface 均已收口；`RendererFrame` 逐字段提取时机清单已
  审完（明细见 goal-archive P1-1）。
- 架构重构两批次已完成并固化为上方结构不变量：2026-07-02 #1-7
  （2d19d2a3..cae7bcbf）与 2026-07-04 旧账清偿（1de8f14b..44244ea3）；明细
  见 memory `architecture-refactor-progress` 与 goal-archive。
- P1 全部五个子队列 2026-07-05 清空（a0031d3a..bb4e8d34 loop run）：仅剩
  P1-3 一项 blocked（creative preview，等 creative-screen 基建）与账本记录
  的 defer 项（bidi/unihex、PIP item_layers GPU 绘制等），均有重启判据。

## 渲染管线差异优先级

P1 队列已清空（blocked/defer 项见各节与账本）；按队列规则 P2/P3 进入：
P2 两节按能关闭 checklist 的最小 slice 消化，新发现工作按优先级插队并在
账本立条目。

### P1-1：GPU Render-State / Render-Graph Fidelity

已完成项见 docs/goal-archive.md#p1-1gpu-render-state--render-graph-fidelity。

仍在推进：无 open 项。target ownership 伞形条目 2026-07-05 以逐 pass 审计
闭项：19 个 step 的写入 target 与 feature pass 相对序对照 vanilla
`LevelRenderer`/`FeatureRenderDispatcher`/`RenderTypes.setOutputTarget` 全部
aligned（bbb 建模 fabulous-on 路径），无绕过 step 编排的 draw 站点（审计
明细见 goal-archive P1-1）。新增 target/feature pass 按结构不变量进入
（step 方法 + `FRAME_STEPS` 登记 + vanilla 依据），不再保留常驻开放项。

完成标准：每个 GPU state slice 有 vanilla `RenderTypes.*`、shader json、
post-chain 或 `LevelRenderer` 依据；测试覆盖 render plan / pipeline key /
target order；能 readback 的视觉路径补 deterministic pixel proof。

### P1-2：实体专用 Renderer 行为

已完成项见 docs/goal-archive.md#p1-2实体专用-renderer-行为；狭义 closeout
已完成。

仍在推进：（空——最后一项 arrow/trident pickup carried 模型已于
2026-07-05 落地；vanilla 泛化 `EntityRenderDispatcher.submit` 的实际消费面
只有 ItemPickupParticle 的三类被捡实体，item/orb/arrow+trident 均已按各自
carried 路径覆盖，不再保留"通用 submit 管线"开放项。）

完成标准：每个实体差异先定位 vanilla renderer/model/layer 源码再改测试；
每个特殊 renderer branch 至少一个状态化测试；不再新增只验证 vertex count
的 textured regression。

### P1-3：物品、Frame 与第一人称表现

已完成项见 docs/goal-archive.md#p1-3物品frame-与第一人称表现（含 item
glint 全家族与 first-person 手持/手臂/use-pose 完成史）。

仍在推进：

- creative inventory-tab preview：blocked on creative-screen 基建（客户端
  `CreativeModeInventoryScreen` 外壳 + `CreativeModeTabs` 物品目录均缺失；
  玩家预览调用点零成本，随外壳落地折叠）。touchscreen snapback 判
  not-needed（vanilla 100% 由 `Options.touchscreen` 门控而 bbb 无 touch
  输入模式），判据记账本，不再列为开放项。

完成标准：每个 item consumer 都以 vanilla `ItemDisplayContext`、display
transform 和 renderer 源码为依据；GUI/world 使用不同 lighting context 时
必须在测试或手动对比记录中说明。

### P1-4：GUI Lighting Surface / Entity-In-UI

狭义 surface closeout 已完成，无未完成项；已完成项与完成标准见
docs/goal-archive.md#p1-4gui-lighting-surface--entity-in-ui。creative
preview 归 P1-3；entity preview 实际 GPU PIP drawing 已于 2026-07-05 完成
（完成记录归档在 goal-archive P1-4 段 entity-in-UI 小节，preview
item_layers 的 GPU 绘制仍为后续 entity-in-UI 子项）。

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

已完成项见 goal-archive P2 小节。2026-07-05 入口审计：原三条伞形核查项中
AO/face culling/render shape 烘焙/fluid overlay/selection/透明排序（段内+
段间）/atlas mip 与 sampler 状态均已对齐 vanilla；2026-07-08
`skipRendering` same-block / bars adjacency culling 也已闭项（判据与锚点见账本
"Terrain Block Presentation Parity" 条目），伞形措辞撤销，改为下列具体缺口
（消化顺序即列出顺序）：

- block-entity 特殊 renderer（player-head profile skin 随 P3 动态纹理/profile
  管线）：
  chest 全家族
  （2026-07-06 首片）、sign + hanging sign 板体 + 牌面文本（2026-07-06
  第二片）、bed + bell（2026-07-06 第三片：`createModelTransform` 转写 +
  cube emitter 补 vanilla `visibleFaces` 逐面可见性 + BlockEvent(1,dir)
  摇摆链）、shulker box + decorated pot（2026-07-06 第四片：box 17 色 ×
  六向 `Direction.getRotation()` 根变换 + BlockEvent(1,count) 开合状态机
  0.1/tick 双 progress lerp；pot BE NBT `sherds`（back/left/right/front）
  → 23 项 sherd→pattern 转写表四面选纹理 + `BlockEvent(1,style)` wobble
  POSITIVE(7 tick)/NEGATIVE(10 tick) 根变换）、banner（2026-07-06 第五片：
  16 色 × standing ROTATION 16 段/wall FACING 双形态 + BE NBT `patterns`
  逐层 tint 合成——base 色底 + ≤16 层 pattern 同 flag 几何重提交、
  `DyeColor.getTextureDiffuseColor` 逐 pass 顶点 tint + flag 摆动
  `(floorMod(x·7+y·9+z·13+gameTime,100)+partial)/100` 相位）、附魔台悬浮书
  + lectern 摆放书（2026-07-07 第六片：共享 `ModelLayers.BOOK` / `BookModel`
  + 单一 `enchanting_table_book` 纹理——附魔台 `bookAnimationTick` 每 tick 状态
  链（最近玩家 3 格朝向 + 0.1/tick 开合 + 确定性随机翻页），native 侧
  `extractRenderState` partial lerp + `submit` 浮动/翻页/根变换；lectern 纯
  `HAS_BOOK` state 派生 + `FACING.getClockWise().toYRot()` 固定开书）、conduit
  （2026-07-08 第七片：water/prismarine frame client tick + active cage/wind/eye
  分片 renderer）、skull/head（2026-07-08 第八片：standing ROTATION_16 / wall
  FACING placement、7 类 skull/head 纹理/模型分派、powered dragon/piglin
  animation tick）、end portal/gateway（2026-07-08 第九片：Y 轴 face source、
  gateway age/cooldown beam tick、BeaconRenderer beam geometry；2026-07-08
  第十一片：专用 `RenderTypes.endPortal()` / `endGateway()` 15/16-layer
  shader parity）、spawner 旋转体（2026-07-08
  第十片：`SpawnData.entity.id` NBT decode + `BaseSpawner.clientTick`
  spin/delay/range ticker + `SpawnerRenderer.submitEntityInSpawner` wrapper
  transform 复用实体模型流）已完成；判据与 defer 边界见账本
  "Terrain Block Presentation Parity" 条目；BE-driven model source 已清零；随行审计
  `Custom`→`Cube` shape 兜底命中清单。

完成标准：每个 block/render shape 差异必须有 vanilla source 或资源 JSON
依据；对视觉 slice 使用确定性 pixel/readback 测试或明确手动对比记录。

### P2：屏幕、HUD、字体与截图

font 全部子项已在 P1-3 五连 slice 完成（剩 bidi/unihex defer，见账本）。
2026-07-05 入口审计：hotbar/crosshair/生命/饥饿/经验条基础档、22 变体
container screen 家族、merchant 交易 UI、book 阅读均已完成；HUD 深度/颜色
load/clear 语义与 vanilla 对齐（stratum/blur depth-clear 随 blur 型 screen
再进入）。具体缺口（消化顺序即列出顺序，判据见账本 "HUD Overlay And
Screen Render Surfaces" 条目）：

- debug overlay（F3 基础 toggle、左列 version/position/help shell、右列
  memory/system/performance basics、F3+1..4 chart/lightmap toggle state、
  F3+B/G/H hitboxes/chunk-borders/advanced-tooltips toggle state、F3+A
  terrain reload request、F3+D clear-chat display action、F3+P focus-pause
  option toggle、F3+H advanced tooltip consumption、F3+V version debug chat
  action、F3+C copy-location clipboard action、F3+T resource-pack reload
  request、F3+A/B/C/G/H/N/P/F4/T local debug feedback、F3+F6 debug-options
  edit help keybind、default GAME_VERSION entry shape、default TPS entry
  shell、default FPS entry shell、F3+4 lightmap preview 实际绘制已完成；
  剩余完整 debug entry 列表、FPS/TPS/network charts 实际绘制、entity
  hitbox/chunk-border 实际绘制、advanced tooltip full parity/持久化、3D
  crosshair 与其它 F3 组合键，低优先）。

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

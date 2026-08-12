# 方块渲染：剔除、遮挡、AO、光照与 layer

机制 ID：`rendering-block-model-pipeline`

## 机制目的和边界

该机制把一个 block state 的 baked quads 变为 chunk layer 顶点，并复刻 Vanilla 的面
剔除、occlusion facts、ambient occlusion（环境遮蔽）、block/sky light、cardinal shade
与 render-layer routing。fluid 另见独立页面。

## Vanilla 26.1 权威实现

- `ModelBlockRenderer.renderModel` 按 quad direction 查询相邻坐标；只有 quad 声明 cull
  时才调用私有 `shouldRenderFace`，最终委托 `Block.shouldRenderFace`。
- `Block.shouldRenderFace` 先查邻居 opposite face 的 occlusion shape；邻居完整覆盖立即
  false；再查 `state.skipRendering(neighbor,direction)`；空邻形 true，空自身形 true，
  否则 `Shapes.joinIsNotEmpty(..., ONLY_FIRST)`。
- `BlockModelLighter` 从面、四角与对角查询 `getShadeBrightness`、`isViewBlocking`、
  `getLightDampening`、`isSolidRender` 和 packed light，并按 quad geometry 权重插值。
- `SectionCompiler` 根据 quad 的 material layer 路由 buffer，另以
  `BlockState.isSolidRender` 构建 section visibility；translucent layer 生成排序状态。

## 决策输入与状态所有权

`BlockBehaviour.BlockStateBase` 缓存或提供 `canOcclude`、`isSolidRender`、
`getFaceOcclusionShape`、`isViewBlocking`、`getLightDampening`、
`getShadeBrightness` 与 `skipRendering`。查询坐标是当前 block、目标相邻 block、对应面
以及 AO 四角/对角；这些彼此独立，render layer 或方块名称不能替代。

## 判断与处理顺序

1. baked model 选择 quad 与 material layer。
2. 对 cull quad 按 Vanilla 顺序比较邻居 shape、`skipRendering` 与自身 shape。
3. 根据 quad face/geometry 选择 base/center light；采样四角和必要对角。
4. 将 AO 权重、packed block/sky light、方向 shade、tint 与 vertex 数据组合。
5. 按 layer 提交；opaque/cutout 保持 depth，translucent 进入排序/混合路径。

## 生命周期和 fallback

configuration/runtime 以 global block-state id 查生成的 26.1 facts；world chunk extraction
携带它们。缺合法 id 时不假造 occlusion 等价事实，使用显式 conservative fallback 并
暴露诊断。chunk 邻居变化使相关 mesh 失效并重建。

### Level section residency 与可见性

Vanilla `LevelRenderer`、`SectionRenderDispatcher.RenderSection` 与
`SectionOcclusionGraph` 共同决定 section compile、脏化、frustum/occlusion traversal、
透明层 resort 和首次 resident fade。BBB 对应的数据流只有一条：

`bbb-world` chunk/section revision → `bbb-native::terrain_runtime::TerrainUploadState`
的 generation-aware bounded prepare/residency/有序 frame projection →
`bbb-renderer` GPU residency、每 section visibility uniform 与 layer submit。

native 在 mesh 首次成功 resident 时记录时间。与 camera section center 的距离平方小于
`768`，或该 section 从 previously-empty 变为非空时，fade duration 为零；其余 section
使用默认 750ms，并按 Vanilla 的整数毫秒进度生成 `ChunkVisibility`。recompile 保留首次
resident 时间，不重启 fade；level replacement、reload reset 或 section removal 同时移除
residency 和 projection。entity、普通 block entity 与 level-load readiness 都查询同一
projection，并统一以 `visibility >= 0.3` 放行。

renderer 不自行解析 world/pack，也不保留第二份 binary-visible owner。solid、cutout 与
translucent draw 绑定同一个小型 per-section uniform；embedded shader 和 selected Vanilla
`terrain.fsh` 均先执行 fog-color→terrain-color 的 `ChunkVisibility` mix，再执行距离 fog。
crumbling 保持自己的常量 visibility，不被 terrain section fade 污染。新 resident mesh 在
收到 generation-matched projection 前不自动可见；stale prepare/projection 在改写 residency
前被拒绝，失败继续使用上一完整 generation。

### Level scene projection 与 block selection

Vanilla `LevelRenderer.extractLevel`、`extract/submitVisibleEntities`、
`extract/submitVisibleBlockEntities`、`extract/submitBlockDestroyAnimation`、
`extract/renderBlockOutline` 与 `doEntityOutline` 的 15 个 scene owner 在 BBB 只经过一条数据流：

`bbb-world` entity/block/block-destruction 真值 → native 同帧 immutable projection → renderer-private
GPU mesh、outline target 与有序 pass submit。

block selection 的 shape 由 world 决定；当前 resource generation 的 block-state model catalog 枚举
weighted variant 的所有候选以及 multipart 的所有 selector，并以所有候选 material flag 的并集决定
提交 phase。solid selection 在 translucent terrain 前提交，含 translucent material 的 selection 在其后
提交。普通轮廓使用黑色 alpha 102、线宽 `max(2.5, surfaceWidth / 1920 * 2.5)`；high-contrast 使用
7px opaque black secondary 后提交 `0xff57ffe1` primary。world border、spectator/adventure gate 在 native
投影前执行；block-destroy overlay 只接收与 camera eye 的 block center 距离平方不超过 1024 的条目。

pack 只发布 immutable CPU model catalog，native 只做 generation-matched material/shape/scene 决策，
renderer 只保留 GPU residency 和 pass placement；三者之间没有 pack/world 反向读取或第二个 selection
resolver。resource reload 在 renderer transaction 内准备 terrain catalog，全部 fallible owner 成功后才
一次发布新 terrain generation；失败继续使用上一完整 generation。

### Level frame graph 与 transparency route

Vanilla `LevelRenderer.renderLevel`、五个 pass builder、五个 nullable target accessor、
`getTransparencyChain`、cloud renderer 和 `endFrame` 的 14 个 frame owner 由一个决策值控制：

`--improved-transparency` + panoramic-camera flag → native immutable
`LevelFrameRoute` projection → renderer-private target/depth/combine/presentation routing。

默认 FANCY 对应 regular route；panoramic camera 即使收到 improved 选项也强制 regular。regular
将 translucent/item/particle/cloud/weather 依次 load 到 main color/depth，不复制 main depth，也不
执行 transparency combine。improved 为五族选择独立 color/depth，main depth 只复制到
translucent、item 与 particles；cloud/weather 独立 clear depth，selected transparency graph 最终写回
main。graph generation 不可用时只有 renderer 内建 combine fallback，且 presentation 始终查询同一
route owner。

两条 route 都保持 clear→sky/main→entity outline post→cloud→weather→optional combine→late debug；
late debug 清 main depth，outline chain 不受 improved 选项控制。resource/post generation 仍由既有
prepare/apply transaction 原子发布，失败保留上一完整 generation；frame route 不缓存或改写 pack。

### Level effect tick 与 block-breaking lifetime

Vanilla `GameRenderer.tick` 只在 level tick 正常运行时调用 `LevelRenderer.tick`；后者先推进
renderer tick、生成 rain particle/audio effect，再按 20-tick cadence 清理 age 严格大于 400 的
block-breaking progress。`destroyBlockProgress` 以 breaker id 更新或移除条目，同一位置显示
`(progress, id)` 排序中的最高 progress。

BBB 的对应单向数据流是 packet/local interaction → `bbb-world` canonical breaking state 与
`block_destruction_render_stages` 有序值投影 → native overlay extraction → renderer crumbling pass。
server 与 local stage 只在 world owner 合并；native 不再保存 resolver/cache。`WeatherEffectRuntime`
唯一拥有跨 level/resource reload 的 weather tick/rain-sound cursor，native 先提交 weather effect，
再调用 world expiry；pause/frozen tick 不推进，server step 恢复推进。level teardown 清除 breaking
entries 和 local stage，但与 Vanilla 的长期 `LevelRenderer` 一样不重置 render/weather cursor。

local stage 的上游同样只有一条路径：world 的 `LocalPlayerInteractionState` 按 mining profile 和
selected-item signature 计算 progress/stage，native 在成功预留对应 `START/ABORT/STOP` command 后才允许
发布 prediction 或清理 target。queue admission 失败保留旧 stage；server block update/ack 再校正同一
world prediction。初次 start 与 creative continue 查询 world border，survival same-target continue 不
重复查询；目标变 air 时清 stage 且本 tick 不提交 hit particle/swing。因而 renderer 只消费最终有序值，
不能从 input latch、command queue 或 block 文件反推破坏进度。最终 local-air publication 同时消费
item catalog 机械提取的唯一 `Item.canDestroyBlock` 直接拒绝覆写（`DebugStickItem`）、Tool creative flag，
以及 block catalog 的 Tree-sitter `GameMasterBlock` class/interface membership；拒绝时仍保留 outbound
action，但不发布 local air prediction，且 source guard 禁止恢复手写物品/方块名表。

网络新增 breaker id 的 retained 上限为 4096；达到上限后已有 id 仍可更新/移除，新 id 被拒绝并计入
`block_destructions_ignored`，`block_destructions_tracked` 始终给出当前 retained count。stage projection
只分配本帧 immutable 值，不保留第二 generation；缺 destroy texture 继续产生零 overlay。

### Level diagnostics 与 gizmo finalize

Vanilla `GameRenderer.extract` 先完成 `LevelRenderer.extractLevel`，再提取 GUI；因此
`getEntityStatistics` 读取的是本帧 `extractVisibleEntities` 已写入的 render-state 数、level entity 总数
和 server simulation distance。BBB 对应地在唯一 entity visibility scratch 完成本帧 survivor 集后，
才构造 `E: visible/total, SD: distance`；survivor 数不从 model-instance 数反推，因为一个 entity 可以
产生多个 renderer piece。reduced-debug 的自定义 entry 仍可显示，未收到 distance 时保持 0。

所有外部多人可达 gizmo 经过同一个 frame-local owner：world/input/resource facts → native
`DebugRenderExtractionBuffers` → immutable `RendererFrame` → renderer debug passes。普通 box/rect/text/line
在 translucent features 后、translucent terrain 前 finalize；`setAlwaysOnTop` 的当前客户端调用均为文字，
它们在 composition 后清 depth 的 late pass 提交。空帧以 replace 清除旧动态 geometry，vector 只回收容量；
GameTest marker 按 level generation/revision 替换，remote brain selection 按 level generation 清除。

Vanilla 26.1 客户端源码没有 `persistForMillis` 或 `fadeOut` 的生产调用；client per-tick collector 因而没有
需要复制的长期 gizmo owner，IntegratedServer per-tick gizmo 已按外部客户端边界登记 not-needed。BBB
不增加第二个通用 collector/shadow lifetime；debug subscription packet 的 2 MiB payload、65,536 collection
value 上限和既有 world expiry 继续约束外部输入，renderer 不反向读取 packet、world 或 pack。

### Level packed-light query

Vanilla `getLightCoords(level, pos)` 先取得 `pos` 的 block state，再进入显式
`getLightCoords(brightnessGetter, level, state, pos)`。后者先判断
`state.emissiveRendering(level, pos)`，命中即返回 block/sky 均为 15；否则 brightness getter
按 sky→block 顺序读取两个 light layer 并打包，最后仅把 block channel 提升到
`state.getLightEmission()`。显式 overload 的 state 与亮度查询坐标不能合并：方块面偏移采样仍以
邻坐标读取 raw brightness，却继续使用正在渲染的 state；fluid/AO 的相邻 state 查询则使用各自事实。

BBB 从同一个 29,873-state runtime report 生成 `light_emission` 与 `emissive_rendering`，由
`bbb-render-types::VanillaLightCoords` 唯一执行 clamp、state override、component max、pack 与 normalize。
`bbb-world::sample_render_light_for_state` 只组合显式 state、`BlockPos` 和原始 light store；默认 query
再从该位置取得 state。terrain snapshot 仅携带两个 fixed-state fact，native block-entity、painting、
particle、weather 与 renderer terrain/fluid 都消费同一纯值 projection，不再各自打包或从 block name、
material/model 猜测。

Particle 在缺 chunk 时严格使用 block 0/sky 15；painting、sign 与 weather 保持各自既有缺失语义。
gameplay、audio mood、vignette 和 light debug 仍读取 raw light，不被 render query 改义。resource-model
element 的 `light_emission` 继续只作用于该 element，不能替代 state emission。state facts 随 protocol
版本固定；terrain mesh 仍只接受 immutable snapshot，level/resource replacement 或 stale worker result
不能部分改写上一 resident generation。

## BBB 实现映射

`bbb-protocol/src/block_state_types.rs::VanillaBlockStateRenderFacts` 和 support facts 是
26.1 projection；`bbb-world/src/chunks/store.rs` 按 global id 赋给 terrain cell；
`bbb-native/src/terrain_runtime.rs` 保真传入
`bbb-renderer/src/terrain.rs::TerrainCell`。剔除/AO/light/cardinal shade 在
`bbb-renderer/src/terrain/mesh/emitter.rs`，layer/透明提交在 terrain renderer。

### FallingDust 静态事实边界

`FallingDustParticle.Provider.createParticle` 的 context-free 决策同样归版本化 block-state
facts，而不由 native 按 resource name/property 重建。checked-in runtime probe 逐 state 记录：
非 air invisible rejection、`FallingBlock.getDustColor` 的 optional RGB，以及仅当
`BlockColors.getTintSource(state,0)` 缺失时才可用的 optional cached map RGB。

`bbb-protocol::block_state_types::vanilla_falling_dust_static_facts` 是唯一公开查询，输入只允许
bounded 26.1 global state id；非法 id 返回 `None`。native 先用 falling-block dust override，
否则查询当前 resource generation 的 dynamic/biome tint，最后才使用 generated optional map
color，并只把 resolved RGBA 交给 renderer。静态 facts 只随 protocol version 变化；dynamic
tint catalog 仍随 resource generation 原子替换，prepare 失败保留上一完整 generation。

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| face culling 顺序 | `ModelBlockRenderer`, `Block.shouldRenderFace` | terrain mesh emitter | `equivalent` | shape/skip/full-neighbor tests |
| 独立 occlusion facts | `BlockStateBase` | protocol → world → `TerrainCell` | `equivalent` | exact facts extraction test |
| AO corner sampling | `BlockModelLighter` | mesh AO sampling | `equivalent` | AO/cardinal fixture tests |
| packed block/sky query | `LevelRenderer.getLightCoords/BrightnessGetter` | generated facts → world explicit-state query → `VanillaLightCoords` | `equivalent` | generated 3-owner evidence、state/position/fallback/reverse-guard tests |
| AO/cardinal/model-element light | lighter + level shade | terrain vertex light | `equivalent` | flat/AO/fluid、model emission、cardinal anchors |
| layer routing | `SectionCompiler` material layer | terrain batches | `equivalent` | cutout/translucent tests + anchors |
| translucent sorting | section sort state | renderer translucent queue | `equivalent` | static/flight/boundary anchors |
| section compile/cull/fade lifecycle | `LevelRenderer`、`RenderSection`、`SectionOcclusionGraph` | native ordered section projection → renderer uniform | `equivalent` | generated 29-owner evidence、fade/threshold/generation/pixel/selected-shader tests |
| backend/reload/resize lifecycle | `LevelRenderer` constructor/close/reload/setLevel/allChanged/resize/resetSampler/needsUpdate | world revision → native generation transaction → renderer-private GPU backend | `equivalent` | generated 9-owner evidence、atomic resize/rollback、RAII、resource/level generation tests |
| level scene extract/submit | `LevelRenderer` 15 scene owners | world → native immutable frame projection → renderer pass/target | `equivalent` | generated 15-owner evidence、selection phase/style、32-block crack、offscreen pixel tests |
| level frame graph | `LevelRenderer` 14 frame owners | CLI/native `LevelFrameRoute` → renderer targets/passes | `equivalent` | generated 14-owner evidence、regular/improved pixels/counters、selected graph rollback |
| level effect lifetime | `LevelRenderer` 4 effect owners | world breaking state + native weather tick → immutable effect/overlay consumers | `equivalent` | generated 4-owner evidence、20/400/freeze/teardown/budget/source-guard tests |
| level diagnostics/gizmos | `LevelRenderer` 3 diagnostics owners | native current-frame visibility/debug projection → renderer standard/late passes | `equivalent` | generated 3-owner evidence、visible/total、generation/stale-clear/pass-order tests |
| falling-dust static color facts | `FallingDustParticle.Provider` | protocol generated facts → native tint decision | `equivalent` | generator stale check + 29,873-state/family tests |

## 结构性不变量

- 不得从 `Opaque/Cutout/Translucent`、render shape 或 block name 推导独立 state facts。
- 查询相邻坐标、opposite direction 与判断顺序必须与 Vanilla 一致。
- packed-light 的 state、brightness query 坐标与 model-element emission 必须保持分离；生产 consumer
  不得恢复 raw-light pack helper 或各自的 clamp/normalize resolver。
- chunk 边缘需要跨 chunk 邻居；unloaded 不能伪装成 air 后永久缓存。
- section fade、entity/block-entity/readiness gate 必须来自同一个 generation-matched
  projection；不得恢复 renderer 自动可见 bridge 或独立 binary-visible cache。
- world 只发布 level/resource revision，native 只拥有 terrain/resource transaction；surface、
  targets、post graphs、sampler/pipeline 和 GPU residency 只由 renderer backend 拥有。下游不能
  反向读取 world/pack，也不能增加可独立漂移的 backend generation。
- resize 必须先准备全部 size-dependent owner，再一次性发布 main/depth、transparency、outline、
  cloud、blur、camera/post targets、bind group 与尺寸/counter；任一 prepare 失败保留上一完整
  backend generation，zero-size 不进入 prepare。释放由 renderer RAII owner 完成，不另设 close 路径。
- block selection phase 必须来自当前 generation 的完整 block-state model material candidate set；不得
  恢复 native/renderer 的按名称、单候选或独立 pack resolver，也不得在两个 phase 同时提交。
- `LevelFrameRoute` 是 feature target、depth copy、combine 与 presentation 的唯一决策 owner；不得恢复
  以 post catalog 是否存在代替 option/panoramic 判断的无条件 improved 路径。
- block-breaking 最高 stage 只能由 world 值投影解析；不得在 native/renderer 恢复 server/local merge、
  progress cache 或独立 expiry cursor。weather effect 必须先于同 tick expiry 和后续 entity effects。
- 性能优化必须重跑石材、玻璃、叶子、发光石与 chunk-boundary ROI。
- FallingDust 不得恢复按 block name/property 匹配 dust/map color；dynamic tint 必须先于
  optional static map fallback。

## 聚焦测试（focused tests）和真实证据

`extracted_terrain_carries_exact_render_occlusion_facts` 验证 protocol facts 到 world；
renderer mesh tests 覆盖内部面、shape、AO、方向光照、邻 chunk 和透明面。共同 replay
在 `static_orbit`、`preloaded_flight`、`chunk_boundary_crossing` 保存同 tick ROI 与
RenderDoc 非空 draw evidence。

本机制的可执行 focused contract 精确绑定下列行为测试，不以 source-string guard
代替 world/mesh 结果：

- `crates/bbb-world/src/chunks/tests/terrain.rs#extracted_terrain_carries_exact_render_occlusion_facts`
  锁定 stone/glass/leaves 的 solid、occlusion、view-blocking、light-dampening、shade 与 face-shape facts；
- `crates/bbb-world/src/chunks/tests/terrain.rs#terrain_skip_rendering_uses_current_session_bars_tag_membership`
  锁定 `skipRendering` 的 current-session tag 输入，禁止名字或固定 ordinal 代替；
- `crates/bbb-renderer/src/terrain/mesh/tests.rs#box_model_culls_only_faces_marked_by_cullface`
  执行 full-neighbor 与 partial own-shape 的非对称 cull 结果；
- `crates/bbb-renderer/src/terrain/mesh/tests.rs#face_culling_consumes_vanilla_face_shape_palette_not_render_cuboids`
  执行 empty/full/partial shape palette 的 `ONLY_FIRST` 方向性；
- `crates/bbb-renderer/src/terrain/mesh/tests.rs#cross_chunk_slab_neighbor_keeps_partial_face`
  验证 chunk 边界邻居查询不会把 partial face 误剪掉；
- `crates/bbb-renderer/src/terrain/mesh/tests.rs#ambient_occlusion_queries_corner_blockers_beyond_side_brightness_samples`
  验证 AO 对角 blocker 与 side brightness 为独立输入；
- `crates/bbb-renderer/src/terrain/mesh/tests.rs#terrain_diffuse_vertices_carry_vanilla_default_cardinal_shade`
  锁定六面 default cardinal shade；
- `crates/bbb-renderer/src/terrain/mesh/tests.rs#section_layer_builder_reuses_texture_layer_routing_without_changing_meshes`
  验证 solid/cutout/translucent 只按 material 路由且不改写 mesh。

这些 focused tests 只关闭实现阶段的决策合同；冻结的 stone/glowstone/leaves/glass
ROI、三个 replay 的结构断言和 draw receipt 仍必须在最终 rendering domain sweep 中重新采集。

section lifecycle 由 native 的 near/previously-empty、far 0→0.3→1、recompile、level
replacement、stale generation 和三个 consumer 共享阈值测试，以及 renderer 的 uniform
layout、旧 bridge reverse guard、translucent resort 和 offscreen pixel fade 测试覆盖；本地
Vanilla 26.1 selected terrain shader 的 compile、override/filter 与 rollback 测试验证同一
consumer。版本化 client mechanism inventory 为全部 29 个 owner 保存实现 symbol 与语义测试
symbol，docs lint 对缺 cell、失效 symbol 和 stale output fail closed。

backend lifecycle 另由 renderer resize plan 的 success/zero-size/分阶段注入失败测试锁定：
prepare 不修改旧尺寸、targets、selected post pixels 或 counters，apply 后所有 target 同尺寸且只
推进一次 counter。level replacement/reload-all/stale mesh、resource reload 成功/失败、outline/cloud、
atlas sampler owner 与 RAII reverse guard 覆盖非 resize owner；generated inventory 为 9 个 Vanilla
method 分别保存 implementation/test symbol，旧逐 consumer resize 调用恢复时 source guard 失败。

scene projection 由 native 的 entity/passenger/camera、block-entity fade/global/crumbling、world-border、
surface-width/high-contrast、resource-generation material phase 和 32-block crack tests 覆盖；renderer
offscreen test 在真实 GPU 后端分别执行 solid/translucent phase，并验证 cyan high-contrast pixels 与唯一
logical selection draw。生成 inventory 为 15 个 Vanilla method 分别保存 implementation/test symbol，
fixture 对缺 cell、失效 symbol、旧 block-selection setter 和双 submission path fail closed。

frame graph 由 CLI default/explicit、panoramic suppression、唯一 native projection、regular/improved
target/depth/load/copy/combine source guard、selected graph generation rollback 与 AMD/RADV whole-frame
readback 覆盖；regular/improved 都保留 clear color 与 cyan block-selection pixels，只有 improved 增加
combine draw。generated inventory 为 14 个 owner 保存非空 implementation/test cell，fixture 对缺 cell
和 stale output fail closed。

effect lifecycle 由 world 的 id/position update、invalid removal、server/local highest-stage、4096 retained
budget、20/400 expiry、level teardown/cursor 测试，以及 native 的 deterministic weather、freeze/step、
weather→expiry→later-effect 顺序、crack/block-entity crumbling 与 missing-texture fallback 测试覆盖。
reverse guard 要求 production 只调用一次 world stage projection，并禁止 native 旧 resolver 恢复；generated
inventory 为 4 个 owner 保存 implementation/test cell，缺 cell、失效 symbol 或 stale output 均失败。

diagnostics 由 current-frame visible/total 测试、single collect/commit/recycle reverse guard、GameTest/remote
generation tests、empty-frame stale clear 以及 standard/always-on-top pass/depth tests 覆盖；generated
inventory 为 3 个 owner 保存 fail-closed implementation/test cell。

light query 由 runtime generator 的非法 emission/emissive 与 stale-output fixtures、protocol 的
magma/conduit/redstone-torch/sculk state、world 的 explicit-state/query-pos/raw-light 分离、renderer 的
flat offset/AO/fluid/state emission，以及 native block-entity max、painting、particle sky-only fallback、
weather 和 stale mesh tests 覆盖。reverse guard 禁止 12 个私有 pack helper 和这些生产 caller 的 raw
query 绕路恢复；generated inventory 为两个 overload 与 `packedBrightness` 共 3 owner 保存 fail-closed cell。

FallingDust 另由 support-fact generator fixtures/stale check、protocol 全量 palette/代表 state
断言和 native provider rejection、dust→dynamic→map、颜色完整性及 random-sequence tests 验证；
reverse guard 禁止旧 handwritten resolver 恢复。

## 当前差异

当前差异只取 parity matrix 与 ledger。新 block state、pack model、section
generation/visibility 或 renderer 优化仍需按机制复验，不能靠全图平均误差宣称等价。

## 账本/goal 路由

本机制的 active/partial 状态只由 parity matrix 与 ledger 路由。

## 精确重开条件

目标版本改变 block-state 表、渲染 layer 或 Vanilla 判断顺序，
或关键 ROI 回归时重开。

## 相关页面和源码索引

- [fluid rendering](fluid-rendering.md)
- [性能与验证](../../architecture/performance-and-validation.md)
- Vanilla：`BlockModelLighter`、`ModelBlockRenderer`、`Block`、`BlockBehaviour`、`SectionCompiler`

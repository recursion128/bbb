# 目标：bbb 渲染管线与 Minecraft Java 26.1 对齐

## 总目标

让 `bbb` 的 renderer / world / native / pack / protocol 边界逐步对齐
Minecraft Java 26.1 官方客户端行为。以 `docs/unsupported-features.md` 为事实清单：
除明确决定不支持的 feature 外，其余 renderer 差异必须收敛为 repo-native 实现。

历史状态不在本文件继续堆叠维护；详见：

- `docs/unsupported-features.md`
- `~/.claude/projects/-home-zgy-Work-bbb/memory/entity-texture-path-status.md`
- 相关专项 memory，例如 entity metadata / proxy entity / texture path 状态

本文件只保留下一阶段目标、未完成项、优先级和完成标准。

## 硬性约束

- 严格按 vanilla 26.1 源码转写：优先使用 `~/Work/mc-code/sources/26.1/`。
- 协议、metadata 索引、render type、layer order、transform、texture path、tint、light、overlay 不能凭记忆实现。
- `bbb-renderer` 不得依赖 `bbb-pack`。
- 每个 slice 保持小而可合并，避免顺手重构和无关格式 churn。
- 每个 slice 必须更新 `docs/unsupported-features.md`；需要长期状态记录时同步更新 memory。
- 提交前默认门禁（当前实际 gate）：
  - `cargo fmt --all --check`
  - `git diff --check`
  - `CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test --workspace`
  - `RUSTFLAGS='-D warnings' CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test -p bbb-renderer --quiet`
  - `RUSTFLAGS='-D warnings' CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test -p bbb-world --quiet`
  - `RUSTFLAGS='-D warnings' CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test -p bbb-pack --quiet`
  - `RUSTFLAGS='-D warnings' CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test -p bbb-native --quiet`

## 当前边界

- 狭义 P0 pipeline closeout 不继续吸收开放式视觉 parity。
- P0 lighting / overlay / outline 当前没有 blocker 记录。
- closeout freeze 后连续三轮 audit 均未发现新的 direct mesh bypass、
  texture-backed / dispatch-owned submission gap，或
  RenderType/order/submit_sequence/missing-atlas/dynamic texture/light/overlay/
  outline 相关 P0 blocker。
- 最新 audit 计数：
  - `rg residual`：38 行
  - `rg fallback`：526 行
  - `rg unsupported`：152 行
- 当前 renderer code-side 分类：
  - `residual` 只剩 `entity_models/dispatch.rs` 的“无 residual mesh-emitting arm”注释。
  - `unsupported` 是 screenshot surface format bail、dynamic-player texture
    render-type defensive panic 和 docs 指针。
  - `fallback` 是 colored debug/profile/terrain/HUD/map/test 或 vanilla fallback 注释。
- 用户已明确恢复 P1 post-closeout parity；当前按 P1-1 狭义 render-state /
  render-graph fidelity 推进。只有命中下方重新打开条件时才回到 P0。

## Slice 选择顺序

1. 每轮先确认当前工作树和未提交 slice；如果已有已验证但未提交的小 slice，
   先完成提交。
2. 运行或复核 quick P0 audit：`rg residual`、`rg fallback`、`rg unsupported`，
   并只按下方 “P0 重新打开条件” 判断是否回到 P0。
3. 只有出现新的 P0 blocker 时，才开启 P0 pipeline regression slice。
4. 如果没有新的 P0 blocker，同一轮自动选择并开启下一条 P1 slice；不再停下来
   等待人工确认是否继续 P1。
5. P1 slice 仍按 `P1-1` render-state / render-graph fidelity 优先；只有
   `P1-1` 当前 checklist 收口后，才转入 `P1-2` / `P1-3`。

## P0 重新打开条件

只有 audit 发现以下任一情况时，才重新开启 P0 pipeline slice：

- 新的 texture-backed direct mesh bypass 或 residual emit 路径。
- texture-backed / dispatch-owned submission 路径出现主要遗漏。
- RenderType / `order` / `submit_sequence` / missing-atlas / dynamic texture 状态缺失。
- light / overlay / outline 的 pipeline 表达出现 blocker。
- GPU path 的更细粒度状态阻塞上述 submission / render-state 表达，而不只是视觉精度后续。

GPU path 仍可继续按后续 parity 改进，但不阻塞狭义 P0 pipeline closeout，除非它直接命中上述条件。

## 渲染管线差异优先级

阶段标记：

- `P0`：只有 audit 重新打开 checklist 时才推进。
- `P1`：post-closeout renderer parity，恢复后优先推进。
- `P2`：terrain / screen / HUD / screenshot 等较宽视觉面。
- `P3`：资源、动态纹理、资源包和 datapack 泛化。

### P0：Pipeline Regression Blocker（条件触发）

目标：只处理重新破坏狭义 pipeline 的问题，不吸收视觉 parity。

仍在推进：

- residual emit / direct mesh bypass：
  - 新增 texture-backed renderer 或 layer 时，如果绕过 submission 直接写 mesh，
    立即回到 P0。
  - `render_textured_submission` / dynamic texture submission helpers 之外出现新的
    texture-backed mesh 写入入口，立即回到 P0。
- submission metadata：
  - texture、render type、tint、transform、light、overlay、outline color、
    `order`、`submit_sequence` 任一项缺失时，下一 slice 先补 metadata。
  - missing-atlas / pending dynamic texture 不能丢 submission-first 记录。
- RenderType 语义：
  - `entityCutout` / `entityCutoutCull` / `entityCutoutZOffset` /
    `entityTranslucent` / `Eyes` / `EnergySwirl` 等出现合并或退化时，先恢复
    vanilla-shaped 表达。
- light / overlay / outline：
  - 新 layer 或 renderer 不能继承错误 light / overlay。
  - invisible、self-visible translucent、hidden glowing outline 分支不能回退到
    普通可见路径。

完成标准：

- audit 能把每个 residual / fallback / unsupported 命中归类为非 P0，或给出
  对应 P0 修复。
- 相关测试断言 submission metadata，而不是只看顶点数量。

### P1-1：GPU Render-State / Render-Graph Fidelity

目标：把 CPU submission metadata 进一步映射成更接近 vanilla 的 GPU state、
target 和排序，而不是长期停留在粗 bucket 折叠。

仍在推进：

- render-state key 拆分：
  - [x] `entityTranslucentEmissive` 从 generic `Eyes` 分离：BreezeEyes 和
    Warden `LivingEntityEmissiveLayer` 提交进入独立 `translucent_emissive`
    mesh / GPU pipeline，保留 vanilla alpha cutout、overlay、per-face lighting，
    并跳过 LightTexture。
  - [x] `entityGlint` / `armorEntityGlint` 基础 GPU path：trident foil 和
    armor foil 提交进入独立 glint mesh / GLINT blend / depth-equal pipeline，
    跳过 LightTexture；shader 现在读取 camera uniform 中按 vanilla
    `TextureTransform.setupGlintTexturing` 推进的动态 offset（默认
    `Options.glintSpeed = 0.5`，`110000` / `30000`ms 双周期）。其他 item
    glint 变体仍属后续 shader/state 细化。
  - [x] no-cull / cull / translucent / translucent-cull item-target baseline
    GPU state：generic `entityCutout` / `entityCutoutCull` /
    `entityTranslucent` / `entityTranslucentCullItemTarget` 路径已用显式
    surface constants 固定 vanilla replacement-vs-translucent blend、
    depth-write `LESS_EQUAL`、LightTexture/overlay、per-face-vs-single-face
    lighting，以及 cull-on / cull-off pipeline 分流；item-target 变体仍只在
    `OutputTarget.ITEM_ENTITY_TARGET` helper 中绘制。
  - [x] `entityCutoutZOffset`：CustomHead skull 和 Shulker 等
    `entityCutoutZOffset` submission 进入独立 `cutout_z_offset` mesh，并给
    static atlas、dynamic player-skin atlas、dynamic profile-texture atlas 都保留
    对应 z-offset bucket。GPU 主 pass 使用专用 cutout-z-offset pipeline，保持
    vanilla `ENTITY_CUTOUT_Z_OFFSET` 的 `ALPHA_CUTOUT 0.1`、LightTexture、
    overlay、`PER_FACE_LIGHTING`、replacement blend、depth-write 和 cull-off
    state，并通过 camera uniform 暴露 vanilla `VIEW_OFFSET_Z_LAYERING`：
    perspective 使用 `scale(1 - 1/4096)`，orthographic 使用
    `translate(z = 1/512)`。
  - [x] `Eyes` emissive alpha blend：spider / enderman / phantom /
    ender-dragon 等 `RenderTypes.eyes` 提交保留独立 eyes mesh / shader，
    GPU pipeline 使用 vanilla `BlendFunction.TRANSLUCENT`、depth-write
    disabled、depth-test `LESS_EQUAL`、cull off、EMISSIVE / NO_OVERLAY /
    NO_CARDINAL_LIGHTING shape，并跳过 LightTexture。
  - [x] `armorCutoutNoCull` / `armorTranslucent`：humanoid / armor-stand /
    wolf / wings / saddle / decor equipment submission 进入专用
    `armor_cutout`、`armor_translucent`，以及动态 profile texture
    armor cutout mesh。GPU 使用 vanilla-shaped armor pipeline：alpha cutoff
    0.1、`NO_OVERLAY`、`PER_FACE_LIGHTING`、LightTexture 绑定、
    depth-write `LESS_EQUAL`、cull off、`VIEW_OFFSET_Z_LAYERING` layered
    view-projection，且仅 `armorTranslucent` 启用 translucent alpha blend。
  - [x] `breezeWind` lightmap-lit scroll：wind charge 和 BreezeWindLayer
    提交进入独立 scroll mesh / shader，使用 vanilla
    `BlendFunction.TRANSLUCENT`、depth-write `LESS_EQUAL`、cull off、
    lightmap-lit、NO_OVERLAY / NO_CARDINAL_LIGHTING texture-matrix scroll；
    更细的 cross bucket 透明排序仍归 sorting 项。
  - [x] `energySwirl` emissive additive scroll：charged creeper / wither
    overlay 提交进入独立 additive scroll mesh / shader，使用 vanilla
    `BlendFunction.ADDITIVE` (`ONE`, `ONE`)、depth-write `LESS_EQUAL`、
    cull off、alpha cutout 0.1、EMISSIVE / NO_OVERLAY shader shape，并跳过
    LightTexture 采样；更细的跨 bucket 透明排序仍归 sorting 项。
  - [x] `waterMask` depth-only boat mask：wooden boat / chest boat
    `BoatRenderer.submitTypeAdditions` 进入独立 water-mask mesh / pipeline，
    使用 vanilla `RenderPipelines.WATER_MASK` 的 color write mask 0、
    depth-write `LESS_EQUAL`、默认 back-face cull、无 texture / LightTexture 绑定，
    并用 `ModelLayers.BOAT_WATER_PATCH` geometry。
  - [x] `crumbling` block-destroy overlay GPU state：本地 destroy-stage overlay
    仍在 vanilla `crumblingBufferSource.endBatch()` 对应 phase 绘制到 main
    target，pipeline 现在使用 vanilla `RenderPipelines.CRUMBLING` 的
    `DST_COLOR` / `SRC_COLOR` blend、alpha cutoff `0.1`、depth-write off、
    `LESS_EQUAL` + polygon offset `-1.0F, -10.0F`。完整 block model-shaped
    crack decals 仍属后续视觉/geometry parity。
  - [x] `entitySolidZOffsetForward` item-frame block model：item-frame /
    glow-item-frame visible border 从普通 block-item solid bucket 拆到
    `solid_z_offset_forward` mesh，GPU 使用专用 item-model pipeline 读取 camera
    uniform 的 `VIEW_OFFSET_Z_LAYERING_FORWARD` layered view-projection；对应 vanilla
    `BlockModelRenderState.submitWithZOffset` /
    `RenderTypes.entitySolidZOffsetForward(TextureAtlas.LOCATION_BLOCKS)`。Painting
    custom geometry 和更精确的 entity-solid shader/cull parity 仍属后续 P1/P2。
  - [x] `item_cutout` / `item_translucent` alpha cutoff：block/flat
    item-model shader 现在匹配 vanilla `core/item.fsh` 和
    `RenderPipelines.ITEM_CUTOUT` / `ITEM_TRANSLUCENT`，先按 atlas texture
    alpha `< ALPHA_CUTOUT 0.1` discard，再应用 submitted tint / vertex color；
    `entitySolidZOffsetForward` item-frame block-model variant 继承同一 cutoff
    顺序。剩余 item foil / special-display variants 仍属后续 P1 细化。
  - [x] `ThrownItemRenderer` legacy billboard alpha cutoff：保留 billboard path
    的 thrown-item projectiles 现在也按 vanilla `ItemStackRenderState.submit` /
    `RenderTypes.itemCutout` / `itemTranslucent` shader shape，先以 item atlas
    alpha `< ALPHA_CUTOUT 0.1` discard，再乘 submitted tint；depth / target 的
    更细 item-special parity 仍属后续。
  - [x] `end_crystal_beam` / guardian beam custom prism state：Guardian attack
    beam、EndCrystal target beam、EnderDragon healing beam 均已通过
    dispatch-owned submission-first 路径记录 vanilla render type、texture、
    tint、transform、light、overlay、`order` / `submit_sequence`，并把自定义
    beam prism geometry 折入 scroll bucket；missing-atlas 覆盖已证明提交先于
    folded geometry。
- target ownership：
  - main target、itemEntity target、translucent target、particles target、
    weather target、clouds target、entity_outline target 的 draw ownership 继续收紧。
  - [x] selection / line append pass：block selection、entity-scene outline、
    entity-target outline 继续写入 itemEntity target 且在 particles 前绘制；
    GPU pipeline 现在使用 vanilla `RenderTypes.lines()` 的
    `VIEW_OFFSET_Z_LAYERING`、translucent blend、depth-write `LESS_EQUAL`、
    普通 block-hit outline `ARGB.black(102)` alpha。屏幕空间线宽与
    high-contrast secondary outline 仍属后续视觉 polish。
  - text / item / block / crumbling / line / selection 等 feature pass 的相对顺序
    继续按 vanilla `LevelRenderer` 和 `FeatureRenderDispatcher` 拆分。
- sorting：
  - [x] blended texture-backed model submit 的 draw-plan sort：main
    translucent 与 itemEntity target 的 `EntityModelTexturedDrawRange` 现在携带
    `order`、camera-distance、insertion index，并有测试直接验证
    `SubmitNodeCollector.order` 优先、同 order 内远到近、等距按提交插入顺序稳定。
  - [x] main translucent feature pass 的 textured / eyes / scroll / additive-scroll
    combined draw plan：BreezeWind、EnergySwirl、Eyes、entityTranslucent /
    entityTranslucentEmissive 等 draw range 共享 `order`、camera-distance、
    insertion index 排序；Breeze `WindLayer` 与 `EyesLayer` 同为 `order(1)` 时
    保持 vanilla layer 注册顺序（wind 先于 eyes）。非 `sortOnUpload` 的
    EndCrystal / Guardian beam scroll range 也进入 range draw，避免 combined
    plan 存在时漏画。
  - [x] terrain translucent 与 entity translucent 的跨 target / cross bucket 顺序：
    depth copy 后先运行 main-target `renderTranslucentFeatures`，再运行
    itemEntity target / block-destroy overlay，随后才是 translucent target 的
    terrain translucent pass；测试固定 `LevelRenderer` 的 target 边界和
    renderer-owned target 写入。
  - [x] particles translucent order 与 itemEntity target 的交界：
    itemEntity target feature pass 和 itemEntity line append 都在 particle target
    之前，transparency combine shader 也按 translucent / itemEntity /
    particles / weather / clouds 插入透明层。
- shader / sampler state：
  - [x] entity texture atlas sampler / mip baseline：static entity atlas、
    dynamic player-skin atlas、dynamic profile-texture atlas 统一走显式
    clamp-to-edge / nearest sampler helper，并固定 single-mip 上传策略；完整
    vanilla mip generation / standalone texture sampler parity 仍是后续更细粒度
    shader-resource 工作。
  - [x] entity / armor glint dynamic texture offset：`entityGlint` scale
    `0.5` 与 `armorEntityGlint` scale `0.16` 保持独立 shader，uniform 记录
    vanilla `Util.getMillis() * glintSpeed * 8` 派生的 `-layerOffset0` /
    `layerOffset1`，shader 在 scale + `rotateZ(π/18)` 后应用 translation。
  - [x] view-offset z layering：`entityCutoutZOffset`、
    `armorCutoutNoCull` / `armorTranslucent`、`armorEntityGlint` shader 使用
    `LayeringTransform.VIEW_OFFSET_Z_LAYERING` 的 layered view-projection
    矩阵；item-frame border 的 `entitySolidZOffsetForward` path 使用
    `VIEW_OFFSET_Z_LAYERING_FORWARD`，普通 `entityGlint` 继续使用未偏移矩阵。
  - [x] entity shader alpha cutoff ordering：`entityCutout` /
    `entityCutoutCull` / `entityCutoutZOffset` / `entityTranslucent` /
    `entityTranslucentCullItemTarget`、armor cutout/translucent、`breezeWind`
    和 `energySwirl` GPU shader 现在按 vanilla `core/entity.fsh` 先以 texture
    sample alpha `< ALPHA_CUTOUT 0.1` discard，再应用 submitted tint；glint
    保持独立 `core/glint.fsh` 形状。
  - [x] weather `WEATHER_DEPTH_WRITE` render-state：target-backed rain/snow
    weather pass 现在用显式 GPU 常量和测试固定 vanilla shader-transparency
    分支：`core/particle` shader shape、`DefaultVertexFormat.PARTICLE`、
    Sampler0 + Sampler2 LightTexture、`BlendFunction.TRANSLUCENT`、cull off、
    depth-write `LESS_EQUAL`。`WEATHER_NO_DEPTH_WRITE` 属于非 shader
    transparency 分支，不阻塞当前 target-backed render graph。
  - [x] lightning `RenderTypes.lightning()` weather-target state：lightning
    bolt geometry 继续写入 `OutputTarget.WEATHER_TARGET`，GPU pipeline 现在用
    显式常量和测试固定 vanilla `RenderPipelines.LIGHTNING` 的
    `core/rendertype_lightning` shader、`DefaultVertexFormat.POSITION_COLOR`、
    `BlendFunction.LIGHTNING`、默认 back-face cull、depth-write `LESS_EQUAL`；
    quad 顶点顺序仍按 vanilla `LightningBoltRenderer.quad(...)` 生成。
  - [x] clouds `CLOUDS` / `FLAT_CLOUDS` cull split：renderer 现在为 fancy
    clouds 和 flat clouds 创建独立 GPU pipeline，并按 `CloudShape` 选择；
    fancy `CloudStatus.FANCY` / vanilla `RenderPipelines.CLOUDS` 使用默认
    back-face cull，flat `RenderPipelines.FLAT_CLOUDS` 保持显式 no-cull，
    二者共享 `rendertype_clouds` shader、translucent blend 和 depth-write
    `LESS_EQUAL`。
  - per RenderType 的 blend、depth write/test、cull、sampler、mip、lightmap、
    overlay、fog、normal diffuse 组合继续拆细。
  - glint / scroll / emissive path 不应只依赖普通 entity shader fallback。

完成标准：

- 每个 GPU state slice 有 vanilla `RenderTypes.*`、shader json、post-chain 或
  `LevelRenderer` 依据。
- 测试覆盖 render plan / pipeline key / target order；能 readback 的视觉路径补
  deterministic pixel proof。

### P1-2：实体专用 Renderer 行为

目标：补齐已经有模型和贴图但 renderer 行为仍缺官方细节的实体。

仍在推进：

- Chicken / pig / cow variant livestock：
  - variant sound。
  - custom/datapack variant assets。
- Spider / slime / magma cube / ghast / blaze / endermite / silverfish / vex /
  allay / phantom：
  - death flip。
  - particle/audio coupling。
  - crumbling。
- Minecart：
  - rail-follow `posOnRail` / `frontPos` / `backPos` 平移与坡度 pitch。
  - NewMinecartBehavior exact weighted `renderPos` interpolation。
  - display block transform / content / light hookup。
- Equine / camel / llama / goat / hoglin / ravager 等大型模型：
  - boost 等 remaining renderer 状态。
  - camel body-anchor y-offset。
  - roar / particle / sound / converting shake。
- Humanoid / illager / piglin / skeleton family：
  - remaining arm poses。
  - use-item sway。
  - attack / crossbow / spell / celebrate / riding 组合冲突。
  - held item 与 use-item 的 layer/order 交互。
- Boss / beam / emissive 类：
  - EnderDragon dying-dissolve render type 等非 beam/emissive 视觉后续。

完成标准：

- 每个实体差异必须先定位 vanilla renderer/model/layer 源码，再改测试。
- 不再新增只验证 vertex count 的 textured regression。
- 对每个特殊 renderer branch 至少有一个状态化测试。

### P1-3：物品、Frame 与第一人称表现

目标：把 item model pipeline 从“主要消费者可画”推进到 vanilla presentation parity。

仍在推进：

- First-person viewmodel：
  - hand transform。
  - use animation。
  - swing animation。
  - map / bow / crossbow / spyglass / shield 等特殊路径。
- Combat / held item arm pose：
  - third-person hand-use sway。
  - kinetic weapon / ticksUsingItem。
  - per-item swing duration。
  - left/right-hand transform 差异。
- HUD / inventory：
  - vanilla font / count / durability / cooldown / tooltip / screen depth behavior。
  - flat/generated item 与 3D block item 在 GUI pass 中的精确排序。

完成标准：

- 每个 item consumer 都以 vanilla `ItemDisplayContext`、display transform 和 renderer 源码为依据。
- GUI/world 使用不同 lighting context 时必须在测试或手动对比记录中说明。

### P1-4：GUI Lighting Surface / Entity-In-UI

目标：把 GUI flat、GUI 3D、entity-in-UI 的 lighting context 和 world item/entity
路径分清。

仍在推进：

- GUI flat item：
  - front-lit / no-world-diffuse 的 shader context。
  - generated item、flat sprite、count/durability/cooldown overlay 的 pass 顺序。
- GUI 3D item：
  - `Lighting.Entry.ITEMS_3D` light directions。
  - block item / model item 与 GUI depth 的相互关系。
- entity-in-UI：
  - entity preview lighting。
  - entity preview transform / scale / scissor / depth isolation。
  - armor / held item / head item 在 UI preview 中的 layer order。
- screen integration：
  - inventory、container、merchant、recipe/book/sign/advancement 等 screen 调用点。
  - GUI pass 与 world pass 的 load/clear/depth ordering。

完成标准：

- GUI/world/entity-in-UI 三类 lighting context 明确分流。
- 至少一个 GUI flat item、一个 GUI 3D item、一个 entity-in-UI path 有 deterministic
  screenshot/readback 或等价 render-plan 测试。

### P1-5：透明排序、粒子与 Level Events

目标：补齐当前粒子、透明对象和官方的排序、限制、provider 细节差距。

仍在推进：

- 粒子 provider-specific behavior：
  - 初速度。
  - lifetime。
  - size curve。
  - alpha/color curve。
  - gravity / collision / player-coupled physics。
- 粒子 sorting：
  - translucent particle order。
  - terrain/item particle option rendering。
  - particle limits/settings。
- atlas mip animation：
  - animated sprite frame advance。
  - missing definition / missing sprite diagnostics。
- LevelEvent particle side effects：
  - smoke/flame/dragon-breath/explosion/cloud/block-face/trial-spawner 之外的剩余事件。

完成标准：

- 每个 particle slice 记录 vanilla provider 类和精确公式。
- 对随机行为使用确定性 seed 或固定样本测试。

### P2：Terrain / Block Render Presentation

目标：把 terrain 从基础 mesh 对齐推进到官方视觉细节。

仍在推进：

- 检查 block render shape、face culling、AO、tint、biome tint、fluid overlay 与 vanilla 差异。
- 补齐破坏进度、selection overlay、block entity 特殊 renderer、透明块排序等剩余 presentation。
- 复核 terrain 与 entity/item 共用 atlas、mip、sampler、lightmap 时的状态差异。

完成标准：

- 每个 block/render shape 差异必须有 vanilla source 或资源 JSON 依据。
- 对视觉 slice 使用确定性 pixel/readback 测试或明确手动对比记录。

### P2：屏幕、HUD、字体与截图

目标：从功能性 HUD 推进到 vanilla screen presentation。

仍在推进：

- vanilla font rendering：
  - glyph atlas。
  - shadow。
  - bidi / style / color。
  - width metrics。
- HUD：
  - hotbar、crosshair、status bars、boss bars、titles、subtitles、debug overlay。
  - screen 与 world pass 的深度/颜色 load/clear 顺序。
- Screens：
  - inventory / container / merchant / recipe / book / sign / advancement 等 screen 的 vanilla 布局。
- Screenshot / readback：
  - 保证 renderer output 可稳定测试。
  - 将更多视觉 slice 接入 deterministic screenshot/readback，而不是只依赖手动对比。

完成标准：

- 新 UI/screen 工作不做临时配置 UI；启动配置仍从命令行进入。
- 视觉结果尽量用 deterministic screenshot/readback 验证。

### P3：资源与动态纹理泛化

目标：扩展更泛化的动态资源加载，覆盖 profile 与非 profile 纹理、资源包和
datapack 组合路径。

仍在推进：

- broader non-profile dynamic texture loading。
- 资源包 override / custom model / datapack registry asset 的动态组合。
- 失败、pending、ready 状态必须有明确 fallback，不画 stale texture。

完成标准：

- 动态资源路径区分 decode、cache、upload、ready 状态。
- submission metadata 在缺 atlas entry 时仍可记录，折叠 geometry 可按 vanilla fallback 或等待策略处理。

每个 slice 开始前先 grep 当前实现，确认该 feature 确实缺失或测试不足；历史上多次出现
“ledger 以为缺失但代码已实现”的情况。

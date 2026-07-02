# 已完成目标归档（goal.md archive）

本文件是 `goal.md` 中已完成条目（`- [x]`）的归档。这些条目按它们原属的优先级章节（P0 / P1-1 / P1-2 / P1-3 / P1-4 / P1-5 / P2 / P3）分组，逐字保留，不做改写、概括或重排；只有当前有已完成条目的章节才在下方出现。本文件仅追加（append-only）：`goal.md` 中的开放项完成后，把对应 `- [x]` 条目原样移动到此处对应章节，`goal.md` 中只保留一行指向本文件的指针。

## P1-1：GPU Render-State / Render-Graph Fidelity

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
    `LESS_EQUAL` + polygon offset `-1.0F, -10.0F`、默认 back-face cull；
    本地 cube overlay 三角 winding 已同步为 outward。完整 block model-shaped
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
  - [x] item-model default cull state：block/flat item-model
    `ITEM_CUTOUT` / `ITEM_TRANSLUCENT` 和 item-frame
    `entitySolidZOffsetForward` GPU path 现在使用 vanilla builder 默认
    back-face cull；mesh bake 按 submitted normal 规范化 triangle indices，
    测试覆盖 `ItemModelQuad`、generated item extrusion 和 block-item baker
    的 front-face 输出。更细的 item special-display / shader ABI parity
    仍属后续 P1/P2。
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
  - [x] selection / line append pass：block selection、entity-scene outline、
    entity-target outline 继续写入 itemEntity target 且在 particles 前绘制；
    GPU pipeline 现在使用 vanilla `RenderTypes.lines()` 的
    `VIEW_OFFSET_Z_LAYERING`、translucent blend、depth-write `LESS_EQUAL`、
    普通 block-hit outline `ARGB.black(102)` alpha。屏幕空间线宽与
    high-contrast secondary outline 仍属后续视觉 polish。
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
  - [x] translucent particle GPU state：当前 billboard 粒子路径继续只覆盖
    已有 provider 表达，但 GPU pipeline 现在按 vanilla
    `RenderPipelines.TRANSLUCENT_PARTICLE` / `PARTICLE_SNIPPET` 固定
    `core/particle` shader、Sampler0 + Sampler2 LightTexture、
    `BlendFunction.TRANSLUCENT`、默认 back-face cull、depth-write
    `LESS_EQUAL`。
  - [x] itemEntity billboard GPU state：当前 flat billboard 路径继续只是
    现有 dropped/thrown item 近似表达，但写入 itemEntity target 的 pipeline
    现在按 vanilla `RenderPipelines.ITEM_TRANSLUCENT` / `RenderTypes.item_translucent`
    固定 `core/item` shape、`ALPHA_CUTOUT 0.1`、Sampler0 + LightTexture、
    `BlendFunction.TRANSLUCENT`、默认 back-face cull、depth-write
    `LESS_EQUAL`。
  - [x] sky family depth-state：`SKY`、`END_SKY`、`SUNRISE_SUNSET`、
    `STARS`、`CELESTIAL` 在 vanilla `RenderPipelines` 中都没有显式
    `DepthStencilState`；renderer 的 sky/end-sky/star/celestial pipelines
    已改为无 depth-stencil state。
  - [x] sky `SKY` / `SUNRISE_SUNSET` blend/cull split：Overworld sky disc 和
    sunrise/sunset 现在按 vanilla `LevelRenderer` draw 顺序拆成 sky ->
    sunrise -> sun/moon/stars；`SKY` pipeline 使用 replace/no blend、默认
    back-face cull、无 depth state，`SUNRISE_SUNSET` 使用 translucent blend、
    默认 back-face cull、无 depth state。二者仍因 wgpu 用 triangle-list
    展开官方 fan。
  - [x] sky `SKY` fog shader shape：sky disc 专用 shader 现在按 vanilla
    `core/sky.fsh` 使用 `FogSkyEnd` 等价的 `camera.fog_visibility_ends.x`，
    以 spherical `0..FogSkyEnd` 和 cylindrical `FogSkyEnd..FogSkyEnd` 混入
    `FogColor`；sunrise/sunset 使用无 fog 的 `position_color` shape，stars
    单独使用 `core/stars` shape。
  - [x] sky `SKY` ColorModulator ABI：sky disc GPU path 现在使用 position-only
    vertex buffer，shader 不再读取 per-vertex color，并通过 sky dynamic uniform
    传递 `ColorModulator = skyColor`，匹配 vanilla
    `RenderPipelines.SKY` / `core/sky`；render pass 测试固定 draw 前绑定
    dynamic uniform。
  - [x] sky `STARS` ColorModulator ABI：stars GPU path 现在使用 position-only
    vertex layout，并通过单独 sky dynamic uniform 传递
    `ColorModulator = vec4(STAR_BRIGHTNESS)`，匹配 vanilla
    `RenderPipelines.STARS` / `core/stars`；测试固定 render pass 在 draw 前绑定
    dynamic uniform。
  - [x] sky `STARS` model matrix ABI：stars GPU path 现在保留 vanilla
    `SkyRenderer.buildStars` 生成的静态星空 vertex buffer，`STAR_ANGLE` 通过
    sky dynamic uniform 的 model matrix 表达 `Y(-90deg) * X(starAngle)`，
    匹配 vanilla `renderSunMoonAndStars` pose stack；测试固定静态顶点中心和
    dynamic matrix 变换后的旧渲染位置一致。
  - [x] sky DynamicTransforms model-view ABI：sky dynamic shader 现在按
    vanilla `ProjMat * DynamicTransforms.ModelViewMat * Position` 拆分
    projection 和 model-view；camera 更新时将 `CameraRenderState.viewRotationMatrix`
    等价矩阵与各 sky-local transform 组合后写入 sky dynamic uniform，
    覆盖 `SKY`、`END_SKY`、`STARS`、`CELESTIAL`。
  - [x] sky `CELESTIAL` ColorModulator ABI：sun/moon GPU path 现在使用
    position+uv vertex layout，绑定 celestial atlas texture 后再绑定单独 sky
    dynamic uniform，传递 `ColorModulator = vec4(1, 1, 1, rainBrightness)`，
    匹配 vanilla `RenderPipelines.CELESTIAL` / `core/position_tex`；
    测试固定 shader shape 和 render pass draw 前的 dynamic uniform 绑定。
  - [x] sky `CELESTIAL` model matrix ABI：sun/moon GPU path 现在保留 vanilla
    `buildCelestialQuad` / `buildMoonPhases` 的静态 unit quad，并分别绘制 sun
    与 moon；`SUN_ANGLE` / `MOON_ANGLE`、`translate(0, 100, 0)` 和
    `scale(30|20, 1, 30|20)` 通过各自 sky dynamic uniform 的 model matrix
    表达，匹配 vanilla `renderSun` / `renderMoon` 的 `writeTransform`。
  - [x] sky `END_SKY` ColorModulator ABI：End sky GPU path 现在使用专用
    `position_tex_color` shader shape，保留 position+uv+vertex color，
    按 vanilla 顺序执行 texture * vertexColor、alpha==0 discard，再乘
    `ColorModulator = vec4(1, 1, 1, 1)`；render pass 在 texture bind 后、
    draw 前绑定 sky dynamic uniform。
  - [x] sky `END_SKY` / `STARS` / `CELESTIAL` default cull：这三条 pipeline
    现在也按 vanilla builder 默认启用 back-face cull；测试固定官方
    `SkyRenderer.buildEndSky` / `buildStars` / celestial quad 的 triangle-list
    展开仍面向相机原点。
  - [x] terrain render-pipeline state：solid/cutout terrain 继续用 replace
    blend + depth-write，translucent terrain 继续用 translucent blend +
    no depth-write；三者现在都按 vanilla `SOLID_TERRAIN` /
    `CUTOUT_TERRAIN` / `TRANSLUCENT_TERRAIN` builder 默认使用 back-face cull
    和 `LESS_EQUAL` depth test。
  - [x] P1-1 closeout audit：当前没有新的狭义 GPU render-state /
    render-graph blocker。`glintTranslucent` / item foil variants 归入 P1-3
    item presentation，standalone texture mip/sampler 泛化归入 P3 resource
    parity，剩余 diffuse/fog/visual polish 归入后续 scoped visual slices；
    不再把泛化项作为阻塞 P1-2 的开放清单。
- RendererFrame extraction timing：
  - [x] sky-flash-dependent environment fields：`lightmap_environment`、
    `clear_color`、`fog_environment`、`sky_environment`、`cloud_environment`
    现在在 `advance_sky_flash_time` 之后提取，匹配 vanilla
    `Minecraft.tick` -> `ClientLevel.tick` -> `GameRenderer.extract` 顺序：
    `ClientLevel.tick` 先递减 `skyFlashTime`，render extract 再读取
    `EnvironmentAttributes` / lightmap state。测试固定 pump 源码顺序，并证明
    `skyFlashTime == 1` 推进一 tick 后不会让下一帧环境继续应用闪光。
  - [x] HUD local-player / hotbar / inventory-screen fields：
    `hud_health`、`hud_food`、`hud_experience_progress`、`hud_selected_slot`、
    hotbar icons 和 inventory screen projection 现在有源码顺序测试与绑定注释，
    固定为 `advance_player_input`、destroy/use input advancement 和
    `advance_local_using_item_ticks` 之后读取；vanilla 依据是
    `Minecraft.tick` 先处理 gameplay keybinds，随后 `GameRenderer.extractGui`
    调用 `Gui.extractRenderState` / `Gui.extractItemHotbar`。
  - [x] item/entity projection fields：dropped item models、item entity
    billboards、entity model instances、held item models、item-frame models 和
    entity block-item models 现在有源码顺序测试与绑定注释，固定为 entity
    animation、client-time、cooldown、input 和 use-item tick 之后读取；vanilla
    依据是 `Minecraft.tick` 先推进 keybinds / `gameRenderer.tick` /
    `level.tickEntities`，随后 `GameRenderer.extract` 调用
    `LevelRenderer.extractLevel` / `extractVisibleEntities`。
  - [x] block-destroy overlay field：`block_destroy_overlays` 现在有源码顺序
    测试与绑定注释，固定为 `advance_block_destruction_render_ticks` 之后读取；
    vanilla 依据是 `LevelRenderer.extractBlockDestroyAnimation` 在 client tick 后的
    render extract 中读取 block-breaking state。
  - [x] outline fields：selection outline、entity-scene outline 和 entity-target
    outline 现在有源码顺序测试与绑定注释，固定为 input / use-item / entity tick
    之后读取同一帧 camera pose；vanilla 依据是 `Minecraft.renderFrame` 先调用
    `pick(partialTicks)`，随后 `GameRenderer.extract` / `LevelRenderer.extractBlockOutline`
    读取 `hitResult` 和 camera state。
  - [x] cloud-frame field：`cloud_frame` 现在有源码顺序测试与绑定注释，固定为
    client-time、partial tick 和 frame camera pose 之后读取；vanilla 依据是
    `LevelRenderer.renderLevel` 为 `addCloudsPass` 读取 `level.getGameTime()`、
    `deltaPartialTick` 和 `cameraRenderState.pos`。
  - [x] weather field：`weather_render_state` 现在有源码顺序测试与绑定注释，固定为
    client-time、partial tick 和 frame camera pose 之后读取；vanilla 依据是
    `LevelRenderer.extractLevel` 调用
    `WeatherEffectRenderer.extractRenderState(level, ticks, deltaPartialTick, cameraPos, ...)`。
  - [x] particle light refresh：粒子 tick 已调整为 input / use-item advance
    之后，particle light refresh 现在有源码顺序测试与绑定注释，固定为当前帧提取输入
    绑定后、RendererFrame commit 前采样；vanilla 依据是 `Minecraft.tick` 先处理
    gameplay input 再 `ParticleEngine.tick`，随后 `LevelRenderer.extractLevel` /
    `ParticleEngine.extract` 调用 `SingleQuadParticle.getLightCoords(partialTicks)`。

## P1-2：实体专用 Renderer 行为

- Chicken / pig / cow variant livestock：
  - [x] P1-2 renderer closeout：内置 variant 的模型、贴图、base
    submission metadata 已覆盖；variant sound 属 audio / sound-registry
    parity，不作为实体 renderer blocker；custom/datapack variant asset 属
    P3 资源/datapack 泛化。
- Spider / slime / magma cube / ghast / blaze / endermite / silverfish / vex /
  allay / phantom：
  - [x] shared living death flip：vanilla base `90°` 与
    `SpiderRenderer` / `EndermiteRenderer` / `SilverfishRenderer` `180°`
    override 已实现并测试。
  - [x] P1-2 renderer closeout：基础模型、texture-backed submission、死亡翻转等
    renderer-owned pose 已覆盖；particle/audio coupling 归入 P1-5 / audio
    effects，crumbling 归入 P2/P1 visual overlay，不作为当前实体 renderer blocker。
- Feline / cat renderer pose：
  - [x] cat sitting pose：`CatRenderer.extractRenderState` 的
    `Cat.isInSittingPose()` 现在从 `TamableAnimal.DATA_FLAGS_ID` id 18 bit 0
    投影到 `feline_is_sitting`，ocelot 保持 false；adult/baby feline 模型应用
    vanilla sitting branch，textured base/collar 共享坐姿 posed tree。
  - [x] cat lie-down / relax model state：`Cat.IS_LYING` id 21 与
    `Cat.RELAX_STATE_ONE` id 22 现在驱动 vanilla client tick easing
    (`lieDownAmount` +0.15/-0.22、`lieDownAmountTail` +0.08/-0.13、
    `relaxStateOneAmount` +0.1/-0.13)，world/native/renderer 投影三组 amount；
    adult/baby feline 模型应用 vanilla lie-down leg/head/tail branch 与 relaxed
    head pitch，textured base/collar 路径覆盖 texture/render type/tint/order/
    submit_sequence 与 transform。
  - [x] CatRenderer.setupRotations base lie-down root transform：`lieDownAmount`
    驱动 whole-body translate `(0.4, 0.15, 0.1) * amount` 和 Z roll
    `90° * amount`，插在 living setup rotation 之后、model flip 之前；
    adult cat 的 `MeshTransformer.scaling(0.8)` 仍接在 feline root 之后，
    textured base/collar submission 共享该 transform。
  - [x] sleeping-player extra translate：world source 按 vanilla
    `Cat.handleLieDown` 用 `new AABB(cat.blockPosition()).inflate(2)` 查找 nearby
    sleeping player，native 透传 `isLyingOnTopOfSleepingPlayer`，renderer 在
    lie-down Z roll 后追加 `translate(0.15 * lieDownAmount, 0, 0)`；textured
    base/collar submission 共享该 transform。
- Minecart：
  - [x] rail-follow `posOnRail` / `frontPos` / `backPos` 平移与坡度 pitch：
    old-render minecart source 按 vanilla `OldMinecartBehavior.getPos` /
    `getPosOffs(..., ±0.3F)` 从当前 rail block-state `shape` 投影三点，native
    透传到 renderer state，renderer 在 no-new-render 分支按
    `AbstractMinecartRenderer.oldRender` 应用 rail 平移、`backPos - frontPos`
    yaw 和 `atan(direction.y) * 73.0` 坡度 pitch。
  - [x] NewMinecartBehavior exact weighted `renderPos` / rotation interpolation。
  - [x] NewMinecartBehavior passenger render offset：乘坐 new-render minecart 的
    实体现在按 vanilla `EntityRenderer.extractRenderState.passengerOffset` 加上
    `getCartLerpPosition(partialTicks) - lerp(xOld, getX)` 的 render-only root
    offset；矿车 lerp drain 后自动回到普通 entity source position。
  - [x] display block transform / content / light baseline：
    `WorldStore::minecart_display_block_state` 解析 custom/default display block
    与 `displayOffset`，renderer 内容块 transform 覆盖 vanilla 0.75 scale /
    offset / `Ry(90)` 且位于 body final flip 之前，native block-model
    attachment 路径携带 block properties、entity light 和 hidden-glowing
    outline-only。
  - [x] TNT fuse scale / white overlay：`MinecartTNT.handleEntityEvent(10)`
    投影 `fuse = 80`，`MinecartTntRenderState.fuseRemainingInTicks` 传到
    renderer instance；native display-block attachment 使用 vanilla final-10-tick
    四次方 scale 和 `OverlayTexture.u(1.0F)` 白闪 overlay。
  - [x] display-block culling bbox expansion：model-target bounds proxy 按
    vanilla `AbstractMinecartRenderer.getBoundingBoxForCulling`，只在 display
    block 非 air 时应用 `expandTowards(0, displayOffset * 0.75 / 16, 0)`，
    并覆盖默认 TNT/chest/hopper offset 与自定义负 offset。
  - [x] P1-2 renderer closeout：minecart body、old/new behavior transform、
    passenger render offset、display block baseline、TNT fuse overlay 和 culling
    bbox 已覆盖；spawner animated block-entity content 属 block-entity special
    renderer / P2 presentation，不阻塞实体 minecart renderer。
- Copper golem：
  - [x] walk / walk-with-item keyframe animation：renderer 现在按 vanilla
    `CopperGolemAnimation.COPPER_GOLEM_WALK` /
    `COPPER_GOLEM_WALK_ITEM` 和 `applyWalk(pos, speed, 2.0, 2.5)`
    驱动 body/head/arms/legs；持物分支在 keyframe 后应用
    `poseHeldItemArmsIfStill` clamp。textured regression 覆盖 texture /
    render type / tint / transform / order / submit_sequence / light /
    overlay，并验证 base 与 eyes 共享 posed tree。
  - [x] idle head-spin keyframe animation：world 侧按 vanilla
    `CopperGolem.COPPER_GOLEM_STATE`（data id 17，serializer 37）和
    `random.nextInt(200, 240)` delayed start 投影 idle timer，native 带入
    `EntityRenderState.copper_golem_idle_seconds`，renderer 按
    `CopperGolemAnimation.COPPER_GOLEM_IDLE` 叠加 body/head keyframe；
    回归测试覆盖 world/native projection、renderer pose，以及 textured
    submission metadata 不变。
  - [x] GETTING_ITEM chest interaction keyframe：world 侧按
    `CopperGolemState.GETTING_ITEM` 启停
    `interactionGetItemAnimationState`，native 带入
    `EntityRenderState.copper_golem_get_item_seconds`，renderer 按
    `CopperGolemAnimation.COPPER_GOLEM_CHEST_INTERACTION_NOITEM_GET`
    叠加 body/head/arms/legs keyframe；回归测试覆盖 world/native
    projection、renderer pose，以及 textured submission metadata 不变。
  - [x] GETTING_NO_ITEM chest interaction keyframe：world 侧按
    `CopperGolemState.GETTING_NO_ITEM` 启停
    `interactionGetNoItemAnimationState`，native 带入
    `EntityRenderState.copper_golem_get_no_item_seconds`，renderer 按
    `CopperGolemAnimation.COPPER_GOLEM_CHEST_INTERACTION_NOITEM_NOGET`
    叠加 body/head/arms/legs keyframe；回归测试覆盖 world/native
    projection、renderer pose，以及 textured submission metadata 不变。
  - [x] DROPPING_ITEM chest interaction keyframe：world 侧按
    `CopperGolemState.DROPPING_ITEM` 启停
    `interactionDropItemAnimationState`，native 带入
    `EntityRenderState.copper_golem_drop_item_seconds`，renderer 按
    `CopperGolemAnimation.COPPER_GOLEM_CHEST_INTERACTION_ITEM_DROP`
    叠加 body/head/arms/legs keyframe；回归测试覆盖 world/native
    projection、renderer pose，以及 textured submission metadata 不变。
  - [x] DROPPING_NO_ITEM chest interaction keyframe：world 侧按
    `CopperGolemState.DROPPING_NO_ITEM` 启停
    `interactionDropNoItemAnimationState`，native 带入
    `EntityRenderState.copper_golem_drop_no_item_seconds`，renderer 按
    `CopperGolemAnimation.COPPER_GOLEM_CHEST_INTERACTION_ITEM_NODROP`
    叠加 body/head/arms/legs keyframe 和 vanilla left-leg identity scale
    channel；回归测试覆盖 world/native projection、renderer pose，以及
    textured submission metadata 不变。
- Equine / camel / llama / goat / hoglin / ravager 等大型模型：
  - [x] P1-2 renderer closeout：equine/camel/llama/goat/hoglin/ravager 的模型、
    texture-backed submission、pose / combat / equipment baseline 已覆盖到当前
    renderer ownership；boost 属 movement/control 或 ride gameplay，不是 vanilla
    renderer pose branch。
  - [x] camel body-anchor y-offset formula/query：`WorldStore::entity_body_anchor_y_offset`
    已按 vanilla `Camel.getBodyAnchorAnimationYOffset` 覆盖 front/rear、sit/stand
    transition、baby/camel_husk、SCALE attribute；passenger/leash 视觉消费路径另列后续
    attachment presentation。
  - [x] piglin / piglin brute / hoglin converting shake：native projection 现在按
    vanilla `PiglinRenderer` / `HoglinRenderer.isShaking = super || isConverting`
    读取 synced immune-to-zombification flags，并用 built-in dimension
    `PIGLINS_ZOMBIFY`（Nether false，其它默认 true）折入
    `LivingEntityRenderer.setupRotations` body shake；Zoglin 不进入 conversion
    override。
  - [x] P1-2 remaining-effects taxonomy：ravager roar particle/knockback、
    entity particle/audio coupling 归入 P1-5 / audio effects；custom-dimension
    `EnvironmentAttributes` 数据来源归入 P3 datapack/world-registry 泛化，不阻塞
    内置维度下的实体 renderer parity。
- Humanoid / illager / piglin / skeleton family：
  - [x] armor stand hit wiggle：entity event `32` 现在投影 vanilla
    `ArmorStandRenderState.wiggle = gameTime - lastHit + partialTick`，并在
    root transform 中按 `sin(wiggle / 1.5 * PI) * 3°` 追加 Y 轴 setup rotation；
    base textured submission 继承相同 transform。
  - [x] P1-2 illager / piglin / skeleton arm-pose audit：skeleton bow aim、
    held spear、zombie-family STAB/WHACK、piglin/brute attack/crossbow/admire、
    illager spell / bow / celebrate / attack / crossbow / riding precedence 已有
    vanilla-backed renderer/native/world 覆盖；剩余 custom `NONE` swing type、
    first-person / item presentation edge cases 归入 P1-3，不作为当前 P1-2 blocker。
  - [x] player spear use-item kinetic sway：using-hand `ArmPose.SPEAR`
    现在按 vanilla `SpearAnimations.thirdPersonHandUse` 使用 `KineticWeapon`
    delay/condition ticks 叠加 arm raise/sway；held item layer 同步追加
    `SpearAnimations.thirdPersonUseItem` transform。
  - [x] player spear use-item kinetic hit feedback：renderer 现在表达
    `LivingEntityRenderState.ticksSinceKineticHitFeedback` 对
    `SpearAnimations.thirdPersonUseItem` 的 held-item kickback 平移
    `(0, -hitFeedback*0.4, +hitFeedback)`，并用 focused transform test 覆盖
    non-zero feedback；world 现在按 vanilla `LivingEntity.handleEntityEvent(2)` /
    `onKineticHit` 投影 `lastKineticHitFeedbackTime` 来源，保留 `> 10` tick
    重新触发门槛，native 将 partial-tick elapsed 值传入 renderer。
  - [x] player held-spear `ArmPose.SPEAR`：非 using 的 main/off-hand spear
    现在投影为显式 spear arm-pose flags，renderer 复用
    `SpearAnimations.thirdPersonHandUse` 的 base pose（`ticksUsingItem <= 0`，
    无 kinetic sway）；off-hand `SPEAR.affectsOffhandPose` 会按 vanilla 跳过
    main-hand `ITEM`，但主手 charged `CROSSBOW_HOLD.isTwoHanded()` 会先把非空
    副手强制成 `ITEM`。
  - [x] player main-hand charged crossbow two-handed override：vanilla
    `AvatarRenderer.getArmPose` 在 `CROSSBOW_HOLD.isTwoHanded()` 时把非空副手
    pose 强制为 `ITEM`，native 现在让主手 charged crossbow hold 压过副手
    spear / bow-use / charged-crossbow 专用 pose，renderer 测试固定最终双臂仍是
    `AnimationUtils.animateCrossbowHold`。
  - [x] zombie-family attack-arm spear STAB：native 现在按
    `ArmedEntityRenderState.extractArmedEntityRenderState` 为非玩家 humanoid
    提取 attack-arm spear `SwingAnimationType.STAB`；zombie / husk / drowned /
    zombie-villager 在 `swingAnimationType == STAB` 时跳过 held-out rewrite；
    `attack_anim > 0` 时运行 inherited `SpearAnimations.thirdPersonAttackHand`
    lunge，并匹配 `AnimationUtils.animateZombieArms` 的 STAB 分支只保留
    `bobArms`。
  - [x] non-player `HumanoidMobRenderer` held-spear `ArmPose.SPEAR`：native
    为 skeleton / zombie / piglin family 投影 base same-hand spear pose，并只对
    zombie-family 保留 `AbstractZombieRenderer` 的 opposite-hand STAB override；
    renderer 复用 `SpearAnimations.thirdPersonHandUse` base pose，覆盖 skeleton
    off-hand `SPEAR.affectsOffhandPose` 对 bow aim 的 suppression、piglin
    custom arm-pose 后置覆盖、zombified-piglin STAB skip held-out rewrite。
  - [x] zombie-family WHACK inherited body twist / arm-anchor：WHACK
    `attack_anim > 0` 时先运行 `HumanoidModel.setupAttackAnimation` 的 body
    yRot 与左右 arm anchor offset，再由 `AnimationUtils.animateZombieArms`
    覆写 arm rotation，adult zombie 状态测试固定 body/anchor/held-out rotation。
  - [x] piglin-family default WHACK：普通 piglin / piglin brute 在非
    `ATTACKING_WITH_MELEE_WEAPON` 的 mid-swing 分支现在走 vanilla
    `PiglinModel.setupAttackAnimation -> super.setupAttackAnimation` 的默认
    `HumanoidModel` body twist / arm anchor / right-arm whack；melee-weapon
    pose 仍保持 `AnimationUtils.swingWeaponDown` 分支。
  - [x] pillager aggressive `ATTACKING`：world/native 现在把 pillager 纳入
    `Mob.isAggressive` render-state 投影；renderer 按 vanilla
    `Pillager.getArmPose` 保持 `CROSSBOW_CHARGE` / `CROSSBOW_HOLD` 优先，
    无 crossbow 且 aggressive 时进入 `IllagerModel.setupAnim` 的
    `ATTACKING` 分支（armed `swingWeaponDown`，empty-hand `animateZombieArms`）。
  - [x] pillager `isHolding(CROSSBOW)` 双手语义：native 现在按 vanilla
    `LivingEntity.isHolding` 检查主手或副手 crossbow，投影为
    `pillager_holds_crossbow`；renderer 继续让 `CROSSBOW_HOLD` 压过
    aggressive `ATTACKING`。
  - [x] illusioner invisible clone body submit：隐身 illusioner 现在按 vanilla
    `IllusionerRenderer.submit` 循环四个 `illusionOffsets`，在实体位置之后、
    living setup rotation 之前追加 offset + age jitter，并且覆盖
    `LivingEntityRenderer.isBodyVisible` 的普通 hidden / force-transparent /
    outline-only 分支，四个 body submission 保留 `entityCutout`、illusioner
    texture、tint、transform、light、overlay、order / submit_sequence。layer 随
    clone 复制由后续 item/layer presentation slice 逐步收口。
  - [x] illusioner invisible custom-head clone layer：`CustomHeadLayer` skull branch
    现在复用同一组 invisible illusioner clone root transform，在每个 clone 的
    base body 后提交一次 skull layer，保留 vanilla `entityCutoutZOffset`、skull
    texture、no-overlay、light、transform、order / submit_sequence。
  - [x] illusioner invisible custom-head generic item clone layer：
    `CustomHeadLayer` 的 non-skull HEAD item branch 现在同样复用 invisible
    illusioner clone root transform；native item-model pass 对可见 illusioner
    生成 1 个 HEAD item mesh，对隐身 illusioner 生成 4 个 clone mesh，并保留
    `ItemDisplayContext.HEAD` / no-overlay 行为。
  - [x] illusioner held-item clone layer / dynamic visibility：native 持物
    item-model pass 现在匹配 vanilla anonymous `ItemInHandLayer`，idle
    illusioner 不提交持物，`isCastingSpell || isAggressive` 时提交；隐身
    illusioner 复用 `IllusionerRenderer.submit` 的四个 clone root transform，
    每个 clone 生成一次 right/left hand item transform，focused tests 覆盖
    idle 0 个、可见 aggressive 1 个、隐身 aggressive/casting 4 个 mesh。
  - [x] illager attack / crossbow / spell / celebrate / riding 组合冲突：
    renderer transform test 现在固定 vanilla `IllagerModel.setupAnim` 顺序：
    riding seated pose 先写 limbs，随后 spell / bow / crossbow hold /
    crossbow charge / celebrate / attack arm-pose branch 可覆写 arms，legs 保持
    seated。
  - [x] main-hand spear STAB held-item layer transform：player STAB arm lunge
    之后，held item transform 追加 vanilla
    `SpearAnimations.thirdPersonAttackItem` 的 local pivot rotation 与 spear
    `forwardMovement = 0.38` 平移。
  - [x] spear held-item 与 use-item 的 layer/order 交互：
    `ItemInHandLayer.submitArmWithItem` 顺序已用 focused test 固定为
    base hand offset → `thirdPersonAttackItem` → `thirdPersonUseItem` →
    item submit，并验证反序矩阵不同。
- Boss / beam / emissive 类：
  - [x] EnderDragon dying-dissolve render type / submission 表达：
    `dragonDeathTime` source projection、`entityCutoutDissolve(dragon.png,
    dragon_exploding.png)`、secondary mask texture、alpha、no-overlay、order /
    submit_sequence、missing-atlas submission-first 测试已覆盖。
  - [x] EnderDragon `dragonRays` / `dragonRaysDepth` 死亡 rays：
    dispatch-owned no-texture custom geometry submission、vanilla 432 seed /
    `rayCount` / inner-outer color、`dragonRays` additive pipeline、
    `dragonRaysDepth` depth-only replay、order / submit_sequence 与
    missing-atlas-independent geometry 测试已覆盖。

## P1-3：物品、Frame 与第一人称表现

- Combat / held item arm pose：
  - [x] player attack-arm spear `STAB` uses vanilla registry default swing duration
    (`Item.Properties.spear(... attackDuration ...)` -> `SwingAnimation.duration`) through
    pack -> native -> world, including off-hand attack-arm STAB item transform.
  - [x] patch-granted custom `swing_animation` stack values override item defaults for
    `SwingAnimation.duration` and `SwingAnimation.type` (including stack `STAB` on non-spear
    items and removed / `WHACK` overrides on default spear stacks).
  - [x] dig-speed / mining-fatigue modifiers apply to entity attack swing duration
    (`MobEffectUtil.hasDigSpeed` HASTE / CONDUIT_POWER priority, else MINING_FATIGUE).
  - [x] runtime item/effect changes during an in-flight swing refresh active
    `SwingAnimation.duration` from the current swinging-arm stack and mob effects.
  - [x] third-person `ItemInHandLayer` resolves held stacks through vanilla
    `getItemHeldByArm`: player `DATA_PLAYER_MAIN_HAND` and mob `MOB_FLAG_LEFTHANDED`
    now swap main/off-hand items across right/left arm transforms and
    `thirdperson_{right,left}hand` display contexts.
  - [x] `minecraft:main_hand` item-model select resolves for owner-backed
    third-person/entity-attached generated item paths and GUI/HUD item icons
    that have a local-player owner context, from vanilla
    `MainHand.get(... owner.getMainArm())` and `GuiGraphicsExtractor.item`
    passing `minecraft.player` to `updateForTopItem`; fake/null-owner item
    consumers still fall back as vanilla does.
  - [x] `minecraft:using_item` condition resolves for owner-backed
    third-person/entity-attached generated item paths from vanilla
    `IsUsingItem.get`: only the active `getUseItem()` hand selects the true
    branch; using the other hand keeps the submitted stack on false.
  - [x] `minecraft:context_dimension` item-model select resolves for GUI/HUD
    item icons and owner-backed third-person generated held-item paths from
    vanilla `ContextDimension.get`: with a `ClientLevel` context it matches
    `level.dimension()`, while no-level item consumers keep the fallback. Tests
    pin GUI/HUD world-dimension selection and held-item fallback / overworld /
    nether mesh branch changes.
  - [x] `minecraft:context_entity_type` item-model select resolves for GUI/HUD
    item icons from vanilla `ContextEntityType.get`: `GuiGraphicsExtractor.item`
    passes the local player owner, so the projected owner type is
    `minecraft:player`; fake/null-owner item consumers still fall back.
  - [x] `minecraft:context_entity_type` item-model select also resolves for
    owner-backed generated held-item paths from vanilla
    `ContextEntityType.get`: native maps renderer owner kind to the vanilla
    entity type key before resolving generated item layers; tests pin player vs
    witch branch selection. Fake/null-owner item consumers still fall back.
  - [x] `minecraft:view_entity` item-model condition resolves for GUI/HUD
    local-player item icons from vanilla `IsViewEntity.get`: vanilla
    `GuiGraphicsExtractor.item` passes `minecraft.player` as owner, so the
    normal camera==player path receives the true branch. Spectator / camera
    entity identity and owner-backed world item consumers remain follow-up.
  - [x] `minecraft:extended_view` item-model condition resolves for GUI/HUD
    local-player item icons from vanilla `ExtendedView.get`: native threads
    `ClientInputState::shift_down()` into the item icon resolver and keeps the
    vanilla `displayContext == GUI` gate, so non-GUI consumers still select the
    false branch even while Shift is down.
  - [x] `minecraft:keybind_down` item-model condition resolves for GUI/HUD
    local-player item icons from vanilla `IsKeybindDown.get`: native projects
    the currently pressed default movement/gameplay/inventory/multiplayer/misc/
    creative non-debug key names and mouse buttons into the item icon resolver,
    including `key.use` / `key.attack` / `key.pickItem`,
    `key.socialInteractions`, `key.quickActions`, screenshot / perspective /
    fullscreen / GUI toggles, creative toolbar activators, `key.spectatorHotbar`,
    hotbar 1-9, and the valid default-unbound `key.smoothCamera` /
    `key.spectatorOutlines` names as false under the vanilla default keymap.
    User-rebound/custom key mappings and debug modifier combos remain
    follow-up.
  - [x] `minecraft:fishing_rod/cast` item-model condition resolves for GUI/HUD
    hotbar local-player selected main-hand fishing rods from vanilla
    `FishingRodCast.get` / `FishingHookRenderer.getHoldingArm`: world tracks
    the local player's fishing bobber through fishing-bobber add-entity owner
    data; only the selected hotbar fishing rod selects true, while non-selected
    slots and no-bobber paths stay false. Fishing-hook billboard / line
    rendering remains P1/P2 visual follow-up.
  - [x] `minecraft:cooldown` item-model range_dispatch resolves for GUI/HUD
    item icons from vanilla `Cooldown.get`: local-player item cooldown percent
    uses the current stack cooldown group and `getCooldownPercent(..., 0.0F)`;
    no-owner / no-cooldown paths resolve as `0.0`.
  - [x] `minecraft:trim_material` item-model select now receives the dynamic
    trim-material registry keys for dropped-item `GROUND` and item-frame `FIXED`
    generated model paths and owner-backed third-person generated held-item
    paths, in addition to GUI/HUD icons; no-registry consumers still fall back
    to the untrimmed model. Tests pin held-item fallback / quartz / diamond
    baked mesh branch changes.
  - [x] GUI/HUD local-player item icons now resolve the vanilla use-tick
    numeric `range_dispatch` properties `minecraft:use_duration`,
    `minecraft:use_cycle`, and `minecraft:crossbow/pull` for the active
    `LivingEntity.getUseItem()` stack; tests pin bow pulling, brush cycle, and
    default crossbow pull texture selection. First-person use-tick threading
    remains P1 follow-up.
  - [x] Owner-backed third-person generated held-item paths now pass the active
    hand's entity use tick counter into item-model `minecraft:use_duration` /
    `minecraft:use_cycle` / `minecraft:crossbow/pull`; tests pin a held bow
    selecting different generated textures at use tick 0 vs 13 while off-hand
    use leaves the main-hand stack on fallback. First-person refinements remain
    P1 follow-up.
  - [x] `minecraft:consumable` `consume_seconds` is preserved by protocol data
    component summary and feeds vanilla `Consumable.consumeTicks()` into
    item-model `minecraft:use_duration` `remaining=true`; tests pin a 0.8s
    apple to 16 remaining ticks and the 26.1 `EnderEyeItem.getUseDuration`
    override to 0 ticks.
  - [x] Quick Charge-modified crossbow charge duration feeds item-model
    `minecraft:crossbow/pull` for GUI/HUD local-player item icons and
    owner-backed third-person generated held-item paths: protocol enchantment
    holder ids plus the world `minecraft:enchantment` registry identify
    `minecraft:quick_charge`, then apply vanilla `CROSSBOW_CHARGE_TIME`
    `-0.25F` per level before `floor(seconds * 20)`. Tests pin hotbar UV
    selection and held-item baked mesh branch changes. First-person generated
    item consumers and custom enchantment effect parsing remain follow-up.
  - [x] `minecraft:component` item-model select now has typed case matching for
    decoded scalar / enum stack components backed by vanilla persistent codecs:
    `minecraft:max_stack_size`, `minecraft:max_damage`, `minecraft:damage`,
    `minecraft:item_model`, `minecraft:rarity`, and
    `minecraft:enchantment_glint_override`, plus the synced integer
    `minecraft:map_id` `MapId` wrapper and the RGB integer wrappers
    `minecraft:dyed_color` / `minecraft:map_color`, plus simple literal
    JSON-string / `{"text": ...}` `minecraft:custom_name` values from
    `ComponentSerialization.CODEC`, plus `minecraft:item_name` simple literal
    patch values and the vanilla item/block default translatable description
    keys from `Item.Properties.finalizeInitializer`. Tests pin texture
    selection for string, numeric, boolean, resource-id, default, patched,
    explicit map-id, explicit color, literal custom-name string / text-object,
    item/block default / literal item-name string / text-object, and removed component
    values. Complex object/list components beyond simple literal text,
    style-sensitive component equality, registry-backed component value decoding,
    custom/datapack component defaults beyond parsed vanilla item properties,
    and transient components that vanilla rejects from `ComponentContents`
    selects, such as `minecraft:map_post_processing`, remain follow-up.
  - [x] `minecraft:custom_model_data` item-model condition now follows vanilla
    conditional `CustomModelDataProperty.get`: protocol preserves the
    `CustomModelData.flags` list, runtime reads `flags[index] == true`, and
    missing, false, out-of-range, or removed component id 17 select the false
    branch. Tests pin texture selection for true/false/out-of-range/removed
    cases and wire decode of floats/flags/strings/colors.
  - [x] `minecraft:custom_model_data` range_dispatch and select now follow the
    other vanilla `CustomModelDataProperty` variants: numeric range dispatch
    reads `floats[index]` and applies vanilla scale / sorted-threshold
    selection, while string select reads `strings[index]` and falls back when
    absent or out of range. Tests pin index handling, scale, inclusive
    threshold selection, and string case matching.
  - [x] `minecraft:broken` and `minecraft:damaged` item-model conditions now
    follow vanilla `ItemStack.nextDamageWillBreak()` and `ItemStack.isDamaged()`:
    effective `damage` / `max_damage` read stack components over item prototype
    defaults, while `minecraft:unbreakable` and removed `damage` /
    `max_damage` components keep the false branch. Tests pin elytra normal /
    damaged / broken texture selection and removed-component gates.
  - [x] `minecraft:charge_type` item-model select now follows vanilla
    `Charge.get`: empty `charged_projectiles` selects `none` / fallback,
    any charged `minecraft:firework_rocket` selects `rocket`, and other
    charged projectiles select `arrow`. Tests pin crossbow fallback, arrow, and
    firework texture selection from decoded charged projectile item templates.
  - [x] `minecraft:selected` item-model condition resolves for HUD hotbar item
    icons from vanilla `IsSelected.get`: the local selected hotbar slot receives
    the true branch and non-selected hotbar slots stay false. Local inventory
    GUI hotbar slots now resolve the same condition for slot
    `36 + selected_hotbar_slot`; same-stack non-selected slots stay false
    instead of matching by item contents. Recognized server-opened container
    GUI hotbar slots now resolve from the vanilla menu slot layout's player
    hotbar start plus selected hotbar index; tests pin generic `9x3` slot `55`
    true while same-stack slot `54` remains false.
  - [x] `minecraft:carried` item-model condition is recognized and resolved
    through an explicit local-player carried-stack context bit from vanilla
    `IsCarried.get` (`LocalPlayer.containerMenu.getCarried() == itemStack`):
    ordinary HUD/GUI slots and generated recipe/offer display items stay on the
    false branch, while call sites that own the actual carried stack can pass
    true without matching by stack contents.
  - [x] GUI inventory cursor-carried item projection now follows vanilla
    `AbstractContainerScreen.extractCarriedItem` for the non-dragging path:
    the world cursor stack renders as a floating GUI item at local
    `mouseX - 8`, `mouseY - 8`, and resolves item-model `minecraft:carried`
    as true while ordinary slots remain false. Tests pin cursor position,
    count label, and carried-vs-slot texture branch selection. GUI quick-craft
    drag preview now applies vanilla `getQuickCraftPlaceCount` /
    `quickCraftingRemainder` to the floating cursor stack; tests pin 11 items
    spread over three slots leaving count label `2`. Touchscreen split-stack
    and snapback animation remain broader GUI surface follow-up.
  - [x] `minecraft:bundle/has_selected_item` and
    `minecraft:bundle/selected_item` now resolve from the explicit local bundle
    selected-item index used by GUI/HUD item icons: unselected bundles stay on
    the normal model, selected bundles compose the open-back, selected item, and
    open-front layers in declaration order. Tests pin normal / unselected /
    selected bundle UVs and nested selected-item projection.
  - [x] `minecraft:component` item-model condition covers the vanilla
    `ComponentMatches.get` component-type / `AnyValue` predicate branch for
    decoded component ids such as `minecraft:rarity` and
    `minecraft:enchantment_glint_override`: default prototype components count
    as present, removed components select false, and non-default patched
    components select true. The concrete `minecraft:damage` predicate now also
    matches vanilla `DamagePredicate.matches` over both `damage` and
    `durability = max_damage - damage` `MinMaxBounds.Ints`; empty
    single-component predicates for `minecraft:bundle_contents`,
    `minecraft:container`, `minecraft:trim`,
    `minecraft:firework_explosion`, `minecraft:fireworks`, and
    `minecraft:jukebox_playable` now match vanilla's component-present path.
    `minecraft:firework_explosion` shape / trail / twinkle constraints are
    decoded from the component and matched against vanilla
    `FireworkExplosionPredicate`; `minecraft:fireworks` now decodes and
    matches `FireworksPredicate.flightDuration` `MinMaxBounds.Ints` plus
    `explosions.size` `CollectionPredicate.size` against the decoded explosions
    count, and `minecraft:fireworks` `explosions.contains` / `count`
    predicates now match decoded explosion shape / trail / twinkle summaries.
    `minecraft:trim`
    direct registry-key or trim-material-tag constraints now match the decoded
    `ArmorTrim.material()` holder id through dynamic trim-material registry keys
    and native trim-material tag catalog, and direct vanilla registry-key or
    trim-pattern-tag constraints now match decoded `ArmorTrim.pattern()` holder
    ids through vanilla `TrimPatterns.bootstrap` order and native trim-pattern
    tag catalog. `minecraft:jukebox_playable` now matches the optional `song`
    HolderSet against the decoded `JukeboxPlayable.song()` holder id through
    vanilla `JukeboxSongs.bootstrap` order, including direct vanilla registry
    keys and native jukebox-song tag entries. `minecraft:potion_contents` now
    matches vanilla `PotionsPredicate` HolderSets against the decoded
    `PotionContents.potion()` holder id through vanilla `Potions` registration
    order, including direct vanilla registry keys and native potion tag entries.
    `minecraft:writable_book_content` now matches decoded raw writable-book
    pages with vanilla `CollectionPredicate` `contains` / `count` / `size`;
    `minecraft:written_book_content` now matches decoded written-book raw
    title, author, `generation` `MinMaxBounds.Ints`, `resolved`, and
    plain-string plus simple literal `ComponentSerialization` text-object page
    collection predicates. `minecraft:villager/variant` now matches decoded
    0-based `VillagerType` holder ids against direct
    registry-key or villager-type-tag HolderSets using the vanilla
    `VillagerType.bootstrap` registry-key order. `minecraft:attribute_modifiers`
    now preserves decoded modifier entries and matches direct plus
    bundle/container-nested `modifiers` collection predicates over direct
    registry-key or attribute-tag `attribute` HolderSets when
    `minecraft:attribute` registry keys are available, plus `id`, `amount`,
    `operation`, `slot`, and `size` / `contains` / `count`. Tool, sword,
    spear, humanoid armor, wolf armor, horse armor, nautilus armor, mace, and
    trident item-prototype default modifiers now feed the same direct plus
    bundle/container-nested predicate path unless the stack explicitly removes
    or overrides `ATTRIBUTE_MODIFIERS`.
    `minecraft:custom_data` now preserves decoded
    custom-data NBT compound summaries and matches direct plus
    bundle/container-nested predicates with vanilla `NbtUtils.compareNbt(..., true)`
    subset-compound and partial-list semantics for JSON-object and SNBT-string
    compound predicate values.
    `minecraft:bundle_contents`
    `items.size` constraints now match vanilla `CollectionPredicate.size`
    against the decoded bundle item count, and `items.contains` / `count`
    now support vanilla `ItemPredicate` direct item-key or item-tag HolderSets,
    stack-count bounds, exact scalar/default `DataComponentMatchers`
    components, and patch-backed simple literal `minecraft:custom_name` /
    `minecraft:item_name` / `minecraft:lore` exact components plus
    `minecraft:unbreakable` Unit exact components and exact
    `minecraft:custom_data` compound components (JSON-object or SNBT-string
    expected values), plus exact `minecraft:potion_contents` components for
    direct potion keys, optional `custom_color`, ordered direct mob-effect
    `custom_effects` payloads including amplifier / duration / ambient /
    particles / icon / recursive hidden-effect details, and optional
    `custom_name`, plus exact `minecraft:writable_book_content`
    ordered `Filterable<String>` page lists with raw and optional filtered
    strings, plus exact `minecraft:written_book_content` components for raw /
    filtered title strings and ordered simple literal plus styled / extra /
    translated component page text summaries, plus
    exact `minecraft:firework_explosion` components for
    `shape`, ordered `colors`, ordered `fade_colors`, `has_trail`, and
    `has_twinkle`, plus exact `minecraft:fireworks` components for
    `flight_duration` and ordered explosion lists, plus nested partial
    `minecraft:damage`, plus exact `minecraft:jukebox_playable` components for
    direct vanilla jukebox-song keys and inline direct-song objects with direct
    or registry sound-event holders, description text summaries,
    `length_in_seconds`, and `comparator_output`, plus exact `minecraft:trim`
    components for direct trim-material registry keys, inline trim-material
    payloads with asset name / override armor assets / description text, direct
    vanilla trim-pattern keys, and inline trim-pattern payloads with asset id /
    description text / decal, plus exact `minecraft:enchantments` and
    `minecraft:stored_enchantments` components for direct enchantment
    registry-key maps, plus exact `minecraft:villager/variant` components for
    direct vanilla villager-type registry keys, plus exact
    `minecraft:lodestone_tracker` components for optional target `GlobalPos`
    and `tracked`, plus exact `minecraft:attribute_modifiers` components for
    ordered modifier lists with direct attribute registry keys, `id`, `amount`,
    `operation`, `slot`, and default / hidden / simple literal plus styled /
    extra override display text summaries,
    `minecraft:firework_explosion`, `minecraft:fireworks`, `minecraft:trim`,
    `minecraft:jukebox_playable`,
    `minecraft:potion_contents`, `minecraft:writable_book_content`,
    `minecraft:written_book_content`, `minecraft:villager/variant`,
    `minecraft:attribute_modifiers`, `minecraft:custom_data`, and
    data-component AnyValue predicates over decoded bundle entries.
    `minecraft:container` now decodes non-empty container entries and matches
    the same direct item-key / item-tag / stack-count / exact scalar component /
    nested partial damage, enchantments, stored-enchantments,
    firework-explosion, fireworks, trim, jukebox-playable, potion-contents,
    writable-book-content, written-book-content, villager-variant,
    attribute-modifiers, and AnyValue predicate collection subset.
    `minecraft:enchantments` and patch-backed
    `minecraft:stored_enchantments` now match decoded enchantment levels and
    direct registry-key or enchantment-tag HolderSet predicates when the
    `minecraft:enchantment` registry keys and native enchantment tag catalog are
    available to the icon resolver;
    GUI/HUD, dropped `GROUND`, item-frame `FIXED`, and owner-backed third-person
    generated held-item paths now thread that registry context, while empty
    `minecraft:enchantments` predicate lists honor vanilla's default empty
    `ENCHANTMENTS` component unless id 13 is removed. Vanilla
    `minecraft:enchanted_book` now also contributes its item-specific default
    empty `STORED_ENCHANTMENTS` component unless id 42 is removed. Exact
    `minecraft:enchantments` and `minecraft:stored_enchantments` component
    maps now compare decoded holder ids and levels against direct enchantment
    registry keys when that dynamic registry is available. Exact
    `minecraft:villager/variant` components now compare decoded holder ids
    against direct vanilla villager-type registry keys. Exact
    `minecraft:lodestone_tracker` components now compare the decoded optional
    target `GlobalPos` and `tracked` flag. Exact
    `minecraft:attribute_modifiers` components now compare decoded ordered
    modifier entries against direct attribute registry keys, numeric amount,
    operation, slot, and default / hidden / simple literal plus styled / extra
    override display text summaries. Exact
    `minecraft:written_book_content` components now compare decoded raw /
    filtered title strings, author, generation, resolved, and ordered simple
    literal plus styled / extra / translated raw / filtered page text summaries.
    Remaining
    constrained `DataComponentPredicate` types such as inline enchantment holder
    payloads / server datapack tag remaps, broader NBT scalar typing, remaining
    concrete partial predicates and complex exact component codecs beyond the
    already covered simple literal name/lore, Unit `unbreakable`, compound
    `custom_data`, filterable-page-list `writable_book_content`, full-field
    `firework_explosion` / `fireworks`, direct-key `jukebox_playable`, and
    direct-key `trim` exact components, potion / mob-effect datapack registry
    remaps for `potion_contents`, attribute modifier inline / datapack
    attribute holder payloads, full style-sensitive written-book page
    `ComponentSerialization` equality, datapack trim-material or trim-pattern
    registry-key remaps, datapack
    villager-type registry remaps, and jukebox-song datapack registry remaps
    remain component-predicate follow-up.
  - [x] `minecraft:has_component` item-model condition now follows vanilla
    `HasComponent.get`: default prototype components such as
    `minecraft:max_stack_size`, `minecraft:item_model`, `minecraft:rarity`,
    and common empty `minecraft:enchantments` count for ordinary
    `ItemStack.has`; vanilla `minecraft:enchanted_book` also counts its default
    empty `minecraft:stored_enchantments` component unless removed. The
    `ignore_default=true` path still uses patch presence
    (`hasNonDefault`) so added and removed component patches both select the
    true branch. Tests pin texture selection for default, added, and removed
    cases.
  - [x] `DataComponents.ITEM_MODEL` root item-model override is preserved and
    consumed in the native icon path: protocol decodes component id 10 as a
    resource id, unpatched stacks use the default item id from vanilla
    `Item.Properties.finalizeInitializer`, patched stacks select the effective
    `ITEM_MODEL` root like `ItemModelResolver.appendItemLayers`, and removed
    id 10 produces no item layers. Tests pin default, alternate, and removed
    behavior through texture UV selection.
  - [x] `minecraft:display_context` item-model select now resolves at runtime
    from vanilla `DisplayContext.get`: GUI/HUD icons pass `gui`, dropped-item
    generated layers pass `ground`, item-frame generated layers pass `fixed`,
    and owner-backed third-person held generated layers pass their hand
    display context. Tests pin texture selection across those contexts.
  - [x] Retained display transforms now follow the effective
    `DataComponents.ITEM_MODEL` root, matching vanilla
    `ItemModelResolver.appendItemLayers` before
    `ModelRenderProperties.applyToLayer`: dropped-item `GROUND`, item-frame
    `FIXED`, owner-backed held contexts, and the HUD GUI 3D block-item path
    query stack-aware transforms. Tests pin default, patched alternate root,
    and removed item-model component behavior.
  - [x] `minecraft:local_time` item-model select now resolves from wall-clock
    time for the vanilla 26.1 chest/trapped-chest `MM-dd` pattern and a broader
    root/en-locale ICU `SimpleDateFormat` subset (`y`/`u` year, `G` era,
    `M`/`L`, `d`, `D` day-of-year, 24/12-hour `H`/`k`/`K`/`h`, `m`/`s`/`S`,
    `E`, `a`, `Z`/`X`/`x` offset fields, and quoted literals), using fixed
    `GMT`/UTC offset and IANA `time_zone` IDs when present or the system local
    zone otherwise. `G`/`u`/`D` follow Java `DateTimeFormatter`/`IsoChronology`
    (`u` = proleptic year, identical to `y` for every CE epoch-millis date;
    era text gated on root/en locale; day-of-year zero-padded by pattern count).
    Tests pin GMT Christmas selection plus cross-midnight `UTC+02:30`,
    `Asia/Tokyo`, UTC date-time / weekday / AM-PM / offset, and a
    `uuuu-DDD-G` proleptic-year / day-of-year / era branch from vanilla
    `LocalTime.get`. Localized week (`w`/`W`/`e`/`c`/`F`), quarter (`Q`/`q`),
    zone-name (`z`/`v`/`V`/`O`) fields and non-English locales remain follow-up.
  - [x] GUI/HUD item icons now thread `WorldTimeState` into
    `minecraft:time` range_dispatch for `source=daytime` / `moon_phase`,
    matching vanilla `Time.get` target values from `EnvironmentAttributes` and
    applying default `wobble=true` standard wobbler smoothing from
    `NeedleDirectionHelper.standardWobbler(0.9F)`. Tests pin no-level fallback,
    overworld day-time and moon-phase texture selection, and a default-wobbled
    first-tick branch that raw non-wobbled target selection would miss.
    `source=random` now uses a persistent per-property Java LCG-shaped random
    source; vanilla seeds this with a client-local unique seed, so native keeps
    a deterministic local seed while preserving per-property advancement.
    Tests pin the random branch selecting a texture instead of falling back.
  - [x] GUI/HUD item icons now project `minecraft:compass` range_dispatch for
    spawn targets from the local-player position / visual yaw and current
    default spawn, matching vanilla `CompassAngle.get` exact non-wobbled target
    rotation for `wobble=false` and the default `wobble=true`
    `NeedleDirectionHelper` smoothing factor `0.8` for valid local-player
    targets. Tests pin no-pose fallback, same-dimension spawn texture
    selection, cross-dimension invalid-target random-spin selection, and a
    default-wobbled valid-target texture-selection branch.
  - [x] `minecraft:lodestone_tracker` target `GlobalPos` and `tracked` flag are
    preserved by the protocol data-component summary and feed GUI/HUD
    `minecraft:compass` range_dispatch for lodestone targets, including
    default valid-target wobble.
    Tests pin wire decode, missing-component random-spin selection,
    same-dimension lodestone texture selection, and cross-dimension
    invalid-target threshold behavior.
  - [x] `last_death_location` from vanilla `CommonPlayerSpawnInfo` is preserved
    in `WorldLevelInfo` and feeds GUI/HUD `minecraft:compass` range_dispatch
    for recovery targets, including default valid-target wobble. Tests pin
    world state projection, no-pose fallback, missing-recovery threshold
    behavior, same-dimension recovery texture selection, and cross-dimension
    invalid-target threshold behavior.
  - [x] `minecraft:compass` no-target / invalid-target rotation now follows
    vanilla `CompassAngleState.getRandomlySpinningRotation`: `target=none` is
    parsed, each baked compass property owns a no-target wobbler/random state,
    `wobble=true` updates once per game tick with factor `0.8`, `wobble=false`
    uses the non-wobbler random value, and the item-model seed hash is added
    before positive modulo. HUD hotbar icons now pass vanilla-shaped
    `slot_index + 1` seeds. Tests pin `target=none` and cross-dimension spawn
    invalid-target branches selecting random-spin textures instead of the old
    fixed `0.0` fallback.
- HUD / inventory：
  - [x] flat/generated item 与 3D block item 在 GUI pass 中的精确排序：
    P1-4 已用 vanilla `GuiGraphicsExtractor.itemDecorations` /
    `GuiItemAtlas` 旁证固定 flat/generated item sprite -> durability /
    cooldown / count decoration order，并用 renderer source-order tests 固定
    HUD base -> GUI 3D item depth-clear pass -> HUD overlay 的 pass 顺序。
    剩余 HUD/inventory parity 继续限定为字体、count/durability/cooldown/
    tooltip 的视觉细节和更宽 screen behavior。
  - [x] flat HUD/inventory item sprite glint：`HudItemIcon` now carries
    `ItemStackSummary::has_foil()` for flat item icons, and the renderer emits
    a standard `RenderTypes.glint()`-shaped HUD overlay after item sprite layers
    and before durability / cooldown / count decorations. The HUD glint shader
    samples `textures/misc/enchanted_glint_item.png` with vanilla
    `GLINT_TEXTURING` scale `8.0`, GLINT blend, GUI camera glint offsets, and
    the item-atlas layer alpha as a mask so transparent sprite pixels do not
    glow. GUI transparent 3D icon splitting and first-person special consumers
    remain follow-ups.
  - [x] GUI flat clock / compass SPECIAL foil decal UVs：
    `HudItemIcon` now carries `HudItemFoil::{None, Standard, Special}` for flat
    HUD / inventory sprites. Native maps foiled clocks and `ItemTags.COMPASSES`
    stacks through `NativeItemRuntime::item_stack_uses_special_foil_texture`,
    and the HUD glint vertices use vanilla GUI `SheetedDecalTextureGenerator`
    scale (`0.5` pose scale with `1/128` texture scale) for SPECIAL UVs while
    keeping the item-atlas alpha mask and HUD base -> glint -> decoration order.
  - [x] GUI 3D block-item translucent / glintTranslucent split：
    `collect_hud_block_item_mesh` now routes block-item GUI quads through the
    same `ItemModelMeshSet` solid/translucent and foil split as world item
    consumers. The GUI item pass draws solid base, solid glint, translucent
    base, and matching `glintTranslucent` inside the same depth-isolated
    `bbb-native-hud-item-pass`, preserving HUD base -> GUI item -> HUD overlay
    ordering while covering translucent 3D inventory icons.

## P1-4：GUI Lighting Surface / Entity-In-UI

- GUI flat item：
  - [x] front-lit / no-world-diffuse render-plan metadata：
    vanilla `GuiItemAtlas` 按 `usesBlockLight() == false` 选择
    `Lighting.Entry.ITEMS_FLAT`；native flat `HudItemIcon` 携带
    `GuiItemLightingEntry::ItemsFlat`，renderer sanitizer 拒绝非 flat lighting
    的 HUD icon，tests pin flat metadata filtering。
  - [x] generated item、flat sprite、count/durability/cooldown overlay pass
    order：vanilla `GuiGraphicsExtractor.itemDecorations` 在 item sprite 之后按
    `itemBar` → `itemCooldown` → `itemCount` 执行；renderer
    `for_each_hud_item_icon_draw_step` 固定 `Layers` → `DurabilityBar` →
    `Cooldown` → `CountLabel`，GUI 3D block item 的 base HUD phase 只跳过
    flat stand-in sprite，post-GUI-item overlay phase 保留 decoration 顺序。
- GUI 3D item：
  - [x] `Lighting.Entry.ITEMS_3D` light directions / render-plan metadata：
    vanilla `GuiItemAtlas` 按 `usesBlockLight()` 选择 `ITEMS_3D`，native
    `block_item_3d_model` 生成 `GuiItemLightingEntry::Items3d`，renderer
    sanitizer 拒绝非 `Items3d` 的 `HudBlockItemModel` 进入 GUI 3D pass；
    tests pin metadata filtering 和 `gui_ortho` 的 `ITEMS_3D` camera lighting。
  - [x] block item / model item 与 GUI depth / decoration order：
    renderer 将 HUD 2D 拆成 base commands 和 `post_gui_item` commands；
    `bbb-native-hud-item-pass` 在 base HUD pass 与
    `bbb-native-hud-overlay-pass` 之间绘制 GUI 3D block-item mesh，只在 GUI
    item pass 清空 depth，让模型面在 slot 内排序，同时 count/durability/
    cooldown、front highlight、tooltip 和 full-screen overlays 画在其上。
    source-order tests 固定 base HUD → GUI 3D item → HUD overlay pass。
- entity-in-UI：
  - [x] entity preview lighting：vanilla `GuiGraphicsExtractor.entity` 强制
    `LightCoordsUtil.FULL_BRIGHT`，`GuiEntityRenderer` 使用
    `Lighting.Entry.ENTITY_IN_UI`；renderer `HudEntityPreview` render-plan
    携带 `GuiItemLightingEntry::EntityInUi`，sanitizer 拒绝非 entity-in-UI
    lighting，并把 preview entity render-state 改写为 full-bright / outline off。
  - [x] entity preview transform / scale / scissor / depth isolation：
    `HudEntityPreview` 记录 vanilla `GuiEntityRenderState` 对应的 GUI rect、
    scale、translation、rotation、override camera rotation 和 scissor；
    sanitizer 要求 PIP depth isolation，并按 `PictureInPictureRenderState.getBounds`
    语义过滤无 visible-bounds 的 preview。Tests pin transform、scissor、
    full-bright、outline 和 invalid-state filtering。
  - [x] smithing result-slot armor/skull/wings equipment projection：
    native smithing armor-stand preview now mirrors vanilla
    `SmithingScreen.updateArmorStandPreview` for currently expressed result
    equipment: humanoid armor material / dye / foil, elytra wings metadata, and
    supported custom-head skulls. Tests pin head/chest/legs/feet armor, elytra
    layer texture, skull projection, tint, and foil.
  - [x] generic held item / non-skull head item 在 UI preview 中的 layer order：
    `HudEntityPreview` now carries explicit item-layer render-plan metadata for
    smithing preview ordinary result stacks and HEAD-slot non-armor/non-skull
    stacks. Native mirrors vanilla `SmithingScreen.updateArmorStandPreview`:
    default result stacks use `ItemDisplayContext.THIRD_PERSON_LEFT_HAND` at
    the `ItemInHandLayer` sequence before wings/custom-head, while non-skull
    HEAD stacks use `ItemDisplayContext.HEAD` at the `CustomHeadLayer`
    sequence. Tests pin item id/count, foil, full-bright light, no-overlay,
    order, submit_sequence, and display context. Actual GPU PIP item drawing
    remains a later entity-in-UI surface.
- screen integration：
  - [x] inventory local-player entity preview call point：
    native local inventory screen now emits a `HudEntityPreview` for the logged-in
    player, using vanilla `InventoryScreen.extractEntityInInventoryFollowsMouse`
    rect / scale / mouse-follow rotation, full-bright entity light, and isolated
    PIP depth metadata.
  - [x] mount inventory entity preview call point：
    native horse / nautilus mount inventory screens now reuse the same
    entity-in-inventory preview path for the mount entity, with vanilla
    `AbstractMountInventoryScreen` rect / scale / offset and mouse-follow
    rotation.
  - [x] smithing empty armor-stand preview call point：
    native smithing screens emit the vanilla empty armor-stand `HudEntityPreview`
    with fixed rect / scale / translation / rotation, `showArms = true`, and
    hidden base plate; result-slot armor/skull/wings equipment projection is now
    covered above.
  - [x] ordinary container / merchant / recipe-book / book / sign /
    advancement entity-preview call-point audit：
    vanilla 26.1 `rg` over `client/gui/screens` finds entity-in-UI calls only in
    `InventoryScreen`, `AbstractMountInventoryScreen`, `SmithingScreen`, and
    the `CreativeModeInventoryScreen` inventory tab. Generic containers,
    merchant, recipe/book, sign, and advancement screens do not call
    `GuiGraphics.entity` / `extractEntityInInventoryFollowsMouse`, so they have
    no missing entity-preview call point in the current P1-4 surface.
    Creative inventory-tab player preview remains a later creative-screen-state
    presentation slice.
  - [x] GUI pass 与 world pass 的 load/clear/depth ordering：
    renderer source-order tests pin world content rendering into the
    renderer-owned main target before transparency combine/final blit, then HUD
    base commands on the surface, then GUI 3D item pass with a freshly-cleared
    depth attachment, then HUD overlay commands before screenshot readback.
    Entity preview PIP actual GPU drawing remains a later surface, not this
    GUI 3D item depth-ordering slice.

## P1-5：透明排序、粒子与 Level Events

- probe 侧 LevelEvent 形状上下文：
  - [x] 2026-07-02 包分发重构遗留的 probe/runtime parity 缺口已关闭：
    sink-less `PlayApplyEffects` 默认回调现在用 `WorldStore::probe_block`
    查询只读 chunk 上下文；sculk-charge pop 按 world collision shape 判断
    `isCollisionShapeFullBlock`，plant-growth random mode 按 vanilla
    `BoneMealItem.addGrowthParticles` 的 water / `BonemealableBlock.Type`
    分支分类。`bbb-net` probe 测试覆盖加载 full block sculk pop 的 40 粒子
    随机流，以及加载 water growth 的 wide random mode，均在后续声音 seed 前推进。

- 粒子 provider-specific behavior：
  - [x] `PortalParticle.Provider`：renderer descriptor now mirrors vanilla
    random sprite selection, `0.1 * (random * 0.2 + 0.5)` quad size,
    brightness-derived `[0.9, 0.3, 1.0]` RGB scaling, `40..49` lifetime,
    portal `1 - (1-progress)^2` render-size curve, start-position tick path,
    and `(age / lifetime)^4` smooth block-light emission.
  - [x] `ReversePortalParticle.ReversePortalProvider`：renderer descriptor
    now mirrors vanilla inherited portal random sprite/color setup, `1.5`
    quad-size multiplier, overridden `60..61` lifetime after consuming the
    parent portal lifetime draw, `1 - progress / 1.5` render-size curve,
    incremental age-scaled velocity tick path, and inherited quartic smooth
    block-light emission.
  - [x] `WaterCurrentDownParticle.Provider`：renderer descriptor now mirrors
    vanilla random sprite selection, fixed `(0, -0.05, 0)` initial velocity,
    `30 + random.nextFloat() * 60` lifetime range, `0.2..0.8` quad-size
    multiplier, opaque particle layer, no-physics metadata, gravity `0.002`,
    and the custom swirl tick path (`xd += 0.6*cos(angle)`, `zd +=
    0.6*sin(angle)`, horizontal damping `0.07`, `angle += 0.08`). The
    water-fluid / on-ground removal gate remains in the world-coupled
    collision/physics follow-up.
  - [x] `FlyTowardsPositionParticle.EnchantProvider` /
    `NautilusProvider`：renderer descriptor now mirrors vanilla random sprite
    selection, command velocity, initial render position at `spawn + velocity`
    while retaining the original spawn position as the curve start,
    `0.1 * (random * 0.5 + 0.2)` quad size, brightness-derived
    `[0.9, 0.9, 1.0]` RGB scaling, `30..39` lifetime, opaque layer,
    no-physics metadata, fly-towards position curve (`pos = 1 - age/lifetime`,
    `y -= (age/lifetime)^4 * 1.2`), and quartic smooth block-light emission.
  - [x] `FlyTowardsPositionParticle.VaultConnectionProvider`：renderer
    descriptor reuses the fly-towards position curve with vanilla `scale(1.5)`,
    glowing full-block light, translucent layer, and
    `LifetimeAlpha(0.0, 0.6, 0.25, 1.0)` at both runtime tick and partial-tick
    vertex emission.
  - [x] `TotemParticle.Provider`：renderer descriptor now mirrors vanilla age
    sprite selection, command velocity, `0.75` quad-size multiplier,
    `60 + random.nextInt(12)` lifetime, translucent layer, `0.6` friction,
    `1.25` gravity, full-bright light coords, both vanilla random color
    branches, and `SimpleAnimatedParticle` half-lifetime alpha fade.
  - [x] `ShriekParticle.Provider`：native submission now preserves
    `ShriekParticleOption.delay` as explicit `initial_delay_ticks`; renderer
    descriptor mirrors vanilla random sprite selection, fixed `0.85` quad
    size, `30` lifetime, fixed `(0, 0.1, 0)` velocity, translucent layer,
    full-block light override, `0.85 * clamp((age + partial) / lifetime *
    0.75, 0, 1)` size curve, and linear alpha fade after delay clears. The
    delayed particle does not tick or submit vertices while `delay > 0`; once
    visible, vertex collection emits the vanilla two rotated quads from
    `ShriekParticle.extract` (`rotationX(-1.0472)` and
    `rotationYXZ(-PI, 1.0472, 0)`).
  - [x] `SnowflakeParticle.Provider`：renderer descriptor mirrors vanilla age
    sprite selection, pale-blue tint, `0.1 * (random * random + 1.0)` quad
    size, command velocity plus random `+-0.05` per axis,
    `16 / (random * 0.8 + 0.2) + 2` lifetime, `1.0` friction, `0.225`
    gravity, physics metadata, opaque layer, and post-tick damping
    (`xd *= 0.95`, `yd *= 0.9`, `zd *= 0.95`).
  - [x] `SuspendedParticle.SporeBlossomAirProvider`：renderer descriptor now
    mirrors vanilla random sprite selection, `y - 0.125` initial position,
    `(0, -0.8, 0)` velocity, `0.6..1.2` quad-size multiplier, overridden
    `500..1000` lifetime after consuming the constructor lifetime draw,
    `[0.32, 0.5, 0.22]` color, `0.01` gravity, `1.0` friction, no physics,
    opaque layer, and existing `ParticleLimit.SPORE_BLOSSOM` cap/release
    behavior.
  - [x] `DripParticle.NectarFallProvider` /
    `SporeBlossomFallProvider`：renderer descriptor now maps
    `falling_nectar` and `falling_spore_blossom` to random sprites, vanilla
    DripParticle opaque layer, zero initial velocity, physics metadata,
    direct gravity motion with `0.98` friction, fixed tints
    `[0.92, 0.782, 0.72]` / `[0.32, 0.5, 0.22]`, gravity `0.007` /
    `0.005`, and lifetimes `16 / (random * 0.8 + 0.2)` /
    `64 / randomBetween(0.1, 0.9)`. On-ground removal remains in the
    world-coupled collision follow-up.
  - [x] `DripParticle.HoneyHangProvider` / `HoneyFallProvider` /
    `HoneyLandProvider`：renderer descriptor now maps `dripping_honey`,
    `falling_honey`, and `landing_honey` to random sprites, vanilla
    DripParticle opaque layer, zero initial velocity, physics metadata,
    fixed honey tints, `0.98` friction, direct gravity motion, hang-particle
    `0.02` post-move damping, lifetimes `100`,
    `64 / (random * 0.8 + 0.2)`, and
    `128 / (random * 0.8 + 0.2)`, with gravity `0.000012`, `0.01`, and
    `0.06`. Hang-to-fall child spawning, fall-to-land child spawning, local
    drip sound, and on-ground collision remain in the world-coupled
    particle/audio follow-up.
  - [x] `DripParticle.ObsidianTearHangProvider` /
    `ObsidianTearFallProvider` / `ObsidianTearLandProvider`：renderer
    descriptor now maps `dripping_obsidian_tear`, `falling_obsidian_tear`,
    and `landing_obsidian_tear` to random sprites, vanilla DripParticle opaque
    layer, zero initial velocity, physics metadata, fixed purple tint,
    `0.98` friction, direct gravity motion, hang-particle `0.02` post-move
    damping, glowing block-light override, lifetimes `100`,
    `64 / (random * 0.8 + 0.2)`, and
    `28 / (random * 0.8 + 0.2)`, with gravity `0.000012`, `0.01`, and
    `0.06`. Hang-to-fall child spawning, fall-to-land child spawning, and
    on-ground collision remain in the world-coupled particle/audio follow-up.
  - [x] `DripParticle.LavaHangProvider` / `LavaFallProvider` /
    `LavaLandProvider`：renderer descriptor now maps `dripping_lava`,
    `falling_lava`, and `landing_lava` to random sprites, vanilla
    DripParticle opaque layer, zero initial velocity, physics metadata,
    `0.98` friction, direct gravity motion, non-glowing world light, initial
    default-white cooling hang color with runtime
    `CoolingDripHangParticle.preMoveUpdate` RGB formula, hang-particle `0.02`
    post-move damping, lifetimes `40`, `64 / (random * 0.8 + 0.2)`, and
    `16 / (random * 0.8 + 0.2)`, with gravity `0.0012`, `0.06`, and `0.06`.
    Hang-to-fall child spawning, fall-to-land child spawning, lava-fluid
    removal, and on-ground collision remain in the world-coupled particle/audio
    follow-up.
  - [x] `DripParticle.WaterHangProvider` / `WaterFallProvider`：renderer
    descriptor now maps `dripping_water` and `falling_water` to random sprites,
    vanilla DripParticle opaque layer, zero initial velocity, physics metadata,
    fixed blue tint, non-glowing world light, `0.98` friction, direct gravity
    motion, hang-particle `0.02` post-move damping, lifetimes `40` and
    `64 / (random * 0.8 + 0.2)`, with gravity `0.0012` and `0.06`.
    Hang-to-fall child spawning, fall-to-splash child spawning, water-fluid
    removal, and on-ground collision remain in the world-coupled particle/audio
    follow-up.
  - [x] `DripParticle.DripstoneLavaHangProvider` /
    `DripstoneLavaFallProvider` / `DripstoneWaterHangProvider` /
    `DripstoneWaterFallProvider`：renderer descriptor now maps
    `dripping_dripstone_lava`, `falling_dripstone_lava`,
    `dripping_dripstone_water`, and `falling_dripstone_water` to random
    sprites, vanilla DripParticle opaque layer, zero initial velocity, physics
    metadata, non-glowing world light, `0.98` friction, direct gravity motion,
    hang-particle `0.02` post-move damping, lava cooling hang RGB runtime
    formula, water fixed blue tint, lava falling tint, lifetimes `40` for hang
    and `64 / (random * 0.8 + 0.2)` for falling, with gravity `0.0012` and
    `0.06`. Hang-to-fall child spawning, fall-to-land/splash child spawning,
    dripstone local sound, fluid removal, and on-ground collision remain in the
    world-coupled particle/audio follow-up.
  - [x] `SuspendedParticle.CrimsonSporeProvider` /
    `WarpedSporeProvider`：renderer descriptor now mirrors vanilla random
    sprite selection, `y - 0.125` initial position, `0.6..1.2` quad-size
    multiplier, `16 / (random * 0.8 + 0.2)` lifetime, no physics,
    `1.0` friction, zero gravity, opaque layer, crimson gaussian micro-drift
    with `[0.9, 0.4, 0.5]` tint, and warped downward random drift with
    `[0.1, 0.1, 0.3]` tint.
  - [x] `SimpleVerticalParticle.PauseMobGrowthProvider` /
    `ResetMobGrowthProvider`：renderer descriptor/test coverage now pins
    vanilla random sprite selection, `0.5..1.1` quad-size scaling, fixed `8`
    lifetime, command velocity with `-0.03` / `+0.03` y offset, `0.98`
    friction, zero gravity, physics metadata, and opaque particle layer.
  - [x] `SquidInkParticle.Provider` / `GlowInkProvider`：renderer descriptor
    now pins age sprite selection, fixed `0.5` quad size, black / glow-ink
    tint, command velocity, `6 / (random * 0.8 + 0.2)` lifetime, `0.92`
    friction, zero gravity, no physics, full-bright light, translucent layer,
    and `SimpleAnimatedParticle` half-lifetime alpha fade updated on runtime
    ticks and reused during vertex emission. The vanilla in-air downward drift
    remains a world-coupled follow-up because particle ticking does not yet
    query block states.
  - [x] `EndRodParticle.Provider` alpha/layer：renderer descriptor now pins
    command velocity, `0.75` quad-size scaling, age sprites, `60..=71`
    lifetime, `0.91` friction, `0.0125` gravity, full-bright light,
    translucent layer, `SimpleAnimatedParticle` half-lifetime alpha fade, and
    vanilla `setFadeColor(15916745)` RGB fade toward `0xF2DEC9` by 20% per
    tick after half lifetime. The collision-free `move` override remains
    provider-specific follow-up.
  - [x] `LavaParticle.Provider` child smoke：native lava spawn commands now
    carry the smoke child particle template and SpriteSet from pack-backed
    particle definitions; renderer runtime records explicit `LavaSmoke` child
    emission state and, after the lava tick, mirrors vanilla `random.nextFloat()
    > age / lifetime` smoke spawning at the current lava position/velocity.
  - [x] `CampfireSmokeParticle.CosyProvider` /
    `SignalProvider`：renderer descriptor now mirrors vanilla random sprite
    selection, constructor `scale(3.0)`, fixed alpha `0.9` / `0.95`,
    lifetime `80..129` / `280..329`, command x/z velocity with
    `yAux + random.nextFloat() / 500.0`, gravity `3.0E-6`, physics metadata,
    translucent particle layer, random x/z drift, and alpha fade during the
    final 60 ticks. World-collision resolution inside vanilla `move` remains
    with the broader particle collision/physics follow-up.
  - [x] option-colored `SpellParticle` providers：native `LevelParticles`
    command resolution now decodes `SpellParticleOption` RGB + `power` for
    `effect` / `instant_effect` and `ColorParticleOption` ARGB for
    `entity_effect`; renderer maps them to vanilla
    `SpellParticle.InstantProvider` / `MobEffectProvider`, applies option
    tint/alpha, and mirrors `setPower(power)` velocity adjustment.
  - [x] `FireworkParticles.FlashProvider`：native command resolution decodes
    `ColorParticleOption` ARGB for `minecraft:flash`; renderer maps the
    provider to fixed lifetime `4`, translucent layer, random sprite selection,
    option tint, and vanilla overlay `getQuadSize` / extract alpha formulas.
  - [x] `FireworkParticles.SparkProvider`：renderer descriptor maps
    `minecraft:firework` to age sprites, vanilla `SimpleAnimatedParticle`
    friction `0.91`, gravity `0.1`, full-bright light, translucent layer,
    command velocity, `0.75` quad-size scale, fixed initial alpha `0.99`,
    `48 + random.nextInt(12)` lifetime, and the half-lifetime alpha fade
    formula. Firework `Starter` trail/twinkle child spawning and audio remain
    deferred to the broader firework rocket / level-event presentation slice.
  - [x] `TrailParticle.Provider`：native command resolution decodes
    `TrailParticleOption` target / RGB color / duration for `minecraft:trail`;
    renderer maps the provider to random sprite selection, option duration,
    vanilla random RGB scaling, `0.26` quad size, opaque layer, full-bright
    light, command velocity, and target interpolation toward the option target.
  - [x] `VibrationSignalParticle.Provider` block target path：native command
    resolution decodes `VibrationParticleOption` block `PositionSource` into
    the block-center `option_target` plus arrival ticks for `minecraft:vibration`;
    renderer maps the provider to random sprite selection, option duration,
    fixed `0.3` quad size, translucent layer, full-block light, zero initial
    velocity, target interpolation, vanilla yaw/pitch/sway state, and the two
    rotated quads from `VibrationSignalParticle.extract`. Entity
    `PositionSource` is consumed without fabricating a target; unresolved
    vibration instances stay out of vertex submission until world/entity lookup
    is available.
  - [x] `TrialSpawnerDetectionParticle.Provider` and
    `SingleQuadParticle.FacingCameraMode.LOOKAT_Y`：renderer now records
    per-instance facing mode and maps `trial_spawner_detection` /
    `_ominous` to age sprites, `scale(1.5)` over the vanilla `0.75`
    single-quad scale, command velocity, `12 / (0.5 + random * 0.5)`
    lifetime, opaque layer, full-block light, grow-to-base size curve,
    physics metadata, and `LOOKAT_Y` vertex transform with world-Y up
    instead of full camera pitch.
  - [x] `DustPlumeParticle.Provider`：renderer descriptor now maps
    `minecraft:dust_plume` to age sprites, vanilla `BaseAshSmokeParticle`
    `0.75` quad-size scale, `7 / (random * 0.8 + 0.2)` lifetime, command
    velocity with `+0.15` y offset, opaque layer, no-physics metadata,
    `0.5` initial gravity, `0.96` friction, `ARGB(0xBAB1C2) - random * 0.2`
    tint, grow-to-base size curve, and the provider tick override that applies
    `gravity *= 0.88` and `friction *= 0.92` before default particle motion.
  - [x] `WaterDropParticle.Provider` / `SplashParticle.Provider`：renderer
    descriptor now maps `rain` and `splash` to random sprites, vanilla
    single-quad size, `8 / (random * 0.8 + 0.2)` lifetime, opaque layer,
    physics metadata, `0.98` friction, direct gravity motion (`yd -=
    gravity` rather than default `0.04 * gravity`), and water-drop damping.
    `rain` mirrors the constructor random x/z velocity damped by `0.3` plus
    `0.1..0.3` y velocity and `0.06` gravity; `splash` uses `0.04` gravity
    and preserves vanilla's horizontal command branch (`ya == 0 && (xa || za)`)
    as `(xa, 0.1, za)`. Ground/block/fluid removal remains in the
    world-coupled collision follow-up.
  - [x] `WakeParticle.Provider`：renderer descriptor now maps `fishing` to
    first sprite initialization, vanilla single-quad size,
    `8 / (random * 0.8 + 0.2)` lifetime, command velocity, opaque layer,
    physics metadata, `0.98` friction, zero gravity, direct motion, damping,
    and the vanilla wake sprite cycle using `SpriteSet.get((60 - lifetime) %
    4, 4)` during ticks.
  - [x] `FlyStraightTowardsParticle.OminousSpawnProvider`：renderer descriptor
    now maps `ominous_spawning` to random sprites, command velocity, initial
    position at `spawn + velocity` while retaining `spawn` as the interpolation
    start, vanilla `0.1 * (random * 0.5 + 0.2)` quad size followed by
    `scale(randomBetween(3, 5))`, `25 + random * 5` lifetime, opaque layer,
    no-physics metadata, full-block light, and the straight-toward tick path
    plus `ARGB.srgbLerp` from `0xFF45AEFE` to white.
  - [x] `FireflyParticle.FireflyProvider`：renderer descriptor now maps
    `minecraft:firefly` to random sprites, vanilla `200..300` inclusive
    lifetime, initial alpha `0`, translucent layer, `speedUpWhenYMotionIsBlocked`,
    `0.96` friction, provider aux velocity (`0.5 - random.nextDouble()` x/z
    and signed `yAux`) through the vanilla `Particle` constructor followed by
    `*0.8`, `0.75 * scale(1.5)` quad-size path, first-tick / 5% random speed
    reroll, alpha fade (`0.3` / `0.5`) and direct smooth block-light fade
    (`0.1` / `0.3`). The in-block removal gate remains a world-coupled
    collision/state follow-up.
  - [x] `FallingLeavesParticle.CherryProvider` / `PaleOakProvider` /
    `TintedLeavesProvider`：native command resolution now decodes
    `ColorParticleOption` ARGB for `minecraft:tinted_leaves`; renderer maps
    `cherry_leaves`, `pale_oak_leaves`, and `tinted_leaves` to random sprites,
    fixed `300` lifetime, opaque layer, `1.0` friction, physics metadata,
    vanilla `scale * (0.05 | 0.075)` quad-size choice, cherry flow-away
    parameters `(fall=0.25, side=2.0, startVelocity=0.0)`, pale/tinted swirl
    parameters `(fall=0.07, side=10.0, startVelocity=0.021)`, tinted RGB
    option color with alpha preserved at the particle default, gravity
    `fallAcceleration * 1.2 * 0.0025`, flow/swirl acceleration, and roll
    spin acceleration. The on-ground / blocked-axis removal gate remains in
    the world-coupled particle collision/physics follow-up.
  - [x] `SculkChargeParticle.Provider` roll：native command resolution decodes
    `SculkChargeParticleOptions.roll` for `minecraft:sculk_charge`; renderer
    stores it as initial `oRoll` / `roll` and applies the vanilla billboard
    roll transform during vertex emission.
  - [x] `DustParticle.Provider` / `DustColorTransitionParticle.Provider`：
    native command resolution decodes RGB color(s) and clamped scale for
    `minecraft:dust` / `minecraft:dust_color_transition`; renderer maps both
    providers to age sprites, opaque layer, vanilla scale-shaped quad size /
    lifetime, random color variation, and transition partial-tick color lerp.
  - [x] `HugeExplosionSeedParticle.Provider`：native command resolution now
    allows the definition-less no-render `minecraft:explosion_emitter` while
    attaching the `minecraft:explosion` child SpriteSet; renderer maps it to
    fixed lifetime `8`, no-render group, six child explosion submissions per
    tick, vanilla `nextDouble() - nextDouble()` offsets scaled by `4.0`, and
    child xAux `age / lifetime` feeding `HugeExplosionParticle` quad size.
  - [x] `GustSeedParticle.Provider`：native command resolution now allows the
    definition-less no-render `gust_emitter_large` / `_small` particle types
    while attaching the `minecraft:gust` child SpriteSet; renderer maps the
    providers as no-render seed particles with vanilla constructor parameters
    `(scale=3.0, lifetime=7, delay=0)` / `(scale=1.0, lifetime=3, delay=2)`,
    inclusive age ticks, three child `gust` submissions when
    `age % (delay + 1) == 0`, random `nextDouble() - nextDouble()` offsets,
    and the vanilla child xAux `age / lifetime` (which `GustParticle.Provider`
    itself ignores).
  - [x] `ElderGuardianParticle.Provider`：native command resolution now allows
    the definition-less `minecraft:elder_guardian` special particle type;
    renderer records fixed lifetime `30`, zero aux/motion/gravity provider
    metadata, translucent `entityTranslucent` intent, and vanilla
    `ParticleRenderType.ELDER_GUARDIANS` group while keeping the atlas
    billboard path limited to `SINGLE_QUADS`. Actual elder guardian model
    rendering remains follow-up special-group visual parity.
  - [x] vanilla `ParticleResources.registerProviders()` descriptor coverage
    guard：renderer tests now enumerate every 26.1 registered particle id and
    assert it maps to an explicit vanilla provider descriptor rather than the
    generic `Particle` fallback. Remaining particle P1/P2 work is now scoped to
    terrain/item atlas rendering, world-coupled collision/tint, LevelEvent
    branches, atlas animation, or special-group drawing rather than silent
    provider fallback.
  - [x] smoke-family initial velocity (`minecraft:smoke` / `large_smoke` /
    `white_smoke`)：renderer descriptors now use
    `ParticleConstructorZeroScaledPlusCommand { scale: 0.1 }` instead of a
    pure command velocity, matching vanilla `SmokeParticle` /
    `WhiteSmokeParticle` (`super(..., 0.1F, 0.1F, 0.1F, xa, ya, za, ...)`) and
    `BaseAshSmokeParticle` base-`Particle` normalized random spread — the same
    velocity shape already reused by `PlayerCloudParticle`. Deterministic
    seed=0 tests pin the intake velocity to `descriptor.sample` (and lava sub
    smoke now correctly layers base spread over the lava command velocity).
    `ash` / `white_ash` keep pure command velocity as follow-up because their
    non-uniform `dir=(0.1, -0.1, 0.1)` (plus `WhiteAsh` provider random offset)
    needs a per-axis-dir velocity variant.
  - [x] ash / white_ash per-axis-dir initial velocity：renderer descriptors now
    use a new `ParticleInitialVelocityDescriptor::BaseAshSmokeSpread { dir,
    provider_offset }` variant (with `BaseAshSmokeOffset::{Zero, WhiteAsh}`)
    instead of pure command velocity, matching vanilla `BaseAshSmokeParticle`
    (`super(..., 0.0, 0.0, 0.0, ...)` base spread, then `xd *= dirX; yd *= dirY;
    zd *= dirZ` with `dir=(0.1, -0.1, 0.1)`, then `xd += xa; yd += ya; zd += za`).
    `AshParticle.Provider` forces the provider velocity to `(0, 0, 0)` and
    `WhiteAshParticle.Provider` ignores the command velocity and adds its own
    negative-biased `xa/ya/za` (`rand*-1.9*rand*0.1`, `rand*-0.5*rand*0.1*5.0`,
    `rand*-1.9*rand*0.1`). Deterministic seed=0 tests reconstruct the base spread
    and offset straight from the vanilla source lines (independent of the
    descriptor under test) and pin the intake velocity, that command velocity is
    ignored, that y is negated+damped, and that white_ash biases y downward.
    Follow-up: `minecraft:dust_plume` also extends `BaseAshSmokeParticle`
    (`dir=(0.7, 0.6, 0.7)`, `ya + 0.15F`) and still uses `CommandWithYOffset`, so
    it has the same missing base-spread×dir shape and needs the same treatment.
  - [x] dust_plume per-axis-dir initial velocity：renderer descriptor now maps
    `minecraft:dust_plume` initial velocity to the same
    `ParticleInitialVelocityDescriptor::BaseAshSmokeSpread { dir, provider_offset }`
    variant instead of the flat `CommandWithYOffset` path, matching vanilla
    `DustPlumeParticle extends BaseAshSmokeParticle`
    (`super(..., 0.7F, 0.6F, 0.7F, xa, ya + 0.15F, za, ...)`): the `Particle`
    7-arg normalized base spread scaled per axis by `dir=(0.7, 0.6, 0.7)`, then
    the command velocity added on top with a `+0.15` y offset. A new
    `BaseAshSmokeOffset::CommandWithYOffset { y_offset }` provider variant threads
    the command velocity (`xa/ya/za`) and adds `y_offset` to `ya`, drawing no RNG
    of its own (`DustPlumeParticle.Provider` passes the command velocity straight
    through). Deterministic seed=86 tests reconstruct the dir-scaled base spread
    from the vanilla source and pin the intake velocity to
    `spread + command_velocity` with `+0.15` on y, resolving the ash/white_ash
    dust_plume follow-up above. The per-tick `gravity *= 0.88` / `friction *= 0.92`
    decay was already implemented as `ParticleTickMotionDescriptor::DustPlume`.
  - [x] trial_spawner_detection / _ominous initial velocity：the shared
    descriptor arm now uses `BaseAshSmokeSpread { dir: [0.0, 0.9, 0.0],
    provider_offset: BaseAshSmokeOffset::CommandWithYOffset { y_offset: 0.0 } }`
    instead of flat `Command`, matching vanilla `TrialSpawnerDetectionParticle`
    (`super(..., 0.0, 0.0, 0.0, ...)` normalized base spread, then
    `xd *= 0.0; yd *= 0.9; zd *= 0.0` and the command velocity added straight
    through — `TrialSpawnerDetectionParticle.Provider` passes `xAux/yAux/zAux`
    with no offset and draws no RNG). x/z drop the base spread (command passes
    through) while the upward y drift is scaled by `0.9`; flat `Command` had
    dropped that y drift. A deterministic seed=51 witness reconstructs the
    dir-scaled base spread from the vanilla source and pins x/z passthrough plus
    the `0.9`-scaled y drift. A vanilla-provider audit of the remaining
    flat-`Command` particles found the rest (fishing, bubble_pop, squid_ink,
    glow_squid_ink, enchant, nautilus, totem_of_undying, end_rod, sculk_charge,
    firework, portal, reverse_portal, etc.) genuinely pass their aux velocity
    straight to the base `Particle` ctor, so flat `Command` is correct there.
- 粒子 sorting：
  - [x] single-quad particle render group / layer order：renderer
    `ParticleInstance` now records vanilla `ParticleRenderType.SINGLE_QUADS`
    and `SingleQuadParticle.Layer` opaque/translucent metadata for the covered
    providers, and vertex collection uses the vanilla `ParticleEngine`
    group order (`SINGLE_QUADS`, then future `ITEM_PICKUP`, then
    `ELDER_GUARDIANS`) with solid layers before translucent layers while
    preserving insertion order inside the same group/layer; the current atlas
    billboard submission path now explicitly consumes only `SINGLE_QUADS` and
    leaves `ELDER_GUARDIANS` for the later model-special render path.
  - [x] current particle-atlas solid vs translucent pass split：single-quad
    atlas billboards now collect separate opaque/translucent vertex batches and
    draw them through vanilla-shaped `OPAQUE_PARTICLE` (no blend) and
    `TRANSLUCENT_PARTICLE` (`BlendFunction.TRANSLUCENT`) pipelines, preserving
    the existing group/layer order and renderer-owned LightTexture sampling.
  - [x] non-particle-atlas terrain/item layer metadata baseline：
    definition-less vanilla block/item atlas particles (`block`, `block_marker`,
    `dust_pillar`, `block_crumble`, `item`, `item_slime`, `item_cobweb`,
    `item_snowball`) now reach renderer submission as commands without
    particle JSON definitions, preserve `raw_options_len`, and record
    terrain/item layer order metadata (`OPAQUE_TERRAIN` / `OPAQUE_ITEMS`)
    instead of falling into generic `OPAQUE`; `falling_dust` remains ordinary
    particle-atlas `OPAQUE` per vanilla. Sprite-transparency-driven
    `TRANSLUCENT_TERRAIN` / `TRANSLUCENT_ITEMS` selection and actual block/item
    atlas UV/tint resolution remain follow-up work.
  - [x] terrain/item particle option metadata baseline：native now decodes
    vanilla `BlockParticleOption` block-state ids for `block`, `block_marker`,
    `falling_dust`, `dust_pillar`, and `block_crumble`, and decodes
    `ItemParticleOption` `ItemStackTemplate` item id / count plus raw component
    patch byte length for `item`; renderer `ParticleSpawnCommand` and
    `ParticleInstance` preserve that metadata beside the terrain/item layer.
  - [x] terrain/item atlas provider-shape, sub-rect metadata, and UV emission
    baseline：
    renderer descriptors now map `TerrainParticle.Provider`,
    `TerrainParticle.DustPillarProvider`,
    `TerrainParticle.CrumblingProvider`, `BlockMarker.Provider`, and
    `BreakingItemParticle` item/slime/cobweb/snowball providers to vanilla
    lifetime, gravity/physics, half-size visual state, fixed terrain gray, and
    item-white metadata; `ParticleInstance` records the vanilla random
    `uo`/`vo` 4x4 sub-rect offsets for `TerrainParticle` /
    `BreakingItemParticle` paths while leaving `block_marker` and
    `falling_dust` unmarked. Billboard vertex emission now converts those
    offsets into vanilla-shaped atlas sub-rect UVs, including the
    `TerrainParticle` / `BreakingItemParticle` horizontal `u0`/`u1` flip, when a
    concrete sprite UV is available.
  - [x] particle texture-atlas ownership metadata baseline：`ParticleInstance`
    now carries explicit `ParticleTextureAtlasKind` derived from vanilla
    `SingleQuadParticle.Layer` intent: ordinary particle layers use
    `Particles`, block / block_marker / dust_pillar / block_crumble layers use
    `Terrain`, and item / slime / cobweb / snowball layers use `Items`.
  - [x] `falling_dust` provider-specific particle-atlas behavior：
    renderer now maps `FallingDustParticle.Provider` as ordinary
    particle-atlas `OPAQUE`, keeps zero constructor velocity, age sprite
    selection, vanilla `32/(random*0.8+0.2)*0.9` lifetime shape,
    `0.67499995` quad-size multiplier with grow-to-base render curve, and
    per-instance roll / rotSpeed runtime motion with Y velocity clamped to
    `-0.14`. Block-state dust tint, invisible-block spawn rejection, and
    on-ground roll reset stay in world-coupled particle follow-up work.
  - [x] `falling_dust` invisible render-shape spawn rejection：native
    `LevelParticles` resolution now mirrors
    `FallingDustParticle.Provider` by rejecting non-air `BlockParticleOption`
    states whose vanilla render shape is `INVISIBLE`, including barrier and
    liquid block states, while still allowing air and preserving packet sample
    RNG consumption before the provider-null result. Block-state dust tint and
    on-ground roll reset remain world/collision-coupled follow-up work.
  - [x] `falling_dust` FallingBlock dust-color tint：native
    `LevelParticles` resolution now mirrors the provider's
    `FallingBlock#getDustColor` branch for vanilla 26.1 sand, red_sand,
    gravel, dragon_egg, anvils, and concrete_powder states by projecting the
    resolved RGB into `ParticleSpawnCommand.option_color`; renderer
    `FallingDustParticle.Provider` instances consume that option color as their
    visual tint. Non-FallingBlock `BlockColors` / map-color fallback and
    on-ground roll reset remain terrain/collision-coupled follow-up work.
  - [x] `ParticleLimit.SPORE_BLOSSOM` active-count cap：renderer runtime
    按 vanilla `ParticleEngine.add` / `ParticleLimit.SPORE_BLOSSOM(1000)`
    拒收第 1001 个 `SuspendedParticle.SporeBlossomAirProvider`
    (`minecraft:spore_blossom_air`) 粒子，不淘汰已接纳粒子，并在粒子过期时释放
    limit 计数；`overrideLimiter` 仍只属于距离/选项降采样，不绕过该分组 limit。
  - [x] client `ParticleStatus` / distance thinning settings：native
    `LevelParticles` dispatch now mirrors vanilla `ClientLevel.doAddParticle`：
    非 override 粒子在 camera eye distance squared `> 1024.0` 时丢弃；
    override-limiter 粒子绕过距离和粒子选项；`--client-particles` 固定
    `ALL` / `DECREASED` / `MINIMAL`，并用 vanilla-shaped Java `Random.nextInt`
    执行 `DECREASED` 的 `nextInt(3)` drop 与 `MINIMAL && alwaysShow` 的
    `nextInt(10)` promote / second `nextInt(3)` drop。
- atlas mip / animation：
  - [x] age-based animated sprite frame advance：renderer particle runtime
    mirrors vanilla `SpriteSet.get(index, max)` frame selection with
    `index * (sprites.size() - 1) / max`, advances age-selected sprites on
    client ticks, keeps random-selected sprites stable, and reaches the last
    frame at the lifetime boundary.
  - [x] missing definition / missing sprite diagnostics：native particle
    resolution records missing definitions, unknown particle types, and missing
    sprites without dropping otherwise renderable spawn commands; renderer
    batch/counter paths preserve those diagnostic counts.
- LevelEvent particle side effects：
  - [x] ender-eye break portal ring：event `2003` now emits the vanilla
    double portal ring (`angle += PI / 20`, velocity radii `-5` and `-7`)
    after consuming the preceding eight `ItemParticleOption(Items.ENDER_EYE)`
    random draws; the ender-eye item particles remain deferred with item
    particle atlas rendering.
  - [x] splash / instant splash potion break spell particles：events `2002` and
    `2007` now consume the preceding eight `ItemParticleOption(Items.SPLASH_POTION)`
    random draws, then emit 100 vanilla-positioned `minecraft:effect` /
    `minecraft:instant_effect` submissions with event-data RGB, random
    brightness, and `SpellParticleOption` power. The splash-potion item
    particles remain deferred with item particle atlas rendering.
  - [x] bee-growth / turtle-egg-placement happy-villager in-block particles：
    events `2011` (`PARTICLES_BEE_GROWTH`) and `2012`
    (`PARTICLES_TURTLE_EGG_PLACEMENT`) now mirror vanilla
    `ParticleUtils.spawnParticleInBlock` for the air/default spread-height
    branch, using event `data` as count, gaussian `0.02` velocity on all axes,
    and full-block random positions.
  - [x] smash-attack dust-pillar particles：event `2013` now mirrors vanilla
    `ParticleUtils.spawnSmashAttackParticles`, using event `data` for the two
    float-bounded loop counts, the event-position block state as
    `BlockParticleOption(ParticleTypes.DUST_PILLAR, state)`, and air state `0`
    as the no-world-context fallback.
  - [x] sculk-shrieker particles：event `3007`
    (`PARTICLES_SCULK_SHRIEK`) now emits the vanilla ten
    `minecraft:shriek` submissions at block center / `SculkShriekerBlock.TOP_Y`
    with `ShriekParticleOption(i * 5)` delays. The waterlogged-gated
    `SCULK_SHRIEKER_SHRIEK` sound branch remains audio/world-state follow-up.
  - [x] pointed-dripstone drip LevelEvent：event `1504` now mirrors vanilla
    `LevelEventHandler` -> `PointedDripstoneBlock.spawnDripParticle` for
    loaded client block state: native validates a downward, unwaterlogged
    `tip`, follows the same upward root search, samples root-above water/lava
    or mud-as-water outside water-evaporating dimensions, falls back to the
    built-in default dripstone particle (`water` outside the Nether, `lava` in
    the Nether), and submits the deterministic `offsetType(XZ)` drip position.
    Custom dimension `visual/default_dripstone_particle` attributes remain P3
    resource-registry follow-up.
  - [x] plant-growth LevelEvent：event `1505`
    (`PARTICLES_AND_SOUND_PLANT_GROWTH`) now mirrors vanilla
    `BoneMealItem.addGrowthParticles` branch selection for loaded client block
    state: BonemealableBlock grower/in-block happy-villager particles,
    rooted-dirt and mangrove-leaves below-position grower particles, water and
    neighbor-spreader wide spread (`count * 3`, `spreadWidth=3.0`,
    `spreadHeight=1.0`), and the `allowFloatingParticles=false` non-air support
    filter across the sampled 7x7 below layer. Shape-sensitive
    `spawnParticleInBlock` heights remain follow-up with the broader block-shape
    particle work.
  - [x] plant-growth LevelEvent sound ordering：event `1505` now records and
    plays the vanilla `minecraft:item.bone_meal.use` positioned sound only after
    the `BoneMealItem.addGrowthParticles` random sequence, including the
    audio-only fallback path where no particle sink is attached, so the sound
    seed matches the post-particle client random state.
  - [x] shape-sensitive in-block LevelEvent particles：events `2011` /
    `2012` and the grower branch of event `1505` now thread loaded block-state
    outline shape max-Y into vanilla `ParticleUtils.spawnParticleInBlock`
    spread height. Missing/unloaded context still uses the vanilla air/default
    `1.0` height; the remaining `2002` / `2003` / `2007` item-particle portions
    stay deferred to terrain/item particle atlas rendering.
  - [x] LevelEvent item-break particles：events `2002` / `2007` now submit the
    eight preceding `minecraft:item` splash-potion break particles with
    vanilla `ItemParticleOption(Items.SPLASH_POTION)`, center position, and
    gaussian/upward velocity before the 100 spell particles; event `2003` now
    submits the eight `minecraft:item` ender-eye break particles with
    `ItemParticleOption(Items.ENDER_EYE)` before the double portal ring. The
    item commands remain definition-less item-atlas submissions carrying raw
    option length plus item id/count/component-patch metadata; actual item
    sprite lookup and terrain/items particle atlas GPU binding stay with the
    broader terrain/item particle rendering backlog.
  - [x] LevelEvent block-destroy particles：events `2001`
    (`PARTICLES_DESTROY_BLOCK`) and `3008`
    (`PARTICLES_AND_SOUND_BRUSH_BLOCK_COMPLETE`) now submit vanilla
    `ClientLevel.addDestroyBlockEffect`-shaped `0.25` density grids from the
    event `data` block-state id, using known native block-outline boxes with a
    full-cube fallback for unsupported shapes. The emitted definition-less
    `minecraft:block` commands carry `BlockParticleOption` metadata, and air
    state `0` is skipped. True `shouldSpawnTerrainParticles` / moving-piston
    rejection, terrain tint, and terrain-atlas GPU binding remain with broader
    block/terrain particle work.
  - [x] `TerrainParticle.createTerrainParticle` provider rejection：native
    `LevelParticles` resolution now drops definition-less `minecraft:block`,
    `minecraft:dust_pillar`, and `minecraft:block_crumble` submissions for air,
    `moving_piston`, and `shouldSpawnTerrainParticles=false` block states after
    packet sample RNG is consumed. `minecraft:block_marker` remains unfiltered
    like vanilla `BlockMarker.Provider`.
  - [x] LevelEvent terrain-particle rejection filters：events `2001` / `3008`
    now follow `ClientLevel.addDestroyBlockEffect` by skipping air and
    `shouldSpawnTerrainParticles=false` states while still allowing
    `moving_piston`; event `2013` now keeps the vanilla
    `TerrainParticle.DustPillarProvider` rejection for air, `moving_piston`, and
    no-terrain-particle states, preserving the event random draws before the
    rejected provider result.
  - [x] sculk-shrieker LevelEvent sound：event `3007` now records and plays the
    vanilla `SCULK_SHRIEKER_SHRIEK` positioned sound after the shriek particle
    branch, using `SculkShriekerBlock.TOP_Y`, volume `2.0`,
    `0.6 + random.nextFloat() * 0.4` pitch, and the loaded block state's
    `waterlogged=true` gate.
  - [x] brush-block-complete LevelEvent sound：event `3008` now mirrors
    vanilla `LevelEventHandler` by resolving the event-data block state through
    `BrushableBlock.getBrushCompletedSound()` for suspicious sand/gravel and
    recording/playing the matching `minecraft:item.brush.brushing.sand` /
    `minecraft:item.brush.brushing.gravel` positioned sound with
    `SoundSource.PLAYERS`, volume `1.0`, and pitch `1.0`. Non-brushable
    event-data block states keep the vanilla no-sound branch while destroy-block
    particles still run.
  - [x] honeycomb wax-on LevelEvent sound ordering：event `3003` now mirrors
    vanilla `LevelEventHandler` by emitting the six-face `minecraft:wax_on`
    particle random sequence before recording/playing
    `minecraft:item.honeycomb.wax_on`; audio-only dispatch advances the same
    `UniformInt.of(3,5)` block-face particle random stream before drawing the
    positioned sound seed.
  - [x] dragon fireball LevelEvent sound ordering：event `2006` now mirrors
    vanilla `LevelEventHandler` by consuming the 200
    `minecraft:dragon_breath` particle random draws before recording/playing
    `minecraft:entity.dragon_fireball.explode` when `data == 1`; `data != 1`
    remains particle-only.
  - [x] potion break LevelEvent sound ordering：events `2002` / `2007` now
    mirror vanilla `LevelEventHandler` by consuming the eight
    `ItemParticleOption(Items.SPLASH_POTION)` break particles and 100
    `minecraft:effect` / `minecraft:instant_effect` spell-particle random draws
    before recording/playing `minecraft:entity.splash_potion.break`.
  - [x] vault activate/deactivate LevelEvent sounds：events `3015` and `3016`
    now mirror vanilla `LevelEventHandler` sound ordering by consuming the local
    vault particle random sequence before drawing
    `(nextFloat - nextFloat) * 0.2 + 1.0` pitch, then recording/playing
    distance-delayed `minecraft:block.vault.activate` /
    `minecraft:block.vault.deactivate` positioned sounds with
    `SoundSource.BLOCKS` and volume `1.0`. Event `3015` keeps the vanilla loaded
    vault block-entity gate; event `3016` always emits the deactivation sound.
    Player connection particles for activation remain broader block-entity
    client-effects follow-up work.
  - [x] solid item enchantment glint：`ItemStackSummary::has_foil()` now
    centralizes the decoded `enchantment_glint_override` / enchantments
    fallback, native projects foiled dropped / held / item-frame / HUD 3D block
    item solid quads into an item glint mesh bucket, and the renderer uploads
    `textures/misc/enchanted_glint_item.png` for vanilla `RenderTypes.glint()`
    draws using `GLINT_TEXTURING` scale `8.0`, GLINT blend, depth-equal,
    no-depth-write, no-cull, and no-lightmap shader state. At that point
    `glintTranslucent`, SPECIAL foil decal pose, and 2D HUD/inventory sprite
    glint remained follow-ups.
  - [x] item-model `glintTranslucent`：foiled dropped / held / item-frame
    translucent item-model quads now mirror into a dedicated
    `glint_translucent` mesh bucket, native aggregates that bucket separately
    from solid item glint, and the renderer draws it inside the itemEntity target
    after translucent item base geometry so vanilla `RenderTypes.glintTranslucent()`
    keeps `OutputTarget.ITEM_ENTITY_TARGET`, `GLINT_TEXTURING`, depth-equal,
    no-depth-write, no-cull, and no-lightmap state. SPECIAL foil decal pose, GUI
    transparent 3D icon parity, and 2D HUD/inventory sprite glint remain follow-ups.
  - [x] SPECIAL item foil decal UVs for current world consumers：`NativeItemRuntime`
    now detects vanilla `CuboidItemModelWrapper.hasSpecialAnimatedTexture`
    (`minecraft:clock` plus current `tags/item/compasses`) and projects foiled
    dropped / held / item-frame item-model submits as `ItemModelFoil::Special`.
    The renderer bakes those glint copies through a `SheetedDecalTextureGenerator`
    shaped UV projection (`textureScale = 1/128`) while base geometry keeps atlas
    UVs; GUI `0.5` and first-person `0.75` decal pose scales are represented in
    the API, but GUI flat sprite and first-person visual consumers remain later
    presentation work.
  - [x] composter fill LevelEvent particles：event `1500` now mirrors vanilla
    `ComposterBlock.handleFill` for ten `minecraft:composter` particles,
    including gaussian `0.02` velocity, `0.1875 + 0.625 * randomFloat` X/Z
    spread, and Y from the loaded block shape's center-column max-Y plus
    `0.03125`; missing/unloaded context falls back to vanilla full-block/unknown
    height.
  - [x] vault activation LevelEvent particles：event `3015` now mirrors the
    local cage smoke/flame branch of
    `VaultBlockEntity.Client.emitActivationParticles`, gated on a loaded vault
    block entity at the event position and emitting 20 `minecraft:smoke`
    particles plus 20 normal/ominous flame particles from
    `randomPosInsideCage`. Player connection particles remain deferred with
    broader block-entity client effects.
  - [x] charged sculk block-face particles：event `3006` now mirrors the
    `count = data >> 6` charged branch for `count > 0`, including
    `UniformInt.of(0, count)` face repetition, full-block six-face vs
    `MultifaceBlock.unpack(data & 63)` face selection, vanilla `0.65` /
    `0.57` / `0.35` step factors, `+-0.005` speed supplier, and
    `SculkChargeParticleOptions` roll (`DOWN` for full block, `UP` for
    multiface); the `count == 0` branch now emits vanilla
    `minecraft:sculk_charge_pop` submissions with target-block full-shape
    context (`40` particles, `0.45` spread) vs non-full/unknown context
    (`20` particles, `0.25` spread), and `0.07` velocity scale.
  - [x] vault deactivation particles：event `3016` now mirrors vanilla
    `VaultBlockEntity.Client.emitDeactivationParticles`, selecting
    `minecraft:small_flame` for `data == 0` or `minecraft:soul_fire_flame`
    otherwise, spawning 20 particles from `randomPosCenterOfCage`
    (`0.4..0.6` on each axis) with gaussian `0.02` velocity.
  - [x] post-sound smoke LevelEvent random stream：events `1501`, `1502`, and
    `1503` now preserve vanilla `LevelEventHandler` ordering in audio-only
    dispatch by recording/playing the lava extinguish, redstone torch burnout,
    or end portal frame fill sound first, then advancing the smoke particle
    random draws before later LevelEvent sounds draw their seeds.
  - [x] trial-spawner post-sound LevelEvent random stream：events `3012`,
    `3013`, `3014`, `3019`, `3020`, and `3021` now preserve vanilla
    `LevelEventHandler` ordering in audio-only dispatch by recording/playing
    the distance-delayed trial-spawner sound first, then advancing the spawn,
    detect-player, eject-item, or become-ominous particle random draws before
    later LevelEvent sounds draw their seeds.
  - [x] sculk charge post-sound LevelEvent random stream：event `3006` now
    preserves vanilla `LevelEventHandler` ordering in audio-only dispatch for
    both charged and fixed-pop branches by recording/playing the sculk charge
    sound first, then advancing charged block-face or pop-particle random draws
    before later LevelEvent sounds draw their seeds. The pop branch threads the
    same full-block context used by visible particles, so full blocks advance
    the 40-particle stream while partial/unknown blocks advance 20.
  - [x] simple particle-only LevelEvent random stream：events `2000`, `2003`,
    `2004`, `2009`, and `2010` now advance their vanilla dispenser-smoke,
    ender-eye item-break, blaze smoke/flame, splash cloud, or white-smoke
    particle random draws in audio-only dispatch before later LevelEvent sounds
    draw their seeds.
  - [x] block-face / axis particle-only LevelEvent random stream：events
    `3002`, `3004`, `3005`, and `3009` now advance their vanilla electric-spark
    axis or block-face, wax-off, scrape, and egg-crack random draws in
    audio-only dispatch before later LevelEvent sounds draw their seeds.
  - [x] LevelEvent particle side-effects coverage：native particle resolution
    now has a coverage test enumerating every vanilla 26.1 `LevelEventHandler`
    event id that emits particles and verifying a representative mapped batch
    with no missing definition / sprite diagnostics. The remaining LevelEvent
    backlog is audio ordering / world-context refinement, not unmapped particle
    event cases.
  - [x] particle light-curve coverage：renderer descriptor tests now enumerate
    the vanilla 26.1 `getLightCoords` override families from
    `net.minecraft.client.particle`: full-bright particles, forced block-light
    particles, age-based smooth block emission, portal/enchant quartic emission,
    `FireflyParticle` light fade, and world-sampled counterexamples. The
    unsupported-features particle runtime ledger no longer tracks light curves
    as an open renderer slice.
  - [x] particle lifetime coverage：renderer descriptor tests now sample every
    `ParticleLifetimeDescriptor` formula family against vanilla 26.1 constructor
    / provider ranges, including base particle, rising, player cloud,
    ash-smoke, crit/random divisor, command-option, portal/reverse-portal,
    falling-dust, dust-scale, and inclusive-tick lifetimes. The P1-5 provider
    checklist no longer tracks lifetime as an open slice.
  - [x] particle quad-size curve coverage：renderer now distinguishes vanilla
    `FlameParticle.getQuadSize` (`1 - progress^2 * 0.5`) from
    `LavaParticle.getQuadSize` (`1 - progress^2`) instead of sharing the flame
    half-shrink curve. Runtime tests cover every modeled quad-size curve:
    constant, grow-to-base, flame, lava, flash overlay, portal, reverse portal,
    and shriek. The P1-5 provider checklist no longer tracks size curve as an
    open slice.

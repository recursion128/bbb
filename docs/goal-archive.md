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
  - [x] opaque 粒子 target ownership（2026-07-05，新增 `opaque_particle_main_pass`
    step）：opaque（`translucent == false`）single-quad 粒子从 `particle_target_pass`
    挪到新的 `opaque_particle_main_pass`，画进 main color+depth，且位于
    `copy_main_depth_to_feature_targets` 之前，使 opaque 粒子深度随主深度拷贝进入
    translucent / itemEntity / particles feature target；translucent 粒子继续留在
    `particle_target_pass` 的 particles target。vanilla 依据：
    `ParticleFeatureRenderer.render`（26.1 lines 46-57，
    `useParticleTarget = particleTarget != null && translucent`）与
    `LevelRenderer.addMainPass`（26.1：`renderSolidFeatures` line 675 在三个
    `copyDepthFrom` line 680-689 之前，translucent 粒子在帧尾
    `renderTranslucentParticles` line 714）。两个粒子 pipeline 共用 vanilla
    `DepthStencilState.DEFAULT`（`LESS_THAN_OR_EQUAL`、depth write on），故 opaque
    pipeline 写主深度与 vanilla `OPAQUE_PARTICLE` 一致。剩余其它 target ownership
    条目继续留 P1-1 队列。
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


### 2026-07-05 迁入：RendererFrame 逐字段提取时机核查（已审完，原文自 goal.md 当前边界）

  `RendererFrame` sky-flash 相关 lightmap/clear/fog/sky/cloud 提取时机已按
  vanilla `Minecraft.tick` -> `ClientLevel.tick` -> `GameRenderer.extract`
  调整为 `advance_sky_flash_time` 后读取。
  `RendererFrame` HUD local-player / hotbar / inventory-screen 投影也已按
  vanilla keybind tick -> GUI extract 顺序验证为 input/use-item tick 后读取。
  `RendererFrame` dropped/held/item-frame/entity-block item projections 和 entity
  model instances 已验证为 entity animation / cooldown / input / use-item tick 后读取。
  `RendererFrame` block-destroy overlay 已验证为本地 destroy render tick 后读取。
  `RendererFrame` selection/entity outlines 已验证为 input/use-item/entity tick 后按
  frame camera pose 读取。
  `RendererFrame` cloud_frame 已验证为 client-time / partial tick / frame camera pose
  后读取。
  `RendererFrame` weather_render_state 已验证为 client-time / partial tick /
  frame camera pose 后读取。
  粒子 light refresh 已调整并验证为 input/use-item 后 tick、frame extract 输入绑定后采样，
  当前 RendererFrame / adjacent renderer-state 提取时机清单已审完。

### 2026-07-05：target ownership 伞形条目逐 pass 审计闭项

- [x] P1-1 target ownership 审计（2026-07-05，HEAD bb4e8d34）：render.rs 全部
  19 个 FRAME_STEPS step 的写入 target 与 vanilla 逐项对照，结论全 aligned，
  伞形条目按"无已知错位，仅剩常态规则"闭项。要点：
  - bbb 无条件创建 main/translucent/item_entity/particle/weather/cloud/
    entity_outline 七个独立 target 并跑 transparency combine——建模的是
    vanilla fabulous-on 路径（`LevelRenderer.java:480-487` 仅
    `transparencyChain != null` 时 createInternal 分离 target），全部对齐
    均为 aligned-by-mode。
  - 曾疑似错位的 selection/line 画入 itemEntity target 实为 vanilla 规定：
    `RenderTypes.java:332-350` `LINES`/`linesTranslucent`/
    `SECONDARY_BLOCK_OUTLINE` 均 `.setOutputTarget(ITEM_ENTITY_TARGET)`。
  - feature pass 相对序与 `FeatureRenderDispatcher.java:53-94`（solid →
    translucent features → translucent particles）及顶层 frame 序
    `LevelRenderer.java:493-534` 一致；copy main depth →
    translucent/itemEntity/particles 与 `:679-689` 三处 copyDepthFrom 一致。
  - 无绕过 step 编排的 draw 站点（render() 本体无 begin_render_pass 由源码
    断言测试强制；跨文件运行时 pass 仅 PIP 的 encode 函数，由对应 step 调用）。
  - 非错位备注：vanilla `addLateDebugPass`（debug gizmo → main，
    `LevelRenderer.java:762-780`）bbb 无对应 step——debug gizmo 功能整体
    未实现且无生产者，判 not-needed 直到 debug 工具面立项；
    weather_target_pass 额外 copy main depth → weather 是 bbb 深度准备细节，
    不移动任何 draw 的 target 归属。

### 2026-07-05 迁入：world border（forcefield）渲染补齐

- [x] world border forcefield 渲染：weather target 内 rain/snow 之后新增
  world border draw，逐行转写 vanilla 26.1 `WorldBorderRenderer`。world 侧
  `WorldBorderState` 补齐 `MovingBorderExtent` lerp 字段
  （`lerp_duration`/`current_size`/`previous_size`），
  `WorldStore::advance_world_border` 按 `ClientLevel.tick ->
  WorldBorder.tick()` 每 tick 递减 lerp 并在归零时坍缩为 static extent；
  `min/max_at(partial_tick)`、`distance_to_border`、`status()`（GROWING
  0x40FF80 / SHRINKING 0xFF3030 / STATIONARY 0x20A0FF）均按
  `WorldBorder.java` / `BorderStatus.java` 转写。提取侧
  `world_border_render_state_for_world` 转写 `WorldBorderRenderer.extract`
  （近边/扩界可见性判据、`alpha = clamp((1 - dist/renderDistance)^4)`、
  status tint）并携带 `renderDistance = chunks * 16`、`depthFar =
  max(renderDistance * 4, 128 * 16)`、`millis % 3000 / 3000` UV 滚动，pump
  中在 border tick 与 weather 提取之后读取（vanilla "border" profiler 段
  顺序），经新增 `RendererFrame.world_border_render_state` 单次提交。
  renderer 侧 `build_world_border_mesh` 每帧重建 vanilla
  `rebuildWorldBorderBuffer` 四面墙 quad（`(floor(min) & 1) * 0.5` U 相位、
  每格 0.5 U、`v0 = -frac(cameraY * 0.5)`、`±depthFar` 墙高）加
  `closestBorder` 距离排序可见面索引（`6 * get2DDataValue`），pipeline 按
  `RenderPipelines.WORLD_BORDER`：`BlendFunction.OVERLAY`
  （SRC_ALPHA/ONE/ONE/ZERO）、cull off、depth-write `LESS_EQUAL` + -3/-3
  depth bias、texel alpha==0 discard、ColorModulator 烘焙进逐顶点颜色；
  `textures/misc/forcefield.png` 由 native 从 pack 读字节经
  `upload_world_border_texture` 上传（repeat/nearest sampler）。测试：
  render.rs 源码顺序断言（weather target 内 rain/snow 之后、combine 之前）、
  pump 源码顺序断言（border tick 在 client time 前、提取在 weather 后）、
  mesh/UV/颜色/剔除确定性单测、lerp tick 与 extract alpha/tint 单测、
  forcefield 纹理加载单测。

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
- Placeholder / remaining renderer bounds:
  - [x] experience-orb source-verified placeholder bounds：native entity
    scene no longer labels XP orb model bounds as `todo_*`; it uses vanilla
    26.1 `EntityType.EXPERIENCE_ORB` `sized(0.5F, 0.5F)` as
    `experience_orb_entity_type_bounds` and tests the exact key plus 0.5 cubed
    placeholder bounds. The actual `ExperienceOrbRenderer` icon quad, animated
    tint, alpha, and item-target translucent submission remain a later
    presentation slice.
  - [x] simple EntityType-sized placeholder bounds batch：native entity scene
    now uses source-verified `*_entity_type_bounds` keys for dragon fireball
    (`1.0`), falling block / TNT (`0.98`, both later replaced by block-model
    attachment slices below), firework rocket (later replaced by the item-billboard
    renderer slice below), item entity (later replaced by the item renderer slice
    below), ominous item spawner (later replaced by the item-cluster renderer
    slice below), and fishing bobber (`0.25`), and keeps the prior XP orb `0.5`
    key on the same helper. Tests pin every id, key, and
    `EntityType.sized(width, height)` box. Display entities, painting, and the
    unknown future-id placeholder stay deferred because their current boxes are not
    direct vanilla `EntityType.sized` renderer boxes.
  - [x] primed TNT block attachment renderer：native entity scene now maps
    vanilla TNT to `EntityModelKind::NoRender` instead of the prior source-verified
    placeholder box, and native item-model attachments render the vanilla
    `TntRenderer` block model path. World reads `PrimedTnt.DATA_FUSE_ID` (id 8)
    and `DATA_BLOCK_STATE_ID` (id 9), defaults to `Blocks.TNT.defaultBlockState()`,
    rejects air block states, and the renderer-owned transform applies the
    `translate(0, 0.5, 0)` pose, final-10-tick fourth-power scale pulse,
    `Ry(-90) / translate(-0.5, -0.5, 0.5) / Ry(90)`, entity light, partial-tick
    fuse projection, and white-strobe overlay. Tests cover default/custom block
    state, partial tick, outline-only hidden glowing attachments, and the
    renderer transform.
  - [x] falling block block attachment renderer：native entity scene now maps
    vanilla falling-block to `EntityModelKind::NoRender` instead of the prior
    source-verified placeholder box. World resolves the add-entity `data`
    block-state id, rejects air/unknown states, and native item-model
    attachments render the selected block model with entity light, vanilla
    `translate(-0.5, 0, -0.5)`, and the
    `blockState != level.getBlockState(entity.blockPosition())` visibility gate.
    Tests cover spawn-data projection, invisible falling-block body submission,
    missing-chunk rendering, and loaded same-block skip.
  - [x] firework rocket item billboard renderer：native entity scene now maps
    vanilla firework rockets to `EntityModelKind::NoRender` instead of the prior
    source-verified placeholder bounds. `WorldStore` reads
    `FireworkRocketEntity.DATA_ID_FIREWORKS_ITEM` (id 8),
    `DATA_ATTACHED_TO_TARGET` (id 9, optional unsigned int), and
    `DATA_SHOT_AT_ANGLE` (id 10), skips attached elytra-boost rockets to mirror
    `shouldRender`, and samples entity light for the item layer. Native submits
    the rocket stack through the existing item atlas billboard path, with the
    vanilla post-camera `Z+180 / Y+180 / X+90` pose represented by
    `ItemEntityBillboardOrientation::FireworkShotAtAngle`. Tests cover metadata
    projection, attached-target suppression, scene `NoRender`, native billboard
    orientation, and renderer vertex axes. Firework explosion / Starter child
    particles remain tracked with particle presentation work.
  - [x] dropped item entity renderer routing：native entity scene now maps
    vanilla dropped item entities to `EntityModelKind::NoRender` instead of the
    prior source-verified placeholder bounds. Vanilla `ItemEntityRenderer`
    submits item-stack clusters, and this repo already routes block/3D dropped
    items through `dropped_item_models` and flat dropped items through
    `item_entity_billboards_from_world`, with `handled_entity_ids` preventing
    double rendering. The item stack metadata projection remains in
    `WorldStore::item_entity_stacks`.
  - [x] ominous item spawner item-cluster renderer：native entity scene now maps
    vanilla ominous item spawners to `EntityModelKind::NoRender` instead of the
    prior source-verified placeholder bounds. `WorldStore` reads
    `OminousItemSpawner.DATA_ITEM` (item-stack metadata id 8) and projects
    `ageInTicks` from entity client animation age plus partial tick. Native
    bakes the carried stack through the shared item cluster helper with the
    vanilla `OminousItemSpawnerRenderer` transform: scale in over the first 50
    ticks, rotate around Y at 40 degrees per tick, and submit at full-bright
    light. Tests cover metadata/age projection, scene `NoRender`, transform
    math, and flat item mesh emission. Ominous spawning particles and sounds
    remain tracked with particle/audio presentation work.
  - [x] P1-2 dying ender dragon GPU `DISSOLVE` mask sampling：`entityCutoutDissolve`
    的垂死龙 body 现在落入专用 dissolve mesh/pipeline
    （`RenderPipelines.ENTITY_CUTOUT_DISSOLVE` = `ENTITY_SNIPPET` + `ALPHA_CUTOUT 0.1`
    + `PER_FACE_LIGHTING` + `DISSOLVE` + `withCull(false)`，无 color-target blend，故
    surface state 与普通 entity cutout 相同、depth write + `LESS_EQUAL`、cull off）。
    每个 dissolve 顶点携带第二组 `mask_uv`，在 mesh-build 时按基础 UV 的归一化模型
    坐标重投影进 `dragon_exploding.png` 的图集 sub-rect（复用同一 entity 图集/sampler，
    无需新 bind group）；WGSL 逐字复刻 `entity.fsh:33-63`：先做基础贴图 `ALPHA_CUTOUT 0.1`
    discard，再 `if (faceVertexColor.a < texture(DissolveMaskSampler, texCoord0).a) discard;`，
    幸存像素把顶点色 alpha（`1 - deathTime/200`）强制为 `1.0`。确定性 headless GPU
    readback（纯红基础贴图 + 两档 mask alpha `0.2`/`0.8`、顶点 `tint.a=0.5`）固定：低于
    mask 的半屏被侵蚀成背景、高于的半屏输出不透明红，`tint.a=1.0`（存活龙）全保留；
    另有 CPU 测试固定 `mask_uv = mask.min + (base_uv - base.min)/base.size * mask.size`
    的逐顶点映射。至此垂死龙死亡视觉 parity 的最后一块 GPU-side DISSOLVE 缺口闭合。
- ItemPickupParticle 泛化 `EntityRenderState` submit 消费面：
  - [x] arrow/trident pickup carried 实体模型（2026-07-05，P1-2 最后一项）：
    world `take_item_entity_pickup_particle_state` 对 arrow/spectral/tipped/trident
    追加 `TakeItemEntityPickupProjectileModel`（tipped 按 vanilla
    `TippableArrowRenderer.isTipped = getColor() > 0` 读 `ID_EFFECT_COLOR` id 11，
    trident foil 读 `ThrownTrident.ID_FOIL` id 12）并携带 extract 时的
    `yRot`/`xRot`；native 投影为 renderer 侧
    `option_item_pickup_projectile_model`（renderer 不依赖 world/protocol，枚举
    定义在 bbb-renderer）；renderer 新提取器复用
    `item_pickup_position_at_partial_tick` 的 vanilla 二次插值位置，bake 克隆
    elder-guardian 模式（`ArrowModel`/`TridentModel` + 强制 `EntityTranslucent`
    + 冻结 pickup light），root transform 逐字复刻 `Ry(yRot-90) * Rz(xRot)`
    （arrow 尾随 0.9 bake scale；trident `Rz(xRot+90)`，foil 追加 order(1)
    glint pass），draw 在 `ITEM_PICKUP` group 内接在 orb billboard 之后、
    elder-guardian 之前，走同一 entity translucent-cull pipeline。测试三层
    GPU-free：world 四态投影（tipped 颜色断言 + item/orb 不回归）、native 命令
    携带、renderer bake 非空 translucent mesh + 插值位置 transform + render.rs
    source-assertion 锁 draw 顺序。至此 vanilla
    `ItemPickupParticleGroup.State.submit` 的三类被捡实体（item stack /
    experience orb / arrow+trident）全部覆盖，"通用 EntityRenderState submit
    管线"开放项随消费面闭合而关闭。

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
    root/en plus selected English regional week-data ICU `SimpleDateFormat`
    subset (`y`/`u` year, supported-English `Y` week-year, `G` era, `Q`/`q`
    quarter, root/en `M`/`L` month widths 1..=5, `d`, `D` day-of-year,
    `g` Julian day, supported-English `w`/`W` week-of-year / week-of-month, `F`
    day-of-week-in-month, supported-English `E`/`e`/`c` weekdays, 24/12-hour
    `H`/`k`/`K`/`h`, `m`/`s`/`S`, `A` milliseconds-in-day, root/en `a`
    AM/PM widths 1..=5, `Z`/`X`/`x` offset fields through width 5,
    localized-GMT `O` offsets, short `z` zone abbreviations, `VV` zone IDs,
    `VVV` exemplar cities, and quoted literals),
    using fixed `GMT`/UTC offset and IANA `time_zone` IDs when present or the
    system local zone otherwise. `G`/`u`/`D` follow Java
    `DateTimeFormatter`/`IsoChronology`
    (`u` = proleptic year, identical to `y` for every CE epoch-millis date;
    era text gated on root/en locale; day-of-year zero-padded by pattern count).
    `Q`/`q` mirror ICU quarter widths 1..=5 for root/en (`3`, `03`, `Q3`,
    `3rd quarter`, `3`), treating format and stand-alone quarter identically
    for the supported locale subset.
    `M`/`L` mirror ICU root/en month widths 1..=5: numeric, zero-padded,
    abbreviated, wide, and narrow (`A` for August), treating format and
    stand-alone month names identically for the supported locale subset.
    `g` mirrors ICU `Calendar.JULIAN_DAY` numeric output for the local
    calendar date, with pattern width controlling minimum zero padding.
    `A` mirrors ICU milliseconds-in-day with pattern width as the minimum
    numeric padding.
    `a` mirrors ICU root/en AM/PM marker widths 1..=5: widths 1..=4 use
    `AM`/`PM`, while width 5 uses narrow `a`/`p`.
    `O` mirrors ICU localized GMT offset widths for root/en: `O`..`OOO` use
    the short form (`GMT+2:30`) and `OOOO` uses the zero-padded long form
    (`GMT+02:30`).
    `Z`/`X`/`x` now cover ICU widths 1..=5 for the no-seconds offsets
    produced by vanilla-supported fixed/GMT/UTC and modern IANA zones:
    `ZZZZ` uses the long localized-GMT form, `ZZZZZ` and `XXXXX` use
    extended ISO8601 with `Z` for UTC in the uppercase fields, and
    `XXXX`/`xxxx` use the basic form.
    `w`/`W` mirror ICU week fields for selected English locale groups:
    root/en/en_AU/en_NZ use Monday/minimal-days=1,
    en_US/en_CA/en_IN/en_ZA use Sunday/minimal-days=1, and en_GB/en_IE use
    Monday/minimal-days=4. Pattern width controls numeric padding; `w` keeps
    late December dates in the current calendar year's final week until Jan 1,
    while en_GB/en_IE Jan 1 dates before the first full week can stay in the
    previous week-year's final week.
    `Y` mirrors the ICU week-year for those same week-data groups, with normal
    year width formatting (`YY` is two-digit, `YYYYY` is zero-padded).
    `F` mirrors ICU day-of-week-in-month as `(day - 1) / 7 + 1`, with pattern
    width controlling numeric padding.
    `e`/`c` use the same locale week data for local weekday numbers
    (`Monday=1` for Monday-first groups, `Sunday=1` for Sunday-first groups)
    and ICU weekday text widths 3..=6; `E` now follows those selected English
    text widths too.
    `z` zone names use the active TZDB abbreviation for explicit IANA zones,
    RFC-822-style fixed-offset short names, and fixed/UTC long names; `VV`
    emits the explicit zone id, while `VVV` emits the IANA zone's exemplar city.
    Tests pin GMT Christmas selection plus cross-midnight `UTC+02:30`,
    `Asia/Tokyo`, UTC date-time / weekday / AM-PM / offset, and a
    `uuuu-DDD-G` proleptic-year / day-of-year / era branch plus a
    `Q`/`q` quarter branch, `g` Julian day branch, root/en `M`/`L` narrow
    month branch, `A` milliseconds-in-day branch, root/en `a` narrow AM/PM
    branch, `O`
    localized-GMT branch, `F`
    day-of-week-in-month branch, selected-English `Y` week-year branch, and
    selected-English `w`/`W` week branch including the year-end `w` boundary,
    Sunday-first regional branch, and Monday/minimal-days=4 Jan 1
    previous-week-year / previous-month `W` branch from vanilla
    `LocalTime.get`, plus selected-English `e`/`c` local weekday branches, a
    short `z` / `VV` / `VVV` IANA-zone branch, fixed-offset `zzzz` branch, and UTC /
    `UTC+02:30` width-4/5 offset branches. IANA long `z`, generic `v`,
    one- and four-letter `V` widths,
    locale-specific week data beyond the selected English regional groups, and
    non-English locales remain
    follow-up.
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
  - [x] generated item material translucency metadata：`GeneratedItemLayer`
    now carries vanilla material translucency from `Material.force_translucent`
    and atlas sprite translucent-pixel metadata into generated flat item slab
    quads. Dropped, held, and item-frame generated item consumers reuse the
    existing item-model solid / `item_translucent` and `glintTranslucent` mesh
    splits, with tests pinning forced and sprite-derived translucent layers plus
    generated-side quads.
  - [x] ordinary first-person hand item pass：local vanilla 26.1
    `GameRenderer` clears the main depth target before `ItemInHandRenderer`
    renders first-person hands, and the ordinary item path applies
    `applyItemArmTransform` (`±0.56, -0.52, -0.72`) before resolving
    `FIRST_PERSON_RIGHT_HAND` / `FIRST_PERSON_LEFT_HAND`. Native now projects
    non-using, non-special local main/offhand stacks through those display
    contexts into dedicated first-person block/generated item-model buckets,
    and the renderer draws them after world transparency composite and before
    HUD overlays with a depth-clear hand pass. Tests cover local-hand stack
    baking, special/use-path rejection, hand transform constants, and renderer
    pass ordering. Use/swing animation and map / bow / crossbow / spyglass /
    shield special paths remain first-person follow-ups.
  - [x] ordinary first-person WHACK swing animation：local vanilla 26.1
    `ItemInHandRenderer.renderHandsWithItems` samples
    `LocalPlayer.getAttackAnim(partialTick)` and `swingingArm`, then the
    ordinary non-using branch applies `applyItemArmTransform` followed by
    `swingArm` only when `ItemStack.getSwingAnimation().type() == WHACK`.
    Native now exposes the local-player attack swing from world animation state,
    threads `entity_partial_tick` into first-person item extraction, applies the
    vanilla `swingArm` translation and `applyItemArmAttackTransform` rotations
    for ordinary WHACK stacks, respects explicit `NONE`, and keeps `STAB` on
    the deferred special path. Tests cover local-player swing sampling, vanilla
    transform math, mesh movement under a main-hand swing, and STAB rejection.
  - [x] first-person STAB attack item transform：local vanilla 26.1
    `ItemInHandRenderer` still applies `applyItemArmTransform` in the non-using
    branch, but `ItemStack.getSwingAnimation().type() == STAB` then calls
    `SpearAnimations.firstPersonAttack` instead of the WHACK swing. Native now
    classifies stack patch STAB and default 26.1 spear resource ids as supported
    first-person attack items, applies the vanilla start / middle / ending
    easing translation and X rotation, and keeps explicit removed
    `swing_animation` on the default WHACK fallback. Tests cover the STAB matrix
    math, stack-patch STAB mesh movement, default spear mesh movement, and
    explicit `NONE` / removed-component swing classification.
  - [x] first-person BLOCK/shield use pose：local vanilla 26.1
    `ItemInHandRenderer.renderArmWithItem` enters the using-item branch for the
    used hand, applies `applyItemArmTransform` when the use animation has no
    custom arm transform, and for `ItemUseAnimation.BLOCK` only applies the
    fixed translate / X/Y/Z rotations when the item is not a `ShieldItem`.
    Native now allows first-person rendering while the local player uses a
    BLOCK item, classifies shield defaults and patch-granted `blocks_attacks`
    with `CONSUMABLE` precedence, keeps shields on the base arm transform, and
    applies the vanilla non-shield BLOCK transform. Tests cover shield rendering
    while using, non-shield mesh movement, consumable precedence, and transform
    matrix order.
  - [x] first-person consumable EAT/DRINK use pose：local vanilla 26.1
    `Consumable.STREAM_CODEC` carries `consumeSeconds` followed by
    `ItemUseAnimation.STREAM_CODEC`, and `ItemInHandRenderer.renderArmWithItem`
    handles `EAT` / `DRINK` by applying `applyEatTransform(frameInterp, arm,
    stack, player)` before `applyItemArmTransform`. Protocol summaries now
    preserve the consumable animation id, and native supports patch-carried
    consumables with EAT/DRINK first-person use pose using local
    `using_item_ticks`, `partial_ticks`, and vanilla `consumeTicks()` seconds
    truncation. Tests cover consumable animation decoding, EAT/DRINK mesh
    movement, shared EAT/DRINK output, and the exact matrix order/formula.
  - [x] default/prototype consumable EAT/DRINK first-person pose：local vanilla
    26.1 `Items.java` declares default consumables through `.food(Foods.X)`,
    `.food(Foods.X, Consumables.X)`, and
    `.component(DataComponents.CONSUMABLE, Consumables.X)`, with
    `Consumables.DEFAULT_FOOD` / `DEFAULT_DRINK` and the named vanilla overrides
    defining consume seconds plus `ItemUseAnimation`. The pack item-registry
    catalog now extracts those default consumables, item runtime exposes them by
    protocol item id, and native first-person item rendering falls back to the
    default prototype when the stack patch neither overrides nor removes
    `CONSUMABLE`. Tests cover default food / dried-kelp / drink parsing,
    runtime default lookup, and mesh movement for a default EAT item without a
    stack consumable patch.
  - [x] first-person goat horn TOOT_HORN use pose：local vanilla 26.1
    `InstrumentItem.getUseAnimation` returns `ItemUseAnimation.TOOT_HORN`;
    `ItemInHandRenderer.renderArmWithItem` applies `applyItemArmTransform` for
    non-custom use animations and has no `TOOT_HORN` switch case, so
    first-person goat horn use keeps the base arm transform. Native now allows
    `minecraft:goat_horn` through first-person item rendering and classifies
    using goat horns as base-arm use animation. Tests cover non-skipped rendering
    and exact idle/use mesh equality with `swing_animation = NONE`.
  - [x] first-person brush BRUSH use pose：local vanilla 26.1
    `BrushItem.getUseAnimation` returns `ItemUseAnimation.BRUSH`,
    `BrushItem.getUseDuration` returns `200`, and `ItemInHandRenderer` applies
    `applyItemArmTransform` before `applyBrushTransform(frameInterp, arm,
    player)`, using remaining ticks modulo the 10-tick brush animation to choose
    the swipe angle. Native now allows `minecraft:brush` through first-person
    rendering, classifies brush use before generic stack `CONSUMABLE` data, and
    applies the vanilla right/left hand brush transform. Tests cover non-skipped
    idle/rendering, using mesh movement, override precedence, and exact matrix
    order/formula.
  - [x] first-person bundle BUNDLE use pose：local vanilla 26.1
    `BundleItem.getUseAnimation` returns `ItemUseAnimation.BUNDLE`, and
    `ItemInHandRenderer.renderArmWithItem` applies `applyItemArmTransform` then
    calls the same `swingArm(attack, poseStack, invert, arm)` helper used by
    ordinary WHACK first-person item attacks. Native now allows
    `minecraft:bundle` through first-person item rendering, classifies bundle
    use before generic stack `CONSUMABLE` data, and reuses the WHACK swing
    transform while the bundle is being used. Tests cover non-skipped using
    rendering, attack-swing mesh movement, and override precedence.
  - [x] first-person trident TRIDENT use pose：local vanilla 26.1
    `TridentItem.getUseAnimation` returns `ItemUseAnimation.TRIDENT` and
    `TridentItem.getUseDuration` returns `72000`; `ItemInHandRenderer` applies
    `applyItemArmTransform`, the fixed throw-charge translate / X-Y-Z rotations,
    charge shake, Z scale, and `Axis.YN` 45 degree rotation using
    `timeHeld = useDuration - (remainingTicks - frameInterp + 1)`. Native now
    allows `minecraft:trident` through first-person item rendering, classifies
    trident use before generic stack `CONSUMABLE` data, and applies the vanilla
    throw-charge transform. Tests cover non-skipped idle/using rendering, mesh
    movement, override precedence, and exact matrix order/formula.
  - [x] first-person bow BOW use pose：local vanilla 26.1
    `BowItem.getUseAnimation` returns `ItemUseAnimation.BOW` and
    `BowItem.getUseDuration` returns `72000`; `ItemInHandRenderer` applies
    `applyItemArmTransform`, the fixed draw translate / X-Y-Z rotations, bow
    power curve, charge shake, Z scale, and `Axis.YN` 45 degree rotation using
    `timeHeld = useDuration - (remainingTicks - frameInterp + 1)`. Vanilla
    `evaluateWhichHandsToRender` renders only the used hand while drawing a bow.
    Native now allows `minecraft:bow` through first-person item rendering,
    classifies bow use before generic stack `CONSUMABLE` data, applies the
    vanilla draw transform, and hides the other hand while using bow/crossbow
    items. Tests cover non-skipped idle/using rendering, used-hand selection,
    mesh movement, override precedence, and exact matrix order/formula.
  - [x] first-person crossbow CROSSBOW use/hold pose：local vanilla 26.1
    `CrossbowItem.getUseAnimation` returns `ItemUseAnimation.CROSSBOW`,
    `CrossbowItem.getUseDuration` returns `72000`, and base
    `CrossbowItem.getChargeDuration` is 25 ticks. `ItemInHandRenderer` has a
    dedicated crossbow branch: uncharged use applies the draw translate /
    X-Y-Z rotations, charge shake, Z scale, and `Axis.YN` 45 degree rotation;
    charged idle main-hand crossbows add the `-0.641864` X hold offset and
    10 degree Y rotation when attack is idle. Native now allows
    `minecraft:crossbow` through first-person item rendering, classifies
    crossbow use before generic stack `CONSUMABLE` data, uses
    `charged_projectiles_items` for charged detection, reuses the bow/crossbow
    used-hand selection, and applies the vanilla uncharged draw / charged idle
    transforms. Tests cover non-skipped idle/using rendering, used-hand
    selection, charged hold movement, override precedence, and exact matrix
    order/formula. Quick Charge-adjusted duration remains a narrower
    enchantment-effect refinement.
  - [x] first-person spyglass idle/scoping visibility：local vanilla 26.1
    `ItemInHandRenderer.renderArmWithItem` is guarded by `!player.isScoping()`,
    so an idle spyglass still renders as a regular first-person hand item, while
    using/scoping with a spyglass hides both hands/items. Native now allows
    `minecraft:spyglass` through first-person item rendering when idle and keeps
    the existing scoping early-return for the local player. Tests cover idle
    spyglass rendering and scoping hiding an otherwise visible offhand item.
  - [x] first-person filled-map base surface：local vanilla 26.1
    `ItemInHandRenderer.renderArmWithItem` checks `DataComponents.MAP_ID` before
    ordinary item rendering, uses `renderTwoHandedMap` only for a main-hand map
    with an empty offhand, and otherwise uses `renderOneHandedMap`; both call
    `renderMap` with the Y/Z flips, `0.38` scale, centering, and `1/128` map
    pixel scale. Native now lets `map_id` stacks through first-person
    extraction, skips the ordinary item-model fallback for map stacks, uploads
    decoded `MapItemData` via the shared dynamic `minecraft:map/<id>` texture
    helper, and submits the map base quad to the depth-cleared hand pass using
    the vanilla one-handed / two-handed transforms. Tests cover decoded map
    texture pixels, surface metadata, missing-map-data non-fallback behavior,
    and exact map tilt / one-handed / two-handed matrix formulas. Background /
    checkerboard, map decorations/text, and first-person player arms remain
    pixel-level follow-ups.
  - [x] first-person filled-map decorations/text：local vanilla 26.1
    `MapRenderer.render(mapRenderState, poseStack, submitNodeCollector, false,
    lightCoords)` renders all known map decorations because `showOnlyFrame` is
    false, increments the decoration count for each rendered decoration, and
    submits order-1 labels after order-0 base/decor sprite geometry. Renderer
    map decoration/text bakers now expose first-person variants that do not
    apply the item-frame `renderOnFrame` filter; native first-person map
    extraction emits decoration sprites and ASCII label surfaces alongside the
    decoded map base surface; and the renderer draws those decoration/text
    surfaces in the depth-cleared hand pass using the shared map-decoration and
    font atlases. Tests cover the non-frame player marker (`type_id = 0`),
    first-person label text, submit order/sequence, and the native hand-path
    integration. Background/checkerboard and first-person player arms remain
    pixel-level follow-ups.
  - [x] first-person filled-map background/checkerboard：local vanilla 26.1
    `ItemInHandRenderer.renderMap` submits `textures/map/map_background.png`
    when `MapItemSavedData` is absent and
    `textures/map/map_background_checkerboard.png` when decoded map data is
    present, using the same first-person map transform as the dynamic map
    content and the `(-7,135)..(135,-7)` border quad. `bbb-render-types` now
    carries the two background texture payloads, item runtime loads them from
    the resource stack, native first-person extraction emits plain backgrounds
    even for missing map data and checkerboards for decoded maps, and the
    renderer draws the background atlas before the dynamic map/decor/text
    surfaces in the depth-cleared hand pass. Tests cover the vanilla quad
    coordinates, atlas filtering/remapping, missing-data plain background, and
    decoded-map checkerboard selection. First-person player arms and
    screenshot-level visual parity remain follow-ups.
  - [x] custom consumable first-person non-EAT/DRINK use animations：local
    vanilla 26.1 `Item.getUseAnimation` reads `DataComponents.CONSUMABLE`
    before `BLOCKS_ATTACKS` / `KINETIC_WEAPON`, and `ItemInHandRenderer` has
    switch cases for generic BOW, TRIDENT, BRUSH, and BUNDLE animations while
    generic NONE/CROSSBOW/SPYGLASS/TOOT_HORN fall through to the base arm
    transform. Native now maps those custom consumable animations to the same
    first-person transforms already used by the vanilla item special cases and
    keeps generic BOW from using the `Items.BOW` / `Items.CROSSBOW`
    used-hand-only selection path. Tests cover all mapped animations, generic
    no-switch base-arm cases, and generic SPEAR without readable kinetic
    weapon data.
  - [x] first-person SPEAR / kinetic use animation：local vanilla 26.1
    `Item.getUseAnimation` falls through to `KINETIC_WEAPON` after consumables,
    and `ItemInHandRenderer.renderArmWithItem` skips `applyItemArmTransform`
    for `SPEAR`, applies only the base hand translation, then calls
    `SpearAnimations.firstPersonUse` with `timeHeld` and
    `getTicksSinceLastKineticHitFeedback(partialTick)`. Native now resolves the
    default tool-material spear `KineticWeapon` timings, exposes the shared
    renderer `SpearKineticWeapon::use_params` timing data to native, applies
    the vanilla first-person translate / rotateAround / hit-feedback transform,
    and samples the local player kinetic feedback state from `WorldStore`.
    Tests cover default spear animation selection, consumable SPEAR duration,
    removal of the prototype kinetic component, the direct transform matrix,
    rendered first-person use movement, and local kinetic hit feedback.
  - [x] first-person local-player arms：local vanilla 26.1
    `ItemInHandRenderer.renderArmWithItem` renders `renderPlayerArm` for an
    empty main hand when the player is visible, renders two `renderMapHand`
    submissions for a main-hand filled map with an empty offhand, and renders a
    one-handed map arm for other filled-map hands. `AvatarRenderer.renderRightHand`
    / `renderLeftHand` reset the selected arm, apply the fixed arm z-rotation,
    toggle the sleeve, and submit the arm with `RenderTypes.entityTranslucent`
    plus `OverlayTexture.NO_OVERLAY` and the frame light coords. Native now
    extracts first-person player arm submissions alongside first-person item
    models, preserving the local player skin, sleeve visibility, packed light,
    empty-hand and filled-map arm transforms, used-hand bow/crossbow selection,
    invisibility/scoping/spectator/camera gates, and dynamic skin fallback
    metadata. Renderer now bakes selected player arm parts into static or ready
    dynamic-player-skin translucent buckets and draws them in the depth-cleared
    hand pass before map/item surfaces. Tests cover vanilla arm/map-hand matrix
    constants, empty-hand and filled-map extraction, sleeve/light/skin
    metadata, static `entityTranslucent` submission, and ready dynamic skin
    atlas routing.
  - [x] first-person generated item use-tick context：local vanilla 26.1
    `UseDuration.get` / `UseCycle.get` read elapsed or remaining use ticks only
    when the item owner is a living entity and `entity.getUseItem() == itemStack`;
    otherwise the property value is `0.0`. Native first-person item extraction
    already knows the local active hand and local `using_item_ticks`, so it now
    passes those values into the shared generated-item bake path instead of
    marking first-person stacks inactive. This lets first-person generated item
    models resolve `minecraft:use_duration` and `minecraft:use_cycle`
    range-dispatch branches the same way owner-backed third-person held items
    already do. Tests use a constant-transform custom consumable with
    `UseAnimation.NONE` to prove idle, start, mid, and full use-duration
    branches produce distinct first-person meshes from local use ticks.
  - [x] HUD/inventory durability bar registry-default `max_damage` fallback：
    vanilla `ItemStack.getMaxDamage()` is `getOrDefault(MAX_DAMAGE, 0)`, and
    servers only patch `damage` for an ordinary damaged stack since
    `max_damage` is a registry default component. Native's
    `hud_item_durability_bar_for_stack` now falls back to the world's
    default-item-max-damage table (`WorldStore::item_max_damage_for_protocol_id`,
    populated at startup from `NativeItemRuntime::item_max_damage_by_protocol_id`)
    when the stack patch omits `max_damage`, matching the existing
    `wolf_armor_crackiness` fallback pattern. An explicit patch `max_damage`
    still takes priority, and `minecraft:unbreakable` still suppresses the bar
    regardless of the default table. Tests cover the default-table fallback,
    patch-value priority over the default table, an empty default table, and
    unbreakable-with-default-table suppression.


### 2026-07-05 迁入：item glint 与 first-person presentation 完成史

- item enchantment glint follow-ups：solid item-model `RenderTypes.glint()` 已覆盖 dropped /
  held / item-frame / HUD 3D block items，含 `ItemStackSummary::has_foil()` 跨 crate 投影、
  item glint mesh bucket、独立 `textures/misc/enchanted_glint_item.png` 上传、`GLINT_TEXTURING`
  scale `8.0` shader、GLINT blend、depth-equal、no-lightmap draw；world/itemEntity-target
  item-model translucent quads now also have `RenderTypes.glintTranslucent()` mesh/draw buckets；
  clock / compass SPECIAL foil decal UVs are covered for current dropped / held / item-frame
  item-model consumers；flat HUD/inventory item sprites now draw an alpha-masked standard
  glint overlay after sprite layers and before durability/cooldown/count decorations, with
  clock / `ItemTags.COMPASSES` GUI SPECIAL foil using the vanilla sheeted-decal UV scale；GUI
  3D block-item icons now split translucent base quads and matching `glintTranslucent`
  inside the GUI item pass；generated flat item layers now carry vanilla
  material translucency (`force_translucent` or sprite translucent pixels) into
  extruded item-model quads for dropped / held / item-frame consumers；普通
  first-person local-player hand stacks now bake through `FIRST_PERSON_*_HAND`
  display contexts into an after-world / before-HUD depth-cleared hand pass,
  using vanilla `applyItemArmTransform` constants and ordinary WHACK `swingArm`
  plus STAB `SpearAnimations.firstPersonAttack` from the local player's
  `attackAnim` / `swingingArm`, first-person BLOCK/shield use pose, and
  patch-carried/default consumable EAT/DRINK `applyEatTransform` use pose, and
  goat horn `TOOT_HORN` base-arm use pose, and brush `BRUSH`
  `applyBrushTransform` use pose, bundle `BUNDLE` use `swingArm`, and trident
  `TRIDENT` throw-charge use pose, and bow `BOW` draw use pose / used-hand
  selection, crossbow uncharged draw / charged idle poses, and spyglass idle /
  scoping visibility, and filled-map decoded base surfaces via the vanilla
  one-handed / two-handed map branches plus first-person `MapRenderer`
  decorations/text plus map background/checkerboard quads, and custom
  consumable BOW/TRIDENT/BRUSH/BUNDLE plus generic no-switch
  NONE/CROSSBOW/SPYGLASS/TOOT_HORN first-person use animation, and SPEAR /
  kinetic first-person use animation with local hit feedback, and first-person
  local-player empty-hand / filled-map player arms using vanilla
  `renderPlayerArm` / `renderMapHand` transforms, player skin sleeves,
  `entityTranslucent`, local light, and dynamic skin atlas routing, and
  first-person generated item consumers now pass the local active-use tick
  context into `minecraft:use_duration` / `minecraft:use_cycle` item-model
  range-dispatch properties, and attack-arm `SWING_ANIMATION(NONE)` stack
  patches now project through native/render-state so PlayerModel and ordinary
  piglin/brute inherited `HumanoidModel.setupAttackAnimation` keep the vanilla
  prologue while skipping WHACK/STAB. 剩余是截图级 viewmodel 视觉校验。
- [x] first-person viewmodel 截图级（headless GPU readback）视觉校验：以
  sentinel-pixel 模式（不做 golden PNG / 全帧 hash，规避驱动/浮点抖动）在真
  wgpu device（本机 lavapipe / llvmpipe Vulkan，测试内确认 adapter 真拿到）上
  真跑三态断言。(1) 手持 item 经 `first_person_item_pass` 的真实
  `item_model_pipeline` + item atlas 渲染，锚点像素（由 item mesh 中心经
  `CameraUniform::from_pose` 的 view_proj 投影 + wgpu 视口变换推导，非盲猜）命中
  item 色、角落保持背景色。(2) 手臂经真实
  `first_person_player_arm_textured_meshes` 产出的 `entityTranslucent` 网格 +
  `entity_model_translucent_pipeline` + entity texture atlas + lightmap 渲染，
  锚点像素（由手臂 mesh centroid 投影推导）命中手臂纹理色、角落保持背景色。
  (3) vanilla WHACK swing 帧 vs 静止帧的同一静止锚点像素 `assert_ne`，把既有
  几何级 `assert_ne!(mesh)`（`first_person_item_models_apply_local_player_whack_swing`）
  抬升为像素级。测试：`bbb-renderer` `item_models.rs`
  `first_person_held_item_renders_visible_pixels_and_swing_moves_them` 与
  `entity_models/tests/player.rs`
  `first_person_player_arm_renders_visible_pixels`（无 GPU adapter 时 skip，
  不在无 adapter 机器上误 fail）。
- [x] HUD/inventory tooltip 官方 background/frame nine-slice sprite：把
  `push_hud_inventory_tooltip` 的单张纯色矩形（`[0.0625,0,0.0625,0.94]`）换成
  vanilla `TooltipRenderUtil.extractTooltipBackground` 的两层 nine-slice blit——
  先 `tooltip/background`、后 `tooltip/frame`，两者都画在同一 `x-3-9,y-3-9` /
  `w+24,h+24` padded rect 上（bbb 既有的
  `hud_inventory_tooltip_background_hud_rect` 已与 vanilla 常量一致）。gui sprite
  管线加 nine-slice 支持：native 侧从 gui atlas 读 `tooltip/background`（mcmeta
  `nine_slice` width/height 100、border 9、`stretch_inner` 缺省 false=tile）与
  `tooltip/frame`（border 10、`stretch_inner` true=stretch）的
  `SpriteImage.gui_metadata.scaling`，经新公开 `HudNineSliceScaling` +
  `upload_hud_tooltip_background` / `_frame` 连同 RGBA 上传；renderer 侧
  `hud/layout.rs` 新增 `nine_slice_segments`，按
  `GuiGraphicsExtractor.blitNineSlicedSprite` 语义把目标矩形拆成 9 区（角块 1:1、
  四边+中心按 `stretch_inner` 拉伸或 `blitTiledSprite` 平铺、末尾行列 tile 裁剪、
  border 按 `min(border, target/2)` clamp、退化时塌成 4 角块），bbb 因每张 sprite
  单独成纹理故 UV 直接是 sprite 分数（`getU(x/spriteWidth)`，`u0=0`）。sprite 缺失
  时回退旧纯色矩形（账本记录）。测试：`bbb-pack` `sprites.rs`
  `sprite_source_reads_gui_nine_slice_stretch_inner_from_mcmeta`（mcmeta 定字节 →
  九宫格参数含 stretch_inner）、`bbb-renderer` `hud/layout.rs`
  `nine_slice_segments_stretch_inner_splits_into_nine_vanilla_regions` /
  `nine_slice_segments_clamp_borders_and_drop_center_when_target_smaller_than_borders` /
  `nine_slice_segments_tile_inner_repeats_and_clips_last_tile` /
  `hud_inventory_tooltip_sprite_segments_layer_background_then_frame_in_vanilla_order`
  （九宫格顶点/UV 确定性、退化 clamp、tile 平铺+裁剪、两层 source-order）。剩余
  rich tooltip 项：non-ASCII font provider 与 bidi 文本整形。
- [x] vanilla font bitmap provider 通用化（P1-3 font 子 slice 1，2026-07-05）：
  从 `font/default.json` 出发解析 `reference`（深度优先原位展开、visited 去重，
  `filter` 在固定 FontOptions=全关下恒通过故忽略）与 `bitmap` provider
  （`BitmapProvider.Definition` codec：`file`/`height` 默认 8/`ascent`/`chars`，
  校验 ascent<=height 与等长行），按 `include/default.json` 顺序
  `nonlatin_european`(ascent 7) → `accented`(height 12, ascent 10) →
  `ascii`(ascent 7) 三页 PNG 纵向堆叠成单张多页 glyph atlas，构建码点键控
  `HudFontGlyphMap`（页间 first-provider-wins 对齐 `FontSet.computeGlyphInfo`、
  页内重复码点 last-wins 对齐 `CodepointMap.put`），advance 用 vanilla
  `(int)(0.5+actualWidth*pixelScale)+1`、`pixelScale=height/glyphH`、
  actualWidth 右向左扫列；像素可见性判定修正为 vanilla
  `NativeImage.getLuminanceOrAlpha`（RGBA 只看 alpha 字节）——官方三张字体
  PNG 是白色调色板+alpha0 透明，旧的 "alpha 或 RGB 非零" 判定在真实资产上
  会把所有 advance 撑满 cell+1（连带修复 digit/ascii 旧路径同源的生产
  bug，真实 `e` advance 由 9 修正为 6，已用真实资产 smoke 验证全链路）；
  glyph 结构（`bbb-render-types`）加 `ascent`，绘制按
  `GlyphBitmap.getTop()`=`7-ascent` 基线对齐（accented 页比 ascii 页高 3px）。
  替换硬编码单页 ascii.png `[glyph;95]` 数组：HUD inventory label/tooltip、
  map decoration 文字全部改码点查找（accented/非拉丁欧洲码点不再退化 `?`；
  CJK 因 unihex/unifont defer——资产树无 unifont zip——仍退化 `?`）；item count
  数字子集保留 ascii.png 数字行路径。space provider 沿用硬编码 space=4、样式
  与 bidi 为后续子项。实现：`bbb-item-model/src/font.rs` + `font/providers.rs`
  （原 ascii_font.rs 演进）、`hud_glyphs.rs`、renderer `hud.rs`/`hud/layout.rs`/
  `item_models/map.rs`、native `hud_assets.rs`。测试：providers 解析
  （reference 链展开顺序/height 默认/ascent 校验/ragged 行/surrogate 码点）、
  height-12 页 advance 公式、fallback 先页胜出、多页堆叠 UV/NUL 槽跳过、
  é/ü/ñ/ж/λ 宽度与字形存在 + `钻` 无字形断言、é(-3px) vs e 基线偏移
  （render-types + layout rect 双处）、hud/map 宽度回退语义保留。
- [x] vanilla font `space` provider（P1-3 font 子 slice 2，2026-07-05）：
  `font/providers.rs` 新增 `FontProviderDefinition` 枚举（`Bitmap`/`Space`），
  `flatten_into` 按 provider 在 `providers` 数组中的原始出现顺序把两种类型
  一起推入同一个有序列表（不再只收 bitmap），解析 `space` provider
  （`SpaceProvider.Definition` codec：`advances: Map<Integer, Float>`）——键
  按 `.chars()` 取单码点（非单码点报错，覆盖 ZWNJ 等非 BMP-safe 场景）、值
  narrrow 到 `u32`（vanilla 是 float，但 `font/include/space.json` 里
  `" "=4`、`‌(ZWNJ)=0` 均为整数，故沿用现有 advance 管线的 `u32`，遇到
  分数/负数 advance 直接报错而非静默截断）。`font.rs` 新增
  `FontAtlasEntry::{Bitmap,Space}`，`build_hud_font_atlas` 按 provider 顺序
  遍历：`Bitmap` 分支照旧铺 atlas 像素+收码点 glyph，`Space` 分支
  `collect_space_glyphs` 直接把 `(codepoint, advance)` 插入
  `HudFontGlyphMap`（`insert_first_wins`，零像素 `HudDigitGlyph`：
  `width=height=0`、`uv` 沿用 `default()`——对应 vanilla `EmptyGlyph.bake`
  返回 `createGlyph=null` 不产出可绘制内容）。atlas 宽高只从 `Bitmap` 分支
  算（`space` 不贡献像素，至少需一个 bitmap 分支兜底，否则报错同旧行为）。
  删除 font.rs 硬编码 `SPACE_ADVANCE=4` 特判：`' '` 的真实 advance 现在来自
  排在 bitmap 页之前的 `space` provider（`FontSet.computeGlyphInfo`
  first-provider-wins ——`font/include/space` 在 `font/default.json` 里排在
  `include/default`（bitmap 页）之前），bitmap 页自己那格空白 space cell
  仍会算出一个 advance 但被 first-wins 盖掉；`bbb-renderer` 的绘制循环
  （`hud.rs` `push_hud_inventory_text_labels`/`push_hud_inventory_tooltip`）
  本来就有 `glyph.width > 0 && glyph.height > 0` 才发 quad 的判断，零像素
  glyph 天然只走 advance 累加分支，未改动绘制代码。测试：providers.rs
  space provider 解析（含 ZWNJ 键、非单码点键报错、分数 advance 报错）、
  reference 链顺序测试同步验证 space 排第一且和三张 bitmap 页共用一个有序
  列表；font.rs `space_provider_advance_wins_over_the_blank_bitmap_space_cell`
  （4 胜过 bitmap 页自算值）、`space_provider_precedes_a_bitmap_page_with_its_own_space_cell`、
  `zero_width_non_joiner_advances_zero_and_bakes_no_pixels`；hud.rs
  `space_provider_zero_pixel_glyphs_advance_without_a_visible_quad`
  （ZWNJ 不退化 `?`、零像素不发 quad、`"a‌b"` 总宽等于 `"ab"`）。
  剩余子项：文本样式宽度/几何、unihex/CJK、bidi（账本 "Vanilla Font
  Provider Coverage" 条目）。
- [x] vanilla font 文本样式宽度/几何机制（P1-3 font 子 slice 3，2026-07-05）：
  调研结论（决定性）——bbb 无 style 输入源：chat component 解码
  `bbb_protocol::component::decode_component_summary` 把任意组件递归拍平成纯
  `String`（只取 `text`/`translate`/`fallback`/`keybind`/`selector`/`nbt`/
  `extra`/`with`），完全丢弃 `bold`/`italic`/`underlined`/`strikethrough`/
  `obfuscated`/`color` 键，故容器标题、tooltip、count 等 HUD 文本零 style；
  解码器本身缺 style 字段，本 slice 缩小为"机制 + 单测锁定 + 账本记输入端
  缺口"，不伪造 style 数据源。实现（`bbb-render-types/src/hud_glyphs.rs`）：
  `HudTextStyle`（bold/italic/underlined/strikethrough/obfuscated 布尔组，
  全 false 默认）；`HudDigitGlyph::styled_advance`（vanilla
  `GlyphInfo.getAdvance(bold)`=advance+`getBoldOffset()`=1，其余样式含
  obfuscated 等宽不改 advance）；`styled_quads`（`BakedSheetGlyph.renderChar`
  顺序：先 shadow 于 `+shadowOffset`=(1,1)、bold 时 shadow 首遍也带 bold 厚度
  再补 `+boldOffset+shadowOffset` 遍，然后 main、bold 补 `+boldOffset`=1 遍；
  每 bold 遍 `extraThickness`=0.1 四向外扩；italic 顶边 shear
  `1-0.25*up`、底边 `1-0.25*down`，`up=getTop()=7-ascent`、
  `down=getTop()+height`）；`styled_effect_rects`
  （`Font.StringRenderOutput.accept`：strikethrough 条 `y+3.5..y+4.5`、
  underline 条 `y+8.0..y+9.0`，均 `effectX0..x+advance`，行首 glyph
  `effectX0=x-1`，advance bold-aware）。`bbb-renderer/src/hud.rs`
  `hud_font_text_width` 改为委托新 `hud_font_text_width_styled`（累加
  `styled_advance`），默认样式与旧纯 advance 宽度逐字节一致（advance 恒整
  `u32`，vanilla 对分数 TTF advance 的 `Mth.ceil` 在此为 no-op）。绘制端未接
  live：HUD 现为 axis-aligned `HudRect` quad，表达不了 italic 斜切，且无 style
  输入，故 `styled_quads`/`styled_effect_rects` 是就绪机制、暂不入实时循环
  （账本记两处输入/原语缺口 + obfuscated 逐 tick 随机字形待随机源与消费端
  落地）。测试：render-types 侧 styled_advance 仅 bold 加宽/其余不变、默认
  单 quad 无斜切无厚度、shadow 首遍偏移 (1,1)、bold 双 quad x 差 1 且带 0.1
  外扩、bold+shadow 四遍序 [T,T,F,F]、italic 顶/底 shear 量与纯水平位移、
  effect 矩形 y 范围/span/行首 -1/bold 加宽；hud.rs 侧
  `hud_font_text_width_styled_adds_bold_offset_per_glyph`（bold "ab"==纯 +2、
  默认与旧函数一致、非 bold 样式不改宽）。剩余子项：style 输入端 chat
  component 投影、italic-capable 绘制原语、unihex/CJK、bidi（账本 "Vanilla
  Font Provider Coverage" 条目）。
- [x] vanilla font style 输入端投影 + HUD live 样式渲染（P1-3 font 子
  slice 4，2026-07-05）：补上子 slice 3 记录的输入端缺口。protocol：
  `bbb_protocol::component` 新增 `ComponentStyle`（bold/italic/underlined/
  strikethrough/obfuscated 为 `Option<bool>`，`color` 为解析后 `0xRRGGBB`；
  `apply_to` 即 vanilla `Style.applyTo` 子键胜出继承）与
  `decode_styled_component_summary` → `Vec<StyledTextRun>`（扁平 run，继承
  已解析；NBT byte 布尔 + `TextColor.parseColor` 命名色/#hex，非法色按宽松
  解码丢键）；旧纯文本 API 改为纯委托（run 拼接 + `"component nbt"` 空回
  退），全部旧消费者逐字节不变。输入面：`OpenScreen.title_styled`、
  `DataComponentPatchSummary.{custom_name_styled,item_name_styled,
  lore_styled}`、world `ContainerState.title_styled`（均 `#[serde(default)]`
  兼容；容器标题现无 HUD label 消费端，仅投影存储）。item-model tooltip
  投影（`item_runtime/tooltip.rs`）：`NativeItemTooltipLine` 增 `runs`
  （`bbb-render-types::HudStyledTextRun`{text,style,color}）；lore 行按
  `ItemLore.LORE_STYLE`（DARK_PURPLE+italic，`ComponentUtils.mergeStyles`
  语义——行内显式键胜出）注入默认样式，hover name 按
  `ItemStack.getStyledHoverName`（rarity 色 wrapper + custom_name 时
  ITALIC）。renderer live：label/tooltip 循环消费 runs——
  `hud_styled_text_pass_geometry`（bold 双 quad/`extraThickness`/bold-aware
  advance、per-run 色 tint、shadow 色 `ARGB.scaleRGB(textColor,0.25)` 样式
  驱动（顺带修正彩色行 shadow 曾固定灰的偏差）、underline/strikethrough
  条按 `StringRenderOutput.visit` 序在该 pass glyph 后绘制），全部几何出自
  已锁定 `styled_quads`/`styled_effect_rects`；`hud_styled_quad_vertices`
  任意角点 quad 原语落地（axis-aligned 时与旧 `hud_quad_vertices` 逐字节
  等价），italic 几何暂剥离（斜切待视觉核对 slice）、obfuscated 原字形；
  count label 保持无样式（vanilla 即无样式）。sanitize 归一：空 runs 合成
  单默认 run，绘制环只有 runs 一条路径。测试：protocol 嵌套 extra 继承/
  各布尔键/命名+hex 色/非法色丢键/`apply_to`/委托等价；data-components
  styled name+lore 解码；world styled title 存储与 set_content 保留；
  item-model lore 默认样式注入与显式 italic:0b/色覆盖、custom name
  italic+rarity 色；renderer plain-run 与旧 cell 逐字节一致、bold 双
  quad+加宽 pen、underline/strikethrough 条 y 带与行首 -1、shadow pass
  偏移+色缩放（白字==旧固定灰）、宽度预算截断、italic/obfuscated 退化
  等价。剩余：italic 斜切放开 + obfuscated 逐 tick 随机字形（账本
  "Vanilla Font Provider Coverage" 条目 Next action）。
- [x] vanilla font italic 斜切放开 + obfuscated 逐帧随机字形（P1-3 font 子
  slice 5，2026-07-05）：关掉子 slice 4 记录的最后两处退化。italic——
  `hud_styled_text_pass_geometry` 删去刻意剥离 italic 的 `geometry_style`，
  直接把 `run.style` 喂给已锁定的 `styled_quads`，italic run 遂绘制斜切角点
  （顶边 `1-0.25*up`、底边 `1-0.25*down`）；非 italic run 因 shear=0 逐字节
  不变。obfuscated——非空格 obfuscated 字形按 vanilla
  `Font.getGlyph`/`FontSet.getRandomGlyph` 替换成等 advance 随机字形。随机链
  设计（结构不变量：禁 wall-clock 随机）：`bbb-render-types/src/hud_glyphs.rs`
  新增 `HudObfuscatedRandom`（vanilla `LegacyRandomSource` 48-bit LCG 克隆，
  常量同 audio.rs：mult=25214903917/inc=11/mask=2^48-1，只用 `next_int_bound`）
  与 `HudObfuscatedGlyphPool`（按 advance 分桶的 `HudFontGlyphMap` 镜像
  `FontSet.glyphsByWidth`，`from_glyph_map` 一次构建、renderer 在
  `upload_hud_font_atlas` 缓存进新字段 `hud_obfuscated_glyph_pool`，绝不逐帧
  扫全表；advance 恒整 `u32`，vanilla `Mth.ceil` 为 no-op，桶键即 advance）。
  种子来源：renderer `self.counters.frame_index`（确定性、非快照、非
  wall-clock）——每 pass 用它 `with_seed` 重置一条 LCG，仅在实际替换时前进
  一步（对齐 vanilla `Font.random` 每 `getRandomGlyph` 触碰一次），故 shadow
  pass 与 main pass 选同一替身、固定帧→固定字形序列、帧号递增→逐帧抖动。
  pen advance 恒取原字形（等 advance 也保 layout 帧稳），空格（码点 32）不
  替换。线路：`hud_styled_text_pass_geometry` 增 `obfuscated_pool`+
  `obfuscated_seed` 两参，`push_hud_inventory_text_labels`/
  `push_hud_inventory_tooltip` 透传，`collect_hud_draws` 两处调用喂
  `&self.hud_obfuscated_glyph_pool` + `self.counters.frame_index`；
  `HudObfuscatedGlyphPool` 经 `crate::hud` 重导出给 renderer。测试：
  render-types 侧 LCG 同种子同流/异种子分流/`next_int_bound` 有界、pool 按
  advance 分桶/选中等 advance/空桶或缺桶 None；renderer 侧 italic live 角点
  对拍 `styled_quads`（含斜切真实位移）、非 italic run 与机制层逐字节且种子
  惰性、obfuscated 固定种子确定性+等 advance+pen 网格不移+序列非全同、异种子
  序列变化、空格永不替身（多种子 2 quad/pen 6+4）。剩余：bidi / unihex
  defer（账本 "Vanilla Font Provider Coverage"）。
- [x] entity preview 实际 GPU PIP drawing（2026-07-05）：完成记录随历史归属
  归档在下方 P1-4 段 entity-in-UI 小节（`entity_preview_pip_passes` step +
  per-preview 隔离 PIP target + GUI-ortho 实体绘制 + HUD blit + headless
  readback）。
- [x] vanilla sign edit screen renderer presentation（P2 HUD slice，
  2026-07-08）：native 从已有 sign editor input state 投影
  `HudSignEditorScreen`，普通 sign 按 block state wood/form 生成 flat-light
  GUI sign PIP preview（96x102，`Lighting.Entry.ITEMS_FLAT`，复用
  `SignModel` / `sign_textured_layer_passes`，无 world block-entity root
  transform），hanging sign 加载并绘制
  `textures/gui/hanging_signs/<wood>.png` 背景。HUD 使用 vanilla
  `sign.edit` / `hanging_sign.edit` 英文标题、普通/hanging 文本中心 y
  （90/125）、line height（10/9）、普通 sign text scale `0.9765628`、
  300ms 光标闪烁、插入/尾部 `_` 光标和 selection overlay；text tint 复用
  `sign_text_base_color` 的 glowing / darkened dye 规则。RendererFrame
  单次提交新增 `hud_sign_editor_screen`，sign screen 打开时 suppress
  inventory screen。测试覆盖 renderer sanitizer（仅 `ItemsFlat + Sign`
  允许进入 sign GUI PIP、hanging screen 丢弃 PIP）和 native runtime
  projection（oak standing sign / bamboo hanging sign）。
- [x] recipe-book overlay shell（P2 HUD slice，2026-07-08）：依据
  `AbstractRecipeBookScreen` / `RecipeBookComponent`，在
  `RecipeBookSettings.open` 为 true 时为 local inventory、crafting table、
  furnace、blast furnace、smoker 投影 147x166
  `textures/gui/recipe_book.png` 面板；非窄屏主 GUI 按 vanilla 相对 recipe
  book origin 右移 149px，slot hover/tooltip、主 GUI 背景、文字、实体预览和
  非 cursor 浮动物品使用同一 offset。剩余：recipe-book paged/category-backed recipe buttons、
  recipe placement、tab visibility/animation、ghost recipe slots、
  narrow-screen overlap。
- [x] recipe-book toggle button（P2 HUD/input slice，2026-07-08）：依据
  `RecipeBookComponent.RECIPE_BUTTON_SPRITES`、`InventoryScreen` /
  `CraftingScreen` / `AbstractFurnaceScreen` 的按钮位置，以及
  `RecipeBookComponent.setVisible` 的设置更新和
  `ServerboundRecipeBookChangeSettingsPacket` 发送行为，加载并投影
  `recipe_book/button` 与 `recipe_book/button_highlighted`，为 local
  inventory、crafting table、furnace、blast furnace、smoker 处理左键 toggle；
  本地更新对应 `RecipeBookSettings` type、保留 filtering，并排队
  `RecipeBookChangeSettingsCommand`。剩余：recipe-book paged/category-backed recipe buttons、
  recipe placement、tab visibility/animation、ghost recipe slots、
  narrow-screen overlap。
- [x] recipe-book filter toggle（P2 HUD/input slice，2026-07-08）：依据
  `RecipeBookComponent.initVisuals` 的 filter `CycleButton` 坐标
  `(xo+110, yo+12)`、尺寸 26x16、`toggleFiltering` +
  `sendUpdateSettings` 行为，以及 crafting/furnace family 的四态 sprite
  名称，加载并投影 `recipe_book/filter_*` 与
  `recipe_book/furnace_filter_*`；recipe book 打开时支持 hover 高亮和左键
  toggle，保留 open、翻转 filtering，并排队
  `RecipeBookChangeSettingsCommand`。剩余：recipe-book paged/category-backed recipe buttons、
  recipe placement、tab visibility/animation、ghost recipe slots、
  narrow-screen overlap。
- [x] recipe-book search input shell（P2 HUD/input slice，2026-07-08）：依据
  `RecipeBookComponent.initVisuals` 的 `EditBox(xo+25, yo+13, 81, 14)`、
  `maxLength=50`、白色文本和 `widget/text_field(_highlighted)` sprites，以及
  `RecipeBookComponent.keyPressed` / `charTyped` 的 focused 搜索框吞键行为，
  在 recipe book 面板投影搜索框，`ClientInputState` 保存搜索文本/焦点，本地
  支持 printable text、backspace/delete、方向键、Home/End、Ctrl+A、点击聚焦
  和 chat-key 聚焦；focused search 下 `E` 不再关闭容器。剩余：
  recipe-book category/page recipe buttons、recipe placement、visible recipe-grid search
  filtering、tab visibility/animation、search cursor/selection rendering、ghost
  recipe slots、narrow-screen overlap。
- [x] recipe-book tab button shell（P2 HUD/input slice，2026-07-08）：依据
  `RecipeBookTabButton` 的 `recipe_book/tab(_selected)` sprites、35x27 尺寸、
  selected x 偏移 -2、fake item icon 坐标，以及
  `RecipeBookComponent.updateTabs` 的 `xOrigin-30` / `yOrigin+3+27*i` 布局，
  加载并投影 crafting/local inventory、furnace、blast furnace、smoker 的
  vanilla tab sets；tab icons 走既有 HUD item/block-model 图标路径；点击 tab
  只更新本地 selected index、不发包，并取消搜索框焦点。剩余：
  recipe-book category/page recipe buttons、recipe placement、recipe-category-backed tab
  visibility、tab notification animation、visible recipe-grid search filtering、
  ghost recipe slots、narrow-screen overlap。
- [x] crafting recipe-book recipe button shell（P2 HUD slice，2026-07-08）：依据
  `RecipeBookPage` 的 20 个 `RecipeButton` 坐标
  `(xo+11+25*(i%5), yo+31+25*(i/5))` 和 `RecipeButton` 的 25x25
  `recipe_book/slot_*` 背景 + `(x+4,y+4)` fake item 结果图标，加载 slot
  sprites，并在 crafting/local inventory 的 search tab 上投影最多 20 条
  `ClientRecipeBookState` 已知 structured crafting recipe 结果；图标复用既有
  HUD item/block-model 路径。剩余：category/page recipe buttons、
  furnace-family raw recipe displays、craftability/multiple-recipe slot sprite
  selection、recipe placement、recipe-category-backed tab visibility、tab
  notification animation、visible recipe-grid search filtering、ghost recipe
  slots、narrow-screen overlap。
- [x] crafting recipe-book category/page shell（P2 HUD/input slice，2026-07-08）：
  依据 `ClientRecipeBook.rebuildCollections` 的 category/group 聚合、
  `SearchRecipeBookCategory.CRAFTING` 的 equipment/building/misc/redstone
  展开顺序、`CraftingRecipeBookComponent.canDisplay` 的 2x2/3x3 grid 判定，
  以及 `RecipeBookPage` 的 20/页、page forward/back 12x17 按钮和
  `current/total` 页码布局，新增共享 crafting recipe-book UI collections；
  runtime 按 selected tab/category 和 page 投影按钮、page arrows 与页码，
  input 点击 arrows 只翻本地页且不发包，tab 切换重置 page。剩余：
  furnace-family raw recipe displays、craftability slot sprite selection、
  recipe placement、recipe-category-backed tab visibility、tab notification
  animation、visible recipe-grid search filtering、search cursor/selection
  rendering、ghost recipe slots、narrow-screen overlap。
- [x] recipe-book placement command shell（P2 input slice，2026-07-08）：
  依据 `RecipeBookPage.mouseClicked` 在 page arrows 之后处理 recipe button、
  左键取 `RecipeButton.getCurrentRecipe()`，以及
  `RecipeBookComponent.tryPlaceRecipe` 调用
  `handlePlaceRecipe(containerId, recipe, event.hasShiftDown())`，在 bbb 的
  recipe book input path 中命中当前可见 crafting recipe button 后排队
  `PlaceRecipeCommand`，携带 open container id、recipe index 和 shift
  状态作为 `use_max_items`；不发额外 settings 包并取消 search focus。剩余：
  furnace-family raw recipe displays、craftability slot sprite selection、
  craftability retry guard、multi-recipe cycling/right-click overlay、
  recipe-category-backed tab visibility、tab notification animation、visible
  recipe-grid search filtering、search cursor/selection rendering、ghost recipe
  slots、narrow-screen overlap。
- [x] crafting recipe-book category tab visibility（P2 HUD/input slice，2026-07-08）：
  依据 `RecipeBookComponent.updateTabs` 固定显示 search tab、category tab
  仅在 `RecipeBookTabButton.updateVisibility` 发现
  `RecipeCollection.hasAnySelected()` 时显示，并按可见序号
  `yOrigin+3+27*visibleIndex` 布局；bbb 现在复用 shared crafting recipe-book
  collections，只显示有 2x2/3x3 可见 recipes 的 crafting category tabs，
  渲染位置向上收拢，点击仍映射回原始 category tab index。剩余：
  furnace-family raw recipe displays、craftability slot sprite selection、
  craftability retry guard、multi-recipe cycling/right-click overlay、tab
  notification animation、visible recipe-grid search filtering、search
  cursor/selection rendering、ghost recipe slots、narrow-screen overlap。
- [x] crafting recipe-book ghost recipe slots（P2 HUD/protocol slice，2026-07-08）：
  依据 `ClientPacketListener.handlePlaceRecipe` 的 container id 匹配门槛、
  `RecipeBookComponent.fillGhostRecipe` / `CraftingRecipeBookComponent.fillGhostRecipe`
  的 result/input 分派、`PlaceRecipeHelper.placeRecipe` 的 shaped recipe
  居中映射，以及 `GhostSlots.extractRenderState` 的 red pre-fill、fake item、
  white post-fill、result decorations 顺序，native 现在把
  `ClientboundPlaceGhostRecipePacket` 解成结构化 `RecipeDisplaySummary`，world
  保存 last ghost display，HUD 对 crafting table 与 local inventory 投影直接
  item/item-stack ghost fills 和 fake items，并在 container id 过期时隐藏。
  剩余：furnace-family raw recipe displays、craftability slot sprite selection、
  craftability retry guard、multi-recipe cycling/right-click overlay、tab
  notification animation、visible recipe-grid search filtering、search
  cursor/selection rendering、tag/composite SlotDisplay ghost ingredient cycling、
  narrow-screen overlap。
- [x] crafting recipe-book visible search filtering（P2 HUD/input slice，2026-07-08）：
  依据 `RecipeBookComponent.updateCollections` 先取 selected tab collections，
  再用 `connection.searchTrees().recipes().search(search.toLowerCase(Locale.ROOT))`
  求交集；`SessionSearchTrees.updateRecipes` 以 result item tooltip lines 和
  item identifiers 建 recipe collection 搜索索引。native 现在在 shared
  crafting recipe-book collection 管线按 result tooltip text、resource id、
  protocol id text 过滤，HUD recipe buttons、page controls/text 与点击命中
  共用同一 filtered collection。剩余：furnace-family raw recipe displays、
  craftability slot sprite selection、craftability retry guard、multi-recipe
  cycling/right-click overlay、tab notification animation、完整
  `FullTextSearchTree` token/namespace-path/intersection 语义、search
  cursor/selection rendering、tag/composite SlotDisplay ghost ingredient
  cycling、narrow-screen overlap。

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
  - [x] entity preview 实际 GPU PIP drawing（P1-3 slice，2026-07-05，完成记录
    归档于此 P1-4 段）：新增 `entity_preview_pip_passes` frame step（登记
    `FRAME_STEPS`，位于 `first_person_item_pass` 与 `hud_passes` 之间——对应
    vanilla `GuiRenderer.prepare` 在 GUI draw 前执行 `preparePictureInPicture`），
    每个已 sanitize 的 `HudEntityPreview` 渲染进 per-preview 持久 PIP target
    （私有 color+depth 纹理，尺寸=preview rect（bbb GUI 像素 1:1 surface 像素，
    vanilla `bounds×guiScale` 之 guiScale=1），仅 bounds 变化时重建
    （`needsAResize`），逐 preview 清空
    `clearColorAndDepthTextures(color,0,depth,1.0)`）。实体 mesh 走生产
    entity-model dispatch/layer 管线在原点烘焙
    （`bake_hud_entity_preview_pip_geometry` 把 cutout/armor/glint/translucent/
    dynamic-skin/dynamic-texture 桶拼接进 `FrameDataBuffer` 流 + 逐桶 draw
    range），用现有 entity model pipelines 绘制；相机
    `CameraUniform::hud_entity_preview_pip` = vanilla
    `setupOrtho(-1000,1000,w,h,invertY)` GUI ortho ×
    `T(w/2,h/2,0)·S(s,s,-s)·T(translation)·R(rotation)`
    （`GuiEntityRenderer.getTranslateY==height/2`），
    `Lighting.Entry.ENTITY_IN_UI` 光照、fog 关闭。HUD pass 以新增
    `HudDrawCommand::EntityPreviewBlit` 在 inventory background 之后、slot
    高亮/物品之前 blit PIP 纹理（vanilla `blitTexture`→
    `addBlitToCurrentLayer` 的提交序）；scissor 预览 blit `rect ∩ scissor`
    并采样对应子 UV，wgpu row-0-top 使 vanilla `v0=1,v1=0` GL 翻转为恒等。
    测试：PIP 相机矩阵链确定性（camera.rs）、桶拼接/index 重基/armor+glint
    路由（gui_preview.rs）、隔离 clear 与共享深度零引用 source-pin、NEAREST
    blit 采样 + resize-only 重建 pin、blit UV 子矩形（hud.rs）、FRAME_STEPS
    顺序自动断言、端到端 headless GPU readback（llvmpipe：blit rect 内实体
    像素命中、PIP 透明像素保留 HUD 背景、rect 外背景不受影响）。剩余：
    preview `item_layers`（仅含 item id 元数据，GPU 绘制需 native 侧烘焙
    item quad 交接）、override-camera orientation 消费（当前 preview 实体无
    billboard 特性受其影响）、PIP glint 滚动时间、creative inventory tab。
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
    `64 / randomBetween(0.1, 0.9)`. Renderer ticks now use the world
    collision callback for their vanilla `move` path and remove the particle
    when `onGround` becomes true.
  - [x] `DripParticle.HoneyHangProvider` / `HoneyFallProvider` /
    `HoneyLandProvider`：renderer descriptor now maps `dripping_honey`,
    `falling_honey`, and `landing_honey` to random sprites, vanilla
    DripParticle opaque layer, zero initial velocity, physics metadata,
    fixed honey tints, `0.98` friction, direct gravity motion, hang-particle
    `0.02` post-move damping, lifetimes `100`,
    `64 / (random * 0.8 + 0.2)`, and
    `128 / (random * 0.8 + 0.2)`, with gravity `0.000012`, `0.01`, and
    `0.06`. The falling provider now removes on `onGround` through the
    collision-backed `move` path. Hang-to-fall child spawning, fall-to-land
    child spawning, and local drip sound remain in the world-coupled
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
    `0.06`. The falling provider now removes on `onGround` through the
    collision-backed `move` path. Hang-to-fall child spawning and
    fall-to-land child spawning remain in the world-coupled particle/audio
    follow-up.
  - [x] `DripParticle.LavaHangProvider` / `LavaFallProvider` /
    `LavaLandProvider`：renderer descriptor now maps `dripping_lava`,
    `falling_lava`, and `landing_lava` to random sprites, vanilla
    DripParticle opaque layer, zero initial velocity, physics metadata,
    `0.98` friction, direct gravity motion, non-glowing world light, initial
    default-white cooling hang color with runtime
    `CoolingDripHangParticle.preMoveUpdate` RGB formula, hang-particle `0.02`
    post-move damping, lifetimes `40`, `64 / (random * 0.8 + 0.2)`, and
    `16 / (random * 0.8 + 0.2)`, with gravity `0.0012`, `0.06`, and `0.06`.
    The falling provider now removes on `onGround` through the collision-backed
    `move` path. Hang-to-fall child spawning, fall-to-land child spawning, and
    lava-fluid removal remain in the world-coupled particle/audio follow-up.
  - [x] `DripParticle.WaterHangProvider` / `WaterFallProvider`：renderer
    descriptor now maps `dripping_water` and `falling_water` to random sprites,
    vanilla DripParticle opaque layer, zero initial velocity, physics metadata,
    fixed blue tint, non-glowing world light, `0.98` friction, direct gravity
    motion, hang-particle `0.02` post-move damping, lifetimes `40` and
    `64 / (random * 0.8 + 0.2)`, with gravity `0.0012` and `0.06`.
    The falling provider now removes on `onGround` through the collision-backed
    `move` path. Hang-to-fall child spawning, fall-to-splash child spawning, and
    water-fluid removal remain in the world-coupled particle/audio follow-up.
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
    `0.06`. The falling providers now remove on `onGround` through the
    collision-backed `move` path. Hang-to-fall child spawning,
    fall-to-land/splash child spawning, dripstone local sound, and fluid
    removal remain in the world-coupled particle/audio follow-up.
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
    particle-atlas `OPAQUE` per vanilla. Later slices add concrete atlas UVs,
    tint, and sprite-transparency-driven `TRANSLUCENT_TERRAIN` /
    `TRANSLUCENT_ITEMS` selection.
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
  - [x] terrain/item particle atlas GPU bind-range baseline：renderer particle
    vertex batches now store per-atlas draw ranges under the vanilla opaque /
    translucent particle pipelines, so `SingleQuadParticle.Layer` selections
    can bind particle, terrain, or item atlas textures before the shared
    lightmap and draw call. Native terrain atlas upload also forwards block
    atlas sprite UVs to the particle renderer path. Later slices add concrete
    block/item sprite resolution, terrain tint, item sprite UV catalog upload,
    and sprite-transparency-driven terrain/item layer selection.
  - [x] terrain particle block-state sprite lookup：native terrain texture state
    now preserves the resolved block model `textures.particle` material, maps
    vanilla block-state ids to terrain atlas sprite ids, and installs that map
    into the particle resolver after terrain atlas upload. Definition-less
    `minecraft:block`, `minecraft:dust_pillar`, and `minecraft:block_crumble`
    spawn commands therefore carry the same particle material sprite id used by
    vanilla `TerrainParticle` via
    `BlockStateModelSet.getParticleMaterial(blockState).sprite()`. Tests cover
    pack particle texture retention, terrain block-state sprite lookup, and
    native command sprite-id emission. Block marker, terrain tint, and generic
    item-stack sprite selection stay with the remaining terrain/item particle
    rendering follow-up.
  - [x] item particle sprite UV catalog upload：`NativeItemRuntime` now exposes
    the stitched item atlas sprite-id to UV catalog using the same half-texel
    content rects as item icon resolution, and native startup forwards that
    catalog to the renderer after the shared item entity atlas is uploaded.
    The particle draw path can therefore bind the item atlas and look up item
    sprite UVs once per-stack `BreakingItemParticle` sprite selection is
    resolved; concrete item sprite selection remains broader terrain/item
    particle rendering follow-up work.
  - [x] sprite-transparency-driven terrain/item particle layer selection：
    terrain and item particle sprite catalogs now carry stitched-atlas
    `hasTranslucent` metadata into renderer state. The particle vertex batcher
    resolves the current terrain/item sprite at emission time and mirrors
    vanilla `SingleQuadParticle.Layer.bySprite`: translucent terrain/item
    sprites route to `TRANSLUCENT_TERRAIN` / `TRANSLUCENT_ITEMS`, while opaque
    and transparent cutout-only sprites stay in `OPAQUE_TERRAIN` /
    `OPAQUE_ITEMS`. Renderer tests cover both terrain and item atlas pipeline
    selection.
  - [x] fixed item particle sprite lookup：renderer `BreakingItemParticle`
    instances now resolve the three fixed vanilla item providers to concrete
    item atlas sprite ids from local 26.1 assets:
    `minecraft:item_slime` -> `minecraft:item/slime_ball`,
    `minecraft:item_cobweb` -> `minecraft:block/cobweb`, and
    `minecraft:item_snowball` -> `minecraft:item/snowball`. Renderer tests
    cover both provider state and item-atlas draw-range emission when the item
    sprite UV catalog contains the fixed sprite. The generic `minecraft:item`
    provider still waits for full ItemStack material selection.
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
    visual tint. map-color fallback, biome-aware per-spawn BlockColors, and
    on-ground roll reset remain terrain/collision-coupled follow-up work.
  - [x] terrain particle BlockColors layer-0 tint：native installs vanilla
    `BlockColors.createDefault()` layer-0 `colorAsTerrainParticle` output for
    terrain particles and non-FallingBlock `falling_dust`, covering constants,
    default colormap tint, redstone power, stem age, lily pad world color, and
    the `grass_block` particle-white special case. `TerrainParticle` providers
    upload `0.6 * tint`, while `falling_dust` uses the raw tint; `block_marker`
    remains sprite-only like vanilla.
  - [x] `falling_dust` foundational static mapColor fallback：native now mirrors
    the provider's final `blockState.getMapColor(level, pos).col` fallback for
    foundational non-tinted static map colors: stone, dirt, base plank colors,
    and `oak_log`'s vanilla `logProperties` axis split. The resolver keeps the
    vanilla branch order (`FallingBlock#getDustColor`, then BlockColors tint,
    then mapColor fallback). Full mapColor catalog coverage, biome-aware
    per-spawn BlockColors, and on-ground roll reset remain follow-up work.
  - [x] `falling_dust` wood/log/stem static mapColor expansion：native extends
    the same final provider fallback through vanilla `Blocks.logProperties`
    axis colors for spruce/birch/jungle/acacia/cherry/pale-oak/mangrove logs,
    bamboo block, stripped log variants, fixed wood/stripped-wood map colors,
    and `netherStemProperties` / hyphae colors for crimson and warped stems.
    Tests pin representative top-vs-side axis splits and static stem/hyphae
    RGBs. Full mapColor catalog coverage, biome-aware per-spawn BlockColors,
    and on-ground roll reset remain follow-up work.
  - [x] `falling_dust` wooden stairs/slabs static mapColor expansion：native now
    mirrors vanilla `registerLegacyStair(base)` / slab mapColor inheritance for
    oak, spruce, birch, jungle, acacia, cherry, dark oak, pale oak, mangrove,
    bamboo, bamboo mosaic, crimson, and warped stairs/slabs. Tests pin
    representative stair and slab states for each inherited color while leaving
    doors, trapdoors, fences, signs, buttons, and pressure plates to later
    wooden-derivative slices. Full mapColor catalog coverage, biome-aware
    per-spawn BlockColors, and on-ground roll reset remain follow-up work.
  - [x] `falling_dust` wooden pressure plates static mapColor expansion：native
    now mirrors vanilla plank mapColor inheritance for oak, spruce, birch,
    jungle, acacia, cherry, dark oak, pale oak, mangrove, bamboo, crimson, and
    warped pressure plates. Tests pin the `powered=true` state for each
    inherited color while leaving buttons, stone/weighted plates, and the wider
    wooden door/trapdoor/fence/sign families to later slices. Full mapColor
    catalog coverage, biome-aware per-spawn BlockColors, and on-ground roll
    reset remain follow-up work.
  - [x] `falling_dust` wooden fixtures static mapColor expansion：native now
    mirrors vanilla plank mapColor inheritance for oak/spruce/birch/jungle/
    acacia/cherry/dark oak/pale oak/mangrove/bamboo/crimson/warped doors,
    trapdoors, fences, and fence gates. It also mirrors vanilla
    `buttonProperties()` leaving all stone, polished blackstone, and wooden
    buttons on `MapColor.NONE.col == 0`. Tests pin representative open /
    waterlogged / connected states for each wooden fixture family plus all
    vanilla button variants. Full mapColor catalog coverage, biome-aware
    per-spawn BlockColors, signs, and on-ground roll reset remain follow-up
    work.
  - [x] `falling_dust` wooden signs static mapColor expansion：native now
    mirrors vanilla `StandingSignBlock`, `WallSignBlock`,
    `CeilingHangingSignBlock`, and `WallHangingSignBlock` mapColor
    registrations for oak/spruce/birch/jungle/acacia/cherry/dark oak/pale oak/
    mangrove/bamboo/crimson/warped signs. Tests pin ordinary sign and wall-sign
    states plus ceiling and wall hanging-sign states, including vanilla's
    `cherry_*_hanging_sign` `TERRACOTTA_PINK` color and
    `spruce_wall_hanging_sign` `WOOD` override. Full mapColor catalog coverage,
    biome-aware per-spawn BlockColors, and on-ground roll reset remain follow-up
    work.
  - [x] `falling_dust` wooden shelf static mapColor expansion：native now mirrors
    vanilla `ShelfBlock` mapColor registrations for oak/spruce/birch/jungle/
    acacia/cherry/dark oak/pale oak/mangrove/bamboo/crimson/warped shelves.
    Tests pin representative facing/powered/side-chain/waterlogged states for
    every shelf family.
  - [x] `falling_dust` banner static mapColor expansion：native now mirrors
    vanilla `BannerBlock` and `WallBannerBlock` registration where every
    `DyeColor` banner variant still uses `MapColor.WOOD` for
    `blockState.getMapColor(...).col`. Tests pin all 16 standing banners and
    all 16 wall banners to the WOOD RGB. Full mapColor catalog coverage,
    biome-aware per-spawn BlockColors, and on-ground roll reset remain follow-up
    work.
  - [x] `falling_dust` colored-family static mapColor expansion：native now
    resolves vanilla `DyeColor.getMapColor()` families for wool, carpets,
    concrete, stained glass, and glazed terracotta, while using the separate
    `MapColor.TERRACOTTA_*` palette for colored terracotta and keeping plain
    terracotta on `MapColor.COLOR_ORANGE`. Tests pin representative DyeColor
    and terracotta RGBs, including the white DyeColor-vs-terracotta split.
    Full mapColor catalog coverage, biome-aware per-spawn BlockColors, and
    on-ground roll reset remain follow-up work.
  - [x] `falling_dust` mineral/natural static mapColor expansion：native extends
    the same final provider fallback through ore families, deepslate variants,
    nether stone/brick families, snow/ice/clay/sandstone/suspicious block
    colors, resource blocks, soul soil/sand, basalt/obsidian/ancient debris,
    and glow lichen. Tests pin representative overworld/deepslate/nether ore
    colors, resource-block RGBs, suspicious sand/gravel properties, snow/ice,
    clay, deepslate, nether, soul, and basalt cases. Full mapColor catalog
    coverage, biome-aware per-spawn BlockColors, and on-ground roll reset remain
    follow-up work.
  - [x] `falling_dust` decorative colored static mapColor expansion：native
    extends the same final provider fallback through bed, candle, and shulker
    box color families. The helper mirrors vanilla's non-DyeColor exceptions:
    bed head parts use `MapColor.WOOL`, white candles use `MapColor.WOOL`, and
    purple shulker boxes use `MapColor.TERRACOTTA_PURPLE`. Tests pin those
    exceptions plus representative foot-bed, base candle, and uncolored shulker
    colors. Full mapColor catalog coverage, biome-aware per-spawn BlockColors,
    and on-ground roll reset remain follow-up work.
  - [x] `falling_dust` cave/emissive static mapColor expansion：native extends
    the same final provider fallback through amethyst blocks/buds, the tuff
    family, calcite, tinted glass, powder snow, sculk sensor/sculk families, and
    all three froglight colors. Tests pin representative amethyst, tuff,
    calcite, tinted-glass, powder-snow, sculk sensor, sculk/shrieker, and
    froglight RGBs. Full mapColor catalog coverage, biome-aware per-spawn
    BlockColors, and on-ground roll reset remain follow-up work.
  - [x] `falling_dust` copper weathering static mapColor expansion：native
    extends the same final provider fallback through vanilla copper weathering
    blocks, including waxed variants, cut/chiseled copper, slabs/stairs,
    doors/trapdoors, grates, bulbs, chests, copper golem statues, lightning
    rods, and raw copper block. Tests pin the four weathering-stage RGBs,
    waxed-stage preservation, raw copper, and representative multi-property
    block states. Full mapColor catalog coverage, biome-aware per-spawn
    BlockColors, and on-ground roll reset remain follow-up work.
  - [x] `falling_dust` nether flora/blackstone static mapColor expansion：
    native extends the same final provider fallback through vanilla nether
    vegetation, magma, nether wart/shroomlight, respawn anchor, smooth basalt,
    and blackstone/polished-blackstone families. Tests pin crimson/warped
    nylium, warped wart, nether wart, warped/crimson fungi, vines, magma,
    respawn anchor, smooth basalt, blackstone, and polished blackstone pressure
    plate RGBs. Full mapColor catalog coverage, biome-aware per-spawn
    BlockColors, and on-ground roll reset remain follow-up work.
  - [x] `falling_dust` quartz/prismarine/End static mapColor expansion：
    native extends the same final provider fallback through vanilla quartz and
    sea-lantern blocks, prismarine variants, End stone brick variants, purpur,
    chorus, and end portal frame static colors. Tests pin quartz block/pillar/
    stairs/bricks, sea lantern, prismarine wall, dark prismarine slab,
    prismarine brick stairs, End stone/wall, end portal frame, purpur
    pillar/slab, and chorus flower RGBs. Full mapColor catalog coverage,
    biome-aware per-spawn BlockColors, and on-ground roll reset remain
    follow-up work.
  - [x] `falling_dust` construction stone/brick static mapColor expansion：
    native extends the same final provider fallback through vanilla
    cobblestone/mossy-cobblestone, stone brick, stone/smooth-stone, andesite,
    granite, diorite, sandstone/red-sandstone, brick, mud brick, and nether
    brick stairs/slab/wall construction variants. Tests pin representative
    stone, mossy cobblestone, granite, diorite, smooth sandstone, red
    sandstone, brick, mud brick, nether brick, and red nether brick RGBs. Full
    mapColor catalog coverage, biome-aware per-spawn BlockColors, and on-ground
    roll reset remain follow-up work.
  - [x] `falling_dust` resin/pale garden static mapColor expansion：native
    extends the same final provider fallback through vanilla 26.1 resin block,
    resin clump, resin brick variants, pale moss, pale moss carpet, pale
    hanging moss, open/closed eyeblossom, and firefly bush static colors. Tests
    pin terracotta-orange resin, light-gray pale moss, orange open eyeblossom,
    metal closed eyeblossom, and plant-green firefly bush RGBs. Full mapColor
    catalog coverage, biome-aware per-spawn BlockColors, and on-ground roll
    reset remain follow-up work.
  - [x] `falling_dust` deepslate construction static mapColor expansion：
    native extends the existing deepslate family fallback through vanilla
    cobbled, polished, tile, and brick deepslate stairs/slab/wall variants that
    inherit their base block mapColor with `ofLegacyCopy`. Tests pin
    representative cobbled stairs, polished slab, tile wall, and brick stairs
    RGBs. Full mapColor catalog coverage, biome-aware per-spawn BlockColors,
    and on-ground roll reset remain follow-up work.
  - [x] `falling_dust` infested stone static mapColor expansion：native now
    mirrors vanilla's distinct CLAY mapColor for infested stone, cobblestone,
    and stone-brick variants while leaving infested deepslate on the existing
    DEEPSLATE family fallback. Tests pin CLAY RGBs plus the infested deepslate
    axis state. Full mapColor catalog coverage, biome-aware per-spawn
    BlockColors, and on-ground roll reset remain follow-up work.
  - [x] `falling_dust` natural static mapColor expansion：native now mirrors
    vanilla static mapColor fallback for non-tinted saplings/dry grass,
    pointed dripstone and dripstone blocks, cave vines, spore blossom, azalea,
    dripleaf, green moss blocks, hanging/rooted dirt, and mud. Tests pin
    representative PLANT, COLOR_PINK, METAL, COLOR_YELLOW, TERRACOTTA_BROWN,
    COLOR_GREEN, DIRT, and TERRACOTTA_CYAN RGBs while leaving tint-sourced
    petals, wildflowers, and leaf litter to the BlockColors path. Full
    mapColor catalog coverage, biome-aware per-spawn BlockColors, and
    on-ground roll reset remain follow-up work.
  - [x] `falling_dust` static foliage mapColor expansion：native now mirrors
    vanilla static mapColor fallback for non-tinted cherry leaves, pale oak
    leaves, azalea leaves, and flowering azalea leaves. Tests pin representative
    leaf `distance` / `persistent` / `waterlogged` states for COLOR_PINK,
    METAL, and PLANT RGBs while leaving oak/spruce/birch/jungle/acacia/dark
    oak/mangrove leaves and leaf litter on the BlockColors/tint path. Full
    mapColor catalog coverage, biome-aware per-spawn BlockColors, and
    on-ground roll reset remain follow-up work.
  - [x] `falling_dust` crop/succulent static mapColor expansion：native now
    mirrors vanilla static mapColor fallback for wheat's age-selected
    PLANT/COLOR_YELLOW branch plus carrots, potatoes, beetroots, nether wart,
    torchflower crop, pitcher crop/plant, cactus, and cactus flower. Tests pin
    age-selected wheat, crop age states, pitcher halves, and cactus states while
    leaving sugar cane and melon/pumpkin stems on the BlockColors/tint path.
    Full mapColor catalog coverage, biome-aware per-spawn BlockColors, and
    on-ground roll reset remain follow-up work.
  - [x] `falling_dust` produce/fungus static mapColor expansion：native now
    mirrors vanilla static mapColor fallback for brown/red mushrooms, huge
    mushroom blocks/stems, pumpkin/carved pumpkin/jack o'lantern, melon, hay
    block, and dried kelp block. Tests pin representative mushroom side
    properties, facing pumpkins, the hay axis state, and produce/storage block
    RGBs. Full mapColor catalog coverage, biome-aware per-spawn BlockColors,
    and on-ground roll reset remain follow-up work.
  - [x] `falling_dust` utility/mechanical static mapColor expansion：native now
    mirrors vanilla static mapColor fallback for bedrock/pistons/spawner/
    crafter/trial spawner/vault STONE blocks, stone pressure plate STONE,
    note/bookshelf/chest/crafting WOOD blocks, cobweb WOOL, TNT FIRE,
    decorated pot TERRACOTTA_RED, light weighted pressure plate GOLD, and heavy
    weighted pressure plate/heavy core METAL, iron door/trapdoor, brewing
    stand, lanterns, cauldron/lava cauldron/powder snow cauldron, hopper,
    stonecutter, and bell. Tests pin representative complex states for pistons,
    note blocks, chiseled bookshelf slots, chest, crafter, vault,
    stone/weighted pressure plates, metal/stone fixtures, and waterlogged
    utility blocks. Full mapColor catalog coverage, biome-aware per-spawn
    BlockColors, and on-ground roll reset remain follow-up work.
  - [x] `falling_dust` functional static mapColor expansion：native now mirrors
    vanilla mapColor registrations for scaffolding SAND, loom/barrel/
    cartography table/fletching table/lectern/smithing table/composter/beehive
    WOOD, smoker/blast furnace STONE, grindstone METAL, bee nest YELLOW, and
    target QUARTZ. Tests pin representative facing/lit/open/book/honey/
    scaffolding states plus target power `0`. Full mapColor catalog coverage,
    biome-aware per-spawn BlockColors, and on-ground roll reset remain follow-up
    work.
  - [x] `falling_dust` glowstone/enchanting/beacon static mapColor expansion：
    native now mirrors vanilla mapColor registrations for glowstone SAND,
    enchanting table COLOR_RED, and beacon DIAMOND. Tests pin each simple state
    to the expected raw mapColor RGB.
  - [x] `falling_dust` default-NONE fixture static mapColor expansion：native now
    mirrors vanilla `BlockBehaviour.Properties` default `MapColor.NONE` for
    visible fixture blocks that register without explicit mapColor and no
    BlockColors layer: ladder, floor/wall torch variants including redstone,
    soul, and copper torches, plus end rod. Tests pin facing, waterlogged, lit,
    wall, and vertical rod states to black `option_color`.
  - [x] `falling_dust` glass/bars default-NONE static mapColor expansion：
    native now mirrors vanilla default `MapColor.NONE` for plain glass, glass
    pane, and iron bars. Tests pin the plain glass state and fully connected
    waterlogged pane/bars states to black `option_color`.
  - [x] `falling_dust` metal bars/chain default-NONE static mapColor expansion：
    native now mirrors vanilla default `MapColor.NONE` for iron chain plus
    copper bars and copper chain weathering/waxed variants. Tests pin fully
    connected waterlogged copper bars and axis/waterlogged chain states.
  - [x] `falling_dust` misc static mapColor expansion：native now mirrors
    vanilla static map colors for redstone block, slime block, petrified oak
    slab, dirt path, frosted ice, and bone block. Tests pin no-property blocks
    plus slab `type`/`waterlogged`, frosted-ice `age`, and bone-block `axis`
    states.
  - [x] `falling_dust` invisible render-shape rejection coverage：tests now pin
    vanilla `!isAir && RenderShape.INVISIBLE` provider null behavior for
    water/lava, bubble column, barrier, structure void, end portal/gateway,
    light, and moving piston while preserving air/cave_air/void_air acceptance.
  - [x] `falling_dust` redstone/rail default-NONE static mapColor expansion：
    native now mirrors vanilla default `MapColor.NONE` for powered/detector/
    activator/ordinary rails, lever, repeater, comparator, tripwire hook, and
    tripwire string blocks that register without explicit mapColor or BlockColors
    tint layers. Tests pin powered and waterlogged rail shapes, lever face,
    repeater delay/lock, comparator mode, and tripwire attachment/direction
    state combinations to black `option_color`.
  - [x] `falling_dust` skull/head default-NONE static mapColor expansion：native
    now mirrors vanilla default `MapColor.NONE` for standing and wall skeleton,
    wither skeleton, zombie, player, creeper, dragon, and piglin skull/head
    blocks. Tests pin powered standing rotations and wall-facing states to black
    `option_color`.
  - [x] `falling_dust` potted default-NONE static mapColor expansion：native now
    mirrors vanilla `flowerPotProperties()` default `MapColor.NONE` for flower
    pot and non-tinted potted sapling/flower/mushroom/cactus/bamboo/fungus/
    roots/azalea/eyeblossom blocks. `potted_fern` stays on the existing
    BlockColors grass-tint path. Tests pin every covered potted block's empty
    state to black `option_color`.
  - [x] `falling_dust` cake default-NONE static mapColor expansion：native now
    mirrors vanilla `CAKE` default `MapColor.NONE` and `CANDLE_CAKE` /
    colored candle cakes copying the cake properties rather than candle
    mapColor. Tests pin cake bite state and every candle-cake lit variant to
    black `option_color`.
  - [x] `falling_dust` redstone utility/control static mapColor expansion：
    native now mirrors vanilla mapColor registrations for redstone lamp
    TERRACOTTA_ORANGE, ender chest and observer STONE, trapped chest and
    daylight detector WOOD, command block BROWN, repeating command block
    PURPLE, chain command block GREEN, and structure/jigsaw/test block
    LIGHT_GRAY. Tests pin representative lit, facing, waterlogged, powered,
    conditional, mode, and orientation states.
  - [x] `falling_dust` aquatic/coral static mapColor expansion：native now
    mirrors vanilla mapColor registrations for live coral BLUE/PINK/PURPLE/RED/
    YELLOW, dead coral GRAY, sea pickle GREEN, and conduit DIAMOND. Tests pin
    representative coral block/plant/fan/wall-fan states plus waterlogged,
    facing, pickles, and conduit states.
  - [x] `falling_dust` bamboo/honey/campfire utility static mapColor expansion：
    native now mirrors vanilla mapColor registrations for bamboo sapling WOOD,
    bamboo and sweet berry bush PLANT, campfire/soul campfire PODZOL,
    honey/honeycomb blocks ORANGE, and lodestone METAL. Tests pin representative
    bamboo age/leaves/stage, sweet berry age, campfire facing/lit/signal_fire/
    waterlogged, and simple honey/lodestone states.
  - [x] `falling_dust` water plant/egg static mapColor expansion：native now
    mirrors vanilla mapColor registrations for seagrass, tall seagrass, kelp,
    kelp plant, and frogspawn WATER; turtle egg SAND; sniffer egg RED; and
    dried ghast GRAY. Tests pin representative tall-seagrass half, kelp age,
    turtle/sniffer hatch, dried-ghast facing/hydration/waterlogged states, and
    the new WATER map color constant.
  - [x] `falling_dust` flower static mapColor expansion：native now mirrors
    vanilla mapColor registrations for dandelion, golden dandelion, torchflower,
    poppy, blue orchid, allium, azure bluet, tulips, oxeye daisy, cornflower,
    wither rose, and lily of the valley as PLANT. Tests pin every plain flower
    block state while keeping potted variants in the existing default-NONE path.
  - [x] `falling_dust` tall flower static mapColor expansion：native now mirrors
    vanilla mapColor registrations for sunflower, lilac, rose bush, and peony as
    PLANT. Tests pin representative upper/lower half states while leaving
    BlockColors-tinted tall grass and large fern out of the static fallback.
  - [x] `falling_dust` fire/cocoa/creaking heart static mapColor expansion：
    native now mirrors vanilla mapColor registrations for fire FIRE, soul fire
    COLOR_LIGHT_BLUE, cocoa PLANT, and creaking heart COLOR_ORANGE. Tests pin
    representative fire adjacency/age, soul fire empty, cocoa age/facing, and
    creaking heart axis/state/natural states.
  - [x] `falling_dust` full static mapColor catalog closeout：native now covers
    the final accepted vanilla 26.1 block states that were not handled by
    FallingBlock dust colors or `BlockColors.createDefault()` layer-0 tint:
    mycelium, packed mud, nether brick fence, nether portal default
    `MapColor.NONE`, stripped pale oak wood, and copper lantern weathering /
    waxed variants. The new
    `falling_dust_colors_cover_all_accepted_vanilla_block_states` test
    enumerates every provider-accepted vanilla block state and asserts that the
    native command resolves either a BlockColors tint or static mapColor.
    Remaining color work is biome-aware per-spawn BlockColors; on-ground roll
    reset remains collision-coupled particle ticking work.
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
  - [x] particle atlas animation upload：native particle runtime now preserves
    animated particle `SpriteImage` sources, restitches the renderer's
    single-mip particle atlas on the same 50 ms tick cadence used by terrain
    texture animation, and writes the updated RGBA frame into the existing
    particle GPU texture before render. Local vanilla 26.1 evidence is
    `assets/minecraft/textures/particle/vibration.png.mcmeta`
    (`frametime: 1`) plus `assets/minecraft/particles/vibration.json`
    referencing `minecraft:vibration`. Tests cover animated atlas-frame
    selection and the 50 ms tick gate.
  - [x] missing definition / missing sprite diagnostics：native particle
    resolution records missing definitions, unknown particle types, and missing
    sprites without dropping otherwise renderable spawn commands; renderer
    batch/counter paths preserve those diagnostic counts.
  - [x] component-driven generic item particle material：native now decodes
    `minecraft:item` particle `ItemStackTemplate` options through the protocol
    `DataComponentPatchSummary` decoder, passes item runtime context through
    the online particle effect sink, and resolves GROUND
    `BreakingItemParticle` material active-layer sprite ids for component-bearing
    stacks, including `minecraft:item_model` root model overrides. The renderer
    continues to randomly select among the resolved item atlas sprite ids.
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
  - [x] particle alpha/color curve coverage：renderer now keeps vanilla-shaped
    alpha/color changes on explicit descriptors and runtime tests:
    `SimpleAnimatedParticle` half-lifetime fade, firework spark's initial
    `0.99` alpha plus half-lifetime fade, firework flash
    `OverlayParticle.extract` alpha, shriek extract-time fade,
    vault-connection `LifetimeAlpha`, firefly `getFadeAmount`, EndRod
    half-lifetime fade-color blending, dust color-transition lerp, and decoded
    option / random / fixed provider tints and terrain particle BlockColors
    layer-0 tint. map-color fallback, biome-aware per-spawn BlockColors, and
    the wider firework `Starter` child-particle presentation stay with their
    owning follow-up items rather than the provider alpha/color curve checklist.
  - [x] LevelEvent audio side-effect closeout：the P1-5 audit rechecked local
    vanilla 26.1 `LevelEventHandler` and confirmed current world/native paths
    cover LevelEvent-derived fixed and randomized local sounds, positioned
    sound recording/playback, particle-before-sound ordering for potion break,
    dragon fireball, wax-on, bone-meal, vault, sculk-shrieker, and cobweb
    place, post-sound particle random-stream advancement for smoke,
    trial-spawner, sculk, simple particle-only, and block-face events,
    camera-relative `globalLevelEvent` sounds, portal-travel local ambience,
    and jukebox start/stop. Remaining adjacent work stays with terrain/item
    atlas rendering, block-entity client-effect presentation, or broader audio
    runtime parity rather than an open P1-5 LevelEvent audio checklist.
  - [x] block-marker terrain particle sprite selection：native block-particle
    spawn commands now resolve `minecraft:block_marker` through the same
    terrain block-state particle material sprite table as `minecraft:block`,
    `minecraft:dust_pillar`, and `minecraft:block_crumble`. This matches
    vanilla 26.1 `BlockMarker`, whose constructor passes
    `BlockStateModelSet.getParticleMaterial(state).sprite()` to
    `SingleQuadParticle`, while keeping the separate vanilla behavior that
    `BlockMarker.Provider` does not run `TerrainParticle.createTerrainParticle`
    air / moving-piston / `shouldSpawnTerrainParticles` rejection.
  - [x] generic item particle default material sprite：native item runtime now
    exports default empty-component `ItemDisplayContext.GROUND` particle
    material active-layer sprite ids by item protocol id, native
    `minecraft:item` particle spawn commands use those item atlas sprites when
    the raw `ItemParticleOption` component patch is exactly empty, and renderer
    `BreakingItemParticle` descriptors randomly select among the provided
    active-layer ids. This follows vanilla 26.1
    `BreakingItemParticle.ItemParticleProvider`, which resolves the stack
    through `ItemModelResolver.updateForTopItem(..., GROUND, ...)` before
    reading `ItemStackRenderState.pickParticleMaterial(random)`. Full component
    patch decoding remains with the wider item-particle material follow-up.
  - [x] biome-aware per-spawn BlockColors for terrain particles：native
    `LevelParticles` handling now passes a world-backed biome sampler into
    particle command resolution. `TerrainParticle` providers and
    non-FallingBlock `falling_dust` compute layer-0 `BlockColors` at each
    actual spawn position using vanilla's `BlockPos.containing(x, y, z)` owner;
    terrain particles emit `0.6 * colorAsTerrainParticle`, while
    `falling_dust` emits the raw tint before falling back to static mapColor.
    Tests cover count-randomized spawns crossing biome boundaries, falling-dust
    foliage tint, and the lightweight terrain particle tint catalog.
  - [x] LevelEvent item-break concrete item sprites：native LevelEvent potion /
    instant-potion / ender-eye break branches now keep their existing vanilla
    `ItemParticleOption` item-template metadata while resolving the first eight
    `minecraft:item` commands through the installed empty-component GROUND item
    material sprite ids. Tests pin splash-potion and ender-eye sprite ids without
    changing the vanilla random stream.
  - [x] particle ground-collision roll coupling：renderer particle ticks now
    accept a collision callback, carry vanilla-shaped `onGround` /
    `stoppedByCollision` state, apply `Particle.tick` ground X/Z damping for
    default particles, and reset `FallingDustParticle` roll on the tick after
    ground contact. Native wires this callback to a world collision probe that
    clips downward vanilla 0.2x0.2 particle AABBs against known block collision
    shape tops. Tests cover default ground damping, falling-dust roll reset, the
    world Y-clip math, and the native pump ordering marker.
  - [x] particle three-axis block-shape clipping：world particle collision now
    resolves Y first and then the largest horizontal component first, matching
    vanilla `Direction.axisStepOrder`, and clips the 0.2x0.2 particle AABB
    against known block collision shape faces for downward, upward, and
    horizontal movement. Tests cover upward ceiling clipping, horizontal side
    clipping, horizontal non-overlap rejection, and the vanilla axis-order rule.
  - [x] plain drip falling on-ground removal：`falling_nectar` and
    `falling_spore_blossom` now use a dedicated `DripFalling` tick motion that
    calls the collision-backed vanilla `move` path and removes the active
    particle when `onGround` becomes true. Tests cover the descriptor mapping
    and runtime removal via a collision callback.
  - [x] drip fall-and-land on-ground removal：`falling_honey`,
    `falling_obsidian_tear`, `falling_lava`, `falling_water`, and both
    dripstone falling variants now use a dedicated `DripFallAndLand` tick
    motion that calls the collision-backed vanilla `move` path and removes the
    active particle when `onGround` becomes true. The landing child particles,
    drip sounds, and fluid removal gates remain deferred; tests cover descriptor
    mapping and runtime removal via a collision callback.
  - [x] water-drop ground removal and drip-land split：`rain` and `splash`
    now route `WaterDropParticle` ticks through the collision-backed vanilla
    `move` path, apply the vanilla `onGround` 50% random removal gate, and keep
    X/Z ground damping after friction. Honey, obsidian-tear, and lava landing
    particles now use a separate `DripLand` tick motion that follows
    `DripParticle` move/friction behavior without the WaterDrop random ground
    removal branch. Block/fluid in-block removal remains deferred to
    world-query-backed particle ticking.
  - [x] water-drop block/fluid in-block removal：`WaterDropParticle` ticks now
    query a world-backed block/fluid surface height after movement, mirroring
    vanilla `max(collisionShape.max(Y, localX, localZ), fluidState.height)`,
    and remove `rain` / `splash` when the particle Y is below that containing
    block's surface. Native wires the query beside the existing particle
    collision callback; world tests cover slab collision and source-water
    fluid heights.
  - [x] DripParticle water/lava matching-fluid removal：renderer particle ticks
    now carry the vanilla `Fluids.WATER` / `Fluids.LAVA` provider type for
    water, lava, and dripstone drip variants, query the world-backed fluid kind
    and height after `DripParticle` move/friction, and remove only when the
    containing block has the same fluid and the particle Y is below that fluid
    surface. `Fluids.EMPTY` honey, obsidian-tear, nectar, and spore-blossom
    providers ignore the fluid sample. Native maps world `TerrainFluidKind`
    into renderer particle fluid kinds; world tests cover water/lava samples
    and renderer tests cover matching, non-matching, and empty-provider cases.
  - [x] bubble water-fluid survival gates：`BubbleParticle.Provider`,
    `BubbleColumnUpParticle.Provider`, and `WaterCurrentDownParticle.Provider`
    now carry a renderer `required_fluid=Water` runtime gate and remove after
    movement when the containing block's world-backed fluid kind is not water,
    matching vanilla `FluidTags.WATER` checks. `WaterCurrentDownParticle` now
    uses the no-physics `move` path before the water check; focused tests cover
    bubble removal outside water, bubble-column survival in water, current-down
    removal outside water, and descriptor/runtime field sampling.
  - [x] BaseAshSmoke Y-blocked speed-up：`SmokeParticle` / `LargeSmokeParticle`
    / `WhiteSmokeParticle` and related `BaseAshSmokeParticle` descriptors already
    carry vanilla `speedUpWhenYMotionIsBlocked=true`; runtime default ticking now
    has focused coverage that a world collision callback blocking Y motion
    applies the vanilla X/Z `1.1` speed-up before friction and ground damping.
  - [x] EndRod collision-free move：`EndRodParticle.Provider` keeps vanilla
    `hasPhysics=true` descriptor metadata for snapshots, but renderer runtime
    marks it as moving without collision so the default tick's gravity/friction
    path skips the world collision callback and mirrors the vanilla
    `EndRodParticle.move` bounding-box translation override.
  - [x] FallingLeaves collision removal：`FallingLeavesParticle` runtime motion
    now uses the world collision callback for the vanilla custom leaf move,
    removes on `onGround`, removes when X or Z velocity is zeroed by collision
    after the first tick (`lifetime < 299`), and keeps the first-tick
    horizontal-block grace.
  - [x] Firefly in-block removal：particle world samples now expose whether the
    containing block state is air; `FireflyParticle` runtime ticks use the
    collision-backed vanilla `super.tick()` move path and then remove when the
    post-move block sample is non-air, while retaining the existing alpha fade
    and first-tick / 5% random speed reroll for air samples.
  - [x] SquidInk in-air downward drift：`SquidInkParticle.Provider` /
    `GlowInkProvider` runtime ticks now reuse the world block-air sample after
    default `super.tick()` motion and apply the vanilla `yd -= 0.0074F` drift
    only when the containing block state is air.
  - [x] Ash / WhiteAsh collision-removal audit：local vanilla 26.1
    `AshParticle` and `WhiteAshParticle` only inherit `BaseAshSmokeParticle.tick`
    and both pass `hasPhysics=false`; renderer focused coverage now pins that
    ash uses default ticking without invoking the world collision callback, so
    the old collision-removal deferred note was stale.
  - [x] DripParticle child spawning：local vanilla 26.1 `DripParticle` confirms
    hang particles spawn their configured falling particle when lifetime expires
    and `FallAndLandParticle` variants spawn landing/splash children on
    `onGround`; native particle commands now carry the corresponding child
    templates and renderer runtime tests cover lifetime-expire and ground-hit
    child intake. Honey / dripstone local sound remains separate audio work.
  - [x] SpellParticle scoping alpha：local vanilla 26.1 `SpellParticle`
    confirms `isCloseToScopingPlayer` checks local player eye distance squared
    `<= 9.0`, first-person camera, and `player.isScoping()`. Native now builds
    the renderer particle scope context from post-input local use-item state,
    the selected/offhand item registry id `minecraft:spyglass`, and the frame
    camera eye position; renderer intake/tick preserves `originalAlpha`, hides
    close first-person spyglass Spell particles at alpha `0.0`, keeps
    `MobEffectProvider`'s constructor-overridden initial alpha, and lerps back
    toward provider alpha by `0.05` when the scope gate clears.
  - [x] PlayerCloud local-player pull：local vanilla 26.1
    `PlayerCloudParticle.tick` confirms the provider runs `super.tick()`, then
    queries a player within `2.0` blocks and, when the cloud is above the
    player's feet, moves particle Y and Y velocity 20% toward player Y /
    `getDeltaMovement().y`. Native now projects post-input local-player
    position and delta movement into renderer particle ticks; renderer
    `PlayerCloudParticle.Provider` and `SneezeProvider` consume that context
    after default motion, with focused coverage for near-player pull and
    far/lower-player no-op behavior. Broader remote-player nearest selection
    remains deferred with other player-coupled emitters.
  - [x] Totem TrackingEmitter entity event：local vanilla 26.1
    `ClientPacketListener.handleEntityEvent(35)` creates a
    `TrackingEmitter(entity, ParticleTypes.TOTEM_OF_UNDYING, 30)`, whose
    constructor immediately ticks and whose tick emits up to 16 particles from
    unit-sphere samples around `entity.getX(xa/4)`, `getY(0.5+ya/4)`, and
    `getZ(za/4)` with velocity `(xa, ya + 0.2, za)`. World/native event
    handling now turns applied entity event `35` into a native tracking-emitter
    batch, sampling current entity AABB width/height and submitting delayed
    `minecraft:totem_of_undying` commands for 30 ticks. Focused tests cover the
    Java `nextFloat` random stream, first command position/velocity, delay
    distribution, and dispatcher gating for missing entities.
  - [x] Totem use entity event sound：local vanilla 26.1
    `ClientPacketListener.handleEntityEvent(35)` calls
    `level.playLocalSound(entity.getX(), entity.getY(), entity.getZ(),
    SoundEvents.TOTEM_USE, entity.getSoundSource(), 1.0F, 1.0F, false)` after
    creating the totem `TrackingEmitter`; `SoundEvents.TOTEM_USE` registers
    `minecraft:item.totem.use`, while `Entity.getSoundSource` defaults to
    `NEUTRAL`, `Player` returns `PLAYERS`, and `Monster` / hostile overrides
    return `HOSTILE`. World/native event handling now records and dispatches a
    positioned totem-use sound for applied event `35`, using the current entity
    position and vanilla-shaped source mapping. Focused tests cover player vs
    zombie source selection, missing-entity gating, and native audio command
    resolution.
  - [x] Vibration entity PositionSource initial target：local vanilla 26.1
    `EntityPositionSource.STREAM_CODEC` confirms entity sources are encoded as
    VarInt entity id plus float `y_offset`, and `getPosition` returns
    `entity.position().add(0, y_offset, 0)`. Native level-particle command
    resolution now preserves entity id / offset for `minecraft:vibration` and
    uses the current `WorldStore::probe_entity_transform` position to seed
    renderer `option_target` when the entity is loaded; focused tests cover the
    unresolved fallback, context resolution, and dispatcher world-context
    projection. Vanilla's per-tick entity target re-query remains deferred.
  - [x] Vibration entity PositionSource per-tick refresh：local vanilla 26.1
    `VibrationSignalParticle.tick` re-queries `target.getPosition(level)` each
    tick and removes the particle when the target position is absent. Renderer
    `ParticleSpawnCommand` / `ParticleInstance` now preserve entity target
    source id plus `y_offset`; native particle ticks project current world
    entity positions into renderer entity-target contexts, and renderer
    vibration ticks refresh `option_target` each tick or remove the particle
    when the source entity is missing. Focused renderer tests cover dynamic
    target refresh, interpolation/rotation against the refreshed target, and
    missing-entity removal.
  - [x] DripParticle fall-and-land local sounds：local vanilla 26.1
    `DripParticle.HoneyFallAndLandParticle.postMoveUpdate` plays
    `SoundEvents.BEEHIVE_DRIP` on ground hit, and
    `DripstoneFallAndLandParticle.postMoveUpdate` plays the pointed dripstone
    lava/water drip sounds after spawning the landing particle. Renderer
    particle ticks now enqueue `ParticleSoundEvent`s for falling honey and
    falling dripstone lava/water ground hits with `SoundSource.BLOCKS`, pitch
    `1.0`, and vanilla `0.3..1.0` volume range; native drains those events
    after particle tick and submits existing positioned audio commands. Focused
    renderer tests cover honey plus dripstone lava/water sound ids, and native
    tests cover positioned sound-state projection plus pump ordering before
    particle light extraction.
  - [x] CampfireSmoke collision-backed move：local vanilla 26.1
    `CampfireSmokeParticle` confirms constructor `setSize(0.25F, 0.25F)`, tick
    removal when alpha is already `<= 0`, random x/z drift, `yd -= gravity`,
    and `move(xd, yd, zd)`. Renderer campfire smoke instances now use the
    vanilla 0.25x0.25 collision AABB and route ticks through the existing
    collision-backed particle move path; focused tests cover query dimensions,
    ground/stopped collision state, retained Y velocity, and alpha-zero
    removal before motion.
  - [x] Firework empty-explosion poof branch：local vanilla 26.1
    `FireworkRocketEntity.handleEntityEvent(17)` calls
    `ClientLevel.createFireworks`; empty explosions spawn
    `random.nextInt(3)+2` `minecraft:poof` particles at the rocket position
    with gaussian X/Z velocity `* 0.05` and fixed Y velocity `0.005`, while
    non-empty explosions remain the broader `Starter` follow-up. Native world
    event handling now detects firework rockets with no decoded explosions, and
    native particle runtime emits the poof batch while preserving
    `ParticleTypes.POOF` `overrideLimiter=true` behavior. Tests cover resolver
    randoms and dispatcher gating.
  - [x] Destroy-block terrain particle tint：local vanilla 26.1
    `ClientLevel.addDestroyBlockEffect` constructs `TerrainParticle` with the
    event block position as the tint position, so tint sources use
    `BlockTintSource.colorAsTerrainParticle(blockState, level, pos)` rather
    than each particle's sampled position. Native level-event context now
    carries the biome id at the event block position, and destroy-block /
    brush-complete `minecraft:block` commands attach the same
    `0.6 * colorAsTerrainParticle` option color used by terrain particles.
    Tests cover resolver tint output and dispatcher biome context threading.
  - [x] DragonBreath special tick motion：local vanilla 26.1
    `DragonBreathParticle.tick` has a provider-specific `hasHitGround` branch:
    unchanged Y position multiplies X/Z velocity by `1.1` before friction, Y
    velocity is not friction-damped until `onGround` first sets
    `hasHitGround`, and the hit-ground branch adds `0.002` upward drift each
    tick before applying friction. Renderer particles now carry this as an
    explicit `DragonBreath` tick descriptor with deterministic runtime tests for
    hovering, rising, and hit-ground branches.
  - [x] SuspendedTown collision-free move：local vanilla 26.1
    `SuspendedTownParticle` overrides `move` to translate the bounding box
    directly instead of using `Particle.move` collision clipping. Renderer
    descriptors now mark the happy-villager, composter, dolphin, mycelium, and
    egg-crack providers as `moves_without_collision`, and runtime tests assert
    the collision callback is not invoked while velocity still damps by vanilla
    friction.
  - [x] Crit constructor tick：local vanilla 26.1 `CritParticle` calls
    `tick()` at the end of its constructor before `DamageIndicatorProvider`
    overrides lifetime or `MagicProvider` applies color scaling. Renderer spawn
    now applies that constructor-time motion for the crit provider family:
    initial age becomes `1`, previous position remains the command position,
    position advances by velocity after gravity, and velocity is damped by
    vanilla `0.7` friction. Tests cover spawn-state motion and provider
    sampling.
  - [x] Flame/Portal collision-free metadata：local vanilla 26.1
    `FlameParticle` and `PortalParticle` keep the base `hasPhysics=true`
    metadata and avoid collision through provider-specific `move` overrides;
    `ReversePortalParticle` inherits the portal path. Renderer descriptors now
    preserve `has_physics=true` while marking flame, small-flame, portal, and
    reverse-portal as `moves_without_collision`. Tests cover flame runtime
    collision bypass plus provider metadata for flame and portal variants.
  - [x] PrimedTnt client smoke side effect：local vanilla 26.1
    `PrimedTnt.tick` decrements fuse, then on the client emits
    `ParticleTypes.SMOKE` at `getX(), getY() + 0.5, getZ()` with zero velocity
    while the post-decrement fuse remains positive. `WorldStore` now projects
    TNT smoke states from the current entity position plus synced/default fuse
    metadata, and native submits one `minecraft:smoke` command per advanced
    entity tick before `ParticleEngine.tick` advances particles. Tests cover the
    post-decrement fuse gate, spawn position, zero velocity, and particle-tick
    ordering. Full local TNT physics/fuse simulation remains broader entity
    parity work.
  - [x] Animate crit TrackingEmitter particles：local vanilla 26.1
    `ClientboundAnimatePacket` defines action `4` as `CRITICAL_HIT` and action
    `5` as `MAGIC_CRITICAL_HIT`; `ClientPacketListener.handleAnimate` maps
    them to `ParticleEngine.createTrackingEmitter(entity, ParticleTypes.CRIT)`
    and `ParticleTypes.ENCHANTED_HIT`. The default `TrackingEmitter`
    constructor uses lifetime `3`, and each tick performs 16 unit-sphere
    samples around the entity's current AABB width/height. `WorldStore` now
    forwards applied animate actions `4`/`5` as semantic tracking-emitter side
    effects, and native maps them to `minecraft:crit` / `minecraft:enchanted_hit`
    delayed batches. Tests cover world forwarding, missing-entity gating, native
    particle ids, entity bounds, and the 3-tick lifetime.
  - [x] GameEvent local-player particle/audio side effects：local vanilla 26.1
    `ClientPacketListener.handleGameEvent` maps event `6`
    (`PLAY_ARROW_HIT_SOUND`) to `SoundEvents.ARROW_HIT_PLAYER` at the local
    player's eye Y, event `9` (`PUFFER_FISH_STING`) to
    `SoundEvents.PUFFER_FISH_STING` at the local player's feet, and event `10`
    (`GUARDIAN_ELDER_EFFECT`) to an `ELDER_GUARDIAN` particle at the local
    player's feet plus `SoundEvents.ELDER_GUARDIAN_CURSE` when
    `Mth.floor(param) == 1`. `WorldStore` now forwards those side effects from
    applied GameEvent packets, and native emits positioned audio plus a
    synthetic single-particle `minecraft:elder_guardian` command through the
    existing particle resolver. Tests cover pose gating, eye/feet positions,
    sound categories, param flooring, and particle packet/context plumbing.
  - [x] TakeItemEntity pickup sounds：local vanilla 26.1
    `ClientPacketListener.handleTakeItemEntity` plays
    `SoundEvents.EXPERIENCE_ORB_PICKUP` for experience orbs and
    `SoundEvents.ITEM_PICKUP` for all other picked entities, both at the picked
    entity position with `SoundSource.PLAYERS`, before shrinking/removing the
    entity. `WorldStore::apply_play_packet` now snapshots the picked entity
    transform before applying `TakeItemEntity`, records the matching positioned
    pickup sound only when the entity exists, and native dispatches it through
    the existing audio sink. Tests cover item, experience-orb, non-item entity,
    and missing-entity cases plus vanilla random pitch consumption order.


### 2026-07-05 迁入：particle-target carried submit（elder guardian / item pickup / experience orb）

- 2026-07-03 elder-guardian particle special-group model submit 已按 vanilla
  `ElderGuardianParticleGroup` 接入 particle target：`ELDER_GUARDIANS` 在
  single-quad 粒子后用 entity translucent pipeline 绘制 bind-pose elder guardian
  模型，保留 vanilla alpha、camera-relative transform、full-bright light、no overlay、
  `0.42553192` 粒子 scale 与 `2.35` elder baked-layer scale。后续同日 slice 已把
  `ItemPickupParticle` 的普通 item-stack carried model submit 接进 particle target：
  TakeItemEntity 命令携带 source entity id、`extractEntity(..., 1.0F)` 形状的
  frozen age 与 source light，renderer 导出 quadratic target 插值位置，native 复用
  dropped-item GROUND item-cluster bake 并在 `ITEM_PICKUP` group 顺序中绘制。剩余
  experience-orb carried submit 也已接入：world 从 `ExperienceOrb.DATA_VALUE`
  捕获 vanilla icon，按 `ExperienceOrbRenderer.getBlockLightLevel` 对 block
  light `+7` 封顶，renderer 用 `textures/entity/experience/experience_orb.png`
  的 16×16 icon billboard、alpha `128/255`、vanilla age 色彩曲线和
  `entityTranslucentCullItemTarget` 形状在同一 `ITEM_PICKUP` group 绘制。剩余
  carried submit 是 component-rich item stack 和更通用 `EntityRenderState`
  entity-submit parity。
- 2026-07-05 P1-5 收尾 slice：component-rich item stack 的 pickup carried bake
  已完成。被捡起的 stack 由 `ClientboundTakeItemEntity` 早已解码出的
  `DataComponentPatchSummary` 提供（不新增第二份 wire decode），native 把它序列化
  成不透明字节挂在 pickup 专用字段 `option_item_pickup_component_patch` 上；
  `ParticleItemOptionState` 保持 `Copy` 不动（level-particle 的 `minecraft:item`
  option 调用面零扰动），renderer 只把该 blob 原样过 command -> instance ->
  `ItemPickupParticleRenderState` 而从不解读它（bbb-renderer 无 bbb-protocol
  依赖）；native bake 反序列化重建 component-rich stack，复用与 dropped item
  完全相同的 GROUND 投影（`item_display_transform_for_stack` /
  `generated_item_layers_for_stack_with_registry_context`）。确定性测试断言同一
  component-rich stack（ITEM_MODEL 覆盖样本）的 pickup carried bake mesh 输出
  与 dropped-item bake 逐字节相等，并断言去掉 patch 后 mesh 变化以证明 patch
  确被消费；renderer 侧另加 patch 过 command -> instance -> render state 的
  round-trip 测试。generic `EntityRenderState` submit（捡箭/三叉戟 3-tick 闪现）
  已移居 P1-2 entity-renderer 队列（2026-07-05 已在 P1-2 完成，见
  P1-2 归档"arrow/trident pickup carried 实体模型"条目）。

### 2026-07-05 迁入：粒子 provider-specific behavior / sorting 完成史（含当时的排除式剩余清单，仅作历史存档）

- 粒子 provider-specific behavior：
  - `falling_dust` 的非 air `RenderShape.INVISIBLE` provider spawn rejection 已
    对齐 vanilla（覆盖 water/lava、bubble column、barrier、structure void、
    end portal/gateway、light、moving piston）；`FallingBlock#getDustColor`
    分支的 sand/red_sand/gravel、
    anvil、dragon_egg、concrete_powder RGB 已投影进 native spawn command 并由
    renderer visual color 消费；非 FallingBlock 的 vanilla `BlockColors`
    layer-0 tint 已覆盖常量、默认 colormap、redstone power、stem age 和 lily pad
    world-color 分支；无 BlockColors tint 的 `falling_dust` 现已用 vanilla
    static mapColor fallback 覆盖基础 stone/dirt/planks、wood/log/bamboo stem
    axis 分支、wooden stairs/slabs/pressure plates/doors/trapdoors/fences/fence
    gates/signs/hanging signs/shelves、banner/wall banner `WOOD`，以及 button /
    glass / glass pane / iron bars / iron chain / copper bars / copper chains /
    misc redstone/slime/bone/frosted-ice/dirt-path/petrified-slab static blocks /
    ladder / torch / redstone torch / soul torch / copper torch / end rod /
    rail / lever / repeater / comparator / tripwire / skull-head / potted /
    cake / air / cave_air / void_air / test_instance_block 默认
    `MapColor.NONE`、
    crimson/warped stem/hyphae 静态色、DyeColor / colored terracotta 系列、
    bed/candle/shulker decorative colored families、
    amethyst/tuff/calcite/sculk/froglight cave/emissive families、copper
    weathering families、nether flora / blackstone static families、
    quartz/prismarine/End static families、construction stone/brick static
    families、deepslate construction variants、infested stone CLAY variants、
    resin/pale garden static families、plant/dripstone/moss/root/mud natural
    static families、non-tinted foliage static families、crop/succulent static
    families、produce/fungus static families、utility/mechanical static
    families（含 stone/weighted pressure plates、utility fixtures、functional
    blocks 与 redstone utility/control blocks）、aquatic/coral static
    families、bamboo/honey/campfire utility static families、water plant/egg
    static families、flower/tall flower static families、fire/cocoa/creaking heart
    static families、glowstone/enchanting/beacon static families，以及
    ore/deepslate/nether、
    snow/ice/clay/sandstone/suspicious block、resource block、mycelium、
    packed_mud、nether_brick_fence、nether_portal、stripped_pale_oak_wood
    与 copper_lantern weathering/waxed variants；全量 mapColor catalog 现由
    `falling_dust_colors_cover_all_accepted_vanilla_block_states` 枚举覆盖。
    Biome-aware per-spawn BlockColors 现在按 vanilla provider 的
    `BlockPos.containing(x, y, z)` 采样 `LevelParticles` 实际 spawn 位置：
    terrain 粒子使用 `0.6 * colorAsTerrainParticle`，非 FallingBlock
    `falling_dust` 使用原始 tint。Particle tick 现在从 native 获得 world
    collision 回调，按 vanilla Y-first / largest-horizontal 轴顺序对
    vanilla 0.2x0.2 粒子 AABB 做三轴 block-shape clipping，应用
    `Particle.onGround` X/Z damping，并在落地后的 tick 清零
    `FallingDustParticle` roll。Plain `DripParticle.FallingParticle`
    providers (`falling_nectar` / `falling_spore_blossom`) 和
    `FallAndLandParticle` falling providers (`falling_honey` /
    `falling_obsidian_tear` / `falling_lava` / `falling_water` /
    dripstone falling variants) 现在也通过 world collision callback 的 vanilla
    `move` 路径在 `onGround` 时移除；
    `WaterDropParticle.Provider` / `SplashParticle.Provider` (`rain` /
    `splash`) 现在用同一 collision-backed `move` 路径，并在 `onGround` 时应用
    vanilla 50% 随机移除与 X/Z damping；honey / obsidian tear / lava
    landing providers 现在拆成 `DripLand` tick，使用 vanilla `DripParticle`
    move/friction 而不套用 `WaterDropParticle` 的随机落地移除；
    `WaterDropParticle` in-block removal 现在查询 world block/fluid surface，
    按 vanilla `collisionShape.max(Y, localX, localZ)` 与 fluid height 的 max
    删除落入方块/流体内部的 `rain` / `splash`；
    `WakeParticle.Provider` (`fishing`) 现在使用 vanilla
    `setSize(0.01F, 0.01F)` collision AABB，并在 0.98 friction 与 wake
    sprite cycle 前走 collision-backed `move(xd, yd, zd)`；
    DripParticle 的 water/lava matching-fluid removal 现在同样通过 world
    fluid kind/height sample 覆盖 `dripping_*`、`falling_*` 与 lava landing
    provider，`Fluids.EMPTY` 的 honey / obsidian tear / nectar / spore
    blossom providers 不参与该门；
    `BubbleParticle` / `BubbleColumnUpParticle` / `WaterCurrentDownParticle`
    现在按 vanilla `FluidTags.WATER` gate 在当前位置格子不含 water fluid
    时移除；
    BaseAshSmoke 系 provider 的 `speedUpWhenYMotionIsBlocked` 也已通过
    default tick 的 world collision callback 覆盖；
    `CampfireSmokeParticle` 现在使用 vanilla `setSize(0.25F, 0.25F)` 的
    0.25x0.25 AABB，通过 collision-backed `move` 路径移动，并在 alpha 已经
    `<= 0` 时跳过运动直接移除；
    `EndRodParticle` 保留 `hasPhysics=true` 元数据但按 vanilla 覆写走
    collision-free `move`；
    `FallingLeavesParticle` 现在通过 world collision callback 覆盖落地移除、
    第二 tick 起的水平轴阻塞移除，以及第一 tick 水平阻塞 grace；
    `FireflyParticle` 现在通过 world block-air sample 覆盖 vanilla
    `super.tick()` 后的非 air block removal，并走 collision-backed default move；
    `SquidInkParticle` / `GlowInkProvider` 现在通过同一 block-air sample 覆盖
    vanilla post-`super.tick()` in-air downward drift；
    `AshParticle` / `WhiteAshParticle` 的源码核对确认 vanilla `hasPhysics=false`，
    不存在额外 collision removal 待办；
    `DripParticle` hang-to-fall 与 fall-to-land/splash child spawning 已按
    vanilla lifetime / `onGround` 触发接到 native child templates；
    `SpellParticle` scoping alpha 现在接收 native local-player eye position /
    first-person spyglass context，覆盖近距 alpha=0 与离开后按 0.05 lerp 回
    `originalAlpha` 的 tick 行为；
    firework rocket entity event `17` 的 empty/no-explosion `createFireworks`
    分支现在按 vanilla 在 rocket 位置生成 `random.nextInt(3)+2` 个
    `minecraft:poof` 粒子，使用 gaussian X/Z `*0.05` 与固定 Y `0.005`
    速度，并保留 `ParticleTypes.POOF` 的 `overrideLimiter=true` 语义，绕过普通
    距离 / 粒子状态门；non-empty explosions 现在从 rocket item metadata
    投影 `FireworkParticles.Starter` 的 small/large ball、star、creeper、
    burst 基础 spark 形状、中心 `flash`、per-spark fade-color，以及
    trail 子 spark 复制 / twinkle 可见性 gate；同一路径现在还按 vanilla life-0
    `Starter.tick()` 播放 local ambient blast / large_blast 声效，使用
    camera distance squared `>=256` 的 far 变体、volume `20.0`、
    `0.95 + random * 0.1` pitch 和 `distanceDelay=true`；含 twinkle 的
    explosion 还会按 `explosions.size()*2 - 1 + 15` tick 延迟播放 local
    ambient twinkle / twinkle_far 声效，释放时使用当前 camera distance
    squared `>=256` 判定 far 变体、volume `20.0`、`0.9 + random * 0.15`
    pitch 和 `distanceDelay=true`；`FireworkRocketEntity.tick` 的客户端 trail
    现在每个 advanced client tick 在 rocket 当前 world transform 位置提交 1 个
    sprite-backed `minecraft:firework` 粒子，速度为 vanilla
    `random.nextGaussian()*0.05` X/Z 与 `-deltaMovement.y*0.5` Y；
    `PlayerCloudParticle.Provider` / `SneezeProvider` 现在接收 native
    local-player position / delta-movement context，覆盖 vanilla
    `super.tick()` 后 2 格内、粒子高于玩家脚部 Y 时对 Y 与 Y 速度的 20% 牵引；
    entity event `35` 的 totem `TrackingEmitter` 现在按 vanilla
    构造时立即 tick、总计 30 tick，每 tick 做 16 次单位球采样，并按实体当前
    AABB width/height 在实体周围生成 `minecraft:totem_of_undying` 粒子，使用
    velocity `(xa, ya + 0.2, za)` 与 delay `0..29` 提交给 renderer；
    `ClientboundAnimate` 动作 `4`/`5` 现在按 vanilla
    `ClientPacketListener.handleAnimate` 生成 crit / enchanted-hit
    `TrackingEmitter`，默认 3 tick，每 tick 16 次单位球采样，并复用实体当前
    AABB width/height；
    ravager entity event `69` 现在按 vanilla `Ravager.handleEntityEvent`
    的 `addRoarParticleEffects` 生成 40 个 `minecraft:poof`：位置为 ravager
    AABB center，速度为 vanilla gaussian `* 0.2` 三轴采样，并保留
    `ParticleTypes.POOF` 的 `overrideLimiter=true`；同事件现在还按
    `Ravager.applyRoarKnockbackClient` 对本地 authoritative、alive、非 ravager
    living target（当前 native world 建模为本地玩家）在 ravager AABB
    `inflate(4.0)` 命中时追加 `strongKnockback`：`dd=max(xd²+zd²,0.001)`，
    delta 加 `(xd/dd*4.0, 0.2, zd/dd*4.0)`；
    witch entity event `15` 现在按 vanilla `Witch.handleEntityEvent` 生成
    `nextInt(35)+10` 个 `minecraft:witch` 粒子：位置为实体 `x/z` 与
    `boundingBox.maxY + 0.5` 加三轴 gaussian `*0.13F`，速度为 0；
    LivingEntity entity event `60` 现在按 vanilla `LivingEntity.makePoofParticles`
    生成 20 个 `minecraft:poof` 粒子：速度为三轴 gaussian `*0.02`，位置为
    `getRandomX(1.0) - vx*10`、`getRandomY() - vy*10`、
    `getRandomZ(1.0) - vz*10`，并使用当前 living entity AABB width/height；
    LivingEntity entity event `67` 现在按 vanilla `LivingEntity.makeDrownParticles`
    生成 8 个 `minecraft:bubble` 粒子：位置为实体 position 加三轴
    `random.triangle(0.0, 1.0)` 偏移，速度为当前 `deltaMovement`；
    LivingEntity entity event `46` 现在按 vanilla `LivingEntity.handleEntityEvent`
    生成 128 个 `minecraft:portal` 粒子：位置沿 `xo/yo/zo` 到当前 position
    插值并加入 width/height 随机偏移，速度为 `(nextFloat()-0.5)*0.2`；
    Snowball entity event `3` 现在按 vanilla `Snowball.handleEntityEvent`
    生成 8 个命中粒子：默认 synced item stack 走 `minecraft:item` + snowball
    stack，显式 empty stack 走 `minecraft:item_snowball`，位置为当前 snowball
    position，速度为 0；
    ThrownEgg entity event `3` 现在按 vanilla `ThrownEgg.handleEntityEvent`
    为非空 item stack 生成 8 个 `minecraft:item` 粒子：缺失 metadata 时使用
    默认 `Items.EGG`，显式 empty stack 不生成粒子，位置为当前 egg position，
    三轴速度为 `(nextFloat()-0.5)*0.08`；
    Arrow entity event `0` 现在按 vanilla `Arrow.handleEntityEvent` 在
    synced `ID_EFFECT_COLOR != -1` 时生成 20 个 `minecraft:entity_effect`
    粒子：位置按当前 arrow AABB 的 `getRandomX(0.5)` / `getRandomY()` /
    `getRandomZ(0.5)` 采样，速度为 0，option RGB 来自 synced color；`-1`
    不生成粒子，`0` 生成黑色粒子；
    Animal entity event `18` 现在按 vanilla `Animal.handleEntityEvent`
    生成 7 个 `minecraft:heart` love 粒子：速度为三轴 gaussian `*0.02`，
    位置按当前 animal AABB 的 `getRandomX(1.0)` / `getRandomY()+0.5` /
    `getRandomZ(1.0)` 采样；Allay entity event `18` 现在也按 vanilla
    `Allay.handleEntityEvent` 生成 3 个复制反馈 `minecraft:heart` 粒子，使用
    同一 `spawnHeartParticle` 速度和位置公式；TamableAnimal 与
    AbstractHorse entity events `6`/`7` 现在按 vanilla `spawnTamingParticles`
    生成驯服反馈粒子：event `7` 成功为 7 个 `minecraft:heart`，event `6`
    失败为 7 个 `minecraft:smoke`，两者使用同一三轴 gaussian `*0.02`
    速度和当前 AABB 位置采样；Villager entity events `12`/`13`/`14`/`42`
    现在按 vanilla `Villager.handleEntityEvent` /
    `AbstractVillager.addParticlesAroundSelf` 生成 5 个粒子，速度为三轴
    gaussian `*0.02`，位置按当前 villager AABB 的 `getRandomX(1.0)` /
    `getRandomY()+1.0` / `getRandomZ(1.0)` 采样，四个事件分别映射为
    `minecraft:heart` / `minecraft:angry_villager` /
    `minecraft:happy_villager` / `minecraft:splash`；Dolphin entity event
    `38` 现在按 vanilla `Dolphin.handleEntityEvent` 生成 7 个
    `minecraft:happy_villager` 粒子：速度为三轴 gaussian `*0.01`，位置按
    当前 dolphin AABB 的 `getRandomX(1.0)` / `getRandomY()+0.2` /
    `getRandomZ(1.0)` 采样；
    Fox entity event `45` 现在按 vanilla `Fox.handleEntityEvent` 在主手非空时
    生成 8 个 `minecraft:item` 粒子：位置为 `position + getLookAngle()/2`
    的 x/z mouth anchor，item option 使用主手 stack，局部速度
    `(rand-.5)*0.1, rand*0.1+0.1, 0` 先 `xRot(-xRot)` 再
    `yRot(-yRot)`，并追加 `+0.05` Y；
    HoneyBlock entity events `53`/`54` 现在按 vanilla `HoneyBlock.showParticles`
    生成 `minecraft:block` 粒子：event `53` 基础 Entity slide 生成 5 个，
    event `54` LivingEntity jump 生成 10 个，均使用
    `Blocks.HONEY_BLOCK.defaultBlockState()`、实体当前位置和零速度；
    Ravager stun client tick 现在按 vanilla `Ravager.aiStep` /
    `stunEffect` 在 event `39` 的 40 tick stun 期间每 tick 消耗 Java-LCG
    client RNG 执行 `nextInt(6)`，命中时在头部 anchor
    `position - width*sin(yBodyRot)` / `position.y + height - 0.3` /
    `position + width*cos(yBodyRot)` 加 `±0.3` x/z jitter 生成灰色
    `minecraft:entity_effect`；EvokerFangs event `4` 启动 `lifeTicks`
    后，tick 到 `14` 时按 vanilla 一次性生成 12 个 `minecraft:crit`
    粒子，使用 `width*0.5` 水平范围、`1.05 + random` Y 偏移和
    `0.3..0.6` Y 速度，并在 renderer `ParticleEngine.tick` 前提交，同一
    event 现在还按 vanilla 在 fang 当前位置播放
    `minecraft:entity.evoker_fangs.attack` positioned sound，source 为默认
    neutral、volume `1.0`、pitch `random.nextFloat()*0.2+0.85`；
    `ClientboundTakeItemEntity` 现在按 vanilla 在 shrink/remove 前创建
    `ItemPickupParticle` runtime command：source 使用被拾取实体当前位置/速度，
    target 使用目标 living entity 或本地玩家 fallback 的 `(feet + eyeY) / 2`
    midpoint，item entity 传入 pre-shrink item stack；renderer 将其纳入
    `ITEM_PICKUP` group，按 3 tick lifetime、target old/current 跟随和
    `(life + partial) / 3` 平方插值推进。普通 item-stack carried model 现在还按
    vanilla particle group 顺序在 single-quad 粒子后、elder guardian special
    group 前绘制：source item render state 冻结 age/light，position 使用 quadratic
    target 插值，GROUND item model/cluster 复用 dropped-item bake；experience
    orb carried submit 现在捕获 `ExperienceOrbRenderState.icon`、经验球专用
    `+7` block light、冻结 age，并用经验球 64×64 texture 的 16×16 icon
    billboard 通过 entity translucent-cull item-target 形状绘制。剩余 carried
    submit 是 component-rich item stack 与更通用 `EntityRenderState` GPU
    entity-submit parity；
    `ClientboundGameEvent` 的 elder-guardian effect 现在按 vanilla 在本地玩家
    脚部位置生成 `minecraft:elder_guardian` 粒子，并在 param floor 为 1 时播放
    `minecraft:entity.elder_guardian.curse`；同组 game event 的
    `minecraft:entity.arrow.hit_player` 与 `minecraft:entity.puffer_fish.sting`
    本地玩家位置声效也已接到 native audio；
    同一 event `35` 现在还按 vanilla `SoundEvents.TOTEM_USE` 在实体当前位置
    播放 `minecraft:item.totem.use` 本地位置声效，source 来自当前实体的
    `getSoundSource()` 映射（player/hostile/default neutral 等）；
    Ravager / IronGolem entity event `4` 现在按 vanilla 固定
    `playSound(sound, 1.0, 1.0)` side effect 在当前实体位置播放
    `minecraft:entity.ravager.attack` / `minecraft:entity.iron_golem.attack`，
    遵守 silent gate 和 `getSoundSource()` 的 hostile / neutral 映射；
    ZombieVillager entity event `16` 现在按 vanilla 在 `getEyeY()` 位置播放
    `minecraft:entity.zombie_villager.cure`，遵守 silent gate，source 为
    hostile，volume `1.0 + random.nextFloat()`，pitch
    `random.nextFloat()*0.7+0.3`；
    Armadillo entity event `64` 现在按 vanilla `Armadillo.handleEntityEvent`
    在实体当前 `getX/getY/getZ` 位置播放 `minecraft:entity.armadillo.peek`，
    source 为 neutral，volume/pitch 固定 `1.0`，并与该 vanilla 分支一致不走
    generic silent gate；
    ArmorStand entity event `32` 现在按 vanilla 命中分支在实体当前位置播放
    `minecraft:entity.armor_stand.hit`，source 为 neutral，volume `0.3`，
    pitch `1.0`，并与既有 hit-wiggle 状态投影一起应用；
    ArmorStand LivingEntity death event `3` 现在按 vanilla `getDeathSound()`
    播放 `minecraft:entity.armor_stand.break`，遵守 generic silent gate，
    volume `1.0`，pitch 为 `(random.nextFloat()-random.nextFloat())*0.2+1.0`；
    Zombie LivingEntity death event `3` 现在按 vanilla `Zombie.getDeathSound()`
    播放 `minecraft:entity.zombie.death`，source 为 hostile，遵守 generic
    silent gate，并使用相同 death-event pitch 随机公式；
    ZombieVillager LivingEntity death event `3` 现在按 vanilla
    `ZombieVillager.getDeathSound()` 播放
    `minecraft:entity.zombie_villager.death`，沿用 hostile source、generic
    silent gate 与 death-event pitch 随机公式；
    Ravager / IronGolem LivingEntity death event `3` 现在按 vanilla
    `getDeathSound()` 播放 `minecraft:entity.ravager.death` /
    `minecraft:entity.iron_golem.death`，沿用 hostile / neutral source 映射、
    generic silent gate 与 death-event pitch 随机公式；
    Witch / Villager LivingEntity death event `3` 现在按 vanilla
    `Witch.getDeathSound()` / `Villager.getDeathSound()` 播放
    `minecraft:entity.witch.death` / `minecraft:entity.villager.death`，
    沿用 hostile / neutral source 映射、generic silent gate 与同一
    death-event pitch 随机公式；
    Skeleton / Stray / Bogged LivingEntity death event `3` 现在按 vanilla
    `getDeathSound()` 播放 `minecraft:entity.skeleton.death` /
    `minecraft:entity.stray.death` / `minecraft:entity.bogged.death`，
    沿用 hostile source、generic silent gate 与同一 death-event pitch
    随机公式；
    `vibration` entity `PositionSource` 现在保留 entity id / yOffset，并在
    native level-particle command resolution 用当前 world entity transform
    生成 `entity.position + (0, yOffset, 0)` 初始 target；renderer particle
    tick 现在接收 native entity target context，每 tick 重算 entity target 并在
    source entity 缺失时移除；
    `DragonBreathParticle` 现在使用 vanilla 专用 tick motion：Y 未移动时先让
    X/Z 速度 `*1.1` 再施加 friction，且 Y 速度只在 `onGround` 设置持久
    `hasHitGround` 后追加 `0.002` 上升并参与摩擦；
    `SuspendedTownParticle` 系 provider（happy_villager / composter /
    dolphin / mycelium / egg_crack）现在表达 vanilla 覆写的 collision-free
    `move`；
    `CritParticle` 系 provider 现在覆盖 vanilla 构造函数尾部立即 `tick()` 的
    spawn-time age/position/velocity 推进；
    `FlameParticle` 与 `PortalParticle` / `ReversePortalParticle` 现在保留
    vanilla `hasPhysics=true` metadata，并通过 collision-free `move` flag 表达其
    覆写路径；
    `PrimedTnt.tick` 的客户端 smoke side effect 现在按当前 world 实体位置和
    synced/default fuse 元数据提交：entity tick clock 推进后，post-decrement fuse
    仍为正的 TNT 每个 advanced tick 在 `x, y + 0.5, z` 生成一个
    `minecraft:smoke`，速度为 0；
    vault activation event-3015 现在解析 vault block-entity
    `shared_data.connected_players` / `connected_particles_range`，按方块
    `facing` 的 keyhole 位置向 in-range loaded player 生成
    `minecraft:vault_connection` 粒子，并在 cage 粒子和 activate sound 前保持
    vanilla 随机消费顺序；
    `OminousItemSpawner.tickClient` 现在按 vanilla `level.gameTime % 5 == 0`
    gate 在实体当前位置提交 `minecraft:ominous_spawning` 粒子：每次触发生成
    `random.nextIntBetweenInclusive(1,3)` 个命令，速度为
    `0.4*(gaussian-gaussian)` 三轴随机 offset 向量，并保留该 particle type 的
    override-limiter 语义；
    剩余 gravity/collision/player-coupled work 是其他特殊 context 和
    player-coupled emitter（不含 TakeItemEntity `ItemPickupParticle` runtime/lifecycle、SpellParticle、本地 PlayerCloud 牵引、
    totem event-35 TrackingEmitter、animate 4/5 crit/enchanted-hit TrackingEmitter、
    ravager event-69 roar poof/knockback、GameEvent elder-guardian 粒子与
    `ELDER_GUARDIANS` model submit、vibration entity target refresh、DragonBreath hit-ground motion 与 SuspendedTown
    collision-free move、Crit constructor tick、Flame/Portal collision-free metadata、PrimedTnt smoke、Witch event-15 magic burst、LivingEntity event-60 poof burst、LivingEntity event-67 drown bubbles、LivingEntity event-46 portal burst、Snowball event-3 item burst、ThrownEgg event-3 item burst、Arrow event-0 entity-effect burst、Fox event-45 item burst、HoneyBlock event-53/54 block particles、OminousItemSpawner tickClient ominous_spawning 粒子），以及 local sound（不含 DripParticle
    honey/dripstone fall-and-land 落地本地声效、totem event-35
    `minecraft:item.totem.use` 本地位置声效、GameEvent arrow-hit / puffer-fish-sting /
    elder-guardian-curse 本地玩家位置声效、TakeItemEntity item / experience-orb
    pickup 本地位置声效、Ravager/IronGolem event-4 fixed attack positioned sounds）
    / block-state removal gates。
  - `TerrainParticle.createTerrainParticle` 的 air / `moving_piston` /
    `shouldSpawnTerrainParticles=false` provider rejection 已覆盖 `block`、
    `dust_pillar`、`block_crumble`；`block_marker` 保持 vanilla 未过滤分支。
  - 初速度。**已收敛**：smoke 系、ash / white_ash、dust_plume、trial_spawner_detection /
    _ominous 的 base-spread×dir 初速度均已对齐 vanilla（见 goal-archive P1-5）。剩余
    仍用纯 `Command` 初速度的 provider（fishing、bubble_pop、squid_ink、glow_squid_ink、
    enchant、nautilus、totem_of_undying、end_rod、sculk_charge、firework、portal、
    reverse_portal 等）经逐个 vanilla-provider 审计确认本就是把 aux 速度直传 base
    `Particle` ctor，flat `Command` 正确，无 gap。初速度这一档不再有可执行小 slice。
  - alpha/color curve。**已收敛**：renderer 现在用显式 alpha/color
    descriptors 覆盖 `SimpleAnimatedParticle` 半生命周期 fade、firework
    spark 初始 `0.99` fade、firework flash extract-time overlay alpha、shriek
    extract-time linear fade、vault connection `LifetimeAlpha`、firefly
    `getFadeAmount`、EndRod fade color、dust transition lerp，以及 option /
    random / fixed provider tints，以及 terrain particle 的 vanilla `BlockColors`
    layer-0 tint，以及 `falling_dust` 的基础 static mapColor fallback（含
    wood/log/stem、wooden stairs/slabs/pressure plates/doors/trapdoors/fences/
    fence gates/signs/hanging signs/shelves、banner/wall banner `WOOD`，以及 button /
    glass / glass pane / iron bars / iron chain / copper bars / copper chains /
    misc redstone/slime/bone/frosted-ice/dirt-path/petrified-slab static blocks /
    ladder / torch / redstone torch / soul torch / copper torch / end rod /
    rail / lever / repeater / comparator / tripwire / skull-head / potted /
    cake / air / cave_air / void_air / test_instance_block 默认
    `MapColor.NONE`、colored block families、
    decorative colored families、
    cave/emissive block families、copper weathering families、nether flora /
    blackstone static families、quartz/prismarine/End static families、
    construction stone/brick static families、deepslate construction variants、
    infested stone CLAY variants、resin/pale garden static families、
    plant/dripstone/moss/root/mud natural static families、non-tinted foliage
    static families、crop/succulent static families、produce/fungus static
    families、ore/deepslate/nether 与 utility/mechanical（含 stone/weighted
    pressure plates、utility fixtures、functional blocks 与 redstone utility/
    control blocks）、aquatic/coral static families、bamboo/honey/campfire utility
    static families、water plant/egg static families、flower/tall flower static
    families、fire/cocoa/creaking heart static families、glowstone/enchanting/
    beacon static families、矿物/自然 static block families，以及 final accepted
    vanilla static states）。全量 mapColor catalog 与 biome-aware per-spawn
    BlockColors 已收敛；firework 非空 explosions 的基础 `Starter`
    spark/flash 与 per-spark fade-color 已由 firework event path 覆盖；
    trail 子 spark 复制、twinkle 可见性 gate、life-0 blast 音效、twinkle
    移除延迟音频，以及 FireworkRocketEntity 客户端 tick trail 粒子也已覆盖。
  - gravity / collision / player-coupled physics。
- 粒子 sorting：
  - terrain/item particle atlas rendering：on-ground roll reset 和三轴
    block-shape collision clipping 已通过 native world collision 回调接入；
    EndRod collision-free move 与 WakeParticle (`fishing`) collision-backed
    `move` 已覆盖，其他 special-context collision / player-coupled physics
    仍属上一节 deferred work。
    Renderer GPU draw ranges now bind particle / terrain / item atlas textures
    once concrete sprite UVs are available; native terrain atlas upload supplies
    block sprite UVs and native item atlas upload supplies item sprite UVs to
    the particle renderer path. `TerrainParticle` providers now resolve
    block-state particle material sprite ids for `minecraft:block`,
    `minecraft:block_marker`, `minecraft:dust_pillar`, and
    `minecraft:block_crumble`. Fixed item providers now resolve
    `minecraft:item_slime`, `minecraft:item_cobweb`, and
    `minecraft:item_snowball` to their vanilla item atlas sprite ids. Generic
    `minecraft:item` particles now decode the `ItemStackTemplate`
    `DataComponentPatch` into the protocol component summary and resolve the
    concrete GROUND item particle material active-layer sprite ids through the
    native item runtime, including component-driven root item-model changes.
    LevelEvent `addDestroyBlockEffect` / brush-complete block particles now use
    vanilla `TerrainParticle(..., blockState, pos)` tint semantics by sampling
    `0.6 * BlockTintSource.colorAsTerrainParticle` at the event block position
    biome.
    LevelEvent splash-potion / ender-eye item-break particles now reuse the
    installed empty-component item material sprite ids for their first eight
    `minecraft:item` commands.
    Terrain/item particle sprite metadata now carries atlas
    `hasTranslucent`, and renderer vertex batching mirrors
    `SingleQuadParticle.Layer.bySprite` by routing current terrain/item sprites
    to `TRANSLUCENT_TERRAIN` / `TRANSLUCENT_ITEMS` when their uploaded sprite
    has translucent pixels.
    Terrain and non-FallingBlock `falling_dust` BlockColors now sample the
    actual spawn block position's biome through the world probe path before
    native emits `ParticleSpawnCommand.option_color`.

- 逐 provider 三态追踪表：
  - [x] 2026-07-05 建表完成：
    docs/unsupported/particle-runtime-vanilla-parity.md 新增
    `## Per-provider tracking table (established 2026-07-05)`，以 vanilla
    26.1 `ParticleResources.registerProviders()` 的全部 110 个 provider 类
    加 3 个代码路径粒子（`TrackingEmitter` / `FireworkParticles.Starter` /
    `ItemPickupParticle`）为行（113 行），逐 provider 对照 vanilla 源码判定
    special-context collision / player-coupled physics / local sounds /
    block-state removal gates 四维的 covered / not-needed / todo 三态。
    结果：30 个 todo 单元格（28 collision + 2 player-coupled；sounds 与
    removal-gates 无 todo），共享根因归为 `[bounds]` / `[leaf-bounds]` /
    `[wake-grow]` / `[nearest-player]` 四组。账本 particle 条目 Next action
    改为 work the todo rows；后续 P1-5 slice 从表中取 todo 行。
  - [x] 2026-07-05 `[bounds]` slice 完成：`collision_size()`
    (crates/bbb-renderer/src/particles/descriptors.rs) 把 24 个 `[bounds]`
    todo provider 的静态碰撞箱尺寸从默认 0.2x0.2 修正为 vanilla `setSize` 值——
    Drip 全家 17 个 0.01（DripParticle.java:25）、rain/splash 0.01
    （WaterDropParticle.java:16，Splash 经 super 继承）、bubble/bubble_column
    0.02（BubbleParticle.java:22 / BubbleColumnUpParticle.java:24）、soul x2 与
    firefly 0.3（`scale(1.5F)` → `Particle.scale` `setSize(0.2F*1.5F)`=0.3，
    Particle.java:77-80 / SoulParticle.java:17 / FireflyParticle.java:94），
    同族共享 match arm，每档带 vanilla 文件:行号注释。这些尺寸经
    instance.rs `move_particle` 的 `ParticleCollisionQuery{half_width,height}`
    喂给 collision-backed move 路径（非死数据）。新增 focused 测试
    `collision_size_matches_vanilla_provider_set_size` 覆盖每档尺寸 + campfire/
    wake 不回归。追踪表 24 行 collision `todo`→`covered`（30→6 todo：剩 3
    `[leaf-bounds]` + 1 `[wake-grow]` + 2 `[nearest-player]`）。
  - [x] 2026-07-05 动态碰撞尺寸 slice 完成（清 `[leaf-bounds]` + `[wake-grow]`）：
    把碰撞尺寸从纯静态查表扩展为支持 per-spawn / per-tick 两种动态形态，机制改动
    集中在 instance.rs（未重构 descriptor 体系）。per-spawn：FallingLeaves
    Cherry/PaleOak/Tinted 三 provider 的 `setSize(size,size)`（`size = scale *
    (nextBoolean ? 0.05F : 0.075F)`，FallingLeavesParticle.java:41-43）在
    `from_spawn_command_*` 直接复用已采样的 `visual.base_quad_size`（= vanilla
    `size`）写入 `collision_width/height`，不新增随机抽取，spawn RNG 序不变。
    per-tick：Wake tick arm 在 `move` 之后按 `life * 0.001`（`life = 60 -
    (lifetime_ticks - age_ticks)`，WakeParticle.java:46-47）更新碰撞箱，增长滞后
    于 move——本 tick 的 move 用上一步尺寸，与 vanilla move-then-setSize 顺序一致；
    初始 0.01 仍由 `collision_size()` 提供。新增 3 个确定性测试：
    `particle_runtime_falling_leaves_collision_size_matches_per_spawn_quad_size`
    （固定 seed 断言 collision==base_quad_size 且落在两档 vanilla 尺寸、两档都出现）、
    `particle_runtime_wake_grows_collision_size_each_tick`（断言 tick N 尺寸序列
    = life*0.001）、`particle_runtime_wake_move_uses_previous_tick_grown_size`
    （断言 move 用滞后一 tick 的尺寸：tick1 half_width 0.005 → tick2 0.010）；既有
    collision 测试不回归。追踪表最后 4 个 collision `todo`→`covered`（6→2 todo：
    仅剩 2 `[nearest-player]` player-coupled）。
  - [x] 2026-07-05 `[nearest-player]` slice 完成（清最后 2 个 todo，追踪表归零）：
    PlayerCloudParticle.Provider / SneezeProvider 的玩家牵引从"仅本地玩家"泛化为
    vanilla `level.getNearestPlayer(x, y, z, 2.0, false)`
    （PlayerCloudParticle.java:51-58；EntityGetter.java:74-88、95-98——
    `filterOutCreative=false` → `EntitySelector.NO_SPECTATORS`，只排 spectator
    不排 creative，严格 `dist < range*range` 取最小平方距离）。方案：per-particle
    最近选择只能在知道粒子位置的 renderer 侧做，故沿 entity_target_contexts
    的切片管道形状——native `particle_player_motion_contexts` 每 tick 投影候选
    玩家列表（本地玩家 pose + `entity_transforms` 中 VANILLA_ENTITY_TYPE_PLAYER_ID
    远程玩家，双向过滤 spectator：本地 `local_player_is_spectator()`、远程
    player_info gamemode），renderer `update_player_cloud_motion` 逐粒子解析
    最近候选再做 y/yd 牵引；远程玩家 delta_movement 取自实体 transform（牵引
    公式读 `player.getY()` + `getDeltaMovement().y`）。顺带把旧实现 `> 4.0` 的
    边界修正为 vanilla 严格 `< 4.0`（dist²==4.0 不再牵引）。
    `ParticleLocalPlayerMotionContext` 更名 `ParticlePlayerMotionContext`，
    advance 链 Option→slice。新增确定性测试：renderer
    `particle_runtime_player_cloud_pulls_toward_nearest_player_candidate`
    （首候选更近 / 次候选更近（sneeze 路径）/ 全部超 2.0 含 dist²==4.0 边界
    三场景断言牵引目标）+ native
    `particle_player_motion_contexts_track_local_and_remote_players`
    （本地+远程候选投影、非玩家实体排除、远程/本地 spectator 排除）。追踪表
    2 个 player-coupled `todo`→`covered`（2→0：全表无 open todo，表头使用
    说明改为"新增 provider 行为缺口先加行/立 todo 再切 slice"）；主账本
    Particle Runtime 条目 Next action 首条同步。

- 透明排序审计 + 跨 section 段间序修复：
  - [x] 2026-07-05 P1-5 最后一个透明排序 slice 收口。先做全链审计：段内
    quad 序（`MeshData.sortQuads` centroid 距离序 + camera resort 重写
    index buffer）、terrain→feature 合成序、粒子排序、within-target draw
    序均逐一对照 vanilla 26.1 复核一致，唯独 translucent terrain 的**跨
    section 段间序**有差异——旧实现 `translucent_target_pass` 按 Vec 存储序
    绘制，`resort_translucent_terrain_for_camera` 只重写段内 index buffer。
    vanilla `ChunkSectionsToRender.renderGroup` 按 `visibleSections` 近→远
    BFS 累积各层 draw（`LevelRenderer.java:1063-1134`），但对 TRANSLUCENT
    层单独 `draws = draws.reversed()`（`ChunkSectionsToRender.java:55-56`），
    即远→近 back-to-front 且每帧随相机更新。修复：为 translucent 层维护
    `terrain_translucent_order`——按每 section 包围盒中心到
    `camera_sort_position()` 距离降序（远→近，等距按 section index 升序稳定）
    排出的绘制序；`translucent_target_pass` 迭代该序。序在每次相机变化
    （挂进 `resort_translucent_terrain_for_camera`，与段内 resort 同坐标基准）
    与每次 mesh 上传（`upload_terrain_mesh_layers` 全量重建，唯一增删路径）
    重建，保证始终是 `terrain_translucent` 的有效相机相关排列；缺 bounds 的
    退化 section 沉到末尾。确定性单测覆盖已知包围盒远→近序、相机移动后重排、
    等距稳定序、boundless 沉底四场景，render.rs source-order 断言 pin 绘制
    循环走排序后的段序。主账本 Renderer Scene Parity 条目 Evidence 与
    per-slice history 同步。

## P2：Terrain / Block Render Presentation

- [x] fluid water overlay side texture：renderer terrain cells now carry the
  water overlay sprite index and a native-projected overlay-neighbor flag.
  Water side faces use `minecraft:block/water_overlay` and suppress the
  reversed side back face when the adjacent vanilla block is a
  `HalfTransparentBlock` or `LeavesBlock`, matching
  `FluidRenderer.tesselate` / `FluidStateModelSet.WATER_MODEL`. Native marks
  glass, tinted/stained glass, ice/frosted ice/blue ice, slime, honey,
  copper grates, and `_leaves` blocks while leaving panes, packed ice, stone,
  and lava outside the overlay path.
- [x] datapack dimension `cardinal_light`：`WorldLevelInfo` now stores resolved
  `WorldCardinalLighting`, with login/respawn resolving
  `minecraft:dimension_type` registry entry raw NBT at the spawn
  `dimension_type_id`. The parser reads the root compound's `cardinal_light`
  string (`default` / `nether`) and treats an omitted field as `default`,
  matching vanilla `DimensionType`'s
  `CardinalLighting.Type.CODEC.optionalFieldOf("cardinal_light",
  CardinalLighting.Type.DEFAULT)`. Missing registry data still falls back to
  the built-in 26.1 dimension profiles.
- [x] block-destroy local/server cube overlay merge：native render extraction
  now has focused coverage for the vanilla-shaped shared progress channel:
  local destroy stages and server `BlockDestruction` progress targeting the
  same position merge before atlas lookup, keep the highest stage, and resolve
  that stage through the official `destroy_stage_0..9` block atlas entries.
  The vanilla basis is `MultiPlayerGameMode` writing local stages through
  `ClientLevel.destroyBlockProgress`, `LevelRenderer.destroyBlockProgress`
  storing local/server updates in one per-position sorted set, and
  `extractBlockDestroyAnimation` extracting the sorted set's highest progress.
- [x] biome 颜色混合半径（2026-07-05，P2 terrain 首片）：terrain 的 grass/
  foliage/dry-foliage/water tint 由 per-cell 单 biome 查表改为 vanilla
  `ClientLevel.calculateBlockTint` 的 `biomeBlendRadius` 邻域平均（硬编码
  vanilla `Options.java` 默认 `IntRange(0,7)` 默认值 2 → 5×5=25 列）。逐条
  复核 vanilla 语义：`Cursor3D(x-r,y,z-r … x+r,y,z+r)` 仅在 x/z 平面、y 固定
  `pos.getY()`；`totalRed/count`（`count=(2r+1)^2`）为 per-channel 整数算术
  平均（截断）；modifier per-sample——`GRASS_COLOR_RESOLVER=Biome::getGrassColor`
  在 resolver 内用该样本 x/z 应用 swamp/dark_forest modifier，然后才平均
  （`BiomeColors.java`/`Biome.getGrassColor`）。vanilla 26.1 有 4 个 resolver
  （grass/foliage/**dry_foliage**/water 全部走 `getBlockTint` 平均），已全部
  接入；spruce/birch 叶是常量（不走 resolver）故不混合。跨 chunk：新增
  `WorldStore::chunk_biome_sampler` 每次 convert 预解析 3×3 邻 chunk 列
  （半径 <16 只可能触达相邻 chunk），边界列取真实邻 chunk biome；邻 chunk 未
  加载的列如实从均值中剔除（render-distance 边缘窗口截断、按可得样本数
  除，不造假数据）。性能：邻域平均仅在 chunk convert（非每帧）、且仅对 biome-
  resolver 方块（grass/foliage/water，跳过 stone/dirt/air 内部）建 5×5 窗口，
  单元格 25 次 O(1) 采样。block-break 粒子仍取中心单 biome（该处 blend 暂缓）。
  测试：`terrain_runtime/textures/tests.rs`（uniform=无变化不回归 / 两 biome
  边界精确算术平均 / swamp modifier per-sample 先于平均 / 未加载列截断按可
  得数平均）+ bbb-world `chunk_biome_sampler_reads_neighbourhood_and_
  truncates_unloaded_columns`。
- [x] 破坏 crack decal 跟随方块 render shape（2026-07-06）：crumbling overlay
  从恒定单位立方改为跟随方块真实 render shape。`BlockDestroyOverlay` 新增
  `shape: TerrainRenderShape`，由 `runtime.rs::block_destroy_render_shape` 经
  `TerrainTextureState::block_render_shape`（薄封装 chunk mesher 的
  `block_render_data`，position 喂 model-variant seed 与所绘 chunk 一致）投影；
  chunk 未加载则退化整立方。`block_destroy.rs` 复用 mesher 自身
  `box_face_corners`/`FACES`/`CROSS_FACES`（提升 `pub(crate)`）+ `[0,1,2,0,2,3]`
  绕序生成面——与 terrain 方块面同一 inward-RHR 绕序（`terrain/mesh/emitter.rs`
  fluid 背面注释为外侧可见绕序的 ground truth），故 decal 仅出现在方块面可见
  的那一侧；顺带纠正旧 overlay 用了相反（outward-RHR）绕序。覆盖 Cube / Box /
  Boxes（slab/stairs/fence/wall）/ Cross·Crosses（两斜面）。UV 按 vanilla
  `SheetedDecalTextureGenerator`（`BlockFeatureRenderer` 把方块模型自身 quad 以
  `textureScale=1.0` 喂 crumbling buffer）：block-local 顶点位置投影到面最近
  `Direction` 的两个垂直轴（down `[px,1-pz]` / up `[px,pz]` / south `[px,1-py]`
  …），故半高 box 只采样 sprite 的覆盖切片（底 slab 侧面显下半）。退化如实：
  `Quads` 退化整立方（无 crumbling 友好 box 分解）；cross 用全平面 decal（mesher
  固定 `[0,1]` cross 平面恒占满 sprite）。z-fight 机制不变（逐顶点法向外推 +
  crumbling pipeline depth bias）。测试：`block_destroy.rs`（slab 半高侧面 + 部分
  sprite 切片 / 多 box stairs 面数 / cross 两平面 / 手算 decal UV→mesh 顶点 /
  Cube 不回归 / `Quads`→cube 退化 / 与 terrain 一致绕序）+ 原生
  `block_destroy_overlays_merge_local_stage...` 断言 shape 字段。
- [x] per-face 遮挡形状 culling 精度（slab/stairs 满面先行，2026-07-06）：
  相邻面剔除从 cell 级布尔（任一 opaque 且有几何的邻居剔全部相邻面）升级为
  vanilla `Block.shouldRenderFace`（`Block.java:304`）的按方向遮挡形状判定。
  复核 vanilla 精确语义：`shouldRenderFace` 取
  `neighbor.getFaceOcclusionShape(dir.opposite())` 为 occluder——full block
  短路剔（`Block.java:306`）、`skipRendering` 剔（310）、occluder 空则渲染
  （314）、自身面 `state.getFaceOcclusionShape(dir)` 空则渲染（319），否则
  `Shapes.joinIsNotEmpty(自身面, occluder, ONLY_FIRST)`（渲染 = 自身面有未被
  occluder 覆盖的部分）。`getFaceOcclusionShape` = `occlusionShape.getFaceShape`
  （`BlockBehaviour.java:512-522`，per-state per-face 缓存），默认 occlusionShape
  = `getShape`。在 bbb「满面 only」遮挡模型下这套语义坍缩为单向邻居判定：
  occluder 满面 → 覆盖任意自身面 → 剔；occluder 非满面 → 保守渲染（自身面空/
  partial-join 的 vanilla 渲染分支只会「更少剔」，忽略它安全）。新增纯函数
  `face_occludes(shape, direction)`（`terrain/mesh.rs`）从 render cuboid 推导——
  `Cube` 六向满；`Cross`/`Crosses`/`Quads` 无遮挡（vanilla foliage/custom 模型
  occlusion shape 空）；`Box` 单 cuboid 贴边且横截面覆盖 16×16 即满面；`Boxes`
  先走单 cuboid 快路径，否则把每个贴边 cuboid 的横截面栅格化到 16×16 边界网格
  取并集判满（楼梯满背面靠两个 box 并集才满，单 box 都不满——与 vanilla
  `Block.isFaceFull(getFaceShape)` 对 box 并集精确一致，且恒为 vanilla 遮挡的子
  集，绝不多剔）。`culls_face_between_cells` 新增 `direction` 参数，四个调用点
  （Cube 面循环、`emit_box`、`box_face_will_render`、`emit_quads`）传各自的 cull
  方向，检查 `face_occludes(neighbor.render_shape, direction.opposite())`；材质门
  仍是既有 `Opaque`（≈ vanilla `canOcclude`），流体分支不变；AO/光照采样仍用
  `is_occluded_by_cell`（与面剔除正交，不动）。修正既有 buggy 断言
  `box_model_culls_only_faces_marked_by_cullface`（slab 半高侧面不再遮邻立方，
  culled 4→2 / opaque 14→16）。新增测试：`face_occludes` 直测（cube/上下 slab/
  stairs 并集背面/cross/quads/空 boxes）、上下 slab 叠满格中间面剔、同向双 slab
  不剔侧面（保守过渲染）、stairs 并集满背面剔邻立方、cutout(玻璃类) slab 不遮、
  cross 邻居不遮、跨 chunk slab 半面不遮。skipRendering（同类玻璃/铁栏杆相邻
  剔除）另记账为独立子项（需跨 crate 方块分类 + TerrainCell 新字段，超本片体量）。
- [x] chest block-entity renderer（2026-07-06，BER 伞形首片，chest 全家族：
  chest/trapped/ender + 8 种 copper chest，waxed 共享风化档纹理）：bbb 首个
  BER 面。world 数据链（`bbb-world/src/chest_lids.rs`）：`ChestLidState` 平铺
  tracker 转写 vanilla `ChestLidController`——`BlockEvent(1, count)` 按 26.1
  客户端派发链 `Level.blockEvent`（对**当前** block state 派发，Level.java:901）
  → `BaseEntityBlock.triggerEvent` → `ChestBlockEntity.triggerEvent` 设
  `shouldBeOpen(count>0)`；`tickLid` 每 tick 0.1 步进、`oOpenness` 拖尾、
  `[0,1]` clamp，runtime pump 里按 running ticks 推进（vanilla client BE
  ticker 受 tick-rate manager 门控）；方块被破坏/卸载或静止全关的条目修剪。
  chest 位置每帧从 chunk block state 派生（`chest_model_source_states`，
  palette 无 chest state 的 section 整段跳过，扫描成本随含 chest section
  数）；双箱按 `ChestBlock.getConnectedDirection`（LEFT→`facing.getClockWise()`，
  同方块反 `type` 校验）配对，openness 取双方 lerped openness 的
  `opennessCombiner` max。dispatch 接法（避免双路径）：chest 实例以
  `EntityModelKind::Chest { texture, half }` 进入既有唯一 entity-model 提交流
  （`RendererFrame.entity_model_instances`，`entity_id` 用 -1 哨兵），不开
  平行 textured 提交路径；root transform 转写 BER 位姿
  `rotationAround(Axis.YP.rotationDegrees(-facing.toYRot()), 0.5, 0, 0.5)`
  且无实体 `scale(-1,-1,1)` 翻转（chest mesh 按 block 空间 Y-up 作模）；光照
  取方块位采样 `block<<4|sky<<20`，双箱按 `BrightnessCombiner` 分量 max。
  renderer（`model_layers/chest.rs`）：`ChestModel.java` 三套 mesh 逐字转写
  （single 14 宽 / left·right 15 宽 bottom/lid/lock，lid+lock 共 pivot
  `offset(0,9,1)`），`setup_anim` 施 `1-(1-o)^3` easing 与 `xRot=-(o·π/2)`；
  19 张 `entity/chest/*.png`（64×64）进共享 entity atlas；render type 用
  vanilla `entityCutoutCull`（cull 开的 cutout 桶）。defer 如实记账：xmas
  Dec 24-26 纹理切换（无 wall-clock 输入）、双箱接缝面 `allOfEnumExcept`
  可见性（bbb 发出接缝 quad，但被拼合箱体完全包裹不可见）、BER
  `breakProgress` crumbling、逐 BE 距离/视锥剔除。测试：world 侧事件门控/
  tick 序列/clamp/修剪/login 清空/枚举配对与 openness 合成；renderer 侧 9 个
  cube+pivot 对照 `ChestModel.java`、easing/角度手算、facing 旋转矩阵点映射、
  ender/copper 纹理选择、cutout-cull mesh 烘焙；native 侧投影 facing/openness/
  光照打包/双箱 max + runtime pump 顺序断言。
- [x] sign + hanging sign block-entity renderer + 牌面文本（2026-07-06，BER
  第二片，12 木种含 pale_oak × standing/wall/hanging ceiling（±`attached`
  vChains middle 变体）/hanging wall）：world 数据链：sign BE NBT 解码收敛进
  bbb-protocol `decode_sign_block_entity_nbt`（`component.rs`），front_text/
  back_text 的 messages[4] 组件复用唯一 `append_component_runs` styled 遍历
  （单一 styled 解码实现；`SignText.DIRECT_CODEC` 形状：dye 名 `color` 缺省
  black、`has_glowing_text`、根 `is_waxed`）；`chunks/sign_text.rs` 薄映射为
  `SignBlockEntityTextState`（缺侧按 vanilla `orElseGet(SignText::new)` 取默
  认）；ChunkData BE section 与 `BlockEntityData` 包共用该 ingest（记录整
  替换）；`set_block_state_id` 在 block 名变更时修剪该位 `sign_text`（渲染
  枚举本就 state 派生，修剪防换牌复活旧文本；BE 记录通用移除仍记账）。
  sign 位置每帧从 block state 派生（`sign_blocks.rs::sign_model_source_
  states`，palette 预检跳段；`rotation16 = seg*22.5°`、wall 族
  `facing.toYRot`、hanging `attached=true → CEILING_MIDDLE(vChains)`，文本
  侧按非空行 gating）。dispatch：实例以 `EntityModelKind::Sign { wood,
  attachment }` 进唯一 entity-model 提交流（-1 哨兵、方块位光照
  `block<<4|sky<<20`）；root transform 转写 `StandingSignRenderer.
  bodyTransformation`：`translate(0.5,0.5,0.5)·Ry(-angle)·scale(2/3,-2/3,
  -2/3)`（RENDER_SCALE=0.6666667），wall 追加 `translate(0,-0.3125,
  -0.4375)`；hanging `translation(0.5,0.9375,0.5)·Ry(-angle)·translate(0,
  -0.3125,0)·scale(1,-1,-1)`。renderer（`model_layers/sign.rs`）：
  `createSignLayer`/`createHangingSignLayer` 逐字转写（board 24×12×2
  texOffs(0,0) + stick 2×14×2 texOffs(0,14)；hanging board 14×10×2
  texOffs(0,12)、plank 16×2×4、chain 平面 texOffs(0,6)/(6,6)
  offset(±5,-6,0) yRot∓π/4、vChains 12×6 texOffs(14,6)）；24 张
  `entity/signs[/hanging]/<wood>.png`（64×32）进共享 entity atlas；render
  type 用 vanilla `entityCutout`（无 cull）。牌面文本：世界空间 glyph quad
  烘焙（`item_models/sign_text.rs::bake_sign_text_surface`），走 item-frame
  map label 的 `minecraft:font/default` atlas、在 entity-translucent
  feature pass 绘制；文本变换 = body · [back: Ry(π)] · `TEXT_OFFSET(0,
  0.33333334,0.046666667)`（hanging `(0,-0.32,0.073)`）· scale
  0.010416667/0.0140625（y 取负）；布局逐字对照 `SignRenderer` 语义：行高
  10/9、单行截断宽 90/60（先按最后空格断词、无空格在溢出字形前硬断）、居中
  `x=-width/2`（Java int 除法）、`y=i*lh-4*lh/2`；颜色 `getDarkColor`：
  `ARGB.scaleRGB(color,0.4F)` 逐通道截断、black+glowing → -988212/0xF0EBCC
  米色；glowing 面用原 dye `getTextColor` + full-bright（15728880）；逐 run
  组件色覆盖底色；bold 双绘/italic 斜切沿用 HUD `styled_quads`。defer 如实
  记账：glowing 8 向 outline glyph pass、underline/strikethrough effect
  bar（需 font-atlas 外白像素）、obfuscated 乱码轮换（按原字形绘制）、
  `is_waxed` 仅存储（vanilla 亦无渲染效果，仅编辑门控）、`filtered_
  messages` 未解码、BE 记录通用移除、逐 BE 距离剔除、vanilla 文本
  POLYGON_OFFSET display mode（以 TEXT_OFFSET z 间隙近似）。测试：protocol
  NBT 解码（4 行/color/glowing/waxed/缺侧）、world BE section+包更新与
  block 变更修剪、`sign_blocks` 枚举/rotation/facing/attached、
  `entity_models/tests/sign.rs`（7 cube+chain pose 对照 vanilla、root 变换
  standing/wall/hanging 点映射、model key、24 纹理表驱动、pass render
  type、cutout mesh 烘焙）、`item_models/sign_text.rs`（文本变换偏移/缩放
  含背面、暗色公式、90/60 截断+断词+bold 7px、居中与行 y 手算、run 色覆盖
  +bold 双绘、空面 None）、`bbb-native/src/sign_scene.rs`（kind/rotation/
  光照打包、双面 gating、glowing full-bright）。
- [x] bed + bell block-entity renderer（2026-07-06，BER 第三片）：bed（16 色
  `minecraft:<color>_bed` × HEAD/FOOT × facing）：颜色/part/facing 每帧从
  block state 派生（`bbb-world/src/bed_blocks.rs`，palette 预检跳段；颜色是
  block id 事实，`BedBlockEntity.getColor` 渲染路径不读 NBT），
  `DoubleBlockCombiner` 配对（`getNeighbourDirection`：FOOT→facing、HEAD→
  反向；同 block + 另一 part + 同 facing）喂 `BrightnessCombiner` 分量 max
  光照。dispatch：`EntityModelKind::Bed { color, part }` 进唯一 entity-model
  提交流（-1 哨兵、`block<<4|sky<<20`）；root transform 逐字转写
  `BedRenderer.createModelTransform`：`translation(0,0.5625,0)·Rx(90°)·
  rotateAround(Rz(180+facing.toYRot()),0.5,0.5,0.5)`，无 entity 翻转。
  renderer（`model_layers/bed.rs`）：`createHeadLayer`/`createFootLayer`
  （atlas 64×64）逐字转写——main 16×16×6 texOffs(0,0)/(0,22)、四腿 3×3×3
  texOffs(50,{6,18,0,12}) 于 `PartPose.rotation(π/2,0,{π/2,π,0,3π/2})`；
  vanilla `visibleFaces`（head main 藏 UP、foot main 藏 DOWN——即两半接缝
  面；四腿藏 DOWN——与可见床垫底面共面否则 z-fight）如实生效：共享 cube
  emitter 新增 vanilla 形状的逐面可见性掩码（`ModelCube::with_visible_
  faces`，`MODEL_CUBE_FACE_*` 按 `Direction.get3DDataValue` 位序；既有模型
  不受影响，`addBox` 默认全可见）。16 张 `entity/bed/<DyeColor.getName()>.
  png` 进共享 entity atlas；render type 用 vanilla `entitySolid`（cull
  桶）。bell（`minecraft:bell` 全 4 attachment）：`BellRenderer.submit` 无
  任何 transform——bell body 四种 attachment 同位渲染；bar/post 支架属
  block model（`bell_floor/wall/ceiling/between_walls.json` 的
  `#bar`/`#post` 元素）由 terrain 路径绘制，BER 只补 body。摇摆链转写
  （`bbb-world/src/bell_blocks.rs`）：`BlockEvent(1,dir)` →
  `triggerEvent`（`clickDirection=from3DDataValue(b1)`（DOWN/UP 可上线但
  不摆）、`ticks=0`、`shaking=true`、重敲重置），`clientTick`
  `if(shaking)ticks++; if(ticks>=50){shaking=false;ticks=0;}`（DURATION
  50）在 runtime pump 按 running ticks 推进；拆除/结束修剪。renderer
  （`model_layers/bell.rs`）：`BellModel.createBodyLayer`（atlas 32×32）
  ——`bell_body` 6×7×6 texOffs(0,0) box(-3,-6,-3) pivot offset(8,12,8)、
  子 `bell_base` 8×2×8 texOffs(0,13) box(4,4,4) offset(-8,-12,-8)；
  `setupAnim` 摆角 `Mth.sin(ticks/π)/(4+ticks/3)`（`ticks=BE.ticks+
  partialTicks`），按 click 方向选轴（N `xRot=-r`/S `+r`/E `zRot=-r`/W
  `+r`）；`entity/bell/bell_body.png` 32×32；render type `entitySolid`。
  defer 如实记账：BER breakProgress crumbling、逐 BE 距离/视锥剔除（同
  chest/sign 边界）、bell resonation 粒子/发光链（gameplay 侧随 raid 特性
  走）。测试：world 侧 16 色表/配对破坏三态/事件门/50 tick 序列/重敲/拆除
  修剪/login 清空/`from3DDataValue` 表；renderer 侧 cube+visibleFaces+腿
  pose 对照 vanilla、S/N/W/E 变换点映射、16 纹理表（DyeColor id 序）、
  `entitySolid` pass、15 面（bed）/12 面（bell）cutout-cull 烘焙证隐藏面、
  摆角 ticks 0/10/25 手算 × 四轴 + DOWN/UP/None 静止；native 侧投影
  kind/角度/光照打包/双半 max + pump 顺序断言。
- [x] shulker box + decorated pot block-entity renderer（2026-07-06，BER
  第四片）：shulker box（`minecraft:shulker_box` + 16 色 ×六向 facing）：
  开合状态机转写（`bbb-world/src/shulker_box_blocks.rs`）——
  `BlockEvent(1,count)` → `ShulkerBoxBlockEntity.triggerEvent`
  （`java:140-155`：count 1→OPENING、0→CLOSING、其余仅记数）、
  `updateAnimation`（`java:66-101`：`progressOld=progress` 后 ±0.1/tick、
  0/1 处闩锁、CLOSED 修剪）在 runtime pump 按 running ticks 推进，
  `getProgress=lerp(partial, progressOld, progress)`。dispatch：
  `EntityModelKind::ShulkerBox{color,facing}` 进唯一 entity-model 提交流
  （-1 哨兵、`block<<4|sky<<20`）；root transform 逐字转写
  `ShulkerBoxRenderer.createModelTransform`（`java:111-121`）：
  `T(0.5,0.5,0.5)·S(0.9995)·R(FACING.getRotation())·S(1,-1,-1)·T(0,-1,0)`
  （`Direction.getRotation()` 表 `Direction.java:144-153`）。renderer
  （`model_layers/shulker_box.rs`）：`ShulkerModel.createBoxLayer` 即 mob
  壳去头（lid 16×12×16 texOffs(0,0)、base 16×8×16 texOffs(0,28)、pivot
  offset(0,24,0)、atlas 64×64）——cube 常量与 shulker mob 共用，17 张
  `entity/shulker/shulker[_<color>].png` 已注册，box 零新增纹理；
  `setupAnim`（`ShulkerBoxRenderer.java:141-145`）`lid.setPos(0,24−
  progress·0.5·16,0)`、`lid.yRot=270°·progress`；render type
  `entityCutout`（`java:137`，mob 用 entityCutoutZOffset）。decorated pot
  （`minecraft:decorated_pot` × HORIZONTAL_FACING）：BE NBT `sherds`
  item-id 列表（`PotDecorations.java:23-52`：≤4 项按 back/left/right/
  front 序，`minecraft:brick`/缺项=素面）解进
  `BlockEntityRecord.decorated_pot_sherds`（chunk 批量 + 单条
  BlockEntityData，方块变更修剪）；sherd→pattern 用转写 23 项常量表
  （`bbb-native/src/decorated_pot_scene.rs`，引
  `DecoratedPotPatterns.java:37-62/72-97`：`<name>_pottery_sherd` →
  `<name>_pottery_pattern`；未知 item → 素面 `decorated_pot_side`，同
  vanilla null-pattern 兜底）。root transform 转写
  `DecoratedPotRenderer`（`java:175-177`）`rotateAround(Ry(180−toYRot),
  0.5,0.5,0.5)`；wobble 按 bell 模式落地（`bbb-world/src/
  decorated_pot_blocks.rs`）：`BlockEvent(1,style.ordinal())`
  （`DecoratedPotBlockEntity.java:167-175`，data<2 门）起 tick 计数器代
  vanilla `gameTime−wobbleStartedAtTick` 时钟（POSITIVE 7 tick/NEGATIVE
  10），重触发重置、到期/拆除修剪；渲染侧转写 `java:150-169`：门
  `0≤progress≤1`，POSITIVE `Rx(−1.5·(cos dt+0.5)·sin(dt/2)·0.015625)` 再
  `Rz(sin dt·0.015625)`（`dt=progress·2π`，pivot (0.5,0,0.5)），NEGATIVE
  `Ry(sin(−progress·3π)·0.125·(1−progress))`。renderer
  （`model_layers/decorated_pot.rs`）：`createBaseLayer`（32×32，
  `java:83-101`）neck texOffs(0,0) box(4,17,4)+(8,3,8) deflate(−0.1) +
  texOffs(0,5) box(5,20,5)+(6,1,6) inflate(0.2) 于
  offsetAndRotation(0,37,16,π,0,0)（CubeDeformation：min−g、size+2g、UV
  取未变形尺寸），top/bottom texOffs(−14,13) 14×0×14 平面于 (1,16,1)/
  (1,0,1)；`createSidesLayer`（16×16，`java:103-112`）14×16×0 平面
  texOffs(1,0) 仅烘 NORTH 面（`ModelCube::with_visible_faces`），pose
  back(15,16,1,0,0,π)/left(1,16,1,0,−π/2,π)/right(15,16,15,0,π/2,π)/
  front(1,16,15,π,0,0)；单棵 7 部件树经 `RetainedParts` 可见性拆 5 个
  `entitySolid` pass（base 贴图管 neck/top/bottom + 每面一个 pattern
  pass）；25 张 `entity/decorated_pot/*`（base 32×32 + side + 23 pattern
  16×16，资产树清点核对）进共享 entity atlas 与 `entity_assets.rs`。
  defer 如实记账：BER breakProgress crumbling、逐 BE 距离/视锥剔除（同
  chest/sign/bed/bell 边界）；box 开合音效/碰撞属 gameplay；26.1 反编译
  `DecoratedPotRenderer.extractRenderState` 未赋 `state.wobbleStyle`
  （反编译伪影），bbb 特意携带 style。测试：world 侧 17 色表/事件门/
  开合闩锁/0.1 步进 + lerp 手算/记数不改状态/拆除修剪/投影、wobble 事件
  门/style 表/重触发/到期修剪/进度投影/facing 表、sherds NBT 顺序/brick
  与缺项素面/方块变更修剪；renderer 侧 shulker cube 对照 + lid pose
  progress 0/0.5/1 手算（(24,0°)/(20,135°)/(16,270°)）+ 六向变换点映射
  （含 0.9995 收缩）+ 17 纹理选择 + `entityCutout` pass + 12 面 cutout
  烘焙、pot 变形 cube/pose 对照 + NORTH 单面 + facing 点映射 + wobble
  正负手算与 >1 门 + 25 纹理表 + 5 pass 逐面 pattern/兜底选择 + 28 面
  cutout-cull 烘焙；native 侧 kind/色/facing/y-rot/光照打包、sherd→
  pattern 表回环（brick/未知兜底）、wobble style+progress 投影 + pump
  顺序断言。
- [x] banner block-entity renderer（2026-07-06，BER 第五片）：32 方块
  （16 色 `minecraft:<color>_banner` ROTATION 16 段 + 16 色
  `<color>_wall_banner` FACING）。world：BE NBT `patterns` 列表
  （`BannerPatternLayers.CODEC`，`{pattern: 注册 id, color: 染料名}`
  compound）解进 `BlockEntityRecord.banner_patterns`
  （`chunks/banner_patterns.rs`，chunk 批量 + 单条 `BlockEntityData`
  双入口、方块变更修剪）；任一条目畸形 → 整列表折叠 None（对应
  `BannerBlockEntity.loadAdditional` 的 `.orElse(EMPTY)` codec 失败
  语义）；base 色是 block-id 事实（`AbstractBannerBlock.getColor`）。
  flag 摆动相位逐字转写 `BannerRenderer.extractRenderState`：
  `(floorMod(x·7+y·9+z·13+gameTime,100L)+partial)/100`，gameTime 用确定性
  `WorldTimeState.game_time`（i32 wrapping 位置哈希 + `rem_euclid`
  floor-mod，`bbb-world/src/banner_blocks.rs`）。dispatch：
  `EntityModelKind::Banner{wall,base_color,layers:[Option<
  BannerPatternLayer>;16]}` 进唯一 entity-model 提交流（-1 哨兵、
  `block<<4|sky<<20`）；root transform 逐字转写 `modelTransformation`：
  `T(0.5,0,0.5)·Ry(−angle)·S(⅔,−⅔,−⅔)`（ground
  `RotationSegment.convertToDegrees`（22.5° 段折入 (−180,180]）、wall
  `FACING.toYRot()`）。renderer（`model_layers/banner.rs`）转写
  `BannerModel.createBodyLayer`（64×64：standing 专属 pole 2×42×2
  texOffs(44,0) 于 (−1,−42,−1)、bar 20×2×2 texOffs(0,42) 于
  (−10,−44,−1)/(−10,−20.5,9.5)）与 `BannerFlagModel.createFlagLayer`
  （flag 20×40×1 texOffs(0,0) 于 (−10,0,−2)、pivot
  (0,−44,0)/(0,−20.5,10.5)）；`setupAnim`：
  `flag.xRot=(−0.0125+0.01·cos(2π·phase))·π`。pattern 合成转写
  `submitBanner`/`submitPatterns`（`BannerRenderer.java:171-208`）：
  frame+flag 无 tint `entitySolid` 提交 `entity/banner/banner_base`，
  随后同一 flag 几何逐层重提交——`base` 面罩先按 base 色 tint、再每层
  `entity/banner/<pattern>` 按 `DyeColor.getTextureDiffuseColor()`
  tint（走既有逐 pass 顶点 tint，即热带鱼 base/pattern 机制），clamp
  `MAX_PATTERNS=16`；pattern pass 走 translucent bucket 代
  `RenderPipelines.BANNER_PATTERN`（`java:282`：TRANSLUCENT 混合 +
  LEQUAL 不写深度）。43 项 pattern 注册表转写
  （`BannerPatterns.java:60-105`，asset_id=注册 id；未知 pattern id/
  染料名整栈折空，对应 registry holder codec 失败——datapack pattern
  bbb 无纹理，同折）。44 张 64×64 `entity/banner/*`（banner_base +
  base + 42 pattern，资产树清点 44 文件）进共享 entity atlas 与
  `entity_assets.rs`。defer 如实记账：BER breakProgress crumbling、
  逐 BE 距离/视锥剔除（同前四片边界）；vanilla bannerPattern 管线不写
  深度、bbb 共享 translucent 管线写深度（等深 LEQUAL 叠层不受影响，
  仅在将来需要专用不写深度 pass 时再议）；banner 物品/盾牌 pattern
  （`SHIELD_PATTERN_BASE`/`submitSpecial` 路径）属 item-model 范畴。
  测试：world 侧 32 方块色/形态表、patterns NBT 层序、畸形条目折叠、
  相位手算（负数 floor-mod + gameTime 步进）、rotation 段/facing 角度、
  方块变更修剪；renderer 侧 cube/pivot 对照（wall 无 pole 树）、摆角
  phase 0/¼/½/1 手算 + prepare 接线、变换点映射（pole 顶→y28、−90°
  yaw）、5 pass 栈（kind/render type/layer id/retained/tint/序列）+
  wall 3 pass 变体、44 纹理表 + atlas 隶属、烘焙 18 面 cutout-cull
  （frame+flag）+ 12 面 translucent 逐 pass tint 重烘；native 侧
  kind/底色/yaw/相位/光照投影、43 pattern 表回环、未知 pattern/染料
  折空、16 层渲染上限。
- [x] 附魔台悬浮书 + lectern 摆放书 block-entity renderer（2026-07-07，BER
  第六片）：两者共享 vanilla `ModelLayers.BOOK` / `BookModel` + 单一
  `entity/enchantment/enchanting_table_book` 64×32 纹理。world：附魔台悬浮书
  是 per-BE 动画（`EnchantingTableBlockEntity`：`time`、`flip`/`oFlip`/`flipT`/
  `flipA`、`open`/`oOpen`、`rot`/`oRot`/`tRot`），每 client tick 由
  `bookAnimationTick`（`EnchantingTableBlockEntity.java:50-106`）推进，转写在
  `bbb-world/src/enchanting_table_books.rs` 为平铺 `Vec<EnchantingTableBookState>`
  于 runtime pump running tick 上对账 + 步进。书朝向 3 格内最近玩家
  （`getNearestPlayer(x+0.5,y+0.5,z+0.5,3.0,false)` → `NO_SPECTATORS`，转写为
  本地玩家 + 远程玩家实体去 spectator，同粒子 nearest-player 先例）、0.1/tick
  开合、随机 `flipT` 目标翻页。vanilla 静态 wall-clock seed `RANDOM` 换成单一
  定 seed 可序列化 `LegacyRandomSource`（`EnchantingBookRandom`），按位置排序
  确定性抽取——vanilla 此处本身非确定（wall-clock seed + BE ticker 序），此为
  忠实确定性替身。lectern 书纯 block-state 派生（`bbb-world/src/lectern_books.rs`，
  无 BE 数据）：仅 `LecternBlock.HAS_BOOK` 为真时渲染，yaw =
  `FACING.getClockWise().toYRot()`。dispatch：`EntityModelKind::EnchantingBook`/
  `LecternBook` 进唯一 entity-model 提交流（-1 哨兵、`block<<4|sky<<20`）。
  `EnchantTableRenderer.extractRenderState` partial-tick lerp（`flip`/`open`/
  `time` + `(-π,π]` 折叠的 `oRot+or·partial` yaw）在
  `enchanting_table_book_scene.rs`；附魔根变换转写 `submit`（`java:61-73`）：
  `T(0.5,0.75,0.5)·T(0,0.1+sin(time·0.1)·0.01,0)·Ry(-yRot)·Rz(80°)`，lectern
  根变换 `LecternRenderer.submit`（`java:46-50`）：`T(0.5,1.0625,0.5)·Ry(-yRot)·
  Rz(67.5°)·T(0,-0.125,0)`（无额外 model scale——mesh 1/16 单位烘进 cube）。
  renderer（`model_layers/book.rs`）转写 `BookModel.createBodyLayer`
  （`BookModel.java:35-53`，64×32：`left_lid` 6×10×0.005 texOffs(0,0)
  offset(0,0,-1)、`right_lid` texOffs(16,0) offset(0,0,1)、`seam` 2×10×0.005
  texOffs(12,0) rotation(0,π/2,0)、`left_pages` 5×8×1 texOffs(0,10) 于 -0.99z、
  `right_pages` texOffs(12,10) 于 -0.01z、`flip_page1/2` 5×8×0.005 texOffs(24,10)）
  与 `BookModel.setupAnim`（`java:55-68`）：`leftLid.yRot=π+openness`、
  `rightLid.yRot=-openness`、pages `±openness`、`flipPageN.yRot=openness−
  openness·2·pageFlipN`、所有 page `x=sin(openness)`，openness 由
  `State.forAnimation` 派生 `(sin(progress·0.02)·0.1+1.25)·open`（renderer 侧
  setup_anim）。翻页分数 `clamp(frac(flip+{0.25,0.75})·1.6−0.3,0,1)` 属 submit
  逻辑（native 侧）；lectern 绑定固定 `State.forAnimation(0,0.1,0.9,1.2)`
  （openness 1.5）。1 张 64×32 `enchanting_table_book` 纹理进共享 entity atlas
  （`ENTITY_MODEL_TEXTURE_REFS` 681）与 `entity_assets.rs`。defer 如实记账：
  BER breakProgress crumbling、逐 BE 距离/视锥剔除（同前五片边界）；附魔翻页
  样式与任意 vanilla 会话不同（双方随机均无确定性契约，bbb 至少可复现）；
  批 >1 running tick 复用当前玩家位置（0/1 tick/帧不可分辨）。测试：world 侧
  最近玩家 3 格 `<range²` 边界、open/rot 追随后松弛、翻页重掷 + flip 缓动、
  新增/修剪表跟踪、source 枚举、随机确定性；lectern 侧 has-book 门控、
  facing→顺时针 yaw 表、撤书修剪；renderer 侧 cube/pivot 对照（含静态 seam）、
  `State.forAnimation` openness 手算（sin 峰值 + lectern 1.5）、`setupAnim`
  cover/page/flip 手算 + prepare 接线、附魔 hover+tip 与 lectern 变换点映射、
  model-key/纹理选择、单 `entitySolid` pass、7 盒 42 面 cutout-cull 烘焙；
  native 侧闭书默认、partial-tick lerp/yaw 抽取、闭书固定翻页分数、
  lerp/frac/wrap 手算、光照打包、lectern has-book 门控 + 固定 state + facing
  yaw；runtime pump tick-before-extract 序断言。
    （submerged 视角可见，底面单面）。
  - terrain / fluid 面已按 chunk 所在维度的 vanilla `CardinalLighting` 着色
    （`BlockModelLighter`：shaded 面 `byFace(dir)`、非 shaded 面 `up()`），由
    `DimensionType.cardinalLightType` 选择、经 `WorldStore` 穿进 `TerrainChunkSnapshot`：
    Nether 维度用 `CardinalLighting.NETHER`（`down`/`up`=0.9，侧面同 DEFAULT），其余
    内建维度用 `DEFAULT`。水侧面现在按 vanilla 对 HalfTransparent / Leaves
    邻居选择 `water_overlay` 并抑制该侧背面；datapack 维度类型覆盖的
    `cardinal_light` 字段现在从 `minecraft:dimension_type` registry NBT 解码。
- 补齐 selection overlay、block entity 特殊 renderer、透明块排序等剩余 presentation；
  破坏进度的 renderer-visible cube crack overlay 已覆盖官方 `destroy_stage_0..9`
  atlas、本地/服务端同位置取最高 stage、400 render tick 过期和 crumbling
  pipeline state；完整模型形状 crack decal 仍随 block destroy presentation 后续推进。
- [x] conduit block-entity renderer（2026-07-08，BER 第七片）：vanilla
  `ConduitBlockEntity.clientTick` / `ConduitRenderer.submit` / `ConduitRenderState`
  转写为 repo-native world→native→renderer 链路。world 侧新增平铺
  `ConduitBlockState` 与 source-state 投影：每 client tick 推进 `tickCount`，
  active 时推进 `activeRotation`，每 `gameTime % 40 == 0` 按 3×3×3 water
  要求 + 5×5×5 prismarine/sea-lantern ring 计数刷新 active/hunting（16/42
  阈值），`getActiveRotation(partial)` = `(activeRotation + partial) * -0.0375`。
  native runtime 在 running ticks 上 advance conduit 状态，并把 inactive
  conduit 投成 shell 单实例、active conduit 投成 cage / outer wind / inner wind /
  camera-facing eye 四个 `EntityModelInstance`，采样 `block<<4 | sky<<20` 光照。
  renderer 新增 `ConduitModelPart`、`EntityModelKind::Conduit { part }`、
  `ConduitModel` 四层 cube（eye 8×8×0 + 0.01 deformation、wind 16³、shell 6³、
  cage 8³）、6 张 `textures/entity/conduit/*` 纹理进共享 entity atlas，layer
  pass 按 vanilla 区分 inactive `entitySolid(base)` 与
  active `entityCutout(cage/wind/wind_vertical/open_eye/closed_eye)`。根变换转写
  inactive shell center + `activeRotation * PI / 180` quirk，active cage bob +
  `(0.5,1,0.5)` 轴旋转，outer wind phase 0/1/2，inner wind 0.875 scale +
  `rotationXYZ(π,0,π)`，eye bob + camera orientation + `Rz(π)·Ry(π)·S(4/3)`。
  测试覆盖 world 激活/水门控/source、native inactive/active/camera eye、renderer
  cube/texture/layer/transform/mesh bucket，以及 runtime tick-before-extract
  顺序。defer 边界保持 BER break-progress crumbling 与逐 BE 距离/视锥剔除。
- [x] skull/head block-entity renderer（2026-07-08，BER 第八片）：vanilla
  `SkullBlockRenderer` / `SkullBlockRenderState` / `SkullBlockEntity` /
  `SkullBlock` / `WallSkullBlock` 转写为 repo-native world→native→renderer
  链路。world 侧新增 7 类 skull/head 映射（skeleton、wither skeleton、zombie、
  player、creeper、dragon、piglin）与平铺 `SkullBlockState`，standing head 从
  `ROTATION_16` 派生 yaw，wall head 从 `FACING` 派生墙面 attachment；只有
  powered dragon/piglin skull/head 按 vanilla client ticker 推进
  `animationTickCount`。native runtime 在 running ticks 上 advance skull 状态，
  并把 source 投成共享 entity-model stream 中的 `EntityModelKind::SkullBlock`，
  采样 `block<<4 | sky<<20` 光照，ground yaw 使用
  `-RotationSegment.convertToDegrees(segment)`，wall 使用 vanilla
  `WallAndGroundTransformations` attachment。renderer 复用既有 custom-head
  `SkullModel` / `DragonHeadModel` / `PiglinHeadModel` 几何，按 vanilla
  skeleton/wither/player/zombie/creeper/dragon/piglin texture 分派
  `entityCutoutZOffset` no-overlay pass，并把 dragon/piglin animation progress
  写入 skull model state。测试覆盖 world family/state/tick/source、native
  ground/wall/animated 投影、renderer key/texture/root transform/mesh bucket，
  以及 runtime tick-before-extract 顺序。defer 边界：player-head BE
  `profile` owner skin 仍归 P3 动态 profile/texture 管线，profileless player
  head 先使用 vanilla default skin fallback。
- [x] end portal/gateway block-entity renderer（2026-07-08，BER 第九片）：
  vanilla `AbstractEndPortalRenderer` / `TheEndPortalRenderer` /
  `TheEndGatewayRenderer` / `TheEndGatewayBlockEntity` / `BeaconRenderer`
  转写为 repo-native world→native→renderer 链路。world 侧新增 gateway
  `Age` NBT 解码、平铺 age/cooldown 状态、BlockEvent(1) cooldown、client
  ticker `beamAnimationTick` 推进，以及 source-state 投影：portal/gateway
  只提交 Y 轴 faces，portal 使用 `T(0,0.375,0) * S(1,0.375,1)`，gateway
  beam 使用 spawn/cooldown percent、`sin(percent*PI)` scale、height、magenta/
  purple `DyeColor`、`floorMod(gameTime,40)+partial` animation time。native
  runtime 在 running ticks 上 advance gateway 状态，并把 source 投成共享
  entity-model stream 中的 `EntityModelKind::EndPortalBlock` + optional
  `EndGatewayBeamRenderState`。renderer 新增 `EndPortalModelKind` /
  `EndPortalModelFace` / `EndGatewayBeamRenderState`，portal/gateway cube
  进入 position-color custom geometry，gateway beam 进入 scroll bucket 并按
  vanilla `BeaconRenderer.renderPart` 生成内层旋转 beam + alpha=32 glow；新增
  `textures/entity/end_portal/end_gateway_beam.png` 到共享 entity atlas
  （688-count）。测试覆盖 world NBT/tick/source、native instance/beam 投影、
  renderer cube transform/faces/beam geometry/sorted draw range，以及 runtime
  tick-before-extract 顺序。defer 边界：portal/gateway cube 当前是可见
  position-color approximation；完整 `RenderTypes.endPortal()` /
  `endGateway()` 15/16-layer shader parity 留在 unsupported ledger。
- [x] spawner 旋转 display entity block-entity renderer（2026-07-08，BER
  第十片）：vanilla `SpawnerRenderer` / `TrialSpawnerRenderer.extractSpawnerData` /
  `BaseSpawner` / `SpawnData` / `SpawnerBlockEntity` 转写为 repo-native
  world→native→renderer 链路。world 侧新增普通 `minecraft:spawner` 平铺
  ticker 与 source-state 投影：chunk 与 `BlockEntityData` 均解码
  `Delay`、`MinSpawnDelay`、`RequiredPlayerRange`、`SpawnData.entity.id`；
  `BaseSpawner.clientTick` 近玩家门控用本地玩家位置近似 vanilla
  `hasNearbyAlivePlayer`，有 display entity 时每 running tick 递减 delay 并
  推进 `spin=(spin+1000/(delay+200))%360`，BlockEvent(1) 重置到
  `minSpawnDelay`。source 投影按 vanilla `lerp(oSpin, spin, partial)*10`
  得出旋转角，实体比例用 `0.53125/max(bbWidth,bbHeight)` 并复用既有
  entity pick bounds 表。protocol 侧新增 resource id→entity type id 窄查询，
  从已有 `VANILLA_ENTITY_TYPE_*_ID` 常量名派生，避免维护第二份表。native
  侧新增 `spawner_scene`，把 source 映射成共享 entity-model stream 的
  `EntityModelInstance`（-1 哨兵、采样 `block<<4|sky<<20` 光照），实体 kind
  走现有 `EntityModelKind` 投影。renderer 侧新增
  `SpawnerDisplayRenderState`，colored entity 根位置统一经
  `entity_root_position_transform`，普通实体仍是原位置平移，spawner display
  额外套 `T(0.5,0.4,0.5)·Ry(spin)·T(0,-0.2,0)·Rx(-30°)·S(scale)` wrapper，
  因此继续复用各 mob 既有模型/纹理/pass。测试覆盖 protocol lookup、world
  NBT/tick/source/event、native instance 投影、renderer wrapper 原点与倾斜
  点。defer 边界：trial spawner display 行为不在普通 spawner 片内；
  `SpawnData.entity` 的自定义实体 metadata/NBT 暂不合成，未知 entity id
  不输出 display instance；逐 BE 距离/视锥剔除与 break-progress crumbling
  仍沿 BER 既有边界。

## P2：屏幕、HUD、字体与截图

### 2026-07-08 迁入：马跳跃条 / jumpable-vehicle contextual bar

- 原 goal.md P2 HUD 缺口行中的「马跳跃条」完成并移出当前待办。
- vanilla 依据：`Gui.willPrioritizeJumpInfo` / `nextContextualInfoState`
  将可跳坐骑栏放在 experience contextual bar 之前；`JumpableVehicleBarRenderer`
  使用 182x5 `ContextualBarRenderer` 位置，背景 `hud/jump_bar_background`，
  cooldown 时整条 `hud/jump_bar_cooldown`，否则按
  `Mth.lerpDiscrete(player.getJumpRidingScale(),0,182)` 裁剪
  `hud/jump_bar_progress`；`LocalPlayer` 蓄力曲线复用已有骑乘跳跃命令路径。
- 实现链：`ClientInputState::riding_jump_scale` 暴露同源蓄力 scale；
  `WorldStore::local_player_rideable_jumping_vehicle_id` 现在应用 first-passenger
  门与共享 saddle-item `canJump()` 门；
  `WorldStore::local_player_rideable_jumping_vehicle_cooldown` 读取被控可跳坐骑的
  cooldown（camel/camel husk 来自已重建的 `Camel.DASH` client cooldown，普通马系为
  `PlayerRideableJumping` 默认 0）；`RendererFrame.hud_jump_bar` 单次提交
  `HudJumpBar { progress, cooldown }`；renderer 上传三张 vanilla jump-bar sprite，
  并在该字段存在时用 jump contextual bar 替代 experience bar，经验等级数字保持独立。
- 边界：nautilus / zombie-nautilus dash cooldown 尚未重建，因此它们目前能显示蓄力
  progress，但 dash 后 cooldown overlay 仍缺；camel `refuseToMove()`（sitting /
  pose transition）的 `canJump()` 附加门尚未并入本地 jumpable-vehicle query；locator
  bar / waypoint priority 仍未实现。
- 测试：`bbb-native`
  `riding_jump_scale_matches_vanilla_local_player_curve`；`bbb-world`
  `local_player_rideable_jumping_vehicle_cooldown_tracks_camel_dash_cooldown`；
  `bbb-renderer`
  `jump_bar_offscreen_frame_replaces_experience_bar_and_uses_cooldown_overlay`。

### 2026-07-06 迁入：生命 heart 变体 + 多行堆叠（P2 HUD 队列该行末片）

- 投影链：`RendererFrame.hud_player_health`（新 `HudPlayerHealth`）取代旧单行
  `hud_health: f32`，携带 health、MAX_HEALTH 属性、absorption、基础 `HeartType`、
  hardcore、Regeneration 门与 client tick。world 侧新增
  `WorldStore::local_player_max_health`（MAX_HEALTH 属性，registry index 19，
  默认 20.0）、`local_player_is_fully_frozen`（`EntityStore::is_fully_frozen` =
  `ticksFrozen >= 140`，从抖动身体判定里抽出复用）；复用已存的
  `local_player_absorption`；login `hardcore` flag 现存入 `WorldGameplayState`
  （`WorldStore::is_hardcore`）。`HeartType.forPlayer` 优先级
  （Gui.java:1438-1450）= poison > wither > fully-frozen > normal，MobEffect id
  按 0 起注册序推导（MobEffects.java：regeneration=9、poison=18、wither=19，与既有
  night_vision=15 / hunger=16 互证）。
- 渲染：`HudHeartKind`（Container/Normal/Poisoned/Withered/Absorbing/Frozen）
  带 `sprite_name(hardcore, half, blinking)`，复现 vanilla 资产命名不对称
  （Normal 前缀 `hardcore_`，带类型的 kind 把 hardcore 嵌在自身前缀之后，
  Container 追加 `_hardcore` 且忽略 half）；sprite 按 `[kind][variant]` 存，
  asset loader 遍历全组合上传（blink 变体不上传）。`hud_player_heart_instances`
  重放 `extractHearts`（Gui.java:820-873）：递减 container 循环画 Container →
  absorbing 叠加（`WITHERED` 保留自身 sprite，否则 `ABSORBING`）→ 基础 fill，
  `xLeft = guiWidth/2-91` 按 `healthRowHeight` 向上堆叠
  （`numHealthRows = ceil((maxHealth+ceil(absorption))/2/10)`，
  `healthRowHeight = max(10-(numHealthRows-2),3)`）。Regeneration 波把
  container `tickCount % ceil(maxHealth+5)` 抬 2px；`currentHealth+absorption
  <= 4` 时每心按 `nextInt(2)` 抖动，种子 `tickCount*312871` 的
  `LegacyRandomSource`——精确复现（vanilla 在 Gui.java:764 重播种，故不同于
  food/air 的 wall-clock 抖动，本片与 vanilla 序列一致）。`armor_hud_rect` 改吃
  投影 `(numHealthRows, healthRowHeight)`，多行生命把护甲行随心上抬
  （`yLineBase-(numHealthRows-1)*healthRowHeight-10`，Gui.java:801；单行默认保持
  原 10px 间距，无回归）。
- 边界（如实 defer）：受击/回血 **blink** 闪烁未实现——需未追踪的
  `player.invulnerableTime`（客户端无同步）与 wall-clock `displayHealth`/
  `lastHealthTime` 延迟保持，均不可确定性复现；`HudPlayerHealth` 恒以
  `blinking = false` 绘制，但 `HeartType::sprite_name` 与已上传的 `*_blinking`
  命名保持完整（矩阵测试覆盖），待 invulnerableTime 落地即可接。
- 测试：bbb-world（hardcore login flag、MAX_HEALTH 属性 + 20.0 回退、140 tick
  冻结阈值）；bbb-renderer layout（`hud_health_rows` 行数/行高、半/空心拆分、
  基础类型跟随、absorption 追加含奇数半心 + withered 覆盖、2 行堆叠、regen 2px
  抬升索引手算、低血抖动序列逐 draw 重放校验、护甲多行上抬）+ sprite 名矩阵
  （kind×hardcore×half×blink 全组合命中真实 vanilla 资产 + hardcore 命名不对称）
  + 一个离屏 readback sentinel（poison 换基础 fill sprite）；氧气/坐骑/护甲联动
  y 布局测试无回归。四条门禁全绿。

### 2026-07-06 迁入：氧气泡条 + 坐骑血量条（含 world metadata 补链）

- world metadata 补链（索引全部按 vanilla 26.1 继承链逐类推导并写入常量注释，
  不从既有测试反推）：`Entity.DATA_AIR_SUPPLY_ID` = 1（Entity.java:255-271
  字段序：SHARED_FLAGS 0 → AIR_SUPPLY 1 → … → TICKS_FROZEN 7）；
  `LivingEntity.DATA_HEALTH_ID` = 9（LivingEntity.java:178-186：FLAGS 8 →
  HEALTH 9 → … → SLEEPING_POS 14）；`Player.DATA_PLAYER_ABSORPTION_ID` = 17
  （Avatar.java:38-39 先占 MAIN_HAND 15 / MODE_CUSTOMISATION 16，
  Player.java:134-139 再 ABSORPTION 17 / SCORE 18 / 肩鹦鹉 19-20，与既有
  肩鹦鹉常量 19/20 互证）。`EntityStore` 新增 `metadata_float` 与
  `air_supply`（默认 300 = define 期 `getMaxAirSupply()`，Entity.java:312）/
  `living_entity_health`（默认 1.0F，LivingEntity.java:314）/
  `player_absorption`（默认 0.0F，Player.java:224）查询；absorption 本片只存
  不画（供 heart 变体片直接用，`WorldStore::local_player_absorption`）。
- 氧气泡条（vanilla `Gui.extractAirBubbles`，Gui.java:887-928 逐行转写）：
  可见门 `isUnderWater || clamp(air,0,max) < max`（:891）；
  `getCurrentAirSupplyBubble` = `Mth.ceil((cur+offset)*10/(float)max)`
  （:922-924），full 用 offset -2、popping 位置用 0、empty = 10 - （offset =
  水下且 cur≠0 时 1 的一拍回填延迟，:926-928），popping 帧仅水下画（:906，
  `hud/air_bursting`），full/empty 间的延迟格当拍不画任何 sprite；全空 +
  偶数 tick 时 empty 壳按 `nextInt(2)` 下坠 wobble（:910，照 food shake 的
  帧计数种子 LCG 方案）。y 线完整重放 `extractPlayerHealth` +
  `getAirBubbleYLine`（:772,784-792,917-920）：`(guiHeight-39)-10`，无坐骑
  hearts 再 -10，随后 `-(ceil(vehicleHearts/10)-1)*10`（0 hearts 时 -1 行=
  +10），步行与 1 行坐骑 hearts 都落在 `guiHeight-49`；x 与 food 同右缘
  `xRight = guiWidth/2+91`、`-(i)*8-9`（:903）。max air 固定 300
  （`Entity.getMaxAirSupply`，Entity.java:2725-2727，Player 不覆写）。
- 坐骑血量条（vanilla `Gui.extractVehicleHealth`/`getVehicleMaxHearts`，
  Gui.java:709-741,974-1005）：坐骑 = local player 直接 vehicle 且为
  LivingEntity（`showVehicleHealth()` 基类即 `instanceof LivingEntity`，
  Entity.java:2349-2351，26.1 无覆写）；hearts = `(int)(maxHealth+0.5F)/2`
  上限 30；`currentHealth = ceil(getHealth())`，每行 20 半心
  （`baseHealth += 20`），container 恒画、`i*2+1+base` 与之比较取
  full/half；行自 `guiHeight-39` 向上 10px 堆叠；hearts>0 时 food 行被替换
  （:784-788）。max health 数据链：vanilla 读 MAX_HEALTH attribute
  （registry index 19，Attributes.java 字段序，与既有 armor 0/gravity 14/
  movement_speed 22 同源推导），bbb 既有 `apply_update_attributes` 对所有
  living entity 存 UpdateAttributes 且 vanilla `ServerEntity.sendPairingData`
  （ServerEntity.java:282-284）在开始追踪时必发 syncable attributes，故链路
  已通；未同步窗口回退 `Attributes.MAX_HEALTH` 默认 20.0（Attributes.java:
  58-60），如实记账。
- 投影链守 RendererFrame 单次提交：`WorldStore::local_player_air_supply/
  local_player_max_air_supply/local_player_vehicle_health`（+ 既有
  `local_player_eye_in_water`）→ `RendererFrame.hud_air`（`HudAirSupply`）/
  `hud_vehicle_health`（`HudVehicleHealth`）→ `set_hud_air`/
  `set_hud_vehicle_health` → `collect_hud_draws`（food 块加 vehicleHearts==0
  门，air 块在 food 后、vehicle hearts 块再后，照 vanilla :790-791/:523-526
  顺序）；6 个新 sprite `hud/air{,_bursting,_empty}`、
  `hud/heart/vehicle_{container,full,half}`（Gui.java:103-108）照 armor 的
  gui atlas 上传模式。
- 边界：bubble 爆裂音 `playAirBubblePoppedSound`（Gui.java:930-937）延后
  （HUD 侧尚无 sound 出口）；heart 变体（absorption/poison/wither/hardcore/
  regen 闪烁/多行生命堆叠）为下一片。
- 测试：bbb-world 三条 metadata 链（air 默认 300/同步 150、absorption 默认
  0/同步 8.0、vehicle health 活体 horse 7.0+15.0 / 无 attribute 回退 20 +
  metadata 默认 1.0 / boat None，索引推导链注释在测试内重述）；bbb-renderer
  布局（气泡行 671 与 food 上一行、坐骑 hearts 2/3 行抬升 661/651、vehicle
  行 681 与 food 同线）、公式手算（300 水下全 full、150 水下 5 full+1 空拍
  +4 empty、61 水下 idx2 popping / 岸上同值 popping 被抑制、0 与负值全
  empty、visible 门四态、wobble 全空+偶 tick 门、max hearts 20→10/15→7/
  15.5→8/100→30 cap/1→0、vehicle fill 跨行 22/21 半心）；离屏 sentinel 两条
  （水下满泡点亮 vs 岸上满氧背景；坐骑 hearts 替换 food 行 + 0-heart 坐骑保
  留 food 行）。既有 hearts/food/armor 测试无回归。

### 2026-07-06 迁入：护甲条渲染

- HUD status-bar 第三片完成：护甲条从既有 attributes 派生。
  `WorldStore::local_player_armor_value`（bbb-world `client/local_player.rs`）
  转写 vanilla `LivingEntity.getArmorValue()` =
  `Mth.floor(getAttributeValue(Attributes.ARMOR))`（LivingEntity.java:1845-1846）：
  已存的 synced ARMOR attribute（`BuiltInRegistries.ATTRIBUTE` 注册序 index
  `0`，Attributes.java:10 首个注册，与 movement 侧既有 gravity 14 /
  jump_strength 15 / movement_speed 22 同源手写常量）经既有
  `AttributeInstance.calculateValue`（`entities::store::vanilla_attribute_value`：
  先 add，再 multiply_base，再 multiply_total）折算后取 floor；无 attribute
  时按 `RangedAttribute` 默认返回 0。
- 投影链：`RendererFrame.hud_armor`（native `runtime.rs` 单次采样
  `world.local_player_armor_value()` → `render_extract.rs` →
  `Renderer::set_hud_armor`），不新增散置 `renderer.set_*`（守 RendererFrame
  单次提交不变量）。绘制在 `collect_hud_draws` 位于 hearts 之前（vanilla
  `Gui.extractPlayerHealth` 先 armor 后 hearts，Gui.java:779/781），仅
  `armor > 0` 才画（`Gui.extractArmor`，Gui.java:800）；10 格逐格按
  `hud_armor_fill`（`i*2+1` 与 armor 比较：`<` full / `==` half / `>` empty，
  Gui.java:805-814）取 `hud/armor_{full,half,empty}` sprite（Gui.java:94-96，
  照 hearts/food 的 pack 上传模式），`armor_hud_rect` 与 hearts 同左边
  （`xLeft = guiWidth/2 - 91` + `i*8`）、上移一行 10px。
- vanilla 常数（26.1 逐条核实）：`yLineArmor = yLineBase - (numHealthRows-1)
  *healthRowHeight - 10`（Gui.java:801），bbb 目前只投影单行生命（未接
  `maxHealth`/absorption 多行），`numHealthRows == 1` 使堆叠项归零，护甲行落
  在 `yLineBase - 10` = `surface_height - 49`；9x9 sprite。
- 边界：多行生命堆叠（absorption/`maxHealth` 行数抬高护甲行）随其余 heart
  variant 一并延后，护甲行用单行偏移；氧气泡/坐骑血量仍缓（需先补 world
  metadata）。
- 测试：bbb-world attribute→armor 派生（floor + 三段 modifier 公式手算
  9.9→9 + 无 attribute/无玩家默认 0）；bbb-renderer `hud_armor_fill` 组合
  （armor 7 → 3 full/1 half/6 empty、满 20 全 full、奇数 1 → slot0 half）与
  `armor_hud_rect` 布局常数（同左边、上移恰一行）；离屏整帧 sentinel 证明
  armor>0 时护甲行像素点亮、armor==0 时保持背景（>0 门控）。既有 hearts/food
  测试无回归。

### 2026-07-05 迁入：actionbar + titles + subtitles 渲染

- HUD overlay 首片完成：actionbar（overlay message）与 title/subtitle 从
  world 状态经 `RendererFrame` 投影（`HudActionBarText`/`HudTitleText`：
  styled runs + post-tick 剩余 tick + fade 窗口 + partial tick + jukebox
  `animate_color` flag）到 `collect_hud_draws` 三个绘制分支，全部走既有
  styled-text 管线（`hud_styled_text_pass_geometry` 增加 pose `scale` 入参，
  1.0 与 label 路径逐位一致；4x title / 2x subtitle 的 pen/字格/阴影偏移/
  effect bar 全量等比，等价 vanilla PoseStack.scale）。
- tick 链补齐：`WorldStore::advance_hud_text_ticks` = vanilla `Gui.tick`
  （Gui.java:1152-1166，Minecraft.tick 每客户端 tick 调用、不受 tick-rate
  freeze 门控 → 用 raw client ticks 而非 running ticks；titleTime 归零时清
  title+subtitle，overlay 计时地板 0 保留文本）。renderer 不自攒状态，每帧
  由剩余 tick + partialTick 现算 alpha。
- vanilla 常数（26.1 逐条核实）：overlay fade `(int)(t*255/20)` 上限 255、
  丢弃门 `alpha > 0`、位置 `(guiWidth/2, guiHeight-68)` + `(-w/2, -4)`
  （Gui.java:308-336）；title fade-in `(total-t)*255/fadeIn`、fade-out
  `t*255/fadeOut`、clamp 0..255、屏心 pose + 4x `(-w/2, -10)`、subtitle 2x
  `(-w/2, 5)`、`ARGB.white(alpha)`（Gui.java:338-377）；彩虹分支
  `Mth.hsvToArgb(t/50, 0.7, 0.6, alpha)`（hue 由剩余时间驱动、确定性；
  h mod 6 但 f 不回卷的 Java quirk 原样保留，Mth.java:451-497）。26.1 无
  旧版 `alpha < 8` 丢弃、无低 alpha 强制不透明。
- 边界：协议层仍将这三类 component 压平为纯文本（单 plain run 投影）；
  accessibility text backdrop（默认 opacity 0）跳过；jukebox now-playing
  （唯一 `animate_color=true` 生产者）未接，flag 已全链路携带；
  `extractSubtitleOverlay` 是声音字幕 overlay（非 title subtitle），仍缓。
- 测试：bbb-world tick 倒数/清理/地板；bbb-native 投影字段透传 + tick 先于
  投影的源序锁；bbb-renderer alpha 公式、彩虹 quirk 确定性、居中/缩放原点、
  scale 几何（字格/阴影/effect bar）、首帧 fade-in alpha=0 丢弃、绘制次序
  （status bars 之后、screen 之前）源序锁。

### 2026-07-05 迁入：boss bars 渲染

- HUD overlay 第二片完成：boss bars 从 `ClientHudState.boss_bars` 经
  `RendererFrame.hud_boss_bars`（`HudBossBar`：plain-run 名称 + 最新包
  progress + `HudBossBarColor`/`HudBossBarOverlay` 枚举，`name()` 词汇即
  vanilla `BossEvent` getName）投影到 `collect_hud_draws`，绘制点位于
  status bars 之后、overlay message 之前（Gui.java:203-217 顺序）。
- vanilla 常数（26.1 逐条核实，BossHealthOverlay.java）：182x5 sheet、
  `x = guiWidth/2 - 91`、y 自 12 起每条 +（10+9）、先画后判 `guiHeight/3`
  截断（:63-77，首条永画）；每条 bar 层序 colored background → notched
  background →（width>0 时）colored progress → notched progress，裁剪宽度
  `Mth.lerpDiscrete(progress,0,182) = floor(p*181)+(p>0?1:0)`（:84-106，
  Mth.java:527-531），UV 取左侧 `width/182` 带；名称居中
  `(guiWidth/2 - w/2, y-9)` 不透明白 + 默认阴影（:71-73）。22 张
  `boss_bar/*` sprite 全部经 vanilla GUI atlas 单图上传（与 crosshair
  同路径，资产树齐全）。
- 边界：投影按 UUID 序（world 用 BTreeMap，vanilla LinkedHashMap 的包到达
  序未建模）；progress 为最新包值（`LerpingBossEvent` 100ms wall-clock 平滑
  未建模）；darken_screen/create_world_fog 仍是 world 侧
  `boss_overlay_should_*` 查询（无天空/雾消费者）、play_music 无音频消费
  者——三者继续 defer。
- 测试：bbb-native 投影（UUID 序/字段透传/风格更新与移除/全 7 色 x 5
  overlay 无丢弃）；bbb-renderer 层序（notched 双层、width=0 跳 fill）、
  lerpDiscrete 公式与 UV 裁剪、行堆叠与 guiHeight/3 截断（首条永画）、
  名称居中原点、getName 词汇 round-trip 与 ordinal-1 notched 索引、
  progress sanitize clamp、绘制次序源序锁（status bars 后、overlay
  message 前）；layout 矩形常数。

### 2026-07-05 迁入：经验等级数字 + 饥饿 hunger-effect 抖动

- HUD 队列第三片完成（一行两小件）。原 goal.md P2 缺口行
  「经验等级数字（level 已在 world，未投影）；饥饿 hunger-effect 抖动」删除。
- 件 1 经验等级数字：`experience.level` 经 `RendererFrame.hud_experience_level`
  投影，`set_hud_experience_level` 按 vanilla `Gui.java:533` 的 `> 0` 门控
  （`hasExperience()` game-mode 门未建模——bbb 有经验态即画经验 HUD，与既有
  progress bar 投影一致）。绘制点在 `collect_hud_draws` 食物行之后、boss
  overlay 之前（vanilla `Gui.extractRenderState` 顺序）；居中
  `x = (guiWidth - font.width)/2`、`y = guiHeight - 24 - 9 - 2`，复用既有
  styled-text 管线：四个 `-16777216` 黑色 `(±1,0)/(0,±1)` 描边拷贝 + 最后
  `-8323296`（0x80FF20）绿色中心，全部 `dropShadow=false`
  （ContextualBarRenderer.java:35-44；lang `gui.experience.level = "%s"` 即只画
  数字）。vanilla 该文本独立于 contextual bar 绘制，故 jump/locator bar 无需
  抑制（bbb 本就不跟踪 jump/locator 态）。
- 件 2 饥饿抖动：镜像 `Gui.extractFood`（Gui.java:958-960）——逐图标
  `yo += random.nextInt(3) - 1`（∈{-1,0,1}），同一 index 的 empty 背景与
  half/full 填充共用该偏移；门控 `saturation <= 0 && tickCount %
  (foodLevel*3+1) == 0`。tick 模数读真实客户端 tick
  （`LightmapTickState.client_tick_count`）；偏移 LCG 用与 vanilla 完全一致的
  `nextInt(3)` 克隆（`HudObfuscatedRandom`），但每帧以渲染帧计数器重新播种
  （vanilla 的 wall-clock `RandomSource` 序列不可复现，改为确定性逐帧闪动）。
- 件 2 hunger 药水变体：`MobEffects.HUNGER`（registry id 16，据 MobEffects.java:70
  经原始 `holderRegistry` stream codec 推得，非 +1）激活时食物行改画
  `food_{empty,half,full}_hunger` sprite（`hud_assets.rs` 加载），变体未上传时
  回退基础 sprite。hunger 数据链已就绪（`world.entity_effect(local_player_id,
  16)`），无需新增 packet 处理。
- 边界：无 game-mode 门（创造模式仍显示经验 HUD）；抖动种子按设计偏离
  vanilla 的 wall-clock 序列。
- 测试：bbb-renderer layout——`hud_food_jitter_offsets`（saturation>0 不抖 /
  tick 模数未命中不抖 / 命中时偏移 ∈{-1,0,1} 且对已知 seed 锁定 LCG 序列 /
  food=0 除数=1 恒触发 / 同 seed 确定性）、`food_hud_rect` y 偏移、
  `hud_experience_level_text_origin` 居中与 Java 整除截断；bbb-renderer hud——
  经验门控 `> 0`、四黑一绿描边 pass 偏移与颜色顺序、hunger sprite 变体选择、
  绘制次序源序锁（食物后、boss 前）；既有 food/layout 测试无回归。

### 2026-07-06 迁入：离屏整帧 readback harness

- HUD 队列基建片完成。原 goal.md P2 缺口行「离屏整帧 readback harness：
  render() 脱离 surface 依赖…」删除。
- 注入点：帧获取从 `render()` 内联 `surface.get_current_texture()` 收进
  `RenderSurface::acquire_frame`（renderer.rs 新枚举：`Window(Surface)` +
  `#[cfg(test)] Offscreen(Arc<Texture>)`），返回 `FrameTarget`
  （`texture()`/`present()`；Offscreen 的 present 为 no-op）。四个吃 frame 的
  step（transparency_blit / first_person_item / hud_passes / finish_frame）
  签名改收 `FrameTarget`；surface 路径语义逐字节不变（Lost/Outdated
  reconfigure+跳帧、Timeout 跳帧、present/screenshot 链原样）。42 条
  render.rs 源序断言与 FRAME_STEPS 双向 meta 测试全部原样通过——render()
  体内只改了获取行与一条注释（`self.surface.acquire_frame(` 因中间有 `.`
  不计入 step 计数，无需动 FRAME_STEPS）。
- 构造拆分：`Renderer::new`（窗口/adapter 协商）→ `with_gpu`（全部
  pipeline/target 构造，单一来源）；`Renderer::new_offscreen(w,h)`
  （cfg(test)，无 adapter 则 None→测试跳过）在 `Bgra8UnormSrgb` 离屏
  target 上建出完整生产 pipeline 集。readback 单源：`finish_screenshot`
  拆出 `read_screenshot_pixels`（256 字节 padded-row + BGRA→RGBA 的唯一
  实现），PNG 保存变薄包装；`render_offscreen_frame()` = 整帧 render() +
  共享 screenshot copy 路径读回 `ScreenshotPixels`。
- 证明测试 `offscreen_frame_renders_hud_sentinel_over_clear_color`
  （offscreen.rs）：320x240、蓝 clear + 居中 4x4 红 crosshair——中心像素红、
  角落蓝，counters 证整帧执行（frame_index=1、hud_draw_calls≥1、
  draw_calls≥4）；llvmpipe 实跑通过。
- 迁移范例：`hud_block_item_renders_visible_pixels_in_its_slot` 由 ~230 行
  手搓 device/pipeline/pass/readback 改为 harness + 公开状态 API
  （update_terrain_texture_atlas / set_hud_hotbar_block_item_models /
  update_camera），断言不变。余量（后续机械迁移，见账本 boundary）：hud.rs
  PIP 测试、item_models.rs 第一人称持物、entity_models player/ender_dragon
  像素测试。
- 顺带修复 harness 首跑揪出的两个潜伏 shader bug（此前无测试构造过完整
  pipeline 集，二者会让生产启动在 create_shader_module 直接 panic）：
  translucent-emissive 实体 shader 的 WGSL 非法 swizzle 赋值
  （`texel.rgb = mix(...)`，2026-06-30 引入）改为重组 vec4；outline 后处理
  四个 shader 的 `let` 数组动态下标（naga 要求 `var`，2026-06-29 引入）改
  `var`。

## 历史 audit 快照

### 2026-07-03（dolphin event slice 后复核，原 goal.md 当前边界）

- 最新 audit 计数（2026-07-03 dolphin event slice 后复核）：
  - `rg residual`：37 行（分类不变，均非 P0 类）
  - `rg fallback`：899 行（ledger / vanilla fallback 文本，分类不变，均非
    P0 类）
  - `rg unsupported`：156 行（分类不变）

## 架构重构批次记录（2026-07-02 / 2026-07-04，原 goal.md 当前边界）

- 2026-07-02 架构重构 1-7 全部完成（13 commits，2d19d2a3..cae7bcbf；进度与
  方法论见 memory `architecture-refactor-progress`）：包分发单点化
  （apply_play_packet + NetEvent 收敛）、entity_scene/item_runtime 拆分、
  实体 ID 常量下沉、EntityState 反转、renderer pipeline builder + 持久帧
  buffer + render() pass 拆分、render_extract 层 + RendererFrame、
  ItemProfiles 子 store、bbb-render-types 叶 crate。上方结构不变量即其产出。
- 2026-07-04 旧账清偿批次全部完成（13 commits，1de8f14b..44244ea3）：
  FRAME_STEPS 补登记+反向断言、粒子 id 常量注册表派生、四大文件内联测试
  外抽（~2.4 万行）、particle_runtime/item_models/renderer particles 拆
  facade+子模块、WorldCounters 宏化、两大测试套件域分组（13+9 子文件）、
  bbb-platform crate 移除（WindowConfig 下沉 native）、音效 seed 确定性化
  （bbb-world 零 wall-clock）、ControlSnapshot 反转（SharedWorld 共享 +
  每帧深拷贝清零）、inventory 子 store 化（InventoryCtx 模式）、账本超大
  条目拆 docs/unsupported/。2026-07-02 评估的 #8-11 旧账全部清偿；上方
  2026-07-04 不变量即其产出。

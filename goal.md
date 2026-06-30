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
  - `rg residual`：80 行
  - `rg fallback`：576 行
  - `rg unsupported`：189 行
- 当前 renderer code-side 分类：
  - `residual` 只剩 `entity_models/dispatch.rs` 的“无 residual mesh-emitting arm”注释。
  - `unsupported` 是 screenshot surface format bail、dynamic-player texture
    render-type defensive panic 和 docs 指针。
  - `fallback` 是 colored debug/profile/terrain/HUD/map/test 或 vanilla fallback 注释。
- 用户已明确恢复 P1 post-closeout parity；当前按 P1-1 狭义 render-state /
  render-graph fidelity 推进。只有命中下方重新打开条件时才回到 P0。

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
    跳过 LightTexture；动态 texture-matrix time offset 与其他 item glint
    变体仍属后续 shader/state 细化。
  - no-cull / cull / z-offset / translucent / translucent-cull item-target。
  - [x] `Eyes` emissive alpha blend：spider / enderman / phantom /
    ender-dragon 等 `RenderTypes.eyes` 提交保留独立 eyes mesh / shader，
    GPU pipeline 使用 vanilla `BlendFunction.TRANSLUCENT`、depth-write
    disabled、depth-test `LESS_EQUAL`、cull off、EMISSIVE / NO_OVERLAY /
    NO_CARDINAL_LIGHTING shape，并跳过 LightTexture。
  - `armorCutoutNoCull` / `armorTranslucent`。
  - `breezeWind` lightmap-lit scroll。
  - [x] `energySwirl` emissive additive scroll：charged creeper / wither
    overlay 提交进入独立 additive scroll mesh / shader，使用 vanilla
    `BlendFunction.ADDITIVE` (`ONE`, `ONE`)、depth-write `LESS_EQUAL`、
    cull off、alpha cutout 0.1、EMISSIVE / NO_OVERLAY shader shape，并跳过
    LightTexture 采样；更细的跨 bucket 透明排序仍归 sorting 项。
  - `waterMask` depth-only 或等价 boat mask state。
  - `end_crystal_beam` / guardian beam custom prism state。
- target ownership：
  - main target、itemEntity target、translucent target、particles target、
    weather target、clouds target、entity_outline target 的 draw ownership 继续收紧。
  - text / item / block / crumbling / line / selection 等 feature pass 的相对顺序
    继续按 vanilla `LevelRenderer` 和 `FeatureRenderDispatcher` 拆分。
- sorting：
  - blended texture-backed model submit 的 camera-distance sort。
  - terrain translucent 与 entity translucent 的跨 target / cross bucket 顺序。
  - particles translucent order 与 itemEntity target 的交界。
- shader / sampler state：
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

## Slice 选择顺序（当前有效）

1. 开始前先看 `git status --short`。若存在已验证但未提交的小 slice，先完成
   提交；若只是未验证文档整理或 audit 结果，先验证后再提交。
2. 如果 audit 发现新的 direct mesh bypass、texture-backed / dispatch-owned
   submission gap，或 RenderType/order/submit_sequence/missing-atlas/
   dynamic texture/light/overlay/outline 相关 blocker，下一 slice 只能是关闭
   该 checklist 项的 P0 工作。
3. 如果没有新的 P0 blocker，不自动开启新的 P1/P2/P3 slice；必须由用户显式
   恢复 post-closeout parity 方向。
4. 恢复 post-closeout parity 后，优先选择能关闭整类 completion standard 的小
   slice，而不是继续补零散 provider / 动画 / GUI 细节。
5. 粒子 provider、terrain、HUD、first-person、GUI 和开放式实体细节只有在
   被显式选中或直接阻塞 P0 checklist 时才展开。
6. residual / fallback / unsupported audit 中发现的 stale 文案先归类或清理；
   如果没有新的 direct mesh bypass，不重新打开狭义 pipeline closeout。

每个 slice 开始前先 grep 当前实现，确认该 feature 确实缺失或测试不足；历史上多次出现
“ledger 以为缺失但代码已实现”的情况。

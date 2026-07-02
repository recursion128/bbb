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
  - `rg fallback`：753 行
  - `rg unsupported`：154 行
- 当前 renderer code-side 分类：
  - `residual` 只剩 `entity_models/dispatch.rs` 的“无 residual mesh-emitting arm”注释。
  - `unsupported` 是 screenshot surface format bail、dynamic-player texture
    render-type defensive panic、pack parser validation bail、unknown packet /
    component diagnostics、tests 和 docs 指针。
  - `fallback` 是 colored debug/profile/terrain/HUD/map/test 或 vanilla fallback 注释。
- [x] Slice selector 现在只允许两个结果：命中新的 P0 重新打开条件则回到
  P0 regression；否则必须自动开启下一条 P1 slice，不再等待额外确认。
- 用户已明确恢复 P1 post-closeout parity；P1-1 狭义 render-state /
  render-graph fidelity、P1-2 实体专用 renderer closeout、P1-4 GUI lighting /
  entity-in-UI render-plan surface 当前已收口。
  P1-3 item / frame / first-person presentation 的剩余项主要是 first-person
  viewmodel 和 touchscreen snapback 这类较宽工作；creative inventory-tab
  preview 需要 creative screen state，entity preview actual GPU PIP drawing
  属于后续视觉渲染面。只有命中下方重新打开条件时才回到 P0。

## Slice 选择顺序（P0 blocker-gated P1 auto-start）

这是 gate-driven 顺序，不是固定的 P0 backlog 扫描。quick P0 audit 或提交后
audit 未命中新的 “P0 重新打开条件”时，结论视为 “没有新的 P0 blocker”。
同一轮必须自动开启新的 P1 slice，并直接进入实现、验证、提交；不要停在等待
确认状态，也不要因为历史 ledger 里的旧 P0/P1 文字重新开启 P0。
换言之，P1 auto-start 是默认分支；只有新的 P0 blocker 能抢占。

1. 每轮先确认当前工作树和未提交 slice；如果已有已验证但未提交的小 slice，
   先完成提交。
2. 运行或复核 quick P0 audit：`rg residual`、`rg fallback`、`rg unsupported`，
   并只按下方 “P0 重新打开条件” 判断是否回到 P0。
3. 如果 audit 命中新的 P0 blocker，开启 P0 pipeline regression slice；该
   slice 必须只修复 blocker，不顺手吸收 P1/P2 视觉 parity。
4. 如果 audit 未命中新的 P0 blocker，本轮立即自动开启新的 P1 slice，并进入
   实现、验证、提交流程；不能停在“等待用户确认 P1”的状态。
5. P1 自动选择规则：先从当前 P1 子队列选首个能用 vanilla 源码旁证、能用
   focused test 或 ledger 更新验证的小 slice；当前子队列无可执行项时，
   顺延到下一条 P1 子队列。
6. P1 slice 已完成 `P1-1` render-state / render-graph fidelity closeout 和
   `P1-2` 狭义实体 renderer closeout；后续默认直接执行当前 P1 子队列的
   下一条小 slice，除非 quick audit 重新打开 P0。
7. P0 clean 时不得停下等待确认；粒子 provider、terrain、HUD/GUI 大面或
   first-person 宽面只按能关闭当前 P1 checklist 的最小 slice 进入，避免开放式
   细节补完重新吞掉 selector。

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

已完成项见 docs/goal-archive.md#p1-1gpu-render-state--render-graph-fidelity。

- target ownership：
  - main target、itemEntity target、translucent target、particles target、
    weather target、clouds target、entity_outline target 的 draw ownership 继续收紧。
  - text / item / block / crumbling / line / selection 等 feature pass 的相对顺序
    继续按 vanilla `LevelRenderer` 和 `FeatureRenderDispatcher` 拆分。

完成标准：

- 每个 GPU state slice 有 vanilla `RenderTypes.*`、shader json、post-chain 或
  `LevelRenderer` 依据。
- 测试覆盖 render plan / pipeline key / target order；能 readback 的视觉路径补
  deterministic pixel proof。

### P1-2：实体专用 Renderer 行为

目标：补齐已经有模型和贴图但 renderer 行为仍缺官方细节的实体。

仍在推进：

已完成项见 docs/goal-archive.md#p1-2实体专用-renderer-行为。

- Boss / beam / emissive 类：
  - GPU `DISSOLVE` mask sampling 精度继续作为后续视觉 parity，不阻塞本轮
    death rays slice。

完成标准：

- 每个实体差异必须先定位 vanilla renderer/model/layer 源码，再改测试。
- 不再新增只验证 vertex count 的 textured regression。
- 对每个特殊 renderer branch 至少有一个状态化测试。
- 当前 P1-2 狭义实体 renderer closeout 已完成；剩余声效、粒子、运动控制、
  block-entity special renderer、datapack/custom asset 和 attachment consumer
  presentation 均已拆出到 P1-5/P2/P3 或非 renderer backlog。

### P1-3：物品、Frame 与第一人称表现

目标：把 item model pipeline 从“主要消费者可画”推进到 vanilla presentation parity。

仍在推进：

已完成项见 docs/goal-archive.md#p1-3物品frame-与第一人称表现。

- item enchantment glint（P1-1 closeout 归入 P1-3）：`entityGlint` GPU 管线已存在可复用，
  需 `has_foil` 跨 crate 投影 + item glint mesh/pipeline，属中型 GPU slice（像素级验证较难）。
  item-model select-property resolver 已高度成熟（local_time `G`/`u`/`D` 已收口最后的静默
  fallback 缺口），glint 是当前 P1-3 剩余里价值较高的下一项。
- First-person viewmodel：
  - hand transform。
  - use animation。
  - swing animation。
  - map / bow / crossbow / spyglass / shield 等特殊路径。
- Combat / held item arm pose：
  - third-person hand-use sway。
  - kinetic weapon / ticksUsingItem。
- HUD / inventory：
  - vanilla font / count / durability / cooldown / tooltip / screen depth behavior。

完成标准：

- 每个 item consumer 都以 vanilla `ItemDisplayContext`、display transform 和 renderer 源码为依据。
- GUI/world 使用不同 lighting context 时必须在测试或手动对比记录中说明。

### P1-4：GUI Lighting Surface / Entity-In-UI

目标：把 GUI flat、GUI 3D、entity-in-UI 的 lighting context 和 world item/entity
路径分清。

仍在推进：

已完成项见 docs/goal-archive.md#p1-4gui-lighting-surface--entity-in-ui。

完成标准：

- [x] GUI/world/entity-in-UI 三类 lighting context 明确分流。
- [x] 至少一个 GUI flat item、一个 GUI 3D item、一个 entity-in-UI path 有 deterministic
  screenshot/readback 或等价 render-plan 测试。
- [x] P1-4 狭义 surface closeout：GUI flat/generated item、GUI 3D item、
  entity-in-UI lighting/transform/scissor/depth-isolation、smithing result
  armor/skull/wings/ordinary-item/head-item projection、screen call-point audit
  和 GUI/world pass ordering 均已有 vanilla 旁证和 deterministic tests。
  creative inventory-tab preview、actual GPU PIP drawing 和 broader
  layer-order drawing 继续作为后续视觉/screen-state parity，不重新打开 P1-4
  狭义 lighting surface。

### P1-5：透明排序、粒子与 Level Events

目标：补齐当前粒子、透明对象和官方的排序、限制、provider 细节差距。

仍在推进：

已完成项见 docs/goal-archive.md#p1-5透明排序粒子与-level-events。

- 粒子 provider-specific behavior：
  - `falling_dust` 的非 air `RenderShape.INVISIBLE` provider spawn rejection 已
    对齐 vanilla；block-state tint 与 on-ground roll reset 仍随 terrain/collision
    查询推进。
  - `TerrainParticle.createTerrainParticle` 的 air / `moving_piston` /
    `shouldSpawnTerrainParticles=false` provider rejection 已覆盖 `block`、
    `dust_pillar`、`block_crumble`；`block_marker` 保持 vanilla 未过滤分支。
  - 初速度。**已收敛**：smoke 系、ash / white_ash、dust_plume、trial_spawner_detection /
    _ominous 的 base-spread×dir 初速度均已对齐 vanilla（见 goal-archive P1-5）。剩余
    仍用纯 `Command` 初速度的 provider（fishing、bubble_pop、squid_ink、glow_squid_ink、
    enchant、nautilus、totem_of_undying、end_rod、sculk_charge、firework、portal、
    reverse_portal 等）经逐个 vanilla-provider 审计确认本就是把 aux 速度直传 base
    `Particle` ctor，flat `Command` 正确，无 gap。初速度这一档不再有可执行小 slice。
  - lifetime。
  - size curve。
  - alpha/color curve。
  - gravity / collision / player-coupled physics。
- 粒子 sorting：
  - terrain/item particle atlas rendering：resolving block/item atlas sprites,
    applying terrain tint, sprite-transparency-driven `TRANSLUCENT_TERRAIN` /
    `TRANSLUCENT_ITEMS`, binding terrain/items particle atlas textures in the
    GPU path, and transparent terrain/items vertex emission remain follow-up
    work.
- atlas mip / animation：
  - mip-level atlas animation beyond age-selected `SpriteSet` frame selection
    remains follow-up work.
- LevelEvent particle side effects：
  - smoke/flame/dragon-breath/explosion/cloud/block-face/trial-spawner/
    vault activation/portal ring/happy-villager/item-break/composter/known-shape block-destroy
    之外的剩余事件。

完成标准：

- 每个 particle slice 记录 vanilla provider 类和精确公式。
- 对随机行为使用确定性 seed 或固定样本测试。

### P2：Terrain / Block Render Presentation

目标：把 terrain 从基础 mesh 对齐推进到官方视觉细节。

仍在推进：

- 检查 block render shape、face culling、AO、tint、biome tint、fluid overlay 与 vanilla 差异。
  - 流体侧面 + 顶面已按 vanilla `FluidRenderer.addFace(..., addBackFace)` 发射反转背面
    （submerged 视角可见，底面单面）。
  - terrain / fluid 面已按 chunk 所在维度的 vanilla `CardinalLighting` 着色
    （`BlockModelLighter`：shaded 面 `byFace(dir)`、非 shaded 面 `up()`），由
    `DimensionType.cardinalLightType` 选择、经 `WorldStore` 穿进 `TerrainChunkSnapshot`：
    Nether 维度用 `CardinalLighting.NETHER`（`down`/`up`=0.9，侧面同 DEFAULT），其余
    内建维度用 `DEFAULT`。follow-up：`water_overlay` 贴图对 HalfTransparent / Leaves
    邻居的选择（该情形 vanilla 会同时抑制该侧背面）；datapack 维度类型覆盖
    `cardinal_light` 字段暂未解码（回退 DEFAULT）。
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

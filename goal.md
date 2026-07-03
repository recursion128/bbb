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
- `bbb-renderer` 不得依赖 `bbb-pack`；`bbb-item-model` 不得依赖 `bbb-renderer`
  （renderer/native 共享的纯值类型放 `bbb-render-types`）。
- 每个 slice 保持小而可合并，避免顺手重构和无关格式 churn。
- 每个 slice 必须更新 `docs/unsupported-features.md`；需要长期状态记录时同步更新 memory。
- 2026-07-02 架构重构后的结构不变量（新 slice 不得回退）：
  - 新增 clientbound play packet 的处理只进 `bbb-world::apply_play_packet`；
    运行时副作用走 `PlayApplyEffects` trait。probe/dispatcher 不得再各写 arm。
  - 实体 type id 常量一律取自 `bbb_protocol::entity_types`，不得内联数字。
  - 实体热路径（spawn/apply/clone）只写窄组件；`EntityState` 投影仅限
    probe/serialize/debug 边界。
  - world→renderer 每帧状态经 `runtime/render_extract.rs` 的 `RendererFrame`
    单次提交；不得在 pump 中新增散置 `renderer.set_*`。
  - `render()` 是纯编排器：新增 pass/copy 必须做成 step 方法、登记
    `FRAME_STEPS` 并保持"定义顺序==执行顺序"（meta 测试强制）。
  - 新建 render pipeline 用 `pipeline_builder::RenderPipelineBuilder`；每帧重建
    的顶点/索引流用 `frame_buffers::FrameDataBuffer`，不得每帧 `create_buffer_init`。
  - item 侧全局默认表进 `bbb-world` 的 `ItemProfiles` 子 store。
- 提交前默认门禁（当前实际 gate）：
  - `cargo fmt --all --check`
  - `git diff --check`
  - `CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test --workspace`
  - `RUSTFLAGS='-D warnings' CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test -p bbb-renderer --quiet`
  - `RUSTFLAGS='-D warnings' CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test -p bbb-world --quiet`
  - `RUSTFLAGS='-D warnings' CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test -p bbb-pack --quiet`
  - `RUSTFLAGS='-D warnings' CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test -p bbb-native --quiet`
  - `RUSTFLAGS='-D warnings' CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test -p bbb-item-model --quiet`
  - `RUSTFLAGS='-D warnings' CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test -p bbb-render-types --quiet`

## 当前边界

- 狭义 P0 pipeline closeout 不继续吸收开放式视觉 parity。
- P0 lighting / overlay / outline 当前没有 blocker 记录。
- closeout freeze 后连续三轮 audit 均未发现新的 direct mesh bypass、
  texture-backed / dispatch-owned submission gap，或
  RenderType/order/submit_sequence/missing-atlas/dynamic texture/light/overlay/
  outline 相关 P0 blocker。
- 2026-07-02 架构重构 1-7 全部完成（13 commits，2d19d2a3..cae7bcbf；进度与
  方法论见 memory `architecture-refactor-progress`）：包分发单点化
  （apply_play_packet + NetEvent 收敛）、entity_scene/item_runtime 拆分、
  实体 ID 常量下沉、EntityState 反转、renderer pipeline builder + 持久帧
  buffer + render() pass 拆分、render_extract 层 + RendererFrame、
  ItemProfiles 子 store、bbb-render-types 叶 crate。上方结构不变量即其产出。
- 最新 audit 计数（2026-07-03 local_time Julian-day slice 复核）：
  - `rg residual`：38 行（分类不变，均非 P0 类）
  - `rg fallback`：889 行（ledger / vanilla fallback 文本，分类不变，均非
    P0 类）
  - `rg unsupported`：155 行（分类不变）
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
   顺延到下一条 P1 子队列。2026-07-02 重构后的子队列消化顺序调整为：
   P1-1 重构红利项（target ownership / RendererFrame 提取时机核查，低成本
   高确定性）→ P1-5 粒子 provider 细节与 probe 形状上下文 → P1-3
   first-person viewmodel 等宽面。宽面项仍只按能关闭 checklist 的最小
   slice 进入。
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

重构红利（2026-07-02 后优先消化，成本显著降低）：

- target ownership / pass 顺序 slice 现在按 `render.rs` 的 step 方法逐个推进：
  每个 target 是一个独立方法，`FRAME_STEPS` + meta 测试直接表达并强制帧顺序，
  对照 vanilla `LevelRenderer` 的调整从"在 1400 行函数里挪块"变成
  "挪一个方法 + 改一行常量"。
- 新增：`RendererFrame` 逐字段提取时机的 vanilla 对齐核查（小 slice 族）。
  pump 现在把每个 world→renderer 值绑定在原 tick 时序点（如 sky environment
  刻意在 `advance_sky_flash_time` 之前取值）；逐字段对照 vanilla 的
  tick→render 顺序，确认该字段应读 tick 前还是 tick 后状态，正确则加注释
  背书，错误则把该 `let` 下移并补 vanilla 依据。每个字段一个可验证小 slice。

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
- 提取时机核查 slice 的完成标准：该字段在 `RendererFrame` 注释或测试中有明确
  的 vanilla 帧顺序依据（读 tick 前 / tick 后哪一个状态）。

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
  range-dispatch properties. 剩余是截图级 viewmodel 视觉校验。
- First-person viewmodel：
  - 截图级视觉校验。
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
    距离 / 粒子状态门；
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
    `ClientboundTakeItemEntity` 现在按 vanilla 在 shrink/remove 前创建
    `ItemPickupParticle` runtime command：source 使用被拾取实体当前位置/速度，
    target 使用目标 living entity 或本地玩家 fallback 的 `(feet + eyeY) / 2`
    midpoint，item entity 传入 pre-shrink item stack；renderer 将其纳入
    `ITEM_PICKUP` group，按 3 tick lifetime、target old/current 跟随和
    `(life + partial) / 3` 平方插值推进。实际 carried `EntityRenderState` GPU
    submit 仍是后续 entity-submit 渲染面；
    `ClientboundGameEvent` 的 elder-guardian effect 现在按 vanilla 在本地玩家
    脚部位置生成 `minecraft:elder_guardian` 粒子，并在 param floor 为 1 时播放
    `minecraft:entity.elder_guardian.curse`；同组 game event 的
    `minecraft:entity.arrow.hit_player` 与 `minecraft:entity.puffer_fish.sting`
    本地玩家位置声效也已接到 native audio；
    同一 event `35` 现在还按 vanilla `SoundEvents.TOTEM_USE` 在实体当前位置
    播放 `minecraft:item.totem.use` 本地位置声效，source 来自当前实体的
    `getSoundSource()` 映射（player/hostile/default neutral 等）；
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
    剩余 gravity/collision/player-coupled work 是其他特殊 context 和
    player-coupled emitter（不含 TakeItemEntity `ItemPickupParticle` runtime/lifecycle、SpellParticle、本地 PlayerCloud 牵引、
    totem event-35 TrackingEmitter、animate 4/5 crit/enchanted-hit TrackingEmitter、
    GameEvent elder-guardian 粒子、vibration entity target refresh、DragonBreath hit-ground motion 与 SuspendedTown
    collision-free move、Crit constructor tick、Flame/Portal collision-free metadata、PrimedTnt smoke），以及 local sound（不含 DripParticle
    honey/dripstone fall-and-land 落地本地声效、totem event-35
    `minecraft:item.totem.use` 本地位置声效、GameEvent arrow-hit / puffer-fish-sting /
    elder-guardian-curse 本地玩家位置声效、TakeItemEntity item / experience-orb
    pickup 本地位置声效）/ block-state removal gates。
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
    BlockColors 已收敛；firework 非空 explosions 的 `Starter` 子粒子
    fade-color 仍归属 firework 宽面，不再作为 provider alpha/color curve 小项跟踪。
  - gravity / collision / player-coupled physics。
- 粒子 sorting：
  - terrain/item particle atlas rendering：on-ground roll reset 和三轴
    block-shape collision clipping 已通过 native world collision 回调接入；
    EndRod collision-free move 已覆盖，其他 special-context collision /
    player-coupled physics 仍属上一节 deferred work。
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
    内建维度用 `DEFAULT`。水侧面现在按 vanilla 对 HalfTransparent / Leaves
    邻居选择 `water_overlay` 并抑制该侧背面；datapack 维度类型覆盖的
    `cardinal_light` 字段现在从 `minecraft:dimension_type` registry NBT 解码。
- 补齐 selection overlay、block entity 特殊 renderer、透明块排序等剩余 presentation；
  破坏进度的 renderer-visible cube crack overlay 已覆盖官方 `destroy_stage_0..9`
  atlas、本地/服务端同位置取最高 stage、400 render tick 过期和 crumbling
  pipeline state；完整模型形状 crack decal 仍随 block destroy presentation 后续推进。
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

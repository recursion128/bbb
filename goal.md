# 目标：bbb 渲染管线与 Minecraft Java 26.1 对齐

## 总目标

让 `bbb` 的 renderer / world / native / pack / protocol 边界逐步对齐
Minecraft Java 26.1 官方客户端行为。以 `docs/unsupported-features.md` 为事实清单：
除明确决定不支持的 feature 外，其余 renderer 差异必须收敛为 repo-native 实现。

已完成历史不在本文件继续堆叠维护；详见：

- `docs/unsupported-features.md`
- `~/.claude/projects/-home-zgy-Work-bbb/memory/entity-texture-path-status.md`
- 相关专项 memory，例如 entity metadata / proxy entity / texture path 状态

本文件只保留下一阶段目标、优先级和完成标准。

## 硬性约束

- 严格按 vanilla 26.1 源码转写：优先使用 `~/Work/mc-code/sources/26.1/`。
- 协议、metadata 索引、render type、layer order、transform、texture path、tint、light、overlay 不能凭记忆实现。
- `bbb-renderer` 不得依赖 `bbb-pack`。
- 每个 slice 保持小而可合并，避免顺手重构和无关格式 churn。
- 每个 slice 必须更新 `docs/unsupported-features.md`；需要长期状态记录时同步更新 memory。
- 提交前默认门禁：
  - `cargo fmt --all --check`
  - `git diff --check`
  - `CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test --workspace`
  - `RUSTFLAGS='-D warnings' CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test -p bbb-renderer --quiet`
  - `RUSTFLAGS='-D warnings' CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test -p bbb-world --quiet`
  - `RUSTFLAGS='-D warnings' CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test -p bbb-pack --quiet`
  - `rm -f crates/bbb-native/src/lib.rs && touch crates/bbb-native/src/main.rs && RUSTFLAGS='-D warnings' CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test -p bbb-native --quiet > /tmp/bbb-native-test.log 2>&1`
  - `grep -c '^warning' /tmp/bbb-native-test.log` 输出必须为 `0`

## 剩余渲染差异优先级

说明：

- 原始 1-4 优先级已整体完成，并在下方和总览表用 `[x]` 标注。
- 按当前 P0/P1/P2/P3 阶段划分，整体已完成优先级：暂无；没有一个
  P0/P1/P2/P3 章节可以整体标为完成。
- 已完成的优先级内子项也已在下方用 `[x]` 标注，并在总览表汇总。
- 阶段标记：`[x]` = 已完成并进入回归维护；未打勾目标仍按 slice 推进。
- 优先级标题同步使用 `[x]` / `[ ]` 标注整体完成状态。
- 下面用“阶段完成”标注已经落地的部分，用“仍在推进”标注明确剩余项。

### [x] 原始优先级 1-4（状态：已完成）

- [x] 1. 扩展 render type 表达，区分 `entityCutout` / `entityCutoutCull` /
  `entityCutoutZOffset` / `entityTranslucent` / `Eyes` / `EnergySwirl` 等。
- [x] 2. 给 texture-backed submission 记录显式 `order`，对应官方
  `SubmitNodeCollector.order(n)`，并保留 same-order `submit_sequence`。
- [x] 3. 将主要 residual emit 路径逐步迁入 submission 生成；当前
  textured 主 residual arm 已清空并移除，texture-backed residual / bespoke
  layer helper 也已收敛为 dispatch-owned submission 生成；后续只剩 GPU path
  的更细粒度状态迁移。
- [x] 4. 使用 vanilla 26.1 源码旁证测试，覆盖 texture、render type、
  tint、transform、order，不再只测顶点数量。

### 优先级完成总览

| 优先级 | 整体完成 | 阶段完成标注 | 仍需推进 |
| --- | --- | --- | --- |
| [x] 原始优先级 1-4 | [x] 已完成 | [x] render type 细分；[x] 显式 `order` / `submit_sequence`；[x] 主要 residual emit 迁入 submission 生成；[x] vanilla 源码旁证测试覆盖 texture/render type/tint/transform/order | 后续差异已拆入 P0/P1/P2/P3 |
| [ ] P0：提交图与 RenderType 语义 | [ ] 进行中 | [x] RenderType 主要表达；[x] texture-backed submission 显式 `order` / `submit_sequence`；[x] 主要特殊 renderer/layer 的 texture/render type/tint/transform/light/overlay/order 测试覆盖；[x] WindCharge `breezeWind` scroll submit 纳入 dispatch-owned submission 生成；[x] BreezeWindLayer `breezeWind` overlay submit 纳入 dispatch-owned submission 生成；[x] BoatRenderer `waterMask` submit 纳入 dispatch-owned submission 生成；[x] ThrownTrident `entityGlint` foil submit 纳入 dispatch-owned submission 生成；[x] Guardian attack beam submit 纳入 dispatch-owned submission 生成；[x] EndCrystal body submit 纳入 dispatch-owned submission 生成；[x] EndCrystal healing beam submit 纳入 dispatch-owned submission 生成；[x] EnderDragon healing beam submit 纳入 dispatch-owned submission 生成；[x] Player base/ears/cape submit 纳入 dispatch-owned submission 生成；[x] HumanoidArmorLayer submit 纳入 dispatch-owned post-base 生成；[x] CustomHeadLayer skull submit 纳入 dispatch-owned post-armor 生成；[x] WingsLayer submit 纳入 dispatch-owned post-CustomHead 生成；[x] Player ParrotOnShoulderLayer / SpinAttackEffectLayer submit 纳入 dispatch-owned post-Wings 生成；[x] VillagerProfessionLayer submit 纳入 dispatch-owned late hook 生成；[x] Pig saddle submit 纳入 dispatch-owned submission 生成；[x] Strider saddle submit 纳入 dispatch-owned submission 生成；[x] Camel/CamelHusk saddle submit 纳入 dispatch-owned submission 生成；[x] Equine base/markings/saddle/body-armor submit 纳入 dispatch-owned submission 生成；[x] LlamaDecorLayer submit 纳入 dispatch-owned submission 生成；[x] Nautilus/ZombieNautilus saddle/body-armor submit 纳入 dispatch-owned submission 生成；[x] WolfArmorLayer visible / invisible ungated submit 纳入 dispatch-owned submission 生成；[x] Creeper/Wither EnergySwirl submit 纳入 dispatch-owned submission 生成；[x] textured 主 residual arm 清空并移除；[x] texture-backed residual / bespoke layer helpers 收敛为 dispatch-owned submission hooks | GPU path 保留更细粒度状态 |
| [ ] P0：光照、Overlay、Invisible / Outline | [ ] 进行中 | [x] 大量实体 base/layer light 与 overlay metadata pinning；[x] colored/textured entity shader 使用 vanilla-shaped `Lightmap.getBrightness` 曲线；[x] texture-backed entity shader 使用 vanilla `Lighting.setupLevel` 默认方向 diffuse；[x] entity / `breezeWind` lightmap 套用 vanilla `BrightnessFactor` notGamma；[x] entity / `breezeWind` lightmap `BlockFactor` 接入 vanilla block-light flicker；[x] entity / `breezeWind` shader 使用 vanilla-shaped `LightmapInfo` 环境颜色/效果 uniform；[x] world/native 按维度写入 Overworld/Nether/End `LightmapEnvironment`；[x] native Overworld day timeline / rain-thunder weather `SKY_LIGHT_FACTOR` 和 `SKY_LIGHT_COLOR`；[x] native End flash THE_END clock `SkyFactor` boost；[x] native boss-overlay darkening / End flash world-fog `/3`；[x] native 从本地玩家 `night_vision` / `darkness` mob effect 写入 `NightVisionFactor`、`DarknessScale` 和 darkness-adjusted `BrightnessFactor`；[x] native lightmap effect state 跟踪 vanilla client duration、darkness `BlendState` fade 和 conduit `waterVisionTime`；[x] `breezeWind` scroll GPU path 使用 submitted block/sky light，`energySwirl` 保持 emissive；[x] spectator-visible invisible living entity 边界；[x] colored fallback self-visible invisible force-transparent alpha；[x] colored fallback hidden-glowing outline color；[x] texture-backed visible glowing static-atlas outline copy；[x] sheep/wolf white-overlay 与 outline folded geometry gates；[x] texture-backed static-atlas outline bucket 基础 GPU 呈现；[x] wolf body armor invisible layer exception；[x] armor stand marker/non-marker invisible outline branches；[x] armor stand armor invisible layer branch；[x] armor stand WingsLayer / CustomHeadLayer skull invisible branch；[x] armor stand held-item / generic head-item invisible item-model pass；[x] player/humanoid armor、wings、cape gate、held/head item invisible 组合 | biome/spatial camera `EnvironmentAttributes`、End flash hide-lightning/local-clock variants、GUI/entity-in-UI lighting variants、统一 overlay、vanilla outline target / composite |
| [ ] P1：实体专用 renderer 行为 | [ ] 进行中 | [x] 多类实体专用模型/layer；[x] boat/raft paddle rowing、hurt roll、bubble wobble、underwater gate、BoatRenderer-only water mask submission、lighting；[x] parrot PARTY jukebox pose 和 player ON_SHOULDER layer；[x] sheep/wolf invisible/outline/overlay layer gates；[x] armor stand armor/wings/custom-head-skull submissions；[x] armor stand held-item / generic head-item item-model pass；[x] player/humanoid invisible armor/wings/skull/cape/held-item layer gates；[x] baby donkey/mule nested equine setupAnim pose；[x] equine renderer-side `animateTail` tail yRot wag；[x] equine client-random `tailCounter` source projection；[x] equine `isInWater` leg-frequency slowdown；[x] equine eat/stand/feed event pose；[x] camel jumpCooldown / `CAMEL_IDLE` render states；[x] camel `updateWalkAnimation` override；[x] sniffer baby `ModelLayers.SNIFFER_BABY` / `snifflet.png` dispatch；[x] feline lower-tail walk wobble；[x] feline crouch/sprint pose branches | 大型模型、humanoid、beam/emissive 细节 |
| [ ] P1：物品、GUI、Frame 与第一人称表现 | [ ] 进行中 | [x] shared item-model primitive；[x] 主要 third-person consumers；[x] profile skin/cape/elytra decode/cache/upload/sampling；[x] glow item-frame border/item lighting；[x] invisible item-frame no-border / `0.5` item offset；[x] item-frame border fractional model depths；[x] filled-map item-frame full-frame render；[x] filled-map dynamic texture-backed base surface submit；[x] filled-map `MapRenderer` decoration sprite submit；[x] filled-map `MapRenderer` decoration name text submit | first-person viewmodel、hand-use/arm pose、item lighting、HUD/inventory |
| [ ] P1：透明排序、粒子与 Level Events | [ ] 进行中 | [x] 部分 LevelEvent particle side effects；[x] LevelEvent 2006 `dragon_breath` + DragonBreath provider 外观；[x] SuspendedTown HappyVillager/Composter/Mycelium/EggCrack/Dolphin provider 外观/初速度；[x] SuspendedParticle Underwater provider 位置偏移、tint、尺寸、lifetime 和物理 metadata；[x] Heart / AngryVillager provider 外观/初速度/位置偏移；[x] Note provider hue/lifetime/初速度；[x] EndRod provider 初速度、尺寸、lifetime 和物理 metadata；[x] Lava provider 随机 sprite、初速度、尺寸/lifetime 和物理 metadata；[x] Snowflake provider tint、初速度、尺寸/lifetime 和物理 metadata；[x] SquidInk/GlowInk provider tint、尺寸、lifetime、初速度和 no-physics metadata；[x] SimpleVertical pause/reset growth provider 随机 sprite、尺寸、y offset 和物理 metadata；[x] PlayerCloud/Sneeze provider 初速度/tint/alpha；[x] Crit/DamageIndicator/EnchantedHit provider 初速度、初始颜色、lifetime 和物理 metadata；[x] Bubble / BubbleColumnUp/BubblePop provider 初速度、尺寸、lifetime 和上浮 metadata；[x] AttackSweep provider 尺寸、tint、lifetime 和 no-motion tick metadata；[x] Simple Spell/Witch provider 初速度、tint、lifetime 和物理 metadata；[x] GlowParticle GlowSquid/WaxOn/WaxOff/Scrape/ElectricSpark provider 初速度、tint、lifetime 和物理 metadata；[x] SoulParticle Soul/SculkSoul provider 位置 jitter、初速度、alpha/尺寸、lifetime 和物理 metadata；[x] HugeExplosion/SonicBoom provider 尺寸、tint、lifetime 和静止速度 metadata；[x] Gust/SmallGust provider 固定尺寸、lifetime 和静止速度 metadata；[x] SculkCharge/SculkChargePop provider 初速度、alpha/尺寸、lifetime 和物理 metadata；[x] Spit/Poof ExplodeParticle provider 初速度、tint、lifetime 和物理 metadata | provider-specific particle behavior、sorting、atlas animation、剩余 LevelEvent |
| [ ] P2：Terrain / Block Render Presentation | [ ] 未完成 | 无整体或阶段完成标注 | terrain AO/tint/fluid overlay、破坏进度、selection overlay、block entity、透明块排序 |
| [ ] P2：屏幕、HUD、字体与截图 | [ ] 未完成 | 无整体或阶段完成标注 | vanilla font、HUD、screens、screenshot/readback |
| [ ] P3：资源与动态纹理泛化 | [ ] 进行中 | [x] player skin/cape/elytra profile dynamic textures decode/cache/upload/fallback | broader dynamic texture loading、resource-pack override、ready/pending/failed 状态 |

### [x] P0 Pipeline Closeout Checklist（狭义 pipeline）

完成口径：这里只判断 renderer submission / render-state pipeline 是否收口，不继续扩展
P1 粒子 provider、实体动画细节、terrain、HUD、first-person 或 GUI parity。狭义
pipeline 全部打勾后，才能把 P0 pipeline 标为 `[x]`；剩余视觉误差必须拆到
P0 visual 或 P1/P2/P3，而不是继续阻塞 pipeline closeout。

硬 checklist：

- [x] residual emit 路径审计清零；或每个 remaining residual 命中都明确归属到
  非 P0 parity（例如 colored-only debug fallback、历史文档、注释）。
- [x] texture-backed / dispatch-owned submission 路径无主要遗漏；新增 renderer 或
  layer 不再绕过 submission 直接写 texture-backed mesh。
- [x] RenderType / `order` / `submit_sequence` / missing-atlas /
  dynamic texture 状态覆盖完整，且测试优先断言 submission metadata，而不是只看
  顶点数量。
- [x] light / overlay / outline 的 pipeline 表达完成；剩余 dynamic Lightmap、
  diffuse、outline 呈现等视觉误差拆到 P0 visual 或后续 P1/P2。
- [x] GPU path 剩余更细粒度状态明确列为后续，不阻塞狭义 pipeline 完成。
- [x] `docs/unsupported-features.md` 中 submission / render type / residual /
  fallback / outline / lighting 相关 stale 项已审计并归档到上述分类。
- [x] closeout gate 固定通过：
  `cargo fmt --all --check`、`git diff --check`、
  `CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test --workspace`、
  renderer/world/pack/native 的 `RUSTFLAGS='-D warnings'` 检查；native 检查后
  `/tmp/bbb-native-test.log` 的 `^warning` 计数必须为 `0`。

2026-06-28 初始 audit baseline：

- `rg residual`：可执行 renderer code 中未发现 texture-backed residual
  mesh-emitting arm；命中主要是 `goal.md` / `docs/unsupported-features.md`
  历史说明，以及 `entity_models/dispatch.rs` 的“textured path has no residual
  mesh-emitting arm”说明；`entity_models/textured.rs` 的旧 residual 迁移注释已
  清理为 explicit submission audit 注释。`docs/unsupported-features.md` 已在
  `Renderer Scene Parity` 下把历史 residual wording 归类为迁移证据，而不是
  当前 P0 blocker；colored-only fallback/debug geometry 归属非 texture-backed
  submission parity。
- `rg fallback`：大量命中是合法资源/协议/item/terrain fallback。P0 pipeline
  相关剩余项是 colored-only `Humanoid` / `Quadruped` / `Placeholder` debug
  fallback；它们不属于 texture-backed submission parity。动态 profile fallback
  属 P3，terrain/item fallback 属 P2/P3。
- `rg unsupported` + `docs/unsupported-features.md` 对照：仍属 P0 blocker 的是
  vanilla outline target / composite、full render-graph sorting、broader
  dynamic `LightTexture` / darkness-adjusted gamma / diffuse parity，以及
  light / overlay / outline metadata 到 GPU presentation 的细粒度状态拆分。粒子 provider、terrain、HUD、
  first-person、GUI 明细不进入本 closeout，除非直接阻塞上述 checklist。
- 2026-06-28 outline closeout：texture-backed static-atlas
  `RenderTypes.outline(...)` bucket 已接入 GPU resident mesh，并用
  `outlineColor` 派生顶点 tint 进行基础呈现；submission metadata 继续保留
  vanilla model tint / texture / light / overlay / order / `submit_sequence`。
  static-atlas 可见 glowing 的 `AFFECTS_OUTLINE` submit 现在也会额外折叠
  `outlineColor`-tinted outline bucket geometry；剩余差异是 vanilla 独立
  `OUTLINE_TARGET` / outline 后处理合成，归入 P0 visual / 后续，不再阻塞
  狭义 pipeline 表达。
- 2026-06-28 submission coverage audit：`rg` 复核显示 texture-backed 写入入口
  集中在 dispatch sink 调用的 `render_textured_submission` /
  dynamic-player submission helpers，先记录 `EntityModelRenderSubmission` 再按
  atlas availability 折叠 mesh；`entity_models/dispatch.rs` 的剩余命中是
  sink-owned `render_textured_layers` 调用和“无 residual mesh-emitting arm”
  注释。测试覆盖面已达到 closeout 口径：78 个 entity model 测试文件断言
  `submit_sequence`，7 个覆盖 missing-atlas / pending-upload submission-first
  行为，25 个覆盖 dynamic player skin / profile texture 状态。
- GPU path deferred inventory：CPU submission graph 已保留 texture / render type /
  tint / transform / light / overlay / outlineColor / `order` /
  `submit_sequence`；后端仍按 bucket 折叠 draw。后续不阻塞狭义 pipeline
  closeout 的 GPU 工作包括按 submission 或等价 render state 绘制、拆分
  `entityCutout*` / `entitySolid` / `armorCutoutNoCull` /
  `entityTranslucent*` / `Eyes` / `waterMask` / glint / scroll 等 pipeline state、
  完整 vanilla outline target / composite、full render-graph sorting，以及
  dynamic `LightTexture` / darkness-adjusted gamma / diffuse 视觉精度。
- 2026-06-28 stale unsupported ledger audit：`docs/unsupported-features.md` 中
  submission / render type / residual / fallback / outline / lighting 相关命中已
  归档：submission、render type、order、missing-atlas、dynamic texture 归入已完成
  的 CPU submission graph；texture-backed residual 命中只保留历史迁移证据和
  dispatch sink 注释；colored fallback/debug geometry、profile dynamic fallback、
  terrain/item fallback 分别归入非 texture-backed debug、P3 dynamic resource、
  P2/P3 presentation；entity static-atlas outline bucket 已完成基础 GPU 呈现；
  vanilla outline target / composite、full render-graph sorting、dynamic
  `LightTexture` / darkness-adjusted gamma / diffuse 和更细 GPU state 全部归入
  P0 visual 或后续 P1/P2/P3，不再
  阻塞狭义 pipeline closeout；colored fallback self-visible invisible
  force-transparent alpha、hidden-glowing outline color，以及 texture-backed
  visible-glowing static-atlas outline bucket copy 已按 vanilla 分支输出并纳入测试。
  本次重跑 audit 计数：`rg residual` 45 行、`rg fallback` 509 行、
  `rg unsupported` 162 行；命中已按上述分类落账。
- 2026-06-28 closeout gate：狭义 pipeline checklist 在最终文档状态下通过
  `cargo fmt --all --check`、`git diff --check`、
  `CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test --workspace`、
  renderer/world/pack/native `RUSTFLAGS='-D warnings'` 检查；native warning
  计数为 `0`。

### [ ] P0：提交图与 RenderType 语义（状态：进行中）

阶段完成：

- [x] render type 表达已经区分 `entityCutout` / `entityCutoutCull` /
  `entityCutoutZOffset` / `entityTranslucent` / `entitySolid` / `Eyes` /
  `armorCutoutNoCull` / `armorTranslucent` / `breezeWind` / `energySwirl` /
  `end_crystal_beam` 等主要实体提交类型。
- [x] texture-backed entity submissions 已记录显式 `order` 和 same-order
  `submit_sequence`，对应 vanilla `SubmitNodeCollector.order(n)`。
- [x] 主要特殊 layer/renderer 的测试已从“只看顶点数量”推进到 texture /
  render type / `vanilla_name()` / tint / transform / light / overlay /
  order / missing-atlas submission-first 覆盖；详见
  `docs/unsupported-features.md` 和 entity texture path memory。
- [x] WindCharge 的 vanilla `breezeWind(wind_charge.png, age * 0.03 % 1, 0)`
  scroll submit 已从 textured residual arm 迁入 shared dispatch sink，保留
  submission-first / missing-atlas 行为。
- [x] BreezeWindLayer 的 vanilla `breezeWind(breeze_wind.png, age * 0.02 % 1, 0)`
  order-1 overlay submit 已从 post-base helper list 迁入 shared dispatch sink，
  保留 body -> wind -> eyes 生成顺序、no-overlay、entity light、U-scroll 和
  missing-atlas submission-first 行为。
- [x] BoatRenderer 的 vanilla `waterMask` order-0 / same-order sequence-1 submit
  已从 post-base helper list 迁入 shared dispatch sink；按 vanilla 只对
  wooden boat / chest boat 生效，`RaftRenderer` bamboo raft / chest raft 不提交
  water patch，并保留 above-water gate、no-overlay、light 和 base -> waterMask
  生成顺序。
- [x] ThrownTridentRenderer 的 vanilla `entityGlint` order-1 foil submit 已从
  post-base helper list 迁入 shared dispatch sink，保留 `ID_FOIL` gate、glint
  texture、flight transform、no-overlay、entity light、base -> foil 生成顺序和
  missing-atlas submission-first 行为。
- [x] GuardianRenderer 的 vanilla `entityCutout(guardian_beam.png)` attack-beam
  submit 已从 post-base helper list 迁入 shared dispatch sink，保留 base ->
  beam 生成顺序、active-target gate、attack-scale tint、full-bright light、
  no-overlay、world-aligned transform、`order` / `submit_sequence` 和
  missing-atlas submission-first 行为。
- [x] EndCrystal body 的 vanilla `entityCutout(end_crystal.png)` submit 已从
  textured residual arm 迁入 shared dispatch sink。
- [x] EndCrystalRenderer 的 vanilla `submitCrystalBeams` / `end_crystal_beam`
  healing-beam submit 已从 post-base helper list 迁入 shared dispatch sink，
  保留 body -> beam 生成顺序、beam target gate、light、no-overlay、
  `order` / `submit_sequence`、custom tiled prism geometry 和 missing-atlas
  submission-first 行为。
- [x] EnderDragonRenderer 的 nearest-crystal `submitCrystalBeams` /
  `end_crystal_beam` healing-beam submit 已从 post-base helper list 迁入
  shared dispatch sink，保留 body -> eyes -> beam 生成顺序、projected
  `beamOffset`、light、no-overlay、`order` / `submit_sequence`、custom tiled
  prism geometry 和 missing-atlas submission-first 行为。
- [x] Player base 的 vanilla `AvatarRenderer` / `LivingEntityRenderer` submit 已从
  colored/textured residual arms 迁入 shared dispatch sink，保留 wide/slim/default/
  profile/dynamic skin、part visibility、force-transparent / outline submission
  metadata 和 missing-atlas submission-first 行为。
- [x] Player `Deadmau5EarsLayer` / `CapeLayer` submit 已从 post-base helper
  list 迁入 shared player dispatch sink，保留 body -> ears -> cape 生成顺序、
  same-skin / dynamic profile texture 路径、invisible gate、WINGS/HUMANOID chest
  gates、no-overlay、`order` / `submit_sequence` 和 missing-atlas / pending-upload
  行为。
- [x] Player `ParrotOnShoulderLayer` / `SpinAttackEffectLayer` submit 已从
  post-base helper 直接调用迁入 dispatch-owned post-Wings layer hook，保留
  `AvatarRenderer` 的 Wings -> shoulder parrots -> riptide spin 相对顺序、
  player light、no-overlay、texture / render type / transform / order 和
  missing-atlas submission-first 行为。
- [x] Player / humanoid mob / armor stand `WingsLayer` submit 已从 post-base
  helper 直接调用迁入 dispatch-owned post-CustomHead hook，保留 vanilla
  CustomHead -> Wings 相对顺序、profile elytra/cape texture override、
  invisible ungated layer 分支、no-overlay、light、order / submit sequence 和
  missing-atlas / pending-upload 行为。
- [x] Player / humanoid mob / villager / armor stand / copper golem 等
  `CustomHeadLayer` skull submit 已从 post-base helper 直接调用迁入
  dispatch-owned post-armor hook，保留 vanilla armor -> CustomHead skull ->
  Wings 相对顺序、host head transform、dynamic player-skin texture 路径、
  invisible ungated layer 分支、no-overlay、light、order 和 missing-atlas /
  pending-upload 行为。
- [x] Player / zombie family / skeleton family / piglin family / armor stand
  `HumanoidArmorLayer` submit 已从 post-base helper 直接调用迁入
  dispatch-owned post-base hook，保留 vanilla slot 顺序 chest -> legs ->
  feet -> head、host pose copy、model-layer / texture / dye metadata、
  invisible ungated layer 分支、no-overlay、light、order / submit sequence 和
  missing-atlas 行为。
- [x] Villager / ZombieVillager 的 `VillagerProfessionLayer` type /
  profession / level submit 已从 post-base helper 直接调用迁入
  dispatch-owned late layer hook，保留 vanilla invisible gate、no-hat
  selection、`order(1..=3)`、light / zero-white overlay 和 missing-atlas
  submission-first 行为。
- [x] PigRenderer 的 vanilla `SimpleEquipmentLayer(PIG_SADDLE)` submit 已从
  post-base helper list 迁入 shared Pig dispatch sink，保留 adult-only gate、
  `armorCutoutNoCull`、`pig_saddle/saddle.png`、entity light、no-overlay、base ->
  saddle 生成顺序和 missing-atlas submission-first 行为。
- [x] StriderRenderer 的 vanilla `SimpleEquipmentLayer(STRIDER_SADDLE)` submit 已从
  post-base helper list 迁入 shared Strider dispatch sink，保留 adult-only gate、
  `armorCutoutNoCull`、`strider_saddle/saddle.png`、entity light、no-overlay、
  base -> saddle 生成顺序和 missing-atlas submission-first 行为。
- [x] CamelRenderer / CamelHuskRenderer 的 vanilla `SimpleEquipmentLayer(CAMEL*_SADDLE)`
  submit 已从 post-base helper list 迁入 shared Camel dispatch sink，保留 adult-only
  camel gate、camel-husk adult renderer 路径、`armorCutoutNoCull`、family-specific
  `camel_saddle` / `camel_husk_saddle` texture、entity light、no-overlay、base ->
  saddle 生成顺序和 missing-atlas submission-first 行为。
- [x] Horse / donkey / mule / skeleton-horse / zombie-horse base submit、HorseMarkingLayer
  translucent overlay、horse/donkey/mule/skeleton-horse/zombie-horse saddle submit，以及
  horse/zombie-horse body-armor submit，已从 colored/textured residual arms 或 post-base
  helper list 迁入 shared dispatch sink；保留 vanilla base -> markings -> body armor ->
  saddle 的 layer 顺序和 missing-atlas submission-first 行为。
- [x] LlamaRenderer 的 vanilla `LlamaDecorLayer` submit 已从 post-base helper list
  迁入 shared Llama dispatch sink，保留 `LLAMA_BODY`、`armorCutoutNoCull`、adult
  carpet gate、adult/baby trader fallback texture、entity light、no-overlay、
  order-1 / sequence-1 和 missing-atlas submission-first 行为。
- [x] NautilusRenderer / ZombieNautilusRenderer 的 vanilla `SimpleEquipmentLayer`
  `NAUTILUS_BODY` / `NAUTILUS_SADDLE` submit 已从 post-base helper list 迁入
  shared Nautilus dispatch sink，保留 adult living / zombie-only gate、body armor ->
  saddle layer 顺序、`armorCutoutNoCull`、family equipment textures、entity light、
  no-overlay、same-order sequence advance、coral armor hide gate 和 missing-atlas
  submission-first 行为。
- [x] WolfRenderer 的 vanilla `WolfArmorLayer` submit 已从 post-base helper list
  迁入 shared Wolf dispatch sink，保留 base -> armor/crack -> collar 调用顺序、
  adult-only gate、`armorCutoutNoCull` body layers、root-collector
  `armorTranslucent` crack submit、entity light、no-overlay、invisible armor
  exception 和 missing-atlas submission-first 行为。
- [x] WolfRenderer 的 invisible ungated `WolfArmorLayer` exception 已从 textured
  loop 的直接 helper 调用迁入 dispatch-owned invisible layer hook，保留 hidden /
  self-visible translucent / glowing outline 分支的 no-base 或 base+armor
  相对顺序、submit sequence 起点、no-overlay、light、outline color 和
  missing-atlas 行为。
- [x] CreeperPowerLayer / WitherArmorLayer 的 vanilla `EnergySwirlLayer.order(1)`
  submit 已从 post-base helper list 迁入 shared dispatch sink，保留 `energySwirl`
  render type、半灰 tint、scroll offsets、no-overlay、light/order 和
  missing-atlas submission-first 行为。
- [x] `entity_model_textured_meshes_with_dynamic_textures` 的主 `if !handled`
  residual arm 已清空并移除；`NoRender` 现在由 shared dispatch sink 显式作为
  vanilla `NoopRenderer` no-submit path 处理。
- [x] texture-backed residual / bespoke layer helper 迁移已完成；剩余非 dispatch
  fallback 只覆盖 colored-only `Humanoid` / `Quadruped` / `Placeholder` 调试几何，
  不再属于 textured submission 迁移范围。

仍在推进：

目标：从“折叠后的 mesh 看起来对”推进到“submission 边界也像 vanilla”。

- 不再新增 texture-backed residual / bespoke layer helper；新 renderer 或 layer
  必须先生成 submission，再按当前后端能力折叠进 mesh bucket。
- 每个 texture-backed path 的测试都应优先断言：
  - texture
  - render type 与 `vanilla_name()`
  - tint
  - root/layer transform
  - explicit `order`
  - same-order `submit_sequence`
  - 必要时再断言折叠后 mesh 的 UV / 顶点 / bounds
- 后端当前仍会把兼容 submission 折叠进 `cutout` / `translucent` / `eyes` /
  dynamic profile texture / scroll / additive scroll buckets，并将 outline
  geometry CPU-side retained。CPU submission metadata 已可审计官方提交图；
  GPU path 后续应逐步按 submission 或等价 render state 保留更细粒度排序和状态，
  但这已归档为 GPU follow-up，不阻塞狭义 pipeline closeout。
- 继续补齐官方 render type 状态，而不退化成粗 bucket：
  - `entityCutout`
  - `entityCutoutCull`
  - `entityCutoutZOffset`
  - `entityTranslucent`
  - `entitySolid`
  - `Eyes`
  - `armorCutoutNoCull`
  - `armorTranslucent`
  - `breezeWind`
  - `energySwirl`
  - `end_crystal_beam`
  - invisible / spectator translucent / glowing outline 相关分支
- 对应 vanilla 依据优先查：
  - `SubmitNodeCollector.order(n)`
  - 各 `*Renderer.submit`
  - `LivingEntityRenderer.getRenderType`
  - 具体 `RenderTypes.*` 调用
  - 各 `RenderLayer` 的 submit 顺序

完成标准：

- 旧的 `entity_model_textured_mesh(...)` 直接 mesh 测试持续减少。
- 新增或迁移测试不只测顶点数量，必须覆盖 texture / render type / tint / transform / order。
- `docs/unsupported-features.md` 中 submission / render type 相关条目保持与测试覆盖一致。

### [ ] P0：光照、Overlay、Invisible / Outline（状态：进行中）

阶段完成：

- [x] 大量实体 base/layer submission 已 pin entity light、hurt/white overlay、
  no-overlay 或 red-row zero-white overlay 的来源，并让 folded vertices 继承
  对应 submission metadata。
- [x] colored/textured entity shader 已用 submitted block/sky light 走 vanilla
  `Lightmap.getBrightness` 形状曲线；eyes 仍保持 emissive。
- [x] texture-backed entity vertices 已携带 vanilla `ModelPart.Polygon.normal`
  等价的面法线，textured shader 已使用 `Lighting.setupLevel` 默认方向光
  (`0.6` power + `0.4` ambient)；colored debug fallback 仍保留 baked shade。
- [x] texture-backed entity normal 已按 vanilla `PoseStack.Pose`
  normal matrix（pose inverse-transpose + normalize）变换，覆盖非等比缩放精度。
- [x] colored/textured/`breezeWind` entity shader 已从标量 lightmap shade 改为
  vanilla `lightmap.fsh` 默认 RGB 组合：block light 使用 `0xFFD88C` tint +
  parabolic mix，sky light 使用默认 white，ambient 默认 black；world/native
  动态 environment 来源已分阶段接入，剩余 biome/spatial/End flash deferred。
- [x] colored/textured/`breezeWind` entity shader 已把 vanilla `LightmapInfo`
  环境颜色/效果字段提升为 renderer uniform：`SkyFactor`、
  `BlockLightTint`、`SkyLightColor`、`AmbientColor`、`NightVisionFactor`、
  `NightVisionColor`、`BossOverlayWorldDarkeningFactor`、`DarknessScale` 和
  `BrightnessFactor` 按 `lightmap.fsh` 顺序组合；`bbb-renderer`
  暴露 `LightmapEnvironment` 供后续 world/native camera attribute probe 接入。
- [x] native 已用 world level dimension info 写入维度级 `LightmapEnvironment`：
  Overworld / Overworld Caves ambient `0x0A0A0A`，Nether `SkyFactor = 0`、
  `Timelines.NIGHT_SKY_LIGHT_COLOR`、ambient `0x302821`，End
  `SkyFactor = 0`、sky light `0xAC60CD`、ambient `0x3F473F`；同时保留
  `--client-gamma` 与 block-light flicker factor。
- [x] native 已按 vanilla `Timelines.OVERWORLD_DAY` 的
  `SKY_LIGHT_FACTOR` / `SKY_LIGHT_COLOR` 关键帧，以及
  `WeatherAttributes.RAIN` / `THUNDER` 的 alpha-blend layer 顺序，驱动
  Overworld lightmap 的 day timeline 与 rain/thunder weather modifiers。
- [x] native lightmap runtime 已按 vanilla `EndFlashState` 的 600 tick
  interval、offset/duration 随机参数和 `sin` 强度曲线，从 THE_END
  default world clock（`clock_id = 1`）叠加 End flash `SkyFactor`；测试固定
  THE_END clock 生效且 Overworld clock / `game_time` 不误触发。
- [x] native 已从 world boss-bar flags 折叠 `shouldDarkenScreen` /
  `shouldCreateWorldFog`：`BossOverlayWorldDarkeningFactor` 按 vanilla
  `GameRenderer` 的 `+0.05` / `-0.0125` tick 状态推进，End flash 在
  `createWorldFog` 时按 vanilla `/3` 写入 `SkyFactor`。
- [x] colored/textured/`breezeWind` entity shader 已套用 vanilla
  `LightmapInfo.BrightnessFactor` 的 `notGamma` mix；`bbb-native --client-gamma`
  作为启动时配置入口，默认值 `0.5` 对齐 vanilla `Options.gamma`。
- [x] colored/textured/`breezeWind` entity shader 的 block light 已从固定
  `1.4` 改为 vanilla `LightmapInfo.BlockFactor` uniform；native 20Hz client tick
  按 `LightmapRenderStateExtractor.tick()` 公式推进 `blockLightFlicker` 并写入
  `blockLightFlicker + 1.4`。
- [x] native 已从本地玩家 `night_vision` / `darkness` mob effect 写入
  `LightmapEnvironment`：`night_vision` 使用 vanilla `GameRenderer.getNightVisionScale`
  的 200 tick 闪烁曲线；`darkness` 使用 vanilla
  `LightmapRenderStateExtractor` 的 brightness modifier、`darknessEffectScale`
  默认值 `1.0` 和 tickCount 余弦 `DarknessScale`；native lightmap runtime 还
  跟踪 vanilla client-side duration 递减，并按
  `MobEffectInstance.BlendState` 的 22 tick step 处理 darkness fade-in/out；
  conduit power 分支按 `LocalPlayer.getWaterVision` 的 `waterVisionTime`
  0..600 tick 累积/离水衰减公式写入 `NightVisionFactor`，且保持
  night-vision effect 优先级。
- [x] `breezeWind` scroll GPU path 已按 vanilla `NO_CARDINAL_LIGHTING` +
  lightmap-lit 语义使用 submitted block/sky light；`energySwirl` 拆到独立
  emissive additive scroll shader。
- [x] spectator-visible invisible living entity 的 world/native/renderer 边界已落地。

仍在推进：

目标：让实体和物品不仅绑定正确纹理，还在 vanilla 光照与 overlay 语义下呈现。

- 补齐 world/native 到 renderer `LightmapEnvironment` 的 biome/spatial camera
  `EnvironmentAttributes` 动态来源；End flash 已覆盖同步 THE_END clock
  的 `SkyFactor` 加成和 boss-overlay world-fog `/3` 分支，剩余
  hide-lightning-flash option 和本地 clock extrapolation 另列后续。
- 补齐实体 smooth / AO 风格采样差异。
- 补齐 GUI/entity-in-UI lighting variants，以及 colored debug fallback；Nether /
  End 静态维度 lightmap 属性、End flash 同步时钟 boost 和 Overworld
  timeline/weather sky-light modifiers 已覆盖，后续只剩 biome/spatial
  modifiers 与 hide-lightning / local-clock 组合状态。
- 将 white overlay progress、hurt/red overlay、freeze/flash 等 overlay 行为统一到所有相关 layer。
- 补齐 base-model invisible handling：
  - invisible 自身视角
  - spectator translucent
  - glowing outline
  - outline color
  - overlay layer 在 invisible / glowing 状态下的 vanilla gate
- 优先实体：
  - [x] sheep base / wool / undercoat 的 white overlay 与 outline（base hurt/white overlay vs wool/undercoat zero-white overlay；invisible glowing base+wool outline submission 与 CPU-side folded outline geometry；undercoat 按 vanilla `!state.isInvisible` 跳过）
  - [x] wolf base / collar / armor 的 white overlay 与 outline（base hurt/white overlay vs collar/armor/crack no-overlay；invisible glowing base outline CPU-side folded geometry；collar 按 vanilla `state.isInvisible` 跳过；armor/crack invisible exception 保留）
  - [x] armor stand marker / non-marker base invisible branches（marker visible/cutout, marker self-visible translucent, marker hidden/glowing no-submit, non-marker hidden/glowing outline）
  - [x] armor stand armor equipment / layer invisible 组合（full/small armor model layers, adult equipment textures, marker hidden/glowing armor-without-base, non-marker hidden/glowing outline base + armor）
  - [x] armor stand WingsLayer / CustomHeadLayer skull invisible 组合（marker hidden/glowing no-base + layer submission；继承 light/no-overlay/outlineColor；texture/render type/transform/order 测试覆盖）
  - [x] armor stand held item item-model pass / generic non-skull custom-head item invisible 组合（marker hidden/glowing no-base + native `held_item_models` 仍烘焙 main-hand 与 HEAD item meshes）
  - [x] player 与 humanoid mob 的 armor / wings / cape / held item overlay 组合（`HumanoidArmorLayer` / `WingsLayer` / `CustomHeadLayer` skull 无 invisible gate：player/zombie hidden no-base 与 zombie glowing outline 覆盖 texture/render type/tint/transform/light/no-overlay/outline/order；`CapeLayer` 的 `!state.isInvisible` gate 已用 player self-visible invisible 回归固定；native `held_item_models` 覆盖 zombie main-hand 与 generic HEAD item visible/hidden/hidden-glowing）

完成标准：

- 每个 slice 都从 vanilla renderer / layer 中确认 `isBodyVisible`、`forceTransparent`、
  `appearGlowing`、`outlineColor`、overlay 坐标来源。
- 测试覆盖 invisible / glowing / normal 至少两个状态，避免只测普通可见路径。

### [ ] P1：实体专用 renderer 行为（状态：进行中）

阶段完成：

- [x] 多类实体专用模型、装备层、emissive/beam layer、动态 player texture
  consumers 已落地；当前清单仍保留需要继续收紧的实体行为和组合状态。

仍在推进：

目标：补齐已经有模型和贴图但 renderer 行为仍缺官方细节的实体。

- Boat / raft：
  - [x] paddle rowing animation（world metadata/passenger gate -> native render state -> renderer paddle pose）
  - [x] hurt/damage roll（VehicleEntity metadata -> native render state -> boat root transform）
  - [x] bubble wobble（bubbleTime metadata -> native render state -> boat root transform）
  - [x] underwater state（world top-fluid test -> native render state -> renderer bubble/water-mask gate）
  - [x] BoatRenderer water mask submission（including vanilla above-water gate；RaftRenderer 无 water patch）
  - [x] lighting（base 和 waterMask 均保留 vanilla `state.lightCoords` +
    `OverlayTexture.NO_OVERLAY`；仅 water-mask depth-only GPU presentation 仍 deferred）
- Parrot：
  - [x] PARTY pose（LevelEvent jukebox state/proximity -> native render state -> renderer dance pose）
  - [x] ON_SHOULDER pose（Player shoulder metadata -> native render state -> left/right ParrotOnShoulderLayer submissions）
- Sheep / wolf：
  - [x] wolf body armor invisible layer exception（hidden / self-visible translucent / glowing outline states keep armor/crack submissions while collar skips）
  - 完整 invisibility / outline / white overlay
  - 其余 render-state extraction parity
- Equine / camel / llama / goat / hoglin / ravager 等大型模型：
  - [x] baby donkey/mule nested `BabyDonkeyModel.setupAnim` default pose（nested leg swing、forced head xRot、tail parent `−π/4` offset）
  - [x] renderer-side equine `animateTail` tail yRot wag（`tail.yRot = cos(ageInTicks * 0.7)`）
  - [x] equine client-random `tailCounter` source projection（local Java LCG `nextInt(200)` start、`++tailCounter > 8` clear -> native `EquineRenderState.animateTail`）
  - [x] equine `isInWater` leg-frequency slowdown（`waterMultiplier = 0.2`）
  - [x] equine eat/stand/feed event pose（`DATA_ID_FLAGS` eating/standing/open-mouth -> `eatAnimation` / `standAnimation` / `feedingAnimation` -> `AbstractEquineModel.setupAnim` head/body/leg transforms）
  - [x] equine/camel saddle ridden bridle/reins visibility（passenger state -> saddle layer visible parts）
  - [x] camel `jumpCooldown` head boost（`DASH` rising edge -> cooldown -> `CamelModel.applyHeadRotation` extra pitch）
  - [x] camel `CAMEL_IDLE` keyframe（local `random.nextInt(40) + 80` cadence -> native render state -> tail/head/ear idle pose）
  - [x] camel `updateWalkAnimation` override（`Pose.STANDING && !DASH` -> `min(distance * 6, 1)`, factor `0.2`; sitting/dashing target `0`）
  - [x] sniffer baby layer/texture dispatch（`AgeableMob.DATA_BABY_ID` -> `ModelLayers.SNIFFER_BABY` / `snifflet.png`，仍按 vanilla `SnifferRenderer` 使用 `SnifferModel` 驱动 baby layer）
  - [x] feline lower-tail walk wobble（`AdultFelineModel.setupAnim` 非 crouch/sprint
    分支的 `tail2.xRot = 1.7278761 + (π/4)·cos(walkAnimationPos)·walkAnimationSpeed`；
    baby `tail2` cubeless，测试覆盖 rest / moving / advanced-position / zero-speed）
  - [x] feline crouch/sprint pose branches（`CatRenderer` / `OcelotRenderer`
    `extractRenderState` 的 `Entity.isCrouching()` / `Entity.isSprinting()` 投影到
    renderer；`AdultFelineModel` / `BabyFelineModel.setupAnim` 的 crouch body/head/tail
    offset、sprint `tail2.y = tail1.y`、sprint leg `0.3` 相位和 branch-specific
    tail wobble amplitude 已测试）
  - boost 等 remaining renderer 状态
  - lighting 与 overlay
- Humanoid / illager / piglin / skeleton family：
  - remaining arm poses
  - use-item sway
  - attack / crossbow / spell / celebrate / riding 组合冲突
  - armor / held item / custom head / wings 的 layer order 交互
- Boss / beam / emissive 类：
  - Ender Dragon / End Crystal / Guardian beam 已有路径继续收紧到 submission/order/render state 级别
  - 检查 remaining beam UV、scroll、additive、emissive、missing-atlas 行为

完成标准：

- 每个实体差异必须先定位 vanilla renderer/model/layer 源码，再改测试。
- 不再新增只验证 vertex count 的 textured regression。
- 对每个特殊 renderer branch 至少有一个状态化测试。

### [ ] P1：物品、GUI、Frame 与第一人称表现（状态：进行中）

阶段完成：

- [x] shared item-model primitive 和主要 third-person consumers 已落地，包括 held
  items、fox/panda-held、item frame contents、custom-head items/skulls、
  player head/body dynamic skins、profile cape、WingsLayer/elytra 等。
- [x] profile skin/cape/elytra 的 decode/cache/upload/sampling 路径已具备。
- [x] filled-map item frame 已按 vanilla `ItemFrameRenderer` / `MapRenderer`
  在 map data 存在时渲染 full-frame map surface，并区分 map-frame border、
  `rotation % 4 * 2`、`0.4375` / `0.5` depth、map color 解码和 glow map
  `15728850` light coords；base map surface 已按 vanilla
  `RenderTypes.text(minecraft:map/<id>)` 迁入 dynamic texture-backed submit，
  并已按 vanilla `showOnlyFrame=true` 提交 frame-visible map decoration
  sprites 和 decoration name text。
- [x] item-frame border 已按 vanilla `block/template_item_frame` /
  `block/template_item_frame_map` 保留普通框 back panel `15.5` 和地图框
  `15.001` 的 fractional model depths。

仍在推进：

目标：把 item model pipeline 从“主要消费者可画”推进到 vanilla presentation parity。

- First-person viewmodel：
  - hand transform
  - use animation
  - swing animation
  - map / bow / crossbow / spyglass / shield 等特殊路径
- Combat / held item arm pose：
  - third-person hand-use sway
  - kinetic weapon / ticksUsingItem
  - per-item swing duration
  - left/right-hand transform 差异
- Item lighting context：
  - GUI front-lit vs world diffuse
  - 当前 baked shade 与 vanilla item lighting 的差异
- Item frame：
  - [x] item-frame border fractional model depths：普通 frame back panel 保留
    vanilla `from.z = 15.5`，map frame 五个模板元素保留 `from.z = 15.001`，
    不再把手写 border boxes 全部舍入到 `15`
  - [x] filled-map full-frame render（map data 存在时）
  - [x] filled-map base surface dynamic texture-backed submit：按 vanilla
    `MapTextureManager.prepareMapTexture` 生成 `minecraft:map/<id>` 128x128
    dynamic texture，按 `MapRenderer.render` 提交 `RenderTypes.text`、白色
    tint、UV `0..1`、`order=0` / `submit_sequence=0`
  - [x] filled-map `MapRenderer` decoration sprite submit：按 vanilla
    `MapDecorationTypes` registry id 映射 sprite / `showOnItemFrame`，
    item-frame `showOnlyFrame=true` 时跳过 player/off-map markers，只提交
    frame-visible sprites 到 `textures/atlas/map_decorations.png`，保留
    `RenderTypes.text`、白色 tint、decoration transform、light、`order=0`
    和 base 后 same-order `submit_sequence`
  - [x] filled-map `MapRenderer` decoration name text submit：按 vanilla
    `Font.width` / `clamp(25 / width, 0, 6/9)` 生成 label transform，
    使用 `textures/font/ascii.png` ASCII glyph atlas 提交白色 text glyphs，
    item-frame `showOnlyFrame=true` 时继承 decoration `showOnItemFrame` gate，
    保留 light、`RenderTypes.text`、`order=1` 和 order 内
    `submit_sequence`
  - [x] invisible item-frame：按 vanilla `state.isInvisible` 清空 frame
    model（不画木框），框内 item 深度从 `0.4375` 改为 `0.5`
  - [x] glow item-frame light：边框按 vanilla `GLOW_FRAME_BRIGHTNESS = 5`
    提升 block light 下限，框内 item 按 vanilla `15728880` full-bright
    light coords；仍无额外 emissive texture/pass
- HUD / inventory：
  - vanilla font / count / durability / cooldown / tooltip / screen depth behavior
  - flat/generated item 与 3D block item 在 GUI pass 中的精确排序

完成标准：

- 每个 item consumer 都以 vanilla `ItemDisplayContext`、display transform 和 renderer 源码为依据。
- GUI/world 使用不同 lighting context 时必须在测试或手动对比记录中说明。

### [ ] P1：透明排序、粒子与 Level Events（状态：进行中）

阶段完成：

- [x] 已有部分 LevelEvent particle side effects，例如 smoke/flame/
  dragon-breath/explosion/cloud/block-face/trial-spawner。
- [x] 已有部分 provider-specific behavior，例如 DragonBreath 与
  SuspendedTown HappyVillager/Composter/Mycelium/EggCrack 的 provider 外观、
  lifetime、物理/速度 metadata。
- [x] DolphinSpeed provider 蓝色 tint、随机 alpha、half lifetime、初速度和物理
  metadata。
- [x] SuspendedParticle Underwater provider `y - 0.125` 初始位置、随机 sprite、
  固定蓝色 tint、随机 SingleQuad 尺寸缩放、`8/(random*.8+.2)` lifetime、
  friction=1、no-physics 和静止速度 metadata。
- [x] Heart / AngryVillager provider 外观、fixed lifetime、grow-to-base size
  curve、初速度、位置偏移和物理 metadata。
- [x] Note provider command-x hue 颜色、fixed lifetime、grow-to-base size curve、
  初速度和物理 metadata。
- [x] EndRodParticle Provider command 初速度、0.75 尺寸缩放、age sprite、
  `60..=71` lifetime、friction=0.91 和 gravity=0.0125 metadata；fade color、
  full-bright light coords / translucent particle layer / collision-free `move`
  override 仍在后续差距内。
- [x] LavaParticle Provider random sprite、constructor-random 水平初速度
  `*0.8`、随机上抛 `0.05..0.45`、`0.2..2.2` 尺寸缩放、shrinking size
  curve、`16/(random*.8+.2)` lifetime、friction=0.999 和 gravity=0.75
  metadata；full-bright block light / child smoke emission 仍在后续差距内。
- [x] SnowflakeParticle Provider age sprite、固定 pale-blue tint、
  `0.1 * (random * random + 1.0)` 尺寸、command `+-0.05` 初速度、
  `16/(random*.8+.2)+2` lifetime、friction=1.0 和 gravity=0.225 metadata；
  extra post-tick damping / opaque particle layer 仍在后续差距内。
- [x] SquidInkParticle Provider / GlowInkProvider age sprite、固定 0.5 尺寸、
  black / glow-ink tint、command 初速度、`6/(random*.8+.2)` lifetime、
  friction=0.92、zero gravity 和 no-physics metadata；alpha fade / in-air
  downward drift / full-bright / translucent particle layer 仍在后续差距内。
- [x] SimpleVerticalParticle PauseMobGrowth / ResetMobGrowth provider random
  sprite、`0.5..1.1` 尺寸缩放、fixed lifetime 8、command 初速度附加
  `-0.03` / `+0.03` y offset、friction=0.98、zero gravity 和 physics metadata；
  opaque particle layer 仍在后续差距内。
- [x] PlayerCloud/Sneeze provider 初速度、固定 sneeze tint/alpha、lifetime 和
  物理 metadata。
- [x] Crit / DamageIndicator / EnchantedHit provider 初速度、随机灰度 / magic
  初始颜色倍率、lifetime、grow-to-base size curve 和物理 metadata。
- [x] Bubble / BubbleColumnUp provider command-scaled 初速度、随机尺寸缩放、
  lifetime、上浮 gravity/friction 和物理 metadata。
- [x] BubblePop provider command 初速度、固定 lifetime 4、age sprite、
  SingleQuadParticle 尺寸/白色 tint，以及不走默认 friction 的 full-gravity tick
  metadata。
- [x] AttackSweep provider xAux-derived 尺寸、随机灰度 tint、固定 lifetime 4、
  age sprite、零 aux Particle 构造速度采样和 no-motion tick metadata；full-bright
  light coords 仍在 per-particle light 后续差距内。
- [x] Simple Spell provider（Infested / RaidOmen / TrialOmen）与 Witch provider
  随机水平初速度、age sprite、lifetime、尺寸缩放、witch 同步紫色 tint 和物理
  metadata；带 option color/power 的 spell variants 仍在剩余 provider-specific
  行为内。
- [x] GlowParticle WaxOn / WaxOff / Scrape / ElectricSpark provider command-scaled
  初速度、固定或二选一 tint、age sprite、lifetime、尺寸缩放和物理 metadata；glow
  light curve 仍在 light/provider-specific 后续差距内。
- [x] GlowParticle GlowSquid provider 随机水平 / `yAux` 初速度、静止水平命令
  x/z dampening、二选一青绿色 tint、age sprite、lifetime、尺寸缩放和物理
  metadata；glow light curve 仍在 light/provider-specific 后续差距内。
- [x] SoulParticle Provider / EmissiveProvider（Soul / SculkSoul）RisingParticle
  位置 jitter、`constructor * 0.01 + aux` 初速度、alpha=1、1.5 尺寸缩放、
  age sprite、lifetime、friction 和 physics metadata；SculkSoul full-bright
  light coords 仍在 per-particle light 后续差距内。
- [x] HugeExplosionParticle Provider（Explosion）xAux-derived quad size、随机灰度
  tint、age sprite、`6..=9` lifetime、静止速度和基础物理 metadata；
  full-bright light coords 仍在 per-particle light 后续差距内。
- [x] SonicBoomParticle Provider 固定 1.5 quad size、随机灰度 tint、age sprite、
  fixed lifetime 16、静止速度和基础物理 metadata；full-bright light coords
  仍在 per-particle light 后续差距内。
- [x] GustParticle Provider / SmallProvider 固定 quad size（1.0 / 0.15）、
  age sprite、`12..=15` lifetime、静止速度和基础物理 metadata；full-bright
  light coords 仍在 per-particle light 后续差距内。
- [x] SculkChargePopParticle Provider command 初速度、alpha=1、base quad size、
  age sprite、`6..=9` lifetime、friction=0.96 和 no-physics metadata；
  full-bright light coords / translucent particle layer 仍在后续差距内。
- [x] SculkChargeParticle Provider command 初速度、alpha=1、1.5 尺寸缩放、
  age sprite、`8..=19` lifetime、friction=0.96 和 no-physics metadata；roll
  option、full-bright light coords / translucent particle layer 仍在后续差距内。
- [x] SpitParticle Provider 与 ExplodeParticle/Poof provider command+random
  初速度、随机灰度 tint、Explode lifetime/尺寸、age sprite、friction 和 gravity
  metadata。

仍在推进：

目标：补齐当前粒子与透明对象和官方的排序、限制、provider 细节差距。

- 粒子 provider-specific behavior：
  - 初速度
  - lifetime
  - size curve
  - alpha/color curve
  - gravity / collision / player-coupled physics
- 粒子 sorting：
  - translucent particle order
  - terrain/item particle option rendering
  - particle limits/settings
- atlas mip animation：
  - animated sprite frame advance
  - missing definition / missing sprite diagnostics
- LevelEvent particle side effects：
  - 在已有 smoke/flame/dragon-breath/explosion/cloud/block-face/trial-spawner
    之外继续补剩余事件

完成标准：

- 每个 particle slice 记录 vanilla provider 类和精确公式。
- 对随机行为使用确定性 seed 或固定样本测试。

### [ ] P2：Terrain / Block Render Presentation（状态：未完成）

目标：把 terrain 从基础 mesh 对齐推进到官方视觉细节。

- 检查 block render shape、face culling、AO、tint、biome tint、fluid overlay 与 vanilla 差异。
- 补齐破坏进度、selection overlay、block entity 特殊 renderer、透明块排序等剩余 presentation。
- 复核 terrain 与 entity/item 共用 atlas、mip、sampler、lightmap 时的状态差异。

完成标准：

- 每个 block/render shape 差异必须有 vanilla source 或资源 JSON 依据。
- 对视觉 slice 使用确定性 pixel/readback 测试或明确手动对比记录。

### [ ] P2：屏幕、HUD、字体与截图（状态：未完成）

目标：从功能性 HUD 推进到 vanilla screen presentation。

- vanilla font rendering：
  - glyph atlas
  - shadow
  - bidi / style / color
  - width metrics
- HUD：
  - hotbar、crosshair、status bars、boss bars、titles、subtitles、debug overlay
  - screen 与 world pass 的深度/颜色 load/clear 顺序
- Screens：
  - inventory / container / merchant / recipe / book / sign / advancement 等 screen 的 vanilla 布局
- Screenshot / readback：
  - 保证 renderer output 可稳定测试
  - 记录平台差异和 llvmpipe fallback

完成标准：

- 新 UI/screen 工作不做临时配置 UI；启动配置仍从命令行进入。
- 视觉结果尽量用 deterministic screenshot/readback 验证。

### [ ] P3：资源与动态纹理泛化（状态：进行中）

阶段完成：

- [x] player skin/cape/elytra profile dynamic textures 已具备 decode、cache、
  upload、atlas sampling、loading/ready/failed fallback 路径。

仍在推进：

目标：在 player skin/cape/elytra 已有路径基础上，扩展更泛化的动态资源加载。

- broader non-profile dynamic texture loading。
- 资源包 override / custom model / datapack registry asset 的动态组合。
- 失败、pending、ready 状态必须有明确 fallback，不画 stale texture。

完成标准：

- 动态资源路径区分 decode、cache、upload、ready 状态。
- submission metadata 在缺 atlas entry 时仍可记录，折叠 geometry 可按 vanilla fallback 或等待策略处理。

## Slice 选择顺序建议

1. 先只做能关闭 “P0 Pipeline Closeout Checklist” 的 P0 slice；不要继续无限补
   P1 粒子 provider、实体动画细节、terrain、HUD、first-person 或 GUI。
2. residual / fallback / unsupported audit 中发现的 stale 文案先归类或清理；
   真正阻塞狭义 pipeline 的项优先实现或拆分。
3. 然后补 P0 lighting / overlay / invisible-outline 的 pipeline 表达；纯视觉误差
   拆到 P0 visual 或 P1/P2，不再混入 pipeline closeout。
4. checklist 全部完成并通过固定 gate 后，才恢复 P1 行为 slice。粒子、GUI、
   first-person 和 terrain presentation 作为 P1/P2 独立脉络推进。

每个 slice 开始前先 grep 当前实现，确认该 feature 确实缺失或测试不足；历史上多次出现
“ledger 以为缺失但代码已实现”的情况。

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
  - `rm -f crates/bbb-native/src/lib.rs && touch crates/bbb-native/src/main.rs && CARGO_TARGET_DIR=/tmp/bbb-target-main cargo test -p bbb-native --quiet > /tmp/bbb-native-test.log 2>&1`
  - `grep -c '^warning' /tmp/bbb-native-test.log` 输出必须为 `0`

## 剩余渲染差异优先级

说明：

- 整体已完成优先级：暂无（没有一个 P0/P1/P2/P3 章节可以整体标为完成）。
- 阶段标记：`[x]` = 已完成并进入回归维护；未打勾目标仍按 slice 推进。
- 下面用“阶段完成”标注已经落地的部分，用“仍在推进”标注明确剩余项。

### P0：提交图与 RenderType 语义（状态：进行中）

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

仍在推进：

目标：从“折叠后的 mesh 看起来对”推进到“submission 边界也像 vanilla”。

- 继续把 residual / bespoke emit 迁移为 submission 生成，再折叠进 mesh bucket。
- 每个 texture-backed path 的测试都应优先断言：
  - texture
  - render type 与 `vanilla_name()`
  - tint
  - root/layer transform
  - explicit `order`
  - same-order `submit_sequence`
  - 必要时再断言折叠后 mesh 的 UV / 顶点 / bounds
- 后端当前仍会把兼容 submission 折叠进 `cutout` / `translucent` / `eyes` / scroll buckets。
  这会隐藏官方提交图差异；后续应逐步让 GPU path 能按 submission 或等价 render state
  保留更细粒度排序和状态。
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

### P0：光照、Overlay、Invisible / Outline（状态：进行中）

阶段完成：

- [x] 大量实体 base/layer submission 已 pin entity light、hurt/white overlay、
  no-overlay 或 red-row zero-white overlay 的来源，并让 folded vertices 继承
  对应 submission metadata。
- [x] spectator-visible invisible living entity 的 world/native/renderer 边界已落地。

仍在推进：

目标：让实体和物品不仅绑定正确纹理，还在 vanilla 光照与 overlay 语义下呈现。

- 补齐 vanilla `LightTexture` 更精确的颜色、gamma 和 block-light tint 曲线。
- 补齐实体 smooth / AO 风格采样差异。
- 补齐 `Lighting.setupLevel` 方向性 diffuse shading 与当前 shader 的差异。
- 将 white overlay progress、hurt/red overlay、freeze/flash 等 overlay 行为统一到所有相关 layer。
- 补齐 base-model invisible handling：
  - invisible 自身视角
  - spectator translucent
  - glowing outline
  - outline color
  - overlay layer 在 invisible / glowing 状态下的 vanilla gate
- 优先实体：
  - sheep base / wool / undercoat 的 white overlay 与 outline
  - wolf base / collar / armor 的 white overlay 与 outline
  - armor stand marker / invisible / translucent 分支
  - player 与 humanoid mob 的 armor / wings / cape / held item overlay 组合

完成标准：

- 每个 slice 都从 vanilla renderer / layer 中确认 `isBodyVisible`、`forceTransparent`、
  `appearGlowing`、`outlineColor`、overlay 坐标来源。
- 测试覆盖 invisible / glowing / normal 至少两个状态，避免只测普通可见路径。

### P1：实体专用 renderer 行为（状态：进行中）

阶段完成：

- [x] 多类实体专用模型、装备层、emissive/beam layer、动态 player texture
  consumers 已落地；当前清单仍保留需要继续收紧的实体行为和组合状态。

仍在推进：

目标：补齐已经有模型和贴图但 renderer 行为仍缺官方细节的实体。

- Boat / raft：
  - [x] paddle rowing animation（world metadata/passenger gate -> native render state -> renderer paddle pose）
  - [x] hurt/damage roll（VehicleEntity metadata -> native render state -> boat root transform）
  - bubble wobble
  - underwater state
  - [x] water mask submission
  - lighting
- Sheep / wolf：
  - 完整 invisibility / outline / white overlay
  - 其余 render-state extraction parity
- Equine / camel / llama / goat / hoglin / ravager 等大型模型：
  - deferred tail / idle / event pose
  - ridden / boost / water / jump-cooldown 等 renderer 状态
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

### P1：物品、GUI、Frame 与第一人称表现（状态：进行中）

阶段完成：

- [x] shared item-model primitive 和主要 third-person consumers 已落地，包括 held
  items、fox/panda-held、item frame contents、custom-head items/skulls、
  player head/body dynamic skins、profile cape、WingsLayer/elytra 等。
- [x] profile skin/cape/elytra 的 decode/cache/upload/sampling 路径已具备。

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
  - filled-map full-frame render
  - invisible frame offset 差异
  - glow frame lighting / emissive 差异
- HUD / inventory：
  - vanilla font / count / durability / cooldown / tooltip / screen depth behavior
  - flat/generated item 与 3D block item 在 GUI pass 中的精确排序

完成标准：

- 每个 item consumer 都以 vanilla `ItemDisplayContext`、display transform 和 renderer 源码为依据。
- GUI/world 使用不同 lighting context 时必须在测试或手动对比记录中说明。

### P1：透明排序、粒子与 Level Events（状态：进行中）

阶段完成：

- [x] 已有部分 LevelEvent particle side effects，例如 smoke/flame/explosion/cloud/
  block-face/trial-spawner。

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
  - 在已有 smoke/flame/explosion/cloud/block-face/trial-spawner 之外继续补剩余事件

完成标准：

- 每个 particle slice 记录 vanilla provider 类和精确公式。
- 对随机行为使用确定性 seed 或固定样本测试。

### P2：Terrain / Block Render Presentation（状态：未完成）

目标：把 terrain 从基础 mesh 对齐推进到官方视觉细节。

- 检查 block render shape、face culling、AO、tint、biome tint、fluid overlay 与 vanilla 差异。
- 补齐破坏进度、selection overlay、block entity 特殊 renderer、透明块排序等剩余 presentation。
- 复核 terrain 与 entity/item 共用 atlas、mip、sampler、lightmap 时的状态差异。

完成标准：

- 每个 block/render shape 差异必须有 vanilla source 或资源 JSON 依据。
- 对视觉 slice 使用确定性 pixel/readback 测试或明确手动对比记录。

### P2：屏幕、HUD、字体与截图（状态：未完成）

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

### P3：资源与动态纹理泛化（状态：进行中）

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

1. 优先收敛 P0 submission / render type / order 测试迁移。
2. 其次补 P0 lighting / overlay / invisible-outline，因为它们会影响所有实体和 layer。
3. 然后按实体族或 renderer feature 做 P1 行为 slice，避免跨多个大模块同时改。
4. 粒子、GUI、first-person 和 terrain presentation 作为 P1/P2 独立脉络推进。

每个 slice 开始前先 grep 当前实现，确认该 feature 确实缺失或测试不足；历史上多次出现
“ledger 以为缺失但代码已实现”的情况。

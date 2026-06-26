# 目标：bbb 与 Minecraft Java 26.1 彻底对齐

## 总目标

让 `bbb` 渲染/世界/协议子系统与 Minecraft Java **26.1** 完全对齐：
`docs/unsupported-features.md` 中，除刻意不支持的 feature 外，其余全部彻底支持。
以**每 slice 一个可合并提交**的方式推进，直到清单清空。

## 硬性约束（每 slice 必须满足）

- **严格按 vanilla 源码转写**：权威源 `~/Work/mc-code/sources/26.1/`，落地前核对确切常量/索引/公式，不臆测。
- **合并门禁（每 slice 全绿）**：
  - `bbb-renderer` / `bbb-world` / `bbb-pack` 各 **0 warning**；
  - native warning **≤ 14**（当前 13）；
  - `cargo fmt --all --check` exit 0；
  - `cargo test --workspace` 全绿。
- **`bbb-renderer` 不得依赖 `bbb-pack`**。
- 每 slice 更新 `docs/unsupported-features.md` + memory（`~/.claude/projects/-home-zgy-Work-bbb/memory/`）。
- 提交前 `rm -f crates/bbb-native/src/lib.rs`（native 是二进制，靠 `touch main.rs` 触发 warning 重算）。
- 始终用中文汇报。

## 已完成（截至 2026-06-26）

实体基础贴图全部接好；近期完成的子特征 / 整条脉络见
memory `entity-texture-path-status.md`。其中**马类（equine）贴图脉络已彻底打通**：
骷髅马/僵尸马、活体马（7 种毛色 + markings 叠层）、成年与幼年驴/骡，
全部走 textured 路径并带 vanilla 步态/头部/尾巴姿态（幼驴因 `setupAnim` 锁 xRot=-30° 而静态）。
村民 / 僵尸村民的类型、职业、等级徽章叠层已完成；猪、adult equine
（马/驴/骡/骷髅马/僵尸马）、adult strider、adult camel/camel_husk、adult living nautilus
和 zombie nautilus 的鞍装备层、llama decor、nautilus body armor、horse/zombie-horse
body armor 装备层已完成，baby 按 vanilla 行为跳过没有 baby model 的装备层。
实体附着方块模型已开始打通：snow golem carved pumpkin head block layer、
iron golem held poppy block layer、mooshroom mushroom block layer、enderman carried block
layer 已完成，copper golem antenna block decoration 也已通过同一方块附着路径完成。
通用 `CustomHeadLayer` 的非 skull/非 armor head-slot item 分支已完成：player、
zombie/skeleton/piglin family、illager、villager/wandering trader、armor stand、
copper golem 会用 `ItemDisplayContext.HEAD` 渲染头槽物品；custom-head skull 头颅专用
分支中的静态 mob 头颅也已完成：`skeleton_skull`、`wither_skeleton_skull`、
`zombie_head`、`creeper_head` 会通过 `SkullModel` 和对应实体贴图渲染；无 profile
组件的 `player_head` 会通过默认 `DefaultPlayerSkin` / humanoid head+hat layer 渲染。
`piglin_head` 会通过专用 PiglinHeadModel 几何和 `wornHeadAnimationPos` 耳朵动画渲染。
`dragon_head` 会通过专用 DragonHeadModel 几何和 `wornHeadAnimationPos` 下颚动画渲染。
`wornHeadAnimationPos` 也已按 vanilla 在乘骑 living entity 时读取载具 walk animation。
`DataComponents.PROFILE` 已按 26.1 `ResolvableProfile.STREAM_CODEC` 保留为结构化
profile summary（full/partial、UUID/name、properties、`PlayerSkin.Patch` 资源纹理/模型覆盖）。
带 profile 的 `player_head` 已按 `PlayerSkinRenderCache` 默认 fallback 选择
`DefaultPlayerSkin.get(UUID)`（显式 UUID、offline-name UUID 或 nil UUID），并支持指向内置默认
player skin 的 `PlayerSkin.Patch` body；剩余的是远程 profile 解析、下载皮肤和任意动态纹理加载。
铜傀儡 vanilla 模型、四态风化贴图和 emissive eyes layer 已完成。
Illager 家族的主要 arm-pose 分支已覆盖到 evoker/illusioner spellcasting、illusioner bow aim、
pillager crossbow hold/charge、evoker/vindicator celebrating，以及 vindicator empty/armed
`ATTACKING`；riding sit 坐姿也已按 `Entity.isPassenger()` 投影。
Zombified piglin 的 `AnimationUtils.animateZombieArms` held-out arms 也已接入，
复用 zombie-arm helper 并按 `isAggressive` / `attack_anim` 驱动。
Vex charging 姿态也已补齐 `setArmsCharging` 的持物分支：native 从 main/off-hand
装备投影 RIGHT/LEFT 手非空状态，renderer 在 charging 时按手选择空手前伸或
`xRot = π*7/6` 的 held-item arm pose。
Fox pouncing / faceplanted 的 `FoxRenderer.setupRotations` body pitch 也已补齐：
root transform 在标准 living setup 后追加 `Rx(-state.xRot)`，叼物层共享该 root。
Wither skull 的 `isDangerous` 纹理切换已补齐：native 读取 26.1
`WitherSkull.DATA_DANGEROUS` 布尔同步字段（index 8）并投影到
`EntityModelKind::WitherSkull { dangerous }`，renderer 在 textured path 里按状态选择
`wither.png` / `wither_invulnerable.png`。
Shulker bullet 的第二次 vanilla submit 也已补齐：textured path 在 base `spark.png`
模型之后复用同一个 posed model，追加 `scale(1.5)` 的 translucent outer shell，
颜色/alpha 使用 vanilla packed color `0x26ffffff`。
实体 textured path 现在显式记录 vanilla-shaped submission 元数据：render type 区分
`entityCutout` / `entityCutoutCull` / `entityCutoutZOffset` /
`entityTranslucent` / `Eyes` / `breezeWind` / `energySwirl`，`collector_order`
对应 `SubmitNodeCollector.order(n)`，并用 `submit_sequence` 保留同 order 内的 layer 顺序。
End Crystal 已从 colored-only fallback 推进到 textured path，绑定
`textures/entity/end_crystal/end_crystal.png`，使用 vanilla 默认 `entityCutout`、
order 0、白 tint 和 `scale(2)·translate(0,-0.5,0)` root transform；dragon healing beam 仍 deferred。
Wolf 湿身 shade tint 已完成：world 侧按 `Wolf.getWetShade(partialTick)` 维护
`isWet/shakeAnimO/shakeAnim` 计时，native 转抄到 render state，renderer 只把
`wetShade` 乘到基础 wolf submit，collar 保持自己的染色 tint/order。
Drowned swimAmount 重姿态已完成：world 侧按 `LivingEntity.updateSwimAmount`
从 synced `Pose.SWIMMING` 维护 `swimAmountO/swimAmount`（每 tick `±0.09`），native
转抄 `swim_amount` 和 `bounding_box_height`，renderer 对 drowned base/outer 同步应用
`DrownedRenderer.setupRotations` body pitch 与 `DrownedModel.setupAnim` arm/leg swim pose。

## 剩余大子系统（按优先级）

1. **实体上的物品渲染器**
   目标中原列的手持物 / 狐狸叼物 / 物品展示框内容已经接到 item-model primitive。
   继续按 `docs/unsupported-features.md` 审计剩余专用 item-on-entity 层（如
   `CustomHeadLayer` / `SkullBlockRenderer` 的远程或动态 profiled-player 皮肤、其他专用装备/物品层等），逐项从
   deferred 改为 covered。
   其中远程 / 动态 player skin 资源管线按优先级推进：
   1. 先解析 profile `textures` property 的 base64 JSON，提取 skin URL、cape URL
      和 slim/wide model 信息；保持现有默认皮肤 fallback。
   2. 补 `ResolvableProfile` 的异步 profile resolution 与缓存：partial name/UUID
      能解析为完整 profile/properties，失败时保留默认皮肤。
   3. 补远程 skin PNG 下载、内存/磁盘缓存、尺寸/格式校验，以及 64x32 旧 skin
      到当前布局的转换。
   4. 扩展 native/render-state 表达，从 `EntityDefaultPlayerSkin` 扩到默认 fallback、
      loading/error fallback、resolved dynamic skin handle 和 slim/wide model。
   5. 扩展 renderer 动态纹理入口：支持运行时上传/替换 dynamic skin texture
      或独立动态 skin atlas，先接 `CustomHeadLayer` / `SkullBlockRenderer`
      的 `player_head`，再推广到玩家实体本体、cape、elytra 等层。
2. **世界侧动画计时器**
   海豚游泳重姿态、panda sit/lie/roll 等 client-tick 动画。

> 落地前务必先在 bbb 里 grep 确认该 feature 确实缺失（历史上多次「以为缺失实则已实现」）。
> 索引/数据陷阱见 memory `entity-metadata-index-layout.md`；模型/代理历史见 `proxy-entity-replacement.md`。

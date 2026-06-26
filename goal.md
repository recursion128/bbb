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
`zombie_head`、`creeper_head` 会通过 `SkullModel`、对应实体贴图和 vanilla
`entityCutoutZOffset` render type 渲染；无 profile 组件的 `player_head` 会通过默认
`DefaultPlayerSkin` / humanoid head+hat layer 和 `entityCutoutZOffset` 渲染。
`piglin_head` 会通过专用 PiglinHeadModel 几何和 `wornHeadAnimationPos` 耳朵动画渲染。
`dragon_head` 会通过专用 DragonHeadModel 几何和 `wornHeadAnimationPos` 下颚动画渲染。
`wornHeadAnimationPos` 也已按 vanilla 在乘骑 living entity 时读取载具 walk animation。
`DataComponents.PROFILE` 已按 26.1 `ResolvableProfile.STREAM_CODEC` 保留为结构化
profile summary（full/partial、UUID/name、properties、`PlayerSkin.Patch` 资源纹理/模型覆盖），
并会解析 profile `textures` property 的 base64 JSON，提取 skin/cape/elytra URL 和
vanilla slim/wide 模型选择（skin 的 `metadata.model=slim`，否则 wide）。
带 profile 的 `player_head` 已按 `PlayerSkinRenderCache` 默认 fallback 选择
`DefaultPlayerSkin.get(UUID)`（显式 UUID、offline-name UUID 或 nil UUID），并支持指向内置默认
player skin 的 `PlayerSkin.Patch` body；renderer 会把 profiled default/dynamic player head
按 vanilla `PlayerSkinRenderCache.renderType()` 记录为 `entityTranslucent` submission。
native/render-state 也已能携带 profile texture URL 派生的 dynamic skin handle、fallback
默认皮肤和 slim/wide model，submission 会保留 dynamic handle；renderer 已补动态
player skin atlas，`CustomHeadLayer` / `SkullBlockRenderer` 的 Ready `player_head`
会改采样上传后的动态 skin，Loading/Failed 仍采样 fallback。native 已补
`ResolvableProfile` 的 name/UUID profile resolution
缓存 primitive、reqwest/rustls HTTP fetcher，以及显式启用的异步 profile resolution
worker；custom-head player head 投影会在 pending/failed 时保留默认 fallback，完成后使用
resolved profile/properties。native 也已补 skin PNG 异步下载队列，成功结果先保留为待上传
数据、失败回写 failed fallback；主循环会在 renderer 上传成功后回写 Ready。玩家实体本体
现在无 PlayerInfo 时会按 UUID 选择 vanilla 18 默认 skin；有 PlayerInfo profile 时会选择
动态 skin/model，Ready 时通过动态 player skin atlas 的 cutout mesh 采样上传后的 skin，
Loading/Failed 仍采样 fallback。native 也已补 cape/elytra 这类普通 profile
texture PNG 的异步下载/缓存 primitive（不走 legacy skin post-process，按 capes/elytra
分目录缓存并 drain 给 renderer），renderer 已补普通 profile texture 动态 atlas
上传入口与采样 mesh primitive；玩家 profile cape layer 现在会在 cape model part
可见且动态 atlas entry ready 时以 vanilla `entitySolid` submission 采样上传后的 cape，
缺 entry 时等待，并按 chest equipment asset 的 WINGS/HUMANOID layer 执行 vanilla
cape suppression/translation。玩家 `WingsLayer` / elytra presentation 也已完成：
pack/native 从 chest equipment asset 投影 WINGS layer 纹理与 `use_player_texture`，
renderer 用 vanilla `ElytraModel` 几何提交 `armorCutoutNoCull`，order 从 0 起，
transform 含官方 `z=0.125` layer 平移，并按官方优先级采样 Ready profile elytra、
profile cape fallback 或静态 `textures/entity/equipment/wings/elytra.png`；缺 profile
atlas entry 时等待。world/native 也已按 vanilla `LivingEntity.elytraAnimationState`
投影 elytra rotX/Y/Z，并转发到 renderer；WINGS layer 也已推广到 humanoid mob、
armor stand 和 baby `ELYTRA_BABY` 模型。world/native 也已补玩家 cloak interpolation：
按 vanilla `ClientAvatarState.moveCloak` 的 0.25 追随、10-block teleport reset、
`AvatarRenderer.extractCapeState` 的 flap/lean/lean2 clamp、partial lerp、fall-flying
lean suppression 和 walk bob 投影到 renderer。剩余的是更泛化的任意动态纹理加载。
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
Shulker attach-face root transform 已完成：world 读取 `DATA_ATTACH_FACE_ID`(16)、
native 转发 `attachFace`，renderer 按 `ShulkerRenderer.setupRotations` 的
`bodyRot + 180` 和 `attachFace.getOpposite().getRotation()` 绕 `(0,0.5,0)` 旋转，
测试覆盖 metadata 投影、transform、texture、render type、tint、order。
实体 textured path 现在显式记录 vanilla-shaped submission 元数据：render type 区分
`entitySolid` / `armorCutoutNoCull` / `entityCutout` / `entityCutoutCull` / `entityCutoutZOffset` /
`entityTranslucent` / `Eyes` / `breezeWind` / `energySwirl` / `end_crystal_beam`，`order`
对应 `SubmitNodeCollector.order(n)`，并用 `submit_sequence` 保留同 order 内的 layer 顺序。
render type 还暴露 vanilla 名称断言，防止这些细分退回粗 bucket。
当前实现还显式记录 render type 到 GPU mesh bucket 的映射（cutout / translucent /
eyes / scroll / additive-scroll），因此 `entityCutout`、`entityCutoutCull`、
`entityCutoutZOffset` 等 vanilla render type 会保留各自 submission 表达，即使当下
backend 把兼容内容折进同一个 cutout mesh。
residual hand-emitted equipment / equine / villager overlay paths are now emitted through
submission metadata before folding into the mesh buckets, including humanoid armor,
horse/donkey/undead-horse base+saddle/body-armor, horse markings, and villager type/profession/level
overlays; custom-head skull residual emits also record texture/render type/tint/transform/order
submissions before folding into cutout or translucent buckets. `breezeWind` / `energySwirl`
scroll residual emits now also go through a shared scrolled submission helper before folding into
the scroll buckets, and Guardian attack beams now record vanilla `entityCutout` submissions before
folding their tiled custom geometry into the scroll bucket through a custom scroll-geometry
submission helper (the submission render type stays `entityCutout` even though the backend folds to
scroll). End Crystal 的 residual textured geometry 也已改为先生成 `entityCutout` submission，
再通过统一 helper 折进 mesh。
End Crystal 已从 colored-only fallback 推进到 textured path，绑定
`textures/entity/end_crystal/end_crystal.png`，使用 vanilla 默认 `entityCutout`、
order 0、白 tint 和 `scale(2)·translate(0,-0.5,0)` root transform；`EndCrystal.DATA_BEAM_TARGET`
custom beam 也已按 vanilla 投影 target center offset，记录
`RenderTypes.endCrystalBeam(end_crystal_beam.png)` submission（order 0 / sequence 1），
再把八面 prism 几何折进 scroll mesh，测试覆盖 texture、render type、tint、transform、order
以及 tiled UV。EnderDragonRenderer 自身的 nearest-crystal healing beam 也已接入：
world 投影最近 EndCrystal 的 bobbed `beamOffset`，native 转抄到 render state，renderer 在
body/eyes 后记录 `end_crystal_beam` submission（order 0 / sequence 2）并复用同一八面 prism
helper；测试同样覆盖 texture、render type、tint、transform、order 和 tiled UV。
Guardian beam 还覆盖缺少 beam atlas entry 时仍记录 vanilla submission、但不生成折叠几何。
Wolf 湿身 shade tint 已完成：world 侧按 `Wolf.getWetShade(partialTick)` 维护
`isWet/shakeAnimO/shakeAnim` 计时，native 转抄到 render state，renderer 只把
`wetShade` 乘到基础 wolf submit，collar 保持自己的染色 tint/order。
Wolf water-shake roll pose 也已完成：同一 world `shakeAnim` 现在作为
`WolfRenderState.shakeAnim` 转抄到 renderer，并按 vanilla
`WolfRenderState.getBodyRollAngle(offset)` 滚动 adult/baby base 与 collar 模型。
Wolf begging/head-roll tilt 已完成：world 侧按 `Wolf.DATA_INTERESTED_ID`
（index 20）维护 `interestedAngleO/interestedAngle`（每 tick 朝目标 `0.4`
ease），native 转抄 `WolfRenderState.headRollAngle`，renderer 在 adult
`real_head` / baby `head` 上叠加 `headRollAngle + getBodyRollAngle(0)`。
Drowned swimAmount 重姿态已完成：world 侧按 `LivingEntity.updateSwimAmount`
从 synced `Pose.SWIMMING` 维护 `swimAmountO/swimAmount`（每 tick `±0.09`），native
转抄 `swim_amount` 和 `bounding_box_height`，renderer 对 drowned base/outer 同步应用
`DrownedRenderer.setupRotations` body pitch 与 `DrownedModel.setupAnim` arm/leg swim pose。
Panda sit/lie/roll client-tick 动画已完成：world 侧按 vanilla `Panda.tick`
维护 `sitAmount/onBackAmount/rollAmount`（active `+0.15`，inactive `-0.19`）和
`rollCounter`，native 转抄 `sitAmount/lieOnBackAmount/rollAmount/rollTime`，renderer
同步应用 `PandaRenderer.setupRotations` 的 roll tumble / sit / lie root transform，以及
`PandaModel.setupAnim` 的 adult/baby sitting、eating/scared、lie-on-back、adult roll limb/head 姿态。

## 剩余大子系统（按优先级）

1. **实体上的物品渲染器**
   目标中原列的手持物 / 狐狸叼物 / 物品展示框内容已经接到 item-model primitive。
   继续按 `docs/unsupported-features.md` 审计剩余专用 item-on-entity 层（如
   `CustomHeadLayer` / `SkullBlockRenderer` 的远程或动态 profiled-player 皮肤、其他专用装备/物品层等），逐项从
   deferred 改为 covered。
   其中远程 / 动态 player skin 资源管线按优先级推进：
   1. DONE：解析 profile `textures` property 的 base64 JSON，提取 skin/cape/elytra URL
      和 slim/wide model 信息；保持现有默认皮肤 fallback。
   2. DONE：native 已补 `ResolvableProfile` resolution/cache 与 custom-head 异步接入：
      按 vanilla 只解析无 properties 且仅有 name 或仅有 UUID 的 dynamic partial
      profile，invalid name / miss / fetch failure 保留默认皮肤 fallback；HTTP fetcher
      覆盖 Mojang name→UUID 与 session profile/properties 解析；main 显式启用 worker
      并 drain 完成结果，`player_head` 投影先返回 fallback，完成后使用 resolved
      profile/properties 与 decoded textures。
   3. DONE：renderer 已补下载后 skin PNG 格式/尺寸校验、64x32 legacy skin
      到 64x64 当前布局的 vanilla `SkinTextureDownloader.processLegacySkin` 转换，以及
      opaque-base / Notch transparency alpha 规则；native 已补 fetcher-backed memory/disk
      skin PNG cache（disk hit 优先、miss 后 fetch/write/process）和 reqwest/rustls HTTP
      fetcher；main 显式启用 skin cache dir 和异步下载 worker，custom-head dynamic
      skin URL 会排队下载，失败回写 failed fallback，成功结果保留给后续 GPU 上传。
   4. DONE：native/render-state 已从 `EntityDefaultPlayerSkin` 扩到默认 fallback、
      dynamic skin handle、slim/wide model、loading/ready/failed 状态，以及按 texture URL
      缓存并可替换 resolved texture handle 的 profile-skin cache；主循环会在 renderer
      上传成功后把对应 URL 标记 Ready，上传失败则标记 Failed。
   5. DONE for `CustomHeadLayer` / `SkullBlockRenderer` `player_head`：renderer 已有
      独立动态 player skin atlas，Ready dynamic submission 会进入动态 atlas mesh 并用
      vanilla `entityTranslucent` 采样上传后的 64x64 skin；Loading/Failed 或缺 atlas entry
      继续采样 fallback 默认皮肤。
   6. DONE for 玩家实体本体：无 PlayerInfo 时按 UUID 复现
      `DefaultPlayerSkin.get(uuid)` 的 18 默认皮肤选择；有 PlayerInfo profile 时 native
      会复用 profile-skin cache，`EntityModelKind::Player` 携带 `EntityPlayerSkin` 并按
      skin model 选择 wide/slim；renderer 为 Ready dynamic player body 使用动态
      player skin atlas 的 cutout mesh，submission 仍记录 vanilla `entityCutout`、
      fallback texture、tint、transform、order。
   7. DONE for native 普通 profile texture primitive：renderer 提供
      `decode_dynamic_player_texture_png`，只校验 PNG 并保留原始 RGBA 尺寸，不套
      `SkinTextureDownloader.processLegacySkin`；native 用同一个 fetcher boundary 为
      cape/elytra 分别维护 memory/disk cache 和异步队列，`NativeItemRuntime` 会从
      profile textures 里排队 cape/elytra URL，main drain 成待上传结果。
   8. DONE for renderer 普通 profile texture 上传 primitive：renderer 维护可变尺寸
      dynamic player texture atlas，`upload_dynamic_player_texture` 按 handle 替换/排序并
      上传 GPU texture；main 会把 cape/elytra 下载成功结果上传到该 atlas。
   9. DONE for renderer 普通 profile texture 采样 primitive：textured submission 可携带
      `EntityDynamicPlayerTexture` handle，Ready entry 会进入绑定动态 profile texture atlas
      的 cutout/translucent bucket，缺 atlas entry 时保留 submission 元数据并回退静态 atlas。
   10. DONE for 玩家 profile CapeLayer dynamic texture presentation：native 会把
      PlayerInfo profile 的 cape URL 投影成 `EntityDynamicPlayerTextureKind::Cape`，
      renderer 在 cape model part 可见且动态 atlas entry ready 时提交
      `RenderTypes.entitySolid(skin.cape().texturePath())` 等价的 profile cape layer，
      并测试 texture、render type、tint、transform、order、WINGS chest suppression、
      HUMANOID chest translation 和缺 entry 等待路径。
   11. DONE for 玩家 `WingsLayer` / elytra presentation：native 会从 chest equipment asset
      投影 vanilla elytra WINGS layer，renderer 生成 `armorCutoutNoCull` submission，
      pin 住 texture、render type、tint、transform、order，并复现 profile elytra 优先、
      cape fallback、静态 equipment texture、缺动态上传等待路径；world/native 已投影
      vanilla `LivingEntity.elytraAnimationState` rotX/Y/Z，并覆盖 humanoid mob、
      armor stand 和 baby `ELYTRA_BABY`。
   12. DONE for full cloak interpolation：world 侧维护 player cloak old/current position、
      walkDist/bob 和 fall-flying ticks，按 vanilla `AvatarRenderer.extractCapeState`
      生成 `capeFlap`/`capeLean`/`capeLean2`；native 转发到 renderer cape submission。
   13. DONE for `PlayerSkin.Patch` body/cape/elytra resource textures：native 会按 vanilla
      `PlayerSkin.with(Patch)` 语义优先使用 patch 的本地 `ClientAsset.ResourceTexture`，
      通过 pack resource stack 加载 `texture_path` PNG。body patch 复用 dynamic player-skin
      上传/采样路径，cape/elytra patch 复用普通 profile texture dynamic atlas
      上传/采样路径；被 patch 覆盖的远程 skin/cape/elytra URL 不会被下载。
      剩余：broader non-profile dynamic texture loading。
> 落地前务必先在 bbb 里 grep 确认该 feature 确实缺失（历史上多次「以为缺失实则已实现」）。
> 索引/数据陷阱见 memory `entity-metadata-index-layout.md`；模型/代理历史见 `proxy-entity-replacement.md`。

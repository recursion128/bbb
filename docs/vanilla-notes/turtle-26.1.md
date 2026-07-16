# Turtle 26.1 vanilla 预提取

日期：2026-07-16。范围只覆盖后续 Turtle entity-family slice 会消费的客户端可观察事实，
重点是 `HAS_EGG`、`LAYING_EGG`、baby 尺寸、render/model、F3+I debug 与网络事件反馈。
本文不是 Turtle AI 重写规格；AI、寻路、繁殖、战利品和服务端权威移动均在末尾标为
`not-applicable`。

源码路径均相对本地权威根 `~/Work/mc-code/sources/26.1/`。实现时应继续使用仓库生成的
catalog/accessor，不能把本文列出的 registry ordinal 或 metadata wire index 复制成新的
手写事实源。

## EntityType、spawn 与默认状态

- registry resource id 是 `minecraft:turtle`；当前由 vanilla `EntityType` 静态注册顺序
  生成的 catalog ordinal 是 `137`。实现必须引用 `bbb_protocol` 生成的
  `VANILLA_ENTITY_TYPE_TURTLE_ID`，不能手写 `137`。
  来源：`net/minecraft/world/entity/EntityType.java`，`EntityType.<clinit>` 的 `TURTLE`
  注册项。
- adult 基础尺寸为 width `1.2F`、height `0.4F`；category 为 `CREATURE`，passenger
  attachment 是 `(0.0, 0.55625, -0.25)`，client tracking range 是 `10`。未显式设置
  eye height，因此走 `EntityDimensions.defaultEyeHeight(height) = height * 0.85F`；没有
  取整步骤。
  来源：`net/minecraft/world/entity/EntityType.java`，`EntityType.<clinit>` 的 `TURTLE`；
  `net/minecraft/world/entity/EntityDimensions.java`，构造器与 `defaultEyeHeight`。
- Turtle 自有普通字段的构造前默认是 `layEggCounter = 0`、`homePos = BlockPos.ZERO`、
  `travelPos = null`、`goingHome = false`。`finalizeSpawn` 随后把 `homePos` 设为实体当前
  `blockPosition()`；这是服务端 spawn 初始化，不应由客户端猜测或重演。
  来源：`net/minecraft/world/entity/animal/turtle/Turtle.java`，字段初始化、构造器与
  `finalizeSpawn`。
- 属性基线是 `MAX_HEALTH = 30.0`、`MOVEMENT_SPEED = 0.25`、`STEP_HEIGHT = 1.0`，建立在
  `Animal.createAnimalAttributes()` 上。客户端实际值仍以 attribute packets/canonical
  world state 为准。
  来源：`net/minecraft/world/entity/animal/turtle/Turtle.java`，`createAttributes`。
- natural-spawn 的海平面、沙地与亮度判据属于服务端 spawn authority，不是本 family
  的客户端实现项。
  来源：`net/minecraft/world/entity/animal/turtle/Turtle.java`，`checkTurtleSpawnRules`。

## Synced metadata 与 wire index 来源

| accessor | owner | serializer | vanilla default | 当前生成 wire index | 客户端语义 |
| --- | --- | --- | --- | --- | --- |
| `DATA_BABY_ID` | `AgeableMob` | `BOOLEAN` | `false` | 由生成 schema 解析继承链；当前为 `16` | adult/baby model、baby bounds、`HAS_EGG` render gate |
| `AGE_LOCKED` | `AgeableMob` | `BOOLEAN` | `false` | 由生成 schema 解析继承链；当前为 `17` | F3+I `AgeLocked` |
| `HAS_EGG` | `Turtle` | `BOOLEAN` | `false` | `AgeableMob` 字段之后的 Turtle 第一个声明；当前为 `18` | adult `egg_belly` 可见性与 root 下移 |
| `LAYING_EGG` | `Turtle` | `BOOLEAN` | `false` | `HAS_EGG` 之后的 Turtle 第二个声明；当前为 `19` | land 分支 front-leg digging 动画 |

- 上表的 owner、serializer、声明顺序与默认值来源：
  `net/minecraft/world/entity/AgeableMob.java`，静态 accessor 声明及
  `defineSynchedData`；`net/minecraft/world/entity/animal/turtle/Turtle.java`，静态
  `HAS_EGG` / `LAYING_EGG` 声明及 `defineSynchedData`。
- wire index 的算法不是“父字段数常量”：`SynchedEntityData.defineId` 调用
  `ClassTreeIdRegistry.define(clazz)`；后者先沿 superclass 查最近的 last id，再取
  `last + 1`，同一 owner 按声明调用顺序继续递增。来源：
  `net/minecraft/network/syncher/SynchedEntityData.java`，`defineId`；
  `net/minecraft/util/ClassTreeIdRegistry.java`，`getLastIdFor` / `define`。
- `BOOLEAN` 使用 `ByteBufCodecs.BOOL`。来源：
  `net/minecraft/network/syncher/EntityDataSerializers.java`，`BOOLEAN` 静态注册。
- 后续迁移应一次派生 Turtle bundle：按 entity type 查询生成 schema 中名为
  `HAS_EGG` 与 `LAYING_EGG` 的 accessor，并验证 owner 为 Turtle、serializer 为
  `BOOLEAN`、typed default 为 boolean；消费端只取 helper 返回的 `data_id/default`。
  `bbb-native` 当前的四个固定 `VANILLA_TURTLE_*` aliases 应一起删除，不能只迁一半。
- Turtle 自有字段没有 byte flag mask，也没有 enum ordinal；两个状态都是独立 boolean。
  不要为它们发明 mask 或枚举。

## Render-state 投影、model 与 resource path

- `isOnLand = !entity.isInWater() && entity.onGround()`；先对 `isInWater` 取反，再与
  `onGround` 做逻辑与。`isLayingEgg = entity.isLayingEgg()`；
  `hasEgg = !entity.isBaby() && entity.hasEgg()`，所以 baby 即使收到 `HAS_EGG=true` 也
  不显示 egg belly，而 `LAYING_EGG` 没有 baby gate。
  来源：`net/minecraft/client/renderer/entity/TurtleRenderer.java`，
  `extractRenderState`。
- adult model layer 为 `minecraft:turtle#main`，baby layer 为
  `minecraft:turtle_baby#main`；默认 layer 名由 `register(model)` 转为 `main`。
  来源：`net/minecraft/client/model/geom/ModelLayers.java`，`TURTLE`、`TURTLE_BABY`
  与 `register(String)`。
- adult texture 是 `textures/entity/turtle/turtle.png`，baby texture 是
  `textures/entity/turtle/turtle_baby.png`。adult render type 为 `entityCutout`，baby
  为 `entityCutoutCull`。
  来源：`net/minecraft/client/renderer/entity/TurtleRenderer.java`，texture constants
  与 `getTextureLocation`；
  `net/minecraft/client/model/animal/turtle/AdultTurtleModel.java` 构造器；
  `net/minecraft/client/model/animal/turtle/BabyTurtleModel.java` 构造器。
- renderer 构造 shadow radius 是 `0.7F`；baby 的最终 shadow radius 是先取
  superclass 结果，再乘 `0.83F`，没有取整。
  来源：`net/minecraft/client/renderer/entity/TurtleRenderer.java`，构造器与
  `getShadowRadius`。
- adult atlas 是 `128 x 64`，baby atlas 是 `16 x 16`。完整 cube/UV/pose 以各自
  `createBodyLayer` 为唯一事实源；仓内已有逐 cube 语义测试，后续 accessor slice 不应
  重写 geometry。
  来源：`net/minecraft/client/model/animal/turtle/AdultTurtleModel.java`，
  `createBodyLayer`；`net/minecraft/client/model/animal/turtle/BabyTurtleModel.java`，
  `createBodyLayer`。
- adult `egg_belly` box 为 origin `(-4.5F, 3.0F, -14.0F)`、size
  `(9.0F, 18.0F, 1.0F)`、UV `(70, 33)`，使用与 body 相同的 pose：offset
  `(0.0F, 11.0F, -10.0F)` 后绕 X 旋转 `PI / 2`。可见时再执行 `root.y--`，即 model
  local Y 精确减 `1.0F`。
  来源：`net/minecraft/client/model/animal/turtle/AdultTurtleModel.java`，
  `createBodyLayer` 与 `setupAnim`。

## Animation 公式与运算顺序

以下公式使用 `float`/`Mth.cos`，vanilla 没有整数取整或 clamp；实现不得改变乘法顺序后
再声称逐值一致。

- `QuadrupedModel.setupAnim` 先设置 head：`xRot = state.xRot * (PI / 180)`，
  `yRot = state.yRot * (PI / 180)`；腿的 base X rotation 是
  `cos(animationPos * 0.6662F + phase) * 1.4F * animationSpeed`。phase 为 `PI` 的是
  left-hind 与 right-front，另外两条腿 phase 为 `0`。
  来源：`net/minecraft/client/model/QuadrupedModel.java`，`setupAnim`。
- `TurtleModel.setupAnim` 在 superclass 之后运行。land 分支先计算
  `swingPos = animationPos * 5.0F`。非 laying 时 `layEgg = 1.0F`、
  `layEggAmplitude = 1.0F`；laying 时分别是 `4.0F` 与 `2.0F`。随后
  `frontSwing = cos(layEgg * swingPos)`、`hindSwing = cos(swingPos)`：right-front Y
  为 `-frontSwing * 8.0F * speed * layEggAmplitude`，left-front 取正；right-hind Y
  为 `-hindSwing * 3.0F * speed`，left-hind 取正。land 分支保留 superclass 的腿 X
  rotation。
  来源：`net/minecraft/client/model/animal/turtle/TurtleModel.java`，`setupAnim`。
- water/non-land 分支按顺序计算
  `swingScale = 0.5F * animationSpeed`，再算
  `swing = cos(animationPos * 0.6662F * 0.6F) * swingScale`。right-hind X=`swing`、
  left-hind X=`-swing`（覆盖 base X）；right-front Z=`-swing`、left-front Z=`swing`
  （front X 保留 base 值）。`LAYING_EGG` 只影响 land 分支。
  来源：`net/minecraft/client/model/animal/turtle/TurtleModel.java`，`setupAnim`。

## Dimensions、collision 与 pick

- adult bounds 直接来自 EntityType 的 `1.2F x 0.4F`。AABB 构造顺序是
  `w = width / 2`、`h = height`，然后
  `(x-w, y, z-w) .. (x+w, y+h, z+w)`；无取整。
  来源：`net/minecraft/world/entity/EntityDimensions.java`，`makeBoundingBox`。
- baby constant 的构造顺序必须保持：取 `EntityType.TURTLE.getDimensions()`；用新的
  attachments builder 把 PASSENGER 点设为
  `(0.0F, EntityType.TURTLE.getHeight(), -0.25F)`；最后对 dimensions 与 attachments
  统一 `scale(0.3F)`。因此当前基础数值为 width `1.2 * 0.3`、height `0.4 * 0.3`、
  default eye height `(0.4 * 0.85) * 0.3`，attachment 为
  `(0.0 * 0.3, 0.4 * 0.3, -0.25 * 0.3)`；全程 float 乘法，无取整。
  来源：`net/minecraft/world/entity/animal/turtle/Turtle.java`，`BABY_DIMENSIONS` 与
  `getDefaultDimensions`；`net/minecraft/world/entity/EntityDimensions.java`，`scale`；
  `net/minecraft/world/entity/EntityAttachments.java`，`scale`。
- `LivingEntity.getDimensions` 在 pose 非 sleeping 时取 Turtle 的 default dimensions，
  再乘通用 `getScale()`；因此客户端 pick bounds 还必须在 adult/baby 尺寸之后组合
  synced scale attribute。来源：`net/minecraft/world/entity/LivingEntity.java`，
  `getDimensions` / `getDefaultDimensions`。
- Turtle 没有覆写 pick 判据；继承 `LivingEntity.isPickable() = !isRemoved()`。专属差异是
  baby dimensions，不是额外 pick inflation。
  来源：`net/minecraft/world/entity/LivingEntity.java`，`isPickable`；
  `net/minecraft/world/entity/animal/turtle/Turtle.java`，`getDefaultDimensions`。
- `canBeLeashed()` 固定返回 `false`，`isPushedByFluid()` 固定返回 `false`。前者是服务端
  interaction authority；后者只在将来核对本地实体物理时使用，不能顺手扩大本次
  metadata accessor slice。
  来源：`net/minecraft/world/entity/animal/turtle/Turtle.java`，`canBeLeashed` 与
  `isPushedByFluid`。

## Client tick / movement 边界

- Turtle 没有 client-only `tick` override。远端实体 transform/interpolation、
  `inWater`、`onGround` 与 walk animation 输入来自通用 entity/network state；Turtle
  专属客户端工作只有上面的 render-state 投影和 model animation。
  来源：`net/minecraft/world/entity/animal/turtle/Turtle.java`（无 `tick` override，只有
  `aiStep` / `travelInWater`）；
  `net/minecraft/client/renderer/entity/TurtleRenderer.java`，`extractRenderState`。
- `aiStep` 在活着、laying、counter >= 1 且 `counter % 5 == 0`、脚下为 sand 时发出
  level event `2001` 和 game event `minecraft:entity_action`。客户端应消费服务端发来的
  event packet，不能自己递增 `layEggCounter` 或重新判沙地。
  来源：`net/minecraft/world/entity/animal/turtle/Turtle.java`，`aiStep`；
  `net/minecraft/world/level/gameevent/GameEvent.java`，`ENTITY_ACTION` 注册。
- `travelInWater` 的顺序是 `moveRelative(0.1F, input)`、按当前 delta movement
  `move(SELF, ...)`、velocity 乘 `0.9`，满足目标/home 条件时再加 Y `-0.005`；
  `TurtleMoveControl` 另含浮力与 speed lerp。这些属于服务端权威 mob movement，标
  `not-applicable`，这里只记录以避免误把它们实现成远端客户端预测。
  来源：`net/minecraft/world/entity/animal/turtle/Turtle.java`，`travelInWater` 与
  `TurtleMoveControl.updateSpeed` / `tick`。

## Save / debug

- Turtle 自有 save 顺序是在 parent save 后写 `home_pos`（`BlockPos.CODEC`）与
  `has_egg`（boolean）。读取 `home_pos` 缺失时默认当前 `blockPosition()`，读取
  `has_egg` 缺失时默认 `false`。`LAYING_EGG`、`layEggCounter`、`travelPos` 与
  `goingHome` 不保存。
  来源：`net/minecraft/world/entity/animal/turtle/Turtle.java`，
  `addAdditionalSaveData` / `readAdditionalSaveData`。
- 本地轻量 F3+I 能从 synced metadata 可靠重建 `has_egg`；非默认 `home_pos` 不同步，
  不能从客户端 state 猜测。需要精确值时必须走已有 server query/probe 路由；若后续要
  改本地 fallback 的 `home_pos` 策略，标“需实现时二次确认”，不要把字段初始
  `BlockPos.ZERO` 与 load-missing 的 `blockPosition()` 混为一谈。
  来源：`net/minecraft/world/entity/animal/turtle/Turtle.java`，字段初始化、
  `finalizeSpawn` 与 `readAdditionalSaveData`。

## Event、particle 与 audio ids

- digging feedback 的 level event id 是 `2001`（`PARTICLES_DESTROY_BLOCK`），data 是
  Turtle 当前位置下方 block state 的 `Block.getId(...)`。client handler 先从 data
  解出 block state；非 air 时播放该 block 的 break sound，volume
  `(soundVolume + 1.0F) / 2.0F`、pitch `soundPitch * 0.8F`，再调用
  `addDestroyBlockEffect`。
  来源：`net/minecraft/world/entity/animal/turtle/Turtle.java`，`aiStep`；
  `net/minecraft/world/level/block/LevelEvent.java`，`PARTICLES_DESTROY_BLOCK`；
  `net/minecraft/client/renderer/LevelEventHandler.java`，`globalLevelEvent` 的 `case 2001`。
- turtle egg placement 的 level event id 是 `2012`
  (`PARTICLES_TURTLE_EGG_PLACEMENT`)；`TurtleEggBlock.onPlace` 在 sand 上且 server side
  时发送 data `15`。client handler 用 data 作为 count，在 block 内生成
  `minecraft:happy_villager`；该 particle 当前生成 catalog ordinal 是 `43`，代码必须
  引用生成常量而不能手写 `43`。
  来源：`net/minecraft/world/level/block/TurtleEggBlock.java`，`onPlace`；
  `net/minecraft/world/level/block/LevelEvent.java`，
  `PARTICLES_TURTLE_EGG_PLACEMENT`；
  `net/minecraft/client/renderer/LevelEventHandler.java`，`case 2012`；
  `net/minecraft/core/particles/ParticleTypes.java`，`HAPPY_VILLAGER` 注册。
- Turtle entity sound resource ids 按 `SoundEvents` 注册段为：
  `minecraft:entity.turtle.ambient_land`、`minecraft:entity.turtle.death`、
  `minecraft:entity.turtle.death_baby`、`minecraft:entity.turtle.hurt`、
  `minecraft:entity.turtle.hurt_baby`、`minecraft:entity.turtle.lay_egg`、
  `minecraft:entity.turtle.shamble`、`minecraft:entity.turtle.shamble_baby`、
  `minecraft:entity.turtle.swim`。另有 TurtleEggBlock 的
  `minecraft:entity.turtle.egg_break`、`egg_crack`、`egg_hatch`。numeric registry id
  由生成 sound catalog 决定，不在消费端手写。
  来源：`net/minecraft/sounds/SoundEvents.java`，Turtle sound registrations。
- adult land ambient 条件是 `!isInWater && onGround && !isBaby`，ambient interval `200`；
  swim sound 是 `TURTLE_SWIM` 且 superclass volume 先乘 `1.5F`；step sound 按 baby
  选择 shamble variant，volume `0.15F`、pitch `1.0F`；hurt/death 同样按 baby 选择。
  来源：`net/minecraft/world/entity/animal/turtle/Turtle.java`，
  `getAmbientSoundInterval`、`getAmbientSound`、`playSwimSound`、`getSwimSound`、
  `playStepSound`、`getHurtSound`、`getDeathSound`。
- lay completion 的服务端顺序是：在 turtle position 播
  `minecraft:entity.turtle.lay_egg`（source `BLOCKS`、volume `0.3F`、pitch
  `0.9F + nextFloat() * 0.2F`）；在目标上方放 `minecraft:turtle_egg`，`eggs` 值为
  `nextInt(4) + 1`，set-block flags `3`；发 `minecraft:block_place` game event；最后清
  `HAS_EGG`、`LAYING_EGG` 并设 love time `600`。客户端只消费 sound/block/event 与
  metadata 更新。
  来源：`net/minecraft/world/entity/animal/turtle/Turtle.java`，
  `TurtleLayEggGoal.tick`；`net/minecraft/world/level/gameevent/GameEvent.java`，
  `BLOCK_PLACE` 注册。

## 仓内现状与后续 slice 路由

- metadata schema 与 defaults 已生成，结构测试覆盖 Turtle parent、两个字段、wire
  index 与 default：`crates/bbb-world/src/entities/schema/generated_26_1.rs`、
  `crates/bbb-world/src/entities/schema.rs` 的
  `compatibility_queries_and_virtual_defaults_preserve_existing_semantics`。后续只应新增/复用 derived
  accessor helper，不改 generated output。
- 当前固定 accessor 消费点是 `crates/bbb-native/src/entity_scene.rs` 的 imports、
  `entity_scene/variants.rs` 的 `turtle_has_egg` / `turtle_laying_egg`，以及
  `input.rs` 的 `debug_push_turtle_additional_save_data`。它们是一个同构 migration
  bundle；不要夹带模型、动画、bounds 或 debug 行为修改。
- render projection 语义测试：`crates/bbb-native/src/entity_scene/tests.rs` 的
  `entity_model_instances_project_turtle_has_egg`、
  `entity_model_instances_project_turtle_laying_egg`、
  `entity_model_kind_projects_turtle_baby_from_data`。
- model/texture/animation 语义测试集中在
  `crates/bbb-renderer/src/entity_models/tests/turtle.rs`，覆盖 adult/baby geometry、egg
  belly、texture refs、land/water 曲线与 laying amplitude；accessor slice 不重复这些
  事实。
- baby pick bounds 与 scale attribute 组合在
  `crates/bbb-world/src/entities/tests/specialized_pick_bounds_and_render_scale.rs` 的
  `baby_pick_bounds_follow_vanilla_metadata` 中已有 Turtle case；on-ground 投影在
  `crates/bbb-world/src/entities/tests/movement_combat_and_swing_projection.rs` 的
  `entity_model_sources_project_on_ground_from_movement`。
- local debug evidence 是 `crates/bbb-native/src/input/tests.rs` 的
  `shift_f3_i_with_permission_copies_local_turtle_save_nbt_to_clipboard`；egg-placement event
  evidence 是 `crates/bbb-native/src/particle_runtime/tests.rs` 的 level-event particle
  测试。

## 明确的 not-applicable 与需二次确认项

- `not-applicable`：goal selector、path navigation、home/travel target 选择、breeding、
  egg placement 决策、natural spawn、成长 loot、雷击伤害与 authoritative combat。
  这些由服务端决定，客户端只接收可观察结果。
  来源：`net/minecraft/world/entity/animal/turtle/Turtle.java`，`registerGoals`、各
  `Turtle*Goal`、`checkTurtleSpawnRules`、`ageBoundaryReached`、`thunderHit`。
- 需实现时二次确认：bbb 的 shadow pipeline 是否已经组合 baby `0.83F`；F3+I 本地
  fallback 对不可观测 `home_pos` 的产品判据。
- 需实现时二次确认：level event `2001` 的现有 generic destroy-block effect 是否同时
  保持 block break sound 公式与 block-state-driven particles；不要因为 `2012` 已有
  focused test 就推断 `2001` 的 Turtle 路径也已有等价 evidence。

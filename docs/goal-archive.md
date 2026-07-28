# goal.md 完成史索引

自 2026-07-21 起本文件只保留里程碑级索引，不再收录逐 slice 完成散文。
完成史的权威来源按优先级为：

- **git log**：commit message 即 slice 描述；用 `git log --grep '<关键词>'` 或
  `git log -S <symbol>` 定位。2026-07-21 之前的逐 slice 归档散文见本文件在
  commit `cbfae1bec` 及更早版本中的原文。
- **coverage ledger 与 overrides**：逐格状态与证据在
  `docs/*-coverage-*.json` 与 `scripts/data/*-overrides-*.json`。
- **工程切片设计合同**：活跃合同在 `docs/*-design.md`；已闭环合同在
  `docs/archive/`。

## 归档规则

- 原子 slice 不写归档条目：commit message + 账本/矩阵 delta 即完整记录。
- 工程 slice 关闭、foundation/milestone 关闭或工作流变更时，在下方索引按日期
  追加 1～3 行摘要（目标、关键提交或证据指针），不写实现散文。

## 里程碑索引

### 无日期旧队列（workflow-v3 之前的 goal.md 队列节，原文见 git 历史）

- P1-1 GPU render-state / render-graph fidelity。
- P1-2 实体专用 renderer 行为。
- P1-3 物品、frame 与第一人称表现（含 font 五连、DISSOLVE、PIP）。
- P1-4 GUI lighting surface / entity-in-UI。
- P1-5 透明排序、粒子与 level events。
- P2 terrain / block render presentation；P2 屏幕、HUD、字体与截图（两批）。
- 历史 audit 快照；2026-07-02/07-04 架构重构批次记录（另见 memory）。

### 按日期

- **2026-07-09**：P1 五节 + P2 terrain 队列节关闭原文迁入；docs/unsupported/
  明细目录并回账本；F3+I local save NBT 全家族批（mob/projectile/minecart 等
  ~72 节）。
- **2026-07-10**：F3+I 收尾（item frame/allay/chest boat/fishing bobber）；
  debug renderer 大批（post-effect、heightmap、water-level、text billboard、
  gizmo/collision、supporting-block、light-level、solid faces、sky-light）；
  advanced tooltip 组件批；F3+I projectile/minecart lifecycle 批；ItemStack
  component save codec 全家族（primitive→potion-contents，~70 节）+ descriptor
  dispatch 重构；vanilla catalog generation 第一批（packet/registry/numeric-id
  ~16 类）；entity metadata schema phase 1–13。
- **2026-07-11**：metadata schema phase 14–15 + Tree-sitter extractor
  foundation + generated metadata consumers 全 mob 家族（~70 节）+
  enum/ordinal/registry-resource resolver；Tree-sitter frontend 迁移
  （packet/core-registry/gameplay-id/item/furnace-fuel/block/sound）；
  bootstrap/instrument/banner/painting/enchantment/animal-variant/jukebox
  catalog；membership 与 action-order catalog 大批（~50 节）；packet coverage
  inventory foundation + handshake/status/login/configuration 与 play 第一批
  逐包 coverage audit。
- **2026-07-12**：play clientbound/serverbound 逐包 coverage audit 收尾
  （~130 节）；login/handshake wire 边界；registry holder lookup 迁移批 +
  typed dimension projection（skybox/ambient-light/weather/environment）；
  terrain membership 生成批（constructor family、render layer、culling、
  tint 等 ~25 节）；类继承 membership 派生批（~20 节）；重复 ID alias 清理；
  StatsScreen 开工。
- **2026-07-13**：StatsScreen 完成；HurtAnimation/Animate/SetEntityLink/
  RotateHead/mount/minecart lerp/respawn-login 生命周期批；presentation
  catalog 生成批（beacon/recipe-book/potion/trim/banner/villager/animal
  variant ~20 节）+ instrument/trim/enchantment registry owner 迁移；
  keepalive/ClientTickEnd/PlayerLoaded/teleport 收口；command suggestion
  呈现；dialog 解码大批开工。
- **2026-07-14**：dialog item component decode/canonicalize/bridge 三大批
  （~90 节）；dialog 呈现与交互（Notice/Confirmation/MultiAction/DialogList/
  ServerLinks、输入 widget、焦点、滚动、IME）。
- **2026-07-15**：dialog 收尾（OpenUrl/click event/screen 返回栈）；Transfer
  reconnect 闭环；RegistryData loader/built-in/holder-consumer inventory 生成；
  holder identity 修复批；catalog lookup bridge 清理批；class-behavior 生成批
  （pickability/sound-source/renderer registrations/arm-pose 等 ~25 节）；
  workflow-v3 落地并归档旧 goal.md 完成史；metadata accessor 统一推导批开工。
- **2026-07-16**：metadata accessor 推导/派生/消费三大批收尾（~170 节，
  含全部 fixture 与生产路径迁移）；serializer/fixture 身份统一批；entity
  coverage matrix foundation；playable-gate harness 连接基线；Turtle 全类目
  coverage 试点；工作流 v4（deny-warnings 入 workspace lints、change-aware
  gate、批量门禁）。
- **2026-07-17**：固定 metadata id fixture 迁移关闭；model-kind/provider-kind
  dispatch 生成批（skeleton/illager/camel/equine/boat/zombie/piglin/hoglin/
  guardian/squid）；entity-event override dispatch owner + 首批 entity events。
- **2026-07-18**：server resource-pack lifecycle 与 reload barrier + known-pack
  协商；可玩门禁阶段 1 基础游玩输入闭环（movement、破坏/放置、连接活性）；
  terrain 按 chunk 增量重建；control lock-free world 快照；模拟/渲染线程拆分
  phase 1–6；签名聊天 phase 1–4；动态 RegistryAccess typed revision phase 1–2。
- **2026-07-19**：RegistryAccess typed revision phase 3–4 closeout；simulation
  publication 结构共享与 entity tick 端到端成本；simulation/render 性能
  phase 3–6（visibility-first extraction、增量 terrain residency、有界后台
  meshing、交付门禁合同）；remote debug subscription 全路径；playable session
  lifecycle 双官方 server 闭环（death/respawn、Nether、pack push/pop、
  Play→Config→Play、Transfer）；terrain section-dirty owner 与精确 GPU
  residency；item/map model-space GPU residency。
- **2026-07-20**：client resource-pack generation 与原子 reload lifecycle；
  不可信 PNG/JSON/model/font/sound 输入与解码聚合预算；UpdateAttributes
  session holder 校验；SetEntityData 生成 schema 校验 + rich value/session
  identity foundation；repo-native immutable command dispatcher；ChatListener
  presentation；AddEntity client lifecycle；157 EntityType spawn/default 与
  transform/interpolation 全量 audit（两类目全 complete）；entity client
  tick/movement 里程碑开跑（FishingHook、boat 物理/碰撞/paddle 音频、
  minecart 烟雾、Enderman/Guardian/Nautilus ambient、equine/Camel/Pig/Strider
  ridden travel、catch-up scheduler）。
- **2026-07-21**：remote boat client tick 状态；entity 与 client-level tick
  交错；entity block effect tick 策略生成与批量应用；Strider renderer
  fixture 稳定；文档瘦身工作流 v5（本索引取代逐 slice 归档散文，goal.md
  优先级刷新，已闭环 design doc 迁入 docs/archive/）；
  `Entity.applyEffectsFromBlocks` 边界全闭合（雪花粒子收尾）；live gate
  基础游玩新增实体攻击场景（summon→pick 面→瞄准攻击→canonical/呈现双移除，
  evidence schema 3，live 实跑全绿）与容器场景（setblock+server 填箱→右键
  开箱→quick-move→关箱→server console 权威确认箱空，evidence schema 4，
  live 实跑全绿）与 chat 场景（新 `net.send_chat_message` RPC 走既有
  last-seen ack 出站管线；消息+emote 命令双 round-trip：server 落日志且
  广播回 canonical chat 状态，evidence schema 5，live 实跑全绿）与 hotbar
  场景（input.key 扩展 hotbar1-9 数字键走真实键盘 handler；canonical
  selected_hotbar_slot + server `data get entity SelectedItemSlot` 双确认，
  evidence schema 6，live 实跑全绿）与声音/粒子推送场景（server console
  playsound/particle → canonical sound_packets+last_sound 精确
  volume/pitch/位置 + level_particles_packets 计数，evidence schema 7，
  live 实跑全绿）与 deterministic screenshot 场景（冻结全部服务端动画源 +
  pitch-90 排除云漂移，双帧 readback 按每通道 ≤3 容差比较——vanilla lightmap
  blockLightFlicker 每 tick 随机游走使全屏小幅抖动成为语义本身，实测跨
  ~10 tick 采样间隔漂移达 ±2 LSB；顺带修
  场景列探测缺中间列视线检查的间歇失败，evidence schema 8，live 实跑
  全绿）与 inventory 背包场景（E 键真实路径开背包 + player menu
  quick-move + server playerdata Slot 区间双确认，evidence schema 9）；
  **修正 canonical 缺口**：vanilla 容器 menu 与背包共享底层 Inventory，
  快照镜像未回写——容器 set_content/set_slot/本地 click 的尾部 36 槽玩家段
  现镜像回 player_slots 与 survival menu 投影。**至此 P1 playable 阶段 1
  基础游玩交互全部闭环**；随后阶段 2 扩展失败证据亦闭环（evidence schema
  10）：失败按 server_start/network/canonical_apply/runtime_side_effect/
  presentation 五档分类并标注场景，失败时刻 best-effort 采集同代
  renderer/audio/net/world counters、pending 写探针双采样与最终
  screenshot，四处 evidence 组装收敛为单一 `_write_live_evidence`。
  **playable 门禁阶段队列清空，转入维护态**。entity 矩阵第四类目
  **client tick/movement 全量 157 行 complete**：四个 runtime checkpoint
  （生成 policy/免疫、swept 移动求值、nonliving 家族、local
  Living/vehicle+arrow/rain/player aiStep）已全部落地门禁，本轮 phase 5
  coverage closeout 把 base cell 翻 complete（`client_tick_audit` 证据集：
  调度/求值/policy/动画推进 + 五个语义测试）、退役 Turtle not-applicable
  试点（server MoveControl 权威、无客户端可观察差异，通用 remote-Living
  行覆盖）、arrow 单段折叠记录为 vanilla-faithful；合同归档
  `docs/archive/entity-client-tick-movement-design.md`。同日 live gate
  抗随机性批（commit "Stabilize live gate scenarios against world randomness"，evidence schema 11）：修 26.1 gamerule 更名缺口
  （legacy camelCase 名静默失败，daylight/weather/随机 tick 从未冻结——
  权威对照 `GameRuleRegistryFix`），screenshot 场景加脚下静态石台排除
  动画纹理，entity 场景 sight-line 全堵时服务端凿列，sound 场景推送前
  清 mob 等声道静默。
- **2026-07-21（同日续）**：entity 矩阵第五类目 **save/debug 全量 157 行
  complete**。F3+I Shift-local `saveWithoutId` 投影全 dispatch 审计闭环：
  24 个 default-arm 类型全部裁决——villager/zombie-villager（VillagerData
  registry 身份重绑）、warden、ender-dragon、area-effect-cloud、
  eye-of-ender、item-entity、ominous-item-spawner（commit "Project
  remaining synced families into local save NBT"）、Display 三兄弟共享
  transformation/billboard/interpolation/shadow/brightness 层 + 各自
  block_state / item+item_display 上下文 / text+style-flags（commit
  "Project display trio local save debug fields"）、Mannequin
  profile/hidden_layers/main_hand/pose/immovable/description 含
  hide_description 与默认 translatable label 省略（commit "Project
  mannequin avatar fields into local save NBT"）；FishingHook/LeashKnot/
  LightningBolt/Marker vanilla 空体验证，probed Player
  `addAdditionalSaveData` 全 server-private 验证（default arm 即
  vanilla-faithful）。投影政策：只投 synced/client-owned 字段，server
  私有字段缺席而非默认值。`save_debug_audit` 证据集扩为
  组装/分派/Mannequin/Display 实现 + 四语义测试 + 三 vanilla 锚点，
  base cell 翻 complete；顺带补齐 coverage JSON 引用的
  events/particles/audio 悬空锚点标题。
- **2026-07-21（同日再续）**：entity 矩阵第六类目 **dimensions/collision/pick
  全量 157 行 complete**。三轮 vanilla 全 override 枚举审计：①bounds sweep
  （37 个 `getDimensions`/`getDefaultDimensions` 覆盖逐一对账，wired
  AreaEffectCloud 半径维度，F3+B debug hitbox 对齐
  `EntityHitboxDebugRenderer` 全非 invisible 语义，commit "Align debug
  hitboxes with vanilla visibility semantics"）；②眼高 sweep（18 处
  `withEyeHeight` 显式声明改用声明 (eye,height) 基准，纯 scale 变体保持
  比例——vanilla `EntityDimensions.scale` 等比缩放眼高，commit "Project
  exact vanilla variant eye heights"）；③attachment sweep（碰撞面
  `canBeCollidedWith`/pushability kinds/`makeBoundingBox` 三覆盖/pick 半径
  验证无缺口；baby 马系/羊驼/熊猫 `withAttachments` 重声明骑乘点落地，
  commit "Ride baby vehicles at vanilla declared attachments"）。
  `dimensions_pick_audit` 证据集扩为 6 实现 + 6 语义测试 + 3 vanilla 锚点。
  开放类目余 4：model/layers、animation、equipment/passenger/leash、
  event/particle/audio。
- **2026-07-21（FishingHook renderer 工程切片）**：闭环 world 同帧 owner/持竿手
  状态、第一/第三人称手部起点、专用裁剪纹理四边形与 16 段屏幕宽度抛物线；
  commit "Render dedicated fishing hook and line"，语义测试覆盖三层边界。
- **2026-07-21（generic entity name display 工程切片）**：闭环 rich/team/score
  world 投影、camera/crosshair/F1 可见性、生成 attachment 与 dispatcher 偏移，
  以及 score→name、see-through→normal 的有序双层字体提交。
- **2026-07-22（entity model/texture/layers 全量闭环）**：生成的 155 个
  `EntityRenderers` 注册加 Player/Mannequin 例外唯一分区全部 157 行，native
  逆向测试拒绝生产 raw fallback；独立 pass、族模型/纹理/层、世界文字自定义
  字体与等宽扰动均有语义证据，矩阵第七类目翻为 complete。
- **2026-07-22（entity animation 全量闭环）**：逆向 157 个 presentation owner
  与生产 model kind，补齐 Armadillo、baby Axolotl、Creaking、Guardian、Humanoid
  顺序和 Dragon heightmap 边缘；Rabbit 本地 RNG idle 明确为 not-needed，矩阵第八类目翻为 complete。
- **2026-07-22（entity event/particle/audio 全量闭环）**：生成的 35 个
  `handleEntityEvent` 声明与三个 listener special 全量逆向，补齐 Rabbit、Hoglin/
  Zoglin/Creaking 和 Slime/Magma Cube 音画消费者，并以 Firework event/tick 守卫证明
  既有爆炸、轨迹及近远音频路径；矩阵第九类目 157 行翻为 complete。
- **2026-07-22（157 entity family 矩阵全量闭环）**：equipment packet Living gate、8 个 wire slot、生成 `canUseSlot`、canonical query 与资源包装备 layer；passenger attachment/controller owner、当前 render-pose 投影与 first-mount side effect；leash 全 offset/rope-holder owner 与两条 rope strip 均有反向门禁。
  第十类目 157 行翻为 complete，矩阵只余 3 个 Vanilla NoopRenderer 的窄 not-applicable cell；工程合同见
  `docs/entity-equipment-passenger-leash-closure-design.md`。
- **2026-07-22（network HolderSet identity 全量闭环）**：十个 `ByteBufCodecs.holderSet`
  调用点完成固定/同步 registry 校验、Ingredient 原子应用、UpdateTags revision 与反向自定义顺序证明；账本 family 全部翻为 complete，合同见
  `docs/registry-network-holder-set-closure-design.md`。
- **2026-07-22（persistent HolderSet identity 全量闭环）**：48 个 `RegistryCodecs.homogeneousList`
  调用点由 registry-owned scalar/list/`#tag` 解析器统一收口；14 个客户端组件/谓词与 6 个同步附魔行翻为 complete，28 个服务端专属行以同步边界裁决为 not-applicable，合同见
  `docs/persistent-holder-set-coverage-design.md`。
- **2026-07-22（persistent Holder reference identity 全量闭环）**：38 个 named persistent
  Holder 调用点由 registry-owned resource identity 解析器统一收口；25 个客户端/同步 registry 行翻为 complete，13 个服务端专属行裁决为 not-applicable，DamageType 与 GameEvent 引用纳入同步附魔原子校验，合同见
  `docs/persistent-holder-reference-coverage-design.md`。
- **2026-07-22（persistent Holder-or-inline 全量闭环）**：Instrument stream 保留完整 direct
  值并可精确持久化，TestEnvironment/Instrument/SoundEvent 三个客户端行共用 registry-owned named/direct 路由；20 个 worldgen/trial/loot 服务端行裁决为 not-applicable，合同见
  `docs/persistent-holder-or-inline-coverage-design.md`。
- **2026-07-22（DimensionType.NETWORK_CODEC 全字段闭环）**：原子 typed revision 保留全部标量、
  `infiniburn` hashed TagKey、七类有界 `IntProvider` 与 monster light settings，并将当前 session holder 投影到 level 快照；合同见 `docs/dimension-type-network-codec-design.md`。
- **2026-07-22（registry holder/tag 194 行全量闭环）**：SlotDisplay 网络 item tag 保留规范身份并在每次配方展示时读取当前 UpdateTags 顺序，替换与删除都有 native 回归；其余 25 个 persistent/generic 调用点以 Vanilla 同步边界裁决为 not-applicable，账本归零 partial；合同见 `docs/registry-holder-tag-final-closure-design.md`。
- **2026-07-22（configuration registry 双包闭环）**：RegistryData contents 与 UpdateTags 在 configuration pending candidate 中隔离，Play 转换时将 typed registry、tag、item defaults 与 native audio/presentation 原子发布；双包 packet coverage 全单元 complete，合同见 `docs/configuration-registry-packets-closure-design.md`。
- **2026-07-22（SetEntityData 包闭环）**：43 个 Vanilla metadata serializer 与解码分支建立生成反向约束；schema、holder session identity、原子 canonical apply 及 157×10 entity consumer 矩阵共同封闭五类 packet coverage，合同见 `docs/set-entity-data-packet-closure-design.md`。
- **2026-07-22（queued light 三包族闭环）**：LevelChunkWithLight/LightUpdate 共用
  world-owned FIFO 与 Vanilla 有界 frame poll，ForgetLevelChunk 原子清除 canonical
  与 pending light；三行 packet coverage 全单元 complete，合同见
  `docs/queued-light-packet-family-closure-design.md`。
- **2026-07-22（Explosion 包闭环）**：registry-dispatched 主粒子、sound holder 与
  weighted block-particle list 完整解码；世界层按 Vanilla 顺序记录定位音频、主粒子
  与击退，原生 `ClientExplosionTracker` 实现 512 上限、空气筛选、权重采样和 level
  generation 清理。packet coverage 全单元 complete，合同见
  `docs/explosion-packet-closure-design.md`。
- **2026-07-22（BlockEvent 包闭环）**：固定 block holder 严格解码后按当前位置状态
  分派完整 `triggerEvent` 族；note block 音画与 piston 原子移动、碰撞、running tick、
  插值场景提交共用 world canonical 状态。packet coverage 全单元 complete，合同见
  `docs/block-event-packet-closure-design.md`。
- **2026-07-22（BlockUpdate 双包族闭环）**：固定 29,873 项 block-state identity 与
  生成的 block→block-entity 映射驱动严格解码、预测协调和 Vanilla 移除/创建/铜族保留；
  原生地形仅替换脏 section 并复用稳定 mesh。双包 coverage 全单元 complete，合同见
  `docs/block-update-packet-family-closure-design.md`。
- **2026-07-22（BlockDestruction 包闭环）**：同一 server/local 最高 stage 投影驱动
  任意 baked block quad 与 Vanilla 允许的 block-entity model submission；精确
  `SheetedDecalTextureGenerator` UV、render-type 筛选及共享 crumbling pass 均有语义
  回归，合同见 `docs/block-destruction-packet-closure-design.md`。
- **2026-07-22（BossEvent 包闭环）**：trusted rich Component 与 `LinkedHashMap` 顺序
  进入 canonical world，native HUD 复现 100ms `LerpingBossEvent`，三类环境 flag 覆盖
  lightmap/fog/sky 与 backend-telemetry 驱动的 End boss music；合同见
  `docs/boss-event-packet-closure-design.md`。
- **2026-07-22（UpdateAdvancements 包闭环）**：trusted rich display、按包顺序的
  canonical tree/progress 与有界 completion handoff 驱动 native 五槽 toast、Vanilla
  滑动/淡入/音效及组件感知图标；合同见 `docs/update-advancements-packet-closure-design.md`。
- **2026-07-22（MerchantOffers 包闭环）**：`ItemCost` 精确组件谓词值进入 canonical
  display patch，商人菜单补齐七行按钮 hover/focus、完整交易栈装饰与 tooltip、折扣删除线、
  补货提示及权威结果槽驱动的 XP 结果段；五类 packet coverage 全部 complete，合同见
  `docs/archive/merchant-offers-packet-closure-design.md`。
- **2026-07-22（命令补全三包族闭环）**：Commands 驱动 selector、block-state 与 item
  component 本地子语法，summon tooltip 在当前资源语言中解析；请求/响应收敛为单 pending
  与单结果，三行 packet coverage 全单元 complete，合同见 `docs/archive/command-completion-packet-closure-design.md`。
- **2026-07-22（CommandBlockEditScreen 包闭环）**：权威 command-block block-entity
  投影驱动原生编辑器与七行命令补全，Done 精确发送 typed packet，Cancel/Escape 零发送；
  五类 packet coverage 全部 complete，合同见 `docs/archive/command-block-editor-packet-closure-design.md`。
- **2026-07-22（TestInstanceBlockEditScreen 双向包闭环）**：typed 嵌套 block-entity
  数据与 screen-scoped rich status/optional size 驱动原生编辑器；INIT/QUERY、四类关闭动作、
  Save 守卫和 Cancel/Escape 均有精确发包回归，双向 coverage 全单元 complete，合同见
  `docs/archive/test-instance-block-editor-packet-closure-design.md`。
- **2026-07-22（StructureBlockEditScreen 包闭环）**：typed block-entity 权威投影驱动
  四模式原生编辑器，Done/Save/Load/Detect 精确发包且 Cancel/Escape 零发送；五类 packet
  coverage 全部 complete，合同见 `docs/archive/structure-block-editor-packet-closure-design.md`。
- **2026-07-22（JigsawBlockEditScreen 双包闭环）**：typed jigsaw NBT 与朝向 joint 规则驱动
  原生六字段编辑器、0～20 滑条和 keep-jigsaws；Generate 严格按 Set→Generate 发包，
  两行 packet coverage 的全部适用单元 complete，合同见 `docs/archive/jigsaw-block-editor-packet-closure-design.md`。
- **2026-07-22（MinecartCommandBlockEditScreen 包闭环）**：命令方块矿车的 generated
  command/last-output metadata 与本地 track-output 状态驱动原生编辑器；右键开屏仍发送
  Interact，Done/Enter 精确发送 typed packet，Cancel/Escape 零发送，五类 packet
  coverage 全部 complete，合同见 `docs/archive/minecart-command-block-editor-packet-closure-design.md`。
- **2026-07-22（TestBlockEditScreen 包闭环）**：typed mode/message/powered block-entity
  投影驱动原生四态编辑器；START 隐藏但保留消息，Done 精确发送 typed packet，Enter、
  Cancel 与 Escape 零发送，普通 UseItemOn 和服务端 mutation/block-update 权威保持不变；
  五类 packet coverage 全部 complete，合同见
  `docs/archive/test-block-editor-packet-closure-design.md`。
- **2026-07-22（SignEditScreen 双向包闭环）**：raw/filtered canonical 快照与启动过滤策略
  贯穿世界文字和编辑预载；90/60px 实测宽度、选择/剪贴板/IME、Done 与移除/越界单次提交
  全部闭环，双向 coverage 全单元 complete，合同见 `docs/archive/sign-editor-packet-closure-design.md`。
- **2026-07-22（PlayerAction 八动作包闭环）**：START/ABORT/STOP 按预测顺序驱动方块替换、
  本地挥手、轮廓击中粒子及 redstone/dragon-egg attack 回调；drop/release/swap/STAB 保持各自
  客户端守卫与服务端权威，五类 packet coverage 全部 complete，合同见 `docs/player-action-packet-closure-design.md`。
- **2026-07-22（实体 transform 五包族闭环）**：PositionSync、三类 MoveEntity 与 Teleport
  统一到递归 controlling-passenger 权威、loaded-section/4096 插值、碰撞目标 carry、旧姿态
  partial-tick 及乘客/车辆回显路径；五行 packet coverage 全单元 complete，沿用
  `docs/archive/entity-transform-interpolation-design.md` 工程合同。
- **2026-07-22（DamageEvent 包闭环）**：生成的 92 种 living EntityType 伤害声音/音量
  策略驱动 canonical walk、hurt/invulnerability 与 40-tick source 生命周期，并在状态
  写入后下发 native 音频；五类 packet coverage 全部 complete，合同见
  `docs/archive/damage-event-packet-closure-design.md`。
- **2026-07-23（EntityEvent 包闭环）**：生成 92 个 living death sound policy
  与 88 个声明 owner，补齐 sound-before-state、first-death lifecycle/睡眠起身及
  92 类型 native 音频穷举；五类 packet coverage 全部 complete，合同见
  `docs/archive/entity-event-packet-closure-design.md`。
- **2026-07-23（UpdateRecipes 包闭环）**：11 种内建 SlotDisplay 完成有界 typed decode、
  当前 tag/fuel/potion/component/dye/trim 上下文解析和原子代次替换；Stonecutter 按钮与
  tooltip 共用 world 首栈解析，五类 coverage 全部 complete，合同见
  `docs/archive/update-recipes-packet-closure-design.md`。
- **2026-07-23（BlockEntityData 包闭环）**：固定 49 项 BlockEntityType holder 与必需
  CompoundTag 严格解码；已有同类型 canonical record 原子替换并按类型投影到编辑器和
  原生场景，五类 coverage 全部 complete，合同见
  `docs/archive/block-entity-data-packet-closure-design.md`。
- **2026-07-23（Animate 包闭环）**：action 2 按 Player 门控复用床边安全落点，原子更新
  standing/sleepingPos、本地姿态与 player-only sleepCounter；100～109 计时驱动
  `0x101020` 原版 HUD 淡出，actions 0/3/4/5 保持既有行为，五类 coverage 全部
  complete，合同见 `docs/archive/animate-player-wake-packet-closure-design.md`。
- **2026-07-23（Attack 包前置闭环）**：固定物品原型与 ItemStack patch 驱动攻击蓄力，
  双强度时钟、missTime、船只忙手及功能门控复现原版输入顺序；连接按选槽修复→攻击发包，
  全部适用 coverage 单元 complete，合同见
  `docs/archive/local-attack-preflight-closure-design.md`。
- **2026-07-23（环境位置采样 foundation）**：生成的 48 项属性目录与 world-owned
  `EnvironmentAttributeMap` 完成维度→直接/高斯生物群系→timeline 顺序，并迁移全部既有
  位置消费者；合同见 `docs/archive/environment-position-sampling-foundation-design.md`。
- **2026-07-23（环境效果消费者闭环）**：通用音乐调度、群系 ambient loop/mood/additions
  与 667 对 animate 粒子/萤火虫/嘎枝之心效果完成原版采样、随机链和音频/粒子桥；
  `CLIENTBOUND_CHUNKS_BIOMES` 全部 coverage complete，合同见
  `docs/archive/environment-effect-consumers-design.md`。
- **2026-07-23（游戏规则屏幕双包闭环）**：生成的 59 项 typed 规则目录驱动首响应、
  权限、编辑、搜索、校验与 changed-only 提交；native 输入和 renderer 复现加载列表及
  放弃确认，两行 packet coverage 全单元 complete，合同见
  `docs/archive/game-rules-screen-closure-design.md`。
- **2026-07-23（书本屏幕双包闭环）**：world-owned View/Edit/Sign 生命周期保留原始/
  过滤 Component，普通 UseItem 预测打开编辑器并由 held-slot authority 守卫提交；
  native 富文本书页、悬浮/点击令牌及签名 UI 闭环，两行 packet coverage 全部适用单元
  complete，合同见 `docs/archive/book-screens-closure-design.md`。
- **2026-07-23（Action-bar/title 富 Component 四包闭环）**：三类 title text 包与
  `SystemChat(overlay=true)` 共用 exact world Component、本地化样式投影及逐 run 字体簿
  测量/混淆/绘制；四行 packet coverage 全单元 complete，合同见
  `docs/archive/action-bar-title-component-closure-design.md`。
- **2026-07-23（Waypoint locator bar 包闭环）**：四类 waypoint 数据按 active camera
  复现距离/yaw/pitch、UUID entity follow、Java tint 与 far-first overlap；selected-pack
  style/sprite、missingno、contextual priority 和事务 reload 完成像素级闭环，五类 coverage
  全部 complete，合同见 `docs/archive/waypoint-locator-bar-closure-design.md`。
- **2026-07-23（World-border vignette 六包族闭环）**：camera-local lightmap 亮度平滑与
  边界静态/移动 warning 阈值驱动原版 tint；selected-pack vignette、精确 destination
  multiply blend 和事务 reload 完成像素级闭环，六行 coverage 全部 complete，合同见
  `docs/archive/world-border-vignette-closure-design.md`。
- **2026-07-23（GameTest highlight 包闭环）**：绝对位置 map 复现同键刷新、严格
  10 秒到期与 level reset；0.02 膨胀绿盒和相对坐标标签经透明合成后的独立深度清除
  pass 呈现，五类 coverage 全部 complete，合同见
  `docs/archive/game-test-highlight-presentation-closure-design.md`。
- **2026-07-23（玩家头颅方块 profile 皮肤闭环）**：共享 ResolvableProfile Codec
  投影贯穿初始区块/BlockEntityData 原子 owner、异步 profile/skin generation 与现有
  dynamic atlas；profileless/ProfiledDefault/ready/failure fallback 均锁定，合同见
  `docs/archive/player-head-block-profile-skin-closure-design.md`。
- **2026-07-23（Mob-effect 客户端行为闭环）**：生成的 40 项目录驱动 canonical
  duration/blend tick、移动/光照/雾/天空/恶心投影和 HUD/背包效果面板；Update/Remove
  两行 coverage 全单元 complete，合同见 `docs/archive/mob-effect-client-behavior-design.md`。
- **2026-07-23（记分板客户端呈现闭环）**：五包族保留任意 signed method byte，
  canonical 富 Component/队伍/number-format 状态驱动右侧 sidebar 与 Tab LIST
  整数/红心目标；UUID 延迟血量与闪烁由 renderer 独占，全部 coverage 单元 complete，
  合同见 `docs/archive/scoreboard-client-presentation-design.md`。
- **2026-07-23（多人观战菜单闭环）**：数字键、中键与滚轮驱动 root/player/team
  菜单、分页/淡出和 UUID 传送；原生 HUD 复现 pack-backed 图标、玩家/队伍头像与
  热栏替换，两行 serverbound coverage 全单元 complete，合同见
  `docs/archive/spectator-gui-menu-design.md`。
- **2026-07-23（GameEvent Win/demo 呈现闭环）**：类型 0/4/5 的富系统消息、
  诗歌 credits、PerformRespawn、demo 提示/可信链接与旁白贯穿 canonical→native；
  pack 资源、credits 音乐、末地传送门/弹窗/vignette 像素呈现使该包五类 coverage
  全部 complete，合同见 `docs/archive/game-event-win-demo-presentation-design.md`。
- **2026-07-23（PlayerCombatKill 与死亡界面闭环）**：terminal trusted Component、
  local-player 身份、score/hardcore 与 showDeathScreen 分支进入 generation-safe
  canonical 状态；20-tick Respawn/Spectate、退出/链接确认、旁白和一次性命令贯穿
  native，普通 HUD 后的红色渐变、富文本字体/链接命中及离屏像素完成 renderer
  呈现；五类 coverage 全部 complete，合同见
  `docs/archive/player-combat-kill-death-screen-design.md`。
- **2026-07-23（PlayerSocialManager 与 Social Interactions 闭环）**：PlayerInfo
  Add/Remove 驱动 canonical 发现/移除状态及原生保留行；P 键、暂停入口、搜索/分页/
  隐藏/屏蔽、可信链接、旁白和 pack-backed renderer 全链闭合，`CLIENTBOUND_PLAYER_INFO_REMOVE`
  五类 coverage 全部 complete，合同见 `docs/archive/player-social-interactions-design.md`。
- **2026-07-23（ItemStack component coverage foundation）**：110 项
  `DataComponents` 的 value type、persistent/显式或回退 network Codec 与六类语义
  单元改由 Tree-sitter 生成；稀疏 evidence overrides、Rust catalog 反向断言和
  component-dependent packet closure guard 落地，合同见
  `docs/archive/item-component-coverage-foundation-design.md`。
- **2026-07-23（递归 ItemStack component 安全前置）**：`USE_REMAINDER`、
  `CHARGED_PROJECTILES`、`BUNDLE_CONTENTS`、`CONTAINER` 的嵌套模板统一采用
  depth-32 解码预算并由 2 MiB 线程上的 1.5 MiB 实测栈门禁反向约束；网络 count
  对齐 Vanilla 非零规则，bundle 权重改为迭代工作栈，tooltip 与持久化调试投影
  拒绝越界递归且不生成截断 SNBT。四项账本只补 partial evidence，未虚报通用
  patch 重编码或无损持久化闭环，合同见
  `docs/archive/recursive-item-component-safety-design.md`。
- **2026-07-23（基础 ItemStack component 六维闭环）**：`MAX_STACK_SIZE`、
  `MAX_DAMAGE`、`DAMAGE`、`UNBREAKABLE` 获得 descriptor-owned 编码、
  canonical 原子预检和 creative untrusted 逐值 framing；四行六维 coverage
  complete，其余 added-value 仍精确拒绝，合同见
  `docs/archive/base-item-component-roundtrip-design.md`。
- **2026-07-23（CustomData component 六维闭环）**：persistent-fallback Codec
  现严格保留非空类型的 CompoundTag 根并由 descriptor-owned bounded NBT 编码
  进入 creative 逐值 framing；canonical publication、持久化 SNBT 与 exact/partial
  item-model predicate 证据齐备，合同见
  `docs/archive/custom-data-component-roundtrip-design.md`。
- **2026-07-23（Typed entity-data component 六维闭环）**：`ENTITY_DATA`、
  `BUCKET_ENTITY_DATA`、`BLOCK_ENTITY_DATA` 现严格校验固定 registry holder 与
  CompoundTag 根、按 Vanilla 剥离网络顶层 `id`，并由 descriptor-owned 编码进入
  creative 逐值 framing；三行六维 coverage complete，合同见
  `docs/archive/typed-entity-data-component-roundtrip-design.md`。
- **2026-07-23（Resolvable profile component 六维闭环）**：`PROFILE` 的
  GameProfile/Partial 判别、Java UTF-16/属性预算与 PlayerSkin.Patch 现可无损
  重编码；派生 profile textures/资源路径规范、creative framing、持久化与异步
  皮肤消费证据齐备，合同见
  `docs/archive/profile-component-roundtrip-design.md`。
- **2026-07-23（Attribute modifiers component 六维闭环）**：
  `ATTRIBUTE_MODIFIERS` 现保留 override Component 原始 NBT，并按固定 attribute
  holder、modifier、slot 与 display union 精确重编码；creative framing、持久化
  SNBT、tooltip 与 item-model predicate 证据齐备，合同见
  `docs/archive/attribute-modifiers-component-roundtrip-design.md`。
- **2026-07-23（Written book content component 六维闭环）**：
  `WRITTEN_BOOK_CONTENT` 现按 Java UTF-16/generation 边界保留并重编码 title、
  author、raw/filtered Component pages 与 resolved；creative framing、持久化
  SNBT、富页面屏幕、tooltip 与 predicate 证据齐备，合同见
  `docs/archive/written-book-content-component-roundtrip-design.md`。
- **2026-07-23（Blocks attacks component 六维闭环）**：
  `BLOCKS_ATTACKS` 现逐位重编码全部 float、damage-type holder-set 与
  sound-holder union；creative framing、持久化 Codec/SNBT、动态 shield 默认值
  及第一/第三人称格挡姿势证据齐备，合同见
  `docs/archive/blocks-attacks-component-roundtrip-design.md`。
- **2026-07-23（Consume-effect component 家族六维闭环）**：
  `CONSUMABLE` 与 `DEATH_PROTECTION` 现按内建 effect registry 精确重编码
  apply/remove/clear/teleport/sound 五类分支、递归 mob-effect、holder-set 与
  sound-holder union；creative framing、持久化 Codec/SNBT、默认组件感知的
  item-model condition 与第一人称使用姿势证据齐备，合同见
  `docs/archive/consume-effects-component-roundtrip-design.md`。
- **2026-07-23（Weapon-interaction component 家族六维闭环）**：
  `PIERCING_WEAPON`、`KINETIC_WEAPON` 与 `SWING_ANIMATION` 现精确重编码
  可选条件、raw float/VarInt、sound-holder 与生成 animation id；creative
  framing、持久化 Codec/SNBT、STAB 输入、spear feedback 和第一/第三人称动画
  证据齐备，合同见
  `docs/archive/weapon-interaction-components-roundtrip-design.md`。
- **2026-07-23（Armor trim component 六维闭环）**：`TRIM` 现精确重编码
  dynamic registry/direct Holder、材质资源覆盖与 canonical Component NBT；
  creative framing、持久化 Codec/SNBT、配方、谓词、tooltip 和 named/direct
  armor sprite 证据齐备，合同见
  `docs/archive/trim-component-roundtrip-design.md`。
- **2026-07-23（Firework component 家族六维闭环）**：
  `FIREWORK_EXPLOSION` 与 `FIREWORKS` 现精确重编码形状、颜色/渐变、trail、
  twinkle、raw flight VarInt 与最多 256 个嵌套爆炸；creative 持久化范围、
  framing、SNBT、tooltip/tint、rocket 状态及粒子/声音证据齐备，合同见
  `docs/archive/firework-components-roundtrip-design.md`。
- **2026-07-23（Potion effect component 家族六维闭环）**：
  `POTION_CONTENTS` 与 `SUSPICIOUS_STEW_EFFECTS` 现精确重编码可选 potion
  holder、颜色、递归 mob-effect、名称及 stew effect/duration 列表；creative
  framing、SNBT、tint/tooltip、谓词与配方证据齐备，合同见
  `docs/archive/potion-effect-components-roundtrip-design.md`。
- **2026-07-23（Potion duration scale component 六维闭环）**：
  `POTION_DURATION_SCALE` 现保留并精确重编码 raw float 位型，creative
  持久化入口执行 `NON_NEGATIVE_FLOAT` 范围；tipped-arrow/lingering-potion
  原型倍率由 `Items.java` CST 派生，经资源重载进入 world 有效 patch，并按
  Vanilla 顺序缩放 potion tooltip，合同见
  `docs/archive/potion-duration-scale-roundtrip-design.md`。
- **2026-07-23（Damage type component 六维闭环）**：`DAMAGE_TYPE` 现按
  session holder 顺序解码、重编码并进入 creative framing；spear 原型默认由
  `Items.java` 派生并经 frozen registry 解析，generic component-select 消费
  显式/default/removal 值，合同见
  `docs/archive/damage-type-item-component-design.md`。
- **2026-07-23（Adventure-mode predicate component 六维闭环）**：
  `CAN_PLACE_ON` 与 `CAN_BREAK` 现无损编解码并按 holder/tag、typed state 与部分
  NBT 求值，驱动冒险模式破坏和方块选框；合同见
  `docs/archive/adventure-mode-predicate-components-design.md`。
- **2026-07-23（递归 ItemStack component 六维闭环）**：`USE_REMAINDER`、
  `CHARGED_PROJECTILES`、`BUNDLE_CONTENTS` 与 `CONTAINER` 现按普通嵌套 patch
  framing、精确 container 槽跨度和 depth-32 原子预算无损编解码；creative
  持久化范围、严格嵌套 SNBT 及 item-model/tooltip 消费证据齐备，合同见
  `docs/archive/recursive-item-components-roundtrip-design.md`。
- **2026-07-23（Tool component 六维闭环）**：`TOOL` 现按 holder-set、raw
  float、VarInt 与 creative flag 精确重编码；默认与 patch 后的规则驱动挖掘
  速度、正确掉落及 creative 本地破坏预测，六维证据见 item component coverage。
- **2026-07-23（Equippable component 六维闭环）**：`EQUIPPABLE` 现按 slot、
  sound-holder、资源键、entity holder-set 与五个 flag 精确重编码；栈补丁的
  asset/slot 覆盖默认装备材质投影，六维证据见 item component coverage。
- **2026-07-23（Food component 六维闭环）**：`FOOD` 现按 VarInt、raw float
  与 bool 精确重编码；`Foods.java` 默认值经资源重载进入 world 有效补丁，
  并按 `Player.canEat` 门控无目标本地使用与双手回退。
- **2026-07-23（Tooltip style component 六维闭环）**：`TOOLTIP_STYLE` 现按
  Identifier 精确编解码并驱动全部 ItemStack HUD tooltip；资源包同命名空间的
  background/frame 精灵对保留 stretch、tile、nine-slice 语义并精确选样式。
- **2026-07-23（Banner pattern provider component 六维闭环）**：
  `PROVIDES_BANNER_PATTERNS` 现按 named/direct holder-set 精确重编码，并以
  session registry 顺序驱动 Loom 输入与 HUD（`211acc28c`、`7984a8162`）。
- **2026-07-23（Debug stick state component 六维闭环）**：`DEBUG_STICK_STATE`
  现按内建 block holder 与 StateDefinition property 精确编解码，并覆盖
  creative framing、canonical 对话投影和持久化 SNBT。
- **2026-07-23（Item component coverage milestone 关闭）**：`LOCK` 现由
  protocol-owned persistent ItemPredicate Codec 驱动 typed NBT、creative
  framing 与 SNBT；110 个 26.1 ItemStack component 六维账本全部闭环。
- **2026-07-23（共享实体剪刀交互工程关闭）**：`Entity` 的拴绳连接与 Mob
  装备剪除现按 32×32×32 邻域、八槽顺序、动态 `PREVENT_ARMOR_CHANGE`、创造/载具/
  潜行条件执行本地 canonical 预测；默认可剪装备由 `Items.java` 派生并随资源重载发布。
- **2026-07-23（工具 useOn 预测工程关闭）**：斧、锄、铲、蜂蜜脾与剪刀的
  方块变换、判定和植物头部成员改由 Vanilla Tree-sitter 目录派生；方块命中面、
  prediction sequence、本地方块/物品突变、冒险与创造门控贯通 world/native。
- **2026-07-23（磁石指南针 useOn 工程关闭）**：`CompassItem.useOn` 现按当前
  维度写入磁石目标，覆盖生存/创造分栈与满背包丢弃分支及精确挥手序列；
  `null` excluded-player 的定位音效不在本地播放但仍推进声音种子，实际播放由
  服务端声音包驱动。
- **2026-07-23（点火 useOn 预测工程关闭）**：`FlintAndSteelItem` 与
  `FireChargeItem` 现覆盖可点亮方块、普通/灵魂火与空黑曜石门框；`FireBlock`
  易燃注册和灵魂火基座由 Vanilla 源/标签生成，音效随机流保留
  excluded-player 语义。
- **2026-07-23（骨粉 useOn 预测工程关闭）**：`BoneMealItem` 现按生成的
  Bonemealable owner、replaceable/skylight 属性、实时 NYLIUM 标签与邻域形状，
  精确区分客户端作物 `PASS` 和满水方块 `SUCCESS`，且只在后者本地挥手。
- **2026-07-23（水药水泥化 useOn 预测工程关闭）**：`PotionItem.useOn` 现按
  内建 water holder 与实时 `convertable_to_mud` 标签预测泥默认态，完整执行
  生存/创造 `createFilledResult`、满背包客户端丢弃挥手，并为两次
  `null` excluded-player 音效推进种子而不提前播放或生成服务端 splash 粒子。
- **2026-07-23（乐器 use 预测工程关闭）**：`InstrumentItem.use` 现解析命名与
  直接 `INSTRUMENT` 持有者，按动态 duration/range 启动吹奏姿态、物品/组件冷却，
  并以本地玩家实体播放 `records` 声源而不误用 Vanilla bootstrap holder 顺序。
- **2026-07-23（Equippable 无目标交换预测工程关闭）**：
  `Equippable.swapWithEquipmentSlot` 现按源码派生 75 个可交换原型及 30 个玩家
  可装备原型，覆盖 holder/tag、绑定诅咒、单件/堆叠、创造/无限材料与满背包挥手，
  并贯通资源重载、world 库存突变和 native 主副手停机顺序。
- **2026-07-23（无目标投掷物 use 预测工程关闭）**：雪球/蛋/末影珍珠/经验瓶/
  喷溅与滞留药水/风弹现按 `ItemStack.use` 瞬时 after-use 顺序执行消耗和组件冷却；
  `Items.java` 的 3 个默认冷却值随资源重载发布，投掷音效的客户端 `null`
  excluded-player 调用仅推进 pitch/seed 随机流，等待服务端声音包实际播放。
- **2026-07-23（弓/望远镜 use 与 FOV 工程关闭）**：弓现按 live arrows tag、
  全玩家库存和无限材料执行 use 门禁，释放保留客户端无弹药/耐久突变及抑制声音
  随机顺序；望远镜覆盖 use/release/自然完成声音，二者共同进入 vanilla camera
  FOV target 与既有逐 tick easing。
- **2026-07-23（Bundle 无目标 use 工程关闭）**：Bundle 现按 selected-first
  与 tick 0/延迟后偶数节奏移除完整内容模板，客户端只预测主手挥动而不生成物品
  实体，并保留 remove-one 实播与 drop-contents 抑制音效的 pitch/seed 顺序。
- **2026-07-23（目标实体本地攻击预测工程关闭）**：普通实体攻击现按
  `send → Player.attack → reset → swing` 预测 redirectable projectile 改向及
  Vehicle/ShulkerBullet/EndCrystal 击退；伤害、附魔和暴击事件仍由服务端拥有。
- **2026-07-23（Equippable 目标装备预测工程关闭）**：
  `Equippable.equipOnTarget` 现按生成的鞍具/挽具原型所有权及 live entity tags，
  覆盖显式 patch、slot/存活/占槽门槛与 item subclass `PASS` 回退；目标装备和
  手持数量仍等待服务端权威修正。
- **2026-07-25（9 类方块空手 useWithoutItem 基础预测工程关闭）**：Button、Lever、
  Door、TrapDoor、FenceGate、Repeater、DaylightDetector、Comparator 与 RedStoneWire
  已贯通 Vanilla 停机、Lever 粒子和导线邻接；Comparator 动态输出刷新仍为纯 TODO。
- **2026-07-25（Comparator 动态输出刷新工程关闭）**：模式切换现按 Vanilla
  `refreshOutputState` 重算弱/强红石、全部生成式 analog owner、容器方块实体及唯一
  匹配物品展示框，并同步预测 `OutputSignal` 与 `powered`。
- **2026-07-25（ContainerButtonClick 四菜单闭环）**：Lectern 保持服务端回写，
  Enchantment 补齐物品/材料/经验门槛，Loom 与 Stonecutter 按 Vanilla 顺序预测
  data、结果槽和 UI 音效；该 serverbound packet 的五维覆盖现全部关闭。
- **2026-07-25（富 Component HashOps 工程关闭）**：协议 canonical Component
  树保留 primitive/array 类型与语义 owner；库存预测复用 profile、item patch
  和命名 dialog codec，direct dialog 继续 fail-closed。
- **2026-07-25（POV Item.use 基础里程碑）**：本地玩家眼位按同步交互距离执行
  `OUTLINE + Fluid` 射线；EnderEye/SpawnEgg 已精确消费命中结果，Trident 已覆盖
  耐久、同步 Riptide 水雨门禁与开始使用状态。
- **2026-07-25（Trident releaseUsing 工程闭环）**：补齐十 tick 释放重检、同步
  `TRIDENT_SOUND` 选择、服务端投掷所有权及 Riptide 冲量、贴地位移和私有旋转状态。
- **2026-07-25（Boat Item.use 工程闭环）**：ANY-fluid POV 现保留 double 命中点，
  并按 Vanilla 的眼点实体遮挡及新船体 block/entity/world-border `noCollision`
  判定本地 PASS/FAIL/SUCCESS；生成实体与物品消耗仍由服务端拥有。
- **2026-07-25（Bottle Item.use 工程闭环）**：SOURCE_ONLY 命中水源后按
  `ItemUtils.createFilledResult` 预测水瓶、满背包额外挥手与本地装瓶声音；龙息云
  owner 不上 wire，维持 Vanilla 远端客户端的服务端权威边界。
- **2026-07-25（BucketItem 本地预测工程关闭）**：空桶取水后的 19 类非默认
  `canSurvive` owner 现由生成支撑事实、动态形状与实时标签精确判定干态保留或销毁；
  空/水/岩浆/生物桶的方块、库存、声音、粒子与 prediction sequence 路径完成闭环。
- **2026-07-25（PlaceOnWaterBlockItem 本地放置工程关闭）**：睡莲与蛙卵现按
  生成 ownership、实时流体/方块标签、精确替换与实体阻挡规则委托至本地
  `BlockItem` 放置，贯通库存、声音及 prediction sequence 回滚。
- **2026-07-25（1023 个 BlockItem 本地放置工程关闭）**：生成目录中的全部
  BlockItem 主/墙体 placement owner 现按 Vanilla 26.1 命中上下文、实时邻域/
  标签/流体、共享随机流、组合方块写入及 prediction sequence 原子回滚执行。
- **2026-07-26（定向本地使用预测工程关闭）**：生成式 entity/mob、item 与 block
  owner 已贯通本地结果、canonical 预测及精确挥手/首人称反馈；`INTERACT`、
  `USE_ITEM_ON` 与 `USE_ITEM` 三条 serverbound 覆盖行不再含预测 `partial`。
- **2026-07-26（OpenScreen 精确标题与全菜单呈现闭环）**：可信 title Component
  从协议解码到 world 生命周期保持 exact，25 个生成式 MenuType 均有反向布局与
  标签守卫；Vanilla 坐标、Merchant 组合及真实字形居中完成，clientbound
  `OPEN_SCREEN` 五维覆盖全部关闭，合同见
  `docs/archive/open-screen-presentation-closure-design.md`。
- **2026-07-26（PlaceGhostRecipe 全语义闭环）**：五种 RecipeDisplay 与十一种
  SlotDisplay 已贯通结构化解码、world 不可变候选快照、五类 screen 的 Vanilla
  槽位/fill/tooltip 呈现；clientbound 五维覆盖全部关闭，合同见
  `docs/archive/place-ghost-recipe-packet-closure-design.md`。
- **2026-07-26（远端合成结果预测工程关闭）**：Inventory/Crafting 结果槽保持
  服务端权威，客户端按 Vanilla `ResultSlot` 仅预测一次定位输入消耗与物品默认
  余留物；RecipeBook 结果合成和配方特定余留物目录已移除，合同见
  `docs/archive/client-crafting-result-prediction-design.md`。
- **2026-07-26（RecipeBook 生命周期工程闭环）**：RecipeToast 与 AdvancementToast
  共用有序五槽调度，高亮配方按可见按钮顺序完成自动 SeenRecipe、动画、提示和旁白；
  双向 packet coverage 适用单元全部关闭，合同见
  `docs/archive/recipe-book-lifecycle-closure-design.md`。
- **2026-07-26（Creative inventory 与 SetCreativeModeSlot 闭环）**：十四类
  Vanilla tab 目录、动态物品、原生屏幕/交互、原子发包与服务端校正全链闭合，
  三条 packet coverage 行全部 complete，合同见
  `docs/archive/creative-inventory-set-slot-closure-design.md`。
- **2026-07-26（Special item resident 投影工程闭环）**：十六类
  `minecraft:special` 叶与局部变换现贯通 GUI、手持、掉落物、展示实体和物品框，
  Banner/Shield/Profile/Pot 组件及显式资源包纹理进入原生动态/owned residency。
- **2026-07-26（Special GUI context 与裁剪工程闭环）**：GUI resident special/
  composite 叶现与 flat icon 共用 bundle、live state、registry、time/compass 和 seed
  selector；浮动物品与 stats 行的逐 item 物理像素 scissor 贯通 residency batching
  与 GPU draw，并在下一无裁剪 draw 前恢复完整 attachment。
- **2026-07-26（Equipment asset layer rendering 工程闭环）**：资源包声明的 19 类
  layer、有效 `Equippable` 资产、owned atlas、染色/foil/trim 与玩家翅膀覆盖已贯通
  world/native/renderer；合同见 `docs/archive/equipment-asset-layer-rendering-design.md`。
- **2026-07-26（动态 Item tint 工程闭环）**：八类 `ItemTintSources` 现共享
  Vanilla `PotionContents` 可见效果加权与活体 scoreboard 队伍色；普通 GUI、
  fakeItem、手持/头戴、首人称、嵌套 bundle 及非活体路径均保留精确 owner
  语义，合同见 `docs/archive/item-tint-parity-design.md`。
- **2026-07-26（Item atlas 动画工程闭环）**：资源包 item sprite 动画现按
  Vanilla 帧时长/插值逐 client tick 原地更新 HUD 与模型/实体/粒子共用 atlas；
  generated-item 侧面轮廓合并全部唯一帧，合同见
  `docs/archive/item-atlas-animation-design.md`。
- **2026-07-26（F1 hideGui 呈现工程闭环）**：单一 post-input 可见性快照现覆盖
  普通 HUD、crosshair、方块选择框与完整首人称 pass，同时保留 sleep、Screen、F3
  和 Toast；隐藏态不清空 owner，恢复无需新事件，合同见
  `docs/archive/hide-gui-presentation-design.md`。
- **2026-07-26（相机 FOV 输入工程闭环）**：启动配置、同步玩家 modifier、死亡/
  流体采样与插值现汇入单一 frame owner，分别驱动 world、culling、locator 和
  独立 HUD-3D 投影；合同见 `docs/archive/camera-fov-inputs-design.md`。
- **2026-07-26（旁观者镜头后处理工程闭环）**：SetCamera 选择刷新与 F4 active
  生命周期现原子交接至 renderer；Creeper/Spider/Invert 的 2/10/2 pass 图在
  world/hand 后、HUD 前执行，合同见
  `docs/archive/camera-post-effect-execution-design.md`。
- **2026-07-26（F5 相机视角生命周期工程闭环）**：第一/后置/前置视角循环、
  镜像与八射线 VISUAL 裁剪、相机实体/首人称可见性、拾取姿态和后处理边界均按
  Vanilla 对齐；合同见 `docs/archive/camera-perspective-lifecycle-design.md`。
- **2026-07-26（望远镜遮罩工程闭环）**：本地 `isScoping`、GUI 帧级
  `scopeScale` 插值、selected-pack scope 纹理、四边黑色遮罩与 crosshair 前提交
  已按 Vanilla 对齐；合同见 `docs/archive/spyglass-scope-overlay-design.md`。
- **2026-07-26（玩家列表呈现工程闭环）**：会话加密状态、打开时系统旁白、玩家
  头像/帽层/倒置规则及 6 档 Vanilla 延迟图标已贯通 net、world、native 与
  renderer；合同见 `docs/archive/player-list-presentation-closure-design.md`。
- **2026-07-26（PauseScreen 工程闭环）**：远端菜单与 F3+Escape 显式保持
  client/level tick；动态 dialog 快照、Feedback 子页、可信链接确认及精确父屏恢复
  已贯通 world/native/renderer，合同见 `docs/archive/pause-screen-closure-design.md`。
- **2026-07-26（原生 profiler 与指标工程闭环）**：F3+1 的 Vanilla 嵌套分区、
  F3+L 十秒/手动记录、真实 CPU/内存/区块/GPU 指标、deviation 与异步 ZIP 输出已
  贯通 simulation/native/renderer；合同见
  `docs/archive/native-profiler-metrics-closure-design.md`。
- **2026-07-27（selected-pack 后处理图工程闭环）**：资源包图、GLSL/include/
  texture、七类 uniform 与 wgpu 图执行现原子接管 blur、三类相机效果、描边和
  transparency 深度合成；合同见
  `docs/archive/selected-pack-post-effect-graph-design.md`。
- **2026-07-27（同步环境 ParticleOptions 工程闭环）**：完整 MapCodec payload、
  registry-aware item 组件、ambient 粒子与滴水石事件 1504 已贯通同一原生
  provider/command 路径；合同见 `docs/archive/environment-particle-options-design.md`。
- **2026-07-27（递归不可信解码栈预算工程闭环）**：Component 全解码/投影、
  flattened SNBT 与 direct Dialog HolderSet 的精确深度边界现均由生产尺寸线程上的
  字节预算反向守卫；合同见
  `docs/archive/recursive-untrusted-decode-stack-budget-design.md`。
- **2026-07-27（通用数据包 Component owner 工程闭环）**：直接 Component/Style
  载体现保留精确 NBT，并在任何 world mutation 前用当前 session RegistryAccess
  原子预检；合同见 `docs/archive/generic-packet-component-owner-design.md`。
- **2026-07-28（稳定 HUD 投影缓存工程闭环）**：scoreboard sidebar 与 player
  list 现按 world、资源、异步 profile/skin 和可见性修订键复用静态负载；心形分数
  的 GUI tick 状态独立逐帧同步，不把动画时间并入缓存键。
- **2026-07-28（稳定方块实体与调试投影缓存工程闭环）**：Bed、Sign、Lectern
  与 GameTest 高亮现按精确 world/resource/filter 修订边缘重建；动态方块实体继续
  逐帧提取，四类调试向量在 native/renderer 间循环复用并保持共享排序。
- **2026-07-28（实体专用裁剪与 section 可见性工程闭环）**：Guardian、
  EndCrystal、Shulker 现按 Vanilla 当前 tick 光束/过渡包围盒裁剪；terrain
  resident 精确发布纵向 section，实体门禁使用非插值当前位置。
- **2026-07-28（原生 Tutorial 系统工程闭环）**：六步骤状态机、三条物品包、
  输入/观察/破坏/背包回调与共享可变高度五槽 toast 已贯通 world、native 和
  renderer；合同见 `docs/archive/tutorial-system-closure-design.md`。

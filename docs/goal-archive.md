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

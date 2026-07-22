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

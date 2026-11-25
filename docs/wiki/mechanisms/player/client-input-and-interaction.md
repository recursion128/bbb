# 客户端本地输入、移动与交互

机制 ID：`player-local-input-movement`

## 机制目的和边界

本页说明原始键鼠输入如何转化为 `LocalPlayer` 状态、移动数据包与游戏模式动作，以及它们的重生与失败边界。输入编排、本地移动物理、准星命中与方块破坏预测共享同一条单向链路，单个输入入口或单一测试通过不能代替整条交互链的完整验收。

世界层拥有玩家状态、命中判定与预测语义；客户端运行时编排输入事件与命令提交；网络层仅编码已确定的动作。相机、HUD、粒子与音频只读消费投影。平台按键不直接修改渲染器，也不建立第二份玩家状态。方块破坏进度与预测序列号（prediction sequence）构成相邻交互契约，提交屏障在本页一并说明。

## Vanilla 26.1 权威实现

源码根由 `cargo dev vanilla path 26.1` 解析，以下是关键决策锚点：

- `net/minecraft/client/player/LocalPlayer.java:227-250`：区块加载门槛、原始输入仅变化时上报、非乘客与乘客两条不同的移动上报顺序，以及每 tick 环境处理器。
- `net/minecraft/client/player/LocalPlayer.java:263-312`：先上报疾跑锁存切换，再按位置、旋转和着地/碰撞状态变化选择移动数据包；位置提醒计数器（20 tick）仅在真正发送位置时归零。
- `net/minecraft/client/player/LocalPlayer.java:798-824`：双击前进窗口超时、疾跑启动，以及奔跑/游泳停止分支；相邻 `canStartSprinting` 与 `shouldStop*` 拥有环境约束。
- `net/minecraft/client/multiplayer/MultiPlayerGameMode.java:292-298`：预测作用域先分配序列号、执行客户端本地预测，再发送携带相同序列号的数据包。

相邻 `MultiPlayerGameMode.setLocalMode/adjustPlayer/createPlayer` 负责游戏模式、玩家能力与实体重建；`Minecraft.startAttack/continueAttack/startUseItem` 是主要交互调用点。`ClientPacketListener.handleRespawn` 决定保留数据还是重建全新玩家状态，不得由输入层猜测。

## 决策输入与状态所有权

| 输入或状态 | 唯一所有者 | 消费方式 |
| --- | --- | --- |
| 原始按键、双击边沿/计时器、上次成功发送基线 | `ClientInputState` | 投影原始输入与疾跑启动请求 |
| 疾跑锁存、姿态、生命、饥饿、属性与能力 | `WorldStore` / `LocalPlayerState` | 移动、载具、战斗与相机/FOV 只读查询 |
| 正在使用的物品及主副手 | 世界层物品使用预测状态 | 移动减速、松手释放与元数据校正 |
| 游戏模式与玩家生命周期 | 世界层登录/重生/游戏事件链 | 客户端查询门槛与 HUD 提取 |
| 方块破坏目标、朝向、手持签名、进度、阶段与冷却 | `LocalPlayerInteractionState` | 鼠标输入按世界决策编排破坏命令 |
| 预测序列号、传送门槛与保留方块快照 | 绑定当前世界的层级状态 | 服务端确认与碰撞校正，不随纯玩家替换清空 |
| 准星命中目标 | `crosshair_target_from_world` | 攻击、使用与中键拾取共用同一命中结果 |

原始 `ServerboundPlayerInputPacket` 中的疾跑位只反映物理按键，双击前进不会伪造该位。松开疾跑键也不单独清除疾跑锁存，停止条件由世界层按饥饿度、移动能力、载具、水体、姿态、碰撞与攻击事件统一裁决。`LIVING_ENTITY_FLAGS` 的正在使用/副手标志经实体模型源校正使用状态，渲染与音频层不独立保存使用状态。

## 判断与处理顺序

### 移动 tick 与状态上报

1. 客户端运行时准备本步鼠标增量，同步世界与加载身份，推进输入边沿和双击计时器。
2. 世界层根据原始启动请求与环境条件校正疾跑状态；移动输入依次乘以 `0.98`、使用物品减速乘子、潜行或水外爬行时的潜行速度属性，再作单位正方形投影。
3. 世界层唯一物理所有者按 tick 步进，消费自动跳跃与上一 tick 的卡滞乘子，完成位移、自动跳跃更新与本步方块效果。窒息逃逸、轻微碰撞角度阈值、可攀爬方块潜行抑制均留在世界层，脚手架走其例外分支。
4. 非乘客先发送疾跑锁存切换，再提交位置/旋转数据包。纯旋转包不重置位置提醒计时器；位移阈值为 `2.0E-4`，位置提醒周期为 20 tick。
5. 乘客按「旋转 → 权威载具移动 → 锁存切换」顺序上报；世界层先清空乘客自身速度，再执行骑乘 tick，客户端运行时不内联坐骑或鞍/跳跃判定。

`advance_player_input_for_client_ticks` 消费模拟层分配的 tick 数，与 `SimulationCore` 保持一致（`VANILLA_MAX_CLIENT_TICKS_PER_STEP = 10`，对应 `Minecraft.java:1252` 的单步 10 tick 追赶上限）；服务端降速只改变真实时间间隔，不改变每物理 tick 固定 0.05 秒的步长常量。

粒子与实体共用 `local_player_collision::clip_axis_collision` 的轴向裁剪遍历，以及 `visit_collision_block_states` 的区块/区段调色板扫描（自动跳过空区段）；通过 `Option<LocalPlayerCollisionContext>` 分别提供空形状上下文与当前实体碰撞上下文。

### 命中、动作与命令提交

`crosshair_target_from_world` 先按当前手持物品的攻击距离判定实体命中，方块结果再按方块交互距离截断；未命中时分别以方块/实体交互距离拾取，排除流体、不可拾取实体与 marker 实体。生产路径经 `crosshair_pick_from_camera` 沿相机射线扫描当前 tick 实体包围盒（排除本地玩家与当前相机附着实体），并在 `(world_revision, CameraPose)` 未变时跨帧复用准星拾取结果；视角旋转、拴绳端点与受控相机均从世界层相机姿态查询，禁止再建独立射线检测。

鼠标入口 `handle_mouse_input_at_partial_tick_with_effects_and_presentation` 是主要交互动作的唯一调度点。左键经 `local_player_attack_is_blocked` 校验后执行攻击与挥手，并处理空挥冷却哨兵；按住破坏受正在使用物品与穿刺武器规则抑制。右键 `start_use_item` 保持水桶预测、乐器与可穿戴装备快速切换的优先级。物品使用（`use-item` / `use-on`）使用预留网络命令槽位，停止使用提交 `ReleaseUseItem`，由 `set_local_using_item_with_hand` / `take_local_using_item` 唯一记录并取出当前使用之手。中键分派方块或实体拾取，是否携带数据由修饰键决定。

`select_hotbar_slot` 仅在世界层当前选中快捷栏槽位发生变化时发送手持槽位命令，tick 循环中无第二发送点。实体交互使用攻击/交互/旁观队列；容器交互使用点击/按钮/配方/槽位状态队列，其中槽位状态与点击原子合并发送；创造模式操作由 `submit_action_inner` 构造提交计划。

方块破坏遵循「权限限制 → 初始世界边界 → 本地预测/攻击 → 网络命令」顺序。生存模式下持续破坏同一目标不重复检查世界边界，目标变为空气时清零进度并停止挥手、击打粒子与音效；创造模式每次持续破坏仍检查边界。本地写入空气前统一检查 `Item.canDestroyBlock`：工具的创造模式可破坏标志、`DebugStickItem` 显式拒绝，以及生成的 `GameMasterBlock` 成员表；拒绝本地销毁不会取消 Vanilla 要求的 `START`/`STOP` 协议命令。

有界网络命令队列构成预测变更的提交屏障：开始、完成或中止破坏前先申请通道 permit，切换目标时一次申请 `ABORT` + `START` 两个 permit；成功后再分配带符号 32 位回绕（wrapping）序列号、执行本地预测，并用预留 permit 发送命令。方块破坏、方块右键使用与物品使用共用同一原子屏障。

## 生命周期和 fallback

当网络队列已满、已关闭或缺少发送端时，不推进疾跑出站基线，留待下一 tick 重试。预测命令未拿到 permit 时，序列号、破坏目标、教程进度、物品/方块本地变更与视听副作用均保持旧值；按键释放或窗口失焦仍走同一原子中止路径。

同一位置重复预测会递增序列号，但保留首次记录的服务端方块状态与玩家位置；`BlockChangedAck` 按有符号 `<=` 闭区间清理保留记录，仅当 `lastTeleportSequence < ack sequence` 时允许碰撞校正回移玩家。同维度重生保留世界预测处理器；跨维度重生、切换世界或替换会话时将其清空。

登录经 `reset_local_player_for_login` 初始化游戏模式与能力；重生经 `reset_local_player_for_respawn` 按 `keep-data` 掩码保留疾跑/姿态/属性修饰符或重建全新状态。`game_type_player_abilities` 是登录、重生与模式切换共用的能力映射；切换为旁观模式时清空垂直速度。每次重生都会递增玩家加载身份，即便世界代际未变，也会丢弃旧双击边沿/计时器并重新对齐疾跑基线。死亡界面在第 20 tick 解锁按钮后，由唯一命令链单次发送重生请求。

### 玩家状态与交互表面

生命值与经验值由 `apply_player_health` / `apply_player_experience` 数据包权威链更新；`apply_local_living_heal` 仅负责与生物交互时的本地治疗预测。HUD 上下文状态栏在定位栏、跳跃蓄力栏与经验条间切换，生命/饥饿/经验的可见性只消费游戏模式门槛。权限实体事件 24–28 经 `apply_local_player_permission_entity_event` 映射为管理员权限等级，不构造实体投影；系统聊天与动作栏覆盖消息共用系统消息应用路径，可见性查询 `can_receive_system_messages`，文本过滤选项来自启动配置。

`Q`/`Ctrl+Q` 经 `drop_local_selected_hotbar_item` 执行丢弃预测，仅当手持非空时挥手；旁观模式在按键层直接拦截。`Esc`/`E` 与传送门触发共用 `close_local_container` 和关闭命令；服务端控制背包的坐骑按 `E` 发送 `OpenInventory`，以 `local_player_server_controlled_inventory_vehicle_id` 为唯一共享判定。`F3+F4` 检查 `debug_game_mode_switcher_can_open`，空挥冷却走 `advance_attack_miss_time`，旁观检查走 `local_player_is_spectator`，飞行等能力切换经 `queue_player_abilities_command` 去重。创造模式丢弃防刷采用每次 `+20`、上限 `1280`、每 tick `-1` 的计数器，生存模式不限流。配方高亮由配方书守卫移除；Vanilla 教程背包钩子为空操作，直接挂载在既有点击链上。

告示牌编辑先经 `apply_open_sign_editor` 的缺失告示牌守卫，再由 `sync_sign_editor` 检查交互距离与单次提交；书本编辑由 `sync_book_screen` 拥有使用入口、文本规则与未签名提交。命令方块与命令方块矿车编辑器等待服务端权威数据后按精确状态单次提交；结构、拼图、测试与测试实例方块编辑器通过 `open_{structure,jigsaw,test,test_instance}_block_editor` 压入 `Screens` 栈，统一负责预加载、值钳制/精确设置、模式切换与响应尺寸。对话框经 `apply_show_dialog` 产生 `ScreenRequest`，由 `show_dialog` 与 `close_dialog` 在 `Screens` 栈上挂起与恢复父屏幕。任何交互表面均不得绕过 `Screens` 栈直接改写当前屏幕状态。

### 环境、骑乘与呈现

疾跑粒子的锁存、水/岩浆、旁观、潜行、死亡、飞行门槛及脚下方块不可见剔除由 `local_player_sprint_particle_state` 统一裁决；客户端运行时仅按速度 `(-4dx, 1.5, -4dz)`、实体宽度随机偏移与坐标钳制发射方块碎屑粒子。暴击与附魔暴击（`crit`/`magicCrit`）仅消费服务端 `Animate` 4/5 广播的追踪发射器，发起者本人也走同一路径，因此本地视觉存在 1 RTT 延迟。

传送门扭曲强度唯一由 `advance_local_player_portal_transition_effect` 推进：内部判定收缩碰撞箱，每 tick 按 `+0.0125`/`-0.05` 步进并钳制，由 0 转为上升时置位音效触发标记，位于门内时置位容器关闭标记。客户端消费这些待处理标记，相机畸变采用 `portal * 20 + nausea * 7`；洞穴氛围值、水下视野与入水/出水音效分别交给环境音频、光照图与水下环境音各自既有的 tick 所有者。

骑乘关系只接受 `apply_set_passengers` 与 `clear_local_player_mount`；坐骑跳跃蓄力释放经 `local_player_rideable_jumping_vehicle_id` 与 `maybe_queue_riding_jump_command` 发出，骑乘疾跑经 `local_player_sprintable_vehicle_id` 判定。实验性矿车旋转行为的可达性由特性标志（feature flag）查询控制。

## BBB 实现映射

| 语义所有者 | 仓库内路径 | 输出与消费者 |
| --- | --- | --- |
| 输入编排与上报基线 | `bbb_client::input::movement` | 世界层移动输入与网络命令 |
| 移动物理与输入归一 | `bbb_client_world::client::local_player_movement` | 权威玩家姿态与移动副作用 |
| 主要鼠标交互动作 | `bbb_client::input::mouse` | 预留网络命令与世界本地预测 |
| 准星命中查询 | `bbb_client::crosshair` | 攻击、使用与中键拾取统一目标 |
| 登录、重生与加载身份 | `bbb_client_world::level` | 玩家状态与世界代际标识 |
| 玩家状态与破坏决策 | `bbb_client_world::client::local_player`、`client::local_player_destroy` | 权限查询与破坏预测，不接触网络套接字 |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 疾跑上报与移动顺序 | `LocalPlayer.tick/sendPosition` | `bbb_client::input::movement` | `aligned` | 下述疾跑顺序与队列回退聚焦测试 |
| 输入缩放与移动物理 | `LocalPlayer.aiStep` | `bbb_client_world::client::local_player_movement` | `aligned` | 本地移动物理聚焦测试集 |
| 玩家重建与加载重启 | `ClientPacketListener.handleRespawn` | `level`、`input::movement` | `aligned` | 同维度重生身份隔离测试 |
| 预测作用域与通道屏障 | `MultiPlayerGameMode.startPrediction` | 世界预测处理器 + 鼠标 permit 预留 | `aligned` | 方块破坏与交互预测聚焦测试 |

## 结构性不变量

- 原始物理按键、权威疾跑锁存与上次成功发送基线三者严格分离，下游消费者只读各自对应的所有者。
- 游戏模式、权限、骑乘、物品使用与准星命中均只有唯一所有者，不得在运行时、渲染或音频层各自保留副本。
- 本地移动只有一条物理步进路径；出站预测动作只有「先预留 permit 再变更并发送」一条提交路径。
- 复合 tick 的外层顺序与内部状态分别通过对应行为测试验证，不以源码文本守卫替代运行时行为检查。

## 聚焦测试（focused tests）和真实证据

`bbb_client::input::movement` 中可定位的语义测试：

- `run_sprint_major_collision_stops_once_before_catch_up_movement_command`：验证碰撞停跑先于追赶移动命令发送。
- `run_sprint_minor_collision_does_not_stop`：验证轻微擦碰不误停疾跑。
- `sprint_command_queue_failure_keeps_baseline_and_retries_next_tick`：验证队列满时保留发送基线并于下一 tick 重试。
- `same_dimension_respawn_resets_player_sprint_baseline_without_replacing_level`：验证同维度重生只重置玩家基线而不替换世界代际。

常用验证命令：`cargo dev test -p bbb-client sprint`；世界层移动物理使用 `cargo dev test -p bbb-client-world local_player_movement`。上述聚焦测试覆盖局部状态与顺序契约，完整可游玩表现另由端到端回放验收。

## 当前差异

创造模式普通容器的 `THROW` 点击未经过丢弃防刷限流器；实验性 `minecart_improvements` 乘客 `rotLerp` 未实现，默认关闭该实验标志时不可达；`crit`/`magicCrit` 粒子消费服务端广播，存在 1 RTT 视觉延迟。上述三项均不属于阻塞性缺口，[客户端账本](../../../unsupported-client-features.md)当前无未关闭的 `# Todo` 项。

## 账本归属

本页对应机制 `player-local-input-movement`（覆盖本地输入、移动、准星命中与方块/实体交互）。产品范围裁决与未关闭差异统一查[客户端账本](../../../unsupported-client-features.md)，推进原则见 [AGENTS.md](../../../../AGENTS.md)。

## 精确重开条件

当 Vanilla 调整疾跑奔跑/游泳条件、移动数据包顺序、输入缩放因子或重生 `keep-data` 规则时须重新核验；若新增游戏模式或 `LocalPlayer` 入口导致背包操作未经过统一路由、队列失败污染了发送基线或预测状态，或同维度重生未重启本地加载消费者，则按对应所有者定位并修复。

## 相关页面和源码索引

- [客户端世界生命周期](../world/client-world-lifecycle.md)：世界代际与会话级状态。
- [客户端协议状态流](../protocol/client-state-flow.md)：数据包、副作用与快照发布顺序。
- [客户端架构边界](../../architecture/client-boundaries.md)：crate 所有权与屏幕/运行时边界。

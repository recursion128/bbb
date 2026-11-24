# 客户端本地输入、移动与交互

机制 ID：`player-local-input-movement`

## 机制目的和边界

本页解释原始输入如何成为 LocalPlayer 状态、移动报文和 game-mode 动作，以及它们的
重生与失败边界。相关 `player-local-*`、`interaction-*` route 各自保留独立 coverage，
这里的集中说明不把一个输入入口或一个通过的测试提升为整类等价证明。

world 拥有玩家、命中与预测语义；native 编排输入和命令；net 编码已确定的动作。
相机、HUD、粒子和音频读取投影。平台按键不直接改 renderer，也不建立另一份玩家状态。
方块破坏的进度与 prediction sequence 是相邻交互合同，见本页的提交边界。

## Vanilla 26.1 权威实现

源码根由 `scripts/vanilla path 26.1` 解析，下面是该根下的决策锚点：

- `net/minecraft/client/player/LocalPlayer.java:227-250`：loaded 门、raw input change-only
  报告，非乘客与乘客的不同移动顺序，以及每 tick ambient handler。
- `net/minecraft/client/player/LocalPlayer.java:263-312`：先报告 sprint latch，再按位置、
  旋转和地面/碰撞变化选报文；position reminder 只在发送位置时归零。
- `net/minecraft/client/player/LocalPlayer.java:798-824`：双击窗口失效、sprint 启动与
  run/swim 停止分支；相邻 `canStartSprinting` 与 `shouldStop*` 拥有环境门。
- `net/minecraft/client/multiplayer/MultiPlayerGameMode.java:292-298`：prediction scope
  先取得 sequence、执行 predictive action，再发送匹配 packet。

相邻 `MultiPlayerGameMode.setLocalMode/adjustPlayer/createPlayer` 负责 mode、abilities 与
玩家构造；`Minecraft.startAttack/continueAttack/startUseItem` 是 primary-action 调用者。
`ClientPacketListener.handleRespawn` 决定 keep-data 与 fresh player，不能由输入层猜测。

## 决策输入与状态所有权

| 输入或状态 | 唯一 owner | 消费方式 |
| --- | --- | --- |
| 原始按键、双击 edge/timer、最后成功发送 baseline | `ClientInputState` | 投影 raw input 与启动请求 |
| sprint latch、pose、health、food、attributes、abilities | `WorldStore` / `LocalPlayerState` | movement、vehicle、combat、camera/FOV 只读 |
| 使用中物品及 hand | world use-prediction state | movement slow、release 与 metadata reconcile |
| game type 与玩家生命周期 | world login/respawn/game-event 链 | native 查询门、HUD 提取 |
| block destroy target/face/item signature/progress/stage/delay | `LocalPlayerInteractionState` | mouse 按世界决策编排命令 |
| prediction sequence、teleport gate、retained blocks | level-bound world state | ack 与碰撞校正，不随单纯玩家替换清空 |
| crosshair | `crosshair_target_from_world` | attack/use/pick 共用命中结果 |

raw `ServerboundPlayerInputPacket` 的 sprint bit 只表示按键；双击前进不伪造它。
松开 sprint 键不单独清掉 latch；world 按 food、mobility、vehicle、water、pose、collision
与 attack 决定停止。`LIVING_ENTITY_FLAGS` 的 is-using/off-hand 经 entity model source
校正 use 状态，render/audio 不独立保存使用状态。

## 判断与处理顺序

### 移动 tick 与报告

1. native 准备本步鼠标 delta，同步 level/load identity，推进输入 edge/timer。
2. world 依据原始启动请求和环境 reconcile sprint；输入归一按 0.98、item-use 乘子、
   crouching/水外 crawling 的 sneaking-speed attribute，再作 unit-square 投影。
3. world 唯一 physics owner 按 tick 步进，消费 auto-jump 与前 tick 的 stuck multiplier，
   完成移动、auto-jump 更新和本步 block effects。suffocation 逃逸、minor collision
   角度门、climbable sneak 抑制都留在 world；scaffolding 使用其例外分支。
4. 非乘客先发送 latch transition，再提交位置/旋转报文。rotation-only 不重置 position
   reminder；位移阈值为 `2.0E-4`，位置 reminder 为 20 tick。
5. 乘客按 rotation → authoritative vehicle movement → latch transition 排序；world
   先清乘客 motion，再执行 ride tick，native 不内联 mount 或 saddle/canJump 判定。

`advance_player_input_for_client_ticks` 消费 simulation 的 tick 数，每步最多 catch up 五 tick；
慢速 server tick 改变墙钟间隔，不改变每 physics tick 的 0.05 秒常量。

粒子与实体共用 `local_player_collision::clip_axis_collision` 的轴遍历和 `visit_collision_block_states` 的区块/区段调色板扫描（跳过空区段）；
通过 `Option<LocalPlayerCollisionContext>` 分别读取空形状上下文与当前实体碰撞上下文。

### 命中、动作与命令提交

`crosshair_target_from_world` 先用 active item attack range 命中，block 结果再按 block
interaction range clamp；miss 时分别以 block/entity range pick，排除 fluid、不可 pick 与 marker。
生产路径经 `crosshair_pick_from_camera` 沿相机射线扫描当前 tick 实体 AABB（排除本地玩家与当前相机实体），并在 `(world_revision, CameraPose)` 未变时跨帧复用准星拾取结果；view rotation、rope hold 端点和 controlled-camera 都从 world camera/pose 查询，不能再建 raycast。

鼠标的 `handle_mouse_input_at_partial_tick_with_effects_and_presentation` 是 primary-action
唯一入口。左键经 `local_player_attack_is_blocked` 后 attack+swing，并处理 miss-time sentinel；
按住破坏受 using-item/piercing 抑制。右键 `start_use_item` 保持 bucket prediction、instrument、
equippable swap 优先级。use-item/use-on 使用预留命令，停止使用提交 ReleaseUseItem，
`set_local_using_item_with_hand` / `take_local_using_item` 唯一记录与 drain hand。
中键分派 block/entity pick，include-data 来自修饰键。

`select_hotbar_slot` 只在 world selected slot 改变时发 held-slot 命令；tick 没有第二发送点。
entity action 使用 attack/interact/spectate 队列；container action 使用 click/button/recipe/
slot-state 队列，slot-state 与 click 原子合发；creative action 由 `submit_action_inner` 形成计划。

block breaking 保持 restriction → 初始 border → prediction/attack → command 顺序。
生存模式同 target 不重复检查 border；目标变 air 时清 progress 且停止 swing/hit particle/sound。
creative 每次 continue 仍检查 border。本地写 air 前统一检查 `Item.canDestroyBlock`：
Tool 的 creative destroy 门、`DebugStickItem` 拒绝以及 generated `GameMasterBlock` membership。
拒绝本地销毁不取消 Vanilla 要求的 START/STOP 命令。

有界 net queue 是预测 mutation 的 commit barrier：start/finish/abort 先取得 permit，retarget
一次取得 ABORT+START 两个 permit；之后分配 signed Java int wrapping sequence、执行预测，
用相同 permit 发送。block breaking、use-on 与 item-use 共用该规则。

## 生命周期和 fallback

queue full、closed 或缺 sender 时，不推进 sprint outbound baseline；下一 tick 重试。
预测命令拿不到 permit 时，sequence、target、tutorial、item/block mutation 与 effect 都保持旧值；
release/focus-loss 仍走同一 atomic abort owner。

同位置重复预测更新 sequence，但保留首次 server state/player position；`BlockChangedAck`
使用 signed `<=` inclusive 清理，只有 `lastTeleportSequence < ack sequence` 才允许碰撞校正移动。
同维度 respawn 保留 level prediction handler；dimension、level 或 session replacement 清空它。

login 用 `reset_local_player_for_login` 建立 mode/abilities；respawn 用
`reset_local_player_for_respawn` 按 keep-data 保留 sprint/pose/attribute modifiers，或重建 fresh 状态。
`game_type_player_abilities` 是 login/respawn/mode change 共用映射；切 spectator 清垂直速度。
每次 respawn 都更新 load identity，即使 level generation 未变，也丢弃旧双击 edge/timer 并
对齐 sprint baseline。death screen 第 20 tick 解锁后，唯一命令链单发 respawn。

### 玩家状态与交互表面

health/experience 由 `apply_player_health` / `apply_player_experience` 包权威链更新；
`apply_local_living_heal` 只拥有 mob-interact 本地预测。HUD contextual bar 在 locator/jump/
experience 之间选择，health/food/experience visibility 只消费 game-mode 门。
permission entity event 24–28 经 `apply_local_player_permission_entity_event` 映射 op level，
不制造 entity projection；system/overlay chat 共用 system-chat apply，visibility 使用
`can_receive_system_messages`，text-filtering 来自 startup 查询。

Q/Ctrl-Q 使用 `drop_local_selected_hotbar_item` 预测，非空才 swing；spectator 在 key 层剔除。
Esc/E 与 portal 共用 `close_local_container` 和 close command；server-controlled mount 的 E
发送 OpenInventory。`local_player_server_controlled_inventory_vehicle_id` 是共享 mount 门；
F3+F4 用 `debug_game_mode_switcher_can_open`，miss time 用 `advance_attack_miss_time`，
spectator 用 `local_player_is_spectator`；abilities toggle 经 `queue_player_abilities_command` 去重。
creative drop spam 使用 +20/1280、每 tick -1，survival 门恒真。
recipe highlight 由 recipe-book 守卫删除；Vanilla tutorial inventory hook 为空，只挂既有 click 链。

sign 打开先经 `apply_open_sign_editor` 的 missing-sign guard，再由 `sync_sign_editor` 检查
range 与单次提交；book 的 `sync_book_screen` 拥有 use、text rules 与 unsigned submission。
command block/minecart editor 等待 authoritative data 后 exact-state 单发；structure/jigsaw/
test/test-instance 方块编辑器通过 `open_{structure,jigsaw,test,test_instance}_block_editor` 压入 `Screens` 栈，拥有 preload、clamp/exact-set、mode cycle 与 response size。
dialog 经 `apply_show_dialog` 产生 `ScreenRequest`，由 `show_dialog` 与 `close_dialog` 在 `Screens` 栈上挂起和恢复父屏幕。
各表面不能绕过 `Screens` 栈直接置 screen 状态。

### 环境、骑乘与呈现

sprint 粒子的 latch、水/岩浆、spectator、蹲、死亡、飞行门和 on-pos/INVISIBLE 剔除由
`local_player_sprint_particle_state` 裁决；native 仅生成速度 `(-4dx,1.5,-4dz)`、宽度随机和
列 clamp 后的 BLOCK 粒子。crit/magicCrit 只消费 server Animate 4/5 tracking emitter 广播，
发起者也经此路径，因此本地效果有 1 RTT 延迟。

portal 强度只由 `advance_local_player_portal_transition_effect` 推进：inside 探测收缩 bbox，
每 tick +0.0125/-0.05 并 clamp，0→rising 置 trigger pending，inside 置 container-close pending。
native 消费 pending，camera 使用 `portal*20+nausea*7`；mood、water vision 与 underwater
enter/exit 各交给 environment audio、lightmap 和 ambient audio 的既有 tick owner。

mount 关系只接受 `apply_set_passengers` / `clear_local_player_mount`；jump charge 释放经
`local_player_rideable_jumping_vehicle_id` 与 `maybe_queue_riding_jump_command`，mounted sprint
经 `local_player_sprintable_vehicle_id`。实验 minecart rotation 的可达性由 feature flag 查询。

## BBB 实现映射

| 语义 owner | repo-local 路径 | 输出与消费者 |
| --- | --- | --- |
| 输入编排、报告 baseline | `bbb_client::input::movement` | world input / net command |
| movement 与输入归一 | `bbb_client_world::client::local_player_movement` | canonical pose、effects |
| primary action | `bbb_client::input::mouse` | 预留命令与 world mutation |
| 命中查询 | `bbb_client::crosshair` | attack/use/pick 统一结果 |
| login/respawn/load | `bbb_client_world::level` | player 与 level identity |
| 玩家状态、销毁决策 | `bbb_client_world::client::local_player`、`client::local_player_destroy` | 查询与预测，不拥有 socket |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| sprint 报告与移动顺序 | `LocalPlayer.tick/sendPosition` | `input::movement` | `aligned` | 下列 sprint tests |
| 输入缩放与 physics | `LocalPlayer.aiStep` | `client::local_player_movement` | `aligned` | movement focused suites |
| 玩家重建与 load 重启 | `ClientPacketListener.handleRespawn` | `level`、`input::movement` | `aligned` | 同维度 respawn test |
| prediction scope | `MultiPlayerGameMode.startPrediction` | level prediction + mouse permit | `aligned` | interaction route evidence |

## 结构性不变量

- raw key、canonical sprint latch、成功发送 baseline 三者独立，所有消费者只读其对应 owner。
- mode、permission、mount、use 与 crosshair 不允许 native/render/audio 各存副本。
- movement 只有一个 physics 路径；出站 prediction 只有预留 permit 的提交路径。
- 复合 tick 的外层顺序与内部状态分别通过对应行为测试验证，不靠源码文本守卫替代运行时行为检查。

## 聚焦测试（focused tests）和真实证据

`bbb_client::input::movement` 中可定位：

- `run_sprint_major_collision_stops_once_before_catch_up_movement_command`：停止顺序。
- `run_sprint_minor_collision_does_not_stop`：minor collision 不误停。
- `sprint_command_queue_failure_keeps_baseline_and_retries_next_tick`：满队列保留 baseline 并重试。
- `same_dimension_respawn_resets_player_sprint_baseline_without_replacing_level`：两种 identity。

适用命令为 `cargo dev test -p bbb-client sprint`；movement world 行为使用
`cargo dev test -p bbb-client-world local_player_movement`。
这些 focused 结果只覆盖对应状态与顺序；playable/live 行为需要独立 receipt。

## 当前差异

creative 普通容器 THROW 未查 drop throttler；实验 `minecart_improvements` 乘客 rotLerp
未实现，默认关闭 flag 时不可达；crit/magicCrit 使用服务端广播，有 1 RTT 视觉延迟。
未关闭状态与产品范围只由客户端 ledger 维护，不在此新增 Todo。

## 账本归属

主机制为 `player-local-input-movement`；sprint/reporting/vehicle/use/inventory/camera 与
`interaction-block-breaking`、prediction、carried-slot、primary action 等保持独立条目。
状态查询[客户端 ledger](../../../unsupported-client-features.md)，推进原则见[AGENTS.md](../../../../AGENTS.md)。

## 精确重开条件

Vanilla 调整 sprint run/swim 门、报文顺序、输入缩放或 respawn keep-data 时重新取证；
新增 game-mode/LocalPlayer 方法导致 inventory 未路由、queue failure 改动了 baseline 或
预测状态、同维度 respawn 未重启本地 load consumer 时，按失败 owner 登记回退。

## 相关页面和源码索引

- [客户端世界生命周期](../world/client-world-lifecycle.md)：level identity 与 session state。
- [状态流](../protocol/client-state-flow.md)：packet、effect、publication 的顺序。
- [客户端边界](../../architecture/client-boundaries.md)：crate ownership、screen/runtime 边界。

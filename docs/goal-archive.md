# Goal 完成里程碑索引

本文件只把工程 milestone 路由到 Git history，不保存合同、取证正文或完成清单。当前机制、
排程和缺口以 [`docs/wiki/`](wiki/README.md)、[`goal.md`](../goal.md) 和 [`docs/unsupported-features.md`](unsupported-features.md)为准。

## 追溯方法

- 按时间：`git log --since=2026-07-22 --until=2026-07-23`。
- 按主题：`git log --grep='registry\|render\|font'`。
- 查已删除历史文档：`git log --all -- <path>` 后执行
  `git show <commit>:<path>`。
- 查实现演进：`git log -p -- <code-path>`。Git history 是完整权威，不重写。

## 里程碑

- **2026-07**：原生架构、P0/coverage、生成式 entity/render policy、session/packet、resource-pack、
  registry revision、simulation publication 与 render-thread 生命周期建立；entity client tick/movement/
  collision/pick/equipment/passenger/leash 与 157 类 entity、194 行 registry holder/tag 与
  light/chunk/block/event/UI packet families、world effect/screen/HUD 与 item-component 110 项六维
  coverage、block/item use prediction 与 container/recipe/creative UI、special item/equipment rendering、
  camera/HUD/pause/player-list、profiler、post-processing、untrusted decode budgets、stable renderer
  extraction、tutorial/debug、resource/platform workflow、packet 256 行与粒子矩阵、block animateTick、
  survival physics、rich text、render state/layer、双向 shaping、可见性/occlusion、entity/block-entity
  presentation、texture sampling、generated registry fallback、lifecycle/fault soak、Linux Wayland/Vulkan
  release gate、Vanilla `hotbar.nbt` 互换、Trial Spawner display projection 与共享 camera Water/Lava
  sampler/Lava fog 逐项闭环。
- **2026-08-02**：官方 26.1 渲染机制/可见 workload/同载性能 P0、unihex/CJK 字体、中文 Vanilla 机制 Wiki
  与 parity/document lint 闭环；文档工作流 v10（当前事实入 Wiki、历史 Git-only、`docs/` allowlist）；
  indexed default pack（asset index 30 统一 lazy `vanilla` provider，zh_cn/OGG/CJK 验收）；external-client
  census（2,927 owner 的唯一 JSON inventory + parity matrix + 双向 route/stale-output 门禁）；core
  pipeline / shader generation / reload-listener 三处收敛为单一 renderer-private generation owner；
  LocalPlayer sprint 的 raw key、canonical latch 与 outbound baseline 分离。
- **2026-08-03**：单日工程 milestone 合并索引（细节 `git log --since=2026-08-03 --until=2026-08-04`）：
  fatal runtime crash lifecycle、架构收敛批（HUD item projection/selected shader/post-effect identity/
  control mailbox/equipment catalog/textured shader/simulation 硬预算/equipment-falling-dust-piston 八项
  单一 owner 化）、external network terminal close、lifecycle terminal budget、Vanilla root window icons、
  authenticated profile-key lifecycle、trusted profile texture-set、architecture convergence census、
  MenuScreens registration、remote disconnect screen lifecycle、indexed disconnect panorama、phase-aware
  disconnect audio teardown、ChatScreen（25 owner）与 DeathScreen（16 owner）decision surfaces、external
  session lifecycle（187 `Minecraft` owner）、LevelRenderer 七机制（77 owner）。
- **2026-08-03/04（LocalPlayer census 首批 closure）**：149 owner 拆 23 route；block-breaking/prediction 统一 level-bound
  handler+barrier；input-movement 20 owner 经唯一 tick/归一/sprint 链收口；environment-effects 8 owner（crit 经 AndSelf 验证、portal intensity/trigger/spinning 补实现）、world-edit-ui 9 owner（九面唯一 sync_*_editor/screen 链）、camera-targeting 9 owner（唯一 crosshair resolver/camera pose/leash/SetCamera 链）、inventory-actions 9 owner（drop 预测/close 命令/creative throttler/recipe/tutorial-noop 链）、vehicle-control 7 owner（ride tick/riding jump/sprintable/SetPassengers/minecart-flag 链）、item-use-state 6 owner（use-prediction/movement 乘子/flags 投影链）、permission-chat-state 6 owner（permission event/system-chat/visibility/text-filtering 链）、game-mode-lifecycle 7 owner（game-event/login-respawn 重建/abilities/switcher 链）、primary-action-routing 3 owner（mouse 左/右键唯一路由链）、game-mode-guards 5 owner（HUD 门/miss-time/spectator/mount 判定链）、session-respawn 4 owner（login-respawn 重建/respawn 命令/death 计数链）、vitals-status 4 owner（health/experience 应用与 contextual bar 链）、movement-reporting 4 owner（per-tick 报文/sprint 转换/abilities 命令链）、status-options/item-use/entity-actions/container-actions/creative-actions 批量 18 owner（game-event/命令族链，补 spectator 垂直清零）、carried-slot-sync/pick-actions 5 owner 收口——**LocalPlayer/game-mode census 22 route 全 equivalent，域关闭**。
- **2026-08-03/04（generated evidence/layout、ClientLevel census 与 lifecycle closures）**：五份 JSON 收敛；
  121 owner 拆 18 route；全部 18 route（entity/identity/chunk-light/dirty-handoff/clock-weather/ambient-animation/respawn-data/session-services/biome-tint/sound-routing/particle-routing/diagnostics-report/map-data/level-events/block-entity-tracking 等）95 owner 经唯一 store/resolver/gate/装配/分派链收口，census 域关闭。

- **2026-08-04（block-entity registration closure）**：`BlockEntityRenderers` 27 条注册收敛为 23 个投影 owner
  并逐条挂上 implementation/test evidence；产码补齐 item display 四项、beacon 光柱（与 end gateway 共用
  `beaconBeam` 几何/管线）、structure_block/test_instance_block 包围盒与错误标记、copper_golem_statue 雕像，
  piston 移动方块回到 `block_entity_visibility` 唯一政策点，route 翻 `equivalent`。

- **2026-08-04（client persistence closure）**：`Options` 145 + `ServerList` 13 共 158 owner 按 canonical
  owner 拆完并逐条挂 evidence。新增 `bbb-native::client_options`（key/编码/默认值由 `Options.java` 生成），
  CLI 参数降级为文件 override，F3+H/F3+P 成为 vanilla 同源的 `Options.save` 触发点并补发
  `ClientInformation`；`server_list.rs` 改按地址复用条目、新建走 quick-play 的 hidden + 16 上限。
  `persistence-options-file`/`-derived`/`-apply`/`persistence-server-list` 四行翻 `equivalent`，无消费面的
  8 个 owner 归 `persistence-options-unconsumed`（`missing`），只服务 options 屏的归 `scope-options-ui`，
  多人服务器选择屏的 5 个归 `scope-multiplayer-server-list`。

- **2026-08-05（HUD/input-window/game-renderer census closures）**：三个 census 工程同日收口，
  全部逐 owner 挂 implementation/test evidence：`Gui` 72 owner 五条 `hud-*` route（camera player 唯一
  裁决点、nausea overlay、`LevelLoadingScreen`、subtitle 位置按 in-game UI 裁决）；
  `native-input-window-surface` 173 owner 十行 route（debug-crash abort、`saySystemQueued` 门控、
  respawn/screen toggle 重置、keybind 组件解析、narrator toast/音量链，GLFW 专有面逐 owner 记 scope）；
  `Minecraft.renderFrame`+`GameRenderer` 53 owner 四条 route（totem 40-tick 状态机、glint/panorama
  接线、framerate 节流表、renderClouds、ao 开关、redraw-arm/clearLevel 守卫；auto-screenshot 记
  `not-needed`）。十九行 matrix 全翻 `equivalent`。

- **2026-08-05（secure-chat/social/reporting closure）**：`security-secure-chat-reporting` 30 owner
  （`Minecraft` chat abilities 9 + `PlayerSocialManager` 12 + `ReportingContext` 9）逐条挂
  implementation/test evidence，matrix 行翻 `equivalent`；离线服务边界=vanilla `OFFLINE_PROPERTIES`
  语义（block list/abuse sender/签名会话仅凭 startup 凭据激活）；census 修一处分歧：matched-name
  system-chat 抑制改走 discovered-names map（vanilla `guessChatUUID`）。authenticated Services 端到端
  仍按账本 deferred。账本 active 项清零，P3/P4 节转维护态。

- **2026-08-05（user-reopened deferred quartet）**：用户重开的四个 deferred 项同日全部落地关闭：
  vanilla 地形 texture filtering 面（`TextureFilteringMethod` 三模式 wrap 解析 + `maxAnisotropyBit`
  采样器 `anisotropy_clamp`/nearest 着色器路由/atlas padding `1<<clamp(bit-1,0,4)`）；toast 与 F3
  overlay 提取到全屏 screen（level-loading/disconnected/win）之上；`SmoothDouble` 平滑镜头转向
  （每帧 mousea 采样、grab 冻结、scoping 优先级、未绑定 `key.smoothCamera` 切换）；IME preedit
  焦点移交（`lastPreeditEvent` 常存 + gain/lose 双向 handoff，`clear_pressed` 不再越权清
  composition，失效由 dialog revision/sign signature 护栏承担）。四 route 回到 `equivalent`。

- **2026-08-05（online-mode removal）**：用户裁决 online 模式为非目标，整体移除已实现但永不可
  验收的在线面：RSA/AES-CFB8 加密登录与 sessionserver join、profile-key 拉取与
  `ChatSessionUpdate`/本地签名链（`chat_session/runtime.rs` 整文件）、bearer-token Services
  block-list/abuse-report REST client（`user_api.rs`）、`--profile-id`/`--access-token` 凭据参数、
  `PlaySessionSecurity` 事件与 tab-list heads 加密位。服务端发起加密握手改为可归因失败；出站
  聊天/命令保持 vanilla「无 key pair」语义；入站签名验证与 last-seen ack 不变。packet coverage
  五条目改判 not-applicable、`identity-profile-key-lifecycle` 行转 `not-needed`、账本 deferred #1
  改 `not-needed`（deferred 6→5）。

原子 slice 的精确边界、测试与 commit hash 只在 Git history 中检索；后续只为新的工程 milestone 追加一行，旧月份过长时继续合并，不恢复历史散文。

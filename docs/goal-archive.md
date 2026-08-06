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
- **2026-08-03/04（LocalPlayer census 首批 closure）**：149 owner 拆 23 route，block-breaking/prediction 统一 level-bound handler+barrier，input-movement 经唯一 tick/归一/sprint 链收口；environment-effects、world-edit-ui、camera-targeting、inventory-actions、vehicle-control、item-use-state、permission-chat-state、game-mode-lifecycle/guards、primary-action-routing、session-respawn、vitals-status、movement-reporting、status-options/item-use/entity/container/creative-actions、carried-slot-sync/pick-actions 各按其唯一 owner 链逐条挂 evidence——LocalPlayer/game-mode census 22 route 全 equivalent，域关闭。
- **2026-08-03/04（generated evidence/layout、ClientLevel census 与 lifecycle closures）**：五份 JSON 收敛；
  121 owner 拆 18 route；全部 18 route（entity/identity/chunk-light/dirty-handoff/clock-weather/ambient-animation/respawn-data/session-services/biome-tint/sound-routing/particle-routing/diagnostics-report/map-data/level-events/block-entity-tracking 等）95 owner 经唯一 store/resolver/gate/装配/分派链收口，census 域关闭。

- **2026-08-04（block-entity registration + client persistence closures）**：`BlockEntityRenderers`
  27 条注册收敛为 23 个投影 owner 逐条挂 evidence（补 item display 四项、beacon/end-gateway 共用光柱、
  structure/test 包围盒、copper_golem_statue；piston 回 `block_entity_visibility` 唯一政策点）；
  `Options` 145 + `ServerList` 13 共 158 owner 拆完挂 evidence：新增生成式 `client_options`、CLI 降级为
  文件 override、F3+H/F3+P 触发 `Options.save` 并补发 `ClientInformation`、server_list 按地址复用 +
  quick-play hidden/16 上限；五行翻 `equivalent`，无消费面归 `missing`/scope 行。

- **2026-08-05（census closures + 用户裁决三批）**：`Gui` 72 owner 五条 `hud-*`、
  `native-input-window-surface` 173 owner 十行、`Minecraft.renderFrame`+`GameRenderer` 53 owner
  四条、`security-secure-chat-reporting` 30 owner，全部逐 owner 挂 evidence 翻 `equivalent`
  （camera player 唯一裁决点、debug-crash abort、totem 40-tick、离线服务边界=vanilla
  `OFFLINE_PROPERTIES`、system-chat 抑制改走 `guessChatUUID` map）。同日用户重开的四个 deferred
  全部落地：texture filtering 面、toast/F3 overlay 提到全屏 screen 之上、`SmoothDouble` 平滑镜头、
  IME preedit 焦点移交（`clear_pressed` 不再越权清 composition）。用户裁决 online 模式为非目标，
  整体移除加密登录/签名会话/Services 凭据面（服务端发起握手改为可归因失败，入站签名验证与
  last-seen ack 不变），packet coverage 五条改 not-applicable、账本 deferred 6→5。用户裁决不卡
  Rust 1.80，工具链升 stable 1.97.1 解锁 ICU4X 2.2，`ui-local-time-icu` 关为 `equivalent`：
  ICU4X 承接 CLDR 文本/zone 名/每 locale 历法数字系统、native 承接数字字段与 ICU4J 周算法、
  generated 表承接 quarter/GMT-zero/unknown-city/root 地区码，8656 行 ICU4J 77.1 goldens 全绿
  （同 kind 多宽度须各持独立 names 实例）。账本 active 清零，P3/P4 转维护态，deferred 5→4。

- **2026-08-06（per-client GPU execution duration）**：`performance-gpu-execution-duration` 关为
  `equivalent`。原判词「内核缺 `fence:fence_signaled`」实为上游改名 `dma_fence:`，同一事件；
  harness 写死旧名即误诊源头。`scripts/gpu_duration_capture.py` 关联 `cs_ioctl`（客户端进程
  上下文，pid 即归属）→`sched_run_job`→`dma_fence_signaled(drm_sched)`，按 `(context, seqno)`
  join，并区分 drm_sched 每 entity 的 scheduled/finished 双 context（join 错会把派发延迟当执行
  时长）。radeonsi 与 RADV 共用该内核路径，一次采集覆盖两 API：关联率 Vulkan 124/124、OpenGL
  2778/2778 均 1.0，Weston/Xwayland/官方客户端按 pid 干净分离，overhead +0.23%。采集以非特权
  `perf` 运行（runner 只需 tracefs 组读 + `perf_event_paranoid=-1`），preflight 改为直接探测
  能力而非按权限位推断。

- **2026-08-06（平台 seat 矩阵重划范围）**：用户裁决 X11 为非目标，`platform-linux-x11-seat-matrix`
  改名 `platform-linux-seat-matrix`，X11 发布行转 `not-needed`（实测：X11 下确实开出
  `window_backend=x11` 窗口、硬件 Vulkan、可达 `Play`，但 live look steering 不收敛，而同一 build
  在同 compositor/同 GPU 的 Wayland 上通过——慢帧在通过与失败的运行里同为 15~18 次，故非成因）。
  原判词「无 maintained interactive runner」已被证伪并替换为真实技术边界：headless backend 根本
  不广播 `wl_seat`，而 weston RDP backend 在客户端接入时即广播带 pointer/keyboard 的 seat；
  剩余工作收窄为把 seat 事件接进平台 gate 的 receipts。manifest 两个 limitation 重划到 Wayland 行
  并重钉守卫测试。

原子 slice 的精确边界、测试与 commit hash 只在 Git history 中检索；后续只为新的工程 milestone 追加一行，旧月份过长时继续合并，不恢复历史散文。

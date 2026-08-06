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

- **2026-07**：原生架构与 P0/coverage 骨架建立（生成式 entity/render policy、session/packet、
  resource-pack、registry revision、simulation publication、render-thread 生命周期），并逐项闭环
  entity 全链路与 157 类 entity、194 行 registry、packet 256 行与粒子矩阵、item-component 110 项
  六维 coverage、use prediction 与 container/recipe/creative UI、camera/HUD/pause/player-list、
  render state/layer 与可见性/occlusion、presentation/texture sampling、survival physics、rich text
  与双向 shaping、profiler/post-processing/untrusted decode budgets、lifecycle/fault soak 与
  Linux Wayland/Vulkan release gate。
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
  27 条注册收敛为 23 个投影 owner；`Options` 145 + `ServerList` 13 共 158 owner 拆完挂 evidence
  （生成式 `client_options`、CLI 降级为文件 override、F3+H/F3+P 触发 `Options.save`），五行翻
  `equivalent`。

- **2026-08-05（census closures + 用户裁决三批）**：`Gui` 72 owner 五条 `hud-*`、
  `native-input-window-surface` 173 owner 十行、`Minecraft.renderFrame`+`GameRenderer` 53 owner
  四条、`security-secure-chat-reporting` 30 owner 全部翻 `equivalent`；同日用户重开的四个 deferred
  （texture filtering、toast/F3 overlay、`SmoothDouble` 镜头、IME preedit 移交）全部落地。用户裁决
  online 模式为非目标，加密登录/签名会话/Services 凭据面整体移除（入站签名验证与 last-seen ack
  不变）。工具链升 stable 1.97.1 解锁 ICU4X 2.2，`ui-local-time-icu` 关为 `equivalent`（ICU4X 文本／
  native 数字与周算法／generated 表三方分工，8656 行 ICU4J 77.1 goldens）。账本 active 首次清零。

- **2026-08-06（per-client GPU execution duration）**：`performance-gpu-execution-duration` 关为
  `equivalent`。原判词「内核缺 `fence:fence_signaled`」实为上游改名 `dma_fence:`。
  `scripts/gpu_duration_capture.py` 按 `(context, seqno)` 关联 `cs_ioctl`→`sched_run_job`→
  `dma_fence_signaled`，区分 drm_sched 每 entity 的 scheduled/finished 双 context；radeonsi 与 RADV
  共用该路径，关联率 1.0，overhead +0.23%，以非特权 `perf` 采集。

- **2026-08-06（平台 seat receipts 接入 + X11 归因自我纠正）**：用户裁决 X11 为非目标，X11 发布行
  转 `not-needed`；当日把「X11 下 look steering 不收敛」判为 X11 缺陷是**错误归因**，同一症状在
  Wayland 真 seat 上复现，根因是真 seat 的 `Focused(false)` 压过控制面伪造焦点、客户端按
  `pauseIfInactive` 自开暂停菜单。`physical-seat-input`、`dpi-transition-fullscreen`、`seat-ime`
  依次关闭，**该行 limitation 集合清空**：live gate 增 seat 模式（`BBB_PLAYABLE_SEAT_INJECT`，
  全程零合成 `input.*`），配方为常驻 `uinput` 设备→libinput→sway(+fcitx5)。连带修掉两个真实
  parity bug——暂停屏吞 F11/F2、IME 全程开着抓走每个字母键（vanilla `TextInputManager` 只在文本框
  获焦时开）。环境陷阱与三条硬前提见 memory `seat-driven-gate`。

- **2026-08-06（最后一批派生化与 declared scope 完成态）**：粒子 provider 覆盖表由
  `scripts/particle_provider_catalog_extractor.py` 从 vanilla 源码逐行推导，成为第五份 generated
  coverage ledger，最后一张手工维护的覆盖表退役（手写版有 12 格与自身注记矛盾）。`local_time.rs`
  两处「已记账」假声明查实并当场关闭：Hijri 四种历法改由 ICU4X 直接格式化，flexible day period
  `B` 改由 ICU4J 转储的 1014 行分段表驱动，goldens 自此零 tolerance。lifecycle soak 由三轮扩到
  五轮并把 RSS spread 限额抬到实测噪声带以上（三轮只留 2 个 post-warmup 采样，判据退化成掷
  硬币）。**工作流 v7**：clean tree 下 change-aware 两步改按 `last-gated` 而非 `HEAD^` 取范围
  ——批次 tip 只看最后一个 commit 会静默跳过较早 commit 需要的步骤，实际发生过一次。全仓文档
  过一遍删掉过时声明（evidence 成员计数、已关闭却仍写 `deferred-external` 的 ICU 行、矩阵摘要
  计数、已删除的 Windows projection），archive 自身的散文压回路由行。账本 active=0、五份 ledger
  无 open row、路线全部 `closed`/`maintenance`，四个 stamp 齐平 HEAD，进入声明范围的目标维护态。

原子 slice 的精确边界、测试与 commit hash 只在 Git history 中检索；后续只为新的工程 milestone 追加一行，旧月份过长时继续合并，不恢复历史散文。

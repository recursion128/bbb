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

- **2026-07-09～20**：原生架构、P0/coverage、生成式 entity/render policy、基础 session/packet、
  resource-pack、registry revision、simulation publication 与 render-thread 生命周期建立。
- **2026-07-21**：entity client tick、movement、save/debug、collision/pick、equipment、
  passenger、leash 与 157 类 entity coverage 收口。
- **2026-07-22**：194 行 registry holder/tag、configuration registry、SetEntityData、
  light/chunk/block/event/UI packet families 与 entity presentation 矩阵闭环。
- **2026-07-23**：world effect、screen/HUD、item-component 110 项六维 coverage 与主要
  item interaction prediction 工程收口。
- **2026-07-25～26**：block/item use prediction、container/recipe/creative UI、special
  item/equipment rendering、camera/HUD/pause/player-list 与 profiler 闭环。
- **2026-07-27～28**：post-processing、untrusted decode budgets、stable renderer
  extraction、tutorial/debug、resource/platform workflow、packet 256 行与粒子矩阵闭环。
- **2026-07-29**：block animateTick、collision/survival physics、rich text、render
  state/layer、双向 shaping、多类 UI screen 与可见性/occlusion 工程闭环。
- **2026-07-30**：entity/block-entity presentation、texture sampling、generated
  registry fallback、lifecycle/fault soak 与 Linux Wayland/Vulkan release gate 闭环。
- **2026-07-31**：Vanilla `hotbar.nbt` 互换、Trial Spawner display projection、共享
  camera Water/Lava sampler 与 Lava fog 闭环。
- **2026-08-02**：官方 26.1 渲染机制/可见 workload/同载性能 P0、unihex/CJK 字体，
  以及中文 Vanilla 机制 Wiki 与 parity/document lint 工程闭环。
- **2026-08-02（文档工作流 v10）**：当前事实收敛到 Wiki，历史 Markdown 正文改由
  Git-only 保留，`docs/` allowlist、30 文件 inventory 与 focused fixtures 防止回流。
- **2026-08-02（indexed default pack）**：asset index 30 成为统一 lazy `vanilla`
  provider，font、language、sound 经共享 pack stack 完成真实 zh_cn、OGG 与 CJK 验收。
- **2026-08-02（external-client census）**：2,927 个 source-derived owner 的唯一 JSON
  inventory、细分 parity matrix 与离线 evidence/双向 route/stale-output 门禁建立。
- **2026-08-02（core pipeline generation convergence）**：core shader GPU 准备、发布
  与回滚收敛为一个 renderer-private generation owner 和一份字段映射。
- **2026-08-02（reload-listener generation closure）**：20 个 Vanilla listener owner 完成
  bounded prepare、单 barrier apply/rollback 与真实 default/selected/live 验收。
- **2026-08-02（shader generation architecture convergence）**：shader sources、static core
  pipelines 与 post graphs 收敛为单一 CPU projection 和 renderer transaction API。
- **2026-08-02（LocalPlayer sprint ownership）**：raw sprint key、canonical player latch 与
  outbound baseline 分离，movement/vehicle/combat/FOV 统一读取 world owner。
- **2026-08-03（fatal runtime crash lifecycle）**：startup/main/worker/render fatal 汇入单一
  bounded report owner，ordered shutdown 后原子发布并以非零状态退出。
- **2026-08-03（HUD item projection convergence）**：ordinary/generated/special/composite GUI
  item 收敛到单一 ordered resident-layer projection，旧 quad compatibility resolver 全部删除。
- **2026-08-03（external network terminal close）**：connection/relay task 归入单一
  bounded lifecycle owner，并在 simulation 后、backend drop 前完成 signal/join。
- **2026-08-03（lifecycle terminal budget ownership）**：版本化 manifest 统一拥有三个
  独立终态 deadline，并以服务器 receipt、零工作 gauge 与 clean exit 完成三轮 live soak。
- **2026-08-03（selected shader compilation convergence）**：17 个 pipeline family 的 stage 编译收敛为一个 renderer-private owner，并以 reverse guard 防回流。
- **2026-08-03（Vanilla root window icons）**：统一 root catalog 按平台完成 bounded startup apply、默认 fallback 与 Wayland 零读取闭环。
- **2026-08-03（authenticated profile-key lifecycle）**：pre-connect key prepare、跨 phase
  manager 生存期与 play-local 签名链收敛为一个 net owner；真实 Services 验收按授权凭据重开。
- **2026-08-03（trusted profile texture-set lifecycle）**：Services property trust、
  skin/cape/elytra all-of fallback、bounded process cache 与 CPU→GPU 重交付收敛为一个 owner。
- **2026-08-03（architecture convergence census）**：首轮审计并关闭 5 项；r2 复审 10 个 production
  crate、1,097 个 Rust 文件、23 条正常依赖边与 2,927 个 Vanilla owner，登记三项 ready 候选。
- **2026-08-03（post-effect identity convergence）**：六个 built-in graph 的 admission、
  camera/outline/transparency/blur dispatch 收敛为一个 renderer-private typed owner。
- **2026-08-03（control mailbox convergence）**：control 五类写请求与 window/services command
  收敛到构造时绑定的单一 simulation mailbox，删除兼容 queue、sink 后装和分类型 shadow drain。
- **2026-08-03（entity equipment catalog convergence）**：armor、wings 与 simple-equipment layer
  统一读取 generation-local resolved catalog，删除测试开关驱动的静态 resolver 与兼容分支。
- **2026-08-03（entity textured shader convergence）**：solid、cutout、translucent 与 z-offset
  pipeline 统一派生自单一 renderer-private source，cull descriptor 成为唯一单双面决策。
- **2026-08-03（simulation mailbox hard budget）**：五类 producer 收敛到独立
  count/retained-byte 分区、背压与 terminal close，control 拒绝及 generation diagnostics 可观测。
- **2026-08-03（MenuScreens registration closure）**：25 项 menu-type→screen-kind 映射改由
  Tree-sitter catalog 唯一生成，并逐项贯通 world、layout/input、render 与 close lifecycle。
- **2026-08-03（reachable screen inventory split）**：将混合的 66-owner 屏幕面
  拆为 Chat 25、Death 16、Disconnected 8 与 Pause 17 四条独立机制 route。
- **2026-08-03（remote disconnect screen lifecycle）**：远端 typed terminal outcome 经原子
  session teardown、唯一 input owner 与 immutable renderer/control projection 呈现并保持存活。
- **2026-08-03（indexed disconnect panorama）**：默认 pack 六面/overlay 经唯一 CPU catalog、
  native transaction 与 renderer-private cube generation 接入可达断线页。
- **2026-08-03（phase-aware disconnect audio teardown）**：Login 保留 menu audio，
  Configuration/Play 经既有 effect FIFO 一次 stop-all，并以三轮 live lifecycle 验收。
- **2026-08-03（ChatScreen decision surface）**：25-owner edit、recent/draft/history、permission、
  component interaction、narration 与 T/Slash control/live lifecycle 收敛到单一 native owner。
- **2026-08-03（DeathScreen decision surface）**：16-owner delay、draft/report return、
  normal/hardcore、URL、resource/pixel 与 native-input live respawn 收敛为单一路径。
- **2026-08-03（equipment/falling-dust/piston convergence）**：equipment 影子收敛为 generation-local asset id；FallingDust 与 piston 固定事实改由 29,873-state generated protocol owner 唯一查询。
- **2026-08-03（external session lifecycle closure）**：187 个 `Minecraft` owner 完成拆分，run/task、
  replacement、reload recovery、panic/join 与终态 diagnostics 经 full/live/三轮 soak 收口。
- **2026-08-03（LevelRenderer 七机制 closure）**：77 个 owner 的 backend/section/frame/scene/effect/
  diagnostics/light 以 generation projection、原子 prepare/apply 与 fail-closed evidence 收口。
- **2026-08-03/04（LocalPlayer census 首批 closure）**：149 owner 拆 23 route；block-breaking/prediction 统一 level-bound
  handler+barrier；input-movement 20 owner 经唯一 tick/归一/sprint 链收口；environment-effects 8 owner（crit 经 AndSelf 验证、portal intensity/trigger/spinning 补实现）、world-edit-ui 9 owner（九面唯一 sync_*_editor/screen 链）、camera-targeting 9 owner（唯一 crosshair resolver/camera pose/leash/SetCamera 链）、inventory-actions 9 owner（drop 预测/close 命令/creative throttler/recipe/tutorial-noop 链）、vehicle-control 7 owner（ride tick/riding jump/sprintable/SetPassengers/minecart-flag 链）、item-use-state 6 owner（use-prediction/movement 乘子/flags 投影链）、permission-chat-state 6 owner（permission event/system-chat/visibility/text-filtering 链）、game-mode-lifecycle 7 owner（game-event/login-respawn 重建/abilities/switcher 链）、primary-action-routing 3 owner（mouse 左/右键唯一路由链）、game-mode-guards 5 owner（HUD 门/miss-time/spectator/mount 判定链）、session-respawn 4 owner（login-respawn 重建/respawn 命令/death 计数链）、vitals-status 4 owner（health/experience 应用与 contextual bar 链）、movement-reporting 4 owner（per-tick 报文/sprint 转换/abilities 命令链）、status-options/item-use/entity-actions/container-actions/creative-actions 批量 18 owner（game-event/命令族链，补 spectator 垂直清零）、carried-slot-sync/pick-actions 5 owner 收口——**LocalPlayer/game-mode census 22 route 全 equivalent，域关闭**。
- **2026-08-03/04（generated evidence/layout、ClientLevel census 与 lifecycle closures）**：五份 JSON 收敛；
  121 owner 拆 18 route；全部 18 route（entity/identity/chunk-light/dirty-handoff/clock-weather/ambient-animation/respawn-data/session-services/biome-tint/sound-routing/particle-routing/diagnostics-report/map-data/level-events/block-entity-tracking 等）95 owner 经唯一 store/resolver/gate/装配/分派链收口，census 域关闭。

原子 slice 的精确边界、测试与 commit hash 只在 Git history 中检索；后续只为新的工程 milestone 追加一行，旧月份过长时继续合并，不恢复历史散文。

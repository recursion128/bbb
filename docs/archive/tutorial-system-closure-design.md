# 原生 Tutorial 系统闭环合同

状态：2026-07-28 闭环。

## 目标与范围

本工程关闭 `ClientboundSetPlayerInventoryPacket` 遗留的 Tutorial pickup
side effect，并以 Vanilla 26.1 的完整状态机为边界，避免为单个 packet 建立
孤立提示：

- 六个步骤：Movement、FindTree、PunchTree、OpenInventory、CraftPlanks、None；
- `SetPlayerInventory`、`SetCursorItem`、`ContainerSetSlot` 三条物品获得入口；
- movement、mouse look、crosshair look-at、destroy progress、open inventory
  五类本地回调；
- 与 AdvancementToast、RecipeToast 共用的五槽 `ToastManager`；
- `--tutorial-step` 启动参数，默认 Movement。

不实现游戏内配置 UI，也不新增 options 文件持久化 owner。步骤转换是 canonical
world state；墙钟动画、排槽和进度平滑属于 native runtime；纹理和绘制属于
renderer。

## Vanilla 依据

行为与顺序取自本地 26.1 源：

- `client/tutorial/Tutorial.java` 与 `TutorialSteps.java`；
- 六个 `*TutorialStep*.java` 实例；
- `ClientPacketListener.handleContainerSetSlot`、`handleSetCursorItem`、
  `handleSetPlayerInventory`；
- `MultiPlayerGameMode.startDestroyBlock`、`continueDestroyBlock`、
  `stopDestroyBlock`；
- `TutorialToast.java`、`Toast.java`、`ToastManager.java`。

关键不变量：

- Tutorial 无实例时第一个 level tick 只创建实例；无 level 时停止实例但保留配置步骤；
- Movement 的 move/look 完成条件均为累计 tick `> 40`，提示延迟分别为
  100 tick 和 move 完成后 20 tick；
- FindTree、PunchTree、OpenInventory、CraftPlanks 提示延迟分别为
  6000、600、600、1200 tick；
- FindTree/PunchTree/CraftPlanks 首 tick 按 inventory 或 stats 跳过已完成工作；
- 三条 item packet 在 canonical slot 应用前通知 Tutorial，即使槽位随后被拒绝；
- destroy 回调保留 0、`-1`、逐 tick clamp progress 与 creative 1.0 的 Vanilla
  顺序；
- 普通背包键触发 OpenInventory，服务器控制的坐骑背包命令不触发；
- TutorialToast 高度为 `7 + max(lines, 2) * 11 + 3`，占槽数为
  `ceil(height / 32)`，排队扫描允许后续较小 toast 绕过暂时放不下的大 toast；
- slide 为 600ms 二次曲线，progress 为 100ms clamp lerp；每次 update 最多播放
  一次 visibility sound，同类 challenge sound 去重。

## 所有权与集成顺序

1. `bbb-world` 保存可序列化步骤实例、tick 计数、目标 toast 与 monotonic id。
2. world toast event 只交接 show edge；hide 与 progress 每帧从 canonical
   `active_tutorial_toast` 对账，避免丢失 hide 后永久滞留。
3. `bbb-native` 在 movement/destroy/keybind/crosshair 路径写入回调，随后推进
   Tutorial tick；catch-up movement 用 pending tick 计数，不能折叠成一个布尔样本。
4. native toast runtime 统一分配连续槽位，持有 wall-clock slide/smoothing 状态，
   并用 renderer 的实际 glyph advance 计算本地化换行和占槽高度。
5. `bbb-renderer` 加载 `toast/tutorial` nine-slice 与五类图标，按 Vanilla
   坐标绘制文本、图标和增减色进度条；`Tutorial.key` 的按键占位符保留 bold
   style，并以 styled glyph advance 参与本地化换行。
6. Respawn 重置 level-scoped toast runtime 后重排仍活跃的 TutorialToast；
   disconnect/configuration re-entry 停止实例。

## 验收

- world tests 覆盖六步骤阈值、首 tick skip、三条 packet 前置通知、respawn/
  disconnect 生命周期与全局事件序列；
- native tests 覆盖 catch-up 输入、普通/坐骑背包分支、连续槽排队绕过、slide
  visibility 与 progress smoothing；
- renderer tests 覆盖可变高度、`slot * height` 布局、连续槽校验与 progress
  sanitize；
- `CLIENTBOUND_SET_PLAYER_INVENTORY.runtime_side_effect` ledger 单元改为 complete；
- 最终提交通过 `scripts/cargo-dev.sh gate`。

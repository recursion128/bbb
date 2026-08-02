# Goal 完成里程碑索引

本文件只负责把完成的工程 milestone 路由到 Git history，不保存 slice 合同、取证正文、
状态快照或逐功能完成清单。当前机制、排程和缺口分别以
[`docs/wiki/`](wiki/README.md)、[`goal.md`](../goal.md) 和
[`docs/unsupported-features.md`](unsupported-features.md)为准。

## 追溯方法

- 按时间：`git log --since=2026-07-22 --until=2026-07-23`。
- 按主题：`git log --grep='registry\|render\|font'`。
- 查已删除历史文档：`git log --all -- <path>` 后执行
  `git show <commit>:<path>`。
- 查实现演进：`git log -p -- <code-path>`。Git history 是完整权威，不重写。

## 里程碑

- **2026-07-09～17**：原生客户端架构、P0 audit、coverage/extractor、生成式
  entity metadata/model/render policy 与基础 session/packet owner 建立。
- **2026-07-18～20**：server/client resource-pack stack、known-pack、registry typed
  revision、simulation publication 和 render-thread 生命周期闭环。
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

原子 slice 的精确边界、测试与 commit hash 只在 Git history 中检索；后续只为新的工程
milestone 追加一行，旧月份过长时继续合并，不恢复历史散文。

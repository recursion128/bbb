# 受维护 Markdown inventory

本表以当前 Git 工作树中的全部 Markdown 为全集；新增或删除页面必须同步本表。它记录
canonical ownership 和迁移结论，不把 archive 的历史内容提升为当前事实。引用者由
\`rg -l -F <basename>\` 对 Markdown、shell、Python、JSON 和 TOML 枚举；每格最多列六个，
无引用的页面仍由本清单和 Wiki 导航规则纳管。

## 处理规则

- “保留 evidence”表示页面只拥有版本化原始清单、测量或详细验收，不拥有一般机制。
- “兼容入口”以 scope banner 链接中文 canonical 页面；旧英文正文不维护独立状态。
- archive 只说明完成时点；准确性列不承诺其路径、状态或 reopen 条件仍然当前。
- 相对 Markdown 链接、Wiki 可达性和 inventory 集合相等由 docs lint 逐次验证。

## 清单

| 当前路径 | 当前职责 | 是否仍准确 | Canonical destination | 处理方式 | 引用者/脚本 | 内容类别 | 重复或冲突 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `AGENTS.md` | agent 执行与协作规则 | 是 | `AGENTS.md` | 保留（特殊权威） | `docs/README.md`、`docs/archive/wiki-engineering-design.md`、`docs/packet-coverage-26.1.json`、`docs/wiki/development/architecture-rules.md`、`docs/wiki/development/build-and-cache.md`、`docs/wiki/development/documentation-workflow.md` | 执行规则 | 无；职责隔离 |
| `README.md` | 仓库与文档入口 | 是 | `docs/README.md` | 保留入口 | `docs/README.md`、`docs/archive/wiki-engineering-design.md`、`docs/wiki/README.md`、`docs/wiki/development/repository-map.md`、`goal.md`、`scripts/docs_wiki_lint.py` | 导航 | 无 |
| `docs/README.md` | 文档系统总入口 | 是 | 同路径 | 保留 canonical | `README.md`、`docs/archive/wiki-engineering-design.md`、`docs/wiki/README.md`、`docs/wiki/development/repository-map.md`、`goal.md`、`scripts/docs_wiki_lint.py` | 导航 | 无 |
| `docs/archive/action-bar-title-component-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/add-entity-lifecycle-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | 无 repo 内引用；由 inventory 纳管 | 完成历史 | 否；非当前权威 |
| `docs/archive/adventure-mode-predicate-components-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/animate-player-wake-packet-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/attribute-modifiers-component-roundtrip-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/banner-pattern-pipeline-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/base-item-component-roundtrip-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/beacon-screen-presentation-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/bidirectional-text-shaping-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/block-destruction-packet-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/block-entity-client-effects-closeout-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/block-entity-data-packet-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/block-event-packet-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/block-feature-ordered-submission-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/block-update-packet-family-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/blocks-attacks-component-roundtrip-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/book-screens-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/boss-event-packet-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/camera-fov-inputs-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/camera-perspective-lifecycle-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/camera-post-effect-execution-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/cartography-table-presentation-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/chat-listener-presentation-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | 无 repo 内引用；由 inventory 纳管 | 完成历史 | 否；非当前权威 |
| `docs/archive/christmas-chest-texture-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/chunk-section-occlusion-closeout-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/client-crafting-result-prediction-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/coalesced-resource-texture-sampling-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/code-of-conduct-screen-presentation-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/command-block-editor-packet-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/command-completion-packet-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/command-dispatcher-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | 无 repo 内引用；由 inventory 纳管 | 完成历史 | 否；非当前权威 |
| `docs/archive/component-select-codec-parity-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/configuration-registry-packets-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/consume-effects-component-roundtrip-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/creative-inventory-set-slot-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/crosshair-entity-feedback-closeout-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/custom-data-component-roundtrip-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/damage-event-packet-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/damage-type-item-component-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/debug-subscription-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | 无 repo 内引用；由 inventory 纳管 | 完成历史 | 否；非当前权威 |
| `docs/archive/dimension-type-network-codec-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/dynamic-registry-revision-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | 无 repo 内引用；由 inventory 纳管 | 完成历史 | 否；非当前权威 |
| `docs/archive/enchantment-table-presentation-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/entity-animation-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | 无 repo 内引用；由 inventory 纳管 | 完成历史 | 否；非当前权威 |
| `docs/archive/entity-block-outline-copy-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/entity-block-outline-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/entity-client-tick-movement-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/entity-equipment-passenger-leash-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/entity-event-packet-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/entity-event-particle-audio-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | 无 repo 内引用；由 inventory 纳管 | 完成历史 | 否；非当前权威 |
| `docs/archive/entity-metadata-rich-values-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | 无 repo 内引用；由 inventory 纳管 | 完成历史 | 否；非当前权威 |
| `docs/archive/entity-model-layers-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | 无 repo 内引用；由 inventory 纳管 | 完成历史 | 否；非当前权威 |
| `docs/archive/entity-render-state-split-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/entity-spawn-defaults-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | 无 repo 内引用；由 inventory 纳管 | 完成历史 | 否；非当前权威 |
| `docs/archive/entity-transform-interpolation-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/environment-effect-consumers-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/environment-particle-options-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/environment-position-sampling-foundation-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/equipment-asset-layer-rendering-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/explosion-packet-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/f3i-observable-entity-save-closeout-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/firework-components-roundtrip-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/game-event-win-demo-presentation-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/game-rules-screen-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/game-test-highlight-presentation-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/generic-packet-component-owner-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/heart-health-blink-presentation-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/hide-gui-presentation-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/item-atlas-animation-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/item-component-coverage-foundation-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/item-tint-parity-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/jigsaw-block-editor-packet-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/jukebox-now-playing-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/lectern-screen-presentation-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/local-attack-preflight-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/local-player-collision-owner-closeout-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/local-player-survival-physics-closeout-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/local-player-travel-ordering-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/loom-table-presentation-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/merchant-offers-packet-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/minecart-command-block-editor-packet-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/mob-effect-client-behavior-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/native-business-snapshot-closeout-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/native-profiler-metrics-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/open-screen-presentation-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/pause-screen-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/persistent-holder-or-inline-coverage-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/persistent-holder-reference-coverage-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/persistent-holder-set-coverage-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/place-ghost-recipe-packet-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/player-action-packet-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/player-combat-kill-death-screen-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/player-head-block-profile-skin-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/player-list-presentation-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/player-reporting-services-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/player-social-interactions-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/potion-duration-scale-roundtrip-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/potion-effect-components-roundtrip-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/profile-component-roundtrip-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/queued-light-packet-family-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/recipe-book-lifecycle-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/recursive-item-component-safety-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/recursive-item-components-roundtrip-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/recursive-untrusted-decode-stack-budget-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/registry-holder-tag-final-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/registry-network-holder-set-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/rich-component-presentation-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/scoreboard-client-presentation-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/selected-pack-post-effect-graph-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/server-data-persistence-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/set-entity-data-packet-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/sign-editor-packet-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/sign-text-rendering-parity-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/signed-chat-session-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | 无 repo 内引用；由 inventory 纳管 | 完成历史 | 否；非当前权威 |
| `docs/archive/smithing-screen-presentation-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/sound-caption-overlay-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/special-gui-item-context-clipping-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | 无 repo 内引用；由 inventory 纳管 | 完成历史 | 否；非当前权威 |
| `docs/archive/special-item-model-rendering-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/archive/item-tint-parity-design.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/spectator-gui-menu-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/sprint-swim-camera-presentation-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/spyglass-scope-overlay-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/standalone-texture-sampling-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/structure-block-editor-packet-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/swing-animation-prototype-default-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/synchronized-enchantment-client-effects-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/targeted-local-use-prediction-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | 无 repo 内引用；由 inventory 纳管 | 完成历史 | 否；非当前权威 |
| `docs/archive/test-block-editor-packet-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/test-instance-block-editor-packet-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/trim-component-roundtrip-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/tutorial-system-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/typed-entity-data-component-roundtrip-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/unihex-cjk-font-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/update-advancements-packet-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/update-recipes-packet-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/vanilla-java-frontend-closeout-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/waypoint-locator-bar-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/weapon-interaction-components-roundtrip-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/wiki-engineering-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md`、`docs/wiki/parity/vanilla-26.1-matrix.md`、`goal.md`、`scripts/docs_wiki_lint.py` | 完成历史 | 否；非当前权威 |
| `docs/archive/world-border-vignette-closure-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/archive/written-book-content-component-roundtrip-design.md` | 已完成 slice 合同/证据 | 仅代表完成时点 | `docs/archive/` 与 Git history | 保留 archive | `docs/goal-archive.md` | 完成历史 | 否；非当前权威 |
| `docs/cargo-build-performance.md` | 详细构建命令与历史测量 | 是，政策已降级 | Wiki build-and-cache | 保留 evidence/兼容入口 | `AGENTS.md`、`docs/wiki/development/build-and-cache.md`、`goal.md`、`scripts/cargo-dev.sh`、`scripts/docs_wiki_lint.py` | 长期事实+历史 | scope banner 消除双 owner |
| `docs/code-organization-style.md` | 英文规则详细示例 | 是，规则已降级 | Wiki architecture-rules | 保留 evidence/兼容入口 | `docs/native-client-architecture.md`、`scripts/docs_wiki_lint.py` | 长期规则 | scope banner 消除双 owner |
| `docs/entity-tick-publication-cost-design.md` | simulation/performance详细合同/证据 | 机制可用但含完成时点叙述 | Wiki 相应领域；本页只作 evidence | 保留 evidence（scope banner） | `docs/simulation-publication-snapshot-design.md`、`docs/simulation-render-performance-design.md` | 长期事实+完成历史 | scope banner 消除双 owner |
| `docs/goal-archive.md` | milestone 索引 | 是 | 同路径 | 保留（特殊权威） | `AGENTS.md`、`docs/README.md`、`docs/archive/add-entity-lifecycle-design.md`、`docs/archive/block-entity-client-effects-closeout-design.md`、`docs/archive/entity-model-layers-closure-design.md`、`docs/archive/equipment-asset-layer-rendering-design.md` | 完成历史 | 无；职责隔离 |
| `docs/lifecycle-soak-gate-design.md` | lifecycle/gate详细合同/证据 | 机制可用但含完成时点叙述 | Wiki 相应领域；本页只作 evidence | 保留 evidence（scope banner） | `docs/goal-archive.md` | 长期事实+完成历史 | scope banner 消除双 owner |
| `docs/linux-platform-acceptance.md` | Linux 平台验收矩阵 | 是 | 同路径；规则见 Wiki | 保留 evidence | `docs/goal-archive.md`、`goal.md` | 验收证据 | 无 |
| `docs/native-client-architecture.md` | 英文架构深度映射 | 是，架构已降级 | Wiki architecture pages | 保留 evidence/兼容入口及旧锚点 | `docs/archive/add-entity-lifecycle-design.md`、`docs/archive/item-atlas-animation-design.md`、`docs/archive/item-tint-parity-design.md`、`docs/entity-tick-publication-cost-design.md`、`docs/packet-coverage-26.1.json`、`docs/resource-pack-lifecycle-design.md` | 长期事实+历史 | scope banner 消除双 owner |
| `docs/official-client-performance-benchmark.md` | 当前 host benchmark 详细证据 | 是 | 同路径；机制见 Wiki | 保留 evidence | `docs/wiki/mechanisms/performance/equal-load-benchmark.md`、`goal.md` | 测量证据 | 无；scope banner 已隔离 |
| `docs/performance-baselines.md` | 性能基线摘要 | 是 | 同路径；方法见 Wiki | 保留 evidence | `docs/cargo-build-performance.md` | 测量证据 | 无 |
| `docs/playable-session-lifecycle-design.md` | lifecycle/gate详细合同/证据 | 机制可用但含完成时点叙述 | Wiki 相应领域；本页只作 evidence | 保留 evidence（scope banner） | 无 repo 内引用；由 inventory 纳管 | 长期事实+完成历史 | scope banner 消除双 owner |
| `docs/registry-data-inventory-26.1.md` | 26.1 生成事实报告 | 是 | 同路径；导航见 parity | 保留 generated evidence | `docs/README.md`、`docs/wiki/parity/vanilla-26.1-matrix.md` | 版本化证据 | 无 |
| `docs/registry-holder-consumers-26.1.md` | 26.1 生成事实报告 | 是 | 同路径；导航见 parity | 保留 generated evidence | `docs/README.md` | 版本化证据 | 无 |
| `docs/resource-data-audio-budget-design.md` | 资源详细合同/证据 | 机制可用但含完成时点叙述 | Wiki 相应领域；本页只作 evidence | 保留 evidence（scope banner） | `docs/native-client-architecture.md`、`docs/resource-input-safety-design.md` | 长期事实+完成历史 | scope banner 消除双 owner |
| `docs/resource-image-memory-budget-design.md` | 资源详细合同/证据 | 机制可用但含完成时点叙述 | Wiki 相应领域；本页只作 evidence | 保留 evidence（scope banner） | `docs/native-client-architecture.md`、`docs/resource-data-audio-budget-design.md`、`docs/resource-input-safety-design.md` | 长期事实+完成历史 | scope banner 消除双 owner |
| `docs/resource-input-safety-design.md` | 资源详细合同/证据 | 机制可用但含完成时点叙述 | Wiki 相应领域；本页只作 evidence | 保留 evidence（scope banner） | `docs/native-client-architecture.md`、`docs/resource-image-memory-budget-design.md`、`docs/resource-pack-lifecycle-design.md` | 长期事实+完成历史 | scope banner 消除双 owner |
| `docs/resource-pack-lifecycle-design.md` | 资源详细合同/证据 | 机制可用但含完成时点叙述 | Wiki 相应领域；本页只作 evidence | 保留 evidence（scope banner） | `docs/archive/item-atlas-animation-design.md`、`docs/archive/item-tint-parity-design.md`、`docs/native-client-architecture.md`、`docs/resource-data-audio-budget-design.md`、`docs/resource-input-safety-design.md` | 长期事实+完成历史 | scope banner 消除双 owner |
| `docs/simulation-publication-snapshot-design.md` | simulation/performance详细合同/证据 | 机制可用但含完成时点叙述 | Wiki 相应领域；本页只作 evidence | 保留 evidence（scope banner） | `docs/entity-tick-publication-cost-design.md`、`docs/simulation-render-performance-design.md`、`docs/simulation-render-thread-design.md` | 长期事实+完成历史 | scope banner 消除双 owner |
| `docs/simulation-render-performance-design.md` | simulation/performance详细合同/证据 | 机制可用但含完成时点叙述 | Wiki 相应领域；本页只作 evidence | 保留 evidence（scope banner） | `docs/official-client-performance-benchmark.md` | 长期事实+完成历史 | scope banner 消除双 owner |
| `docs/simulation-render-thread-design.md` | simulation/performance详细合同/证据 | 机制可用但含完成时点叙述 | Wiki 相应领域；本页只作 evidence | 保留 evidence（scope banner） | `docs/entity-tick-publication-cost-design.md`、`docs/native-client-architecture.md`、`docs/resource-pack-lifecycle-design.md`、`docs/simulation-publication-snapshot-design.md`、`docs/simulation-render-performance-design.md` | 长期事实+完成历史 | scope banner 消除双 owner |
| `docs/unsupported-features.md` | 未关闭缺口 ledger | 是 | 同路径 | 保留（特殊权威） | `AGENTS.md`、`README.md`、`docs/README.md`、`docs/archive/action-bar-title-component-closure-design.md`、`docs/archive/add-entity-lifecycle-design.md`、`docs/archive/block-entity-client-effects-closeout-design.md` | 当前状态 | 无；职责隔离 |
| `docs/vanilla-notes/turtle-26.1.md` | Vanilla turtle 原始取证 | 是，限 26.1 | 同路径 | 保留 evidence | 无 repo 内引用；由 inventory 纳管 | 源码证据 | 无 |
| `docs/wiki/README.md` | 中文 Wiki 架构/机制/流程 | 是（本轮源码/代码审计） | `docs/wiki/README.md` | 保留 canonical | `README.md`、`docs/README.md`、`docs/archive/wiki-engineering-design.md`、`docs/wiki/development/repository-map.md`、`goal.md`、`scripts/docs_wiki_lint.py` | 长期机制/流程 | 无；职责隔离 |
| `docs/wiki/architecture/client-boundaries.md` | 中文 Wiki 架构/机制/流程 | 是（本轮源码/代码审计） | `docs/wiki/architecture/client-boundaries.md` | 保留 canonical | `AGENTS.md`、`docs/entity-tick-publication-cost-design.md`、`docs/native-client-architecture.md`、`docs/playable-session-lifecycle-design.md`、`docs/simulation-publication-snapshot-design.md`、`docs/simulation-render-thread-design.md` | 长期机制/流程 | 无；职责隔离 |
| `docs/wiki/architecture/offline-runtime.md` | 中文 Wiki 架构/机制/流程 | 是（本轮源码/代码审计） | `docs/wiki/architecture/offline-runtime.md` | 保留 canonical | `docs/wiki/README.md`、`docs/wiki/mechanisms/performance/equal-load-benchmark.md`、`docs/wiki/mechanisms/resources/offline-asset-trust.md`、`docs/wiki/parity/vanilla-26.1-matrix.md` | 长期机制/流程 | 无；职责隔离 |
| `docs/wiki/architecture/overview.md` | 中文 Wiki 架构/机制/流程 | 是（本轮源码/代码审计） | `docs/wiki/architecture/overview.md` | 保留 canonical | `AGENTS.md`、`docs/native-client-architecture.md`、`docs/resource-pack-lifecycle-design.md`、`docs/wiki/README.md`、`docs/wiki/development/repository-map.md`、`docs/wiki/mechanisms/protocol/client-state-flow.md` | 长期机制/流程 | 无；职责隔离 |
| `docs/wiki/architecture/performance-and-validation.md` | 中文 Wiki 架构/机制/流程 | 是（本轮源码/代码审计） | `docs/wiki/architecture/performance-and-validation.md` | 保留 canonical | `docs/entity-tick-publication-cost-design.md`、`docs/simulation-render-performance-design.md`、`docs/wiki/README.md`、`docs/wiki/mechanisms/performance/equal-load-benchmark.md`、`docs/wiki/mechanisms/rendering/block-rendering.md` | 长期机制/流程 | 无；职责隔离 |
| `docs/wiki/archive/README.md` | 中文 Wiki 架构/机制/流程 | 是（本轮源码/代码审计） | `docs/wiki/archive/README.md` | 保留 canonical | `README.md`、`docs/README.md`、`docs/archive/wiki-engineering-design.md`、`docs/wiki/README.md`、`docs/wiki/development/repository-map.md`、`goal.md` | 长期机制/流程 | 无；职责隔离 |
| `docs/wiki/decisions/README.md` | 中文 Wiki 架构/机制/流程 | 是（本轮源码/代码审计） | `docs/wiki/decisions/README.md` | 保留 canonical | `README.md`、`docs/README.md`、`docs/archive/wiki-engineering-design.md`、`docs/wiki/README.md`、`docs/wiki/development/repository-map.md`、`goal.md` | 长期机制/流程 | 无；职责隔离 |
| `docs/wiki/development/architecture-rules.md` | 中文 Wiki 架构/机制/流程 | 是（本轮源码/代码审计） | `docs/wiki/development/architecture-rules.md` | 保留 canonical | `AGENTS.md`、`docs/README.md`、`docs/code-organization-style.md`、`docs/wiki/README.md` | 长期机制/流程 | 无；职责隔离 |
| `docs/wiki/development/build-and-cache.md` | 中文 Wiki 架构/机制/流程 | 是（本轮源码/代码审计） | `docs/wiki/development/build-and-cache.md` | 保留 canonical | `AGENTS.md`、`docs/README.md`、`docs/cargo-build-performance.md`、`docs/wiki/README.md`、`docs/wiki/architecture/performance-and-validation.md` | 长期机制/流程 | 无；职责隔离 |
| `docs/wiki/development/document-inventory.md` | 中文 Wiki 架构/机制/流程 | 是（本轮源码/代码审计） | `docs/wiki/development/document-inventory.md` | 保留 canonical | `docs/wiki/README.md`、`docs/wiki/archive/README.md`、`docs/wiki/development/documentation-workflow.md`、`docs/wiki/development/repository-map.md`、`scripts/docs_wiki_lint.py`、`scripts/tests/test_docs_lint.py` | 长期机制/流程 | 无；职责隔离 |
| `docs/wiki/development/documentation-workflow.md` | 中文 Wiki 架构/机制/流程 | 是（本轮源码/代码审计） | `docs/wiki/development/documentation-workflow.md` | 保留 canonical | `docs/README.md`、`docs/wiki/README.md`、`docs/wiki/development/repository-map.md` | 长期机制/流程 | 无；职责隔离 |
| `docs/wiki/development/repository-map.md` | 中文 Wiki 架构/机制/流程 | 是（本轮源码/代码审计） | `docs/wiki/development/repository-map.md` | 保留 canonical | `docs/README.md`、`docs/wiki/README.md` | 长期机制/流程 | 无；职责隔离 |
| `docs/wiki/development/testing-and-gates.md` | 中文 Wiki 架构/机制/流程 | 是（本轮源码/代码审计） | `docs/wiki/development/testing-and-gates.md` | 保留 canonical | `docs/README.md`、`docs/lifecycle-soak-gate-design.md`、`docs/playable-session-lifecycle-design.md`、`docs/wiki/README.md`、`docs/wiki/development/architecture-rules.md` | 长期机制/流程 | 无；职责隔离 |
| `docs/wiki/glossary.md` | 中文 Wiki 架构/机制/流程 | 是（本轮源码/代码审计） | `docs/wiki/glossary.md` | 保留 canonical | `docs/wiki/README.md`、`scripts/tests/test_docs_lint.py` | 长期机制/流程 | 无；职责隔离 |
| `docs/wiki/mechanism-template.md` | 中文 Wiki 架构/机制/流程 | 是（本轮源码/代码审计） | `docs/wiki/mechanism-template.md` | 保留 canonical | `docs/wiki/README.md`、`docs/wiki/development/documentation-workflow.md` | 长期机制/流程 | 无；职责隔离 |
| `docs/wiki/mechanisms/performance/equal-load-benchmark.md` | 中文 Wiki 架构/机制/流程 | 是（本轮源码/代码审计） | `docs/wiki/mechanisms/performance/equal-load-benchmark.md` | 保留 canonical | `docs/official-client-performance-benchmark.md`、`docs/wiki/README.md`、`docs/wiki/architecture/offline-runtime.md`、`docs/wiki/architecture/performance-and-validation.md`、`docs/wiki/parity/vanilla-26.1-matrix.md` | 长期机制/流程 | 无；职责隔离 |
| `docs/wiki/mechanisms/protocol/client-state-flow.md` | 中文 Wiki 架构/机制/流程 | 是（本轮源码/代码审计） | `docs/wiki/mechanisms/protocol/client-state-flow.md` | 保留 canonical | `docs/wiki/README.md`、`docs/wiki/architecture/overview.md`、`docs/wiki/parity/vanilla-26.1-matrix.md` | 长期机制/流程 | 无；职责隔离 |
| `docs/wiki/mechanisms/rendering/block-rendering.md` | 中文 Wiki 架构/机制/流程 | 是（本轮源码/代码审计） | `docs/wiki/mechanisms/rendering/block-rendering.md` | 保留 canonical | `docs/wiki/README.md`、`docs/wiki/mechanisms/performance/equal-load-benchmark.md`、`docs/wiki/mechanisms/rendering/fluid-rendering.md`、`docs/wiki/parity/vanilla-26.1-matrix.md` | 长期机制/流程 | 无；职责隔离 |
| `docs/wiki/mechanisms/rendering/fluid-rendering.md` | 中文 Wiki 架构/机制/流程 | 是（本轮源码/代码审计） | `docs/wiki/mechanisms/rendering/fluid-rendering.md` | 保留 canonical | `docs/wiki/README.md`、`docs/wiki/mechanisms/performance/equal-load-benchmark.md`、`docs/wiki/mechanisms/rendering/block-rendering.md`、`docs/wiki/parity/vanilla-26.1-matrix.md` | 长期机制/流程 | 无；职责隔离 |
| `docs/wiki/mechanisms/resources/offline-asset-trust.md` | 中文 Wiki 架构/机制/流程 | 是（本轮源码/代码审计） | `docs/wiki/mechanisms/resources/offline-asset-trust.md` | 保留 canonical | `docs/resource-data-audio-budget-design.md`、`docs/resource-image-memory-budget-design.md`、`docs/resource-input-safety-design.md`、`docs/resource-pack-lifecycle-design.md`、`docs/wiki/README.md`、`docs/wiki/mechanisms/ui/font-and-text.md` | 长期机制/流程 | 无；职责隔离 |
| `docs/wiki/mechanisms/ui/font-and-text.md` | 中文 Wiki 架构/机制/流程 | 是（本轮源码/代码审计） | `docs/wiki/mechanisms/ui/font-and-text.md` | 保留 canonical | `docs/wiki/README.md`、`docs/wiki/mechanisms/resources/offline-asset-trust.md`、`docs/wiki/parity/vanilla-26.1-matrix.md` | 长期机制/流程 | 无；职责隔离 |
| `docs/wiki/parity/README.md` | 中文 Wiki 架构/机制/流程 | 是（本轮源码/代码审计） | `docs/wiki/parity/README.md` | 保留 canonical | `README.md`、`docs/README.md`、`docs/archive/wiki-engineering-design.md`、`docs/wiki/README.md`、`docs/wiki/development/repository-map.md`、`goal.md` | 长期机制/流程 | 无；职责隔离 |
| `docs/wiki/parity/vanilla-26.1-matrix.md` | 中文 Wiki 架构/机制/流程 | 是（本轮源码/代码审计） | `docs/wiki/parity/vanilla-26.1-matrix.md` | 保留 canonical | `docs/wiki/README.md`、`docs/wiki/architecture/overview.md`、`docs/wiki/parity/README.md`、`scripts/docs_wiki_lint.py`、`scripts/tests/test_docs_lint.py` | 长期机制/流程 | 无；职责隔离 |
| `goal.md` | 当前路线、active contract 与门禁 | 是 | `goal.md` | 保留（特殊权威） | `AGENTS.md`、`README.md`、`docs/README.md`、`docs/archive/debug-subscription-design.md`、`docs/archive/dynamic-registry-revision-design.md`、`docs/archive/server-data-persistence-design.md` | 当前状态 | 无；职责隔离 |
| `scripts/vanilla-source-tools/README.md` | 源码工具局部说明 | 是 | 同路径 | 保留 tool-local | `README.md`、`docs/README.md`、`docs/archive/wiki-engineering-design.md`、`docs/wiki/README.md`、`docs/wiki/development/repository-map.md`、`goal.md` | 开发流程 | 无 |

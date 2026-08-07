# 受维护 Markdown inventory

本表列出仓库内全部受维护 Markdown，并由 `scripts/docs_wiki_lint.py` 与实际文件集合做
双向校验。历史正文不占当前 inventory；需要追溯时走 Git history。表中 “canonical
destination” 表示内容的唯一长期归属，不表示另有一份副本。

## 处理规则

- `docs/wiki/**` 保存长期中文架构、机制、术语与开发方法。
- `AGENTS.md`、`goal.md`、`docs/unsupported-features.md` 和紧凑
  `docs/goal-archive.md` 保留特殊流程/状态职责。
- 根与 `docs/` 的 README 只导航；tool-local README 只解释相邻工具。
- 其余 `docs/` Markdown 由 lint 拒绝，历史实现只从 Git 查询。

## 清单

| 当前路径 | 当前职责 | 是否仍准确 | Canonical destination | 处理方式 | 引用者/脚本 | 内容类别 | 重复或冲突 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `AGENTS.md` | agent 执行与协作规则 | 是 | 同路径 | 保留特殊权威 | `goal.md`、Wiki 开发页 | 执行规则 | 无 |
| `README.md` | 仓库入口 | 是 | 同路径 | 保留导航 | `docs/README.md` | 导航 | 无 |
| `docs/README.md` | 文档系统入口 | 是 | 同路径 | 保留导航 | 根 README、docs lint | 导航 | 无 |
| `docs/goal-archive.md` | 紧凑 milestone→Git 路由 | 是 | 同路径 | 保留特殊权威 | `AGENTS.md`、`goal.md` | 历史路由 | 不保存历史正文 |
| `docs/unsupported-features.md` | 未关闭缺口唯一 ledger | 是 | 同路径 | 保留特殊权威 | `goal.md`、parity matrix | 当前状态 | 不保存完成史 |
| `docs/wiki/README.md` | 中文 Wiki 入口与导航 | 是 | 同路径 | 保留 canonical | docs README、docs lint | 导航 | 无 |
| `docs/wiki/architecture/client-boundaries.md` | 状态分类与 crate owner 边界 | 是 | 同路径 | 保留 canonical | AGENTS、机制页 | 长期架构 | 无 |
| `docs/wiki/architecture/offline-runtime.md` | 官方 26.1 离线运行体系 | 是 | 同路径 | 保留 canonical | Wiki、benchmark 机制页 | 长期架构 | 无 |
| `docs/wiki/architecture/overview.md` | 总体主链路与 repo-native 边界 | 是 | 同路径 | 保留 canonical | AGENTS、Wiki | 长期架构 | 无 |
| `docs/wiki/architecture/performance-and-validation.md` | 性能口径与真实性门禁 | 是 | 同路径 | 保留 canonical | benchmark、rendering 机制页 | 长期架构 | 无 |
| `docs/wiki/archive/README.md` | Git 历史查阅方法 | 是 | 同路径 | 保留 canonical | Wiki | 历史路由 | 不保存历史正文 |
| `docs/wiki/decisions/README.md` | 架构决策归属索引 | 是 | 同路径 | 保留 canonical | Wiki | 决策导航 | 无 |
| `docs/wiki/development/architecture-rules.md` | 代码组织与语义 owner 规则 | 是 | 同路径 | 保留 canonical | AGENTS、Wiki | 开发规则 | 无 |
| `docs/wiki/development/build-and-cache.md` | Cargo target/cache 与测量方法 | 是 | 同路径 | 保留 canonical | AGENTS、goal、cargo helper | 开发规则 | 无 |
| `docs/wiki/development/document-inventory.md` | 全 Markdown 双向清单 | 是 | 同路径 | 保留 canonical | docs lint、Wiki | 文档流程 | 自描述行由 lint 覆盖 |
| `docs/wiki/development/macos-and-linux.md` | 双平台开发链路与 cross-check 边界 | 是 | 同路径 | 保留 canonical | Wiki、cargo helper、portability 测试 | 开发规则 | 无 |
| `docs/wiki/development/documentation-workflow.md` | 文档 taxonomy 与唯一权威规则 | 是 | 同路径 | 保留 canonical | Wiki、repository map | 文档流程 | 无 |
| `docs/wiki/development/repository-map.md` | crate、工具、生成物和文档地图 | 是 | 同路径 | 保留 canonical | Wiki | 开发导航 | 无 |
| `docs/wiki/development/testing-and-gates.md` | focused/full/playable/soak/platform 门禁 | 是 | 同路径 | 保留 canonical | AGENTS、Wiki | 开发规则 | 排程只指向 goal |
| `docs/wiki/development/vanilla-workspace.md` | Vanilla 供应链、cache、CLI 与能力矩阵 | 是 | 同路径 | 保留 canonical | scripts/vanilla、开发入口 | 开发流程 | raw Vanilla 内容在仓外 |
| `docs/wiki/glossary.md` | 中英文术语规范 | 是 | 同路径 | 保留 canonical | Wiki 页面 | 术语 | 无 |
| `docs/wiki/mechanism-template.md` | 机制页固定中文模板 | 是 | 同路径 | 保留 canonical | 文档工作流 | 模板 | 无 |
| `docs/wiki/mechanisms/performance/equal-load-benchmark.md` | 官方客户端同载机制与当前证据摘要 | 是 | 同路径 | 保留 canonical | performance 架构页、parity | 机制/证据 | raw 产物在仓外 |
| `docs/wiki/mechanisms/protocol/client-state-flow.md` | protocol→world→renderer-input 状态流 | 是 | 同路径 | 保留 canonical | Wiki、parity | 机制 | 无 |
| `docs/wiki/mechanisms/rendering/block-rendering.md` | 方块剔除、AO、光照与 layer | 是 | 同路径 | 保留 canonical | Wiki、parity | 机制 | 无 |
| `docs/wiki/mechanisms/rendering/fluid-rendering.md` | 流体表面、overlay、tint 与透明边界 | 是 | 同路径 | 保留 canonical | Wiki、parity | 机制 | 无 |
| `docs/wiki/mechanisms/resources/offline-asset-trust.md` | asset-index/object-store 可信链 | 是 | 同路径 | 保留 canonical | Wiki、parity | 机制 | 无 |
| `docs/wiki/mechanisms/ui/font-and-text.md` | font provider、glyph metrics、atlas 与 shaping | 是 | 同路径 | 保留 canonical | Wiki、parity | 机制 | 无 |
| `docs/wiki/parity/README.md` | 机制覆盖审计入口说明 | 是 | 同路径 | 保留 canonical | Wiki | 审计导航 | 不拥有调度状态 |
| `docs/wiki/parity/vanilla-26.1-matrix.md` | 26.1 mechanism-level 覆盖矩阵 | 是 | 同路径 | 保留 canonical | Wiki、docs lint | 审计索引 | open 状态链接 ledger/goal |
| `goal.md` | 当前路线、active contract 与门禁排程 | 是 | 同路径 | 保留特殊权威 | AGENTS、ledger、Wiki | 当前流程/状态 | 不保存完成史 |

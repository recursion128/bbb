# 文档工作流与 canonical ownership

## 特殊权威

`AGENTS.md` 只拥有 agent 协作规则；`goal.md` 只拥有当前路线、active contract 与门禁；
`docs/unsupported-features.md` 只拥有未关闭缺口；`docs/goal-archive.md` 只提供紧凑历史
路由，完整完成史由 Git history 拥有。Wiki 拥有长期机制、架构、术语与验证方法。
parity matrix 只做覆盖索引，不能成为另一份 active/deferred ledger。

## 页面类别

| 类别 | canonical 内容 | 允许内容 |
| --- | --- | --- |
| architecture | 边界、所有权、生命周期 | 稳定数据流和不变量 |
| mechanism | Vanilla 决策级证据与 BBB 映射 | 状态对照、测试、route 链接 |
| parity | mechanism-level coverage 索引 | 稳定 id、状态、证据链接 |
| development | repo/workflow/build/test 规则 | 命令和维护方法 |
| evidence | 版本化 JSON、fixture、checkout-local stamp/report | 可复现身份，不拥有一般机制 |
| history route | 紧凑 milestone 索引与 Git history | 定位历史，不保留完成合同正文 |

当前 taxonomy 刻意不为 `audio`、`persistence`、每一种 screen 等领域创建空目录页：总体
架构先记录其 owner，parity matrix 给出机制级 route；只有完成本地 Vanilla 取证与 BBB
映射后才新增机制页。这样保留用户建议的信息层次，同时避免用标题和英文占位内容伪装
覆盖。六个首批纵向页先覆盖资源可信链、核心状态流、block、fluid、font 和等负载比较。

## 写作与迁移

Wiki 标题、正文、表说明和术语解释使用简体中文；标识符、API 与路径保留原文。迁移旧
页前先将长期事实写入 canonical 页面，再更新所有引用，最后删除旧正文。受维护文档不
保留 compatibility redirect、完成合同目录或历史取证散文；`docs/goal-archive.md` 只给
紧凑 milestone 路由，完整正文由 Git objects/history 保留，不复制 completion prose 到
当前机制页。

机制页使用[模板](../mechanism-template.md)，本地 Vanilla 26.1 源码存在时重新取证。
每个关键行为记录 owner、输入/坐标、顺序、fallback、生命周期、BBB projection 与
focused evidence。发现 product gap 时只更新 ledger/goal route，不在 docs slice 实现。

## 机械检查

[`document-inventory.md`](document-inventory.md)覆盖所有受维护 Markdown。docs lint 检查
Wiki 入口、相对链接、孤儿页、唯一 mechanism id、合法状态、未关闭状态 route、
inventory 覆盖、repo-local path、允许的 `docs/` Markdown 布局、陈旧完成语言和中文正文。

外部客户端 census 的唯一生成 evidence 是
`docs/evidence/vanilla-26.1/client-mechanism-inventory.json`。它不保存状态或任务，只保存共享
Tree-sitter Java frontend 提取的稳定 owner id、源码位置/哈希、mechanism 映射和可定位 BBB
evidence。`scripts/generate-client-mechanism-inventory.py --check` 对完整输出做字节级 stale
检查；Wiki lint 还独立检查 owner/mechanism 唯一性、稳定排序、汇总计数、BBB 路径、
inventory→matrix 映射和 ledger↔matrix 双向 route。矩阵的 `equivalent` 行必须通过该
inventory 或本地链接/路径指向可执行、可定位证据，不能只写“已有测试”。

该 inventory 与 packet、registry-holder、item-component、entity、particle-provider 五份
coverage ledger，以及 client-options、key-mapping 两份 catalog，共同组成
`docs/evidence/vanilla-26.1/` 的封闭版本集合（八个成员）。五份 coverage ledger 的开放行
数由 `scripts/coverage-status.py` 汇总，catalog 与 inventory 不带状态。docs lint 拒绝缺失/
额外成员、`docs/` 根级 JSON 及旧路径引用；generator、Rust consumer、gate 与 Wiki 只能读取
这一目录，不保留兼容副本。

本地 Vanilla tree 存在时，门禁重跑 extractor 并校验全部 source hashes；不存在时明确
打印 exact-output/source-hash 降级，但仍检查版本化 JSON 的 schema、generator-input hash、
repo-owned evidence 和双向 route，因此离线 CI 不依赖个人绝对路径，也不会静默接受断链。

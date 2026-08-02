# 文档工作流与 canonical ownership

## 特殊权威

`AGENTS.md` 只拥有 agent 协作规则；`goal.md` 只拥有当前路线、active contract 与门禁；
`docs/unsupported-features.md` 只拥有未关闭缺口；`docs/goal-archive.md` 与 Git history
拥有完成史。Wiki 拥有长期机制、架构、术语与验证方法。parity matrix 只做覆盖索引，
不能成为另一份 active/deferred ledger。

## 页面类别

| 类别 | canonical 内容 | 允许内容 |
| --- | --- | --- |
| architecture | 边界、所有权、生命周期 | 稳定数据流和不变量 |
| mechanism | Vanilla 决策级证据与 BBB 映射 | 状态对照、测试、route 链接 |
| parity | mechanism-level coverage 索引 | 稳定 id、状态、证据链接 |
| development | repo/workflow/build/test 规则 | 命令和维护方法 |
| evidence | 版本化清单、测量、验收原始摘要 | 可复现身份，不拥有一般机制 |
| archive | 已完成合同与历史 | 当时证据，不声明当前状态 |

当前 taxonomy 刻意不为 `audio`、`persistence`、每一种 screen 等领域创建空目录页：总体
架构先记录其 owner，parity matrix 给出机制级 route；只有完成本地 Vanilla 取证与 BBB
映射后才新增机制页。这样保留用户建议的信息层次，同时避免用标题和英文占位内容伪装
覆盖。六个首批纵向页先覆盖资源可信链、核心状态流、block、fluid、font 和等负载比较。

## 写作与迁移

Wiki 标题、正文、表说明和术语解释使用简体中文；标识符、API 与路径保留原文。迁移旧
页前先将长期事实写入 canonical 页面，再更新所有引用，最后才改为 compatibility
redirect、evidence、archive 或删除。`git log` 足以保留被删除正文，不复制 completion
prose 到当前机制页。

机制页使用[模板](../mechanism-template.md)，本地 Vanilla 26.1 源码存在时重新取证。
每个关键行为记录 owner、输入/坐标、顺序、fallback、生命周期、BBB projection 与
focused evidence。发现 product gap 时只更新 ledger/goal route，不在 docs slice 实现。

## 机械检查

[`document-inventory.md`](document-inventory.md)覆盖所有受维护 Markdown。docs lint 检查
Wiki 入口、相对链接、孤儿页、唯一 mechanism id、合法状态、未关闭状态 route、
inventory 覆盖、repo-local path、双 canonical marker、陈旧完成语言和中文正文。
本地 Vanilla tree 存在时校验记录的关键类；不存在时打印明确降级，不使 CI 依赖个人
绝对路径。

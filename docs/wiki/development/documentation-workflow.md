# 文档工作流与 canonical ownership

## 特殊权威

| 文档 | 唯一职责 |
| --- | --- |
| `AGENTS.md` | 产品边界、推进原则与 agent 协作规则 |
| `docs/unsupported-client-features.md`、`docs/unsupported-server-features.md` | 客户端/服务端未关闭的功能缺口 |
| `docs/client-extensions.md` | 已选定的 BBB 自有客户端能力 |
| [`testing-and-gates.md`](testing-and-gates.md) | 开发循环、测试选择、落地、full gate 与发布 |
| Wiki 其余页面 | 长期机制、架构、术语与开发方法 |

完成史只由 Git 拥有，不建立 archive/index/milestone 页面。

## 页面类别

| 类别 | canonical 内容 |
| --- | --- |
| architecture | crate 边界、所有权、生命周期与跨边界合同 |
| mechanism | 实际机制、设计决策与 BBB owner |
| development | 仓库、构建、测试与流程规则 |
| evidence | `docs/evidence/` 下的版本化 JSON 与 fixture |

同一行为的说明只放机制页，边界页用短合同和链接导航。按真实语义建目录，不建空页或占位页。

## 写作与迁移

- 正文用简体中文，标识符、API 与路径保留原文；客户端专属页用 `client-` 前缀，服务端用 `server-`。
- 机制页使用[模板](../mechanism-template.md)，记录 owner、输入、关键规则与验收测试；Vanilla 行号
  锚点只用于协议、常量和关键规则。记录 bbb 的设计，不逐类复述 Java 实现。
- 改名同步导航、[文档 inventory](document-inventory.md) 与代码/脚本引用；不保留 redirect。
- 文档只写当前事实，不写迁移史、完成过程或测试快照；追溯用 `git log -- <path>`。
- 行为或流程规则变化时同步更新 canonical 页、实际执行脚本与 `AGENTS.md` 指针，删除被取代的条文。

## Ledger 与接手信息

两份 ledger 只保留未关闭的功能缺口（active / 带 `Reopen when ...` 的 deferred）和
`# Not needed` 范围裁决；功能落地时在同一提交删除对应条目。

active 项坐落在 `## <domain>` 标题下，简洁描述玩家或存档可观察的缺口与验收结论：协议同步、
规则/常量/掉落结果、读取官方存档不丢数据、红石机器可用。
tick 内部顺序、RNG 流、goal 相位、对象身份、回滚、Codec 多模式/异常域与诊断文本不登记；推进
原则见 [AGENTS.md](../../../AGENTS.md)。服务端验收覆盖当前 bbb client，不要求官方客户端连接 BBB。

`not-needed` 只登记范围裁决并保留 Vanilla/代码依据与重开条件，删除裁决会丢失范围边界；Java 内部机制
（RNG 流、绘制顺序、内部类接线）已由 `AGENTS.md` 统一声明为非目标，不逐条登记。

## 客户端扩展登记

Vanilla 已有而 BBB 缺失的行为进客户端 ledger；Vanilla 没有的已选定能力进
[`docs/client-extensions.md`](../../client-extensions.md)。构想先讨论，确定后再登记。

```text
## client-extension/<slug>
- Status: active
- Owner: client
- Goal: 用户可观察的目标
- Non-goals: 本次不覆盖的范围
- Dependencies: -
- Acceptance: 可执行验证命令及其必须证明的行为
- Enablement: CLI 启动方式、默认值与关闭后的行为
- Server capability: 无需服务端支持，或明确的服务端能力与缺失时行为
```

每项一个非空单行值，slug 用小写字母、数字和连字符；`Dependencies` 是 `-` 或逗号分隔的扩展 ID；
`deferred` 须补 `- Reopen when: ...`。扩展默认不改变 Vanilla 行为，只通过 CLI 启用；需要服务端
配合时声明能力检测与缺失行为，不向外部服务器擅自发送自定义协议。验收落地时删除条目，长期
行为写入相应 `client-` Wiki 页。

## 机械检查

`cargo dev docs-lint`（`scripts/docs_lint.py`）检查 Wiki 入口、相对链接、孤儿页、唯一 mechanism id、ledger 条目规范、
[inventory](document-inventory.md) 覆盖、允许的 `docs/` 布局、陈旧完成语言、中文正文与单文件行数棘轮，
并对全部 46 个受管生成器的 Rust 源码与证据文件做字节级 exact-output 检查（默认检查 2 个离线生成器与
23 个快速 Vanilla 源码生成器；`--all-generators` 额外覆盖 2 个慢速 Vanilla 生成器与 19 个 JVM 运行时
生成器；`--skip-vanilla` 仅运行 2 个离线生成器以支持便携模式）。

`docs/evidence/` 只由生成脚本写入（如 key-mapping 与 client-options catalog），
不手改；生成器的 `--check` 模式负责新鲜度。

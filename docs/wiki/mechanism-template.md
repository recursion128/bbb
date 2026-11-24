# 机制页面模板

新机制页复制以下结构并删除说明文字。Vanilla 对等机制必须从本地 26.1 源码重新取证。
BBB 自有扩展使用 `扩展 ID：client-extension/<slug>`，不声明 Vanilla `机制 ID`；以下“Vanilla 26.1 权威实现”与“Vanilla/BBB 对照”两节改为“BBB 行为合同”和“启用方式与服务端能力”。
客户端 Vanilla 机制复用已有的 `机制 ID：`；服务端机制页（`server-*.md`）必须使用 `server-` 前缀的 `机制 ID`，不借用客户端 ID 或制造覆盖状态。
按实际所有者、状态、顺序与验收写作，仅对触及的 Vanilla 交互边界取证。

## 机制目的和边界

说明机制负责什么、不负责什么。

## Vanilla 26.1 权威实现

列出权威实现类、关键方法和相邻调用者。

## 决策输入与状态所有权

记录状态来源、查询坐标、线程或阶段。

## 判断与处理顺序

用有序步骤描述 Vanilla 顺序，不以截图反推。

## 生命周期和 fallback

说明创建、刷新、消费、清理以及失败回退结果。

## BBB 实现映射

列出语义所有者、状态投影、消费者和仓库内源码路径。

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 示例 | `Owner.method` | `crate/module` | `unverified` | 待补 |

## 结构性不变量

列出不可被优化或重构破坏的约束。

## 聚焦测试（focused tests）和真实证据

记录测试名、锚点或可复现命令；外部大产物只记录身份和路径边界。

## 当前差异

只解释差异；未关闭状态链接对应差异账本，不在此维护另一份待办清单。

## 账本归属

自有扩展链接 [`docs/client-extensions.md`](../client-extensions.md)；Vanilla 差异按领域链接 [`docs/unsupported-client-features.md`](../unsupported-client-features.md) 或 [`docs/unsupported-server-features.md`](../unsupported-server-features.md) 的权威条目，推进原则见 [`AGENTS.md`](../../AGENTS.md)；没有开放项时状态只取对应差异账本，不补写完成记录。页面命名遵守[文档工作流](development/documentation-workflow.md#写作与迁移)。

## 精确重开条件

给出可机械判断的目标版本、所有者、消费者或外部环境变化，不能只写“未来需要时”。

## 相关页面和源码索引

列出 Wiki 相邻页、BBB 源码路径、Vanilla 类与方法。

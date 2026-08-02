# 机制页面模板

新机制页复制以下结构并删除说明文字。页面必须从本地 Vanilla 26.1 源码重新取证。

## 机制目的和边界

说明机制负责什么、不负责什么。

## Vanilla 26.1 权威实现

列出 owner 类、关键方法和相邻调用者。

## 决策输入与状态所有权

记录状态来源、查询坐标、线程或阶段。

## 判断与处理顺序

用有序步骤描述 Vanilla 顺序，不以截图反推。

## 生命周期与 fallback

说明创建、刷新、消费、清理以及失败结果。

## BBB 实现映射

列出 semantic owner、projection、消费者和 repo-local path。

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 示例 | `Owner.method` | `crate/module` | `unverified` | 待补 |

## 结构性不变量

列出不可被优化或重构破坏的约束。

## 聚焦测试（focused tests）和真实证据

记录测试名、锚点或可复现命令；外部大产物只记录身份和路径边界。

## 当前差异

只解释差异；未关闭状态链接 ledger，不在此维护另一份 Todo。

## 账本/goal 路由

链接 `goal.md` 或 `docs/unsupported-features.md` 的权威条目。已关闭项说明无需路由。

## 精确重开条件

给出可机械判断的目标版本、owner、consumer 或外部环境变化，不能只写“未来需要时”。

## 相关页面和源码索引

列出 Wiki 相邻页、BBB path、Vanilla 类/方法。

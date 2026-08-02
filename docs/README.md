# BBB 文档入口

本目录记录 `bbb` 这个 Minecraft Java 26.1 repo-native 客户端的机制、架构、开发
流程、当前缺口与完成史。第一次阅读建议从[中文 Wiki](wiki/README.md)开始。

## 权威来源

| 内容 | 唯一权威来源 |
| --- | --- |
| 当前优先级、active slice、门禁 | [`goal.md`](../goal.md) |
| agent 执行与协作规则 | [`AGENTS.md`](../AGENTS.md) |
| 未关闭功能缺口 | [`unsupported-features.md`](unsupported-features.md) |
| 完成 milestone 索引 | [`goal-archive.md`](goal-archive.md) 与 Git history |
| 长期架构、机制和验证方法 | [Wiki](wiki/README.md) |
| 版本化生成事实 | 同目录 `packet-coverage-26.1.json`、`registry-holder-consumer-coverage-26.1.json`、`item-component-coverage-26.1.json`、`entity-coverage-26.1.json` 与其代码生成 owner |

Wiki 的 parity matrix 是审计入口，不是另一份调度 ledger。机制尚未关闭时，矩阵只
链接上述权威状态文件。

## 开发入口

- [仓库地图](wiki/development/repository-map.md)
- [架构规则](wiki/development/architecture-rules.md)
- [测试与门禁](wiki/development/testing-and-gates.md)
- [构建与缓存](wiki/development/build-and-cache.md)
- [文档工作流与 inventory](wiki/development/documentation-workflow.md)

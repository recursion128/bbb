# BBB 文档入口

本目录记录 `bbb` 这个 Minecraft Java 26.1 repo-native 客户端与 headless dedicated
server 的机制、架构、开发流程与当前缺口。第一次阅读建议从
[中文 Wiki](wiki/README.md)开始；完成史只从 Git 查询。

## 权威来源

| 内容 | 唯一权威来源 |
| --- | --- |
| 产品边界、推进原则与 agent 协作规则 | [`AGENTS.md`](../AGENTS.md) |
| 开发循环、测试、落地与发布 | [开发流程](wiki/development/testing-and-gates.md) |
| 客户端未关闭功能缺口 | [`unsupported-client-features.md`](unsupported-client-features.md) |
| dedicated server 未关闭功能缺口 | [`unsupported-server-features.md`](unsupported-server-features.md) |
| BBB 自有客户端未关闭能力 | [`client-extensions.md`](client-extensions.md) |
| 完成史 | Git commit、diff 与 path history |
| 长期架构、机制和验证方法 | [Wiki](wiki/README.md) |
| 版本化生成事实与收据 | [`evidence/vanilla-26.1/`](evidence/vanilla-26.1/)，生成与新鲜度规则见[文档工作流](wiki/development/documentation-workflow.md#机械检查) |

## 开发入口

- [仓库地图](wiki/development/repository-map.md)
- [架构规则](wiki/development/architecture-rules.md)
- [开发流程与测试](wiki/development/testing-and-gates.md)
- [命令行与查询参数参考](wiki/development/cli-reference.md)
- [安全边界与生产部署指南](wiki/development/security-and-deployment.md)
- [构建与缓存](wiki/development/build-and-cache.md)
- [文档工作流与 inventory](wiki/development/documentation-workflow.md)

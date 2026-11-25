# BBB 文档入口

本目录记录 `bbb`（Minecraft Java 26.1 仓库原生桌面/WebGPU 多人客户端与仅离线模式无头独立服务端）的架构、机制、开发流程、参数参考与功能差异。首次阅读建议按角色进入：

- **玩家与部署运维**：[项目首页与快速开始](../README.md) → [CLI 与查询参数参考](wiki/development/cli-reference.md) → [安全边界与生产部署](wiki/development/security-and-deployment.md) → [打包与托管配方](../packaging/README.md)
- **源码贡献与开发**：[仓库地图](wiki/development/repository-map.md) → [开发流程与测试](wiki/development/testing-and-gates.md) → [架构规则](wiki/development/architecture-rules.md) → [构建与缓存](wiki/development/build-and-cache.md)
- **深入机制与架构**：[中文机制 Wiki](wiki/README.md) → [总体架构与主链路](wiki/architecture/overview.md)

完成史只从 Git 查询，不在文档中保留归档或历史快照。

## 权威来源

| 内容 | 唯一权威来源 |
| --- | --- |
| 产品边界、推进原则与 agent 协作规则 | [`AGENTS.md`](../AGENTS.md) |
| 开发循环、测试、落地与发布 | [开发流程与测试](wiki/development/testing-and-gates.md) |
| 客户端未关闭功能缺口与范围裁决 | [`unsupported-client-features.md`](unsupported-client-features.md) |
| 独立服务端未关闭功能缺口与范围裁决 | [`unsupported-server-features.md`](unsupported-server-features.md) |
| BBB 自有客户端未关闭能力 | [`client-extensions.md`](client-extensions.md) |
| 完成史 | Git 提交、变更差异与路径历史 |
| 长期架构、机制和验证方法 | [中文机制 Wiki](wiki/README.md) |
| 版本化生成事实与收据 | [`evidence/vanilla-26.1/`](evidence/vanilla-26.1/)，生成与新鲜度规则见[文档工作流](wiki/development/documentation-workflow.md#机械检查) |

## 开发入口

- [仓库地图](wiki/development/repository-map.md)
- [架构规则](wiki/development/architecture-rules.md)
- [开发流程与测试](wiki/development/testing-and-gates.md)
- [命令行与查询参数参考](wiki/development/cli-reference.md)
- [安全边界与生产部署指南](wiki/development/security-and-deployment.md)
- [构建与缓存](wiki/development/build-and-cache.md)
- [文档工作流与清单](wiki/development/documentation-workflow.md)

# BBB 中文机制 Wiki

本 Wiki 是 Vanilla 26.1 客户端机制与 BBB 长期工程事实的 canonical（唯一权威）
入口。页面解释决策输入、查询顺序、状态所有权、fallback（回退行为）和验证证据；
不复制 `goal.md` 或缺口 ledger 的实时状态。

## 导航

### 架构

- [总体架构与主链路](architecture/overview.md)
- [官方离线运行体系](architecture/offline-runtime.md)
- [边界与状态分类](architecture/client-boundaries.md)
- [性能与验证边界](architecture/performance-and-validation.md)

### 首批机制纵向页

- [离线资产可信链](mechanisms/resources/offline-asset-trust.md)
- [protocol → world → renderer-input 状态流](mechanisms/protocol/client-state-flow.md)
- [方块渲染](mechanisms/rendering/block-rendering.md)
- [fluid rendering](mechanisms/rendering/fluid-rendering.md)
- [字体与文本](mechanisms/ui/font-and-text.md)
- [官方客户端等负载比较](mechanisms/performance/equal-load-benchmark.md)

### 审计与开发

- [Vanilla 26.1 parity 审计说明](parity/README.md)
- [Vanilla 26.1 parity matrix](parity/vanilla-26.1-matrix.md)
- [术语表](glossary.md)
- [机制页面模板](mechanism-template.md)
- [仓库地图](development/repository-map.md)
- [架构规则](development/architecture-rules.md)
- [测试与门禁](development/testing-and-gates.md)
- [构建与缓存](development/build-and-cache.md)
- [文档工作流](development/documentation-workflow.md)
- [受维护 Markdown inventory](development/document-inventory.md)
- [决策索引](decisions/README.md)
- [历史资料说明](archive/README.md)

## 阅读和维护规则

页面中的 Rust/Java 类型、方法、packet、registry、resource location、路径和命令保留
原文，叙述使用简体中文。关键 Vanilla 行为必须重新核对本地
`~/Work/mc-code/sources/26.1/`；源码定位描述类与方法，不把官方源码复制进仓库。

机制审计使用 `equivalent`、`partial`、`missing`、`active`、
`deferred-external`、`not-needed`、`unverified`。这些词描述覆盖证据；真正的工程
排程只服从 `goal.md` 和 `docs/unsupported-features.md`。

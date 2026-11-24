# BBB 中文机制 Wiki

本 Wiki 是 Vanilla 26.1 客户端、独立服务端机制与 BBB 长期工程事实的唯一权威入口。页面解释决策输入、查询顺序、状态所有权、回退行为和验证证据；不复制差异账本的实时状态。

## 导航

### 架构

- [总体架构与主链路](architecture/overview.md)
- [客户端边界与状态分类](architecture/client-boundaries.md)
- [服务端边界与 Agent 管理](architecture/server-boundaries.md)

### 客户端机制

| 领域 | 阅读入口 |
| --- | --- |
| 启动、运行与控制 | [运行时、控制邮箱、会话替换与退出](mechanisms/runtime/client-runtime-and-control.md)；[本地配置、按键与窗口](mechanisms/persistence/client-options-and-platform.md) |
| 网络与协议 | [连接、配置期与注册表](mechanisms/protocol/client-connection-and-registries.md)；[协议 → 世界 → 渲染器状态流](mechanisms/protocol/client-state-flow.md) |
| 玩家与世界 | [输入、移动与交互预测](mechanisms/player/client-input-and-interaction.md)；[关卡、区块光照、实体与效果生命周期](mechanisms/world/client-world-lifecycle.md) |
| 场景呈现 | [帧、实体装备、物品、粒子与环境](mechanisms/rendering/client-scene-and-rendering.md)；[方块渲染](mechanisms/rendering/client-block-rendering.md)；[流体渲染](mechanisms/rendering/client-fluid-rendering.md) |
| 界面与文本 | [屏幕、HUD、背包、编辑器与对话框](mechanisms/ui/client-screens-hud-and-inventory.md)；[字体与文本](mechanisms/ui/client-font-and-text.md)；[聊天编辑、命令语法与 UTF-16 文本](mechanisms/ui/client-chat-and-command-text.md) |
| 聊天与身份 | [消息验证、已见序列与皮肤信任](mechanisms/security/client-chat-and-identity.md) |
| 音频与资源 | [音频、种子与播放生命周期](mechanisms/audio/client-audio-pipeline.md)；[离线资产可信链](mechanisms/resources/client-offline-asset-trust.md) |

### 服务端机制

| 领域 | 阅读入口 |
| --- | --- |
| 连接与会话 | [离线登录、配置、压缩与游玩交接](mechanisms/protocol/server-connections-and-sessions.md) |
| 世界调度 | [tick、唯一写者、光照屏障与回调阶段](mechanisms/world/server-tick-and-world.md) |
| 区块与生成 | [票据、噪声群系、地物结构、光照与 FULL 发布](mechanisms/world/server-chunks-and-worldgen.md) |
| 方块与机器 | [方块更新、计划/随机刻、流体与方块实体](mechanisms/blocks/server-block-fluid-and-block-entities.md) |
| 红石 | [信号、邻更、延时元件与活塞](mechanisms/blocks/server-redstone-and-pistons.md) |
| 实体与 AI | [身份、追踪、运动、目标与大脑 AI、刷怪与袭击](mechanisms/entities/server-entities-and-ai.md) |
| 玩家玩法 | [移动、传送、背包菜单、物品使用、战斗与重生](mechanisms/player/server-player-inventory-and-combat.md) |
| 数据驱动规则 | [数据包、注册表与标签、配方、战利品、附魔与模板](mechanisms/resources/server-datapack-and-loot.md) |
| 命令与进度 | [执行上下文、函数、计分板、Boss 血条、成就与奖励](mechanisms/control/server-commands-and-progression.md) |
| 持久化 | [保存、恢复与资源重载](mechanisms/persistence/server-save-and-reload.md) |
| 远程管理 | [MCP、授权与 tick 请求](mechanisms/control/server-mcp-administration.md) |

同一领域的两端职责不同：客户端保存可见副本、预测和呈现，服务端裁决玩法、生成和持久状态。机制页记录已核验的实际路径及其限制；具体未关闭缺口与范围裁决仍以对应差异账本为准。

### 审计与开发

- [术语表](glossary.md)
- [机制页面模板](mechanism-template.md)
- [仓库地图](development/repository-map.md)
- [架构规则](development/architecture-rules.md)
- [开发流程与测试](development/testing-and-gates.md)
- [命令行与查询参数参考](development/cli-reference.md)
- [安全边界与生产部署指南](development/security-and-deployment.md)
- [构建与缓存](development/build-and-cache.md)
- [Vanilla workspace 与供应链](development/vanilla-workspace.md)
- [macOS 与 Linux 开发](development/macos-and-linux.md)
- [Windows 客户端](development/windows-client.md)
- [Web 客户端与网关](development/web-client.md)
- [文档工作流](development/documentation-workflow.md)
- [受维护 Markdown 清单](development/document-inventory.md)
- [决策索引](decisions/README.md)

## 阅读和维护规则

页面中的 Rust/Java 类型、方法、数据包、注册表、资源定位符、路径和命令保留原文，正文叙述使用简体中文。关键 Vanilla 行为必须重新核对 `cargo dev vanilla path 26.1`（或 `scripts/vanilla path 26.1`）返回的已校验源码；源码定位描述类与方法，不把官方源码复制进仓库。

机制审计状态使用 `aligned`、`equivalent`、`partial`、`missing`、`active`、`deferred-external`、`not-needed`、`unverified`。这些词描述覆盖证据；当前未关闭任务与范围裁决只取[客户端功能差异](../unsupported-client-features.md)与[服务端功能差异](../unsupported-server-features.md)；推进原则见 [`AGENTS.md`](../../AGENTS.md)，开发流程与门禁只取[开发流程与测试](development/testing-and-gates.md)。

BBB 自有能力的当前任务见[客户端扩展](../client-extensions.md)，登记规则见[文档工作流](development/documentation-workflow.md#客户端扩展登记)。

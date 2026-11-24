# 受维护 Markdown 文档清单

本表列出仓库内全部受维护的 Markdown 文档，并由 `scripts/docs_wiki_lint.py` 与实际文件集合做
双向校验。历史正文不占当前清单；需要追溯时查询 Git 历史。表中“规范归属”表示内容的唯一长期归属，不表示另有一份副本。

## 处理规则

- `docs/wiki/**` 保存长期中文架构、机制、术语与开发方法。
- [`AGENTS.md`](../../../AGENTS.md)、客户端/服务端两份未支持特性账本与客户端扩展账本保留特殊的协作规则与当前状态职责。
- 仓库根目录与 `docs/` 的 README 只负责导航；工具就近 README 只解释相邻工具。
- 其余 `docs/` 下的 Markdown 由文档检查拒绝，历史实现只从 Git 查询。

## 清单

| 当前路径 | 当前职责 | 是否仍准确 | 规范归属 | 处理方式 | 引用者/脚本 | 内容类别 | 重复或冲突 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `AGENTS.md` | 产品边界、推进原则与协作执行规则 | 是 | 同路径 | 保留特殊权威 | Wiki 开发页、账本 | 执行规则 | 无 |
| `README.md` | 仓库入口 | 是 | 同路径 | 保留导航 | [`docs/README.md`](../../README.md) | 导航 | 无 |
| `SECURITY.md` | 漏洞披露策略与离线/MCP/网关安全边界概览 | 是 | 同路径 | 保留导航 | 根 README、安全部署页 | 安全说明 | 无 |
| `docs/client-extensions.md` | BBB 自有客户端未关闭能力 | 是 | 同路径 | 保留特殊权威 | AGENTS、文档入口、文档检查 | 当前状态 | 无 |
| `docs/README.md` | 文档系统入口 | 是 | 同路径 | 保留导航 | 根 README、文档检查 | 导航 | 无 |
| `docs/unsupported-server-features.md` | 独立服务端未关闭缺口唯一账本 | 是 | 同路径 | 保留特殊权威 | AGENTS、服务端架构页 | 当前状态 | 不保存完成史 |
| `docs/unsupported-client-features.md` | 客户端未关闭缺口唯一账本 | 是 | 同路径 | 保留特殊权威 | AGENTS、客户端 Wiki | 当前状态 | 不保存完成史 |
| `docs/wiki/README.md` | 中文 Wiki 入口与导航 | 是 | 同路径 | 保留为规范页 | 文档入口、文档检查 | 导航 | 无 |
| `docs/wiki/architecture/client-boundaries.md` | 客户端状态分类与 crate 所有权边界 | 是 | 同路径 | 保留为规范页 | AGENTS、机制页 | 长期架构 | 无 |
| `docs/wiki/architecture/overview.md` | 总体主链路与仓库原生边界 | 是 | 同路径 | 保留为规范页 | AGENTS、Wiki | 长期架构 | 无 |
| `docs/wiki/architecture/server-boundaries.md` | 独立服务端、存档/数据包与 MCP 管理边界 | 是 | 同路径 | 保留为规范页 | AGENTS、Wiki | 长期架构 | 无 |
| `docs/wiki/decisions/README.md` | 架构决策归属索引 | 是 | 同路径 | 保留为规范页 | Wiki | 决策导航 | 无 |
| `docs/wiki/development/architecture-rules.md` | 代码组织与语义所有权规则 | 是 | 同路径 | 保留为规范页 | AGENTS、Wiki | 开发规则 | 无 |
| `docs/wiki/development/build-and-cache.md` | Cargo 目标目录、缓存与测量方法 | 是 | 同路径 | 保留为规范页 | AGENTS、`crates/bbb-xtask`（`cargo dev` / `cargo worker`） | 开发规则 | 无 |
| `docs/wiki/development/cli-reference.md` | `bbb-client`/`bbb-server`/`bbb-web-host`/`bbb-admin` 命令行与 Web 查询参数参考 | 是 | 同路径 | 保留为规范页 | Wiki、README、Web 指南 | 开发参考 | 无 |
| `docs/wiki/development/document-inventory.md` | 全仓库 Markdown 双向清单 | 是 | 同路径 | 保留为规范页 | 文档检查、Wiki | 文档流程 | 自描述行由文档检查覆盖 |
| `docs/wiki/development/macos-and-linux.md` | 双平台开发链路与跨平台类型检查边界 | 是 | 同路径 | 保留为规范页 | Wiki、`crates/bbb-xtask`（`cargo dev` / `cargo worker`）、可移植性测试 | 开发规则 | 无 |
| `docs/wiki/development/documentation-workflow.md` | 文档分类与唯一权威规则 | 是 | 同路径 | 保留为规范页 | Wiki、仓库地图 | 文档流程 | 无 |
| `docs/wiki/development/repository-map.md` | crate、工具、生成物和文档地图 | 是 | 同路径 | 保留为规范页 | Wiki | 开发导航 | 无 |
| `docs/wiki/development/security-and-deployment.md` | 离线威胁模型、MCP TLS/OAuth、Web 网关 SSRF/COOP/COEP 与生产部署清单 | 是 | 同路径 | 保留为规范页 | Wiki、SECURITY、README、Web 指南 | 安全与部署 | 无 |
| `docs/wiki/development/testing-and-gates.md` | 开发循环、测试选择、落地、完整门禁、真机验收与发布 | 是 | 同路径 | 保留为规范页 | AGENTS、Wiki | 开发规则 | 无 |
| `docs/wiki/development/vanilla-workspace.md` | Vanilla 供应链、本地缓存、命令行与能力矩阵 | 是 | 同路径 | 保留为规范页 | `scripts/vanilla`、开发入口 | 开发流程 | 原始 Vanilla 内容位于仓库之外 |
| `docs/wiki/development/web-client.md` | Web WASM + WebGPU 客户端与 `bbb-web-host` 网关架构、构建运行与验证门禁 | 是 | 同路径 | 保留为规范页 | Wiki、客户端边界、仓库地图 | 开发流程 | 无 |
| `docs/wiki/development/windows-client.md` | Windows 原生客户端构建、运行、截图录像与本机 GPU-PV 环境接手 | 是 | 同路径 | 保留为规范页 | Wiki、构建与缓存、客户端平台文档 | 开发流程 | 无 |
| `docs/wiki/glossary.md` | 中英文术语规范 | 是 | 同路径 | 保留为规范页 | Wiki 页面 | 术语 | 无 |
| `docs/wiki/mechanism-template.md` | 机制页固定中文模板 | 是 | 同路径 | 保留为规范页 | 文档工作流 | 模板 | 无 |
| `docs/wiki/mechanisms/protocol/client-state-flow.md` | 协议→世界→渲染器输入状态流 | 是 | 同路径 | 保留为规范页 | Wiki | 机制 | 无 |
| `docs/wiki/mechanisms/rendering/client-block-rendering.md` | 方块剔除、环境光遮蔽、光照与渲染层 | 是 | 同路径 | 保留为规范页 | Wiki | 机制 | 无 |
| `docs/wiki/mechanisms/rendering/client-fluid-rendering.md` | 流体表面、叠加层、着色与透明边界 | 是 | 同路径 | 保留为规范页 | Wiki | 机制 | 无 |
| `docs/wiki/mechanisms/resources/client-offline-asset-trust.md` | 资产索引与对象存储可信链 | 是 | 同路径 | 保留为规范页 | Wiki | 机制 | 无 |
| `docs/wiki/mechanisms/ui/client-chat-and-command-text.md` | 聊天界面文本编辑、命令语法着色、确认页与 UTF-16/SNBT packrat 语法 | 是 | 同路径 | 保留为规范页 | Wiki、字体与文本机制 | 机制 | 无 |
| `docs/wiki/mechanisms/ui/client-font-and-text.md` | 字体提供器、字形度量、图集与文本塑形 | 是 | 同路径 | 保留为规范页 | Wiki | 机制 | 无 |
| `docs/wiki/mechanisms/runtime/client-runtime-and-control.md` | 客户端邮箱、会话替换、重载与退出 | 是 | 同路径 | 保留为规范页 | Wiki、架构边界、相邻机制 | 长期机制 | 无 |
| `docs/wiki/mechanisms/player/client-input-and-interaction.md` | 客户端输入、移动与预测交互 | 是 | 同路径 | 保留为规范页 | Wiki、架构边界、相邻机制 | 长期机制 | 无 |
| `docs/wiki/mechanisms/world/client-world-lifecycle.md` | 客户端世界关卡、tick、光照与效果生命周期 | 是 | 同路径 | 保留为规范页 | Wiki、架构边界、相邻机制 | 长期机制 | 无 |
| `docs/wiki/mechanisms/audio/client-audio-pipeline.md` | 客户端声音随机种子、资源与播放生命周期 | 是 | 同路径 | 保留为规范页 | Wiki、架构边界、相邻机制 | 长期机制 | 无 |
| `docs/wiki/mechanisms/world/server-tick-and-world.md` | 服务端 tick、光照屏障与跨阶段回调 | 是 | 同路径 | 保留为规范页 | Wiki、架构边界、相邻机制 | 长期机制 | 无 |
| `docs/wiki/mechanisms/persistence/server-save-and-reload.md` | 服务端保存、恢复与数据包安装 | 是 | 同路径 | 保留为规范页 | Wiki、架构边界、相邻机制 | 长期机制 | 无 |
| `docs/wiki/mechanisms/blocks/server-redstone-and-pistons.md` | 服务端信号传播、更新时序与活塞生命周期 | 是 | 同路径 | 保留为规范页 | Wiki、服务端边界、tick 机制 | 长期机制 | 无 |
| `docs/wiki/mechanisms/control/server-mcp-administration.md` | 服务端 MCP、授权、请求顺序与结果记录 | 是 | 同路径 | 保留为规范页 | Wiki、架构边界、相邻机制 | 长期机制 | 无 |
| `docs/wiki/mechanisms/blocks/server-block-fluid-and-block-entities.md` | 服务端方块、流体与方块实体 | 是 | 同路径 | 保留为规范页 | Wiki、架构边界、相邻机制 | 长期机制 | 无 |
| `docs/wiki/mechanisms/control/server-commands-and-progression.md` | 服务端命令、计分板与进度 | 是 | 同路径 | 保留为规范页 | Wiki、架构边界、相邻机制 | 长期机制 | 无 |
| `docs/wiki/mechanisms/entities/server-entities-and-ai.md` | 服务端实体、生成与 AI | 是 | 同路径 | 保留为规范页 | Wiki、架构边界、相邻机制 | 长期机制 | 无 |
| `docs/wiki/mechanisms/persistence/client-options-and-platform.md` | 客户端本地配置、按键与平台窗口 | 是 | 同路径 | 保留为规范页 | Wiki、架构边界、相邻机制 | 长期机制 | 无 |
| `docs/wiki/mechanisms/player/server-player-inventory-and-combat.md` | 服务端玩家、背包与战斗 | 是 | 同路径 | 保留为规范页 | Wiki、架构边界、相邻机制 | 长期机制 | 无 |
| `docs/wiki/mechanisms/protocol/client-connection-and-registries.md` | 客户端连接、协议阶段与注册表 | 是 | 同路径 | 保留为规范页 | Wiki、架构边界、相邻机制 | 长期机制 | 无 |
| `docs/wiki/mechanisms/protocol/server-connections-and-sessions.md` | 服务端连接、配置握手与玩家会话 | 是 | 同路径 | 保留为规范页 | Wiki、架构边界、相邻机制 | 长期机制 | 无 |
| `docs/wiki/mechanisms/rendering/client-scene-and-rendering.md` | 客户端场景与渲染：帧提取、实体层和资源驻留 | 是 | 同路径 | 保留为规范页 | Wiki、架构边界、相邻机制 | 长期机制 | 无 |
| `docs/wiki/mechanisms/resources/server-datapack-and-loot.md` | 服务端数据包、规则资源与战利品 | 是 | 同路径 | 保留为规范页 | Wiki、架构边界、相邻机制 | 长期机制 | 无 |
| `docs/wiki/mechanisms/security/client-chat-and-identity.md` | 客户端聊天与身份：签名、显示和皮肤信任 | 是 | 同路径 | 保留为规范页 | Wiki、架构边界、相邻机制 | 长期机制 | 无 |
| `docs/wiki/mechanisms/ui/client-screens-hud-and-inventory.md` | 客户端屏幕、HUD 与库存：输入、投影和服务端权威 | 是 | 同路径 | 保留为规范页 | Wiki、架构边界、相邻机制 | 长期机制 | 无 |
| `docs/wiki/mechanisms/world/server-chunks-and-worldgen.md` | 服务端区块、生成与 FULL 状态发布 | 是 | 同路径 | 保留为规范页 | Wiki、架构边界、相邻机制 | 长期机制 | 无 |
| `packaging/README.md` | 桌面客户端、服务端与 Web 网关发布打包配方，含启动器、systemd/launchd 与 Caddy/Nginx 反代说明 | 是 | 同路径 | 保留工具就近说明 | `packaging/bbb-server.service`、`scripts/tests/test_packaging.py` | 工具说明 | 无 |
| `scripts/fixtures/mcp-tls/README.md` | 管理端 TLS 测试夹具的用途与禁用范围 | 是 | 同路径 | 保留工具就近说明 | `bbb-server-admin` TLS 测试 | 工具说明 | 无 |

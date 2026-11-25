# 服务端 MCP 管理

机制 ID：`server-mcp-administration`

## 机制目的和边界

本页定义独立服务端的外部管理面契约：`bbb-mcp` 与 `bbb-server-admin` 提供的 MCP（Model Context Protocol）工具/资源服务，以及 `bbb-admin` CLI 客户端如何安全地通过有界邮箱在权威世界线程执行查询、名单维护、保存、重载与停服。
游戏内命令语法与权限归[服务端命令、计分板与进度](server-commands-and-progression.md)，存档落盘保证归[服务端保存与资源重载](../persistence/server-save-and-reload.md)，`cargo dev world-build` 命令行建图流程见[CLI 参考](../../development/cli-reference.md#用-mcp-建图cargo-dev-world-build)。

## Vanilla 26.1 权威实现

- `IncomingRpcMethods.java`、`MinecraftApi.java`、`MinecraftExecutorServiceImpl.java`：Vanilla 26.1 独立服务端的管理方法定义与主线程执行投递边界。
- `ServerStateService.java:20-27`：区分触发保存服务与向服务器主线程提交停机指令。

BBB 不暴露 Vanilla WebSocket JSON-RPC、RCON、Query 或交互式 stdin 控制台，统一以 MCP 协议和 `bbb-admin` CLI 承载同等管理域能力。

## 决策输入与状态所有权

| 输入 | 所有者 | 下游 |
| --- | --- | --- |
| Streamable HTTP/SSE/TLS 传输、鉴权（Bearer / OAuth 2.1 / 环回权限）、速率/并发限额、超时预算、`request_id` 提取与操作记录 | `bbb-mcp`（`server`、`auth`、`oauth`） | 认证后的 MCP 请求分发 |
| MCP 工具/资源 schema、参数边界校验与 `AdminHost` 适配 | `bbb-server-admin`（`tools`、`host`） | 强类型 `bbb-server-core::admin::AdminRequest` |
| 权限范围检查（`required_scope`）与 `(principal, request_id)` 幂等去重（`IdempotencyRecord`） | `bbb-server-core::admin`（`protocol`、`state`） | 世界内权威管理执行 |
| `AdminGateway` → 有界世界邮箱投递、单次响应等待/tick 截止时间与 `ResourceChange` 订阅广播 | `bbb-server::management`（`RuntimeGateway`） | 世界线程 `WorldCommand::Admin` 队列 |
| 世界只读借用查询（含 `world.info` / `world.blocks` / `world.surface`）、名单变更与命令执行 | `ServerWorld::apply_admin` | 权威结果物化与保存/重载/停服触发 |

## 判断与处理顺序

1. **传输、鉴权与并发预算**：`bbb-mcp` 在默认 10 秒 `transport_timeout` 内完成 TLS 握手与完整 HTTP 请求读取，执行并发连接与 SSE 信号量限额、认证失败限流（`AuthRateLimiter`）、Origin 检查与鉴权（本机环回权限范围、Bearer 文件凭据或远程强制的 TLS + OAuth 2.1），并提取幂等键 `request_id`；SSE 空闲等待事件与正常心跳不计入单帧写出超时。
2. **协议映射与边界校验**：`bbb-server-admin` 校验工具/资源 schema 与参数边界（管理命令文本独立限制 256 个 Unicode 标量字符，`world.blocks` 限制 1–256 个坐标，`world.surface` 限制最多 256 列），构造强类型 `AdminRequest` 交由 `bbb-server::management` 投递至有界世界邮箱。
3. **主线程执行与幂等**：`ServerWorld::apply_admin` 在 tick 边界核验 `required_scope` 与 `(principal, request_id)` 幂等记录，直接借用在线玩家、实体库、区块、计分板与世界规则执行查询或命令，仅在输出边界物化自有响应。
4. **只读世界查询契约**：`world.info`（出生点、维度 ID、建造高度与内存中区块数）、`world.blocks`（按输入顺序返回状态 ID/名称/属性，未 `FULL` 返回 `null`）与 `world.surface`（按 Z/X 增序返回 `loaded` 与最高非空气含液体方块 `topY`）均仅需 `read` 权限，只读取现有状态而不触发区块生成（需加载施工区块时经 `command.execute` 执行 `forceload`）。
5. **生命周期协同与设置持久化**：保存、数据包重载与停服请求转交运行时对应协调器；受理停服仅代表进入有序停机流程，保存完成以实际 IO 确认为准。通过管理接口修改的服务端设置（如接收 `/transfer` 的 `accept_transfers`，对应 CLI `--accepts-transfers true|false` 与持久化键 `accepts-transfers`，默认拒绝转入）在 tick 边界生效并回写至 `--admin-dir/server-settings.json`。

## 生命周期和 fallback

读写超时立即关闭对应 HTTP/SSE 连接并释放连接名额。`request_timeout`（默认 10 秒）仅限制客户端等待响应的时间；已进入邮箱的受托操作继续按序完成并写回原操作记录，绝不因超时产生并发写冲突或半截世界状态。

## BBB 实现映射

| 职责 | 生产路径 | 边界说明 |
| --- | --- | --- |
| MCP 传输、鉴权与限额 | `crates/bbb-mcp/src` | HTTP/SSE、TLS、OAuth 2.1、Bearer/环回鉴权、限流与操作记录 |
| 服务端管理适配与 schema | `crates/bbb-server-admin/src` | 工具/资源 schema、参数校验与 `AdminHost` |
| 轻量运维 CLI | `crates/bbb-admin/src` | 仅依赖 `bbb-mcp`，与代理工具共享同一端点与 schema |
| 世界内管理与邮箱桥接 | `crates/bbb-server-core/src/admin`、`crates/bbb-server/src/management.rs` | 权限范围校验、幂等记录、`world::admin_surface` 与 `RuntimeGateway` |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 主线程安全查询、名单与规则管理 | `MinecraftApi` / `MinecraftExecutorServiceImpl` | `bbb-server-admin` + `ServerWorld::apply_admin` | `aligned` | `bbb-server-admin` 与 `bbb-server` 管理集成测试 |
| 保存与停服状态分离 | `ServerStateService.java:20-27` | `bbb-server::runtime` | `aligned` | 保存屏障与停服操作测试 |
| WebSocket JSON-RPC / RCON / Query / stdin 控制台 | `DedicatedServer` 传统管理入口 | 由 MCP + `bbb-admin` 替代 | `not-needed` | 见[服务端差异账本](../../../unsupported-server-features.md)范围裁决 |

## 结构性不变量

- 管理面绝不成为第二个世界写者；所有读写权威世界的操作必须经有界邮箱在世界线程执行。
- MCP 权限范围（`read < manage < command`）与游戏内玩家权限等级相互独立，不能互相冒充。
- 只读查询直接借用当前 `ServerWorld`，不维护脱离权威状态的影子缓存表，也不隐式触发区块生成。

## 聚焦测试（focused tests）和真实证据

- `cargo dev test -p bbb-mcp`：验证 HTTP/SSE 超时、OAuth/Bearer 鉴权、限流、慢速连接回收与操作生命周期。
- `cargo dev test -p bbb-server-admin`：验证 MCP schema、参数边界校验与 `AdminHost` 映射。
- `cargo dev test -p bbb-admin`：验证 CLI 参数解析、CA 证书回退与管理操作调用。

## 当前差异

BBB 以 MCP 与 `bbb-admin` CLI 取代 Vanilla WebSocket JSON-RPC、RCON、Query 与 stdin 控制台（已在账本登记为 `not-needed`）。当前服务端账本无未关闭的管理面功能缺口。

## 账本归属

管理协议的范围裁决及后续未关闭缺口统一归属[服务端差异账本](../../../unsupported-server-features.md)，部署安全边界见[安全与部署指南](../../development/security-and-deployment.md)，进程边界见[服务端边界](../../architecture/server-boundaries.md)，推进原则见 [`AGENTS.md`](../../../../AGENTS.md)。

## 精确重开条件

当 MCP 传输安全预算调整、新增管理域资源（如多世界热管理），或产品范围显式决定新增外部管理协议适配层时重验本页。

## 相关页面和源码索引

- [服务端边界](../../architecture/server-boundaries.md)：服务端 crate 依赖边界与启动入口。
- [CLI 与查询参数参考](../../development/cli-reference.md#用-mcp-建图cargo-dev-world-build)：`bbb-admin` 参数与 `cargo dev world-build` 批量建图流程。
- [服务端命令、计分板与进度](server-commands-and-progression.md)：管理端共享的命令执行语义。
- [服务端保存与资源重载](../persistence/server-save-and-reload.md)：管理端触发的保存与数据包重载契约。

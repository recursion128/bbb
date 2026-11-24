# 服务端 MCP 管理

机制 ID：`server-mcp-administration`

## 机制目的和边界

本页定义 dedicated server 的外部管理面契约：`bbb-server-admin` 与 `bbb-mcp` 提供的 MCP（Model Context Protocol）工具/资源服务，以及 `bbb-admin` CLI 客户端如何安全地通过有界 mailbox 在权威世界线程执行查询、名单维护、保存、重载与停服。
游戏内命令语法与权限归[服务端命令、计分板与进度](server-commands-and-progression.md)，存档落盘保证归[服务端保存与资源重载](../persistence/server-save-and-reload.md)。

## Vanilla 26.1 权威实现

- `IncomingRpcMethods.java`、`MinecraftApi.java`、`MinecraftExecutorServiceImpl.java`：Vanilla 26.1 dedicated server 的管理方法定义与主线程执行投递边界。
- `ServerStateService.java:20-27`：区分触发保存服务与向服务器主线程提交停机指令。

BBB 不暴露 Vanilla WebSocket JSON-RPC、RCON、Query 或交互式 stdin 控制台，统一以 MCP 协议和 `bbb-admin` CLI 承载同等的管理域能力。

## 决策输入与状态所有权

| 输入 | owner | 下游 |
| --- | --- | --- |
| HTTP/SSE/TLS 传输、超时预算与 operation 记录 | `bbb-mcp` | 认证后的 MCP 请求分发 |
| MCP schema、工具/资源路由与 `AdminHost` 适配 | `bbb-server-admin` | 强类型 `bbb-server-core::admin::AdminRequest` |
| Scope 鉴权、请求去重、速率/并发预算 | `bbb-server-core::admin`（及 runtime `management`） | 有界管理 mailbox |
| 世界只读借用查询、名单变更与命令执行 | `ServerWorld::apply_admin` | 权威结果物化与保存/重载/停服触发 |

## 判断与处理顺序

1. **传输与鉴权**：`bbb-mcp` 在默认 10 秒 `transport_timeout` 内完成 TLS 握手与完整 HTTP 请求读取，校验 bearer token 与权限 scope；SSE 空闲等待事件与正常心跳不计入单帧写出超时。
2. **协议映射与预算**：`bbb-server-admin` 校验参数 schema、去重键与请求预算（命令文本受独立 256 字符上限约束），构造 typed `AdminRequest` 投递进 runtime 有界 mailbox。
3. **主线程执行**：`ServerWorld::apply_admin` 在 tick 边界借用真实在线玩家、实体库、计分板与世界规则执行查询或命令，仅在输出边界将结果物化为 owned 响应。
4. **生命周期协同与设置持久化**：保存、datapack 重载与停服请求转交 runtime 对应协调器；受理停服仅代表进入有序停机流程，保存完成以实际 IO 确认为准。通过管理接口修改的服务端设置（例如接收 `/transfer` 的 `accept_transfers`，对应启动参数 `--accepts-transfers true` 与持久化键 `accepts-transfers`，默认拒绝转入）在 tick 边界生效并回写至 `--admin-dir/server-settings.json`。

## 生命周期和 fallback

读写超时立即关闭对应 HTTP/SSE 连接并释放连接名额。`request_timeout` 仅限制客户端等待响应的时间；已进入 mailbox 的受托操作继续按序完成并写回原 operation 记录，绝不因超时产生并发写冲突或半截世界状态。

## BBB 实现映射

| 职责 | 生产路径 | 边界说明 |
| --- | --- | --- |
| MCP 传输层 | `crates/bbb-mcp` | HTTP/SSE、TLS、超时与 operation 状态 |
| 服务端管理适配 | `crates/bbb-server-admin` | 工具/资源 schema 与 `AdminHost` |
| CLI 管理端 | `crates/bbb-admin` | 面向运维人员与脚本的同一 MCP 客户端 |
| 世界内管理执行 | `crates/bbb-server-core` / `crates/bbb-server` | `admin`、`world::admin_surface` 与 runtime mailbox |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 主线程安全查询、名单与规则管理 | `MinecraftApi` / `MinecraftExecutorServiceImpl` | `bbb-server-admin` + `ServerWorld::apply_admin` | `aligned` | `bbb-server-admin` 与 `bbb-server` 管理集成测试 |
| 保存与停服状态分离 | `ServerStateService.java:20-27` | `bbb-server::runtime` | `aligned` | 保存屏障与停服操作测试 |
| WebSocket JSON-RPC / RCON / Query / stdin 控制台 | `DedicatedServer` 传统管理入口 | 由 MCP + `bbb-admin` 替代 | `not-needed` | 见服务端 ledger 范围裁决 |

## 结构性不变量

- 管理面绝不成为第二个世界 writer；所有读写权威世界的操作必须经 mailbox 在 world 线程执行。
- MCP scope 鉴权与游戏内玩家权限相互独立，不能互相冒充。
- 查询直接借用线上真实 `ServerWorld`，不维护脱离权威状态的影子缓存表。

## 用 MCP 建图

| 工具 | 输入 | 返回 |
| --- | --- | --- |
| `world.info` | `{}` | 出生点，以及运行中维度的资源 ID、含端点的建造高度和内存中的区块数 |
| `world.blocks` | `dimension` 和 1 到 256 个 `{x,y,z}` 坐标 | 按输入顺序返回方块状态 ID、资源名和属性。区块尚未 FULL 时，`state` 为 `null` |
| `world.surface` | `dimension`、`minX`、`minZ`、`maxX`、`maxZ`，矩形含端点，最多 256 列 | 按 Z 行和 X 列增序返回 `loaded` 和 `topY`。`topY` 是最高非空气方块的 Y，包含液体。未加载的列或全空气列的 `topY` 为 `null` |

三个查询使用 `read` 权限。维度使用带命名空间的 ID，例如 `minecraft:overworld`。查询只读取现有状态，遇到尚未 FULL 的区块直接报告。需要施工的区块可由在线玩家加载，或用 `command.execute` 执行 `forceload`。施工复用 `setblock`、`fill`、`clone`、`place template` 等命令。当前管理端点可用 `bbb-admin tools --matching world. --schemas` 查看参数。

```powershell
bbb-admin call world.surface dimension=minecraft:overworld minX=0 minZ=0 maxX=15 maxZ=15
```

需要创建独立存档或重跑一套建图命令时，可用下列命令构建并运行本地入口。

```powershell
cargo dev build -p bbb-server -p bbb-admin
cargo dev world-build --world D:/bbb-data/build-demo --seed demo --terrain flat --load-chunk minecraft:overworld 0 0 --commands D:/bbb-data/build.mcfunction --verify D:/bbb-data/verify.mcfunction
```

命令文件采用 UTF-8，每行一条命令，允许空行、`#` 注释和可选的前导斜杠。入口使用回环地址启动自己的服务端，并为本次施工创建临时 MCP 凭据。建图命令按文件顺序执行，遇到失败报告文件和行号，再正常关服保存此前已执行的命令。成功后显式落盘并停服。提供 `--verify` 时会重开同一存档，再执行验证文件中的命令。地形和种子应用于新存档。

`--load-chunk` 可重复传入，接收维度和区块内的任意方块 X、Z 坐标。入口会登记并保留强加载票据，等 `world.surface` 确认区块已 FULL，再执行建图或验证命令。上面的建图文件可以用 `fill 0 64 0 2 64 2 minecraft:gold_block` 放置方块，验证文件可以用 `execute if block 1 64 1 minecraft:gold_block` 检查重开后的结果。

## 聚焦测试（focused tests）和真实证据

- `cargo dev test -p bbb-mcp`：验证 HTTP/SSE 超时、慢速连接回收与 operation 生命周期。
- `cargo dev test -p bbb-server-admin`：验证 MCP schema、scope 鉴权与 `AdminRequest` 映射。
- `cargo dev test -p bbb-admin`：验证 CLI 参数解析与管理操作调用。

## 当前差异

BBB 以 MCP 与 `bbb-admin` CLI 取代 Vanilla WebSocket JSON-RPC、RCON、Query 与 stdin console（已在[服务端 ledger](../../../unsupported-server-features.md) 登记为 `not-needed`）。

## 账本归属

管理协议取舍记录于[服务端 ledger](../../../unsupported-server-features.md)，进程与产品边界见[服务端边界](../../architecture/server-boundaries.md)，推进原则见 [AGENTS.md](../../../../AGENTS.md)。

## 精确重开条件

当 MCP 传输安全预算调整、新增管理域资源（如多世界热管理），或产品范围显式决定新增外部管理协议适配层时重验本页。

## 相关页面和源码索引

- [服务端边界](../../architecture/server-boundaries.md)：服务端 crate 依赖边界与启动入口。
- [服务端命令、计分板与进度](server-commands-and-progression.md)：管理端共享的命令执行语义。
- [服务端保存与资源重载](../persistence/server-save-and-reload.md)：管理端触发的保存与 datapack reload 契约。

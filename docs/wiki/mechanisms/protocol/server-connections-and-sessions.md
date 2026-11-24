# 服务端连接与会话

机制 ID：`server-connections-and-sessions`

## 机制目的和边界

本页定义 dedicated server 从 TCP 接入、分帧/压缩、协议 phase 转换到有序 Play 输入交付与连接关闭的权威契约。
在声明的 offline-only 认证边界内，服务端严格遵守 Minecraft Java 26.1 wire 协议（可观察的 packet、字段、phase、顺序与同步），不限制连接方仅为 `bbb-client`。
客户端反向路径见[客户端连接](client-connection-and-registries.md)，Play 输入的世界侧消费见[玩家、背包与战斗](../player/server-player-inventory-and-combat.md)。

## Vanilla 26.1 权威实现

- `ServerConnectionListener`、`ServerLoginPacketListenerImpl`、`ServerConfigurationPacketListenerImpl`、`ServerGamePacketListenerImpl`：拥有 TCP 接入、Login/Configuration/Play 阶段切换、KeepAlive 与断线生命周期。
- `GameProtocols.java:190-215`：注册菜单、玩家状态、teleport、实体移除与 respawn 等 clientbound Play stream codec。

## 决策输入与状态所有权

| 输入 | owner | 下游 |
| --- | --- | --- |
| TCP listener、frame/compression、phase 状态机与重配置屏障 | `bbb-server-net`（`listener`、`transport`、`session`、`driver`） | 有序入站消息与出站发送队列 |
| 26.1 wire frame 与 packet codec | `bbb-protocol` | 强类型协议帧与编解码 |
| 同代动态 registry、tags 与 `SelectKnownPacks` 去重 | `bbb-server-core::registry_sync` + `bbb-server::runtime` | Configuration 阶段预编码同步流 |
| Play 阶段 payload mailbox、字节预算与预编码广播（`SharedWorldPacket`） | `bbb-server::runtime` | `ServerWorld::tick` 顺序解码与无逐接收者 `tokio::spawn` 路由投递 |
| 在线玩家游戏状态与无签名聊天窗口 | `bbb-server-core::world::session` | 权威世界状态与广播 |

网络线程与权威世界状态严格分离：连接身份归 `bbb-server-net`，游戏内状态归 `bbb-server-core`，网络线程不直接修改世界。

## 判断与处理顺序

1. **接入与分帧**：读取 VarInt 帧长与压缩长度预算，超界或非法帧立即拒绝。
2. **Handshake / Status / Login**：offline 模式收到 `Hello` 后校验准入（白名单、封禁、人数上限），按序下发压缩配置与 `LoginFinished`；收到在线加密包（如 `Key`）明确拒绝。登录阶段设一次性 30 秒超时。
3. **Configuration、重配置与同步屏障**：握手先下发 `SelectKnownPacks`（`minecraft:core` `26.1`）并根据客户端回包跳过匹配的内置核心元素，发送预编码动态 registry 与 tags（同时兼容 Configuration 期 `CookieResponse` 与 `CustomClickAction`），`driver` 每 15 秒发送并校验 `KeepAlive`（超时或 ID 不匹配即断开）；客户端完成配置确认后切换进入 Play 并投递 Join。Play 期发送 `StartConfiguration` 后连接进入重配置状态，等待 `ConfigurationAcknowledged`（`PlayServerbound::ConfigurationAcknowledged` / Vanilla `ServerboundConfigurationAcknowledgedPacket`）重入 Configuration 并丢弃在途旧 Play 包。
4. **Play 有序投递与背压**：Play 入站保留 packet ID 与原始 payload，按实际堆容量计入有界 mailbox 预算；`ServerWorld::tick` 逐条解码为 `PlayServerbound` 并应用，实际消费或丢弃后才归还预算。入站额度饱和时出站仍继续推进；玩家入战与脱战同步标准 26.1 `PlayerCombatEnter` / `PlayerCombatEnd` 包。
5. **预编码广播与聊天校验**：多接收者广播通过 `SharedWorldPacket` 按压缩阈值单次预编码/压缩帧，并由 `RouteDispatch::flush` 经 `poll_fn` 轮询投递而无需逐接收者 `tokio::spawn`。offline 会话忽略 `ChatSessionUpdate`；`ChatAck`、`Chat` 与 `ChatCommandSigned` 经 `PlayerState::apply_last_seen_offset` / `apply_last_seen_update`（`world/player_state.rs`，对应 Vanilla `LastSeenMessagesValidator`）校验确认窗口与校验和，并以无签名聊天消息广播。

## 生命周期和 fallback

正常关闭按队列顺序发送原因包与 `Close`，投递失败才强制中断 socket。单帧 TCP 写出设 10 秒期限，防止慢读客户端阻塞关闭。
连接驱动结束后，先将已接受且等待预算的输入排空，最后由同一连接任务唯一提交 `Disconnect`，不从 world tick 侧重复排入断线事件。

## BBB 实现映射

| 职责 | 生产路径 | 边界说明 |
| --- | --- | --- |
| 协议编解码 | `crates/bbb-protocol` | 26.1 各 phase packet 与组件/NBT wire 格式 |
| 连接驱动与会话阶段 | `crates/bbb-server-net` | Handshake/Status/Login/Configuration/Play 与重配置状态机 |
| Mailbox、预编码广播与背压接线 | `crates/bbb-server` | 连接任务、预编码同步/广播帧、有界输入预算与 world 线程桥接 |
| Play 会话与注册表同步快照 | `crates/bbb-server-core` | `world::session` 玩家准入、聊天窗口、断线清理与 `registry_sync` |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| Phase 协议与 Play codec | `GameProtocols.java:190-215` | `bbb-protocol` / `bbb-server-net` | `aligned` | 协议编解码单测与进服 E2E |
| Offline 登录与配置阶段 KeepAlive | `ServerLoginPacketListenerImpl` / `ServerConfigurationPacketListenerImpl` | `bbb-server-net::session`、`driver` | `aligned` | `bbb-server-net` 会话阶段与超时测试 |
| `SelectKnownPacks` 与动态 registry/tags 同步 | `RegistryDataLoader` / 配置阶段同步包 | `bbb-server-core::registry_sync` + `bbb-server-net::session` | `aligned` | 已知包去重、配置同步与重配置测试 |
| 在线加密、举报与第三方转发 | Mojang session / 举报 / proxy 扩展 | 声明范围外 | `not-needed` | 见[服务端差异账本](../../../unsupported-server-features.md) |

## 结构性不变量

- 同一连接上入站 Play 包的解码顺序与出站包的发送顺序严格保序；服务端在启动时冻结发布统一的规范注册表顺序（`SynchronizedRegistries` / `PreparedConnectionConfig`），各会话只读共享该冻结顺序，但各自的 `SelectKnownPacks` 去重状态与重配置生命周期相互独立，互不串写。
- 网络驱动不在有界 mailbox 之外开辟无界旁路队列，也不直接写 `ServerWorld`。
- 单个连接生命周期最多向 world 提交一次最终 `Disconnect`。

## 聚焦测试（focused tests）和真实证据

- `cargo dev test -p bbb-server-net`：覆盖分帧预算、offline 登录、Configuration `KeepAlive`、注册表同步与慢读关闭期限。
- `cargo dev test -p bbb-protocol`：覆盖 26.1 Serverbound/Clientbound 各 phase 编解码 round-trip。
- `cargo dev e2e client`：真实进程验证登录、配置同步、进服、移动、背包与重连。

## 当前差异

Mojang 在线加密认证、签名聊天密钥链、官方玩家举报与 BungeeCord/Velocity 转发不在 offline-only 产品范围内；当前服务端差异账本的开放项与范围裁决统一以[服务端差异账本](../../../unsupported-server-features.md)为准，若后续发现协议或会话未关闭缺口则登记于该账本。
registry 同步中的组件字节布局无需与 Vanilla 逐字节相同，但须满足 26.1 wire 解码与语义一致。

## 账本归属

协议与会话的范围裁决及后续发现的未关闭缺口统一归属[服务端差异账本](../../../unsupported-server-features.md)，产品范围见[服务端边界](../../architecture/server-boundaries.md)，推进原则见 [AGENTS.md](../../../../AGENTS.md)。

## 精确重开条件

当目标协议版本升级、`GameProtocols` 阶段/包目录变更、Configuration 同步屏障调整，或产品范围显式纳入在线认证/代理转发协议时重验本页与对应 codec。

## 相关页面和源码索引

- [服务端边界](../../architecture/server-boundaries.md)：进程模型与 offline-only 范围。
- [客户端连接、协议阶段与注册表](client-connection-and-registries.md)：客户端侧协议状态机。
- [服务端玩家、背包与战斗](../player/server-player-inventory-and-combat.md)：Play 包在权威世界中的应用。

# 客户端协议、世界与渲染状态流

机制 ID：`protocol-client-state-flow`

## 机制目的和边界

该机制保证网络数据包只被解码一次、按连接到达顺序应用到唯一世界所有者，并以同一帧一致的只读投影交给渲染器。渲染快照不等同于世界真值，控制接口快照也不得反向改写权威状态。

## Vanilla 26.1 权威实现

`ClientHandshakePacketListenerImpl`、配置阶段监听器与 `ClientPacketListener` 随协议阶段接收强类型数据包。`ClientPacketListener` 将持久变更写入 `ClientLevel`、实体、计分板、配方与注册表等所有者，同时调用 `Minecraft`、`LevelRenderer`、音频与界面处理一次性副作用。`LevelRenderer` 再从当前 `ClientLevel` 与相机提取本帧可见渲染状态。

## 决策输入与状态所有权

当前连接阶段决定哪些数据包合法可解码；数据包到达顺序决定世界应用顺序；当前会话注册表代际决定动态 ID 与 holder 的解析结果。世界层拥有持久游戏状态，运行时副作用出口拥有瞬时事件消费，渲染器拥有 GPU 后端与呈现状态。

## 判断与处理顺序

1. `bbb_client_net::types::connection::ConnectionState` 仅放行当前协议阶段合法的数据包。
2. `bbb-protocol` 将载荷解码为结构化类型，并在入口预先校验长度、枚举、NBT 与资源预算。
3. Play 阶段数据包按到达顺序交由 `WorldStore::apply_play_packet` 应用。
4. `PlayApplyEffects` 按应用顺序交付音频、粒子等瞬时事件。客户端界面切换排入强类型 `ScreenRequest` FIFO；Configuration 与 Play 入口在每个 `NetEvent` 之后立即消费该队列，确保后续事件看到此前真实的屏幕结果。单条输入或控制命令产生的屏幕请求同样紧随该命令之后消费。
5. `SimulationCore::step` 推进本地 tick、客户端预测与服务端校正，同时独占活动世界与输入状态。
6. 渲染阶段直接从借用的 `SimulationView` 提取 `RendererFrame`，绑定一致的资源代际与相机状态。控制接口与 MCP 另消费快照发布层产出的不可变世界、界面与运行时投影，绝不回写活动状态。

## 生命周期和 fallback

`Handshake → Status/Login → Configuration → Play` 的阶段转换全部显式推进；重新连接会建立全新的注册表与世界实例。非法数据包一律 fail closed 或触发明确的断开连接；无法识别的动态 holder 不得猜测为其他 ID。渲染资源缺失只影响呈现回退，绝不改写权威世界状态。

连接替换、远端断线、音频全部停止与网络线程退出的完整顺序由[客户端运行时](../runtime/client-runtime-and-control.md)拥有；音频后端的消费与清理见[客户端音频链路](../audio/client-audio-pipeline.md)。本页维护「数据包 → 副作用 → 投影发布」的单一数据流顺序。

## 离线模式签名边界

在线认证模式是明确的产品非目标：客户端不持有本地 profile 密钥对、不执行加密登录与 Mojang 会话加入、不发送 `ChatSessionUpdate`。Vanilla 的 `AccountProfileKeyPairManager` 与本地聊天签名链在 BBB 中不设运行时所有者，出站聊天与可签名命令参数保持 Vanilla「无密钥对」语义（`ChatMessage.signature` 恒为 `None`，`ArgumentSignatures` 恒为空列表），而 `last-seen` 确认管线照常运行。入站方向保持完整验证：远端玩家的 `InitializeChat` 会话与签名消息仍通过公共 Services 公钥集验证（`services_profile_keys` 仅做无凭据 GET）。当服务端发起加密握手时，客户端以可归因错误失败（`login::reject_encryption_request`）。范围裁决与重开条件由客户端账本统一维护。

## BBB 实现映射

- `bbb-protocol`：协议线格式编解码与安全预算。
- `bbb_client_net::types::connection`：连接阶段状态机。
- `bbb_client_world::play_apply`：`WorldStore::apply_play_packet` 与 `PlayApplyEffects`，分离持久状态与瞬时副作用。
- `bbb_client::simulation`：主线程模拟核心与步进调度。
- `bbb_client::simulation::publication`：控制接口与 MCP 只读快照的原子发布。
- `bbb_client::runtime::frame::update_renderer`：单帧提取与当前屏幕 HUD 组装。
- `bbb_client::runtime::render_extract::RendererFrame`：送往渲染器的不可变帧输入。

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 按协议阶段解码 | 各阶段数据包监听器 / `GameProtocols` | `bbb_client_net::types::connection::ConnectionState`、`bbb-protocol` | `aligned` | 协议阶段与编解码行为测试 |
| 有序持久状态应用 | `ClientPacketListener` → `ClientLevel` | `WorldStore::apply_play_packet` | `aligned` | Play 阶段数据包应用测试 |
| 瞬时副作用交付 | 监听器 → 渲染/音频/界面 | `PlayApplyEffects`、`ScreenRequest` FIFO | `aligned` | 副作用顺序与屏幕替换测试 |
| 模拟快照发布 | tick/渲染边界 | `SimulationCore`、快照发布层 | `aligned` | 快照集成与一致性测试 |
| 渲染帧提取 | `LevelRenderer` / 渲染状态 | `RendererFrame` | `aligned` | 客户端帧提取单元测试 |
| Profile 密钥管理与本地签名会话 | `AccountProfileKeyPairManager` / `ClientPacketListener` | 离线模式：无密钥对表面，出站恒无签名 | `not-needed` | 客户端账本 `# Not needed` 裁决；入站验证见 `bbb_client_world::client::player_info` |

## 结构性不变量

- 网络数据包到达顺序不得因多消费者竞争而重排。
- 权威状态、派生状态、瞬时事件与后端状态四类边界严禁互相冒充。
- 渲染与控制快照均为只读投影，且携带明确的代际或 tick 标识。
- 动态注册表相关数值只在当前会话上下文中解码。

## 聚焦测试（focused tests）和真实证据

协议、世界与客户端的聚焦测试分别验证编解码、状态应用、`ScreenRequest` FIFO 与快照发布；端到端会话回放执行方式见[开发流程与测试](../../development/testing-and-gates.md)。本机制的核心行为测试包括：

- `bbb_client_net::event_stream::tests::batch_01::login_compression_query_and_finish_follow_vanilla_state_sequence`：验证登录回包、压缩生效与切入 Configuration 阶段顺序。
- `bbb_client_net::event_stream::tests::batch_01::configuration_control_responses_precede_finish_and_play_handoff`：验证配置期控制响应不被注册表应用阻塞，且仅在权威确认后切入 Play 阶段。
- `bbb_client_net::event_stream::tests::batch_02::play_start_configuration_acknowledges_and_resets_configuration_dedup_state`：验证 Play 重入 Configuration 的确认顺序、阶段切换与监听器去重状态重置。
- `bbb_client_world::play_apply::tests::stream_and_audio_events::apply_play_packet_orders_typed_explosion_effects_before_knockback`：验证同一数据包的瞬时副作用与权威玩家状态按 Vanilla 顺序恰好应用一次。
- `bbb_client::control::server::tests::batch_02::client_ui_reads_canonical_world_state`：验证 `world.client_ui` 查询保留协议与客户端对话框投影字段。
- `bbb_client::runtime::events::dispatcher::terminal_tests::command_confirmation_show_clear_show_fifo_and_container_replacement`：验证跨事件的 `ScreenRequest` FIFO 与真实屏幕替换。

上述用例全部直接断言状态迁移或数据结果，不以源码文本检查替代运行时行为验证。

## 当前差异

[客户端账本](../../../unsupported-client-features.md)当前无未关闭的 `# Todo` 缺口；离线模式与本地 profile 密钥对的范围裁决登记于该账本 `# Not needed` 段落。后续新增协议包或消费链若存在缺口，统一以账本为准。

## 账本归属

协议状态流与数据包范围裁决及未关闭差异统一由[客户端账本](../../../unsupported-client-features.md)维护。

## 精确重开条件

当目标协议版本、`GameProtocols` 数据包表或阶段切换与副作用应用顺序发生变化时，须重新核验对应的「编解码 → 状态应用 → 消费者」全链路。

## 相关页面和源码索引

- [总体架构](../../architecture/overview.md)
- [客户端架构边界](../../architecture/client-boundaries.md)
- Vanilla：`GameProtocols`、`ClientPacketListener`、`ClientLevel`、`LevelRenderer`

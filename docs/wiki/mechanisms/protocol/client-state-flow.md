# 客户端 protocol → world → renderer-input 状态流

机制 ID：`protocol-client-state-flow`

## 机制目的和边界

该机制保证 wire packet 被解码一次、按连接顺序应用到唯一 world owner，并以一帧一致
projection 交给 renderer。它不把 renderer snapshot 当作世界，也不允许 control
snapshot 回写 canonical state。

## Vanilla 26.1 权威实现

`ClientHandshakePacketListenerImpl`、configuration listener 与 `ClientPacketListener`
随 protocol 阶段接收 typed packet。`ClientPacketListener` 将持久变更写入
`ClientLevel`、entity、scoreboard、recipe/registry 等 owner，同时调用
`Minecraft`、`LevelRenderer`、sound/UI 等处理一次性效果。`LevelRenderer` 从当前
`ClientLevel` 和 camera 提取可见渲染状态。

## 决策输入与状态所有权

连接阶段决定某 packet 是否可解码；packet arrival order 决定 apply order；当前
registry 决定 dynamic id/holder 解析。world 拥有持续状态，runtime effect sink 拥有
瞬时消费，renderer 拥有 GPU backend。

## 判断与处理顺序

1. `bbb-client-net::ConnectionState` 只接受当前 phase 合法 packet。
2. `bbb-protocol` 解码为结构化类型；长度、枚举与 NBT/resource budget 先验证。
3. play packet 按到达顺序交给 `WorldStore::apply_play_packet`。
4. `PlayApplyEffects` 按 apply 顺序交付音频、粒子等事件。native UI 变化排入 typed
   `ScreenRequest` FIFO；configuration/play 入口在每个 NetEvent 后立即消费，后一个 event
   因而看到此前的真实屏幕结果。输入/control 单条命令产生的请求也在该命令后消费。
5. `SimulationCore::step` 完成本地 tick、预测与校正，同时持有唯一 live world/input。
6. render 从借用的 `SimulationView` 生成 `RendererFrame`，使用一致的 resource generation
   和相机状态。control/MCP 另消费 publication 发布的不可变 world/UI/运行时投影，不回写 live state。

## 生命周期和 fallback

handshake → status/login → configuration → play 的阶段转换显式发生；重连建立新 registry
和 world。无效 packet fail closed 或产生明确 disconnect；不认识的动态 holder 不可猜成
另一个 id。缺 renderer resource 只影响 presentation fallback，不修改 canonical world。

连接替换、远端断线、stop-all 与线程退出的完整顺序由
[客户端运行时](../runtime/client-runtime-and-control.md)拥有；声音 backend 的消费与清理见
[音频链路](../audio/client-audio-pipeline.md)。本链路保留 packet → effect → publication 的单一顺序。

## Offline-only 签名边界

Online 模式是明确的产品非目标：客户端不持有 profile key pair、不做
encrypted login/session join、不发 `ChatSessionUpdate`。Vanilla 的
`AccountProfileKeyPairManager`/play-local 签名链没有当前 owner，出站聊天与
signable command 参数保持 vanilla「无 key pair」语义（`ChatMessage.signature` 恒 None、
`ArgumentSignatures` 恒空），last-seen ack 管线照常。入站方向不变：远端玩家的
`InitializeChat` session 仍用 Services 公钥集验证（`services_profile_keys` 为无凭据 GET）。
服务端发起 encryption 握手时连接以可归因错误失败（`login::reject_encryption_request`）。
重开条件只由唯一 gap ledger 维护。

## BBB 实现映射

`bbb-protocol` 拥有 wire codec；`bbb_client_net::types::connection` 拥有阶段；
`bbb_client_world::play_apply` 的 `WorldStore::apply_play_packet` 与 `PlayApplyEffects` 分开
状态和副作用；`bbb_client::simulation` 拥有 simulation；
`bbb_client::simulation::publication` 发布 control/world；
`bbb_client::runtime::frame::update_renderer` 组装帧并选择当前 Screen HUD；
`bbb_client::runtime::render_extract::RendererFrame` 是 renderer input。

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| phase-specific decode | packet listeners / `GameProtocols` | `bbb-client-net::ConnectionState`, `bbb-protocol` | `aligned` | phase/codec 行为测试 |
| ordered persistent apply | `ClientPacketListener` → `ClientLevel` | `WorldStore::apply_play_packet` | `aligned` | play-apply tests |
| transient side effects | listener → renderer/audio/UI | `PlayApplyEffects`、ScreenRequest FIFO | `aligned` | recording-effects tests |
| simulation publication | tick/render boundary | `SimulationCore`, publication | `aligned` | snapshot integration tests |
| renderer extraction | `LevelRenderer`/render states | `RendererFrame` | `aligned` | native renderer-frame tests |
| profile-key manager / play session | `AccountProfileKeyPairManager` / `ClientPacketListener` | offline-only：无 profile-key surface，出站恒 unsigned | `not-needed` | ledger（online 模式非目标）；入站验证见 `bbb_client_world::client::player_info` |

## 结构性不变量

- network arrival order 不能被多个消费者竞争改变。
- canonical、derived、transient、backend 四类状态不能互相冒充。
- renderer/control snapshot 都是 read-only，且标记 generation/tick。
- dynamic registry 值只在当前 session context 解码。

## 聚焦测试（focused tests）和真实证据

protocol、world 与 native focused tests 分别验证 codec、apply、ScreenRequest FIFO 和
publication。会话回放比较真实输出，执行入口见[开发流程与测试](../../development/testing-and-gates.md)。
本机制的可执行决策合同包括：

- `bbb_client_net::event_stream::tests::batch_01::login_compression_query_and_finish_follow_vanilla_state_sequence`
  验证 login 回包、compression 生效与 configuration phase 切换；
- `bbb_client_net::event_stream::tests::batch_01::configuration_control_responses_precede_finish_and_play_handoff`
  验证 configuration control 回包不被 canonical registry apply 阻塞，且 finish 只在
  canonical acknowledgement 后切换为 play；
- `bbb_client_net::event_stream::tests::batch_02::play_start_configuration_acknowledges_and_resets_configuration_dedup_state`
  验证 play 重入 configuration 的 acknowledgement、phase 与 listener-local dedup 状态顺序；
- `bbb_client_world::play_apply::tests::stream_and_audio_events::apply_play_packet_orders_typed_explosion_effects_before_knockback`
  验证同一 typed packet 的瞬时 effect 与 canonical player state 按 Vanilla 顺序只应用一次；
- `bbb_client::control::server::tests::batch_02::client_ui_reads_canonical_world_state`
  验证 world.client_ui 查询保留协议与 native Dialog 投影的字段；
- `bbb_client::runtime::events::dispatcher::terminal_tests::command_confirmation_show_clear_show_fifo_and_container_replacement`
  验证跨 event 的 ScreenRequest FIFO 与真实屏幕替换。

这些都是执行状态迁移或数据结果的行为测试，不使用 source-string guard 代替语义
断言。

## 当前差异

当前差异只取 ledger；新增 packet 若尚未逐机制取证，不由本页推断关闭状态。

## 账本归属

packet gap 只由 [`unsupported-client-features.md`](../../../unsupported-client-features.md) 维护。

## 精确重开条件

目标 protocol 版本、`GameProtocols` packet 表或 packet 合同变化时，重新核验相应 codec→apply→消费者链。

## 相关页面和源码索引

- [总体架构](../../architecture/overview.md)
- [边界与状态分类](../../architecture/client-boundaries.md)
- Vanilla：`GameProtocols`、`ClientPacketListener`、`ClientLevel`、`LevelRenderer`

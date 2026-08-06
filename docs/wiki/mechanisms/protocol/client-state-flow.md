# protocol → world → renderer-input 状态流

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

1. `bbb-net::ConnectionState` 只接受当前 phase 合法 packet。
2. `bbb-protocol` 解码为结构化类型；长度、枚举与 NBT/resource budget 先验证。
3. play packet 按到达顺序交给 `WorldStore::apply_play_packet`。
4. `PlayApplyEffects` 在同一 apply 顺序发出音频、粒子、screen/runtime side effect。
5. `SimulationCore::step` 完成本地 tick、预测与校正。
6. publication 在一致边界生成 world/control snapshot；`RendererFrame` 引用同一 tick、
   resource generation 和相机状态。

## 生命周期和 fallback

handshake → status/login → configuration → play 的阶段转换显式发生；重连建立新 registry
和 world。无效 packet fail closed 或产生明确 disconnect；不认识的动态 holder 不可猜成
另一个 id。缺 renderer resource 只影响 presentation fallback，不修改 canonical world。

远端 terminal packet/transport error 在 `bbb-net` 收敛为一次 bounded `DisconnectReason`，并保持在
所有先行 event 之后；本地主动 close 与裸 relay-channel close 都不会产生该值。simulation 在 input
barrier 前消费它、清空 level/registry/server-pack session 并发布 `ClientInputState` 的断线 screen
generation，control 与 renderer 只读同一 publication。该路径不会再把远端断线改写为进程退出；
用户 back/`app.quit` 才进入正常 shutdown lifecycle。

terminal audio 仍走同一个 `PlayApplyEffects`/native audio FIFO：Login failure 不停止声音，
Configuration/Play 在先行 packet audio 后提交恰好一次 unfiltered stop-all，再进入 terminal
resource apply。phase 决策只属于 simulation；audio backend 不缓存 connection/session 状态，下一份
空 tickable projection 会清除旧 generation 的 entity-bound identity。

## Offline-only 签名边界

Online 模式是明确的非目标（2026-08-05 用户裁决）：客户端不持有 profile key pair、不做
encrypted login/session join、不发 `ChatSessionUpdate`。Vanilla 的
`AccountProfileKeyPairManager`/play-local 签名链对应面按 scope 决策移除，出站聊天与
signable command 参数保持 vanilla「无 key pair」语义（`ChatMessage.signature` 恒 None、
`ArgumentSignatures` 恒空），last-seen ack 管线照常。入站方向不变：远端玩家的
`InitializeChat` session 仍用 Services 公钥集验证（`services_profile_keys` 为无凭据 GET）。
服务端发起 encryption 握手时连接以可归因错误失败（`login.rs#reject_encryption_request`）。
恢复条件与当时被移除的实现清单由唯一 gap ledger 维护。

## BBB 实现映射

`bbb-protocol` 拥有 wire codec；`bbb-net/src/types/connection.rs` 拥有阶段；
`bbb-world/src/play_apply.rs` 的 `WorldStore::apply_play_packet` 与 `PlayApplyEffects` 分开
状态和副作用；`bbb-native/src/simulation.rs` 拥有 simulation；
`bbb-native/src/simulation/publication.rs` 发布 control/world；
`bbb-native/src/runtime/render_extract.rs::RendererFrame` 是 renderer input。

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| phase-specific decode | packet listeners / `GameProtocols` | `bbb-net::ConnectionState`, `bbb-protocol` | `equivalent` | packet coverage 256 closed |
| ordered persistent apply | `ClientPacketListener` → `ClientLevel` | `WorldStore::apply_play_packet` | `equivalent` | play-apply tests |
| transient side effects | listener → renderer/audio/UI | `PlayApplyEffects` | `equivalent` | recording-effects tests |
| simulation publication | tick/render boundary | `SimulationCore`, publication | `equivalent` | snapshot integration tests |
| renderer extraction | `LevelRenderer`/render states | `RendererFrame` | `equivalent` | native renderer-frame tests |
| profile-key manager / play session | `AccountProfileKeyPairManager` / `ClientPacketListener` | offline-only：无 profile-key surface，出站恒 unsigned | `not-needed` | ledger（online 模式非目标）；入站验证见 chat 机制页 |

## 结构性不变量

- network arrival order 不能被多个消费者竞争改变。
- canonical、derived、transient、backend 四类状态不能互相冒充。
- renderer/control snapshot 都是 read-only，且标记 generation/tick。
- dynamic registry 值只在当前 session context 解码。

## 聚焦测试（focused tests）和真实证据

生成 coverage 报告记录全部 256 个 clientbound packet 的关闭证据；protocol、world 与
native focused tests 分别验证 codec、apply 和 publication。共同 replay 的 camera log
进一步证明 player-position 序列到两个客户端一致。

## 当前差异

全部 clientbound packet 已关闭，本机制无未关闭差异。新增 packet 若尚未逐机制取证，
矩阵行应先记为 `unverified` 并链接 ledger，不由本页推断已完成。

## 账本/goal 路由

packet gap 只由 [`unsupported-features.md`](../../../unsupported-features.md) 路由。

## 精确重开条件

目标 protocol 版本或 `GameProtocols` packet 表变化时重做 coverage。

## 相关页面和源码索引

- [总体架构](../../architecture/overview.md)
- [边界与状态分类](../../architecture/client-boundaries.md)
- Vanilla：`GameProtocols`、`ClientPacketListener`、`ClientLevel`、`LevelRenderer`

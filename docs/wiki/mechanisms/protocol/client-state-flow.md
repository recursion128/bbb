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

## 认证 profile key 与 play-local 签名链

Vanilla `ConnectScreen.startConnecting` 在 TCP 建连前调用
`Minecraft.prepareForMultiplayer`，后者把 `AccountProfileKeyPairManager.prepareKeyPair`
串入同一 future chain。manager 独占 credential、pending fetch、accepted key 和一小时 refresh
下限；`ClientPacketListener` 只在 encrypted Play Login 再请求 prepare，并独占当前
`LocalChatSession` 与 advancing message chain。tick 仅在已有 local chat session 且 accepted
key 到期刷新；新 key 才重建 session/encoder 并先发 `ChatSessionUpdate`，同 key 保留链。

BBB 对应的唯一 manager 是 `bbb-net::chat_session::runtime::ProfileKeyManager`。它在
`run_event_stream` 等待 Services public keys 和 TCP 前启动一个 5 秒、128 KiB 上限的请求；
RSA-2048、canonical DER、signature 和 timestamp 校验仍由唯一私有 certificate resolver
负责。Play Login 只把 immutable accepted key 投影为 play-local session；reconfiguration 和
Transfer 清除 session/encoder，不中止 pending fetch，也不丢 manager key。仅当前 encrypted
Play 可以提交 `ChatSessionUpdate`；offline/no-token 不启动 HTTP，失败保留 accepted key，
没有 key 时继续 unsigned。manager drop 才 abort pending task，release 路径不创建
`profilekeys` 私钥缓存。

本地 bounded HTTP/TCP fixtures 覆盖 prepare、refresh、changed/unchanged key、failure
rollback、phase reset 和 update-before-message；真实 offline live session 另验证 no-token
路径不访问 Services。真实 Mojang Services + encrypted online-mode server 验收需要外部授权
凭据 runner，当前状态与重启条件由唯一 gap ledger 维护。

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
| profile-key manager / play session | `AccountProfileKeyPairManager` / `ClientPacketListener` | `bbb-net::chat_session::runtime` | `deferred-external` | bounded HTTP/TCP fixtures；真实凭据见 ledger |

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

主要客户端领域已进入 parity matrix；未逐机制验证项保持 `unverified` 并链接 ledger，
不由本页推断已完成。

## 账本/goal 路由

packet gap 只由 [`unsupported-features.md`](../../../unsupported-features.md) 路由。

## 精确重开条件

目标 protocol 版本或 `GameProtocols` packet 表变化时重做 coverage。

## 相关页面和源码索引

- [总体架构](../../architecture/overview.md)
- [边界与状态分类](../../architecture/client-boundaries.md)
- Vanilla：`GameProtocols`、`ClientPacketListener`、`ClientLevel`、`LevelRenderer`

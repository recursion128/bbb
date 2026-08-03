# 边界与状态分类

## 四类状态

| 类别 | 定义 | 典型 owner | 发布规则 |
| --- | --- | --- | --- |
| canonical state | packet 或本地预测后仍可查询的权威客户端副本 | `bbb-world::WorldStore`、registry/resource generation | 只在所属线程/阶段变更 |
| derived snapshot | 为 control、renderer、诊断提取的只读投影 | `RendererFrame`、`ControlSnapshot` | 在一致边界原子替换，不回写 world |
| transient observed event | 声音、粒子、一次性 UI 等有序副作用 | `PlayApplyEffects` 实现 | exactly-once 顺序消费，不伪装成字段 |
| backend runtime state | socket、GPU buffer、audio voice、window handle | net/renderer/audio/native runtime | 后端生命周期内有效，重建不改变游戏真值 |

## 所有权规则

- 一项长期事实只有一个 semantic owner。消费者持有 id、handle 或 projection，不维护
  可独立漂移的副本。
- protocol 类型描述 wire；world 类型描述客户端语义。不能把原始 packet 当持久世界。
- renderer input 必须携带 Vanilla 决策需要的独立事实。不能从
  `Opaque/Cutout/Translucent`、方块名称或 render shape 猜测 `canOcclude`、
  `isSolidRender`、`isViewBlocking`、光衰减、shade、shape、`skipRendering` 或 fluid
  overlay。
- reload 先构造完整 generation，再原子发布；失败时旧 generation 仍可用。
- 本地预测必须可被服务端 packet 校正；诊断 snapshot 不得成为控制输入。

## 本地输入与 LocalPlayer sprint owner

原始按键与玩家语义状态不能共用一个字段。`bbb-native::ClientInputState` 只保存
`sprint_key_down`、双击前进 edge/timer 和最后成功发送的 sprint baseline；
`bbb-world::LocalPlayerState` 唯一保存 canonical sprint latch，并按 Vanilla
`canStartSprinting`、`shouldStopRunSprinting`、`shouldStopSwimSprinting` 处理 food、mobility、
vehicle、water、pose、collision 与 attack。movement、vehicle、combat 和 camera/FOV 只读取
world latch，不反向读取原始按键。

raw `ServerboundPlayerInputPacket` 只投影按键位；双击前进不会伪造其 sprint bit，松开 sprint
键也不会单独清除已经启动的 latch。非乘客 tick 先发送 latch transition，再发送 player
movement；乘客 tick 保持 rotation、authoritative vehicle movement、latch transition 顺序。
level generation 变化时丢弃旧 edge/timer 并把 outbound baseline 对齐新 world；队列发送失败
不推进 baseline，所以下一 tick 会重试而不形成 shadow state。

## 生命周期

启动创建平台与资源基础，连接依次经过 handshake、configuration、play。configuration
建立 registry/data projection；play 建立 world。resource reload 可跨 play 独立发生，
但一帧 renderer input 必须引用同一 resource generation。断线先停止新 packet 应用，
再清理 world/backend；control 只能看到最后一个明确标记状态的 snapshot。

## External network backend terminal close

`bbb-native::network_runtime::NetworkRuntime` 是 external connection future 和 event-relay
task 的唯一 lifecycle owner。startup 只构造 `ConnectionOptions`；simulation 取得有界 net
event receiver 与 command sender，既看不到 task handle，也不拥有 terminal close state。relay
保持 connection event 顺序，并在每次成功交付后唤醒 simulation；simulation receiver 关闭时立即
停止，不把旧 session event 留给下一进程。

gameplay `NetCommand::Disconnect` 仍由 simulation 按正常 session 语义发送。terminal close 另有
独立 one-shot signal，所以 command queue 已满、已关闭或 simulation fatal 都不能阻止 connection
future 被取消并通过 drop 关闭 socket。连接自然结束、服务器断开或 terminal signal 最多产生一个
最终 `Disconnected` event；重复 shutdown 幂等。

`LoopExiting` 的固定顺序是：释放 cursor capture、请求并 join simulation、signal 并 join network
task pair、发布最终 `running=false/connected=false` snapshot，随后 event-loop closure 才释放
renderer/audio/window 和 Tokio runtime。两个 network task 共用 5 秒总 deadline；task panic/join
failure 进入既有 `RuntimeFailure`/crash-report owner，超时会 abort 并 reap 剩余 task 后同样报告。
无连接、正常远端结束和成功 close 不产 fatal report。`JoinSet` drop 只负责 event-loop 建立前发生
startup failure 时的 abort safety net，不能替代 production `shutdown_and_join`；focused source guard
锁定唯一调用点及 simulation→network→snapshot 顺序。

<a id="profile-texture-trust-and-presentation"></a>

## Profile texture 信任与 presentation

`bbb-item-model::skin_runtime` 是 profile property、默认皮肤、skin/cape/elytra 集合和远端
PNG CPU 生命周期的唯一 semantic owner。net 在连接前 bounded prepare Services 的
`PROFILE_PROPERTY` key；world 只保存不可变 profile projection；native 传入“本地/远端”查询坐标，
drain 已解码图像并在 render thread 回报 GPU commit；renderer 只拥有 atlas、handle 和当前
presentation generation，不反向读取 profile、HTTP 或磁盘 cache。

lookup key 使用 UUID 与完整 packed `textures` property。带签名 property 按 SHA-1/RSA 和 bounded
Services key set 验证；远端 `PlayerInfo` 的 unsigned/invalid 集合直接使用 UUID 默认皮肤且不发
HTTP，本地玩家与 profile item 可使用 insecure 集合。URL 只接受 `http|https` 和精确
`textures.minecraft.net`，按 authlib basename 复用 type-specific texture，完整 URL 冲突则失败；
磁盘 identity 使用 Guava `hashUnencodedChars` 等价 SHA-1。skin、cape、elytra 必须全部成功后才发布，
任一 URL/download/decode/upload 失败都会使整个 base set 保持默认 fallback；pack-backed patch 仍从
同一 immutable resource generation 查询并遵守 priority/filter。

profile-result access cache 的 TTL 是 15 秒；bounded texture entry、decoded PNG 和磁盘 identity 属于
进程生命周期。disconnect、transfer 或 resource generation 边沿只失效 renderer projection，保留的
CPU 图像在新 generation 重交付而不重复 HTTP。resource reload 先完成所有 CPU/GPU/audio fallible
prepare/commit，再把 process owner 移入新 `NativeItemRuntime`；失败时旧 generation 和其 texture owner
完全不动。control counters 同时报告 profile-set/asset/key bytes 与 async queue/resident/delivery bytes；
source guard 禁止恢复 URL-only skin cache、分离 cape/elytra resolver 或 generation-keyed CPU shadow
cache。

## Fatal failure 与 crash report

`bbb-native::crash_report` 是 fatal diagnostics 的唯一 owner。startup `Result`、main panic、
simulation panic/join error，以及 render-effect、resize、frame 和 platform event-loop fatal
只提交 `RuntimeFailure`；首个 failure 保持 canonical，最多 8 个后续 failure 作为 suppressed
diagnostics。worker、renderer、control 或 snapshot 都不直接写报告。

报告复刻 Vanilla `CrashReport`/`Minecraft.crash` 的 header、UTC time、description、error、
head/thread/backtrace、category、system details 和 save-once 判断。单字段、category/entry 数、
suppressed 数、retained bytes（512 KiB）和最终 UTF-8 文本（1 MiB）都有明确预算，Unicode
截断会写入诊断。目录只来自启动参数 `--crash-report-dir`，默认是 game-directory 语义下的
`crash-reports`；安全时间戳文件名不含 server/user 输入，临时文件 sync 后用同目录 hard-link
原子 no-clobber 发布，重名追加序号。

fatal event 先请求 event-loop exit；`LoopExiting` join simulation，closure drop 释放
renderer/audio/window，`event_loop.run` 返回后外层 runtime 才持久化。保存成功退出 1，写失败
仍把 bounded report 输出 stderr 并退出 2；正常 close/quit、probe/status/build-info 成功不产报告。
focused process test 与 production-binary acceptance 都验证“失败生成一份且非零退出，下一进程
成功且不覆盖旧报告”。

## 测试责任

protocol 测 byte/类型边界，world 测 ordered apply 与 canonical state，native 测
publication/consumer wiring，renderer/audio 测 backend 输入合同。跨边界测试证明同一
id、generation、tick 与顺序，而不是重复每层的内部测试。

<a id="mojang-client-telemetry"></a>

## Mojang 客户端遥测边界

BBB 不收集或上传 Mojang client telemetry。Vanilla 26.1 的
`WorldSessionTelemetryManager` 会把 server brand 写入 `WorldLoadEvent` 并尝试通过独立
event sender 发送；这不是 multiplayer protocol canonical state。BBB 仍可把对应 wire
字段解码为诊断事实，但不得为它创建外部 telemetry runtime owner。只有项目明确把该
外部服务纳入产品表面时才重新审计；当前裁决由
[缺口账本](../../unsupported-features.md)唯一维护。

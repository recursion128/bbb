# 客户端连接、协议阶段与注册表

机制 ID：`registries-synchronized-data`

## 机制目的和边界

本页拥有 socket bytes 到连接事件、configuration registry 到可查询 holder 的交接。
客户端连接任意声明范围内的外部 offline server；BBB 服务端的反向路径见
[服务端连接](server-connections-and-sessions.md)。两端共享 wire 类型，分别拥有状态机。
world 的 play 应用见[协议状态流](client-state-flow.md)，会话替换与线程退出见
[运行时](../runtime/client-runtime-and-control.md)。

## Vanilla 26.1 权威实现

- `ClientHandshakePacketListenerImpl.java:171-202`：LoginFinished 建 configuration listener，
  发送 login acknowledgement，再切出站协议并发送 brand/client information。
- `ClientHandshakePacketListenerImpl.java:225-229`：收到 compression 后切换解压配置。
- `ClientConfigurationPacketListenerImpl.java:80-105`：先收集 registry/tags，并回答 known-pack 选择。
- `RegistryDataCollector.java:156-173`：有 contents 时加载新 registry，否则只更新已有 tags；
  freeze 后再更新 components。

packet id 与字段由 `GameProtocols`、对应 phase protocols 和 packet codec 拥有，不能把同一
数字当成跨 phase 的同一消息。协议表来源与资产版本的固定方式见[离线资产](../resources/client-offline-asset-trust.md)。

## 决策输入与状态所有权

| 输入 | owner | 下游 |
| --- | --- | --- |
| TCP buffer、压缩阈值、连接写入 | `RawConnection` | 已分帧 packet id/payload |
| phase、hello latch、bundle、cookie carry | `bbb-client-net` | ordered `NetEvent` |
| packet codec、NBT/Component、固定 type/catalog | `bbb-protocol` / `bbb-facts` | typed packet 与固定身份 |
| 同步 registry 到达顺序、tags、typed candidate | world `RegistrySet` | 一个 registry revision |
| known-pack 原始元素与 resource provider | pack / registry codec | 经协商才可补齐的 stub 数据 |
| registry-dependent audio、item 与环境状态 | 各 world/native consumer | 同代派生结果 |

固定 block/item/entity/component 身份与服务端同步 holder 是两种来源。
动态 holder 的索引由本 session registry 的顺序决定，不能用内置表碰巧相同的索引替代。
`bbb-client-world` 归一 codec identifier，`bbb-registry-codec` 解释元素；renderer 不解码 registry。

`RegistryElementData` 只持有一种输入树：网络 NBT 或保留数字词法的 JSON。源 codec 借用
这棵树读取字段；常驻数据不再附带规范化 NBT 副本。配置提交时构建同代强类型缓存，
绘制、玩法与 test instance block 查询复用缓存；冷路径沿用共享 exact codec。
Dialog 诊断需要旧标量形状时，临时调用 `source_order_nbt_projection`，保留源字段序，
JSON 投影借用源树，不先克隆整棵树。

GameTest 的客户端定义直接消费共享 codec 的只读强类型视图，按当前候选解析 holder 的
编号与来源，并在生成缓存前校验引用成员。内联环境继承元素来源，命名环境保持引用。
Dialog 的每次 exact 解码生成一份 canonical 值供网络投影和引用查询使用；CustomData
的 compound 与其它动态 NBT 共用有界遍历。

## 判断与处理顺序

### 帧、压缩与 phase

1. `try_read_frame` 检查 VarInt 长度；不完整帧留在 buffer 等待，零长度、负数及超上限拒绝。
2. 压缩启用时读取声明解压长度，先验证逻辑预算，再以声明长度加一限制 inflate，最后核对长度。
   未压缩 frame 与压缩后的逻辑 body 有各自上限，不能只限制 socket buffer。
3. 根据当前 phase 解码 typed packet；字段长度、enum、NBT 和 Component 预算由 codec 拒绝。
4. Login hello 每连接只发送一次；offline 身份不进入在线加密/session join，收到 encryption
   request 明确失败。其产品范围和聊天签名区别见[聊天与身份](../security/client-chat-and-identity.md)。
5. LoginFinished 后先完成 acknowledgement 和配置端初始化，后续数据按 configuration 路径消费。
   压缩切换属于 transport 边沿，不由 world 自行判断。
6. play bundle 在 closing delimiter 前保留消息，到齐后提交一个有序 batch；最多 4096 个 packet，
   中途 StartConfiguration 被拒绝。delimiter 不作为游戏事件下发。

未知 packet 的结果取具体 phase decoder/dispatcher：有些会形成 UnsupportedPacket 诊断，
非法结构或顺序会拒绝连接；不能把“可识别 ID”“成功 decode”和“已有 world 行为”混为一谈。

### Configuration candidate 与提交屏障

1. 新 configuration 建 pending collection，registry packet 按到达顺序收集 entry。
2. known-pack 回答只选择本地可验证的交集；省略数据的 entry 必须有已选择 pack 可供物化。
3. tags 可以先到，但解析要等待完整 registry；同步和固定 registry 分层解析。
4. 构造 `RegistryRevisionCandidate`，验证重复 key、non-empty 要求、元素 codec、holder/tag 引用，
   同时准备依赖该 revision 的 typed 值。候选失败不覆盖当前已提交的 registry 和派生 item 状态。
5. 收到 FinishConfiguration 先发布事件给 simulation；
   `runtime::events::dispatcher` 调用 `finish_registry_configuration`，成功后同步 audio registry
   并排 `NetCommand::FinishConfiguration`，失败排 Disconnect。
6. net 收到成功命令才发送 finish response、进入 Play 并发出 phase 变化；收包线程不绕过
   world 校验自行放行。运行时 barrier 仍保持同一 command/effect 顺序。

known-pack 的 registry JSON 与资源读取共用保留 UTF-16 的 parser，并保存原始 bytes；
文件、树和 generation 预算各收费一次，超限仍拒绝整个候选。score 的 name/objective 从
源 codec、NBT 到公开 canonical writer 保留原始 JavaString；typed description 的文本展示可
替换孤立 surrogate，规范 NBT 不替换。实际网络 packet、服务器 JSON 和本地 known-pack
文件的发布、坏 required 字段隔离及合法恢复见 `registries::tests::component_score`。 Component
click action 和 Dialog 字符串的三来源规范 NBT 发布验证见 `registries::tests::component_strings`；
同一三来源数据通过 Reference ShowDialog 打开 typed controls 并初始化 raw editor；
boolean substitution、single-option id 和 text raw 值经 runtime 动态模板/CustomClickAction
保留原单元。单行 cursor 按代码点移动，多行按 UTF-16 单元移动，插入和删除直接修改 raw 单元；
HUD 的 pair 内部光标/prefix 与 IME 几何经单位坐标投影消费，机制见
[屏幕与 HUD](../ui/client-screens-hud-and-inventory.md)。规范源发布不替代实际绘制验收。

网络 tags 中无法解析的 holder 不暴露给已检查视图；world 单独保留 unresolved 诊断。
`registries::state` 的固定 raw/resource 双向查询与 availability 共用
`bbb-facts::static_registry_elements` 的 95 个 STATIC registry、6812 个元素；目录来自官方
`RegistryDumpReport.java:37-45` 的真实 raw IDs，并以 CST 校验 registry 身份、顺序和默认键。
loader-ordered 动态 fallback 仍先于 STATIC 查询；已有 published contents 即使为空，miss
也不回退。Builtin membership 不接受伪造固定元素。负值和超界 raw ID 保留 unresolved，
成功 tag 成员的顺序与重复保持入包值；configuration pending tags 仍随完整 candidate 原子提交。
生成的内置表服务固定身份与允许的默认来源，不向缺少协商数据的动态 registry 静默兜底。
共享 registry codec 的错误显式区分 Native 语义拒绝与预算、coverage、resolver 或内部
合同失败；共享 speculative 分支只为被拒绝的语义分支回滚计费，终止错误保留计费。
Component 的 lenient separator、模糊内容/对象选择和 profile 备选 reader 直接传播原始
终止错误及其字段归属；普通 malformed separator 仍按 Native lenient optional 合同省略。
Native 的语义 fallback 不授权终止错误进入默认值或 partial。实际 configuration 回归覆盖
known-pack JSON 与网络 NBT 的浅层多列表 node budget、内嵌 SNBT 字符串的 parser bound
拒绝、当前 revision 保留及后续合法候选恢复；SNBT 语法错误仍为 Native 语义拒绝，入口在 `bbb_client_world::registries::tests::codec_terminal`。
无范围 `Codec.FLOAT`/`Codec.DOUBLE` 字段复用 numeric primitive token，接受合法 NBT 的
NaN 与正负 Infinity：attribute modifier amount、custom model floats
(`CustomModelData.java:12-22`)、food saturation (`FoodProperties.java:20-28`)、map
coordinates/rotation (`MapDecorations.java:10-30`)、tool 默认速度 (`Tool.java:18-25`)、
blocks attacks 的 base/factor (`BlocksAttacks.java:138-145`、`BlocksAttacks.java:171-177`)、
kinetic weapon 及其 conditions (`KineticWeapon.java:42-54`、`KineticWeapon.java:149-155`)、
内联 sound range (`SoundEvent.java:16-20`) 和 predicate double bounds
(`MinMaxBounds.java:201-210`)。各字段原有 range gate、boxed bounds 顺序和等点压缩继续
由对应 source owner 执行；sound 的 malformed range 仍按 lenient optional 合同省略。
网络 NBT 读取先 canonicalize signed zero；JSON 保留源数值的零符号供 boxed comparison
使用，输出 NBT 再归一化为正零。

固定 map decoration 成员来自 `BuiltInRegistries.java:306` 的 generated catalog，
`registries::state` 使用它进行 holder/resource 正反查；RegistryData packet 不能创建固定
成员。实际 configuration candidate 经 trim description 的 hover item 发布 typed
presentation；数值、默认省略、完整 map catalog 和非法成员拒绝、坏 required field 的旧
revision 保留及后续候选恢复，入口在
`bbb_client_world::registries::tests::component_numeric_families`；attribute amount 的
独立消费用例保留在 `bbb_client_world::registries::tests::component_numeric`。

## 生命周期和 fallback

原生客户端接受主机名、IPv4、IPv6 和带端口的地址。省略端口时使用 25565。默认端口上的连接会查询 `_minecraft._tcp` SRV 记录，显式指定其它端口时直连。状态查询、登录和转服共用 TCP 目标解析。

新连接清空旧 transport phase、已发 hello 和 carried-slot observation，并建立新的 registry session。
play→configuration、Transfer、普通 terminal close 的 carry 规则由运行时页面拥有；cookie、
server pack 和 CommonListenerCookie 不是一概随 player respawn 清空。

资源包 push 的响应策略、下载身份和异步 generation 校验归 pack/runtime，不让网络包接收成功
直接表示资源已经应用。compression、bundle 和 event queue 均属于当前连接，旧连接的半包或
半 bundle 不进入新 session。关闭与背压沿有界 channel 和统一 NetworkRuntime owner 收敛。

初次收到服务器资源包时，桌面和 Web 会显示服务器说明。接受后客户端下载资源包；拒绝可选包可继续游玩，拒绝必需包会断开连接。启用服务器列表存储时，客户端按服务器地址记录选择。启动参数 `--server-resource-pack-policy` 或 Web 查询参数 `server_resource_pack_policy` 可以显式指定 `prompt`、`accept` 或 `decline`。

## BBB 实现映射

| owner | 生产路径 | 边界 |
| --- | --- | --- |
| frame | `bbb_protocol::frame` | 分帧、压缩与逻辑长度 |
| transport | `bbb_client_net::connection` | TCP、写顺序、压缩配置 |
| phase/event | `bbb_client_net::event_stream`、`event_stream::configuration` | typed event 和 finish response |
| bundle | `bbb_client_net::play_bundle` | 有序 batch、数量与 terminal 限制 |
| registries | `bbb_client_world::registries::state`、`registries::revision` | pending 与 committed revision |
| finish consumer | `bbb_client::runtime::events::dispatcher` | 校验后才排网络 response |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| phase switch | handshake/configuration listeners | event stream | `aligned` | loopback、phase tests |
| registry freeze | `RegistryDataCollector` | revision candidate | `aligned` | atomic rejection 与 known-pack tests |
| ordered bundle | packet bundler | play bundle assembler | `aligned` | delimiter、上限和 terminal tests |
| wire 输入预算 | frame / stream codecs | protocol frame 与 packet codecs | `aligned` | framing 和压缩拒绝测试 |

## 结构性不变量

- packet id 必须与 phase/direction 绑定，codec 不拥有 world 状态。
- 同步 holder、tag 与 typed projection 属于同一 registry revision。
- Finish response 必须发生在 candidate 成功之后；失败不保留半份新 registry。
- 内置身份、known-pack 数据和远端 registry 不互相冒充权威来源。

## 聚焦测试（focused tests）和真实证据

`bbb_client_world::registries::tests::configuration`：
`configuration_registry_validation_rejects_empty_non_empty_registry_atomically`、
`configuration_tags_resolve_after_all_registry_packets_arrive`、
`selected_known_pack_materializes_stub_data_before_registry_commit`、
`omitted_registry_data_without_a_selected_known_pack_fails_closed`。

`bbb_client_net::play_bundle`：
`bundle_holds_packets_until_closing_delimiter_and_preserves_order`、
`terminal_packet_is_rejected_only_inside_bundle`。
`bbb_protocol::frame` 的
`compressed_frame_rejects_oversized_declared_output_before_inflate` 覆盖分配前预算。

按改动运行 `cargo dev test -p bbb-client-world registries::tests::configuration`、
`cargo dev test -p bbb-client-net play_bundle` 或
`cargo dev test -p bbb-protocol frame::tests`。
这些证明 phase/candidate 合同；packet consumer 是否完整仍由独立 route 与 runtime evidence 判定。

## 当前差异

online authentication 是产品范围外项。各 phase 对未知 packet 的实际诊断/拒绝行为，以及
registry consumer 的未关闭差异，以[客户端账本](../../../unsupported-client-features.md)为准。

## 账本归属

本页集中解释 protocol、registry 机制；主 ID 为 `registries-synchronized-data`，相邻机制
保留原 ID。推进原则见 [AGENTS.md](../../../../AGENTS.md)。

## 精确重开条件

版本、packet catalog、phase 边沿、compression、bundle、registry loader 顺序或 finish barrier
变化时重验；出现旧 holder 跨 session 泄漏、无已选 pack 却物化 stub、候选拒绝后新 typed 值
仍可见等情况，按失败 owner 定位。

## 相关页面和源码索引

- [协议状态流](client-state-flow.md)：play apply 与 publication。
- [运行时](../runtime/client-runtime-and-control.md)：重配置、transfer 与 terminal close。
- [服务端数据包](../resources/server-datapack-and-loot.md)：同步数据的服务端来源。

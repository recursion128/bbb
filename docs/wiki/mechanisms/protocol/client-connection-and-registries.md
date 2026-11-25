# 客户端连接、协议阶段与注册表

机制 ID：`registries-synchronized-data`

## 机制目的和边界

本页定义从原生 TCP 或浏览器二进制 WebSocket 字节流到有序连接事件、以及从 Configuration 阶段同步注册表到可查询 holder 的完整交接契约。客户端连接任意声明范围内的外部仅离线模式多人服务器；BBB 服务端的反向路径见[服务端连接与会话](server-connections-and-sessions.md)。两端共享协议线格式类型，分别拥有各自的状态机。Play 阶段数据包在世界层的应用见[客户端协议状态流](client-state-flow.md)，会话替换与网络线程退出见[客户端运行时](../runtime/client-runtime-and-control.md)。

## Vanilla 26.1 权威实现

- `ClientHandshakePacketListenerImpl.java:171-202`：收到 `LoginFinished` 后构造配置阶段监听器、发送登录确认，再切换出站协议阶段并发送客户端品牌（brand）与 `ClientInformation`。
- `ClientHandshakePacketListenerImpl.java:225-229`：收到压缩阈值包后切换传输层解压配置。
- `ClientConfigurationPacketListenerImpl.java:80-105`：按序收集同步注册表与标签，并回应 known-pack 选择。
- `RegistryDataCollector.java:156-173`：携带元素内容时构建新注册表，否则仅更新已有标签；冻结注册表后再更新组件原型。

数据包 ID 与字段定义由 `GameProtocols`、各阶段协议定义与编解码器共同决定，不得将同一数值当作跨阶段的同一消息。协议表来源与官方资产版本锁定机制见[离线资产可信链](../resources/client-offline-asset-trust.md)。

## 决策输入与状态所有权

| 输入/状态 | 唯一所有者 | 下游输出 |
| --- | --- | --- |
| TCP / WebSocket 缓冲、压缩阈值与连接写入 | `bbb_client_net::transport::ClientStream`（原生 `RawConnection` / Web `net_worker.rs` + `bbb-web-host` `/ws` 桥） | 已分帧的数据包 ID 与载荷 |
| 协议阶段、hello 锁存、bundle 与 cookie 携带 | `bbb-client-net` | 有序 `NetEvent` |
| 数据包编解码、NBT/组件与固定目录 | `bbb-protocol` / `bbb-facts` | 强类型数据包与固定身份 |
| 同步注册表到达顺序、标签与强类型候选 | 世界层 `RegistrySet` | 原子提交的注册表代际（revision） |
| known-pack 原始元素与资源提供者 | 资源包 / `bbb-registry-codec` | 仅经协商才可物化的省略条目数据 |
| 依赖注册表的音频、物品与环境状态 | 世界层与客户端各消费者 | 同代际派生状态 |

固定方块/物品/实体/组件身份与服务端动态同步 holder 来自两种不同来源：动态 holder 的数字索引完全由当前会话注册表的到达顺序决定，不得用内置表中碰巧相同的索引替代。`bbb-client-world` 归一化编解码标识符，`bbb-registry-codec` 负责元素解码；渲染器不解码任何注册表。

`RegistryElementData` 仅持有一棵原始输入树：网络 NBT 或保留数字原始词法的 JSON。源编解码器直接借用该树读取字段，常驻数据不再附带规范化 NBT 副本。配置提交时构建同代际强类型缓存，渲染、玩法与测试实例方块查询复用缓存；冷路径沿用共享精确编解码器。对话框诊断需要旧标量形状时，临时调用 `source_order_nbt_projection` 保留源字段顺序，JSON 投影借用源树而不预先克隆整棵树。

GameTest 的客户端定义直接消费共享编解码器的只读强类型视图，按当前候选解析 holder 编号与来源，并在生成缓存前校验引用成员。内联环境继承元素来源，命名环境保持引用。对话框每次精确解码产出规范值供网络投影与引用查询复用；`CustomData` 复合标签与其他动态 NBT 共用有界遍历。

## 判断与处理顺序

### 分帧、压缩与协议阶段切换

1. `try_read_frame` 检查 VarInt 帧长；半包留在缓冲区等待，零长度、负数及超预算长度立即拒绝。
2. 启用压缩时读取声明解压长度，先核对逻辑字节预算，再以「声明长度 + 1」限制 inflate 输出，最后校验实际长度。未压缩帧与解压后逻辑正文分别拥有独立上限，不能只限制套接字缓冲区。
3. 按当前协议阶段解码为强类型数据包；字符串长度、枚举、NBT 与组件安全预算在编解码入口统一拒绝。
4. 每条连接仅发送一次 Login hello；离线身份不参与在线加密与会话登录，收到服务端加密请求时立即以明确错误失败。其产品范围与聊天签名的区别见[客户端聊天与身份](../security/client-chat-and-identity.md)。
5. `LoginFinished` 后先完成确认并初始化配置端，后续数据按 Configuration 路径消费。压缩阈值切换属于传输层边沿，不由世界层自行推断。
6. Play 阶段的 bundle 在结束分隔符前暂存包序列，到齐后作为单个有序批次提交；单批上限 4,096 个数据包，中途收到 `StartConfiguration` 直接拒绝。分隔符本身不作为游戏事件下发。

未知数据包的处理结果取决于具体阶段的解码与分派规则：部分形成 `UnsupportedPacket` 诊断，非法结构或阶段错序则断开连接；不得将「可识别 ID」「成功解码」与「具备世界消费行为」混为一谈。

### 配置期候选与提交屏障

1. 进入新 Configuration 阶段时创建待提交集合，注册表数据包按到达顺序记录条目。
2. known-pack 应答仅选择本地可验证的交集；服务端省略数据的条目必须存在对应已选 pack 方可物化。
3. 标签包可先于注册表到达，但解析必须等待完整注册表齐备；动态同步注册表与固定静态注册表分层解析。
4. 构造 `RegistryRevisionCandidate`，校验重复键、非空约束、元素编解码与 holder/标签引用，并同步准备依赖该代际的强类型投影。候选校验失败时绝不覆盖当前已提交的注册表和派生状态。
5. 收到 `FinishConfiguration` 后先向模拟层发布事件；`runtime::events::dispatcher` 调用 `finish_registry_configuration`，校验成功后同步音频注册表并排队 `NetCommand::FinishConfiguration`，失败则排队断开连接。若服务端在配置期下发自定义载荷 `bbb:content_registry`（启用 `--mod` / `--mod-packs` 时），世界层同步冻结当次会话的只读 `ContentRegistry` 快照（详见[客户端边界](../../architecture/client-boundaries.md)）。
6. 网络层仅在收到成功命令后才发回 `FinishConfiguration` 响应、切入 Play 阶段并发出阶段切换事件；收包线程不得绕过世界层校验自行放行。

- **原始 UTF-16 与三来源一致发布**：known-pack 注册表 JSON 与网络 NBT 共用保留原始 UTF-16 单元的解析器，文件、树节点与代际预算各计费一次。`score` 组件（`name`/`objective`）、组件点击载荷与对话框字符串在规范 NBT 输出与运行时动态模板/`CustomClickAction` 中均保持原始 `JavaString` 单元，仅在展示侧按需替换孤立代理项（光标与 IME 消费见[屏幕、HUD 与库存](../ui/client-screens-hud-and-inventory.md)）。
- **静态注册表与动态回退隔离**：`registries::state` 的固定原始 ID/资源名查询共用 `bbb-facts::static_registry_elements`（95 个静态注册表、6,812 个元素，源自 `RegistryDumpReport.java:37-45`）与 `BuiltInRegistries.java:306` 地图装饰目录；`RegistryData` 不能新增固定成员，一旦某注册表已发布内容（即便为空），未命中不回退静态表。未解析标签 holder 单独记入诊断，成功成员保留入包顺序与重复项。
- **终止错误 vs 语义拒绝**：注册表编解码器显式区分 Vanilla 语义拒绝与安全预算/解析器/内部契约等终止错误；试探性分支仅对语义拒绝回滚预算，终止错误直接向上传播且不被宽容可选回退吞没。
- **无界浮点与零符号规则**：无范围约束的 `Codec.FLOAT`/`Codec.DOUBLE` 字段（属性修饰符、`CustomModelData.java:12-22`、`FoodProperties.java:20-28`、`MapDecorations.java:10-30`、`Tool.java:18-25`、`BlocksAttacks.java:138-145` / `:171-177`、`KineticWeapon.java:42-54` / `:149-155`、`SoundEvent.java:16-20`、`MinMaxBounds.java:201-210`）接受合法 NBT 中的 `NaN` 与正负 `Infinity`；网络 NBT 读取及写出时归一化带符号零，JSON 保留原零符号供装箱比较。

## 生命周期和 fallback

原生客户端接受主机名、IPv4、IPv6 及带端口地址，省略端口时默认 `25565` 并查询 `_minecraft._tcp` SRV 记录（显式端口直连）；状态查询、登录与转服共用同一解析路径。在 `wasm32` Web 客户端上，网络连接运行在独立的共享内存 Web Worker（`crates/bbb-client/src/web/net_worker.rs`）中，通过 `bbb-web-host` 的白名单 `/ws` 二进制桥接接入上游 TCP 服务端，并与桌面端复用同一 `ClientStream` 与协议状态机。

建立新连接时清空旧传输阶段、已发 hello 标记与手持槽位观测，并初始化新的注册表会话。Play→Configuration 重配置、Transfer 转服与普通终态断开的跨会话携带规则由[客户端运行时](../runtime/client-runtime-and-control.md)拥有；cookie、服务器资源包与 `CommonListenerCookie` 并非一律随玩家重生清空。

服务器资源包推送的应答策略、下载身份与异步代际校验归属资源包与运行时，收到推送包并不意味着资源已经应用。压缩状态、bundle 缓冲与事件队列均绑定当前连接，旧连接的半包或半截 bundle 绝不渗入新会话。连接关闭与背压沿有界通道和统一 `NetworkRuntime` 收敛。

初次收到服务器资源包时，桌面端与 Web 端均会展示服务器提示说明。玩家接受后下载资源包；拒绝可选包可继续游玩，拒绝必需包则断开连接。启用服务器列表存储时，客户端按服务器地址记忆选择。可通过启动参数 `--server-resource-pack-policy` 或 Web 查询参数 `server_resource_pack_policy` 显式指定 `prompt`、`accept` 或 `decline`。

## BBB 实现映射

| 语义所有者 | 生产路径 | 边界职责 |
| --- | --- | --- |
| 帧编解码 | `bbb_protocol::frame` | 分帧、压缩与逻辑长度预算 |
| 传输连接 | `bbb_client_net::transport`、`bbb_client_net::connection`、`crates/bbb-client/src/web/net_worker.rs` | 原生 TCP 与 Web 二进制 WebSocket 读写顺序及压缩切换 |
| 阶段与事件流 | `bbb_client_net::event_stream`、`event_stream::configuration` | 强类型连接事件与配置完成响应 |
| 批次打包器 | `bbb_client_net::play_bundle` | 有序批次组装、数量与终态包限制 |
| 注册表状态 | `bbb_client_world::registries::state`、`registries::revision` | 待提交候选与已提交原子代际 |
| 完成屏障消费者 | `bbb_client::runtime::events::dispatcher` | 校验提交后才排队网络确认 |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 协议阶段切换 | 握手与配置阶段监听器 | `bbb_client_net::event_stream` | `aligned` | 本地回环与阶段切换测试 |
| 注册表冻结与提交 | `RegistryDataCollector` | `registries::revision` 候选提交 | `aligned` | 原子拒绝与 known-pack 物化测试 |
| 有序包批次（bundle） | 数据包打包器 | `bbb_client_net::play_bundle` | `aligned` | 分隔符、上限与终态包拒绝测试 |
| 协议线格式安全预算 | 分帧与流编解码器 | `bbb_protocol::frame` 与包编解码器 | `aligned` | 分帧与解压前预算拒绝测试 |

## 结构性不变量

- 数据包 ID 必须严格绑定协议阶段与方向，编解码器不持有世界状态。
- 动态同步 holder、标签与强类型派生投影必须属于同一注册表代际。
- `FinishConfiguration` 响应必须排在候选原子提交成功之后；提交失败不得残留半份新注册表。
- 内置静态身份、known-pack 协商数据与服务端动态注册表三者不得互相冒充权威来源。

## 聚焦测试（focused tests）和真实证据

`bbb_client_world::registries::tests`：

- `configuration`：`configuration_registry_validation_rejects_empty_non_empty_registry_atomically`、`configuration_tags_resolve_after_all_registry_packets_arrive`、`selected_known_pack_materializes_stub_data_before_registry_commit`、`omitted_registry_data_without_a_selected_known_pack_fails_closed`
- `component_score` / `component_strings`：网络包、服务端 JSON 与 known-pack 三来源 UTF-16 规范 NBT 发布与坏字段隔离
- `codec_terminal` / `component_numeric_families` / `component_numeric`：终止错误预算拦截、无界浮点与地图装饰成员校验

`bbb_client_net::play_bundle`：

- `bundle_holds_packets_until_closing_delimiter_and_preserves_order`
- `terminal_packet_is_rejected_only_inside_bundle`

`bbb_protocol::frame`：

- `compressed_frame_rejects_oversized_declared_output_before_inflate`（覆盖分配前解压预算拦截）

按改动范围运行 `cargo dev test -p bbb-client-world registries::tests::configuration`、`cargo dev test -p bbb-client-net play_bundle` 或 `cargo dev test -p bbb-protocol frame::tests`。上述用例验证阶段与候选提交契约，具体数据包消费者行为仍由各自机制的聚焦测试与运行时回放覆盖。

## 当前差异

在线账号加密认证属于明确的产品范围外项（已作为 `# Not needed` 登记于账本）。[客户端账本](../../../unsupported-client-features.md)当前无未关闭的 `# Todo` 缺口；若后续发现协议阶段、未知包处理或注册表消费者差异，统一登记至该账本。

## 账本归属

主机制 ID 为 `registries-synchronized-data`。范围裁决与未关闭差异统一以[客户端账本](../../../unsupported-client-features.md)为准，推进原则见 [AGENTS.md](../../../../AGENTS.md)。

## 精确重开条件

当目标协议版本、数据包目录、阶段切换边沿、压缩规则、bundle 限制、注册表加载顺序或完成屏障发生变化时，须重新核验。若出现旧 holder 跨会话泄漏、未协商 known-pack 却物化省略条目，或候选拒绝后新强类型值仍可见，按对应所有者定位修复。

## 相关页面和源码索引

- [客户端协议状态流](client-state-flow.md)：Play 阶段应用与快照发布。
- [客户端运行时](../runtime/client-runtime-and-control.md)：重配置、转服与终态断开。
- [服务端数据包与战利品](../resources/server-datapack-and-loot.md)：同步注册表数据的服务端来源。

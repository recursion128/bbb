# 客户端聊天与身份：签名、显示和皮肤信任

机制 ID：`security-secure-chat-reporting`

## 机制目的和边界

本页区分聊天协议验证、可见消息、社交过滤以及 profile texture 信任。
BBB 客户端按 offline-only 范围连接外部多人服务器，没有本地 profile key pair。
出站 unsigned 不代表可以跳过入站 session/signature 验证，也不取消 last-seen 确认。
皮肤资源信任与聊天身份签名使用不同的 key family 和消息格式，不能混为一种认证。

本页不宣称在线账号登录、所有 profile 下载行为都与官方服务等价。
连接阶段和 registry 来源见 [连接与注册表](../protocol/client-connection-and-registries.md)。

## Vanilla 26.1 权威实现

源码根由 `scripts/vanilla path 26.1` 解析。

| 决策 | 精确源码锚点 | 关键事实 |
| --- | --- | --- |
| 入站聊天验证 | `net/minecraft/client/multiplayer/ClientPacketListener.java:1041-1078` | 先 global index、解包 cached signatures，再 sender/session/link validator 与 presentation |
| last-seen 与发送 | `net/minecraft/client/multiplayer/ClientPacketListener.java:2632-2667` | processed signature 累积 ack；聊天/可签名命令带 last-seen update |
| 官方举报范围依据 | `net/minecraft/client/multiplayer/chat/report/ReportingContext.java:32-54` | Vanilla 草稿与提交链，整套按产品范围排除 |
| 社交服务开关 | `net/minecraft/client/gui/screens/social/PlayerSocialManager.java:44-59` | offline 时 blocked service 返回 false；online refresh 串行后查询 |
| profile texture 集合 | `net/minecraft/client/resources/SkinManager.java:80-132` | secure lookup 过滤默认皮肤，UUID/property cache，skin/cape/elytra allOf 发布 |

global chat index 与单个 sender 的 signed message index 分别验证。
Vanilla 先将成功解包的签名写入 cache，再检查未知 sender 或 message validator 失败。
不能因为消息最后不显示，就省略此前协议状态推进。
last-seen 的“已处理”和“展示给用户”是分离输入。

## 决策输入与状态所有权

| 状态 | BBB owner | 身份与生命周期 |
| --- | --- | --- |
| remote chat session/key | `bbb-client-world` chat/player info | sender UUID、session UUID、过期时间 |
| signature cache、global index、last-seen | world `client::chat` | 当前连接的聊天协议状态 |
| 显示消息与 narration | world chat presentation | delivery revision、GUI ticks、过滤配置 |
| discovered names、hidden/blocked | world `client::social` | UUID 与 services generation |
| 出站包形状 | net `chat_session` | typed request、last-seen update |
| profile 解析、异步皮肤 HTTP 拉取与签名校验 | `bbb-client-net`（`profile_textures`、`services_profile_keys`） | UUID、完整 packed textures property、磁盘/内存缓存 |
| profile textures CPU 解码与驻留交付 | `bbb-client-render::item_model::skin_runtime` | UUID、完整 packed textures property |
| 皮肤 atlas/GPU handle | renderer | presentation/resource generation |

packet 解码只建立结构化输入，不从 raw component 文本决定信任级别。
显示文本、本地化 runs、click/hover、narration 与 report evidence 分别从 canonical 消息派生。
renderer 无权把“绿色/灰色标签”反向写成签名验证结论。

## 判断与处理顺序

1. 收到 player chat 后先核对全局 index；错序产生明确 disconnect outcome。
2. 从完整签名或 bounded signature cache 解包 last-seen body，并推进协议 cache。
3. 查 sender 的 remote session，构造含 sender/session/index 的 signed link。
4. 验证 key、消息链、签名与相关时效；错误消息走对应失败处理。
5. presentation 应用 chat type decoration、visibility、hidden/blocked、filter mask 与显示延迟。
6. 将 processed signature 与是否展示交给 last-seen tracker；需要时产生独立 ack。
7. 用户发送聊天或命令时，生成并消费 last-seen update，然后选择对应 unsigned 包形状。

### 入站签名和 last-seen

`client::chat::message_resolution` 使用 RSA PKCS#1 v1.5 与 SHA-256 验证 remote message。
签名输入含 sender UUID、session UUID、message index、salt、秒级 timestamp、UTF-8 内容和 last-seen。
未知 key 格式、无法解包的 cached signature 或不匹配链必须由 owning validator 处理。
签名验证不能仅验证展示后的字符串，因为 decoration 与 unsigned content 不是原始 signed body。

world chat 保存 128 项 signature cache 和 20 项 last-seen tracker。
ack offset 超过 64 的阈值与发送时显式 drain 各有路径，不能只在聊天输入时才确认。
ring 未填满和 wrap 后仍按协议顺序生成 update/checksum。
显示历史清空只改变 display messages，不清空仍服务于当前连接的 signature/last-seen 状态。

delete-chat 通过 signature identity 定位，而不是按显示文本匹配。
尚未到删除时间的消息延迟替换为 marker，保留其显示时序位置。
本地化、换行、滚动或隐藏 GUI 不会给该消息另分配协议身份。

### 出站 unsigned 边界

`bbb_client_net::chat_session` 对普通聊天写 `signature: None`。
无 signable arguments 的命令使用普通 `ChatCommand`。
有 signable arguments 的命令保留 `ChatCommandSigned` 的包结构，但 argument signatures 是空列表。
timestamp、salt 和 last-seen update 仍照 typed request 传递。
请求声称需要签名却没有 arguments 时显式失败，不静默转换命令类别。

这对应 Vanilla 无 key pair 的出站语义。
BBB 不生成伪密钥、不借用入站公钥签名，也不声称 offline UUID 经过 Mojang 账号认证。
last-seen 只确认消息处理，不证明发送者身份，也不替代服务器对命令权限的检查。

### 显示与社交

world `chat::presentation` 拥有 display、delay、trust tag、近期社交身份历史和 narration 的状态。
native 解析 rich component 到 HUD runs、click/hover/insertion 与可访问性文本。
show-item hover 使用共享 item-tooltip builder；URL 动作继续进入既有确认路径。
chat restrictions screen 的发送权限来自 canonical visibility/permission 配置。

social discovery 使用名字→UUID map 保留发现记录，player-info remove 不等于抹掉所有历史社交行。
hidden 和 blocked 独立保存；查询时排除本地玩家与 nil UUID 的不当屏蔽。
Services block list 替换必须同时匹配 active 标志与 generation。
offline runtime 的 Services block-list 会话恒 inactive，测试中的 active fixture 不构成在线产品入口。

`SocialChatHistory` 仅保存近期聊天发送者身份与签名存在标记，维持社交列表的离线玩家和排序。
窗口包含最近 1,024 个已接受事件；system 事件同样占槽，可见聊天的 100 条窗口不能替代它。
该历史不保存举报正文、签名证据链、举报 ID 或草稿。入站验证和 delayed message 的时间戳、
完整签名仍由聊天协议 owner 持有，用于信任标签、删除匹配与确认。

官方玩家举报整套流程按用户产品裁决为 `not-needed`，包括入口、草稿、证据与提交。
暂停菜单直接执行断开；死亡页面保留 normal/hardcore 原有退出与延迟确认。
范围与重开条件唯一取 [客户端 ledger](../../../unsupported-client-features.md#official-player-reporting)。

### profile textures、Services 与资源发布

net 在连接前 bounded prepare Services 公钥，并分开保存 `PROFILE_PROPERTY` 与 player certificate key family。
`bbb_client_net::services_profile_keys` 按 SHA-1/RSA 检查完整 `textures` property 的签名，`bbb_client_net::profile_textures` 负责异步 Mojang profile 解析与皮肤/披风字节拉取，`skin_runtime` 在纯 CPU/渲染侧解码并缓存图像（renderer 不依赖 HTTP 客户端）。
`ResolvableProfileData` 保留完整 `Vec<GameProfileProperty>`，name/value/signature 保留原始 UTF-16，并先选择首个名称精确为 `textures` 的属性，再解码；
该项损坏时不跳到后续同名项。`verify_profile_texture_signature` 通过 `JavaString::try_to_string` 拒绝 `value` 或 `signature` 中含孤立 surrogate 的输入，并按 UTF-8 字节校验 SHA-1/RSA 签名；缓存键仍使用原始属性单元，内存预算按 UTF-16
存储字节计费。
HTTP session profile 的 `properties` 按 authlib `PropertyMap.Serializer` 接受数组和对象两种形式。
数组形式忽略非对象元素，属性字段接受 string/number/bool；对象形式只处理数组值，其元素
允许单元素数组递归解包。数字保留 JSON 词法。同名属性按首见名称分组，组内保持原顺序；
对象重复键以最后值覆盖，保留首见位置。signature 缺失与显式 null 分别为无签名和失败。
properties 缺失/null 使最终构造失败，非 null 标量产生空属性；先前重复 properties 的解码失败
不会被后一个有效字段掩盖。`http_profile_client_parses_loopback_name_and_session_profile_and_caches` 覆盖 loopback
HTTP 到 resolved profile，`http_profile_properties_follow_authlib_shapes_and_primitive_coercion`
和重复根字段用例覆盖形状、转换与失败顺序。
world 只保存 profile projection；native 传入本地/远端查询坐标。
远端 PlayerInfo 要求 secure，本地玩家和 profile item 可以查询 insecure 集合。

BBB 远端 unsigned/invalid property 直接选择 UUID 默认皮肤，不发 HTTP。
这是可见 fallback 与资源请求时机的明确区别：Vanilla `createLookup` 先调用 `get(profile)`，
异步下载/注册结果之后才以 `requireSecure` 过滤；本页不把两者请求轨迹宣称为相同。

texture URL 仅接受 http/https 与精确 `textures.minecraft.net` host。
type-specific texture identity 使用 authlib 风格 basename，完整 URL 冲突失败。
磁盘 key 使用 Guava `hashUnencodedChars` 等价的 UTF-16 little-endian SHA-1。
profile lookup key 则包含 UUID 和完整 packed property，不能退化成 URL-only cache。

skin、cape、elytra 是一个 base set，要求下载、decode 与 GPU upload 所需步骤全部成功。
任一失败保持默认 fallback，不发布一半新 skin、一半旧 cape 的组合。
pack-backed skin patch 仍从同一 immutable resource generation 解析 priority/filter。

profile result cache 的 access TTL 为 15 秒；bounded texture entries、decoded PNG 与磁盘 identity 属于进程 owner。
disconnect/transfer/resource 边沿失效 renderer projection，CPU 图像可在新 generation 重新交付。
native drain 已解码图像，render thread 回报 GPU commit，renderer 不直接发 HTTP 或读磁盘。
reload 完成所有 CPU/GPU/audio fallible prepare/commit 后才转移 process texture owner。
失败时旧 generation 与旧 texture owner 均保持，不重建 generation-keyed CPU shadow cache。

control counters 同时报告 profile-set/asset/key bytes 和 async queue/resident/delivery bytes。观察计数不成为另一份资源生命周期 owner。

### 服务器行为准则

code of conduct 属于服务器配置期内容与用户接受状态，不属于聊天签名或身份认证。
`bbb_client::code_of_conduct` 以 connected server key 保存 accepted text hash。
当前 world 没有规则文本或没有连接身份时，不会伪造一次有效接受。
规则文本改变后旧 hash 不再匹配；清除当前服务器接受状态写回同一个 store。
overlay 显示、接受动作和配置期继续流程仍由当前连接上下文约束。
接受某服务器规则不等于允许资源 URL、trusted profile 或 Mojang 在线服务访问。

## 生命周期和 fallback

连接替换需重建聊天序列与 session-bound 状态，不能继承旧连接的 next index。
清空显示、停止 Services、替换 report environment 与释放 renderer projection 各有独立入口。
公钥获取失败或 texture 验证失败不升级信任；默认皮肤仍可用于显示。

## BBB 实现映射

| owner | 实际路径 | 生产职责 |
| --- | --- | --- |
| 入站聊天 | `bbb_client_world::client::chat` | index、session、signature、last-seen |
| 签名 payload | `bbb_client_world::client::chat::message_resolution` | RSA 验证与 packed signature 解包 |
| 出站聊天 | `bbb_client_net::chat_session` | unsigned 包形状和 last-seen 传递 |
| 社交 | `bbb_client_world::client::social`、`bbb_client_world::client::chat::presentation` | 过滤与近期身份历史 |
| 公钥输入与签名校验 | `bbb_client_net::services_profile_keys` | bounded Services key 获取/解析与 `textures` 属性签名验证 |
| 异步 profile/skin HTTP 拉取 | `bbb_client_net::profile_textures` | Mojang profile 解析、允许域名/大小校验与本地磁盘缓存 |
| profile texture 驻留 | `bbb_client_render::item_model::skin_runtime` | 纯 CPU PNG 解码、驻留预算与跨代重投递 |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| chat/session 边界 | packet listener + social manager | world/chat/social + net | `aligned` | chat、last-seen 与 social tests（offline 范围） |
| profile trust 和下载 | `SkinManager` | `bbb_client_net::profile_textures` + `skin_runtime` | `aligned` | property signature test；下载时机差异见正文 |
| 在线服务 | `UserApiService` | offline availability gate | `not-needed` | ledger offline 范围裁决；不由本地 builder 测试证明在线能力 |

## 结构性不变量

- 出站签名为空与入站验证、last-seen 确认必须保持独立。
- display、report log、signature cache 不因同一清屏动作一起销毁。
- report result 必须匹配 generation/id/attempt，screen return 必须匹配 generation。
- profile-property trust 与 chat certificate trust 不混用 key family。
- skin/cape/elytra 原子发布，renderer 只拥有 presentation 与纯 CPU 图像解码驻留，不依赖 HTTP 或磁盘 I/O。

## 聚焦测试（focused tests）和真实证据

| 测试 | 路径 | 证明的语义 |
| --- | --- | --- |
| `outbound_requests_are_unsigned_without_a_session` | `bbb_client_net::chat_session` | 无签名同时保留 signed-command 形状与 last-seen |
| `player_chat_bad_global_index_requests_disconnect_without_presentation` | `bbb_client_world::client::chat::tests::messages` | 错序先拒绝，不展示、不推进 signature cache |
| `player_social_services_block_list_rejects_stale_or_inactive_replacement` | `bbb_client_world::client::social` | Services generation/active 双门 |
| `profile_texture_property_trust_requires_a_valid_services_signature` | `bbb_client_net::services_profile_keys` | signed、缺 key、unsigned 与 malformed 区分 |

这些是已定位的语义测试，存在测试不等于已执行外部服务验收。

## 当前差异

offline 客户端无本地 profile key pair 和在线账号认证；整套官方举报是明确的范围排除。
皮肤 secure fallback 与下载时机的区别已在正文明确，不能由主 chat 机制状态吞并。
其余差异和范围裁决见 [客户端 ledger](../../../unsupported-client-features.md)，不在此维护第二份 Todo。

## 账本归属

[客户端 ledger](../../../unsupported-client-features.md) 的 persistence/profile/secure-session 和 resource 领域拥有剩余差异。
[AGENTS.md](../../../../AGENTS.md) 拥有 offline 范围与推进原则。

## 精确重开条件

协议 global index、signed body、signature cache 或 last-seen ring 格式变化时重核 chat 验证。
offline scope、公钥来源、Services availability 或 report environment identity 改变时重核安全边界。
profile URL policy、cache identity、requireSecure 时机或 image-set 发布阶段改变时重核 texture trust。

## 相关页面和源码索引

- [屏幕、HUD 与库存](../ui/client-screens-hud-and-inventory.md)：draft、URL、restrictions 与 narration 的界面消费。
- [场景与渲染](../rendering/client-scene-and-rendering.md)：player texture GPU presentation。
- [离线资源信任](../resources/client-offline-asset-trust.md)：资源输入与事务边界。
- [客户端选项与平台](../persistence/client-options-and-platform.md)：启动配置和本地状态文件。

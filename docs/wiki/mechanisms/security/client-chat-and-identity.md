# 客户端聊天与身份：签名、显示和皮肤信任

机制 ID：`security-secure-chat-reporting`

## 机制目的和边界

本页界定聊天协议验证、可见消息展示、本地社交过滤以及玩家 profile 纹理信任的边界。BBB 客户端在离线（offline-only）产品范围内连接外部多人服务器，不持有本地 profile 密钥对。出站消息无签名并不意味着可以跳过入站会话/签名验证，也不取消 `last-seen` 消息确认。皮肤属性签名验证与聊天身份签名使用不同的密钥族与消息格式，严禁混为一谈。

本页不宣称支持 Mojang 在线账号登录或覆盖全部在线 profile 下载路径；协议阶段与注册表来源见[客户端连接、协议阶段与注册表](../protocol/client-connection-and-registries.md)。

## Vanilla 26.1 权威实现

源码根由 `cargo dev vanilla path 26.1` 解析：

| 决策 | 精确源码锚点 | 关键事实 |
| --- | --- | --- |
| 入站聊天验证 | `net/minecraft/client/multiplayer/ClientPacketListener.java:1041-1078` | 先校验全局序号、解包缓存签名，再经发送者会话与签名链验证器处理并送入展示层 |
| `last-seen` 与出站发送 | `net/minecraft/client/multiplayer/ClientPacketListener.java:2632-2667` | 已处理签名持续累积确认；聊天与可签名命令携带 `last-seen` 更新 |
| 官方举报范围依据 | `net/minecraft/client/multiplayer/chat/report/ReportingContext.java:32-54` | Vanilla 举报草稿与提交链，整套按产品范围排除 |
| 社交屏蔽服务边界 | `net/minecraft/client/gui/screens/social/PlayerSocialManager.java:44-59` | 离线时服务屏蔽查询恒为 `false`；在线刷新串行完成后查询 |
| profile 纹理集合 | `net/minecraft/client/resources/SkinManager.java:80-132` | 安全查找过滤默认皮肤，按 UUID 与完整属性缓存，皮肤/披风/鞘翅整体发布 |

全局聊天序号与单个发送者的签名消息序号彼此独立验证。Vanilla 先将成功解包的签名写入签名缓存，再处理未知发送者或验证器拒绝分支，绝不能因消息最终不展示而跳过协议状态推进。`last-seen` 中的「协议已处理」与「已展示给用户」同样属于分离输入。

## 决策输入与状态所有权

| 状态 | BBB 唯一所有者 | 身份与生命周期 |
| --- | --- | --- |
| 远端聊天会话与公钥 | `bbb-client-world` 聊天与玩家信息模块 | 发送者 UUID、会话 UUID、密钥过期时间 |
| 签名缓存、全局序号与 `last-seen` 环形窗 | 世界层 `client::chat` | 当前连接的聊天协议状态 |
| 显示消息、信任标签、近期社交历史与旁白 | 世界层 `client::chat::presentation` | 投递修订号、GUI tick、过滤配置 |
| 发现的玩家名、隐藏列表与启动期屏蔽列表 | 世界层 `client::social` + `ChatPresentationConfig` | 玩家 UUID（`blocked_players` 由 CLI 参数 `--blocked-player <UUID>` 注入） |
| 出站聊天与命令包形状 | 网络层 `bbb_client_net::chat_session` | 强类型请求与 `last-seen` 更新 |
| profile 解析、异步皮肤拉取与属性签名校验 | `bbb-client-net`（`profile_textures`、`services_profile_keys`） | UUID、完整打包 `textures` 属性、磁盘与内存缓存 |
| profile 纹理 CPU 解码与驻留交付 | `bbb_client_render::item_model::skin_runtime` | UUID、完整打包 `textures` 属性 |
| 皮肤图集与 GPU 纹理句柄 | 渲染器 | 呈现与资源代际 |

数据包解码仅构造结构化输入，不从原始组件文本推断安全信任级别。显示文本、本地化片段、点击/悬停事件、旁白与近期社交身份历史（`SocialChatHistory`）均分别从权威消息派生；渲染器无权将「绿色/灰色信任标签」反向视作签名验证结论。

## 判断与处理顺序

1. 收到玩家聊天包后先核对全局序号；错序直接触发明确的断开连接结果。
2. 从完整签名或有界签名缓存解包 `last-seen` 消息体，并推进协议签名缓存。
3. 查询发送者的远端聊天会话，构造包含发送者、会话 ID 与消息序号的签名链。
4. 验证公钥、签名链、消息签名与时效；验证失败走对应的未验证或拒绝分支。
5. 展示层依次应用聊天类型装饰、聊天可见性设置、隐藏/屏蔽名单、文本过滤掩码与消息显示延迟。
6. 将已处理签名及是否展示的结果提交给 `last-seen` 追踪器，达到阈值时生成独立确认包。
7. 本地发送聊天或命令时，先生成并消费 `last-seen` 更新，再选择对应的无签名出站数据包形状。

### 入站签名和 `last-seen`

`client::chat::message_resolution` 使用 RSA PKCS#1 v1.5 与 SHA-256 验证远端消息签名。签名载荷覆盖发送者 UUID、会话 UUID、消息序号、随机盐、秒级时间戳、UTF-8 原文及 `last-seen` 列表。未知密钥格式、无法解包的缓存签名或断链情况均由验证所有者显式处理；验证必须基于原始签名正文，绝不能拿装饰后或未签名替代文本做校验。

世界聊天状态维护 128 项签名缓存与 20 项 `last-seen` 追踪窗口。确认偏移超过 64 时的主动独立确认与发言时的显式消费各有路径，不会只在玩家发言时才发确认。环形缓冲未填满与回绕后均严格按协议顺序产出更新与校验和。清空聊天显示历史只清理可见消息列表，不清除仍服务于当前连接的签名缓存与 `last-seen` 协议状态。

删除聊天消息（`DeleteChat`）严格按签名身份匹配，绝不按渲染文本匹配。尚未到达删除时刻的消息延迟替换为占位标记并保持原显示时序。本地化、自动换行、滚动或隐藏聊天栏不会赋予消息新的协议身份。

### 出站无签名边界

`bbb_client_net::chat_session` 对普通聊天写入 `signature: None`。不含可签名参数的命令发送普通 `ChatCommand`；包含可签名参数的命令保持 `ChatCommandSigned` 数据包结构，但参数签名列表为空，时间戳、盐与 `last-seen` 更新仍按强类型请求正常携带。若请求声称需要签名却不含可签名参数，则显式报错，绝不静默改写命令类别。

该行为严格对齐 Vanilla「无本地 profile 密钥对」时的出站语义：BBB 既不生成伪造密钥、也不借用入站公钥签名，也不声称离线 UUID 经过 Mojang 在线认证。`last-seen` 仅确认协议已处理过的入站消息，不证明发送者身份，也不替代服务端权限校验。

### 显示与社交

世界层 `client::chat::presentation` 拥有消息显示列表、延迟投递队列、信任标签、近期社交身份历史（`SocialChatHistory`）与旁白状态。客户端将富文本组件投影为 HUD 片段、点击/悬停/插入目标与无障碍文本；`show_item` 悬浮框复用共享物品提示框构建器，URL 点击继续进入既有确认弹窗。聊天限制提示页的发送权限读取权威可见性与权限配置。

社交发现表（`PlayerSocialState.discovered_names_to_uuid`）以玩家名到 UUID 的映射记录已见玩家；收到 `PlayerInfoRemove` 时不抹除已见或近期发言行，而是将其标为已离线（`removed = true`）。隐藏名单（`hidden_players`）与屏蔽名单（`blocked_players`）统一保存在 `ChatPresentationConfig` 中并分开管理：隐藏列表由 `hide_social_player` / `show_social_player` 在运行期动态维护，屏蔽列表由启动期 `session_shared::build_initial_world_and_net_counters` 从 CLI 参数 `--blocked-player <UUID>`（可重复）注入；社交查询（`social_player_is_blocked`、`social_should_hide_message_from`）显式排除本地玩家自身与 nil UUID。离线客户端不接入 Mojang 在线社交屏蔽服务。

`SocialChatHistory` 仅保存近期聊天发送者身份与签名存在标记，用于维持社交列表中的离线发言者与排序权重。窗口容量为最近 1,024 条已接受事件，系统消息同样占用槽位，可见聊天的 100 条显示窗口不能替代它。该历史不保存举报正文、签名证据链、举报 ID 或举报草稿；入站验证及延迟消息所需的时间戳与完整签名仍由聊天协议所有者独立持有，用于信任标签、删除匹配与 `last-seen` 确认。

官方玩家举报整套流程已按产品范围裁决为 `not-needed`，包含所有入口、草稿、证据构建与提交界面；暂停菜单直接执行断开连接，死亡界面保留普通/极限模式原有退出与延迟确认。范围依据与重开条件唯一以[客户端账本](../../../unsupported-client-features.md#official-player-reporting)为准。

### Profile 纹理、Services 公钥与资源发布

网络层在连接前有界获取并解析公共 Services 公钥，并分开维护 `PROFILE_PROPERTY` 与玩家聊天证书两套密钥族。`bbb_client_net::services_profile_keys` 按 SHA-1/RSA 校验完整 `textures` 属性签名；`bbb_client_net::profile_textures` 负责异步 Mojang profile 解析、允许域名/大小校验与桌面本地磁盘缓存（`wasm32` 目标通过 `crates/bbb-client-net/src/wasm_fetch.rs` 经 `bbb-web-host` `/proxy/skin` 域名白名单代理拉取）；`bbb_client_render::item_model::skin_runtime` 在纯 CPU/渲染侧解码 PNG 并维护驻留预算（渲染器不依赖 HTTP 客户端或磁盘 I/O）。

`ResolvableProfileData` 保留完整 `Vec<GameProfileProperty>`，名称、值与签名均保留原始 UTF-16 单元，且先选中首个名称精确等于 `textures` 的属性再解码，该项损坏时不跳过尝试后续同名属性。`verify_profile_texture_signature` 通过 `JavaString::try_to_string` 拒绝 `value` 或 `signature` 含孤立代理项的输入，并按 UTF-8 字节验证 SHA-1/RSA 签名；缓存键仍保留原始属性单元，内存预算按 UTF-16 存储字节计费。

HTTP 会话 profile 中的 `properties` 遵循 Authlib `PropertyMap.Serializer`，同时支持数组与对象两种 JSON 形态：数组形态忽略非对象元素且字段接受字符串/数字/布尔；对象形态仅处理数组值且支持单元素数组递归解包；数字保留原始 JSON 词法。同名属性按首次出现分组并维持组内顺序；对象重复键以末值覆盖并保留首次位置。缺失 `signature` 视作无签名，显式 `null` 视作失败；`properties` 缺失或 `null` 导致整体失败，非 `null` 标量产生空属性，且前一个重复 `properties` 的解码失败不会被后一个合法字段掩盖。`http_profile_client_parses_loopback_name_and_session_profile_and_caches` 与 `http_profile_properties_follow_authlib_shapes_and_primitive_coercion` 覆盖上述回环请求、形态与失败序。世界层仅持有 profile 投影，客户端提供本地/远端查询坐标；远端 `PlayerInfo` 要求安全验证（secure），本地玩家与头颅/profile 物品允许查询非安全（insecure）集合。

对远端无签名或验签失败的 `textures` 属性，BBB 直接回退至按 UUID 派生的默认皮肤而不发起 HTTP 请求。这与 Vanilla `SkinManager.createLookup`（先发起异步拉取与注册，再按 `requireSecure` 过滤）在网络请求时机上存在显式差异，最终可见回退结果一致。

纹理 URL 仅允许 `http`/`https` 与精确主机名 `textures.minecraft.net`。按纹理类型使用 Authlib 风格 basename 标识，完整 URL 冲突时拒绝。磁盘缓存键采用与 Guava `hashUnencodedChars` 等价的 UTF-16 小端 SHA-1；profile 查找键包含 UUID 与完整打包属性，绝不退化为仅按 URL 索引。

皮肤、披风与鞘翅组成一个原子基础集合，下载、解码与 GPU 上传全部成功后才整体发布，任一环节失败维持默认回退，绝不呈现「一半新皮肤、一半旧披风」的撕裂状态。资源包内置皮肤补丁仍从同一不可变资源代际按优先级与过滤规则解析。

profile 结果缓存访问 TTL 为 15 秒；有界纹理条目、解码后 PNG 与磁盘标识归属进程级所有者。断线、转服与资源重载边沿仅失效渲染器投影，进程级 CPU 图像可在新代际重新投递。客户端运行时排空已解码图像，渲染线程回报 GPU 提交结果，渲染器绝不发 HTTP 请求或读磁盘。资源重载仅在所有可失败 CPU/GPU/音频准备提交全部完成后才移交进程级纹理所有者，失败时旧代际与旧纹理所有者一同保留。控制计数器同步上报 profile 集合/资产/密钥字节及异步队列/驻留/交付字节，观测计数本身不构成第二生命周期所有者。

### 服务器行为准则

服务器行为准则（Code of Conduct）属于配置期提示内容与本地接受记录，不属于聊天签名或身份认证。`bbb_client::code_of_conduct` 按当前连接服务器键持久化已接受文本哈希（桌面端写入私有配置目录 `code-of-conduct.json`，可用 `--code-of-conduct-store` 或 `BBB_CODE_OF_CONDUCT_STORE` 覆盖；Web 端写入 `localStorage["bbb.code_of_conduct.json"]`）；当世界无规则文本或无连接身份时不伪造接受状态。规则文本变更后旧哈希自然失效；清除当前服务器接受状态写回同一存储。遮罩显示、确认操作与配置期放行均受当前连接上下文约束，接受规则文本不放宽任何资源 URL、profile 信任或在线服务边界。

## 生命周期和 fallback

连接替换必须重建聊天全局序号与会话绑定状态，绝不继承旧连接的下一序号。清空聊天显示历史、重置聊天/社交会话状态与释放渲染器纹理投影各有独立入口。Services 公钥获取失败或属性验签失败绝不提升信任级别，玩家仍正常回退至默认皮肤。

## BBB 实现映射

| 语义所有者 | 实际路径 | 生产职责 |
| --- | --- | --- |
| 入站聊天协议 | `bbb_client_world::client::chat` | 全局序号、远端会话、签名缓存与 `last-seen` |
| 签名载荷验证 | `bbb_client_world::client::chat::message_resolution` | RSA-SHA256 验签与压缩签名解包 |
| 出站聊天会话 | `bbb_client_net::chat_session` | 无签名出站包形状与 `last-seen` 传递 |
| 社交与展示过滤 | `bbb_client_world::client::social`、`bbb_client_world::client::chat::presentation` | 隐藏/启动期屏蔽过滤、社交排序与近期身份历史 |
| Services 公钥与验签 | `bbb_client_net::services_profile_keys` | 有界公钥拉取解析与 `textures` 属性签名校验 |
| 异步 profile 与皮肤拉取 | `bbb_client_net::profile_textures` | Mojang profile 解析、允许主机/大小校验与本地磁盘缓存 |
| 皮肤 CPU 解码与驻留 | `bbb_client_render::item_model::skin_runtime` | 纯 CPU PNG 解码、驻留预算与跨代重投递 |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 聊天验证与社交边界 | `ClientPacketListener` + `PlayerSocialManager` | `client::chat`、`client::social` + `chat_session` | `aligned` | 入站验签、`last-seen`、社交排序与 1,024 窗口测试 |
| profile 纹理信任与下载 | `SkinManager` | `bbb_client_net::profile_textures` + `skin_runtime` | `aligned` | 属性签名校验与回环 HTTP 测试（请求时机差异见正文） |
| 在线账号与官方举报服务 | `UserApiService`、`ReportingContext` | 离线产品范围边界（无在线认证与举报表面） | `not-needed` | 客户端账本 `# Not needed` 范围裁决 |

## 结构性不变量

- 出站无签名契约与入站签名验证、`last-seen` 确认必须保持独立，互不削弱。
- 聊天显示历史、近期社交身份历史（`SocialChatHistory`）与签名缓存不会因同一次清屏动作一并销毁。
- 社交隐藏与屏蔽查询始终排除本地玩家自身与 nil UUID，且社交列表展示不反写聊天协议验证状态。
- profile 属性签名校验与聊天证书签名校验严禁混用密钥族。
- 皮肤、披风与鞘翅作为整体原子发布；渲染器仅持有呈现资源与纯 CPU 图像解码驻留，不依赖 HTTP 或磁盘 I/O。

## 聚焦测试（focused tests）和真实证据

| 测试 | 路径 | 证明的语义 |
| --- | --- | --- |
| `outbound_requests_are_unsigned_without_a_session` | `bbb_client_net::chat_session` | 无签名出站同时保留可签名命令包形状与 `last-seen` |
| `player_chat_bad_global_index_requests_disconnect_without_presentation` | `bbb_client_world::client::chat::tests::messages` | 全局序号错序先触发断连，不展示也不推进签名缓存 |
| `player_social_discovery_hidden_blocked_and_sorting_share_chat_owner` | `bbb_client_world::client::social` | 发现名表、隐藏/屏蔽过滤、排除本人/nil 与排序共享同一所有者 |
| `player_info_remove_preserves_seen_and_recent_social_rows_as_removed` | `bbb_client_world::client::social` | 玩家离服后保留已见与近期发言社交行并标为 `removed` |
| `recent_social_identity_expires_after_1024_accepted_events_including_system_messages` | `bbb_client_world::client::social` | 近期社交身份按含系统消息在内的 1,024 事件窗口过期 |
| `profile_texture_property_trust_requires_a_valid_services_signature` | `bbb_client_net::services_profile_keys` | 区分有效签名、缺公钥、无签名与畸形属性 |

上述用例均为可定位的语义单元测试，不代表已执行在线 Mojang 服务实网验收。

## 当前差异

离线客户端无本地 profile 密钥对与在线账号认证，且官方玩家举报整套流程已明确排除出产品范围；远端无签名皮肤跳过 HTTP 请求直接回退默认皮肤的时机差异已在正文说明。[客户端账本](../../../unsupported-client-features.md)当前无未关闭的 `# Todo` 缺口，范围裁决及后续差异均以该账本为准。

## 账本归属

安全、在线会话与官方举报的范围裁决统一归属[客户端账本](../../../unsupported-client-features.md)；离线范围与推进原则见 [AGENTS.md](../../../../AGENTS.md)。

## 精确重开条件

当协议全局序号、签名载荷、签名缓存或 `last-seen` 环形窗格式变化时须重核聊天验证；当离线产品范围、Services 公钥来源或社交屏蔽配置规则调整时重核安全边界；当 profile 纹理 URL 白名单、缓存键、`requireSecure` 时机或三件套原子发布流程变化时重核皮肤信任链路。

## 相关页面和源码索引

- [屏幕、HUD 与库存](../ui/client-screens-hud-and-inventory.md)：聊天草稿、URL 确认、发送限制与旁白的界面消费。
- [场景与渲染](../rendering/client-scene-and-rendering.md)：玩家皮肤的 GPU 呈现。
- [离线资产可信链](../resources/client-offline-asset-trust.md)：资源输入与事务边界。
- [客户端本地配置、按键与平台窗口](../persistence/client-options-and-platform.md)：启动配置与本地状态文件。

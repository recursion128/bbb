# 客户端聊天编辑、命令语法与 UTF-16 文本处理

机制 ID：`ui-chat-and-command-text`

## 机制目的和边界

本页定义客户端聊天输入（`ChatScreen`）、单行/多行文本编辑器、本地命令解析与语法着色、补全候选、点击动作确认页，以及 `JavaString` 原始 UTF-16 单元在网络 UTF-8、NBT、SNBT、命令参数 Packrat 语法与语言资源中的边界契约。
字体提供者、字形度量、图集、旧版 `§` 格式码与二维字形点击几何归[客户端字体与文本](client-font-and-text.md)；全局屏幕栈与 GUI 视口/缩放分别归[屏幕、HUD 与库存](client-screens-hud-and-inventory.md)及[客户端本地配置、按键与平台窗口](../persistence/client-options-and-platform.md)；入站聊天签名验证与社交屏蔽见[客户端聊天与身份](../security/client-chat-and-identity.md)。

## Vanilla 26.1 权威实现

| 主题 | Vanilla 类/方法与锚点 | 核心契约 |
| --- | --- | --- |
| 聊天与文本编辑 | `ChatScreen`、`EditBox`、`MultiLineEditBox`、`AbstractTextAreaWidget`、`BookEditScreen`、`BookSignScreen`、`AbstractSignEditScreen` | 原始 UTF-16 编辑缓冲、显示起点滚动、选区/光标绘制序与鼠标双击/拖拽 |
| 本地命令与补全 | `CommandSuggestions`、`CommandDispatcher`、`CommandContextBuilder`、`SharedSuggestionProvider.java:31`、`SuggestionProviders.java:20-22` | 最内层 redirect 上下文语法着色、UTF-16 补全匹配与 `MATCH_SPLITTER`（`._/`） |
| SNBT 与参数语法 | `SnbtGrammar`、`TagParser`、`ItemParser`、`BlockStateParser`、`ItemPredicateArgument`、`ComponentPredicateParser`、`EntitySelectorParser`、`NbtPathArgument`、`ResourceOrIdArgument` | Packrat 语法、空白跳过、`ErrorCollector.LongestOnly` 最远拒绝与游标回退规则 |
| 线协议与 NBT 编码 | `Utf8String.java:10-31`、`StringTag.java:52-54`、`StringTag.java:61-62`、`NbtIo.java:159-179`、`Style.java:613-628`、Netty 4.2.7.Final `ByteBufUtil` | 3× 字节上限与 UTF-16 单元上限、Modified UTF-8 原始保留与 Netty 代理消费规则 |
| 确认页与选中高亮 | `ClientPacketListener`（unattended 点击命令）、`Screen`、`ConfirmScreen`、`RenderPipelines.java:648-655`、`BlendFunction.java:10-12`、`BlendFunction.java:16-22` | 模态确认、旁白延迟、反色与蓝色叠加选区混合管线 |

## 决策输入与状态所有权

| 输入/状态 | 唯一所有者 | 下游消费 |
| --- | --- | --- |
| `ChatScreen` 模式、编辑值、历史（100）、草稿、限制屏父状态与点击确认父状态 | `bbb_client::input::ClientInputState`、`input::chat_runtime`、`input::chat_mouse` | HUD 渲染投影与网络出站队列 |
| 单行/多行原始 UTF-16 编辑缓冲、光标、选区与持久 `display_start` | `bbb_client::input::text_field::{TextField, MultilineTextField}` | 聊天、书本、告示牌、对话框、命令方块与创造搜索框共用 |
| 本地命令解析、参数跨度、最内层 redirect 边界与补全候选 | `WorldStore::parse_command_utf16`、`bbb_client_world::commands` | `input::chat_runtime` 语法着色区间与补全列表 |
| SNBT 与各命令参数 UTF-16 扫描器、线协议编解码与 Modified UTF-8 | `bbb_protocol`（`codec::java_utf8`、`nbt_summary::snbt`、`component::command::syntax`） | 客户端、服务端与注册表编解码共享 |
| 语言资源原始 UTF-16 映射与组件模板解析 | `bbb_client_assets::LanguageCatalog`、`bbb_client_assets::component_text` | HUD、提示框、物品名、书本与世界文本 |
| 确认页布局、无纹理 `core/gui` 选区混合与旁白 | `bbb_client_render::hud::{HudGuiPipelines, chat_input, text_layout}`、`ScreenNarrationRuntime` | 只读消费投影，不回写编辑或协议状态 |

## 判断与处理顺序

### 1. UTF-16 `JavaString` 规范值与边界编码

1. **内部规范存储与哈希**：NBT 字符串（`StringTag.java:61-62` Modified UTF-8）、`RegistryJsonValue` / `ResourceJsonValue`（保留原始数字词法与孤立代理）、`LanguageCatalog`（单一 UTF-16 映射 + 合法键派生显示缓存）、书本原始/过滤页与签名标题/作者、`Style.insertion`（`Style.java:613-628`）、`CustomModelData`、药水 `custom_name`、profile 属性、方块状态属性映射、地图装饰键与旗帜翻译键均以 `JavaString` 保留原始 UTF-16 单元。组件 `HashOps` 按类型标记、长度与小端 UTF-16 单元计算哈希；序列化为 JSON 时孤立代理以 `\uXXXX` 转义，显示侧投影 `U+FFFD` 绝不回流为键、哈希或规范 NBT。
2. **文件根名策略**：`servers.dat` 与快捷栏 NBT 读取遵循 `NbtIo.java:159-179` / `StringTag.java:52-54`，按长度跳过根名字节不解码，写出空根名，防止畸形根名阻断合法载荷。
3. **书本编辑与签名边界**：书页编辑按 UTF-16 单元移动光标，签名标题框按代码点移动（编辑上限 15 单元，线协议标题上限 32 单元，单页 1024 单元，上限 100 页）；`BookEditScreen.saveChanges` 先更新本地 `ItemStack` 并清空旧过滤页再发包，签名确认发包后即关闭而不本地预测成书；签名按钮与 `Enter` 共享 `StringUtil.isBlank` 单元判定，标题仅裁去两端 `unit <= 0x20`。
4. **网络 UTF-8 编解码边界**：
   - **入站**：`codec::java_utf8` 按 `Utf8String.java:10-31` 先验 `3 × max_units` 字节界限，畸形序列按 Netty/JDK 规则替换为 `U+FFFD`，再校验 UTF-16 单元上限（聊天消息 `256`、无/有签名命令 `32767`、补全请求 `32500`、补全结果 `32767`、签名参数名 `16`）。
   - **出站**：`Encoder::write_utf16_string` / `write_java_string` 按锁定的 Netty 4.2.7.Final `ByteBufUtil` 规则编码——孤立代理写 `?`，高代理后继非高低代理时写其低 8 位字节。
5. **剪贴板与进程参数边界**：`copy_to_clipboard` 与 `open_url` / `open_file` 在内部与跨线程通道保留原始 `JavaString`；仅在写入系统剪贴板或传给操作系统进程边界时按 JDK UTF-8 将孤立代理转为 `?`（剪贴板同时遵守 GLFW 首个 `NUL` 截断规则）。

### 2. SNBT 与命令参数 Packrat 语法及扫描器

SNBT、物品组件补丁、物品谓词与资源引用/内联值共享基于 `JavaString` 的 Packrat 语法解析器，对齐 Vanilla `ErrorCollector.LongestOnly` 最远游标拒绝与 21 个 `snbt.parser.*` 翻译键：

- **SNBT 核心词法规则**：所有终结符匹配前按 Java 空白规则跳过空白（`1 . 5`、`0 x1` 等合法）；`literal` 按 `float_literal → integer_literal → quoted → { → [ → unquoted` 前缀语法尝试，不贪婪消费整个 token；`0` 后与 `x|X` 后带 `cut`（拒绝 `01`、`0xg`），`b|B` 前不带 `cut`；值层转换失败（如 `128b` 溢出）停在字面量末尾而不回退 `unquoted`；数组与空键在 `]` 或值末尾触发 action 拒绝；引号内原始高代理与 `\uDC00` 转义先拼接再校验。
- **有界安全预算**：按 [`AGENTS.md`](../../../../AGENTS.md) 保留 `MAX_NBT_DEPTH = 64` 嵌套上限；安全预算拒绝的翻译键为 `None`，由命令层走对应拒绝分支。

各命令参数族统一由 `bbb-protocol` 的 `*_utf16` 扫描器在原始缓冲区执行（`&str` 入口仅为字节偏移适配器）：

| 参数族 | 生产扫描器入口 | 关键语法与错误游标契约 |
| --- | --- | --- |
| 通用字符串/数字/Token/标识符 | Brigadier UTF-16 扫描器、`scan_command_resource_location_argument_utf16` | 引号仅转义当前引号或 `\`；标识符校验失败退回起点；函数标签先消费 `#` |
| SNBT / 文本 / 样式 | `scan_command_snbt_argument_utf16`、`scan_command_text_argument_utf16` | 语法错误报最远游标与 `snbt.parser.*` 键；`withCodec` 编解码失败退回参数起点（含前导空白） |
| 物品组件补丁（`item_stack`） | `CommandItemComponentPatchScannerUtf16` | 先检 `]` 再跳空白（`[]` 合法、`[ ]` 非法）；值编解码错误先于缺失分隔符，游标指向值起点 |
| 物品谓词（`item_predicate`） | 物品谓词 Packrat 扫描器（`ScannedCommandItemPredicateTerm`） | 保留 `!` 取反与 `|` / `,` 分组；`~` 先查专用子谓词再回退任意已注册组件（含临时组件 `Unit.CODEC`）；未知标识错误指向标识末尾（不含后续空白），值编解码错误指向值末尾 |
| 方块状态与方块谓词 | `scan_command_block_identifier_utf16`、`scan_command_block_body_utf16` | 先跳空白（`[ ]` 合法）；允许直接尾逗号；`IntegerProperty` 接受 37 段 BMP Unicode 十进制数字（`property_accepts_value`）；标签 vague 属性不校具体定义 |
| 实体选择器与 NBT 路径 | `parse_command_entity_selector_prefix_utf16`、`parse_command_nbt_path_value_prefix_utf16` | 保留原始 `JavaString` 选项/节点名与 `nbt=` SNBT，不经 `String` 往返；NBT 路径保留 `CommandNbtPathParseError.offset` 精确偏移 |
| 资源引用/内联值与粒子 | `scan_command_resource_or_id_utf16`、`scan_command_particle_options_utf16` | 资源先试 `Identifier.readNonEmpty` 再落 SNBT；粒子 `{` 必须紧邻名称，编解码失败为无上下文错误（`cursor = None`） |
| 目录/轴/运算符/范围/角度/时间/坐标/UUID | `scan_command_{catalog,swizzle,range,angle,time,coordinates}_argument_utf16` 等 | 范围错退回整体起点并区分 `0..-0`；时间按 JDK `Math.round(float)` 位运算取整；坐标区分为轴起点或参数起点；未知运算符无上下文游标 |

### 3. 本地命令解析、语法着色与补全

1. **最内层 Redirect 上下文着色**：仅当输入以 `/` 开头时，`input::chat_runtime::chat_input_syntax_state` 将 `WorldStore::parse_command_utf16` 产出的参数 UTF-16 跨度与 `innermost_context_argument_start` 转为 `HudCommandInputSyntax`。`hud::text_layout::hud_command_input_runs` 仅对最内层 redirect 上下文（对应 `getLastChild().getArguments()`）的参数按 5 色循环着色（AQUA / YELLOW / GREEN / LIGHT_PURPLE / GOLD），其余为字面量灰 `0xAAAAAA`，未消费尾部为红 `0xFF5555`，草稿灰斜体优先于语法色；同名参数按 Brigadier `LinkedHashMap` 插入序折叠。
2. **UTF-16 补全匹配与替换**：
   - 字面量节点按整串转小写后切尾段（`JavaString::to_lowercase_root` 保留孤立代理与 `Final_Sigma`），provider / 注册表候选按尾段转小写匹配（`SharedSuggestionProvider.MATCH_SPLITTER` 在 `._/` 后重启；带 `#`/`!`/`!#` 前缀时按 `commonPrefix` 过滤）；
   - 按 `SuggestionsBuilder.suggest` 丢弃与当前 `remaining` 精确相等的候选；
   - 补全起点由 `CommandContextBuilder.findSuggestionContext` 决定（尾随分隔符推进至下一子节点）；
   - 服务端补全与本地补全共用 `JavaString` 候选文本，替换保留光标后后缀并按请求 ID 淘汰过期响应，纯光标移动不重建补全。

### 4. `ChatScreen` 文本与生命周期

- **输入与提交**：`ClientInputState` 持有 `MESSAGE`/`COMMAND` 模式、256 单元编辑值、100 条历史与最新草稿；提交前按 Java `trim`、Character 空白合并、NBSP 转空格、再 `trim` 与 256 单元截断归一化（不修复孤立代理）。按键事件与 IME commit 共用文本处理器，预编辑文本不改写已提交值也不触发补全。
- **鼠标、滚轮与 Shift 插入**：点击按「补全弹窗 → 聊天消息 → `EditBox`」分派；同屏幕修订、同按钮且间隔 `< 250 ms` 触发双击选词；Shift-click 聊天插入返回未消费以继续进入 `EditBox` 定位（不更新 `initial`），普通消息点击消费成功则更新 `initial`。
- **选区高亮与多行几何**：`HudGuiPipelines`（归属 `CorePipelines.hud_gui`，资源包 `minecraft:core/gui` 事务重载见[离线资产可信链](../resources/client-offline-asset-trust.md)）拥有普通填充、反色与蓝色叠加三条无纹理管线（`RenderPipelines.java:648-655`、`BlendFunction.java:10-12` / `:16-22`）。单行输入与书名先绘选区再绘光标，书页、多行输入与告示牌先绘光标再绘选区；跨行尾选区延伸至控件宽减 4，由内缩 1 px 裁剪与父裁剪求交（书页固定 `122×134`、14 行上限）。GUI 视口浮点投影归[客户端本地配置、按键与平台窗口](../persistence/client-options-and-platform.md)。
- **尺寸变化与重载**：GUI 尺寸变化先将当前原始值存为 `initial` 再重建输入框并置光标至末尾；有效资源重载从 `initial` 重建聊天框，过期确认不重建。

## 生命周期和 fallback

- **点击动作载荷与确认页**：`Screen.clickCommandAction` → `check_unattended_command_utf16` 区分直接发送无签名包、解析/权限确认与签名确认（打开聊天或复制带 `/` 命令）；`open_url` 经唯一 `UntrustedUriIdentity` 解析比较并走链接确认弹窗；`open_file` 仅限客户端自造组件（拒绝服务端下发）；`suggest_command` 遵守 `ExtraCodecs.CHAT_STRING`（拒 `§`、DEL 与 C0，允许孤立代理）。
- **模态栈与失效保护**：确认页提交与父屏幕恢复严格校验世界、资源与屏幕修订号，模态期间阻断底层输入；对话框静态/动态命令在确认期间保留表单，取消不执行 `after_action`；`ClearDialog` 不穿过已打开或恢复的确认页，`ShowDialog` 可挂起并恢复确认页，而死亡/容器/换世替换直接丢弃过期确认与挂起父状态。
- **确认页旁白**：初始化快照经有序平台副作用交付，标题与完整正文一起播报（不受视觉 15 行上限截断）；鼠标悬停延迟 `750 ms`、键盘操作延迟 `200 ms`，仅键盘触发打开时初始聚焦 Accept；可用性检查 `GameNarrator.isActive`，TTS 可用且模式为 Off 时仍推进 collector 而仅抑制发声。

## BBB 实现映射

| 语义所有者 | 生产路径 | 边界职责 |
| --- | --- | --- |
| 文本编辑缓冲 | `crates/bbb-client/src/input/text_field.rs` | `TextField` / `MultilineTextField` 原始 UTF-16、光标、选区与 `display_start` |
| 聊天输入、补全与鼠标 | `crates/bbb-client/src/input/chat_runtime.rs`、`crates/bbb-client/src/input/chat_mouse.rs` | 历史/草稿/归一化、语法着色区间、补全与双击/拖拽分派 |
| 本地命令解析与补全 | `crates/bbb-client-world/src/commands` | `WorldStore::parse_command_utf16`、最内层 redirect 边界与补全生成 |
| SNBT 与命令 UTF-16 扫描器 | `crates/bbb-protocol/src/nbt_summary`、`crates/bbb-protocol/src/component/command`、`crates/bbb-protocol/src/codec` | `SnbtGrammar`、`*_utf16` 扫描器、Netty/JDK UTF-8 与 Modified UTF-8 |
| 语言资源与组件哈希 | `crates/bbb-client-assets/src`、`crates/bbb-registry-codec/src/item_hash` | `LanguageCatalog`、`component_text` 与 `HashOps` UTF-16 哈希 |
| 选区混合管线与渲染布局 | `crates/bbb-client-render/src/hud` | `HudGuiPipelines`、`chat_input`、`text_layout`（只读消费投影） |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| UTF-16 `JavaString` 书本/NBT/JSON/语言保留 | `StringTag`、`Gson`、`Language`、`Utf8String` | `bbb-protocol`、`bbb-client-assets`、`bbb-client-world` | `aligned` | Netty/JDK/Gson 差分与原始单元往返测试 |
| SNBT 与命令参数 Packrat 语法及最远拒绝 | `SnbtGrammar`、`ItemParser`、`ComponentPredicateParser` | `nbt_summary::snbt`、`component::command::syntax` | `equivalent` | `vanilla_differential` 与 `vanilla_grammar_differential` 差分表 |
| `ChatScreen` 编辑、历史、草稿、语法着色与补全 | `ChatScreen`、`EditBox`、`CommandSuggestions` | `bbb_client::input::chat_runtime`、`hud::text_layout` | `aligned` | 编辑/滚动/着色/补全与 GPU 读回聚焦测试 |
| 点击命令/URL/剪贴板确认页与旁白 | `ClientPacketListener`、`Screen`、`ConfirmScreen` | `bbb_client::input`、`ScreenNarrationRuntime` | `aligned` | 代际失效、悬停/焦点旁白与 Shift 插入测试 |

## 结构性不变量

- `ChatScreen` 渲染器与控制接口不得恢复独立编辑缓冲、权限、点击或历史/草稿所有者。
- 命令语法区间只能来自当前值的当次本地解析最内层 redirect 上下文；渲染器不解析世界、不重排参数顺序，也不让语法色压过草稿格式器或改写显示/光标切分。
- 规范 NBT/JSON/书本/聊天草稿的 `JavaString` 原始 UTF-16 单元与显示投影（`U+FFFD`）严格分离，网络 UTF-8 替换规则只在协议线编解码边界生效。
- 确认页提交与父屏幕恢复必须校验屏幕、世界与资源代际，模态输入期间阻断底层交互。

## 聚焦测试（focused tests）和真实证据

| 领域 | 代表测试与命令入口 | 验证边界 |
| --- | --- | --- |
| SNBT 与 Packrat 差分 | `cargo dev test -p bbb-protocol vanilla_differential`、`vanilla_grammar_differential` | 8 张 SNBT 差分表、21 项拒绝键、物品补丁/谓词、资源引用与 `depth_bound_is_a_named_divergence` |
| UTF-8 / Netty / 原始单元 | `cargo dev test -p bbb-protocol java_utf8`、`cargo dev test -p bbb-client-world raw_` | `java_utf8_matches_netty_heap_and_direct_oracle`、原始 `JavaString` 书本/SNBT/选择器/NBT 路径 |
| 补全与参数扫描 | `cargo dev test -p bbb-client-world completion` | `literal_completion_lowercases_the_whole_buffer_before_the_tail`、`resource_candidates_cut_the_tail_at_the_common_prefix`、`selector_option_value_replay_matches_the_pinned_jar` |
| 语法着色与选区 GPU | `cargo dev test -p bbb-client chat_input_syntax`、`cargo dev test -p bbb-client-render chat_input` | 最内层 redirect 边界、五色循环、未读红尾、草稿优先与反色/蓝色叠加像素读回 |
| 确认页、旁白与 Shift 插入 | `cargo dev test -p bbb-client shift_insertion`、`cargo dev test -p bbb-client confirmation` | Shift 插入穿透、代际失效、悬停/键盘焦点旁白与模态输入阻断 |

## 当前差异

`MAX_NBT_DEPTH = 64` 作为 [`AGENTS.md`](../../../../AGENTS.md) 要求的有界安全预算有意保留（Vanilla SNBT/NBT 不设该层数上限），不构成未关闭功能缺口。[客户端账本](../../../unsupported-client-features.md)当前无未关闭的 `# Todo` 缺口。

## 账本归属

本页对应机制 `ui-chat-and-command-text`；范围裁决（含官方玩家举报排除）与后续未关闭差异统一登记在[客户端账本](../../../unsupported-client-features.md)，推进原则见 [`AGENTS.md`](../../../../AGENTS.md)。

## 精确重开条件

目标版本改变 Brigadier / SNBT / `ItemParser` / `ComponentPredicateParser` / `BlockStateParser` / `EntitySelectorParser` 语法、网络 `Utf8String` 长度界限，或 `ChatScreen` / 确认页生命周期规则时重验本页。

## 相关页面和源码索引

- [客户端字体与文本](client-font-and-text.md)：字体提供者、字形度量、图集、旧版 `§` 与二维字形点击几何。
- [屏幕、HUD 与库存](client-screens-hud-and-inventory.md)：全局屏幕栈替换与输入分发。
- [客户端本地配置、按键与平台窗口](../persistence/client-options-and-platform.md)：`set_gui_viewport` GUI 缩放与平台窗口。
- [客户端聊天与身份](../security/client-chat-and-identity.md)：聊天签名验证、`last-seen` 与社交过滤。
- [离线资产可信链](../resources/client-offline-asset-trust.md)：资源包 `core/gui` 着色器事务重载。
- [服务端保存与重载](../persistence/server-save-and-reload.md)：服务端存档 NBT 根名与编码边界。

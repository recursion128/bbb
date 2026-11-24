# 客户端聊天编辑、命令语法与 UTF-16 文本处理

机制 ID：`ui-chat-and-command-text`

## 机制目的和边界

本页定义客户端在聊天输入（`ChatScreen`）、单行/多行编辑器、命令本地解析与语法着色、补全候选、点击动作确认页，
以及 `JavaString` 原始 UTF-16 单元在网络 UTF-8、NBT、SNBT、物品/方块/选择器/资源 packrat 语法与语言资源中的边界契约。
字体 provider、glyph metrics、atlas 烘焙、legacy `§` 格式与二维字形点击几何由[客户端字体与文本](client-font-and-text.md)拥有；
入站聊天签名验证与社交屏蔽见[客户端聊天与身份](../security/client-chat-and-identity.md)。

## Vanilla 26.1 权威实现

- `ChatScreen`、`EditBox`、`MultiLineEditBox`、`AbstractTextAreaWidget`、`BookEditScreen`、`BookSignScreen`：
  原始 UTF-16 编辑缓冲、显示起点、选区与点击/拖拽交互顺序。
- `CommandSuggestions`、`CommandDispatcher`、`CommandContextBuilder`、`SharedSuggestionProvider`：
  本地命令解析、最内层 redirect context 语法着色与 UTF-16 补全候选匹配。
- `SnbtGrammar`、`TagParser`、`ItemParser`、`BlockStateParser`、`ItemPredicateArgument`、`ComponentPredicateParser`、
  `EntitySelectorParser`、`NbtPathArgument`、`ResourceOrIdArgument`：packrat 语法与最远错误位置规则。
- `ClientPacketListener`（unattended 点击命令确认）、`Screen`（链接/文件/命令点击与旁白）、`Utf8String` 与锁定 Netty `ByteBufUtil`：
  网络 UTF-8 替换与 UTF-16 单元上限。

## 决策输入与状态所有权

`ClientInputState` 拥有 `ChatScreen` 的编辑值、光标/选区、历史、草稿、确认页父状态与输入权限投影；
`WorldStore::parse_command_utf16` 与 `bbb-protocol` 的 `*_utf16` scanner 拥有原始 UTF-16 词法/语法结果；
`bbb-client-assets::LanguageCatalog` 与 `bbb-registry-codec` / `bbb-protocol` 保留原始 `JavaString` 单元，
renderer 与平台剪贴板/进程出口只消费只读投影或在边界执行目标编码。

## 判断与处理顺序

### UTF-16 `JavaString` 规范值与资源/网络编码

世界侧书本编辑状态以 UTF-16 单元保存页面与标题，显示文本为派生值，签名/取消和状态序列化保留孤立 surrogate。
输入已使用原始单元进行插入、删除、选择、分行及定位；编辑页按单元移动，签名框按代码点移动。
编辑页 HUD 携带原始文本及单元光标，分行、逐光标切片绘制和选择宽度直接消费原始范围。
签名框已复用原始 EditBox 布局，按光标切分 forward 序列，保留 15 单元插入光标上限；
居中与选择使用格式宽度，字形裁切与分段绘制使用原始字符宽度。签名框显示起点保存在输入状态，
按光标/选择的更新顺序滚动，投影后由绘制和点击定位共同消费。
书本页面和标题从 `BookEditSubmission` 经 native、`NetCommand::EditBook` 到网络编码均保留原始单元；
标题仅裁掉两端 `unit<=0x20`，普通字符串调用也进入同一书本编码器。页面 1024 单元、标题 wire 32 单元和
100 页上限在编码前检查，编辑标题仍受 15 单元限制。
`Encoder::write_utf16_string` 复现 Vanilla 26.1 锁定的 Netty 4.2.7.Final
[ByteBufUtil](https://github.com/netty/netty/blob/netty-4.2.7.Final/buffer/src/main/java/io/netty/buffer/ByteBufUtil.java)
原始编码：孤立 surrogate 写 `?`；高 surrogate 消耗后继单元，后继既非低 surrogate 又非高 surrogate 时
直接写其低 8 位。使用缓存中的官方 netty-buffer/netty-common 依赖实测 heap/direct 缓冲区，
`[D800,20AC]` 均发送 `3F AC`，不是 U+FFFD 的 UTF-8；codec 固定向量和实际 net socket 测试覆盖该边界。
签名按钮的绘制与点击共享 `StringUtil.isBlank` 单元判断；Enter 只拒绝原始空标题，
纯空格标题会按 Java trim 提交空串（`BookSignScreen.keyPressed`）。NBSP 等 Unicode 空格会禁用按钮，
Enter 提交仍保留它们；U+0085 不是此处的 Java 空白。已用准备的 Java 工具链枚举全部 BMP 单元核对空白集合。
`DataComponentPatchSummary` 的 writable 页面与 filters 已使用 `JavaString`，唯一规范数据为原始 UTF-16 单元，
其 serde 表示输出 `\uXXXX` 转义的 JSON 字符串。编辑器从这些单元重开，保存直接写回单元；显示投影不参与物品值重建。
registry NBT source codec、持久投影、对话框物品组件与 HashOps 使用同一个规范值，
100 页与每页 1024 单元的限制同时覆盖 raw 与 filtered。
未签名保存已按 Vanilla `BookEditScreen.saveChanges` 先更新本地 ItemStack 再发送包；
队列缺失、关闭或已满时仍关闭编辑器，保留修改并清空旧 filters，重开读取本地修改。
签名确认按 `BookSignScreen.keyPressed` 在发送调用后关闭，不等待入队成功，也不预测签名物品；
物品内容等待服务端更新。
`HashOps.createString` 按原始 UTF-16 长度与单元哈希，`StringTag.java:61-62` 则用 Modified UTF8 的
`writeUTF` 保存原始单元。网络编码的替换行为不能用于本地组件、哈希或持久值；
副手过滤页保存、重开、物品克隆并转移到主手及服务端回包替换已按原始单元定向验证；
持久物品栈的 NBT 二进制往返保留孤立 surrogate。
具名 NBT 的公共 codec 与 `NamedCompound` 使用同一个 `JavaString` 保留根名原始单元；
`encode_named_nbt_summary_java_string` 与现有 `&str` 编码入口共享 modified UTF-8 单元编码器，
普通文本不先物化 UTF-16 缓冲。实际 `servers.dat` 与 hotbar 读取改用共享 file-root policy：
按长度跳过名称字节，不解码名称；写出空名称（`NbtIo.java:159-179`、`StringTag.java:52-54`）。
因此孤立代理和畸形根名字节均不会阻止有效 payload 读取，但截断、payload 与尾随字节仍校验。
保存层的同一边界见[服务端保存](../persistence/server-save-and-reload.md)。

共享 NBT 与 canonical component 的孤立 UTF-16 字符串已接入客户端哈希原语：
计算 CRC 的同时保留导出的原始 NBT，探针导出和原生 SNBT 调试输出也保留这些单元。

`nbt_summary::snbt_encode` 的同一遍历提供诊断 `String` 与原始 `JavaString` 输出。
`canonical_snbt_java_string` 和 `canonical_snbt_key_java_string` 保留 `StringTagVisitor`
按 Java 键排序及 `StringTag.quoteAndEscape` 的原始单元；诊断 API 用可重解析转义表示孤立
代理项（`StringTag.quoteAndEscape`、`StringTagVisitor.visitCompound`）。
`bbb_registry_codec::item_hash::hash_block_predicate` 将原始写出交给 UTF-16 哈希；持久组件 `component_matchers` 与源编解码器 `source_nbt` 同样保存原始 NBT 字符串，并以原始单元计算 JSON 写出长度。源遍历器仍受原有节点、集合与嵌套深度预算约束。
writable 页面与 filters 的组件哈希复用该 UTF-16 哈希原语，并区分孤立 surrogate 与 U+FFFD。
网络文本入口复用同一 UTF-8 scanner：`Decoder::read_string` 返回普通文本，
`read_java_string` 直接生成 `JavaString` 单元，避免先建立临时 Rust String。
先按 `Utf8String.java:10-29` 检查 3× 字节上限，再检查解码后的 UTF-16 单元数。
畸形 UTF-8 消费对照准备的 JDK 25 `String.decodeUTF8_UTF16`：例如 `ED A0 80` 和截断的 `ED A0`
均产生一个 U+FFFD，错误后继保留给下一轮解码。Netty 4.2.7.Final heap/direct 的 234,213 组
输入（全部单/双字节及边界字节组合）得到相同结果，固定摘要由 `java_utf8_matches_netty_heap_and_direct_oracle` 验证。
实际 socket EditBook 测试同时检查发送字节与服务端解码结果；受限标识符在此共享解码之后
执行自己的语法校验。网络 UTF-8 的替换行为不代替 NBT modified UTF-8 的原始单元保留。
`RegistryJsonValue` 的 JSON 值解析已保留孤立 surrogate，合法高低对仍规范为普通 String。
语法、深度与数量校验仍由既有入口负责；解码后的原始单元接入 writable 页面/filter codec 和动态 NBT writer。
准备的 Gson 2.13.2 严格模式与 BBB 定向测试均核对全部 65,536 个单元；
server/known-pack JSON 的实际组件解码同时覆盖页面、过滤页和嵌套 custom_data。
component 的 literal 与 `text` 字段已贯通 JSON/NBT source、校验、collapsed 形式和兄弟组件追加；
规范 NBT 始终保留原始单元。显示片段按 `StringDecomposer.feedChar` 将孤立 surrogate 投影为 U+FFFD，
不再跳过整段文本；带样式节点与兄弟片段保留继承关系，writer 长度仍按原始单元计算。
`translate`、`fallback` 和字符串参数也保留原始单元。共享
`bbb-client-assets::component_text` 先按原始键查语言表，递归解析参数和兄弟组件，
在显示边界将字面片段投影为文本；
数值参数使用 Java 数字格式。孤立 surrogate 的 key 不会先替换为 U+FFFD 再查询语言表，
避免误命中另一个合法 key。source writer 长度与规范 NBT 均不使用显示替换值。
`keybind` 同样保留 Java 字符串单元；按 `KeybindContents.MAP_CODEC` 的字符串 codec 与
`KeyMapping.createNameSupplier` 的未知名称回退，已注册名称查询当前绑定的语言键，
未注册名称按原始键查询语言表；两者均以零参数进入同一模板分解，保留节点样式。
语言资源由 `LanguageCatalog` 使用单一原始 UTF-16 map 保存；普通 `get(&str)` 借用仅含合法
文本键的派生显示缓存，`get_raw` 按原始单元查找。pack 优先级、重复键覆盖和 deprecated
改名保持源顺序；格式归一化使用 Java 默认正则的 ASCII 数字范围，孤立单元不丢失。
序列化保留正常文本对象形状，含原始键时使用键值对数组，反序列化重建显示缓存。
语言 JSON 数字复用 `ResourceJsonValue` 的 `RawValue` reader 保存原词法，
`1e2`、`-0` 和 `1e400` 经资源优先级、改名及真实 native 组件显示保持拼写
（`Language.loadFromJson`）。数值消费者在既有 accessor / NBT 投影处按需解析，
超范围数值仍拒绝；已解码的 `serde_json::Value` 无法恢复原词法。资源树的数字相等比较
按词法，数字文本的存储计入既有资源预算；该相等比较不替代消费者的数值比较。
`get_raw_units` 借用 UTF-16 切片查询，`JavaString::push_str` 用于翻译键后缀拼接。
模板扫描器共用 ASCII token 规则；合法原始键借用现有显示查询，模板的 `&str` 入口直接借用
字节；孤立代理键借用原始 UTF-16 单元查询。命中后不构造 fallback，也不增加第二份原始值缓存。
显示模板不再作为语言键参与查表。`%s`、`%1$s` 与 `%%` 共用分解规则；非法 token 或缺失参数
使整份模板按原文字面显示。原生组件与物品名、lore、唱片、乐器、盔甲纹饰、附魔和画作描述
复用此 owner，描述的 NBT 原树贯穿 registry/组件状态，附魔与画作不从已显示的名称反推键。
旧状态缺少原树时保留其已有显示投影；不会拿替换后的文字重新查询语言表。
`NativeItemRuntime` 只保存绑定查询函数，启动和资源重载均注入 options 的同一查询入口；
悬浮名、tooltip 及其调用方据此读取当前绑定，不复制配置表。
`Style.insertion` 按 `Style.java:613-628` 保留 Java 字符串，贯通 source、规范 NBT、样式继承、
HUD 交互与聊天编辑器。Shift-click 按 `ChatScreen.handleComponentClicked` 选择插入文本；
编辑器过滤控制单元并按既有长度规则截断，孤立 surrogate 不会提前替换，光标仍按 UTF-16 单元更新。
样式快照通过 `JavaString` 的 `\uXXXX` 转义 JSON 字符串无损往返孤立 surrogate。
共享 SNBT reader 以 UTF-16 单元读取，引号内原始片段与转义先拼接，再检查结果；
例如原始高 surrogate 与 `\uDC00` 可以拼成合法补充字符。依据为 `SnbtGrammar.java` 的
`PLAIN_STRING_CHUNK`、quoted string 拼接与 map key 规则；空白按 Java Character 规则消费。
`parse_snbt_compound_utf16` 接入 registry JSON/NBT flattened compound、持久 custom/typed data、
持久组件 NBT 谓词和 dialog 物品组件，原始字符串值一直保留到组件 patch。
命令参数的 `parse_snbt_prefix_utf16` 与 `parse_snbt_compound_prefix_utf16` 返回消费的 UTF-16
单元数，保留调用方后缀，不执行全文入口的尾部检查；普通字符串 prefix API 复用同一实现，
仍返回 UTF-8 字节位置。两种入口分别对应 `TagParser.parseAsArgument` 的 tag 与 compound 参数。
`raw_snbt_quotes_join_raw_and_escaped_surrogates_before_key_validation` 与
`raw_flattened_custom_data_reaches_dialog_item_component` 覆盖拼接和实际消费者。

#### Packrat 语法差分基线

SNBT、物品堆叠组件补丁与资源引用/内联值三支已不再靠逐项 codec 测试推断，而是拿 Vanilla 官方
jar 里的解析器逐输入差分：`SnbtGrammar.createParser(NbtOps.INSTANCE)` 经
`Grammar.parseForCommands`（即 `TagParser.parseAsArgument`）、`ItemParser.fillSuggestions` /
`ItemParser.State` 状态机，以及 `Identifier.readNonEmpty` 加 `ResourceOrIdArgument.createGrammar`
的第二候选。两侧对同一输入比较接受与否、值与消费位置；拒绝时还比较
`ErrorCollector.LongestOnly` 与 `Grammar.parseForCommands` 选出的错误位置。表在
`nbt_summary::snbt::vanilla_differential` 与 `syntax::vanilla_grammar_differential`，
每行都记着 Vanilla 的期望结果。

核验确认并据此实现的 grammar 规则：

- 所有终端在匹配前先 `skipWhitespace`（`StringReaderTerms.TerminalCharacters.parse`），
  因此 `1 . 5` 是 double `1.5`、`- 1` 是 int `-1`、`0 x1` 是 int `1`，数字内部也可以有空白。
- `literal` 按 `float_literal` → `integer_literal` → quoted → `{` → `[` → unquoted 依次尝试；
  数字分支不整段读 token，而是按前缀 grammar 停下并把余下留给调用方：`1.5.5` 是 `1.5d`（消费 3）、
  `1abc` 是 int `1`（消费 1）、`1e` 是 int `1`、`1fb` 是 float `1.0f`（消费 2）、
  `1 e5` 是 double `100000.0`。float 四个候选按源序，先命中的决定字面量，即使它的值被拒绝：
  `1e400` 溢出后仍回落到 int `1`，`1.5b` 是 double `1.5`（`b` 留给调用方）。
- `integer_literal` 的 `0` 后 `cut` 让 `01` 直接失败，而 `b|B` 前没有 `cut` 让 `0b`、`0b2`、
  `0b_1` 都成为 byte `0`；`x|X` 后的 `cut` 让 `0xg` 失败而不回退；`0_1`/`0_0` 因 `_` 触边使
  `decimal_numeral` 失败，落到 `marker(decimal_numeral,"0")`，得到消费 1 的 int `0`。
  后缀 `u|U`、`s|S` 命中后其类型字母仍可失败并回退，所以 `1sx` 是 short `1s`、`1ux` 是 int `1`。
  十进制默认 signed int、`0x`/`0b` 默认 unsigned，unsigned 带负号报错。
- `SimpleHexLiteralParseRule` 与转义候选的位置规则：`"\u12"` 报到第一个十六进制位，
  `"\U00110000"` 报到 8 位数字之后；未知转义报到转义字符本身。
- 值层失败与语法层失败不同：`integer_literal` 只构造字面量，转换发生在 `literal` 的 action，
  所以 `128b` 是"匹配成功但值被拒"，不再尝试 `unquoted_string_or_builtin`，游标停在 `128b` 之后；
  只有整个数字 grammar 失败（`01`、`1_`、`+_1`）才走 unquoted 兜底，并把它读到的 run 计入错误位置。
- 数组在 `]` 之后才建值（`listLiteral` 的 action），所以 `[I;1l]`、`[B;128b]`、`[I;3000000000]`
  的错误位置是闭括号之后，而不是出错元素处。
- `mapEntry` 的 action 在读完值之后才拒绝空键，所以 `{"":1}` 的位置是值末尾。
- 物品堆叠组件补丁与 `ItemParser.State.readComponents` 的循环条件、`!`、`=`、分隔符与空白
  顺序逐项一致；`stone[damage=1 b]` 因共享 SNBT 前缀而是 byte `1b`。
- 资源引用/内联值先试非空标识符再读 SNBT：`a:b:c`、`A:B` 的标识符分支失败后落到内联值，
  `true`、`012`、`:`、`..` 是引用，空 run 记作合法的 `minecraft:`。

以上差分在两套语料上收敛：手写语料 316 例与随机 fuzz 语料 2056 例除下面的深度边界外零分歧。

拒绝本身现在整条对照。`Grammar.parseForCommands` 抛的是
`ErrorCollector.LongestOnly` 在最远游标上留下的第一条，所以读取器把它整条带出来：
`SnbtParseFailure` 有游标、Vanilla 的翻译 key（21 个 `snbt.parser.*` 加终结符的
`argument.literal.incorrect`）与该 key 的参数。每条拒绝都记在同一个 mark 上而不是由调用方自己
造：`32768s` 因此答后缀终结符 `b|B` 而不是取值越界，`{"":1}` 答 `.` 而不是 `empty_key`
（`{"":"x"}` 才答 `empty_key`）。`snbt.parser.number_parse_failure` 的参数是
`SnbtGrammar.IntegerLiteral.create` 当时那次 JDK 调用自己的
`NumberFormatException` 文本，六种措辞各一行。`snbt.parser.expected_integer_type` 与
`expected_float_type` 不可达：两个后缀规则都只产出本族类型。差分表逐行断言游标、key 与参数，
`every_refusal_key_the_grammar_throws` 每个 key 一行。

命令面照抄这一条：方块参数的方块实体 compound（`bbb_server_types::block_state_parse`）、
`minecraft:nbt_compound_tag` 与 `minecraft:nbt_tag`、`minecraft:component`、`minecraft:style`、
NBT 路径的 `{…}` 过滤器（`NbtPathArgument.parseNode`、`NbtPathArgument.readObjectNode`）与 `minecraft:item_stack`
的 `[…]` 组件值（`ItemParser.State.readComponent`）都报那条 key 与那个游标。
包住语法失败的只有 codec 那一半：`CommandArgumentParser.withCodec`（`parseForCommands`）与
`ERROR_MALFORMED_COMPONENT` 各自重抛并把游标退回值起点，所以 `/tellraw <玩家> {` 答语法失败而
`/tellraw <玩家> {foo:1}` 才答 `argument.component.invalid`，`/give … [minecraft:damage={]`
答语法失败而 `[minecraft:damage="x"]` 才答 `arguments.item.component.malformed`。

按 [AGENTS.md](../../../../AGENTS.md) 保留的有界安全预算：`MAX_NBT_DEPTH` 限制为 64 层（Vanilla 不设上限，400 层列表在 Vanilla 中可解析，本仓库按安全预算拒绝）；这类安全预算拒绝的翻译键为 `None`，命令面退回自身拒绝分支。物品注册表与组件编解码查询、`ERROR_MALFORMED_COMPONENT` 的失败文本一律由调用方负责。物品谓词分支（`ComponentPredicateParser`）见下文「物品谓词 Packrat 语法差分」。
命令 dispatcher 以原始单元为准：`CommandTreeState::parse_command_utf16` / `WorldStore::parse_command_utf16`
返回 `RawCommandParseResult`，参数位置与 `cursor` 是 Java `char`（UTF-16 单元）下标，参数值与原
`String` 入口共用的 `SignableCommandArgument.value` 都是 `JavaString`。`parse_argument` 之后的每个
argument family 直接调用 `bbb-protocol` 的 `*_utf16` scanner，实体选择器与 NBT 路径也在内，
没有任何族再经 `String` 投影扫描。剩下的 `String::from_utf16_lossy` 只服务 ASCII 名字查询
（方块属性键值、记分项 criteria）、错误文本与旧 `&str` 报告，任何位置都不从它推导。抛出错误一律带原始单元位置：
`RawCommandSyntaxErrorState { cursor, contextual }` 中 `contextual: false` 表示 Vanilla 的
`CommandSyntaxException` 没有 reader 上下文（例如 `MessageArgument.parseText` 的超长、
`OperationArgument.getSimpleOperation` 的未知符号、粒子 codec 拒绝），此时位置是该参数起点，字节投影
`CommandSyntaxErrorState.cursor` 仍按 Vanilla 保持 `None`。`&str` 入口保留为字节位置的适配器。
`raw_arguments_span_the_redirect_context_in_units`、
`raw_message_argument_caps_at_256_units_and_reports_the_unit_cursor` 与
`raw_errors_carry_the_unit_cursor_they_were_raised_at` 覆盖跨 redirect 的跨度、
256 单元边界与错误位置；真实连接发送命令时孤立 surrogate 仍按锁定 Netty 规则写成 `?`。
本地补全的字面名匹配也已按单元比较：`CommandSuggestions.updateCommandInfo` 把整条命令和编辑框光标
交给 `getCompletionSuggestions`，Brigadier 的 `SuggestionsBuilder` 在光标处截断、把整串转小写后
才按 `SuggestionContext.startPos` 切出尾段，`LiteralCommandNode.listSuggestions` 再拿节点自身
小写名做 `startsWith`。两侧都是 UTF-16 单元，所以名字里的代理对按两个单元参与，只敲到高位
代理也能补全；`JavaString::to_lowercase_root` 对孤立 surrogate 保持原单元，并按 run 计算
`Final_Sigma` 上下文。provider/registry 候选也改读同一尾段的原始单元：
`SharedSuggestionProvider.suggest` 与
`suggestResource`（经 `filterResources`）的模式是
`builder.getRemaining().toLowerCase(Locale.ROOT)`，即尾段本身而不是小写缓冲的尾段，
所以 `TeamArgument.listSuggestions` 的队名、`SuggestionProviders.AVAILABLE_SOUNDS`
（`SuggestionProviders.java:20-22`）的音效 id 与 registry 元素都按单元匹配；`SharedSuggestionProvider.matchesSubStr`
只在 `SharedSuggestionProvider.MATCH_SPLITTER`（`SharedSuggestionProvider.java:31` 的 `._/`）之后重启，
孤立 surrogate 只等于自身。候选文本类型因此是 `JavaString`：`CommandCompletionSuggestion.text`
贯通本地补全结果、客户端补全列表的 `CommandSuggestionState.text`、
`CommandSuggestions.sortSuggestions` 的 `startsWith`、`apply_command_suggestion_utf16` 的拼接、
`FormattedText.of(suggestion.getText())` 的 runs 解析与非命令 tab 补全。仍把候选文本写成
`String` 的只剩两处显示面：叙述文本的本地化入口 `localized_command_suggestion_narration`
（`String::from_utf16_lossy`）与 probe 观察报告写出 JSON 时的文本形式（合式文本才走该形式）。
该匹配由 `resource_provider_completion_matches_sound_ids_by_utf16_units`（音效 id 按 `suggestResource`
匹配，U+FFFD 与孤立 surrogate 尾段都命中不了）固定；期望取自
锁定 client-26.1 jar 上运行 `SharedSuggestionProvider.suggest` / `suggestResource` 的实际输出，
不是从源码推的。方块/物品/选择器补全的结构扫描也不再读该投影：`[`/`{`/`,`/`=`/`|`/引号切分直接在
缓冲区单元上做（`split_once_unit` 与选项段回放），候选文本由
`CandidateText` 从同一批单元拼出，`unit_of_byte` 因此退出补全路径（只剩 `normalized_command`
与旧 `&str` 报告的字节映射；消息里的选择器现在按 `selector.consumed` 的单元推进）。注册表与定义查询保持 ASCII：
`ascii_text` 只在全部单元为 ASCII 时投影，供 `normalize_resource_location`、属性名/值与组件名查找
使用，非 ASCII 单元在旧投影里是 U+FFFD，两侧同样不匹配。选择器的选项列表也已按单元回放
`EntitySelectorParser.parseOptions`，补全路径不再借 `String` 扫描器判定某段取值是否完整；
`bbb-protocol` 的那个扫描器只剩 parse 路径，现在同样读原始单元。
命令编辑器按完整 UTF-16 输入、实际光标、建议光标及候选上下文复用解析和建议列表。
命令树、玩家/团队名称、声音目录与有效 server suggestion 更新推进命令域失效点；registry
和 level 复用已有代次，未变化的 HUD 帧不重新构建候选。

三条路径的期望同样取自锁定 jar：物品堆叠
`minecraft:stone[minecraft:custom_name="<孤立 surrogate>"` 回答该缓冲区加 `,` 与 `]`（组件 codec
接受孤立 surrogate，解析停在缺失的 `]`；`ItemParser.fillSuggestions`
与 `ItemParser.State.suggestNextOrEndComponents`），选择器
`@e[name="<孤立 surrogate>",sort=` 回答该缓冲区加四个排序名（`EntitySelectorParser.fillSuggestions`），
方块谓词 `#minecraft:logs["<孤立 surrogate>"=x,` 回答该
缓冲区加 `axis=`（`BlockStateParser.readVagueProperties` 不校验
vague 键值）。方块状态一侧同一份输入由取值层回答：`chest[facing="<孤立 surrogate>",` 的取值被定义
拒绝，`BlockStateParser.setValue` 把游标退回取值起点，于是候选就是四个朝向、整段
替换带该单元的余段——没有候选携带它，也不会出现 U+FFFD。该匹配由
`block_state_completion_replaces_a_value_the_definition_rejects` 固定。

方块状态与方块谓词补全的取值层按 jar 的读取器回放：`BlockStateParser.readProperties`
与 `BlockStateParser.readVagueProperties` 逐段消费属性名与取值，
建议状态就是回放停下的那个——`suggestPropertyName`、`suggestEquals`、
`addSuggestions`（不读余段，所以 `chest[facing=u` 给四个朝向、`oak_log[axis=q` 给
`x`/`y`/`z`、`chest[waterlogged=tru` 给 `false` 与 `true`）、`suggestVaguePropertyValue`
（标签侧不校验取值，`#logs[axis=x` 因此给 `]`、`y`、`z` 而不给 `x`）、
`suggestNextPropertyOrEnd` 与 `suggestOpenNbt`；被拒的取值把游标退回取值
起点（`BlockStateParser.setValue`），所以 `chest[waterlogged=maybe,` 只答 `false` 与 `true`，不进入属性名层。
id 层只在尾段自己的 id 没解析出来时活动：`readBlock`/`readTag` 成功即把建议函数换成
`BlockStateParser.suggestOpenPropertiesOrNbt`/`suggestOpenVaguePropertiesOrNbt`，
只有 `ERROR_UNKNOWN_BLOCK`（`readBlock`）、`ERROR_UNKNOWN_TAG`（`readTag`）与读不出
id 的失败路径回到 id 起点、留着 `suggestItem`/`suggestTag` 答整段，所以
`/block chest` 只答 `chest[` 与 `chest{`（不再附 11 个 `minecraft:*chest`）、`/match #minecraft:logs`
只答 `#minecraft:logs[`，而 `/block oak_lo` 仍答 6 个 `*oak_log`、`/block chestt` 为空。
同一层的另一种事实是整数属性：`setValue` 问定义自己的 `Property.getValue`，
`IntegerProperty.getValue` 把取值当 `int` 解析并要求落在定义列出的
范围内，所以 `stage=00`、`+0`、`-0` 与带前导零的写法都是 `stage=0`（单独的 `+`/`-` 与 `0x0` 一样不是整数）、读取器停在取值之后
（`oak_sapling[stage=00` 答 `oak_sapling[stage=00]`，`cocoa[age=00` 答 `cocoa[age=00,` 与
`cocoa[age=00]`），而 `007`、`2147483648`、`0x0`、`0_0` 仍被拒。定义种类这个读法不需要新表：
26.1 全注册表 120 个不同（property 类、名、取值列表）里 31 个是 `IntegerProperty`，取值全为十进制
整数的也正好是这 31 个，且它们的取值列表与派生出的 `min..max` 逐项相同，所以“取值字符串全是
`int`”与“定义是整数属性”等价（`BlockStateRegistry::compare_block_property_values` 早已按同一读法
判定数值序）。`Integer.parseInt` 自己读的数字集是 JDK 的 `Character.digit(c, 10)`，BBB 手写它那
37 段（BMP 内每段十个连续值），所以带引号的 `stage="٠"`（U+0660）与 `stage="０"`（U+FF10）同样
是被列出的值，而 `²`（No）、`Ⅰ`（Nl）与超范围的 `"٢"` 仍被拒。
选择器一侧 `EntitySelectorOptions.suggestNames` 两侧都转小写
（`@e[SORT` 答 `@e[sort=`），选项值 provider 只在取值仍失败时活动（`EntitySelectorParser.parseOptions`），
`SuggestionsBuilder.suggest` 在候选文本等于余段原文时丢弃
该候选（`oak_log[axis=x` 只答 `oak_log[axis=x]`，`@e[sort=nearest` 只答两个定界符）。
这些规则由 `block_state_completion_replaces_a_value_the_definition_rejects`、
`block_state_completion_stops_at_a_value_the_definition_rejects`、
`block_state_completion_offers_ids_only_while_the_tail_has_no_id`、
`block_state_completion_accepts_every_spelling_of_a_listed_integer_value`、
`block_predicate_completion_offers_ids_only_while_the_tag_has_no_id`、
`selector_completion_lowercases_option_names_on_both_sides`、
`selector_completion_ends_a_complete_option_value_with_its_delimiters` 与
`block_predicate_completion_follows_the_vague_value_reader` 固定，逐输入期望取自锁定 client-26.1 jar
的 `fillSuggestions` applied 文本；整数属性这条事实同时由 parse 层的
`block_state_accepts_the_integer_spellings_of_a_listed_value` 按同一 jar 的 `parseForBlock` 钉住，
补全侧与 parse 侧共用 `property_accepts_value`。选择器的选项列表按
`EntitySelectorParser.parseOptions` 逐单元回放：每段取值交给该
选项自己的 modifier 读，被拒的取值把建议函数停在该 modifier 当时装好的 provider 上——只有 `sort`、
`gamemode` 与 `type`（`EntitySelectorOptions.register`）装了自己的
provider，其余停在 `EntitySelectorParser.SUGGEST_NOTHING`（`parseOptions`）——所以既不会继续列选项名，
也不会把该段的可用性写进状态；唯一在读取值之前就生效的是 `type` 的 `setTypeLimitedInversely`。
21 个选项的取值语法由 `selector_option_value_replay_matches_the_pinned_jar` 的
162 条输入逐条钉在同一 jar 的 applied 文本上。带 `prefix` 的 provider 候选表按
`SharedSuggestionProvider.filterResources` 过滤：尾段为空
时整表直接过，其余尾段只按 `Strings.commonPrefix` 截断，所以 `@e[type=!` 同时到达 `!` 的 id 表与
`!#` 的标签表（jar 161 个候选），而与 `prefix` 毫无公共部分的尾段一个表也到不了；尾段不必完整带上
`prefix`。该规则由 `resource_candidates_cut_the_tail_at_the_common_prefix` 按同一 jar 的
`suggestResource` 逐前缀（``、`#`、`!`、`!#`）钉住，调用点由
`selector_type_candidates_reach_the_tag_table_through_the_common_prefix`（`@e[type=` 322、
`@e[type=!` 与 `@e[type=!minecraft:zombie,type=` 各 161）与
`block_predicate_tag_candidates_follow_the_same_prefix_filter`（`SharedSuggestionProvider.suggestRegistryElements`
的 `#` 支）覆盖。补全起点由 Brigadier 1.3.10 的
`CommandContextBuilder.findSuggestionContext` 决定：
游标越过已解析范围时父节点换成最后一个已解析节点、起点是该范围末尾加一，所以尾随分隔符
问的是该节点自己的子节点而不是它自己。字面名、参数与 redirect 三种父节点走同一入口，由
`the_completion_context_advances_for_every_parent_kind` 按同一 brigadier 逐输入钉住（`/tell `、
`/tell @e `、`/w `、`/w @e ` 及各自的非空尾段），`a_trailing_separator_reaches_the_next_argument_table`
钉住 `/block `（1168 个方块 id）、`/match `（1183 = 15 个 `#tag` 加 1168 个 id）与 `/target `
（六个选择器加在线玩家名）三张整表。空尾段的选择器字母来自
`EntitySelectorParser.parseNameOrUUID`：只有读得动时才把
`EntitySelectorParser.parse` 装好的 `suggestNameOrSelector` 换成 `suggestName`，所以空尾段在玩家名之外
还列出六个选择器。三类差分（方块状态、方块谓词、选择器）共 255 输入逐行对照该 jar，全部一致。
该字面名匹配由 `literal_completion_lowercases_the_whole_buffer_before_the_tail`
（`İ` 展开会移动尾段起点，与真实 1.3.10 行为一致）固定，并按托管 JDK 25 的 `String.toLowerCase(Locale.ROOT)` 保持 `to_lowercase_root` 的孤立 surrogate 与 `Final_Sigma` 上下文。
物品模型 custom_data 谓词的 SNBT 字符串也使用此共享 parser，对应
`CustomDataPredicate.CODEC` → `NbtPredicate.CODEC` → `TagParser.LENIENT_CODEC`；
`custom_data_predicate_snbt_uses_shared_grammar_in_item_matching` 覆盖转义孤立 surrogate、
二进制整数、重复键覆盖及非法输入拒绝，并验证实际物品匹配结果。
外层物品模型 JSON 也由同一个 `ResourceJsonValue` 读取（字体定义面早已复用），因此 `\uD800` 不会导致整份物品模型资源解析失败；`ItemModelSelectProperty`、`ItemModelSelectWhen` 与 `ComponentSelectWhen` 保留原始属性名与条件分支，`ItemModelScalar::String(JavaString)` 保留原始单元，component 型 `select` 经 `ResourceJsonValue::to_nbt_summary` 与 `bbb_registry_codec::decode_persistent_component_select_value_from_source_with_resolver` / `decode_persistent_component_select_value_with_resolver` 直接取得标签（复合标签键同样保留原始单元）。
CustomModelData 的字符串列表从 source/NBT 到 typed patch 与 Dialog ItemBody 同样使用
`JavaString`。模型选择与点击 HashOps 读取原始单元，`NbtSummaryValue::String(JavaString)` 直接保存原始 UTF-16 单元，
`JavaString::with_utf8` 对合法文本直接借用或构造 UTF-8 而不额外复制 UTF-16 缓冲。网络编码先以同一遍历计算长度，
再直接写输出缓冲，保留 Netty 代理处理且不为每个字符串分配中间字节 Vec。
有界任意字符串的出站入口为 `Encoder::write_java_string`：先检查原始 UTF-16 单元数，
再调用共享 Netty 编码；CustomModelData 与 PotionContents 共用该入口。药水 custom_name
从 source、持久 NBT 到 typed patch 和 Dialog ItemBody 使用 `JavaString`；持久字符串不套用
网络字段的长度限制。模型匹配与 HashOps 读取原始单元，serde 统一输出 `\uXXXX` 转义的 JSON 字符串；保存和调试 SNBT 不从显示替换值恢复原文。
药水悬停名称由 `item_runtime::tooltip` 以原始 custom_name
或药水定义的名称后缀查询语言表，未设置时使用 empty；显式移除 PotionContents 则回到物品名。
短翻译键借用栈缓冲，命中后才生成显示文本，不经替换值查表。
`item_model_select_cases_keep_the_units_the_pack_wrote` 与
`resource_json_tags_keep_unpaired_string_units_and_refuse_such_keys` 钉住读取端。
文本玩家对象与持久化 profile 共用 `profile_components` 的借用输入解析和规范写出；
JSON/NBT 由 `RegistryElementValueRef` 直接读取，纹理派生信息只由 wire/UI 投影生成。
属性 map 最多 16 个键，合法多值和长值由 map 保存；list 形式沿用自身的条目和字符串限制。
profile 属性列表的 name/value/signature 使用 `JavaString`，贯通 JSON/NBT source、规范字段分组、
持久校验、dialog 物品 patch、HashOps 与原生 SNBT 导出；列表形式长度限制按 64/32767/1024
UTF-16 单元计算。规范 JSON 长度不使用 U+FFFD 投影。
属性网络字段用共享 Netty encoder 与 Java decoder，持久值和缓存键保持原始单元；
[皮肤信任机制](../security/client-chat-and-identity.md)说明签名校验边界。
`raw_profile_properties_reach_dialog_patch_without_replacement` 覆盖实际消费者。
HTTP session profile 的属性键、值与**玩家名**都保留原始单元：Gson 读 `\uXXXX` 不配对，
所以名字可能带任何单元，`NameToIdResponse` / `SessionProfileResponse` 都经共享
`JavaString` JSON helper 解析，`NameAndId` 与 `ResolvedGameProfile` 一路按单元携带，
registry source 使用同一实现。组件侧的 `ResolvableProfileSummary.name` 由
`ExtraCodecs.PLAYER_NAME`（`Codec.string(0, 16)` 加 `StringUtil.isValidPlayerName`）限死为
可打印 ASCII，是精确边界：带孤立单元的名字根本不是玩家名，`into_resolvable` 记为缺省而不是
折成一个 `U+FFFD` 名字，于是两种拼法不会互相冒充。
`http_profile_client_parses_loopback_name_and_session_profile_and_caches` 覆盖该链路。
属性谓词的 name/exact/min/max 与 `block_state_properties` 的键和值使用 `JavaString`，
source codec、持久投影、dialog 与 inventory HashOps 共用原始单元；实际方块属性查询只接受
能无损转换且存在于方块定义中的名字和值。JSON 映射通过 `serialize_java_string_map` 写出带 `\uXXXX` 转义键的 JSON 对象，不通过替换字符充当键。
`StatePropertiesPredicate` 和 `BlockItemStateProperties` 的字符串 wire 仍遵循 `STRING_UTF8`
替换规则；原始单元直接交给共享 Netty encoder，检查长度后才写入外部 buffer。
`BlockStateProperties` 只保存一份有序 JavaString 键值，重复原始键覆盖旧值但保留首次位置，
Map.equals 不依赖迭代序。JSON、NBT 和 wire 直接读写同一有序条目。
HashOps 对原始 UTF-16 键和值分别求哈希后排序，显示值不参与查等或哈希。
known-pack dialog 使用 registry codec 的共享临时 source-order 投影，保留原有标量规范化；
网络输入使用唯一 NBT 输入树的字段序，所有权见[连接与注册表](../protocol/client-connection-and-registries.md)。
实际 dialog、物品模型 select、掉落属性复制和放置消费者共享 typed map，引用切换时采用
被引用 registry entry 的真实来源。
JSON/NBT/wire 的孤立代理键碰撞、重复覆盖、树桶和保存恢复用例分别验证源序与最终覆盖值。
profile 属性 map、分数条件键与 `SetBlockProperties` 的属性 map 通过
原始 map accessor 保存 UTF-16；重复原始键先取最后一个可见值，再进行 codec 校验，
保持首个出现位置。JSON/NBT 入口测试区分孤立代理项、U+FFFD、问号与字面转义，
profile 和地图装饰的 JSON 写出长度也按原始单元计算。资源位置与固定字段仍使用各自校验。
地图装饰由 protocol 的共享 `parse_map_decorations_nbt` 解码，typed ID、Dialog 与服务端地图
运行时均携带 `JavaString`。重复键先按原始单元覆盖，再解码最终值；JSON 保留首次位置，
NBT 输入复用 JDK HashMap 迭代实现。source 投影通过现有 decode context 显式传递来源，
不会把所有 compound 当作 JSON 插入序。地图运行时按收到的顺序插入，覆盖旧值不移动位置；
借用普通文本查询不会新建 UTF-16 缓冲，实际图标网络发布不携带该内部键。
native 相等按原始键和 Java 浮点值比较，忽略 map 顺序；客户端 HashOps 另按原始 UTF-16
算键哈希并排序，与运行时迭代分开。规范 NBT 由 typed 值派生，copy/clear 只同步本组件缓存。
配方输入、真实合成、保存重读、slot wire、Dialog 和地图消费者的测试分别覆盖这条链，
包括互异孤立代理键、U+FFFD、问号、重复覆盖与普通文本。
物品模型 `ItemBlockState` 按资源中的原始属性名直接查询物品组件 map，不要求名字来自
实际方块定义（`ItemBlockState.get`）。pack 解析与 native select 共用 `JavaString`。
方块实体与 tag-query 的 NBT reader 共用 protocol 的 Modified UTF-8 解码器，键和值保存
`JavaString`；固定字段按原始键取最后写入值，资源字段仅作无损文本转换后校验。
未知原始字段不妨碍 skull 等已知字段读取。查询的 compact/pretty SNBT 使用共享引用规则，
孤立代理项以可重解析转义输出，字面反斜杠独立转义；compact 键按 Java 单元排序，
root 字段过滤按原始键精确匹配。banner 翻译键从 registry/component codec 到方块实体、
物品与 dialog 状态均为 `JavaString`；asset ID 保留资源标识符校验。持久 NBT 与 HashOps
消费原始单元，出站只在共享 UTF-8 encoder 边界执行替换；物品与织布机 tooltip
先按原始单元追加颜色后缀和查表，再生成显示。网络替换可能造成的 map 键碰撞与集合顺序
是独立合同，不能由 wrapper 的 Rust Eq/Hash 推断 Java 行为。
组件内旧版 `§` 格式码的调用面接线与消费者见[客户端字体与文本](client-font-and-text.md)。

## 生命周期和 fallback

点击命令有独立于主动输入的执行合同。`Screen.clickCommandAction` 调用 unattended 入口，
`ClientPacketListener.sendUnattendedCommand` 依次检查当前源解析、签名参数和无权限源解析。
只有无需确认的命令直接发送 unsigned packet；解析/权限确认接受后也发送 unsigned packet。
签名确认按 `ClientPacketListener.openCommandSendConfirmationWindow` 打开命令聊天编辑器或复制命令；取消返回
打开确认前的屏幕。BBB 的组件点击接入原生确认状态和共享 ConfirmScreen 布局：接受/取消
使用 generation 校验，模态输入阻断底层文本、IME、点击与滚轮，聊天父状态挂起后按结果恢复。
新的死亡、容器、书本或 level 状态替换旧父屏幕时，失效确认不能发送或恢复旧聊天。
dialog 静态、动态、正文和 Escape 命令使用同一入口；动态模板在触发时求值。
确认期间保留表单，取消不执行 `after_action`；接受或无需确认时才关闭、保持或进入等待，
等待计时从接受后开始。签名命令根据接受后的目标屏幕判断打开聊天还是复制，而非根据当前
是否存在 dialog 判断；CLOSE 返回已有世界或 native 父屏幕时复制并恢复，返回游戏时可打开聊天。
确认期间收到服务端 `ClearDialog` 仅记录包计数，保留隐藏表单；配置事件和游玩包都消费同一
当前屏幕判断。事件队列逐项同步失效状态，先到的容器替换不会留下影响后续清除的旧确认。
`ShowDialog` 将可见确认挂起，新 dialog 的关闭或等待返回会恢复原确认与表单；链接确认和
命令确认共享世界侧返回栈，命令及聊天父状态仍由 native input 拥有。恢复时更新屏幕版本关联，
连续清除包不能穿过已恢复的确认，容器/死亡等替换则丢弃挂起状态。表单控件值和编辑状态保留，
恢复的拖拽状态只在当前物理左键按下时驱动修改。游戏规则父屏幕保留未提交规则、无效整数输入、
丢弃确认状态和 native 搜索/选择状态；关闭 dialog 或等待返回时恢复同一 generation，容器或
level 替换则清理。按 `ClientPacketListener.handleGameRuleValues`，隐藏期间的规则响应只记录接收
事实，不交给父屏幕；原本仍在加载的页面返回后也不会被这些响应偷偷填充。
确认页旁白通过 simulation 输入效果、render publication 与 native narrator 接通：标题与完整正文
一起播报，不受视觉正文 15 行上限影响；实际播报时选择新焦点或悬停按钮，并使用各自的操作提示。
鼠标移动延迟 750 ms，按键和鼠标操作延迟 200 ms；隐藏时停止该页旁白，返回时按新的 presentation
重新播报完整内容，并保留父页已经播报过的焦点。可见确认页会抑制底层页面的旁白。
确认页仅在最后输入类型为键盘时初始聚焦 Accept，鼠标打开时没有焦点；无焦点且无悬停时
旁白提供屏幕导航提示。Tab/Shift+Tab 分别从首尾进入并循环，方向键沿两按钮的横向布局移动，
到边界后不循环；上下方向在已有焦点时保持原按钮。按键重复继续导航，无焦点时确认键不执行命令。
native、renderer 按钮高亮与旁白均消费同一个可空焦点。悬停由每帧 projection 使用当前物理指针
重新换算的 GUI 坐标计算，复用点击命中布局；页面打开、返回、缩放和指针离开都重新计算。
高亮与延迟旁白消费同一帧的悬停结果，simulation 不再保存独立的确认页悬停副本。
`Screen.init` 的立即旁白发生在下一次绘制之前：新确认页没有已绘制悬停，返回已有确认页时
使用该页上次绘制留下的悬停；随后才保存本帧结果供下一次返回使用。`GameRenderer.extractGui`
在绘制后执行延迟旁白，因此它使用本帧悬停。`Screen.resize` 只重新布局，不触发初始化旁白。
创建和恢复确认页时保存独立初始化快照，经有序 platform effect 消费；快照包含当时的焦点、
输入类型与时间，不会被同批后续按键或关闭操作覆盖。即使页面未进入 render publication，初始化
旁白也不会丢失；绘制只推进已收到初始化事件的同一 presentation，不从空快照或过期快照
重建初始化。确认页中的旁白快捷键请求和配置写入在每条输入命令后提交，避免被模态页面的
提前返回滞留。
`Screen.shouldRunNarration` 经 `GameNarrator.isActive` 检查 TTS 后端可用性，不检查旁白选项：
后端可用而模式为 Off 时仍更新 collector，由 `saySystemNow` 抑制发声；无后端时不选择控件、
不更新 collector，也不消费到期播报。重新启用的播报请求延迟 2 秒，并只收集尚未播报的内容。
`suggest_command` 使用 `JavaString` 保留原始 UTF-16 单元；组件 NBT 解码、样式继承、再编码和
HUD 点击到聊天输入替换都不提前转成 U+FFFD。`ExtraCodecs.CHAT_STRING` 的检查逐单元拒绝
控制字符、DEL 和 `§`，允许孤立 surrogate；输入框按 256 单元截断并把光标/选择移到末尾，
显示投影可以替换孤立单元，编辑规范值保持原样。静态 dialog action 也保留该值，并按原始单元
计算规范 JSON 长度；dialog 的 `insertText` 仍按 `Screen` 默认实现不修改聊天。
其余点击载荷现在用同一形状：`run_command` 与 `open_url` 的组件值、dialog 的静态
`run_command`/`open_url` 与 `dynamic/run_command` 模板（包括模板自己的字面段）都是
`JavaString`。`ExtraCodecs.CHAT_STRING` 只拒 `§`、DEL 与 C0，`ExtraCodecs.UNTRUSTED_URI`
交给 `java.net.URI`，而 `URI$Parser.scanEscape` 对非 ASCII 只问“不是 space char、不是 ISO
control”——孤立 surrogate 与 `U+FFFD` 同属该类，所以判定可以在投影上做，取值仍按单元保存。
唯一 URI parser 同次返回 `UntrustedUriIdentity`，`OpenUrl` 按此结构比较：scheme 和 server
host 只折叠 ASCII 大小写，raw path/query/fragment/userinfo/opaque 内容与 registry authority
只折叠 percent escape 的 hex 大小写，缺失和空值保持区别。server authority 解析失败且 raw
registry 字符合法时遵循 JDK `URI(String)` 的 fallback；IPv6 内 embedded IPv4 接受前导零，
保留 host 的原拼写，不把不同 IP 文本、默认 port 或 path 点段归一。原 URL 单元仍用于 NBT、
JSON 长度与确认页，并从真实 SystemChat wire decode 经过 HUD canonical target 传递到确认与打开请求。
`run_command` 经 `Commands.trimOptionalPrefix`（按单元去掉可选 `/`）与
`check_unattended_command_utf16` 到确认页，确认页画的是投影、发出去的是原始单元；
`open_url` 的单元一路到 `DialogLinkConfirmationState` 与 `PlatformEffect::OpenUri`，
到进程参数边界才按 JDK 编码把孤立 surrogate 写成 `?`。`open_file` 只可能来自客户端自造的
组件（`ClickEvent.Action.OPEN_FILE` 的 `allowFromServer = false`，序列化 codec 直接拒绝
服务端的这一动作），它按 `Screen.defaultHandleClickEvent` 交给 `PlatformEffect::OpenPath`，路径同样在
该边界编码。dialog 的 `single_option` 取值是 `SingleOptionInput.Entry.id`（纯 `Codec.STRING`），
选中项的 id 就是表单提交值（`InputControlHandlers.SingleOptionHandler.addControl`），模板替换与 `dynamic/custom`
的 tag 都按单元产出。
`copy_to_clipboard` 的组件和静态 dialog action 也保留 `JavaString`，允许控制字符和孤立
surrogate；规范 JSON 长度按原始单元及转义计算。聊天/书本点击和 dialog 消费同一原始值，
跨线程 `ClipboardWrite` 保留单元，到 native 剪贴板边界才按 Java `String.getBytes(UTF_8)`
把孤立 surrogate 替换为 `?`，并按 GLFW C 字符串规则在首个 NUL 截止；合法 surrogate pair
保持原字符。空载荷不发起写入，非空但以 NUL 开头的载荷仍写入空剪贴板。
协议入站文本由 `codec::java_utf8` 的同一扫描器实现 `Utf8String.read`
（`Utf8String.java:10-31`）：先按 `3 × 最大单元数` 收字节界限，Netty UTF-8 解码将畸形
序列替换为 `U+FFFD`，最后按 UTF-16 单元数收长度。`Decoder::read_string` 产出 Rust
`String`；`read_java_string` 直接产出原始单元容器，供书本文本使用，避免先分配 `String`
再复制为 UTF-16。两者共享畸形序列的消费规则与失败时的读取位置；网络 UTF-8 解码不会
产生孤立代理单元，持久 NBT 或资源 JSON 输入仍可保留它们。
聊天消息上限 256，unsigned/signed 命令包上限 32767，补全请求 32500，补全结果文字 32767，
签名参数名 16；这些是包字段界限，不能用聊天编辑器的 256 单元上限替代命令包限制。
编码中的消息/命令/参数名也按 UTF-16 计长。
`wire_strings_reach_the_suggestion_consumer_the_way_utf8string_reads_them` 从真实
clientbound 包解码出发覆盖这两条：畸形字节带着替换字符到达补全消费者，
16384 个星平面字符按 32768 单元越界（按 `chars` 只算 16384）。
出站聊天、命令及补全请求以 `JavaString` 通过 `NetCommand` 和聊天准备阶段；协议包复用同一
字段结构和编码器，入站默认 `String`，出站使用原始单元。`Encoder.write_utf16_string` 执行
锁定 Netty 的 surrogate 消费规则（与剪贴板的 JDK 编码不同），普通字符串也走同一个编码入口。
原生普通聊天提交在原始单元上规范化，并将同一值加入历史和发送队列；离线消息无签名，
签名命令包仍携带空参数签名列表和现有 last-seen 更新。命令解析与补全生成的原始值入口仍开放。
`WorldStore.check_unattended_command` 提供四种检查结果；无权限源在同一 parser 遍历中跳过受限
子节点，并保留 literal 优先级与 redirect 行为。检查原样解析已经去掉可选 `/` 的命令，
不再修剪空格或第二个 `/`。普通聊天补全仍沿用原有前缀归一化入口；确认接受后的 unsigned
发送不经过签名参数与 last-seen 消费路径，签名分支仅打开聊天或复制带 `/` 的命令。

## BBB 实现映射

### ChatScreen 文本与生命周期 owner

`ui-chat-screen-surface` 的唯一可变所有者是模拟层的 `ClientInputState`：它持有 `MESSAGE`/`COMMAND` 模式、256 个 UTF-16 单元的编辑值、最多 100 条近期历史、最新草稿、历史游标与缓冲、退出原因以及限制屏返回前的父状态。`Full`/`System`/`Hidden` 聊天权限仅从同代际 `ChatPresentationConfig.visibility` 查询；渲染器命中目标同时提供 Shift 扫描命中的 `target_index` 与普通扫描命中的 `click_target_index`，模拟层按当前 Shift 键选取其一并重新解析权威点击事件或插入继承的 `Style.insertion`，两者均校验世界、投递、资源与屏幕修订号。

渲染器仅接收有界富文本片段、草稿样式、限制提示/限制屏与命中目标，不持有点击或插入载荷；控制接口 `input.key`/`input.text` 走同一键盘与 IME 处理器，并从不可变模拟代际发布当前编辑值及历史/草稿/权限投影。屏幕旁白复用统一的延迟 `ScreenNarrationRuntime`。限制屏返回时恢复原 `ChatScreen`；其他屏幕替换以 `INTERRUPTED` 收口，成功提交以 `DONE` 收口，失败提交保留当前代际。端到端可游玩测试禁止使用 `net.send_chat_*` 捷径，分别验证 `Full` 下的草稿/历史/消息/命令闭环以及 `System` 下的拦截聊天与放行命令状态。

原生 `KeyboardInput` 先提交按键动作，再提交系统给出的完整修饰后文本；普通 Latin 字符与
独立 IME commit 共用文本 handler。释放键不提交文字，Ctrl 快捷键的控制字符不插入文本，
因此粘贴不会额外输入 `v`，输入法也不依赖按物理键猜字符。
Chat、告示牌和 Dialog 共用预编辑文本转换；composition 不改变已提交的输入或触发补全请求。
Chat 的预编辑浮层和原生候选区域共用当前滚动光标的投影，提交、关屏和焦点变化沿同一输入 owner 清理或重送。

Chat 与各单行/多行文本输入框共用 `input::text_field::{TextField, MultilineTextField}` 持有 raw UTF-16 value、cursor、selection 和持久 display start；
`input::chat_runtime` 连接键盘、IME、历史、草稿、归一化与补全，`input::chat_mouse` 连接鼠标。
公共 String/scalar 字段仅为显示和既有状态接口的派生投影，不能反向覆盖 raw edit。
命令方块与命令方块矿车编辑器复用同一 `TextField`，并保持不过滤（unfiltered）字体测量。

窗口 MouseInput/CursorMoved 经 simulation 按补全弹窗、聊天消息、EditBox 顺序分发。
点击使用 `floor(x)-4` 和当前 display start；Shift 保留 anchor，双击先求两个词边界再顺序滚动，
拖动允许越过输入框，释放和失焦终止拖动。同一 screen revision、相同按钮且间隔严格小于
250 ms 的已消费点击才参与双击；打开新 screen 不继承旧 Chat 的点击记录。
依据为 `ChatScreen.mouseClicked`、`MouseHandler.onButton`（双击判定）与 `EditBox.findClickedPositionInText`。

显示起点按 `EditBox.scrollTo` 的原条件顺序更新。UTF-16 位置可以落在 surrogate 对中间；
正向宽度查询与反向尾部查询分别执行各自的字符分解和浮点比较，不将位置对齐到 Rust scalar。
renderer 接收 `HudChatInputView`，在 raw display/cursor 边界切分后分别转换显示片段，因此半对
可分别显示 U+FFFD。输入、后缀、cursor 和选区范围使用同一 filtered glyph map。
`setValue`、插入、删除、全选分别保留各次 cursor/selection 滚动顺序，不能由最终位置重算代替。

命令语法着色的唯一来源是同一份本地 parse：`input::chat_runtime::chat_input_syntax_state` 只在值以
`/` 开头时（`CommandSuggestions.updateCommandInfo`）把 `world.parse_command` 的 argument 范围与未消费
cursor 转成 raw value 的 UTF-16 区间（`HudCommandInputSyntax`），renderer 的
`hud::text_layout::hud_command_input_runs` 复刻 `CommandSuggestions.formatText`：literal 灰
`0xAAAAAA`、AQUA/YELLOW/GREEN/LIGHT_PURPLE/GOLD 五色按 argument 顺序循环、未消费尾部红 `0xFF5555`、
其余回到 literal，最后 `EditBox.applyFormat` 的先注册者优先让 draft 的灰色斜体
（`ChatScreen.init` / `ChatScreen.formatChat`）压过语法色。renderer 不重新解析；区间是 raw value 的 UTF-16 单位，
display start 与 head/tail 切分仍走上面 `EditBox.extractWidgetRenderState` 的既有路径，因此 surrogate 对中间
的切分和滚动后的偏移与未着色时一致。

argument 集合只取最内层 redirect context：`formatText` 读的是
`currentParse.getContext().getLastChild().getArguments()`（`CommandSuggestions.formatText`），而
redirect 或 fork 的子节点会新建一个 `CommandContextBuilder` 并以跳过分隔符后的 cursor 继续
parse（`CommandDispatcher.parseNodes`；`ExecuteCommand.register` 的 `run` redirect 到
dispatcher root，`addConditional` 以 `fork` 回到 `execute` literal），所以最深一次 redirect 之前
解析出的 argument 属于外层 context。本地 parser 在同一趟 parse 里随该次下降记录
`CommandParseResult::innermost_context_argument_start`（`arguments` 中首个归最内层 context 所有的
下标，更深的 redirect 会覆盖为更后的边界，正好对应 `getLastChild()` 走的 child 链），
`/execute ... run <cmd>` 因此只染 receiver 命令自己的 argument，外层 argument 保持 literal 灰；
无 redirect 的路由该下标为 0，整条扁平列表就是唯一 context。
同名参数仍按 Brigadier `LinkedHashMap` 的插入位置与最后范围折叠（`CommandContextBuilder.withArgument`），这发生在同一个上下文内，属于 redirect 之前的原始源分段阶段：切片与折叠均直接在原始值的 UTF-16 单元上进行，早于[客户端字体与文本](client-font-and-text.md)中 `RichTextComponent.source_styled_parts` 保留的源分段之后才执行的 bidi 重排与按字体切片（`SubStringSource.substring`、`FormattedBidiReorder.reorder`），因此 bidi 重排与字形合并均不会改变参数序号或范围。名字的最后一个范围排在后插入名字之后
（`a b a`）时 map 顺序会回读，`formatText` 的 `text.substring(unformattedStart, start)`
（`CommandSuggestions.formatText`）会因起点后于终点抛异常，本地丢弃该范围让其保持 literal，
不重复着色。

输入头尾与后缀绘制后，`hud::chat_input` 提交同一区域的白色反色和蓝色叠加，cursor 最后绘制。
`HudGuiPipelines` 统一拥有无纹理 `core/gui` 的普通填充、反色和叠加管线：反色 RGB 的源/目标系数为
`ONE_MINUS_DST_COLOR`/`ONE_MINUS_SRC_COLOR`，alpha 替换；蓝色层的 RGB/alpha 均为 `ONE`/`ONE`
相加。白字选中后呈蓝色，背景保留反色结果，不以蓝色实心矩形遮字。依据为
`GuiGraphicsExtractor.textHighlight`、`RenderPipelines.java:648-655` 与 `BlendFunction.java:16-22`。
两次 draw 共用 scissor；空裁剪同时跳过，随后 sprite 恢复自己的绑定，draw counters 计入两次绘制。

资源准备从 `ShaderSourceCatalog` 投影 `minecraft:core/gui` 的两个 stage；任一 stage 被资源包
覆盖时，`core_shaders::gui` 适配双方。顶点先从 HUD NDC 还原 GUI 像素，再执行资源包的位置
运算、Y 向下的正交投影与 z=-11000 的 ModelView；片段保留颜色运算和零 alpha discard。
16 字节 GUI 尺寸 uniform 随绘制更新，resize 不创建逐帧 GPU 对象。三条 pipeline、bind group 和
buffer 作为一个 owner，由 `hud::HudGuiPipelines`（`CorePipelines.hud_gui`）准备，经 `transactional_shader_resource_reload` 原子交换并在失败时回滚；移除覆盖
恢复内置管线。GPU 测试覆盖实际资源 stage 的替换、编译失败、提交后失败、恢复与位移。

`set_gui_viewport` 在启动与每次非零 resize 时从 `SimulationInputRuntime` 接收 ceil 布局尺寸与
整数 GUI scale；GUI scale 只有这一个 owner。
布局和鼠标继续使用 ceil；`HudDraws::project_vertices` 在统一上传前将布局 NDC 转到
physical size/guiScale 的浮点投影，所有共享 HUD 顶点只转换一次。GUI item/map 相机与选区
uniform 使用同一浮点 extent；scissor 直接乘整数 scale，再裁到 attachment。依据为
`Window.setGuiScale`、`GuiRenderer.draw` 与 `GuiRenderer.enableScissor`。
physical resize 即使未改变 ceil 布局也刷新相机，零尺寸保留已接受状态；强制 Unicode 的极小窗口
允许投影 extent 小于 1。整帧 GPU 用例覆盖奇数 framebuffer、相同 ceil 的 resize、整除尺寸、
文字/选区与地图相机，资源重载用例另验证资源包顶点位移按整数 scale 映射到物理像素。

普通填充使用源 alpha/反源 alpha 混合 RGB，alpha 使用 `ONE`/反源 alpha；仅 alpha 恰为零时
丢弃。`hud::sprite_draws` 的显式 Fill 入口没有纹理参数，支持纯色、上下渐变和变换后的浮点角点；
`render::hud_commands` 按提交顺序切换管线与绑定，保留裁剪和 GUI item/blur 分层边界。
Chat 输入与消息背景、文本衬底、计分板/玩家列表、调试图表、库存 fill layers、耐久/冷却条、
进度线、望远镜外部遮罩、睡眠遮罩和暂停/死亡渐变已消费该入口。资源包 `core/gui` 覆盖影响
这些填充，字体 atlas、下划线/删除线、图片和缺图纹理后备继续使用各自的纹理路径。
依据为 `ChatScreen.extractRenderState`、`GuiGraphicsExtractor.fill`、
`GuiGraphicsExtractor.itemBar` 与 `BlendFunction.java:10-12`。

共享单行输入和书名在文字后绘制选区，再绘制光标；书页、多行输入和告示牌在光标后绘制选区。
告示牌在字体宽度的整数除二之后施加浮点 pose，选区和插入光标保留分数角点，避免 ceil 扩大覆盖；
append cursor 随当前行提交，insert cursor 与选区随后提交。依据为
`AbstractSignEditScreen.extractSignText` 与 `MultiLineEditBox.extractContents`。
实际 GPU 读回覆盖普通填充的零/低/半透明 alpha、渐变、跨 sprite/selection 管线切换、空裁剪恢复、
资源覆盖提交/回滚，以及书页、书名、共享输入和两类告示牌的文字/选区/光标像素。
书页与共享多行输入复用选区几何：跨越行尾的选区延伸到控件宽度减 4，换行符和空行保留
高亮，最终由控件内缩 1 像素的 scissor 与父裁剪相交。可见性使用滚动后的 9 像素行框，
首个可见且包含 cursor 的行绘制插入光标；行尾 `_` 在最后可见行后提交，不受最大长度触发切换。
书页固定使用 122×134 控件、14 行上限；共享输入恢复父裁剪后先绘制滚动条，再绘制计数。
依据为 `AbstractTextAreaWidget.extractWidgetRenderState`、`AbstractTextAreaWidget.withinContentAreaTopBottom` 和
`MultiLineEditBox.extractContents`。定向测试覆盖换行/空行延伸、零/负宽度、换行边界光标归属、
滚动与父裁剪、书页反向选区及选中 GUI shader 的实际像素。
共享确认布局用于死亡与暂停屏 URL 等当前范围内的确认页面，保留完整标题与限定行数的说明。官方玩家举报整套流程已按产品范围裁决移除，范围与重开条件见[客户端账本](../../../unsupported-client-features.md#official-player-reporting)，不在本页聊天编辑与确认机制范围内。

server completion 的 start/length 保持协议的 UTF-16 单位；本地 dispatcher 的 UTF-8 字节范围
只在输入边界转换一次。替换拼接完整 raw value 的 prefix/suffix，保留光标后的内容，随后按
`CommandSuggestions.SuggestionsList.useSuggestion` 执行 setValue、cursor、selection。
请求文本为触发编辑时的原始 UTF-16 光标前缀；请求状态、重复请求去重与服务端响应匹配
均比较原始单元，孤立代理项不会与 U+FFFD 混同。本地 parser 的补全候选匹配按单元比较：
字面名一侧读 `SuggestionsBuilder.getRemainingLowerCase`（整串转小写后切出的尾段），
provider/registry 一侧读 `getRemaining().toLowerCase(ROOT)`（尾段本身），两侧都是原始单元，
候选文本与补全列表保存的 `text` 也是 `JavaString`。
纯键盘/鼠标光标移动不产生 onValueChange，也不重建既有补全。
新编辑的请求 ID 使旧响应失效；无效范围有界拒绝。依据为 `EditBox.moveCursorTo`、
`CommandSuggestions.updateCommandInfo` 与 `ClientSuggestionProvider.customSuggestion`。

recent、history buffer 和 draft 保存 raw UTF-16。提交先按 Java `trim` 删除两端 `unit<=0x20`，
再以 Character whitespace 合并空白、逐个把 NBSP 换成空格，再次 trim，最后 raw 截断 256 units；
截断不修复孤立 surrogate。显示投影不替代历史原文。普通聊天经 `JavaString` 进入发送队列；命令删除归一化原文的首个 `/`
单元后，以同一原文进入 unsigned 或 signed 命令队列，由共享 raw encoder 执行 Netty 字节语义。
命令包分类与可签名参数值保留原始单元；`WorldStore::parse_command_utf16` 经 `commands::dispatcher` 的 UTF-16 核心解析，客户端 `input::chat_runtime` 消费同一结果。补全的字面名匹配、provider/注册表候选匹配与候选文本也按原始单元比较。
共享 Brigadier 字符串扫描只有一个 UTF-16 实现：裸词按 ASCII 字符集停止，引号内保留任意单元，
仅允许转义当前引号或反斜杠；非法转义的位置与原始错误单元保存在 `CommandSyntaxErrorUtf16`。
旧 `&str` API 只负责输入编码与字节位置映射，错误展示才使用字符串投影。行为对照锁定的
Brigadier 1.3.10 `StringReader.readString`/`readStringUntil`。数字词法也复用 UTF-16 扫描，
仅消费 `isAllowedNumber` 的 ASCII 数字、点和减号；数值格式、范围校验仍由参数 owner 执行。
通用 token 扫描按字面空格停止，保留 tab、其他 Unicode 空白及孤立 surrogate；旧入口仍映射为
字节位置。资源标识符和通配选择器也复用 UTF-16 扫描，只有已确认的 ASCII 片段才转换为
标识符字符串；孤立 surrogate 和其他非法单元留给调用方，标识符校验失败回到起点。
选择器允许空模式，补为 `minecraft:` 后由注册表匹配决定是否失败；缺少注册表不能提前判定。
依据 `Identifier.read` 和 `ResourceSelectorArgument.parse`。
方块属性和物品组件补丁的 `skipWhitespace` 复用与 SNBT 相同的 Java 字符表，
接受 U+001C、U+1680、U+2000 等空白，拒绝 NBSP、U+2007、U+202F 和孤立 surrogate。
物品组件列表先检查 `]` 再跳过本轮前导空白，因此 `[]` 有效而 `[ ]` 无效；方块属性列表
则先跳空白，`[ ]` 有效。顺序依据 `ItemParser.State.readComponents` 与 `BlockStateParser.readProperties`，
组件值返回调用方并完成 codec 校验后，下一次扫描才读取分隔符；非法值错误优先于缺失
分隔符、未闭合列表和后续未知组件，错误位置指向值起点，依据 `ItemParser.State.readComponent`。
`item_component_value_error_precedes_separator_error_and_points_to_value` 经过实际 registry-aware
物品参数解析器验证该顺序。`CommandItemComponentPatchScannerUtf16` 在同一原始缓冲区上
逐项读取资源名、空白和 SNBT，组件名起点、值起点、错误及结束位置均按 UTF-16 单元计。
旧字符串入口只拥有一次编码和位置映射，不保留第二套扫描逻辑；值仍交给原组件 codec 校验。
`raw_item_component_input_reaches_codec_without_projection` 覆盖原始/转义 surrogate 拼成键、
孤立 surrogate 值、删除组件和未读后缀。物品谓词的头部及逐项测试也复用 UTF-16 核心，
标识符必须非空；标点、标识符及否定号后按 Java 空白规则扫描，保留等值和子谓词中的原始 SNBT。
客户端 codec 在读取下一项分隔符前校验当前值，错误位置使用值末尾；这与物品组件补丁的
值起点规则不同，依据 `ComponentPredicateParser` 的 `test` 规则及 `ItemPredicateArgument.ComponentWrapper.decode`。
`item_predicate_whitespace_and_codec_error_order_match_grammar` 验证含空白的条件组合及错误优先级，
`raw_item_predicate_value_reaches_component_codec` 验证原始值进入组件 codec。dispatcher 经共享 UTF-16 核心调用参数 scanner。
`~` 的名称解析先查专用子谓词，再回退到任意已注册组件的存在检查，包括临时组件；
依据 `ItemPredicateArgument.Context.lookupPredicateType` 与 `createComponentExistencePredicate`，
回退值使用 `Unit.CODEC`，接受 compound 并忽略其字段。裸组件存在检查和 `=` 仍只接受
持久化组件；物品组件补丁也不允许临时组件。补全的运算符及值后分隔符使用同一名称判定，
但名称候选仍遵循 Vanilla 的持久化组件与专用谓词列表。
`item_predicate_transient_component_existence_uses_unit_codec` 和
`command_item_predicate_completion_accepts_component_existence_fallback` 覆盖解析、codec 与补全边界。
谓词的未知物品、标签、组件和子谓词错误指向标识符末尾，且不包含其后空白；
`ResourceLookupRule.parse` 在读取标识符后调用 `ItemPredicateArgument.Context` 的 lookup，
构建异常时不回退 reader。物品堆叠参数的 `ItemParser` 则主动回退到名称起点。
`item_predicate_lookup_errors_point_after_identifier` 覆盖真实 registry-aware 路径及 UTF-8 适配位置，
`raw_item_predicate_lookup_error_uses_identifier_end_before_whitespace` 覆盖原始 UTF-16 位置。
终结符不在此列：`StringReaderTerms.TerminalCharacters.parse` 的每个字符终结符先 `skipWhitespace` 再取 mark，
所以只有 `~` 那条拒绝落在名称后的空白之后。

### 物品谓词 Packrat 语法差分

物品谓词分支按真实注册表逐输入对照 `ItemPredicateArgument`
（`ItemPredicateArgument` 构造、`ComponentPredicateParser.createGrammar`）：
15857 行语料里 9736 行整条相同（接受/拒绝、游标与被选中的那条拒绝），其余 6121 行落在
物品/标签注册表与组件 codec 这条既有边界上，只断言本仓库不会比 Vanilla 更早拒绝。
`vanilla_grammar_differential` 的 `Refusal` 逐条命名 `ErrorCollector.LongestOnly` 留下的那一项。
差分改掉了三处：`tag` 规则（`TagParseRule.parse`）现在把 SNBT grammar 自己的最远失败原样带出，
`~` 后名称未知时报的是 `componentType` 那条（两条存在同一个 mark，组件查找先存），
以及临时组件（codec 为 null、只有 `~` 的存在检查回退接受）报 `~` 终结符而不是未知组件。
候选顺序由 `ComponentPredicateParser.createGrammar` 中 `test` 的三个备选决定：裸名称解析成功后，第一备选在同一单元上
留下的 `=` 仍比分隔符靠前，所以 `stick[damage` 答 `=` 而 `stick[damage=1` 答 `|`。
`ScannedCommandItemPredicateTerm` 带上 `negated` 与 `alternative`，把 `term` 的 `!`
与 `alternatives` 的 `|`（相对 `conditions` 的 `,`）分组保留给调用方。
`arguments.item.predicate.unknown` 不可达：`ItemPredicateArgument.Context.lookupPredicateType`
对任何已注册组件都回退成存在检查，所以每个 `name~value` 的名称都是组件名。

方块属性及尾随 compound 由 `scan_command_block_body_utf16` 的单一 UTF-16 核心读取，
旧字符串 API 仅适配编码与位置。客户端通过逐项回调，在 `=` 前查询属性定义，
在分隔符和尾随 SNBT 前校验值；未知名称、重复名称及非法值分别使用名称或值的起点。
依据 `BlockStateParser.readProperties`，直接尾逗号可接受，逗号后空白再闭合会进入下一次
属性读取。标签的 vague 属性不查询具体定义，允许空名称/空值；缺少赋值号回到名称起点，
读取值后直接 EOF 回到值起点，未知标签回到 `#` 起点（`readVagueProperties`、`readTag`）。
注册表/状态定义尚未收到时仍保留未完成验证状态。
`command_block_properties_validate_before_following_syntax` 覆盖实际属性定义和错误顺序，
`block_predicate_resolves_tags_but_keeps_vague_properties` 覆盖标签边界。
方块标识符也复用 `scan_command_block_identifier_utf16`，保留标签限制、名称和未读后缀位置；
`raw_command_block_identifier_preserves_cursor_and_unread_units` 覆盖 raw 与旧字节接口。
完整 dispatcher 由 `WorldStore::parse_command_utf16` 提供原始入口。

资源引用或内联值参数使用 `scan_command_resource_or_id_utf16`：先按 Java 空白规则定位，
优先尝试非空资源标识符，再读取原始 SNBT；因此 `true` 和 `012` 是引用，带引号的同名值则是
内联字符串。旧字节接口仅做编码和位置映射。依据 `ResourceOrIdArgument.createGrammar`、
`ReferenceResult.parse` 与 `InlineResult.parse`，查询和 codec 异常使用已消费参数的末尾位置。
可选注册表不存在时 Vanilla 返回 null，仍允许语法解析成功；存在注册表时继续执行查询或 codec。
`raw_command_resource_or_id_preserves_values_and_identifier_priority` 覆盖原始值和选择顺序，
`command_resource_or_id_errors_use_end_and_raw_values_reach_dialog_codec` 覆盖实际查询、错误位置和
原始 dialog 标题进入 codec；`unavailable_optional_lookup_matches_vanilla_null_parse_result` 保留缺少可选注册表的行为。
该候选顺序已与 `Identifier.readNonEmpty` 加 SNBT 第二候选逐输入差分一致；该参数由 dispatcher 的原始单元入口消费。

文本和样式参数由 `scan_command_text_argument_utf16` 读取原始 SNBT，再由共享 component/style
codec 校验；`CommandArgumentParser.withCodec` 在 codec 失败时回到参数起点，包含起点后的前导
空白。粒子选项由 `scan_command_particle_options_utf16` 读取，仅紧邻名称的 `{` 启动选项，
中间空白不被跳过；扫描器不接收必填开关，无选项时统一返回空结果，所需字段只由 codec 判断。
`ParticleArgument.readParticleType` 的未知名称异常使用名称末尾位置。
`command_raw_text_and_style_reach_codec_and_errors_rewind_to_start` 覆盖原始文本与 insertion、错误回退，
`command_particle_raw_item_options_reach_codec_and_unknown_name_ends_at_cursor` 覆盖原始物品组件值进入 codec，
`raw_command_particle_options_keep_compound_adjacency_and_positions` 覆盖相邻语法、原始值及旧接口字节位置。
粒子选项 codec 的异常没有 reader 上下文：`CommandSyntaxError.cursor` 使用可选字节位置，
codec 错误序列化为 `null`，语法错误仍保留数值位置。dispatcher 只为存在的位置加回命令前缀。
缺少选项时先将空对象交给 codec，所需字段缺失也产生无上下文错误，依据 `ParticleArgument.readParticle`。
`command_particle_codec_error_has_no_context_after_dispatch_and_serialization` 覆盖命令树解析、前缀偏移、
缺失/非法参数与 JSON 往返；dispatcher 保留原始输入和参数跨度。

NBT 路径的引号名称及列表下标复用共享 Brigadier 字符串/数字扫描器，删除独立读取逻辑。
依据 `NbtPathArgument.parseNode` 和锁定 Brigadier `StringReader.readInt`/`readStringUntil`，
无效或溢出下标回到数字起点，合法整数后的非法终止符指向终止符；无效转义指向转义字符，
空引号名称指向闭引号之后。客户端参数扫描直接保留 `CommandNbtPathParseError.offset`，
加回参数起点，不再把所有错误压到路径开头。
`command_nbt_path_errors_retain_reader_positions` 覆盖小数/溢出/重复负号下标、分隔符、引号与 UTF-8 前缀。
路径的名称、源文本与节点取值现在都是缓冲区自己的单元：`CommandNbtPathNodeKind` 的名字是
`JavaString`，`parse_command_nbt_path_value_prefix_utf16` 按单元返回消费长度与错误偏移，
旧 `&str` 入口只把这两个数映射成字节，`CommandNbtPath::source()` 仍答投影文本供消息使用。
名称匹配与 `getOrCreate` 建键均保留 `NbtSummaryEntry.name` 的原始单元，因此孤立代理项的名称不与 `U+FFFD` 键相等，NBT 路径读入与建键均不折叠代理项。实体选择器同理按单元回放选项：名字、条件取值与 `nbt=` 中的 SNBT 均不再经过 `String` 往返（`parse_command_entity_selector_prefix_utf16`），消息中的内嵌选择器也按单元推进。
`selector_conditions_are_read_in_the_buffers_own_units` 的每个计数取自锁定 client-26.1 jar 的
`EntitySelectorParser` 实跑，`WorldStore::parse_command_utf16` 直接输出参数跨度与 `JavaString` 取值。
普通 NBT 与 compound 参数通过 `scan_command_snbt_argument_utf16` 复用共享原始 SNBT 解析器，
分别允许任意 tag 或要求 compound，消费位置按 UTF-16 单元返回；原有参数入口只映射字节位置。
这对应 `NbtTagArgument` 的 `SnbtGrammar` 和 `CompoundTagArgument` 的 compound parser。
`command_snbt_raw_arguments_keep_kind_and_consumed_boundaries` 覆盖孤立 surrogate、前导空白、
类型拒绝、未读后缀及客户端旧参数入口。SNBT 语法失败的完整 packrat 错误位置已按 Vanilla parser 逐输入差分核验，见上文差分基线。
目录型参数由 `scan_command_catalog_argument_utf16` 统一读取：槽位名消费到普通空格，
镜像、旋转、颜色、显示槽位、实体锚点、游戏模式与高度图读取 Brigadier 未加引号字符。
成员判定复用生成目录；错误保留原始单元，实体锚点恢复参数起点，其余使用读取结束位置。
依据 `SlotArgument.parse`、`SlotsArgument.parse`、`StringRepresentableArgument.parse`、
`ColorArgument.parse`、`ScoreboardSlotArgument.parse`、`EntityAnchorArgument.parse` 与
`GameModeArgument.parse`；高度图只接受 `HeightmapTypeArgument` 的保留类型及小写名称。
dispatcher 的原始 UTF-16 入口消费同一参数结果，客户端旧入口仅做字节映射。
轴集合通过 `scan_command_swizzle_argument_utf16` 按单元读取，遇到非法或重复轴时，
游标位于刚读取单元之后，对应 `coordinates/SwizzleArgument.parse`；旧入口只映射字节位置。
记分板运算符对应 `OperationArgument.parse`/`getSimpleOperation`：输入已耗尽时报上下文错误，
有剩余输入但运算符不合法时不附带游标。`command_swizzle_raw_errors_and_operation_context`
覆盖这些边界及客户端参数入口。
资源位置、维度和函数参数经 `scan_command_resource_location_argument_utf16` 复用同一标识符
扫描器。函数标签先消费 `#`，因此非法标识符的错误起点在 `#` 之后；对应
`item/FunctionArgument.parse` 和 `Identifier.read`。后者允许空路径，与 `readNonEmpty` 分开。
`command_resource_argument_raw_scanning_and_tag_error_start` 覆盖原始单元、空路径、标签错误
位置及维度/资源参数的客户端入口。
整数与浮点范围通过 `scan_command_range_argument_utf16` 共用原始单元扫描；数字片段只接受
数字、负号和非范围分隔符的小数点。对应 `MinMaxBounds.Bounds.fromReader`，两端的格式错误
均恢复整个范围起点；空范围和反向范围同样使用该起点。整数按 i32、浮点按 double 解析；
后者按 Java `Double.compareTo` 区分正负零，`0..-0` 为反向范围。
`command_range_raw_boundaries_and_endpoint_error_rewind` 覆盖开放端点、孤立 surrogate 后缀、
右端点格式错误、整数溢出和客户端入口错误位置。
角度和时间参数分别经 `scan_command_angle_argument_utf16`、`scan_command_time_argument_utf16`
复用原始 float 词法。依据 `AngleArgument.parse`，空输入为不完整角度，普通空格处可得到零角度，
相对前缀 `~` 后仍按原始单元读取。时间依据 `TimeArgument.parse` 先以 float 乘单位，再按
受管 JDK `Math.round(float)` 的位运算取整，避免先加 `0.5f` 导致的二次舍入；最后检查最小值。
`command_angle_time_raw_units_and_float_rounding` 覆盖孤立 surrogate、单位错误及
`0.49999997t`、`8388609t`、负半整数等精度边界，并检查客户端参数入口。
坐标和 UUID 前缀分别经 `parse_command_coordinates_prefix_utf16`、`parse_command_uuid_prefix_utf16`
读取原始单元，旧入口复用同一核心。坐标数字复用 Brigadier 数字词法，绝对整坐标按 i32、
相对和局部坐标按 double；UUID 只将已扫描的 ASCII 片段交给 Java UUID 兼容解析。
依据 `WorldCoordinate`、`WorldCoordinates`、`LocalCoordinates` 与 `UuidArgument.parse`；
`command_coordinate_uuid_prefixes_preserve_raw_suffixes` 覆盖原始后缀、混合坐标拒绝及客户端入口。
坐标详细错误由 `scan_command_coordinates_argument_utf16` 统一产生，客户端参数入口直接保留
错误而不再经 Option 折叠：缺少轴间分隔符恢复参数起点，数字失败保留数字起点；世界坐标中
遇到 `^` 使用当前轴位置，局部坐标中遇到世界轴恢复参数起点，缺失末轴使用读取结束位置。
二维位置、列位置和旋转的空输入按各自 argument 的不完整参数处理。
`command_coordinate_errors_keep_axis_or_reader_start` 覆盖这些区别及客户端字节位置映射。旧 `Option` API 仅保留前缀判定用途；完整命令分发器使用原始 UTF-16 输入与错误位置。

GUI 尺寸变化先保存当前原始值为初始值（`initial`），再重建输入框并从显示起点 0 执行 `setValue`；光标与选区移至末尾，历史位置回到近期长度，历史缓冲与草稿标记保留。资源测量投影更新本身不滚动。完整重载的有效确认从已保存初始值重建当前聊天框，过期确认不重建，对应 `LoadingOverlay.tick` 调用 `screen.init` 而非 `ChatScreen.resize`。非 Shift 的消息点击事件消费成功时更新初始值，Shift 插入与补全弹窗点击不更新它。

已签名书标题、过滤标题与作者的权威值为 `JavaString`，对话框物品投影使用相同原始单元，摘要与对话框序列化统一输出 `\uXXXX` 转义的 JSON 字符串。物品模型的成书/书与笔精确匹配及书标题、作者的部分谓词，借用 `ResourceJsonString` 的单元迭代与实际值比较，不分配显示字符串或混同 `U+FFFD`。组件 `HashOps` 复用 `bbb_registry_codec::item_hash::ops::{hash_string, hash_java_string}` 的类型标记、长度与小端 UTF-16 单元编码，再按既有映射规则排序。书标题、作者悬浮提示与铁砧预填先经共享 `is_minecraft_blank` 查询原始值（`StringUtil.isWhitespace`），再在显示出口派生文本；作者直接写入最终翻译字符串，不增加临时显示缓存。调试 SNBT 经原始 NBT 写出器保留代理转义，probe 报告才作显示转换。

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| UTF-16 `JavaString` 书本/NBT/JSON/语言保留 | `StringTag`、`Gson`、`Language`、`Utf8String` | `bbb-protocol`、`bbb-client-assets`、`bbb-client-world` | `aligned` | Netty/JDK/Gson 差分与原始单元往返测试 |
| SNBT 与命令参数 Packrat 语法及最远拒绝 | `SnbtGrammar`、`ItemParser`、`ComponentPredicateParser` | `nbt_summary::snbt`、`commands::syntax` | `equivalent` | `vanilla_differential` 与 `vanilla_grammar_differential` 逐输入差分表 |
| `ChatScreen` 编辑、历史、草稿、语法着色与补全 | `ChatScreen`、`EditBox`、`CommandSuggestions` | `bbb-client::input::chat_runtime`、`hud::text_layout` | `aligned` | 编辑/滚动/着色/补全与 GPU 读回聚焦测试 |
| 点击命令/URL/剪贴板确认页与旁白 | `ClientPacketListener`、`Screen`、`ConfirmScreen` | `bbb-client::input`、`ScreenNarrationRuntime` | `aligned` | 代际失效、悬停/焦点旁白与 Shift 插入测试 |

## 结构性不变量

- `ChatScreen` 渲染器与控制接口不得恢复独立编辑缓冲、权限、点击或历史/草稿所有者。
- 命令语法区间只能来自当前值的当次本地解析最内层 redirect 上下文；渲染器不解析世界、不重排参数顺序，也不得让语法色压过草稿格式器或改写既有的显示/光标切分。
- 规范 NBT/JSON/书本/聊天草稿的 `JavaString` 原始 UTF-16 单元与显示投影（`U+FFFD`）严格分离，网络 UTF-8 替换规则只在协议线编解码边界生效。
- 确认页提交与父屏幕恢复必须校验屏幕、世界与资源代际，模态输入期间阻断底层交互。

## 聚焦测试（focused tests）和真实证据

SNBT 与命令语法的差分证据来自 `vanilla_differential` 的八张表（`numbers_that_only_a_prefix_grammar_accepts`、`whitespace_is_skipped_by_every_terminal`、`compounds_lists_and_arrays`、`strings_and_escapes`、`builtins`、`rejected_tokens` 等）加 `every_refusal_key_the_grammar_throws`、`flattened_codec_accepts_only_a_compound_root`、`depth_bound_is_a_named_divergence`，以及 `vanilla_grammar_differential` 的物品堆叠、物品组件值、资源引用/内联值与 `Identifier` 词法四张表；每行同时钉住 Vanilla 的期望结果。

命令语法着色的聚焦证据包括 `chat_input_command_syntax_colors_arguments_and_cycles_after_five`、`chat_input_command_syntax_marks_the_unread_tail_and_returns_to_literal`、`chat_input_draft_style_outranks_a_parse_the_screen_still_holds`、`chat_input_command_syntax_geometry_follows_scrolled_offsets_and_split_pairs` 与整帧 `chat_input_whole_frame_draws_parsed_argument_colors_and_unread_red`（GPU 读回断言五色与红色实际像素）；输入侧由 `chat_input_syntax_uses_utf16_units_of_the_local_parse`、`chat_input_syntax_marks_the_unread_tail_and_skips_plain_messages`、`chat_input_syntax_follows_the_context_argument_map_order`、`chat_input_syntax_colours_only_the_innermost_redirect_context`（`/execute if … run cmd …` 形状的命令树经真实 `apply_commands` + `parse_command` + 投影，断言外层参数不进着色区间、无 redirect 时区间不变、未读尾部仍红）与 `chat_input_syntax_keeps_the_parse_while_a_draft_is_shown` 覆盖真实按键与 IME 路径；边界本身由 `bbb-client-world` 的 `parse_records_the_innermost_context_argument_boundary` 固定为 `arguments` 下标。

## 当前差异

`ChatScreen.mouseClicked` 的 Shift 插入返回未消费，同一次点击继续进入 `EditBox`。`apply_chat_component_interaction` 保留这一顺序：先插入原始 UTF-16，再分派光标、选区与拖拽，不更新初始值；非 Shift 点击事件按原契约消费并更新初始值。`shift_insertion_target_inserts_and_a_plain_click_falls_through` 从真实模拟鼠标命令断言插入、选区锚点、拖动、释放与初始值。

本地字面名、字符串提供者、资源与可召唤实体提供者在构造候选前，按原始 UTF-16 丢弃与完整 `remaining` 大小写敏感相等的文本，前缀匹配仍使用各自的小写规则。这对应锁定的 Brigadier 1.3.10 `SuggestionsBuilder.suggest(String)`，不改变整数重载、服务端补全响应、替换起点或排序。带 `#`/`!#` 的资源候选先比较完整前缀与资源名再分配文本；`resource_candidates_cut_the_tail_at_the_common_prefix` 固定前缀与精确相等边界。

`MAX_NBT_DEPTH = 64` 作为 [AGENTS.md](../../../../AGENTS.md) 要求的有界安全预算有意保留，不构成未关闭功能缺口。[客户端账本](../../../unsupported-client-features.md)当前无未关闭的 `# Todo` 缺口，产品范围裁决及后续差异统一以该账本为准。

## 账本归属

本页对应 `ui-chat-and-command-text`（涵盖 `ui-chat-screen-surface` 与命令/SNBT/UTF-16 文本边界）；范围裁决与未关闭生产差异统一登记在[客户端账本](../../../unsupported-client-features.md)，推进原则见 [AGENTS.md](../../../../AGENTS.md)。

## 精确重开条件

目标版本改变 Brigadier / SNBT / `ItemParser` / `ComponentPredicateParser` / `BlockStateParser` /
`EntitySelectorParser` 语法、网络 `Utf8String` 长度界限，或 `ChatScreen` / 确认页生命周期规则时重验本页。

## 相关页面和源码索引

- [客户端字体与文本](client-font-and-text.md)：font provider、glyph metrics、atlas、legacy `§` 与二维字形点击几何。
- [屏幕、HUD 与库存](client-screens-hud-and-inventory.md)：全局 screen 替换与输入分发。
- [客户端聊天与身份](../security/client-chat-and-identity.md)：聊天签名验证、last-seen 与社交过滤。
- [服务端保存与重载](../persistence/server-save-and-reload.md)：服务端存档 NBT 根名与编码边界。

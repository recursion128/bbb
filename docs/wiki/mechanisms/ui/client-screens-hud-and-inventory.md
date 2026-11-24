# 客户端屏幕、HUD 与库存：输入、投影和服务端权威

机制 ID：`ui-screen-lifecycle`

## 机制目的和边界

本页拥有 screen 替换、HUD 顺序、容器输入以及 editor/dialog 的客户端生命周期。
屏幕状态表达当前交互上下文；HUD projection 表达本帧显示结果。
服务端菜单内容、客户端预测、焦点/文本编辑和 GPU 表面不属于同一类状态。

BBB 是外部多人客户端，不提供游戏内配置 UI。
启动配置见 [客户端选项与平台](../persistence/client-options-and-platform.md)。
字体 provider、glyph 和排版见 [字体与文本](client-font-and-text.md)，不在每个 screen 重做。
聊天签名和社交的语义 owner 见 [聊天与身份](../security/client-chat-and-identity.md)。

## Vanilla 26.1 权威实现

源码根由 `scripts/vanilla path 26.1` 获取。

| 决策 | 精确源码锚点 | 相邻行为 |
| --- | --- | --- |
| screen 替换 | `net/minecraft/client/Minecraft.java:1112-1165` | removed/added、null fallback、鼠标与 key release、初始化 |
| HUD 提取顺序 | `net/minecraft/client/gui/Gui.java:200-224` | camera overlays/crosshair→hotbar/effects/boss→sleep→文字/chat/list/subtitles |
| 注册表 | `net/minecraft/client/gui/screens/MenuScreens.java:76-100` | menu type 与 screen 构造器的声明映射 |
| death 动作 | `net/minecraft/client/gui/screens/DeathScreen.java:43-89` | 初始化禁用、respawn、退出确认 |
| 容器预测包 | `net/minecraft/client/multiplayer/MultiPlayerGameMode.java:442-479` | 核对 container id、保存旧 slots、本地 clicked、生成 changed-slot hashes、发送 state id |

`setScreen(null)` 不是无条件回到游戏：无 level 进入 title，死亡按策略显示 death 或 respawn，
否则可恢复 chat。teardown 期间回到 in-game GUI 会抛错。
屏幕初始化、鼠标 release 和 key release 必须与同一次替换一致。
Vanilla 的 death button 由 `DeathScreen.tick` 在第 20 tick 激活；BBB 的对应测试固定这一边界。

## 决策输入与状态所有权

| 状态 | 唯一 BBB owner | 关键身份 |
| --- | --- | --- |
| 容器 slots、cursor、state id、title/data | `bbb-client-world::client::inventory` | container id、open-container revision |
| death cause/score/hardcore 与当前页 | world 解码请求；native `DeathScreen` 持有活动状态 | Death 屏幕生命周期 |
| 当前 screen、返回屏、focus 和输入法编辑 | native `Screens` 与各 Screen payload | 当前屏幕及其真实输入上下文 |
| 菜单类型到 screen kind | native generated catalog | protocol menu type id |
| 共享布局、tooltip 和 HUD 数据 | native input/layout 与 runtime projection | 帧内 world/resource/input 借用 |
| 顶点、clip、blur 和 GPU item | renderer HUD | GUI 尺寸、frame/resource generation |

protocol catalog 只回答 menu registry id 是否有效，不拥有屏幕布局。
world 只保存当前容器数据；renderer 不解析 menu id，也不计算 click transaction。
native layout 使用生成的 `menu_screens_26_1`，不另建手写类型判定表。

## 判断与处理顺序

1. 网络解码验证 screen/menu 数据，world 接纳 open/close/content/slot/data 更新。
2. world 更新菜单事实并排 ScreenRequest；native 消费请求，以单次替换安装新屏幕。
3. input 根据当前 Screen 和实际 hit target 分派按键、鼠标及文本。
4. local click 先验证目标并预测，构造保留 state id 的 changed-slot/cursor hash 请求。
5. net 发送 typed command；服务端更新继续写入 world 的同一 inventory owner。
6. native 从同一帧借用视图生成 HUD、screen、tooltip 和 narration；renderer 只消费投影。
7. close、respawn 或 session replacement 使旧 surface、focus 和 action 身份失效。

统计屏的 simulation step 只同步 tab 行数、loading/focus 和滚动范围，不生成 HUD 或物品模型。
`ScreenRequest::StatsUpdated` 会解除返回链中保留统计屏的 loading 状态，即使当前被 Dialog 覆盖；
`runtime::stats_screen` 从 world 的 numeric stat values 与已解析 identity 建立帧内借用索引；
只读 HUD 投影只构造当前 tab 的行，物品先按数值和 registry id 排序、截取可见区，再解析模型。
hover 与 narration 继续消费同一行顺序和滚动坐标；统计数据与交互状态不保存在投影缓存中。

命令输入的 UTF-16 语法范围与已解析候选行是 `bbb-render-types` 的共享纯值；input 构造，
runtime 直接交给 HUD，渲染前的范围清理不回写屏幕状态。

### HUD 顺序和观察对象

Vanilla GUI 先检查 level-loading 与 hideGui 门。
camera overlay 和 crosshair 位于 hotbar/effects/boss 之前；sleep overlay 有独立位置。
普通准星和准星攻击进度条共用带纹理反色混合管线，按背景颜色保持可见；其它 HUD sprite 使用普通透明混合。
scoreboard、action bar、title、chat、player list 和 subtitle 保持后续层级。
subtitle 是否延后还受当前 screen 是否属于 in-game UI 控制。

BBB 在 `runtime::render_extract` 从 world 生成 action bar、title、chat、player list 等 typed 数据。
action bar 只在剩余 display ticks 为正时投影。
subtitle 依附有效 title；sleep overlay 使用当前整数 sleep counter，不能偷换成插值时钟。
HUD camera player 和本地输入玩家可能不同，远端 camera 不因此获得本地 inventory。

库存槽位静态表、Beacon 按钮、RecipeBook 按钮坐标与 Loom 行数共用
`input/inventory/layout.rs`；显示哪些条目的规则仍归实际消费者。debug-options 的内容区、
footer 和 search-box 几何共用 renderer 的 `hud/debug_layout.rs`。书本与讲台共用
`bbb_client_render::hud_book_layout` 的整数原点，进度窗口与 Done 按钮共用
`runtime/advancements.rs` 的整数原点；进度标签相对几何归 `input/advancements_screen.rs`，
绘制图标/sprite 选择仍归 `runtime/advancements.rs`。暂停页与调试选项的禁用按钮默认使用
`0xFFA0A0A0` 文字色，保留显式组件颜色；定位栏携带 `client_tick` 以推进动画 GUI sprite 帧。
`bbb_client_render::hud` 拥有 GPU 文字、clip 与绘制层；Stats、Social 和 Code of Conduct 共用
`hud/screen_draws.rs` 的 sprite scrollbar 绘制，滚动范围和交互仍归各自屏幕。
`render` 的 GUI 深度清理和 screen blur 与 [场景帧顺序](../rendering/client-scene-and-rendering.md) 对齐。
物品模型共享同一个 ordered-layers resolver，hotbar 与 inventory 消费已有投影。
热栏保存、旧格式迁移与原生物品 NBT 复制共用 RegistryCodec 的持久 writer；
HashOps 只计算预测点击需要的哈希，不构造 NBT，保存也不计算该哈希。
持久 writer 过滤 transient 组件并保留其余合法内容，热栏编码失败仍按单格处理。
旧 JSON 热栏借用原始单格文本逐格读取，再升级并保存；容器只读取规范 slots 与跨度，乐器只读取
holder 或 direct 载荷，旧派生说明、items 与计数键由规范状态输出。无效载荷按单格处理。

### screen 替换和 overlay

`ClientInputState` 的 `Screens` 保存当前 Screen 与实际返回屏，栈顶就是当前屏。
Container/Creative、书本、各编辑器、Dialog family 和命令确认携带自己的状态；RecipeBook
是容器组件。world 保留菜单 slots/state id、creative catalog、配方、registry、命令和权限。
屏幕是否打开由真实 Screen 决定，不能用菜单数据存在或创造能力替代。
`SimulationInputRuntime` 维护鼠标捕获状态；当前 `ClientInputState::wants_cursor` 查询与窗口焦点、
Code of Conduct、聊天状态及输入事件共同参与 grab/release 判断。
容器经 Esc/E 关闭且没有后续屏幕时，键盘分派立即恢复捕获，下一次聊天或玩法按键无需额外点击。
服务端关屏回到游戏时，由 `SimulationInputRuntime` 在输入分派前恢复捕获；跨维度等加载界面退下后恢复。
保留父界面、失焦和初始未捕获窗口不因此抓取鼠标。
背包和坐骑预览只保留世界实体的外观与姿态，提交到独立预览原点；世界坐标不参与 GUI 投影。
游戏内容器背景始终提交透明渐变，独立于背景模糊选项。
书本也提交游戏内透明渐变；暂停和进度页使用普通菜单的模糊与背景纹理，进度标题独立于选中标签标题。
配方书空搜索框未聚焦时显示本地化搜索提示；结果按钮只画物品图标，保留真实产出数量而不绘制数量装饰。
统计页按窗口宽度排列顶部三个标签，绘制与命中共用同一矩形；列表从标签底部开始，
正文与页脚分隔线、菜单纹理和模糊沿用官方屏幕规则。

world 完成 ShowDialog holder/registry 解码后排入 `ScreenRequest::ShowDialog(DialogState)`；
ClearDialog 只排请求并记收包计数。native 在每个网络 event 和每条输入/control 命令后消费同一 FIFO。
Dialog 首次覆盖进度屏时重置进度拖拽状态并排队 `SeenAdvancements::ClosedScreen`。
普通 Dialog/Waiting 的新正文替换保留原返回屏；Link/命令确认被新 Dialog 覆盖时保留原确认与表单。
Clear 按当前屏规则处理，不关闭 Link、Waiting 或命令确认；Warning 的下层正文可以改变。
手持书本的 `run_command` 在直接发送或完成命令确认后关闭书本或进入预填聊天；讲台保持容器关闭路径。

真正的 `set_screen` 替换丢弃已失效返回链；子屏关闭只 pop 到真实 parent。本地
`ScreenRequest::CloseContainer` 可移除 Dialog 下实际保留的 Container/Creative 而让 Dialog
继续存在，Back 不复活已关闭的容器。服务端 `ContainerClose` 按
`LocalPlayer.clientSideCloseContainer` 的 `setScreen(null)` 语义走 `CloseScreen`，关闭 Dialog
及其返回链，parent 不复活。CloseLevelScreens、CloseDeathScreen、CloseGameRules 按请求选择性清理。

按键和文本由 `input/key_events.rs` 按当前 Screen 分派；鼠标、滚轮和 IME 由
`simulation/input_commands.rs` 路由到同一屏幕的 handler。`InputControlRequest::Text` 与 IME 保留各自的
传递和提交语义。F2/F11、modifier、Code of Conduct 与玩法 fallthrough 保留实际处理顺序。
`runtime/frame.rs::update_renderer` 按当前 Screen 选择 HUD，明确提供 screen-present、
in-game-UI-present 与 blur。loading、Code of Conduct、聊天历史、toast/F3 等 overlay 仍有自己的 owner。

resource reload LoadingOverlay 由 reload barrier 拥有。
它跟随 renderer/resource 单代原子提交，不建立另一份普通 screen overlay 状态。
debug-options 的打开门消费 interrupt 条件；不能在输入和 renderer 两处各自判断能否中断。

### PauseScreen 与 DeathScreen

pause input 是 Esc 和 F3+Esc 的共同入口。
外部多人连接不能暂停真正的服务端世界，两者都走 with-menu 分支。
singleplayer no-menu 行为不成为另一个可达产品路径。

pause custom additions 从 world dialog tags 决定单 dialog、多项 custom-options list 或 server-links。
链接进入共享 URL 确认 owner；disconnect 直接执行断开。官方玩家举报及草稿确认已按
产品范围裁决移除，范围和重开条件见客户端 ledger。

NowPlaying 使用 startup `musicToast` 与最终音量，在实际 pause/show-menu 上判断是否显示。
它有独立 owner，画在 screen/tooltip 后，不占共享 toast queue 槽位。
options callback 因无游戏内 options UI 不可达；桌面与 Web 暂停菜单上的 `Options...` 按钮保持禁用状态，
悬停时显示提示说明设置由命令行参数或 `options.txt` 配置。

`DisconnectedScreen` 在展示断线原因及可选的服务器报告链接/报告目录按钮之后，桌面端同时提供
`Reconnect` 与 `Quit Game` 按钮（保留 `Quit` 退出语义），其中 `Reconnect` 以当前会话保存的
`ConnectionOptions` 关闭既有网络任务并重启 `NetworkRuntime`、清除断线屏恢复重连流程；
Web 端因浏览器标签页不提供桌面退出按钮，底部主按钮直接作为 `Reconnect` 触发页面重连。

combat packet 经 world 产生带 cause、score、hardcore 的请求，native DeathScreen 持有当前页。
death input 负责第 20 tick 按钮门、焦点、normal/hardcore 及 URL/退出确认。
cause-link 页面持有唯一链接 payload，与普通 Dialog 链接共用 URL options 决策。
respawn 和 disconnect 都沿唯一 native action→net command 链，不从 control 绕过按钮门。
主屏和延迟退出确认用 death gradient；通用 link 确认用 in-world background/blur。

### 共享提示队列

native 拥有提示的排队、槽位分配与动画时间。教程文字在本地化和分行后创建
`HudTutorialToastLines`，由 renderer 的构造入口统一清理文本、样式和行数预算，
以不可变 `Arc` 内容在 native 状态与逐帧 HUD 投影间共享。高度与占槽数从同一内容推导；
renderer 每帧校验可见比例、进度和槽位。


### 容器预测与确认

server `state_id` 是最近同步的容器版本，click 将它回传供服务器判断预测依据。
它不是客户端可自行递增的事务成功标志，也不是 block prediction sequence ack。
clientbound content/slot/cursor 更新是继续收敛到服务端权威状态的实际入口。
页面不引入不存在的通用“每次 click 成功 ack”包。

`InventoryState.player_slots` 是玩家 0–42 槽位物品与 bundle selection 的唯一可写 owner。
菜单仅保存自有格；`container_slot` / `container_slots` 按同一映射借用玩家格，未知菜单不猜玩家区。
packet、本地物品使用、creative、商人自动填充和点击提交共用这些槽位入口。
control/serde 输出完整菜单视图，反序列化以显式玩家格优先，再吸收菜单中独有的玩家数据。

local click 借用变更前的 stack，只构造一个候选菜单，使用协议 HashedStack 表达 changed slots 与 carried item。
无打开容器、非法 slot、不可 hash 的 carried item 都返回构造错误。
协议定义的 outside slot 保留其特殊含义，不能按普通非负 slot 校验一律拒绝。

`menu_state` 同时管理 container 0 的本地 inventory 与 server-open container。
container 0 收到内容更新不等于自动打开本地 inventory screen。
已有 server container 时，本地 inventory open 会拒绝覆盖它。
mount screen 的容器数据和实体 admission 复用 world owner；respawn 清空容器并重置 quick craft。

crafting、furnace、merchant、anvil、smithing、loom、stonecutter 等沿各自 inventory 子 owner 计算。
显示一个结果格不意味着客户端拥有最终合成、经验或服务端交易执行权。
菜单专属 authority/fallback 必须查看对应 owner 与 ledger，不能从通用 click 能力反推。

### editor、dialog、tooltip 与可访问性

book、sign、command/structure/jigsaw/test block editor、创造搜索与聊天输入共用 `input/text_field.rs` 的 `TextField`/`MultilineTextField`、焦点闪烁时间戳与 `HudTextField` 投影。
文本缓冲、selection、IME preedit、滚动与 focus 属于 native，不随 renderer 重绘重置。
提交时仍经过当前上下文和权限/服务端协议边界，绘制成功不表示服务端已接受编辑。

native `input/dialog.rs` 持有 Dialog 定义、输入值、焦点、拖拽、滚动、双击与 IME；
`runtime/dialog_screen/` 根据该状态及只读 world 查询生成布局、控件投影和动作。
ItemBody 解码时通过 RegistryCodec 的持久组件 reader 准备物品摘要，并解析同一冻结发布的 holder
成员；布局与 tooltip 借用该摘要的 `Arc`。control 报告保留原始 NBT 和 `value_data` 投影，
不随后续 registry 发布重新解释。摘要不进入报告序列化，报告反序列化不构造运行时物品；
嵌套诊断模板不保留摘要缓存。每个实际物品正文增加一份摘要及其持有的数据，随 Dialog 生命周期释放。
Dialog 布尔值复用 RegistryCodec 的 NbtOps 转换，引用定义经来源投影后的数值标签也能正确读取。
Warning、Link、Waiting 各自的真实 payload 随 Screen 生命周期保存。
任意 Dialog input 字符串与 text 编辑值由 native 的 `JavaString` 保留原始单元，canonical writer、
动态模板与 CustomClickAction 提交直接消费同一值。单行按代码点移动，多行按 UTF-16 单元移动；
HUD 只保存全文显示投影与单位 cursor/selection，不拥有第二份可编辑文本。
单行在屏内按 cursor 切出前后片段；多行先用完整 pair 换行，仅在 blink 或有效 IME 请求时
局部切片，选区宽度独立按单位 prefix 测量。显示文本与 sanitizer 保留 NUL/control 和合法长文本，
counter 按 UTF-16 单元计数；IME 请求沿用 screen revision 隔离
（`EditBox.java:408-432`、`MultiLineEditBox.java:131-168`）。
剪贴板选区复用 Java UTF-8 转换与平台 NUL 截断，复制不改表单，剪切只删除原始选中单元。
背包物品 tooltip 和 rich component 的 show-item hover 使用同一个 native item-tooltip builder。
renderer 接受已布局的 tooltip lines，不重新解码组件语义。

屏幕 builder 生成 narration frame，由 `narrator_runtime::ScreenNarrations` 消费
`ScreenNarrationFrames`；输入调度以 `ScreenNarrationTarget` 指定目标。命令确认首次打开或恢复时的
初始化快照经有序 `CommandConfirmationInitialized` effect 交付，即使打开到关闭期间未绘制，
也保留需要播报的输出。keyboard focus、聊天 restrictions 和 tooltip 文本来自同一屏幕投影。
clipboard 请求（含 `Shift+F3+I` 的 `debug_recreate` 导出完整 `ResolvableProfile.properties` 命令）经 input 的 `DebugClipboard` 与平台接入处理；dialog 不直接读写系统剪贴板。
字体/本地化资源变更应使相关 projection cache 失效，而不是保留上一语言的交互文本。

## 生命周期和 fallback

menu id 不在 protocol catalog 时在解码边界拒绝；native 未命中 screen kind 时无 surface。
Lectern 使用专用 book surface，其余注册菜单使用对应 inventory HUD 布局。
clientbound/local close、respawn 和 session replacement 清理同一个 inventory state。
资源更新失败继续保留上一完整 HUD/icon/atlas generation；不能只换按钮图而保留旧 hit layout。

## BBB 实现映射

| owner | 实际路径 | 投影或输出 |
| --- | --- | --- |
| inventory 语义 | `bbb_client_world::client::inventory` | slots、state id、click request |
| menu 注册 | `bbb_client::input::inventory::menu_screens_26_1` | `VanillaMenuScreenKind` |
| screen 生命周期 | `bbb_client::input::screen` | 当前屏、返回链与选择性关闭 |
| death 输入 | `bbb_client::input::death_screen` | 延迟按钮、当前页与 action |
| pause 输入 | `bbb_client::input::pause_screen` | menus、disconnect |
| disconnected 输入 | `bbb_client::input::disconnected_screen` | 按钮焦点、Reconnect 与 Quit/OpenPath 请求 |
| dialog 状态 | `bbb_client::input::dialog` | 编辑值、focus、IME 与动作输入 |
| dialog 投影 | `bbb_client::runtime::dialog_screen` | HUD、布局、typed command |
| 输入分派 | `bbb_client::input::key_events`、`simulation::input_commands` | 当前 Screen handler |
| 帧与 HUD 提取 | `bbb_client::runtime::frame`、`runtime::render_extract` | immutable renderer data |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| screen 替换与清理 | `Minecraft.setScreen` | native Screens | `aligned` | death/dialog lifecycle 用例 |
| 容器点击 | `MultiPlayerGameMode.handleContainerInput` | world inventory→net | `aligned` | click 拒绝用例 |
| HUD 和 narration | `Gui`、screen owner | native projection→renderer/narrator | `aligned` | narration frame 用例 |
| 注册表 | `MenuScreens` | generated native catalog | `aligned` | generator exact-output 与 native consumer |

## 结构性不变量

- screen identity、container id、resource generation 分别验证，不能彼此充当替代键。
- world 只有一份当前容器；native 不保留第二份 slots 或 shadow screen catalog。
- 控制接口复用真实 input action，不跳过 death、dialog 或权限门。
- tooltip、item model 和 narration 使用既有语义 owner，renderer 不反查业务状态。

## 聚焦测试（focused tests）和真实证据

| 测试 | 路径 | 证明的语义 |
| --- | --- | --- |
| `death_screen_buttons_unlock_after_twenty_ticks_and_respawn_once` | `bbb_client::input::death_screen` | 19 tick 不发包，20 tick 只发一次 respawn |
| `build_container_click_slot_rejects_missing_container_invalid_slot_and_unhashable_carried_item` | `bbb_client_world::client::inventory::tests::container_clicks` | 非法预测请求明确失败 |
| `chat_and_restrictions_reuse_the_screen_narration_frame_owner` | `bbb_client::runtime::tests::hud_frame` | chat/restrictions 共享 narration 文本 |
| `dialog_warning_button_opens_confirm_screen_and_dispatches_back_or_disconnect` | `bbb_client::runtime::dialog_screen::tests::screens` | warning 确认和后续 action |

定向测试与会话回放的选择见[开发流程与测试](../../development/testing-and-gates.md)。

menu catalog 的重复、遗漏、声明顺序变化由
`scripts/menu_screen_catalog_extractor.py` fixture 与 generated exact-output
共同拒绝。death offscreen 验证 icon/background/button/confirmation 像素；playable lifecycle
需观察 disabled→第 20 tick active，再经 input.key:enter 进入唯一 action。

## 当前差异

本页的 screen-lifecycle 状态不合并所有菜单业务和平台可访问性状态。
服务端拥有最终容器效果；外部客户端无 integrated-server pause，也无游戏内配置 UI。
开放差异和范围裁决以 [客户端 ledger](../../../unsupported-client-features.md) 的 UI/inventory 领域为准。

## 账本归属

[AGENTS.md](../../../../AGENTS.md) 拥有推进原则；未关闭差异只取 [客户端 ledger](../../../unsupported-client-features.md)，本页不保存任务进度。

## 精确重开条件

`setScreen` fallback、cursor owner、reload overlay 或 debug interrupt 消费者改变时重核生命周期。
menu registry 顺序、container hash/state-id 格式或 close/respawn 清理顺序改变时重核 inventory。
Death 按钮门、Dialog 返回规则或 IME/hit target 失效规则改变时，重核对应玩家动作与输出。

## 相关页面和源码索引

- [连接与注册表](../protocol/client-connection-and-registries.md)：screen 输入的连接阶段和 registry 来源。
- [客户端输入与交互](../player/client-input-and-interaction.md)：真实 input 分派与预测边界。
- [客户端架构边界](../../architecture/client-boundaries.md)：crate 所有权和状态分类。

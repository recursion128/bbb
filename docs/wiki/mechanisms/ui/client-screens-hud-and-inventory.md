# 客户端屏幕、HUD 与库存：输入、投影和服务端权威

机制 ID：`ui-screen-lifecycle`

## 机制目的和边界

本页定义屏幕替换、HUD 绘制顺序、容器输入以及各类编辑器与对话框的客户端生命周期。屏幕状态表达当前交互上下文，HUD 投影表达本帧显示结果；服务端菜单内容、客户端本地预测、焦点与文本编辑状态以及 GPU 表面分属不同类别的状态，互不混淆。

BBB 是外部多人客户端，不提供游戏内配置界面，启动期配置见[客户端本地配置、按键与平台窗口](../persistence/client-options-and-platform.md)。字体 provider、字形度量与排版见[字体与文本](client-font-and-text.md)，不在各个屏幕重复实现；聊天签名与社交过滤见[客户端聊天与身份](../security/client-chat-and-identity.md)。

## Vanilla 26.1 权威实现

源码根由 `scripts/vanilla path 26.1` 获取：

| 决策 | 精确源码锚点 | 相邻行为 |
| --- | --- | --- |
| 屏幕替换 | `net/minecraft/client/Minecraft.java:1112-1165` | `removed`/`added` 回调、空屏回退、鼠标与按键释放、新屏初始化 |
| HUD 提取顺序 | `net/minecraft/client/gui/Gui.java:200-224` | 相机遮罩/准星 → 快捷栏/状态效果/Boss 栏 → 睡眠遮罩 → 文字/聊天/玩家列表/字幕 |
| 菜单屏幕注册表 | `net/minecraft/client/gui/screens/MenuScreens.java:76-100` | 菜单类型与屏幕构造器的声明映射 |
| 死亡界面动作 | `net/minecraft/client/gui/screens/DeathScreen.java:43-89` | 初始化禁用按钮、第 20 tick 解锁、重生与退出确认 |
| 容器预测数据包 | `net/minecraft/client/multiplayer/MultiPlayerGameMode.java:442-479` | 核对容器 ID、记录旧槽位、执行本地点击、生成变更槽位哈希并发送 `state_id` |

`setScreen(null)` 并非无条件回到游戏：无世界时退回非游戏状态，玩家死亡时按策略展示死亡界面或直接重生，否则可恢复常规操作；拆除世界期间禁止重新进入游戏内界面。屏幕初始化、鼠标捕获释放与按键清空必须在同一次替换中保持一致。Vanilla 死亡按钮由 `DeathScreen.tick` 在第 20 tick 激活，BBB 由聚焦测试严格锁定这一边界。

## 决策输入与状态所有权

| 状态 | 唯一 BBB 所有者 | 关键身份 |
| --- | --- | --- |
| 容器槽位、光标物品、`state_id`、标题与数据槽 | `bbb-client-world::client::inventory` | 容器 ID、打开容器修订号 |
| 死亡原因、分数、极限模式与当前页 | 世界层解码请求；客户端 `DeathScreen` 持有活动状态 | 死亡屏幕生命周期 |
| 当前屏幕、返回链、焦点与输入法编辑 | 客户端 `Screens` 栈与各屏幕状态载荷 | 当前屏幕及其真实输入上下文 |
| 菜单类型到屏幕种类映射 | 客户端生成目录（`menu_screens_26_1`） | 协议菜单类型 ID |
| 共享布局、悬浮提示框与 HUD 投影 | 客户端输入/布局与运行时帧提取 | 帧内借用的世界、资源与输入视图 |
| 顶点、裁剪、背景模糊与 GPU 物品渲染 | 渲染器 HUD 子系统 | GUI 逻辑尺寸、帧与资源代际 |

协议目录仅判断菜单注册表 ID 是否合法，不拥有屏幕布局；世界层仅持有当前容器权威数据；渲染器不解析菜单 ID，也不计算容器点击事务。客户端布局直接消费生成的 `menu_screens_26_1`，不维护手写菜单类型判断表。

## 判断与处理顺序

1. 网络解码校验屏幕与菜单数据，世界层接纳打开、关闭、全量内容、单槽与数据属性更新。
2. 世界层更新菜单真值并排入 `ScreenRequest`；客户端运行时消费请求，以单次屏幕替换安装新界面。
3. 输入系统按当前活动屏幕与实际命中目标分派键盘、鼠标、滚轮与文本输入。
4. 本地容器点击先校验目标并执行预测，构造携带最新 `state_id`、变更槽位哈希与光标物品哈希的点击请求。
5. 网络层发送强类型命令；服务端权威同步更新继续写回世界层同一库存所有者。
6. 客户端运行时从同一帧借用视图生成 HUD、屏幕、悬浮提示框与旁白；渲染器只消费不可变投影。
7. 关屏、重生或会话替换同步失效旧界面、焦点与待处理动作身份。

统计屏在模拟步仅同步页签行数、加载标志、焦点与滚动范围，不预先构建 HUD 或物品模型。`ScreenRequest::StatsUpdated` 会解除返回链中保留统计屏的加载态，即便此时被对话框覆盖。`runtime::stats_screen` 从世界层数值统计与已解析注册表身份构造帧内借用索引；只读 HUD 投影仅生成当前页签行，物品按数值与注册表 ID 排序并截取可见区后再解析模型。悬停与旁白复用同一行序与滚动坐标，统计数据与交互状态不留在投影缓存中。

命令输入的 UTF-16 语法着色区间与已解析补全候选行是 `bbb-render-types` 的共享纯值；输入层构造，运行时直接转交 HUD，渲染前范围清理不回写屏幕状态。

### HUD 顺序和观察对象

Vanilla GUI 先检查世界加载与隐藏 HUD 门槛；相机遮罩与准星排在快捷栏、状态效果与 Boss 栏之前；睡眠遮罩位于独立位置。普通准星与准星攻击进度条共用带纹理反色混合管线，在任意背景下保持可见；其余 HUD 精灵采用标准透明混合。计分板、动作栏、标题、聊天栏、玩家列表与字幕按序位于后续层级；字幕是否延后受当前屏幕是否属于游戏内界面影响。

BBB 在 `runtime::render_extract` 从世界层生成动作栏、标题、聊天与玩家列表等强类型数据：动作栏仅在剩余显示 tick 为正时投影；副标题依附有效主标题；睡眠遮罩读取当前整数睡眠计数器，绝不用插值时钟替代。相机观察实体与本地输入玩家可能不同，旁观远端实体时不会错误获得对方背包。

容器槽位静态坐标表、信标按钮、配方书按钮坐标与织布机行数共用 `input/inventory/layout.rs`，具体显示规则归各自消费者。调试选项屏幕的内容区、页脚与搜索框几何共用渲染器 `hud/debug_layout.rs`。书本与讲台共用 `bbb_client_render::hud_book_layout` 的整数原点；进度窗口与 Done 按钮共用 `runtime/advancements.rs` 的整数原点，进度标签相对几何归 `input/advancements_screen.rs`，图标与精灵选择归 `runtime/advancements.rs`。暂停页与调试选项禁用按钮默认使用 `0xFFA0A0A0` 文字色并保留显式组件颜色；定位栏携带 `client_tick` 以推进动画精灵帧。`bbb_client_render::hud` 拥有 GPU 文字、裁剪与绘制层；统计、社交与行为准则屏共用 `hud/screen_draws.rs` 的滚动条绘制，滚动范围与交互仍归各自屏幕。GUI 深度清理与屏幕模糊与[场景帧顺序](../rendering/client-scene-and-rendering.md)保持一致。物品模型复用统一的有序层解析器，快捷栏与背包消费同一投影。

创造模式快捷栏保存、旧格式迁移与原生物品 NBT 复制共用 `bbb-registry-codec` 的持久写出器；`HashOps` 仅计算预测点击所需哈希而不构造 NBT，保存也不计算该哈希。持久写出器过滤瞬态组件并保留合法数据，单格编码失败按单格隔离处理。旧 JSON 快捷栏逐格借用原始文本升级并写回当前 NBT 存档格式，无效载荷同样按单格隔离。

### 屏幕替换与遮罩层

`ClientInputState` 的 `Screens` 栈持有当前屏幕与真实返回链，栈顶即活动屏幕。容器/创造背包、书本、各类方块编辑器、对话框族与命令确认页各自持有独立状态，配方书作为容器屏幕组件存在。世界层保留菜单槽位、`state_id`、创造物品目录、配方、注册表、命令树与权限。屏幕是否打开由真实 `Screens` 栈决定，不得用菜单数据存在或创造能力标志替代。

`SimulationInputRuntime` 管理鼠标捕获状态，`ClientInputState::wants_cursor` 与窗口焦点、行为准则遮罩、聊天状态及输入事件共同决定捕获与释放。容器经 `Esc`/`E` 关闭且无后续屏幕时，键盘分派立即恢复光标捕获，后续玩法按键无需额外点击；服务端关屏或跨维度加载退下回到游戏时，同样在输入分派前恢复捕获。保留父界面、窗口失焦或初始未捕获时不强行抓取鼠标。

背包与坐骑界面的实体预览仅保留外观与姿态并提交至独立预览原点，不混入世界坐标。游戏内容器与书本背景始终绘制半透明渐变，不受菜单背景模糊选项影响；暂停页与进度页采用普通菜单背景纹理与模糊，进度窗口标题独立于选中页签标题。配方书空搜索框未聚焦时显示本地化提示；结果按钮仅绘制物品图标，保留真实产出数量而不绘制角标数字。统计页按窗口宽度排布顶部三个页签，绘制与点击命中共用同一矩形，正文与页脚分隔线遵循官方屏幕规则。

世界层完成 `ShowDialog` 的 holder/注册表解码后排入 `ScreenRequest::ShowDialog(DialogState)`；`ClearDialog` 仅排请求并记录收包计数。客户端运行时在每个网络事件及每条输入/控制命令后立即消费同一 FIFO。对话框首次覆盖进度屏时重置拖拽状态并排队 `SeenAdvancements::ClosedScreen`。普通对话框与等待屏的新正文替换保留原返回屏；链接或命令确认页被新对话框覆盖时保留原确认与表单。`ClearDialog` 按当前屏规则处理，不关闭链接确认、等待屏或命令确认；警告页的下层正文仍可更新。手持书本点击 `run_command` 在直接发送或完成确认后关闭书本或进入预填聊天；讲台保持容器关闭路径。

真正的 `set_screen` 替换会丢弃已失效返回链，子屏关闭只弹回真实父屏。本地 `ScreenRequest::CloseContainer` 可移除对话框下方保留的容器/创造屏而保持对话框继续打开，返回时不复活已关闭容器；服务端 `ContainerClose` 按 `LocalPlayer.clientSideCloseContainer` 的 `setScreen(null)` 语义走 `CloseScreen`，一并关闭对话框及其返回链。`CloseLevelScreens`、`CloseDeathScreen` 与 `CloseGameRules` 按请求执行选择性清理。

键盘与文本事件由 `input/key_events.rs` 按当前活动屏幕分派，鼠标、滚轮与 IME 由 `simulation/input_commands.rs` 路由至同一屏幕处理器。`InputControlRequest::Text` 与 IME 保持各自提交语义，全局快捷键（`F2`/`F11`）、修饰键、行为准则遮罩与玩法穿透维持既有先后顺序。`runtime/frame.rs::update_renderer` 按当前屏幕选择 HUD 投影，明确提供 `screen_present`、游戏内界面标志与背景模糊参数；加载界面、行为准则、聊天历史、提示队列与 F3 调试信息各自拥有独立所有者。

资源重载加载遮罩（`LoadingOverlay`）由重载屏障拥有，跟随渲染器/资源单代原子提交，不另建普通屏幕状态。调试选项界面的打开门槛消费同一中断判定条件，避免输入层与渲染层口径漂移。

### 暂停屏与死亡屏

`Esc` 与 `F3+Esc` 共享同一暂停输入入口；外部多人连接不能暂停服务端世界，两者均走带菜单分支，不暴露单人无菜单暂停路径。

暂停屏自定义附加项根据世界层对话框标签在单对话框、多项自定义选项列表与服务器链接间选择；链接进入共享 URL 确认弹窗，断开按钮直接执行断连。官方玩家举报与草稿退出确认已按产品范围裁决移除。`NowPlaying` 提示读取启动配置 `musicToast` 与最终音乐音量，在实际打开暂停菜单时判断是否显示，它拥有独立绘制通道，排在屏幕与悬浮提示之后，不占用共享提示队列槽位。因为客户端不提供游戏内配置界面，桌面与 Web 暂停菜单上的 `Options...` 按钮保持禁用状态，悬停提示说明配置由命令行参数或 `options.txt` 提供。

`DisconnectedScreen` 展示断线原因及可选的服务器报告链接/本地报告目录按钮后，桌面端同时提供 `Reconnect` 与 `Quit Game`（`Back`）按钮：`Reconnect` 按当前会话保存的 `ConnectionOptions` 关闭既有网络任务、重启 `NetworkRuntime` 并关闭断线页恢复重连，`Quit Game` 触发正常应用退出；Web 端因浏览器标签页不提供退出按钮，底部主按钮直接映射为 `Reconnect` 触发页面重连。

战斗死亡包经世界层产生包含死因、分数与极限模式标志的请求，客户端 `DeathScreen` 持有当前活动页。死亡输入处理器控制第 20 tick 按钮解锁、焦点导航、普通/极限分支以及死因链接与退出确认。死因链接页持有唯一链接载荷，与对话框链接共用 URL 确认选项。重生与断开均走唯一「输入动作 → 网络命令」链，控制接口不得绕过第 20 tick 按钮门。主屏与延迟退出确认使用死亡红黑渐变背景，通用链接确认使用游戏内背景与模糊。

### 共享提示队列

客户端运行时统一管理提示（toast）的排队、槽位分配与动画计时。教程文本在本地化与分行后构造 `HudTutorialToastLines`，由渲染器入口统一清理样式与行数预算，以不可变 `Arc` 在输入状态与逐帧 HUD 投影间共享；高度与占槽数由同一内容推导，渲染器每帧校验可见比例、进度与槽位。

### 容器预测与确认

服务端 `state_id` 是最近一次同步的容器状态版本号，点击数据包将其回传供服务端校验客户端预测基线；它既不是客户端自增的事务成功号，也不是方块预测序列确认。服务端后续下发的全量内容、单槽或光标更新才是收敛至权威状态的实际闭环，协议中不存在通用的「每次点击成功 ack」数据包。

`InventoryState.player_slots` 是玩家 0–42 槽位物品及收纳袋（bundle）选中项的唯一可写所有者。打开的外部菜单仅保存自有槽位，通过 `container_slot` / `container_slots` 按映射借用玩家槽位，未知菜单不猜测玩家区布局。数据包应用、本地物品使用、创造模式、村民交易自动填充与点击提交共用这些槽位入口。控制与序列化输出完整菜单视图，反序列化优先采纳显式玩家槽位再吸收菜单独有数据。

本地点击借用变更前物品栈构造单个候选菜单，并使用协议 `HashedStack` 表达变更槽位与光标手持物。无打开容器、越界槽位或不可哈希的手持物均返回构造错误；协议定义的容器外丢弃槽位（outside slot）保留其特殊语义，不被普通非负槽位检查误拒。

`menu_state` 同时维护 0 号本地玩家背包与服务端打开的外部容器：收到 0 号容器内容包不会自动打开背包界面；已有外部容器打开时，本地打开背包请求不会覆盖当前容器。坐骑背包数据与实体准入复用世界层所有者；玩家重生清空容器状态并重置快捷合成（quick craft）。

工作台、熔炉、村民交易、铁砧、锻造台、织布机、切石机等沿各自库存子所有者执行客户端本地预测；预览结果格不代表客户端拥有最终合成、经验扣除或交易成交权威。

### 编辑器、对话框、悬浮提示与无障碍旁白

书本、告示牌、命令/结构/拼图/测试方块编辑器、创造背包搜索框与聊天输入框共用 `input/text_field.rs` 的 `TextField`/`MultilineTextField`、光标闪烁时间戳与 `HudTextField` 投影。文本缓冲、选区、IME 预编辑、滚动位置与焦点均由客户端输入层独占，不随渲染重绘重置；编辑提交仍受上下文权限与服务端协议校验约束，界面绘制成功不代表服务端已接受修改。

`input/dialog.rs` 持有对话框定义、控件值、焦点、拖拽、滚动、双击与 IME 状态；`runtime/dialog_screen/` 据该状态与只读世界查询产出布局、控件投影与动作。`ItemBody` 解码时经 `bbb-registry-codec` 持久组件读取器构造物品摘要并解析同一冻结代际的 holder 成员，布局与悬浮提示借用该摘要的 `Arc`；控制诊断保留原始 NBT 与 `value_data` 投影，不随后续注册表发布重解释，也不将运行时摘要写进报告序列化。对话框布尔值复用 `bbb-registry-codec` 的 `NbtOps` 转换，确保引用定义经来源投影后的数值标签也能正确读取。警告、链接与等待屏各自随屏幕生命周期持有载荷。

任意对话框字符串输入与文本编辑值由客户端 `JavaString` 保留原始 UTF-16 单元，规范写出、动态模板与 `CustomClickAction` 直接消费同一值。单行按代码点移动光标，多行按 UTF-16 单元移动；HUD 仅接收显示投影与单元下标，不持有第二份可编辑文本。单行在屏内按光标切分前后段，多行先以完整代理对换行并在光标闪烁或有效 IME 时局部切片，选区宽度按单元前缀独立测量。显示文本与清理器保留 `NUL`/控制字符及合法长文本，字符计数器按 UTF-16 单元计数，IME 请求按屏幕修订号隔离（`EditBox.java:408-432`、`MultiLineEditBox.java:131-168`）。剪贴板选区复用 Java UTF-8 转换与平台 `NUL` 截断，复制不改表单，剪切仅删除原始选中单元。背包物品提示框与富文本组件 `show_item` 悬停共用同一物品提示框构建器，渲染器直接绘制已布局的行。

屏幕构建器生成旁白帧（`ScreenNarrationFrames`），交由 `narrator_runtime::ScreenNarrations` 按 `ScreenNarrationTarget` 播报。命令确认页首次打开或恢复的初始化快照经有序 `CommandConfirmationInitialized` 副作用交付，即便开闭之间未经历渲染帧也不丢失初始化播报。键盘焦点、聊天限制提示与悬浮提示文本均来自同一屏幕投影。剪贴板写入请求（含 `Shift+F3+I` 的 `debug_recreate` 导出完整 `ResolvableProfile.properties` 命令）经 `DebugClipboard` 与平台层统一执行，对话框不直接读写系统剪贴板。字体或语言资源变更会失效相关投影缓存，避免残留上一语言的交互文本。

## 生命周期和 fallback

不在协议目录中的菜单 ID 在解码边界直接拒绝；客户端未匹配屏幕种类时不打开无效界面。讲台使用专用书本阅读界面，其余已注册菜单映射至对应容器 HUD 布局。服务端关屏、本地关屏、重生与会话替换统一清理同一库存状态。资源重载失败继续保留上一完整 HUD/图标/图集代际，避免出现贴图已切换而点击热区仍停留在旧布局的错位状态。

## BBB 实现映射

| 语义所有者 | 实际路径 | 投影或输出 |
| --- | --- | --- |
| 容器与背包语义 | `bbb_client_world::client::inventory` | 槽位状态、`state_id` 与点击预测请求 |
| 菜单屏幕注册目录 | `bbb_client::input::inventory::menu_screens_26_1` | `VanillaMenuScreenKind` |
| 屏幕栈生命周期 | `bbb_client::input::screen` | 当前活动屏、返回链与选择性清理 |
| 死亡界面输入 | `bbb_client::input::death_screen` | 20 tick 延迟按钮门、当前页与动作 |
| 暂停界面输入 | `bbb_client::input::pause_screen` | 菜单动作、自定义对话框与断开连接 |
| 断线界面输入 | `bbb_client::input::disconnected_screen` | 按钮焦点、`Reconnect` 与 `Exit`/`OpenPath` 请求 |
| 对话框交互状态 | `bbb_client::input::dialog` | 控件编辑值、焦点、IME 与提交动作 |
| 对话框投影构建 | `bbb_client::runtime::dialog_screen` | HUD 布局、控件投影与强类型命令 |
| 输入统一分派 | `bbb_client::input::key_events`、`simulation::input_commands` | 按当前活动屏幕路由事件 |
| 帧与 HUD 提取 | `bbb_client::runtime::frame`、`runtime::render_extract` | 送往渲染器的不可变投影数据 |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 屏幕替换与返回链清理 | `Minecraft.setScreen` | `bbb_client::input::screen` | `aligned` | 死亡屏、对话框与容器替换生命周期测试 |
| 容器点击预测与哈希请求 | `MultiPlayerGameMode.handleContainerInput` | `client::inventory` → 网络命令 | `aligned` | 非法容器/槽位/不可哈希物品拒绝与点击测试 |
| HUD 顺序与无障碍旁白 | `Gui`、各屏幕旁白所有者 | 客户端帧投影 → 渲染器与旁白运行时 | `aligned` | 聊天/限制页/确认页旁白帧测试 |
| 菜单屏幕注册映射 | `MenuScreens` | 生成目录 `menu_screens_26_1` | `aligned` | 生成器精确输出校验与客户端消费测试 |

## 结构性不变量

- 屏幕代际、容器 ID 与资源代际各自独立校验，不得互相充当替代键。
- 世界层只维护一份权威当前容器；客户端输入层不保留第二份槽位副本或影子屏幕目录。
- 控制接口与 MCP 必须复用真实输入处理器，不得绕过死亡 20 tick 门、对话框确认或权限门槛。
- 悬浮提示框、物品模型与旁白共用既有语义所有者，渲染器不得反查业务状态。

## 聚焦测试（focused tests）和真实证据

| 测试 | 路径 | 证明的语义 |
| --- | --- | --- |
| `death_screen_buttons_unlock_after_twenty_ticks_and_respawn_once` | `bbb_client::input::death_screen` | 19 tick 不发包，第 20 tick 解锁并仅发送一次重生 |
| `build_container_click_slot_rejects_missing_container_invalid_slot_and_unhashable_carried_item` | `bbb_client_world::client::inventory::tests::container_clicks` | 缺失容器、非法槽位与不可哈希手持物明确拒绝 |
| `chat_and_restrictions_reuse_the_screen_narration_frame_owner` | `bbb_client::runtime::tests::hud_frame` | 聊天页与限制提示页共享旁白帧所有者 |
| `dialog_warning_button_opens_confirm_screen_and_dispatches_back_or_disconnect` | `bbb_client::runtime::dialog_screen::tests::screens` | 警告页按钮打开确认屏并正确分派返回或断开 |

聚焦测试与端到端回放入口见[开发流程与测试](../../development/testing-and-gates.md)。菜单目录的重复、遗漏与声明序变化由 `scripts/menu_screen_catalog_extractor.py` 夹具及精确输出校验共同拦截；死亡界面同时由离屏像素测试与可游玩生命周期回放覆盖。

## 当前差异

服务端始终拥有容器最终状态权威；外部多人客户端不提供单人集成服务端暂停、游戏内配置界面、触屏分堆动画或官方玩家举报界面。[客户端账本](../../../unsupported-client-features.md)当前无未关闭的 `# Todo` 缺口，上述范围裁决均登记于该账本的 `# Not needed` 段落中。

## 账本归属

推进原则见 [AGENTS.md](../../../../AGENTS.md)；未关闭差异与产品范围裁决统一由[客户端账本](../../../unsupported-client-features.md)维护，本页不保存进度清单。

## 精确重开条件

当 `setScreen` 回退逻辑、光标捕获规则、资源重载遮罩或调试中断门槛变化时须重核生命周期；当菜单注册顺序、容器 `HashedStack`/`state_id` 格式或关屏/重生清理序变化时须重核库存链路；当死亡按钮解锁门、对话框返回规则或 IME/热区失效规则变化时重核对应交互测试。

## 相关页面和源码索引

- [客户端连接、协议阶段与注册表](../protocol/client-connection-and-registries.md)：屏幕输入的连接阶段与注册表来源。
- [客户端本地输入、移动与交互](../player/client-input-and-interaction.md)：输入分派与本地预测边界。
- [客户端架构边界](../../architecture/client-boundaries.md)：crate 所有权与状态分类。

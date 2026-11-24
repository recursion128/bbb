# 客户端本地配置、按键与平台窗口

机制 ID：`persistence-options-file`

## 机制目的和边界

本页说明 CLI、options.txt、servers.dat 与平台窗口如何交接。选项文件中的值、真正生效的
startup 参数和运行时观察是不同状态；文件里能保存某个 key，不代表其 consumer 已实现。
配置只在命令行启动时提供，没有游戏内配置 UI；按键与 HUD/screen 交互见
[屏幕与背包](../ui/client-screens-hud-and-inventory.md)。

## Vanilla 26.1 权威实现

- `Options.java:1576-1595`：先收集原始配置，再 datafix；缺文件保留默认。
- `Options.java:1702-1709`、`Options.java:1759-1768`：直接写 options 文件，最后 broadcastOptions。
- `QuickPlay.java:96-108`：按服务器地址查列表，缺失时新增 hidden 项后连接。
- `Window.java:411-438`：GUI scale 保持最小分辨率，Unicode scale 取偶数，逻辑尺寸向上取整。

Vanilla Windows/macOS/Linux 的 backend 细节不能通过这些纯值规则证明；平台运行证据由
platform acceptance 门禁裁决。

## 决策输入与状态所有权

| 输入/状态 | owner | 用途 |
| --- | --- | --- |
| CLI 与是否显式给值 | startup / args | 明确覆盖文件中的选项 |
| raw options、version、dynamic groups | `bbb-client` | datafix、编码与报告 |
| key translation / physical binding | options key_bindings 与 input runtime | 输入路由，不保存 world latch |
| server 地址、名字、icon 与 hidden | `bbb-client::server_list` | opt-in servers.dat 持久化 |
| window size/mode、GUI scale、cursor/title | `window_surface` 与 platform event loop | winit 请求与实际状态观察 |
| window icon、menu background、字体资源 | pack/native/renderer 各自 generation | 平台或 GPU 消费 |

## 判断与处理顺序

1. 解析 CLI，保留显式参数来源；读取文件并进行旧版本 datafix。
2. 以 catalog 默认值补缺字段，解码各 option，再应用真正显式的 CLI override。
3. startup 将有效选项投给对应 owner：视距/输入、音量、语言、renderer、window 等；
   原始文本保留与是否被当前产品消费分别判断。
4. 创建窗口，应用 display mode/尺寸/标题/icon，观察真实 framebuffer 与 refresh rate。
5. 根据 framebuffer 计算 GUI scale 和逻辑尺寸；屏幕布局与鼠标命中使用同一 scale。
6. 平台事件作为 mailbox input，游戏动作仍由 simulation 决定；render thread 不直接改世界。
7. 允许的持久化变化交回文件 owner，网络 ClientInformation 通过既有 command 路径发送。

### 本地文件读取与保存

`bbb-client::server_list` 是 `servers.dat` 的唯一 owner。它复刻 Vanilla 命令行进服的那条路径
——`QuickPlay.joinMultiplayerWorld` 读列表、按**地址**（不看名字，先可见后隐藏）找条目、找不到
就以 `selectServer.defaultName` 的位置新建一条 hidden 条目、写回；随后
`ClientPacketListener.handleServerData` 走 `ServerList.saveSingleServer` 更新同一条。因此
`--server-list-name` 只在新建时生效，新建条目挂 `hidden`、插在 hidden 段首并把 hidden 段裁到 16
条，写回用 `Util.safeReplaceFile`（临时文件 + 一份 `_old` + 失败回滚），与 options 的直接截断写入分开。`size`/`swap`/`replace`/`remove`/`unhide` 只服务多人服务器选择屏，不在范围内（见
[ledger](../../../unsupported-client-features.md)；`replace` 在 26.1 本身也无调用方）。BBB 把这个 owner 设
成显式 opt-in：只有同时给了 `--server-list-store` 与 `--server-list-name` 才构造它，避免默认就动
真实 game directory。

`bbb-client` 是 `options.txt` 的唯一 owner，key/编码/默认值全部由
`scripts/generate-client-options-catalog.py` 从 `Options.java` 派生。读取按 Vanilla 语义：只在
第一个 `:` 切分、坏行跳过、缺文件用默认、未知 key 不回写；写入是直接截断重写（**没有**临时文件或
备份——那是 `ServerList` 才有的）。完整 raw map 会先按文件 `version`（缺失/非法为 0）执行
source-derived 的 `DataFixTypes.OPTIONS` registration，再进入任何 current codec 或 startup
binding；registration/order、fix source hash 与 LWJGL2 键码同样由该 generator 锁定，
fix 抛错时整组保留 defaults，匹配 `Options.load` 的外层 catch。CLI 参数是 override，只有用户真的
在命令行打了才压过文件值。
启动与加载会按 Vanilla 生命周期写回 `startedCleanly`；运行期会写盘的快捷键包括 F11（`fullscreen`）、Ctrl+B（`narrator`）、F3+H（`advancedItemTooltips`）与 F3+P（`pauseOnLostFocus`），存盘之后按 `Options.save` 的收尾无条件补发一次 `ClientInformation`。桌面端 `F3+S` 会将动态纹理图集（地形、GUI、字体、粒子及动态玩家皮肤/纹理图集）以 PNG 导出至 `<game-dir>/screenshots/debug`（`wasm32` 不启用）。桌面端旁白开启且未指定 `--narrator-command` 时回退到内置操作系统 TTS 旁白器；显式传入 `--narrator-command`（及可选 `--narrator-command-arg`）时优先覆盖内置 TTS。

### 窗口与按键投影

`window_surface` 将零尺寸的启动请求夹到至少 1，GUI scale 从 1 开始，在下一档仍容纳
320×240 GUI pixels 时增长；auto 不受用户 cap 提前停止，强制 Unicode 时 scale 为偶数。
GUI 宽高使用向上取整，不能把 framebuffer pixels 直接当 GUI 坐标。

preferred fullscreen mode 的解析、匹配与 fallback 由窗口 owner 完成；配置请求与实际 monitor
状态分开观察。refresh rate 从平台单位转换，cursor change 受当前状态门控。窗口关闭走
[运行时统一退出](../runtime/client-runtime-and-control.md)，不单独遗留网络 task。

key mapping 的标识与默认键来自版本化 catalog；physical key、按下/释放、click 消费与
客户端玩法状态各归对应 owner。文件迁移中的 LWJGL2 key 与当前 physical mapping 不混为
第二套 gameplay action。焦点、screen 抢占和释放顺序仍通过 input/mailbox 合同实现。

BBB 启动扩展参数 `--dynamic-lights`（Web `?dynamic_lights=`）、`--lod-distance`（Web `?lod_distance=`）
与 `--ext <slug>`（Web `?ext=zoom,hud_info,item_inspector`）均只从命令行或浏览器查询参数读取，不写入
`options.txt`，也不提供游戏内配置界面；未指定时保持 Vanilla 默认行为（`off` / `0` / 空扩展集）。

## 生命周期和 fallback

options 文件缺失保留默认，非法行按 codec/迁移边界处理；迁移整组失败保留默认，未知 key
不获得任意执行权限。直接截断的 options 保存不具有 servers.dat 的备份回滚合同。
servers.dat 明确 opt-in，不在默认启动中改用户真实游戏目录。

窗口 mode 不存在时走实际平台 fallback；资源准备、平台配置和真实设备观察分别报告。
客户端只连接外部服务器，pause 不冻结外部世界，title/server-selection/Realms 与 integrated
server 的范围见客户端 ledger。相应字段或目录存在不是实现这些 UI 的证明。

## BBB 实现映射

| owner | 路径 | 输出 |
| --- | --- | --- |
| options | `bbb_client::options`、`datafix`、`derived` | 读取、迁移、生效值和报告 |
| bindings | `bbb_client::options::key_bindings` | key 名称与绑定 |
| startup | `bbb_client::startup` | CLI precedence、平台与 runtime 构造 |
| server list | `bbb_client::server_list` | 地址查找、hidden entry 与 icon 更新 |
| platform | `bbb_client::window_surface`、`window_icon` | scale/mode/cursor/title/icon |
| catalog generator | `scripts/generate-client-options-catalog.py` | source-derived 默认值和迁移身份 |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| load/datafix/save | `Options` | bbb-client | `aligned` | 迁移、坏行和 save tests |
| 地址匹配与 hidden 项 | `QuickPlay` / `ServerList` | server_list | `aligned` | opt-in 产品边界；保名、裁剪与更新测试 |
| GUI scale | `Window` | window_surface | `aligned` | 纯值 scale tests 与平台 host receipt 分开 |
| key binding | `KeyMapping` / options | key_bindings、input | `aligned` | catalog 与输入路由测试 |

## 结构性不变量

- CLI 未显式提供的默认值不能错误覆盖文件值。
- raw 保存值、effective option 与 runtime observation 不互相冒充。
- options 与服务器列表有不同写入语义，不能统一声称原子保存。
- 窗口配置只改变平台/presentation，不能新增可变 world owner。

## 聚焦测试（focused tests）和真实证据

`bbb_client::options` 的
`a_line_splits_on_its_first_colon_only_and_bad_lines_are_skipped`、
`old_options_are_data_fixed_before_current_fields_are_decoded`、
`a_throwing_legacy_key_fix_retains_all_defaults_like_options_load` 与
`save_round_trips_and_drops_keys_this_client_does_not_know` 覆盖文件语义。

`bbb_client::server_list` 的
`an_entry_the_player_already_named_for_the_address_is_reused` 与
`a_created_entry_heads_the_hidden_block_and_trims_it_to_sixteen` 覆盖地址与 hidden 顺序。
`bbb_client::startup::tests` 的
`server_list_owner_requires_explicit_store_name_and_live_connection` 验证启动门。

入口为 `cargo dev test -p bbb-client`、
`cargo dev test -p bbb-client server_list` 或
`cargo dev test -p bbb-client window_surface`。
纯计算和文件测试不能替代真实 display mode、Wayland seat、窗口 close 与 macOS platform evidence。

## 当前差异

游戏内配置 UI、服务器选择屏与其它范围外入口保留在 ledger；未消费 option 和实际平台支持
分别查 ledger 与平台验收边界。

## 账本归属

主 ID `persistence-options-file`；options-derived/apply/unconsumed、server-list、key mapping
和 platform rows 保留各自独立状态。[客户端账本](../../../unsupported-client-features.md)拥有
未关闭差异，[AGENTS.md](../../../../AGENTS.md)拥有推进原则。

## 精确重开条件

Vanilla option/registration/default/encoding、CLI precedence、键码、server-list schema、
window backend 或 display-coordinate 合同变化时重核。出现未显式启动却改文件、迁移失败
写坏配置、换 scale 后点击偏移或失焦留下按下状态时，按文件/平台/input owner 定位。

## 相关页面和源码索引

- [运行时](../runtime/client-runtime-and-control.md)：mailbox、资源恢复与退出。
- [离线资产](../resources/client-offline-asset-trust.md)：启动资源的可信来源。
- [字体与文本](../ui/client-font-and-text.md)：语言与字体投影。

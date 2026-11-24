# 客户端本地配置、按键与平台窗口

机制 ID：`persistence-options-file`

## 机制目的和边界

本页说明命令行参数（CLI）、`options.txt`、`servers.dat` 与平台窗口之间的交接边界。配置文件中的原始值、启动期实际生效的参数以及运行期平台观测属于三类独立状态：文件能够保存某个键，并不代表对应的运行期消费者已存在或启用。

所有配置仅在启动时通过命令行参数或 `options.txt` 提供，客户端不提供游戏内配置界面；按键路由与 HUD/屏幕交互见[屏幕、HUD 与库存](../ui/client-screens-hud-and-inventory.md)。

## Vanilla 26.1 权威实现

- `Options.java:1576-1595`：先收集原始配置键值，再按文件版本执行 datafix；文件缺失时保留默认值。
- `Options.java:1702-1709`、`Options.java:1759-1768`：直接重写 `options.txt`，最后广播客户端设置（`broadcastOptions`）。
- `QuickPlay.java:96-108`：按服务器地址查找列表条目，缺失时新增隐藏条目并写回后再连接。
- `Window.java:411-438`：GUI 缩放档位保持最小逻辑分辨率（320×240），启用强制 Unicode 字体时取偶数档位，逻辑宽高向上取整。

Windows、macOS、Linux 各平台窗口后端的具体行为无法仅由纯数值规则证明，实际平台支持以对应平台验收边界为准。

## 决策输入与状态所有权

| 输入/状态 | 唯一所有者 | 用途 |
| --- | --- | --- |
| CLI 参数及显式传参标记 | `bbb_client::startup` | 仅当显式传参时覆盖配置文件中的值 |
| 原始选项映射、版本号与动态分组 | `bbb_client::options` | datafix 迁移、编解码与诊断报告 |
| 按键翻译名与物理键位绑定 | `options::key_bindings` 与输入运行时 | 输入路由，不保存世界状态锁存 |
| 服务器地址、名称、图标与隐藏标志 | `bbb_client::server_list` | 显式启用时的 `servers.dat` 持久化 |
| 窗口尺寸/显示模式、GUI 缩放、光标与标题 | `window_surface` 与平台事件循环 | 向 winit 发起请求并观测实际平台状态 |
| 窗口图标、菜单背景与字体资源 | 资源包/客户端/渲染器各自代际 | 平台窗口或 GPU 渲染消费 |

## 判断与处理顺序

1. 解析 CLI 参数并记录哪些字段由用户显式传入；读取 `options.txt` 并按旧版本号执行 datafix 迁移。
2. 以目录默认值补全缺失字段，解码各项配置，再叠加显式传入的 CLI 覆盖值。
3. 启动流程将生效后的选项分发至各归属模块：视距与输入、音量、语言、渲染器、窗口等；文件原始保留值与产品是否消费分别判定。
4. 创建平台窗口，应用显示模式、尺寸、标题与窗口图标，并观测真实帧缓冲尺寸与刷新率。
5. 基于帧缓冲像素尺寸计算 GUI 缩放档位与逻辑尺寸；屏幕布局与鼠标命中共用同一缩放结果。
6. 平台事件作为输入消息入队，游戏动作仍由模拟层决定；渲染线程不直接修改世界状态。
7. 允许落盘的运行期配置变更交回文件所有者写盘，需要通知服务端的设置通过既有网络命令路径发送 `ClientInformation`。

### 本地文件读取与保存

`bbb_client::server_list` 是 `servers.dat` 的唯一所有者。它复刻 Vanilla 命令行直连进服路径：`QuickPlay.joinMultiplayerWorld` 读取列表、按**地址**（忽略名称，先查可见区再查隐藏区）匹配条目；若未找到则以 `selectServer.defaultName` 的位置新建一条隐藏条目并写回；随后 `ClientPacketListener.handleServerData` 经 `ServerList.saveSingleServer` 更新同一条目。因此 `--server-list-name` 仅在新建条目时生效，新建条目带有 `hidden` 标记、插在隐藏区首位并将隐藏区裁剪至最多 16 条。写回采用 `Util.safeReplaceFile` 策略（临时文件 + `_old` 备份 + 失败回滚），与 `options.txt` 的直接截断写入区分开来。`size`/`swap`/`replace`/`remove`/`unhide` 仅服务于多人服务器选择屏，不在外部直连客户端范围内（见[客户端账本](../../../unsupported-client-features.md)；`replace` 在 26.1 本身也无调用方）。BBB 将该所有者设为显式 opt-in：仅当同时提供 `--server-list-store` 与 `--server-list-name` 时才启用，避免默认启动触碰用户真实游戏目录。

`bbb_client::options` 是 `options.txt` 的唯一所有者，键名、编解码规则与默认值均由 `scripts/generate-client-options-catalog.py` 从 `Options.java` 机械派生。读取遵循 Vanilla 语义：仅在首个 `:` 切分、跳过格式错误行、缺文件回退默认值、未知键不回写；写入采用直接截断重写（不创建临时文件或备份，这与 `ServerList` 不同）。完整原始映射会先按文件 `version`（缺失或非法视作 0）执行源码派生的 `DataFixTypes.OPTIONS` 注册链，再进入现行编解码或启动绑定；注册顺序、迁移源码哈希与 LWJGL2 键码映射均由该生成脚本锁定，任一迁移抛错时整组保留默认值，与 `Options.load` 外层异常捕获一致。CLI 参数仅作为覆盖层，只有命令行显式提供时才会覆盖文件值。

启动与加载阶段按 Vanilla 生命周期写回 `startedCleanly`；运行期会触发写盘的快捷键包括 `F11`（`fullscreen`）、`Ctrl+B`（`narrator`）、`F3+H`（`advancedItemTooltips`）与 `F3+P`（`pauseOnLostFocus`），写盘后按 `Options.save` 收尾逻辑无条件补发一次 `ClientInformation`。桌面端 `F3+S` 会将动态纹理图集（地形、GUI、字体、粒子及动态玩家皮肤/纹理图集）以 PNG 导出至 `<game-dir>/screenshots/debug`（`wasm32` 不启用）。桌面端旁白开启且未指定 `--narrator-command` 时回退到内置操作系统 TTS 旁白器；显式传入 `--narrator-command`（及可选 `--narrator-command-arg`）时优先覆盖内置 TTS。

### 窗口与按键投影

`window_surface` 将零尺寸启动请求钳制至至少 1 像素。GUI 缩放档位从 1 起步，在下一档位仍能容纳 320×240 GUI 逻辑像素时递增；自动缩放（`0`）不受用户上限提前截断，强制 Unicode 字体时缩放档位强制为偶数。GUI 逻辑宽高对商向上取整，严禁直接把帧缓冲物理像素当作 GUI 坐标使用。

首选全屏模式的解析、匹配与回退由窗口所有者负责；配置请求值与实际显示器状态分开观测。刷新率从平台单位换算，光标样式变更受当前状态门控。窗口关闭走[客户端运行时统一退出](../runtime/client-runtime-and-control.md)，不得遗留孤立网络任务。

按键映射的标识与默认键位来自版本化目录；物理键位、按下/释放状态、点击消费与客户端玩法状态各归其主。旧版迁移中的 LWJGL2 键码与现行物理映射严格隔离，不形成第二套玩法动作。窗口焦点、屏幕抢占与按键释放顺序继续通过输入命令队列契约保证。

BBB 自有启动扩展参数 `--dynamic-lights`（Web `?dynamic_lights=`）、`--lod-distance`（Web `?lod_distance=`）与 `--ext <slug>`（Web `?ext=zoom,hud_info,item_inspector`）均只从命令行或浏览器查询参数读取，不写入 `options.txt`，也不提供游戏内配置界面；未指定时保持 Vanilla 默认行为（`off` / `0` / 空扩展集）。

## 生命周期和 fallback

`options.txt` 缺失时保留默认值，非法行按编解码或迁移边界跳过；迁移整组失败时整体回退默认值，未知键不具备任何执行能力。直接截断写入的 `options.txt` 不提供 `servers.dat` 的备份回滚语义。`servers.dat` 严格 opt-in，默认启动不修改用户真实游戏目录。

请求的显示模式不可用时回退至平台实际模式；资源准备、平台配置请求与真实设备观测分别报告。客户端仅连接外部多人服务器，暂停界面不冻结服务端世界；标题屏、服务器选择屏、Realms 与本地集成服务端均属范围外项，配置文件或目录中存在对应字段不代表已实现这些界面。

## BBB 实现映射

| 语义所有者 | 路径 | 输出 |
| --- | --- | --- |
| 本地配置 | `bbb_client::options`、`datafix`、`derived` | 文件读取、版本迁移、生效值与诊断报告 |
| 按键绑定 | `bbb_client::options::key_bindings` | 按键名称映射与绑定查询 |
| 启动编排 | `bbb_client::startup` | CLI 覆盖优先级、平台初始化与运行时构造 |
| 服务器列表 | `bbb_client::server_list` | 地址查找、隐藏条目维护与图标更新 |
| 平台窗口 | `bbb_client::window_surface`、`window_icon` | GUI 缩放、显示模式、光标、标题与窗口图标 |
| 目录生成器 | `scripts/generate-client-options-catalog.py` | 源码派生的默认值与迁移注册表 |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 配置读取、迁移与保存 | `Options` | `bbb_client::options` | `aligned` | 迁移、坏行跳过与写回往返测试 |
| 地址匹配与隐藏条目 | `QuickPlay` / `ServerList` | `bbb_client::server_list` | `aligned` | opt-in 产品边界；保名、裁剪与单条更新测试 |
| GUI 缩放计算 | `Window` | `bbb_client::window_surface` | `aligned` | 纯数值缩放测试（与实机平台验收分开） |
| 按键绑定与路由 | `KeyMapping` / `Options` | `key_bindings`、`input` | `aligned` | 目录一致性与输入路由测试 |

## 结构性不变量

- CLI 未显式传入的默认值不得覆盖 `options.txt` 中的已有值。
- 文件原始保存值、启动生效选项与运行期平台观测三者不得互相冒充。
- `options.txt` 与 `servers.dat` 拥有不同写盘语义，不得统一宣称为原子备份写入。
- 窗口配置只影响平台与呈现层，不得引入第二个可变世界状态所有者。

## 聚焦测试（focused tests）和真实证据

`bbb_client::options` 的以下用例覆盖文件语义：

- `a_line_splits_on_its_first_colon_only_and_bad_lines_are_skipped`
- `old_options_are_data_fixed_before_current_fields_are_decoded`
- `a_throwing_legacy_key_fix_retains_all_defaults_like_options_load`
- `save_round_trips_and_drops_keys_this_client_does_not_know`

`bbb_client::server_list` 的以下用例覆盖地址匹配与隐藏区顺序：

- `an_entry_the_player_already_named_for_the_address_is_reused`
- `a_created_entry_heads_the_hidden_block_and_trims_it_to_sixteen`

`bbb_client::startup::tests::server_list_owner_requires_explicit_store_name_and_live_connection` 验证 `servers.dat` 的显式启用门槛。

测试入口：`cargo dev test -p bbb-client`、`cargo dev test -p bbb-client server_list` 或 `cargo dev test -p bbb-client window_surface`。纯数值计算和文件测试不能替代真实显示模式、Wayland 窗口关闭或跨平台图形环境的实机证据。

## 当前差异

[客户端账本](../../../unsupported-client-features.md)当前无未关闭的 `# Todo` 缺口；本机制涉及的产品范围裁决（游戏内配置界面、多人服务器选择屏以及 X11/macOS/Windows 自动化平台发布回执）均登记在账本的 `# Not needed` 段落中。

## 账本归属

主机制 ID 为 `persistence-options-file`。未关闭差异与范围裁决统一由[客户端账本](../../../unsupported-client-features.md)维护，推进原则见 [AGENTS.md](../../../../AGENTS.md)。

## 精确重开条件

当 Vanilla 调整选项注册表、默认值、编解码、迁移链、键码或 `servers.dat` 结构，或 BBB 更改 CLI 覆盖优先级、窗口后端或显示坐标契约时，须重新核验。若出现未显式传参却改写配置文件、迁移失败损坏原文件、切换缩放后点击坐标偏移，或失焦后残留按键按下状态，按对应文件/平台/输入所有者定位修复。

## 相关页面和源码索引

- [客户端运行时](../runtime/client-runtime-and-control.md)：命令队列、资源恢复与统一退出。
- [离线资产可信链](../resources/client-offline-asset-trust.md)：启动资源的可信来源。
- [字体与文本](../ui/client-font-and-text.md)：语言与字体投影。

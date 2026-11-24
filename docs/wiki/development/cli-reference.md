# 命令行参数与 Web 查询参数参考

BBB 客户端、独立服务端（Dedicated Server）、Web 网关与管理工具均采用启动期配置，不提供游戏内设置界面。
本页汇总 `bbb-client`、`bbb-server`、`bbb-web-host`、`bbb-admin`、Web 浏览器 URL 查询参数以及
`cargo dev web-up` / `cargo build-web` 的权威参数面（具体解析实现以各自 crate 源码为准）。

## `bbb-client`（桌面客户端）

源码入口：`crates/bbb-client/src/startup.rs`。客户端先从 `--game-dir` 加载 `options.txt`，再由显式传入的
CLI 参数覆盖对应选项（未显式传入的 CLI 默认值不覆盖已保存配置）。

### 连接、身份与目录

| 参数 | 默认值 | 说明 |
| --- | --- | --- |
| `--game-dir <PATH>` | `.` | 管理 `options.txt`、`resourcepacks/`、`downloads/`（服务端资源包缓存）、`screenshots/`（含 `screenshots/debug/`）与 `videos/` 的目录；不自动读写 `servers.dat`（仅配合 `--server-list-store` + `--server-list-name` + `--connect-server` 使用）或 `hotbar.nbt`（位于私有配置目录 `%APPDATA%\bbb-client\hotbar.nbt` / `~/Library/Application Support/bbb-client/hotbar.nbt` / `${XDG_CONFIG_HOME:-~/.config}/bbb-client/hotbar.nbt`，可用 `BBB_HOTBAR_PRESET_STORE` 覆盖） |
| `--vanilla-root <PATH>` | 回退链：`BBB_VANILLA_ROOT` → `BBB_VANILLA_DIR` → `--game-dir` → 平台缓存 / `.minecraft` | 由 `cargo dev vanilla prepare 26.1` 准备的官方 26.1 资源根目录 |
| `--server <ADDR>` | `127.0.0.1:25565` | 目标 Minecraft Java 26.1 离线服务器地址（显式传入时默认隐含 `--connect-server`，与 `--status-server` / `--probe-server` 组合时除外） |
| `--connect-server` | `false` | 启动后直接连接 `--server`（显式传入 `--server <ADDR>` 时自动启用；省略 `--server` 时用于直连默认地址 `127.0.0.1:25565`） |
| `--quick-play-multiplayer <ADDR>` | 无 | Vanilla 快速联机入口，与 `--status-server`、`--probe-server` 互斥 |
| `--status-server <ADDR>` | 无 | 仅查询服务器列表状态并退出（与 `--probe-server` 互斥） |
| `--username <NAME>` | 随机 `Player<6位数字>` | 离线登录玩家名（未显式指定时每次启动在 `100000..=999999` 范围随机生成） |
| `--uuid <UUID>` | 无 | 显式离线 UUID（兼容带/不带连字符；非法时告警并回退至用户名派生 UUID） |
| `--build-info` | `false` | 打印构建信息并退出 |
| `--server-list-store <PATH>` / `--server-list-name <NAME>` | 无 | 配合 `--connect-server` 显式持久化或更新服务器列表记录 |
| `--code-of-conduct-store <PATH>` | 私有配置存储 | 覆盖已同意服务器行为准则存储路径 |
| `--crash-report-dir <PATH>` | `crash-reports`（相对当前工作目录） | 崩溃报告输出目录 |

### 窗口、显示与画面

> **画质预设默认值说明**：默认的 `fancy` 画质预设会覆盖部分原始 CLI 默认值（`render-distance=16`、`prioritize-chunk-updates=player-affected`、`cloud-range=64`、`terrain-texture-filtering=rgss`、`max-anisotropy-bit=1`），除非在命令行显式传入对应参数，或 `options.txt` 中设置了 `graphicsPreset:"custom"`。

| 参数 | 默认值 | 说明 |
| --- | --- | --- |
| `--width <INT>` / `--height <INT>` | `854` / `480` | 初始窗口宽高（退化值钳位到 1） |
| `--fullscreen [BOOL]` | `false` | 启动全屏（同时支持无参 `--fullscreen` 与 `--fullscreen true/false`） |
| `--fullscreen-width <INT>` / `--fullscreen-height <INT>` | 无 | 全屏首选分辨率宽高 |
| `--fullscreen-resolution <SPEC>` | 无 | 显式全屏显示模式（如 `1920x1080@60`） |
| `--gui-scale <0..>` | `0`（自动） | GUI 缩放倍率 |
| `--render-distance <2..=32>` | `12` | 渲染视距（区块） |
| `--simulation-distance <5..=32>` | `12` | 客户端模拟视距（区块） |
| `--graphics-preset <fast\|fancy\|fabulous\|custom>` | `fancy` | 画质预设 |
| `--improved-transparency` | `false` | 启用改进透明合成（Fabulous 透明排序管线） |
| `--biome-blend-radius <0..=7>` | `2` | 群系颜色混合半径 |
| `--prioritize-chunk-updates <none\|player-affected\|nearby>` | `none` | 区块网格重编译优先级策略 |
| `--mipmap-levels <0..=4>` | `4` | 地形纹理 Mipmap 层数 |
| `--terrain-texture-filtering <none\|rgss\|anisotropic>` | `none` | 地形纹理过滤模式（`--max-anisotropy-bit <1..=3>` 控制各向异性阶数，默认 `2`） |
| `--vsync <BOOL>` | `true` | 垂直同步开关 |
| `--framerate-limit <10..=260\|inf\|unlimited>` | `260`（无上限） | 帧率上限（同时接受 `inf` / `unlimited` 表示无上限） |
| `--inactivity-fps-limit <minimized\|afk>` | `afk` | 非活跃降帧策略 |
| `--gamma <FLOAT>` | `0.5` | 亮度因子 |
| `--fov <30..=110>` / `--fov-effect-scale <0..=1>` | `70` / `1.0` | 视野角及疾跑/状态效果 FOV 缩放 |
| `--smooth-lighting <BOOL>` | `true` | 平滑光照开关 |
| `--view-bobbing <BOOL>` | `true` | 行走视角晃动开关 |
| `--render-clouds <off\|fast\|fancy>` / `--cloud-range <2..=128>` | `fancy` / `128` | 云层模式与云距 |
| `--cutout-leaves <BOOL>` / `--entity-shadows <BOOL>` | `true` / `true` | 镂空树叶与实体阴影 |
| `--particles <all\|decreased\|minimal>` | `all` | 粒子密度 |
| `--entity-distance-scaling <0.5..=5.0>` | `1.0` | 实体渲染距离倍率 |
| `--weather-radius <3..=10>` | `10` | 天气粒子半径 |
| `--glint-speed <0..=1>` / `--glint-strength <0..=1>` | `0.5` / `0.75` | 附魔流光速度与强度 |
| `--menu-background-blurriness <0..=10>` | `5` | 菜单背景模糊强度 |
| `--damage-tilt-strength <0..=1>` | `1.0` | 受伤视角倾斜强度 |
| `--vignette <BOOL>` / `--screen-effect-scale <FLOAT>` | `true` / `1.0` | 暗角与屏幕特效倍率 |
| `--hide-lightning-flash` / `--high-contrast-block-outline` | `false` | 隐藏闪电闪白 / 高对比度方块描边 |
| `--advanced-item-tooltips` / `--pause-on-lost-focus <BOOL>` | `false` / `true` | 高级物品提示 / 失焦暂停 |

### 语言、字体、音频、输入与聊天

| 参数 | 默认值 | 说明 |
| --- | --- | --- |
| `--locale <CODE>` | `en_us` | 客户端语言代码（如 `zh_cn`） |
| `--force-unicode-font <BOOL>` / `--japanese-glyph-variants <BOOL>` | `false` / `false` | 强制 Unicode 字体 / 日文字形变体 |
| `--narrator <off\|all\|chat\|system>` / `--narrator-hotkey <BOOL>` | `off` / `true` | 旁白模式与 `Ctrl+B` 快捷键 |
| `--narrator-command <PATH>` / `--narrator-command-arg <ARG>` | 无（使用内置桌面操作系统 TTS） | 覆盖内置桌面操作系统语音合成（TTS）旁白的自定义外部命令及参数 |
| `--show-subtitles <BOOL>` / `--notification-display-time <FLOAT>` | `false` / `1.0` | 字幕显示与通知停留倍率 |
| `--sound-device <NAME>` / `--directional-audio <BOOL>` | 空（系统默认） / `false` | 音频输出设备与 OpenAL HRTF 定向音频 |
| `--music-toast <never\|pause\|pause-and-toast>` / `--music-frequency <default\|frequent\|constant>` | `never` / `default` | 音乐提示与播放频率 |
| `--raw-mouse-input <BOOL>` / `--allow-cursor-changes <BOOL>` | `true` / `true` | 原始鼠标输入与指针样式切换 |
| `--mouse-sensitivity <FLOAT>` / `--mouse-wheel-sensitivity <FLOAT>` | `0.5` / `1.0` | 鼠标与滚轮灵敏度 |
| `--invert-mouse-x <BOOL>` / `--invert-mouse-y <BOOL>` / `--discrete-mouse-scroll <BOOL>` | `false` | 鼠标轴反转与离散滚轮 |
| `--toggle-crouch\|sprint\|attack\|use <BOOL>` / `--auto-jump <BOOL>` | `false` / `true` | 按键切换模式与自动跳跃 |
| `--main-hand <left\|right>` / `--skin-parts <U8>` / `--attack-indicator <off\|crosshair\|hotbar>` | `right` / `0x7f` / `crosshair` | 主手、皮肤图层掩码与攻击冷却指示器 |
| `--chat-visibility <full\|system\|hidden>` / `--chat-delay-seconds <FLOAT>` | `full` / `0.0` | 聊天可见性与延迟 |
| `--chat-scale\|width\|height-focused\|height-unfocused\|line-spacing\|opacity <FLOAT>` | Vanilla 默认 | 聊天框缩放、尺寸、行距与透明度 |
| `--chat-colors\|links\|links-prompt <BOOL>` / `--save-chat-drafts <BOOL>` | `true` / `false` | 聊天格式色、链接提示与草稿保存 |
| `--only-show-secure-chat <BOOL>` / `--hide-matched-names <BOOL>` / `--text-filtering <BOOL>` | `false` / `true` / `false` | 安全聊天过滤与名称屏蔽 |
| `--allow-server-listing <BOOL>` / `--blocked-player <UUID>` | `false` / 空 | 服务端列表匿名控制与屏蔽玩家列表（可重复） |
| `--text-background-opacity <0..=1>` / `--background-for-chat-only <BOOL>` | `0.5` / `true` | 文本背景不透明度与作用范围 |
| `--tutorial-step <STEP>` / `--joined-first-server <BOOL>` | `movement` / `false` | 教程步骤与首次联机标记 |

### 资源包、代理、引擎底座与扩展

| 参数 | 默认值 | 说明 |
| --- | --- | --- |
| `--resource-pack-dir <PATH>` | 空 | 启动加载的本地资源包路径（可重复） |
| `--server-resource-pack-policy <prompt\|accept\|decline>` | 无（未保存时弹窗） | 服务端推送资源包的自动策略 |
| `--server-resource-pack-cache-dir <PATH>` / `--player-skin-cache-dir <PATH>` | `<game-dir>/downloads` / `<temp>/bbb-client-player-skins`（Web 端为内存路径 `/__bbb_mem__/player-skins`） | 服务端资源包与玩家皮肤缓存目录 |
| `--proxy-host <HOST>`、`--proxy-port <PORT>`、`--proxy-user`、`--proxy-pass` | 无（端口默认 `8080`） | 服务端资源包下载显式 SOCKS5 代理 |
| `--dynamic-lights <off\|fast\|fancy>` | `off` | L2 渲染底座：手持/实体点光源模式 |
| `--lod-distance <0 或 16..=256>` | `0`（关闭） | L2 渲染底座：远景地表 LOD 视距（区块） |
| `--ext <SLUGS>` | 空 | L3 客户端体验增强扩展（逗号分隔，如 `zoom,hud_info,item_inspector`） |

### 控制、MCP、截图、录像、调试与无头探针

| 参数 | 默认值 | 说明 |
| --- | --- | --- |
| `--control <ADDR>` / `--mcp-bind <ADDR>` | 无 | 暴露本地行分隔 JSON 控制端或 Streamable HTTP MCP 端（仅环回地址） |
| `--lockstep` / `--lockstep-audio-log <PATH>` | `false` / 无 | 确定性虚拟时钟步进（需 `--control` 或 `--mcp-bind`）及音频 JSONL 日志 |
| `--screenshot <PATH>` / `--exit-after-screenshot` | 无 / `false` | 进服渲染后自动截图并可选退出 |
| `--record-video <PATH>`、`--record-fps <1..=120>`、`--record-bitrate <U32>` | 无 / `30` / `8000000` | 原生 H.264 + AAC MP4 录像（仅 Windows） |
| `--reduced-debug-info <BOOL>`；内部隐藏调试参数：`--debug-profile`、`--debug-profile-store <PATH>`、`--debug-hotkeys`、`--debug-feature-count`、`--debug-show-local-server-entity-hit-boxes`、`--renderer-input-latency-diagnostics`、`--remote-debug-renderer <RENDERER>` | 关闭 / `default` | 调试覆层开关与隐藏的开发者诊断参数（`--help` 默认隐藏） |
| 内部隐藏探针参数：`--probe-server` 及 `--probe-after-first-chunk-packets`、`--probe-server-interactions`、`--probe-server-operator`、`--probe-server-dimension-transitions`、`--probe-server-entity-runtime`、`--probe-server-container-menu`、`--probe-timeout-seconds`、`--probe-hold-seconds` | 关闭 | 无头协议/玩法探针模式（`--help` 默认隐藏，由 `bbb-client-probe` 与服务端 E2E 门禁驱动） |

---

## `bbb-server`（独立服务端）

源码入口：`crates/bbb-server/src/options.rs`。标有「持久化覆盖」的项在 `--admin-dir` 下
`server-settings.json` 与 CLI 之间遵循 `显式 CLI > server-settings.json > Vanilla 默认值`。

### 监听、连接背压与生命周期

| 参数 | 默认值 | 说明 |
| --- | --- | --- |
| `--bind <ADDR>` | `127.0.0.1:25565` | Minecraft 26.1 TCP 监听地址 |
| `--max-connections <USIZE>` | `128` | 允许同时存在的 TCP 连接任务上限（含状态探测） |
| `--max-connections-per-ip <USIZE>` | `16` | 单个远端 IP 同时允许建立的 TCP 连接上限（本机多客户端并发需按需调高） |
| `--compression-threshold <I32>` | `256` | 协议压缩阈值（字节）；`-1` 禁用压缩 |
| `--connection-event-entries <USIZE>` / `--connection-event-bytes <USIZE>` | `64` / `2097152`（2 MiB） | 单连接网络→世界入站队列条数与字节上限 |
| `--connection-command-entries <USIZE>` / `--connection-command-bytes <USIZE>` | `64` / `16777216`（16 MiB） | 单连接世界→网络出站队列条数与字节上限 |
| `--shutdown-timeout-seconds <U64>` | `10` | 收到 `SIGINT`/`SIGTERM` 后等待连接排空的超时秒数 |

### 世界、存档、玩法与准入

| 参数 | 默认值 | 说明 |
| --- | --- | --- |
| `--world <PATH>` | 无（临时超平坦世界） | 26.1 存档目录；空目录按下列预设创建新世界，退出时落盘 |
| `--level-seed <STRING>` | 空（随机） | 新建世界种子（数字直接使用，文本按 Java `String.hashCode` 哈希） |
| `--level-type <PRESET>` | `normal` | 新建世界预设：`normal`、`flat`、`large_biomes`、`amplified`（允许 `minecraft:` 前缀） |
| `--generate-structures <BOOL>` / `--hardcore` | `true` / `false` | 新建世界是否生成结构 / 极限模式 |
| `--safe-mode` | `false` | 仅加载内置 vanilla 数据包，忽略存档选包 |
| `--motd <TEXT>` | `A BBB Server` | 状态列表描述（持久化覆盖） |
| `--max-players <I32>` | `20` | 最大在线人数（持久化覆盖） |
| `--difficulty <peaceful\|easy\|normal\|hard>` | `easy` | 世界难度（持久化覆盖） |
| `--gamemode <survival\|creative\|adventure\|spectator>` / `--force-gamemode` | `survival` / `false` | 默认游戏模式（持久化覆盖）与强制覆盖老玩家模式 |
| `--view-distance <2..=32>` / `--simulation-distance <2..=32>` | `10` / `10` | 视距与模拟距离（持久化覆盖） |
| `--spawn-protection <I32>` | `16` | 出生点保护半径，`<= 0` 关闭（持久化覆盖） |
| `--allow-flight <BOOL>` | `false` | 允许浮空飞行不踢出（持久化覆盖） |
| `--pause-when-empty-seconds <I32>` | `60` | 无人在线后保存并暂停模拟的秒数，`<= 0` 禁用（持久化覆盖） |
| `--entity-broadcast-range-percentage <10..=1000>` | `100` | 实体追踪广播距离百分比（持久化覆盖） |
| `--enable-status <BOOL>` / `--hide-online-players <BOOL>` / `--accepts-transfers <BOOL>` | `true` / `false` / `false` | 状态回复、匿名在线列表与接受跨服转移（持久化覆盖） |
| `--op-permission-level <0..=4>` / `--function-permission-level <0..=4>` | `4` / `2` | 管理员（OP）默认权限等级（持久化覆盖）与 `.mcfunction` 执行权限等级 |
| `--admin-dir <PATH>` | 无（仅内存） | 存放 `whitelist.json`、`ops.json`、`banned-players.json`、`banned-ips.json` 与 `server-settings.json` |
| `--white-list <BOOL>` / `--enforce-whitelist <BOOL>` | `false` / `false` | 启用白名单与踢出不在白名单中的在线玩家（持久化覆盖） |
| `--player-idle-timeout <MIN>` | `0`（不踢出） | 玩家空闲踢出分钟数（持久化覆盖） |
| `--bug-report-link <URL>` / `--code-of-conduct <DIR>` | 无 | 服务器反馈链接（`http`/`https`）与 `<lang>.txt` 行为准则目录 |
| `--resource-pack <URL>`、`--resource-pack-sha1 <HEX40>`、`--resource-pack-id <UUID>`、`--require-resource-pack`、`--resource-pack-prompt <JSON>` | 无 | 服务端推送资源包配置 |
| `--mod <SLUG>` / `--mod-packs <PATH>`（别名 `--mod-pack`） | 空 | 启用内置玩法扩展（如 `demo_boss`）或加载数据驱动模组清单/目录（可重复） |

### MCP 管理端与安全

| 参数 | 默认值 | 说明 |
| --- | --- | --- |
| `--no-mcp` | `false` | 完全关闭 MCP 管理端 |
| `--mcp-bind <ADDR>` / `--mcp-path <PATH>` | `127.0.0.1:25585` / `/mcp` | Streamable HTTP MCP 监听地址与路径；非环回地址必须同时配置 TLS 与 OAuth 2.1 |
| `--mcp-loopback-scope <none\|read\|manage\|command>` | `manage` | 本机环回无凭据调用授予的权限范围（`none` 强制本机也出示令牌） |
| `--mcp-token-file <PATH>` | 无 | Bearer 凭据文件（每行 `principal:scope:secret`，不经命令行参数传密钥） |
| `--mcp-allowed-origin <ORIGIN>` | 空（全拒） | 允许的浏览器 `Origin` 白名单（可重复） |
| `--mcp-tls-cert <PATH>` / `--mcp-tls-key <PATH>` | 无 | 远程 MCP 的 PEM 证书链与私钥 |
| `--mcp-oauth-resource <URI>`、`--mcp-oauth-issuer <ISSUER>`、`--mcp-oauth-jwks <PATH>` | 无 | 远程 MCP 的 OAuth 2.1 资源标识（`aud`）与按位置配对的签发方/JWKS |
| `--mcp-request-timeout-seconds <U64>` | `10` | 管理请求等待主世界 tick 响应的秒数（超时转为异步操作记录供查询） |

---

## `bbb-web-host`（Web 网关）

源码入口：`crates/bbb-web-host/src/main.rs`。

| 参数 | 默认值 | 说明 |
| --- | --- | --- |
| `--bind <ADDR>` | `127.0.0.1:8080` | HTTP / WebSocket 监听地址 |
| `--server <HOST:PORT>` | `127.0.0.1:25565` | 默认上游 Minecraft Java 26.1 服务器 |
| `--allow-server <HOST:PORT>` | 空 | 额外允许 `?server=` 连接的上游地址白名单（可重复） |
| `--vanilla-root <PATH>` | `BBB_VANILLA_ROOT` 或平台缓存 | 已准备的 Vanilla 26.1 资源根目录，用于组装 `core-26.1.zip` |
| `--wasm-dir <PATH>` | `BBB_WEB_OUT_DIR` 或 `<target-root>/bbb-web-dist`（`<target-root>` 依次取 `BBB_TARGET_ROOT`、Unix `/tmp` 或 Windows `%TEMP%`） | `cargo build-web` 产出的 WASM 静态目录 |
| `--cache-dir <PATH>` | `<target-root>/bbb-web-cache` | `core-26.1.zip` 磁盘缓存目录（`cargo dev web-up` 与 E2E 脚本另支持 `BBB_WEB_CACHE_DIR` 覆盖） |
| `--username <NAME>` | 空（每浏览器配置自动生成） | `?username=` 省略时的默认离线玩家名 |
| `--locale <CODE>` | 无 | `?lang=` / `?locale=` 省略时的默认语言代码 |
| `--uuid <UUID>` | 无 | `?uuid=` 省略时的默认离线 UUID |
| `--render-distance <2..=32>` | 无 | `?render_distance=` 省略时的默认渲染视距 |
| `--server-resource-pack-policy <prompt\|accept\|decline>` | 无（默认 `prompt`） | 服务端资源包默认策略 |
| `--allow-pack-loopback` | `false` | 允许 `/proxy/pack` 访问环回地址（仅限本地联调与测试） |
| `--allow-pack-private` | `false` | 允许 `/proxy/pack` 访问 RFC 1918 / ULA 私网地址（自托管局域网资源包；不放行环回、链路本地或云元数据地址，不影响 `/proxy/skin`） |
| `--mcp-bind <ADDR>` / `--control <ADDR>` | 无 | 仅限环回地址：通过浏览器 `/control-ws` 反向暴露 MCP 与行分隔 JSON 控制端 |

---

## `bbb-admin`（MCP 管理 CLI）

源码入口：`crates/bbb-admin/src/cli.rs`。全局参数位于任意子命令之前或之后均可生效：

| 全局参数 | 默认值 | 说明 |
| --- | --- | --- |
| `--endpoint <URL>` | `http://127.0.0.1:25585/mcp` | 目标 MCP 管理端 URL（`http://` 或 `https://`） |
| `--token-file <PATH>` | 无 | 存放 Bearer 密钥的文件（取首条非空非注释行） |
| `--ca-file <PATH>` | 空 | 信任的自签/私建 PEM CA 证书（可重复） |
| `--timeout-seconds <U64>` | `30` | 单次请求超时秒数 |
| `--raw` | `false` | 输出原始 JSON-RPC 结果对象 |

子命令：`hello`、`status`、`tools [--schemas] [--matching <TEXT>]`、`resources`、`read <URI>`、
`operation <ID>`、`call <TOOL> [NAME=VALUE ...] [--arguments-json <JSON>] [--request-id <ID>]`、
`watch <URI...> [--events <COUNT>]`。

---

## Web 客户端 URL 查询参数

源码入口：`crates/bbb-client/src/startup/web.rs`。浏览器地址栏查询参数覆盖网关下发的默认配置：

| 查询参数 | 合法值 / 说明 |
| --- | --- |
| `server` | 上游 `host:port`（必须在 `{--server} ∪ {--allow-server ...}` 白名单内） |
| `username` / `uuid` | 覆盖当前标签页的离线玩家名与 UUID |
| `server_resource_pack_policy`（别名 `resource_pack_policy`） | `prompt`、`accept`、`decline` |
| `render_distance` | `2..=32` 区块 |
| `fov` | `30..=110` 度 |
| `gui_scale` | `0`（自动）或正整数 |
| `graphics` | `fast`、`fancy`、`fabulous`、`custom` |
| `improved_transparency` | 布尔值（`true`/`false`、`1`/`0`、`on`/`off`、`yes`/`no`）：改进透明合成 |
| `clouds` | `off`（或 `false`/`0`）、`fast`、`fancy`（或 `true`/`1`） |
| `vsync` | 布尔值：垂直同步开关 |
| `gamma` | 浮点数：亮度因子 |
| `smooth_lighting` | 布尔值：平滑光照开关 |
| `particles` | `all`/`0`、`decreased`/`1`、`minimal`/`2` |
| `subtitles` | 布尔值：字幕显示开关 |
| `sensitivity` | `0.0..=1.0`：鼠标灵敏度 |
| `bob_view` | 布尔值：行走视角晃动开关 |
| `fov_effect_scale` | `0.0..=1.0`：疾跑/状态效果 FOV 缩放 |
| `lang`（别名 `locale`） | 语言代码（如 `zh_cn`、`en_us`，RTL 语言包按需拉取） |
| `dynamic_lights` | `off`、`fast`、`fancy` |
| `lod_distance` | `0`（关闭）或 `16..=256` 区块 |
| `ext` | 逗号分隔或重复传入的体验增强扩展标识符（如 `zoom,hud_info,item_inspector`） |
| `narrator` | `off`/`0`、`all`/`1`、`chat`/`2`、`system`/`3` |
| `volume_<category>` / `soundCategory_<category>` | `0.0..=1.0` 音量覆盖；`<category>` 支持全部 11 类：`master`、`music`、`record`、`weather`、`block`、`hostile`、`neutral`、`player`、`ambient`、`voice`、`ui` |
| `opt.<key>` | 通用 `options.txt` 键与 `key_*` 按键绑定透传（如 `opt.renderDistance=12`、`opt.key_key.attack=key.mouse.right`）；按白名单与类型校验，仅在当前标签页内存生效，不回写 `localStorage["bbb.options.txt"]`，未知键或非法值拒绝启动 |
| `pack` | 初始 `.zip` 资源包 URL（可重复传入，经 `/proxy/pack` 校验，共享 256 MiB 上限） |
| `control` | `1` / `true`：强制启用反向 `/control-ws` 通道 |
| `software_gpu` | `1` / `true`：允许选择软件 WebGPU 适配器（供 CI/SwiftShader E2E） |
| `lockstep` | `1` / `true`：确定性步进时钟模式 |

---

## `cargo build-web` 与 `cargo dev web-up`

源码入口：`crates/bbb-xtask/src/web.rs`（可通过 `cargo build-web --help` 与 `cargo web-up --help` 实时查看）：

- **`cargo build-web`**：`--dev`、`--release`、`--profile <dev|release|wasm-release>`（默认 `$BBB_WEB_PROFILE` 或 `release`）、`--out-dir <dir>`、`--wasm-target-dir <dir>`。
- **`cargo dev web-up` / `cargo web-up`**：`--bind <addr>`、`--server-bind <addr>`、`--vanilla-root <dir>`、`--wasm-dir <dir>`、`--cache-dir <dir>`、`--world <dir>`、`--gamemode <mode>`、`--difficulty <level>`、`--view-distance <chunks>`、`--admin-dir <dir>`、`--username <name>`、`--locale <code>`（默认 `zh_cn`）、`--control <addr>`、`--mcp-bind <addr>`、`--allow-pack-loopback`、`--rebuild-web`、`--release`。

## 相关文档

- [Web 客户端与网关](web-client.md)
- [安全与生产部署指南](security-and-deployment.md)
- [开发流程、测试与发布](testing-and-gates.md)
- [打包与托管配方](../../../packaging/README.md)

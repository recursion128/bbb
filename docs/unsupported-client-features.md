# 客户端功能差异

本账本仅登记 `bbb` 客户端与 Minecraft Java 26.1 之间的未关闭功能缺口（active / 带 `Reopen when ...` 的 deferred）以及 `# Not needed` 范围裁决；不保留完成历史，功能落地时直接删除对应条目，历史由 Git 维护。

维护规则见[文档工作流](wiki/development/documentation-workflow.md#ledger-与接手信息)。

# Todo

以下只登记主分支生产路径中玩家可观察的功能缺口，验收按 [`AGENTS.md`](../AGENTS.md) 的推进原则。

# Not needed

以下保留 Vanilla 26.1 行为依据、产品范围裁决及重开条件。

<a id="official-player-reporting"></a>

## 官方玩家举报

- **官方玩家举报为 `not-needed`。** 客户端明确不接入 Mojang 官方玩家举报服务，也不保留本地模拟界面，涵盖聊天/皮肤/名称举报入口、举报原因与消息选择、备注、草稿与退出确认、证据打包、异步提交、结果页、举报专用资产及控制投影（`ReportPlayerScreen`、`ReportingContext`）。入站消息签名验证、已见消息确认、聊天信任标签、社交隐藏/屏蔽与近期玩家记录仍独立支持。重开条件：仅当完整接入官方玩家举报服务纳入声明的产品范围时重开。

## 会话、身份与遥测

- **正版在线模式（认证 Services 与加密传输）为 `not-needed`。** 声明的客户端范围为仅离线模式。遇到发起 Vanilla 加密握手的服务器时，客户端在协商任何加密套件前即返回可归因的 `"online-mode servers are unsupported"` 错误并终止连接。远端玩家的签名聊天与宣告会话仍使用公开 Services 公钥集解码并校验，可签名命令参数保留 Vanilla 签名命令包格式与空签名列表（与无密钥对的 Vanilla 客户端一致）。Vanilla `Main --uuid` 与在线凭据不同，它用于选择离线启动身份，因此 BBB 保留该 CLI 接口及 Authlib/JDK 宽容解析与用户名派生回退，供登录、权威本地玩家状态与服务端资源包请求头使用。离线服务边界遵循 Vanilla `UserApiService.OFFLINE_PROPERTIES` 语义（权限标志与封禁范围均为空），因此账号标志聊天限制与 Vanilla 离线回退一样保持惰性。重开条件：仅当推翻仅离线范围裁决，并将加密登录、会话加入与档案密钥生命周期纳入产品范围时重开。
- **Mojang 客户端遥测回调为 `not-needed`。** `bbb` 没有外部遥测上报器。Vanilla `WorldSessionTelemetryManager.onServerBrandReceived` 仅把服务端品牌写入 `WorldLoadEvent` 并尝试发送 Mojang 遥测；服务端品牌协议状态与本地诊断仍独立拥有。重开条件：若离线外部客户端范围新增遥测发送器时重开。

## 平台发布门禁

- **X11 发布行收据为 `not-needed`。** 声明的 Linux 发布行为 Wayland。`bbb` 保留 `winit` 的 X11 分支作为可达实现细节，但不签发 X11 平台发布收据。重开条件：仅当推翻该范围裁决时重开。
- **macOS aarch64 客户端发布收据（`platform-macos-release`）为 `not-needed`。** 桌面客户端交付覆盖 Linux x86_64 与 Windows x86_64。客户端仍支持针对 macOS 编译，且 `scripts/cross_check.py` 持续编译验证该目标，但没有维护中的 macOS 桌面 GUI 物理运行器，因此 Cocoa/Metal/Quartz/CoreAudio 原生宿主收据不在交付范围内。重开条件：仅当推翻该裁决并具备维护中的 macOS aarch64 桌面运行器可产出收据时重开。
- **Windows x86_64 自动化物理宿主收据（`platform-windows-release`）为 `not-needed`。** Windows x86_64 / DX12 原生客户端构建、启动、软件 GPU WARP 端到端测试（`cargo dev e2e client --software-gpu`）与 `cargo dev` 工作流均受支持；`scripts/cross_check.py` 在非 macOS 宿主默认检查 `aarch64-apple-darwin`（在 macOS 宿主选择已安装的 `x86_64-unknown-linux-gnu` 或 `x86_64-pc-windows-msvc`，或在已安装时接受显式 `--target x86_64-pc-windows-msvc`）。由于 `scripts/platform-acceptance-gate.sh` 仅驱动 Unix 宿主且无维护中的自动化 Windows 物理 GUI 运行器，Win32/DX12/IME/WASAPI 原生宿主收据不在自动发布门禁范围内。重开条件：仅当推翻该裁决并新增维护中的 Windows x86_64 物理 GUI 运行器时重开。

## 输入、移动、菜单与交互

- **触屏拆分堆叠与回弹动画为 `not-needed`。** Vanilla 将两者均门控于 `Options.touchscreen`（`AbstractContainerScreen.java:336`, `342`, `411`, `489`；回弹插值 `:146-158`），而 `bbb` 没有触屏输入模式。

## 渲染、HUD、调试叠加层与配置

- **自动世界图标截图（`tryTakeScreenshotIfNeeded` / `takeAutoScreenshot`）为 `not-needed`。** 两者均门控于 `Minecraft.isLocalServer()` 与内置服务端的 `getWorldScreenshotFile()`，外部多人客户端永远不会进入 1000 ms 重试锁存、已渲染区段就绪检查或 64×64 世界图标写盘路径；手动 F2 截图保留各自所有者。重开条件：若 `bbb` 新增内置单人服务端面时重开。
- **`setRenderBlockOutline` 为 `not-needed`。** 其在 Vanilla 中仅用于 `Minecraft.grabHugeScreenshot` 的 4096 宽全景截图前后临时关闭并恢复方块轮廓，而 `bbb` 没有全景/巨幅截图模式（运行中的 `LevelFrameRoute` 全景标志恒为 `false`）。重开条件：随任何全景截图能力一并重开。
- **`visualize_chunks_on_server` 为 `not-needed`。** Vanilla 26.1 `ChunkDebugRenderer.emitGizmos` 仅在 `Minecraft.getSingleplayerServer()` 返回 `IntegratedServer` 时通过该服务端执行器读取 `ServerLevel` / `ServerChunkCache` 刷新数据；外部多人客户端没有内置服务端，Vanilla 等价行为为空。重开条件：若仓库新增进程内内置服务端边界时重开。
- **实体碰撞箱本地服务端绿框与差值箭头为 `not-needed`。** Vanilla 26.1 `EntityHitboxDebugRenderer.getServerEntity` 仅通过 `Minecraft.getSingleplayerServer()` → `IntegratedServer` → `ServerLevel.getEntity` 解析服务端实体；外部多人客户端恒得 `null` 并显示 `bbb` 已实现的红色 `Missing Server Entity` 3D 公告牌。重开条件：随进程内内置服务端边界重开。
- **无外部客户端 HUD 行的单人专用调试条目为 `not-needed`。** `LocalDifficulty`、`EntitySpawnCounts` 与 `ChunkGenerationStats` 均读取内置本地服务端镜像（`ServerLevel`、`NaturalSpawner.SpawnState`、`ChunkGenerator` / `RandomState` / `BiomeSource`），外部客户端不拥有这些对象。
- **与 Vanilla 截图逐像素黄金哈希一致为 `not-needed`。** 驱动与浮点光栅抖动使跨独立 GPU 技术栈的全帧像素哈希不可靠。重开条件：仅当同后端、同驱动确定性光栅化合同可用时重开。
- **`F3+F4` `GameModeSwitcherScreen` 鼠标释放修饰键路径为 `not-needed`。** Vanilla 仅对 `keyDebugModifier.matchesMouse(event)` 使用 `mouseReleased`（`KeyMapping.matchesMouse` 仅对 `MOUSE` 类型绑定为真），而 `bbb` 的调试修饰键固定为键盘 `F3` 路径。
- **游戏内设置 UI 与 UI 驱动的运行时选项修改器为 `not-needed`。** 配置通过命令行/URL 查询参数在启动时提供（例如 `--advanced-item-tooltips`），并配合 Vanilla `options.txt`（及浏览器 `localStorage["bbb.options.txt"]`）加载/迁移与快捷键回写（`F11`、`Ctrl+B`、`F3+H`、`F3+P`）；`bbb` 不提供游戏内配置界面。此裁决同时覆盖 `Minecraft.updateMaxMipLevel`（Vanilla 26.1 唯一调用方为 `VideoSettingsScreen`，`bbb` 使用 Vanilla 默认 mip 级别 4）。重开条件：若 CLI 或受支持的运行时接口在启动后变更 mipmap 级别（并原子重建完整资源与 GPU 代次）时重开。
- **世界访问/删除、资源包复制与内置服务端区块存储 `SystemToast` 辅助函数为 `not-needed`。** `SystemToast.onWorldAccessFailure`、`onWorldDeleteFailure` 与 `onPackCopyFailure` 仅由标题/选世界界面或游戏内选包界面调用，`onChunkLoadFailure`/`onChunkSaveFailure` 仅由 `IntegratedServer` 调用（`SystemToast.java:132-188`、`IntegratedServer.java:352-363`）。重开条件：前一组随受支持的世界/资源包选择界面重开，后一组随内置服务端重开。
- **Vanilla OpenGL GPU 警告列表重载监听器为 `not-needed`。** `GpuWarnlistManager.Preparations.apply` 仅在 `GpuDevice.getBackendName()` 严格等于 `OpenGL` 时返回警告，用于驱动 `Options`/`VideoSettingsScreen` 的增强透明分支及 `Minecraft.fillSystemReport` 可选输出。`bbb` 使用 `wgpu` 的 Vulkan/Metal/DX12/WebGPU 后端且无游戏内设置界面，Vanilla 谓词恒为空。重开条件：仅当 OpenGL 后端与增强透明警告进入产品范围时重开。

## 资源包、屏幕与静态注册表

- **服务器列表编辑界面所有者为 `not-needed`。** `ServerList.size`/`swap`/`replace`/`remove`/`unhide` 仅服务于 `ServerSelectionList` 与 `JoinMultiplayerScreen`（`replace` 在 26.1 中甚至无任何调用方），而 CLI / Web `QuickPlay.joinMultiplayerWorld` 仅使用 `load`/`get(String)`/`add`/`save`，已由 `bbb_client::server_list` 拥有（桌面端持久化至 `servers.dat`，Web 端至 `localStorage["bbb.servers.dat"]`）。重开条件：随服务器选择界面重开。
- **标题栏标语重载监听器为 `not-needed`。** Vanilla `SplashManager.prepare` 读取 `minecraft:texts/splashes.txt` 仅服务于 `TitleScreen.init`（`Minecraft.getSplashManager().getSplash()`），CLI 直连客户端没有标题界面。重开条件：当标题界面纳入产品范围时重开。
- **内置索引资源包 ZIP（`programmer_art`、`high_contrast`）为 `not-needed`。** `ClientPackSource` 通过资源包仓库/设置界面暴露这些包；`bbb` 接受显式启动/服务端资源包（以及 Web `?pack=`、拖拽与 `#bbb-pack-picker` `.zip` 挂载），不提供内置资源包 ID 选择界面或 CLI 参数。重开条件：当新增内置资源包 ID 选择面时重开。
- **索引中的 Realms 语言与轮播图资产为 `not-needed`。** Vanilla 共享语言管理器可见 `realms:lang/en_us`，`AddRealmPopupScreen` 枚举 `realms:textures/gui/images`，而 Realms 服务与界面位于外部多人客户端边界之外。重开条件：若 Realms 纳入产品范围时重开。
- **服务器/世界列表图标与选包界面动态纹理为 `not-needed`。** `bbb` 直接配置启动/服务端/Web 资源包而无对应选择界面，地图纹理、玩家皮肤/披风/鞘翅与同步资源包呈现由各自所有者负责。重开条件：若上述选择界面纳入范围时重开。
- **网络同步自定义方块或实体类型注册表身份为 `not-needed`。** Vanilla 26.1 `RegistryDataLoader.SYNCHRONIZED_REGISTRIES` 不包含 `BLOCK` 与 `ENTITY_TYPE`，其协议 ID 固定且资源包不可新增条目。重开条件：若目标协议同步其中任一注册表时重开。
- **自定义/数据包属性提示框定义为 `not-needed`。** Vanilla 26.1 通过 `BuiltInRegistries.registerSimple` 在 `WORLDGEN_REGISTRIES` 与 `SYNCHRONIZED_REGISTRIES` 之外注册 `minecraft:attribute`，35 项固定 Vanilla 目录已自动生成。重开条件：若属性变为网络同步注册表时重开。
- **自定义/数据包药水效果定义为 `not-needed`。** Vanilla 26.1 通过 `BuiltInRegistries.registerSimple` 在 `WORLDGEN_REGISTRIES` 与 `SYNCHRONIZED_REGISTRIES` 之外注册 `minecraft:mob_effect`，40 项固定目录已自动生成。重开条件：若状态效果变为网络同步注册表时重开。
- **自定义/数据包药水注册表定义为 `not-needed`。** Vanilla 26.1 通过 `BuiltInRegistries.registerSimple`（`Potions::bootstrap`）在 `WORLDGEN_REGISTRIES` 与 `SYNCHRONIZED_REGISTRIES` 之外注册 `minecraft:potion`。重开条件：若药水变为网络同步注册表时重开。
- **自定义/数据包村民类型与职业定义为 `not-needed`。** 两个默认注册表均位于 `BuiltInRegistries`（`VillagerType::bootstrap`、`VillagerProfession::bootstrap`）且不在 `WORLDGEN_REGISTRIES` / `SYNCHRONIZED_REGISTRIES` 中，7 项与 15 项固定目录已自动生成。重开条件：若任一注册表变为网络同步时重开。
- **自定义/数据包饰纹陶罐纹样定义为 `not-needed`。** Vanilla 26.1 通过 `BuiltInRegistries.registerSimple`（`DecoratedPotPatterns::bootstrap`）在 `WORLDGEN_REGISTRIES` 与 `SYNCHRONIZED_REGISTRIES` 之外注册 `minecraft:decorated_pot_pattern`。重开条件：若陶罐纹样变为网络同步注册表时重开。
- **自定义/数据包菜单类型定义为 `not-needed`。** Vanilla 26.1 通过 `BuiltInRegistries.registerSimple` 在 `WORLDGEN_REGISTRIES` 与 `SYNCHRONIZED_REGISTRIES` 之外注册 `minecraft:menu`，25 项固定菜单目录已自动生成。重开条件：若菜单类型变为网络同步注册表时重开。
- **自定义难度、装备槽位/槽位组、实体姿态与声音来源枚举值为 `not-needed`。** Vanilla 26.1 将 `Difficulty`（4 项）、`EquipmentSlot` / `EquipmentSlotGroup`、`Pose`（18 项）与 `SoundSource`（11 项）定义为封闭 Java 枚举并已生成固定目录，无服务端或数据包扩展面。
- **自定义实体数据序列化器与客户端游戏事件包类型为 `not-needed`。** Vanilla 26.1 在代码中静态分配 43 项 `EntityDataSerializers` 与 14 项 `ClientboundGameEventPacket.Type` 字节 ID，无网络同步注册表面。
- **自定义物品、方块或方块状态注册表定义为 `not-needed`。** Vanilla 26.1 物品协议 ID（1506 项）与方块/全局方块状态 ID（29,873 个状态）来自固定内置客户端注册而非网络同步的服务端/数据包注册表；修改内置注册顺序的模组客户端不在 Vanilla 26.1 线协议目标范围内。
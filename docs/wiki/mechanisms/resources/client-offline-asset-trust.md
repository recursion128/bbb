# 客户端 Vanilla 默认资源包与资产索引可信链

机制 ID：`resources-offline-asset-trust`

## 机制目的和边界

本机制将官方 26.1 资产索引（asset index）作为默认 `vanilla` 资源包的通用只读提供者，而不是在启动时把 4,749 个对象全部载入内存，也不是为字体、语言和音频分别建立私有回退路径。其边界覆盖元数据与索引可信校验、命名空间与根资源可见性、资源包优先级、过滤器、伴随 `.mcmeta` 元数据、资源代际原子提交以及各消费者数据流；客户端绝不下载、复制、物化或提交官方对象文件。

## Vanilla 26.1 权威实现

`GameConfig.FolderData.getExternalAssetSource` 在存在 asset-index ID 时创建 `IndexedAssetSource.createIndexFs`。`ClientPackSource.createVanillaPackSource` 依次调用 `VanillaPackResourcesBuilder.pushJarResources` 与 `pushAssetPath(externalAssetRoot)`；构建器在最终生成时会反转路径列表，因此在同一个 `vanilla` 资源包内部，外部索引路径优先于 jar 默认资源。默认包仅暴露 `minecraft` 与 `realms` 两个命名空间；`icons/**` 与 `pack.mcmeta` 则通过 `VanillaPackResources.getRootResource` 访问。

`MultiPackResourceManager` 按「vanilla → 启动/已选资源包 → 服务器资源包」从低到高的优先级建立回退管理器。单资源查询从高到低取首个可见项，遇到高层过滤器即停止；资源栈查询先从高到低收集再反转为从低到高。单资源 `.mcmeta` 取优先级不低于内容提供者的最高可见伴随元数据；资源栈元数据则与各自资源包的内容一一绑定。`LanguageManager`、`FontManager` 与 `SoundManager` 全部从同一资源管理器查询。

## 决策输入与状态所有权

输入包括 `26.1.json` 中的 asset-index ID/SHA-1/大小、索引 `30`、对象键/哈希/大小、源码或 jar 等价资产，以及本地已选与服务器下发资源包。外部本地缓存持有物理字节；`bbb_client_assets::asset_index::VanillaAssetIndex` 持有有界的「键 → 对象描述符」映射（crate 根导出只读观测类型 `VanillaAssetIndexObservation`）；`PackResourceStack` 拥有统一查询与内存预算；`ClientResourceGeneration` 拥有单次原子发布。字体图集、语言目录、声音目录与音频解码器各自只持有派生状态，不重新决定资源来源。

`ClientResourceGeneration` 按需读取并保留一份完整 `models` 树的原始立方体模型，方块与物品目录共享该集合，读取、解析与预算准入由同一加载器执行。`PackRoots` 克隆共享同一代际，资源重载时创建新代际所有者。父模型可位于自定义命名空间或路径，仍遵循统一资源栈优先级解析，派生的方块状态选择与物品展示契约各自保持独立。

## 判断与处理顺序

1. `PackRoots::discover` 从本地 26.1 版本元数据定位资产索引，不依赖 `BBB_MC_GENERATED_ASSETS_DIR` 或物化资产目录。
2. 读取版本元数据与资产索引时校验 ID、文件大小、SHA-1、对象数量、单对象与总声明字节数以及路径安全性；仅保留描述符，不在枚举时打开全部 4,749 个对象。
3. 复合 `vanilla` 资源包先查询外部索引提供者，再查询源码/jar 等价提供者；外层启动包与服务器资源包优先级更高，高层过滤器可隐藏低层索引条目。
4. 命名空间枚举仅返回 `minecraft` 与 `realms`；合法根资源查询仅暴露 `icons/**` 与 `pack.mcmeta`。绝对路径、`..`、反斜杠、`NUL` 字节及非法命名空间一律拒绝。
5. JSON、ZIP、OGG 等消费者在实际读取选中对象时才惰性校验大小与 SHA-1；单个对象缺失或损坏不影响枚举或读取其他无关对象。
6. 新资源代际的全部准备步骤成功后才一次性原子提交；任何阶段失败都保留上一代际，旧会话提供者不会泄漏至新状态。

## 资源消费者数据流

- **语言**：`PackMetadataCatalog` 通过根资源 `pack.mcmeta` 获取语言元数据；`LanguageCatalog` 按 `en_us → 选定语言` 遍历同一资源栈，保留 `replace`/过滤、双向文本（bidi）元数据与弃用键映射规则。例如 `zh_cn` 中的 `menu.multiplayer=多人游戏` 直接来自索引对象，进入 `NativeItemRuntime` 与控制诊断输出。
- **字体**：字体定义 JSON、位图 PNG 与 `unihex` ZIP 全部由 `PackResourceStack` 提供；引用与过滤展开后遵循首供优先（first-provider-wins），不存在字体专用的索引回退解析器。
- **音频**：各命名空间的 `sounds.json` 按资源栈从低到高合并，保留 `replace`、文件/事件引用、`stream`/`preload` 与衰减规则。普通文件条目仅在选中播放时校验索引 OGG 并交由有界解码器；`preload: true` 则按 Vanilla 重载/应用队列去重，由单一「最新代际优先」后台线程提前解码并按代际非阻塞安装。构建声音目录本身不急切解码全部 4,564 个音频，也不要求音频设备就绪；预加载请求、完成、失败、待处理、取消与过期丢弃均计入控制观测。
- **GUI**：由同一 `PackRoots`/`PackResourceStack` 提供资源。客户端 `hud_assets.rs` 安装 GUI 精灵图集与独立 GUI 纹理；渲染器 `hud/gui_atlas.rs::GuiSprites` 与 `renderer/pack_resources.rs` 持有 GPU 代际。小精灵与动画帧共用图集，容器背景等 `GUI_TEXTURES` 采用按路径缓存的独立纹理。图集化不改变层级、裁剪与命中几何，也不要求界面压缩为单次绘制。
- **全景图**：`bbb-client-assets::PanoramaCatalog` 是六面贴图与覆盖层的唯一不可变 CPU 所有者；它按 Vanilla `_1, _3, _5, _4, _0, _2` 层序从同一资源栈查询，保留资源包优先级、过滤、惰性完整性校验、PNG 预算与等尺寸约束。客户端只在资源事务中编排 GPU 提交，并以实时 tick 推进屏幕局部旋转；渲染器独占立方体贴图与覆盖层 GPU 代际，按「立方体 → 覆盖层 → 模糊 → 菜单背景 → 控件」顺序提交。重载失败回滚至上一完整代际。Vanilla OpenGL `CubeMapTexture` 在上传前会纵向翻转各面，而 WGPU 的复制与采样原点本身即保持 PNG 从上到下的显示方向，因此 BBB 不做额外翻转，并由非对称面像素测试锁定最终画面。启动缺少全景图目录时直接回退菜单背景，不设第二解析器。
- **窗口图标**：`bbb_client_assets::WindowIconCatalog` 是唯一 CPU 所有者（标准家族完整解析 5 张 PNG，macOS 家族有界读取并校验 ICNS）；`bbb_client::window_icon` 负责平台选择与启动期一次性应用——Windows 通过 `WindowExtWindows` 设置 16 px 窗口图标与 256 px 任务栏图标，X11 一次向 `_NET_WM_ICON` 写入 16/32/48/128/256 五段 ARGB，macOS 直接应用原始 ICNS，Wayland 与未支持后端跳过且不读取对象；任一失败回退平台默认图标，不参与运行期资源重载。
- **Web（`wasm32`）虚拟资源存储与本地包挂载**：浏览器端由 `bbb_client_assets::virtual_store` 承载 `bbb-web-host` 下发的 `/assets/core-26.1.zip`、`/assets/asset-index.json`（缓存于 Cache Storage `bbb-core-assets-v1`）与按需拉取的语言/音频对象，经 `PackRoots::from_web_bundle` 接入同一 `PackResourceStack`；运行期通过 `?pack=`、画布拖放或 `#bbb-pack-picker`（`crates/bbb-client/src/web/packs.rs`）挂载的本地 `.zip` 资源包同样触发原子代际重载。

## 索引对象消费矩阵

资产索引 `30` 中的 4,749 个对象全部可由索引提供者枚举；「文件存在」「提供者可达」「消费者已实现」与「当前产品需要」四者分别判定，互不替代：

| 对象家族 | 对象数 | Vanilla 所有者 | 提供者可达 | BBB 消费者与当前界面 | 范围与精确重开条件 |
| --- | ---: | --- | --- | --- | --- |
| 字体定义与 `unihex` | 5 | `FontManager`、`FontSet`、`UnihexProvider` | 是（`minecraft`） | 共享字体图集；供 HUD、屏幕与世界文本使用 | 当前范围消费 |
| Minecraft 语言 JSON | 141 | `LanguageManager`、`ClientLanguage` | 是（按选定语言惰性读取） | `LanguageCatalog` / `NativeItemRuntime`；供 CLI 语言切换使用 | 当前范围消费 |
| `sounds.json` 与 OGG | 4,565 | `SoundManager`、`SoundEngine` | 是（定义读取、OGG 惰性打开） | `SoundCatalog` / 音频运行时；供外部会话播放 | 当前范围消费 |
| 根 `pack.mcmeta` | 1 | `VanillaPackResources`、元数据节 | 是（根资源） | 资源包与语言元数据；供重载读取 | 当前范围消费 |
| 窗口图标 PNG / ICNS | 12 | `Minecraft` → `Window.setIcon` → `IconSet` | 是（根资源） | `WindowIconCatalog` → 启动期跨平台窗口图标应用 | 当前范围消费 |
| 标题全景图 | 7 | `GameRenderer`、`GuiRenderer`、`CubeMap`、`CubeMapTexture`、`PanoramaRenderer` | 是（`minecraft`） | `PanoramaCatalog` → 断线状态页立方体/覆盖层 GPU 渲染 | 当前范围消费 |
| 内置资源包 ZIP | 2 | `ClientPackSource`、资源包仓库 | 是（`minecraft:resourcepacks/**`） | 当前无游戏内资源包选择或高对比度/程序员艺术切换界面 | 范围裁决见客户端账本；引入内置包选择入口时重开 |
| Realms 语言与轮播图 | 16 | `LanguageManager`、`AddRealmPopupScreen` | 是（`realms`） | Realms 服务与界面明确不在外部多人客户端范围内 | 范围裁决见客户端账本；Realms 进入范围时重开 |

按家族合计 4,749 个对象，其中 4,731 个归属当前产品范围消费者，18 个对应账本已裁决为范围外的内置资源包与 Realms 界面。该统计仅描述提供者数据面，范围外对象同样保持提供者可达但不会被急切读取。

## 生命周期和 fallback

动态同步注册表的呈现范围限定于 Vanilla 26.1 实际列入 `RegistryDataLoader.SYNCHRONIZED_REGISTRIES` 且存在当前客户端消费者的注册表。`BuiltInRegistries.BLOCK` 与 `ENTITY_TYPE` 属于客户端内建身份，不经网络自定义。任何新增消费者都必须复用现有资源代际、`RegistryAccess` 解析器、有界待提交/就绪/失败状态与精确回退，不得另建第二套注册表、模式或资源包栈。

元数据或索引结构错误一律 fail closed；合法的已选包或服务器包可覆盖索引中的 Vanilla 资源。单个对象缺失、大小不符或 SHA-1 不符时，仅在实际消费者读取该对象时按 Vanilla 缺失/无效语义处理，绝不触发第二解析器。新代际加载失败时保留上一代际及诊断；成功后预算、目录、图集与音频所有者同步换代。

每次服务器资源包重载均在产生请求的模拟步之后、下一次模拟步之前，由主循环完整执行 CPU 准备、GPU/音频事务提交与代际确认入队；后续断开连接或转服事件不得越过该屏障。常规重载失败时，渲染器、音频、模拟层与活动资源根全部保留上一完整代际，仅将新包标记为 `FAILED_RELOAD`，随后恰好执行一次恢复重载。恢复成功后，世界层 UI 事实驱动共享 `PACK_LOAD_FAILURE` 系统提示（toast）；若恢复重载再次失败，则冻结相同的目标栈以防每 tick 死循环热重试，并通过一次幂等的传输关闭、连接代际重置、音频全部停止与有界断线页终止当前外部会话。下载/物化使用「UUID + 推送序号」、重载确认使用代际号分别拒绝过期结果。

### 重载监听器逐项投影

下表按 Vanilla `Minecraft` 的注册顺序逐一映射 20 个资源重载监听器。当前产品范围内的监听器共用同一代际准备、提交与回滚契约，范围外项保留明确重开条件：

| 序 | Vanilla 监听器与职责 | BBB 投影实现 | 提交与回滚所有者 | 状态 | 实现模块或重开条件 |
| ---: | --- | --- | --- | --- | --- |
| 1 | `LanguageManager`：读取元数据后构造选定语言 | `NativeItemRuntime::load_with_locale` | 物品运行时 + 渲染器双向标志同代提交 | 已有 | `bbb_client::resource_reload` |
| 2 | `TextureManager`：重载所有 `ReloadableTexture`，缺失回退缺图纹理 | 地形/HUD/实体/天空/物品/粒子纹理加载器 | 渲染器纹理检查点 | 已有 | `bbb_client_render::resource_reload` |
| 3 | `ShaderManager`：枚举核心/包含/后处理着色器，预加载静态管线 | `ShaderSourceCatalog` 与后处理共用预处理；`CorePipelines` 静态管线同代原子提交 | 渲染器管线代际（`CorePipelines`） | 已有 | `bbb_client_render::render_core::core_shaders`、`resource_reload` |
| 4 | `SoundManager`：按栈合并 `sounds.json` 并重载音频引擎 | `bbb_client::audio_runtime::NativeAudioRuntime::prepare_reload` | 音频提交位于渲染器屏障末端 | 已有 | `bbb_client::audio_runtime` |
| 5 | `SplashManager`：读取 `texts/splashes.txt`，仅用于 `TitleScreen` | 无标题屏标语界面 | 无消费者 | 非目标 | 见[客户端账本](../../../unsupported-client-features.md)；标题屏进入范围时重开 |
| 6 | `AtlasManager`：准备、拼接并上传已知精灵图集 | 地形/HUD/实体/天空/物品/粒子图集加载器 | 渲染器图集检查点 | 已有 | `bbb_client::resource_reload` |
| 7 | `FontManager`：解析定义/提供者/引用后替换 `FontSet` | 统一资源栈字体目录与图集 | 渲染器字体与 HUD 缓存检查点 | 已有 | `bbb_client_render::item_model::font` |
| 8 | `GrassColorReloadListener`：读取草地色图 | `TerrainColorMaps.grass` | 地形代际 | 已有 | `bbb_client_assets::colors` |
| 9 | `FoliageColorReloadListener`：读取树叶色图 | `TerrainColorMaps.foliage` | 地形代际 | 已有 | `bbb_client_assets::colors` |
| 10 | `DryFoliageColorReloadListener`：读取干枯树叶色图 | `TerrainColorMaps.dry_foliage` | 地形代际 | 已有 | `bbb_client_assets::colors` |
| 11 | `ModelManager`：发现并烘焙方块与物品模型 | `NativeItemRuntime` + 地形模型目录 | 物品/地形代际 | 已有 | `bbb_client_render::item_model::item_runtime` |
| 12 | `EquipmentAssetManager`：解析 `equipment/*.json` | `EquipmentAssetCatalog` | 世界默认值与实体纹理同代提交 | 已有 | `bbb_client_assets::equipment_assets` |
| 13 | `EntityRenderDispatcher`：重建实体渲染器映射 | 实体模型纹理与网格代际 | 渲染器实体检查点 | 已有 | `bbb_client::entity_assets` |
| 14 | `BlockEntityRenderDispatcher`：重建方块实体渲染器映射 | 共享方块实体模型与纹理呈现 | 渲染器实体/HUD 检查点 | 已有 | `bbb_client_render::resource_reload` |
| 15 | `ParticleResources`：粒子定义 → 精灵集合 → 粒子图集 | `NativeParticleResources::load` | 模拟层与渲染器粒子同代提交 | 已有 | `bbb_client::particle_runtime` |
| 16 | `WaypointStyleManager`：解析 `waypoint_style/*.json` | `WaypointStyleCatalog` | HUD 路径点检查点 | 已有 | `bbb_client_assets::waypoint_styles` |
| 17 | `LevelRenderer`：重建轮廓与天空渲染器 | 后处理/天空资源 + 全部区块重载 | 渲染器事务 + 地形重载请求 | 已有 | `bbb_client::resource_reload` |
| 18 | `CloudRenderer`：解码云层格网并标记重建 | `try_load_sky_textures` 云层数据 | 渲染器天空/云层检查点 | 已有 | `bbb_client::sky_assets` |
| 19 | `GpuWarnlistManager`：仅匹配 Vanilla OpenGL 正则警告列表 | 发货后端为 `wgpu`（Vulkan/Metal/DX12/WebGPU），无游戏内选项警告 UI | 无消费者 | 非目标 | 见[客户端账本](../../../unsupported-client-features.md)；支持 OpenGL/选项警告时重开 |
| 20 | `PeriodicNotificationManager`：按地区 ISO3 调度周期系统提示 | 宿主地区 → ISO3、GCD 周期表与共享系统提示 | 客户端 HUD 运行时；GPU/音频屏障成功后替换，失败保留旧计划 | 已有 | `bbb_client::runtime::periodic_notifications` |

着色器 CPU 准备阶段只有一个 GLSL 源码所有者：`ShaderSourceCatalog` 枚举已选资源栈中的 `.vsh`/`.fsh`，通过同一栈解析 `.glsl` 引入，限制最多 512 个阶段/包含资源、32 层引入深度与代际驻留字节数；`PostEffectCatalog` 只引用该不可变目录，不另建第二套引入解析器。`PeriodicNotificationCatalog` 读取最高优先级 `minecraft:regional_compliancies.json`，限制最多 256 个国家分组与 1,024 条通知，保留缺失或损坏时回退空计划的语义，而安全预算超限仍拒绝整代加载。两者均携带资源代际号。

`PostEffectCatalog::renderer_projection` 是唯一的跨 crate 着色器代际投影，其中的核心着色器源码与后处理图必须属于同一代际；CPU 目录仅在 `bbb-client-assets` 中读取资源包，不可变值经 `bbb-render-types` 交给渲染器，`bbb-client-render` 不直接接触资源包。启动与服务器资源包重载均只调用一次 `transactional_shader_resource_reload`，固定先提交核心管线再提交后处理图，并将后续图集、HUD、实体、天空与音频提交纳入同一回滚屏障。

静态核心着色器管线（含地形、天空、HUD 及 BBB 远景 `lod_terrain` 管线）统一归 `CorePipelines` 拥有；`Renderer` 另独立持有 `EntityPosePipelines`（内置着色器纯立方体 GPU 蒙皮管线）以及透明合成与公告牌物品回退管线。启动时由内置源码构造，重载时由资源包的 `SelectedCoreShaders` 经同一构造函数编译（各管线族的顶点/片元模块统一走渲染器私有 `compile_selected_shader_stages`）；选择未变时复用现值，否则整值原子替换，任一阶段失败立即回滚。独立 `reload_core_shaders`/`reload_post_effect_graphs` 仅限 crate 内部与测试可见，防止生产路径分裂提交。六个 Vanilla 内置后处理链（相机实体三分支、实体轮廓、改善透明度与 GUI 模糊）由渲染器私有 `VanillaPostEffect` 唯一标识，缺失时按 Vanilla 跳过该效果，编译失败则在既有事务内回滚上一代际。

区域周期通知目录按宿主 ISO3 地区码构造最大公约数固定周期表，重载时保留已逝时间、重启选中项的初始延迟，并仅在事务成功后替换 HUD 调度表；单帧追赶有界，跳过的周期仍正确推进已逝时间。

## BBB 实现映射

- `bbb_client_assets::asset_index`：`VanillaAssetIndex` 可信描述符表、只读观测 `VanillaAssetIndexObservation` 与惰性完整性校验。
- `bbb_client_assets::resources`：复合 `vanilla` 包、命名空间与根资源查询、单资源/栈查询、过滤器、伴随元数据与内存预算。
- `bbb_client_assets::roots`：`PackRoots` 发现与资源代际管理。
- `bbb-client-assets` / `bbb-client-render` / `bbb-client` / `bbb-client-audio`：分别承载语言与声音目录、字体图集、着色器/后处理事务提交以及有界 OGG 解码。
- `ClientResourceDiagnostics`：向控制接口与端到端门禁暴露索引 ID、对象计数、当前选定语言及真实翻译探针结果。

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 版本与索引元数据校验 | 启动器与索引元数据 | `bbb_client_assets::asset_index::VanillaAssetIndex::discover` | `aligned` | 元数据、ID、哈希与大小夹具测试 |
| 复合 `vanilla` 包内顺序 | 外部索引 → jar 默认资源 | `PackResourceStack` 默认包条目 | `aligned` | 源码冲突覆盖、回退与列表并集测试 |
| 资源包、过滤与元数据顺序 | 回退资源管理器 | 已选资源包栈与代际提交 | `aligned` | 单资源、资源栈、过滤器与 `.mcmeta` 测试 |
| 命名空间与根资源枚举 | `minecraft`、`realms` 与根资源 | 索引资源提供者 | `aligned` | 枚举列表、根资源与路径安全拒绝测试 |
| 惰性完整性与预算统计 | 索引对象流打开校验 | 描述符表 + 消费者按需读取校验 | `aligned` | 缺失、大小/SHA-1 不符、惰性读取与预算计费测试 |
| 语言、字体与音频消费 | 共享资源管理器 | 共享 `PackResourceStack` | `aligned` | `zh_cn` 翻译、CJK 字形与 OGG 播放验收 |
| 窗口图标消费 | `IconSet` + `Window.setIcon` | `WindowIconCatalog` → `bbb_client::window_icon` | `aligned` | 根路径/顺序/预算，以及 Windows/X11/macOS/Wayland 平台分支测试 |

## 结构性不变量

- 严禁将官方对象存储、资产索引副本或字体/声音 ZIP 提交入库或物化到仓库目录。
- 复合 `vanilla` 包内「外部索引优先于源码/jar」与外层资源包优先级不得颠倒或混淆。
- 索引枚举不得打开或哈希全部对象；完整性校验必须推迟到实际读取。
- 字体、语言与音频不得各自恢复私有的索引回退解析器。
- 窗口图标不得绕过 `WindowIconCatalog` 走专用路径；已选包、启动包与服务器包不参与 Vanilla 根图标查询；X11 不得把五张标准图降级为单张 winit 图标。
- 同一代际内的消费者不得混用两次重载的资源，准备或提交失败必须完整保留上一代际。

## 聚焦测试（focused tests）和真实证据

聚焦测试覆盖版本与索引元数据校验、非法键与路径穿越拦截、缺失/大小/SHA-1 不符处理、命名空间枚举、根资源、单资源与资源栈查询、源码/索引/已选/服务器包优先级、过滤器、`.mcmeta`、无关对象惰性跳过、内存预算、20 个重载监听器与着色器事务回滚。资源与平台验证基于干净的托管资产目录校验 `zh_cn` 翻译、有界 OGG 打开、CJK 与补充平面字形度量/UV、窗口图标 PNG 尺寸与 ICNS 完整性，以及各平台窗口后端行为。

## 当前差异

内置可选资源包 ZIP（`programmer_art`、`high_contrast`）、Realms 语言与轮播图以及 OpenGL GPU 警告列表（`GpuWarnlistManager`）均不在当前产品范围内，其裁决与重开条件统一登记于[客户端账本](../../../unsupported-client-features.md)的 `# Not needed` 段落；当前账本无未关闭的资源资产 `# Todo` 缺口。

## 账本归属

范围外资产家族与产品裁决统一登记于[客户端账本](../../../unsupported-client-features.md)。

## 精确重开条件

当目标协议版本升级、资产索引/提供者/资源包优先级规则变动、出现绕过共享 `PackResourceStack` 的消费者，或上表中列出的范围外界面进入产品范围时，须重新核验对应条目。

## 相关页面和源码索引

- [字体与文本](../ui/client-font-and-text.md)
- Vanilla：`GameConfig.FolderData`、`IndexedAssetSource`、`ClientPackSource`、`VanillaPackResourcesBuilder`、`MultiPackResourceManager`、`LanguageManager`、`FontManager`、`SoundManager`、`IconSet`
- BBB：`bbb-client-assets`、`bbb-client-render`、`bbb-client`、`bbb-client-audio`

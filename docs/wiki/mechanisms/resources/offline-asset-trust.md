# Vanilla 默认资源包与 asset-index/object-store 可信链

机制 ID：`resources-offline-asset-trust`

## 机制目的和边界

该机制把官方 26.1 asset index 作为默认 `vanilla` pack 的通用只读 provider，而不是
把 4749 个 object 启动时载入内存，也不是给 font、language、sound 分别建立 fallback。
它覆盖 metadata/index 信任链、namespace/root 可见性、pack priority、filter、companion
metadata、generation 与 consumer 数据流；不下载、复制、materialize 或提交官方对象。

## Vanilla 26.1 权威实现

`GameConfig.FolderData.getExternalAssetSource` 在存在 asset-index id 时创建
`IndexedAssetSource.createIndexFs`。`ClientPackSource.createVanillaPackSource` 依次调用
`VanillaPackResourcesBuilder.pushJarResources`、`pushAssetPath(externalAssetRoot)`；builder
构建时反转 path 列表，所以同一个 `vanilla` pack 内 external indexed path 先于
jar/default resources。它只暴露 `minecraft`、`realms` namespace；`icons/**` 与
`pack.mcmeta` 由 `VanillaPackResources.getRootResource` 访问。

`MultiPackResourceManager` 按 vanilla→startup/selected→server 的低到高 pack priority
建立 fallback manager。single resource 从高到低取第一个可见项，遇到高层 filter 即停止；
resource stack 先按高到低收集再反转为低到高。single `.mcmeta` 取不低于内容 provider 的
最高可见 companion；stack metadata 与各自 pack 的内容绑定。`LanguageManager`、
`FontManager`、`SoundManager` 都从同一 resource manager 查询。

## 决策输入与状态所有权

输入是 `26.1.json` 中 asset-index id/SHA-1/size、index `30`、object key/hash/size、
source/jar-equivalent assets 以及 selected/server packs。外部缓存拥有物理字节；
`bbb-pack::VanillaAssetIndex` 拥有有界 key→object descriptor，`PackResourceStack` 拥有
统一查询与预算，`ClientResourceGeneration` 拥有一次原子发布。font atlas、language
catalog 和 sound catalog/audio decoder 只拥有各自派生状态，不重新决定资源来源。

## 判断与处理顺序

1. `PackRoots::discover` 从本地 26.1 metadata 定位 index，不依赖
   `BBB_MC_GENERATED_ASSETS_DIR` 或 materialized assets 目录。
2. 读取 metadata/index 时校验 id、size、SHA-1、object 数量、单对象/总声明字节和安全
   key；只保留 descriptor，不打开 4749 个 object。
3. composite `vanilla` pack 先查 indexed provider，再查 source/jar-equivalent provider；
   外层 startup/server pack 保持更高优先级，filter 可隐藏 indexed 低层。
4. namespace 枚举只返回 `minecraft`/`realms`；合法 root 查询只暴露 `icons/**` 与
   `pack.mcmeta`。绝对路径、`..`、反斜杠、NUL 和非法 namespace 被拒绝。
5. JSON/ZIP/OGG 等 consumer 实际读取选中 object 时才校验 size/SHA-1；一个缺失或损坏
   object 不妨碍枚举或读取无关 object。
6. 新 resource generation 全部 prepare 成功后一次提交；失败保留上一 generation，旧
   session/provider 不泄漏到新状态。

## language、font 与 audio 数据流

- language：`PackMetadataCatalog` 通过 root `pack.mcmeta` 取得 language metadata；
  `LanguageCatalog` 按 `en_us`→selected locale 遍历同一 stack。pack 的 replace/filter、
  bidi metadata 与 deprecated mapping 保持原语义。`zh_cn` 的 `menu.multiplayer=多人游戏`
  来自 index object，并进入 `NativeItemRuntime` 与 control resource diagnostics。
- font：definition JSON、bitmap 和 unihex ZIP 都由 `PackResourceStack` 提供；reference/filter
  展开之后仍是 first-provider-wins。不存在 index-specific font resolver。
- audio：各 namespace 的 `sounds.json` 按 resource-stack 低到高合并，`replace`、file/event
  reference、stream/preload、attenuation 保持既有语义。选中的 file entry 才验证 indexed
  OGG 并交给 bounded decoder；catalog 构建不 eager decode 4564 个声音，也不要求音频设备。

## Indexed object 消费矩阵

index 30 的 4749 个 object 全部可由 provider 枚举；“物理存在”“provider 可达”“consumer
已实现”“当前产品 surface 需要”分别判定，不能互相替代。

| 家族 | object 数 | Vanilla owner | provider 可达 | BBB consumer / 当前 surface | 状态与精确重开条件 |
| --- | ---: | --- | --- | --- | --- |
| font definition/unihex | 5 | `FontManager`, `FontSet`, `UnihexProvider` | 是，`minecraft` | 共享 font atlas；HUD/screen/world text 需要 | `equivalent` |
| Minecraft locale JSON | 141 | `LanguageManager`, `ClientLanguage` | 是，按 selected locale 读取 | `LanguageCatalog`/`NativeItemRuntime`；CLI locale 需要 | `equivalent` |
| `sounds.json` + OGG | 4565 | `SoundManager`, `SoundEngine` | 是，definition 读取、OGG lazy open | `SoundCatalog`/native audio；外部 session 需要 | `equivalent` |
| root `pack.mcmeta` | 1 | `VanillaPackResources`, metadata sections | 是，root resource | pack/language metadata；reload 需要 | `equivalent` |
| window icons PNG/ICNS | 12 | `Minecraft`→`Window.setIcon`→`IconSet` | 是，root resource | native window 存在，但尚未设置 icon | `partial`；[ledger](../../../unsupported-features.md) |
| title panorama | 7 | `GuiRenderer.CubeMap`, `Panorama` | 是，`minecraft` | CLI 直连客户端无 title/panorama screen | `not-needed`；title/menu panorama 进入产品范围时重开 |
| built-in pack ZIP | 2 | `ClientPackSource`, pack repository/options | 是，`minecraft:resourcepacks/**` | 无 pack-selection/high-contrast/programmer-art id surface | `not-needed`；增加内置 pack id 选择或对应 UI/CLI 时重开 |
| Realms language/carousel | 16 | `LanguageManager`, `AddRealmPopupScreen` | 是，`realms` | Realms service/screen 明确不在外部多人客户端范围 | `not-needed`；Realms service/screen 进入范围时重开 |

按 object 计数为：`equivalent=4712`、`partial=12`、`not-needed=25`、
`deferred-external=0`，合计 4749。not-needed 判断只关闭对应 presentation consumer；这些
字节仍保持 provider 可达，不会因此被 eager 读取。

## 生命周期和 fallback

metadata/index 结构错误 fail closed；selected/server pack 的合法资源仍可覆盖 indexed
vanilla。单个缺失、size 不符或 SHA-1 不符 object 在其 consumer 打开时按该 consumer 的
Vanilla missing/invalid 语义处理，不会触发第二 resolver。新 generation 加载失败时，运行时
保留上一代及其 diagnostics；成功后预算、catalog、atlas 与 audio owner 一起换代。

### Reload listener 逐 owner 投影

下表顺序直接对应 `Minecraft` 的注册顺序。`existing` 只表示当前已有候选 consumer evidence，
仍须随本工程逐项完成 prepare/apply/rollback 验证；它不是 parity matrix 的关闭状态。

<!-- reload-listener-projections:start -->
| 序 | Inventory owner | Vanilla listener / 查询 | BBB projection | apply / rollback owner | 状态 | Evidence / reopen |
| ---: | --- | --- | --- | --- | --- | --- |
| 1 | `registration:reload-listener:net.minecraft.client.Minecraft#registerReloadListener:0` | `LanguageManager`：metadata 后构造 selected language | `NativeItemRuntime::load_with_locale` | item runtime + renderer bidi 同代提交 | `existing` | `crates/bbb-native/src/resource_reload.rs` |
| 2 | `registration:reload-listener:net.minecraft.client.Minecraft#registerReloadListener:1` | `TextureManager`：重读所有 `ReloadableTexture`，缺失用 missing texture | terrain/HUD/entity/sky/item/particle texture loaders | renderer texture owner checkpoint | `existing` | `crates/bbb-renderer/src/resource_reload.rs` |
| 3 | `registration:reload-listener:net.minecraft.client.Minecraft#registerReloadListener:2` | `ShaderManager`：枚举 core/include/post-effect，preload 静态 pipeline | `ShaderSourceCatalog` 与 post effect 共享预处理；63 个 current pipeline（含 portal、outline、beam、lightning、line、text、cloud、lightmap family）驱动同一原子 generation，其余 static pipeline 仍待迁移 | renderer pipeline generation | `active` | [ledger](../../../unsupported-features.md) |
| 4 | `registration:reload-listener:net.minecraft.client.Minecraft#registerReloadListener:3` | `SoundManager`：按 stack 合并 `sounds.json` 并 reload engine | `NativeAudioRuntime::prepare_reload` | audio commit 位于 renderer barrier 末端 | `existing` | `crates/bbb-native/src/audio_runtime.rs` |
| 5 | `registration:reload-listener:net.minecraft.client.Minecraft#registerReloadListener:4` | `SplashManager`：读取 `texts/splashes.txt`，仅 `TitleScreen` 取值 | 无 title splash surface | 无 consumer | `not-needed` | [ledger](../../../unsupported-features.md)；title screen 进入范围时重开 |
| 6 | `registration:reload-listener:net.minecraft.client.Minecraft#registerReloadListener:5` | `AtlasManager`：prepare/stitch/upload 15 个 known atlas | terrain/item/particle/HUD/entity atlas loaders | renderer atlas checkpoint | `existing` | `crates/bbb-native/src/resource_reload.rs` |
| 7 | `registration:reload-listener:net.minecraft.client.Minecraft#registerReloadListener:6` | `FontManager`：definition/provider/reference prepare 后替换 `FontSet` | unified stack font catalog + atlas | renderer font/HUD cache checkpoint | `existing` | `crates/bbb-item-model/src/font.rs` |
| 8 | `registration:reload-listener:net.minecraft.client.Minecraft#registerReloadListener:7` | `GrassColorReloadListener`：读取 grass colormap | `TerrainColorMaps.grass` | terrain generation | `existing` | `crates/bbb-pack/src/colors.rs` |
| 9 | `registration:reload-listener:net.minecraft.client.Minecraft#registerReloadListener:8` | `FoliageColorReloadListener`：读取 foliage colormap | `TerrainColorMaps.foliage` | terrain generation | `existing` | `crates/bbb-pack/src/colors.rs` |
| 10 | `registration:reload-listener:net.minecraft.client.Minecraft#registerReloadListener:9` | `DryFoliageColorReloadListener`：读取 dry-foliage colormap | `TerrainColorMaps.dry_foliage` | terrain generation | `existing` | `crates/bbb-pack/src/colors.rs` |
| 11 | `registration:reload-listener:net.minecraft.client.Minecraft#registerReloadListener:10` | `ModelManager`：discover/bake block/item models | `NativeItemRuntime` + terrain model catalog | item/terrain generation | `existing` | `crates/bbb-item-model/src/item_runtime.rs` |
| 12 | `registration:reload-listener:net.minecraft.client.Minecraft#registerReloadListener:11` | `EquipmentAssetManager`：解析 `equipment/*.json` | `EquipmentAssetCatalog` | world defaults + entity textures 同代提交 | `existing` | `crates/bbb-pack/src/equipment_assets.rs` |
| 13 | `registration:reload-listener:net.minecraft.client.Minecraft#registerReloadListener:12` | `EntityRenderDispatcher`：重建 entity/avatar/mannequin renderer map | entity model texture/mesh generation | renderer entity owner checkpoint | `existing` | `crates/bbb-native/src/entity_assets.rs` |
| 14 | `registration:reload-listener:net.minecraft.client.Minecraft#registerReloadListener:13` | `BlockEntityRenderDispatcher`：重建 block-entity renderer map | shared block-entity model/texture presentation | renderer entity/HUD owner checkpoint | `existing` | `crates/bbb-renderer/src/resource_reload.rs` |
| 15 | `registration:reload-listener:net.minecraft.client.Minecraft#registerReloadListener:14` | `ParticleResources`：definition→sprite set→atlas | `NativeParticleResources::load` | simulation + renderer particle owner 同代提交 | `existing` | `crates/bbb-native/src/particle_runtime.rs` |
| 16 | `registration:reload-listener:net.minecraft.client.Minecraft#registerReloadListener:15` | `WaypointStyleManager`：解析 `waypoint_style/*.json` | `WaypointStyleCatalog` | HUD waypoint owner checkpoint | `existing` | `crates/bbb-pack/src/waypoint_styles.rs` |
| 17 | `registration:reload-listener:net.minecraft.client.Minecraft#registerReloadListener:16` | `LevelRenderer`：重建 outline/sky renderer | post/sky resources + reload all chunks | renderer transaction + terrain reload request | `existing` | `crates/bbb-native/src/resource_reload.rs` |
| 18 | `registration:reload-listener:net.minecraft.client.Minecraft#registerReloadListener:17` | `CloudRenderer`：decode cloud cells，apply 标记 mesh rebuild | `try_load_sky_textures` cloud data | renderer sky/cloud checkpoint | `existing` | `crates/bbb-native/src/sky_assets.rs` |
| 19 | `registration:reload-listener:net.minecraft.client.Minecraft#registerReloadListener:18` | `GpuWarnlistManager`：只在 Vanilla OpenGL backend 匹配 regex | BBB shipping backend 非 OpenGL，且无 improved-transparency options UI | 无 current consumer | `not-needed` | [ledger](../../../unsupported-features.md)；支持 OpenGL/options warning 时重开 |
| 20 | `registration:reload-listener:net.minecraft.client.Minecraft#registerReloadListener:19` | `PeriodicNotificationManager`：host ISO3 选择、delay/period system toast | host locale→ISO3、GCD schedule、reload elapsed reset 与 shared system toast | native HUD runtime；GPU/audio barrier 成功后替换，失败保留旧 schedule | `existing` | `crates/bbb-native/src/runtime/periodic_notifications.rs` |
<!-- reload-listener-projections:end -->

Phase B 的 CPU prepare 只有一个 GLSL owner：`ShaderSourceCatalog` 枚举 selected stack 中的
`.vsh`/`.fsh`，通过同一 stack 解析 `.glsl` import，并限制 512 个 stage/include 资源、32 层
import depth 和 generation retained bytes；`PostEffectCatalog` 只引用该 immutable catalog，
不再拥有第二套 import resolver。`PeriodicNotificationCatalog` 读取最高优先级
`minecraft:regional_compliancies.json`，限制 256 个 country group、1024 条 notification，
保留 Vanilla 的缺失/损坏→空准备语义；安全预算拒绝仍使整个 generation 失败。两者均携带
resource generation。CPU catalog 只在 `bbb-pack` 解释 pack；不可变 shader/atlas 值经
`bbb-render-types` 投影，`bbb-renderer` 的生产依赖不读取 pack。Phase C 已把
`core/terrain` 三个当前 pipeline、24 个从单一 `core/entity` 派生的 entity/beam/item/resident
variant、4 dynamic/resident `core/item`、3 particle/weather、2 position-color、5 个 `core/glint`
entity/armor/item/HUD variant、5 个 end-portal world/resident/HUD owner 与 post graph 放进同一 renderer transaction；构建任一 pipeline 或后续 atlas/HUD/entity/sky/audio owner 失败会
恢复旧 pipeline generation。
core pipeline 的 GPU 准备集由 renderer 内部唯一私有
`CorePipelineGeneration` 拥有；family container 只是同质 handle 值组，不再各自
声明 generation。成功发布与失败回滚共用同一份
`swap_with_renderer` 字段映射，generation 在全部 handle 之后一起交换；公共
reload API 与 pack → render-types → renderer 依赖方向不变。glint 的 scale、view-offset-Z、
resident instance/decal UV 与 HUD clip/mask 只在 renderer 的共享 adapter 中形成窄 GPU
projection；各 family consumer 不再分别解析 selected shader，也不能独立发布 generation。
renderer 的 87 个长期 pipeline owner 由 source guard 机械分成：63 个已在统一 core
generation、14 个仍映射到 Vanilla static pipeline/core shader、10 个无一对一 static owner。
后者只有 8 个已由 `PostEffectGraphRuntime` 拥有 selected path 的 default/内部 copy，以及
Vanilla registry 中不存在的 BBB colored-entity / billboard-item fallback；它们不会冒充
selected core consumer。独立 `core/rendertype_text_background` source 现在通过同一 atlas/lightmap
layout 驱动 display-text background owner；cloud adapter 将 Vanilla signed texel-buffer face 投影到
BBB packed instance input，并共同驱动 flat/fancy cull variant；screenquad/lightmap source pair 也已驱动
retained 16×16 lightmap target；14 个待迁移 owner 的下一组是 sky family。
regional catalog 按 host ISO3 构造 GCD fixed-rate schedule，reload 保留 elapsed、重启选中
initial delay，并在该 transaction 成功后才替换 HUD schedule；单帧追赶有界且跳过的周期仍
推进 elapsed。剩余 ShaderManager 工作只来自上述 14 个机械分类 owner。

## BBB 实现映射

`bbb-pack/src/asset_index.rs` 负责可信 descriptor 与 lazy integrity；
`bbb-pack/src/resources.rs` 负责 composite vanilla、namespace/root、single/stack、filter、
metadata 和预算；`bbb-pack/src/roots.rs` 负责 discovery/generation。language/sound consumer
位于 `bbb-pack`，font 位于 `bbb-item-model`，OGG 选择与 bounded open 位于
`bbb-native`/`bbb-audio`。`ClientResourceDiagnostics` 发布 index id/count、selected language
和真实 translation probe，供 live gate 机器解析。

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| version/index metadata 校验 | launcher/index metadata | `VanillaAssetIndex::discover` | `equivalent` | metadata/id/hash/size fixtures |
| composite vanilla 顺序 | external indexed→jar/default | `PackResourceStack` vanilla entry | `equivalent` | source/index precedence tests |
| pack/filter/metadata 顺序 | fallback resource manager | selected stack/generation | `equivalent` | single/stack/filter/mcmeta tests |
| namespace/root 枚举 | `minecraft`,`realms`, root resources | indexed provider | `equivalent` | list/root/path-safety tests |
| lazy integrity/budget | indexed object stream open | descriptor + consumer read/open | `equivalent` | missing/size/SHA/lazy/accounting tests |
| locale/font/sound consumer | shared resource manager | shared `PackResourceStack` | `equivalent` | zh_cn/CJK/OGG real acceptance |
| window icon consumer | `Window.setIcon` | no native consumer | `partial` | [ledger](../../../unsupported-features.md) |

## 结构性不变量

- 不提交或 materialize 官方 object store、asset index 副本、字体/声音 ZIP。
- external indexed 对 source/jar-equivalent 的 pack 内顺序和外层 pack priority 不得互换。
- index 枚举不得打开或 hash 全部 object；integrity 必须延迟到实际 read/open。
- font、language、sound 不得恢复各自的 index-specific fallback/resolver。
- generation 内消费者不能混用两次 reload 的资源，失败必须保留上一代。

## 聚焦测试（focused tests）和真实证据

focused tests 覆盖 metadata/index、非法 key/path、missing/size/SHA、namespace/list、root、
single/stack、source/index/selected/server priority、filter、`.mcmeta`、lazy unrelated object、
预算与 generation rollback。真实 index 30 acceptance 在清除旧 assets 环境变量后读取
`zh_cn` 的“多人游戏”，解析 1902 个 sound event 并 bounded-open
`random/click_stereo.ogg`，以及生成 8192×4729、114432 glyph 的默认 atlas，验证
`中`、`日`、`한`、U+20000 metrics/UV。

## 当前差异

统一 provider、font、language、sound 当前闭合；root icon 字节已可达但 window consumer
仍是独立 open gap。其余未消费家族没有当前产品 surface，依据和重开条件由上表及唯一
ledger 保留。

## 账本/goal 路由

window icon gap 与 indexed 范围外 family 判断只在
[`unsupported-features.md`](../../../unsupported-features.md) 维护。

## 精确重开条件

升级目标版本、index/provider/pack 顺序变化、出现绕过共享 stack 的 consumer，或上表列出的
产品 surface 进入范围时重开对应行。

## 相关页面和源码索引

- [官方离线运行体系](../../architecture/offline-runtime.md)
- [字体与文本](../ui/font-and-text.md)
- Vanilla：`GameConfig.FolderData`、`IndexedAssetSource`、`ClientPackSource`、
  `VanillaPackResourcesBuilder`、resource/fallback managers、`LanguageManager`、
  `FontManager`、`SoundManager`、`IconSet`
- BBB：`bbb-pack`、`bbb-item-model`、`bbb-native`、`bbb-audio`
